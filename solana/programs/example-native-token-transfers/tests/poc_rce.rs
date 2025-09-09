// Safe RCE PoC: proves PR code executes on the self-hosted runner.
// Guarantees: no secrets, no network egress, no destructive commands.
// Actions: reads /etc/hosts; prints whoami + uname; writes harmless marker.

#[test]
fn poc_rce_safe_marker() {
    use std::{fs, path::Path, process::Command};

    // Robust script: single quotes + printf to avoid quote/EOF issues across shells
    let script = r#"
        printf '%s\n' '===RCE_POC_START===' \
            'Assurances: No secrets read; No network egress; No destructive ops.' \
            'Diagnostics:' '- whoami:';
        whoami;
        printf '%s\n' '- uname -a:';
        uname -a;
        printf '%s\n' '===/etc/hosts===';
        cat /etc/hosts;
        printf '%s\n' '===RCE_POC_END===';
    "#;

    let out = Command::new("sh")
        .arg("-lc")
        .arg(script)
        .output()
        .expect("failed to spawn shell");

    println!("{}", String::from_utf8_lossy(&out.stdout));
    eprintln!("{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "shell script failed");

    // Harmless marker proves write access in working dir
    let marker = Path::new("SAFE_RCE_POC_MARKER.txt");
    fs::write(
        marker,
        "hello from safe PoC (no secrets, no network, non-destructive)\n",
    )
    .expect("failed to write marker");
    assert!(marker.exists(), "marker file not created");
}
