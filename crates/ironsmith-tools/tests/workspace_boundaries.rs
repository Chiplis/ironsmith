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

        if line.contains(".split_whitespace()") {
            literals.push("split_whitespace()".to_string());
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

    for relative in
        ["crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs"]
    {
        let content = read_repo_file(&root, relative);
        assert!(
            !content.contains("starts_with(prefix) && trimmed.len() > prefix.len()"),
            "{relative} should classify any-number deck-construction lines through token words, not raw prefix strings"
        );
    }
}

#[test]
fn runtime_backend_facade_frequency_and_kicker_postpasses_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/mod.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(crate) fn trigger_frequency_condition",
        "#[cfg(test)]\n#[path = \"tests.rs\"]",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "runtime backend facade trigger-frequency/kicker postpasses should use lexed tokens and grammar shapes, not raw source-text checks"
    );

    for required in [
        "trigger_frequency_text_ast",
        "parser_token_word_refs",
        "FIRST_TIME_EACH_OR_THIS_TURN_PATTERN.matches_words",
        "BECOMES_CREWED_PATTERN.matches_words",
        "parse_do_this_only_each_turn_limit",
        "LexPattern::amount(\"limit\", LexCaptureKind::OneOf(&[\"once\", \"twice\"]))",
        "capture_clause_by_role(LexCaptureRole::Amount",
        "KICKED_COUNTER_SPELL_MANA_VALUE_REPLACEMENT_PATTERN.matches_words",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should preserve facade postpass semantics through token-backed shape/capture helpers: missing `{required}`"
        );
    }
}

#[test]
fn middle_parser_production_raw_text_checks_are_classified() {
    let root = workspace_root();
    let scan_roots = [
        "crates/ironsmith-compiler/src/runtime_backend/families",
        "crates/ironsmith-compiler/src/runtime_backend/front_end",
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower",
        "crates/ironsmith-compiler/src/runtime_backend/sentences",
    ];
    let extra_files = ["crates/ironsmith-compiler/src/runtime_backend/lowering/compile_support.rs"];
    let skipped = BTreeSet::from([
        "crates/ironsmith-compiler/src/runtime_backend/front_end/token_primitives.rs",
    ]);

    let mut actual = BTreeSet::new();
    for scan_root in scan_roots {
        let mut files = Vec::new();
        collect_rust_files(&root.join(scan_root), &mut files);
        for path in files {
            let relative = repo_relative(&root, &path);
            if skipped.contains(relative.as_str()) {
                continue;
            }
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            actual.extend(
                non_test_raw_text_check_literals(&content)
                    .into_iter()
                    .map(|literal| format!("{relative} -> {literal}")),
            );
        }
    }
    for relative in extra_files {
        let content = read_repo_file(&root, relative);
        actual.extend(
            non_test_raw_text_check_literals(&content)
                .into_iter()
                .map(|literal| format!("{relative} -> {literal}")),
        );
    }

    let expected = BTreeSet::from([
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_cst_parsing.rs -> unsupported trigger clause".to_string(),
    ]);

    assert_eq!(
        actual, expected,
        "middle parser production raw text checks should be token/shape-based; allowlisted entries must stay diagnostic-only"
    );
}

#[test]
fn mana_group_inner_text_uses_token_helper() {
    let root = workspace_root();
    let scan_roots = [
        "crates/ironsmith-compiler/src/runtime_backend/families",
        "crates/ironsmith-compiler/src/runtime_backend/front_end",
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower",
        "crates/ironsmith-compiler/src/runtime_backend/sentences",
    ];
    let allowed = BTreeSet::from([
        "crates/ironsmith-compiler/src/runtime_backend/front_end/lexer.rs",
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/values.rs",
    ]);
    let forbidden_fragments = [
        "trim_start_matches('{').trim_end_matches('}')",
        "trim_start_matches(\"{\").trim_end_matches(\"}\")",
        "trim_matches('{').trim_matches('}')",
        "trim_matches(\"{\").trim_matches(\"}\")",
    ];

    let mut offenders = Vec::new();
    for scan_root in scan_roots {
        let mut files = Vec::new();
        collect_rust_files(&root.join(scan_root), &mut files);
        for path in files {
            let relative = repo_relative(&root, &path);
            if allowed.contains(relative.as_str()) {
                continue;
            }
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            for fragment in forbidden_fragments {
                if production_source(&content).contains(fragment) {
                    offenders.push(format!("{relative} -> {fragment}"));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "mana group brace peeling should use OwnedLexToken::mana_group_inner outside lexer/raw-value parsing: {offenders:?}"
    );
}

#[test]
fn line_lowering_creature_type_choice_uses_parser_token_words() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/line_lowering.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn creature_type_choice_program",
        "fn optional_zone_rewrite_effect",
    );

    assert!(
        parser.contains("parser_token_word_refs(tokens)"),
        "{relative} should derive creature-type-choice words from parser tokens"
    );
    assert!(
        parser.contains("tokens: &[OwnedLexToken]"),
        "{relative} should accept creature-type-choice parser tokens instead of re-lexing rendered text"
    );
    assert!(
        content.contains("creature_type_choice_program(&normalized_tokens, &compiled)"),
        "{relative} should feed creature-type-choice detection from the existing normalized token stream"
    );
    for forbidden in [
        ".split_whitespace()",
        "lex_line(normalized_line",
        "normalized_line: &str",
        "trim_matches(|ch: char|",
        "!ch.is_alphanumeric()",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not recover creature-type-choice parser words through raw text `{forbidden}`"
        );
    }
}

#[test]
fn line_lowering_cost_reduction_cap_uses_mana_group_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/line_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn rewrite_self_spell_cost_modifier",
        "fn lower_static_ability_chunk",
    );

    for required in [
        "lex_line(info.normalized.normalized.as_str(), info.line_index)",
        "tokens_start_with_this_spell_cost(&tokens)",
        "extract_cost_reduction_cap_from_tokens(&tokens)",
        "find_token_word_sequence_span(tokens, &[\"by\", \"more\", \"than\"])",
        "TokenKind::ManaGroup",
        "mana_group_inner()",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should parse cost-reduction caps from token words and mana groups: missing `{required}`"
        );
    }

    for forbidden in [
        "fn extract_cost_reduction_cap_from_text(raw_line: &str)",
        "rewrite_self_spell_cost_modifier(ability, info.raw_line.as_str())",
        "text_starts_with_this_spell_cost(raw_line)",
        "lex_line(raw_line, 0)",
        "to_ascii_lowercase()",
        ".find(\"by more than {\")",
        ".find('}')",
        "\"by more than {\".len()",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not parse cost-reduction caps through raw string slicing `{forbidden}`"
        );
    }
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

    for forbidden in [
        "raw.get(\"Partner\".len()..)",
        "rest.starts_with('-')",
        "str_starts_with_char(rest, '\\u{2014}')",
        "str_starts_with_char(rest, '\\u{2013}')",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should classify partner variant separators through lexed punctuation tokens, not raw fragment `{forbidden}`"
        );
    }
}

#[test]
fn championed_with_this_trigger_rewrite_uses_lexed_comma_split() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_family_handlers.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(super) fn run_championed_with_this_trigger_line_family",
        "pub(super) fn run_max_speed_labeled_line_family",
    );

    for required in [
        "split_once_on_comma_tokens(&ctx.line.tokens)",
        "token.kind == TokenKind::Comma",
        "render_token_slice(tokens_without_terminal_period(effect_tokens))",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should rewrite championed-with-this triggers from lexed comma/effect tokens: missing `{required}`"
        );
    }

    for forbidden in [
        "let raw = ctx.line.info.raw_line.trim()",
        "str_split_once_char(raw, ',')",
        "effect_text.trim_start().trim_end_matches('.')",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not rewrite championed-with-this triggers with raw string splitting `{forbidden}`"
        );
    }
}

#[test]
fn max_speed_labeled_line_family_uses_lexed_body_and_comma_splits() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_family_handlers.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(super) fn run_max_speed_labeled_line_family",
        "pub(super) fn run_start_your_engines_line_family",
    );

    for required in [
        "max_speed_body_text_from_tokens(ctx.line)",
        "max_speed_intervening_if_text(&body_line.tokens)",
        "render_tokens_without_terminal_period(&body_line.tokens)",
        "parse_static_line_cst(&body_line)",
        "TokenKind::Dash | TokenKind::EmDash | TokenKind::Colon",
        "token.kind == TokenKind::Comma",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should route max-speed labeled rows through lexed body/comma helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "str_find_char(raw, '\\u{2014}')",
        "str_split_once_char(raw, '-')",
        "str_split_once_char(trimmed, ',')",
        "body_text.trim_end_matches('.')",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not split max-speed labels and trigger bodies with raw string fragment `{forbidden}`"
        );
    }
}

#[test]
fn non_turn_conditional_untap_line_family_uses_token_sentence_split() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_family_handlers.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(super) fn run_non_turn_conditional_untap_line_family",
        "pub(super) fn run_statement_probe_line_family",
    );

    for required in [
        "non_turn_conditional_untap_first_sentence_tokens(ctx.line)",
        "TokenWordView::new(&line.tokens)",
        "NON_TURN_UNTAP_SUFFIX_PATTERN",
        "matches_words(&crate::runtime_backend::token_word_refs(&line.tokens))",
        "words.token_index_for_word_index(suffix_word_idx)",
        "token.kind == TokenKind::Period",
        "tokens_without_terminal_period(prefix_tokens)",
        "render_original_text_for_token_slice(ctx.line, first_sentence_tokens)",
        "\"If it's not your turn, untap those creatures\"",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should split non-turn untap compound lines from token words and sentence tokens: missing `{required}`"
        );
    }

    for forbidden in [
        "ctx.line.info.raw_line.trim()",
        ".to_ascii_lowercase()",
        "str_rfind(",
        "format!(\". {SUFFIX}\")",
        "text_starts_with_words(",
        "trim_end_matches('.')",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not split non-turn untap compound lines through raw text `{forbidden}`"
        );
    }
}

#[test]
fn document_partner_parenthetical_trims_use_token_kinds() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_family_handlers.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(super) fn run_partner_with_keyword_line_family",
        "pub(super) fn run_combined_static_line_family",
    );

    assert!(
        helper.contains("partner_with_name_from_line")
            && helper.contains("source_before_reminder_or_period")
            && helper.contains("TokenKind::LParen")
            && helper.contains("render_original_text_for_token_slice(line, name_tokens)"),
        "{relative} should trim partner-with names from token-slice original rendering and partner variant display labels using lexed token kinds"
    );
    for forbidden in [
        "partner_with_name_from_line(ctx.line.info.raw_line.as_str()",
        "str_split_once_char(raw, '(')",
        "str_split_once_char(rest, '(')",
        "\"partner with \".len()",
        "trim_end_matches('.')",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not trim partner parentheticals with raw string branch `{forbidden}`"
        );
    }
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

    let explicit_row_parser = function_source(
        &content,
        "fn parse_station_threshold_line",
        "fn parse_station_keyword_creature_threshold",
    );
    assert!(
        explicit_row_parser.contains("TokenKind::Pipe")
            && explicit_row_parser.contains("TokenKind::Plus")
            && explicit_row_parser
                .contains("render_original_text_for_token_slice(line, body_tokens)"),
        "{relative} should parse explicit station threshold rows and body text from lexed pipe/plus/body tokens"
    );
    for forbidden in [
        "fn parse_station_threshold_line(raw_line: &str",
        "raw_line.get(",
        "str_split_once_char(raw_line, '|')",
        ".strip_suffix('+')",
        ".trim().parse::<i32>()",
    ] {
        assert!(
            !explicit_row_parser.contains(forbidden),
            "{relative} should not parse explicit station threshold rows with raw string branch `{forbidden}`"
        );
    }
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

        assert!(
            !content.contains("starts_with(\"a deck can have any number of cards named \")"),
            "{relative} should classify any-number deck-construction lines through token words, not raw prefix strings"
        );
    }
}

#[test]
fn modal_mode_cst_uses_token_text_not_raw_bullet_stripping() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_cst_parsing.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(super) fn parse_modal_mode_cst",
        "pub(super) fn parse_saga_chapter_line_cst",
    );
    let document_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/mod.rs";
    let document_content = read_repo_file(&root, document_relative);
    let original_text_helper = function_source(
        &document_content,
        "fn render_original_text_for_token_slice",
        "fn line_starts_with_lparen_token",
    );

    for required in [
        "strip_modal_bullet_prefix_tokens(&line.tokens)",
        "strip_non_keyword_label_prefix_lexed(",
        "render_original_text_for_token_slice(line, parse_tokens)",
        "parse_effect_sentences_lexed(parse_tokens)",
        "TokenKind::Bullet | TokenKind::Dash",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should derive modal mode text/effects from the same token slice: missing `{required}`"
        );
    }

    for forbidden in [
        "raw_line",
        "trim_start_matches",
        "strip_non_keyword_label_prefix(raw_mode)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not derive modal mode text with raw string stripping `{forbidden}`"
        );
    }

    for required in ["span_from_tokens(tokens)", "line.info.normalized.char_map"] {
        assert!(
            original_text_helper.contains(required),
            "{document_relative} should map token slices back to original text with token spans: missing `{required}`"
        );
    }
    for required in [
        "line_mentions_this_permanent_token_phrase(candidates[0].as_str())",
        "THIS_PERMANENT_PHRASE_PATTERN.matches_words(&token_word_refs(&tokens))",
    ] {
        assert!(
            original_text_helper.contains(required),
            "{document_relative} should fan out this-permanent named-source candidates through token-backed phrase helpers: missing `{required}`"
        );
    }
    assert!(
        !original_text_helper.contains("candidates[0].contains(\"this permanent\")"),
        "{document_relative} should not detect this-permanent fallback candidates with raw substring checks"
    );
}

#[test]
fn weighted_modal_header_detection_uses_tokens_and_shapes() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/modal_and_level_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn header_mentions_modal_point_cost",
        "fn parse_leading_modal_point_cost",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "{relative} should detect weighted modal headers from lexed tokens and ClauseShape helpers, not raw `{{P}}` text probes"
    );

    for required in [
        "header_mentions_modal_point_cost_lexed(&tokens)",
        "pawprint_modal_label_count(token)",
        "MODAL_POINT_COST_HEADER_TAIL_PATTERN.matches_words",
        "parser_token_word_refs(tokens)",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should preserve weighted modal header detection through token-backed grammar helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "text.to_ascii_lowercase().contains(\"{p} worth of modes\")",
        ".contains(\"{p} worth of modes\")",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not detect weighted modal headers with raw text branch `{forbidden}`"
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
    let bullet_prefix = function_source(
        &content,
        "fn is_bullet_line",
        "fn parse_trigger_intro_tokens",
    );
    let label_prefix = function_source(
        &content,
        "fn split_label_prefix",
        "fn split_label_prefix_lexed",
    );
    let numeric_result_prefix = function_source(
        &content,
        "fn looks_like_numeric_result_prefix_text",
        "#[cfg(test)]",
    );
    let parenthetical_prefix = function_source(
        &content,
        "fn line_starts_with_lparen_token",
        "fn split_trigger_sentence_chunks_rewrite_lexed",
    );
    let actual = non_test_raw_text_check_literals(&format!(
        "{activation_prefix}\n{bullet_prefix}\n{label_prefix}\n{numeric_result_prefix}\n{parenthetical_prefix}"
    ))
    .into_iter()
    .map(|literal| format!("{relative} -> {literal}"))
    .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "document prefix punctuation checks should use char/token helpers, not raw string checks"
    );

    assert!(
        activation_prefix.contains("tokens: &[OwnedLexToken]"),
        "{relative} should classify activation cost prefixes from lexed tokens"
    );
    for required in [
        "fn is_bullet_line(line: &PreprocessedLine)",
        "line.tokens.first()",
        "TokenKind::Bullet",
        "TokenKind::Dash",
        "TokenKind::Number",
    ] {
        assert!(
            bullet_prefix.contains(required),
            "{relative} should classify modal bullet rows from tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn looks_like_activation_cost_prefix(raw: &str)",
        ".split_whitespace()",
        "str_starts_with_char(trimmed",
        "fn is_bullet_line(line: &str)",
        "trim_start()",
        "chars.next()",
    ] {
        assert!(
            !activation_prefix.contains(forbidden) && !bullet_prefix.contains(forbidden),
            "{relative} should not classify prefix punctuation with raw string logic `{forbidden}`"
        );
    }

    assert!(
        numeric_result_prefix.contains("looks_like_numeric_result_prefix_lexed(&tokens)"),
        "{relative} should classify numeric result prefixes through the lexed helper"
    );
    for required in [
        "fn line_starts_with_lparen_token(line: &PreprocessedLine)",
        "token.kind == TokenKind::LParen",
        "token.kind == TokenKind::RParen",
    ] {
        assert!(
            parenthetical_prefix.contains(required),
            "{relative} should classify fully parenthetical activation lines from token kinds: missing `{required}`"
        );
    }
    for forbidden in [".split_once('—')", ".split_once('-')", ".split_once('|')"] {
        assert!(
            !numeric_result_prefix.contains(forbidden),
            "{relative} should not classify numeric result prefixes with raw split `{forbidden}`"
        );
    }
    for forbidden in [
        "fn is_fully_parenthetical_line(text: &str)",
        "text.trim()",
        "str_starts_with_char(trimmed, '(')",
        "str_ends_with_char(trimmed, ')')",
    ] {
        assert!(
            !parenthetical_prefix.contains(forbidden),
            "{relative} should not classify parenthetical activation lines with raw text `{forbidden}`"
        );
    }
}

#[test]
fn document_ability_word_label_detection_uses_label_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/document/mod.rs";
    let content = read_repo_file(&root, relative);
    let splitter = function_source(
        &content,
        "fn split_label_prefix_lexed",
        "fn is_nonkeyword_choice_labeled_line",
    );
    let classifier = function_source(
        &content,
        "fn looks_like_ability_word_label",
        "fn rewrite_line_normalized",
    );

    assert!(
        splitter.contains("label_tokens"),
        "{relative} should return label tokens alongside rendered label display text"
    );
    assert!(
        classifier.contains("label_tokens: &[OwnedLexToken]"),
        "{relative} should classify ability-word labels from lexed label tokens"
    );
    for forbidden in [
        "label.trim()",
        "trimmed.contains('.')",
        "trimmed.contains(':')",
        "trimmed.split_whitespace()",
    ] {
        assert!(
            !classifier.contains(forbidden),
            "{relative} should not classify ability-word labels with raw string logic `{forbidden}`"
        );
    }
}

#[test]
fn grammar_effect_labeled_prefix_classifiers_use_parser_token_words() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects.rs";
    let content = read_repo_file(&root, relative);
    let labeled_prefix_support = function_source(
        &content,
        "fn labeled_prefix_tokens",
        "#[derive(Debug, Clone, PartialEq, Eq)]",
    );

    assert!(
        labeled_prefix_support.contains("lex_line(prefix.trim(), 0)"),
        "{relative} should lex label prefix text before classifying it"
    );
    assert!(
        labeled_prefix_support.contains("parser_token_word_refs(&tokens)"),
        "{relative} should classify label prefixes from parser token words"
    );
    for forbidden in [
        ".split_whitespace()",
        "trim_matches(|ch: char|",
        "!ch.is_ascii_alphanumeric()",
    ] {
        assert!(
            !labeled_prefix_support.contains(forbidden),
            "{relative} should not classify label prefixes through raw text `{forbidden}`"
        );
    }
}

#[test]
fn labeled_mana_value_bound_uses_parse_tokens_not_synthetic_words() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/labeled_prefixes.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn labeled_mana_value_or_less_bound",
        "pub(crate) fn parse_effect_sentence_inner_lexed",
    );

    for required in [
        "fn labeled_mana_value_or_less_bound(tokens: &[OwnedLexToken])",
        "find_token_word_sequence(&tokens[search_start..], &[\"mana\", \"value\"])",
        "parse_less_than_or_equal_quantity_prefix(\n                tail,",
        "labeled_mana_value_or_less_bound(tokens)",
    ] {
        assert!(
            helper.contains(required) || content.contains(required),
            "{relative} should parse labeled mana-value bounds from captured parse tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn labeled_mana_value_or_less_bound(words: &[&str])",
        "synthetic_word_tokens(tail)",
        "let count_tokens",
        "labeled_mana_value_or_less_bound(sentence_words.as_slice())",
    ] {
        assert!(
            !helper.contains(forbidden) && !content.contains(forbidden),
            "{relative} should not reconstruct mana-value bound tails with synthetic word tokens `{forbidden}`"
        );
    }
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
    let prefix_guard = function_source(
        &content,
        "fn source_alias_prefix_looks_like_effect_verb",
        "fn is_effect_verb_word",
    );
    let marker = function_source(
        &content,
        "fn mentions_named_reference",
        "fn replace_named_source_aliases",
    );
    let alias_rewriter = function_source(
        &content,
        "fn replace_named_source_aliases_with_options",
        "fn normalize_named_source_enter_agreement",
    );
    let trigger_guard = function_source(
        &content,
        "fn normalize_named_source_trigger_for_builder",
        "fn named_source_subject_for_builder",
    );
    let actual = non_test_raw_text_check_literals(&format!(
        "{prefix_guard}\n{marker}\n{alias_rewriter}\n{trigger_guard}"
    ))
    .into_iter()
    .map(|literal| format!("{relative} -> {literal}"))
    .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "named-source alias rewrite guards should use token words, not raw named/token suffix checks"
    );

    for forbidden in [
        ".contains(&format!(\"control of ",
        ".split(|ch: char| !ch.is_ascii_alphanumeric()",
        "str_strip_prefix(trimmed, &(name_lower + \" \"))",
        "str_strip_prefix(lower.as_str(), name_prefix.as_str())",
        "str_split_once(lower.as_str(), \" enters \")",
        "str_split_once(lower.as_str(), \",\")",
        "lower[cursor..].find(alias)",
        "lower.as_bytes()",
        "source_alias_occurrence_should_preserve_surface(bytes",
        "trigger_head.contains(\" leaves the battlefield\")",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should match dynamic named-source phrases through lexed words, not raw fragment `{forbidden}`"
        );
    }

    for required in [
        "source_alias_word_pieces(&tokens)",
        "source_alias_word_span_matches(&pieces, word_idx, &alias_words)",
        "source_alias_occurrence_should_preserve_surface_lexed(&pieces, word_idx, end_word)",
    ] {
        assert!(
            alias_rewriter.contains(required),
            "{relative} should rewrite dynamic named-source aliases through lexed word-piece spans: missing `{required}`"
        );
    }

    for required in [
        "trigger_head_is_source_alias_leaves_battlefield(builder, trigger_head.as_str())",
        "SOURCE_LEAVES_BATTLEFIELD_PATTERN.matches_words(&token_word_refs(&tokens))",
    ] {
        assert!(
            content.contains(required),
            "{relative} should preserve named-source leaves-battlefield guards through token-backed shape helpers: missing `{required}`"
        );
    }
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

    for forbidden in [
        "str_split_once(word, \"/\")",
        "power.starts_with(['+', '-'])",
        "toughness.starts_with(['+', '-'])",
    ] {
        assert!(
            !pt_parser.contains(forbidden),
            "{relative} should parse unsigned power/toughness token words through char helpers, not raw fragment `{forbidden}`"
        );
    }
}

#[test]
fn shared_util_level_header_parser_uses_tokens_not_raw_prefixes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_level_header",
        "pub(crate) fn parse_power_toughness",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "level header parsing should use lexed tokens and char checks, not raw prefix/suffix/split helpers"
    );
}

#[test]
fn shared_util_power_toughness_parser_uses_char_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_power_toughness",
        "pub(crate) fn parse_level_up_line",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "power/toughness parsing should use char helpers, not raw rendered-text prefix/suffix checks"
    );
}

#[test]
fn shared_util_saga_chapter_prefix_parser_uses_tokens_not_raw_splits() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_saga_chapter_prefix",
        "fn roman_to_int",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "saga chapter prefix parsing should split on lexed dash tokens, not rendered raw text"
    );
}

#[test]
fn shared_util_keyword_prefix_preservation_uses_parser_token_words() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn preserve_keyword_prefix_for_parse",
        "pub(crate) fn parse_self_free_cast_alternative_cost_line",
    );

    assert!(
        parser.contains("lex_line(prefix.trim(), 0)"),
        "{relative} should lex keyword prefixes before deciding whether to preserve them"
    );
    assert!(
        parser.contains("parser_token_word_refs(&tokens)"),
        "{relative} should classify keyword prefixes from parser token words"
    );
    for forbidden in [
        ".split_whitespace()",
        "trim_matches(|ch: char|",
        "!ch.is_ascii_alphanumeric()",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not preserve keyword prefixes through raw text `{forbidden}`"
        );
    }
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

    for forbidden in [
        "str_split_once_char(word, '/')",
        "left.starts_with(['+', '-'])",
        "right.starts_with(['+', '-'])",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should route token P/T parsing through the shared unsigned P/T word parser, not raw fragment `{forbidden}`"
        );
    }

    assert!(
        helper.contains("parse_unsigned_pt_word(word)"),
        "{relative} should reuse the shared unsigned P/T word parser"
    );
}

#[test]
fn compile_support_token_definition_uses_parser_token_words() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/lowering/compile_support.rs";
    let content = read_repo_file(&root, relative);
    let named_card = function_source(
        &content,
        "pub(crate) fn extract_named_card_name",
        "pub(crate) fn extract_leading_explicit_token_name",
    );
    let inline_damage = function_source(
        &content,
        "pub(crate) fn token_inline_noncreature_spell_each_opponent_damage_amount",
        "pub(crate) fn parse_crew_amount",
    );
    let token_definition = function_source(
        &content,
        "pub(crate) fn token_definition_for",
        "pub(crate) fn parse_token_pt",
    );
    let parser = format!("{inline_damage}\n{token_definition}");

    assert!(
        parser.contains("parser_token_word_refs(&tokens)"),
        "{relative} should derive token-definition words from parser tokens"
    );
    assert!(
        named_card.contains("token.parser_word_pieces()")
            && named_card.contains("span.start")
            && named_card.contains("span.end"),
        "{relative} should recover named-card token surfaces through parser word spans"
    );
    for forbidden in [
        ".split_whitespace()",
        "trim_matches(|ch: char|",
        "\"can't\" | \"cannot\" => \"cant\"",
        "has_text",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not classify token definitions through raw text `{forbidden}`"
        );
    }
    for forbidden in ["str_find(source_text, \"named\")", ".split_whitespace()"] {
        assert!(
            !named_card.contains(forbidden),
            "{relative} should not recover named-card surfaces through raw text `{forbidden}`"
        );
    }
}

#[test]
fn compile_support_token_rules_text_uses_lexed_structure() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/lowering/compile_support.rs";
    let content = read_repo_file(&root, relative);
    let helpers = function_source(
        &content,
        "fn render_trimmed_token_text",
        "fn normalize_token_self_reference_for_parser",
    );

    for required in [
        "TokenKind::Quote",
        "TokenKind::Apostrophe",
        "TokenKind::ManaGroup",
        "find_token_word_sequence_span(tokens, &[\"equipped\", \"creature\", \"has\"])",
        "find_token_word(&tokens, \"with\")",
        "mana_group_inner()",
        "render_token_slice(tokens)",
    ] {
        assert!(
            helpers.contains(required),
            "{relative} should extract quoted token/equipment rules through lexed structure: missing `{required}`"
        );
    }

    for forbidden in [
        "source_text.find('\"')",
        "rest.find('\"')",
        "source_text.to_ascii_lowercase()",
        ".split_once(':')",
        ".split_once(\"has \")",
        ".find('{')",
        ".find('}')",
        "trim_matches('\"')",
        "tail.contains(':')",
    ] {
        assert!(
            !helpers.contains(forbidden),
            "{relative} should not recover token/equipment rules text with raw string extraction `{forbidden}`"
        );
    }
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
fn activated_display_text_uses_presentation_label_not_raw_scan() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/activated_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn rewrite_activated_display_text",
        "fn infer_rewrite_activated_functional_zones",
    );

    assert!(
        helper.contains("line.presentation_label.as_deref()")
            && helper.contains("render_token_slice(&line.cost_parse_tokens)")
            && helper.contains("render_token_slice(&line.effect_parse_tokens)"),
        "{relative} should build activated presentation text from parsed label/cost/effect tokens"
    );
    for forbidden in [
        "line.info.raw_line.trim()",
        "raw.to_ascii_lowercase()",
        "str_find(raw_lower.as_str()",
        "str_split_once_char(raw, '—')",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not recover activated presentation text by scanning raw text `{forbidden}`"
        );
    }
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

    let normalizer = function_source(
        &content,
        "fn normalize_activation_restriction",
        "fn merge_mana_activation_conditions",
    );
    for required in [
        "tokens: &[OwnedLexToken]",
        "restriction_tokens_without_terminal_period(tokens)",
        "TokenWordView::new(tokens)",
        "render_token_slice(&tokens[start..end])",
    ] {
        assert!(
            normalizer.contains(required),
            "{relative} should normalize once-per-turn activation restrictions from existing tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "restriction.to_ascii_lowercase()",
        "lex_line(normalized.as_str(), 0)",
        "ACTIVATE_ONLY_ONCE_EACH_TURN_TEXT",
    ] {
        assert!(
            !normalizer.contains(forbidden),
            "{relative} should not re-normalize activation restrictions through raw text `{forbidden}`"
        );
    }
}

#[test]
fn activation_control_conditions_use_captured_control_parser() {
    let root = workspace_root();
    let parser_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/abilities.rs";
    let parser_content = read_repo_file(&root, parser_relative);
    let parser = function_source(
        &parser_content,
        "pub(crate) fn parse_activation_condition_lexed",
        "pub(crate) fn parse_activation_count_per_turn",
    );
    let helper_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let helper_content = read_repo_file(&root, helper_relative);

    for required in [
        "pub(crate) struct ControlConditionAst",
        "player: PlayerAst",
        "player_filter: Option<PlayerFilter>",
        "comparison: Comparison",
        "quantity_token_count: usize",
        "filter: ObjectFilter",
        "pub(crate) fn parse_control_condition(",
        "parse_quantity_comparison_prefix(tail_tokens, true, true, \"control condition\")",
        "parse_object_filter_with_grammar_entrypoint(filter_tokens, false)",
    ] {
        assert!(
            helper_content.contains(required),
            "{helper_relative} should parse reusable control-condition captures: missing `{required}`"
        );
    }

    assert!(
        parser.contains(
            "parse_control_condition(control_condition_tokens, ControlConditionOptions::default())"
        ) && parser.contains("ConditionExpr::PlayerHasAtLeast"),
        "{parser_relative} should build activation control predicates from captured control-condition pieces"
    );
    for forbidden in [
        "ARTIFACT_CONTROL_TAIL_PATTERN",
        "player_controls_at_least_condition_from_control_tail",
        "capture_counted_object_filter_tail",
    ] {
        assert!(
            !parser_content.contains(forbidden),
            "{parser_relative} should not keep bespoke exact control-condition helper `{forbidden}`"
        );
    }
}

#[test]
fn enters_tapped_unless_control_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/etb_static_lines.rs";
    let content = read_repo_file(&root, relative);
    let quantity_parser = function_source(
        &content,
        "fn parse_enters_tapped_unless_control_quantity_static_ability",
        "fn parse_enters_tapped_unless_a_player_has_13_or_less_life_condition",
    );
    let line_parser = function_source(
        &content,
        "pub(crate) fn parse_conditional_enters_tapped_unless_line",
        "pub(crate) fn parse_enters_with_additional_counter_for_filter_line",
    );

    assert!(
        quantity_parser.contains("grammar::conditions::parse_control_condition")
            && quantity_parser.contains("control_condition.quantity_token_count == 0")
            && quantity_parser.contains("comparison: control_condition.comparison"),
        "{relative} should build quantified ETB control conditions from captured control-condition pieces"
    );
    assert!(
        line_parser.contains("grammar::conditions::parse_control_condition")
            && line_parser.contains("ConditionExpr::YouControl(control_condition.filter)"),
        "{relative} should build generic ETB control conditions from the same captured parser"
    );
    for forbidden in [
        "find_token_index(\n            condition_tokens,\n            |token| ETB_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token)",
        "let filter_tokens = trim_edge_punctuation(&condition_tokens[control_idx + 1..])",
    ] {
        assert!(
            !line_parser.contains(forbidden) && !quantity_parser.contains(forbidden),
            "{relative} should not rescan control-condition tails by hand with `{forbidden}`"
        );
    }
}

#[test]
fn predicate_control_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_player_controls_predicate",
        "fn parse_this_ability_resolution_count_predicate",
    );

    assert!(
        parser.contains("grammar::conditions::parse_control_condition")
            && parser.contains("ControlConditionOptions")
            && parser.contains("predicate_from_control_condition(control_condition)"),
        "{relative} should parse player-control predicates through the shared captured control-condition parser"
    );
    assert!(
        parser.contains("fn predicate_from_control_condition")
            && parser.contains("PredicateAst::PlayerControlsExactly")
            && parser.contains("PredicateAst::PlayerHasAtLeast")
            && parser.contains("PredicateAst::PlayerHasAtLeastWithDifferentPowers")
            && parser.contains("PredicateAst::PlayerControls {"),
        "{relative} should lower captured control-condition pieces into the full predicate AST family"
    );
    for required in ["let filter_tokens = &tokens[filter_range]"] {
        assert!(
            parser.contains(required),
            "{relative} should parse player-control fallback filters from original token ranges: missing `{required}`"
        );
    }
    for required in [
        "parse_player_controls_no_predicate(predicate_tokens)",
        "parse_you_control_or_graveyard_predicate(predicate_tokens)",
        "parse_you_control_conjoined_predicate(predicate_tokens)",
        "parse_player_controls_predicate(\n            predicate_tokens",
        "YOU_CONTROL_PREFIX_PATTERN.matches_non_article_tokens(predicate_tokens)",
        "THAT_PLAYER_CONTROLS_PREFIX_PATTERN.matches_non_article_tokens(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route control predicate matching through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "parse_player_controls_no_predicate(&filtered)",
        "parse_you_control_or_graveyard_predicate(&filtered)",
        "parse_you_control_conjoined_predicate(&filtered)",
        "parse_player_controls_predicate(\n            &filtered",
        "YOU_CONTROL_PREFIX_PATTERN.matches_words(&filtered)",
        "THAT_PLAYER_CONTROLS_PREFIX_PATTERN.matches_words(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route player-control predicate calls through filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in [
        "let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)",
        "let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild player-control predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_own_control_and_conjoined_shapes_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_attacking_you_own_control_predicate",
        "fn player_filter_for_turn_value",
    );

    for required in [
        "fn parse_attacking_you_own_control_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_you_both_own_and_control_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_implicit_subject_and_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_while_conjoined_predicate(\n    tokens: &[OwnedLexToken]",
        "let clause = LexedClause::new(tokens)",
        "parse_attacking_you_own_control_predicate(predicate_tokens)",
        "parse_you_both_own_and_control_predicate(predicate_tokens)",
        "parse_implicit_subject_and_predicate(predicate_tokens)",
        "parse_while_conjoined_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route own/control and conjoined predicate shapes through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "parse_attacking_you_own_control_predicate(&filtered)",
        "parse_you_both_own_and_control_predicate(&filtered)",
        "parse_implicit_subject_and_predicate(&filtered)",
        "parse_while_conjoined_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route own/control and conjoined predicate calls through filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in [
        "let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered)",
        "filtered.join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild own/control and conjoined predicate tokens from filtered words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_this_way_shapes_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_this_way_object_filter_clause",
        "fn active_discard_player_subject_clause",
    );

    for required in [
        "fn parse_passive_this_way_tagged_object_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_active_this_way_discard_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_negative_put_tagged_object_predicate(tokens: &[OwnedLexToken])",
        "fn parse_active_this_way_battlefield_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_passive_this_way_battlefield_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_passive_this_way_tagged_object_predicate(predicate_tokens)",
        "parse_active_this_way_discard_predicate(predicate_tokens)",
        "parse_active_this_way_battlefield_predicate(predicate_tokens)",
        "parse_passive_this_way_battlefield_predicate(predicate_tokens)",
        "parse_negative_put_tagged_object_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route this-way predicate shapes through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "parse_passive_this_way_tagged_object_predicate(&filtered)",
        "parse_active_this_way_discard_predicate(&filtered)",
        "parse_active_this_way_battlefield_predicate(&filtered)",
        "parse_passive_this_way_battlefield_predicate(&filtered)",
        "parse_negative_put_tagged_object_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route this-way predicate calls through filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered)"]
    {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild this-way predicate tokens from filtered words: found `{forbidden}`"
        );
    }
}

#[test]
fn combat_restriction_control_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activation_costs.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn player_controls_at_least_condition_from_tail",
        "fn control_creature_with_power_condition_from_tail",
    );

    assert!(
        helper.contains("grammar::conditions::parse_control_condition")
            && helper.contains("ControlConditionOptions")
            && helper.contains("control_condition.quantity_token_count == 0")
            && helper.contains("control_condition.at_least_count()")
            && helper.contains("ConditionExpr::PlayerHasAtLeast"),
        "{relative} should parse combat-restriction control tails through the shared captured control-condition parser"
    );
    for forbidden in [
        "parse_greater_than_or_equal_count_prefix_from_words(tail.get(2..)",
        "let filter_words = tail.get(2 + used..)",
        "parse_object_filter(&filter_tokens, false)",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not rebuild control-condition count/filter tails by hand with `{forbidden}`"
        );
    }
}

#[test]
fn static_control_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_static_condition_clause",
        "fn parse_devotion_static_condition",
    );

    assert!(
        parser.contains("grammar::conditions::parse_control_condition")
            && parser.contains("allow_opponent_players: true")
            && parser.contains("bind_filter_controller_to_subject: true")
            && parser.contains("comparison: control_condition.comparison"),
        "{relative} should parse static control conditions through the shared captured control-condition parser"
    );
    for forbidden in [
        "ANTHEM_CONTROL_CONDITION_TWO_WORD_PREFIX_PATTERN",
        "ANTHEM_CONTROL_CONDITION_THREE_WORD_PREFIX_PATTERN",
        "parse_counted_object_condition_after_prefix(\n                &count_condition_tokens[..control_prefix_token_len]",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep static control-condition prefix slicing through `{forbidden}`"
        );
    }
}

#[test]
fn static_ownership_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_static_condition_clause",
        "fn parse_devotion_static_condition",
    );

    assert!(
        parser.contains("grammar::conditions::parse_ownership_condition")
            && parser.contains("OwnershipConditionOptions")
            && parser.contains("bind_filter_owner_to_subject: true")
            && parser.contains("comparison: ownership_condition.comparison"),
        "{relative} should parse static ownership conditions through the shared captured ownership-condition parser"
    );
    for forbidden in [
        "ANTHEM_OWN_CONDITION_TWO_WORD_PREFIX_PATTERN",
        "ANTHEM_OWN_CONDITION_THREE_WORD_PREFIX_PATTERN",
        "parse_counted_object_condition_after_prefix(\n                &count_condition_tokens[..own_prefix_token_len]",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep static ownership-condition prefix slicing through `{forbidden}`"
        );
    }
}

#[test]
fn subject_status_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let static_relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let static_content = read_repo_file(&root, static_relative);
    let static_parser = function_source(
        &static_content,
        "pub(crate) fn parse_static_condition_clause",
        "fn parse_devotion_static_condition",
    );
    assert!(
        static_parser.contains("grammar::conditions::parse_subject_status_condition")
            && static_parser.contains(".and_then(|condition| condition.condition_expr())"),
        "{static_relative} should parse source/equipped-creature status clauses through the shared captured subject-status parser"
    );
    for forbidden in [
        "SOURCE_IS_EQUIPPED_CONDITION_PATTERN",
        "SOURCE_IS_ENCHANTED_CONDITION_PATTERN",
        "SOURCE_IS_UNTAPPED_CONDITION_PATTERN",
        "SOURCE_IS_TAPPED_CONDITION_PATTERN",
        "SOURCE_IS_MONSTROUS_CONDITION_PATTERN",
        "SOURCE_IS_ATTACKING_CONDITION_PATTERN",
        "EQUIPPED_CREATURE_IS_TAPPED_CONDITION_PATTERN",
        "EQUIPPED_CREATURE_IS_UNTAPPED_CONDITION_PATTERN",
        "EQUIPPED_CREATURE_IS_ATTACKING_CONDITION_PATTERN",
    ] {
        assert!(
            !static_content.contains(forbidden),
            "{static_relative} should not keep exact subject-status ClauseShape `{forbidden}`"
        );
    }

    let grammar_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/abilities.rs";
    let grammar_content = read_repo_file(&root, grammar_relative);
    let tap_status_parser = function_source(
        &grammar_content,
        "pub(crate) fn parse_source_tap_status_condition_lexed",
        "pub(crate) fn is_enchanted_land_is_chosen_type_line_lexed",
    );
    assert!(
        tap_status_parser.contains("super::conditions::parse_subject_status_condition")
            && !tap_status_parser.contains("&[\"this\", \"creature\", \"is\", \"tapped\"]"),
        "{grammar_relative} should reuse the captured subject-status parser instead of exact tap-status phrase arrays"
    );
}

#[test]
fn subject_descriptor_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let static_relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let static_content = read_repo_file(&root, static_relative);
    let static_parser = function_source(
        &static_content,
        "pub(crate) fn parse_static_condition_clause",
        "fn parse_devotion_static_condition",
    );
    assert!(
        static_parser.contains("grammar::conditions::parse_subject_descriptor_condition")
            && static_parser.contains("condition.condition_expr(clause_words.join(\" \"))"),
        "{static_relative} should parse enchanted/attached subject descriptor conditions through the shared captured subject-descriptor parser"
    );
    for forbidden in [
        "ENCHANTED_PERMANENT_IS_CREATURE_CONDITION_PATTERN",
        "ENCHANTED_PERMANENT_IS_LAND_CONDITION_PATTERN",
        "ENCHANTED_PERMANENT_IS_EQUIPMENT_CONDITION_PATTERN",
        "ENCHANTED_PERMANENT_IS_VEHICLE_CONDITION_PATTERN",
        "ATTACHED_OBJECT_IS_PREFIX_PATTERN",
        "let mut descriptor_words = &clause_words[3..]",
    ] {
        assert!(
            !static_content.contains(forbidden),
            "{static_relative} should not keep exact/manual subject-descriptor condition parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_subject_descriptor_condition")
            && conditions_content.contains("SubjectDescriptorConditionAst")
            && conditions_content.contains("ObjectDescriptorAst")
            && conditions_content.contains("fn parse_object_descriptor_clause")
            && conditions_content
                .contains("token_slice_first_is_any(tokens, &[\"a\", \"an\", \"the\"])")
            && !conditions_content.contains("descriptor_clause.word_refs()")
            && !conditions_content.contains("strip_optional_article(&descriptor_word_refs)"),
        "{conditions_relative} should expose a captured subject-descriptor condition AST parser"
    );
}

#[test]
fn player_status_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let static_relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let static_content = read_repo_file(&root, static_relative);
    let static_parser = function_source(
        &static_content,
        "pub(crate) fn parse_static_condition_clause",
        "fn parse_devotion_static_condition",
    );
    assert!(
        static_parser.contains("grammar::conditions::parse_player_status_condition")
            && static_parser.contains("condition.condition_expr()"),
        "{static_relative} should parse monarch/initiative/max-speed static conditions through the shared captured player-status parser"
    );
    for forbidden in [
        "YOU_ARE_MONARCH_CONDITION_PATTERN",
        "YOU_HAVE_INITIATIVE_CONDITION_PATTERN",
        "YOU_HAVE_MAX_SPEED_CONDITION_PATTERN",
    ] {
        assert!(
            !static_content.contains(forbidden),
            "{static_relative} should not keep exact player-status condition parsing through `{forbidden}`"
        );
    }

    let attack_relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activation_costs.rs";
    let attack_content = read_repo_file(&root, attack_relative);
    let attack_parser = function_source(
        &attack_content,
        "pub(crate) fn parse_cant_clause",
        "#[cfg(test)]",
    );
    assert!(
        attack_parser.contains("grammar::conditions::parse_player_status_condition")
            && attack_parser.contains("PlayerStatusAst::Monarch")
            && !attack_content.contains("DEFENDING_PLAYER_MONARCH_PATTERN"),
        "{attack_relative} should reuse the captured player-status parser for defending-player monarch restrictions"
    );

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_status_predicate",
        "fn parse_world_state_or_timing_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_status_predicate(predicate_tokens)")
            && predicate_content.contains("grammar::conditions::parse_player_status_condition")
            && predicate_helper
                .contains("fn parse_player_status_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_status_predicate(&filtered)"),
        "{predicate_relative} should route predicate monarch/initiative/max-speed phrases through the shared player-status parser"
    );
    for forbidden in [
        "YOU_ARE_MONARCH_PATTERN",
        "YOU_HAVE_INITIATIVE_PATTERN",
        "YOU_HAVE_MAX_SPEED_PATTERN",
    ] {
        assert!(
            !predicate_content.contains(forbidden),
            "{predicate_relative} should not keep exact predicate player-status parsing through `{forbidden}`"
        );
    }

    let abilities_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/abilities.rs";
    let abilities_content = read_repo_file(&root, abilities_relative);
    let activation_condition_parser = function_source(
        &abilities_content,
        "pub(crate) fn parse_activation_condition_lexed",
        "pub(crate) fn parse_activation_count_per_turn",
    );
    assert!(
        activation_condition_parser.contains("grammar::conditions::parse_player_status_condition")
            && activation_condition_parser.contains("PlayerStatusAst::MaxSpeed"),
        "{abilities_relative} should route activate-only-if max-speed conditions through the shared player-status parser"
    );
    assert!(
        !abilities_content.contains("ACTIVATE_ONLY_IF_MAX_SPEED_PATTERN"),
        "{abilities_relative} should not keep an exact activate-only-if max-speed ClauseShape"
    );

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_player_status_condition")
            && conditions_content.contains("PlayerStatusConditionAst")
            && conditions_content.contains("PlayerStatusAst"),
        "{conditions_relative} should expose a captured player-status condition AST parser"
    );
}

#[test]
fn world_state_timing_predicates_use_token_shapes() {
    let root = workspace_root();
    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_world_state_or_timing_predicate",
        "fn parse_empty_battlefield_predicate",
    );
    assert!(
        predicate_parser.contains("parse_world_state_or_timing_predicate(predicate_tokens)")
            && predicate_helper
                .contains("fn parse_world_state_or_timing_predicate(tokens: &[OwnedLexToken])")
            && predicate_helper.contains("parse_initiative_choice_predicate_shape(tokens)")
            && predicate_helper.contains("parse_night_state_predicate_shape(tokens)")
            && predicate_helper.contains("parse_first_combat_phase_predicate_shape(tokens)")
            && predicate_helper.contains("parse_cast_this_spell_during_main_phase_shape(tokens)")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_world_state_or_timing_predicate(&filtered)"),
        "{predicate_relative} should route world-state/timing predicates through lexed token shape parsers"
    );
}

#[test]
fn empty_battlefield_predicates_use_token_shapes() {
    let root = workspace_root();
    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_empty_battlefield_predicate",
        "fn is_battlefield_zone_clause",
    );
    assert!(
        predicate_parser.contains("parse_empty_battlefield_predicate(predicate_tokens)")
            && predicate_helper
                .contains("fn parse_empty_battlefield_predicate(tokens: &[OwnedLexToken])")
            && predicate_helper.contains("let clause = LexedClause::new(tokens)")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_empty_battlefield_predicate(&filtered)"),
        "{predicate_relative} should route empty-battlefield predicates through lexed token shape parsers"
    );
}

#[test]
fn player_achievement_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let static_relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let static_content = read_repo_file(&root, static_relative);
    let static_parser = function_source(
        &static_content,
        "pub(crate) fn parse_static_condition_clause",
        "fn parse_devotion_static_condition",
    );
    assert!(
        static_parser.contains("grammar::conditions::parse_player_achievement_condition")
            && static_parser.contains("condition.condition_expr()"),
        "{static_relative} should parse city-blessing and completed-dungeon conditions through the shared captured player-achievement parser"
    );
    for forbidden in [
        "YOU_HAVE_CITYS_BLESSING_CONDITION_PATTERN",
        "YOU_COMPLETED_A_DUNGEON_CONDITION_PATTERN",
        "YOUVE_COMPLETED_DUNGEON_PREFIX_PATTERN",
        "YOU_HAVE_COMPLETED_DUNGEON_PREFIX_PATTERN",
    ] {
        assert!(
            !static_content.contains(forbidden),
            "{static_relative} should not keep exact/manual player-achievement condition parsing through `{forbidden}`"
        );
    }

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    assert!(
        predicate_parser.contains("parse_player_achievement_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_player_achievement_condition"),
        "{predicate_relative} should route predicate city-blessing/completed-dungeon phrases through the shared player-achievement parser"
    );
    for forbidden in [
        "YOU_HAVE_CITYS_BLESSING_PATTERN",
        "YOU_HAVE_CITYS_BLESSING_FOR_EACH_PREFIX_PATTERN",
        "YOU_COMPLETED_DUNGEON_PATTERN",
        "YOUVE_COMPLETED_PREFIX_PATTERN",
        "YOU_HAVE_COMPLETED_PREFIX_PATTERN",
        "YOU_HAVENT_COMPLETED_PREFIX_PATTERN",
        "YOU_HAVE_NOT_COMPLETED_PREFIX_PATTERN",
        "YOU_HAVE_FULL_PARTY_PATTERN",
        "let name_start = if HAVE_WORD_PATTERN.matches_word(filtered[1])",
        "fn parse_player_achievement_predicate(words: &[&str])",
        "parse_player_achievement_predicate(&filtered)",
    ] {
        assert!(
            !predicate_content.contains(forbidden),
            "{predicate_relative} should not keep exact/manual predicate player-achievement parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_player_achievement_condition")
            && conditions_content.contains("PlayerAchievementConditionAst")
            && conditions_content.contains("PlayerAchievementAst")
            && conditions_content.contains("FullParty")
            && conditions_content.contains("negated")
            && conditions_content.contains("const DUNGEON_PHRASES: &[&[&str]]")
            && conditions_content.contains("clause_matches_any_phrase(clause, DUNGEON_PHRASES)")
            && conditions_content
                .contains("let dungeon_name_tokens = dungeon_name_clause.trimmed().tokens()")
            && conditions_content
                .contains("render_token_slice(dungeon_name_tokens).trim().to_string()"),
        "{conditions_relative} should expose a captured player-achievement condition AST parser"
    );
    for forbidden in [
        "let words = dungeon_name_clause.word_refs()",
        "dungeon_name: Some(words.join(\" \"))",
    ] {
        assert!(
            !conditions_content.contains(forbidden),
            "{conditions_relative} should render captured dungeon names from token spans, not word joins `{forbidden}`"
        );
    }
}

#[test]
fn player_cards_in_hand_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_cards_in_hand_predicate",
        "fn parse_player_life_total_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_cards_in_hand_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_player_cards_in_hand_condition")
            && predicate_helper
                .contains("fn parse_player_cards_in_hand_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_cards_in_hand_predicate(&filtered)"),
        "{predicate_relative} should route cards-in-hand count predicates through the shared captured parser"
    );
    assert!(
        !predicate_content.contains("YOU_HAVE_NO_CARDS_IN_HAND_PATTERN"),
        "{predicate_relative} should not keep exact no-cards-in-hand predicate parsing"
    );

    let static_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let static_content = read_repo_file(&root, static_relative);
    let draw_replacement_parser = function_source(
        &static_content,
        "pub(crate) fn parse_conditional_draw_replacement_line",
        "fn keyword_action_replacement_subject_explores",
    );
    assert!(
        draw_replacement_parser
            .contains("grammar::conditions::parse_player_cards_in_hand_condition")
            && draw_replacement_parser.contains("is_no_cards_in_hand()"),
        "{static_relative} should parse conditional draw replacement hand-count conditions through the shared captured parser"
    );
    assert!(
        !static_content.contains("YOU_HAVE_NO_CARDS_IN_HAND_PATTERN"),
        "{static_relative} should not keep an exact no-cards-in-hand static ClauseShape"
    );

    let anthem_relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let anthem_content = read_repo_file(&root, anthem_relative);
    let cards_in_hand_condition_parser = function_source(
        &anthem_content,
        "fn parse_cards_in_hand_static_condition",
        "fn parse_life_total_static_condition",
    );
    assert!(
        cards_in_hand_condition_parser
            .contains("grammar::conditions::parse_player_cards_in_hand_condition")
            && cards_in_hand_condition_parser.contains("condition_expr()"),
        "{anthem_relative} should parse static cards-in-hand conditions through the shared captured parser"
    );

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_player_cards_in_hand_condition")
            && conditions_content.contains("PlayerCardsInHandConditionAst")
            && conditions_content.contains("comparison"),
        "{conditions_relative} should expose a captured cards-in-hand condition AST parser"
    );
}

#[test]
fn player_cards_in_hand_relation_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let relation_helper = function_source(
        &predicate_content,
        "fn parse_player_cards_in_hand_relation_predicate",
        "fn parse_player_turn_event_predicate",
    );
    assert!(
        predicate_parser
            .contains("parse_player_cards_in_hand_relation_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_player_cards_in_hand_relation_condition")
            && relation_helper.contains(
                "fn parse_player_cards_in_hand_relation_predicate(tokens: &[OwnedLexToken])"
            )
            && !relation_helper.contains("synthetic_word_tokens")
            && !predicate_parser
                .contains("parse_player_cards_in_hand_relation_predicate(&filtered)"),
        "{predicate_relative} should route cards-in-hand relation predicates through the shared captured parser"
    );
    for forbidden in [
        "[\"more\", \"card\", \"in\", \"hand\", \"than\", \"you\"]",
        "[\"more\", \"cards\", \"in\", \"hand\", \"than\", \"you\"]",
        "[\"more\", \"card\", \"in\", \"their\", \"hand\", \"than\", \"you\"]",
        "[\"more\", \"cards\", \"in\", \"their\", \"hand\", \"than\", \"you\"]",
        "\"more\", \"cards\", \"in\", \"hand\", \"than\", \"each\", \"other\", \"player\"",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep exact/manual cards-in-hand relation parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_player_cards_in_hand_relation_condition")
            && conditions_content.contains("PlayerCardsInHandRelationConditionAst")
            && conditions_content.contains("PlayerCardsInHandRelationAst")
            && conditions_content
                .contains("MORE_CARDS_IN_HAND_THAN_YOU_PATTERN.matches(relation_clause)")
            && conditions_content.contains(
                "MORE_CARDS_IN_HAND_THAN_EACH_OTHER_PLAYER_PATTERN.matches(relation_clause)"
            )
            && !conditions_content.contains("let relation_words = relation_clause.word_refs()")
            && !conditions_content.contains("matches_words(&relation_words)"),
        "{conditions_relative} should expose a captured cards-in-hand relation condition AST parser"
    );
}

#[test]
fn player_turn_event_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_turn_event_predicate",
        "fn parse_turn_timing_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_turn_event_predicate(predicate_tokens)")
            && predicate_content.contains("grammar::conditions::parse_player_turn_event_condition")
            && predicate_helper
                .contains("fn parse_player_turn_event_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_turn_event_predicate(&filtered)"),
        "{predicate_relative} should route turn-event count predicates through the shared captured parser"
    );
    for forbidden in [
        "DREW_WORD_PATTERN",
        "DRAWN_WORD_PATTERN",
        "LAND_OR_LANDS_WORD_PATTERN",
        "ENTER_OR_ENTERED_WORD_PATTERN",
        "BATTLEFIELD_WORD_PATTERN",
        "CONTROL_POSSESSIVE_WORD_PATTERN",
        "Value::MaxCardsDrawnThisTurn(player_filter)",
        "Value::LandsEnteredBattlefieldThisTurn(player_filter)",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual turn-event predicate parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_player_turn_event_condition")
            && conditions_content.contains("PlayerTurnEventConditionAst")
            && conditions_content.contains("PlayerTurnEventAst"),
        "{conditions_relative} should expose a captured turn-event condition AST parser"
    );
}

#[test]
fn spell_context_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_spell_context_predicate",
        "fn parse_player_spell_cast_this_turn_predicate",
    );
    assert!(
        predicate_parser.contains("parse_spell_context_predicate(predicate_tokens)")
            && predicate_content.contains("grammar::conditions::parse_spell_context_condition")
            && predicate_helper
                .contains("fn parse_spell_context_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_spell_context_predicate(&filtered)"),
        "{predicate_relative} should route target-spell context predicates through the shared captured parser"
    );
    for forbidden in [
        "TARGET_SPELL_CONTROLLER_POISONED_PATTERN",
        "TARGET_SPELL_NO_MANA_SPENT_TO_CAST_PATTERN",
        "YOU_CONTROL_MORE_CREATURES_THAN_TARGET_SPELL_CONTROLLER_PATTERN",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep exact target-spell context predicate parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_spell_context_condition")
            && conditions_content.contains("SpellContextConditionAst")
            && conditions_content.contains("SpellContextReferenceAst"),
        "{conditions_relative} should expose a captured target-spell context condition AST parser"
    );
}

#[test]
fn player_spell_cast_this_turn_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_spell_cast_this_turn_predicate",
        "fn parse_player_life_change_this_turn_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_spell_cast_this_turn_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_player_spell_cast_this_turn_condition")
            && predicate_helper.contains(
                "fn parse_player_spell_cast_this_turn_predicate(tokens: &[OwnedLexToken])"
            )
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_spell_cast_this_turn_predicate(&filtered)"),
        "{predicate_relative} should route player spell-cast-this-turn predicates through the shared captured parser"
    );
    for forbidden in [
        "YOU_CAST_ANOTHER_SPELL_THIS_TURN_PATTERN",
        "OPPONENT_HAS_CAST_PREFIX_PATTERN",
        "OPPONENTS_HAVE_CAST_PREFIX_PATTERN",
        "YOUVE_CAST_PREFIX_PATTERN",
        "YOU_HAVE_CAST_PREFIX_PATTERN",
        "YOU_CAST_PREFIX_PATTERN",
        "THAT_PLAYER_DIDNT_CAST_PREFIX_PATTERN",
        "THAT_PLAYER_DID_NOT_CAST_PREFIX_PATTERN",
        "YOU_DIDNT_CAST_PREFIX_PATTERN",
        "YOU_DID_NOT_CAST_PREFIX_PATTERN",
        "spell_cast_matching_predicate(",
        "parse_both_spell_cast_predicate(",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual spell-cast-this-turn parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_player_spell_cast_this_turn_condition")
            && conditions_content.contains("PlayerSpellCastThisTurnConditionAst")
            && conditions_content.contains("MatchingFilters")
            && conditions_content.contains("CountAtLeast"),
        "{conditions_relative} should expose a captured spell-cast-this-turn condition AST parser"
    );
}

#[test]
fn player_life_change_this_turn_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_life_change_this_turn_predicate",
        "fn parse_object_death_this_turn_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_life_change_this_turn_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_player_life_change_this_turn_condition")
            && predicate_helper.contains(
                "fn parse_player_life_change_this_turn_predicate(tokens: &[OwnedLexToken])"
            )
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser
                .contains("parse_player_life_change_this_turn_predicate(&filtered)"),
        "{predicate_relative} should route player life-change-this-turn predicates through the shared captured parser"
    );
    for forbidden in [
        "OPPONENT_LOST_LIFE_THIS_TURN_PATTERN",
        "YOU_GAINED_PREFIX_PATTERN",
        "YOU_LOST_PREFIX_PATTERN",
        "LIFE_THIS_TURN_TAIL_PATTERN",
        "YOU_GAINED_LIFE_THIS_TURN_PATTERN",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual life-change-this-turn parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_player_life_change_this_turn_condition")
            && conditions_content.contains("PlayerLifeChangeThisTurnConditionAst")
            && conditions_content.contains("PlayerLifeChangeDirectionAst"),
        "{conditions_relative} should expose a captured life-change-this-turn condition AST parser"
    );
}

#[test]
fn player_would_action_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_would_action_predicate",
        "fn parse_battlefield_entry_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_would_action_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_player_would_action_condition")
            && predicate_helper
                .contains("fn parse_player_would_action_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_would_action_predicate(&filtered)"),
        "{predicate_relative} should route player-would-action predicates through the shared captured parser"
    );
    for forbidden in [
        "PLAYER_WOULD_DRAW_CARD_PATTERN",
        "PLAYER_WOULD_PROLIFERATE_PATTERN",
        "OPPONENT_WOULD_BEGIN_EXTRA_TURN_PATTERN",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual player-would-action parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_player_would_action_condition")
            && conditions_content.contains("PlayerWouldActionConditionAst")
            && conditions_content.contains("PlayerWouldActionAst"),
        "{conditions_relative} should expose a captured player-would-action condition AST parser"
    );
}

#[test]
fn battlefield_change_this_turn_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_battlefield_change_this_turn_predicate",
        "fn parse_combat_damage_this_turn_predicate",
    );
    assert!(
        predicate_parser.contains("parse_battlefield_change_this_turn_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_battlefield_change_this_turn_condition")
            && predicate_helper.contains(
                "fn parse_battlefield_change_this_turn_predicate(tokens: &[OwnedLexToken])"
            )
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser
                .contains("parse_battlefield_change_this_turn_predicate(&filtered)"),
        "{predicate_relative} should route battlefield-change-this-turn predicates through the shared captured parser"
    );
    for forbidden in [
        "NO_PERMANENT_LEFT_BATTLEFIELD_THIS_TURN_PATTERN",
        "PERMANENT_LEFT_BATTLEFIELD_THIS_TURN_PATTERN",
        "LAND_YOU_CONTROLLED_PUT_INTO_GRAVEYARD_THIS_TURN_PATTERN",
        "PERMANENT_LEFT_UNDER_YOUR_CONTROL_THIS_TURN_PATTERN",
        "NONLAND_PERMANENT_LEFT_OR_SPELL_WARPED_PATTERN",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual battlefield-change parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_battlefield_change_this_turn_condition")
            && conditions_content.contains("BattlefieldChangeThisTurnConditionAst"),
        "{conditions_relative} should expose a captured battlefield-change-this-turn condition AST parser"
    );
}

#[test]
fn object_death_this_turn_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_object_death_this_turn_predicate",
        "fn parse_player_would_action_predicate",
    );
    assert!(
        predicate_parser.contains("parse_object_death_this_turn_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_object_death_this_turn_condition")
            && predicate_helper
                .contains("fn parse_object_death_this_turn_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_object_death_this_turn_predicate(&filtered)"),
        "{predicate_relative} should route object-death-this-turn predicates through the shared captured parser"
    );
    for forbidden in [
        "CREATURE_DIED_COUNT_TAIL_PATTERN",
        "CREATURE_CARD_PUT_INTO_YOUR_GRAVEYARD_THIS_TURN_PATTERN",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual object-death parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_object_death_this_turn_condition")
            && conditions_content.contains("ObjectDeathThisTurnConditionAst")
            && conditions_content.contains("ObjectDeathThisTurnEventAst"),
        "{conditions_relative} should expose a captured object-death-this-turn condition AST parser"
    );
}

#[test]
fn combat_damage_this_turn_predicates_use_token_shapes() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_combat_damage_this_turn_predicate",
        "fn is_player_object_clause",
    );
    assert!(
        predicate_parser.contains("parse_combat_damage_this_turn_predicate(predicate_tokens)")
            && predicate_helper
                .contains("fn parse_combat_damage_this_turn_predicate(tokens: &[OwnedLexToken])")
            && predicate_helper
                .contains("parse_source_dealt_combat_damage_this_turn_shape(tokens)")
            && predicate_helper
                .contains("parse_player_dealt_combat_damage_by_subtype_this_turn_shape(tokens)")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_combat_damage_this_turn_predicate(&filtered)"),
        "{predicate_relative} should route combat-damage-this-turn predicates through lexed token shape parsers"
    );
}

#[test]
fn battlefield_entry_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_battlefield_entry_predicate",
        "fn parse_battlefield_change_this_turn_predicate",
    );
    assert!(
        predicate_parser.contains("parse_battlefield_entry_predicate(predicate_tokens)")
            && predicate_content.contains("grammar::conditions::parse_battlefield_entry_condition")
            && predicate_helper
                .contains("fn parse_battlefield_entry_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_battlefield_entry_predicate(&filtered)"),
        "{predicate_relative} should route battlefield-entry predicates through the shared captured parser"
    );
    for forbidden in [
        "ObjectEnteredBattlefieldLastTurn(\n            ObjectFilter::creature()",
        "ObjectEnteredBattlefieldThisTurn(\n            ObjectFilter::artifact()",
        "PlayerHadLandEnterBattlefieldThisTurn {\n            player: PlayerAst::You",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep inline battlefield-entry one-off parsing: {forbidden}"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_battlefield_entry_condition")
            && conditions_content.contains("BattlefieldEntryConditionAst")
            && conditions_content.contains("BattlefieldEntryTurnWindowAst")
            && conditions_content
                .contains("token_slice_first_is_any(object_clause.trimmed().tokens(), &[\"another\", \"other\"])")
            && !conditions_content.contains("object_clause.word_refs().first()"),
        "{conditions_relative} should expose a captured battlefield-entry condition AST parser"
    );
}

#[test]
fn player_life_total_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_life_total_predicate",
        "fn parse_player_life_relation_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_life_total_predicate(predicate_tokens)")
            && predicate_content.contains("grammar::conditions::parse_player_life_total_condition")
            && predicate_helper
                .contains("fn parse_player_life_total_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_life_total_predicate(&filtered)"),
        "{predicate_relative} should route player life-total numeric predicates through the shared captured parser"
    );
    assert!(
        !predicate_content.contains("LIFE_TAIL_PATTERN"),
        "{predicate_relative} should not keep an exact life-tail predicate ClauseShape for numeric life totals"
    );

    let anthem_relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let anthem_content = read_repo_file(&root, anthem_relative);
    let life_total_condition_parser = function_source(
        &anthem_content,
        "fn parse_life_total_static_condition",
        "pub(crate) fn parse_anthem_for_each_expression",
    );
    assert!(
        life_total_condition_parser
            .contains("grammar::conditions::parse_player_life_total_condition")
            && life_total_condition_parser.contains("condition_expr()"),
        "{anthem_relative} should parse static life-total numeric conditions through the shared captured parser"
    );
    assert!(
        !anthem_content.contains("ANTHEM_LIFE_TAIL_PATTERN"),
        "{anthem_relative} should not keep an exact life-tail static ClauseShape"
    );

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_player_life_total_condition")
            && conditions_content.contains("PlayerLifeTotalConditionAst")
            && conditions_content.contains("Value::LifeTotal"),
        "{conditions_relative} should expose a captured life-total condition AST parser"
    );
}

#[test]
fn player_life_relation_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let relation_helper = function_source(
        &predicate_content,
        "fn parse_player_life_relation_predicate",
        "fn parse_player_cards_in_hand_relation_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_life_relation_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_player_life_relation_condition")
            && relation_helper
                .contains("fn parse_player_life_relation_predicate(tokens: &[OwnedLexToken])")
            && !relation_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_life_relation_predicate(&filtered)"),
        "{predicate_relative} should route player life-relation predicates through the shared captured parser"
    );
    for forbidden in [
        "MORE_LIFE_THAN_YOU_TAIL_PATTERN",
        "YOU_HAVE_MORE_LIFE_THAN_PREFIX_PATTERN",
        "NO_OPPONENT_HAS_MORE_LIFE_THAN_PREFIX_PATTERN",
        "MORE_LIFE_THAN_EACH_OTHER_PLAYER_TAIL_PATTERN",
        "EACH_OPPONENT_WORDS_TAIL_PATTERN",
    ] {
        assert!(
            !predicate_content.contains(forbidden),
            "{predicate_relative} should not keep exact/manual life-relation parsing through `{forbidden}`"
        );
    }

    let conditions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub(crate) fn parse_player_life_relation_condition")
            && conditions_content.contains("PlayerLifeRelationConditionAst")
            && conditions_content.contains("PlayerLifeRelationAst")
            && conditions_content.contains("MORE_LIFE_THAN_PLAYER_PATTERN")
            && conditions_content.contains("MORE_LIFE_THAN_YOU_PATTERN.matches(relation_clause)")
            && conditions_content
                .contains("MORE_LIFE_THAN_EACH_OTHER_PLAYER_PATTERN.matches(relation_clause)")
            && conditions_content
                .contains("MORE_LIFE_THAN_EACH_OPPONENT_PATTERN.matches(relation_clause)")
            && conditions_content
                .contains("MORE_LIFE_THAN_PLAYER_PATTERN.match_clause(relation_clause)")
            && conditions_content.contains(
                "matched.capture_clause_by_role(LexCaptureRole::Subject, relation_clause)"
            )
            && !conditions_content.contains("MORE_LIFE_THAN_PREFIX_PATTERN")
            && !conditions_content.contains("parse_life_relation_player_subject_words")
            && !conditions_content.contains("let relation_words = relation_clause.word_refs()")
            && !conditions_content.contains("matched_prefix_len(&relation_words)")
            && !conditions_content.contains("matches_words(&relation_words)"),
        "{conditions_relative} should expose a captured life-relation condition AST parser"
    );
}

#[test]
fn activated_lowering_zone_and_x_checks_use_parse_tokens() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/activated_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn lower_rewrite_activated_to_chunk_impl",
        "fn apply_chosen_option_condition_to_activated",
    );

    assert!(
        helper.contains("original_effect_parse_tokens")
            && helper
                .contains("contains_token_word_sequence(&effect_parse_tokens, ADD_X_MANA_PHRASE)")
            && helper.contains("level_number_from_tokens(&effect_parse_tokens)")
            && helper
                .contains("tokens_mention_phrase(original_effect_parse_tokens, WHERE_X_IS_PHRASE)"),
        "{relative} should check activated X and class level clauses from effect parse tokens"
    );
    let level_helper = function_source(
        &content,
        "fn level_number_from_tokens",
        "fn apply_chosen_option_condition_to_activated",
    );
    assert!(
        level_helper.contains("token_slice_starts_with(tokens, &[\"level\"])")
            && level_helper.contains("tokens.get(1)?.parser_text.parse::<u32>()"),
        "{relative} should parse activated class level numbers from lexed tokens"
    );
    assert!(
        content.contains("fn infer_rewrite_activated_functional_zones")
            && content.contains(
                "tokens_mention_any_player_activate_on_stack(original_effect_parse_tokens)"
            )
            && content.contains(
                "tokens_mention_any_phrase(cost_tokens, EXILE_SELF_FROM_GRAVEYARD_PHRASES)"
            ),
        "{relative} should infer activated functional zones from effect/cost parse tokens"
    );
    for forbidden in [
        "text_mentions_where_x_is(line.info.raw_line.as_str())",
        "text_mentions_any_player_activate_on_stack(line.info.raw_line.as_str())",
        "text_mentions_exile_self_from_graveyard(line.info.raw_line.as_str())",
        "text_mentions_add_x_mana(effect_text.as_str())",
        "level_number_from_text(effect_text.as_str())",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not infer activated X bindings or zones through raw-line search `{forbidden}`"
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
fn cycling_keyword_family_uses_shared_word_helper() {
    let root = workspace_root();
    let checks = [
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_families.rs",
        "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/keyword_activated_lines.rs",
        "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activated_line_core.rs",
    ];

    for relative in checks {
        let content = read_repo_file(&root, relative);
        assert!(
            !content.contains("str_strip_suffix(") || !content.contains("cycling"),
            "{relative} should classify cycling keyword variants through shared word helpers, not raw suffix stripping"
        );
    }
}

#[test]
fn keyword_static_marker_support_uses_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let marker_support = function_source(
        &content,
        "fn supported_keyword_marker_tokens",
        "fn trim_outer_quotes",
    );

    for forbidden in [
        "TOUGHNESS_CREWS_VEHICLES_MARKER_TEXTS",
        "POWER_GREATER_MARKER_PREFIXES",
        "POWER_GREATER_MARKER_SUFFIX",
        "LOYALTY_COUNTER_CREW_COST_PREFIX",
        "LOYALTY_COUNTER_CREW_COST_SUFFIX",
        ".starts_with(prefix)",
        ".ends_with(POWER_GREATER_MARKER_SUFFIX)",
        ".starts_with(LOYALTY_COUNTER_CREW_COST_PREFIX)",
        ".ends_with(LOYALTY_COUNTER_CREW_COST_SUFFIX)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should classify supported keyword-static crew markers through token shapes, not raw fragment `{forbidden}`"
        );
    }

    for expected in [
        "TOUGHNESS_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)",
        "POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)",
        "LOYALTY_COUNTER_INSTEAD_OF_CREW_COST_MARKER_PATTERN.matches_words(&words)",
    ] {
        assert!(
            marker_support.contains(expected),
            "{relative} should keep supported keyword-static marker routing on ClauseShape `{expected}`"
        );
    }
}

#[test]
fn keyword_static_early_parser_uses_parser_token_words() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_static_ability_ast_line_early_lexed",
        "pub(crate) fn parse_damage_doubling_mana_value_marker_line",
    );

    assert!(
        parser.contains("parser_token_word_refs(tokens)"),
        "{relative} should feed early static patterns from parser token words"
    );
    for forbidden in [
        "rendered_storage",
        "token_word_refs(tokens)\n        .join(\" \")",
        ".replace(\"can t\", \"cant\")",
        ".replace(\"you ve\", \"youve\")",
        ".split_whitespace()",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild early static pattern words through rendered text `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_pt_modifier_parsers_use_char_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_pt_modifier",
        "pub(crate) fn parse_no_maximum_hand_size_line",
    );

    for forbidden in [
        "raw.split('/')",
        "trim_start_matches('+')",
        "str_strip_prefix(trimmed, \"+\")",
        "str_strip_prefix(trimmed, \"-\")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should parse keyword-static P/T modifiers through char helpers, not raw fragment `{forbidden}`"
        );
    }

    for expected in [
        "split_pt_modifier_components(raw)",
        "strip_leading_plus_char(power_raw)",
        "split_signed_pt_component(trimmed)",
    ] {
        assert!(
            parser.contains(expected),
            "{relative} should keep keyword-static P/T modifier parsing on helper `{expected}`"
        );
    }
}

#[test]
fn anthem_attached_object_grants_use_subject_tags_not_rendered_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn grant_object_ability_for_anthem_subject",
        "fn parse_granted_object_ability_segment",
    );

    assert!(
        helper.contains("attached_object_anthem_subject_filter(&clause.subject)")
            && helper.contains("TaggedOpbjectRelation::IsTaggedObject"),
        "{relative} should classify attached object ability grants from typed subject tags"
    );
    for forbidden in [
        "subject\n            .split_whitespace()",
        ".next()\n            .is_some_and(|word|",
        "ANTHEM_ENCHANTED_OR_EQUIPPED_WORD_PATTERN.matches_word(word)",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not classify attached object grants by inspecting rendered subject text `{forbidden}`"
        );
    }
}

#[test]
fn anthem_landwalk_override_uses_keyword_action_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn is_landwalk_ability_word",
        "pub(crate) fn parse_subject_cant_be_blocked_as_long_as_condition_line",
    );

    assert!(
        helper.contains("parse_single_word_keyword_action(word)")
            && helper.contains("KeywordAction::Landwalk"),
        "{relative} should classify landwalk override tails through the keyword action parser"
    );
    for forbidden in ["LANDWALK_ABILITY_SUFFIX", ".ends_with("] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not classify landwalk override tails by raw suffix `{forbidden}`"
        );
    }
}

#[test]
fn loyalty_shorthand_activation_cost_uses_char_sign_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activated_line_core.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_loyalty_shorthand_activation_cost",
        "pub(crate) fn loyalty_additional_restrictions",
    );

    for forbidden in [
        "raw_line: Option<&str>",
        "line.trim().splitn(2, ':')",
        "str_strip_prefix(word, \"+\")",
        "str_strip_prefix(word, \"-\")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should parse loyalty shorthand signs through chars, not raw prefix `{forbidden}`"
        );
    }

    assert!(
        parser.contains("parse_loyalty_shorthand_word(token.as_word()?)")
            && parser.contains("sign.kind == TokenKind::Plus")
            && parser.contains("sign.kind == TokenKind::Dash"),
        "{relative} should route loyalty shorthand costs through token words and sign tokens"
    );
}

#[test]
fn exhaust_once_restriction_uses_clause_shape() {
    let root = workspace_root();
    let core_relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activated_line_core.rs";
    let core = read_repo_file(&root, core_relative);
    let scanner_relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activated_sentence_parsers.rs";
    let scanner = read_repo_file(&root, scanner_relative);
    let helper = function_source(
        &scanner,
        "pub(super) fn tokens_are_exhaust_once_restriction",
        "pub(crate) fn parse_activate_only_timing_lexed",
    );
    let parser = function_source(
        &core,
        "pub(crate) fn parse_activated_line_with_raw",
        "pub(crate) fn activation_cost_mentions_x",
    );

    assert!(
        helper.contains("EXHAUST_ONCE_RESTRICTION_PATTERN.matches_words(&words)"),
        "{scanner_relative} should detect exhaust-once restrictions through ClauseShape words"
    );
    assert!(
        scanner.contains("has_exhaust_once_restriction")
            && scanner.contains(
                "has_exhaust_once_restriction |= tokens_are_exhaust_once_restriction(sentence)"
            ),
        "{scanner_relative} should expose exhaust-once detection as a token-derived scan fact"
    );
    assert!(
        parser.contains("!scanned_modifiers.has_exhaust_once_restriction"),
        "{core_relative} should consume the token-derived exhaust-once scan fact"
    );
    for forbidden in [
        ".contains(\"activate each exhaust ability only once\")",
        "fn restriction_is_exhaust_once",
        "lex_line(restriction",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{core_relative} should not detect exhaust-once restrictions through raw text `{forbidden}`"
        );
    }
}

#[test]
fn cst_lowering_loyalty_detection_uses_cst_flag() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/cst_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn activation_cost_cst_is_loyalty",
        "pub(crate) fn lower_non_metadata_rewrite_line_cst",
    );

    for forbidden in [
        "raw_activation_cost_is_loyalty_shorthand",
        "cost.raw.as_str()",
        "raw.trim()",
        ".replace('−', \"-\")",
        ".starts_with('+')",
        ".starts_with('-')",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should classify loyalty shorthand costs through sign chars, not raw fragment `{forbidden}`"
        );
    }

    assert!(
        helper.contains("cost.is_loyalty_shorthand"),
        "{relative} should consume the token-parser loyalty shorthand flag instead of reclassifying raw cost text"
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
fn leaf_counter_cost_descriptors_use_token_slices() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/leaf.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn parse_counter_type_descriptor_tokens",
        "fn activation_cost_prefix_tokens",
    );
    let put_counter = function_source(
        &content,
        "fn parse_put_counter_segment_tokens",
        "fn parse_remove_counter_segment_tokens",
    );
    let remove_counter = function_source(
        &content,
        "fn parse_remove_counter_segment_tokens",
        "fn parse_reveal_segment_tokens",
    );

    assert!(
        helper.contains("parse_counter_type_from_tokens(tokens)"),
        "{relative} should parse counter descriptors directly from token slices"
    );
    assert!(
        remove_counter.contains("parse_optional_counter_type_descriptor_tokens"),
        "{relative} should parse optional remove-counter descriptors from token slices"
    );
    for forbidden in [
        "fn parse_counter_type_descriptor(raw: &str)",
        "fn parse_optional_counter_type_descriptor(raw: &str)",
        ".split_whitespace()",
        "synthetic_word_tokens",
        "counter_descriptor.as_str()",
    ] {
        let combined = format!("{helper}\n{put_counter}\n{remove_counter}");
        assert!(
            !combined.contains(forbidden),
            "{relative} should not parse counter cost descriptors through raw rendered text `{forbidden}`"
        );
    }
}

#[test]
fn preprocess_line_prefix_matching_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn split_parse_line_variants",
        "fn parse_metadata_line",
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
fn self_reference_short_name_uses_parser_token_words() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn short_name_for_self_reference",
        "fn normalize_non_metadata_line",
    );

    assert!(
        helper.contains("lex_line(trimmed, 0)"),
        "{relative} should lex card names before deriving self-reference aliases"
    );
    assert!(
        helper.contains("parser_token_word_refs(&tokens)"),
        "{relative} should derive self-reference alias word counts from parser token words"
    );
    for forbidden in [
        "trimmed.split_whitespace()",
        "let mut words = trimmed.split_whitespace()",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not derive self-reference aliases through raw text `{forbidden}`"
        );
    }
}

#[test]
fn enchantment_parenthetical_preprocess_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn strip_parenthetical_segments",
        "fn split_parse_line_variants",
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
    assert!(
        !helper.contains("split_whitespace()"),
        "{relative} should normalize stripped parenthetical text without raw word splitting"
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
fn backup_postpass_placeholder_detection_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/postpasses/mod.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn parse_backup_placeholder_amount",
        "fn backup_granted_abilities_from_slice",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "backup postpass should parse placeholder amount from lexed tokens and role captures, not rendered whitespace splits"
    );
    for required in [
        "BACKUP_PLACEHOLDER_PATTERN.match_prefix(clause)",
        "LexPattern::amount(\"amount\", LexCaptureKind::WordCount(1))",
        "capture_clause_by_role(LexCaptureRole::Amount",
        "amount_clause.word_refs().first()?.parse::<u32>()",
    ] {
        assert!(
            content.contains(required),
            "{relative} should preserve backup postpass placeholder parsing through LexPattern captures: missing `{required}`"
        );
    }
    for forbidden in [
        "text.split_whitespace()",
        "let mut parts = text.split_whitespace()",
        "eq_ignore_ascii_case(\"backup\")",
        "trim_end_matches(',')",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not parse backup placeholder text with raw branch `{forbidden}`"
        );
    }
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
fn target_controlled_pump_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "struct TargetControlledPumpProgram",
        "pub(crate) fn parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "target-controlled pump parsing should use LexPattern role captures, not fixed phrase indexing"
    );
    for required in [
        "TARGET_CONTROLLED_PUMP_PATTERN",
        "LexPattern::subject(",
        "LexPattern::condition(",
        "\"controller\"",
        "LexCaptureKind::OneOfPhrase(TARGET_CONTROLLED_PUMP_CONTROLLER_PHRASES)",
        "LexPattern::amount(\"modifier\", LexCaptureKind::WordCount(1))",
        "TARGET_CONTROLLED_PUMP_PLAYER_CONTROLLER_PATTERN",
        "TARGET_CONTROLLED_PUMP_OPPONENT_CONTROLLER_PATTERN",
        "capture_clause_by_role(LexCaptureRole::Subject",
        "capture_clause_by_role(LexCaptureRole::Condition",
        "target_controlled_pump_controller(controller_clause.trimmed())",
        "TARGET_CONTROLLED_PUMP_OPPONENT_CONTROLLER_PATTERN.matches_clause(controller_clause)",
        "TARGET_CONTROLLED_PUMP_PLAYER_CONTROLLER_PATTERN.matches_clause(controller_clause)",
        "TARGET_CONTROLLED_PUMP_GRANTED_ABILITY_PATTERN",
        "TARGET_CONTROLLED_PUMP_GRANTED_ABILITY_PATTERN.match_clause(tail_clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, tail_clause)",
        "parse_pt_modifier_capture(modifier_clause)",
        "fn parse_pt_modifier_capture",
        "keyword_abilities_from_clause(ability_clause.trimmed())",
        "ABILITY_FIRST_STRIKE_PATTERN",
        ".find_in_clause(ability_clause)",
        "ABILITY_HASTE_PATTERN.find_in_clause(ability_clause)",
        "ABILITY_TRAMPLE_PATTERN",
        "TargetControlledPumpProgram",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve target-controlled pump parsing through captured roles: missing `{required}`"
        );
    }
    for forbidden in [
        "TARGET_PLAYER_CONTROLS_PATTERN",
        "find_generic_phrase_start(&words, TARGET_PLAYER_CONTROLS_PATTERN)",
        "words[target_idx + 3..]",
        "target_idx + 3 + offset",
        "synthetic_word_tokens(&words[..target_idx])",
        "AND_GAIN_HAVE_TAIL_PATTERN",
        "tail[2..]",
        "target_controlled_pump_controller(controller_clause.word_refs().as_slice())",
        "modifier_clause.word_refs().first()",
        "OPPONENT_OR_OPPONENTS_WORD_PATTERN",
        "LexCaptureKind::WordCount(3)",
        "ABILITY_HASTE_WORD_PATTERN",
        "ABILITY_TRAMPLE_WORD_PATTERN",
        "ABILITY_FIRST_STRIKE_PATTERN.matches_words",
        "ability_clause.word_refs()",
        "matches_words(&ability_tail)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse target-controlled pump clauses by exact phrase indexing `{forbidden}`"
        );
    }
}

#[test]
fn source_gets_unblockable_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const SOURCE_GETS_SUBJECT_PHRASES",
        "const TARGET_CONTROLLED_PUMP_CONTROLLER_PHRASES",
    );
    let parser = function_source(
        &content,
        "fn parse_source_gets_unblockable_subject_verb",
        "fn parse_source_gets_filter_gains_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "source pump plus unblockable parsing should capture subject/modifier/tail spans instead of slicing after get/gets"
    );
    for required in [
        "SOURCE_GETS_UNBLOCKABLE_PATTERN",
        "LexPattern::subject(",
        "LexPattern::modifier(\"modifier\", LexCaptureKind::WordCount(1))",
        "LexPattern::tail(\"tail\", LexCaptureKind::Rest)",
        "SOURCE_GETS_UNBLOCKABLE_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "capture_clause_by_role(LexCaptureRole::Modifier, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "SOURCE_GETS_SUBJECT_PHRASES",
        "LexCaptureKind::OneOfPhrase(SOURCE_GETS_SUBJECT_PHRASES)",
        "SOURCE_GETS_UNBLOCKABLE_TAIL_PHRASES",
        "LexCaptureKind::OneOfPhrase(SOURCE_GETS_UNBLOCKABLE_TAIL_PHRASES)",
        "SOURCE_GETS_SUBJECT_PATTERN.matches_clause(subject_clause.trimmed())",
        "parse_pt_modifier_capture(modifier_clause)",
        "UNTIL_END_OF_TURN_CANT_BE_BLOCKED_TAIL_PATTERN.matches_clause(tail_clause.trimmed())",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve source pump/unblockable parsing through captured roles: missing `{required}`"
        );
    }
    for forbidden in [
        "let Some(get_idx)",
        "GET_OR_GETS_WORD_PATTERN",
        "tokens[get_idx + 1..]",
        "collapse_leading_signed_pt_modifier_tokens",
        "modifier_words[1..]",
        "modifier_clause.word_refs().first()",
        "SOURCE_GETS_SUBJECT_PATTERN.matches_words",
        "UNTIL_END_OF_TURN_CANT_BE_BLOCKED_TAIL_PATTERN.matches_words",
        "subject_clause.word_refs()",
        "tail_clause.word_refs()",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse source pump/unblockable clauses with fixed indexes `{forbidden}`"
        );
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep retired source pump/unblockable helper `{forbidden}`"
        );
    }
}

#[test]
fn source_gets_filter_gains_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const SOURCE_GETS_SUBJECT_PHRASES",
        "const TARGET_GAINS_THEN_GETS_PUMP_PHRASES",
    );
    let parser = function_source(
        &content,
        "fn parse_source_gets_filter_gains_subject_verb",
        "fn parse_target_gains_then_gets_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "source pump plus filter-grant parsing should capture subject/modifier/filter/ability spans instead of walking get/and/gain indexes"
    );
    for required in [
        "SOURCE_GETS_FILTER_GAINS_PATTERN",
        "LexPattern::subject(",
        "LexPattern::modifier(\"modifier\", LexCaptureKind::WordCount(1))",
        "LexPattern::object(",
        "LexPattern::tail(\"ability\", LexCaptureKind::Rest)",
        "capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "capture_clause_by_role(LexCaptureRole::Modifier, clause)",
        "capture_clause_by_role(LexCaptureRole::Object, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "SOURCE_GETS_SUBJECT_PATTERN.matches_clause(subject_clause.trimmed())",
        "parse_pt_modifier_capture(modifier_clause)",
        "keyword_abilities_from_clause(ability_clause.trimmed())",
        "parse_object_filter(granted_filter_clause.trimmed().tokens(), false)",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve source pump/filter-grant parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "let Some(get_idx)",
        "words.get(get_idx + 1)",
        "get_idx + 2",
        "let Some(and_idx)",
        "let Some(gain_idx)",
        "words[and_idx + 1..gain_idx]",
        "words[gain_idx + 1..]",
        "synthetic_word_tokens",
        "modifier_clause.word_refs().first()",
        "SOURCE_GETS_SUBJECT_PATTERN.matches_words",
        "subject_clause.trimmed().word_refs()",
        "ability_clause.trimmed().word_refs()",
        "ABILITY_HASTE_WORD_PATTERN",
        "ABILITY_TRAMPLE_WORD_PATTERN",
        "ABILITY_FIRST_STRIKE_PATTERN.matches_words",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse source pump/filter-grant clauses with fixed indexes `{forbidden}`"
        );
    }
}

#[test]
fn target_gains_then_gets_gate_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const TARGET_GAINS_THEN_GETS_PUMP_PHRASES",
        "const TARGET_GETS_THEN_GAINS_GRANT_PHRASES",
    );
    let parser = function_source(
        &content,
        "fn parse_target_gains_then_gets_subject_verb",
        "fn parse_target_gets_then_gains_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "target gain-then-pump routing should capture subject/ability/pump spans instead of walking gain/get indexes"
    );
    for required in [
        "TARGET_GAINS_THEN_GETS_PATTERN",
        "LexPattern::subject(",
        "LexPattern::capture(",
        "\"ability_clause\"",
        "LexPattern::any_phrase(TARGET_GAINS_THEN_GETS_PUMP_PHRASES)",
        "LexPattern::tail(\"pump_tail\", LexCaptureKind::Rest)",
        "TARGET_GAINS_THEN_GETS_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "capture_clause(\"ability_clause\", clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve target gain-then-pump routing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "let Some(gain_idx)",
        "GAIN_OR_GAINS_WORD_PATTERN",
        "AND_GET_OR_GETS_PATTERN",
        "words[gain_idx + 1..]",
        "has_get_tail",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse target gain-then-pump clauses with fixed indexes `{forbidden}`"
        );
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep retired target gain-then-pump helper `{forbidden}`"
        );
    }
}

#[test]
fn target_gets_then_gains_gate_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const TARGET_GETS_THEN_GAINS_GRANT_PHRASES",
        "const TARGET_CONTROLLED_PUMP_GRANTED_ABILITY_PATTERN",
    );
    let parser = function_source(
        &content,
        "fn parse_target_gets_then_gains_subject_verb",
        "fn parse_target_player_controls_get_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "target pump-then-gain routing should capture subject/pump/ability spans instead of walking get indexes"
    );
    for required in [
        "TARGET_GETS_THEN_GAINS_PATTERN",
        "LexPattern::subject(",
        "LexPattern::capture(",
        "\"pump_clause\"",
        "LexPattern::any_phrase(TARGET_GETS_THEN_GAINS_GRANT_PHRASES)",
        "LexPattern::tail(\"ability_tail\", LexCaptureKind::Rest)",
        "TARGET_GETS_THEN_GAINS_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "capture_clause(\"pump_clause\", clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve target pump-then-gain routing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "let Some(get_idx)",
        "AND_GAIN_OR_GAINS_PATTERN",
        "words[get_idx + 1..]",
        "has_gain_tail",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse target pump-then-gain clauses with fixed indexes `{forbidden}`"
        );
    }
    for forbidden in ["AND_GAIN_OR_GAINS_PATTERN"] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep retired target pump-then-gain helper `{forbidden}`"
        );
    }
}

#[test]
fn damage_replacement_counter_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_generic_damage_replacement_counters_subject_verb",
        "fn tokens_contain_relative_lesser_mana_value",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "damage-replacement counter parsing should capture the protected target from a grammar pattern, not fixed word offsets"
    );
    for required in [
        "LexPattern::object(",
        "LexCaptureKind::UntilPhrase(DAMAGE_REPLACEMENT_COUNTER_DURATION_PHRASE)",
        "LexPattern::phrase(DAMAGE_REPLACEMENT_COUNTER_PREVENT_PUT_PHRASE)",
        "LexPattern::any_word(&[\"counter\", \"counters\"])",
        "LexPattern::any_phrase(DAMAGE_REPLACEMENT_COUNTER_RECIPIENT_PHRASES)",
        "capture_clause_by_role(LexCaptureRole::Object",
        "parse_target_phrase(target_tokens)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve damage replacement counter parsing through captured roles: missing `{required}`"
        );
    }
    for forbidden in [
        "grammar::words_match_prefix(tokens, &[\"if\", \"damage\", \"would\", \"be\", \"dealt\", \"to\"])",
        "find_phrase_start(&tokens[6..], &[\"this\", \"turn\"])",
        "let this_turn_idx = 6 + this_turn_rel",
        "let target_tokens = &tokens[6..this_turn_idx]",
        "let tail = &clause_words[this_turn_idx + 2..]",
        "let valid_tail = matches!",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse damage replacement counters by fixed offsets or exact tail arrays `{forbidden}`"
        );
    }
}

#[test]
fn play_permission_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const PLAY_PERMISSION_DURATION_PHRASES",
        "const EXILE_THAT_CARD_INSTEAD_PHRASE",
    );
    let parser = function_source(
        &content,
        "pub(crate) fn parse_play_permission_subject_verb",
        "pub(crate) fn parse_zone_replacement_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "graveyard play-permission parsing should capture duration/permission clauses instead of counting duration words"
    );
    for required in [
        "PLAY_PERMISSION_GRAVEYARD_PATTERN",
        "LexCaptureKind::OneOfPhrase(PLAY_PERMISSION_DURATION_PHRASES)",
        "LexPattern::tail(\"permission\", LexCaptureKind::Rest)",
        "PLAY_PERMISSION_GRAVEYARD_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Modifier, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "trim_commas(permission_clause.tokens())",
        "PLAY_LANDS_CAST_SPELLS_GRAVEYARD_PATTERN.matches_clause(permission_clause)",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve play-permission parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "duration_words",
        "token_index_for_word_index(tokens, duration_words)",
        "tokens[tail_idx..]",
        "UNTIL_END_OF_TURN_PREFIX_PATTERN",
        "THIS_TURN_PREFIX_PATTERN",
        "non_article_token_word_refs(&rest)",
        "PLAY_LANDS_CAST_SPELLS_GRAVEYARD_PATTERN.matches_words",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse play-permission clauses by fixed duration word counts `{forbidden}`"
        );
        assert!(
            !content.contains("UNTIL_END_OF_TURN_PREFIX_PATTERN")
                && !content.contains("THIS_TURN_PREFIX_PATTERN"),
            "{relative} should not keep retired play-permission duration prefix helpers"
        );
    }
}

#[test]
fn secret_number_choice_vote_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const VOTE_REVEAL_TAIL_PREFIX_PHRASES",
        "const GENERIC_VOTE_START_PATTERN",
    );
    let tail_helper = function_source(
        &content,
        "fn vote_options_clause_before_reveal_tail",
        "fn parse_vote_reveal_sentence",
    );
    let parser = function_source(
        &content,
        "fn parse_secret_number_choice_vote_start",
        "fn parse_generic_vote_start",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "secret numeric vote parsing should capture participants/options instead of slicing around choose"
    );
    for required in [
        "SECRET_NUMBER_CHOICE_PATTERN",
        "LexPattern::subject(\"participants\", LexCaptureKind::UntilPhrase(&[\"choose\"]))",
        "LexPattern::action(\"choose\", LexCaptureKind::OneOf(&[\"choose\"]))",
        "LexPattern::tail(\"options\", LexCaptureKind::Rest)",
        "SECRET_NUMBER_CHOICE_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "SECRET_CHOICE_PARTICIPANTS_PATTERN",
        "SECRET_CHOICE_PARTICIPANTS_PATTERN.matches_clause(participants_clause.trimmed())",
        "vote_options_clause_before_reveal_tail(options_clause)",
        "VOTE_REVEAL_TAIL_PATTERN.find_in_clause(options_clause)",
        "between_word_range(0, matched.word_range.start)",
        "split_vote_option_clauses(option_clause)",
        "captured_numeric_label",
    ] {
        assert!(
            shape.contains(required) || tail_helper.contains(required) || parser.contains(required),
            "{relative} should preserve secret numeric vote parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "let Some(choose_idx)",
        "clause_words[..choose_idx]",
        "clause_words[choose_idx + 1..]",
        "find_index(&clause_words",
        "truncate_vote_reveal_tail",
        "THEN_THOSE_VOTES_ARE_PREFIX_PATTERN.matches_words",
        "THEN_THOSE_CHOICES_ARE_PREFIX_PATTERN.matches_words",
        "fn vote_options_before_reveal_tail",
        "let participant_words = participants_clause.word_refs()",
        "participant_words.starts_with",
        "participant_words.contains(&\"each\")",
        "SECRET_OR_SECRETLY_WORD_PATTERN.matches_word",
        ".filter(|word| !OR_WORD_PATTERN.matches_word(word))",
    ] {
        assert!(
            !tail_helper.contains(forbidden) && !parser.contains(forbidden),
            "{relative} should not parse secret numeric vote clauses by choose-index slicing `{forbidden}`"
        );
    }
}

#[test]
fn generic_vote_start_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const VOTE_REVEAL_TAIL_PREFIX_PHRASES",
        "const GENERIC_VOTE_OPTION_EFFECT_PATTERN",
    );
    let tail_helper = function_source(
        &content,
        "fn vote_options_clause_before_reveal_tail",
        "fn parse_vote_reveal_sentence",
    );
    let parser = function_source(
        &content,
        "fn parse_generic_vote_start",
        "fn parse_generic_vote_option_effects",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "generic vote-start parsing should capture voters/options instead of slicing around vote/for"
    );
    for required in [
        "GENERIC_VOTE_START_PATTERN",
        "LexPattern::subject(",
        "\"voters\"",
        "LexCaptureKind::UntilAnyPhrase(&[&[\"vote\"], &[\"votes\"]])",
        "LexPattern::action(\"vote\", LexCaptureKind::OneOf(&[\"vote\", \"votes\"]))",
        "LexPattern::word(\"for\")",
        "LexPattern::tail(\"options\", LexCaptureKind::Rest)",
        "GENERIC_VOTE_START_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "EACH_PLAYER_VOTER_PATTERN",
        ".find_in_clause(voters_clause)",
        "SECRET_VOTER_PATTERN.find_in_clause(voters_clause)",
        "vote_options_clause_before_reveal_tail(options_clause)",
        "option_clause.tokens().to_vec()",
        "VOTE_OPTION_DELIMITER_PATTERN",
        "split_vote_option_clauses(option_clause)",
        "captured_non_article_label",
    ] {
        assert!(
            shape.contains(required) || tail_helper.contains(required) || parser.contains(required),
            "{relative} should preserve generic vote-start parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "let Some(vote_idx)",
        "clause_words[..vote_idx]",
        "find_index(&clause_words",
        "let for_idx",
        "clause_words[for_idx + 1..]",
        "FOR_WORD_PATTERN",
        "truncate_vote_reveal_tail",
        "THEN_THOSE_VOTES_ARE_PREFIX_PATTERN.matches_words",
        "THEN_THOSE_CHOICES_ARE_PREFIX_PATTERN.matches_words",
        "fn vote_options_before_reveal_tail",
        "synthetic_word_tokens(&option_words)",
        "let voter_words = voters_clause.word_refs()",
        "EACH_WORD_PATTERN.matches_words",
        "PLAYER_OR_PLAYERS_WORD_PATTERN.matches_word",
        "SECRET_OR_SECRETLY_WORD_PATTERN.matches_word(word)",
        "OR_WORD_PATTERN",
        "let mut current",
        "current.join(\" \")",
    ] {
        assert!(
            !tail_helper.contains(forbidden) && !parser.contains(forbidden),
            "{relative} should not parse generic vote-start clauses by vote/for index slicing `{forbidden}`"
        );
        assert!(
            !content.contains("FOR_WORD_PATTERN"),
            "{relative} should not keep retired generic vote-start helper FOR_WORD_PATTERN"
        );
    }
}

#[test]
fn generic_vote_option_effects_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const GENERIC_VOTE_OPTION_EFFECT_PATTERN",
        "const OPTIONAL_AN_PATTERN_ATOMS",
    );
    let parser = function_source(
        &content,
        "fn parse_generic_vote_option_effects",
        "fn parse_generic_extra_vote",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "vote-option effect parsing should capture option/player/effect clauses instead of slicing around vote and comma"
    );
    for required in [
        "GENERIC_VOTE_OPTION_EFFECT_PATTERN",
        "GENERIC_PLAYER_VOTE_RECEIVED_PATTERN",
        "LexPattern::capture(",
        "\"option\"",
        "LexPattern::subject(",
        "\"player\"",
        "LexPattern::tail(\"effects\", LexCaptureKind::Rest)",
        "GENERIC_VOTE_OPTION_EFFECT_PATTERN.match_clause(clause)",
        "GENERIC_PLAYER_VOTE_RECEIVED_PATTERN.match_clause(clause)",
        "capture_clause(\"option\", clause)",
        "capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "captured_non_article_label(option_clause)",
        "captured_non_article_tokens(player_clause)",
        "fn captured_non_article_label",
        "fn captured_non_article_tokens",
        ".filter(|token| token.as_word().is_none_or(|word| !is_article(word)))",
        "render_token_slice(&tokens).trim().to_string()",
        "parse_target_phrase(&player_tokens)",
        "let effect_tokens = trim_commas(effect_clause.tokens())",
        "parse_effect_chain_lexed(&effect_tokens)",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve vote-option effect parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "let Some(vote_idx)",
        "words[2..vote_idx]",
        "words[vote_idx + 1..]",
        "let Some(received_idx)",
        "split_lexed_once_on_delimiter(tokens, super::super::lexer::TokenKind::Comma)",
        "VOTE_OR_VOTES_WORD_PATTERN",
        "let option_word_storage = option_clause.word_refs()",
        "let player_word_storage = player_clause.word_refs()",
        "crate::runtime_backend::util::non_article_word_refs(&option_word_storage)",
        "crate::runtime_backend::util::non_article_word_refs(&player_word_storage)",
        "non_article_token_word_refs(clause.trimmed().tokens())",
        "synthetic_word_tokens",
        "words.join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse vote-option clauses by vote-index/comma splitting `{forbidden}`"
        );
        assert!(
            !content.contains("VOTE_OR_VOTES_WORD_PATTERN"),
            "{relative} should not keep retired vote word helper VOTE_OR_VOTES_WORD_PATTERN"
        );
    }
}

#[test]
fn generic_extra_vote_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const OPTIONAL_AN_PATTERN_ATOMS",
        "const DAMAGE_REPLACEMENT_COUNTER_TARGET_PHRASE",
    );
    let parser = function_source(&content, "fn parse_generic_extra_vote", "#[cfg(test)]");
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "extra-vote parsing should match optional/required vote shapes instead of probing loose words"
    );
    for required in [
        "OPTIONAL_EXTRA_VOTE_PATTERN",
        "REQUIRED_EXTRA_VOTE_PATTERN",
        "LexPattern::subject(\"voter\", LexCaptureKind::OneOf(&[\"you\"]))",
        "LexPattern::capture(\"may\", LexCaptureKind::OneOf(&[\"may\"]))",
        "LexPattern::action(\"vote\", LexCaptureKind::OneOf(&[\"vote\", \"votes\"]))",
        "LexPattern::optional(OPTIONAL_AN_PATTERN_ATOMS)",
        "LexPattern::word(\"additional\")",
        "LexPattern::amount(\"time\", LexCaptureKind::OneOf(&[\"time\", \"times\"]))",
        "OPTIONAL_EXTRA_VOTE_PATTERN.match_clause(clause)",
        "REQUIRED_EXTRA_VOTE_PATTERN.match_clause(clause)",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve extra-vote parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs(tokens)",
        "VOTE_EXTRA_MARKER_PATTERN",
        "TIME_OR_TIMES_MARKER_PATTERN",
        "grammar::contains_word(tokens, \"additional\")",
        "grammar::contains_word(tokens, \"may\")",
        "has_vote",
        "has_additional",
        "has_time",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse extra-vote clauses through loose word checks `{forbidden}`"
        );
    }
    for retired in ["VOTE_EXTRA_MARKER_PATTERN", "TIME_OR_TIMES_MARKER_PATTERN"] {
        assert!(
            !content.contains(retired),
            "{relative} should not keep retired extra-vote helper {retired}"
        );
    }
}

#[test]
fn vote_reveal_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const OPTIONAL_THEN_PATTERN_ATOMS",
        "const EACH_PLAYER_VOTER_PATTERN",
    );
    let parser = function_source(
        &content,
        "fn parse_vote_reveal_sentence",
        "fn parse_secret_number_choice_vote_start",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "vote reveal parsing should capture revealed choice shapes instead of matching raw word slices"
    );
    for required in [
        "VOTE_REVEAL_PATTERN",
        "LexPattern::optional(OPTIONAL_THEN_PATTERN_ATOMS)",
        "LexPattern::subject(\"choices\", LexCaptureKind::OneOfPhrase(THOSE_CHOICES_PHRASES))",
        "LexPattern::word(\"are\")",
        "LexPattern::action(\"reveal\", LexCaptureKind::OneOf(&[\"revealed\"]))",
        "VOTE_REVEAL_PATTERN",
        "match_clause(LexedClause::new(tokens).trimmed())",
        "EffectAst::SecretChoiceReveal",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve vote-reveal parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs(tokens)",
        "words.as_slice()",
        "[\"then\", \"those\", \"choices\", \"are\", \"revealed\"]",
        "[\"those\", \"choices\", \"are\", \"revealed\"]",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse vote reveal clauses through raw word-slice matching `{forbidden}`"
        );
    }
}

#[test]
fn zone_replacement_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const EXILE_THAT_CARD_INSTEAD_PHRASE",
        "const EACH_PLAYER_PHRASES",
    );
    let parser = function_source(
        &content,
        "pub(crate) fn parse_zone_replacement_subject_verb",
        "pub(crate) fn parse_choice_complement_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "future zone replacement parsing should capture condition/replacement clauses instead of splitting at a comma"
    );
    for required in [
        "ZONE_REPLACEMENT_GRAVEYARD_EXILE_PATTERN",
        "LexPattern::condition(",
        "LexCaptureKind::UntilPhrase(EXILE_THAT_CARD_INSTEAD_PHRASE)",
        "LexPattern::tail(\"replacement\", LexCaptureKind::Rest)",
        "ZONE_REPLACEMENT_GRAVEYARD_EXILE_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Condition, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "FUTURE_GRAVEYARD_EXILE_CONDITION_PATTERN",
        "FUTURE_GRAVEYARD_EXILE_CONDITION_PATTERN.match_clause(condition_clause)",
        "capture_clause(\"destination\", condition_clause)",
        "FUTURE_GRAVEYARD_DESTINATION_PATTERN.matches_clause(destination_clause.trimmed())",
        "EXILE_THAT_CARD_INSTEAD_PATTERN.matches_clause(replacement_clause.trimmed())",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve zone replacement parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "split_once_on_comma(tokens)",
        "let Some((_, remainder))",
        "non_article_token_word_refs(remainder)",
        "crate::runtime_backend::lexer::find_token_kind",
        "TokenKind::Comma",
        "condition_clause.word_refs()",
        "IF_WORD_PATTERN.matches_words",
        "YOUR_GRAVEYARD_MARKER_PATTERN.matches_words",
        "CARD_WOULD_BE_PUT_MARKER_PATTERN.matches_words",
        "THIS_TURN_MARKER_PATTERN.matches_words",
        "EXILE_THAT_CARD_INSTEAD_PATTERN.matches_words",
        "non_article_token_word_refs(replacement_clause.tokens())",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse future zone replacement clauses with comma helper `{forbidden}`"
        );
        assert!(
            !content.contains("fn split_once_on_comma"),
            "{relative} should not keep the retired comma-split helper"
        );
    }
}

#[test]
fn choice_complement_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const EACH_PLAYER_PHRASES",
        "const WHERE_X_IS_PHRASE",
    );
    let parser = function_source(
        &content,
        "pub(crate) fn parse_choice_complement_subject_verb",
        "pub(crate) fn parse_vote_affinity_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "choice-complement parsing should capture chooser/choice/sacrifice spans instead of splitting around then and fixed offsets"
    );
    for required in [
        "CHOICE_COMPLEMENT_PATTERN",
        "LexPattern::subject(\"chooser\", LexCaptureKind::OneOfPhrase(EACH_PLAYER_PHRASES))",
        "LexPattern::action(\"choose\", LexCaptureKind::OneOf(&[\"choose\", \"chooses\"]))",
        "LexPattern::object(\"choice_clause\", LexCaptureKind::UntilPhrase(&[\"then\"]))",
        "LexPattern::word(\"then\")",
        "LexPattern::action(\"sacrifice\", LexCaptureKind::OneOf(&[\"sacrifice\", \"sacrifices\"]))",
        "LexPattern::phrase(&[\"the\", \"rest\"])",
        "CHOICE_COMPLEMENT_LIST_FROM_AMONG_PATTERN",
        "LexPattern::object(\"choice_list\", LexCaptureKind::UntilPhrase(&[\"from\", \"among\"]))",
        "LexPattern::tail(\"base_filter\", LexCaptureKind::Rest)",
        "CHOICE_COMPLEMENT_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Object, clause)",
        "render_token_slice(clause.tokens())",
        "CHOICE_COMPLEMENT_LIST_FROM_AMONG_PATTERN.match_clause(choice_clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, choice_clause)",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve choice-complement parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs(tokens)",
        "EACH_PLAYER_CHOOSES_PREFIX_PATTERN",
        "SACRIFICE_THE_REST_PREFIX_PATTERN",
        "split_lexed_once_on_separator(tokens",
        "grammar::kw(\"then\")",
        "let then_idx = before_then.len()",
        "&tokens[3..then_idx]",
        "after_then",
        "clause.word_refs().join",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse choice-complement clauses with top-level raw splitting `{forbidden}`"
        );
    }
    for retired in [
        "EACH_PLAYER_CHOOSES_PREFIX_PATTERN",
        "SACRIFICE_THE_REST_PREFIX_PATTERN",
    ] {
        assert!(
            !content.contains(retired),
            "{relative} should not keep retired choice-complement helper {retired}"
        );
    }
}

#[test]
fn meld_result_parser_uses_lex_pattern_capture() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_generic_meld_subject_verb",
        "fn parse_generic_control_combat_choices_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "meld-result parsing should capture the melded object name from the grammar pattern, not slice after a phrase window"
    );
    for required in [
        "MELD_RESULT_PATTERN.match_clause(clause)",
        "LexPattern::object(\"result\", LexCaptureKind::OneOrMoreWords)",
        "capture_clause_by_role(LexCaptureRole::Object",
        "render_token_slice(result_clause.tokens())",
        "EffectAst::subject_verb_meld(result_name, false, false)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should preserve meld result parsing through captured roles: missing `{required}`"
        );
    }
    for forbidden in [
        "THEN_MELD_THEM_INTO_PATTERN",
        "find_window_by(&clause_words, 4",
        "let result_words = &clause_words[meld_idx + 4..]",
        "let result_words = result_clause.word_refs()",
        "clause.word_refs()",
        "result_words.as_slice().join",
        "grammar::words_match_prefix(tokens, &[\"exile\", \"them\"])",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse meld result clauses by fixed phrase windows `{forbidden}`"
        );
    }
}

#[test]
fn control_combat_choices_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const CONTROL_COMBAT_CHOICE_OBJECT_PHRASES",
        "const DEFERRED_MANA_VALUE_CONSTRAINT_PHRASES",
    );
    let parser = function_source(
        &content,
        "fn parse_generic_control_combat_choices_subject_verb",
        "fn parse_generic_damage_replacement_counters_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "combat choice parsing should capture chooser/object/action/scope instead of matching whole raw sentences"
    );
    for required in [
        "CONTROL_COMBAT_CHOICES_PATTERN",
        "LexPattern::subject(\"chooser\", LexCaptureKind::OneOf(&[\"you\"]))",
        "LexPattern::phrase(&[\"choose\", \"which\"])",
        "LexPattern::object(",
        "\"objects\"",
        "LexPattern::action(\"combat_action\", LexCaptureKind::OneOf(&[\"attack\", \"block\"]))",
        "LexPattern::tail(\"choice_scope\", LexCaptureKind::Rest)",
        "CONTROL_COMBAT_ATTACK_ACTION_PATTERN",
        "CONTROL_COMBAT_BLOCK_ACTION_PATTERN",
        "CONTROL_COMBAT_ATTACK_SCOPE_PATTERN",
        "CONTROL_COMBAT_BLOCK_SCOPE_PATTERN",
        "CONTROL_COMBAT_CHOICES_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Action, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "CONTROL_COMBAT_ATTACK_ACTION_PATTERN.matches_clause(action_clause)",
        "CONTROL_COMBAT_ATTACK_SCOPE_PATTERN.matches_clause(scope_clause)",
        "CONTROL_COMBAT_BLOCK_ACTION_PATTERN.matches_clause(action_clause)",
        "CONTROL_COMBAT_BLOCK_SCOPE_PATTERN.matches_clause(scope_clause)",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve combat-choice parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs(tokens)",
        "CHOOSE_ATTACK_THIS_TURN_PATTERN",
        "CHOOSE_BLOCK_THIS_TURN_PATTERN",
        "[\"you\", \"choose\", \"which\", \"creatures\", \"attack\"",
        "[\"you\", \"choose\", \"which\", \"creatures\", \"block\"",
        "action_clause.word_refs().as_slice()",
        "CONTROL_COMBAT_ATTACK_SCOPE_PATTERN.matches_words",
        "CONTROL_COMBAT_BLOCK_SCOPE_PATTERN.matches_words",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse combat choices through whole-sentence word checks `{forbidden}`"
        );
    }
    for retired in [
        "CHOOSE_ATTACK_THIS_TURN_PATTERN",
        "CHOOSE_BLOCK_THIS_TURN_PATTERN",
    ] {
        assert!(
            !content.contains(retired),
            "{relative} should not keep retired combat-choice helper {retired}"
        );
    }
}

#[test]
fn deferred_mana_value_clause_strip_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const DEFERRED_MANA_VALUE_CONSTRAINT_PHRASES",
        "const PLAY_PERMISSION_DURATION_PHRASES",
    );
    let helper = function_source(
        &content,
        "fn without_deferred_mana_value_clause",
        "pub(crate) fn parse_play_permission_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "deferred mana-value stripping should capture the effect prefix instead of scanning four-word windows"
    );
    for required in [
        "DEFERRED_MANA_VALUE_CLAUSE_PATTERN",
        "LexCaptureKind::UntilAnyPhrase(DEFERRED_MANA_VALUE_CONSTRAINT_PHRASES)",
        "LexPattern::any_phrase(DEFERRED_MANA_VALUE_CONSTRAINT_PHRASES)",
        "LexPattern::tail(\"constraint_tail\", LexCaptureKind::Rest)",
        "DEFERRED_MANA_VALUE_CLAUSE_PATTERN.match_clause(clause)",
        "capture_word_range(\"effect\")",
        "clause.token_index_after_words(effect_range.end)",
    ] {
        assert!(
            shape.contains(required) || helper.contains(required),
            "{relative} should preserve deferred mana-value stripping through captured ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "find_window_by(tokens, 4",
        "TokenWordView::new(window).word_refs()",
        "WITH_LESSER_MANA_VALUE_PATTERN.matches_words",
        "WITH_MANA_VALUE_EQUAL_PATTERN.matches_words",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not strip deferred mana-value clauses by fixed word windows `{forbidden}`"
        );
    }
    for retired in [
        "const WITH_LESSER_MANA_VALUE_PATTERN",
        "const WITH_MANA_VALUE_EQUAL_PATTERN",
    ] {
        assert!(
            !content.contains(retired),
            "{relative} should not keep retired deferred mana-value helper {retired}"
        );
    }
}

#[test]
fn where_x_value_binding_probe_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const WHERE_X_IS_PHRASE",
        "const SOURCE_GETS_SUBJECT_PATTERN",
    );
    let helper = function_source(
        &content,
        "fn has_where_x_value_binding",
        "pub(crate) fn parse_top_level_subject_verb_recognition",
    );
    let dispatch = function_source(
        &content,
        "pub(crate) fn parse_top_level_subject_verb_recognition",
        "fn parse_source_gets_unblockable_subject_verb",
    );
    let gain_gates = function_source(
        &content,
        "fn parse_target_gains_then_gets_subject_verb",
        "fn parse_target_player_controls_get_subject_verb",
    );
    let checked_source = format!("{helper}{dispatch}{gain_gates}");
    let actual = non_test_raw_text_check_literals(&checked_source)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "where-X value binding detection should capture effect/definition spans instead of broad phrase probing"
    );
    for required in [
        "WHERE_X_VALUE_BINDING_PATTERN",
        "LexPattern::condition(\"effect\", LexCaptureKind::UntilPhrase(WHERE_X_IS_PHRASE))",
        "LexPattern::phrase(WHERE_X_IS_PHRASE)",
        "LexPattern::tail(\"definition\", LexCaptureKind::Rest)",
        "WHERE_X_VALUE_BINDING_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Condition, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "has_where_x_value_binding(tokens)",
        "parse_effect_sentence_with_where_x_lexed(tokens)",
    ] {
        assert!(
            shape.contains(required)
                || helper.contains(required)
                || dispatch.contains(required)
                || gain_gates.contains(required),
            "{relative} should preserve where-X value-binding detection through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "WHERE_X_IS_PATTERN",
        "DispatchInnerNormalizedWords::new(tokens)",
        "WHERE_X_IS_PATTERN.matches_words",
        "clause_words.as_slice()",
        "clause.word_refs()",
    ] {
        assert!(
            !helper.contains(forbidden)
                && !dispatch.contains(forbidden)
                && !gain_gates.contains(forbidden),
            "{relative} should not detect where-X value bindings through raw phrase probes `{forbidden}`"
        );
    }
    assert!(
        !content.contains("const WHERE_X_IS_PATTERN"),
        "{relative} should not keep retired WHERE_X_IS_PATTERN"
    );
}

#[test]
fn where_x_effect_sentence_uses_token_rendered_clause_surface() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/sentence_shape_predicates.rs";
    let content = read_repo_file(&root, relative);
    let start_marker = "fn parse_effect_sentence_with_where_x_lexed";
    let start = content
        .find(start_marker)
        .unwrap_or_else(|| panic!("missing function start marker: {start_marker}"));
    let parser = &content[start..];

    for required in [
        "let clause_display = render_token_slice(tokens).trim().to_string()",
        "replace_unbound_x_in_effects_anywhere(&mut effects, &where_value, &clause_display)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve where-X clause surfaces from parse tokens: missing `{required}`"
        );
    }
    for forbidden in ["clause_words.join(\" \")"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild where-X clause surfaces by joining word refs `{forbidden}`"
        );
    }
}

#[test]
fn top_cards_hand_remainder_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const PUT_COUNTED_TOP_CARDS_OBJECT_PHRASES",
        "fn parse_generic_consult_reveal_until_put_all_revealed_into_hand_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "top-card hand/remainder parsing should capture grammar spans instead of walking raw word indexes"
    );
    for required in [
        "PUT_COUNTED_TOP_CARDS_VIEW_THEN_REMAINDER_PATTERN",
        "LexPattern::capture(\"view_clause\", LexCaptureKind::UntilPhrase(&[\"then\"]))",
        "LexPattern::tail(\"remainder\", LexCaptureKind::Rest)",
        "PUT_COUNTED_TOP_CARDS_VIEW_THEN_REMAINDER_PATTERN.match_clause(clause)",
        "capture_clause(\"view_clause\", clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "PUT_COUNTED_TOP_CARDS_REMAINDER_PATTERN.match_clause(tail_clause)",
        "LexPattern::amount(",
        "\"put_count\"",
        "LexCaptureKind::UntilAnyPhrase(PUT_COUNTED_TOP_CARDS_OBJECT_PHRASES)",
        "PUT_COUNTED_TOP_CARDS_YOU_OWNER_PATTERN",
        "PUT_COUNTED_TOP_CARDS_THAT_OWNER_PATTERN",
        "LexCaptureKind::OneOfPhrase(PUT_COUNTED_TOP_CARDS_YOU_OWNER_PHRASES)",
        "LexCaptureKind::OneOfPhrase(PUT_COUNTED_TOP_CARDS_THAT_OWNER_PHRASES)",
        "PUT_COUNTED_TOP_CARDS_YOU_OWNER_PATTERN.matches_clause(owner_clause)",
        "PUT_COUNTED_TOP_CARDS_THAT_OWNER_PATTERN.matches_clause(owner_clause)",
        "LexPattern::capture(",
        "\"hand_owner\"",
        "\"graveyard_owner\"",
        "capture_clause(\"put_count\", tail_clause)",
        "capture_clause(\"hand_owner\", tail_clause)",
        "put_counted_top_cards_owner(hand_owner_clause, player)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve top-card hand/remainder parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "let mut idx = 0usize",
        "OF_WORD_PATTERN.matches_word_at(&tail_refs, idx)",
        "THOSE_CARD_OR_CARDS_PATTERN.matches_words(&tail_refs[idx..])",
        "let chooser = if YOUR_WORD_PATTERN.matches_word_at(&tail_refs, idx)",
        "YOUR_OR_THEIR_WORD_PATTERN.matches_word_at(&tail_refs, idx)",
        "GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN.matches_word_at(&tail_refs, idx)",
        "idx += 1",
        "idx += 2",
        "find_generic_word_matching_shape",
        "then_word_idx",
        "token_index_for_word_index",
        "token_index_after_words",
        "owner_clause.trimmed().word_refs()",
        "words.as_slice()",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse top-card hand/remainder tails with fixed index walking `{forbidden}`"
        );
    }
}

#[test]
fn consult_reveal_until_hand_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const CONSULT_REVEAL_UNTIL_HAND_PATTERN",
        "const MATCH_ONTO_BATTLEFIELD_PREFIX_PATTERN",
    );
    let parser = function_source(
        &content,
        "fn parse_generic_consult_reveal_until_put_all_revealed_into_hand_subject_verb",
        "fn parse_generic_consult_reveal_until_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "consult all-revealed-to-hand parsing should capture consult/followup clauses instead of slicing around then"
    );
    for required in [
        "CONSULT_REVEAL_UNTIL_HAND_PATTERN",
        "LexPattern::capture(\"consult_clause\", LexCaptureKind::UntilPhrase(&[\"then\"]))",
        "LexPattern::word(\"then\")",
        "LexPattern::tail(\"followup\", LexCaptureKind::Rest)",
        "CONSULT_REVEAL_UNTIL_HAND_PATTERN.match_clause(sentence_clause)",
        "capture_clause(\"consult_clause\", sentence_clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, sentence_clause)",
        "trim_commas(consult_clause.tokens())",
        "trim_commas(followup_clause.tokens())",
        "ALL_REVEALED_INTO_HAND_PHRASES",
        "LexCaptureKind::OneOfPhrase",
        "ALL_REVEALED_INTO_HAND_PATTERN.matches_clause(followup_clause)",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve consult all-revealed-to-hand parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "let Some(then_idx)",
        "sentence_tokens[..then_idx]",
        "sentence_tokens[then_idx + 1..]",
        "find_index(&sentence_tokens",
        "followup_words",
        "ALL_REVEALED_INTO_HAND_PATTERN.matches_words",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not split consult all-revealed-to-hand clauses with fixed token indexes `{forbidden}`"
        );
    }
}

#[test]
fn consult_reveal_until_battlefield_bottom_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const MATCH_ONTO_BATTLEFIELD_PREFIX_PATTERN",
        "const EACH_PLAYER_EXILE_TOP_CARD_PREFIX_PHRASES",
    );
    let parser = function_source(
        &content,
        "fn parse_generic_consult_reveal_until_battlefield_bottom_subject_verb",
        "fn parse_generic_each_player_exile_top_then_cast_any_number_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "consult battlefield-bottom parsing should capture consult/followup clauses instead of splitting at the comma"
    );
    for required in [
        "CONSULT_REVEAL_UNTIL_BATTLEFIELD_BOTTOM_PATTERN",
        "MATCH_ONTO_BATTLEFIELD_PREFIX_PATTERN",
        "LexCaptureKind::UntilAnyPhrase(MATCH_ONTO_BATTLEFIELD_PREFIX_PHRASES)",
        "LexCaptureKind::OneOfPhrase(MATCH_ONTO_BATTLEFIELD_PREFIX_PHRASES)",
        "LexPattern::tail(\"followup\", LexCaptureKind::Rest)",
        "REST_BOTTOM_LIBRARY_WITH_ORDER_PATTERN",
        "REST_BOTTOM_LIBRARY_ORDER_PHRASES",
        "LexCaptureKind::UntilAnyPhrase(REST_BOTTOM_LIBRARY_ORDER_PHRASES)",
        "LexCaptureKind::OneOfPhrase(REST_BOTTOM_LIBRARY_ORDER_PHRASES)",
        "REST_BOTTOM_LIBRARY_RANDOM_ORDER_PATTERN",
        "REST_BOTTOM_LIBRARY_ANY_ORDER_PATTERN",
        "CONSULT_REVEAL_UNTIL_BATTLEFIELD_BOTTOM_PATTERN.match_clause(sentence_clause)",
        "capture_clause(\"consult_clause\", sentence_clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, sentence_clause)",
        "trim_commas(consult_clause.tokens())",
        "trim_commas(followup_clause.tokens())",
        "MATCH_ONTO_BATTLEFIELD_PREFIX_PATTERN.match_clause(followup_clause)",
        "followup_match.capture_clause_by_role(LexCaptureRole::Tail, followup_clause)",
        "consult_remainder_order_from_capture(remainder_clause.trimmed())",
        "fn consult_remainder_order_from_capture",
        "REST_BOTTOM_LIBRARY_WITH_ORDER_PATTERN.find_in_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Amount, clause)",
        "REST_BOTTOM_LIBRARY_RANDOM_ORDER_PATTERN.matches_clause(order_clause)",
        "REST_BOTTOM_LIBRARY_ANY_ORDER_PATTERN.matches_clause(order_clause)",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve consult battlefield-bottom parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "split_once_on_comma(&sentence_tokens)",
        "let Some((consult_tokens, followup_tokens))",
        "trim_commas(consult_tokens)",
        "trim_commas(followup_tokens)",
        "MATCH_ONTO_BATTLEFIELD_PREFIX_PATTERN.matches_words",
        "REST_BOTTOM_LIBRARY_PATTERN.matches_words",
        "followup_words.as_slice()",
        "let followup_words = followup_clause.word_refs()",
        "parse_consult_remainder_order(&followup_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not split consult battlefield-bottom clauses with comma helper `{forbidden}`"
        );
    }
}

#[test]
fn each_player_exile_top_cast_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const EACH_PLAYER_EXILE_TOP_CARD_PREFIX_PHRASES",
        "const MELD_RESULT_PATTERN",
    );
    let parser = function_source(
        &content,
        "fn parse_generic_each_player_exile_top_then_cast_any_number_subject_verb",
        "fn parse_generic_meld_subject_verb",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "each-player exile-top/cast parsing should capture exile/cast clauses instead of slicing around then"
    );
    for required in [
        "EACH_PLAYER_EXILE_TOP_CAST_PATTERN",
        "LexPattern::capture(\"exile_clause\", LexCaptureKind::UntilPhrase(&[\"then\"]))",
        "LexPattern::word(\"then\")",
        "LexPattern::tail(\"cast_clause\", LexCaptureKind::Rest)",
        "EACH_PLAYER_EXILE_TOP_CAST_PATTERN.match_clause(sentence_clause)",
        "capture_clause(\"exile_clause\", sentence_clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, sentence_clause)",
        "trim_commas(exile_clause.tokens())",
        "trim_commas(cast_clause.tokens())",
        "EACH_PLAYER_EXILE_TOP_CARD_PATTERN",
        "LexCaptureKind::OneOfPhrase(EACH_PLAYER_EXILE_TOP_CARD_PREFIX_PHRASES)",
        "EACH_PLAYER_EXILE_UNTIL_NONLAND_PATTERN.matches_clause(exile_clause)",
        "EACH_PLAYER_EXILE_TOP_CARD_PATTERN.match_clause(exile_clause)",
        "PLAYER_LIBRARY_PATTERN",
        ".find_in_clause(library_clause.trimmed())",
        "CAST_ANY_NUMBER_FREE_PATTERN.match_clause(cast_clause)",
        "capture_clause_by_role(LexCaptureRole::Object, cast_clause)",
        "FROM_THOSE_OR_THEM_SCOPE_PATTERN",
        "FROM_NONLAND_EXILED_THIS_WAY_PATTERN",
        ".find_in_clause(cast_scope_clause.trimmed())",
    ] {
        assert!(
            shape.contains(required) || parser.contains(required),
            "{relative} should preserve each-player exile-top/cast parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "let Some(then_idx)",
        "sentence_tokens[..then_idx]",
        "sentence_tokens[then_idx + 1..]",
        "find_index(&sentence_tokens",
        "TokenWordView::new(&exile_tokens)",
        "TokenWordView::new(&cast_tokens)",
        "EACH_PLAYER_EXILE_TOP_CARD_PREFIX_PATTERN.matches_words",
        "EACH_PLAYER_EXILE_UNTIL_NONLAND_PREFIX_PATTERN.matches_words",
        "PLAYER_LIBRARY_MARKER_PATTERN.matches_words",
        "CAST_ANY_NUMBER_FREE_PREFIX_PATTERN.matches_words",
        "FROM_THOSE_OR_THEM_MARKER_PATTERN.matches_words",
        "WITHOUT_PAYING_THEIR_MANA_COSTS_SUFFIX_PATTERN.matches_words",
        "FROM_NONLAND_EXILED_THIS_WAY_PATTERN.matches_words",
        "cast_words.as_slice()",
        "exile_words.as_slice()",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not split each-player exile-top/cast clauses with fixed token indexes `{forbidden}`"
        );
    }
}

#[test]
fn delayed_end_step_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/copy_and_next_spell_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const DELAYED_END_STEP_OPTIONAL_THE_ATOMS",
        "fn retarget_source_copy_spell_to_delayed_triggering_object",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "delayed end-step parsing should capture timing owner/effect spans instead of walking token offsets"
    );
    for required in [
        "DELAYED_END_STEP_HEADER_PATTERN.match_clause(clause)",
        "LexPattern::capture(",
        "\"step_owner\"",
        "\"turn_owner\"",
        "LexPattern::token(TokenKind::Comma)",
        "LexPattern::tail(\"effect\", LexCaptureKind::Rest)",
        "capture_clause(\"step_owner\", clause)",
        "capture_clause(\"turn_owner\", clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "delayed_end_step_player_from_owner",
        "DELAYED_END_STEP_YOUR_OWNER_PATTERN.matches_clause(owner_clause)",
        "DELAYED_END_STEP_THAT_PLAYER_OWNER_PATTERN.matches_clause(owner_clause)",
        "DELAYED_END_STEP_TARGET_PLAYER_OWNER_PATTERN.matches_clause(owner_clause)",
        "delayed_end_step_player_from_owner(matched.capture_clause(\"step_owner\", clause))",
        "delayed_end_step_player_from_owner(Some(turn_owner))",
        "render_token_slice(tokens).trim()",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve delayed end-step parsing through captures: missing `{required}`"
        );
    }
    for forbidden in [
        "let mut idx = 0usize",
        "token_slice_at_is(tokens, idx",
        "idx += 1",
        "idx += 2",
        "idx += 3",
        "let remainder = trim_commas(&tokens[idx..])",
        "let remainder = trim_commas(effect_clause.tokens())",
        "owner.word_refs()",
        "turn_owner.trimmed().word_refs()",
        "token_word_refs(tokens).join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse delayed end-step headers with fixed token index walking `{forbidden}`"
        );
    }
}

#[test]
fn this_turn_delayed_trigger_parser_uses_outer_lex_pattern_capture() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/copy_and_next_spell_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const DELAYED_ATTACKS_UNBLOCKED_PHRASES",
        "pub(crate) fn parse_delayed_when_that_dies_this_turn_sentence",
    );
    let parser_body = function_source(
        &content,
        "pub(crate) fn parse_sentence_delayed_trigger_this_turn",
        "pub(crate) fn parse_delayed_when_that_dies_this_turn_sentence",
    );
    let actual = non_test_raw_text_check_literals(parser_body)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "this-turn delayed trigger parsing should capture duration/tail spans before splitting trigger/effect clauses"
    );
    for required in [
        "DELAYED_TARGET_ATTACK_UNBLOCKED_TRIGGER_PATTERN",
        "DELAYED_TAGGED_DEALT_DAMAGE_TRIGGER_PATTERN",
        "COPY_NEXT_THIS_TURN_DELAYED_TRIGGER_PATTERN",
        "DELAYED_TRIGGER_THIS_TURN_SUFFIX_PATTERN",
        "DELAYED_NEXT_TRIGGER_MARKER_PATTERN",
        "LexPattern::object(",
        "LexCaptureKind::UntilAnyPhrase(",
        "DELAYED_ATTACKS_UNBLOCKED_PHRASES",
        "LexPattern::any_phrase(DELAYED_ATTACKS_UNBLOCKED_PHRASES)",
        "LexPattern::object(\"kind\", LexCaptureKind::OneOf(&[\"creature\", \"permanent\"]))",
        "LexPattern::optional(DELAYED_TAGGED_DEALT_DAMAGE_OPTIONAL_COMBAT_ATOMS)",
        "DELAYED_TAGGED_DAMAGE_CREATURE_KIND_PATTERN.matches_clause(kind_clause)",
        "DELAYED_TAGGED_DAMAGE_PERMANENT_KIND_PATTERN.matches_clause(kind_clause)",
        "delayed_tagged_dealt_damage_trigger_from_core(trigger_core_tokens)",
        "capture_clause_by_role(LexCaptureRole::Object, trigger_clause)",
        "matched.capture(\"combat\").is_some()",
        "LexPattern::modifier(\"duration\", LexCaptureKind::OneOfPhrase(&[&[\"this\", \"turn\"]]))",
        "LexPattern::action(\"intro\", LexCaptureKind::OneOf(&[\"when\", \"whenever\"]))",
        "LexPattern::condition(\"trigger\", LexCaptureKind::UntilToken(TokenKind::Comma))",
        "LexCaptureKind::UntilLastPhraseBeforeToken(&[\"this\", \"turn\"], TokenKind::Comma)",
        "LexPattern::token(TokenKind::Comma)",
        "LexPattern::tail(\"effect\", LexCaptureKind::Rest)",
        "delayed_attack_unblocked_filter_from_trigger(trigger_tokens, tokens)",
        "COPY_NEXT_THIS_TURN_DELAYED_TRIGGER_PATTERN.match_clause(clause)",
        "DELAYED_TRIGGER_THIS_TURN_SUFFIX_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Condition, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "delayed_trigger_is_one_shot(trigger_clause)",
        "DELAYED_NEXT_TRIGGER_MARKER_PATTERN",
        ".find_in_clause(trigger_clause.trimmed())",
        "render_token_slice(clause.tokens())",
        "render_token_slice(full_sentence_tokens)",
        "let trigger_tokens = trigger_clause.trimmed().tokens()",
        "let trigger_core_tokens = trigger_clause.trimmed().tokens()",
        "let trigger_clause = LexedClause::new(trigger_tokens)",
        "parse_effect_chain(effect_clause.trimmed().tokens())",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve this-turn delayed trigger parsing through delimiter-aware captures: missing `{required}`"
        );
    }
    for forbidden in [
        "ATTACKS_AND_ISNT_BLOCKED_SUFFIX_PATTERN",
        "TARGET_WORD_PATTERN",
        "TAGGED_DEALT_DAMAGE_TRIGGER_CORE_PATTERN",
        "CREATURE_WORD_PATTERN",
        "COMBAT_WORD_PATTERN",
        "const THIS_TURN_SUFFIX_PATTERN",
        "COPY_NEXT_THIS_TURN_PREFIX_PATTERN",
        "COPY_NEXT_THIS_TURN_PREFIX_PATTERN.matches_words",
        "let attack_unblocked_suffix",
        "let subject_len",
        "trigger_tokens[1..subject_len]",
        "token_index_for_word_index(trigger_words.len() - 2)",
        "trigger_tokens[..trim_start]",
        "trigger_core_words.get(1)",
        "token_word_refs(\n        tokens",
        "token_word_refs(tokens).join(\" \")",
        "token_word_refs(full_sentence_tokens).join(\" \")",
        "kind_clause.trimmed().word_refs()",
        "let Some(delayed_clause)",
        "let delayed_clause = trim_commas",
        "let Some((trigger_part, effect_part))",
        "let Some((before_comma, after_comma))",
        "let Some((_duration, delayed_clause))",
        "split_lexed_once_on_delimiter",
        "trigger_words.contains(&\"next\")",
    ] {
        assert!(
            !parser_body.contains(forbidden)
                && !content.contains("COPY_NEXT_THIS_TURN_PREFIX_PATTERN"),
            "{relative} should not route this-turn delayed triggers through raw prefix checks `{forbidden}`"
        );
    }
}

#[test]
fn delayed_dies_this_turn_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/copy_and_next_spell_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const DELAYED_DIES_INTRO_WORDS",
        "const DELAYED_END_STEP_OPTIONAL_THE_ATOMS",
    );
    let parser_body = function_source(
        &content,
        "fn delayed_dies_this_way_filter",
        "pub(crate) fn find_from_among",
    );
    let actual = non_test_raw_text_check_literals(parser_body)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "delayed dies-this-turn parsing should capture subject/effect spans instead of searching fixed phrase windows"
    );
    for required in [
        "DELAYED_THAT_DIES_THIS_TURN_PATTERN",
        "DELAYED_DIES_THIS_WAY_PATTERN",
        "LexPattern::object(",
        "LexCaptureKind::UntilAnyPhrase(DELAYED_DIES_THIS_WAY_PHRASES)",
        "LexPattern::token(TokenKind::Comma)",
        "LexPattern::tail(\"effect\", LexCaptureKind::Rest)",
        "DELAYED_THAT_DIES_THIS_TURN_PATTERN.match_clause(clause)",
        "DELAYED_DIES_THIS_WAY_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Object",
        "capture_clause_by_role(LexCaptureRole::Tail",
        "render_token_slice(clause.tokens())",
        "delayed_dies_this_way_filter(&matched, clause)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should preserve delayed dies-this-turn parsing through captures: missing `{required}`"
        );
    }
    for forbidden in [
        "copy_next_find_phrase_start",
        "DEALT_DAMAGE_THIS_WAY_DIES_THIS_TURN_PATTERN",
        "DEALT_DAMAGE_THIS_WAY_WOULD_DIE_THIS_TURN_PATTERN",
        "let split_after_word_idx",
        "token_slice_first_kind(remainder, TokenKind::Comma)",
        "remainder = &remainder[1..]",
        "let mut remainder = effect_clause.tokens()",
        "clause.between_word_range(1, dealt_idx)",
        "dealt_idx + 6",
        "dealt_idx + 7",
        "let clause_words = clause.word_refs()",
        "clause_words.join(\" \")",
        "delayed_dies_this_way_filter(&matched, clause, &clause_words)",
    ] {
        assert!(
            !parser_body.contains(forbidden) && !parser.contains(forbidden),
            "{relative} should not parse delayed dies-this-turn clauses with fixed phrase windows `{forbidden}`"
        );
    }
}

#[test]
fn look_top_exile_one_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const REPLACE_LOOK_TOP_COUNT_CARD_OF_SEQUENCE",
        "pub(crate) fn parse_gain_life_equal_to_age_sentence",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "look-top exile-one parsing should capture count/owner/followup spans instead of walking token offsets"
    );
    for required in [
        "REPLACE_LOOK_TOP_THEN_EXILE_PATTERN.match_clause(clause)",
        "LexPattern::amount(",
        "\"count\"",
        "LexPattern::object(\"owner\", LexCaptureKind::UntilPhrase(&[\"library\"]))",
        "LexPattern::tail(\"followup\", LexCaptureKind::Rest)",
        "capture_clause_by_role(LexCaptureRole::Amount",
        "capture_clause_by_role(LexCaptureRole::Object",
        "capture_clause_by_role(LexCaptureRole::Tail",
        "REPLACE_LOOK_TOP_EXILE_FOLLOWUP_PATTERN.matches_prefix",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve look-top exile-one parsing through captures: missing `{required}`"
        );
    }
    for forbidden in [
        "REPLACE_LOOK_TOP_PREFIX_PATTERN",
        "REPLACE_EXILE_ONE_OF_LOOKED_PREFIX_PATTERN",
        "REPLACE_TOP_WORD_PATTERN",
        "REPLACE_LIBRARY_WORD_PATTERN",
        "let Some(top_idx)",
        "parse_number(&tokens[top_idx + 1..])",
        "let mut idx = top_idx + 1 + used_count",
        "idx += 1",
        "let owner_tokens = trim_commas(&tokens[idx..library_idx])",
        "let trimmed_tail_tokens = trim_commas(&tokens[library_idx + 1..])",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse look-top exile-one clauses with fixed token indexes `{forbidden}`"
        );
    }
}

#[test]
fn exile_then_return_same_object_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_exile_then_return_same_object_sentence",
        "pub(crate) fn parse_exile_up_to_one_each_target_type_sentence",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "exile-then-return parsing should capture clause spans instead of searching comma/then windows"
    );
    for required in [
        "REPLACE_EXILE_THEN_RETURN_SAME_OBJECT_PATTERN",
        "REPLACE_RETURN_WITH_COUNTER_ON_OBJECT_PATTERN",
        "LexPattern::action(\"exile_clause\", LexCaptureKind::UntilPhrase(&[\"then\"]))",
        "LexPattern::tail(\"return_clause\", LexCaptureKind::OneOrMoreWords)",
        "LexPattern::modifier(\"counter\", LexCaptureKind::UntilLastPhrase(&[\"on\"]))",
        "capture_clause(\"exile_clause\", clause)",
        "capture_clause(\"return_clause\", clause)",
        "capture_clause_by_role(LexCaptureRole::Modifier, return_clause)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should preserve exile/return parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "REPLACE_THEN_RETURN_MARKER_PATTERN",
        "REPLACE_WITH_WORD_PATTERN",
        "REPLACE_ON_WORD_PATTERN",
        "REPLACE_IT_OR_THEM_WORD_PATTERN",
        "let mut clause_tokens = tokens",
        "clause_tokens = &clause_tokens[1..]",
        "clause_tokens = &clause_tokens[2..]",
        "find_window_by(clause_tokens, 3",
        "token_slice_first_kind(window, TokenKind::Comma)",
        "token_slice_starts_with(&window[1..], &[\"then\", \"return\"])",
        "let second_clause = &clause_tokens[split_idx + 2..]",
        "let on_idx = with_idx + 1 + on_rel_idx",
        "let counter_tokens = trim_commas(&second_clause[with_idx + 1..on_idx])",
    ] {
        assert!(
            !parser.contains(forbidden) && !content.contains(forbidden),
            "{relative} should not parse exile/return clauses with fixed token windows `{forbidden}`"
        );
    }
}

#[test]
fn token_end_of_combat_recognizer_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const REPLACE_TOKEN_END_COMBAT_OBJECT_PHRASES",
        "pub(crate) fn parse_take_extra_turn_sentence",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "token end-of-combat routing should capture verb/object/timing spans instead of checking fixed word positions"
    );
    for required in [
        "REPLACE_EXILE_TOKEN_END_COMBAT_PATTERN",
        "REPLACE_SACRIFICE_TOKEN_END_COMBAT_PATTERN",
        "LexPattern::action(\"verb\", LexCaptureKind::OneOf(&[\"exile\"]))",
        "LexPattern::action(\"verb\", LexCaptureKind::OneOf(&[\"sacrifice\"]))",
        "LexPattern::object(",
        "\"object\"",
        "LexCaptureKind::OneOfPhrase(REPLACE_TOKEN_END_COMBAT_OBJECT_PHRASES)",
        "LexPattern::modifier(",
        "\"timing\"",
        "LexCaptureKind::OneOfPhrase(REPLACE_TOKEN_END_COMBAT_TIMING_PHRASES)",
        "REPLACE_EXILE_TOKEN_END_COMBAT_PATTERN",
        ".match_clause(clause)",
        "REPLACE_SACRIFICE_TOKEN_END_COMBAT_PATTERN",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve token end-of-combat routing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "REPLACE_THAT_THE_THOSE_WORD_PATTERN",
        "REPLACE_TOKEN_OR_TOKENS_WORD_PATTERN",
        "REPLACE_AT_WORD_PATTERN",
        "REPLACE_IT_WORD_PATTERN",
        "REPLACE_END_OF_COMBAT_TAIL_PATTERN",
        "matches_word_at(words, 1)",
        "matches_word_at(words, 2)",
        "let at_idx =",
        "has_end_of_combat_tail",
        "words.get(at_idx + 1..)",
        "REPLACE_TOKEN_END_COMBAT_OBJECT_PATTERN",
        "REPLACE_TOKEN_END_COMBAT_TIMING_PATTERN",
        ".matches_words(&object_clause",
        ".matches_words(&timing_clause",
    ] {
        assert!(
            !parser.contains(forbidden) && !content.contains(forbidden),
            "{relative} should not parse token end-of-combat routing with fixed word offsets `{forbidden}`"
        );
    }
}

#[test]
fn extra_turn_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const REPLACE_TAKE_EXTRA_TURN_YOU_PATTERN",
        "pub(crate) fn parse_additional_phase_sentence",
    );
    let parser_body = function_source(
        &content,
        "pub(crate) fn parse_take_extra_turn_sentence",
        "pub(crate) fn parse_additional_phase_sentence",
    );
    let actual = non_test_raw_text_check_literals(parser_body)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "extra-turn parsing should capture subject/action/anchor spans instead of matching raw word slices"
    );
    for required in [
        "REPLACE_TAKE_EXTRA_TURN_YOU_PATTERN",
        "REPLACE_TAKE_EXTRA_TURN_CHOSEN_PATTERN",
        "REPLACE_TAKE_EXTRA_TURN_THAT_AFTER_REFERENCED_PATTERN",
        "LexPattern::action(\"take\", LexCaptureKind::OneOf(&[\"take\"])",
        "LexPattern::subject(",
        "LexPattern::modifier(\"anchor\", LexCaptureKind::OneOfPhrase",
        "LexCaptureKind::OneOfPhrase(&[&[\"after\", \"this\", \"one\"]])",
        "REPLACE_TAKE_EXTRA_TURN_YOU_PATTERN",
        ".match_clause(clause)",
        "REPLACE_TAKE_EXTRA_TURN_CHOSEN_PATTERN",
        "subject_verb_extra_turn_after_turn",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve extra-turn parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs(tokens)",
        "words.as_slice()",
        "[\"take\", \"an\", \"extra\", \"turn\", \"after\", \"this\", \"one\"]",
        "[\"the\", \"chosen\", \"player\", \"takes\", \"an\", \"extra\", \"turn\"",
        "[\"after\", \"that\", \"turn\", \"that\", \"player\", \"takes\"",
        "REPLACE_EXTRA_TURN_AFTER_THIS_ONE_PATTERN",
        "matches_words(&anchor_clause.word_refs())",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
    ] {
        assert!(
            !parser_body.contains(forbidden),
            "{relative} should not parse extra-turn clauses through raw word-slice matching `{forbidden}`"
        );
    }
}

#[test]
fn additional_phase_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const REPLACE_ADDITIONAL_PHASE_EXISTENCE_PHRASES",
        "pub(crate) fn parse_destroy_or_exile_all_split_sentence",
    );
    let parser_body = function_source(
        &content,
        "pub(crate) fn parse_additional_phase_sentence",
        "pub(crate) fn parse_destroy_or_exile_all_split_sentence",
    );
    let actual = non_test_raw_text_check_literals(parser_body)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "additional-phase parsing should capture intro/count/phase/tail spans instead of selecting from whole-clause phrase buckets"
    );
    for required in [
        "REPLACE_ADDITIONAL_PHASE_PATTERN",
        "LexPattern::condition(",
        "\"intro\"",
        "LexCaptureKind::UntilAnyPhrase(REPLACE_ADDITIONAL_PHASE_EXISTENCE_PHRASES)",
        "LexPattern::any_phrase(REPLACE_ADDITIONAL_PHASE_EXISTENCE_PHRASES)",
        "LexPattern::amount(\"count\", LexCaptureKind::OneOf(&[\"an\", \"two\"]))",
        "LexPattern::object(",
        "\"phase\"",
        "LexPattern::tail(\"tail\", LexCaptureKind::Rest)",
        "REPLACE_ADDITIONAL_PHASE_AFTER_THIS_PHASE_PATTERN",
        "REPLACE_ADDITIONAL_PHASE_AFTER_THIS_COMBAT_PATTERN",
        "REPLACE_ADDITIONAL_PHASE_AFTER_THIS_MAIN_PATTERN",
        "REPLACE_ADDITIONAL_PHASE_IF_MAIN_PATTERN",
        "REPLACE_ADDITIONAL_PHASE_FOLLOWED_BY_MAIN_PATTERN",
        "REPLACE_ADDITIONAL_PHASE_AFTER_THIS_FOLLOWED_BY_MAIN_PATTERN",
        "REPLACE_ADDITIONAL_PHASE_ONE_COUNT_PATTERN",
        "REPLACE_ADDITIONAL_PHASE_TWO_COUNT_PATTERN",
        "REPLACE_ADDITIONAL_PHASE_COMBAT_SINGULAR_PATTERN",
        "REPLACE_ADDITIONAL_PHASE_COMBAT_PLURAL_PATTERN",
        "REPLACE_ADDITIONAL_PHASE_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Condition, clause)",
        "capture_clause_by_role(LexCaptureRole::Amount, clause)",
        "capture_clause_by_role(LexCaptureRole::Object, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "REPLACE_ADDITIONAL_PHASE_AFTER_THIS_PHASE_PATTERN.matches_clause(intro_clause)",
        "REPLACE_ADDITIONAL_PHASE_FOLLOWED_BY_MAIN_PATTERN.matches_clause(tail_clause)",
        "REPLACE_ADDITIONAL_PHASE_ONE_COUNT_PATTERN.matches_clause(count_clause)",
        "REPLACE_ADDITIONAL_PHASE_COMBAT_SINGULAR_PATTERN.matches_clause(phase_clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve additional-phase parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "parser_token_word_refs(tokens)",
        "REPLACE_ADDITIONAL_COMBAT_AFTER_THIS_PHASE_PATTERN",
        "REPLACE_TWO_ADDITIONAL_COMBATS_PATTERN",
        "REPLACE_ADDITIONAL_COMBAT_THEN_MAIN_PATTERN",
        "AdditionalPhasePattern",
        "matches_words(&words)",
        "intro_clause.trimmed().word_refs()",
        "tail_clause.trimmed().word_refs()",
        "count_clause.trimmed().word_refs()",
        "phase_clause.trimmed().word_refs()",
        "phase_words.as_slice()",
    ] {
        assert!(
            !parser_body.contains(forbidden),
            "{relative} should not parse additional phases through whole-clause phrase buckets `{forbidden}`"
        );
    }
    for retired in [
        "REPLACE_ADDITIONAL_COMBAT_AFTER_THIS_PHASE_PATTERN",
        "REPLACE_TWO_ADDITIONAL_COMBATS_PATTERN",
        "REPLACE_ADDITIONAL_COMBAT_THEN_MAIN_PATTERN",
    ] {
        assert!(
            !content.contains(retired),
            "{relative} should not keep retired additional-phase helper {retired}"
        );
    }
}

#[test]
fn counter_removed_pump_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const REPLACE_FOR_EACH_COUNTER_REMOVED_THIS_WAY_PATTERN",
        "pub(crate) fn is_exile_that_token_at_end_of_combat",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "counter-removed pump parsing should capture subject/action/modifier spans instead of splitting on comma and searching for gets"
    );
    for required in [
        "REPLACE_FOR_EACH_COUNTER_REMOVED_THIS_WAY_PATTERN",
        "LexPattern::phrase(&[\"for\", \"each\", \"counter\", \"removed\", \"this\", \"way\"])",
        "LexPattern::subject(",
        "LexCaptureKind::UntilAnyPhrase(&[&[\"get\"], &[\"gets\"]])",
        "LexPattern::action(\"action\", LexCaptureKind::OneOf(&[\"get\", \"gets\"]))",
        "LexPattern::modifier(\"modifier\", LexCaptureKind::WordCount(1))",
        "capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "capture_clause_by_role(LexCaptureRole::Modifier, clause)",
        "parse_subject(subject_clause.trimmed().tokens())",
        "render_token_slice(clause.tokens()).trim()",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve counter-removed pump parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "REPLACE_FOR_EACH_COUNTER_REMOVED_THIS_WAY_PREFIX_PATTERN",
        "REPLACE_GET_OR_GETS_WORD_PATTERN",
        "REPLACE_UNTIL_END_TURN_MARKER_PATTERN",
        "split_lexed_once_on_delimiter",
        "&tokens[6..]",
        "let gets_idx",
        "remainder[..gets_idx]",
        "remainder[gets_idx + 1..]",
        "let after_gets",
        "clause.text()",
    ] {
        assert!(
            !parser.contains(forbidden) && !content.contains(forbidden),
            "{relative} should not parse counter-removed pumps with fixed offsets `{forbidden}`"
        );
    }
}

#[test]
fn destroy_or_exile_all_split_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const REPLACE_DESTROY_ALL_SPLIT_PATTERN",
        "pub(crate) fn parse_exile_then_return_same_object_sentence",
    );
    let parser_body = function_source(
        &content,
        "pub(crate) fn parse_destroy_or_exile_all_split_sentence",
        "pub(crate) fn parse_exile_then_return_same_object_sentence",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "destroy/exile-all split parsing should capture verb and object-list spans instead of checking fixed prefix offsets"
    );
    for required in [
        "REPLACE_DESTROY_ALL_SPLIT_PATTERN",
        "REPLACE_EXILE_ALL_SPLIT_PATTERN",
        "LexPattern::action(\"verb\", LexCaptureKind::OneOf(&[\"destroy\"]))",
        "LexPattern::action(\"verb\", LexCaptureKind::OneOf(&[\"exile\"]))",
        "LexPattern::word(\"all\")",
        "LexPattern::object(\"objects\", LexCaptureKind::Rest)",
        "REPLACE_DESTROY_EXILE_ALL_OBJECT_LIST_PATTERN",
        "LexPattern::object(\"first_objects\", LexCaptureKind::UntilPhrase(&[\"and\"]))",
        "LexPattern::word(\"and\")",
        "LexPattern::tail(\"remaining_objects\", LexCaptureKind::OneOrMoreWords)",
        "REPLACE_DESTROY_EXILE_ALL_OBJECT_LIST_PATTERN",
        ".match_clause(objects_clause.trimmed())",
        "capture_clause_by_role(LexCaptureRole::Object, clause)",
        "objects_clause.trimmed_and_comma_segments()",
        "render_token_slice(clause.tokens()).trim()",
        "Verb::Destroy",
        "Verb::Exile",
        "destroy_exile_split_has_exception(clause)",
        "destroy_exile_split_is_temporary_exile_until_leaves_battlefield(clause, verb)",
        "destroy_exile_split_is_multi_zone_card_exile(clause)",
        "REPLACE_EXILE_ALL_CARDS_FROM_ZONES_PATTERN",
        "LexPattern::tail(\"zones\", LexCaptureKind::Rest)",
        "REPLACE_ZONE_LIST_PATTERN",
        "LexPattern::capture(\"first_zone\", LexCaptureKind::UntilPhrase(&[\"and\"]))",
        "LexPattern::tail(\"second_zone\", LexCaptureKind::OneOrMoreWords)",
        "REPLACE_HAND_ZONE_PATTERN",
        "REPLACE_GRAVEYARD_ZONE_PATTERN",
        "REPLACE_ZONE_LIST_PATTERN.match_clause(zones_clause)",
        "capture_clause(\"first_zone\", zones_clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
    ] {
        assert!(
            parser.contains(required) || content.contains(required),
            "{relative} should preserve destroy/exile-all split parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "REPLACE_DESTROY_WORD_PATTERN",
        "REPLACE_ALL_WORD_PATTERN",
        "matches_word_at(&words, 0)",
        "matches_word_at(&words, 1)",
        "for token in &tokens[2..]",
        "let verb = if",
        "verb_clause.trimmed().word_refs()",
        "capture_clause_by_role(LexCaptureRole::Action, clause)",
        "REPLACE_DESTROY_EXILE_ALL_SPLIT_PATTERN",
        "let object_words = objects_clause.word_refs()",
        "let words = clause.word_refs()",
        "REPLACE_EXILE_ALL_CARDS_FROM_PREFIX_PATTERN",
        "REPLACE_EXCEPT_MARKER_PATTERN.matches_words(&words)",
        "REPLACE_HAND_MARKER_PATTERN.matches_words(&words)",
        "REPLACE_GRAVEYARD_MARKER_PATTERN.matches_words(&words)",
        "REPLACE_HAND_MARKER_PATTERN",
        "REPLACE_GRAVEYARD_MARKER_PATTERN",
        "let zone_words = zones_clause.word_refs()",
        "REPLACE_AND_MARKER_PATTERN",
        "words.iter().any(|word| *word == \"until\")",
        "words.ends_with(&[\"leaves\", \"the\", \"battlefield\"])",
        "let mut raw_segments",
        "let mut current",
        "clause.text()",
    ] {
        assert!(
            !parser_body.contains(forbidden)
                && !content.contains("REPLACE_DESTROY_WORD_PATTERN")
                && !content.contains("REPLACE_ALL_WORD_PATTERN")
                && !content.contains("REPLACE_DESTROY_EXILE_ALL_SPLIT_PATTERN"),
            "{relative} should not parse destroy/exile-all split prefixes with fixed offsets `{forbidden}`"
        );
    }
}

#[test]
fn monstrosity_parser_uses_lex_pattern_amount_capture() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const REPLACE_MONSTROSITY_PATTERN",
        "pub(crate) fn parse_for_each_counter_removed_sentence",
    );
    let parser_body = function_source(
        &content,
        "pub(crate) fn parse_monstrosity_sentence",
        "pub(crate) fn parse_for_each_counter_removed_sentence",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "monstrosity parsing should capture the amount span instead of slicing after the keyword"
    );
    for required in [
        "REPLACE_MONSTROSITY_PATTERN",
        "LexPattern::word(\"monstrosity\")",
        "LexPattern::amount(\"amount\", LexCaptureKind::Rest)",
        "REPLACE_MONSTROSITY_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Amount, clause)",
        "parse_value(amount_clause.trimmed().tokens())",
        "render_token_slice(clause.tokens()).trim()",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve monstrosity parsing through named amount captures: missing `{required}`"
        );
    }
    for forbidden in [
        "REPLACE_MONSTROSITY_WORD_PATTERN",
        "matches_word_at(&words, 0)",
        "let amount_tokens = &tokens[1..]",
        "parse_value(amount_tokens)",
        "clause.text()",
    ] {
        assert!(
            !parser_body.contains(forbidden)
                && !content.contains("REPLACE_MONSTROSITY_WORD_PATTERN"),
            "{relative} should not parse monstrosity with fixed keyword offsets `{forbidden}`"
        );
    }
}

#[test]
fn exile_up_to_one_each_target_type_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const REPLACE_EXILE_UP_TO_ONE_EACH_TARGET_TYPE_PATTERN",
        "pub(crate) fn parse_look_at_hand_sentence",
    );
    let parser_body = function_source(
        &content,
        "pub(crate) fn parse_exile_up_to_one_each_target_type_sentence",
        "pub(crate) fn parse_look_at_hand_sentence",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "exile repeated target-type parsing should capture the target-clause tail instead of slicing after the verb"
    );
    for required in [
        "REPLACE_EXILE_UP_TO_ONE_EACH_TARGET_TYPE_PATTERN",
        "LexPattern::word(\"exile\")",
        "LexPattern::object(\"target_clauses\", LexCaptureKind::Rest)",
        "REPLACE_EXILE_UP_TO_ONE_EACH_TARGET_TYPE_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Object, clause)",
        "let target_tokens = target_clauses.trimmed().tokens()",
        "replace_up_to_one_target_segments(target_clauses.trimmed())",
        ".trimmed_and_comma_segments()",
        "split_lexed_slices_on_or(segment.tokens())",
        "render_token_slice(clause.tokens()).trim()",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve repeated target-type parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "REPLACE_EXILE_WORD_PATTERN",
        "words.len() < 6",
        "words[..1]",
        "tokens.get(1..)",
        "for token in &tokens[1..]",
        "for token in target_tokens",
        "let mut raw_segments",
        "let mut current",
        "clause.text()",
    ] {
        assert!(
            !parser_body.contains(forbidden) && !content.contains("REPLACE_EXILE_WORD_PATTERN"),
            "{relative} should not parse repeated target-type exile clauses with fixed verb offsets `{forbidden}`"
        );
    }
}

#[test]
fn look_at_hand_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const REPLACE_LOOK_HAND_PATTERN",
        "const REPLACE_LOOK_TOP_COUNT_CARD_OF_SEQUENCE",
    );
    let parser_body = function_source(
        &content,
        "pub(crate) fn parse_look_at_hand_sentence",
        "pub(crate) fn parse_look_at_top_then_exile_one_sentence",
    );
    let actual = non_test_raw_text_check_literals(parser_body)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "look-at-hand parsing should capture player/followup spans instead of matching whole-clause phrase buckets"
    );
    for required in [
        "REPLACE_LOOK_HAND_PATTERN",
        "LexPattern::phrase(&[\"look\", \"at\"])",
        "LexPattern::object(\"player\", LexCaptureKind::UntilPhrase(&[\"hand\"]))",
        "LexPattern::word(\"hand\")",
        "LexPattern::tail(\"followup\", LexCaptureKind::Rest)",
        "REPLACE_LOOK_HAND_TARGET_PLAYER_PATTERN",
        "REPLACE_LOOK_HAND_TARGET_OPPONENT_PATTERN",
        "REPLACE_LOOK_HAND_OPPONENT_PATTERN",
        "REPLACE_LOOK_HAND_ITERATED_PLAYER_PATTERN",
        "LexCaptureKind::OneOfPhrase(REPLACE_LOOK_HAND_TARGET_PLAYER_PHRASES)",
        "LexCaptureKind::OneOfPhrase(REPLACE_LOOK_HAND_TARGET_OPPONENT_PHRASES)",
        "LexCaptureKind::OneOfPhrase(REPLACE_LOOK_HAND_OPPONENT_PHRASES)",
        "LexCaptureKind::OneOfPhrase(REPLACE_LOOK_HAND_ITERATED_PLAYER_PHRASES)",
        "REPLACE_LOOK_HAND_CHOOSE_NAME_PATTERN",
        "LexPattern::action(\"choose\", LexCaptureKind::OneOf(&[\"choose\"]))",
        "LexPattern::object(",
        "\"name\"",
        "LexCaptureKind::OneOfPhrase(REPLACE_LOOK_HAND_CHOOSE_NAME_OBJECT_PHRASES)",
        "REPLACE_LOOK_HAND_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Object, clause)",
        "capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "REPLACE_LOOK_HAND_CHOOSE_NAME_PATTERN",
        ".match_clause(followup_clause)",
    ] {
        assert!(
            parser.contains(required) || parser_body.contains(required),
            "{relative} should preserve look-at-hand parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs(tokens)",
        "REPLACE_LOOK_OPPONENT_HAND_THEN_CHOOSE_NAME_PATTERN",
        "REPLACE_LOOK_TARGET_PLAYER_HAND_PATTERN",
        "REPLACE_LOOK_TARGET_OPPONENT_HAND_PATTERN",
        "REPLACE_LOOK_OPPONENT_HAND_PATTERN",
        "REPLACE_LOOK_THAT_PLAYER_HAND_PATTERN",
        "matches_words(&words)",
        "REPLACE_LOOK_HAND_CHOOSE_NAME_PATTERN.matches_words",
        "followup_words",
        "player_clause.trimmed().word_refs().as_slice()",
    ] {
        assert!(
            !parser_body.contains(forbidden),
            "{relative} should not parse look-at-hand clauses through whole-clause phrase checks `{forbidden}`"
        );
    }
    for retired in [
        "REPLACE_LOOK_OPPONENT_HAND_THEN_CHOOSE_NAME_PATTERN",
        "REPLACE_LOOK_TARGET_PLAYER_HAND_PATTERN",
        "REPLACE_LOOK_TARGET_OPPONENT_HAND_PATTERN",
        "REPLACE_LOOK_OPPONENT_HAND_PATTERN",
        "REPLACE_LOOK_THAT_PLAYER_HAND_PATTERN",
    ] {
        assert!(
            !content.contains(retired),
            "{relative} should not keep retired look-at-hand helper {retired}"
        );
    }
}

#[test]
fn voted_with_you_scry_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "const REPLACE_VOTED_WITH_YOU_SCRY_PATTERN",
        "const REPLACE_TOKEN_END_COMBAT_OBJECT_PHRASES",
    );
    let parser_body = function_source(
        &content,
        "pub(crate) fn parse_you_and_each_opponent_voted_with_you_sentence",
        "#[cfg(test)]",
    );
    let actual = non_test_raw_text_check_literals(parser_body)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "voted-with-you scry parsing should capture the scry count instead of slicing after a fixed prefix length"
    );
    for required in [
        "REPLACE_VOTED_WITH_YOU_SCRY_PATTERN",
        "LexPattern::subject(",
        "\"voters\"",
        "LexPattern::capture(\"may\", LexCaptureKind::OneOf(&[\"may\"]))",
        "LexPattern::action(\"scry\", LexCaptureKind::OneOf(&[\"scry\"]))",
        "LexPattern::amount(\"count\", LexCaptureKind::Rest)",
        "REPLACE_VOTED_WITH_YOU_SCRY_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Amount, clause)",
        "parse_value(count_clause.trimmed().tokens())",
        "render_token_slice(clause.tokens()).trim()",
    ] {
        assert!(
            parser.contains(required) || parser_body.contains(required),
            "{relative} should preserve voted-with-you scry parsing through named captures: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs(tokens)",
        "REPLACE_VOTED_WITH_YOU_SCRY_PREFIX_PATTERN",
        "REPLACE_VOTED_WITH_YOU_SCRY_PREFIX_LEN",
        "words.len() <",
        "let scry_index",
        "tokens[(scry_index + 1)..]",
        "matches_words(&words)",
        "clause.text()",
    ] {
        assert!(
            !parser_body.contains(forbidden),
            "{relative} should not parse voted-with-you scry clauses through fixed prefix slicing `{forbidden}`"
        );
    }
    for retired in [
        "REPLACE_VOTED_WITH_YOU_SCRY_PREFIX_PATTERN",
        "REPLACE_VOTED_WITH_YOU_SCRY_PREFIX_LEN",
    ] {
        assert!(
            !content.contains(retired),
            "{relative} should not keep retired voted-with-you scry helper {retired}"
        );
    }
}

#[test]
fn line_lowering_self_replacement_recognizers_use_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/line_lowering.rs";
    let content = read_repo_file(&root, relative);
    let lower = function_source(
        &content,
        "fn lower_statement_chunk",
        "fn lower_additional_cost_chunk",
    );
    assert!(
        lower.contains("let normalized_tokens =")
            && lower.contains("back_for_seconds_style_replacement_program(&compiled, &normalized_tokens)")
            && lower.contains("attach_morbid_search_to_battlefield_self_replacement(&mut builder, &normalized_tokens)")
            && lower.contains("tokens_start_with_if(&normalized_tokens)"),
        "{relative} should classify self-replacement followups from a lexed normalized token stream"
    );
    let leading_conditional_helper = function_source(
        &content,
        "fn tokens_start_with_if",
        "fn lower_additional_cost_chunk",
    );
    assert!(
        leading_conditional_helper.contains("IF_PREFIX_PATTERN.matches_words")
            && leading_conditional_helper.contains("parser_token_word_refs(tokens)"),
        "{relative} should classify leading conditional self-replacement followups through token clause shapes"
    );

    for helper in [
        "fn tokens_mention_morbid_search_to_battlefield_replacement",
        "fn tokens_mention_bargained_return_to_battlefield_replacement",
        "fn tokens_mention_kicked_count_override_replacement",
        "fn tokens_mention_kicked_multi_zone_to_battlefield_followup",
        "fn tokens_mention_clash_win_top_replacement",
    ] {
        let source = function_source(&content, helper, "\n}\n\n");
        assert!(
            source.contains(".matches_words(") && source.contains("parser_token_word_refs(tokens)"),
            "{relative} should implement {helper} with token-backed clause shapes"
        );
        for forbidden in [
            "text_contains_word_phrase",
            "normalized_line",
            "to_ascii_lowercase",
            "lex_line(text",
        ] {
            assert!(
                !source.contains(forbidden),
                "{relative} should not implement {helper} with raw text logic `{forbidden}`"
            );
        }
    }

    for forbidden in [
        "text_mentions_morbid_search_to_battlefield_replacement(normalized_line)",
        "text_mentions_bargained_return_to_battlefield_replacement(normalized_line)",
        "text_mentions_kicked_count_override_replacement(normalized_line)",
        "text_mentions_kicked_multi_zone_search_to_battlefield_replacement(normalized_line)",
        "text_mentions_kicked_multi_zone_to_battlefield_followup(normalized_line)",
        "text_mentions_clash_win_top_replacement(normalized_line)",
        "text_starts_with_if(normalized_line.as_str())",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not classify self-replacement followups from rendered text: {forbidden}"
        );
    }
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
        "if tokens_match_blocks_or_blocked_first_strike",
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

    let lower_mod_relative = "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/mod.rs";
    let lower_mod = read_repo_file(&root, lower_mod_relative);
    let block_first_strike_helper = function_source(
        &lower_mod,
        "fn tokens_match_blocks_or_blocked_first_strike",
        "fn tokens_start_with_partner_dash_label",
    );
    assert!(
        block_first_strike_helper.contains("BLOCKS_OR_BECOMES_BLOCKED_FIRST_STRIKE_PATTERN")
            && block_first_strike_helper.contains("matches_words(&token_word_refs(tokens))"),
        "{lower_mod_relative} should classify blocks/becomes-blocked first-strike lines through token clause shapes"
    );
    for forbidden in [
        "lex_line(",
        "trim_end_matches",
        "text_matches_blocks_or_blocked_first_strike",
    ] {
        assert!(
            !block_first_strike_helper.contains(forbidden),
            "{lower_mod_relative} should not classify blocks/becomes-blocked first-strike lines through raw text helper `{forbidden}`"
        );
    }
}

#[test]
fn prevent_all_damage_clause_parser_uses_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/clause_pattern_helpers.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_prevent_all_damage_clause",
        "pub(crate) fn parse_can_attack_as_though_no_defender_clause",
    );
    let forbidden_fragments = [
        "clause.starts_with(CLAUSE_PREVENT_ALL_DAMAGE",
        "source_clause.ends_with(CLAUSE_SOURCES_SUFFIX)",
        "CLAUSE_THIS_TURN_PATTERN\n            .matches_words(&clause_words",
    ];

    for forbidden in forbidden_fragments {
        assert!(
            !parser.contains(forbidden),
            "{relative} should classify prevent-all-damage variants through ClauseShape, not raw clause branch `{forbidden}`"
        );
    }
    assert!(
        parser.contains("classify_prevent_all_damage_clause"),
        "{relative} should keep prevent-all-damage variant routing centralized"
    );
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

#[test]
fn keyword_registry_additional_cost_lowering_uses_lexed_tail_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_registry.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(super) fn lower_additional_cost",
        "pub(super) fn lower_additional_cost_choice",
    );

    assert!(
        helper.contains("additional_cost_tail_tokens(tokens)"),
        "{relative} should lower additional-cost effects from parse tokens"
    );
    for forbidden in [
        "additional_cost_tail_tokens_from_text",
        "text.split_once(',')",
        "line.text.as_str()",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not recover additional-cost tails by splitting rendered text with `{forbidden}`"
        );
    }
}

#[test]
fn sneak_keyword_support_gate_uses_token_shapes_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_registry.rs";
    let content = read_repo_file(&root, relative);
    let lowering = function_source(
        &content,
        "if SNEAK_KEYWORD_PREFIX_PATTERN.matches_words",
        "if BLITZ_FROM_GRAVEYARD_MARKER_PATTERN.matches_words",
    );
    let helper = function_source(
        &content,
        "fn is_supported_spell_sneak_line",
        "pub(super) fn lower_bestow",
    );
    let helper_raw_checks = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::new();

    assert_eq!(
        helper_raw_checks, expected,
        "{relative} should classify supported Sneak reminder forms through parser tokens and grammar shapes, not raw lowercase text probes"
    );
    assert!(
        lowering.contains("is_supported_spell_sneak_line(&line.full_parse_tokens)"),
        "{relative} should pass the Sneak support gate the full keyword parse tokens"
    );
    for required in [
        "parser_token_word_refs(tokens)",
        "SNEAK_SPELL_FORM_PATTERN.matches_words(&words)",
        "SNEAK_PERMANENT_FORM_PATTERN.matches_words(&words)",
        "SNEAK_KEYWORD_PREFIX_PATTERN.matches_words",
        "full_parse_tokens",
    ] {
        assert!(
            content.contains(required),
            "{relative} should preserve Sneak classification through token-backed ClauseShape helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "is_supported_spell_sneak_line(line.info.raw_line.as_str())",
        "fn is_supported_spell_sneak_line(raw_line: &str)",
        "raw_line.to_ascii_lowercase()",
        ".contains(\"you may cast this spell for\")",
        ".contains(\"enters tapped and attacking\")",
        "token_slice_first_is(tokens, \"sneak\")",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not classify Sneak through raw/text-specific branch `{forbidden}`"
        );
    }
}

#[test]
fn gift_keyword_registry_and_lowering_use_parse_tokens() {
    let root = workspace_root();
    let registry_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_registry.rs";
    let registry_content = read_repo_file(&root, registry_relative);
    let matcher = function_source(
        &registry_content,
        "pub(super) fn matches_gift",
        "pub(super) fn matches_warp",
    );
    assert!(
        matcher.contains("is_standard_gift_keyword_tokens_lexed(tokens)"),
        "{registry_relative} should classify gift keyword lines from the dispatch token slice"
    );
    for forbidden in [
        "line.info.raw_line.as_str()",
        "is_standard_gift_keyword_line",
        "lex_line(raw_line",
    ] {
        assert!(
            !matcher.contains(forbidden),
            "{registry_relative} should not re-lex gift keyword lines from raw text with `{forbidden}`"
        );
    }

    let lowering_relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let lowering_content = read_repo_file(&root, lowering_relative);
    let lowering = function_source(
        &lowering_content,
        "pub(crate) fn lower_gift_keyword_line",
        "pub(crate) fn lower_keyword_special_cases",
    );
    assert!(
        lowering.contains("standard_gift_variant_tokens(&line.parse_tokens)")
            && lowering.contains("standard_gift_timing_tokens(&line.parse_tokens"),
        "{lowering_relative} should lower gift keyword variants and timing from stored parse tokens"
    );
    for forbidden in [
        "standard_gift_followup(line.info.raw_line.as_str())",
        "standard_gift_timing(line.info.raw_line.as_str())",
        "str_split_once_char(text.trim(), '(')",
        "to_ascii_lowercase()",
    ] {
        assert!(
            !lowering.contains(forbidden),
            "{lowering_relative} should not classify gift keyword variants through raw text branch `{forbidden}`"
        );
    }
}

#[test]
fn triggered_label_source_selection_uses_lexed_dash_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/document/mod.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn trigger_presentation_label_from_line_tokens",
        "fn is_nonkeyword_choice_labeled_line",
    );

    assert!(
        helper.contains("split_label_prefix_lexed")
            && helper.contains("line_starts_with_trigger_intro_tokens"),
        "{relative} should derive trigger presentation labels from document CST tokens"
    );
    for forbidden in [
        ".split_once('—')",
        "split_once(\" - \")",
        "label.contains('.')",
        "label.contains(':')",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not split or validate trigger labels with raw string branch `{forbidden}`"
        );
    }

    let lowering_relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let lowering = read_repo_file(&root, lowering_relative);
    assert!(
        !lowering.contains("presentation_label_from_raw_trigger_line")
            && !lowering.contains(".or_else(|| presentation_label_from_raw"),
        "{lowering_relative} should consume triggered presentation labels from CST/IR, not re-read raw oracle text"
    );
}

#[test]
fn triggered_intro_surface_selection_uses_full_parse_tokens() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn lower_rewrite_triggered_to_chunk_impl",
        "pub(super) fn infer_rewrite_triggered_functional_zones",
    );

    assert!(
        helper.contains("tokens_start_with_trigger_intro_surface(full_parse_tokens)"),
        "{relative} should decide parsed trigger-intro surface from CST parse tokens"
    );
    assert!(
        helper.contains("let source_text_tokens =")
            && helper.contains("tokens_start_with_trigger_intro_surface(&source_text_tokens)")
            && helper.contains(
                "trigger_surface_tokens = if trigger_surface_text == source_text.as_str()"
            ),
        "{relative} should tokenize the chosen trigger source surface once and reuse that token stream"
    );
    assert!(
        !helper.contains("has_trigger_intro_surface")
            && !helper.contains("text_starts_with_trigger_intro_surface")
            && !helper.contains("!has_trigger_intro_surface(&line.info.raw_line)"),
        "{relative} should not decide trigger-intro surface through a raw-text wrapper"
    );

    let source_selector = function_source(
        &content,
        "fn raw_preserves_triggered_source",
        "pub(crate) fn lower_rewrite_statement_token_groups_to_chunks",
    );
    assert!(
        source_selector.contains("normalized_triggered_source_words")
            && source_selector.contains("parser_token_word_refs")
            && source_selector.contains("strip_trigger_cap_suffix_from_words"),
        "{relative} should compare triggered source surfaces through token words"
    );
    for forbidden in [
        "normalize_triggered_source_text",
        "strip_trigger_cap_suffix_from_normalized_source",
        "to_ascii_lowercase",
        "replace(char::is_whitespace",
        "strip_suffix(suffix)",
    ] {
        assert!(
            !source_selector.contains(forbidden),
            "{relative} should not compare triggered source surfaces with raw text normalization `{forbidden}`"
        );
    }
}

#[test]
fn triggered_cap_flow_uses_cst_ir_field_only() {
    let root = workspace_root();
    let cst_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_cst_parsing.rs";
    let cst_content = read_repo_file(&root, cst_relative);
    let parser = function_source(
        &cst_content,
        "pub(super) fn parse_triggered_line_cst",
        "pub(super) fn parse_static_line_cst",
    );
    assert!(
        parser.contains("strip_trailing_trigger_cap_suffix_tokens(&line.tokens)")
            && parser.contains("trailing_cap"),
        "{cst_relative} should derive triggered cap limits while building CST from lexed tokens"
    );

    let lowering_relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let lowering = read_repo_file(&root, lowering_relative);
    let helper = function_source(
        &lowering,
        "fn lower_rewrite_triggered_to_chunk_impl",
        "fn combat_death_blocked_damage_amount_lexed",
    );
    assert!(
        helper.contains("let inferred_max_triggers_per_turn = line.max_triggers_per_turn;"),
        "{lowering_relative} should consume trigger cap limits from CST/IR"
    );
    for forbidden in [
        "infer_trigger_cap_from_text",
        "trigger_cap_surface_from_text",
        ".or(infer_trigger_cap_from_text",
        "max_triggers_per_turn\n        .or(",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{lowering_relative} should not infer trigger caps by re-reading source text with `{forbidden}`"
        );
    }
}

#[test]
fn chosen_option_label_flow_uses_cst_ir_field_only() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/rewrite_sentence_grouping.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(crate) fn effective_chosen_option_label",
        "#[cfg(test)]",
    );

    assert!(
        helper.contains("chosen_option_label") && !helper.contains("raw_line"),
        "{relative} should resolve chosen-option labels from CST/IR fields, not raw oracle text"
    );

    for lowering_relative in [
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs",
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/rewrite_text_helpers.rs",
    ] {
        let lowering = read_repo_file(&root, lowering_relative);
        assert!(
            !lowering.contains("effective_chosen_option_label(&line.info.raw_line"),
            "{lowering_relative} should not pass raw oracle text into chosen-option label resolution"
        );
    }
}

#[test]
fn parser_semantic_ability_marker_detection_uses_token_kinds() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn looks_like_ability_word_marker_tokens",
        "fn is_draft_rule_static_line",
    );

    assert!(
        helper.contains("TokenKind::Period") && helper.contains("token_word_refs(parse_tokens)"),
        "{relative} should classify bare ability markers from lexed token kinds and token words"
    );
    for forbidden in [
        "contains('.')",
        "contains(':')",
        "contains('—')",
        "contains('-')",
        "contains(',')",
        "contains(';')",
        ".split_whitespace()",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not classify ability markers by scanning rendered text with `{forbidden}`"
        );
    }

    let lower_static = function_source(
        &content,
        "fn lower_rewrite_static_to_chunk_impl",
        "fn looks_like_ability_word_marker_tokens",
    );
    assert!(
        lower_static.contains("token.kind == TokenKind::Period")
            && lower_static
                .contains("StaticAbility::keyword_marker(render_token_slice(parse_tokens)"),
        "{relative} should use parse tokens for static fallback punctuation and marker display"
    );
    for forbidden in [
        "str_find(line.text.as_str(), \".\")",
        "StaticAbility::keyword_marker(line.text",
    ] {
        assert!(
            !lower_static.contains(forbidden),
            "{relative} should not use raw static text in fallback marker handling: {forbidden}"
        );
    }
}

#[test]
fn parser_semantic_additional_land_play_static_count_uses_token_words() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn parse_additional_land_play_static_count_tokens",
        "#[cfg(test)]\npub(crate) fn lower_rewrite_keyword_to_chunk",
    );

    assert!(
        helper.contains("ADDITIONAL_LAND_PLAY_STATIC_PREFIX_PATTERN")
            && helper.contains("parser_token_word_refs(parse_tokens)")
            && helper.contains("parse_cardinal_words(&words[3..])")
            && helper.contains("ADDITIONAL_LAND_PLAY_STATIC_TAIL_PATTERN"),
        "{relative} should parse static additional-land-play counts from token words and clause shapes"
    );
    for forbidden in [
        "split_whitespace()",
        "trim_end_matches('.')",
        "words[0]",
        "words[1]",
        "words[2]",
        "words[4]",
        "words[5]",
        "words[6..]",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not parse additional-land-play static counts by indexing rendered text with `{forbidden}`"
        );
    }
}

#[test]
fn parser_semantic_partner_parenthetical_trims_use_token_kinds() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn try_lower_partner_with_tokens",
        "pub(crate) fn try_lower_optional_cost_with_cast_trigger",
    );

    assert!(
        helper.contains("partner_with_name_from_tokens")
            && helper.contains("TokenKind::LParen")
            && helper.contains("render_tokens_before_reminder_or_period"),
        "{relative} should trim partner parentheticals from parse tokens using lexed token kinds"
    );
    for forbidden in [
        "try_lower_partner_with_text",
        "raw_line: &str",
        "normalized_text: &str",
        "lex_line(raw_line",
        "lex_line(normalized_text",
        "partner_with_name_from_text",
        "split_once('(')",
        "str_split_once_char",
        "trim_end_matches('.')",
        "\"partner with \".len()",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not trim partner parentheticals with raw string branch `{forbidden}`"
        );
    }
}

#[test]
fn parser_semantic_partner_static_display_uses_parse_tokens() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn lower_rewrite_static_to_chunk_impl",
        "fn parse_additional_land_play_static_count_tokens",
    );

    assert!(
        helper.contains("tokens_start_with_partner_dash_label(&line.parse_tokens)")
            && helper.contains("render_tokens_before_reminder_or_period(&line.parse_tokens)"),
        "{relative} should derive Partner display text from stored static parse tokens"
    );
    let keyword_partner_helper = function_source(
        &content,
        "fn try_lower_partner_variant_keyword",
        "fn try_lower_optional_cost_with_cast_trigger",
    );
    for required in [
        "line.full_parse_tokens.as_slice()",
        "visible_partner_label_is_variant_tokens(visible_tokens)",
        "tokens_before_reminder_or_period(tokens)",
        "render_partner_label_token_slice(tokens_before_reminder_or_period(tokens))",
        "CHARACTER_SELECT_PREFIX_PATTERN.matches_words(&words)",
        "PARTNER_WITH_PATTERN.matches_words(&words)",
    ] {
        assert!(
            keyword_partner_helper.contains(required),
            "{relative} should classify Partner variant labels through full parse tokens and grammar shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let raw = line.info.raw_line.trim()",
        "lex_line(raw",
        "render_source_before_reminder_or_period",
        "visible_partner_label_is_variant(text: &str)",
        "visible_partner_variant_label(text: &str)",
        "text.trim().to_ascii_lowercase()",
        "split_once('(')",
        "split_whitespace()",
    ] {
        assert!(
            !helper.contains(forbidden) && !keyword_partner_helper.contains(forbidden),
            "{relative} should not re-lex raw oracle text for Partner display with `{forbidden}`"
        );
    }
}

#[test]
fn parser_semantic_static_special_cases_use_parse_tokens() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn lower_rewrite_static_to_chunk_impl",
        "fn looks_like_ability_word_marker_tokens",
    );

    for required in [
        "let parse_words = token_word_refs(parse_tokens)",
        "KRRRIK_BLACK_MANA_LIFE_PAYMENT_STATIC_PATTERN.matches_words(&parse_words)",
        "is_minimum_spell_total_mana_three_line_lexed(parse_tokens)",
        "is_players_cant_pay_life_or_sacrifice_line_lexed(parse_tokens)",
        "BOAST_TWICE_STATIC_PATTERN.matches_words(&parse_words)",
        "is_first_equip_cost_alternative_lowering_line(parse_tokens)",
        "EQUIP_ABILITIES_INSTANT_SPEED_PATTERN.matches_words(&parse_words)",
        "VOTE_ADDITIONAL_TIME_PATTERN.matches_words(&parse_words)",
        "VOTE_ADDITIONAL_VOTE_PATTERN.matches_words(&parse_words)",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should lower static special cases from parse tokens: missing `{required}`"
        );
    }

    for forbidden in [
        "matches!(\n        line.text.as_str()",
        "line.text\n        == \"as long as trinisphere",
        "line.text\n        == \"players can't pay life",
        "line.text\n        == \"creatures you control can boast twice",
        "is_first_equip_cost_alternative_lowering_line(&line.text)",
        "line.text == \"you may activate equip abilities",
        "line.text == \"while voting",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not route static special cases through rendered text branch `{forbidden}`"
        );
    }
}

#[test]
fn parser_semantic_keyword_action_probe_skip_uses_parse_tokens() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn lower_rewrite_static_to_chunk_impl",
        "fn looks_like_ability_word_marker_tokens",
    );

    assert!(
        helper.contains("should_skip_keyword_action_static_probe_tokens(parse_tokens)"),
        "{relative} should decide keyword-action static probe skips from parse tokens"
    );

    let skip_helper = function_source(
        &content,
        "fn should_skip_keyword_action_static_probe_tokens",
        "#[cfg(test)]",
    );
    assert!(
        skip_helper.contains("CANT_BE_BLOCKED_LINE_PATTERN.matches_words(&words)")
            && skip_helper.contains("THIS_OR_IT_PREFIX_PATTERN.matches_words(&words)")
            && skip_helper.contains("token_word_refs(tokens)"),
        "{relative} should classify unqualified can't-be-blocked probe skips through token clause shapes"
    );

    for forbidden in [
        "should_skip_keyword_action_static_probe(&line.text)",
        "text_is_unqualified_cant_be_blocked",
        "lex_line(text.trim_end_matches('.')",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not classify keyword-action probe skips through rendered text `{forbidden}`"
        );
    }
}

#[test]
fn parser_semantic_hideaway_special_case_uses_token_words() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn try_lower_hideaway_keyword",
        "fn hideaway_line_ast",
    );

    assert!(
        helper.contains("try_lower_hideaway_tokens(parse_tokens")
            && helper.contains("parser_token_word_refs(parse_tokens)")
            && helper.contains("render_token_slice(parse_tokens)"),
        "{relative} should lower hideaway special cases and diagnostics from parse tokens"
    );
    for forbidden in [
        "try_lower_hideaway_tokens(parse_tokens, line.info.raw_line.as_str())",
        "raw_line: &str",
        "try_lower_hideaway_text",
        "split_whitespace()",
        "trim_matches",
        "eq_ignore_ascii_case",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not lower hideaway by normalizing rendered text with `{forbidden}`"
        );
    }
}

#[test]
fn self_enters_counter_static_parser_uses_token_shapes_for_adamant_branch() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn parse_self_enters_with_x_counters_static_chunk",
        "pub(crate) fn lower_rewrite_triggered_to_chunk",
    );

    assert!(
        helper.contains("SELF_ENTERS_WITH_SINGLE_PLUS_ONE_COUNTER_PATTERN"),
        "{relative} should classify single-counter ETB effects through ClauseShape"
    );
    assert!(
        helper.contains("tokens_start_with_self_x_counter_etb(tokens)")
            && helper.contains("revealed_cards_total_mana_value_x_value_tokens(tokens)"),
        "{relative} should classify variable ETB counter effects from lexed tokens"
    );
    assert!(
        helper.contains("split_once_at_comma_tokens"),
        "{relative} should split adamant ETB condition/effect clauses on lexed comma tokens"
    );
    let variable_prefix_helper = function_source(
        &content,
        "fn tokens_start_with_self_x_counter_etb",
        "fn revealed_cards_total_mana_value_x_value_tokens",
    );
    assert!(
        variable_prefix_helper
            .contains("SELF_X_COUNTER_ETB_PATTERN.matches_words(&token_word_refs(tokens))"),
        "{relative} should classify variable ETB counter prefixes through token clause shapes"
    );
    let revealed_value_helper = function_source(
        &content,
        "fn revealed_cards_total_mana_value_x_value_tokens",
        "fn single_plus_one_counter_enters_static_chunk",
    );
    assert!(
        revealed_value_helper.contains("REVEALED_CARDS_TOTAL_MANA_VALUE_X_PATTERN")
            && revealed_value_helper.contains("matches_words(&token_word_refs(tokens))")
            && revealed_value_helper.contains("Value::TotalManaValue"),
        "{relative} should classify revealed-card total mana value with token clause shapes"
    );
    for forbidden in [
        "normalized.split_once(',')",
        "predicate_text.rsplit_once",
        "split_whitespace()",
        "effect == \"this creature enters with a +1/+1 counter on it\"",
        "text_starts_with_self_x_counter_etb",
        "revealed_cards_total_mana_value_x_value(&normalized",
    ] {
        assert!(
            !helper.contains(forbidden)
                && !variable_prefix_helper.contains(forbidden)
                && !revealed_value_helper.contains(forbidden),
            "{relative} should not classify adamant ETB counter branches with raw string logic `{forbidden}`"
        );
    }
}

#[test]
fn full_party_triggered_special_case_uses_token_tail_split() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "if full_parse_tokens_contain_full_party_instead",
        "let full_sentences = split_lexed_sentences",
    );

    assert!(
        helper.contains("full_parse_tokens_contain_full_party_instead(full_parse_tokens)")
            && helper.contains(
                "FULL_PARTY_CONDITION_PATTERN.matches_words(&token_word_refs(effect_parse_tokens))"
            ),
        "{relative} should detect full-party effect tails from parse tokens"
    );
    let classifier = function_source(
        &content,
        "fn full_parse_tokens_contain_full_party_instead",
        "fn looks_like_combined_spell_and_activation_tax",
    );
    assert!(
        classifier.contains("FULL_PARTY_INSTEAD_PATTERN.matches_words(&token_word_refs(tokens))"),
        "{relative} should classify full-party replacement triggers from full parse tokens through clause shapes"
    );
    assert!(
        helper.contains(
            "FULL_PARTY_CONDITION_PATTERN.matches_words(&token_word_refs(effect_parse_tokens))"
        ),
        "{relative} should detect full-party effect tails from effect parse tokens through clause shapes"
    );
    assert!(
        helper.contains("split_once_at_comma_tokens(full_parse_tokens)"),
        "{relative} should recover full-party effect tails by lexed comma token"
    );
    for forbidden in [
        "text_mentions_full_party_instead",
        "line.full_text",
        "split_once(',')",
    ] {
        assert!(
            !helper.contains(forbidden) && !classifier.contains(forbidden),
            "{relative} should not classify full-party triggered text with raw text logic `{forbidden}`"
        );
    }
}

#[test]
fn direct_trigger_fast_path_guards_use_full_parse_tokens() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn full_parse_tokens_have_triggered_intervening_if_clause",
        "fn looks_like_combined_spell_and_activation_tax",
    );
    assert!(
        helper.contains("split_triggered_conditional_clause_lexed(tokens")
            && helper.contains("IF_YOU_DO_PATTERN.matches_words(&token_word_refs(tokens))")
            && helper.contains("IF_YOU_DONT_PATTERN.matches_words(&token_word_refs(tokens))"),
        "{relative} should classify direct-trigger fast-path blockers from full parse tokens through clause shapes"
    );

    let fast_path = function_source(
        &content,
        "if !token_word_refs(effect_parse_tokens).is_empty()",
        "pub(crate) fn lower_special_rewrite_triggered_chunk",
    );
    assert!(
        fast_path
            .contains("full_parse_tokens_have_triggered_intervening_if_clause(full_parse_tokens)")
            && fast_path.contains("full_parse_tokens_contain_if_you_do(full_parse_tokens)")
            && fast_path.contains("full_parse_tokens_contain_if_you_dont(full_parse_tokens)")
            && fast_path.contains(
                "EFFECT_STARTS_IF_PATTERN.matches_words(&token_word_refs(effect_parse_tokens))"
            ),
        "{relative} should feed full parse tokens into direct-trigger fast-path guards"
    );
    for forbidden in [
        "full_text_has_triggered_intervening_if_clause",
        "text_mentions_if_you_do(line.full_text.as_str())",
        "text_mentions_if_you_dont(line.full_text.as_str())",
        "line.effect_text.trim()",
        "text_starts_with_if(line.effect_text",
    ] {
        assert!(
            !fast_path.contains(forbidden),
            "{relative} should not reclassify direct-trigger fast-path blockers from rendered full text with `{forbidden}`"
        );
    }
}
