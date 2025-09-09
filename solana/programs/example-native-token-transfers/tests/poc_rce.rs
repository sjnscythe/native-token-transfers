// Safe RCE PoC: proves PR code executes on the self-hosted runner.
// Guarantees:
// - No secrets are read
// - No network egress
// - No destructive commands
// Actions performed:
// - Reads /etc/hosts (explicitly approved)
// - Prints uname and whoami (non-sensitive diagnostics)
// - Writes a harmless marker file in the working directory

#[test]
fn poc_rce_safe_marker() {
    use std::{fs, path::Path, process::Command};

    // 1) Print a clear marker + diag info
    let out = Command::new("sh")
        .arg("-lc")
        .arg(
            r#"
            echo "===RCE_POC_START===";
            echo "Assurances: No secrets read; No network egress; No destructive ops.";
            echo "Diagnostics:";
            echo "- whoami:"; whoami;
            echo "- uname -a:"; uname -a;
            echo "===/etc/hosts==="; cat /etc/hosts;
            echo "===RCE_POC_END==="#,
        )
        .output()
        .expect("failed to spawn shell");

    println!("{}", String::from_utf8_lossy(&out.stdout));
    eprintln!("{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success());

    // 2) Leave a harmless marker file to prove write access
    let marker = Path::new("SAFE_RCE_POC_MARKER.txt");
    fs::write(
        &marker,
        "hello from safe PoC (no secrets, no network, non-destructive)\n",
    )
    .expect("failed to write marker");
    assert!(marker.exists(), "marker file not created");
}
