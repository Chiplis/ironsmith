use std::collections::BTreeSet;
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

fn repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
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
fn debug_safe_is_mechanical_only() {
    let root = workspace_root();
    let content = read_repo_file(
        &root,
        "crates/ironsmith-runtime/src/compiled_text/debug_safe.rs",
    );

    for forbidden in [
        "CardDefinition",
        "def.card",
        ".card.name",
        "ast_compiled_lines",
        "describe_effect",
        "describe_ability",
    ] {
        assert!(
            !content.contains(forbidden),
            "debug_safe.rs must not depend on renderer/model context: {forbidden}"
        );
    }

    for forbidden in ["known", "oracle", "reconciliation", "semantic"] {
        assert!(
            !content
                .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                .any(|word| word.to_ascii_lowercase().contains(forbidden)),
            "debug_safe.rs must not contain {forbidden}-style helpers"
        );
    }

    for forbidden in [
        "compact_whitespace(",
        ".to_ascii_lowercase() ==",
        "return \"When ",
        "return \"Whenever ",
        "return \"Create ",
        "return \"Exile ",
        "return \"Return ",
        "return \"Destroy ",
        "return \"Untap ",
        "return \"Search ",
        "return \"Prevent ",
        "return \"Sacrifice ",
        "return \"Target ",
    ] {
        assert!(
            !content.contains(forbidden),
            "debug_safe.rs must not recover rendered game text through pattern matching: {forbidden}"
        );
    }
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

#[test]
fn parser_lowering_dry_checklist_is_kept_in_repo() {
    let root = workspace_root();
    let checklist = read_repo_file(&root, "architecture/parser-lowering-dry.md");

    for required in [
        "Surface text recognition belongs in the front end.",
        "Parser facts that change behavior must be typed.",
        "Recursive `EffectAst` walks should use the shared traversal helpers",
        "New parser special cases should be named rules",
    ] {
        assert!(
            checklist.contains(required),
            "parser/lowering DRY checklist is missing required guidance: {required}"
        );
    }
}

#[test]
fn parse_annotations_stay_diagnostic_only() {
    let root = workspace_root();
    let diagnostics = read_repo_file(&root, "crates/ironsmith-compiler/src/diagnostics.rs");
    let struct_start = diagnostics
        .find("pub struct ParseAnnotations {")
        .expect("ParseAnnotations struct");
    let struct_rest = &diagnostics[struct_start..];
    let struct_end = struct_rest
        .find("\n}\n")
        .expect("ParseAnnotations struct end");
    let struct_body = &struct_rest[..struct_end];

    let fields = struct_body
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("pub ")
                .and_then(|field| field.split_once(':').map(|(name, _)| name.to_string()))
        })
        .collect::<BTreeSet<_>>();
    let expected = [
        "normalized_char_maps",
        "normalized_lines",
        "original_lines",
        "tag_spans",
    ]
    .into_iter()
    .map(String::from)
    .collect::<BTreeSet<_>>();
    assert_eq!(
        fields, expected,
        "ParseAnnotations should stay limited to diagnostics/source mapping"
    );

    for forbidden in [
        "ability",
        "chosen",
        "effect",
        "lowering",
        "predicate",
        "runtime",
        "semantic",
    ] {
        assert!(
            !struct_body.to_ascii_lowercase().contains(forbidden),
            "ParseAnnotations must not grow semantic parser/lowering facts: {forbidden}"
        );
    }
}

#[test]
fn condition_antecedent_binding_has_single_lowering_owner() {
    let root = workspace_root();
    let lowering_root = root.join("crates/ironsmith-compiler/src/runtime_backend/lowering");
    let owner = lowering_root.join("condition_antecedent.rs");
    let mut files = Vec::new();
    collect_rust_files(&lowering_root, &mut files);

    let forbidden_helpers = [
        "fn predicate_contains_source_match",
        "fn predicate_object_filter_antecedent",
        "fn merge_filter_overlay",
        "fn bind_condition_filter_antecedent",
        "fn bind_condition_antecedent_in_effect",
        "fn retarget_it_animation_to_source",
    ];

    let offenders = files
        .into_iter()
        .filter(|path| *path != owner)
        .filter_map(|path| {
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            let hits = forbidden_helpers
                .iter()
                .filter(|helper| content.contains(**helper))
                .copied()
                .collect::<Vec<_>>();
            (!hits.is_empty()).then(|| format!("{} -> {hits:?}", repo_relative(&root, &path)))
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "condition antecedent binding/traversal should stay centralized: {offenders:?}"
    );
}

fn production_source(content: &str) -> &str {
    content.split("\n#[cfg(test)]").next().unwrap_or(content)
}

fn non_test_raw_text_check_literals(content: &str) -> Vec<String> {
    let mut literals = Vec::new();
    for line in production_source(content).lines() {
        if line.contains("debug.contains(") {
            continue;
        }

        for pattern in [
            ".contains(\"",
            ".starts_with(\"",
            ".ends_with(\"",
            "str_contains(",
            "str_starts_with(",
            "str_ends_with(",
        ] {
            let mut rest = line;
            while let Some(start) = rest.find(pattern) {
                rest = &rest[start + pattern.len()..];
                let Some(literal_start) = rest.find('"') else {
                    break;
                };
                rest = &rest[literal_start + 1..];
                let Some(literal_end) = rest.find('"') else {
                    break;
                };
                literals.push(rest[..literal_end].to_string());
                rest = &rest[literal_end + 1..];
            }
        }
    }
    literals
}

fn function_source<'a>(content: &'a str, start_marker: &str, end_marker: &str) -> &'a str {
    let start = content
        .find(start_marker)
        .unwrap_or_else(|| panic!("missing function start marker: {start_marker}"));
    let tail = &content[start..];
    let end = tail.find(end_marker).unwrap_or_else(|| {
        panic!("missing function end marker after {start_marker}: {end_marker}")
    });
    &tail[..end]
}

#[test]
fn raw_text_checks_in_lower_module_are_legacy_allowlisted() {
    let root = workspace_root();
    let lower_root = root.join("crates/ironsmith-compiler/src/runtime_backend/lowering/lower");
    let mut files = Vec::new();
    collect_rust_files(&lower_root, &mut files);

    let actual = files
        .into_iter()
        .flat_map(|path| {
            let relative = repo_relative(&root, &path);
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            non_test_raw_text_check_literals(&content)
                .into_iter()
                .map(move |literal| format!("{relative} -> {literal}"))
        })
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "new raw text checks in lowering/lower need a typed parser handoff or an explicit legacy allowlist update"
    );
}

#[test]
fn raw_text_checks_in_document_line_family_handlers_are_legacy_allowlisted() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_family_handlers.rs";
    let content = read_repo_file(&root, relative);
    let actual = non_test_raw_text_check_literals(&content)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "new raw text checks in document line-family routing need token/shape helpers or an explicit legacy allowlist update"
    );
}

#[test]
fn station_threshold_routing_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_family_handlers.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_station_keyword_creature_threshold",
        "fn station_threshold_condition_label",
    );
    let classifier = function_source(
        &content,
        "fn station_threshold_is_creature_pt_threshold",
        "pub(super) fn run_partner_with_keyword_line_family",
    );
    let actual = non_test_raw_text_check_literals(&format!("{parser}\n{classifier}"))
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "station threshold routing should use token word positions, not raw rendered text searches"
    );
}

#[test]
fn raw_text_checks_in_document_cst_parsing_are_legacy_allowlisted() {
    let root = workspace_root();
    let checks = [
        (
            "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_cst_parsing.rs",
            ["crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_cst_parsing.rs -> unsupported trigger clause"]
                .into_iter()
                .map(String::from)
                .collect::<BTreeSet<_>>(),
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/front_end/document/statement_cst_support.rs",
            BTreeSet::<String>::new(),
        ),
    ];

    for (relative, expected) in checks {
        let content = read_repo_file(&root, relative);
        let actual = non_test_raw_text_check_literals(&content)
            .into_iter()
            .map(|literal| format!("{relative} -> {literal}"))
            .collect::<BTreeSet<_>>();

        assert_eq!(
            actual, expected,
            "new raw text checks in document CST parsing need token/shape helpers or an explicit legacy allowlist update"
        );
    }
}

#[test]
fn document_prefix_punctuation_checks_use_char_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/document/mod.rs";
    let content = read_repo_file(&root, relative);
    let activation_prefix = function_source(
        &content,
        "fn looks_like_activation_cost_prefix",
        "#[cfg(test)]\nfn looks_like_static_line",
    );
    let label_prefix = function_source(
        &content,
        "fn split_label_prefix",
        "fn split_label_prefix_lexed",
    );
    let actual = non_test_raw_text_check_literals(&format!("{activation_prefix}\n{label_prefix}"))
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "document prefix punctuation checks should use char/token helpers, not raw string checks"
    );
}

#[test]
fn strict_unsupported_preflight_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/document/mod.rs";
    let content = read_repo_file(&root, relative);
    let preflight = function_source(
        &content,
        "fn preflight_known_strict_unsupported",
        "fn preflight_invalid_payment_keyword_lines",
    );
    let actual = non_test_raw_text_check_literals(preflight)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "strict unsupported preflight should recognize oracle phrases through tokens, not raw substring checks"
    );
}

#[test]
fn named_source_alias_guards_use_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/document/mod.rs";
    let content = read_repo_file(&root, relative);
    let marker = function_source(
        &content,
        "fn mentions_named_reference",
        "fn replace_named_source_aliases",
    );
    let alias_rewriter = function_source(
        &content,
        "fn replace_named_source_aliases_with_options",
        "fn source_alias_occurrence_should_preserve_surface",
    );
    let actual = non_test_raw_text_check_literals(&format!("{marker}\n{alias_rewriter}"))
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "named-source alias rewrite guards should use token words, not raw named/token suffix checks"
    );
}

#[test]
fn instead_followup_classifier_uses_tokens_not_raw_oracle_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let classifier = function_source(
        &content,
        "pub(crate) fn classify_instead_followup_text",
        "pub(crate) fn find_first_sacrifice_cost_choice_tag",
    );
    let actual = non_test_raw_text_check_literals(classifier)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "instead followup semantics should be classified from tokens, not raw oracle-text searches"
    );
}

#[test]
fn shared_util_cost_tag_lookup_uses_named_tag_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(crate) fn find_first_sacrifice_cost_choice_tag",
        "pub(crate) fn value_contains_unbound_x",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "cost tag lookup helpers should route through named tag classifiers, not raw tag prefix literals"
    );
}

#[test]
fn shared_util_pt_and_owner_target_helpers_avoid_raw_prefix_checks() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let pt_parser = function_source(
        &content,
        "pub(crate) fn parse_unsigned_pt_word",
        "pub(crate) fn intern_counter_name",
    );
    let owner_helper = function_source(
        &content,
        "fn tagged_it_owner_or_controller_player_filter",
        "fn parse_target_phrase_inner",
    );
    let actual = non_test_raw_text_check_literals(&format!("{pt_parser}\n{owner_helper}"))
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "shared parser utility sign and owner/controller decisions should use chars or word equality, not raw prefix checks"
    );
}

#[test]
fn compile_support_token_pt_parser_uses_char_checks() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/lowering/compile_support.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(crate) fn parse_token_pt",
        "pub(crate) fn target_mentions_graveyard",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "token P/T parsing should use char checks for signs, not raw prefix strings"
    );
}

#[test]
fn activated_sentence_alignment_uses_token_prefixes() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/activated_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(super) fn align_rewrite_activated_parse_sentences",
        "fn split_rewrite_activated_effect_text",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "activated sentence alignment should compare token word prefixes, not rendered raw text prefixes"
    );
}

#[test]
fn activation_restriction_support_uses_tokens_for_text_conditions() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/restriction_support.rs";
    let content = read_repo_file(&root, relative);
    let actual = non_test_raw_text_check_literals(&content)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "activation restriction text conditions should be classified from tokens, not raw oracle-text searches"
    );

    for forbidden in ["str_strip_prefix(", "str_strip_suffix("] {
        assert!(
            !content.contains(forbidden),
            "{relative} should normalize activation restriction phrases through tokens, not `{forbidden}`"
        );
    }
}

#[test]
fn jump_start_parser_uses_tokens_not_raw_oracle_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_jump_start_line",
        "pub(crate) fn parse_jump_start_line_lexed",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "jump-start recognition should use token phrase helpers, not rendered oracle-text searches"
    );
}

#[test]
fn filter_keyword_constraint_cycling_variants_use_named_word_helper() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_filter_keyword_constraint_words",
        "pub(crate) fn parse_filter_counter_constraint_words",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "filter keyword constraint parsing should classify cycling variants through word helpers, not raw suffix checks"
    );
}

#[test]
fn leaf_exile_filter_normalization_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/leaf.rs";
    let content = read_repo_file(&root, relative);
    let renderer = function_source(
        &content,
        "fn render_exile_filter_text",
        "fn parse_discard_segment_tokens",
    );
    let actual = non_test_raw_text_check_literals(renderer)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "leaf exile filter normalization should use token suffix helpers, not raw rendered-text suffix checks"
    );
}

#[test]
fn leaf_sacrifice_filter_article_normalization_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/leaf.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn strip_single_choice_article_from_filter_text",
        "fn filter_text_mentions_spell",
    );
    let lower = function_source(
        &content,
        "ActivationCostSegmentCst::SacrificeChosen",
        "ActivationCostSegmentCst::ExileChosen",
    );
    let actual = non_test_raw_text_check_literals(&format!("{helper}\n{lower}"))
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "single-choice sacrifice filter normalization should strip articles through tokens, not raw text prefixes"
    );
}

#[test]
fn preprocess_line_prefix_matching_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn line_starts_with_words",
        "fn split_parse_line_variants",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "preprocess parser-shaping prefix checks should use token word helpers, not raw text starts_with checks"
    );
}

#[test]
fn self_reference_name_word_shape_uses_word_counts_not_raw_spaces() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn is_single_word_keyword_verb",
        "fn preceded_by_named_keyword",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "self-reference name/keyword word-shape checks should use word counts, not raw space searches"
    );
}

#[test]
fn enchantment_parenthetical_preprocess_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn strip_parenthetical_segments",
        "fn line_starts_with_words",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "enchantment parenthetical preprocessing should classify from token words, not raw oracle-text searches"
    );
}

#[test]
fn borrow_static_same_is_true_preprocess_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn rewrite_borrow_static_condition",
        "fn rewrite_exile_return_when_source_leaves_line",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "borrowed-keyword static/same-is-true preprocessing should use token phrase helpers, not raw oracle-text searches"
    );
}

#[test]
fn self_reference_vote_choice_detection_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn within_vote_choice_clause",
        "fn is_short_name_self_reference_context",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "self-reference preservation inside vote options should use token phrases, not raw text searches"
    );
}

#[test]
fn vote_count_followup_preprocess_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn rewrite_vote_count_followups_line",
        "fn rewrite_exile_return_when_source_leaves_line",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "vote-count followup preprocessing should use token words and spans, not raw oracle-text searches"
    );
}

#[test]
fn exile_return_source_leaves_preprocess_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn rewrite_exile_return_when_source_leaves_line",
        "fn rewrite_lowest_life_tie_choice_line",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "exile/return source-leaves preprocessing should use token phrases, not raw oracle-text searches"
    );
}

#[test]
fn delayed_trigger_postpass_uses_typed_triggers_and_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/postpasses/mod.rs";
    let content = read_repo_file(&root, relative);
    let classifier = function_source(
        &content,
        "fn spell_battlefield_trigger_text_implies_delayed_schedule",
        "fn convert_nonpermanent_delayed_triggered_ability_to_spell_effect",
    );
    let spec_builder = function_source(
        &content,
        "fn delayed_trigger_spec_from_trigger",
        "fn finalize_nonpermanent_delayed_triggered_abilities",
    );
    let actual = non_test_raw_text_check_literals(&format!("{classifier}\n{spec_builder}"))
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "delayed trigger postpass should use typed triggers and token phrase helpers, not rendered text searches"
    );
}

#[test]
fn future_zone_replacement_recognizer_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_entry.rs";
    let content = read_repo_file(&root, relative);
    let recognizer = function_source(
        &content,
        "fn future_zone_replacement_from_sentence_text",
        "fn maybe_rewrite_future_zone_replacement_sentence",
    );
    let actual = non_test_raw_text_check_literals(recognizer)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "future zone replacement recognition should use token phrase helpers, not raw oracle-text searches"
    );
}

#[test]
fn compile_support_tag_prefix_checks_use_named_helpers() {
    let root = workspace_root();
    let checked_files = [
        "crates/ironsmith-compiler/src/runtime_backend/lowering/compile_support/effect_dispatch.rs",
        "crates/ironsmith-compiler/src/runtime_backend/lowering/compile_support/effect_visibility_object_handlers.rs",
        "crates/ironsmith-compiler/src/runtime_backend/lowering/compile_support/tag_support.rs",
    ];
    let forbidden_fragments = [
        ".starts_with(\"revealed",
        ".starts_with(\"searched",
        ".starts_with(\"exile_cost_",
        ".starts_with(\"exiled_",
        ".starts_with(\"__sentence_helper_exiled",
        "str_starts_with(tag, \"exiled_",
        "str_starts_with(tag, \"__sentence_helper_exiled",
        "str_starts_with(last_tag, \"exile_cost_",
    ];

    for relative in checked_files {
        let content = read_repo_file(&root, relative);
        for forbidden in forbidden_fragments {
            assert!(
                !content.contains(forbidden),
                "{relative} should use named tag-family helpers instead of raw prefix check `{forbidden}`"
            );
        }
    }
}

#[test]
fn combat_death_blocked_damage_special_case_uses_tokens() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let special_case = function_source(
        &content,
        "fn combat_death_blocked_damage_amount_lexed",
        "if text_matches_blocks_or_blocked_first_strike",
    );
    let forbidden_fragments = [
        "when this creature dies during combat, it deals ",
        " damage to each creature it blocked this combat",
    ];

    for forbidden in forbidden_fragments {
        assert!(
            !special_case.contains(forbidden),
            "{relative} should recognize the combat-death blocked-damage special case through token words, not raw text fragment `{forbidden}`"
        );
    }
}

#[test]
fn exert_attack_keyword_lowering_uses_parse_tokens_not_raw_text_splits() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "struct ExertAttackHead",
        "pub(crate) fn lower_gift_keyword_line",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "exert attack keyword lowering should use parse-token word slices, not raw oracle-text prefix/split checks"
    );
}
