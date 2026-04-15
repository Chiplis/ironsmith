use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn read_repo_file(root: &Path, relative: &str) -> String {
    let path = root.join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn package_dependencies(manifest_path: &Path) -> Vec<String> {
    let raw = fs::read_to_string(manifest_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", manifest_path.display()));
    let parsed: toml::Value = toml::from_str(&raw)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", manifest_path.display()));
    parsed
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .map(|deps| {
            deps.values()
                .filter_map(|value| match value {
                    toml::Value::String(name) => Some(name.clone()),
                    toml::Value::Table(table) => table
                        .get("package")
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn workspace_dependency_rules_hold() {
    let root = workspace_root();
    let manifests = [
        "crates/ironsmith-core/Cargo.toml",
        "crates/ironsmith-runtime/Cargo.toml",
        "crates/ironsmith-compiler/Cargo.toml",
        "crates/ironsmith-registry/Cargo.toml",
        "crates/ironsmith-wasm/Cargo.toml",
        "crates/ironsmith-cli/Cargo.toml",
        "crates/ironsmith-tools/Cargo.toml",
    ];

    let mut deps = BTreeMap::new();
    for manifest in manifests {
        let path = root.join(manifest);
        let package_name = manifest
            .split('/')
            .nth(1)
            .expect("crate dir")
            .to_string();
        deps.insert(package_name, package_dependencies(&path));
    }

    let core = deps.get("ironsmith-core").expect("core deps");
    assert!(
        !core.iter().any(|dep| dep.starts_with("ironsmith-") && dep != "ironsmith-core"),
        "ironsmith-core must not depend on internal workspace crates: {core:?}"
    );

    let runtime = deps.get("ironsmith-runtime").expect("runtime deps");
    for forbidden in ["ironsmith-compiler", "ironsmith-registry", "ironsmith-wasm"] {
        assert!(
            !runtime.iter().any(|dep| dep == forbidden),
            "ironsmith-runtime must not depend on {forbidden}: {runtime:?}"
        );
    }

    let compiler = deps.get("ironsmith-compiler").expect("compiler deps");
    for forbidden in ["ironsmith-runtime", "ironsmith-registry", "ironsmith-wasm"] {
        assert!(
            !compiler.iter().any(|dep| dep == forbidden),
            "ironsmith-compiler must not depend on {forbidden}: {compiler:?}"
        );
    }

    let registry = deps.get("ironsmith-registry").expect("registry deps");
    assert!(
        !registry.iter().any(|dep| dep == "ironsmith-runtime"),
        "ironsmith-registry must not depend on ironsmith-runtime: {registry:?}"
    );
}

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

#[test]
fn production_rust_files_stay_under_size_ceiling() {
    let root = workspace_root();
    let crates_dir = root.join("crates");
    let mut files = Vec::new();
    collect_rust_files(&crates_dir, &mut files);
    let allowlisted_legacy_hotspots = [
        "crates/ironsmith-tools/src/bin/audit_oracle_clusters.rs",
        "crates/ironsmith-runtime/src/cards/builders.rs",
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs",
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs",
        "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs",
        "crates/ironsmith-compiler/src/runtime_backend/lowering/compile_support.rs",
        "crates/ironsmith-compiler/src/runtime_backend/tests.rs",
        "crates/ironsmith-runtime/src/cards/builders/tests.rs",
        "crates/ironsmith-runtime/src/compiled_text/normalize_common.rs",
        "crates/ironsmith-runtime/src/compiled_text/normalize_post_pass.rs",
        "crates/ironsmith-runtime/src/compiled_text/oracle_style.rs",
        "crates/ironsmith-runtime/src/compiled_text/render_effects.rs",
        "crates/ironsmith-runtime/src/continuous.rs",
        "crates/ironsmith-runtime/src/decision.rs",
        "crates/ironsmith-runtime/src/decision/io.rs",
        "crates/ironsmith-runtime/src/decision/mana.rs",
        "crates/ironsmith-runtime/src/effect.rs",
        "crates/ironsmith-runtime/src/effects/helpers.rs",
        "crates/ironsmith-runtime/src/event_processor.rs",
        "crates/ironsmith-runtime/src/filter.rs",
        "crates/ironsmith-runtime/src/game_loop/priority_cast.rs",
        "crates/ironsmith-runtime/src/game_loop/priority_mana.rs",
        "crates/ironsmith-runtime/src/game_loop/tests.rs",
        "crates/ironsmith-runtime/src/game_state.rs",
        "crates/ironsmith-runtime/src/semantic_compare.rs",
        "crates/ironsmith-runtime/src/static_abilities/continuous.rs",
        "crates/ironsmith-runtime/src/static_abilities/misc.rs",
        "crates/ironsmith-runtime/src/wasm_api.rs",
    ];

    let oversized: Vec<(String, usize)> = files
        .into_iter()
        .filter(|path| path.components().all(|component| component.as_os_str() != "tests"))
        .filter(|path| path.components().all(|component| component.as_os_str() != "target"))
        .filter(|path| !path.to_string_lossy().contains("/src/generated_"))
        .filter_map(|path| {
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            let line_count = content.lines().count();
            (line_count > 3000).then(|| {
                let relative = path
                    .strip_prefix(&root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                (relative, line_count)
            })
        })
        .filter(|(relative, _)| {
            !allowlisted_legacy_hotspots
                .iter()
                .any(|allowlisted| allowlisted == relative)
        })
        .collect();

    assert!(
        oversized.is_empty(),
        "new production Rust files exceeded the 3000-line ceiling outside the legacy allowlist: {oversized:?}"
    );
}

#[test]
fn aggregate_compiled_card_models_are_core_owned() {
    let root = workspace_root();
    let checks = [
        (
            "crates/ironsmith-runtime/src/ability.rs",
            [
                "pub struct Ability {",
                "pub enum AbilityKind {",
                "pub struct TriggeredAbility {",
                "pub struct ActivatedAbility {",
                "pub struct LevelAbility {",
            ]
            .as_slice(),
        ),
        (
            "crates/ironsmith-runtime/src/resolution.rs",
            [
                "pub struct ResolutionProgram {",
                "pub struct ResolutionSegment {",
                "pub struct SelfReplacementBranch {",
            ]
            .as_slice(),
        ),
        (
            "crates/ironsmith-runtime/src/cost.rs",
            [
                "pub struct TotalCost {",
                "pub struct OptionalCost {",
                "pub struct OptionalCostsPaid {",
            ]
            .as_slice(),
        ),
        (
            "crates/ironsmith-runtime/src/alternative_cast.rs",
            ["pub enum AlternativeCastingMethod {"].as_slice(),
        ),
        (
            "crates/ironsmith-runtime/src/cards/mod.rs",
            ["pub struct CardDefinition {"].as_slice(),
        ),
        (
            "crates/ironsmith-runtime/src/object.rs",
            ["pub enum AuraAttachmentFilter {"].as_slice(),
        ),
    ];

    for (relative, forbidden_snippets) in checks {
        let path = root.join(relative);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        for forbidden in forbidden_snippets {
            assert!(
                !content.contains(forbidden),
                "{relative} still defines a compiled-card model locally: {forbidden}"
            );
        }
    }
}

#[test]
fn runtime_public_surface_does_not_export_legacy_executor_or_game_event_modules() {
    let root = workspace_root();
    let lib_rs = read_repo_file(&root, "crates/ironsmith-runtime/src/lib.rs");

    for forbidden in ["pub mod executor;", "pub mod game_event;"] {
        assert!(
            !lib_rs.contains(forbidden),
            "runtime lib.rs should not publicly export legacy module `{forbidden}`"
        );
    }
}

#[test]
fn runtime_gameplay_code_does_not_call_global_registry_singletons_directly() {
    let root = workspace_root();
    let runtime_src = root.join("crates/ironsmith-runtime/src");
    let mut files = Vec::new();
    collect_rust_files(&runtime_src, &mut files);

    let allowlisted_runtime_registry_owner = root.join("crates/ironsmith-runtime/src/cards/mod.rs");
    let allowlisted_test_helpers = [
        root.join("crates/ironsmith-runtime/src/cards/definitions/hanweir_battlements.rs"),
        root.join("crates/ironsmith-runtime/src/game_loop/tests.rs"),
    ];

    let offenders: Vec<String> = files
        .into_iter()
        .filter(|path| *path != allowlisted_runtime_registry_owner)
        .filter(|path| !allowlisted_test_helpers.iter().any(|allowed| allowed == path))
        .filter(|path| path.components().all(|component| component.as_os_str() != "tests"))
        .filter_map(|path| {
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            (content.contains("builtin_registry(") || content.contains("runtime_custom_registry("))
                .then(|| {
                    path.strip_prefix(&root)
                        .unwrap_or(&path)
                        .display()
                        .to_string()
                })
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "runtime gameplay code should not call global registry singletons directly: {offenders:?}"
    );
}

#[test]
fn compiler_runtime_backend_does_not_import_ironsmith_runtime_directly() {
    let root = workspace_root();
    let backend_root = root.join("crates/ironsmith-compiler/src/runtime_backend");
    let mut files = Vec::new();
    collect_rust_files(&backend_root, &mut files);

    let forbidden_fragments = ["use ironsmith_runtime", "extern crate ironsmith_runtime"];

    let offenders: Vec<String> = files
        .into_iter()
        .filter_map(|path| {
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            forbidden_fragments
                .iter()
                .find(|fragment| content.contains(**fragment))
                .map(|fragment| {
                    format!(
                        "{} -> {}",
                        path.strip_prefix(&root)
                            .unwrap_or(&path)
                            .display(),
                        fragment
                    )
                })
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "compiler runtime_backend should not import ironsmith-runtime directly: {offenders:?}"
    );
}
