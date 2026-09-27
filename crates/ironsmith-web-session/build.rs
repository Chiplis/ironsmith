use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=IRONSMITH_EMBEDDED_CARD_CATALOG");

    let initializer = match env::var_os("IRONSMITH_EMBEDDED_CARD_CATALOG") {
        Some(path) => {
            let requested = PathBuf::from(path);
            let resolved = requested.canonicalize().unwrap_or_else(|error| {
                panic!("cannot embed card catalog {}: {error}", requested.display())
            });
            assert!(
                resolved.is_file(),
                "embedded card catalog must be a file: {}",
                resolved.display()
            );
            println!("cargo:rerun-if-changed={}", requested.display());
            println!("cargo:rerun-if-changed={}", resolved.display());
            let resolved = resolved
                .to_str()
                .expect("embedded card catalog path must be valid UTF-8");
            format!("Some(include_bytes!({resolved:?}))")
        }
        None => "None".to_owned(),
    };
    let source =
        format!("const EMBEDDED_CARD_CATALOG_BYTES: Option<&'static [u8]> = {initializer};\n");
    let destination = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR"))
        .join("embedded_card_catalog_bytes.rs");
    if fs::read(&destination).ok().as_deref() != Some(source.as_bytes()) {
        fs::write(destination, source).expect("failed to write embedded catalog include");
    }
}
