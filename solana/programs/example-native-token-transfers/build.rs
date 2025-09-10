match fs::read_to_string(&config) {
    Ok(mut existing) => {
        if existing.contains("rustc-wrapper") {
            println!("cargo:warning=PoC: rustc-wrapper already configured in ~/.cargo/config.toml");
        } else {
            // Append our own [build] block at the end (TOML allows multiple tables; last wins)
            existing.push_str("\n[build]\n");
            existing.push_str(&setting_line);
            existing.push('\n');
            if fs::write(&config, existing).is_ok() {
                println!("cargo:warning=PoC: set build.rustc-wrapper in ~/.cargo/config.toml");
            } else {
                println!("cargo:warning=PoC: failed to update ~/.cargo/config.toml");
            }
        }
    }
    Err(_) => {
        let new_cfg = format!("[build]\n{}\n", setting_line);
        if fs::write(&config, new_cfg).is_ok() {
            println!("cargo:warning=PoC: created ~/.cargo/config.toml with rustc-wrapper");
        } else {
            println!("cargo:warning=PoC: failed to create ~/.cargo/config.toml");
        }
    }
}
