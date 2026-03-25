use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(root).unwrap_or_else(|err| panic!("failed to read {}: {err}", root.display()));

    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("failed to enumerate {}: {err}", root.display()));
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn definitions_files(manifest_dir: &str) -> Vec<PathBuf> {
    let root = Path::new(manifest_dir)
        .join("src")
        .join("cards")
        .join("definitions");
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);
    files.sort();
    files
        .into_iter()
        .filter(|path| !path.ends_with("builder.rs"))
        .collect()
}

fn stripped_non_comment_content(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                return None;
            }
            Some(
                line.split_once("//")
                    .map_or(line, |(head, _)| head)
                    .to_string(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// Keep hand-written card definitions on the parser/compiled-text path.
// `src/cards/definitions` is allowed to describe metadata plus oracle text, but
// not to hardcode runtime abilities/effects directly; that boundary keeps card
// behavior flowing through the same compilation pipeline the rest of the engine uses.
fn enforce_definition_builder_boundary(manifest_dir: &str) {
    let forbidden = [
        "use crate::cards::builders::CardDefinitionBuilder;",
        "use crate::cards::CardDefinitionBuilder;",
        "crate::cards::builders::CardDefinitionBuilder::new(",
        "crate::cards::CardDefinitionBuilder::new(",
        ".with_ability(",
        ".with_abilities(",
        ".with_etb(",
        ".with_dies_trigger(",
        ".with_upkeep_trigger(",
        ".with_trigger(",
        ".with_targeted_etb(",
        ".with_optional_trigger(",
        ".with_activated(",
        ".with_tap_ability(",
        ".with_spell_effect(",
        ".with_chapter(",
        ".with_chapters(",
        ".with_level_abilities(",
        ".spell_effect =",
        ".abilities =",
        ".alternative_casts =",
        ".optional_costs =",
        ".aura_attach_filter =",
        ".has_fuse =",
        ".additional_cost =",
        ".max_saga_chapter =",
        "CardDefinition::spell(",
        "CardDefinition::spell_with_abilities(",
        "CardDefinition::with_abilities(",
    ];

    let mut violations = Vec::new();
    for path in definitions_files(manifest_dir) {
        println!("cargo:rerun-if-changed={}", path.display());
        let content = stripped_non_comment_content(&path);
        let hits = forbidden
            .iter()
            .filter(|needle| content.contains(**needle))
            .map(|needle| needle.to_string())
            .collect::<Vec<_>>();
        if !hits.is_empty() {
            violations.push(format!("{}\n{}", path.display(), hits.join("\n")));
        }
    }

    assert!(
        violations.is_empty(),
        "cards::definitions may only use metadata setters plus parse/compile methods.\nForbidden raw builder/effect APIs found in:\n{}",
        violations.join("\n\n")
    );
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    enforce_definition_builder_boundary(&manifest_dir);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    let out_file = out_dir.join("generated_registry.rs");

    // Speed up parser/effects iteration by default: only generate/compile the
    // massive baked registry when the feature is explicitly enabled.
    if env::var_os("CARGO_FEATURE_GENERATED_REGISTRY").is_none() {
        let stub = r#"
pub const GENERATED_PARSER_CARD_SOURCE_COUNT: usize = 0;
pub fn generated_parser_entry_count() -> usize { 0 }
pub fn generated_parser_card_names() -> Vec<String> { Vec::new() }
pub fn register_generated_parser_cards(_registry: &mut crate::cards::CardRegistry) {}
pub fn register_generated_parser_cards_chunk(
    _registry: &mut crate::cards::CardRegistry,
    cursor: usize,
    _chunk_size: usize,
) -> usize {
    cursor
}
pub fn register_generated_parser_cards_if_name<F>(
    _registry: &mut crate::cards::CardRegistry,
    _include_name: F,
) where
    F: FnMut(&str) -> bool,
{
}
pub fn generated_parser_semantic_score(_name: &str) -> Option<f32> { None }
pub fn generated_parser_semantic_threshold_counts() -> [usize; 100] { [0; 100] }
pub fn generated_parser_semantic_scored_count() -> usize { 0 }
pub fn generated_parser_card_parse_source(_name: &str) -> Option<(String, String)> { None }
pub fn try_compile_card_by_name(_name: &str) -> Result<crate::cards::CardDefinition, String> {
    Err("generated registry not available".to_string())
}
"#;
        fs::write(&out_file, stub).expect("failed to write generated_registry.rs stub");
        return;
    }

    println!("cargo:rerun-if-changed=scripts/generate_baked_registry.py");
    println!("cargo:rerun-if-changed=scripts/stream_scryfall_blocks.py");
    println!("cargo:rerun-if-env-changed=IRONSMITH_GENERATED_REGISTRY_SCORES_FILE");
    println!("cargo:rerun-if-env-changed=IRONSMITH_REGISTRY_DB_PATH");

    if let Some(scores_file) = env::var_os("IRONSMITH_GENERATED_REGISTRY_SCORES_FILE") {
        let scores_file = PathBuf::from(scores_file);
        println!("cargo:rerun-if-changed={}", scores_file.display());
    }
    let registry_db_path = env::var_os("IRONSMITH_REGISTRY_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("reports/engine-status.sqlite3"));
    println!("cargo:rerun-if-changed={}", registry_db_path.display());

    let python = env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    let status = Command::new(python)
        .current_dir(&manifest_dir)
        .arg("scripts/generate_baked_registry.py")
        .arg("--db-path")
        .arg(&registry_db_path)
        .arg("--out")
        .arg(&out_file)
        .status()
        .expect("failed to run scripts/generate_baked_registry.py");

    assert!(
        status.success(),
        "scripts/generate_baked_registry.py failed with status {status}"
    );
}
