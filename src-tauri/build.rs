use std::fs;
use std::path::Path;

fn load_env_file() {
    let env_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.env");
    println!("cargo:rerun-if-changed={}", env_path.display());

    let Ok(contents) = fs::read_to_string(&env_path) else {
        return;
    };

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            println!("cargo:rustc-env={}={}", key.trim(), value.trim());
        }
    }
}

fn main() {
    load_env_file();
    tauri_build::build()
}
