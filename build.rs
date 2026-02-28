use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=cards.json");
    println!("cargo:rerun-if-changed=scripts/generate_baked_registry.py");
    println!("cargo:rerun-if-changed=scripts/stream_scryfall_blocks.py");
    println!("cargo:rerun-if-env-changed=IRONSMITH_GENERATED_REGISTRY_SKIP_NAMES_FILE");

    if let Some(skip_file) = env::var_os("IRONSMITH_GENERATED_REGISTRY_SKIP_NAMES_FILE") {
        let skip_file = PathBuf::from(skip_file);
        println!("cargo:rerun-if-changed={}", skip_file.display());
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    let out_file = out_dir.join("generated_registry.rs");

    // Speed up parser/effects iteration by default: only generate/compile the
    // massive baked registry when the feature is explicitly enabled.
    if env::var_os("CARGO_FEATURE_GENERATED_REGISTRY").is_none() {
        let stub = r#"
pub const GENERATED_PARSER_CARD_SOURCE_COUNT: usize = 0;
pub fn register_generated_parser_cards(_registry: &mut crate::cards::CardRegistry) {}
pub fn register_generated_parser_cards_if_name<F>(
    _registry: &mut crate::cards::CardRegistry,
    _include_name: F,
) where
    F: FnMut(&str) -> bool,
{
}
"#;
        fs::write(&out_file, stub).expect("failed to write generated_registry.rs stub");
        return;
    }

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
