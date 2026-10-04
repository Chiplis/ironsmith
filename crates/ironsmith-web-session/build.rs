use std::env;
use std::fs;
use std::path::PathBuf;

#[cfg(feature = "dynamic-compile")]
fn dungeon_card_relocations(value: &serde_json::Value, path: &str, start: u32, count: u32,
    out: &mut Vec<serde_json::Value>) {
    match value {
        serde_json::Value::Object(fields) => {
            if let Some(card_value) = fields.get("card") {
                // Recognize the complete typed Card schema, not arbitrary IDs
                // in event/effect payloads. Token definitions nest this schema.
                if let Ok(card) = serde_json::from_value::<ironsmith_core::card::Card>(card_value.clone()) {
                    for (field, id) in [("id", Some(card.id)), ("other_face", card.other_face)] {
                        if let Some(id) = id {
                            let offset = id.0.checked_sub(start).expect("builtin card identity precedes source allocation");
                            assert!(offset < count, "builtin card identity escapes source allocation");
                            out.push(serde_json::json!({"pointer": format!("{path}/card/{field}"), "original": id.0, "offset": offset}));
                        }
                    }
                }
            }
            for (key, child) in fields {
                let key = key.replace('~', "~0").replace('/', "~1");
                dungeon_card_relocations(child, &format!("{path}/{key}"), start, count, out);
            }
        }
        serde_json::Value::Array(values) => for (index, child) in values.iter().enumerate() {
            dungeon_card_relocations(child, &format!("{path}/{index}"), start, count, out);
        },
        _ => {}
    }
}

#[cfg(feature = "dynamic-compile")]
fn bake_builtin_dungeons() {
    let artifacts: Vec<_> = ironsmith_card_source_build::dungeon_sources().into_iter().map(|source| {
        let before = ironsmith_core::ids::snapshot_id_counters();
        ironsmith_compiler_runtime_build::compile_to_runtime_definition(&source.name, source.block.clone(), false)
            .unwrap_or_else(|error| panic!("cannot compile builtin dungeon {}: {error}", source.name));
        let after = ironsmith_core::ids::snapshot_id_counters();
        assert_eq!((before.player, before.object), (after.player, after.object),
            "builtin source compilation must not allocate gameplay identities");
        let card_id_allocation = after.card.checked_sub(before.card).expect("card counter overflow");
        // The build script is single-threaded apart from this worker; align
        // artifact compilation with the legacy source's identity namespace.
        ironsmith_core::ids::restore_id_counters(before);
        let artifact = ironsmith_compiler_runtime_build::compile_to_artifact(&source.name, source.block, false)
            .unwrap_or_else(|error| panic!("cannot bake builtin dungeon {}: {error}", source.name)).0;
        let definition = serde_json::to_value(&artifact.payload.definition).expect("builtin definition serializes");
        let mut relocations = Vec::new();
        dungeon_card_relocations(&definition, "", before.card, card_id_allocation, &mut relocations);
        assert!(relocations.iter().any(|entry| entry["pointer"] == "/card/id"));
        serde_json::json!({ "artifact": artifact, "card_id_allocation": card_id_allocation, "relocations": relocations })
    }).collect();
    let bytes = serde_json::to_vec(&artifacts).expect("builtin dungeon artifacts serialize");
    let destination = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR"))
        .join("builtin_dungeon_artifacts.json");
    if fs::read(&destination).ok().as_deref() != Some(bytes.as_slice()) {
        fs::write(destination, bytes).expect("failed to write builtin dungeon artifacts");
    }
}

fn main() {
    #[cfg(feature = "dynamic-compile")]
    std::thread::Builder::new()
        .name("bake-builtin-dungeons".into())
        // Compiler/typed-artifact serde frames exceed the platform main
        // thread's stack; use the repository's native compiler test allowance.
        .stack_size(64 * 1024 * 1024)
        .spawn(bake_builtin_dungeons)
        .expect("failed to start builtin dungeon compilation")
        .join()
        .expect("builtin dungeon compilation failed");
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
