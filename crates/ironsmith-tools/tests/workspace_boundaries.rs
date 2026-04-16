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
                .enumerate()
                .filter_map(|(index, value)| match value {
                    toml::Value::String(name) => Some(name.clone()),
                    toml::Value::Table(table) => table
                        .get("package")
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned)
                        .or_else(|| deps.keys().nth(index).map(std::string::ToString::to_string)),
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
        let package_name = manifest.split('/').nth(1).expect("crate dir").to_string();
        deps.insert(package_name, package_dependencies(&path));
    }

    let core = deps.get("ironsmith-core").expect("core deps");
    assert!(
        !core
            .iter()
            .any(|dep| dep.starts_with("ironsmith-") && dep != "ironsmith-core"),
        "ironsmith-core must not depend on internal workspace crates: {core:?}"
    );

    let runtime = deps.get("ironsmith-runtime").expect("runtime deps");
    for forbidden in ["ironsmith-compiler", "ironsmith-registry", "ironsmith-wasm"] {
        assert!(
            !runtime.iter().any(|dep| dep == forbidden),
            "ironsmith-runtime must not depend on {forbidden}: {runtime:?}"
        );
    }

    let runtime_lib_rs = read_repo_file(&root, "crates/ironsmith-runtime/src/lib.rs");
    assert!(
        !runtime_lib_rs.contains("compiler_integration"),
        "runtime must not export compiler_integration after compiler/runtime boundary cleanup"
    );

    let compiler = deps.get("ironsmith-compiler").expect("compiler deps");
    for forbidden in ["ironsmith-runtime", "ironsmith-registry", "ironsmith-wasm"] {
        assert!(
            !compiler.iter().any(|dep| dep == forbidden),
            "ironsmith-compiler must not depend on {forbidden}: {compiler:?}"
        );
    }
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
        "crates/ironsmith-core/src/effect.rs",
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
        .filter(|path| {
            path.components()
                .all(|component| component.as_os_str() != "tests")
        })
        .filter(|path| {
            path.components()
                .all(|component| component.as_os_str() != "target")
        })
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
fn migrated_effect_payloads_are_core_owned() {
    let root = workspace_root();
    let core_effect = read_repo_file(&root, "crates/ironsmith-core/src/effect.rs");
    let compiler_effects = read_repo_file(&root, "crates/ironsmith-compiler/src/effects/mod.rs");
    let migrated = [
        "AddManaEffect",
        "AddManaFromCommanderColorIdentityEffect",
        "AddManaOfAnyColorEffect",
        "AddManaOfAnyOneColorEffect",
        "AttachObjectsEffect",
        "AttachToEffect",
        "BolsterEffect",
        "CantEffect",
        "CastSourceEffect",
        "CastTaggedEffect",
        "ChooseCardNameEffect",
        "ChooseCardTypeEffect",
        "ChooseNewTargetsEffect",
        "CipherEffect",
        "ConsultTopOfLibraryEffect",
        "ControlPlayerEffect",
        "CreateEmblemEffect",
        "DiscardHandEffect",
        "DiscoverEffect",
        "EmitKeywordActionEffect",
        "EnergyCountersEffect",
        "ExchangeTextBoxesEffect",
        "ExileInsteadOfGraveyardEffect",
        "ExtraTurnAfterNextTurnEffect",
        "ExtraTurnEffect",
        "FlipEffect",
        "GainLifeEffect",
        "GrantBySpecEffect",
        "GrantEffect",
        "LookAtTopCardsEffect",
        "LoseTheGameEffect",
        "MayMoveToZoneEffect",
        "ModifyPowerToughnessForEachEffect",
        "MonstrosityEffect",
        "MoveAllCountersEffect",
        "NinjutsuCostEffect",
        "NinjutsuEffect",
        "PreventAllCombatDamageEffect",
        "PreventAllDamageEffect",
        "ProliferateEffect",
        "PutOntoBattlefieldEffect",
        "PutTaggedRemainderOnLibraryBottomEffect",
        "RearrangeLookedCardsInLibraryEffect",
        "RegenerateEffect",
        "RemoveAnyCountersAmongEffect",
        "RemoveUpToAnyCountersEffect",
        "RenownEffect",
        "RepeatEffectsEffect",
        "RepeatProcessEffect",
        "ReturnFromGraveyardToBattlefieldEffect",
        "ReturnFromGraveyardToHandEffect",
        "RevealTopEffect",
        "SacrificePlayerEffect",
        "SetBasePowerToughnessEffect",
        "ShuffleLibraryEffect",
        "ShuffleObjectsIntoLibraryEffect",
        "TagAttachedToSourceEffect",
        "TagTriggeringObjectEffect",
        "TransformEffect",
        "UnearthEffect",
        "VoteEffect",
        "WinTheGameEffect",
    ];

    for effect in migrated {
        assert!(
            core_effect.contains(&format!("pub struct {effect}"))
                || core_effect.contains(&format!("pub enum {effect}")),
            "{effect} data must live in ironsmith-core"
        );
        assert!(
            !compiler_effects.contains(&format!("pub struct {effect}"))
                && !compiler_effects.contains(&format!("pub enum {effect}")),
            "compiler effects module must not define a local {effect} payload after migration"
        );
    }

    let integration = read_repo_file(&root, "crates/ironsmith-registry/src/compiler_runtime.rs");
    assert!(
        !integration.contains(
            "ironsmith::effects::GainLifeEffect::new(payload.amount.clone(), payload.player.clone())",
        ),
        "compiler integration must not reconstruct GainLifeEffect field by field"
    );
}

#[test]
fn migrated_static_ability_model_is_core_owned() {
    let root = workspace_root();
    let core_static_abilities =
        read_repo_file(&root, "crates/ironsmith-core/src/static_ability_model.rs");
    let compiler_static_abilities =
        read_repo_file(&root, "crates/ironsmith-compiler/src/static_abilities.rs");

    assert!(
        core_static_abilities.contains("pub struct StaticAbility<"),
        "core must define the shared StaticAbility data model"
    );
    assert!(
        core_static_abilities.contains("pub enum StaticAbilityPayload<"),
        "core must define the shared StaticAbilityPayload data model"
    );
    assert!(
        !compiler_static_abilities.contains("pub struct StaticAbility"),
        "compiler must not define a local StaticAbility data model after migration"
    );
    assert!(
        !compiler_static_abilities.contains("pub enum StaticAbilityPayload"),
        "compiler must not define a local StaticAbilityPayload data model after migration"
    );
    assert!(
        compiler_static_abilities
            .contains("pub type StaticAbility = ironsmith_core::StaticAbility<"),
        "compiler should alias the core StaticAbility data model"
    );
    assert!(
        compiler_static_abilities
            .contains("pub type StaticAbilityPayload = ironsmith_core::StaticAbilityPayload<"),
        "compiler should alias the core StaticAbilityPayload data model"
    );

    assert!(
        !root
            .join("crates/ironsmith-runtime/src/compiler_integration.rs")
            .exists(),
        "runtime compiler integration module should be deleted after compiler/runtime boundary cleanup"
    );
}

#[test]
fn compiler_boundary_adapter_has_no_semantic_conversion_tables() {
    let root = workspace_root();
    let core_cost = read_repo_file(&root, "crates/ironsmith-core/src/cost_model.rs");
    let core_ability = read_repo_file(&root, "crates/ironsmith-core/src/ability_model.rs");
    let core_definition = read_repo_file(&root, "crates/ironsmith-core/src/definition_model.rs");
    let compiler_costs = read_repo_file(&root, "crates/ironsmith-compiler/src/costs/mod.rs");
    let runtime_effect_interpreter = read_repo_file(
        &root,
        "crates/ironsmith-runtime/src/effect_model_interpreter.rs",
    );
    let integration = read_repo_file(&root, "crates/ironsmith-registry/src/compiler_runtime.rs");

    assert!(
        core_cost.contains("pub enum Cost<E>"),
        "core must own the non-static cost data model"
    );
    assert!(
        compiler_costs.contains("pub type Cost = ironsmith_core::Cost<crate::effect::Effect>;"),
        "compiler costs should alias the core cost data model"
    );
    assert!(
        core_cost.contains("impl<E> CoreCostComponent for Cost<E>"),
        "core-owned costs should provide the core tap-cost constructor"
    );
    assert!(
        core_ability.contains("pub fn try_map<SA2, T2, E2, C2, Error>"),
        "core Ability should own structural mapping across non-static families"
    );
    assert!(
        core_definition.contains("pub fn try_map<A2, E2, C2, AC2, OC2, Error>"),
        "core CardDefinition should own structural mapping across non-static families"
    );

    for forbidden in [
        "fn convert_cost",
        "fn convert_total_cost",
        "fn convert_optional_cost",
        "fn convert_resolution_program",
        "fn convert_effect(",
        "fn convert_ability",
        "fn convert_card_definition",
    ] {
        assert!(
            !integration.contains(forbidden),
            "compiler boundary adapter must not recreate the old non-static conversion table entry `{forbidden}`"
        );
    }

    assert!(
        runtime_effect_interpreter.contains("pub trait EffectModel")
            && runtime_effect_interpreter.contains("pub fn interpret_effect_model"),
        "runtime should own the compiler-free effect model interpreter"
    );
    assert!(
        integration.contains(
            "impl ironsmith::effect_model_interpreter::EffectModel for CompilerEffectModel"
        ),
        "compiler boundary adapter should stay a compiler model adapter, not the semantic effect table"
    );
}

#[test]
fn runtime_public_surface_does_not_export_legacy_executor_or_game_event_modules() {
    let root = workspace_root();
    let lib_rs = read_repo_file(&root, "crates/ironsmith-runtime/src/lib.rs");
    let effects_mod = read_repo_file(&root, "crates/ironsmith-runtime/src/effects/mod.rs");

    for forbidden in ["pub mod executor;", "pub mod game_event;"] {
        assert!(
            !lib_rs.contains(forbidden),
            "runtime lib.rs should not publicly export legacy module `{forbidden}`"
        );
    }

    assert!(
        !root
            .join("crates/ironsmith-runtime/src/executor.rs")
            .exists(),
        "legacy executor.rs should not exist once effect execution is effects-owned"
    );
    assert!(
        !lib_rs.contains("ExecutionContext"),
        "runtime lib.rs should not publicly surface the legacy ExecutionContext name"
    );
    assert!(
        !lib_rs.contains("\npub mod compiler;\n"),
        "runtime lib.rs should not publicly export the legacy compiler prelude/module surface"
    );
    assert!(
        !lib_rs.contains("pub mod tooling;"),
        "runtime lib.rs should not expose parse-backed tooling helpers; tooling belongs in ironsmith-tools"
    );
    assert!(
        !root
            .join("crates/ironsmith-runtime/src/tooling.rs")
            .exists(),
        "runtime tooling.rs should not exist after parse-backed tooling moved to ironsmith-tools"
    );
    assert!(
        !lib_rs.contains("pub use cards::{CardDefinition, CardDefinitionBuilder, CardRegistry};")
            && !lib_rs.contains("\npub use cards::CardDefinitionBuilder;"),
        "runtime lib.rs should not publicly export CardDefinitionBuilder in normal builds"
    );
    let cards_mod = read_repo_file(&root, "crates/ironsmith-runtime/src/cards/mod.rs");
    assert!(
        !cards_mod.contains("\npub use builders::{ParseAnnotations, TextSpan};"),
        "runtime cards module should not export parser annotation/span helper types in normal builds"
    );
    let runtime_builders = read_repo_file(&root, "crates/ironsmith-runtime/src/cards/builders.rs");
    for forbidden in [
        "#[path = \"../../../ironsmith-compiler/",
        "include_str!(\n                \"../../../ironsmith-compiler/",
        "../../../ironsmith-compiler/src/runtime_backend",
    ] {
        assert!(
            !runtime_builders.contains(forbidden),
            "runtime cards/builders.rs should not path-load or include compiler backend sources: {forbidden}"
        );
    }
    assert!(
        effects_mod.contains("pub type EffectContext<'a> = context::ExecutionContext<'a>;"),
        "effects/mod.rs should keep EffectContext as the public execution-context name"
    );
    assert!(
        effects_mod.contains("pub(crate) use context::ExecutionContext;"),
        "effects/mod.rs should keep ExecutionContext crate-private for runtime internals"
    );
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
        .filter(|path| {
            !allowlisted_test_helpers
                .iter()
                .any(|allowed| allowed == path)
        })
        .filter(|path| {
            path.components()
                .all(|component| component.as_os_str() != "tests")
        })
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
                        path.strip_prefix(&root).unwrap_or(&path).display(),
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

#[test]
fn runtime_code_does_not_import_legacy_executor_module_paths() {
    let root = workspace_root();
    let runtime_src = root.join("crates/ironsmith-runtime/src");
    let mut files = Vec::new();
    collect_rust_files(&runtime_src, &mut files);

    let offenders: Vec<String> = files
        .into_iter()
        .filter_map(|path| {
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            (content.contains("crate::executor")
                || content.contains("use crate::executor")
                || content.contains("super::executor"))
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
        "runtime source should not import legacy executor module paths: {offenders:?}"
    );
}
