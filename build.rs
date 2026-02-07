use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=cards.json");
    println!("cargo:rerun-if-changed=scripts/generate_baked_registry.py");
    println!("cargo:rerun-if-changed=scripts/stream_scryfall_blocks.py");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    let out_file = out_dir.join("generated_registry.rs");

    let python = env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    let status = Command::new(python)
        .current_dir(&manifest_dir)
        .arg("scripts/generate_baked_registry.py")
        .arg("--out")
        .arg(&out_file)
        .status()
        .expect("failed to run scripts/generate_baked_registry.py");

    assert!(
        status.success(),
        "scripts/generate_baked_registry.py failed with status {status}"
    );
}
