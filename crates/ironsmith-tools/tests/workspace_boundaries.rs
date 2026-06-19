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

fn source_contains_required(source: &str, required: &str) -> bool {
    if source.contains(required) {
        return true;
    }
    let compact_source = source
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace() && *ch != ',')
        .collect::<String>();
    let compact_required = required
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace() && *ch != ',')
        .collect::<String>();
    compact_source.contains(&compact_required)
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
        "runtime backend facade trigger-frequency/kicker postpasses should use lexed tokens and word-slice helpers, not raw source-text checks"
    );

    for required in [
        "trigger_frequency_text_ast",
        "parser_token_word_refs",
        "lexer::word_slice_contains_any_phrase(\n            &words,\n            FIRST_TIME_EACH_OR_THIS_TURN_PHRASES,\n        )",
        "lexer::word_slice_contains_phrase(&words, BECOMES_CREWED_PHRASE)",
        "parse_do_this_only_each_turn_limit",
        "let clause = LexedClause::new(&normalized_tokens)",
        "LexPattern::amount(\"limit\", LexCaptureKind::OneOf(&[\"once\", \"twice\"]))",
        "capture_clause_by_role(LexCaptureRole::Amount",
        "KICKED_COUNTER_SPELL_MANA_VALUE_REPLACEMENT_PHRASES\n        .iter()\n        .all(|phrase| lexer::word_slice_contains_phrase(&words, phrase))",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should preserve facade postpass semantics through token-backed shape/capture helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "FIRST_TIME_EACH_OR_THIS_TURN_PATTERN",
        "BECOMES_CREWED_PATTERN",
        "KICKED_COUNTER_SPELL_MANA_VALUE_REPLACEMENT_PATTERN",
        "ClauseShape",
        "FIRST_TIME_EACH_OR_THIS_TURN_PATTERN.matches_words",
        "BECOMES_CREWED_PATTERN.matches_words",
        "KICKED_COUNTER_SPELL_MANA_VALUE_REPLACEMENT_PATTERN.matches_words",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not route facade postpass phrase gates through raw word refs: found `{forbidden}`"
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
fn grammar_values_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/values.rs";
    let content = read_repo_file(&root, relative);
    let stat_parser = function_source(
        &content,
        "fn parse_value_stat_segment_shape",
        "#[cfg(test)]",
    );

    for required in [
        "enum ValueStatSubjectShape",
        "enum ValueStatAxisShape",
        "struct ValueStatSegmentShape",
        "enum ValueManaValueSubjectShape",
        "struct ValueManaValueSegmentShape",
        "struct PlayersWhoControlMoreValueShape",
        "const PLAYERS_WHO_CONTROL_MORE_THAN_YOU_PATTERN: LexPattern<'static>",
        "const VALUE_STAT_SEGMENT_PATTERN: LexPattern<'static>",
        "const VALUE_MANA_VALUE_SEGMENT_PATTERN: LexPattern<'static>",
        "LexPattern::object(\"filter\", LexCaptureKind::UntilPhrase(&[\"than\", \"you\"]))",
        "LexPattern::subject(\n        \"subject\",",
        "LexPattern::object(\n        \"subject\",",
        "LexPattern::action(\"axis\", LexCaptureKind::OneOf(&[\"power\", \"toughness\"]))",
        "fn parse_players_who_control_more_value_shape(",
        "filter_word_start: filter_capture.word_range.start",
        "filter_word_end: filter_capture.word_range.end",
        "fn parse_value_stat_segment_shape(clause: LexedClause<'_>) -> Option<ValueStatSegmentShape>",
        "fn parse_value_mana_value_segment_shape(\n    clause: LexedClause<'_>,",
        "matched.capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "matched.capture_clause_by_role(LexCaptureRole::Action, clause)",
        "matched.capture_clause_by_role(LexCaptureRole::Object, clause)",
        "fn value_from_stat_segment_shape(shape: ValueStatSegmentShape) -> Value",
        "fn value_from_mana_value_segment_shape(shape: ValueManaValueSegmentShape) -> Value",
        "fn parse_value_stat_segment(clause: LexedClause<'_>) -> Option<Value>",
        "fn parse_value_mana_value_segment(clause: LexedClause<'_>) -> Option<Value>",
        "let clause = LexedClause::new(tokens)",
        "let segment_clause = |start: usize, end: usize| -> Option<LexedClause<'_>>",
        "clause.between_word_range(tail_start + start, tail_start + end)",
        "parse_value_stat_segment(segment_clause)",
        "parse_value_mana_value_segment(segment_clause)",
        "let shape = parse_players_who_control_more_value_shape(tokens)?",
        "word == \"x\"",
        ".position(|window| window == EQUAL_TO_PHRASE)",
        "segment[0] != THAT_WORD",
        "!segment.ends_with(MANA_VALUE_SUFFIX)",
        "find_index(tail, |word| *word == PLUS_WORD)",
    ] {
        assert!(
            content.contains(required) || stat_parser.contains(required),
            "{relative} should parse value grammar through captured shapes and explicit token-word checks: missing `{required}`"
        );
    }

    for forbidden in [
        "use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers",
        "ClauseShape",
        "clause_shape!",
        "values_shape_matches_words",
        "synthetic_word_tokens(words)",
        ".matches_words(",
        "SOURCE_POWER_SEGMENT_PATTERN",
        "SOURCE_TOUGHNESS_SEGMENT_PATTERN",
        "TAGGED_POWER_SEGMENT_PATTERN",
        "TAGGED_TOUGHNESS_SEGMENT_PATTERN",
        "SACRIFICED_POWER_SEGMENT_PATTERN",
        "SACRIFICED_TOUGHNESS_SEGMENT_PATTERN",
        "EXILED_POWER_SEGMENT_PATTERN",
        "EXILED_TOUGHNESS_SEGMENT_PATTERN",
        "EXPLOITED_POWER_SEGMENT_PATTERN",
        "EXPLOITED_TOUGHNESS_SEGMENT_PATTERN",
        "POWER_WORD_PATTERN",
        "TOUGHNESS_WORD_PATTERN",
        "SACRIFICED_MARKER_PATTERN",
        "TAGGED_SPELL_MANA_VALUE_SEGMENT_PATTERN",
        "TAGGED_CARD_OR_SACRIFICED_MANA_VALUE_SEGMENT_PATTERN",
        "SOURCE_MANA_VALUE_SEGMENT_PATTERN",
        "WHERE_X_IS_PREFIX_PATTERN",
        "THE_WORD_PATTERN",
        "NUMBER_OF_PREFIX_PATTERN",
        "PLAYERS_WHO_CONTROL_MORE_PREFIX_PATTERN",
        "THAN_WORD_PATTERN",
        "THAN_YOU_TAIL_PATTERN",
        "X_VALUE_WORD_PATTERN",
        "EQUAL_TO_PATTERN",
        "THAT_WORD_PATTERN",
        "MANA_VALUE_SUFFIX_PATTERN",
        "PLUS_WORD_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep value grammar probes as one-off ClauseShape constants: found `{forbidden}`"
        );
    }
}

#[test]
fn runtime_backend_matches_words_is_clause_shape_primitive_only() {
    let root = workspace_root();
    let allowed = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/clause_pattern_helpers.rs";
    let mut files = Vec::new();
    collect_rust_files(
        &root.join("crates/ironsmith-compiler/src/runtime_backend"),
        &mut files,
    );

    let mut offenders = BTreeSet::new();
    for path in files {
        let relative = repo_relative(&root, &path);
        if relative == allowed {
            continue;
        }
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if content.contains(".matches_words(") {
            offenders.insert(relative);
        }
    }

    assert!(
        offenders.is_empty(),
        "runtime_backend shape gates should use token-backed helpers; raw matches_words callers outside the ClauseShape primitive: {offenders:#?}"
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
        parser.contains("word_slice_contains_phrase(&words, CREATURE_TYPE_OF_YOUR_CHOICE_PHRASE)"),
        "{relative} should classify creature-type-choice phrases through token word helpers"
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
fn document_line_family_shape_gates_use_direct_word_matching() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_family_handlers.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn line_family_words_start_with_pattern",
        "fn line_starts_with_words",
    );

    for required in [
        "fn line_family_words_start_with_pattern",
        "fn line_family_words_start_with_phrase",
        "fn line_family_words_contain_phrase",
        "fn line_family_words_contain_all_phrases",
        "fn line_family_words_end_with_any",
        "pattern.match_prefix_word_refs(words).is_some()",
        "words.starts_with(phrase)",
        "!phrase.is_empty() && words.windows(phrase.len()).any(|window| window == phrase)",
        ".any(|phrase| words.ends_with(phrase))",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose direct word-matching document line-family pattern helpers: missing `{required}`"
        );
    }
    assert!(
        content.contains("line_family_words_start_with_pattern(")
            && content.contains("line_family_words_contain_all_phrases(")
            && content.contains("line_family_words_end_with_any("),
        "{relative} should route document line-family shape gates through direct word helpers"
    );
    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains(".matches_words(")
            && !content.contains("synthetic_word_tokens(words)"),
        "{relative} should not route document line-family shape gates through ClauseShape/raw word refs"
    );
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
        ".ends_with(suffix))",
        "crate::runtime_backend::token_word_refs(&line.tokens)",
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
        "THIS_PERMANENT_PHRASE_PATTERN",
        ".find_in_clause(LexedClause::new(&tokens))",
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
        "{relative} should detect weighted modal headers from lexed tokens and word-slice helpers, not raw `{{P}}` text probes"
    );

    for required in [
        "header_mentions_modal_point_cost_lexed(&tokens)",
        "pawprint_modal_label_count(token)",
        "word_slice_contains_phrase(&token_word_refs(tokens), &[\"worth\", \"of\", \"modes\"])",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should preserve weighted modal header detection through token-backed grammar helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "text.to_ascii_lowercase().contains(\"{p} worth of modes\")",
        ".contains(\"{p} worth of modes\")",
        "MODAL_POINT_COST_HEADER_TAIL_PATTERN",
        "MODAL_POINT_COST_HEADER_TAIL_PATTERN.matches_words",
        "parser_token_word_refs(tokens)",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not detect weighted modal headers with raw text branch `{forbidden}`"
        );
    }
}

#[test]
fn whole_clause_shape_gates_use_lexed_clause_matching() {
    let root = workspace_root();

    let parser_support_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/parser_support.rs";
    let parser_support = read_repo_file(&root, parser_support_relative);
    assert!(
        parser_support
            .contains("word_slice_contains_phrase(&token_word_refs(before), &[\"this\", \"way\"])"),
        "{parser_support_relative} should route comma-prefix this-way followup detection through token word matching"
    );
    assert!(
        parser_support
            .contains("word_slice_contains_phrase(&token_word_refs(tokens), &[\"this\", \"way\"])"),
        "{parser_support_relative} should route full-clause this-way followup detection through token word matching"
    );
    assert!(
        !parser_support.contains("THIS_WAY_PATTERN")
            && !parser_support.contains("ClauseShape")
            && !parser_support.contains(".matches_words("),
        "{parser_support_relative} should not route this-way followup detection through ClauseShape/raw word refs"
    );

    let primitives_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/primitives.rs";
    let primitives = read_repo_file(&root, primitives_relative);
    for required in [
        "const POWER_AXIS_SUFFIXES: &[&[&str]]",
        "POWER_AXIS_SUFFIXES\n        .iter()\n        .any(|suffix| current_words.ends_with(suffix))",
        "remaining_words.first().copied() == Some(TOUGHNESS_WORD)",
        "token.is_word(OR_WORD)",
    ] {
        assert!(
            primitives.contains(required),
            "{primitives_relative} should route primitive word/suffix probes through explicit token-word helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "use crate::runtime_backend::effect_sentences::clause_pattern_helpers",
        "ClauseShape",
        "clause_shape!",
        "POWER_AXIS_SUFFIX_PATTERN",
        "TOUGHNESS_WORD_PATTERN",
        "OR_WORD_PATTERN",
        "COMPARISON_OR_TAIL_WORD_PATTERN",
        "THAN_WORD_PATTERN",
        "EQUAL_WORD_PATTERN",
    ] {
        assert!(
            !primitives.contains(forbidden),
            "{primitives_relative} should not keep primitive word/suffix probes as ClauseShape constants: found `{forbidden}`"
        );
    }

    let search_support_relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/search_library_support.rs";
    let search_support = read_repo_file(&root, search_support_relative);
    assert!(
        search_support.contains(
            "word_slice_contains_phrase(&token_word_refs(tokens), &[\"for\", \"as\", \"long\", \"as\"])"
        ),
        "{search_support_relative} should route source-remains-tapped duration gates through token word matching"
    );
    assert!(
        !search_support.contains("FOR_AS_LONG_AS_PATTERN")
            && !search_support.contains("ClauseShape")
            && !search_support.contains("clause_shape")
            && !search_support.contains(".matches_words("),
        "{search_support_relative} should not route source-remains-tapped duration gates through ClauseShape/raw word refs"
    );

    let divvy_relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/divvy.rs";
    let divvy = read_repo_file(&root, divvy_relative);
    assert!(
        divvy
            .contains("fn first_sentence_has_prefix(sentences: &[SentenceInput], prefix: &[&str])"),
        "{divvy_relative} should preserve first-sentence prefix checks against sentence token inputs"
    );
    assert!(
        divvy.contains("word_slice_starts_with(&token_word_refs(sentence.lowered()), prefix)"),
        "{divvy_relative} should route dynamic first-sentence prefixes through token word matching"
    );
    assert!(
        divvy.contains("DIVVY_SEARCH_LIBRARY_GRAVEYARD_CREATURE_CARDS_PREFIX,\n            )"),
        "{divvy_relative} should route the divvy search prefix through token word matching"
    );
    for forbidden in [
        "ClauseShape",
        "DIVVY_SEARCH_LIBRARY_GRAVEYARD_CREATURE_CARDS_PREFIX_PATTERN",
        ".matches_words(&words.word_refs())",
        "DIVVY_SEARCH_LIBRARY_GRAVEYARD_CREATURE_CARDS_PREFIX_PATTERN\n                .matches_words(&word_refs)",
    ] {
        assert!(
            !divvy.contains(forbidden),
            "{divvy_relative} should not route divvy prefix gates through raw word refs: found `{forbidden}`"
        );
    }

    let unsupported_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/unsupported_shape_diagnostics.rs";
    let unsupported = read_repo_file(&root, unsupported_relative);
    for required in [
        "word_slice_starts_with(&words[gain_idx + 1..], UNSUPPORTED_GAIN_LIFE_EQUAL_TO_PREFIX)",
        "word_slice_contains_phrase(&words[gain_idx + 4..], UNSUPPORTED_ITS_POWER_PHRASE)",
        "word_slice_starts_with(&words[gain_idx + 1..], UNSUPPORTED_X_PLUS_PREFIX)",
        "unsupported_word_is_gain(word)",
        "unsupported_word_is_negation(words[gain_idx - 1])",
    ] {
        assert!(
            unsupported.contains(required),
            "{unsupported_relative} should route unsupported diagnostic gates through token word matching: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "UNSUPPORTED_GAIN_LIFE_EQUAL_TO_PREFIX_PATTERN",
        "UNSUPPORTED_ITS_POWER_MARKER_PATTERN",
        "UNSUPPORTED_X_PLUS_PREFIX_PATTERN",
        "UNSUPPORTED_GAIN_LIFE_EQUAL_TO_PREFIX_PATTERN.matches_words(words)",
        "UNSUPPORTED_ITS_POWER_MARKER_PATTERN.matches_words(tail)",
        "UNSUPPORTED_X_PLUS_PREFIX_PATTERN.matches_words(&words[gain_idx + 1..])",
    ] {
        assert!(
            !unsupported.contains(forbidden),
            "{unsupported_relative} should not route unsupported diagnostic gates through ClauseShape/raw word refs: found `{forbidden}`"
        );
    }

    let subject_verb_followups_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_entry/subject_verb_followups.rs";
    let subject_verb_followups = read_repo_file(&root, subject_verb_followups_relative);
    for required in [
        "word_slice_eq(\n        &token_word_refs(sentence_tokens),\n        SKIP_TURN_WHILE_THIS_ARTIFACT_TAPPED_WORDS,\n    )",
        "word_slice_starts_with(&tail_words, OF_THOSE_TOKENS_PREFIX)",
        "let trailing_words = &tail_words[OF_THOSE_TOKENS_PREFIX.len()..]",
        "word_slice_eq_any(trailing_words, CREATE_THOSE_TOKENS_TRAILING_WORDS)",
        "word_slice_starts_with(\n        &token_word_refs(sentence_tokens),\n        WHEN_ONE_OR_MORE_CARDS_MILLED_THIS_WAY_PREFIX,\n    )",
    ] {
        assert!(
            subject_verb_followups.contains(required),
            "{subject_verb_followups_relative} should route subject-verb followup gates through token word matching: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "SKIP_TURN_WHILE_THIS_ARTIFACT_TAPPED_PATTERN",
        "OF_THOSE_TOKENS_PREFIX_PATTERN",
        "CREATE_THOSE_TOKENS_TRAILING_PATTERN",
        "WHEN_ONE_OR_MORE_CARDS_MILLED_THIS_WAY_PREFIX_PATTERN",
        "LexedClause::new(sentence_tokens).matches_words",
        "OF_THOSE_TOKENS_PREFIX_PATTERN.matches_words(&tail_words)",
        "CREATE_THOSE_TOKENS_TRAILING_PATTERN.matches_words(trailing_words)",
    ] {
        assert!(
            !subject_verb_followups.contains(forbidden),
            "{subject_verb_followups_relative} should not route subject-verb followup gates through raw word refs: found `{forbidden}`"
        );
    }

    let generic_sequences_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/sequence_rules/generic_subject_verb_sequences/mod.rs";
    let generic_sequences = read_repo_file(&root, generic_sequences_relative);
    for required in [
        "let Some(flashback_tail) = first_clause.from_word(gain_idx + 1)",
        "word_slice_eq(&flashback_tail.word_refs(), FLASHBACK_UNTIL_END_TAIL)",
        "fn non_article_tokens_eq(tokens: &[OwnedLexToken], expected: &[&str]) -> bool",
        "word_slice_starts_with(&second_words, PREVENTED_DAMAGE_COUNTER_FOLLOWUP_PREFIX)",
        "word_slice_contains_all_words(&second_words, PREVENTED_DAMAGE_COUNTER_FOLLOWUP_WORDS)",
        "second_words.iter().position(|word| *word == \"on\")",
    ] {
        assert!(
            generic_sequences.contains(required),
            "{generic_sequences_relative} should route generic sequence gates through reusable token word helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_non_article_tokens(",
        "FLASHBACK_UNTIL_END_TAIL.matches(flashback_tail)",
        "FLASHBACK_UNTIL_END_TAIL.matches_words(&first_words[gain_idx + 1..])",
        "PREVENTED_DAMAGE_COUNTER_FOLLOWUP.matches_non_article_tokens(second_clause.tokens())",
        "PREVENTED_DAMAGE_COUNTER_FOLLOWUP.matches_words(&second_words)",
        "ON_WORD_PATTERN.matches_word(word)",
        "ON_WORD_PATTERN.matches_words(&[*word])",
    ] {
        assert!(
            !generic_sequences.contains(forbidden),
            "{generic_sequences_relative} should not route generic sequence gates through one-off ClauseShape/raw word refs: found `{forbidden}`"
        );
    }

    let generic_quads_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/sequence_rules/generic_subject_verb_sequences/quads.rs";
    let generic_quads = read_repo_file(&root, generic_quads_relative);
    for required in [
        "let words = non_article_token_word_refs(&trimmed);",
        "word_slice_starts_with(&words, EXILE_ONE_LOOKED_CARD_FACE_DOWN_PREFIX)",
        "word_slice_eq(\n            &non_article_token_word_refs(prefix.tokens()),\n            CAST_EXILED_CARD_FREE_PREFIX,",
        "word_slice_eq_any(\n        &LexedClause::new(&trimmed).word_refs(),\n        EXILED_CARD_HAND_FOLLOWUP_CLAUSES,",
    ] {
        assert!(
            generic_quads.contains(required),
            "{generic_quads_relative} should route quad sequence gates through reusable token word helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "EXILE_ONE_LOOKED_CARD_FACE_DOWN_REST_BOTTOM_PATTERN.matches_words(&words)",
        "EXILE_ONE_LOOKED_CARD_FACE_DOWN_REST_BOTTOM_PATTERN.matches_non_article_tokens(&trimmed)",
        "EXILED_CARD_HAND_FOLLOWUP_PATTERN.matches_words(&words)",
        "EXILED_CARD_HAND_FOLLOWUP_PATTERN.matches(LexedClause::new(&trimmed))",
        "CAST_EXILED_CARD_FREE_PREFIX_PATTERN.matches_words(&words[..if_word_idx])",
        "CAST_EXILED_CARD_FREE_PREFIX_PATTERN.matches_non_article_tokens(prefix.tokens())",
    ] {
        assert!(
            !generic_quads.contains(forbidden),
            "{generic_quads_relative} should not route quad sequence gates through one-off ClauseShape/raw word refs: found `{forbidden}`"
        );
    }

    let keyword_lines_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/keyword_lines.rs";
    let keyword_lines = read_repo_file(&root, keyword_lines_relative);
    for required in [
        "enum ProtectionChosenTargetShape",
        "const KEYWORD_PROTECTION_EACH_MANA_VALUE_PATTERN: LexPattern<'static>",
        "const KEYWORD_THE_CHOSEN_PLAYER_PATTERN: LexPattern<'static>",
        "const KEYWORD_THE_CHOSEN_COLOR_PATTERN: LexPattern<'static>",
        "fn protection_chosen_target_shape(",
        "let clause = LexedClause::new(tokens)",
        "let target_tail = clause.from_word(idx + 1)?",
        "KEYWORD_PROTECTION_EACH_MANA_VALUE_PATTERN\n            .match_prefix(target_tail)",
        "protection_chosen_target_shape(target_tail)",
        "ProtectionChosenTargetShape::Player",
        "ProtectionChosenTargetShape::Color",
    ] {
        assert!(
            keyword_lines.contains(required),
            "{keyword_lines_relative} should route keyword protection shape gates through captured LexPattern classifiers: missing `{required}`"
        );
    }
    for forbidden in [
        "KEYWORD_AND_WORD_PATTERN",
        "KEYWORD_PROTECTION_WORD_PATTERN",
        "KEYWORD_FROM_WORD_PATTERN",
        "KEYWORD_WITH_WORD_PATTERN",
        "KEYWORD_PERMANENT_OR_PERMANENTS_WORD_PATTERN",
        "KEYWORD_THE_LAST_CHOSEN_COLOR_PATTERN",
        "KEYWORD_COLOR_OR_COLORS_WORD_PATTERN",
        "ClauseShape",
        "clause_shape!",
        "KEYWORD_PROTECTION_EACH_MANA_VALUE_PATTERN.matches_words(target_tail)",
        "KEYWORD_PROTECTION_EACH_MANA_VALUE_PATTERN.matches(target_tail)",
        "KEYWORD_THE_CHOSEN_PLAYER_PATTERN.matches_words(&words[idx + 1..])",
        "KEYWORD_THE_CHOSEN_PLAYER_PATTERN.matches(target_tail)",
        "KEYWORD_THE_CHOSEN_COLOR_PATTERN.matches_words(&words[idx + 1..])",
        "KEYWORD_THE_CHOSEN_COLOR_PATTERN.matches(target_tail)",
        "KEYWORD_THE_LAST_CHOSEN_COLOR_PATTERN.matches_words(&words[idx + 1..])",
        "KEYWORD_THE_LAST_CHOSEN_COLOR_PATTERN.matches(target_tail)",
    ] {
        assert!(
            !keyword_lines.contains(forbidden),
            "{keyword_lines_relative} should not keep keyword protection gates as one-off ClauseShape or raw-word probes: found `{forbidden}`"
        );
    }

    let line_cst_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_cst_parsing.rs";
    let line_cst = read_repo_file(&root, line_cst_relative);
    for required in [
        "ANY_NUMBER_NAMED_DECK_CONSTRUCTION_PREFIX_PATTERN.matches_prefix(LexedClause::new(tokens))",
        "FIRST_EQUIP_COST_ALTERNATIVE_PREFIX_PATTERN.matches_prefix(clause)",
        "contains_token_word_sequence(\n            tokens,",
        "clause.ends_with_any(&[\n            &[\"each\", \"turn\"],",
        "ADDITIONAL_LAND_PLAY_PREFIX_PATTERN.matches_prefix(LexedClause::new(tokens))",
        "CAN_BLOCK_ADDITIONAL_CREATURES_PREFIX_PATTERN.matches_prefix(clause)",
    ] {
        assert!(
            line_cst.contains(required),
            "{line_cst_relative} should route document line classifiers through token-backed shape helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "ANY_NUMBER_NAMED_DECK_CONSTRUCTION_PREFIX_PATTERN.matches_words(&words)",
        "FIRST_EQUIP_COST_ALTERNATIVE_PATTERN.matches_words(&token_word_refs(tokens))",
        "ADDITIONAL_LAND_PLAY_PREFIX_PATTERN.matches_words(&words)",
        "CAN_BLOCK_ADDITIONAL_CREATURES_PATTERN.matches_words(&token_word_refs(tokens))",
        "FIRST_EQUIP_COST_ALTERNATIVE_PATTERN.matches(",
        "CAN_BLOCK_ADDITIONAL_CREATURES_PATTERN.matches(",
    ] {
        assert!(
            !line_cst.contains(forbidden),
            "{line_cst_relative} should not route document line classifiers through raw word refs: found `{forbidden}`"
        );
    }

    let remove_destroy_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/remove_destroy.rs";
    let remove_destroy = read_repo_file(&root, remove_destroy_relative);
    for required in [
        "token_slice_words_eq(&tokens[from_idx + 1..], COMBAT_WORDS)",
        "word_slice_contains_phrase(",
        "END_OF_COMBAT_PHRASE",
        "word_slice_contains_any_phrase(",
        "COMBAT_HISTORY_DESTROY_PHRASES",
        "token_slice_words_eq_any(&target_tokens, ATTACHED_SUPPORTED_TARGET_WORDS)",
        "word_slice_ends_with_any(&filter_words, CHOSEN_THIS_WAY_SUFFIXES)",
        "word_slice_find_phrase_start(&clause_words, DEALT_DAMAGE_THIS_TURN_TAIL)",
        "word_slice_find_phrase_start(clause_words, THAT_DEALT_DAMAGE_TO_PHRASE)",
    ] {
        assert!(
            remove_destroy.contains(required),
            "{remove_destroy_relative} should route remove/destroy gates through token-backed word-slice helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "LexedClause",
        "_PATTERN",
        "COMBAT_WORD_PATTERN.matches_words(&tail_words)",
        "END_OF_COMBAT_PATTERN.matches_words(&original_clause_words)",
        "COMBAT_HISTORY_DESTROY_PATTERN.matches_words(&clause_words)",
        "ATTACHED_SUPPORTED_TARGET_PATTERN.matches_words(&target_words)",
        "CHOSEN_THIS_WAY_SUFFIX_PATTERN.matches_words(&filter_words)",
        "shape.matches_words(window)",
    ] {
        assert!(
            !remove_destroy.contains(forbidden),
            "{remove_destroy_relative} should not route remove/destroy gates through raw word refs: found `{forbidden}`"
        );
    }

    let combat_damage_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/combat_and_damage_family.rs";
    let combat_damage = read_repo_file(&root, combat_damage_relative);
    for required in [
        "const TARGET_REFERENCE_WORDS: &[&str]",
        "const ZONE_SUFFIX_START_WORDS: &[&str]",
        "const ALL_OR_EACH_WORDS: &[&str]",
        "const TARGET_REFERENCE_HEAD_WORDS: &[&str]",
        "fn token_is_word(token: &OwnedLexToken, expected: &str) -> bool",
        "fn token_is_any_word(token: &OwnedLexToken, expected: &[&str]) -> bool",
        "TARGET_REFERENCE_WORDS.contains(word)",
        "ZONE_SUFFIX_START_WORDS.contains(word)",
        "ALL_OR_EACH_WORDS.contains(word)",
        "TARGET_REFERENCE_HEAD_WORDS.contains(word)",
        "token_is_word(first, TRANSFORM_WORD)",
        "token_is_word(first, CONVERT_WORD)",
    ] {
        assert!(
            combat_damage.contains(required),
            "{combat_damage_relative} should route singleton combat/damage word gates through constants and token-word helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        "combat_clause_first_matches",
        "TARGET_REFERENCE_WORD_PATTERN.matches_words(&[*word])",
        "ZONE_SUFFIX_START_WORD_PATTERN.matches_words(&[*word])",
        "ALL_OR_EACH_WORD_PATTERN.matches_words(&[*word])",
        "TARGET_REFERENCE_HEAD_PATTERN.matches_words(&[*word])",
    ] {
        assert!(
            !combat_damage.contains(forbidden),
            "{combat_damage_relative} should not route singleton combat/damage word gates through ClauseShape adapters or one-word slices: found `{forbidden}`"
        );
    }

    let misc_actions_relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/misc_actions.rs";
    let misc_actions = read_repo_file(&root, misc_actions_relative);
    for required in [
        "word_slice_eq_any(&clause_words, MONARCH_WORDS)",
        "word_slice_eq(&target_words, &[IT_WORD])",
        "word_slice_eq_any(&clause_words, END_TURN_WORDS)",
        "word_slice_eq(&clause_words, END_STEP_YOU_LOSE_WORDS)",
        "word_slice_eq_any(&token_words, COIN_WORDS)",
        "word_slice_eq_any(&token_words, SELF_FLIP_TARGET_WORDS)",
        "word_slice_eq(&words, THEM_WORDS)",
        "word_slice_contains_phrase(&clause_words, FOR_EACH_PHRASE)",
        "word_slice_contains_phrase(&clause_words, CHOSEN_THIS_WAY_PHRASE)",
        "word_slice_contains_all_words(&words, NEXT_COMBAT_PHASE_THIS_TURN_WORDS)",
        "word_slice_contains_all_words(&words, COMBAT_PHASE_TURN_WORDS)",
        "word_slice_contains_all_words(&words, DRAW_STEP_WORDS)",
        "word_slice_contains_all_words(&words, TURN_WORDS)",
        "word_slice_starts_with_any(&words[start_idx..], FOR_EACH_PREFIXES)",
        "word_slice_starts_with(&words[start_idx..], FOR_EACH_EXPLICIT_PREFIX)",
    ] {
        assert!(
            misc_actions.contains(required),
            "{misc_actions_relative} should route whole-clause action gates through direct word-slice helpers: missing `{required}`"
        );
    }
    for required in [
        "second == SIDED_WORD",
        "DIE_WORDS.contains(&third)",
        "ALL_OR_EACH_WORDS.contains(word)",
        "CARD_OR_CARDS_WORDS.contains(&word)",
        "*word == ON_WORD",
        "*word == THIS_WORD",
        "THOSE_OR_THEM_WORDS.contains(word)",
    ] {
        assert!(
            misc_actions.contains(required),
            "{misc_actions_relative} should route singleton action word gates through direct word comparisons: missing `{required}`"
        );
    }
    for required in [
        "COUNTER_OR_COUNTERS_WORDS.contains(word)",
        "word_slice_starts_with_any(reference, THIS_REFERENCE_PREFIXES)",
        "parse_counter_type_from_tokens(&tokens[counter_start..counter_end])",
        "word_slice_starts_with(text_words, BEGINNING_OF_PREFIX)",
    ] {
        assert!(
            misc_actions.contains(required),
            "{misc_actions_relative} should route remaining misc action gates through direct token/word helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "LexedClause",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        ".matches(",
        "fn misc_shape_matches_words(",
        "_PATTERN",
    ] {
        assert!(
            !misc_actions.contains(forbidden),
            "{misc_actions_relative} should not route misc action gates through ClauseShape adapters: found `{forbidden}`"
        );
    }

    let sacrifice_discard_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/sacrifice_discard.rs";
    let sacrifice_discard = read_repo_file(&root, sacrifice_discard_relative);
    for required in [
        "word_slice_eq_any(\n        &crate::runtime_backend::token_word_refs(tokens),\n        DISCARD_SAME_MANA_VALUE_FILTER_PHRASES,",
        "word_slice_eq_any(\n        &crate::runtime_backend::token_word_refs(rest),\n        MANA_SPENT_TO_CAST_SELF_PHRASES,",
        "word_slice_eq(&words[*idx..*idx + marker_words.len()], marker_words)",
        "word_slice_eq(&tail.word_refs(), SACRIFICE_UNLESS_ESCAPED_WORDS)",
        "token_is_word(token, SACRIFICE_UNLESS_WORD)",
        "TAGGED_IT_OR_CARD_PHRASES",
        "SACRIFICE_UNLESS_OPPONENT_DAMAGED_WORDS",
        "CHOICE_SUFFIX_FIVE_WORDS",
        "word_slice_eq(&filter_token_words, TAGGED_TOKEN_WORDS)",
        "word_slice_contains_any_phrase(\n        &crate::runtime_backend::token_word_refs(tokens),\n        ATTACHED_OBJECT_EXCLUSION_PHRASES,",
        "word_slice_eq_any(&clause_words, DISCARD_HAND_PHRASES)",
        "word_slice_eq(&clause_words, DISCARD_THOSE_CARDS_WORDS)",
        "token_is_word(token, DISCARD_ALL_WORD)",
        "DISCARD_CARD_OR_CARDS_WORDS.contains(word)",
        "word_slice_eq(\n            &crate::runtime_backend::token_word_refs(&qualifier_tokens),\n            DISCARD_THE_QUALIFIER_WORDS,",
        "word_slice_eq(&trailing_words, DISCARD_AT_RANDOM_WORDS)",
        "word_slice_eq(&trailing_words, DISCARD_WITH_THAT_NAME_WORDS)",
        "word_slice_eq_any(qualifier_words.as_slice(), DISCARD_CHOSEN_COLOR_PHRASES)",
    ] {
        assert!(
            sacrifice_discard.contains(required),
            "{sacrifice_discard_relative} should route sacrifice/discard gates through word-slice helpers and reusable phrase constants: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "LexedClause",
        ".matches_word(",
        ".matches_token(",
        ".matches(",
        "matches_non_article_tokens",
        "marker_pattern",
        "DISCARD_SAME_MANA_VALUE_FILTER_PATTERN.matches_words(&words)",
        "MANA_SPENT_TO_CAST_SELF_PATTERN.matches_words(&words)",
        "TAGGED_IT_OR_CARD_PATTERN\n                .matches_words(&crate::runtime_backend::token_word_refs(&sacrifice_tokens))",
        "SACRIFICE_UNLESS_OPPONENT_DAMAGED_PATTERN.matches_words(&unless_words)",
        "TAGGED_IT_OR_CARD_PATTERN.matches_words(&filter_words)",
        "TAGGED_TOKEN_PATTERN.matches_words(&filter_words)",
        "ATTACHED_OBJECT_EXCLUSION_PATTERN.matches_words(&sacrifice_words)",
        "DISCARD_HAND_PATTERN.matches_words(&clause_words)",
        "TAGGED_IT_OR_CARD_PATTERN.matches_words(&clause_words)",
        "DISCARD_THOSE_CARDS_PATTERN.matches_words(&clause_words)",
        "DISCARD_THE_QUALIFIER_PATTERN\n            .matches_words(&crate::runtime_backend::token_word_refs(&qualifier_tokens))",
        "DISCARD_AT_RANDOM_PATTERN.matches_words(&trailing_words)",
        "DISCARD_WITH_THAT_NAME_PATTERN.matches_words(&trailing_words)",
        "CHOICE_SUFFIX_FIVE_WORD_PATTERN.matches_words(&filter_words.to_word_refs())",
        "marker_pattern.matches_words(window)",
        "SACRIFICE_UNLESS_ESCAPED_PATTERN.matches_words(tail)",
        "DISCARD_CHOSEN_COLOR_PATTERNS.matches_words(qualifier_words.as_slice())",
    ] {
        assert!(
            !sacrifice_discard.contains(forbidden),
            "{sacrifice_discard_relative} should not route sacrifice/discard gates through ClauseShape adapters or raw word refs: found `{forbidden}`"
        );
    }
    for forbidden in [
        "SACRIFICE_UNLESS_WORD_PATTERN.matches_word(word)",
        "DISCARD_CARD_OR_CARDS_WORD_PATTERN.matches_word(word)",
        "SACRIFICE_UNLESS_WORD_PATTERN.matches_words(&[*word])",
        "DISCARD_CARD_OR_CARDS_WORD_PATTERN.matches_words(&[*word])",
    ] {
        assert!(
            !sacrifice_discard.contains(forbidden),
            "{sacrifice_discard_relative} should not route singleton sacrifice/discard word gates through one-word slices: found `{forbidden}`"
        );
    }

    let token_copy_control_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/token_copy_control_family.rs";
    let token_copy_control = read_repo_file(&root, token_copy_control_relative);
    for required in [
        "word_slice_starts_with(&reveal_words, EACH_PLAYER_REVEALS_TOP_PREFIX)",
        "word_slice_eq(&rest_clause.word_refs(), EACH_PLAYER_PUTS_REST_GRAVEYARD)",
        "word_slice_starts_with(&head_words, YOU_EXILE_PREFIX)",
        "word_slice_eq_any(&on_target_clause.word_refs(), IT_OR_THEM_PHRASES)",
        "fn token_copy_tail_returns_this_to_owner_hand(words: &[&str]) -> bool",
        "fn token_copy_tail_puts_this_on_top_of_owner_library(words: &[&str]) -> bool",
        "token_copy_tail_starts_with_that_player(&tail_words)",
        "token_copy_tail_returns_this_to_owner_hand(&tail_words)",
        "token_copy_tail_puts_this_on_top_of_owner_library(&tail_words)",
        "token_copy_tail_starts_with_choose_card_name(&tail_words)",
    ] {
        assert!(
            token_copy_control.contains(required),
            "{token_copy_control_relative} should route token-copy/control gates through word-slice helpers and reusable tail recognizers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        "fn token_copy_shape_matches_words(",
        "synthetic_word_tokens(words)",
        "LexedClause::new(rest_clause.tokens())",
        "LexedClause::new(head_slice.tokens())",
        "LexedClause::new(on_target_clause.tokens())",
        "EACH_PLAYER_PUTS_REST_GRAVEYARD_PATTERN.matches_words(rest_words)",
        "YOU_EXILE_PREFIX_PATTERN.matches_words(&head_words)",
        "IT_OR_THEM_PATTERN.matches_words(&on_target_words)",
        "THAT_PLAYER_TAIL_PREFIX_PATTERN.matches_words(&tail_words)",
        "RETURN_THIS_OWNER_HAND_TAIL_PATTERN.matches_words(&tail_words)",
        "PUT_THIS_OWNER_TOP_LIBRARY_TAIL_PATTERN.matches_words(&tail_words)",
        "CHOOSE_CARD_NAME_TAIL_PREFIX_PATTERN.matches_words(&tail_words)",
    ] {
        assert!(
            !token_copy_control.contains(forbidden),
            "{token_copy_control_relative} should not route token-copy/control gates through ClauseShape adapters or raw word refs: found `{forbidden}`"
        );
    }
    assert!(
        !token_copy_control.contains(".matches_words("),
        "{token_copy_control_relative} should not route token-copy/control shape gates through raw word refs"
    );

    let document_mod_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/mod.rs";
    let document_mod = read_repo_file(&root, document_mod_relative);
    for required in [
        "let segment_clause = LexedClause::new(segment)",
        "CUMULATIVE_UPKEEP_PREFIX_PATTERN.matches_prefix(segment_clause)",
        "token.is_word(\"echo\")",
        "SOURCE_LEAVES_BATTLEFIELD_PATTERN",
        "THIS_PERMANENT_PHRASE_PATTERN",
        ".find_in_clause(LexedClause::new(&tokens))",
        "WHEN_ONE_OR_MORE_PREFIX_PATTERN.matches_prefix(LexedClause::new(tokens))",
    ] {
        assert!(
            document_mod.contains(required),
            "{document_mod_relative} should route document module gates through LexedClause/token matching: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "CUMULATIVE_UPKEEP_PREFIX_PATTERN.matches_words(&segment_words)",
        "ECHO_WORD_PATTERN.matches_words(&[*word])",
        "SOURCE_LEAVES_BATTLEFIELD_PATTERN.matches_words(&token_word_refs(&tokens))",
        "THIS_PERMANENT_PHRASE_PATTERN.matches_words(&token_word_refs(&tokens))",
        "WHEN_ONE_OR_MORE_PREFIX_PATTERN.matches_words(&words)",
        "document_shape_matches_words",
        "CUMULATIVE_UPKEEP_PREFIX_PATTERN.matches(segment_clause)",
        "ECHO_WORD_PATTERN.matches_token(token)",
    ] {
        assert!(
            !document_mod.contains(forbidden),
            "{document_mod_relative} should not route document module gates through raw word refs: found `{forbidden}`"
        );
    }

    let trigger_clause_relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/trigger_clause_core.rs";
    let trigger_clause = read_repo_file(&root, trigger_clause_relative);
    let trigger_shape_helpers = function_source(
        &trigger_clause,
        "fn token_words_match_prefix",
        "const PUT_INTO_YOUR_GRAVEYARD_SUFFIXES",
    );
    for required in [
        "fn trigger_clause_shape_matches_words(",
        "shape.matches_word_slice(words)",
        "trigger_clause_shape_matches_words(&words, *shape)",
        "trigger_clause_shape_matches_words(window, shape)",
        "trigger_clause_shape_matches_words(words, ONE_OR_MORE_PREFIX_PATTERN)",
        "trigger_clause_shape_matches_words(words, CARD_OR_CARDS_PATTERN)",
        "trigger_clause_shape_matches_words(words, CARD_OR_CARDS_WORD_PATTERN)",
        "trigger_clause_shape_matches_words(words, PERMANENT_OR_PERMANENTS_WORD_PATTERN)",
        "fn trigger_clause_shape_matches_word",
        "fn trigger_clause_token_matches_shape",
        "fn trigger_clause_shape_matches_word_at",
        "trigger_clause_token_matches_shape(token, *shape)",
    ] {
        assert!(
            trigger_shape_helpers.contains(required),
            "{trigger_clause_relative} should route trigger helper shape gates through direct word matching: missing `{required}`"
        );
    }
    for forbidden in [
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "shape.matches_words(&words)",
        "shape.matches_words(window)",
        "ONE_OR_MORE_PREFIX_PATTERN.matches_words(words)",
        "CARD_OR_CARDS_PATTERN.matches_words(words)",
        "CARD_OR_CARDS_WORD_PATTERN.matches_words(words)",
        "PERMANENT_OR_PERMANENTS_WORD_PATTERN.matches_words(words)",
    ] {
        assert!(
            !trigger_shape_helpers.contains(forbidden),
            "{trigger_clause_relative} should not route trigger helper shape gates through raw word refs: found `{forbidden}`"
        );
    }

    let trigger_suffix_helper = function_source(
        &trigger_clause,
        "fn trigger_suffix_word_len",
        "fn trigger_subject_tokens_before_suffix",
    );
    assert!(
        trigger_suffix_helper.contains("trigger_clause_shape_matches_words(words, suffix.shape)"),
        "{trigger_clause_relative} should route trigger suffix lookup through token-backed matching"
    );
    assert!(
        !trigger_suffix_helper.contains("suffix.shape.matches_words(words)"),
        "{trigger_clause_relative} should not route trigger suffix lookup through raw word refs"
    );

    let trigger_counter_helper = function_source(
        &trigger_clause,
        "fn trigger_counter_type_from_descriptor",
        "fn trigger_counter_recipient_tokens",
    );
    assert!(
        trigger_counter_helper.contains(
            "trigger_clause_shape_matches_words(&words, ENERGY_COUNTER_DESCRIPTOR_PATTERN)"
        ),
        "{trigger_clause_relative} should route trigger counter descriptor matching through token-backed shapes"
    );
    assert!(
        !trigger_counter_helper
            .contains("ENERGY_COUNTER_DESCRIPTOR_PATTERN\n            .matches_words(&words)")
            && !trigger_counter_helper
                .contains("ENERGY_COUNTER_DESCRIPTOR_PATTERN.matches_words(&words)"),
        "{trigger_clause_relative} should not route trigger counter descriptor matching through raw word refs"
    );

    let trigger_damage_suffix_helper = function_source(
        &trigger_clause,
        "fn dealt_damage_suffix_subject_word_idx",
        "pub(crate) fn strip_leading_trigger_intro",
    );
    for required in [
        "trigger_clause_shape_matches_words(words, DEALT_COMBAT_DAMAGE_SUFFIX_PATTERN)",
        "trigger_clause_shape_matches_words(words, DEALT_DAMAGE_SUFFIX_PATTERN)",
    ] {
        assert!(
            trigger_damage_suffix_helper.contains(required),
            "{trigger_clause_relative} should route dealt-damage suffix matching through token-backed shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "DEALT_COMBAT_DAMAGE_SUFFIX_PATTERN.matches_words(words)",
        "DEALT_DAMAGE_SUFFIX_PATTERN.matches_words(words)",
    ] {
        assert!(
            !trigger_damage_suffix_helper.contains(forbidden),
            "{trigger_clause_relative} should not route dealt-damage suffix matching through raw word refs: found `{forbidden}`"
        );
    }

    let trigger_clause_parser = function_source(
        &trigger_clause,
        "pub(crate) fn parse_trigger_clause_lexed",
        "fn parse_loyalty_ability_trigger_tail_lexed",
    );
    for required in [
        "trigger_clause_shape_matches_words(words, NOT_ITERATED_PLAYER_TURN_DRAW_TRIGGER_SUFFIX)",
        "trigger_clause_shape_matches_words(words, NOT_YOUR_TURN_DRAW_TRIGGER_SUFFIX)",
        "trigger_clause_shape_matches_words(words, NOT_OPPONENTS_TURN_DRAW_TRIGGER_SUFFIX)",
        "ENTERS_FROM_YOUR_GRAVEYARD_ORIGIN_PATTERN",
        "ENTERS_FROM_GRAVEYARD_ORIGIN_PATTERN",
        "ENTERS_FROM_YOUR_HAND_ORIGIN_PATTERN",
        "trigger_clause_shape_matches_words(&tail_words, ENTERS_FROM_HAND_ORIGIN_PATTERN)",
        "trigger_clause_shape_matches_words(&tail_words, ENTERS_FROM_EXILE_ORIGIN_PATTERN)",
        "SOURCE_TRIGGER_CREATURE_SUBJECT_PATTERN",
        "SOURCE_TRIGGER_LAND_SUBJECT_PATTERN",
        "SOURCE_TRIGGER_ARTIFACT_SUBJECT_PATTERN",
        "SOURCE_TRIGGER_ENCHANTMENT_SUBJECT_PATTERN",
        "SOURCE_TRIGGER_PLANESWALKER_SUBJECT_PATTERN",
        "SOURCE_TRIGGER_BATTLE_SUBJECT_PATTERN",
        "trigger_clause_shape_matches_words(&words, PLAYERS_FINISH_VOTING_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, YOU_CYCLE_THIS_CARD_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, YOU_CYCLE_OR_DISCARD_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, YOU_COMMIT_CRIME_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, OPPONENT_COMMITS_CRIME_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, PLAYER_COMMITS_CRIME_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, YOU_UNLOCK_THIS_DOOR_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, THIS_CARD_BECOMES_PLOTTED_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, YOU_EXPEND_TRIGGER_PREFIX)",
        "trigger_clause_shape_matches_words(&words, OPPONENT_EXPENDS_WITH_ARTICLE_TRIGGER_PREFIX)",
        "trigger_clause_shape_matches_words(&words, OPPONENT_EXPENDS_TRIGGER_PREFIX)",
        "trigger_clause_shape_matches_words(&words, THE_RING_TEMPTS_YOU_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, CHAOS_ENSUES_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(tail_words, CYCLE_CARD_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(tail_words, CYCLE_ANOTHER_CARD_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(tail, EXERT_CREATURE_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(tail_words, CREW_VEHICLE_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(tail, EXPLORE_LAND_CARD_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(tail, EXPLORE_NONLAND_CARD_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(tail, NAME_STICKER_PUT_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(becomes_tapped_words, BECOMES_TAPPED_TRIGGER_SUFFIX)",
        "trigger_clause_shape_matches_words(becomes_tapped_words, THIS_BECOMES_TAPPED_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, THIS_BECOMES_UNTAPPED_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, THIS_BECOMES_MONSTROUS_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, BECOMES_MONSTROUS_TRIGGER_SUFFIX)",
        "trigger_clause_shape_matches_words(&words, THIS_MUTATES_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, MUTATES_TRIGGER_SUFFIX)",
        "trigger_clause_shape_matches_words(&words, THIS_TURNED_FACE_UP_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, TURNED_FACE_UP_TRIGGER_SUFFIX)",
        "BECOMES_TARGET_OF_PREFIX_PATTERN",
        "trigger_clause_shape_matches_words(tail_words, SPELL_OR_ABILITY_TARGET_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(tail_words, ONLY_IT_ABILITY_TARGET_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(tail_words, SPELL_OR_SPELLS_SUFFIX_PATTERN)",
        "trigger_clause_shape_matches_words(tail_words, BACKUP_ABILITY_TARGET_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(&words, SOURCE_DEALT_DAMAGE_TRIGGER_PREFIX)",
        "trigger_clause_shape_matches_words(&words, SOURCE_DEALT_COMBAT_DAMAGE_TRIGGER_PREFIX)",
        "trigger_clause_shape_matches_words(&words, SOURCE_DEALS_TRIGGER_PREFIX)",
        "trigger_clause_shape_matches_words(&words, SOURCE_DEALS_DAMAGE_TO_TRIGGER_PREFIX)",
        "trigger_clause_shape_matches_words(&words, SOURCE_DEALS_DAMAGE_TRIGGER_PREFIX)",
        "trigger_clause_shape_matches_words(&words, DAMAGE_WORD_PATTERN)",
        "trigger_clause_shape_matches_words(&amount_words, NONCOMBAT_DAMAGE_AMOUNT_PATTERN)",
        "trigger_clause_shape_matches_words(&words, YOU_GAIN_LIFE_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX)",
        "YOU_GAIN_LIFE_PREFIX_PATTERN",
        "trigger_clause_shape_matches_words(&words, LOSE_LIFE_TRIGGER_SUFFIX)",
        "trigger_clause_shape_matches_words(&words, LOSE_GAME_TRIGGER_SUFFIX)",
        "trigger_clause_shape_matches_words(&words[..words.len() - 3], LOSE_LIFE_TRIGGER_SUFFIX)",
        "trigger_clause_shape_matches_words(&words, DRAW_A_CARD_TRIGGER_SUFFIX)",
        "trigger_clause_shape_matches_words(subject, YOU_DRAW_CARD_TRIGGER_SUBJECT_PATTERN)",
        "OPPONENT_EFFECT_DISCARDS_THIS_CARD_TRIGGER_PATTERN",
        "trigger_clause_shape_matches_words(&tail_words, THIS_WAY_REVEAL_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(&filter_words, SOURCE_ARTIFACT_WORD_PATTERN)",
        "SOURCE_CREATURE_WORD_PATTERN",
        "SOURCE_ENCHANTMENT_WORD_PATTERN",
        "trigger_clause_shape_matches_words(&filter_words, SOURCE_LAND_WORD_PATTERN)",
        "SOURCE_PLANESWALKER_WORD_PATTERN",
        "trigger_clause_shape_matches_words(&words, YOU_OPEN_ATTRACTION_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, YOU_CLAIM_ATTRACTION_PRIZE_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(tail_words, EXPLOIT_CREATURE_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(&words, THIS_EXPLOITS_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, YOU_COMPLETE_DUNGEON_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, WINS_CLASH_TRIGGER_SUFFIX_PATTERN)",
        "PASSIVE_COUNTER_PUT_TAIL_PATTERN",
        "trigger_clause_shape_matches_words(&words, ONE_OR_MORE_PREFIX_PATTERN)",
        "trigger_clause_shape_matches_words(tail_words, ATTACKS_AND_IS_NOT_BLOCKED_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(&words, THIS_BLOCKS_OR_BECOMES_BLOCKED_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, THIS_BLOCKS_OR_BECOMES_BLOCKED_BY_TRIGGER_PREFIX)",
        "trigger_clause_shape_matches_words(&words, THIS_BLOCKS_PREFIX_PATTERN)",
        "trigger_clause_shape_matches_words(tail, ATTACKS_A_PLAYER_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(tail, ATTACKS_OPPONENT_TAIL_PATTERN)",
        "ATTACKS_DEFENDING_PLAYER_TAIL_PATTERN",
        "ATTACKS_OPPONENT_OR_PLANESWALKER_TAIL_PATTERN",
        "ATTACKS_PLANESWALKER_OR_BATTLE_TAIL_PATTERN",
        "trigger_clause_shape_matches_words(&words, CRAFT_EXILED_FROM_BATTLEFIELD_TRIGGER_PATTERN)",
        "FINAL_CHAPTER_ABILITY_RESOLVES_TRIGGER_PATTERN",
        "trigger_clause_shape_matches_words(&words, DAY_NIGHT_CHANGED_TRIGGER_PATTERN)",
        "SHARED_SUBJECT_ETB_OR_COMBAT_DAMAGE_TAIL_PATTERN",
        "SHARED_SUBJECT_ETB_OR_ATTACK_TAIL_PATTERN",
        "trigger_clause_shape_matches_words(&tail_words[..1], OR_WORD_PATTERN)",
        "ATTACKS_YOU_OR_PLANESWALKER_YOU_CONTROL_TAIL_PATTERN",
        "trigger_clause_shape_matches_words(&words, YOU_CAST_THIS_SPELL_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&searched_words, LIBRARY_SEARCH_TARGET_PATTERN)",
        "trigger_clause_shape_matches_words(&shuffled_words, LIBRARY_SHUFFLE_TARGET_PATTERN)",
        "trigger_clause_shape_matches_words(&gifted_words, GIFT_TAIL_PATTERN)",
        "trigger_clause_shape_matches_words(tail_words, ACTIVATED_ABILITY_TAIL_PATTERN)",
        "MANA_ABILITY_TAIL_PATTERN",
        "trigger_clause_shape_matches_words(&words, COMBAT_DAMAGE_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, DIES_DURING_YOUR_TURN_SUFFIX)",
        "trigger_clause_shape_matches_words(&words, BEGINNING_END_STEP_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, NEXT_END_STEP_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, BEGINNING_UPKEEP_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, BEGINNING_DRAW_STEP_TRIGGER_PATTERN)",
        "BEGINNING_FIRST_MAIN_PHASE_TRIGGER_PATTERN",
        "BEGINNING_SECOND_MAIN_PHASE_TRIGGER_PATTERN",
        "BEGINNING_PRECOMBAT_MAIN_TRIGGER_PATTERN",
        "BEGINNING_POSTCOMBAT_MAIN_TRIGGER_PATTERN",
        "trigger_clause_shape_matches_words(&words, BEGINNING_COMBAT_TRIGGER_PATTERN)",
        "trigger_clause_shape_matches_words(&words, THIS_LEAVES_BATTLEFIELD_TRIGGER_PATTERN)",
        "LEAVES_BATTLEFIELD_SUFFIX_PATTERN",
        "OR_IS_PUT_INTO_EXILE_FROM_BATTLEFIELD_TAIL_PATTERN",
        "trigger_clause_shape_matches_words(&words, ENTERS_OR_LEAVES_BATTLEFIELD_SUFFIX_PATTERN)",
        "OR_TRANSFORMS_INTO_TAIL_PREFIX_PATTERN",
        "trigger_clause_shape_matches_words(&words, UNDER_YOUR_CONTROL_PATTERN)",
        "trigger_clause_shape_matches_words(&words, UNDER_OPPONENT_CONTROL_PATTERN)",
        "trigger_clause_shape_matches_words(&words, UNTAPPED_WORD_PATTERN)",
        "trigger_clause_shape_matches_words(&words, TAPPED_WORD_PATTERN)",
        "trigger_clause_shape_matches_words(zone_change_words, ClauseShape::new().suffix(tail))",
    ] {
        assert!(
            trigger_clause_parser.contains(required),
            "{trigger_clause_relative} should route trigger clause parser shape gates through token-backed matching: missing `{required}`"
        );
    }
    for forbidden in [
        "NOT_ITERATED_PLAYER_TURN_DRAW_TRIGGER_SUFFIX.matches_words(words)",
        "NOT_YOUR_TURN_DRAW_TRIGGER_SUFFIX.matches_words(words)",
        "NOT_OPPONENTS_TURN_DRAW_TRIGGER_SUFFIX.matches_words(words)",
        "ENTERS_FROM_YOUR_GRAVEYARD_ORIGIN_PATTERN.matches_words(&tail_words)",
        "ENTERS_FROM_GRAVEYARD_ORIGIN_PATTERN.matches_words(&tail_words)",
        "ENTERS_FROM_YOUR_HAND_ORIGIN_PATTERN.matches_words(&tail_words)",
        "ENTERS_FROM_HAND_ORIGIN_PATTERN.matches_words(&tail_words)",
        "ENTERS_FROM_EXILE_ORIGIN_PATTERN.matches_words(&tail_words)",
        "SOURCE_TRIGGER_CREATURE_SUBJECT_PATTERN.matches_words(subject_words)",
        "SOURCE_TRIGGER_LAND_SUBJECT_PATTERN.matches_words(subject_words)",
        "SOURCE_TRIGGER_ARTIFACT_SUBJECT_PATTERN.matches_words(subject_words)",
        "SOURCE_TRIGGER_ENCHANTMENT_SUBJECT_PATTERN.matches_words(subject_words)",
        "SOURCE_TRIGGER_PLANESWALKER_SUBJECT_PATTERN.matches_words(subject_words)",
        "SOURCE_TRIGGER_BATTLE_SUBJECT_PATTERN.matches_words(subject_words)",
        "PLAYERS_FINISH_VOTING_TRIGGER_PATTERN.matches_words(&words)",
        "YOU_CYCLE_THIS_CARD_TRIGGER_PATTERN.matches_words(&words)",
        "YOU_CYCLE_OR_DISCARD_TRIGGER_PATTERN.matches_words(&words)",
        "YOU_COMMIT_CRIME_TRIGGER_PATTERN.matches_words(&words)",
        "OPPONENT_COMMITS_CRIME_TRIGGER_PATTERN.matches_words(&words)",
        "PLAYER_COMMITS_CRIME_TRIGGER_PATTERN.matches_words(&words)",
        "YOU_UNLOCK_THIS_DOOR_TRIGGER_PATTERN.matches_words(&words)",
        "THIS_CARD_BECOMES_PLOTTED_TRIGGER_PATTERN.matches_words(&words)",
        "YOU_EXPEND_TRIGGER_PREFIX.matches_words(&words)",
        "OPPONENT_EXPENDS_WITH_ARTICLE_TRIGGER_PREFIX.matches_words(&words)",
        "OPPONENT_EXPENDS_TRIGGER_PREFIX.matches_words(&words)",
        "THE_RING_TEMPTS_YOU_TRIGGER_PATTERN.matches_words(&words)",
        "CHAOS_ENSUES_TRIGGER_PATTERN.matches_words(&words)",
        "CYCLE_CARD_TAIL_PATTERN.matches_words(tail_words)",
        "CYCLE_ANOTHER_CARD_TAIL_PATTERN.matches_words(tail_words)",
        "EXERT_CREATURE_TAIL_PATTERN.matches_words(tail)",
        "CREW_VEHICLE_TAIL_PATTERN.matches_words(tail_words)",
        "EXPLORE_LAND_CARD_TAIL_PATTERN.matches_words(tail)",
        "EXPLORE_NONLAND_CARD_TAIL_PATTERN.matches_words(tail)",
        "NAME_STICKER_PUT_TAIL_PATTERN.matches_words(tail)",
        "BECOMES_TAPPED_TRIGGER_SUFFIX.matches_words(&words)",
        "THIS_BECOMES_TAPPED_TRIGGER_PATTERN.matches_words(&words)",
        "THIS_BECOMES_UNTAPPED_TRIGGER_PATTERN.matches_words(&words)",
        "THIS_BECOMES_MONSTROUS_TRIGGER_PATTERN.matches_words(&words)",
        "BECOMES_MONSTROUS_TRIGGER_SUFFIX.matches_words(&words)",
        "THIS_MUTATES_TRIGGER_PATTERN.matches_words(&words)",
        "MUTATES_TRIGGER_SUFFIX.matches_words(&words)",
        "THIS_TURNED_FACE_UP_TRIGGER_PATTERN.matches_words(&words)",
        "TURNED_FACE_UP_TRIGGER_SUFFIX.matches_words(&words)",
        "BECOMES_TARGET_OF_PREFIX_PATTERN.matches_words(&words[becomes_idx + 1..])",
        "SPELL_OR_ABILITY_TARGET_TAIL_PATTERN.matches_words(tail_words)",
        "ONLY_IT_ABILITY_TARGET_TAIL_PATTERN.matches_words(tail_words)",
        "SPELL_OR_SPELLS_SUFFIX_PATTERN.matches_words(tail_words)",
        "BACKUP_ABILITY_TARGET_TAIL_PATTERN.matches_words(tail_words)",
        "SOURCE_DEALT_DAMAGE_TRIGGER_PREFIX.matches_words(&words)",
        "SOURCE_DEALT_COMBAT_DAMAGE_TRIGGER_PREFIX.matches_words(&words)",
        "SOURCE_DEALS_TRIGGER_PREFIX.matches_words(&words)",
        "SOURCE_DEALS_DAMAGE_TO_TRIGGER_PREFIX.matches_words(&words)",
        "SOURCE_DEALS_DAMAGE_TRIGGER_PREFIX.matches_words(&words)",
        "DAMAGE_WORD_PATTERN.matches_words(&words)",
        "NONCOMBAT_DAMAGE_AMOUNT_PATTERN.matches_words(&amount_words)",
        "YOU_GAIN_LIFE_TRIGGER_PATTERN.matches_words(&words)",
        "DURING_YOUR_TURN_TRIGGER_SUFFIX.matches_words(&words)",
        "YOU_GAIN_LIFE_PREFIX_PATTERN.matches_words(&words[..words.len() - 3])",
        "LOSE_LIFE_TRIGGER_SUFFIX.matches_words(&words)",
        "LOSE_GAME_TRIGGER_SUFFIX.matches_words(&words)",
        "LOSE_LIFE_TRIGGER_SUFFIX.matches_words(&words[..words.len() - 3])",
        "DRAW_A_CARD_TRIGGER_SUFFIX.matches_words(&words)",
        "YOU_DRAW_CARD_TRIGGER_SUBJECT_PATTERN.matches_words(subject)",
        "OPPONENT_EFFECT_DISCARDS_THIS_CARD_TRIGGER_PATTERN.matches_words(&words)",
        "THIS_WAY_REVEAL_TAIL_PATTERN.matches_words(&tail_words)",
        "SOURCE_ARTIFACT_WORD_PATTERN.matches_words(&filter_words)",
        "SOURCE_CREATURE_WORD_PATTERN.matches_words(&filter_words)",
        "SOURCE_ENCHANTMENT_WORD_PATTERN.matches_words(&filter_words)",
        "SOURCE_LAND_WORD_PATTERN.matches_words(&filter_words)",
        "SOURCE_PLANESWALKER_WORD_PATTERN.matches_words(&filter_words)",
        "YOU_OPEN_ATTRACTION_TRIGGER_PATTERN.matches_words(&words)",
        "YOU_CLAIM_ATTRACTION_PRIZE_TRIGGER_PATTERN.matches_words(&words)",
        "EXPLOIT_CREATURE_TAIL_PATTERN.matches_words(tail_words)",
        "THIS_EXPLOITS_TRIGGER_PATTERN.matches_words(&words)",
        "YOU_COMPLETE_DUNGEON_TRIGGER_PATTERN.matches_words(&words)",
        "WINS_CLASH_TRIGGER_SUFFIX_PATTERN.matches_words(&words)",
        "PASSIVE_COUNTER_PUT_TAIL_PATTERN.matches_words(&words[counter_word_idx..])",
        "ONE_OR_MORE_PREFIX_PATTERN.matches_words(&words)",
        "ATTACKS_AND_IS_NOT_BLOCKED_TAIL_PATTERN.matches_words(tail_words)",
        "THIS_BLOCKS_OR_BECOMES_BLOCKED_TRIGGER_PATTERN.matches_words(&words)",
        "THIS_BLOCKS_OR_BECOMES_BLOCKED_BY_TRIGGER_PREFIX.matches_words(&words)",
        "THIS_BLOCKS_PREFIX_PATTERN.matches_words(&words)",
        "ATTACKS_A_PLAYER_TAIL_PATTERN.matches_words(tail)",
        "ATTACKS_OPPONENT_TAIL_PATTERN.matches_words(tail)",
        "ATTACKS_DEFENDING_PLAYER_TAIL_PATTERN.matches_words(tail)",
        "ATTACKS_OPPONENT_OR_PLANESWALKER_TAIL_PATTERN.matches_words(tail)",
        "ATTACKS_PLANESWALKER_OR_BATTLE_TAIL_PATTERN.matches_words(tail)",
        "CRAFT_EXILED_FROM_BATTLEFIELD_TRIGGER_PATTERN.matches_words(&words)",
        "FINAL_CHAPTER_ABILITY_RESOLVES_TRIGGER_PATTERN.matches_words(&words)",
        "DAY_NIGHT_CHANGED_TRIGGER_PATTERN.matches_words(&words)",
        "SHARED_SUBJECT_ETB_OR_COMBAT_DAMAGE_TAIL_PATTERN.matches_words(&tail_words)",
        "SHARED_SUBJECT_ETB_OR_ATTACK_TAIL_PATTERN.matches_words(&tail_words)",
        "OR_WORD_PATTERN.matches_words(&tail_words[..1])",
        "ATTACKS_YOU_OR_PLANESWALKER_YOU_CONTROL_TAIL_PATTERN.matches_words(tail_words)",
        "YOU_CAST_THIS_SPELL_TRIGGER_PATTERN.matches_words(&words)",
        "LIBRARY_SEARCH_TARGET_PATTERN.matches_words(&searched_words)",
        "LIBRARY_SHUFFLE_TARGET_PATTERN.matches_words(&shuffled_words)",
        "GIFT_TAIL_PATTERN.matches_words(&gifted_words)",
        "ACTIVATED_ABILITY_TAIL_PATTERN.matches_words(tail_words)",
        "MANA_ABILITY_TAIL_PATTERN.matches_words(tail_words)",
        "COMBAT_DAMAGE_TRIGGER_PATTERN.matches_words(&words)",
        "DIES_DURING_YOUR_TURN_SUFFIX.matches_words(&words)",
        "BEGINNING_END_STEP_TRIGGER_PATTERN.matches_words(&words)",
        "NEXT_END_STEP_TRIGGER_PATTERN.matches_words(&words)",
        "BEGINNING_UPKEEP_TRIGGER_PATTERN.matches_words(&words)",
        "BEGINNING_DRAW_STEP_TRIGGER_PATTERN.matches_words(&words)",
        "BEGINNING_FIRST_MAIN_PHASE_TRIGGER_PATTERN.matches_words(&words)",
        "BEGINNING_SECOND_MAIN_PHASE_TRIGGER_PATTERN.matches_words(&words)",
        "BEGINNING_PRECOMBAT_MAIN_TRIGGER_PATTERN.matches_words(&words)",
        "BEGINNING_POSTCOMBAT_MAIN_TRIGGER_PATTERN.matches_words(&words)",
        "BEGINNING_COMBAT_TRIGGER_PATTERN.matches_words(&words)",
        "THIS_LEAVES_BATTLEFIELD_TRIGGER_PATTERN.matches_words(&words)",
        "LEAVES_BATTLEFIELD_SUFFIX_PATTERN.matches_words(&words[leaves_word_idx..])",
        "OR_IS_PUT_INTO_EXILE_FROM_BATTLEFIELD_TAIL_PATTERN\n                .matches_words(&words[dies_word_idx + 1..])",
        "ENTERS_OR_LEAVES_BATTLEFIELD_SUFFIX_PATTERN.matches_words(&words)",
        "OR_TRANSFORMS_INTO_TAIL_PREFIX_PATTERN.matches_words(&words[enters_word_idx + 1..])",
        "UNDER_YOUR_CONTROL_PATTERN.matches_words(&words)",
        "UNDER_OPPONENT_CONTROL_PATTERN.matches_words(&words)",
        "UNTAPPED_WORD_PATTERN.matches_words(&words)",
        "TAPPED_WORD_PATTERN.matches_words(&words)",
        "ClauseShape::new()\n            .suffix(tail)\n            .matches_words(zone_change_words)",
    ] {
        assert!(
            !trigger_clause_parser.contains(forbidden),
            "{trigger_clause_relative} should not route trigger clause parser shape gates through raw word refs: found `{forbidden}`"
        );
    }
    for forbidden in [".matches_word(", ".matches_token(", ".matches_words("] {
        assert!(
            !trigger_clause.contains(forbidden),
            "{trigger_clause_relative} should not call ClauseShape word/token adapter methods directly: found `{forbidden}`"
        );
    }

    let abilities_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/abilities.rs";
    let abilities = read_repo_file(&root, abilities_relative);
    for required in [
        "enum ActivateOnlyTimingMarker",
        "fn parse_activate_only_timing_marker(tokens: &[OwnedLexToken]) -> Option<ActivateOnlyTimingMarker>",
        "const ONCE_EACH_TURN_MARKER_PATTERN: LexPattern<'static>",
        "const DURING_COMBAT_MARKER_PATTERN: LexPattern<'static>",
        "const DURING_YOUR_TURN_MARKER_PATTERN: LexPattern<'static>",
        "const DURING_OPPONENTS_TURN_MARKER_PATTERN: LexPattern<'static>",
        "LexPattern::action(\n            \"timing\",",
        "pattern.find_in_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Action, clause)",
        "let timing_marker = parse_activate_only_timing_marker(tokens)",
        "ActivateOnlyTimingMarker::OnceEachTurn",
        "ActivateOnlyTimingMarker::DuringCombat",
        "ActivateOnlyTimingMarker::DuringYourTurn",
        "ActivateOnlyTimingMarker::DuringOpponentsTurn",
        "enum ManaUsageSpecShape",
        "fn classify_mana_usage_spec_shape(spec_clause: LexedClause<'_>) -> ManaUsageSpecShape",
        "const UNSUPPORTED_MARKER_PATTERN: LexPattern<'static>",
        "LexCaptureKind::OneOf(MANA_USAGE_UNSUPPORTED_MARKER_WORDS)",
        "UNSUPPORTED_MARKER_PATTERN.find_in_clause(spec_clause)",
        "capture_clause_by_role(LexCaptureRole::Action, spec_clause)",
        "let spec_shape = classify_mana_usage_spec_shape(spec_clause)",
        "spec_shape == ManaUsageSpecShape::Unsupported",
        "spec_shape == ManaUsageSpecShape::PlainSpell",
    ] {
        assert!(
            abilities.contains(required),
            "{abilities_relative} should route ability marker gates through LexedClause matching: missing `{required}`"
        );
    }
    for forbidden in [
        "ONCE_EACH_TURN_MARKER_PATTERN.matches(clause)",
        "DURING_COMBAT_MARKER_PATTERN.matches(clause)",
        "DURING_YOUR_TURN_MARKER_PATTERN.matches(clause)",
        "DURING_OPPONENTS_TURN_MARKER_PATTERN.matches(clause)",
        "ONCE_EACH_TURN_MARKER_PATTERN.matches_words(&words)",
        "DURING_COMBAT_MARKER_PATTERN.matches_words(&words)",
        "DURING_YOUR_TURN_MARKER_PATTERN.matches_words(&words)",
        "DURING_OPPONENTS_TURN_MARKER_PATTERN.matches_words(&words)",
        "MANA_USAGE_UNSUPPORTED_MARKER_PATTERN",
        "PLAIN_SPELL_USAGE_WORD_PATTERN",
        "MANA_USAGE_UNSUPPORTED_MARKER_PATTERN.matches_words(&spec_words)",
    ] {
        assert!(
            !abilities.contains(forbidden),
            "{abilities_relative} should not route ability marker gates through raw word refs: found `{forbidden}`"
        );
    }

    let activated_lowering_relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/activated_lowering.rs";
    let activated_lowering = read_repo_file(&root, activated_lowering_relative);
    for required in [
        "fn is_activation_mana_source_restriction_sentence(tokens: &[OwnedLexToken])",
        "word_slice_starts_with(&words, ACTIVATION_MANA_SOURCE_RESTRICTION_PREFIX)",
        "word_slice_ends_with(&words, ACTIVATION_MANA_SOURCE_RESTRICTION_SUFFIX)",
        "tokens_start_with_phrase(&tokens, SPEND_THIS_MANA_ONLY_PREFIX)",
        "tokens_start_with_phrase(&tokens, WHEN_YOU_SPEND_THIS_MANA_TO_CAST_PREFIX)",
        "word_slice_starts_with(&words, WHERE_X_IS_PREFIX)",
    ] {
        assert!(
            activated_lowering.contains(required),
            "{activated_lowering_relative} should route activated lowering mana gates through token word matching: missing `{required}`"
        );
    }
    for forbidden in [
        "fn is_activation_mana_source_restriction_sentence(words: &[&str])",
        "ClauseShape",
        "clause_shape",
        "LexedClause::new(&tokens)",
        "ACTIVATION_MANA_SOURCE_RESTRICTION_PATTERN.matches_words(words)",
        "ACTIVATION_MANA_SOURCE_RESTRICTION_PATTERN",
        "SPEND_THIS_MANA_ONLY_PREFIX_PATTERN.matches_words(sentence_words.as_slice())",
        "SPEND_THIS_MANA_ONLY_PREFIX_PATTERN",
        "WHEN_YOU_SPEND_THIS_MANA_TO_CAST_PREFIX_PATTERN\n                .matches_words(sentence_words.as_slice())",
        "WHEN_YOU_SPEND_THIS_MANA_TO_CAST_PREFIX_PATTERN",
        "WHERE_X_IS_PREFIX_PATTERN.matches_words(&words)",
        "WHERE_X_IS_PREFIX_PATTERN",
    ] {
        assert!(
            !activated_lowering.contains(forbidden),
            "{activated_lowering_relative} should not route activated lowering mana gates through raw word refs: found `{forbidden}`"
        );
    }

    let lower_mod_relative = "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/mod.rs";
    let lower_mod = read_repo_file(&root, lower_mod_relative);
    for required in [
        "word_slice_contains_phrase(&token_word_refs(tokens), BECOMES_TAPPED_DURING_YOUR_TURN_PHRASE)",
        "word_slice_contains_phrase(&words, DO_THIS_ONLY_ONCE_EACH_TURN_PHRASE)",
        "word_slice_contains_phrase(&words, DO_THIS_ONLY_TWICE_EACH_TURN_PHRASE)",
        "word_slice_starts_with(&token_word_refs(tokens), &[\"remove\"])",
        "word_slice_starts_with(&token_word_refs(tokens), &[\"level\", \"up\"])",
        "word_slice_contains_any_phrase(&token_word_refs(tokens), DAMAGE_TO_EACH_PLAYER_CREATURES_PHRASES)",
        "word_slice_starts_with(&words, BLOCKS_OR_BECOMES_BLOCKED_FIRST_STRIKE_PREFIX)",
        "word_slice_ends_with(&words, BLOCKS_OR_BECOMES_BLOCKED_FIRST_STRIKE_SUFFIX)",
        "word_slice_contains_any_phrase(&words, AS_THIS_ENTERS_PHRASES)",
    ] {
        assert!(
            lower_mod.contains(required),
            "{lower_mod_relative} should route lower/mod helper gates through token word matching: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "BECOMES_TAPPED_DURING_YOUR_TURN_PATTERN",
        "DO_THIS_ONLY_ONCE_EACH_TURN_PATTERN",
        "DO_THIS_ONLY_TWICE_EACH_TURN_PATTERN",
        "REMOVE_PREFIX_PATTERN",
        "LEVEL_UP_PREFIX_PATTERN",
        "DAMAGE_TO_EACH_PLAYER_CREATURES_PATTERN",
        "BLOCKS_OR_BECOMES_BLOCKED_FIRST_STRIKE_PATTERN",
        "DAY_NIGHT_STARTS_DAY_PATTERN",
        "BECOMES_TAPPED_DURING_YOUR_TURN_PATTERN.matches_words(&token_word_refs(tokens))",
        "DO_THIS_ONLY_ONCE_EACH_TURN_PATTERN.matches_words(&words)",
        "DO_THIS_ONLY_TWICE_EACH_TURN_PATTERN.matches_words(&words)",
        "REMOVE_PREFIX_PATTERN.matches_words(&token_word_refs(tokens))",
        "LEVEL_UP_PREFIX_PATTERN.matches_words(&token_word_refs(tokens))",
        "DAMAGE_TO_EACH_PLAYER_CREATURES_PATTERN.matches_words(&token_word_refs(tokens))",
        "BLOCKS_OR_BECOMES_BLOCKED_FIRST_STRIKE_PATTERN.matches_words(&token_word_refs(tokens))",
        "DAY_NIGHT_STARTS_DAY_PATTERN.matches_words(&token_word_refs(tokens))",
    ] {
        assert!(
            !lower_mod.contains(forbidden),
            "{lower_mod_relative} should not route lower/mod helper gates through ClauseShape/raw word refs: found `{forbidden}`"
        );
    }

    let rewrite_support_relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/rewrite_support.rs";
    let rewrite_support = read_repo_file(&root, rewrite_support_relative);
    for required in [
        "let words = token_word_refs(tokens)",
        "word_slice_contains_any_phrase(&words, STATIC_LIBRARY_SEARCH_ZONE_PHRASES)",
        "word_slice_contains_phrase(&words, FROM_YOUR_LIBRARY_PHRASE)",
        "word_slice_contains_any_phrase(&words, CAST_OR_PLAY_SELF_FROM_GRAVEYARD_PHRASES)",
        "word_slice_contains_any_phrase(&words, CAST_OR_PLAY_SELF_FROM_EXILE_PHRASES)",
        "word_slice_contains_phrase(&words, CAUSES_YOU_TO_DISCARD_THIS_CARD_PHRASE)",
        "word_slice_contains_phrase(&words, INSTEAD_OF_PUTTING_IT_INTO_YOUR_GRAVEYARD_PHRASE)",
        "word_slice_contains_any_phrase(&words, RETURN_SELF_FROM_GRAVEYARD_PHRASES)",
        "word_slice_contains_phrase(&words, DISCARD_THIS_CARD_PHRASE)",
    ] {
        assert!(
            rewrite_support.contains(required),
            "{rewrite_support_relative} should route rewrite support zone gates through token word matching: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "LexedClause",
        "STATIC_LIBRARY_SEARCH_ZONE_PATTERN",
        "CAST_OR_PLAY_SELF_FROM_GRAVEYARD_PATTERN",
        "CAST_OR_PLAY_SELF_FROM_EXILE_PATTERN",
        "DISCARD_TO_BATTLEFIELD_REPLACEMENT_ZONE_PATTERN",
        "RETURN_SELF_FROM_GRAVEYARD_PATTERN",
        "DISCARD_THIS_CARD_PATTERN",
        "STATIC_LIBRARY_SEARCH_ZONE_PATTERN.matches_words(&words)",
        "CAST_OR_PLAY_SELF_FROM_GRAVEYARD_PATTERN.matches_words(&words)",
        "CAST_OR_PLAY_SELF_FROM_EXILE_PATTERN.matches_words(&words)",
        "DISCARD_TO_BATTLEFIELD_REPLACEMENT_ZONE_PATTERN.matches_words(&words)",
        "RETURN_SELF_FROM_GRAVEYARD_PATTERN.matches_words(&words)",
        "DISCARD_THIS_CARD_PATTERN.matches_words(&words)",
        "fn token_words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool",
        ".matches_words(words)",
    ] {
        assert!(
            !rewrite_support.contains(forbidden),
            "{rewrite_support_relative} should not route rewrite support zone gates through raw word refs: found `{forbidden}`"
        );
    }

    let keyword_activated_relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/keyword_activated_lines.rs";
    let keyword_activated = read_repo_file(&root, keyword_activated_relative);
    for required in [
        "CRAFT_WITH_PREFIX_PATTERN.matches_prefix(LexedClause::new(tokens))",
        "words.as_slice() == [\"artifact\"]",
        "words.as_slice() == [\"creature\"]",
        "words.as_slice() == [\"one\", \"or\", \"more\"]",
        "CRAFT_RED_INSTANT_SORCERY_MATERIAL_TAIL_PATTERN.matches_clause(tail)",
        "PAY_LIFE_COST_PATTERN.matches_clause(LexedClause::new(&cost_tokens))",
        "*word == \"equip\"",
        "matches!(*word, \"or\" | \"and\" | \"and/or\")",
    ] {
        assert!(
            keyword_activated.contains(required),
            "{keyword_activated_relative} should route keyword activated gates through LexedClause/token helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "CRAFT_WITH_PREFIX_PATTERN.matches_words(&words)",
        "CRAFT_ARTIFACT_MATERIAL_PATTERN.matches_words(&words)",
        "CRAFT_CREATURE_MATERIAL_PATTERN.matches_words(&words)",
        "CRAFT_ONE_OR_MORE_MATERIAL_PATTERN.matches_words(&words)",
        "CRAFT_RED_INSTANT_SORCERY_MATERIAL_TAIL_PATTERN.matches_words(&words[used..])",
        "PAY_LIFE_COST_PATTERN.matches_words(&cost_words)",
        "EQUIP_WORD_PATTERN.matches_words(&[*word])",
        "CHANNEL_WORD_PATTERN",
        "CYCLING_WORD_PATTERN",
        "LANDCYCLING_WORD_PATTERN",
        "EQUIP_WORD_PATTERN",
        "EQUIP_SUBTYPE_CONNECTOR_PATTERN",
        "CREATURE_WORD_PATTERN",
        "RECONFIGURE_WORD_PATTERN",
        "CRAFT_ARTIFACT_MATERIAL_PATTERN",
        "CRAFT_CREATURE_MATERIAL_PATTERN",
        "CRAFT_ONE_OR_MORE_MATERIAL_PATTERN",
    ] {
        assert!(
            !keyword_activated.contains(forbidden),
            "{keyword_activated_relative} should not route keyword activated gates through raw word refs: found `{forbidden}`"
        );
    }

    let activated_sentence_parsers_relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activated_sentence_parsers.rs";
    let activated_sentence_parsers = read_repo_file(&root, activated_sentence_parsers_relative);
    for required in [
        "use super::super::lex_patterns::LexPattern",
        "fn activated_sentence_tokens_match_pattern",
        "pattern.matches(LexedClause::new(tokens))",
        "fn activated_sentence_words_start_with_at",
        "const AND_ONLY_ONCE_EACH_TURN_WORDS",
        "activated_sentence_tokens_match_pattern(tokens, ACTIVATE_ONLY_ONCE_EACH_TURN_PATTERN)",
        "activated_sentence_words_start_with_at(&words, index, AND_ONLY_ONCE_EACH_TURN_WORDS)",
        "NEXT_SPELL_YOU_CAST_THIS_TURN_TAIL_PATTERN.matches_prefix(tail)",
        "LESS_TO_CAST_PATTERN.matches_prefix(tail)",
        "LESS_TO_ACTIVATE_MARKER_PATTERN",
        ".find_in_clause(clause)",
        "NEXT_SPELL_COST_REDUCTION_MARKER_WORDS",
        ".all(|word| clause.contains_word(word))",
        "EXHAUST_ONCE_RESTRICTION_PATTERN.matches(LexedClause::new(tokens))",
    ] {
        assert!(
            activated_sentence_parsers.contains(required),
            "{activated_sentence_parsers_relative} should route activated sentence parser gates through LexPattern/token helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "fn activated_sentence_shape_matches_words",
        "fn activated_sentence_words_match_pattern",
        "synthetic_word_tokens(words)",
        "AND_ONLY_ONCE_EACH_TURN_PATTERN",
        "activated_sentence_words_match_pattern(",
        "NEXT_SPELL_YOU_CAST_THIS_TURN_TAIL_PATTERN.matches_words(&clause_words[spell_idx..])",
        "LESS_TO_CAST_PATTERN.matches_words(&clause_words[less_idx..])",
        "LESS_TO_ACTIVATE_MARKER_PATTERN.matches_words(&clause_words)",
        "NEXT_SPELL_COST_REDUCTION_MARKER_PATTERN.matches_words(&clause_words)",
        "NEXT_SPELL_COST_REDUCTION_MARKER_PATTERN",
        "EXHAUST_ONCE_RESTRICTION_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !activated_sentence_parsers.contains(forbidden),
            "{activated_sentence_parsers_relative} should not route activated sentence parser gates through raw word refs: found `{forbidden}`"
        );
    }
    assert!(
        !activated_sentence_parsers.contains(".matches_words("),
        "{activated_sentence_parsers_relative} should not route activated sentence parser gates through raw word refs"
    );

    let conditionals_relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/conditionals.rs";
    let conditionals = read_repo_file(&root, conditionals_relative);
    for required in [
        "word_slice_eq(&clause_words, COUNTER_TARGET_SPELL_IF_KICKED_WORDS)",
        "word_slice_eq_any(\n        &clause_words,\n        COUNTER_TARGET_SECOND_SPELL_CAST_THIS_TURN_WORDS,\n    )",
        "word_slice_starts_with(&clause_words, EXILE_TARGET_CREATURE_PREFIX)",
        "word_slice_contains_phrase(&clause_words, GREATEST_POWER_AMONG_CREATURES_PHRASE)",
        "word_slice_contains_any_phrase(&clause_words, ON_BATTLEFIELD_PHRASES)",
    ] {
        assert!(
            conditionals.contains(required),
            "{conditionals_relative} should route conditional sentence gates through token word matching: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "LexedClause::new(tokens)",
        "COUNTER_TARGET_SPELL_IF_KICKED_PATTERN",
        "COUNTER_TARGET_SECOND_SPELL_CAST_THIS_TURN_PATTERN",
        "EXILE_TARGET_CREATURE_PREFIX_PATTERN",
        "GREATEST_POWER_AMONG_CREATURES_PATTERN",
        "ON_BATTLEFIELD_PATTERN",
        "COUNTER_TARGET_SPELL_IF_KICKED_PATTERN.matches_words(&clause_words)",
        "COUNTER_TARGET_SECOND_SPELL_CAST_THIS_TURN_PATTERN.matches_words(&clause_words)",
        "EXILE_TARGET_CREATURE_PREFIX_PATTERN.matches_words(&clause_words)",
        "GREATEST_POWER_AMONG_CREATURES_PATTERN.matches_words(&clause_words)",
        "ON_BATTLEFIELD_PATTERN.matches_words(&clause_words)",
    ] {
        assert!(
            !conditionals.contains(forbidden),
            "{conditionals_relative} should not route conditional sentence gates through raw word refs: found `{forbidden}`"
        );
    }

    let primitive_registry_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/registry.rs";
    let primitive_registry = read_repo_file(&root, primitive_registry_relative);
    for required in [
        "token.as_word().is_some_and(|word| word == expected)",
        "fn registry_word_is_card_or_cards(word: &str) -> bool",
        "word_slice_eq_any(&words, REGISTRY_TARGET_OPPONENT_OBJECT_WORDS)",
        "word_slice_eq_any(&words, REGISTRY_TARGET_PLAYER_OBJECT_WORDS)",
        "word_slice_eq_any(&words, REGISTRY_THAT_PLAYER_OBJECT_WORDS)",
        "subject_clause.word_refs() != YOU_SUBJECT_WORDS",
    ] {
        assert!(
            primitive_registry.contains(required),
            "{primitive_registry_relative} should route primitive registry gates through captures plus reusable token word helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "ClauseShape::new()\n        .exact(&[expected])\n        .matches_words",
        "REGISTRY_TARGET_OPPONENT_OBJECT_PATTERN.matches(object_clause.lexed())",
        "REGISTRY_TARGET_PLAYER_OBJECT_PATTERN.matches(object_clause.lexed())",
        "REGISTRY_THAT_PLAYER_OBJECT_PATTERN.matches(object_clause.lexed())",
        "REGISTRY_YOU_SUBJECT_PATTERN.matches(subject_clause.lexed())",
        "REGISTRY_TARGET_OPPONENT_OBJECT_PATTERN.matches_words(&object_words)",
        "REGISTRY_TARGET_PLAYER_OBJECT_PATTERN.matches_words(&object_words)",
        "REGISTRY_THAT_PLAYER_OBJECT_PATTERN.matches_words(&object_words)",
        "REGISTRY_YOU_SUBJECT_PATTERN.matches_words(&subject_clause.word_refs())",
    ] {
        assert!(
            !primitive_registry.contains(forbidden),
            "{primitive_registry_relative} should not route primitive registry gates through raw word refs: found `{forbidden}`"
        );
    }

    let sentence_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/sentence_registry.rs";
    let sentence = read_repo_file(&root, sentence_relative);
    assert!(
        sentence.contains("word_slice_eq_any(words.as_slice(), X_CANT_BE_ZERO_WORDS)"),
        "{sentence_relative} should route the X-can't-be-zero gate through token word matching"
    );
    assert!(
        !sentence.contains("X_CANT_BE_ZERO_PATTERN")
            && !sentence.contains("ClauseShape")
            && !sentence.contains(".matches_words("),
        "{sentence_relative} should not route the X-can't-be-zero gate through ClauseShape/raw word refs"
    );

    let cost_relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/damage_and_cost_rewrites.rs";
    let cost = read_repo_file(&root, cost_relative);
    assert!(
        cost.contains("word_slice_starts_with(clause_words.as_slice(), &[\"the\", \"next\"])"),
        "{cost_relative} should route the next-spell cost-reduction gate through token word matching"
    );
    assert!(
        !cost.contains("NEXT_SPELL_COST_REDUCTION_PREFIX_PATTERN")
            && !cost.contains("ClauseShape")
            && !cost.contains(".matches_words("),
        "{cost_relative} should not route the next-spell cost-reduction gate through ClauseShape/raw word refs"
    );

    let statement_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/statement_cst_support.rs";
    let statement = read_repo_file(&root, statement_relative);
    assert!(
        statement.contains("fn is_die_roll_result_adjustment_statement(tokens: &[OwnedLexToken])")
            && statement.contains(
                "DIE_ROLL_RESULT_ADJUSTMENT_PREFIX_PATTERN.matches_prefix(LexedClause::new(tokens))"
            )
            && statement
                .contains("contains_token_word_sequence(tokens, &[\"you\", \"may\", \"pay\"])"),
        "{statement_relative} should route the die-roll statement gate through token-backed shape helpers"
    );
    assert!(
        !statement.contains(
            "DIE_ROLL_RESULT_ADJUSTMENT_PATTERN.matches_words(&token_word_refs(&line.tokens))"
        ) && !statement.contains("DIE_ROLL_RESULT_ADJUSTMENT_PATTERN"),
        "{statement_relative} should not route the die-roll statement gate through raw word refs or ClauseShape constants"
    );
    assert!(
        statement.contains(
            "REVEAL_THIS_CARD_FROM_HAND_PATTERN.matches_clause(LexedClause::new(left_tokens))"
        ),
        "{statement_relative} should route the reveal-from-hand colon fallback through LexedClause matching"
    );
    assert!(
        !statement.contains(
            "REVEAL_THIS_CARD_FROM_HAND_PATTERN.matches_words(&token_word_refs(left_tokens))"
        ),
        "{statement_relative} should not route the reveal-from-hand colon fallback through raw word refs"
    );
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
    assert!(
        labeled_prefix_support
            .contains("parser_token_word_refs(&tokens).as_slice() == MAX_SPEED_LABEL"),
        "{relative} should classify exact label-prefix gates from parser token words"
    );
    for forbidden in [
        "MAX_SPEED_LABEL_PATTERN",
        "MAX_SPEED_LABEL_PATTERN.matches_words(&words)",
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
fn grammar_effect_direct_shape_gates_use_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "token_word_refs(&tokens[cursor - 2..cursor]).as_slice() == IF_YOU_PHRASE",
        "token_word_refs(&core_tokens).as_slice() == THAT_WOULD_BE_DEALT_PHRASE",
        "words_contain_all(&words, LOSE_MANA_STEPS_PHASES_END_WORDS)",
        "TRAILING_THAT_PLAYER_SHUFFLE_PHRASES",
        "token_slice_starts_with(&raw_filter_tokens, THAT_MANY_PREFIX)",
        "tail.word_refs().as_slice() == THIS_TURN_PHRASE",
        "tokens_contain_any_non_article_word(tokens, &[\"target\", \"this\", \"that\", \"it\"])",
        "matches!(target_words.as_slice(), [\"player\"] | [\"players\"])",
        "target_words.as_slice() == [\"you\"]",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct grammar effect shape gates through token/phrase helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "IF_YOU_PREFIX_PATTERN",
        "THAT_WOULD_BE_DEALT_PATTERN",
        "LOSE_MANA_STEPS_PHASES_END_PATTERN",
        "TRAILING_THAT_PLAYER_SHUFFLE_MARKER_PATTERN",
        "THAT_MANY_PREFIX_PATTERN",
        "THIS_TURN_PATTERN",
        "PREVENT_DAMAGE_REFERENCE_MARKER_PATTERN",
        "PREVENT_DAMAGE_PLAYERS_TARGET_PATTERN",
        "YOU_TARGET_PATTERN",
        "IF_YOU_PREFIX_PATTERN\n                    .matches_words(&token_word_refs(&tokens[cursor - 2..cursor]))",
        "THAT_WOULD_BE_DEALT_PATTERN.matches_words(&core_words)",
        "LOSE_MANA_STEPS_PHASES_END_PATTERN.matches_words(&words)",
        "TRAILING_THAT_PLAYER_SHUFFLE_MARKER_PATTERN\n            .matches_words(&crate::runtime_backend::token_word_refs(tokens))",
        "THAT_MANY_PREFIX_PATTERN.matches_words(&token_word_refs(&raw_filter_tokens))",
        "THIS_TURN_PATTERN.matches_words(&words[this_turn_idx + 2..])",
        "PREVENT_DAMAGE_REFERENCE_MARKER_PATTERN\n        .matches_words(&source_words)",
        "PREVENT_DAMAGE_PLAYERS_TARGET_PATTERN.matches_words(&target_words)",
        "YOU_TARGET_PATTERN.matches_words(&target_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct grammar effect shape gates through raw word refs: found `{forbidden}`"
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
        "SOURCE_LEAVES_BATTLEFIELD_PATTERN",
        ".find_in_clause(LexedClause::new(&tokens))",
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
fn shared_util_production_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);

    assert!(
        content.contains("fn shared_util_shape_matches_words(")
            && content.contains("fn shared_util_shape_matches_word(")
            && content.contains("fn shared_util_token_matches_shape("),
        "{relative} should expose token-backed shared-util shape helpers"
    );
    assert!(
        !content.contains(".matches_word(")
            && !content.contains(".matches_token(")
            && !content.contains(".matches_words("),
        "{relative} should not route production shape gates through ClauseShape word/token adapters"
    );
}

#[test]
fn shared_util_for_each_count_value_uses_direct_word_shape_gates() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_for_each_count_value_words",
        "pub(crate) fn is_article",
    );
    let shape_helper = function_source(
        &content,
        "fn shared_util_shape_matches_words",
        "fn shared_util_shape_matches_word(",
    );

    for required in [
        "fn shared_util_shape_matches_words(",
        "shape.matches_word_slice(words)",
    ] {
        assert!(
            shape_helper.contains(required),
            "{relative} should expose a direct word-matching shared-util shape helper: missing `{required}`"
        );
    }

    for required in [
        "shared_util_shape_matches_words(words, FOR_EACH_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(&words[idx..], BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(&words[idx..], CREATURE_TYPES_AMONG_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(&words[idx..], COLORS_AMONG_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(count_words, SOURCE_REGENERATED_THIS_TURN_COUNT_PATTERN)",
        "shared_util_shape_matches_words(count_words, YOU_DREW_CARDS_THIS_TURN_PATTERN)",
        "shared_util_shape_matches_words(count_words, OPPONENT_DREW_CARDS_THIS_TURN_PATTERN)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route for-each count value gates through token-backed helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "FOR_EACH_PREFIX_PATTERN.matches_words(words)",
        "BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(&words[idx..])",
        "CREATURE_TYPES_AMONG_PREFIX_PATTERN.matches_words(&words[idx..])",
        "COLORS_AMONG_PREFIX_PATTERN.matches_words(&words[idx..])",
        "SOURCE_REGENERATED_THIS_TURN_COUNT_PATTERN.matches_words(count_words)",
        "YOU_DREW_CARDS_THIS_TURN_PATTERN.matches_words(count_words)",
        "OPPONENT_DREW_CARDS_THIS_TURN_PATTERN.matches_words(count_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route for-each count value gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn shared_util_value_expr_terms_use_token_backed_shape_gates() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_value_expr_term_words",
        "pub(crate) fn parse_value_expr_words",
    );

    for required in [
        "shared_util_shape_matches_words(words, *shape)",
        "shared_util_shape_matches_words(words, OTHER_RESULT_VALUE_PATTERN)",
        "shared_util_shape_matches_words(words, NUMBER_OF_REMOVED_THIS_WAY_PATTERN)",
        "shared_util_shape_matches_words(words, YOUR_SPEED_VALUE_PATTERN)",
        "shared_util_shape_matches_words(words, TARGET_PLAYER_SPEED_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(words, SOURCE_POWER_SHORT_PATTERN)",
        "shared_util_shape_matches_words(words, SOURCE_POWER_LONG_PATTERN)",
        "shared_util_shape_matches_words(words, SOURCE_TOUGHNESS_SHORT_PATTERN)",
        "shared_util_shape_matches_words(words, SOURCE_TOUGHNESS_LONG_PATTERN)",
        "shared_util_shape_matches_words(words, SOURCE_MANA_VALUE_SHORT_PATTERN)",
        "shared_util_shape_matches_words(words, SOURCE_MANA_VALUE_LONG_PATTERN)",
        "shared_util_shape_matches_words(words, NUMBER_OF_PATTERN)",
        "shared_util_shape_matches_words(reference, SOURCE_COUNTER_REFERENCE_PATTERN)",
        "shared_util_shape_matches_words(reference, TAGGED_COUNTER_REFERENCE_PATTERN)",
        "shared_util_shape_matches_words(filter_words, BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(filter_words, COLORS_AMONG_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(filter_words, DIFFERENT_POWERS_AMONG_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(filter_words, SPELL_CAST_THIS_TURN_COUNT_PATTERN)",
        "shared_util_shape_matches_words(filter_words, *suffix_pattern)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route value expression shape gates through token-backed helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "shape.matches_words(words)",
        "OTHER_RESULT_VALUE_PATTERN.matches_words(words)",
        "NUMBER_OF_REMOVED_THIS_WAY_PATTERN.matches_words(words)",
        "YOUR_SPEED_VALUE_PATTERN.matches_words(words)",
        "TARGET_PLAYER_SPEED_PREFIX_PATTERN.matches_words(words)",
        "SOURCE_POWER_SHORT_PATTERN.matches_words(words)",
        "SOURCE_POWER_LONG_PATTERN.matches_words(words)",
        "SOURCE_TOUGHNESS_SHORT_PATTERN.matches_words(words)",
        "SOURCE_TOUGHNESS_LONG_PATTERN.matches_words(words)",
        "SOURCE_MANA_VALUE_SHORT_PATTERN.matches_words(words)",
        "SOURCE_MANA_VALUE_LONG_PATTERN.matches_words(words)",
        "NUMBER_OF_PATTERN.matches_words(words)",
        "SOURCE_COUNTER_REFERENCE_PATTERN.matches_words(reference)",
        "TAGGED_COUNTER_REFERENCE_PATTERN.matches_words(reference)",
        "BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(filter_words)",
        "COLORS_AMONG_PREFIX_PATTERN.matches_words(filter_words)",
        "DIFFERENT_POWERS_AMONG_PREFIX_PATTERN.matches_words(filter_words)",
        "SPELL_CAST_THIS_TURN_COUNT_PATTERN.matches_words(filter_words)",
        "suffix_pattern.matches_words(filter_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route value expression shape gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn shared_util_parse_subject_player_phrases_use_token_backed_shape_gates() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_subject",
        "fn parse_token_type_from_word",
    );

    for required in [
        "shared_util_shape_matches_words(slice, MOST_CARDS_IN_HAND_SUBJECT_PATTERN)",
        "shared_util_shape_matches_words(slice, MOST_LIFE_SUBJECT_PATTERN)",
        "shared_util_shape_matches_words(slice, LOWEST_LIFE_SUBJECT_PATTERN)",
        "shared_util_shape_matches_words(&words, ANY_NUMBER_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, YOU_OR_YOUR_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, TARGET_OPPONENT_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, TARGET_PLAYER_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, PLAYER_OF_YOUR_CHOICE_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, OPPONENT_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, OTHER_PLAYER_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, DEFENDING_PLAYER_EDGE_PATTERN)",
        "shared_util_shape_matches_words(slice, DEFENDING_PLAYER_SUFFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, ATTACKING_PLAYER_EDGE_PATTERN)",
        "shared_util_shape_matches_words(slice, ATTACKING_PLAYER_SUFFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, THAT_PLAYER_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, VOTER_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, CHOSEN_PLAYER_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, THAT_PLAYERS_OR_THEIR_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, OWNERS_OF_THOSE_OBJECTS_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, ITS_CONTROLLER_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, ITS_OR_THEIR_OWNER_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, THIS_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, ITS_OR_THEIR_CONTROLLER_SUFFIX_PATTERN)",
        "shared_util_shape_matches_words(slice, ITS_OR_THEIR_OWNER_SUFFIX_PATTERN)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route parse_subject player phrase gates through token-backed helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "MOST_CARDS_IN_HAND_SUBJECT_PATTERN.matches_words(slice)",
        "MOST_LIFE_SUBJECT_PATTERN.matches_words(slice)",
        "LOWEST_LIFE_SUBJECT_PATTERN.matches_words(slice)",
        "ANY_NUMBER_PREFIX_PATTERN.matches_words(&words)",
        "YOU_OR_YOUR_PREFIX_PATTERN.matches_words(slice)",
        "TARGET_OPPONENT_PREFIX_PATTERN.matches_words(slice)",
        "TARGET_PLAYER_PREFIX_PATTERN.matches_words(slice)",
        "PLAYER_OF_YOUR_CHOICE_PREFIX_PATTERN.matches_words(slice)",
        "OPPONENT_PREFIX_PATTERN.matches_words(slice)",
        "OTHER_PLAYER_PREFIX_PATTERN.matches_words(slice)",
        "DEFENDING_PLAYER_EDGE_PATTERN.matches_words(slice)",
        "DEFENDING_PLAYER_SUFFIX_PATTERN.matches_words(slice)",
        "ATTACKING_PLAYER_EDGE_PATTERN.matches_words(slice)",
        "ATTACKING_PLAYER_SUFFIX_PATTERN.matches_words(slice)",
        "THAT_PLAYER_PREFIX_PATTERN.matches_words(slice)",
        "VOTER_PREFIX_PATTERN.matches_words(slice)",
        "CHOSEN_PLAYER_PREFIX_PATTERN.matches_words(slice)",
        "THAT_PLAYERS_OR_THEIR_PREFIX_PATTERN.matches_words(slice)",
        "OWNERS_OF_THOSE_OBJECTS_PREFIX_PATTERN.matches_words(slice)",
        "ITS_CONTROLLER_PREFIX_PATTERN.matches_words(slice)",
        "ITS_OR_THEIR_OWNER_PREFIX_PATTERN.matches_words(slice)",
        "THIS_PREFIX_PATTERN.matches_words(slice)",
        "ITS_OR_THEIR_CONTROLLER_SUFFIX_PATTERN.matches_words(slice)",
        "ITS_OR_THEIR_OWNER_SUFFIX_PATTERN.matches_words(slice)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route parse_subject player phrase gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn shared_util_parse_target_player_phrases_use_token_backed_shape_gates() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_target_phrase_inner",
        "fn split_target_phrase_by_or",
    );

    for required in [
        "shared_util_shape_matches_words(token_words.as_slice(), YOUR_OPPONENTS_TARGET_PATTERN)",
        "DEFENDING_PLAYER_CHOICE_TARGET_PATTERN",
        "shared_util_shape_matches_words(token_words.as_slice(), CHOSEN_AT_RANDOM_TAIL_PATTERN)",
        "shared_util_shape_matches_words(token_words.as_slice(), AT_RANDOM_TAIL_PATTERN)",
        "shared_util_shape_matches_words(&all_words, ANY_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&all_words, ANY_OTHER_TARGET_PATTERN)",
        "shared_util_shape_matches_words(all_words.as_slice(), UP_TO_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(&target_words, OTHER_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&all_words[1..], OF_THOSE_OR_THEM_TAIL_PATTERN)",
        "shared_util_shape_matches_words(&all_words, IT_OR_THEM_WITH_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(&all_words, TAGGED_OBJECT_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&all_words, REST_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, EQUIPPED_OBJECT_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, ENCHANTED_OBJECT_TARGET_PATTERN)",
        "CREATURE_TAPPED_FOR_THIS_SPELL_COST_PATTERN",
        "shared_util_shape_matches_words(&words_all, ANY_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&words_all, ANY_OTHER_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, TARGET_OR_TARGETS_WORD_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, TOP_CARD_TARGET_SHORTHAND_PATTERN)",
        "CARDS_TARGET_SHORTHAND_PATTERN",
        "shared_util_shape_matches_words(&remaining_words, PLAYER_ON_YOUR_TEAM_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, ANY_PLAYER_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, ENCHANTED_PLAYER_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, THAT_PLAYER_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, CHOSEN_PLAYER_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, THAT_OPPONENT_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, DEFENDING_PLAYER_EDGE_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, ITS_OR_THEIR_CONTROLLER_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, ITS_OR_THEIR_OWNER_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, YOU_OR_YOUR_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, ONE_OF_YOUR_OPPONENTS_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, OPPONENT_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, SPELL_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, TRIGGERING_SPELL_TARGET_PATTERN)",
        "TRIGGERING_SPELL_OR_ABILITY_TARGET_PATTERN",
        "shared_util_shape_matches_words(&remaining_words, SOURCE_PT_REFERENCE_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, SOURCE_PT_REFERENCE_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, IT_INSTEAD_THIS_WAY_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, TOKEN_CREATED_THIS_WAY_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, ITSELF_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, HIM_OR_HER_TARGET_PATTERN)",
        "shared_util_shape_matches_words(&remaining_words, THEM_TARGET_PATTERN)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route target player phrase gates through token-backed helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "YOUR_OPPONENTS_TARGET_PATTERN.matches_words(token_words.as_slice())",
        "DEFENDING_PLAYER_CHOICE_TARGET_PATTERN.matches_words(token_words.as_slice())",
        "CHOSEN_AT_RANDOM_TAIL_PATTERN.matches_words(token_words.as_slice())",
        "AT_RANDOM_TAIL_PATTERN.matches_words(token_words.as_slice())",
        "ANY_TARGET_PATTERN.matches_words(&all_words)",
        "ANY_OTHER_TARGET_PATTERN.matches_words(&all_words)",
        "UP_TO_PREFIX_PATTERN.matches_words(all_words.as_slice())",
        "OTHER_TARGET_PATTERN.matches_words(&target_words)",
        "OF_THOSE_OR_THEM_TAIL_PATTERN.matches_words(&all_words[1..])",
        "IT_OR_THEM_WITH_PREFIX_PATTERN.matches_words(&all_words)",
        "TAGGED_OBJECT_TARGET_PATTERN.matches_words(&all_words)",
        "REST_TARGET_PATTERN.matches_words(&all_words)",
        "EQUIPPED_OBJECT_TARGET_PATTERN.matches_words(&remaining_words)",
        "ENCHANTED_OBJECT_TARGET_PATTERN.matches_words(&remaining_words)",
        "CREATURE_TAPPED_FOR_THIS_SPELL_COST_PATTERN.matches_words(&remaining_words)",
        "ANY_TARGET_PATTERN.matches_words(&words_all)",
        "ANY_OTHER_TARGET_PATTERN.matches_words(&words_all)",
        "TARGET_OR_TARGETS_WORD_PATTERN.matches_words(&remaining_words)",
        "TOP_CARD_TARGET_SHORTHAND_PATTERN.matches_words(&remaining_words)",
        "CARDS_TARGET_SHORTHAND_PATTERN.matches_words(&remaining_words)",
        "PLAYER_ON_YOUR_TEAM_TARGET_PATTERN.matches_words(&remaining_words)",
        "ANY_PLAYER_TARGET_PATTERN.matches_words(&remaining_words)",
        "ENCHANTED_PLAYER_TARGET_PATTERN.matches_words(&remaining_words)",
        "THAT_PLAYER_TARGET_PATTERN.matches_words(&remaining_words)",
        "CHOSEN_PLAYER_TARGET_PATTERN.matches_words(&remaining_words)",
        "THAT_OPPONENT_TARGET_PATTERN.matches_words(&remaining_words)",
        "DEFENDING_PLAYER_EDGE_PATTERN.matches_words(&remaining_words)",
        "ITS_OR_THEIR_CONTROLLER_TARGET_PATTERN.matches_words(&remaining_words)",
        "ITS_OR_THEIR_OWNER_TARGET_PATTERN.matches_words(&remaining_words)",
        "YOU_OR_YOUR_PREFIX_PATTERN.matches_words(&remaining_words)",
        "ONE_OF_YOUR_OPPONENTS_TARGET_PATTERN.matches_words(&remaining_words)",
        "OPPONENT_TARGET_PATTERN.matches_words(&remaining_words)",
        "SPELL_TARGET_PATTERN.matches_words(&remaining_words)",
        "TRIGGERING_SPELL_TARGET_PATTERN.matches_words(&remaining_words)",
        "TRIGGERING_SPELL_OR_ABILITY_TARGET_PATTERN.matches_words(&remaining_words)",
        "SOURCE_PT_REFERENCE_PREFIX_PATTERN.matches_words(&remaining_words)",
        "SOURCE_PT_REFERENCE_TARGET_PATTERN.matches_words(&remaining_words)",
        "IT_INSTEAD_THIS_WAY_PREFIX_PATTERN.matches_words(&remaining_words)",
        "TOKEN_CREATED_THIS_WAY_TARGET_PATTERN.matches_words(&remaining_words)",
        "ITSELF_TARGET_PATTERN.matches_words(&remaining_words)",
        "HIM_OR_HER_TARGET_PATTERN.matches_words(&remaining_words)",
        "THEM_TARGET_PATTERN.matches_words(&remaining_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route target player phrase gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn shared_util_quantity_prefix_helpers_use_token_backed_shape_gates() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let comparison = function_source(
        &content,
        "pub(crate) fn parse_quantity_comparison_prefix",
        "pub(crate) fn comparison_to_at_least_threshold",
    );
    let choice_tokens = function_source(
        &content,
        "pub(crate) fn parse_choice_count_token_prefix_consumed",
        "pub(crate) fn parse_choice_or_range_count_token_prefix_consumed",
    );
    let choice_words = function_source(
        &content,
        "pub(crate) fn parse_choice_count_word_prefix",
        "pub(crate) fn strip_leading_articles",
    );

    for required in [
        "shared_util_shape_matches_words(\n        &crate::runtime_backend::token_word_refs(tokens),\n        AT_LEAST_PREFIX_PATTERN",
        "shared_util_shape_matches_words(\n        &crate::runtime_backend::token_word_refs(tokens),\n        AT_MOST_PREFIX_PATTERN",
    ] {
        assert!(
            comparison.contains(required),
            "{relative} should route quantity comparison prefix gates through token-backed helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "AT_LEAST_PREFIX_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens))",
        "AT_MOST_PREFIX_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens))",
    ] {
        assert!(
            !comparison.contains(forbidden),
            "{relative} should not route quantity comparison prefix gates through raw word refs: found `{forbidden}`"
        );
    }

    for required in [
        "shared_util_shape_matches_words(&words, ANY_NUMBER_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(&words, UP_TO_PREFIX_PATTERN)",
    ] {
        assert!(
            choice_tokens.contains(required),
            "{relative} should route token choice-count prefix gates through token-backed helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ANY_NUMBER_PREFIX_PATTERN.matches_words(&words)",
        "UP_TO_PREFIX_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !choice_tokens.contains(forbidden),
            "{relative} should not route token choice-count prefix gates through raw word refs: found `{forbidden}`"
        );
    }

    for required in [
        "shared_util_shape_matches_words(words, ANY_NUMBER_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(words, UP_TO_PREFIX_PATTERN)",
    ] {
        assert!(
            choice_words.contains(required),
            "{relative} should route word choice-count prefix gates through token-backed helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ANY_NUMBER_PREFIX_PATTERN.matches_words(words)",
        "UP_TO_PREFIX_PATTERN.matches_words(words)",
    ] {
        assert!(
            !choice_words.contains(forbidden),
            "{relative} should not route word choice-count prefix gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn shared_util_cast_restriction_helpers_use_token_backed_shape_gates() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let free_cast = function_source(
        &content,
        "pub(crate) fn parse_self_free_cast_alternative_cost_line",
        "pub(crate) fn parse_self_free_cast_alternative_cost_line_lexed",
    );
    let flash = function_source(
        &content,
        "pub(crate) fn parse_flash_with_additional_cost_line",
        "pub(crate) fn parse_reinforce_line_lexed",
    );
    let cast_only = function_source(
        &content,
        "pub(crate) fn parse_cast_this_spell_only_line",
        "pub(crate) fn parse_cast_this_spell_only_line_lexed",
    );

    for required in
        ["shared_util_shape_matches_words(&clause_words, SELF_FREE_CAST_ALTERNATIVE_COST_PATTERN)"]
    {
        assert!(
            free_cast.contains(required),
            "{relative} should route free-cast shape gates through token-backed helpers: missing `{required}`"
        );
    }
    for forbidden in ["SELF_FREE_CAST_ALTERNATIVE_COST_PATTERN.matches_words(&clause_words)"] {
        assert!(
            !free_cast.contains(forbidden),
            "{relative} should not route free-cast shape gates through raw word refs: found `{forbidden}`"
        );
    }

    for required in [
        "shared_util_shape_matches_words(&clause_words, FLASH_WITH_ADDITIONAL_COST_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(&suffix_words, FLASH_WITH_ADDITIONAL_COST_SUFFIX_PATTERN)",
    ] {
        assert!(
            flash.contains(required),
            "{relative} should route flash additional-cost shape gates through token-backed helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "FLASH_WITH_ADDITIONAL_COST_PREFIX_PATTERN.matches_words(&clause_words)",
        "FLASH_WITH_ADDITIONAL_COST_SUFFIX_PATTERN.matches_words(&suffix_words)",
    ] {
        assert!(
            !flash.contains(forbidden),
            "{relative} should not route flash additional-cost shape gates through raw word refs: found `{forbidden}`"
        );
    }

    for required in [
        "shared_util_shape_matches_words(line_words.as_slice(), CAST_THIS_SPELL_ONLY_PREFIX_PATTERN)",
        "shared_util_shape_matches_words(tail, CAST_ONLY_NO_PERMANENTS_NAMED_PREFIX_PATTERN)",
        "CAST_ONLY_DECLARE_ATTACKERS_TAIL_PATTERN",
        "CAST_ONLY_DECLARE_ATTACKERS_IF_ATTACKED_TAIL_PATTERN",
        "shared_util_shape_matches_words(tail, CAST_ONLY_DURING_COMBAT_TAIL_PATTERN)",
        "shared_util_shape_matches_words(tail, CAST_ONLY_COMBAT_BEFORE_BLOCKERS_TAIL_PATTERN)",
        "shared_util_shape_matches_words(tail, CAST_ONLY_COMBAT_AFTER_BLOCKERS_TAIL_PATTERN)",
        "CAST_ONLY_YOUR_COMBAT_BEFORE_BLOCKERS_TAIL_PATTERN",
        "shared_util_shape_matches_words(tail, CAST_ONLY_OPPONENT_COMBAT_TAIL_PATTERN)",
        "shared_util_shape_matches_words(tail, CAST_ONLY_BEFORE_ATTACKERS_TAIL_PATTERN)",
        "shared_util_shape_matches_words(tail, CAST_ONLY_BEFORE_COMBAT_DAMAGE_TAIL_PATTERN)",
        "shared_util_shape_matches_words(tail, CAST_ONLY_OPPONENTS_UPKEEP_TAIL_PATTERN)",
        "CAST_ONLY_OPPONENT_TURN_AFTER_UPKEEP_TAIL_PATTERN",
        "shared_util_shape_matches_words(tail, CAST_ONLY_YOUR_END_STEP_TAIL_PATTERN)",
        "shared_util_shape_matches_words(tail, CAST_ONLY_CAST_ANOTHER_SPELL_TAIL_PATTERN)",
        "CAST_ONLY_CAST_ANOTHER_GREEN_SPELL_TAIL_PATTERN",
        "shared_util_shape_matches_words(tail, CAST_ONLY_OPPONENT_CAST_CREATURE_TAIL_PATTERN)",
        "shared_util_shape_matches_words(tail, CAST_ONLY_CREATURE_ATTACKING_YOU_TAIL_PATTERN)",
        "shared_util_shape_matches_words(tail, CAST_ONLY_AFTER_COMBAT_TAIL_PATTERN)",
        "shared_util_shape_matches_words(tail, CAST_ONLY_CONTROL_SNOW_LAND_TAIL_PATTERN)",
        "CAST_ONLY_FEWER_CREATURES_THAN_EACH_OPPONENT_TAIL_PATTERN",
        "shared_util_shape_matches_words(tail, CAST_ONLY_IF_YOU_CONTROL_PREFIX_PATTERN)",
    ] {
        assert!(
            cast_only.contains(required),
            "{relative} should route cast-only restriction shape gates through token-backed helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "CAST_THIS_SPELL_ONLY_PREFIX_PATTERN.matches_words(line_words.as_slice())",
        "CAST_ONLY_NO_PERMANENTS_NAMED_PREFIX_PATTERN.matches_words(tail)",
        "CAST_ONLY_DECLARE_ATTACKERS_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_DECLARE_ATTACKERS_IF_ATTACKED_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_DURING_COMBAT_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_COMBAT_BEFORE_BLOCKERS_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_COMBAT_AFTER_BLOCKERS_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_YOUR_COMBAT_BEFORE_BLOCKERS_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_OPPONENT_COMBAT_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_BEFORE_ATTACKERS_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_BEFORE_COMBAT_DAMAGE_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_OPPONENTS_UPKEEP_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_OPPONENT_TURN_AFTER_UPKEEP_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_YOUR_END_STEP_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_CAST_ANOTHER_SPELL_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_CAST_ANOTHER_GREEN_SPELL_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_OPPONENT_CAST_CREATURE_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_CREATURE_ATTACKING_YOU_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_AFTER_COMBAT_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_CONTROL_SNOW_LAND_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_FEWER_CREATURES_THAN_EACH_OPPONENT_TAIL_PATTERN.matches_words(tail)",
        "CAST_ONLY_IF_YOU_CONTROL_PREFIX_PATTERN.matches_words(tail)",
    ] {
        assert!(
            !cast_only.contains(forbidden),
            "{relative} should not route cast-only restriction shape gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn shared_util_source_zone_phrase_helpers_use_token_backed_shape_gates() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let helpers = function_source(
        &content,
        "pub(crate) fn contains_source_from_your_graveyard_phrase",
        "pub(crate) fn is_basic_color_word",
    );

    for required in [
        "shared_util_shape_matches_words(window, SOURCE_FROM_YOUR_GRAVEYARD_PATTERN)",
        "shared_util_shape_matches_words(window, SOURCE_FROM_YOUR_HAND_PATTERN)",
        "shared_util_shape_matches_words(window, FROM_COMMAND_ZONE_PATTERN)",
        "shared_util_shape_matches_words(words, DISCARD_THIS_CARD_PATTERN)",
    ] {
        assert!(
            helpers.contains(required),
            "{relative} should route source-zone phrase helpers through token-backed shapes: missing `{required}`"
        );
    }

    for forbidden in [
        "SOURCE_FROM_YOUR_GRAVEYARD_PATTERN.matches_words(window)",
        "SOURCE_FROM_YOUR_HAND_PATTERN.matches_words(window)",
        "FROM_COMMAND_ZONE_PATTERN.matches_words(window)",
        "DISCARD_THIS_CARD_PATTERN.matches_words(words)",
    ] {
        assert!(
            !helpers.contains(forbidden),
            "{relative} should not route source-zone phrase helpers through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn shared_util_next_end_step_delay_flags_use_token_backed_shape_gates() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(crate) fn parse_next_end_step_token_delay_flags",
        "pub(crate) fn token_index_for_word_index",
    );

    for required in [
        "shared_util_shape_matches_words(tail_words, BEGINNING_OF_END_STEP_PATTERN)",
        "shared_util_shape_matches_words(tail_words, SACRIFICE_DELAY_REFERENCE_PATTERN)",
        "shared_util_shape_matches_words(tail_words, EXILE_DELAY_REFERENCE_PATTERN)",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should route next-end-step delay flags through token-backed shapes: missing `{required}`"
        );
    }

    for forbidden in [
        "BEGINNING_OF_END_STEP_PATTERN.matches_words(tail_words)",
        "SACRIFICE_DELAY_REFERENCE_PATTERN.matches_words(tail_words)",
        "EXILE_DELAY_REFERENCE_PATTERN.matches_words(tail_words)",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not route next-end-step delay flags through raw word refs: found `{forbidden}`"
        );
    }
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
fn compile_support_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/lowering/compile_support.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "const EQUIPPED_CREATURE_PHRASE: &[&str] = &[\"equipped\", \"creature\"]",
        "word_slice_contains_phrase(words, EQUIPPED_CREATURE_PHRASE)",
        "word_slice_eq(&words, &[\"shapeshifter\"])",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route compile-support shape gates through token word helpers: missing `{required}`"
        );
    }
    assert!(
        !content.contains("compile_support_shape_matches_words(")
            && !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains(".matches_words("),
        "{relative} should not route compile-support shape gates through ClauseShape/raw word refs"
    );
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

    let text_only_condition = function_source(
        &content,
        "fn parse_text_only_activation_restriction_condition_tokens",
        "let tokens = lex_line(restriction, 0).unwrap_or_default()",
    );
    let text_only_shape = function_source(
        &content,
        "fn text_only_activation_restriction_shape",
        "pub(crate) fn apply_pending_restrictions_to_ability",
    );
    for required in [
        "enum TextOnlyActivationRestriction",
        "fn text_only_activation_restriction_shape(\n    tokens: &[OwnedLexToken],\n) -> Option<TextOnlyActivationRestriction>",
        "const DID_NOT_ATTACK_THIS_TURN_PATTERN: LexPattern<'static>",
        "const SOURCE_ATTACKED_THIS_TURN_PATTERN: LexPattern<'static>",
        "LexCaptureKind::OneOfPhrase(DID_NOT_ATTACK_THIS_TURN_PHRASES)",
        "LexCaptureKind::OneOfPhrase(SOURCE_ATTACKED_THIS_TURN_SUBJECTS)",
        "let clause = LexedClause::new(tokens)",
        "capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "capture_clause_by_role(LexCaptureRole::Action, clause)",
        "match text_only_activation_restriction_shape(tokens)?",
        "TextOnlyActivationRestriction::SourceDidNotAttackThisTurn",
        "TextOnlyActivationRestriction::SourceAttackedThisTurn",
    ] {
        assert!(
            content.contains(required)
                || text_only_shape.contains(required)
                || text_only_condition.contains(required),
            "{relative} should parse text-only activation restrictions through token clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "let words = token_word_refs(tokens)",
        "DID_NOT_ATTACK_THIS_TURN_PATTERN.matches(clause)",
        "SOURCE_ATTACKED_THIS_TURN_PATTERN.matches(clause)",
        "DID_NOT_ATTACK_THIS_TURN_PATTERN.matches_words(&words)",
        "SOURCE_ATTACKED_THIS_TURN_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not parse text-only activation restrictions through raw word refs: found `{forbidden}`"
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
fn control_and_ownership_conditions_share_possession_capture_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let content = read_repo_file(&root, relative);
    let shared = function_source(
        &content,
        "fn match_possession_relation_shape",
        "fn parse_control_condition_shape",
    );
    let control = function_source(
        &content,
        "fn parse_control_condition_shape",
        "fn parse_control_condition_subject_clause",
    );
    let ownership = function_source(
        &content,
        "fn parse_ownership_condition_shape",
        "fn parse_ownership_condition_subject_clause",
    );

    for required in [
        "struct PossessionRelationCapture",
        "fn match_possession_relation_shape",
        "LexPattern::subject(\"subject\", LexCaptureKind::UntilAnyPhrase(action_phrases))",
        "LexPattern::action(\"action\", LexCaptureKind::OneOf(action_words))",
        "LexPattern::tail(\"amount_and_object\", LexCaptureKind::OneOrMoreWords)",
        "LexPattern::modifier(\"modifier\", LexCaptureKind::OneOf(&[\"with\"]))",
    ] {
        assert!(
            content.contains(required),
            "{relative} should expose a shared captured possession-relation shape: missing `{required}`"
        );
    }
    assert!(
        shared.contains("capture_clause_by_role(LexCaptureRole::Subject, clause)")
            && shared.contains("capture_by_role(LexCaptureRole::Tail)")
            && shared.contains("token_range_for_word_range"),
        "{relative} should turn possession-relation captures into subject/tail token ranges"
    );
    assert!(
        control.contains("match_possession_relation_shape(")
            && ownership.contains("match_possession_relation_shape("),
        "{relative} should parse control and ownership conditions through the same captured possession-relation shape"
    );
    for forbidden in [
        "let modifier_atoms = [",
        "let basic_atoms = [",
        "LexPattern::new(&basic_atoms).match_clause",
    ] {
        assert!(
            !control.contains(forbidden) && !ownership.contains(forbidden),
            "{relative} should not rebuild possession relation patterns inside control/ownership parsers: found `{forbidden}`"
        );
    }
}

#[test]
fn player_relation_conditions_use_captured_relation_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/conditions.rs";
    let content = read_repo_file(&root, relative);
    let life_relation = function_source(
        &content,
        "fn parse_life_relation_shape",
        "fn parse_no_opponent_more_life_than_shape",
    );
    let cards_relation = function_source(
        &content,
        "fn parse_cards_in_hand_relation_shape",
        "pub(crate) fn parse_player_turn_event_condition",
    );
    let life_parser = function_source(
        &content,
        "fn parse_player_life_relation_shape",
        "fn parse_life_relation_shape",
    );
    let cards_parser = function_source(
        &content,
        "fn parse_player_cards_in_hand_relation_shape",
        "fn parse_cards_in_hand_relation_shape",
    );

    for required in [
        "enum LifeRelationShape",
        "enum CardsInHandRelationShape",
        "fn parse_life_relation_shape(relation_clause: LexedClause<'_>) -> Option<LifeRelationShape>",
        "fn parse_cards_in_hand_relation_shape(\n    relation_clause: LexedClause<'_>,\n) -> Option<CardsInHandRelationShape>",
        "LexPattern::subject(\n            \"player\",",
        "LexCaptureKind::OneOfPhrase(&[&[\"you\", \"do\"], &[\"you\"]])",
        "LexPattern::subject(\"player\", LexCaptureKind::Rest)",
        "LexPattern::any_phrase(MORE_CARDS_IN_HAND_THAN_PREFIXES)",
        "matched.capture_clause_by_role(LexCaptureRole::Subject, relation_clause)",
    ] {
        assert!(
            content.contains(required)
                || life_relation.contains(required)
                || cards_relation.contains(required),
            "{relative} should classify player relation tails through captured shapes: missing `{required}`"
        );
    }

    for required in [
        "match parse_life_relation_shape(relation_clause)?",
        "LifeRelationShape::MoreThanYou",
        "LifeRelationShape::MoreThanEachOtherPlayer",
        "LifeRelationShape::MoreThanEachOpponent",
        "LifeRelationShape::MoreThanPlayer(player)",
        "match parse_cards_in_hand_relation_shape(relation_clause)?",
        "CardsInHandRelationShape::MoreThanYou",
        "CardsInHandRelationShape::MoreThanEachOtherPlayer",
    ] {
        assert!(
            life_parser.contains(required) || cards_parser.contains(required),
            "{relative} should consume captured player relation shapes in condition parsers: missing `{required}`"
        );
    }

    for forbidden in [
        "use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers",
        "ClauseShape",
        "clause_shape!",
        "MORE_LIFE_THAN_YOU_PATTERN: ClauseShape",
        "MORE_LIFE_THAN_EACH_OTHER_PLAYER_PATTERN: ClauseShape",
        "MORE_LIFE_THAN_EACH_OPPONENT_PATTERN: ClauseShape",
        "MORE_CARDS_IN_HAND_THAN_YOU_PATTERN: ClauseShape",
        "MORE_CARDS_IN_HAND_THAN_EACH_OTHER_PLAYER_PATTERN: ClauseShape",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep player relation tails as one-off ClauseShape probes: found `{forbidden}`"
        );
    }
}

#[test]
fn filter_player_relation_core_shapes_use_captured_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/player_relations.rs";
    let content = read_repo_file(&root, relative);
    let capture_helper = function_source(
        &content,
        "fn relation_captured_prefix",
        "impl SpellFilterComparisonAxis",
    );
    let axis_parser = function_source(
        &content,
        "pub(super) fn parse_spell_filter_comparison_axis_words",
        "pub(super) fn parse_player_relation_verb",
    );
    let verb_parser = function_source(
        &content,
        "pub(super) fn parse_player_relation_verb",
        "pub(super) fn parse_player_relation_subject",
    );
    let subject_parser = function_source(
        &content,
        "pub(super) fn parse_player_relation_subject",
        "pub(super) fn apply_player_relation",
    );
    let negated_parser = function_source(
        &content,
        "fn parse_negated_you_relation_shape",
        "impl SpellFilterComparisonAxis",
    );
    let negated_applier = function_source(
        &content,
        "pub(super) fn try_apply_negated_you_relation_clause",
        "pub(super) fn try_apply_chosen_player_graveyard_clause",
    );
    let compound_parser = function_source(
        &content,
        "fn parse_chosen_player_graveyard_shape",
        "impl SpellFilterComparisonAxis",
    );
    let compound_appliers = function_source(
        &content,
        "pub(super) fn try_apply_chosen_player_graveyard_clause",
        "pub(super) fn find_filter_prefix_consumed",
    );
    let event_parser = function_source(
        &content,
        "fn parse_relation_event_shape",
        "impl SpellFilterComparisonAxis",
    );
    let event_appliers = function_source(
        &content,
        "pub(super) fn parse_put_there_from_battlefield_this_turn_words",
        "pub(super) fn try_apply_put_there_from_battlefield_this_turn_clause",
    );

    for required in [
        "const RELATION_AXIS_PATTERN: LexPattern<'static>",
        "const RELATION_VERB_PATTERN: LexPattern<'static>",
        "const RELATION_SUBJECT_PATTERN: LexPattern<'static>",
        "const NEGATED_YOU_RELATION_PATTERN: LexPattern<'static>",
        "const CHOSEN_PLAYER_GRAVEYARD_PATTERN: LexPattern<'static>",
        "const JOINT_OWNER_CONTROLLER_PATTERN: LexPattern<'static>",
        "const OWNER_OR_CONTROLLER_PATTERN: LexPattern<'static>",
        "const PUT_THERE_FROM_BATTLEFIELD_THIS_TURN_PATTERN: LexPattern<'static>",
        "const PUT_THERE_FROM_ANYWHERE_THIS_TURN_PATTERN: LexPattern<'static>",
        "const GRAVEYARD_FROM_BATTLEFIELD_THIS_TURN_PATTERN: LexPattern<'static>",
        "const ENTERED_BATTLEFIELD_THIS_TURN_PATTERN: LexPattern<'static>",
        "const DRAWN_THIS_TURN_PATTERN: LexPattern<'static>",
        "const LEADING_TAGGED_REFERENCE_WORDS: &[&str]",
        "const IT_OR_THEM_WORDS: &[&str]",
        "LexPattern::action(\n    \"axis\",",
        "LexPattern::action(\n    \"verb\",",
        "LexPattern::action(\n        \"event\",",
        "LexPattern::subject(\n    \"player\",",
        "LexPattern::object(\n        \"zone\",",
        "LexPattern::optional(&[LexPattern::subject(",
        "LexCaptureKind::OneOfPhrase(&[&[\"power\"], &[\"toughness\"], &[\"mana\", \"value\"]])",
        "LexCaptureKind::OneOf(&[\"cast\", \"casts\", \"control\", \"controls\", \"own\", \"owns\"])",
        "LexCaptureKind::OneOfPhrase(&[\n            &[\"dont\", \"control\"],",
        "pattern.match_prefix_word_refs(words)",
        "matched.capture_by_role(role)",
        "words.get(capture.word_range.clone())",
        "parse_relation_axis_shape(words)",
        "parse_relation_verb_shape(words)",
        "parse_relation_subject_shape(words, pronoun_player_filter)",
        "parse_negated_you_relation_shape(words)",
        "parse_chosen_player_graveyard_shape(words)",
        "parse_joint_owner_controller_shape(&words[subject_consumed..])",
        "parse_owner_or_controller_shape(&words[subject_consumed..])",
        "parse_relation_event_shape(words, PUT_THERE_FROM_BATTLEFIELD_THIS_TURN_PATTERN)",
        "parse_relation_event_shape(words, PUT_THERE_FROM_ANYWHERE_THIS_TURN_PATTERN)",
        "parse_relation_event_shape(words, GRAVEYARD_FROM_BATTLEFIELD_THIS_TURN_PATTERN)",
        "parse_entered_battlefield_this_turn_shape(words)",
        "parse_relation_event_shape(words, DRAWN_THIS_TURN_PATTERN)",
        "let (verb, consumed) = parse_negated_you_relation_shape(words)?",
    ] {
        assert!(
            content.contains(required)
                || capture_helper.contains(required)
                || axis_parser.contains(required)
                || verb_parser.contains(required)
                || subject_parser.contains(required)
                || negated_parser.contains(required)
                || negated_applier.contains(required)
                || compound_parser.contains(required)
                || compound_appliers.contains(required)
                || event_parser.contains(required)
                || event_appliers.contains(required),
            "{relative} should parse player relation axes, verbs, subjects, negations, compound relations, and timing events through captured shapes: missing `{required}`"
        );
    }

    for forbidden in [
        "POWER_AXIS_PREFIX_PATTERN",
        "TOUGHNESS_AXIS_PREFIX_PATTERN",
        "MANA_VALUE_AXIS_PREFIX_PATTERN",
        "CAST_RELATION_VERB_PREFIX_PATTERN",
        "CONTROL_RELATION_VERB_PREFIX_PATTERN",
        "OWN_RELATION_VERB_PREFIX_PATTERN",
        "YOU_RELATION_SUBJECT_PREFIX_PATTERN",
        "OPPONENT_RELATION_SUBJECT_PREFIX_PATTERN",
        "THEY_RELATION_SUBJECT_PREFIX_PATTERN",
        "YOUR_TEAM_RELATION_SUBJECT_PREFIX_PATTERN",
        "YOUR_OPPONENTS_RELATION_SUBJECT_PREFIX_PATTERN",
        "THAT_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN",
        "TARGET_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN",
        "TARGET_OPPONENT_RELATION_SUBJECT_PREFIX_PATTERN",
        "DEFENDING_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN",
        "ATTACKING_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN",
        "TARGET_CONTROLLER_RELATION_SUBJECT_PREFIX_PATTERN",
        "DONT_CONTROL_PREFIX_PATTERN",
        "DONT_OWN_PREFIX_PATTERN",
        "DO_NOT_CONTROL_PREFIX_PATTERN",
        "DO_NOT_OWN_PREFIX_PATTERN",
        "YOU_DONT_CONTROL_PREFIX_PATTERN",
        "YOU_DONT_OWN_PREFIX_PATTERN",
        "YOU_DO_NOT_CONTROL_PREFIX_PATTERN",
        "YOU_DO_NOT_OWN_PREFIX_PATTERN",
        "CHOSEN_PLAYER_GRAVEYARD_PREFIX_PATTERN",
        "THE_CHOSEN_PLAYER_GRAVEYARD_PREFIX_PATTERN",
        "BOTH_OWN_AND_CONTROL_PREFIX_PATTERN",
        "OWN_OR_CONTROL_PREFIX_PATTERN",
        "PUT_THERE_FROM_BATTLEFIELD_THIS_TURN_PREFIX_PATTERN",
        "PUT_THERE_FROM_ANYWHERE_THIS_TURN_PREFIX_PATTERN",
        "GRAVEYARD_FROM_BATTLEFIELD_THIS_TURN_PREFIX_PATTERN",
        "ENTERED_YOUR_CONTROL_THIS_TURN_LONG_PREFIX_PATTERN",
        "ENTERED_YOUR_CONTROL_THIS_TURN_MID_PREFIX_PATTERN",
        "ENTERED_YOUR_CONTROL_THIS_TURN_SHORT_PREFIX_PATTERN",
        "ENTERED_OPPONENT_CONTROL_THIS_TURN_LONG_PREFIX_PATTERN",
        "ENTERED_OPPONENT_CONTROL_THIS_TURN_MID_PREFIX_PATTERN",
        "ENTERED_OPPONENT_CONTROL_THIS_TURN_SHORT_PREFIX_PATTERN",
        "ENTERED_THIS_TURN_LONG_PREFIX_PATTERN",
        "ENTERED_THIS_TURN_MID_PREFIX_PATTERN",
        "ENTERED_THIS_TURN_SHORT_PREFIX_PATTERN",
        "DRAWN_THIS_TURN_PREFIX_PATTERN",
        "OTHER_WORD_PATTERN",
        "LEADING_TAGGED_REFERENCE_WORD_PATTERN",
        "IT_OR_THEM_WORD_PATTERN",
        "ClauseShape",
        "clause_shape!",
        "shape_prefix_consumed",
        "player_relation_shape_matches_words",
        "synthetic_word_tokens(words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep player relation core parsing as one-off ClauseShape constant `{forbidden}`"
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
    assert!(
        line_parser.contains("let condition_shape_clause = LexedClause::new(&condition_tokens)")
            && line_parser
                .contains("ETB_FIRST_THREE_TURNS_PATTERN.matches(condition_shape_clause)"),
        "{relative} should match ETB unless condition shapes against captured condition tokens"
    );
    for required in [
        "let clause = LexedClause::new(tokens)",
        "ETB_ENTER_OR_ENTERS_MARKER_PATTERN.matches(clause)",
        "ETB_TAPPED_MARKER_PATTERN.matches(clause)",
        "ETB_UNLESS_MARKER_PATTERN.matches(clause)",
    ] {
        assert!(
            line_parser.contains(required),
            "{relative} should match conditional enters-tapped guards against the full token clause: missing `{required}`"
        );
    }
    for forbidden in [
        "find_token_index(\n            condition_tokens,\n            |token| ETB_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token)",
        "let filter_tokens = trim_edge_punctuation(&condition_tokens[control_idx + 1..])",
        "let condition_words = crate::runtime_backend::token_word_refs(&condition_tokens)",
        "ETB_FIRST_THREE_TURNS_PATTERN.matches_words(&condition_words)",
        "ETB_ENTER_OR_ENTERS_MARKER_PATTERN.matches_words(&clause_words)",
        "ETB_TAPPED_MARKER_PATTERN.matches_words(&clause_words)",
        "ETB_UNLESS_MARKER_PATTERN.matches_words(&clause_words)",
    ] {
        assert!(
            !line_parser.contains(forbidden) && !quantity_parser.contains(forbidden),
            "{relative} should not rescan control-condition tails by hand with `{forbidden}`"
        );
    }
}

#[test]
fn enters_with_counter_condition_tails_match_captured_clauses() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/etb_static_lines.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_enters_with_counter_plus_for_each_tail_tokens",
        "fn parse_enters_with_counter_object_filter_tokens",
    );

    for required in [
        "const ETB_FOR_EACH_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & [\"for\", \"each\"])",
        "ETB_FOR_EACH_PREFIX_PATTERN.matches(for_each_clause)",
        "ETB_COLORS_MANA_SPENT_CONDITION_TAIL_PATTERN.matches(spent_tail_clause)",
        "ETB_SPELLS_THIS_TURN_TAIL_PATTERN.matches(spell_tail_clause)",
        "parse_dynamic_cost_modifier_value(for_each_clause.tokens())",
        "parse_quantity_comparison_prefix(\n        amount_clause.tokens()",
    ] {
        assert!(
            content.contains(required) || parser.contains(required),
            "{relative} should match ETB counter condition tails as captured token clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "for_each_clause.word_refs().starts_with(FOR_EACH_PHRASE)",
        "ETB_COLORS_MANA_SPENT_CONDITION_TAIL_PATTERN\n            .matches_words(&spent_tail_clause.word_refs())",
        "ETB_SPELLS_THIS_TURN_TAIL_PATTERN.matches_words(&spell_tail_clause.word_refs())",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not match ETB counter condition tails through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn enters_with_added_abilities_tail_matches_ability_clause() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/etb_static_lines.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_enters_with_added_abilities_tail",
        "fn parse_enters_with_added_abilities_prefix",
    );

    assert!(
        parser.contains(
            "CAN_ATTACK_AS_THOUGH_NO_DEFENDER_PATTERN.matches(LexedClause::new(ability_tokens))"
        ),
        "{relative} should match added-ability ETB tails against the captured ability token clause"
    );
    for forbidden in [
        "let ability_words = crate::runtime_backend::lexer::token_word_refs(ability_tokens)",
        "CAN_ATTACK_AS_THOUGH_NO_DEFENDER_PATTERN.matches_words(&ability_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not flatten added-ability ETB tails into raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn etb_clause_shape_guards_match_captured_clauses() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/etb_static_lines.rs";
    let content = read_repo_file(&root, relative);
    let tapped_filter_parser = function_source(
        &content,
        "pub(crate) fn parse_enters_tapped_for_filter_line",
        "fn parse_enters_tapped_unless_control_quantity_static_ability",
    );
    let reveal_parser = function_source(
        &content,
        "pub(crate) fn parse_reveal_from_hand_or_enters_tapped_line",
        "fn parse_revealed_this_way_or_control_condition",
    );
    let additional_counter_parser = function_source(
        &content,
        "pub(crate) fn parse_enters_with_additional_counter_for_filter_line",
        "fn parse_as_enters_clause",
    );
    let as_enters_characteristics_parser = function_source(
        &content,
        "pub(crate) fn parse_as_enters_becomes_characteristics_for_filter_line",
        "pub(crate) fn parse_as_enters_or_turns_face_up_pt_choice_line",
    );
    let pt_choice_parser = function_source(
        &content,
        "pub(crate) fn parse_as_enters_or_turns_face_up_pt_choice_line",
        "fn parse_pt_choice_characteristic_options",
    );
    let source_reference_helper = function_source(
        &content,
        "fn is_etb_source_reference_clause",
        "fn starts_with_etb_source_reference",
    );
    let trigger_intro_helper = function_source(
        &content,
        "fn etb_starts_with_trigger_intro_after_label",
        "pub(crate) fn parse_enters_tapped_with_counters_line",
    );
    let added_abilities_tail = function_source(
        &content,
        "fn parse_enters_with_added_abilities_tail",
        "fn parse_enters_with_counter_line",
    );

    for required in [
        "ETB_COMMON_CREATURE_TYPE_VALUE_PATTERN.matches(LexedClause::new(tokens))",
        "ETB_TAPPED_MARKER_PATTERN.matches(entry_clause.tail_clause)",
        "ETB_UNTAPPED_MARKER_PATTERN.matches(entry_clause.tail_clause)",
        "ETB_OR_MARKER_PATTERN.matches(condition_clause)",
        "ETB_ENTERS_TAPPED_PHRASE_PATTERN.matches(entry_prefix)",
        "let before_word_len = LexedClause::new(before_enter).word_len()",
        "LexedClause::new(before_enter).between_word_range(keep_word_count, before_word_len)",
        "shape.matches(suffix_clause)",
        "fn etb_find_prefix_shape_start(\n    clause: LexedClause<'_>,",
        "clause\n            .after_words(idx)",
        "shape.matches(tail)",
        "ETB_ENTER_OR_ENTERS_MARKER_PATTERN.matches(LexedClause::new(tokens))",
        "ETB_TAPPED_MARKER_PATTERN.matches(LexedClause::new(tokens))",
        "ETB_UNLESS_MARKER_PATTERN.matches(LexedClause::new(tokens))",
        "ETB_COPY_MARKER_PATTERN.matches(LexedClause::new(tokens))",
        "etb_token_word_is(token, ETB_THIS_WORD)",
        "ETB_AS_THIS_LAND_ENTERS_PREFIX_PATTERN.matches(clause)",
        "ETB_REVEAL_FROM_HAND_MARKER_PATTERN.matches(clause)",
        "etb_find_prefix_shape_start(clause, &ETB_IF_YOU_DONT_PREFIX_PATTERN)",
        "ETB_LAND_REVEAL_TRAILING_TAPPED_PATTERN.matches(trailing)",
        "clause\n            .after_words(if_you_dont_idx + 3)",
        "ETB_AS_LONG_AS_THIS_IN_YOUR_GRAVEYARD_PATTERN.matches(clause)",
        "ETB_AS_LONG_AS_PREFIX_PATTERN.matches(clause)",
        "ETB_WITH_ADDITIONAL_COUNTERS_PATTERN.matches(clause)",
        "ETB_TRIGGER_INTRO_AFTER_LABEL_PATTERN.matches(LexedClause::new(&subject_tokens))",
        "ETB_IT_BECOMES_PREFIX_PATTERN.matches(as_enters.tail_clause)",
        "ETB_IN_ADDITION_TO_ITS_OTHER_TYPE_PATTERN.matches(as_enters.tail_clause)",
        "let Some(addition_idx) = etb_find_prefix_shape_start(",
        "as_enters.tail_clause,\n        &ETB_IN_ADDITION_TO_ITS_OTHER_PREFIX_PATTERN,",
        "let subject_clause = LexedClause::new(as_enters.subject_tokens)",
        "ETB_SELF_SUBJECT_PATTERN.matches(subject_clause)",
        "ETB_IT_BECOMES_YOUR_CHOICE_OF_PREFIX_PATTERN.matches(as_enters.tail_clause)",
        "ETB_FACE_UP_CHOICE_TAIL_PATTERN.matches(as_enters.tail_clause)",
        "let subject = subject_clause.text()",
        "SOURCE_PRONOUN_SUBJECT_PATTERN.matches(clause)",
        "ETB_TRIGGER_INTRO_AFTER_LABEL_PATTERN.matches(LexedClause::new(body_tokens))",
        "let tail_clause = LexedClause::new(&tail)",
        "ENTERS_WITH_ADDED_ABILITIES_AND_WITH_TAIL_PATTERN.matches(tail_clause)",
        "ENTERS_WITH_ADDED_ABILITIES_WITH_TAIL_PATTERN.matches(tail_clause)",
        "fn etb_word_is_any(word: &str, expected: &[&str]) -> bool",
        "fn etb_token_word_is_any(token: &OwnedLexToken, expected: &[&str]) -> bool",
    ] {
        assert!(
            content.contains(required)
                || tapped_filter_parser.contains(required)
                || reveal_parser.contains(required)
                || additional_counter_parser.contains(required)
                || as_enters_characteristics_parser.contains(required)
                || pt_choice_parser.contains(required)
                || source_reference_helper.contains(required)
                || trigger_intro_helper.contains(required)
                || added_abilities_tail.contains(required),
            "{relative} should match ETB clause-shape guards against captured clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "ETB_COMMON_CREATURE_TYPE_VALUE_PATTERN.matches_words(&LexedClause::new(tokens).word_refs())",
        "ETB_TAPPED_MARKER_PATTERN.matches_words(&entry_clause.tail_clause.word_refs())",
        "ETB_UNTAPPED_MARKER_PATTERN.matches_words(&entry_clause.tail_clause.word_refs())",
        "ETB_OR_MARKER_PATTERN.matches_words(&condition_clause.word_refs())",
        "ETB_ENTERS_TAPPED_PHRASE_PATTERN.matches_words(&entry_prefix.word_refs())",
        "let before_words = before_word_view.word_refs()",
        "ETB_PLAYED_BY_YOUR_OPPONENTS_SUFFIX_PATTERN.matches_words(&before_words)",
        "ETB_PLAYED_BY_AN_OPPONENT_SUFFIX_PATTERN.matches_words(&before_words)",
        "ETB_PLAYED_BY_OPPONENTS_SUFFIX_PATTERN.matches_words(&before_words)",
        "SOURCE_PRONOUN_SUBJECT_PATTERN.matches_words(&words)",
        "ETB_TRIGGER_INTRO_AFTER_LABEL_PATTERN\n        .matches_words(&crate::runtime_backend::lexer::token_word_refs(body_tokens))",
        "let words = crate::runtime_backend::lexer::token_word_refs(&tail)",
        "ENTERS_WITH_ADDED_ABILITIES_AND_WITH_TAIL_PATTERN.matches_words(&words)",
        "ENTERS_WITH_ADDED_ABILITIES_WITH_TAIL_PATTERN.matches_words(&words)",
        "fn etb_find_prefix_shape_start(words: &[&str], shape: &ClauseShape<'static>)",
        "shape.matches_words(&words[idx..])",
    ] {
        assert!(
            !content.contains(forbidden)
                && !tapped_filter_parser.contains(forbidden)
                && !source_reference_helper.contains(forbidden)
                && !trigger_intro_helper.contains(forbidden)
                && !added_abilities_tail.contains(forbidden),
            "{relative} should not unwrap captured clauses into raw word refs for ETB clause guards: found `{forbidden}`"
        );
    }
    for forbidden in [
        "ETB_ENTER_OR_ENTERS_MARKER_PATTERN.matches_words(&clause_words)",
        "ETB_TAPPED_MARKER_PATTERN.matches_words(&clause_words)",
        "ETB_UNLESS_MARKER_PATTERN.matches_words(&clause_words)",
        "ETB_COPY_MARKER_PATTERN.matches_words(&clause_words)",
        "ETB_THIS_WORD_PATTERN.matches_word_at(&clause_words, 0)",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route ETB word-level probes through ClauseShape adapters: found `{forbidden}`"
        );
    }
    for forbidden in [
        "ETB_AS_THIS_LAND_ENTERS_PREFIX_PATTERN.matches_words(&clause_words)",
        "ETB_REVEAL_FROM_HAND_MARKER_PATTERN.matches_words(&clause_words)",
        "etb_find_prefix_shape_start(&clause_words, &ETB_IF_YOU_DONT_PREFIX_PATTERN)",
        "ETB_LAND_REVEAL_TRAILING_TAPPED_PATTERN.matches_words(trailing)",
        "ETB_UNLESS_MARKER_PATTERN.matches_words(&clause_words)",
    ] {
        assert!(
            !reveal_parser.contains(forbidden),
            "{relative} should not match reveal-or-enters-tapped guards through raw word refs: found `{forbidden}`"
        );
    }
    for forbidden in [
        "ETB_AS_LONG_AS_THIS_IN_YOUR_GRAVEYARD_PATTERN.matches_words(&clause_words)",
        "ETB_AS_LONG_AS_PREFIX_PATTERN.matches_words(&clause_words)",
        "ETB_WITH_ADDITIONAL_COUNTERS_PATTERN.matches_words(&clause_words)",
        "let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens)",
        "subject_words.first().copied()",
    ] {
        assert!(
            !additional_counter_parser.contains(forbidden),
            "{relative} should not match additional-counter ETB guards through raw word refs: found `{forbidden}`"
        );
    }
    for forbidden in [
        "ETB_IT_BECOMES_PREFIX_PATTERN.matches_words(after_enter)",
        "ETB_IN_ADDITION_TO_ITS_OTHER_TYPE_PATTERN.matches_words(after_enter)",
        "etb_find_prefix_shape_start(after_enter, &ETB_IN_ADDITION_TO_ITS_OTHER_PREFIX_PATTERN)",
    ] {
        assert!(
            !as_enters_characteristics_parser.contains(forbidden),
            "{relative} should not match as-enters characteristic tails through raw word refs: found `{forbidden}`"
        );
    }
    for forbidden in [
        "let subject_words_vec = crate::runtime_backend::token_word_refs(as_enters.subject_tokens)",
        "ETB_SELF_SUBJECT_PATTERN.matches_words(subject_words)",
        "ETB_IT_BECOMES_YOUR_CHOICE_OF_PREFIX_PATTERN.matches_words(after_enter)",
        "ETB_FACE_UP_CHOICE_TAIL_PATTERN.matches_words(after_enter)",
        "let subject = subject_words.join(\" \")",
    ] {
        assert!(
            !pt_choice_parser.contains(forbidden),
            "{relative} should not match as-enters choice subjects through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn enters_with_counter_conditions_match_captured_condition_clause() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/etb_static_lines.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_enters_with_counter_condition_clause",
        "fn parse_enters_with_counter_object_filter_tokens",
    );

    for required in [
        "let condition_clause = LexedClause::new(&condition_tokens)",
        "condition_clause.is_empty()",
        "ETB_ATTACKED_THIS_TURN_CONDITION_PATTERN.matches(condition_clause)",
        "ETB_SOURCE_WAS_CAST_CONDITION_PATTERN.matches(condition_clause)",
        "ETB_THIS_SPELL_WAS_KICKED_CONDITION_PATTERN.matches(condition_clause)",
        "ETB_THIS_SPELL_ESCAPED_CONDITION_PATTERN.matches(condition_clause)",
        "ETB_CREATURE_DIED_THIS_TURN_CONDITION_PATTERN.matches(condition_clause)",
        "ETB_OPPONENT_LOST_LIFE_THIS_TURN_CONDITION_PATTERN.matches(condition_clause)",
        "ETB_PERMANENT_LEFT_UNDER_YOUR_CONTROL_CONDITION_PATTERN.matches(condition_clause)",
        "ETB_NOT_CAST_OR_NO_MANA_SPENT_CONDITION_PATTERN.matches(condition_clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should match enters-with-counter conditions against the captured condition clause: missing `{required}`"
        );
    }
    for forbidden in [
        "let condition_words = crate::runtime_backend::lexer::token_word_refs(&condition_tokens)",
        "ETB_ATTACKED_THIS_TURN_CONDITION_PATTERN.matches_words(&condition_words)",
        "ETB_SOURCE_WAS_CAST_CONDITION_PATTERN.matches_words(&condition_words)",
        "ETB_THIS_SPELL_WAS_KICKED_CONDITION_PATTERN.matches_words(&condition_words)",
        "ETB_THIS_SPELL_ESCAPED_CONDITION_PATTERN.matches_words(&condition_words)",
        "ETB_CREATURE_DIED_THIS_TURN_CONDITION_PATTERN.matches_words(&condition_words)",
        "ETB_OPPONENT_LOST_LIFE_THIS_TURN_CONDITION_PATTERN.matches_words(&condition_words)",
        "ETB_PERMANENT_LEFT_UNDER_YOUR_CONTROL_CONDITION_PATTERN.matches_words(&condition_words)",
        "ETB_NOT_CAST_OR_NO_MANA_SPENT_CONDITION_PATTERN.matches_words(&condition_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not match enters-with-counter conditions through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn where_x_reference_values_match_captured_reference_clause() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/etb_static_lines.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_where_x_is_fixed_plus_reference_value",
        "fn parse_where_x_noncombat_damage_to_opponents_clause",
    );

    for required in [
        "ETB_SACRIFICED_CREATURE_POWER_PREFIX_PATTERN.matches(captured.reference_clause)",
        "ETB_SACRIFICED_CREATURE_TOUGHNESS_PREFIX_PATTERN\n            .matches(captured.reference_clause)",
        "ETB_TAGGED_CREATURE_MANA_VALUE_PREFIX_PATTERN.matches(captured.reference_clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should match fixed-plus-reference value tails against the captured reference clause: missing `{required}`"
        );
    }
    for forbidden in [
        "let value_words = captured.reference_clause.word_refs()",
        "ETB_SACRIFICED_CREATURE_POWER_PREFIX_PATTERN.matches_words(&value_words)",
        "ETB_SACRIFICED_CREATURE_TOUGHNESS_PREFIX_PATTERN.matches_words(&value_words)",
        "ETB_TAGGED_CREATURE_MANA_VALUE_PREFIX_PATTERN.matches_words(&value_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not unwrap fixed-plus-reference value tails into raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn where_x_filter_value_gates_match_filter_clauses() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/etb_static_lines.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "ETB_AND_GRAVEYARD_MARKER_PATTERN.matches(LexedClause::new(filter_tokens))",
        "ETB_SACRIFICED_MARKER_PATTERN.matches(LexedClause::new(filter_tokens))",
        "ETB_BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches(LexedClause::new(filter_tokens))",
        "ETB_CREATURE_TYPES_AMONG_PREFIX_PATTERN.matches(LexedClause::new(filter_tokens))",
        "ETB_COLORS_AMONG_PREFIX_PATTERN.matches(LexedClause::new(filter_tokens))",
        "ETB_CARD_TYPES_AMONG_CARDS_PREFIX_PATTERN.matches(LexedClause::new(filter_tokens))",
        "ETB_CARD_TYPES_AMONG_PREFIX_PATTERN.matches(LexedClause::new(filter_tokens))",
        "ETB_GRAVEYARD_MARKER_PATTERN.matches(LexedClause::new(filter_tokens))",
        "ETB_YOUR_HAND_COUNT_VALUE_PATTERN.matches(LexedClause::new(&filter_tokens))",
    ] {
        assert!(
            content.contains(required),
            "{relative} should match where-X filter value gates against captured filter clauses: missing `{required}`"
        );
    }

    for forbidden in [
        "ETB_AND_GRAVEYARD_MARKER_PATTERN.matches_words(&filter_words)",
        "ETB_SACRIFICED_MARKER_PATTERN.matches_words(&filter_words)",
        "ETB_BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words)",
        "ETB_CREATURE_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words)",
        "ETB_COLORS_AMONG_PREFIX_PATTERN.matches_words(&filter_words)",
        "ETB_CARD_TYPES_AMONG_CARDS_PREFIX_PATTERN.matches_words(&filter_words)",
        "ETB_CARD_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words)",
        "ETB_GRAVEYARD_MARKER_PATTERN.matches_words(&filter_words)",
        "ETB_YOUR_HAND_COUNT_VALUE_PATTERN.matches_words(&filter_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not match where-X filter value gates through raw word refs: found `{forbidden}`"
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
        "fn control_predicate_quantity_tokens(\n    tokens: &[OwnedLexToken],\n    words: &TokenWordView<'_>",
        "words\n        .token_range_for_word_range(prefix_len, words.len())",
        "predicate_quantity_prefix_tokens(&tokens[range])",
        "control_predicate_quantity_tokens(tokens, &words_view, prefix_len)",
        "let mut filter_end = words.len()",
        "clause\n            .between_word_range(filter_end.saturating_sub(3), filter_end)",
        "clause_matches_any_phrase(tail, WITH_DIFFERENT_POWERS_TAIL_PHRASES)",
        "let filter_clause = LexedClause::new(filter_tokens)",
        "parse_outlaw_shorthand_filter(filter_clause)",
        "fn parse_outlaw_shorthand_filter(clause: LexedClause<'_>)",
        "let trimmed_tokens = strip_leading_article_tokens(clause.tokens())",
        "OUTLAW_SHORTHAND_FILTER_PHRASES",
    ] {
        assert!(
            content.contains(required) || parser.contains(required),
            "{relative} should parse player-control fallback quantities from original token ranges: missing `{required}`"
        );
    }
    for required in [
        "parse_player_controls_no_predicate(predicate_tokens)",
        "parse_you_control_or_graveyard_predicate(predicate_tokens)",
        "parse_you_control_conjoined_predicate(predicate_tokens)",
        "object_clause.tokens().is_empty()",
        "control_object.tokens().is_empty()",
        "left_object.tokens().is_empty() || right_object.tokens().is_empty()",
        "parse_player_controls_predicate(\n            predicate_tokens",
        "non_article_token_words_starts_with_any(predicate_tokens, YOU_CONTROL_PREFIXES)",
        "non_article_token_words_starts_with_any(predicate_tokens, THAT_PLAYER_CONTROLS_PREFIXES)",
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
        "fn predicate_quantity_prefix(words: &[&str])",
        "fn predicate_number_prefix(words: &[&str])",
        "fn predicate_at_least_quantity_prefix(words: &[&str])",
        "control_predicate_quantity(&words, prefix_len)",
        "let mut control_words = words[filter_start..].to_vec()",
        "WITH_DIFFERENT_POWERS_TAIL_PATTERN\n            .matches_words(&control_words[control_words.len().saturating_sub(3)..])",
        "WITH_DIFFERENT_POWERS_TAIL_PATTERN.matches(tail)",
        "control_words.truncate(control_words.len().saturating_sub(3))",
        "let filter_end = filter_start + control_words.len()",
        "parse_outlaw_shorthand_filter(&control_words)",
        "parse_outlaw_shorthand_filter(&filter_clause.word_refs())",
        "fn parse_outlaw_shorthand_filter(words: &[&str])",
        "strip_leading_article_word_refs(words)",
        "OUTLAW_SHORTHAND_FILTER_PATTERN.matches_words(trimmed)",
        "OUTLAW_SHORTHAND_FILTER_PATTERN.matches(LexedClause::new(trimmed_tokens))",
        "object_clause.word_refs().is_empty()",
        "control_object.word_refs().is_empty()",
        "left_object.word_refs().is_empty() || right_object.word_refs().is_empty()",
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
fn predicate_source_zone_and_ability_count_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let source_zone_parser = function_source(
        &content,
        "fn parse_source_zone_predicate",
        "fn parse_outlaw_shorthand_filter",
    );
    let ability_parser = function_source(
        &content,
        "fn parse_this_ability_resolution_count_predicate",
        "fn parse_color_only_object_filter_word_refs",
    );

    for required in [
        "fn parse_source_zone_predicate(tokens: &[OwnedLexToken])",
        "fn parse_this_ability_resolution_count_predicate(tokens: &[OwnedLexToken])",
        "parse_source_zone_predicate(predicate_tokens)",
        "parse_this_ability_resolution_count_predicate(predicate_tokens)",
        "ability_resolution_ordinal_count(clause)",
        "LexPattern::amount(\"count\", LexCaptureKind::WordCount(1))",
        "matched.capture_clause(\"count\", clause)",
        "ordinal_number_word(count_token.parser_text())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route source-zone and ability-count predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn source_zone_from_words(words: &[&str])",
        "fn parse_this_ability_resolution_count_predicate(filtered: &[&str])",
        "source_zone_from_words(&filtered)",
        "parse_this_ability_resolution_count_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route source-zone or ability-count predicates through filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in [
        "let words = clause.word_refs()",
        "let count = match words.as_slice()",
    ] {
        assert!(
            !ability_parser.contains(forbidden),
            "{relative} should not match ability-count predicates through raw word slices: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !source_zone_parser.contains(forbidden) && !ability_parser.contains(forbidden),
            "{relative} should not rebuild source-zone or ability-count predicate tokens from raw words: found `{forbidden}`"
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
        "left_clause.tokens().is_empty() || right_clause.tokens().is_empty()",
        "let Some(right_first) = right_clause.token(0) else",
        "let right_starts_with_have = token_word_is(right_first, HAVE_WORD)",
        "token_word_is(right_first, YOU_WORD)",
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
        "left_clause.word_refs().is_empty() || right_clause.word_refs().is_empty()",
        "let right_words = right_clause.word_refs()",
        "right_words.first().copied()",
        "HAVE_WORD_PATTERN.matches_word(right_first)",
        "YOU_WORD_PATTERN.matches_word(right_first)",
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
        "filter_clause.tokens().is_empty()",
        "CARD_OR_CARDS_WORD_PATTERN.matches_token(token)",
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
        "let filter_words = filter_clause.word_refs()",
        "if filter_words.is_empty()",
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
    for forbidden in ["CARD_OR_CARDS_WORD_PATTERN.matches_word(token.parser_text())"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should match this-way card nouns through token shapes: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_final_filtered_adapters_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let repeated_parser = function_source(
        &content,
        "fn parse_repeated_if_or_predicate",
        "fn predicate_reference_prefix_tokens",
    );

    for required in [
        "fn parse_stack_object_targets_only_source_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_repeated_if_or_predicate(\n    tokens: &[OwnedLexToken]",
        "left_clause.tokens().is_empty() || right_clause.tokens().is_empty()",
        "parse_repeated_if_or_predicate(predicate_tokens)",
        "parse_stack_object_targets_only_source_predicate(predicate_tokens)",
        "strip_leading_article_tokens(clause.trimmed().tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route the final predicate adapters through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "parse_stack_object_targets_only_source_predicate(&filtered)",
        "parse_repeated_if_or_predicate(&filtered)",
        "let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["left_clause.word_refs().is_empty() || right_clause.word_refs().is_empty()"] {
        assert!(
            !repeated_parser.contains(forbidden),
            "{relative} should not inspect repeated-if branches through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn object_filter_exact_reference_recognizers_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let recognizers = function_source(
        &content,
        "let mut all_words = non_article_word_refs(&all_words_with_articles);",
        "try_apply_distinct_powers_clause(&mut filter, &mut all_words);",
    );

    for required in [
        "non_article_token_words_eq(&base_tokens, ACTIVATED_ABILITY_WORDS)",
        "non_article_token_words_eq(&base_tokens, TRIGGERED_ABILITY_WORDS)",
        "non_article_token_words_eq_any(&base_tokens, ACTIVATED_OR_TRIGGERED_ABILITY_PHRASES)",
        "non_article_token_words_eq_any(&base_tokens, REST_REVEALED_OBJECT_PHRASES)",
    ] {
        assert!(
            recognizers.contains(required),
            "{relative} should run exact reference recognizers through non-article token helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ACTIVATED_ABILITY_PATTERN",
        "TRIGGERED_ABILITY_PATTERN",
        "ACTIVATED_OR_TRIGGERED_ABILITY_PATTERN",
        "REST_REVEALED_OBJECT_PATTERN",
        "ACTIVATED_ABILITY_PATTERN.matches_words(&all_words)",
        "TRIGGERED_ABILITY_PATTERN.matches_words(&all_words)",
        "ACTIVATED_OR_TRIGGERED_ABILITY_PATTERN.matches_words(&all_words)",
        "REST_REVEALED_OBJECT_PATTERN.matches_words(&all_words)",
    ] {
        assert!(
            !recognizers.contains(forbidden),
            "{relative} should not use mutable raw word vectors for exact reference recognizers: found `{forbidden}`"
        );
    }
}

#[test]
fn reference_tag_stage_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "fn non_article_parser_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str>",
        "fn non_article_token_words_contains_any_phrase",
        "fn non_article_token_words_starts_with_any",
        "fn word_is_any(word: &str, expected: &[&str]) -> bool",
        "fn find_phrase_start(words: &[&str], phrase: &[&str]) -> Option<usize>",
    ] {
        assert!(
            content.contains(required),
            "{relative} should expose token-word helpers for reference-tag gates: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape!",
        "reference_tag_shape_matches_words",
        ".matches_words(",
        ".matches_non_article_tokens(",
        "find_exact_window",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route reference-tag gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn reference_tag_stage_uses_parser_token_words_for_legacy_word_core() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "let raw_words_with_articles =",
        "try_apply_distinct_powers_clause(&mut filter, &mut all_words);",
    );

    assert!(
        parser.contains("let raw_words_with_articles = parser_token_word_refs(&base_tokens);"),
        "{relative} should derive the legacy word core from token parser words"
    );
    assert!(
        !parser.contains("GrammarFilterNormalizedWords::new(&base_tokens)"),
        "{relative} should not rebuild the object-filter word core through GrammarFilterNormalizedWords"
    );
}

#[test]
fn object_filter_tap_activated_ability_qualifier_uses_token_mirror() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let qualifier = function_source(
        &content,
        "let has_tap_activated_ability =",
        "let mut referenced_zones = Vec::new();",
    );

    assert!(
        qualifier.contains("has_tap_activated_ability_phrase(&all_words)"),
        "{relative} should use the phrase helper for tap-activated-ability qualifiers"
    );
    for forbidden in [
        "TAP_ACTIVATED_ABILITY_PATTERN",
        "TAP_ACTIVATED_ABILITY_PATTERN.matches_words(&all_words)",
    ] {
        assert!(
            !qualifier.contains(forbidden),
            "{relative} should not use ClauseShape adapters for tap-activated-ability qualifiers: found `{forbidden}`"
        );
    }
}

#[test]
fn object_filter_reference_phrase_probes_use_token_mirror() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "try_apply_drawn_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);",
        "if let Some(attacking_filter) =",
    );

    for required in [
        "strip_be_put_on_reference_prefix(&mut all_words, &segment_tokens)",
        "BLOCKED_BY_TAGGED_OBJECT_PHRASES",
        "ENTERED_THIS_TURN_UNSUPPORTED_PHRASE",
        "TAGGED_COUNTER_STATE_DISJUNCTION_PHRASES",
        "SUSPENDED_CARD_DISJUNCTION_PHRASES",
        "POWER_OR_TOUGHNESS_PHRASES",
        "TARGET_PLAYER_REFERENCE_PHRASES",
        "TARGET_OPPONENT_REFERENCE_PHRASES",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should use non-article token helper phrase probes: missing `{required}`"
        );
    }
    for required in [
        "const BE_VERB_WORDS: &[&str]",
        "const PUT_ON_PREFIX: &[&str]",
        "const PUT_ON_REFERENCE_WORDS: &[&str]",
        "word_slice_starts_with(&put_on_words, PUT_ON_PREFIX)",
        "word_slice_contains_any_word(&put_on_words, PUT_ON_REFERENCE_WORDS)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should parse put-on reference prefixes through token-derived word helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "BE_VERB_WORD_PATTERN",
        "PUT_ON_REFERENCE_PATTERN",
        "BLOCKED_BY_TAGGED_OBJECT_PATTERN",
        "ENTERED_THIS_TURN_UNSUPPORTED_PATTERN",
        "TAGGED_COUNTER_STATE_DISJUNCTION_PATTERN",
        "SUSPENDED_CARD_DISJUNCTION_PATTERN",
        "POWER_OR_TOUGHNESS_PATTERN",
        "TARGET_PLAYER_REFERENCE_PATTERN",
        "TARGET_OPPONENT_REFERENCE_PATTERN",
        "BLOCKED_BY_TAGGED_OBJECT_PATTERN.matches_words(&all_words)",
        "ENTERED_THIS_TURN_UNSUPPORTED_PATTERN.matches_words(&all_words)",
        "TAGGED_COUNTER_STATE_DISJUNCTION_PATTERN.matches_words(&all_words)",
        "SUSPENDED_CARD_DISJUNCTION_PATTERN.matches_words(&all_words)",
        "POWER_OR_TOUGHNESS_PATTERN.matches_words(&all_words)",
        "TARGET_PLAYER_REFERENCE_PATTERN.matches_words(&all_words)",
        "TARGET_OPPONENT_REFERENCE_PATTERN.matches_words(&all_words)",
        "BE_VERB_WORD_PATTERN.matches_word(all_words[0])",
        "PUT_ON_REFERENCE_PATTERN.matches_words(&all_words[1..4])",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not use mutable raw word vectors for reference phrase probes: found `{forbidden}`"
        );
    }
}

#[test]
fn object_filter_other_than_exclusions_use_word_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "// Object filters should not absorb trailing duration clauses",
        "// \"legendary or Rat card\"",
    );

    for required in [
        "word == UNTIL_WORD",
        "non_article_token_words_eq(&base_tokens[idx..idx + 2], OTHER_THAN_PREFIX)",
        "SELF_REFERENCE_WORDS.contains(&word)",
        "OBJECT_REFERENCE_NOUN_WORDS.contains(&word)",
        "non_article_token_words_starts_with_any(tail_tokens, EXCLUSION_RELATION_IGNORED_PREFIXES)",
        "AND_OR_WORDS.contains(&word)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route other-than exclusion probes through word helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "UNTIL_WORD_PATTERN",
        "OTHER_THAN_PREFIX_PATTERN",
        "SELF_REFERENCE_WORD_PATTERN",
        "OBJECT_REFERENCE_NOUN_WORD_PATTERN",
        "EXCLUSION_RELATION_IGNORED_PREFIX_PATTERN",
        "AND_OR_WORD_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep other-than exclusion ClauseShape adapter `{forbidden}`"
        );
    }
}

#[test]
fn object_filter_stat_axis_probes_use_word_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "for idx in 0..all_words.len() {\n        let (is_base_reference, pt_word_idx)",
        "let mut saw_permanent = false;",
    );

    for required in [
        "word_slice_starts_with(&all_words[idx..], BASE_POWER_TOUGHNESS_PREFIX)",
        "word_slice_starts_with(&all_words[idx..], POWER_TOUGHNESS_PREFIX)",
        "all_words[idx] == POWER_WORD",
        "all_words[idx] == TOUGHNESS_WORD",
        "word_slice_starts_with(&all_words[idx..], MANA_VALUE_PREFIX)",
        "word_slice_contains_phrase(&clause_words, POWER_GREATER_THAN_BASE_POWER_PHRASE)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route stat-axis object filter probes through word helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "BASE_POWER_TOUGHNESS_PATTERN",
        "POWER_TOUGHNESS_PATTERN",
        "BASE_WORD_PATTERN",
        "POWER_WORD_PATTERN",
        "TOUGHNESS_WORD_PATTERN",
        "AND_WORD_PATTERN",
        "MANA_VALUE_PATTERN",
        "POWER_GREATER_THAN_BASE_POWER_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep stat-axis ClauseShape adapter `{forbidden}`"
        );
    }
}

#[test]
fn object_filter_target_fragment_parser_uses_token_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "let parse_target_fragment = |fragment_tokens: &[OwnedLexToken]|",
        "if let Some(or_token_idx) = target_tokens",
    );
    let split = function_source(
        &content,
        "if let Some(or_token_idx) = target_tokens",
        "let mut all_words = non_article_word_refs(&all_words_with_articles);",
    );

    for required in [
        "TARGET_OR_TARGETS_WORDS.contains(&word)",
        "word == THAT_WORD",
        "word == ONLY_WORD",
        "word == SINGLE_WORD",
        "non_article_token_words_starts_with(&fragment_tokens, YOU_TARGET_PREFIX)",
        "non_article_token_words_starts_with_any(&fragment_tokens, OPPONENT_TARGET_PREFIXES)",
        "non_article_token_words_starts_with_any(&fragment_tokens, PLAYER_TARGET_PREFIXES)",
        "word == OR_WORD",
    ] {
        assert!(
            parser.contains(required) || split.contains(required),
            "{relative} should parse target fragments through token words and non-article helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "A_SINGLE_PREFIX_PATTERN",
        "TARGET_OR_TARGETS_WORD_PATTERN",
        "THAT_WORD_PATTERN",
        "ONLY_WORD_PATTERN",
        "SINGLE_WORD_PATTERN",
        "YOU_TARGET_PREFIX_PATTERN",
        "OPPONENT_TARGET_PREFIX_PATTERN",
        "PLAYER_TARGET_PREFIX_PATTERN",
        "OR_WORD_PATTERN",
        "let fragment_words_view = GrammarFilterNormalizedWords::new(&fragment_tokens)",
        "let target_words_view = GrammarFilterNormalizedWords::new(&fragment_tokens)",
        "let target_words_view = GrammarFilterNormalizedWords::new(target_tokens)",
        "lower_words_find_index(&target_words",
    ] {
        assert!(
            !parser.contains(forbidden) && !split.contains(forbidden),
            "{relative} should not rebuild target fragment word lists for token-range parsing: found `{forbidden}`"
        );
    }
}

#[test]
fn object_filter_attacked_this_turn_probe_uses_token_mirror() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "if non_article_token_words_contains_phrase(&segment_tokens, ATTACKED_THIS_TURN_PHRASE)",
        "for (idx, word) in all_words.iter().enumerate()",
    );

    assert!(
        parser.contains(
            "non_article_token_words_contains_phrase(&segment_tokens, ATTACKED_THIS_TURN_PHRASE)"
        ),
        "{relative} should use the non-article token helper for attacked-this-turn probes"
    );
    assert!(
        !content.contains("ATTACKED_THIS_TURN_PATTERN"),
        "{relative} should not keep the attacked-this-turn ClauseShape adapter"
    );
}

#[test]
fn object_filter_type_list_conjunctions_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "let segments = split_lexed_slices_on_or(&segment_tokens);",
        "let has_constraints = !filter.card_types.is_empty()",
    );

    for required in [
        "fn non_article_parser_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str>",
        "fn non_article_token_words_contains_any_word(tokens: &[OwnedLexToken], words: &[&str]) -> bool",
        "non_article_parser_word_refs(segment)",
        "non_article_token_words_contains_any_word(&segment_tokens, TYPE_LIST_CONJUNCTION_WORDS)",
        "non_article_token_words_contains_any_word(&segment_tokens, &[\"and\"])",
        "non_article_token_words_contains_any_word(&segment_tokens, &[\"or\"])",
        "non_article_token_words_contains_any_word(&segment_tokens, &[\"and/or\"])",
    ] {
        assert!(
            content.contains(required) || parser.contains(required),
            "{relative} should derive type-list conjunctions from non-article token helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "AND_OR_MARKER_PATTERN",
        "AND_MARKER_PATTERN",
        "OR_MARKER_PATTERN",
        "AND_OR_WORD_MARKER_PATTERN",
        "TYPE_LIST_CONJUNCTION_PATTERN",
        "let segment_words_view = GrammarFilterNormalizedWords::new(segment)",
        "AND_OR_MARKER_PATTERN.matches_words(&all_words)",
        "AND_MARKER_PATTERN.matches_words(&all_words)",
        "OR_MARKER_PATTERN.matches_words(&all_words)",
        "AND_OR_WORD_MARKER_PATTERN.matches_words(&all_words)",
        "TYPE_LIST_CONJUNCTION_PATTERN.matches_words(&all_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not use mutable raw word vectors for type-list conjunctions: found `{forbidden}`"
        );
    }
}

#[test]
fn object_filter_strict_compound_and_basic_land_helpers_use_token_words() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let strict_parser = function_source(
        &content,
        "// Strict mode: detect structural patterns",
        "Ok(filter)",
    );
    let basic_land_helper = function_source(
        &content,
        "fn strip_other_than_basic_land_cards_tokens",
        "fn apply_basic_land_exception",
    );

    for required in [
        "let input_words = non_article_parser_word_refs(tokens)",
        "word_slice_starts_with(&all_words[idx..], OTHER_THAN_BASIC_LAND_PREFIX)",
        "non_article_token_words_starts_with(\n            &segment_tokens[idx..],\n            OTHER_THAN_BASIC_LAND_PREFIX,\n        )",
        "CARD_OR_CARDS_WORDS.contains",
    ] {
        assert!(
            strict_parser.contains(required) || basic_land_helper.contains(required),
            "{relative} should use direct word helpers in strict/basic-land helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "OTHER_THAN_BASIC_LAND_PREFIX_PATTERN",
        "CARD_OR_CARDS_WORD_PATTERN",
        "let input_words_view = GrammarFilterNormalizedWords::new(&tokens)",
        "let token_words = GrammarFilterNormalizedWords::new(&segment_tokens[idx..]).to_word_refs()",
        "OTHER_THAN_BASIC_LAND_PREFIX_PATTERN.matches_words(&token_words)",
    ] {
        assert!(
            !strict_parser.contains(forbidden) && !basic_land_helper.contains(forbidden),
            "{relative} should not rebuild strict/basic-land helper word lists: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_module_shape_helpers_use_direct_word_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "fn keyword_static_shape_matches_words",
        "fn keyword_static_shape_matches_word",
        "fn keyword_static_token_matches_shape",
        "fn keyword_static_shape_matches_word_at",
        "fn keyword_static_shape_matches_last_word",
        "shape.matches_word_slice(words)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should expose direct word-matching keyword-static shape helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "let tokens = super::lexer::synthetic_word_tokens(words)",
        "shape.matches(LexedClause::new(&tokens))",
        ".matches_words(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not call ClauseShape word/token adapter methods directly: found `{forbidden}`"
        );
    }
}

#[test]
fn cost_modifier_prefix_conditions_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_cost_modifier_prefix_condition",
        "fn parse_optional_life_additional_cost_reduction_line",
    );

    for required in [
        "DURING_TURNS_OTHER_THAN_YOURS_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "DURING_YOUR_TURN_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "AS_LONG_AS_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse cost-modifier prefix conditions from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let all_words = crate::runtime_backend::token_word_refs(tokens)",
        "DURING_TURNS_OTHER_THAN_YOURS_PREFIX_PATTERN.matches_words(&all_words)",
        "DURING_YOUR_TURN_PREFIX_PATTERN.matches_words(&all_words)",
        "AS_LONG_AS_PREFIX_PATTERN.matches_words(&all_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route cost-modifier prefix conditions through all_words: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_quantity_and_counter_filters_use_source_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let life_condition = function_source(
        &content,
        "fn parse_life_total_or_less_spell_cost_condition",
        "fn mentioned_instant_sorcery_card_types",
    );
    let double_counters = function_source(
        &content,
        "pub(crate) fn parse_double_counters_replacement_line",
        "pub(crate) fn parse_double_token_creation_replacement_line",
    );
    let double_tokens = function_source(
        &content,
        "pub(crate) fn parse_double_token_creation_replacement_line",
        "pub(crate) fn parse_prevent_all_combat_damage_to_source_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "YOU_HAVE_PREFIX_PATTERN.matches(clause)",
        "let quantity_clause = clause.between_word_range(2, words.len().saturating_sub(1))?",
        "YOUR_LIFE_TOTAL_IS_PREFIX_PATTERN.matches(clause)",
        "let quantity_clause = clause.after_words(4)?",
        "GENERIC_DOUBLE_COUNTERS_UNDER_YOUR_CONTROL_PATTERN.matches_non_article_tokens(tokens)",
        "PLUS_ONE_COUNTERS_WOULD_BE_PUT_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "TWICE_THAT_MANY_PLUS_ONE_COUNTERS_TAIL_PATTERN.matches_non_article_tokens(tokens)",
        "find_index(tokens, |token| TWICE_WORD_PATTERN.matches_token(token))",
        "parse_object_filter_lexed(&tokens[prefix_len..twice_idx], false)",
        "DOUBLE_TOKEN_CREATION_UNDER_YOUR_CONTROL_PATTERN.matches_non_article_tokens(tokens)",
        "YOU_CREATE_ONE_OR_MORE_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "TOKEN_OR_TOKENS_WORD_PATTERN.matches_token(token)",
        "let descriptor_tokens = &tokens[add_one_prefix_len..token_idx]",
        "TREASURE_WORD_PATTERN.matches(LexedClause::new(descriptor_tokens))",
        "ADDITIONAL_TOKEN_REPLACEMENT_PREFIX_PATTERN.matches_non_article_tokens(after_token)",
    ] {
        assert!(
            life_condition.contains(required)
                || double_counters.contains(required)
                || double_tokens.contains(required),
            "{relative} should preserve source token ranges for keyword static quantity/filter parsing: missing `{required}`"
        );
    }
    for forbidden in [
        "YOU_HAVE_PREFIX_PATTERN.matches_words(words)",
        "YOUR_LIFE_TOTAL_IS_PREFIX_PATTERN.matches_words(words)",
        "let quantity_tokens = tokens.get(2..tokens.len().saturating_sub(1))?",
        "let quantity_tokens = tokens.get(4..).unwrap_or_default()",
        "crate::runtime_backend::lexer::synthetic_word_tokens(\n                &tail_words",
        "crate::runtime_backend::lexer::synthetic_word_tokens(&line_words[prefix_len..twice_idx])",
        "let line_words = crate::runtime_backend::token_word_refs(tokens)",
        "PLUS_ONE_COUNTERS_WOULD_BE_PUT_PREFIX_PATTERN.matches_words(&line_words)",
        "TWICE_THAT_MANY_PLUS_ONE_COUNTERS_TAIL_PATTERN.matches_words(&line_words)",
        "DOUBLE_TOKEN_CREATION_UNDER_YOUR_CONTROL_PATTERN.matches_words(&line_words)",
        "YOU_CREATE_ONE_OR_MORE_PREFIX_PATTERN.matches_words(&line_words)",
        "TREASURE_WORD_PATTERN.matches_words(&descriptor_words)",
    ] {
        assert!(
            !life_condition.contains(forbidden)
                && !double_counters.contains(forbidden)
                && !double_tokens.contains(forbidden),
            "{relative} should not rebuild keyword static parser tokens from raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_named_subject_helpers_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let activated_helper = function_source(
        &content,
        "fn activated_ability_subject_special_filter",
        "pub(crate) fn parse_pregame_begin_on_battlefield_line",
    );
    let hand_name_helper = function_source(
        &content,
        "fn only_creature_cards_in_hand_named",
        "fn dynamic_cards_drawn_this_turn_player_tokens",
    );
    let cost_condition = function_source(
        &content,
        "pub(crate) fn parse_this_spell_cost_condition",
        "fn parse_conjoined_this_spell_cost_condition",
    );

    for required in [
        "fn activated_ability_subject_special_filter(tokens: &[OwnedLexToken])",
        "SOURCES_WITH_CHOSEN_NAME_PATTERN.matches_non_article_tokens(tokens)",
        "activated_ability_subject_special_filter(subject_tokens)",
        "fn only_creature_cards_in_hand_named(clause: LexedClause<'_>)",
        "YOU_HAVE_NO_OTHER_CREATURE_CARDS_PREFIX_PATTERN.matches(clause)",
        "OR_IF_PHRASE_PATTERN.matches(clause)",
        "ONLY_OTHER_CREATURE_CARDS_NAMED_PREFIX_PATTERN.matches(clause)",
        "only_creature_cards_in_hand_named(clause)",
    ] {
        assert!(
            activated_helper.contains(required)
                || hand_name_helper.contains(required)
                || cost_condition.contains(required),
            "{relative} should route named subject helpers through token clause shapes: missing `{required}`"
        );
    }

    for forbidden in [
        "fn activated_ability_subject_special_filter(words: &[&str])",
        "SOURCES_WITH_CHOSEN_NAME_PATTERN.matches_words(words)",
        "activated_ability_subject_special_filter(&subject_words)",
        "fn only_creature_cards_in_hand_named(words: &[&str])",
        "YOU_HAVE_NO_OTHER_CREATURE_CARDS_PREFIX_PATTERN\n        .matches_words(words)",
        "OR_IF_PHRASE_PATTERN.matches_words(words)",
        "ONLY_OTHER_CREATURE_CARDS_NAMED_PREFIX_PATTERN.matches_words(words)",
        "only_creature_cards_in_hand_named(&w)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route named subject helpers through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_untap_and_combat_maximum_parsers_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let untap_parser = function_source(
        &content,
        "pub(crate) fn parse_filter_dont_untap_during_controllers_untap_steps_line",
        "fn parse_graveyard_metric_threshold_condition",
    );
    let combat_parser = function_source(
        &content,
        "pub(crate) fn parse_no_more_than_creatures_can_attack_or_block_each_combat_line",
        "pub(crate) fn parse_characteristic_defining_pt_line",
    );

    for required in [
        "find_index(tokens, |token| {\n        DONT_OR_DOESNT_WORD_PATTERN.matches_token(token)",
        "UNTAP_WORD_PATTERN.matches_token(token)",
        "CONTROLLERS_UNTAP_STEP_TAIL_PATTERN.matches_non_article_tokens(tail)",
        "let subject_text = render_token_slice(&subject_tokens)",
        "let tail = &tokens[used..]",
        "CREATURES_CAN_ATTACK_YOU_EACH_COMBAT_TAIL_PATTERN\n        .matches_non_article_tokens(tail)",
        "CREATURES_CAN_ATTACK_EACH_COMBAT_TAIL_PATTERN.matches_non_article_tokens(tail)",
        "CREATURES_CAN_BLOCK_EACH_COMBAT_TAIL_PATTERN.matches_non_article_tokens(tail)",
    ] {
        assert!(
            untap_parser.contains(required) || combat_parser.contains(required),
            "{relative} should parse untap/combat maximum lines from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let line_words = crate::runtime_backend::token_word_refs(tokens)",
        "let subject_text = crate::runtime_backend::token_word_refs(&subject_tokens).join(\" \")",
        "CONTROLLERS_UNTAP_STEP_TAIL_PATTERN.matches_words(tail)",
        "token_index_for_word_index(tokens, dont_word_idx)",
        "let tail = crate::runtime_backend::token_word_refs(&tokens[used..])",
        "CREATURES_CAN_ATTACK_YOU_EACH_COMBAT_TAIL_PATTERN.matches_words(&tail)",
        "CREATURES_CAN_ATTACK_EACH_COMBAT_TAIL_PATTERN.matches_words(&tail)",
        "CREATURES_CAN_BLOCK_EACH_COMBAT_TAIL_PATTERN.matches_words(&tail)",
    ] {
        assert!(
            !untap_parser.contains(forbidden) && !combat_parser.contains(forbidden),
            "{relative} should not route untap/combat maximum parsers through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_graveyard_metric_threshold_uses_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_graveyard_metric_threshold_condition",
        "pub(crate) fn parse_conditional_source_spell_keyword_line",
    );

    for required in [
        "THERE_IS_OR_ARE_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "CARD_TYPES_IN_YOUR_GRAVEYARD_METRIC_PATTERN.matches_non_article_tokens(rest)",
        "MANA_VALUES_IN_YOUR_GRAVEYARD_METRIC_PATTERN.matches_non_article_tokens(rest)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse graveyard metric thresholds from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let words_all = crate::runtime_backend::token_word_refs(tokens)",
        "THERE_IS_OR_ARE_PREFIX_PATTERN.matches_words(&words_all)",
        "let rest_words = crate::runtime_backend::token_word_refs(rest)",
        "CARD_TYPES_IN_YOUR_GRAVEYARD_METRIC_PATTERN.matches_words(&rest_words)",
        "MANA_VALUES_IN_YOUR_GRAVEYARD_METRIC_PATTERN.matches_words(&rest_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route graveyard metric thresholds through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_conditional_spell_keyword_uses_captured_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let constants = function_source(
        &content,
        "const CONDITIONAL_SPELL_KEYWORD_WORDS",
        "const YOU_MAY_HAVE_PREFIX_PATTERN",
    );
    let parser = function_source(
        &content,
        "pub(crate) fn parse_conditional_source_spell_keyword_line",
        "pub(crate) fn parse_enters_tapped_with_choose_color_line",
    );

    for required in [
        "const CONDITIONAL_SOURCE_SPELL_KEYWORD_PATTERN: LexPattern<'static>",
        "LexPattern::action(\"keyword\", LexCaptureKind::OneOf(CONDITIONAL_SPELL_KEYWORD_WORDS))",
        "LexPattern::condition(\"condition\", LexCaptureKind::OneOrMoreWords)",
        "CONDITIONAL_SOURCE_SPELL_KEYWORD_PATTERN.match_clause(clause)",
        "capture_clause_by_role(LexCaptureRole::Action, clause)",
        "capture_clause_by_role(LexCaptureRole::Condition, clause)",
        "trim_commas(condition_clause.tokens())",
    ] {
        assert!(
            constants.contains(required) || parser.contains(required),
            "{relative} should parse conditional spell keywords from a captured clause shape: missing `{required}`"
        );
    }
    for forbidden in [
        "let clause_words = crate::runtime_backend::token_word_refs(tokens)",
        "keyword_find_prefix_shape_start(&clause_words, &THIS_SPELL_HAS_PREFIX_PATTERN)",
        "CONDITIONAL_SPELL_KEYWORD_WORD_PATTERN.matches_word(keyword_word)",
        "AS_LONG_AS_PREFIX_PATTERN.matches_words(&clause_words[this_idx + 4..])",
        "token_index_for_word_index(tokens, this_idx + 7)",
        "clause_words.join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route conditional spell keyword parsing through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_enters_tapped_choose_color_uses_token_word_view() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_enters_tapped_with_choose_color_line",
        "pub(crate) fn parse_damage_not_removed_cleanup_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "ENTERS_TAPPED_LINE_PATTERN.matches(clause)",
        "let words = clause.words()",
        "TAPPED_WORD_PATTERN.matches_word(word)",
        "words.token_index_for_word_index(tapped_word_idx)",
        "render_token_slice(tokens)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse enters-tapped choose-color lines from token word views: missing `{required}`"
        );
    }
    for forbidden in [
        "let clause_words = crate::runtime_backend::token_word_refs(tokens)",
        "ENTERS_TAPPED_LINE_PATTERN.matches_words(&clause_words)",
        "find_index(&clause_words, |word| TAPPED_WORD_PATTERN.matches_word(word))",
        "token_index_for_word_index(tokens, tapped_word_idx)",
        "clause_words.join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route enters-tapped choose-color parsing through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_source_is_chosen_color_uses_token_word_view() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_source_is_chosen_color_line",
        "pub(crate) fn parse_choose_creature_type_as_enters_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let words = clause.words()",
        "IS_WORD_PATTERN.matches_word(word)",
        "let word_refs = words.word_refs()",
        "let Some(subject_clause) = clause.between_word_range(0, is_idx) else",
        "SOURCE_IT_PATTERN.matches(subject_clause)",
        "words.token_index_for_word_or_end(is_idx + 1)",
        "CHOSEN_COLOR_TAIL_PATTERN.matches(chosen_color_tail)",
        "THE_CHOSEN_COLOR_TAIL_PATTERN.matches(chosen_color_tail)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse chosen-color source lines from token word views: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "find_index(&words, |word| IS_WORD_PATTERN.matches_word(word))",
        "let chosen_color_tail = &words[is_idx + 1..]",
        "SOURCE_IT_PATTERN.matches_words(subject_words)",
        "CHOSEN_COLOR_TAIL_PATTERN.matches_words(chosen_color_tail)",
        "THE_CHOSEN_COLOR_TAIL_PATTERN.matches_words(chosen_color_tail)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route chosen-color source lines through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_as_enters_simple_choice_parsers_use_token_tail_wrapper() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_as_enters_choice_subject_tokens",
        "pub(crate) fn parse_revealed_hand_choose_nonland_card_name_as_enters_line",
    );
    let note_life = function_source(
        &content,
        "pub(crate) fn parse_note_life_total_as_enters_line",
        "pub(crate) fn parse_source_is_chosen_type_in_addition_line",
    );
    let creature_type = function_source(
        &content,
        "pub(crate) fn parse_choose_creature_type_as_enters_line",
        "fn trigger_duplication_tail_matches",
    );
    let color_player = function_source(
        &content,
        "pub(crate) fn parse_choose_color_as_enters_line",
        "pub(crate) fn parse_damage_redirect_to_source_line",
    );

    for required in [
        "fn parse_as_enters_choice_subject_clause",
        "let word_refs = clause.word_refs()",
        "THE_BATTLEFIELD_PREFIX_PATTERN.matches(tail)",
        "fn parse_as_enters_choice_subject_tokens",
        "let clause = LexedClause::new(tokens)",
        "let words = clause.words()",
        "words.token_index_for_word_or_end(tail_word_idx)",
        "parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_AURA_SUBJECTS)",
        "parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)",
        "parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS)",
        "CHOOSE_CARD_NAME_TAIL_PATTERN.matches(LexedClause::new(tail_tokens))",
        "NOTE_YOUR_LIFE_TOTAL_TAIL_PATTERN.matches(LexedClause::new(tail_tokens))",
        "let tail_words = LexedClause::new(tail_tokens).word_refs()",
    ] {
        assert!(
            parser.contains(required)
                || note_life.contains(required)
                || creature_type.contains(required)
                || color_player.contains(required),
            "{relative} should route simple as-enters choice parsers through token tail wrapper: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = parser_token_word_refs(tokens)",
        "parse_as_enters_choice_subject_words(&words, AS_ENTERS_AURA_SUBJECTS)",
        "parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)",
        "parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS)",
        "THE_BATTLEFIELD_PREFIX_PATTERN.matches_words(&words[idx..])",
        "CHOOSE_CARD_NAME_TAIL_PATTERN.matches_words(&words[idx..])",
        "LexedClause::new(tail_tokens).matches_words(&[\"note\", \"your\", \"life\", \"total\"])",
        "words.get(idx..) != Some(&[\"note\", \"your\", \"life\", \"total\"][..])",
        "let words = crate::runtime_backend::token_word_refs(tokens)",
    ] {
        assert!(
            !parser.contains(forbidden)
                && !note_life.contains(forbidden)
                && !creature_type.contains(forbidden)
                && !color_player.contains(forbidden),
            "{relative} should not route simple as-enters choice parsers through raw full-line word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_choose_color_attached_uses_token_word_view() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_choose_color_as_becomes_attached_line",
        "pub(crate) fn parse_choose_player_as_enters_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let words = clause.words()",
        "AS_THIS_PREFIX_PATTERN.matches(clause)",
        "words.token_index_for_word_index(3)",
        "BECOMES_ATTACHED_TO_TAIL_PATTERN.matches(attached_tail.before(3))",
        "CHOOSE_WORD_PATTERN.matches_word(word)",
        "let word_refs = words.word_refs()",
        "render_token_slice(tokens)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse choose-color attached lines from token word views: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "AS_THIS_PREFIX_PATTERN.matches_words(&words)",
        "BECOMES_ATTACHED_TO_TAIL_PATTERN.matches_words(&words[3..6])",
        ".position(|word| CHOOSE_WORD_PATTERN.matches_word(word))",
        "words.join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route choose-color attached lines through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_damage_redirect_uses_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let redirect_to_source = function_source(
        &content,
        "pub(crate) fn parse_damage_redirect_to_source_line",
        "pub(crate) fn parse_damage_redirect_to_source_controller_line",
    );
    let redirect_to_controller = function_source(
        &content,
        "pub(crate) fn parse_damage_redirect_to_source_controller_line",
        "pub(crate) fn parse_no_more_than_creatures_can_attack_or_block_each_combat_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let words = clause.words()",
        "words.token_index_for_word_index(11)",
        "DAMAGE_REDIRECT_TO_SOURCE_PREFIX_PATTERN.matches(clause)",
        "DAMAGE_REDIRECT_TO_SOURCE_TAIL_PATTERN.matches(LexedClause::new(&tokens[tail_idx..]))",
        "IF_PREFIX_PATTERN.matches(clause)",
        "words.find_window_by(5",
        "words.token_index_for_word_or_end(would_idx + 5)",
        "IT_DEALS_DAMAGE_TO_ITS_CONTROLLER_INSTEAD_TAIL_PATTERN\n        .matches(LexedClause::new(&tokens[tail_idx..]))",
        "clause\n            .between_word_range(1, would_idx)",
    ] {
        assert!(
            redirect_to_source.contains(required) || redirect_to_controller.contains(required),
            "{relative} should parse damage-redirection statics from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "DAMAGE_REDIRECT_TO_SOURCE_PREFIX_PATTERN.matches_words(&words)",
        "DAMAGE_REDIRECT_TO_SOURCE_TAIL_PATTERN.matches_words(&words[11..])",
        "IF_PREFIX_PATTERN.matches_words(&words)",
        "find_window_by(&words",
        "let tail = &words[would_idx + 5..]",
        "IT_DEALS_DAMAGE_TO_ITS_CONTROLLER_INSTEAD_TAIL_PATTERN.matches_words(tail)",
    ] {
        assert!(
            !redirect_to_source.contains(forbidden) && !redirect_to_controller.contains(forbidden),
            "{relative} should not route damage-redirection statics through raw full-line word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_source_damage_prevention_uses_token_slices() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_prevent_damage_to_you_from_source_filter_line",
        "pub(crate) fn parse_replace_damage_with_counters_instead_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "IF_PREFIX_PATTERN.matches(clause)",
        "clause.find_phrase_start(&[\"would\", \"deal\", \"damage\", \"to\", \"you\"])",
        "let words = clause.words()",
        "words.token_index_for_word_or_end(would_idx + 5)",
        "let tail_clause = LexedClause::new(&tokens[tail_idx..])",
        "let tail_words = tail_clause.words()",
        "tail_words.first_is(\"prevent\")",
        "tail_words.slice_eq(2, &[\"of\", \"that\", \"damage\"])",
        ".between_word_range(1, would_idx)",
        "parse_damage_source_filter_tokens(source_tokens)",
        "clause.between_words_trimmed(1, would_idx + 5).text()",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse source damage prevention from token slices: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "word_slice_first_is(&words, \"if\")",
        "word_slice_find_phrase_start(&words, &[\"would\", \"deal\", \"damage\", \"to\", \"you\"])",
        "let tail = &words[would_idx + 5..]",
        "word_slice_first_is(tail, \"prevent\")",
        "word_slice_eq(&tail[2..], &[\"of\", \"that\", \"damage\"])",
        "parse_damage_source_filter_words(&words[1..would_idx])",
        "words[1..would_idx + 5].join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route source damage prevention through raw full-line word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_damage_to_counter_replacement_uses_clause_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_replace_damage_with_counters_instead_line",
        "pub(crate) fn parse_double_counters_replacement_line",
    );

    assert!(
        parser.contains(
            "NONCOMBAT_DAMAGE_TO_OPPONENT_CREATURE_MINUS_COUNTER_REPLACEMENT_PATTERN\n        .matches_non_article_tokens(tokens)"
        ),
        "{relative} should route noncombat damage counter replacement through an article-insensitive clause shape"
    );
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "let soul_scar_words = [",
        "let soul_scar_words_no_articles = [",
        "words.as_slice() != soul_scar_words",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route noncombat damage counter replacement through duplicate raw word arrays: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_damage_amount_replacement_source_articles_use_clause_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_damage_amount_replacement_line",
        "fn parse_damage_amount_replacement_target_filters",
    );

    assert!(
        parser.contains("ARTICLE_WORD_PATTERN.matches(LexedClause::new(source_tokens))"),
        "{relative} should detect article-only replacement sources with a clause shape"
    );
    assert!(
        parser.matches("IF_PREFIX_PATTERN.matches(clause)").count() >= 2,
        "{relative} should guard damage replacement parsers with token clause prefix shapes"
    );
    for required in [
        "fn keyword_find_exact_clause_window(",
        ".between_word_range(idx, idx + width)",
        ".is_some_and(|window| shape.matches(window))",
        "keyword_find_exact_clause_window(clause, 4, WOULD_DEAL_DAMAGE_TO_PHRASE_PATTERN)",
        "IT_DEALS_THAT_MUCH_DAMAGE_PLUS_PHRASE_PATTERN",
        "find_damage_multiplier_would_deal_phrase(clause)",
        "IT_DEALS_MULTIPLE_THAT_DAMAGE_TO_PHRASE_PATTERN",
        "keyword_find_exact_clause_window(clause, 5, WOULD_DEAL_DAMAGE_TO_YOU_PHRASE_PATTERN)",
    ] {
        assert!(
            content.contains(required) || parser.contains(required),
            "{relative} should route damage replacement phrase windows through token clause windows: missing `{required}`"
        );
    }
    for forbidden in [
        "let source_words = parser_token_word_refs(source_tokens)",
        "ARTICLE_WORD_PATTERN.matches_words(&source_words)",
        "IF_PREFIX_PATTERN.matches_words(&words)",
        "WOULD_DEAL_DAMAGE_TO_PHRASE_PATTERN.matches_words(window)",
        "IT_DEALS_THAT_MUCH_DAMAGE_PLUS_PHRASE_PATTERN.matches_words(window)",
        "IT_DEALS_MULTIPLE_THAT_DAMAGE_TO_PHRASE_PATTERN.matches_words(window)",
        "WOULD_DEAL_COMBAT_DAMAGE_TO_PHRASE_PATTERN.matches_words(window)",
        "WOULD_DEAL_DAMAGE_TO_YOU_PHRASE_PATTERN.matches_words(window)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not flatten replacement source tokens to check article-only sources: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_minimum_damage_replacement_prefixes_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_minimum_damage_amount_replacement_line",
        "pub(crate) fn parse_enter_as_copy_as_enters_line",
    );

    for required in [
        "let clause = LexedClause::new(&tokens)",
        "MINIMUM_RED_NONCOMBAT_DAMAGE_PREFIX_PATTERN.matches(clause)",
        ".is_some_and(|tail| AN_OPPONENT_PREFIX_PATTERN.matches(tail))",
        "clause.between_word_range(MINIMUM_RED_NONCOMBAT_DAMAGE_PREFIX_LEN, to_idx)",
        "clause.between_word_range(source_deals_idx + 6, words.len() - 1)",
        "fn damage_floor_value_clause_matches(clause: LexedClause<'_>) -> bool",
        "SOURCE_POWER_VALUE_PATTERN.matches(clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse minimum damage replacement guards through token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "MINIMUM_RED_NONCOMBAT_DAMAGE_PREFIX_PATTERN.matches_words(&words)",
        "AN_OPPONENT_PREFIX_PATTERN.matches_words(&words[to_idx + 1..])",
        "fn damage_floor_value_words_match(words: &[&str])",
        "SOURCE_POWER_VALUE_PATTERN.matches_words(words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse minimum damage replacement guards through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_draw_replacement_shape_gates_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_draw_replacement_exile_top_and_play_line",
        "pub(crate) fn parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "DRAW_REPLACEMENT_EXILE_TOP_PLAY_PREFIX_PATTERN.matches(clause)",
        ".is_some_and(|tail| DRAW_REPLACEMENT_EXILE_TOP_PLAY_TAIL_PATTERN.matches(tail))",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse draw-replacement shape gates through token clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "DRAW_REPLACEMENT_EXILE_TOP_PLAY_PREFIX_PATTERN.matches_words(&words)",
        "DRAW_REPLACEMENT_EXILE_TOP_PLAY_TAIL_PATTERN.matches_words(&words[11..])",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse draw-replacement shape gates through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_exile_replacement_shape_gates_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let exile_to_exile = function_source(
        &content,
        "pub(crate) fn parse_exile_to_exile_instead_of_graveyard_line",
        "fn parse_would_be_put_into_graveyard_owner_words",
    );
    let would_die = function_source(
        &content,
        "pub(crate) fn parse_exile_would_die_instead_line",
        "pub(crate) fn parse_pay_life_or_enter_tapped_line",
    );

    for required in [
        "EXILE_TO_EXILE_INSTEAD_OF_GRAVEYARD_MARKER_PATTERN.matches(clause)",
        ".is_some_and(|tail| WASNT_CYCLED_TAIL_PATTERN.matches(tail))",
        "let filter_clause = LexedClause::new(filter_tokens)",
        "CARD_OR_TOKEN_FILTER_PATTERN.matches(filter_clause)",
        "CARD_FILTER_PATTERN.matches(filter_clause)",
        "CREATURE_CARD_FILTER_PATTERN.matches(filter_clause)",
        "CYCLING_CARD_FILTER_PATTERN.matches(filter_clause)",
        ".is_some_and(|tail| DAMAGE_BY_PREFIX_PATTERN.matches(tail))",
        "WOULD_DIE_EXILE_INSTEAD_TAIL_PATTERN.matches(clause)",
        "let Some(damager_clause) = clause.between_word_range(damager_start, damager_end) else",
        "THIS_DAMAGED_BY_SOURCE_PATTERN.matches(damager_clause)",
        "EQUIPPED_CREATURE_DAMAGED_BY_PATTERN.matches(damager_clause)",
        "ENCHANTED_CREATURE_DAMAGED_BY_PATTERN.matches(damager_clause)",
    ] {
        assert!(
            exile_to_exile.contains(required) || would_die.contains(required),
            "{relative} should parse exile replacement shape gates through token clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "EXILE_TO_EXILE_INSTEAD_OF_GRAVEYARD_MARKER_PATTERN.matches_words(&words)",
        "WASNT_CYCLED_TAIL_PATTERN.matches_words(tail)",
        "CARD_OR_TOKEN_FILTER_PATTERN.matches_words(filter_words)",
        "CARD_FILTER_PATTERN.matches_words(filter_words)",
        "CREATURE_CARD_FILTER_PATTERN.matches_words(filter_words)",
        "CYCLING_CARD_FILTER_PATTERN.matches_words(filter_words)",
        "DAMAGE_BY_PREFIX_PATTERN.matches_words(&words[dealt_idx + 1..])",
        "WOULD_DIE_EXILE_INSTEAD_TAIL_PATTERN.matches_words(&words)",
        "THIS_DAMAGED_BY_SOURCE_PATTERN.matches_words(&damager_words)",
        "EQUIPPED_CREATURE_DAMAGED_BY_PATTERN.matches_words(&damager_words)",
        "ENCHANTED_CREATURE_DAMAGED_BY_PATTERN.matches_words(&damager_words)",
    ] {
        assert!(
            !exile_to_exile.contains(forbidden) && !would_die.contains(forbidden),
            "{relative} should not parse exile replacement shape gates through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_enter_as_copy_shape_gates_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_enter_as_copy_as_enters_line",
        "pub(crate) fn parse_choose_color_as_enters_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        ".is_some_and(|tail| ENTER_AS_COPY_EXILE_TWO_CREATURE_CARDS_PATTERN.matches(tail))",
        "ENTER_AS_COPY_IF_YOU_DO_PATTERN.matches(clause)",
        "ENTER_AS_COPY_COUNTER_POWER_MARKER_PATTERN.matches(clause)",
        "YOU_MAY_HAVE_PREFIX_PATTERN.matches(clause)",
        ".is_some_and(|tail| AS_A_COPY_OF_PREFIX_PATTERN.matches(tail))",
        "let copy_source_clause = clause",
        "THIS_COPY_SOURCE_PREFIX_PATTERN.matches(copy_source_clause)",
        "ENCHANTED_COPY_SOURCE_PREFIX_PATTERN.matches(copy_source_clause)",
        "let Some(tail_clause) = clause.after_words(except_idx + 1) else",
        "ITS_NAME_IS_PREFIX_PATTERN.matches(tail_clause)",
        ".is_some_and(|name_clause| THIS_COPY_SOURCE_PREFIX_PATTERN.matches(name_clause))",
        "IT_HAS_PREFIX_PATTERN.matches(tail_clause)",
        "NOT_LEGENDARY_COPY_EXCEPTION_PREFIX_PATTERN.matches(tail_clause)",
        "IT_IS_OR_ITS_PREFIX_PATTERN.matches(tail_clause)",
        "IN_ADDITION_TO_ITS_OTHER_CREATURE_TYPES_PREFIX_PATTERN.matches(tail)",
        "IN_ADDITION_TO_ITS_OTHER_TYPES_PREFIX_PATTERN.matches(tail)",
        "COPY_POWER_TOUGHNESS_FROM_SELF_TAIL_PATTERN.matches(remainder_clause)",
        "AND_IT_HAS_PREFIX_PATTERN.matches(remainder_clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse enter-as-copy shape gates through token clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "ENTER_AS_COPY_EXILE_TWO_CREATURE_CARDS_PATTERN\n            .matches_words(&clause_words[enter_idx + 1..])",
        "ENTER_AS_COPY_IF_YOU_DO_PATTERN.matches_words(&clause_words)",
        "ENTER_AS_COPY_COUNTER_POWER_MARKER_PATTERN.matches_words(&clause_words)",
        "YOU_MAY_HAVE_PREFIX_PATTERN.matches_words(&clause_words)",
        "AS_A_COPY_OF_PREFIX_PATTERN.matches_words(&clause_words[after_enter..])",
        "AS_A_COPY_OF_PREFIX_PATTERN.matches_words(&clause_words[idx..])",
        "THIS_COPY_SOURCE_PREFIX_PATTERN.matches_words(copy_source_words)",
        "ENCHANTED_COPY_SOURCE_PREFIX_PATTERN.matches_words(copy_source_words)",
        "THIS_COPY_SOURCE_PREFIX_PATTERN.matches_words(&name_words)",
        "ITS_NAME_IS_PREFIX_PATTERN.matches_words(tail)",
        "IT_HAS_PREFIX_PATTERN.matches_words(tail)",
        "NOT_LEGENDARY_COPY_EXCEPTION_PREFIX_PATTERN.matches_words(tail)",
        "IT_IS_OR_ITS_PREFIX_PATTERN.matches_words(tail)",
        "IN_ADDITION_TO_ITS_OTHER_CREATURE_TYPES_PREFIX_PATTERN\n                .matches_words(&tail[remainder_start..])",
        "IN_ADDITION_TO_ITS_OTHER_TYPES_PREFIX_PATTERN\n                .matches_words(&tail[remainder_start..])",
        "COPY_POWER_TOUGHNESS_FROM_SELF_TAIL_PATTERN\n                    .matches_words(&tail[remainder_start..])",
        "AND_IT_HAS_PREFIX_PATTERN.matches_words(&tail[remainder_start..])",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse enter-as-copy shape gates through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_characteristic_pt_shortcut_uses_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_characteristic_defining_pt_line",
        "let mut parsed_power: Option<Value> = None;",
    );

    for required in [
        "CHARACTERISTIC_POWER_TOUGHNESS_PATTERN.matches_non_article_tokens(tokens)",
        "CHARACTERISTIC_EQUAL_TO_PATTERN.matches_non_article_tokens(tokens)",
        "find_token_word_sequence_span(tokens, &[\"equal\", \"to\"])",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse the characteristic P/T shortcut from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let line_words = crate::runtime_backend::token_word_refs(tokens)",
        "CHARACTERISTIC_POWER_TOUGHNESS_PATTERN.matches_words(&line_words)",
        "CHARACTERISTIC_EQUAL_TO_PATTERN.matches_words(&line_words)",
        "keyword_find_prefix_shape_start(&line_words, &EQUAL_TO_PREFIX_PATTERN)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route the characteristic P/T shortcut through raw line words: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_cost_less_and_that_much_prefixes_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let cost_less = function_source(
        &content,
        "pub(crate) fn parse_if_this_spell_costs_less_to_cast_line",
        "pub(crate) fn parse_if_this_spell_costs_less_to_cast_line_lexed",
    );
    let that_much = function_source(
        &content,
        "pub(crate) fn parse_add_mana_that_much_value",
        "pub(crate) fn parse_players_skip_upkeep_line",
    );

    for required in [
        "IF_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "THIS_SPELL_COSTS_PREFIX_PATTERN.matches_non_article_tokens(&tail_tokens)",
        "parser_token_word_refs(tokens).join(\" \")",
        "CAST_WORD_MARKER_PATTERN.matches(LexedClause::new(remaining_tokens))",
        "THAT_MUCH_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
    ] {
        assert!(
            cost_less.contains(required) || that_much.contains(required),
            "{relative} should parse cost-less/that-much prefixes from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let words_all = crate::runtime_backend::token_word_refs(tokens)",
        "IF_PREFIX_PATTERN.matches_words(&words_all)",
        "let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens)",
        "THIS_SPELL_COSTS_PREFIX_PATTERN.matches_words(&tail_words)",
        "CAST_WORD_MARKER_PATTERN.matches_words(&remaining_words)",
        "THAT_MUCH_PREFIX_PATTERN.matches_words(&words_all)",
    ] {
        assert!(
            !cost_less.contains(forbidden) && !that_much.contains(forbidden),
            "{relative} should not route cost-less/that-much prefix parsing through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_cost_target_specs_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_this_spell_target_condition",
        "pub(crate) fn parse_cost_modifier_prefix_condition",
    );

    for required in [
        "let target_clause = LexedClause::new(&target_tokens)",
        "YOU_TARGET_PREFIX_PATTERN.matches(target_clause)",
        "OPPONENT_TARGET_PREFIX_PATTERN.matches(target_clause)",
        "PLAYER_TARGET_PREFIX_PATTERN.matches(target_clause)",
        "let target_clause = LexedClause::new(target_tokens)",
        "OPPONENT_OR_OPPONENTS_TARGET_PREFIX_PATTERN.matches(target_clause)",
        "PLAYER_OR_PLAYERS_TARGET_PREFIX_PATTERN.matches(target_clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse cost target specs through token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let target_words = crate::runtime_backend::token_word_refs(&target_tokens)",
        "let target_words = crate::runtime_backend::token_word_refs(target_tokens)",
        "YOU_TARGET_PREFIX_PATTERN.matches_words(&target_words)",
        "OPPONENT_TARGET_PREFIX_PATTERN.matches_words(&target_words)",
        "PLAYER_TARGET_PREFIX_PATTERN.matches_words(&target_words)",
        "OPPONENT_OR_OPPONENTS_TARGET_PREFIX_PATTERN.matches_words(&target_words)",
        "PLAYER_OR_PLAYERS_TARGET_PREFIX_PATTERN.matches_words(&target_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse cost target specs through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_spells_cost_modifier_markers_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_spells_cost_modifier_line",
        "pub(crate) fn parse_spell_and_player_activated_ability_cost_modifier_line",
    );
    let optional_life = function_source(
        &content,
        "fn parse_optional_life_additional_cost_reduction_line",
        "pub(crate) fn parse_spells_cost_modifier_line",
    );

    for required in [
        "THOSE_SPELLS_PAID_LIFE_THIS_WAY_PATTERN.matches(tail)",
        "let clause = LexedClause::new(tokens)",
        "FIRST_SPELL_EACH_TURN_COST_MODIFIER_PATTERN.matches(clause)",
        "let between_clause = LexedClause::new(between_tokens)",
        "YOU_CAST_PHRASE_PATTERN.matches(between_clause)",
        "FROM_YOUR_GRAVEYARD_PHRASE_PATTERN.matches(between_clause)",
        "OPPONENT_WORD_MARKER_PATTERN.matches(between_clause)",
        "CAST_OR_CASTS_WORD_MARKER_PATTERN.matches(between_clause)",
        "WHERE_X_IS_MARKER_PATTERN.matches(LexedClause::new(remaining_tokens))",
        "keyword_find_exact_clause_window(",
        "LexedClause::new(remaining_tokens),",
        "WHERE_X_IS_PREFIX_PATTERN,",
    ] {
        assert!(
            parser.contains(required) || optional_life.contains(required),
            "{relative} should parse spells-cost modifier markers through token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "THOSE_SPELLS_PAID_LIFE_THIS_WAY_PATTERN.matches_words(&additional_words[those_spells_idx..])",
        "FIRST_SPELL_EACH_TURN_COST_MODIFIER_PATTERN.matches_words(&clause_words)",
        "YOU_CAST_PHRASE_PATTERN.matches_words(&between_words)",
        "FROM_YOUR_GRAVEYARD_PHRASE_PATTERN.matches_words(&between_words)",
        "OPPONENT_WORD_MARKER_PATTERN.matches_words(&between_words)",
        "CAST_OR_CASTS_WORD_MARKER_PATTERN.matches_words(&between_words)",
        "WHERE_X_IS_MARKER_PATTERN.matches_words(&remaining_words)",
        "WHERE_X_IS_PREFIX_PATTERN.matches_words(window)",
    ] {
        assert!(
            !parser.contains(forbidden) && !optional_life.contains(forbidden),
            "{relative} should not parse spells-cost modifier markers through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_cost_modifier_family_markers_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let cycling = function_source(
        &content,
        "fn parse_cycling_cost_alternative_line",
        "fn parse_player_activated_ability_cost_modifier_clause",
    );
    let activated = function_source(
        &content,
        "fn parse_player_activated_ability_cost_modifier_clause",
        "fn strip_relative_target_clause",
    );
    let spell_and_player = function_source(
        &content,
        "pub(crate) fn parse_spell_and_player_activated_ability_cost_modifier_line",
        "fn parse_cycling_cost_alternative_line",
    );
    let strip_relative = function_source(
        &content,
        "fn strip_relative_target_clause",
        "pub(crate) fn parse_trailing_targets_condition_in_cost_modifier",
    );
    let trailing_targets = function_source(
        &content,
        "pub(crate) fn parse_trailing_targets_condition_in_cost_modifier",
        "pub(crate) fn parse_flashback_cost_modifier_line",
    );
    let flashback = function_source(
        &content,
        "pub(crate) fn parse_flashback_cost_modifier_line",
        "pub(crate) fn parse_equip_cost_modifier_line",
    );
    let equip = function_source(
        &content,
        "pub(crate) fn parse_equip_cost_modifier_line",
        "pub(crate) fn parse_foretelling_cards_cost_modifier_line",
    );
    let foretelling = function_source(
        &content,
        "pub(crate) fn parse_foretelling_cards_cost_modifier_line",
        "pub(crate) fn parse_cost_modifier_amount",
    );

    for required in [
        "AS_LONG_AS_PREFIX_PATTERN.matches(clause)",
        ".is_some_and(|body_clause| YOU_MAY_PAY_PREFIX_PATTERN.matches(body_clause))",
        "AND_ABILITIES_PREFIX_PATTERN.matches_non_article_tokens(window)",
        "let Some(activator_clause) = clause.between_word_range(1, activate_idx) else",
        "YOU_SUBJECT_PATTERN.matches(activator_clause)",
        "YOUR_OPPONENTS_ACTIVATOR_PATTERN.matches(activator_clause)",
        "TO_ACTIVATE_PHRASE_PATTERN.matches(remaining_clause)",
        "UNLESS_THEYRE_MANA_ABILITIES_PATTERN.matches(remaining_clause)",
        "THAT_TARGET_OR_TARGETS_PREFIX_PATTERN.matches_non_article_tokens(window)",
        "let Some(condition_clause) =",
        "IF_IT_TARGET_OR_TARGETS_PREFIX_PATTERN.matches(condition_clause)",
        "YOU_PAY_PHRASE_PATTERN.matches(clause)",
        "OPPONENTS_PAY_PHRASE_PATTERN.matches(clause)",
        "FORETELLING_CARDS_FROM_HAND_COSTS_PREFIX_PATTERN.matches(clause)",
        "ANY_PLAYER_TURN_PATTERN.matches(clause)",
    ] {
        assert!(
            cycling.contains(required)
                || activated.contains(required)
                || spell_and_player.contains(required)
                || strip_relative.contains(required)
                || trailing_targets.contains(required)
                || flashback.contains(required)
                || equip.contains(required)
                || foretelling.contains(required),
            "{relative} should parse cost-modifier family markers through token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "AS_LONG_AS_PREFIX_PATTERN.matches_words(&clause_words)",
        "YOU_MAY_PAY_PREFIX_PATTERN.matches_words(body_words)",
        "AND_ABILITIES_PREFIX_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(window))",
        "YOU_SUBJECT_PATTERN.matches_words(activator_words)",
        "YOUR_OPPONENTS_ACTIVATOR_PATTERN.matches_words(activator_words)",
        "TO_ACTIVATE_PHRASE_PATTERN.matches_words(&remaining_words)",
        "UNLESS_THEYRE_MANA_ABILITIES_PATTERN.matches_words(&remaining_words)",
        "THAT_TARGET_OR_TARGETS_PREFIX_PATTERN\n            .matches_words(&crate::runtime_backend::token_word_refs(window))",
        "IF_IT_TARGET_OR_TARGETS_PREFIX_PATTERN.matches_words(condition_words)",
        "YOU_PAY_PHRASE_PATTERN.matches_words(&clause_words)",
        "OPPONENTS_PAY_PHRASE_PATTERN.matches_words(&clause_words)",
        "FORETELLING_CARDS_FROM_HAND_COSTS_PREFIX_PATTERN.matches_words(&clause_words)",
        "ANY_PLAYER_TURN_PATTERN.matches_words(&clause_words)",
    ] {
        assert!(
            !cycling.contains(forbidden)
                && !activated.contains(forbidden)
                && !spell_and_player.contains(forbidden)
                && !strip_relative.contains(forbidden)
                && !trailing_targets.contains(forbidden)
                && !flashback.contains(forbidden)
                && !equip.contains(forbidden)
                && !foretelling.contains(forbidden),
            "{relative} should not parse cost-modifier family markers through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_this_spell_cost_condition_quantity_tails_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_this_spell_cost_condition",
        "fn parse_conjoined_this_spell_cost_condition",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        ".is_some_and(|tail| POISON_COUNTERS_TAIL_PATTERN.matches(tail))",
        ".is_some_and(|tail| CARDS_IN_OPPONENT_GRAVEYARD_TAIL_PATTERN.matches(tail))",
        ".is_some_and(|tail| LANDS_TAIL_PATTERN.matches(tail))",
        ".is_some_and(|tail| MORE_CREATURES_THAN_YOU_TAIL_PATTERN.matches(tail))",
        "TOTAL_CREATURE_CARDS_IN_ALL_GRAVEYARDS_TAIL_PATTERN.matches(tail)",
        ".is_some_and(|tail| SPELLS_THIS_TURN_TAIL_PATTERN.matches(tail))",
        ".is_some_and(|tail| CARDS_THIS_TURN_TAIL_PATTERN.matches(tail))",
        ".is_some_and(|tail| CREATURES_THIS_TURN_TAIL_PATTERN.matches(tail))",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse this-spell cost quantity tails through token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "POISON_COUNTERS_TAIL_PATTERN.matches_words(tail)",
        "CARDS_IN_OPPONENT_GRAVEYARD_TAIL_PATTERN.matches_words(tail)",
        "LANDS_TAIL_PATTERN.matches_words(tail)",
        "MORE_CREATURES_THAN_YOU_TAIL_PATTERN.matches_words(tail)",
        "TOTAL_CREATURE_CARDS_IN_ALL_GRAVEYARDS_TAIL_PATTERN.matches_words(tail)",
        "SPELLS_THIS_TURN_TAIL_PATTERN.matches_words(tail)",
        "CARDS_THIS_TURN_TAIL_PATTERN.matches_words(tail)",
        "CREATURES_THIS_TURN_TAIL_PATTERN.matches_words(tail)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse this-spell cost quantity tails through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_starting_life_buyback_and_target_cost_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_starting_life_bonus_line",
        "pub(crate) fn parse_if_this_spell_costs_less_to_cast_line",
    );

    for required in [
        "YOU_START_THE_GAME_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "ADDITIONAL_LIFE_MARKER_PATTERN.matches_non_article_tokens(tokens)",
        "BUYBACK_COSTS_COST_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "LESS_WORD_PATTERN.matches_token(token)",
        "find_token_word_sequence_span(tokens, &[\"this\", \"spell\", \"costs\"])",
        "TARGET_BEYOND_MORE_MARKER_PATTERN.matches_non_article_tokens(tokens)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse starting-life, buyback, and target-cost markers from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "YOU_START_THE_GAME_PREFIX_PATTERN.matches_words(&words)",
        "ADDITIONAL_LIFE_MARKER_PATTERN.matches_words(&words)",
        "BUYBACK_COSTS_COST_PREFIX_PATTERN.matches_words(&words)",
        "THIS_SPELL_COSTS_PREFIX_PATTERN.matches_words(&words)",
        "keyword_find_prefix_shape_start(&words, &THIS_SPELL_COSTS_PREFIX_PATTERN)",
        "TARGET_BEYOND_MORE_MARKER_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route starting-life, buyback, or target-cost markers through raw line words: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_upkeep_and_legend_markers_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_players_skip_upkeep_line",
        "pub(crate) fn parse_all_permanents_colorless_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "SKIP_YOUR_UPKEEP_STEP_PATTERN.matches(clause)",
        "let words = clause.words()",
        "words.token_index_for_word_or_end(5)",
        "parse_static_condition_clause(&tokens[condition_idx..])",
        "DOESNT_WORD_MARKER_PATTERN.matches_non_article_tokens(tokens)",
        "DOES_NOT_PHRASE_PATTERN.matches_non_article_tokens(tokens)",
        "LEGEND_RULE_APPLY_MARKER_PATTERN.matches_non_article_tokens(tokens)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse upkeep and legend-rule markers from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = parser_token_word_refs(tokens)",
        "SKIP_YOUR_UPKEEP_STEP_PATTERN.matches_words(&words)",
        "words.join(\" \")",
        "parse_static_condition_clause(&tokens[5..])",
        "DOESNT_WORD_MARKER_PATTERN.matches_words(&words)",
        "DOES_NOT_PHRASE_PATTERN.matches_words(&words)",
        "LEGEND_RULE_APPLY_MARKER_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route upkeep or legend-rule markers through raw parser words: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_type_addition_uses_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_subject_are_card_types_in_addition_to_their_other_types_line",
        "pub(crate) fn parse_all_cards_spells_permanents_colorless_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let words = clause.words()",
        "words.find_window_by(1",
        "BE_WORD_PATTERN.matches_word(word)",
        "keyword_find_exact_clause_window(clause, 5, IN_ADDITION_TO_OTHER_PATTERN)",
        ".between_word_range(0, be_idx)",
        ".between_word_range(be_idx + 1, addition_idx)",
        "CHOSEN_TYPE_PATTERN.matches(added_clause)",
        "parse_object_filter_lexed(subject_tokens, false)",
        "for descriptor in added_clause",
        ".filter_map(OwnedLexToken::as_word)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse type-addition lines from token word ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "find_index(&words, |word| BE_WORD_PATTERN.matches_word(word))",
        "let tail = &words[be_idx + 1..]",
        "find_window_by(tail, 5",
        "IN_ADDITION_TO_OTHER_PATTERN.matches_words(window)",
        "let subject_tokens = &tokens[..be_idx]",
        "let added_words = &tail[..addition_idx]",
        "CHOSEN_TYPE_PATTERN.matches_words(added_words)",
        "for descriptor in &tail[..addition_idx]",
        "let added_words = added_clause.word_refs()",
        "parse_object_filter(subject_tokens, false)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route type-addition lines through raw word slices or word-as-token indexes: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_all_cards_color_markers_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_all_cards_spells_permanents_colorless_line",
        "pub(crate) fn parse_all_are_color_and_type_addition_line",
    );

    for required in [
        "ALL_CARDS_SPELLS_PERMANENTS_COLORLESS_PATTERN.matches_non_article_tokens(tokens)",
        "ALL_CARDS_SPELLS_PERMANENTS_CHOSEN_COLOR_PATTERN.matches(LexedClause::new(tokens))",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse all-cards color markers from clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "ALL_CARDS_SPELLS_PERMANENTS_COLORLESS_PATTERN.matches_words(&words)",
        "let words = parser_token_word_refs(tokens)",
        "matches!(\n        words.as_slice()",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route all-cards color markers through raw word arrays: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_color_and_type_addition_uses_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_all_are_color_and_type_addition_line",
        "pub(crate) fn parse_all_creatures_are_color_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let words = clause.words()",
        "words.find_window_by(1",
        "ARE_WORD_PATTERN.matches_word(word)",
        "words.token_index_for_word_index(are_idx + 2)",
        "AND_ARE_PREFIX_PATTERN.matches(LexedClause::new(&tokens[and_are_token_idx..]))",
        "let descriptor_start = are_idx + 4",
        "keyword_find_exact_clause_window(clause, 5, IN_ADDITION_TO_THEIR_OTHER_PREFIX_PATTERN)",
        "clause.between_word_range(addition_idx + 5, words.len())",
        "CREATURE_TYPE_SCOPE_PATTERN.matches(scope_clause)",
        "clause.between_word_range(descriptor_start, addition_idx)",
        "for descriptor in descriptor_clause",
        ".filter_map(OwnedLexToken::as_word)",
        "render_token_slice(tokens)",
        ".between_word_range(0, are_idx)",
        "parse_object_filter_lexed(subject_tokens, false)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse color/type-addition lines from token word ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "find_index(&words, |word| ARE_WORD_PATTERN.matches_word(word))",
        "let tail = &words[are_idx + 4..]",
        "IN_ADDITION_TO_THEIR_OTHER_PREFIX_PATTERN.matches_words(window)",
        "IN_ADDITION_TO_THEIR_OTHER_PREFIX_PATTERN.matches_words(&tail[*idx..])",
        "let scope = &tail[addition_idx + 5..]",
        "CREATURE_TYPE_SCOPE_PATTERN.matches_words(scope)",
        "for descriptor in &tail[..addition_idx]",
        "let descriptor_words = descriptor_clause.word_refs()",
        "words.join(\" \")",
        "let subject_tokens = &tokens[..are_idx]",
        "parse_object_filter(subject_tokens, false)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route color/type-addition lines through raw word slices or word-as-token indexes: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_all_creatures_are_color_uses_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_all_creatures_are_color_line",
        "pub(crate) fn parse_subjects_are_basic_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let words = clause.words()",
        "words.find_window_by(1",
        "BE_WORD_PATTERN.matches_word(word)",
        "clause.between_word_range(are_idx + 1, words.len())",
        "let mut color_words = color_clause",
        ".filter_map(OwnedLexToken::as_word)",
        ".between_word_range(0, are_idx)",
        "parse_object_filter_lexed(subject_tokens, false)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse all-creatures-are-color lines from token word ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "find_index(&words, |word| BE_WORD_PATTERN.matches_word(word))",
        "match &words[are_idx + 1..]",
        "let color_words = color_clause.word_refs()",
        "let subject_tokens = &tokens[..are_idx]",
        "parse_object_filter(subject_tokens, false)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route all-creatures-are-color lines through raw word slices or word-as-token indexes: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_subjects_are_basic_uses_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_subjects_are_basic_line",
        "pub(crate) fn parse_nonbasic_lands_are_basic_land_type_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let words = clause.words()",
        "words.find_window_by(1",
        "BE_WORD_PATTERN.matches_word(word)",
        "clause.between_word_range(be_idx + 1, words.len())",
        "BASIC_TAIL_PATTERN.matches(tail_clause)",
        ".between_word_range(0, be_idx)",
        "trim_lexed_commas(clause.tokens())",
        "parse_object_filter_lexed(subject_tokens, false)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse subjects-are-basic lines from token word ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "find_index(&words, |word| matches!(*word, \"is\" | \"are\"))",
        "word_slice_eq(&words[be_idx + 1..], &[\"basic\"])",
        "tail_clause.matches_words(&[\"basic\"])",
        "let subject_tokens = trim_lexed_commas(&tokens[..be_idx])",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route subjects-are-basic lines through raw word slices or word-as-token indexes: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_land_type_parsers_use_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_nonbasic_lands_are_basic_land_type_line",
        "pub(crate) fn parse_lands_are_pt_creatures_still_lands_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let words = clause.words()",
        "words.find_window_by(1",
        "IS_OR_ARE_WORD_PATTERN.matches_word(word)",
        "clause.between_word_range(subtype_idx, words.len())",
        "let mut subtype_words = subtype_clause",
        ".filter_map(OwnedLexToken::as_word)",
        "clause.between_word_range(0, be_idx)",
        "parse_object_filter_lexed(subject_tokens, false)",
        "subject_clause.words().contains_window_by(1",
        "LAND_OR_LANDS_WORD_PATTERN.matches_word(word)",
        "clause.between_word_range(be_idx + 1, words.len())",
        "EVERY_BASIC_LAND_TYPE_ADDITION_TAIL_PATTERN.matches(after_be_clause)",
        "clause.between_word_range(subtype_word_idx + 1, words.len())",
        "LAND_TYPE_ADDITION_TAIL_PATTERN.matches(tail_clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse land-type statics from token word ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "IS_OR_ARE_WORD_PATTERN.find_word(&words)",
        "find_index(&words, |word| IS_OR_ARE_WORD_PATTERN.matches_word(word))",
        "word_refs_at_is_article(&words, subtype_idx)",
        "let subtype_words = &words[subtype_idx..]",
        "let subtype_words = subtype_clause.word_refs()",
        "let subject_tokens = &tokens[..be_idx]",
        "let filter_tokens = &tokens[..be_idx]",
        "let filter = parse_object_filter(filter_tokens, false)",
        "let filter = parse_object_filter(subject_tokens, false)",
        "let filter_words = &words[..be_idx]",
        "EVERY_BASIC_LAND_TYPE_ADDITION_TAIL_PATTERN.matches_words(&words[be_idx + 1..])",
        "let tail = &words[subtype_word_idx + 1..]",
        "LAND_TYPE_ADDITION_TAIL_PATTERN.matches_words(tail)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route land-type statics through raw word slices or word-as-token indexes: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_land_animation_uses_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_lands_are_pt_creatures_still_lands_line",
        "pub(crate) fn parse_filter_is_pt_creature_in_addition_and_has_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let words = clause.words()",
        "words.find_window_by(1",
        "IS_OR_ARE_WORD_PATTERN.matches_word(word)",
        "clause.between_word_range(be_idx + 3, words.len())",
        "STILL_LAND_ANIMATION_TAIL_PATTERN.matches(tail_clause)",
        ".between_word_range(0, be_idx)",
        "parse_object_filter_lexed(filter_tokens, false)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse land animation statics from token word ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "find_index(&words, |word| IS_OR_ARE_WORD_PATTERN.matches_word(word))",
        "let tail = &words[be_idx + 3..]",
        "STILL_LAND_ANIMATION_TAIL_PATTERN.matches_words(tail)",
        "let filter_tokens = &tokens[..be_idx]",
        "parse_object_filter(filter_tokens, false)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route land animation statics through raw word slices or word-as-token indexes: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_heterogeneous_animation_attached_probe_uses_token_word_view() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_filter_is_pt_creature_in_addition_and_has_line",
        "pub(crate) fn parse_subject_is_subtype_with_base_pt_and_granted_abilities_line",
    );

    for required in [
        "let clause_words = LexedClause::new(tokens).word_refs()",
        "let attached_subject = LexedClause::new(&subject_tokens)",
        ".words()",
        ".first()",
        "ENCHANTED_OR_EQUIPPED_WORD_PATTERN.matches_word(word)",
        "let before_has_clause = LexedClause::new(&before_has)",
        "let raw_before_has_words = before_has_clause.word_refs()",
        ".between_word_range(tail_start_word, tail_end_word)",
        ".is_some_and(|tail_clause| OTHER_TYPE_ADDITION_TAIL_PATTERN.matches(tail_clause))",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should detect attached heterogeneous-animation subjects through token word views: missing `{required}`"
        );
    }
    assert!(
        !parser.contains("crate::runtime_backend::token_word_refs(&subject_tokens)"),
        "{relative} should not rebuild raw subject words for heterogeneous-animation attached probe"
    );
    assert!(
        !parser.contains("let clause_words = crate::runtime_backend::token_word_refs(tokens)"),
        "{relative} should not rebuild raw full-line words for heterogeneous-animation helper context"
    );
    assert!(
        !parser.contains("crate::runtime_backend::token_word_refs(&before_has)"),
        "{relative} should not rebuild raw before-has words for heterogeneous-animation parser"
    );
    assert!(
        !parser.contains("OTHER_TYPE_ADDITION_TAIL_PATTERN.matches_words(tail)"),
        "{relative} should not match heterogeneous-animation type-addition tails through raw word slices"
    );
}

#[test]
fn keyword_static_pay_life_etb_gates_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_pay_life_or_enter_tapped_line",
        "pub(crate) fn parse_copy_activated_abilities_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let words = clause.word_refs()",
        "AS_THIS_CONTAINS_PAY_LIFE_PATTERN.matches(clause)",
        "IF_YOU_DONT_PHRASE_PATTERN.matches(clause)",
        ".is_some_and(|trailing| PAY_LIFE_ENTER_TAPPED_TAIL_PATTERN.matches(trailing))",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse pay-life ETB gates through token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "normalized_storage",
        "word.replace",
        "AS_THIS_CONTAINS_PAY_LIFE_PATTERN.matches_words(&words)",
        "IF_YOU_DONT_PHRASE_PATTERN.matches_words(&words)",
        "PAY_LIFE_ENTER_TAPPED_TAIL_PATTERN.matches_words(trailing)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse pay-life ETB gates through manual normalized word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_copy_activated_abilities_gates_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_copy_activated_abilities_line",
        "pub(crate) fn parse_spend_mana_as_any_color_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        ".is_some_and(|tail| HAS_ALL_ACTIVATED_ABILITIES_OF_PATTERN.matches(tail))",
        "let filter_clause = LexedClause::new(&filter_tokens)",
        ".is_some_and(|window| ACTIVATE_EACH_OF_THOSE_ONCE_TAIL_PATTERN.matches(window))",
        ".is_some_and(|window| SAME_NAME_AS_SOURCE_CREATURE_PATTERN.matches(window))",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse copy-activated-ability gates through token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "HAS_ALL_ACTIVATED_ABILITIES_OF_PATTERN.matches_words(&clause_words[idx..])",
        "ACTIVATE_EACH_OF_THOSE_ONCE_TAIL_PATTERN.matches_words(window)",
        "SAME_NAME_AS_SOURCE_CREATURE_PATTERN.matches_words(window)",
        "parser_token_word_refs(&filter_tokens)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse copy-activated-ability gates through raw word windows: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_subtype_base_pt_grant_animation_uses_token_word_views() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_subject_is_subtype_with_base_pt_and_granted_abilities_line",
        "pub(crate) fn parse_creatures_cant_block_line",
    );

    for required in [
        "let attached_subject = LexedClause::new(&subject_tokens)",
        ".words()",
        ".first()",
        "ENCHANTED_OR_EQUIPPED_WORD_PATTERN.matches_word(word)",
        "let type_words = LexedClause::new(&type_tokens).word_refs()",
        "let word_len = LexedClause::new(&after_with).word_len()",
        "LexedClause::new(&after_with).between_word_range(idx, idx + 6)",
        "LOSES_ALL_OTHER_CREATURE_TYPES_PATTERN.matches(window_clause)",
        "LexedClause::new(&after_with).token_index_for_word_index(note_start)",
        "let after_with_clause = LexedClause::new(after_with)",
        "let after_with_words = after_with_clause.word_refs()",
        "BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches(after_with_clause)",
        "after_with_clause.token_index_for_word_index(ability_start_word_idx)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse subtype/base-PT/grant animation helper words through token views: missing `{required}`"
        );
    }
    for forbidden in [
        "crate::runtime_backend::token_word_refs(&subject_tokens)",
        "let type_words = crate::runtime_backend::token_word_refs(&type_tokens)",
        "crate::runtime_backend::token_word_refs(&after_with)",
        "token_index_for_word_index(&after_with, note_start)",
        "token_index_for_word_index(&after_with, ability_start_word_idx)",
        "BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches_words(&after_with_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild subtype/base-PT/grant animation helper words through raw word helpers: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_choose_not_untap_uses_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_may_choose_not_to_untap_during_untap_step_line",
        "pub(crate) fn parse_untap_during_each_other_players_untap_step_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "YOU_MAY_CHOOSE_NOT_TO_UNTAP_PREFIX_PATTERN.matches(clause)",
        "DURING_YOUR_UNTAP_STEP_TAIL_PATTERN.matches(clause)",
        "let words = clause.words()",
        "clause.between_word_range(6, words.len() - 4)",
        "MAY_CHOOSE_NOT_UNTAP_SOURCE_SUBJECT_PATTERN.matches(subject_clause)",
        "let subject = subject_clause.text()",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse choose-not-untap lines from token word ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "YOU_MAY_CHOOSE_NOT_TO_UNTAP_PREFIX_PATTERN.matches_words(&words)",
        "DURING_YOUR_UNTAP_STEP_TAIL_PATTERN.matches_words(&words)",
        "let subject_words = &words[6..words.len() - 4]",
        "MAY_CHOOSE_NOT_UNTAP_SOURCE_SUBJECT_PATTERN.matches_words(subject_words)",
        "subject_words.join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route choose-not-untap lines through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_activation_cost_uses_captured_shape_and_mana_permission_tails_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let activation_cost = function_source(
        &content,
        "pub(crate) fn parse_activated_abilities_cost_increase_line",
        "pub(crate) fn parse_activated_abilities_cant_be_activated_line_lexed",
    );
    let mana_permission = function_source(
        &content,
        "pub(crate) fn parse_spend_mana_as_any_color_line",
        "include!(\"keyword_lines.rs\");",
    );

    for required in [
        "fn parse_activated_abilities_cost_increase_spec",
        "LexPattern::object(\n            \"subject\",\n            LexCaptureKind::UntilAnyPhrase(ACTIVATED_ABILITY_COST_VERB_PHRASES),\n        )",
        "LexPattern::modifier(\"additional_cost\", LexCaptureKind::UntilPhrase(TO_ACTIVATE_PHRASE))",
        "matched\n        .capture_clause_by_role(LexCaptureRole::Object, clause)",
        "matched\n        .capture_clause_by_role(LexCaptureRole::Modifier, clause)",
        "parse_object_filter_lexed(spec.subject_tokens, false)",
        "parse_activation_cost(additional_cost_tokens)",
    ] {
        assert!(
            activation_cost.contains(required),
            "{relative} should parse activated-ability cost increases from captured grammar pieces: missing `{required}`"
        );
    }

    for required in [
        "SPEND_MANA_ANY_TYPE_CAST_PREFIX_PATTERN.matches(clause)",
        "PLAYERS_MAY_SPEND_MANA_ANY_COLOR_PREFIX_PATTERN.matches(clause)",
        "YOU_MAY_SPEND_MANA_ANY_COLOR_PREFIX_PATTERN.matches(clause)",
        "PAY_ACTIVATION_COSTS_OF_PREFIX_PATTERN.matches_non_article_tokens(&tail_tokens)",
        "ABILITY_OR_ABILITIES_MARKER_PATTERN.matches_non_article_tokens(ability_tokens)",
        "ACTIVATE_ABILITIES_OF_PREFIX_PATTERN.matches_non_article_tokens(&tail_tokens)",
    ] {
        assert!(
            activation_cost.contains(required) || mana_permission.contains(required),
            "{relative} should parse activation-cost/mana-permission tails from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let clause_words = crate::runtime_backend::token_word_refs(tokens)",
        "ACTIVATED_ABILITIES_OF_PREFIX_PATTERN.matches_words(&clause_words)",
        "find_index(&clause_words, |word|",
        "let subject_tokens = trim_commas(&tokens[3..cost_idx])",
        "let amount_words = crate::runtime_backend::token_word_refs(&amount_tokens)",
        "ADDITIONAL_COST_PREFIX_PATTERN.matches_words(&amount_words)",
        "crate::runtime_backend::token_word_refs(&cost_tokens)",
        "let tail_words = crate::runtime_backend::token_word_refs(&cost_tokens[to_token_idx..])",
        "TO_ACTIVATE_PREFIX_PATTERN.matches_words(&tail_words)",
        "TO_ACTIVATE_PREFIX_PATTERN.matches_non_article_tokens(&cost_tokens[to_token_idx..])",
        "SPEND_MANA_ANY_TYPE_CAST_PREFIX_PATTERN.matches_words(&clause_words)",
        "PLAYERS_MAY_SPEND_MANA_ANY_COLOR_PREFIX_PATTERN.matches_words(&clause_words)",
        "YOU_MAY_SPEND_MANA_ANY_COLOR_PREFIX_PATTERN.matches_words(&clause_words)",
        "let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens)",
        "PAY_ACTIVATION_COSTS_OF_PREFIX_PATTERN.matches_words(&tail_words)",
        "ACTIVATE_ABILITIES_OF_PREFIX_PATTERN.matches_words(&tail_words)",
    ] {
        assert!(
            !activation_cost.contains(forbidden) && !mana_permission.contains(forbidden),
            "{relative} should not route activation-cost/mana-permission tail parsing through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_search_attack_land_and_retrace_gates_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let search = function_source(
        &content,
        "pub(crate) fn parse_control_opponents_while_searching_libraries_line",
        "pub(crate) fn parse_cast_this_spell_as_though_it_had_flash_line",
    );
    let attack_land = function_source(
        &content,
        "pub(crate) fn parse_attacks_each_combat_if_able_line",
        "pub(crate) fn parse_play_lands_from_graveyard_line",
    );
    let retrace = function_source(
        &content,
        "pub(crate) fn parse_graveyard_cards_have_retrace_line",
        "fn parse_retrace_grant_card_types",
    );

    for required in [
        "CONTROL_OPPONENTS_WHILE_SEARCHING_PATTERN.matches(LexedClause::new(tokens))",
        "OPPONENT_SEARCH_EXILE_FOUND_CARDS_PATTERN.matches(LexedClause::new(tokens))",
        "CAST_THIS_CARD_FROM_LIBRARY_WHILE_SEARCHING_PATTERN.matches(LexedClause::new(tokens))",
        "ATTACHED_CONTROLLER_ATTACK_EACH_COMBAT_PATTERN.matches(clause)",
        ".is_some_and(|tail| ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches(tail))",
        "YOU_MAY_PLAY_PREFIX_PATTERN.matches(clause)",
        ".is_some_and(|tail| UP_TO_PREFIX_PATTERN.matches(tail))",
        ".is_some_and(|tail| ADDITIONAL_LAND_PLAY_TAIL_PATTERN.matches(tail))",
        ".is_some_and(|tail| RETRACE_TAIL_PATTERN.matches(tail))",
        "let Some(prefix_clause) = clause.between_word_range(prefix_start, have_idx) else",
        "STATIC_IN_YOUR_GRAVEYARD_SUFFIX_PATTERN.matches(prefix_clause)",
    ] {
        assert!(
            search.contains(required)
                || attack_land.contains(required)
                || retrace.contains(required),
            "{relative} should parse search/attack/land/retrace gates through token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "CONTROL_OPPONENTS_WHILE_SEARCHING_PATTERN.matches_words(&words)",
        "OPPONENT_SEARCH_EXILE_FOUND_CARDS_PATTERN.matches_words(&words)",
        "CAST_THIS_CARD_FROM_LIBRARY_WHILE_SEARCHING_PATTERN.matches_words(&words)",
        "ATTACHED_CONTROLLER_ATTACK_EACH_COMBAT_PATTERN.matches_words(&words)",
        "ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches_words(&words[attack_idx..])",
        "YOU_MAY_PLAY_PREFIX_PATTERN.matches_words(&words)",
        "UP_TO_PREFIX_PATTERN.matches_words(&words[count_word_idx..])",
        "ADDITIONAL_LAND_PLAY_TAIL_PATTERN.matches_words(rest_words)",
        "RETRACE_TAIL_PATTERN.matches_words(&words[have_idx + 1..])",
        "STATIC_IN_YOUR_GRAVEYARD_SUFFIX_PATTERN.matches_words(prefix)",
    ] {
        assert!(
            !search.contains(forbidden)
                && !attack_land.contains(forbidden)
                && !retrace.contains(forbidden),
            "{relative} should not parse search/attack/land/retrace gates through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_activation_restriction_wrapper_uses_lexed_spec() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_activated_abilities_cant_be_activated_line",
        "pub(crate) fn parse_activated_abilities_cost_increase_line",
    );

    for required in [
        "parse_activated_abilities_cant_be_activated_spec_lexed(tokens)",
        "let subject_tokens = spec.subject_tokens",
        "parse_object_filter_lexed(subject_tokens, false)",
        "let non_mana_only = spec.non_mana_only",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should lower activated-ability restrictions from the lexed captured spec: missing `{required}`"
        );
    }
    for forbidden in [
        "let normalized = crate::runtime_backend::token_word_refs(tokens)",
        "ACTIVATED_ABILITIES_OF_PREFIX_PATTERN.matches_words(&normalized)",
        "find_index(&normalized, |word| CANT_WORD_PATTERN.matches_word(word))",
        "CANT_BE_ACTIVATED_TAIL_PATTERN.matches_words(tail)",
        "let subject_tokens = trim_commas(&tokens[3..cant_idx])",
        "MANA_ABILITIES_EXCEPTION_PATTERN.matches_words(&normalized)",
        "parse_object_filter(&subject_tokens, false)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not keep the old raw-word activated-ability restriction parser: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_trigger_duplication_and_untap_if_tails_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let trigger_duplication = function_source(
        &content,
        "fn trigger_duplication_tail_matches",
        "fn parse_trigger_duplication_source_filter",
    );
    let trigger_event = function_source(
        &content,
        "fn parse_trigger_duplication_source_filter",
        "fn parse_trigger_duplication_core",
    );
    let trigger_core = function_source(
        &content,
        "fn parse_trigger_duplication_core",
        "pub(crate) fn parse_trigger_duplication_line",
    );
    let untap_parser = function_source(
        &content,
        "pub(crate) fn parse_doesnt_untap_during_untap_step_line",
        "pub(crate) fn parse_flying_restriction_line",
    );

    for required in [
        "fn trigger_duplication_tail_matches(tokens: &[OwnedLexToken]) -> bool",
        "TRIGGER_DUPLICATION_TAIL_PATTERN.matches_non_article_tokens(tokens)",
        "trigger_duplication_tail_matches(&tail_tokens)",
        "TRIGGER_DUPLICATION_SOURCE_PATTERN.matches(LexedClause::new(&tokens))",
        "let clause = LexedClause::new(&tokens)",
        "let clause_display = render_token_slice(&tokens)",
        "TURNING_FACE_UP_TRIGGER_DUPLICATION_PATTERN.matches(clause)",
        "YOU_CASTING_OR_COPYING_PREFIX_PATTERN.matches(clause)",
        "IF_WORD_PATTERN.matches_token(token)",
        "let clause_display = render_token_slice(tokens)",
        "IF_PREFIX_PATTERN.matches(LexedClause::new(&head_tokens))",
        "let body_clause = LexedClause::new(body_tokens)",
        "ClauseShape::new().prefix(prefix).matches(body_clause)",
        ".is_some_and(|tail| WHILE_PREFIX_PATTERN.matches(tail))",
        "let source_body_clause = LexedClause::new(source_body_tokens)",
        ".matches(source_body_clause)",
        "TO_TRIGGER_SUFFIX_PATTERN.matches(source_body_clause)",
        "ClauseShape::new().suffix(suffix).matches(clause)",
        "let subject_len = clause.word_len() - suffix.len()",
        "let Some(subject_clause) = clause.between_word_range(0, subject_len) else",
        "PLAYER_SUBJECT_PATTERN.matches(subject_clause)",
        "YOU_SUBJECT_PATTERN.matches(subject_clause)",
        "OPPONENT_SUBJECT_PATTERN.matches(subject_clause)",
    ] {
        assert!(
            trigger_duplication.contains(required)
                || trigger_event.contains(required)
                || trigger_core.contains(required)
                || untap_parser.contains(required),
            "{relative} should parse trigger-duplication/untap-if tails from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "fn trigger_duplication_tail_matches(words: &[&str])",
        "TRIGGER_DUPLICATION_TAIL_PATTERN.matches_words(words)",
        "let filter_words = crate::runtime_backend::token_word_refs(&tokens)",
        "TRIGGER_DUPLICATION_SOURCE_PATTERN.matches_words(&filter_words)",
        "let phrase_words = crate::runtime_backend::token_word_refs(&tokens)",
        "TURNING_FACE_UP_TRIGGER_DUPLICATION_PATTERN.matches_words(&phrase_words)",
        "YOU_CASTING_OR_COPYING_PREFIX_PATTERN.matches_words(&phrase_words)",
        "phrase_words.join(\" \")",
        "let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens)",
        "IF_WORD_PATTERN.matches_first_word(&tail_words)",
        "let head_words = crate::runtime_backend::token_word_refs(&head_tokens)",
        "IF_PREFIX_PATTERN.matches_words(&head_words)",
        "let clause_words = crate::runtime_backend::token_word_refs(tokens)",
        "ClauseShape::new().prefix(prefix).matches_words(&body_words)",
        "WHILE_PREFIX_PATTERN.matches_words(tail)",
        "let source_words = crate::runtime_backend::token_word_refs(source_body_tokens)",
        "TO_TRIGGER_SUFFIX_PATTERN.matches_words(&source_words)",
        "let phrase_words = clause.word_refs()",
        "ClauseShape::new()\n            .suffix(suffix)\n            .matches_words(&phrase_words)",
        "PLAYER_SUBJECT_PATTERN.matches_words(subject_words)",
        "YOU_SUBJECT_PATTERN.matches_words(subject_words)",
        "OPPONENT_SUBJECT_PATTERN.matches_words(subject_words)",
    ] {
        assert!(
            !trigger_duplication.contains(forbidden)
                && !trigger_event.contains(forbidden)
                && !trigger_core.contains(forbidden)
                && !untap_parser.contains(forbidden),
            "{relative} should not route trigger-duplication/untap-if tail parsing through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_untap_display_and_mana_cost_grants_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let other_players_untap = function_source(
        &content,
        "pub(crate) fn parse_untap_during_each_other_players_untap_step_line",
        "pub(crate) fn parse_doesnt_untap_during_untap_step_line",
    );
    let life_mana_value = function_source(
        &content,
        "pub(crate) fn parse_life_mana_value_instead_of_mana_cost_grant_line",
        "pub(crate) fn parse_fixed_mana_cost_instead_of_mana_cost_grant_line",
    );
    let mana_value = function_source(
        &content,
        "pub(crate) fn parse_mana_value_instead_of_mana_cost_grant_line",
        "pub(crate) fn parse_life_mana_value_instead_of_mana_cost_grant_line",
    );
    let fixed_mana_cost = function_source(
        &content,
        "pub(crate) fn parse_fixed_mana_cost_instead_of_mana_cost_grant_line",
        "pub(crate) fn parse_grant_flash_to_noncreature_spells_line",
    );

    for required in [
        "let subject_text = render_token_slice(&subject_tokens)",
        "MANA_VALUE_INSTEAD_OF_MANA_COST_GRANT_PREFIX_PATTERN\n        .matches(LexedClause::new(head_tokens))",
        "LIFE_MANA_VALUE_INSTEAD_OF_MANA_COST_GRANT_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "LIFE_MANA_VALUE_INSTEAD_OF_MANA_COST_TAIL_PATTERN\n        .matches_non_article_tokens(tokens.get(by_idx + 1..).unwrap_or_default())",
        "SPELL_OR_SPELLS_CONTAINS_PATTERN.matches_non_article_tokens(subject_tokens)",
        "let clause = LexedClause::new(tokens)",
        "YOU_MAY_PAY_PREFIX_PATTERN.matches(clause)",
        "WHERE_WORD_PATTERN.matches_token(token)",
        "let tail_clause = LexedClause::new(tail_tokens)",
        "RATHER_THAN_PAY_MANA_COST_FOR_PREFIX_PATTERN.matches(tail_clause)",
        "tail_clause.token_index_after_words(7)",
    ] {
        assert!(
            other_players_untap.contains(required)
                || mana_value.contains(required)
                || life_mana_value.contains(required)
                || fixed_mana_cost.contains(required),
            "{relative} should use token-shaped parsing for untap display/mana-cost grants: missing `{required}`"
        );
    }
    for forbidden in [
        "let line_words = crate::runtime_backend::token_word_refs(tokens)",
        "parser_token_word_refs(tokens).join(\" \")",
        "let subject_text = crate::runtime_backend::token_word_refs(&subject_tokens).join(\" \")",
        "super::lexer::parser_token_word_refs(head_tokens)",
        "MANA_VALUE_INSTEAD_OF_MANA_COST_GRANT_PREFIX_PATTERN.matches_words(words.as_slice())",
        "let words = super::lexer::parser_token_word_refs(tokens)",
        "LIFE_MANA_VALUE_INSTEAD_OF_MANA_COST_GRANT_PREFIX_PATTERN.matches_words(words.as_slice())",
        "let tail_words =\n        crate::runtime_backend::token_word_refs(tokens.get(by_idx + 1..).unwrap_or_default())",
        "LIFE_MANA_VALUE_INSTEAD_OF_MANA_COST_TAIL_PATTERN.matches_words(&tail_words)",
        "let subject_words = crate::runtime_backend::token_word_refs(subject_tokens)",
        "SPELL_OR_SPELLS_CONTAINS_PATTERN.matches_words(&subject_words)",
        "YOU_MAY_PAY_PREFIX_PATTERN.matches_words(words.as_slice())",
        "let tail_words = super::lexer::parser_token_word_refs(tail_tokens)",
        "RATHER_THAN_PAY_MANA_COST_FOR_PREFIX_PATTERN.matches_words(tail_words.as_slice())",
        "tail_tokens.get(7..)",
    ] {
        assert!(
            !other_players_untap.contains(forbidden)
                && !mana_value.contains(forbidden)
                && !life_mana_value.contains(forbidden)
                && !fixed_mana_cost.contains(forbidden),
            "{relative} should not route untap display/mana-cost grants through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_exile_counter_permission_grant_uses_lexed_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_you_may_cast_exile_counter_cards_with_mana_permission_line",
        "pub(crate) fn parse_surveilled_graveyard_play_life_cost_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let word_view = clause.words()",
        "let word_refs = word_view.word_refs()",
        "PLAY_LANDS_CAST_NONCREATURE_EXILED_PREFIX_PATTERN.matches(clause)",
        "keyword_find_prefix_shape_start(clause, &AND_YOU_MAY_SPEND_MANA_PREFIX_PATTERN)",
        "keyword_find_prefix_shape_start(clause, &THAT_HAVE_PREFIX_PATTERN)",
        ".filter(|idx| *idx + 2 <= and_idx)",
        "clause\n            .between_word_range(counters_idx + 1, word_refs.len())",
        "ON_THEM_PREFIX_PATTERN.matches(tail)",
        "OPPONENT_OWNED_PREFIX_PATTERN.matches(owner_clause)",
        "word_view.token_range_for_word_range(counter_start_idx, counters_idx + 1)",
        "SPEND_SNOW_MANA_AS_ANY_COLOR_FOR_THOSE_SPELLS_PATTERN.matches(spend_clause)",
        "SPEND_MANA_AS_ANY_COLOR_FOR_THOSE_SPELLS_PATTERN.matches(spend_clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route exile-counter permission grant parsing through lexed clause ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = parser_token_word_refs(tokens)",
        "PLAY_LANDS_CAST_NONCREATURE_EXILED_PREFIX_PATTERN.matches_words(&words)",
        "keyword_find_prefix_shape_start(&word_refs, &AND_YOU_MAY_SPEND_MANA_PREFIX_PATTERN)",
        "keyword_find_prefix_shape_start(&words, &AND_YOU_MAY_SPEND_MANA_PREFIX_PATTERN)",
        "WITH_WORD_PATTERN.find_word(&words[..and_idx])",
        "keyword_find_prefix_shape_start(&words[..and_idx], &THAT_HAVE_PREFIX_PATTERN)",
        "ON_THEM_PREFIX_PATTERN.matches_words(&words[counters_idx + 1..])",
        "let owner_words =",
        "OPPONENT_OWNED_PREFIX_PATTERN.matches_words(owner_words)",
        "let counter_tokens = &tokens[counter_start_idx..counters_idx + 1]",
        "let spend_words = &words[and_idx..]",
        "SPEND_SNOW_MANA_AS_ANY_COLOR_FOR_THOSE_SPELLS_PATTERN.matches_words(spend_words)",
        "SPEND_MANA_AS_ANY_COLOR_FOR_THOSE_SPELLS_PATTERN.matches_words(spend_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route exile-counter permission grant parsing through raw word offsets: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_surveilled_graveyard_permission_uses_clause_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_surveilled_graveyard_play_life_cost_line",
        "pub(crate) fn parse_you_may_static_grant_line",
    );

    assert!(
        parser.contains(
            "SURVEILLED_GRAVEYARD_PLAY_LIFE_COST_PATTERN.matches(LexedClause::new(tokens))"
        ),
        "{relative} should parse surveilled graveyard life-cost permission through a clause shape"
    );
    for forbidden in [
        "let words = parser_token_word_refs(tokens)",
        "word.replace(['\\'', '’'], \"\")",
        "let refs = normalized.iter().map(String::as_str).collect::<Vec<_>>()",
        "refs.as_slice()",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse surveilled graveyard life-cost permission through hand-normalized raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_you_may_static_grant_uses_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_you_may_static_grant_line",
        "pub(crate) fn parse_play_from_permission_with_haste_this_way_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "SOURCE_LINKED_EXILE_CAST_PREFIX_PATTERN.matches(clause)",
        "ANY_MANA_CAST_SUFFIX_PATTERN.matches(clause)",
        "clause.word_len() > 19 + 11",
        "CAST_SINGLE_SPELL_PATTERN.matches(clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse static grant line probes through clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let words = parser_token_word_refs(tokens)",
        "SOURCE_LINKED_EXILE_CAST_PREFIX_PATTERN.matches_words(&words)",
        "ANY_MANA_CAST_SUFFIX_PATTERN.matches_words(&words)",
        "words.len() > 19 + 11",
        "let clause_words = crate::runtime_backend::token_word_refs(tokens)",
        "CAST_SINGLE_SPELL_PATTERN.matches_words(&clause_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse static grant line probes through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_play_from_haste_followup_uses_clause_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_play_from_permission_with_haste_this_way_line",
        "pub(crate) fn parse_you_may_look_top_card_any_time_line",
    );

    assert!(
        parser.contains(
            "CAST_CREATURE_THIS_WAY_HASTE_SENTENCE_PATTERN.matches(LexedClause::new(haste_sentence))"
        ),
        "{relative} should parse cast-this-way haste follow-up through a clause shape"
    );
    for forbidden in [
        "let haste_words = parser_token_word_refs(haste_sentence)",
        "\"if\", \"you\", \"cast\", \"a\", \"creature\", \"spell\", \"this\", \"way\", \"it\", \"gains\", \"haste\"",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse cast-this-way haste follow-up through raw word arrays: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_pregame_and_extra_block_parsers_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let pregame_begin = function_source(
        &content,
        "pub(crate) fn parse_pregame_begin_on_battlefield_line",
        "pub(crate) fn parse_pregame_mulligan_redraw_line",
    );
    let pregame_mulligan = function_source(
        &content,
        "pub(crate) fn parse_pregame_mulligan_redraw_line",
        "pub(crate) fn parse_pregame_choose_color_line",
    );
    let pregame_choose_color = function_source(
        &content,
        "pub(crate) fn parse_pregame_choose_color_line",
        "pub(crate) fn parse_combined_pregame_choose_color_line",
    );
    let extra_block = function_source(
        &content,
        "pub(crate) fn parse_can_block_additional_creature_each_combat_line",
        "pub(crate) fn parse_skulk_rules_text_line",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "let clause_display = render_token_slice(tokens)",
        "PREGAME_BEGIN_ON_BATTLEFIELD_PATTERN.matches(clause)",
        "find_token_word_sequence_span(tokens, &[\"on\", \"the\", \"battlefield\"])",
        "PREGAME_COUNTER_ON_IT_TAIL_PATTERN.matches_non_article_tokens(trailing)",
        "PREGAME_EXILE_FROM_HAND_TAIL_PATTERN.matches_non_article_tokens(trailing)",
        "PREGAME_MULLIGAN_REDRAW_PATTERN.matches(LexedClause::new(tokens))",
        "let words = LexedClause::new(tokens).words()",
        "let tail_start = choose_idx + consumed",
        ".is_some_and(|tail| BEFORE_GAME_BEGINS_TAIL_PATTERN.matches(tail))",
        "SOURCE_CAN_BLOCK_PREFIX_PATTERN.matches(clause)",
        "BLOCK_ADDITIONAL_DURATION_TAIL_PATTERN.matches_non_article_tokens(tail)",
    ] {
        assert!(
            pregame_begin.contains(required)
                || pregame_mulligan.contains(required)
                || pregame_choose_color.contains(required)
                || extra_block.contains(required),
            "{relative} should parse pregame/additional-block static lines from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let clause_words = crate::runtime_backend::token_word_refs(tokens)",
        "PREGAME_BEGIN_ON_BATTLEFIELD_PATTERN.matches_words(&clause_words)",
        "NOT_STARTING_PLAYER_CONDITION_PATTERN.matches_words(&clause_words)",
        "keyword_find_prefix_shape_start(&clause_words, &ON_THE_BATTLEFIELD_PREFIX_PATTERN)",
        "PREGAME_COUNTER_ON_IT_TAIL_PATTERN.matches_words(&trailing)",
        "PREGAME_EXILE_FROM_HAND_TAIL_PATTERN.matches_words(&trailing)",
        "PREGAME_MULLIGAN_REDRAW_PATTERN.matches_words(&clause_words)",
        "BEFORE_GAME_BEGINS_TAIL_PATTERN.matches_words(tail)",
        "let normalized = crate::runtime_backend::token_word_refs(tokens)",
        "SOURCE_CAN_BLOCK_PREFIX_PATTERN.matches_words(&normalized)",
        "BLOCK_ADDITIONAL_DURATION_TAIL_PATTERN.matches_words(tail)",
    ] {
        assert!(
            !pregame_begin.contains(forbidden)
                && !pregame_mulligan.contains(forbidden)
                && !pregame_choose_color.contains(forbidden)
                && !extra_block.contains(forbidden),
            "{relative} should not route pregame/additional-block parsing through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_count_as_card_named_uses_token_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn parse_count_as_card_named_for_spell_effect_line",
        "fn parse_static_ability_ast_line_early_lexed",
    );
    let early_parser = function_source(
        &content,
        "fn parse_static_ability_ast_line_early_lexed",
        "pub(crate) fn parse_static_ability_ast_line_lexed",
    );

    for required in [
        "fn parse_count_as_card_named_for_spell_effect_line(tokens: &[OwnedLexToken])",
        "let clause = LexedClause::new(tokens)",
        "COUNT_AS_CARD_NAMED_GRAVEYARD_PREFIX_PATTERN.matches(clause)",
        ".between_word_range(count_idx, count_idx + 6)",
        ".is_some_and(|tail| COUNT_IT_AS_A_CARD_NAMED_PATTERN.matches(tail))",
        "parse_count_as_card_named_for_spell_effect_line(tokens)",
    ] {
        assert!(
            helper.contains(required) || early_parser.contains(required),
            "{relative} should parse count-as-card-named static lines through token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_count_as_card_named_for_spell_effect_line(words: &[&str])",
        "COUNT_AS_CARD_NAMED_GRAVEYARD_PREFIX_PATTERN.matches_words(words)",
        "COUNT_IT_AS_A_CARD_NAMED_PATTERN.matches_words(tail)",
        "parse_count_as_card_named_for_spell_effect_line(&words)",
    ] {
        assert!(
            !helper.contains(forbidden) && !early_parser.contains(forbidden),
            "{relative} should not parse count-as-card-named static lines through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_ward_wrapper_uses_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_ward_static_ability_line",
        "pub(crate) fn parse_ward_discard_card_type_cost",
    );

    for required in [
        "WARD_WORD_PATTERN.matches_token(token)",
        "render_token_slice(tokens)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse/render the ward wrapper from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let clause_words = crate::runtime_backend::token_word_refs(tokens)",
        "WARD_WORD_PATTERN.matches_first_word(&clause_words)",
        "clause_words.join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route the ward wrapper through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_ward_discard_cost_uses_token_word_view() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_ward_discard_card_type_cost",
        "pub(crate) fn parse_composed_anthem_effects_line",
    );

    for required in [
        "let words = LexedClause::new(tokens).words()",
        "DISCARD_WORD_PATTERN.matches_word(word)",
        "words.token_index_for_word_index(idx)",
        "LexedClause::new(&tokens[count_token_idx..used_end]).word_len()",
        "words.token_index_for_word_or_end(idx)",
        "WARD_DISCARD_HAND_TAIL_PATTERN.matches(LexedClause::new(&tokens[tail_token_idx..]))",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse ward discard costs from token word views: missing `{required}`"
        );
    }
    for forbidden in [
        "let cost_words = crate::runtime_backend::token_word_refs(tokens)",
        "DISCARD_WORD_PATTERN.matches_first_word(&cost_words)",
        "let words_tail = &cost_words[idx..]",
        "WARD_DISCARD_HAND_TAIL_PATTERN.matches_words(words_tail)",
        "while cost_words",
        "while let Some(word) = cost_words.get(idx)",
        "idx != cost_words.len()",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route ward discard costs through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_composed_anthem_uses_token_shape_guards() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_composed_anthem_effects_line",
        "pub(crate) fn parse_filter_dont_untap_during_controllers_untap_steps_line",
    );

    for required in [
        "find_token_word_sequence_span(tokens, &[\"until\", \"end\", \"of\", \"turn\"]).is_some()",
        "WHERE_X_IS_PREFIX_PATTERN.matches_non_article_tokens(&where_tail)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should guard composed anthem parsing with token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let clause_words = crate::runtime_backend::token_word_refs(tokens)",
        "contains_until_end_of_turn(&clause_words)",
        "let where_words = crate::runtime_backend::token_word_refs(&where_tail)",
        "WHERE_X_IS_PREFIX_PATTERN.matches_words(&where_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not guard composed anthem parsing through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_keyword_action_replacement_prefixes_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_keyword_action_replacement_line",
        "pub(crate) fn parse_exile_to_countered_exile_instead_of_graveyard_line",
    );

    for required in [
        "YOU_PROLIFERATE_TWICE_INSTEAD_PATTERN.matches_non_article_tokens(tokens)",
        "OPPONENT_PROLIFERATES_TWICE_INSTEAD_PATTERN.matches_non_article_tokens(tokens)",
        "CONTROLLED_CREATURE_EXPLORE_REPLACEMENT_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "let Some(tail_clause) = LexedClause::new(tokens).after_words(8) else",
        "YOU_SCRY_PREFIX_PATTERN.matches(tail_clause)",
        "fn keyword_action_replacement_subject_explores(clause: LexedClause<'_>)",
        "EXPLORE_REPLACEMENT_SUBJECT_PATTERN.matches(clause)",
        ".is_some_and(keyword_action_replacement_subject_explores)",
        "EXPLORES_TWICE_TAIL_PATTERN.matches(tail_clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse keyword-action replacement prefixes from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "YOU_PROLIFERATE_TWICE_INSTEAD_PATTERN.matches_words(&line_words)",
        "OPPONENT_PROLIFERATES_TWICE_INSTEAD_PATTERN.matches_words(&line_words)",
        "CONTROLLED_CREATURE_EXPLORE_REPLACEMENT_PREFIX_PATTERN.matches_words(&line_words)",
        "YOU_SCRY_PREFIX_PATTERN.matches_words(tail)",
        "EXPLORE_REPLACEMENT_SUBJECT_PATTERN.matches_words(words)",
        "keyword_action_replacement_subject_explores(&tail[then_idx + 1..])",
        "EXPLORES_TWICE_TAIL_PATTERN.matches_words(tail)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route keyword-action replacement prefixes through raw line words: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_dynamic_cost_front_branches_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let helpers = function_source(
        &content,
        "fn dynamic_cards_drawn_this_turn_player",
        "pub(crate) fn parse_can_be_attached_only_to_line",
    );
    let parser = function_source(
        &content,
        "pub(crate) fn parse_dynamic_cost_modifier_value",
        "pub(crate) fn parse_add_mana_that_much_value",
    );

    for required in [
        "fn dynamic_cards_drawn_this_turn_player_tokens(tokens: &[OwnedLexToken]) -> Option<PlayerFilter>",
        "fn dynamic_spell_cast_this_turn_player_tokens(tokens: &[OwnedLexToken]) -> Option<PlayerFilter>",
        "dynamic_cards_drawn_this_turn_player_tokens(tokens)",
        "find_index(tokens, |token| EACH_WORD_PATTERN.matches_token(token))",
        "KICK_COUNT_DYNAMIC_PREFIX_PATTERN.matches_non_article_tokens(filter_tokens)",
        "CREATURES_DIED_THIS_TURN_PREFIX_PATTERN.matches_non_article_tokens(filter_tokens)",
        "LIFE_OPPONENTS_LOST_THIS_TURN_PREFIX_PATTERN.matches_non_article_tokens(filter_tokens)",
        "dynamic_spell_cast_this_turn_player_tokens(filter_tokens)",
        "CARD_TYPES_IN_GRAVEYARD_DYNAMIC_PATTERN.matches_non_article_tokens(filter_tokens)",
        "CREATURE_TYPES_AMONG_PREFIX_PATTERN.matches_non_article_tokens(filter_tokens)",
        "let filter_clause = LexedClause::new(filter_tokens)",
        "CARD_TYPES_AMONG_MARKER_PATTERN.matches(filter_clause)",
        "COUNTERS_REMOVED_THIS_WAY_PATTERN.matches(filter_clause)",
        "DESTROYED_THIS_WAY_PATTERN.matches(filter_clause)",
        "SACRIFICED_THIS_WAY_PATTERN.matches(filter_clause)",
        "DISCARDED_THIS_WAY_PATTERN.matches(filter_clause)",
        "EXILED_THIS_WAY_PATTERN.matches(filter_clause)",
        "REVEALED_THIS_WAY_PATTERN.matches(filter_clause)",
        "SOURCE_COUNTER_REFERENCE_PREFIX_PATTERN.matches(tail)",
        "THIS_WAY_PATTERN.matches(filter_clause)",
        "let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);\n    if word_slice_starts_with_any(",
    ] {
        assert!(
            helpers.contains(required) || parser.contains(required),
            "{relative} should parse dynamic cost front branches from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "fn dynamic_cards_drawn_this_turn_player(words: &[&str])",
        "fn dynamic_spell_cast_this_turn_player(words: &[&str])",
        "let words_all = crate::runtime_backend::token_word_refs(tokens)",
        "find_index(&words_all, |word| EACH_WORD_PATTERN.matches_word(word))",
        "KICK_COUNT_DYNAMIC_PREFIX_PATTERN.matches_words(&filter_words)",
        "CREATURES_DIED_THIS_TURN_PREFIX_PATTERN.matches_words(&filter_words)",
        "LIFE_OPPONENTS_LOST_THIS_TURN_PREFIX_PATTERN.matches_words(&filter_words)",
        "CARD_TYPES_IN_GRAVEYARD_DYNAMIC_PATTERN.matches_words(&filter_words)",
        "CREATURE_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words)",
        "CARD_TYPES_AMONG_MARKER_PATTERN.matches_words(&filter_words)",
        "COUNTERS_REMOVED_THIS_WAY_PATTERN.matches_words(&filter_words)",
        "DESTROYED_THIS_WAY_PATTERN.matches_words(&filter_words)",
        "SACRIFICED_THIS_WAY_PATTERN.matches_words(&filter_words)",
        "DISCARDED_THIS_WAY_PATTERN.matches_words(&filter_words)",
        "EXILED_THIS_WAY_PATTERN.matches_words(&filter_words)",
        "REVEALED_THIS_WAY_PATTERN.matches_words(&filter_words)",
        "SOURCE_COUNTER_REFERENCE_PREFIX_PATTERN.matches_words(tail)",
        "THIS_WAY_PATTERN.matches_words(&filter_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route dynamic cost front branches through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_reduced_hand_size_guards_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_reduced_maximum_hand_size_line",
        "pub(crate) fn parse_effect_discard_to_library_replacement_line",
    );

    for required in [
        "fn max_hand_size_subject_prefix(clause: LexedClause<'_>)",
        "MAX_HAND_SIZE_YOU_SUBJECT_PATTERN.matches(clause)",
        "MAX_HAND_SIZE_AS_LONG_AS_PREFIX_PATTERN.matches_non_article_tokens(tokens)",
        "let line_clause = LexedClause::new(tokens)",
        "let Some(tail) = line_clause.after_words(word_idx)",
        "max_hand_size_subject_prefix(tail)",
        ".is_some_and(|tail| MAX_HAND_SIZE_IS_PATTERN.matches(tail))",
        "let working_clause = LexedClause::new(working_tokens)",
        "max_hand_size_subject_prefix(working_clause)",
        ".is_some_and(|tail| MAX_HAND_SIZE_REDUCED_PATTERN.matches(tail))",
        ".is_some_and(|tail| MAX_HAND_SIZE_INCREASED_PATTERN.matches(tail))",
        ".is_some_and(|tail| MAX_HAND_SIZE_SEVEN_MINUS_CARD_TYPES_PATTERN.matches(tail))",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse reduced hand-size guards with token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "fn max_hand_size_subject_prefix(words: &[&str])",
        "MAX_HAND_SIZE_YOU_SUBJECT_PATTERN.matches_words(words)",
        "let tail = &line_words[word_idx..]",
        "max_hand_size_subject_prefix(&line_words)",
        "MAX_HAND_SIZE_AS_LONG_AS_PREFIX_PATTERN.matches_words(&line_words)",
        "MAX_HAND_SIZE_IS_PATTERN.matches_words(tail)",
        "MAX_HAND_SIZE_REDUCED_PATTERN.matches_words(tail)",
        "MAX_HAND_SIZE_INCREASED_PATTERN.matches_words(tail)",
        "MAX_HAND_SIZE_SEVEN_MINUS_CARD_TYPES_PATTERN.matches_words(tail)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse reduced hand-size guards through raw word tails: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_characteristic_pt_axis_parser_keeps_token_word_view() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_characteristic_defining_pt_line",
        "fn parse_characteristic_defining_relative_value",
    );

    for required in [
        "let line_words = LexedClause::new(tokens).words()",
        "parse_characteristic_axis_clause_start(&line_words, idx)",
        "line_words.token_index_for_word_index(value_start_word_idx)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should keep characteristic P/T axis parsing tied to token-word view: missing `{required}`"
        );
    }
    assert!(
        !parser.contains("let line_words = crate::runtime_backend::token_word_refs(tokens)"),
        "{relative} should not detach characteristic P/T axis parsing into raw line words"
    );
}

#[test]
fn keyword_static_characteristic_pt_value_helpers_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_characteristic_defining_relative_value",
        "pub(crate) fn parse_shuffle_into_library_from_graveyard_line",
    );

    for required in [
        "let words = LexedClause::new(trimmed).words()",
        "THAT_NUMBER_PREFIX_PATTERN.matches_non_article_tokens(trimmed)",
        "SOURCE_POWER_VALUE_PATTERN.matches_non_article_tokens(trimmed)",
        "SOURCE_TOUGHNESS_VALUE_PATTERN.matches_non_article_tokens(trimmed)",
        "CARD_TYPES_AMONG_MARKER_PATTERN.matches_non_article_tokens(trimmed)",
        "let words = LexedClause::new(tokens).words()",
        "NUMBER_OF_PREFIX_PATTERN.matches_non_article_tokens(start)",
        "BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_non_article_tokens(start)",
        "COLORS_AMONG_PREFIX_PATTERN.matches_non_article_tokens(start)",
        "CARD_TYPES_AMONG_PREFIX_PATTERN.matches_non_article_tokens(start)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse characteristic P/T value helpers from token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "THAT_NUMBER_PREFIX_PATTERN.matches_words(&words)",
        "SOURCE_POWER_VALUE_PATTERN.matches_words(&trimmed_words)",
        "SOURCE_TOUGHNESS_VALUE_PATTERN.matches_words(&trimmed_words)",
        "CARD_TYPES_AMONG_MARKER_PATTERN.matches_words(&trimmed_words)",
        "let words = crate::runtime_backend::token_word_refs(tokens)",
        "NUMBER_OF_PREFIX_PATTERN.matches_words(&initial_start_words)",
        "BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(&start_words)",
        "COLORS_AMONG_PREFIX_PATTERN.matches_words(&start_words)",
        "CARD_TYPES_AMONG_PREFIX_PATTERN.matches_words(&start_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not route characteristic P/T value helpers through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn object_filter_other_than_exclusion_uses_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/reference_tag_stage.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "// \"other than this/it/them ...\"",
        "if let Some(mut disjunction) = parse_attached_reference_or_another_disjunction(&base_tokens)?",
    );

    for required in [
        "non_article_token_words_eq(&base_tokens[idx..idx + 2], OTHER_THAN_PREFIX)",
        "for piece in token.parser_word_pieces()",
        "let tail_tokens = &base_tokens[idx + 2..];",
        "non_article_token_words_starts_with_any(tail_tokens, EXCLUSION_RELATION_IGNORED_PREFIXES)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse other-than exclusions from token-derived word helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "OTHER_THAN_PREFIX_PATTERN",
        "EXCLUSION_RELATION_IGNORED_PREFIX_PATTERN",
        "let base_words_view = GrammarFilterNormalizedWords::new(&base_tokens[..idx])",
        "let tail_words_view = GrammarFilterNormalizedWords::new(&base_tokens[idx + 2..])",
        "OTHER_THAN_PREFIX_PATTERN.matches_words(&[",
        "EXCLUSION_RELATION_IGNORED_PREFIX_PATTERN.matches_words(&tail_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild other-than exclusion word lists: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_source_attack_control_gate_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "parse_source_did_not_attack_or_enter_control_this_turn_shape(predicate_tokens)",
        "fn is_source_did_not_attack_subject_clause(clause: LexedClause<'_>) -> bool",
        "strip_leading_article_tokens(clause.trimmed().tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route the source attack/control predicate gate through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "let source_state_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&filtered)",
        "parse_source_did_not_attack_or_enter_control_this_turn_shape(&source_state_tokens)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild source state predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_exploited_triggering_object_uses_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_exploited_triggering_object_predicate",
        "pub(crate) fn parse_predicate",
    );

    for required in [
        "fn parse_exploited_triggering_object_predicate(tokens: &[OwnedLexToken])",
        "LexPattern::subject(\"subject\", LexCaptureKind::WordCount(1))",
        "LexPattern::action(\"action\", LexCaptureKind::OneOf(&[\"exploited\"]))",
        "LexPattern::object(\"object\", LexCaptureKind::Rest)",
        "clause_matches_phrase(subject, &[\"it\"])",
        "clause_matches_any_phrase(object, &[&[\"that\", \"creature\"], &[\"that\", \"object\"]])",
        "parse_exploited_triggering_object_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required) || parser.contains(required),
            "{relative} should parse exploited-triggering-object predicates with captured token clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "IT_EXPLOITED_TRIGGERING_PATTERN",
        "IT_EXPLOITED_TRIGGERING_PATTERN.matches_words(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not parse exploited-triggering-object predicates from exact filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_spell_lifecycle_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_spell_lifecycle_predicate",
        "fn parse_you_cast_source_shape",
    );

    for required in [
        "fn parse_spell_lifecycle_predicate(tokens: &[OwnedLexToken])",
        "parse_you_cast_source_shape(tokens)",
        "parse_tagged_was_cast_shape(tokens)",
        "parse_this_spell_was_cast_from_shape(tokens)",
        "parse_no_spells_cast_last_turn_shape(tokens)",
        "parse_this_spell_paid_named_label_shape(tokens)",
        "parse_target_was_kicked_shape(tokens)",
        "parse_spell_lifecycle_predicate(predicate_tokens)",
        "strip_leading_article_tokens(clause.trimmed().tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route spell lifecycle predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_spell_lifecycle_predicate(words: &[&str])",
        "parse_spell_lifecycle_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild spell lifecycle predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild spell lifecycle predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_paid_cost_label_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_paid_cost_label_predicate",
        "fn paid_cost_tail_is_negated",
    );

    for required in [
        "fn parse_paid_cost_label_predicate(tokens: &[OwnedLexToken])",
        "let clause = LexedClause::new(tokens)",
        "parse_paid_cost_label_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route paid-cost label predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_paid_cost_label_predicate(words: &[&str])",
        "parse_paid_cost_label_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild paid-cost label predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild paid-cost label predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_attached_tagged_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_attached_tagged_predicate",
        "fn parse_this_permanent_attached_to_shape",
    );

    for required in [
        "fn parse_attached_tagged_predicate(tokens: &[OwnedLexToken])",
        "parse_this_permanent_attached_to_shape(tokens)",
        "parse_attached_tagged_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route attached-tagged predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_attached_tagged_predicate(words: &[&str])",
        "parse_attached_tagged_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild attached-tagged predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild attached-tagged predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_sacrificed_state_uses_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_sacrificed_permanent_state_predicate",
        "fn parse_tagged_exiled_predicate",
    );
    let color_helper = function_source(
        &content,
        "fn parse_color_only_object_filter_tokens",
        "fn strip_clause_suffix",
    );

    for required in [
        "fn parse_sacrificed_permanent_state_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_color_only_object_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter>",
        "token_word_is(token, AND_WORD) || token_word_is(token, OR_WORD)",
        "parse_color(token.parser_text())",
        "parse_non_color(token.parser_text())",
        "parse_color_only_object_filter_tokens(clause.tokens())",
        "LexPattern::word(\"sacrificed\")",
        "LexPattern::subject(\"subject\", LexCaptureKind::WordCount(1))",
        "let Some(subject_token) = subject.token(0) else",
        "parse_card_type(subject_token.parser_text())",
        "token_word_is(subject_token, PERMANENT_WORD)",
        "LexPattern::word(\"was\")",
        "LexPattern::modifier(\"descriptor\", LexCaptureKind::Rest)",
        "parse_object_filter(descriptor.tokens(), false)",
        "parse_color_only_object_filter_clause(descriptor)",
        "parse_sacrificed_permanent_state_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required)
                || parser.contains(required)
                || color_helper.contains(required),
            "{relative} should parse sacrificed-state predicates through captured token clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "SACRIFICED_WORD_PATTERN.matches_word_at(&filtered",
        "WAS_WORD_PATTERN.matches_word_at(&filtered",
        "synthetic_word_tokens(\n                &filtered",
        "parse_color_only_object_filter_words(&filtered",
        "let Some(subject_word) = subject.word_refs().first().copied()",
        "parse_card_type(subject_word)",
        "PERMANENT_WORD_PATTERN.matches_word(subject_word)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not parse sacrificed-state predicates from filtered raw word indexes: found `{forbidden}`"
        );
    }
    for forbidden in ["parse_color_only_object_filter_word_refs(&clause.word_refs())"] {
        assert!(
            !color_helper.contains(forbidden),
            "{relative} should parse captured color-only object filters from tokens: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_tagged_state_and_exile_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_tagged_exiled_predicate",
        "fn parse_tagged_controlled_permanent_shape",
    );

    for required in [
        "fn parse_tagged_exiled_predicate(tokens: &[OwnedLexToken])",
        "fn parse_tagged_state_predicate(tokens: &[OwnedLexToken])",
        "parse_tagged_state_predicate(predicate_tokens)",
        "parse_tagged_exiled_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route tagged state/exile predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_tagged_exiled_predicate(words: &[&str])",
        "fn parse_tagged_state_predicate(words: &[&str])",
        "parse_tagged_state_predicate(&filtered)",
        "parse_tagged_exiled_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild tagged state/exile predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild tagged state/exile predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_among_value_shapes_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_colors_among_predicate",
        "fn parse_life_total_at_least_starting_predicate",
    );

    for required in [
        "fn parse_colors_among_predicate(tokens: &[OwnedLexToken])",
        "fn parse_card_types_among_predicate(tokens: &[OwnedLexToken])",
        "fn permanents_you_control_scope(clause: LexedClause<'_>)",
        "fn cards_in_your_graveyard_scope(clause: LexedClause<'_>)",
        "fn permanents_and_your_graveyard_scope(clause: LexedClause<'_>)",
        "clause_matches_any_phrase(clause, PERMANENTS_YOU_CONTROL_SCOPE_PHRASES)",
        "clause_matches_any_phrase(clause, CARDS_IN_YOUR_GRAVEYARD_SCOPE_PHRASES)",
        "clause_matches_any_phrase(scope, SACRIFICED_PERMANENTS_SCOPE_PHRASES)",
        "clause.between_word_range(battlefield_end, battlefield_end + 1)",
        "clause_matches_phrase(tail, PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PHRASE)",
        "clause_matches_phrase(tail, PERMANENTS_AND_OR_SPLIT_CONNECTOR_PHRASE)",
        "fn parse_happily_style_conjoined_predicate(tokens: &[OwnedLexToken])",
        "parse_happily_style_conjoined_predicate(predicate_tokens)",
        "parse_colors_among_predicate(predicate_tokens)",
        "parse_card_types_among_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route among-value predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_colors_among_predicate(words: &[&str])",
        "fn parse_card_types_among_predicate(words: &[&str])",
        "fn parse_happily_style_conjoined_predicate(words: &[&str])",
        "parse_happily_style_conjoined_predicate(&filtered)",
        "parse_colors_among_predicate(&filtered)",
        "parse_card_types_among_predicate(&filtered)",
        "fn permanents_you_control_scope(words: &[&str])",
        "fn cards_in_your_graveyard_scope(words: &[&str])",
        "fn permanents_and_your_graveyard_scope(words: &[&str])",
        "PERMANENTS_YOU_CONTROL_SCOPE_PATTERN.matches(clause)",
        "CARDS_IN_YOUR_GRAVEYARD_SCOPE_PATTERN.matches(clause)",
        "SACRIFICED_PERMANENTS_SCOPE_PATTERN.matches(scope)",
        "PERMANENTS_YOU_CONTROL_SCOPE_PATTERN.matches_words(words)",
        "CARDS_IN_YOUR_GRAVEYARD_SCOPE_PATTERN.matches_words(words)",
        "let scope_words = scope.word_refs()",
        "scope_words.as_slice()",
        "PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PATTERN.matches(tail)",
        "PERMANENTS_AND_OR_SPLIT_CONNECTOR_PATTERN.matches(tail)",
        "PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PATTERN.matches_words(tail)",
        "PERMANENTS_AND_OR_SPLIT_CONNECTOR_PATTERN.matches_words(tail)",
        "permanents_you_control_scope(&scope.word_refs())",
        "permanents_and_your_graveyard_scope(&scope_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild among-value predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild among-value predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_revealed_or_controlled_subtype_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_revealed_or_controlled_subtype_predicate",
        "fn is_card_graveyard_existential_clause",
    );

    for required in [
        "fn parse_revealed_or_controlled_subtype_predicate(\n    tokens: &[OwnedLexToken]",
        "let revealed_token = revealed_subtype.token(0)?",
        "let controlled_token = controlled_subtype.token(0)?",
        "revealed_token.parser_text() != controlled_token.parser_text()",
        "parse_subtype_word(revealed_token.parser_text())",
        "parse_revealed_or_controlled_subtype_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route revealed/control subtype predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_revealed_or_controlled_subtype_predicate(words: &[&str])",
        "parse_revealed_or_controlled_subtype_predicate(&filtered)",
        "let revealed_words = revealed_subtype.word_refs()",
        "let controlled_words = controlled_subtype.word_refs()",
        "parse_subtype_word(revealed_words.first().copied()?)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild revealed/control subtype predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild revealed/control subtype predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_vote_results_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_vote_result_predicate",
        "fn parse_spell_context_predicate",
    );

    for required in [
        "fn parse_vote_result_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_vote_option_result_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_no_vote_objects_matched_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_vote_result_predicate(predicate_tokens, true)",
        "parse_vote_result_predicate(predicate_tokens, false)",
        "option.tokens().is_empty()",
        "render_token_slice(option.tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route vote-result predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_vote_result_predicate(\n    words: &[&str]",
        "fn parse_vote_option_result_predicate(words: &[&str]",
        "fn parse_no_vote_objects_matched_predicate(\n    words: &[&str]",
        "parse_vote_result_predicate(&filtered, true)",
        "parse_vote_result_predicate(&filtered, false)",
        "option.word_refs().is_empty()",
        "option.word_refs().join(\" \")",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild vote-result predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild vote-result predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_x_value_comparison_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_x_value_comparison_predicate",
        "fn parse_paid_cost_label_predicate",
    );

    for required in [
        "fn parse_x_value_comparison_predicate(tokens: &[OwnedLexToken])",
        "let words = clause.word_refs()",
        "parse_x_value_comparison_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route X-value comparison predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_x_value_comparison_predicate(words: &[&str])",
        "parse_x_value_comparison_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild X-value comparison predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild X-value comparison predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_ring_bearer_temptation_uses_capture_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_source_is_your_ring_bearer_predicate",
        "fn parse_stack_object_targets_only_source_predicate",
    );

    for required in [
        "fn parse_ring_has_tempted_you_this_game_predicate(\n    tokens: &[OwnedLexToken]",
        "LexPattern::action(\"tempted\", LexCaptureKind::WordCount(3))",
        "LexPattern::amount(\"count\", LexCaptureKind::UntilPhrase(&[\"or\", \"more\"]))",
        "used != count_clause.tokens().len()",
        "fn parse_ring_bearer_temptation_predicate(tokens: &[OwnedLexToken])",
        "left_clause.tokens().is_empty() || right_clause.tokens().is_empty()",
        "parse_ring_bearer_temptation_predicate(tokens)",
    ] {
        assert!(
            content.contains(required) || parser.contains(required),
            "{relative} should route ring-bearer temptation predicates through captured token clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_ring_has_tempted_you_this_game_predicate(words: &[&str])",
        "fn parse_ring_bearer_temptation_predicate(words: &[&str])",
        "used != count_clause.word_refs().len()",
        "left_clause.word_refs().is_empty() || right_clause.word_refs().is_empty()",
        "parse_ring_bearer_temptation_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden) && !parser.contains(forbidden),
            "{relative} should not parse ring-bearer temptation predicates from raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_count_parity_uses_capture_pattern_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_count_parity_predicate",
        "fn parse_player_cards_in_hand_relation_predicate",
    );

    for required in [
        "fn parse_count_parity_predicate(tokens: &[OwnedLexToken])",
        "LexPattern::subject(\"count\", LexCaptureKind::WordCount(2))",
        "LexPattern::object(\"scope\", LexCaptureKind::UntilPhrase(&[\"is\"]))",
        "parity.token(0)?.parser_text()",
        "parse_count_parity_predicate(predicate_tokens)",
        "render_token_slice(scope.tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route count-parity predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_count_parity_predicate(words: &[&str])",
        "parse_count_parity_predicate(&filtered)",
        "words.split_at",
        "scope.join(\" \")",
        "parity.word_refs().first().copied()",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse count parity by splitting filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_graveyard_threshold_uses_capture_tokens() {
    let root = workspace_root();
    let helper_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/meld_and_special_subjects.rs";
    let parser_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let helper_content = read_repo_file(&root, helper_relative);
    let parser_content = read_repo_file(&root, parser_relative);
    let helper = function_source(
        &helper_content,
        "pub(super) fn parse_graveyard_threshold_predicate",
        "pub(super) fn parse_mana_spent_to_cast_predicate",
    );

    for required in [
        "pub(super) fn parse_graveyard_threshold_predicate(\n    tokens: &[OwnedLexToken]",
        "LexPattern::subject(\"prefix\", LexCaptureKind::WordCount(2))",
        "LexPattern::object(\"body\", LexCaptureKind::UntilLastPhrase(&[\"in\"]))",
        "LexPattern::modifier(\"graveyard_owner\", LexCaptureKind::Rest)",
        "enum GraveyardThresholdOwner",
        "const GRAVEYARD_THRESHOLD_OWNER_PATTERN: LexPattern<'static>",
        "LexCaptureKind::OneOfPhrase(&[",
        "fn parse_graveyard_threshold_owner_shape(",
        "fn graveyard_threshold_owner_player(owner: LexedClause<'_>) -> Option<PlayerAst>",
        "parse_graveyard_threshold_owner_shape(owner)?",
        "graveyard_threshold_owner_player(owner)",
        "parse_quantity_comparison_prefix(tokens, false, false, \"graveyard threshold\")",
        "parse_object_filter(&normalized_filter_tokens, false)",
    ] {
        assert!(
            helper_content.contains(required),
            "{helper_relative} should parse graveyard threshold predicates from captured token clauses: missing `{required}`"
        );
    }
    assert!(
        parser_content.contains("parse_graveyard_threshold_predicate(predicate_tokens)"),
        "{parser_relative} should route graveyard threshold predicates through captured predicate tokens"
    );
    for forbidden in [
        "pub(super) fn parse_graveyard_threshold_predicate(\n    filtered: &[&str]",
        "parse_graveyard_threshold_predicate(&filtered)",
        "crate::runtime_backend::lexer::synthetic_word_tokens(normalized_filter_words)",
        "let tail = &filtered[tail_start..]",
        "rfind_index(tail",
        "let owner_words = owner.word_refs()",
        "match owner_words.as_slice()",
        "YOUR_GRAVEYARD_OWNER_PATTERN",
        "THAT_PLAYER_GRAVEYARD_OWNER_PATTERN",
        "TARGET_PLAYER_GRAVEYARD_OWNER_PATTERN",
        "TARGET_OPPONENT_GRAVEYARD_OWNER_PATTERN",
        "OPPONENT_GRAVEYARD_OWNER_PATTERN",
    ] {
        assert!(
            !helper_content.contains(forbidden) && !parser_content.contains(forbidden),
            "graveyard threshold parsing should not split filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !helper.contains(forbidden),
            "{helper_relative} should not rebuild graveyard threshold parser tokens from raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_mana_spent_helpers_use_tokens() {
    let root = workspace_root();
    let helper_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/meld_and_special_subjects.rs";
    let predicate_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let etb_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/etb_static_lines.rs";
    let helper_content = read_repo_file(&root, helper_relative);
    let predicate_content = read_repo_file(&root, predicate_relative);
    let etb_content = read_repo_file(&root, etb_relative);
    let helper = function_source(
        &helper_content,
        "pub(super) fn parse_mana_spent_to_cast_predicate",
        "pub(super) fn parse_mana_symbol_word",
    );
    let predicate = function_source(
        &predicate_content,
        "fn parse_mana_spent_capture_predicate",
        "fn parse_mana_symbol_spent_to_cast_shape",
    );

    for required in [
        "pub(super) fn parse_mana_spent_to_cast_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_mana_symbol_word(token.parser_text())",
        "pub(crate) fn parse_same_color_mana_spent_to_cast_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_mana_spent_capture_predicate(tokens: &[OwnedLexToken])",
        "let symbol_words = symbol_clause.word_refs()",
        "word_is_any(word, MANA_SYMBOL_WORDS)",
        "parse_mana_symbol(token.parser_text()).ok()",
        "parse_mana_spent_capture_predicate(predicate_tokens)",
        "parse_same_color_mana_spent_to_cast_predicate(tokens)",
        "parse_mana_spent_to_cast_predicate(tokens)",
        "parse_same_color_mana_spent_to_cast_predicate(\n            &condition_tokens",
    ] {
        assert!(
            helper_content.contains(required)
                || predicate_content.contains(required)
                || etb_content.contains(required),
            "mana-spent parsing should use token slices end-to-end: missing `{required}`"
        );
    }
    for forbidden in [
        "parse_mana_spent_capture_predicate(&filtered)",
        "fn parse_mana_spent_capture_predicate(words: &[&str])",
        "parse_same_color_mana_spent_to_cast_predicate(\n            &condition_words",
        "LexedClause::new(token).word_refs()",
        "let symbol_words = tokens",
        "let symbol_words = symbol_clause.word_refs()",
        "MANA_SYMBOL_WORD_PATTERN.matches_word(word)",
        "parse_mana_symbol(word).ok()",
    ] {
        assert!(
            !predicate_content.contains(forbidden) && !etb_content.contains(forbidden),
            "mana-spent parsing should not route through filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !helper.contains(forbidden) && !predicate.contains(forbidden),
            "{helper_relative} and {predicate_relative} should not rebuild mana-spent parser tokens from raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_counted_object_shapes_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_counted_objects_have_counter_predicate",
        "fn parse_happily_style_conjoined_predicate",
    );

    for required in [
        "fn parse_counted_objects_have_counter_predicate(tokens: &[OwnedLexToken])",
        "fn parse_counted_source_exiled_objects_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_counted_objects_have_counter_predicate(predicate_tokens)",
        "parse_counted_source_exiled_objects_predicate(predicate_tokens)",
        "clause_starts_with_any_phrase(tail, BEEN_EXILED_WITH_THIS_SOURCE_PREFIXES)",
        "let object_tokens = &counted_object.tokens()[used..]",
        "parse_counted_object_counter_constraint_clause(counter)",
        "fn parse_counted_object_counter_constraint_clause(\n    clause: LexedClause<'_>",
        "let words = TokenWordView::new(clause.tokens())",
        "words.token_index_after_words(consumed_words)",
        "token_word_is_any(token, OTHER_OR_ANOTHER_WORDS)",
        "token_word_is_any(token, CARD_OR_CARDS_WORDS)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route counted-object predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_counted_objects_have_counter_predicate(words: &[&str])",
        "fn parse_counted_source_exiled_objects_predicate(words: &[&str])",
        "parse_counted_objects_have_counter_predicate(&filtered)",
        "parse_counted_source_exiled_objects_predicate(&filtered)",
        "BEEN_EXILED_WITH_THIS_SOURCE_PREFIX_PATTERN.matches(tail)",
        "BEEN_EXILED_WITH_THIS_SOURCE_PREFIX_PATTERN.matches_words(&tail_words)",
        "let counted_words = counted_object.word_refs()",
        "let object_words = &counted_words[used..]",
        "let counter_words = counter.word_refs()",
        "parse_filter_counter_constraint_words(&counter_words)",
        "if consumed != counter_words.len()",
        "CARD_OR_CARDS_WORD_PATTERN.matches_word(word)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild counted-object predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild counted-object predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_diagnostic_instead_tail_uses_clause_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn predicate_diagnostic_tokens",
        "fn render_unsupported_predicate_message",
    );

    for required in [
        "let maybe_clause = LexedClause::new(maybe_predicate)",
        "let maybe_word_len = maybe_clause.word_len()",
        ".between_word_range(maybe_word_len - 3, maybe_word_len)",
        "clause_matches_any_phrase(tail, COST_PAID_INSTEAD_TAIL_PHRASES)",
        ".between_word_range(maybe_word_len - 4, maybe_word_len)",
        "clause_matches_phrase(tail, COST_NOT_PAID_INSTEAD_TAIL_PHRASE)",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should trim diagnostic instead tails with captured clause ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "let maybe_words = LexedClause::new(maybe_predicate).word_refs()",
        "COST_PAID_INSTEAD_TAIL_PATTERN.matches(tail)",
        "COST_NOT_PAID_INSTEAD_TAIL_PATTERN.matches(tail)",
        "COST_PAID_INSTEAD_TAIL_PATTERN.matches_words(&maybe_words[maybe_words.len() - 3..])",
        "COST_NOT_PAID_INSTEAD_TAIL_PATTERN\n                .matches_words(&maybe_words[maybe_words.len() - 4..])",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not trim diagnostic instead tails through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_card_types_in_graveyard_subject_uses_captured_clause() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn graveyard_card_types_subject",
        "fn card_types_graveyard_lead_player_clause",
    );

    for required in [
        "fn graveyard_card_types_subject(clause: LexedClause<'_>) -> Option<PlayerAst>",
        "clause_matches_phrase(clause, YOUR_GRAVEYARD_PHRASE)",
        "clause_matches_any_phrase(clause, TARGET_PLAYER_GRAVEYARD_PHRASES)",
        "clause_matches_any_phrase(clause, OPPONENT_GRAVEYARD_PHRASES)",
        "graveyard_card_types_subject(graveyard)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should lower card-types-in-graveyard subjects from captured clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "fn graveyard_card_types_subject(words: &[&str])",
        "YOUR_GRAVEYARD_PATTERN.matches_words(words)",
        "TARGET_PLAYER_GRAVEYARD_PATTERN.matches_words(words)",
        "OPPONENT_GRAVEYARD_PATTERN.matches_words(words)",
        "graveyard_card_types_subject(&graveyard.word_refs())",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not lower card-types-in-graveyard subjects from raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_comparison_player_subject_uses_captured_clause() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn graveyard_possessive_matches_subject",
        "fn parse_keyword_subject_object_filter_tokens",
    );

    for required in [
        "fn graveyard_possessive_matches_subject(player: PlayerAst, possessive: LexedClause<'_>) -> bool",
        "possessive.token(0)",
        "token_word_is(token, YOUR_WORD)",
        "token_word_is(token, THEIR_WORD)",
        "fn comparison_player_subject_clause(clause: LexedClause<'_>) -> Option<PlayerAst>",
        "let word_len = clause.word_len()",
        "clause_matches_phrase(clause, THAT_PLAYER_SUBJECT_PREFIX)",
        "clause_matches_any_phrase(clause, AN_OR_THE_OPPONENT_SUBJECT_PHRASES)",
        "token_word_is(token, YOU_WORD)",
        "token_word_is(token, PLAYER_SUBJECT_WORD)",
        "graveyard_possessive_matches_subject(player, possessive)",
        "let player = comparison_player_subject_clause(subject)?",
        "fn object_starts_with_more_than_clause(clause: LexedClause<'_>) -> bool",
        "token_word_is(first, MORE_WORD)",
        "token_word_is(token, THAN_WORD)",
        "object_starts_with_more_than_clause(object)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should lower comparison graveyard predicates from captured clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "fn graveyard_possessive_matches_subject(player: PlayerAst, possessive: &str) -> bool",
        "YOUR_WORD_PATTERN.matches_word(possessive)",
        "THEIR_WORD_PATTERN.matches_word(possessive)",
        "let possessive_word = possessive.word_refs().first().copied()",
        "graveyard_possessive_matches_subject(player, possessive_word)",
        "fn comparison_player_subject(words: &[&str]) -> Option<(PlayerAst, usize)>",
        "THAT_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words)",
        "TARGET_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words)",
        "OPPONENT_SUBJECT_PREFIX_PATTERN.matches_words(words)",
        "let (player, consumed) = comparison_player_subject(&subject.word_refs())",
        "if consumed != subject.word_refs().len()",
        "let object_words = object.word_refs()",
        "object_words.first().is_some_and(|word| *word == \"more\")",
        "word_slice_contains_word(&object_words[1..], \"than\")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not lower comparison graveyard predicates from raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_card_in_your_graveyard_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_card_in_your_graveyard_predicate",
        "fn parse_object_on_battlefield_predicate",
    );
    let subtype_helper = function_source(
        &content,
        "fn parse_subtype_card_descriptor_clause",
        "fn parse_card_in_your_graveyard_predicate",
    );
    let named_helper = function_source(
        &content,
        "fn parse_named_object_filter_name_tail",
        "fn graveyard_card_types_subject",
    );

    for required in [
        "fn parse_card_in_your_graveyard_predicate(tokens: &[OwnedLexToken])",
        "let clause = LexedClause::new(tokens)",
        "parse_card_in_your_graveyard_predicate(predicate_tokens)",
        "descriptor.tokens().is_empty()",
        "object.tokens().is_empty()",
        "parse_object_filter(trimmed_tokens, false)",
        "parse_subtype_card_descriptor_clause(descriptor)",
        "let descriptor_tokens = strip_leading_article_tokens(clause.trimmed().tokens())",
        "token_word_is_any(&descriptor_tokens[1], CARD_OR_CARDS_WORDS)",
    ] {
        assert!(
            content.contains(required)
                || named_helper.contains(required)
                || subtype_helper.contains(required),
            "{relative} should route graveyard-card predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_card_in_your_graveyard_predicate(words: &[&str])",
        "parse_card_in_your_graveyard_predicate(&filtered)",
        "let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)",
        "let trimmed_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(trimmed)",
        "descriptor.word_refs().is_empty()",
        "object.word_refs().is_empty()",
        "let descriptor_words = descriptor.word_refs()",
        "descriptor_words.strip_prefix(&[\"an\"])",
        "descriptor_words.strip_prefix(&[\"a\"])",
    ] {
        assert!(
            !parser.contains(forbidden)
                && !named_helper.contains(forbidden)
                && !subtype_helper.contains(forbidden),
            "{relative} should not rebuild graveyard-card predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_half_starting_life_threshold_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_half_starting_life_total_threshold_predicate",
        "fn parse_life_total_subject_clause",
    );

    for required in [
        "fn parse_half_starting_life_total_threshold_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_half_starting_life_total_threshold_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route half-starting-life predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_half_starting_life_total_threshold_predicate(words: &[&str])",
        "parse_half_starting_life_total_threshold_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild half-starting-life predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild half-starting-life predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_life_total_static_thresholds_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_life_total_at_least_starting_predicate",
        "fn parse_counted_objects_have_counter_predicate",
    );

    for required in [
        "const LIFE_TOTAL_AT_LEAST_LAST_NOTED_PHRASES: &[&[&str]]",
        "fn parse_life_total_at_least_starting_predicate(tokens: &[OwnedLexToken])",
        "fn parse_life_total_at_least_last_noted_predicate(\n    tokens: &[OwnedLexToken]",
        "non_article_token_words_eq_phrase(tokens, LIFE_TOTAL_AT_LEAST_STARTING_PHRASE)",
        "non_article_token_words_eq_any(tokens, LIFE_TOTAL_AT_LEAST_LAST_NOTED_PHRASES)",
        "parse_life_total_at_least_starting_predicate(&cleaned_tokens[life_idx + 1..])",
        "parse_life_total_at_least_starting_predicate(predicate_tokens)",
        "parse_life_total_at_least_last_noted_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route life-total static thresholds through token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_life_total_at_least_starting_predicate(words: &[&str])",
        "fn parse_life_total_at_least_last_noted_predicate(words: &[&str])",
        "LIFE_TOTAL_AT_LEAST_STARTING_PATTERN.matches_non_article_tokens(tokens)",
        "LIFE_TOTAL_AT_LEAST_LAST_NOTED_PATTERN.matches_non_article_tokens(tokens)",
        "parse_life_total_at_least_starting_predicate(&words[life_idx + 1..])",
        "parse_life_total_at_least_starting_predicate(&filtered)",
        "parse_life_total_at_least_last_noted_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route life-total static thresholds through filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["matches!(\n        words"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not keep life-total static thresholds as raw word slice matches: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_you_life_total_at_most_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_you_life_total_at_most_predicate",
        "fn life_total_at_most_from_amount_tokens",
    );

    for required in [
        "fn parse_you_life_total_at_most_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_you_life_total_at_most_predicate(predicate_tokens)",
        "let clause = LexedClause::new(tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route life-total-at-most predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_you_life_total_at_most_predicate(\n    words: &[&str]",
        "parse_you_life_total_at_most_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild life-total-at-most predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild life-total-at-most predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_player_object_keyword_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_graveyard_escape_keyword_predicate",
        "fn parse_keyword_subject_object_in_zone_filter",
    );
    let helper = function_source(
        &content,
        "fn parse_keyword_subject_object_filter_tokens",
        "fn parse_graveyard_escape_keyword_predicate",
    );

    for required in [
        "fn parse_keyword_subject_object_filter_tokens(\n    object_tokens: &[OwnedLexToken]",
        "let object_tokens = strip_leading_article_tokens(object_tokens)",
        "non_article_token_words_eq_any(object_tokens, NONLAND_CARD_OBJECT_PHRASES)",
        "*last = OwnedLexToken::synthetic_word(\"card\")",
        "parse_keyword_subject_object_filter_tokens(object.tokens())",
        "fn parse_graveyard_escape_keyword_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_player_object_keyword_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_filter_keyword_constraint_tokens(keyword.tokens())",
        "consumed != keyword.tokens().len()",
        "token_word_is(token, CONTROL_WORD)",
        "token_word_is_any(token, ZONE_WORDS)",
        "parse_graveyard_escape_keyword_predicate(tokens)",
        "parse_player_object_keyword_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required) || helper.contains(required),
            "{relative} should route player-object keyword predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_graveyard_escape_keyword_predicate(\n    words: &[&str]",
        "fn parse_player_object_keyword_predicate(\n    words: &[&str]",
        "fn parse_keyword_subject_object_filter_words",
        "parse_player_object_keyword_predicate(&filtered)",
        "parse_keyword_subject_object_filter_words(object_words.as_slice())",
        "parse_keyword_subject_object_filter_words(&object_words)",
        "let keyword_words = keyword.word_refs()",
        "parse_filter_keyword_constraint_words(&keyword_words)",
        "consumed != keyword_words.len()",
        "let subject_words = subject.word_refs()",
        "CONTROL_WORD_PATTERN.matches_word(word)",
        "ZONE_WORD_PATTERN.matches_word(word)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route player-object keyword predicates through filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden) && !helper.contains(forbidden),
            "{relative} should not rebuild player-object keyword predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_source_state_identity_keyword_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_source_identity_predicate",
        "fn parse_source_crewed_by_exactly_predicate",
    );

    for required in [
        "fn parse_source_identity_predicate(tokens: &[OwnedLexToken])",
        "parse_identity_descriptor_filter_tokens(descriptor_clause.tokens())",
        "fn parse_identity_descriptor_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter>",
        "parse_card_type(token.parser_text())",
        "parse_subtype_flexible(token.parser_text())",
        "fn parse_source_keyword_predicate(tokens: &[OwnedLexToken])",
        "fn parse_filter_keyword_constraint_tokens(\n    tokens: &[OwnedLexToken],\n) -> Option<(FilterKeywordConstraint, usize)>",
        "let consumed_tokens = words.token_index_after_words(consumed_words)?",
        "parse_filter_keyword_constraint_tokens(keyword.tokens())",
        "consumed != keyword.tokens().len()",
        "fn parse_source_simple_state_predicate(tokens: &[OwnedLexToken])",
        "fn parse_source_power_threshold_predicate(tokens: &[OwnedLexToken])",
        "parse_source_simple_state_predicate(predicate_tokens)",
        "parse_source_identity_predicate(predicate_tokens)",
        "parse_source_keyword_predicate(predicate_tokens)",
        "parse_source_power_threshold_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route source state/identity/keyword predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_source_identity_predicate(words: &[&str])",
        "fn parse_source_keyword_predicate(words: &[&str])",
        "fn parse_source_simple_state_predicate(words: &[&str])",
        "fn parse_source_power_threshold_predicate(words: &[&str])",
        "parse_source_simple_state_predicate(&filtered)",
        "parse_source_identity_predicate(&filtered)",
        "parse_source_keyword_predicate(&filtered)",
        "parse_source_power_threshold_predicate(&filtered)",
        "let descriptor_words = descriptor_clause.word_refs()",
        "parse_identity_descriptor_filter_words(&descriptor_words)",
        "let keyword_words = keyword.word_refs()",
        "parse_filter_keyword_constraint_words(&keyword_words)",
        "consumed != keyword_words.len()",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild source state/identity/keyword predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild source state/identity/keyword predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_source_crewed_by_exactly_uses_capture_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_source_crewed_by_exactly_predicate",
        "fn parse_source_bare_state_shape",
    );

    for required in [
        "fn parse_source_crewed_by_exactly_predicate(\n    tokens: &[OwnedLexToken]",
        "LexPattern::subject(\"source\", LexCaptureKind::UntilPhrase(action_phrase))",
        "LexPattern::amount(\"count\", LexCaptureKind::WordCount(1))",
        "LexPattern::object(\"filter\", LexCaptureKind::Rest)",
        "parse_source_crewed_by_exactly_predicate(predicate_tokens)",
        "render_token_slice(tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route crewed-by-exactly predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_source_crewed_by_exactly_predicate(\n    words: &[&str]",
        "parse_source_crewed_by_exactly_predicate(&filtered)",
        "find_index(words",
        "word_slice_starts_with(tail",
        "let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filter_words)",
        "words.join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse crewed-by-exactly predicates by scanning filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_source_attachment_count_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_source_attachment_count_predicate",
        "fn parse_attachment_count_filter_tokens",
    );

    for required in [
        "fn parse_source_attachment_count_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_source_attachment_count_predicate(predicate_tokens)",
        "render_token_slice(tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route source attachment-count predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_source_attachment_count_predicate(\n    words: &[&str]",
        "parse_source_attachment_count_predicate(&filtered)",
        "words.join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild source attachment-count predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild source attachment-count predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_source_verbless_counted_counter_uses_capture_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_source_verbless_counted_counter_predicate",
        "fn parse_there_are_no_counters_on_source_predicate",
    );

    for required in [
        "fn parse_source_verbless_counted_counter_predicate(tokens: &[OwnedLexToken])",
        "LexPattern::object(\"counter\", LexCaptureKind::UntilPhrase(&[\"on\"]))",
        "LexPattern::modifier(\"target\", LexCaptureKind::Rest)",
        "predicate_number_or_more_prefix_tokens(counter_clause.tokens())",
        "parse_source_verbless_counted_counter_predicate(predicate_tokens)",
        "parse_source_verbless_counted_counter_predicate(&predicate_tokens_after_if(",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route source verbless counted-counter predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_source_verbless_counted_counter_predicate(words: &[&str])",
        "parse_source_verbless_counted_counter_predicate(&filtered)",
        "fn is_counter_on_source_pronoun_tail(words: &[&str])",
        "crate::runtime_backend::lexer::synthetic_word_tokens(counter_and_tail",
        "OR_MORE_PREFIX_PATTERN.matches_words(tail)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not parse source verbless counted counters by splitting filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in
        ["let counter_tokens =\n            crate::runtime_backend::lexer::synthetic_word_tokens"]
    {
        assert!(
            !parser.contains(forbidden),
            "{relative} should parse source verbless counted-counter tails from captured token ranges: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_counter_quantity_or_more_uses_token_offsets() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let helper_region = function_source(
        &content,
        "fn predicate_number_or_more_prefix_tokens",
        "fn control_predicate_quantity_tokens",
    );
    let terminal_counter = function_source(
        &content,
        "fn parse_terminal_counter_phrase_shape",
        "fn parse_source_has_counter_predicate",
    );
    let source_has_counter = function_source(
        &content,
        "fn parse_source_has_counter_predicate",
        "fn parse_basic_land_types_among_lands_predicate",
    );

    for required in [
        "fn predicate_number_or_more_prefix_tokens(tokens: &[OwnedLexToken]) -> Option<(u32, usize)>",
        "let clause = LexedClause::new(tokens)",
        "let tail_token_idx = words.token_index_after_words(1)?",
        "clause_matches_phrase(LexedClause::new(tail), OR_MORE_PREFIX)",
        "tail_token_idx + used",
        "predicate_number_or_more_prefix_tokens(tokens)",
        "count_and_type.tokens().is_empty()",
        "token_word_is(token, NO_WORD)",
        "is_triggering_object_counter_subject_clause(subject_clause)",
        "is_exact_counter_on_triggering_object_tail_clause(target_clause)",
        "fn is_triggering_object_counter_subject_clause(clause: LexedClause<'_>) -> bool",
        "fn is_exact_counter_on_triggering_object_tail_clause(clause: LexedClause<'_>) -> bool",
    ] {
        assert!(
            helper_region.contains(required)
                || terminal_counter.contains(required)
                || source_has_counter.contains(required),
            "{relative} should parse predicate `or more` quantities with token offsets: missing `{required}`"
        );
    }
    assert!(
        source_has_counter
            .contains("predicate_number_or_more_prefix_tokens(counter_clause.tokens()).is_some()"),
        "{relative} should use the shared token-offset `or more` helper in source counter predicates"
    );
    for forbidden in [
        "let words = LexedClause::new(tokens).word_refs()",
        ".get(used..used + 2)",
        "OR_MORE_PREFIX_PATTERN.matches_words(tail)",
        "OR_MORE_PREFIX_PATTERN.matches_words(counter_words.get(1..).unwrap_or_default())",
        ".get(1..3)\n            .is_some_and(|tail| OR_MORE_PREFIX_PATTERN.matches_words(tail))",
        "count_and_type.word_refs().is_empty()",
        "let counter_words = counter_clause.word_refs()",
        "matches!(counter_words.as_slice(), [\"no\", ..])",
        "is_triggering_object_counter_subject(&subject_clause.word_refs())",
        "is_exact_counter_on_triggering_object_tail(&target_clause.word_refs())",
        "fn is_triggering_object_counter_subject(words: &[&str])",
        "fn is_exact_counter_on_triggering_object_tail(words: &[&str])",
        "let count_words = count_clause.word_refs()",
    ] {
        assert!(
            !helper_region.contains(forbidden)
                && !terminal_counter.contains(forbidden)
                && !source_has_counter.contains(forbidden),
            "{relative} should not mix word slices with token offsets for predicate `or more` quantities: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_basic_land_and_combat_shapes_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_basic_land_types_among_lands_predicate",
        "fn parse_you_attacked_this_turn_shape",
    );

    for required in [
        "fn parse_basic_land_types_among_lands_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_combat_turn_predicate(tokens: &[OwnedLexToken])",
        "parse_basic_land_types_among_lands_predicate(predicate_tokens)",
        "parse_combat_turn_predicate(predicate_tokens)",
        "render_token_slice(tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route basic-land/combat predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_basic_land_types_among_lands_predicate(\n    words: &[&str]",
        "fn parse_combat_turn_predicate(words: &[&str])",
        "parse_basic_land_types_among_lands_predicate(&filtered)",
        "parse_combat_turn_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild basic-land/combat predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild basic-land/combat predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_simple_capture_wrappers_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_turn_timing_predicate",
        "fn parse_vote_result_predicate",
    );

    for required in [
        "fn parse_turn_timing_predicate(tokens: &[OwnedLexToken])",
        "fn parse_opponent_controls_tagged_object_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_secret_choices_match_predicate(tokens: &[OwnedLexToken])",
        "parse_turn_timing_predicate(predicate_tokens)",
        "parse_opponent_controls_tagged_object_predicate(predicate_tokens)",
        "parse_secret_choices_match_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route simple capture-wrapper predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_turn_timing_predicate(words: &[&str])",
        "fn parse_opponent_controls_tagged_object_predicate(words: &[&str])",
        "fn parse_secret_choices_match_predicate(words: &[&str])",
        "parse_turn_timing_predicate(&filtered)",
        "parse_opponent_controls_tagged_object_predicate(&filtered)",
        "parse_secret_choices_match_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild simple capture-wrapper predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild simple capture-wrapper predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_demonstrative_descriptor_tail_uses_token_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn parse_single_card_type_card_descriptor_tokens",
        "fn parse_or_predicate",
    );

    for required in [
        "fn parse_single_card_type_card_descriptor_tokens(tokens: &[OwnedLexToken])",
        "fn demonstrative_descriptor_filter_tokens(\n    tokens: &[OwnedLexToken]",
        "fn demonstrative_reference_prefix(clause: LexedClause<'_>) -> Option<DemonstrativeReferencePrefix>",
        "LexPattern::any_phrase(&[&[\"it\"], &[\"its\"], &[\"it\", \"s\"]])",
        "LexPattern::object(\"reference\", LexCaptureKind::WordCount(1))",
        "matched.capture_clause_by_role(LexCaptureRole::Object, clause)",
        "token_word_is_any(reference_token, PREDICATE_REFERENCE_NOUN_WORDS)",
        "token_word_is(reference_token, ENCHANTMENT_WORD)",
        "reference_is_creature: token_word_is(reference_token, CREATURE_WORD)",
        "let reference = demonstrative_reference_prefix(clause)?",
        "fn clause_word_range_matches_phrase(",
        "fn clause_word_range_matches_any_phrase(",
        "fn clause_word_at_is_any(clause: LexedClause<'_>, word_idx: usize, expected: &[&str]) -> bool",
        "clause\n        .after_words(reference_end)",
        "token_word_is(token, CARD_WORD)",
        "clause_word_range_matches_any_phrase(clause, descriptor_start, DOESNT_HAVE_PHRASES)",
        "clause_word_range_matches_phrase(clause, descriptor_start, DOES_NOT_HAVE_PHRASE)",
        "clause_word_at_is_any(clause, descriptor_start, IS_OR_ARE_WORDS)",
        "clause_word_range_matches_phrase(clause, descriptor_start, NOT_TOKEN_PREFIX)",
        "fn demonstrative_reference_kind(tokens: &[OwnedLexToken])",
        "demonstrative_reference_prefix(LexedClause::new(tokens)).map(|reference| reference.kind)",
        "let demonstrative_reference = demonstrative_reference_kind(predicate_tokens)",
        "let is_it = demonstrative_reference == Some(DemonstrativeReferenceKind::It)",
        "let Some(reference) = demonstrative_reference_prefix(clause) else",
        "token_word_is_any(token, HAS_OR_HAVE_WORDS)",
        "let reference = demonstrative_reference_prefix(subject)?",
        "subject\n        .after_words(reference.word_len)",
        "token_word_is(token, CREATURE_WORD)",
        "fn parse_demonstrative_mana_value_predicate(\n    tokens: &[OwnedLexToken]",
        "clause_matches_any_phrase(comparison, COLORS_SPENT_TO_CAST_SOURCE_TAIL_PHRASES)",
        "fn parse_demonstrative_total_power_toughness_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_demonstrative_power_or_toughness_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_demonstrative_toxic_predicate(tokens: &[OwnedLexToken])",
        "fn parse_demonstrative_shares_predicate(tokens: &[OwnedLexToken])",
        "fn contains_most_common_color_among_all_permanents_clause(tokens: &[OwnedLexToken])",
        "MOST_COMMON_COLOR_AMONG_ALL_PERMANENTS_PATTERN\n        .find_in_clause(LexedClause::new(tokens))",
        "LexPattern::action(\"action\", LexCaptureKind::OneOf(&[\"has\", \"have\"]))",
        "LexPattern::object(\"keyword\", LexCaptureKind::Rest)",
        "parse_demonstrative_toxic_predicate(predicate_tokens)",
        "parse_demonstrative_mana_value_predicate(predicate_tokens)",
        "parse_demonstrative_total_power_toughness_predicate(predicate_tokens)",
        "parse_demonstrative_power_or_toughness_predicate(predicate_tokens)",
        "parse_demonstrative_shares_predicate(predicate_tokens)",
        "!contains_most_common_color_among_all_permanents_clause(predicate_tokens)",
        "let words = clause.words()",
        "words.token_range_for_word_range(descriptor_start, words.len())",
        "descriptor_tokens.insert(0, OwnedLexToken::synthetic_word(\"nontoken\"))",
        "demonstrative_descriptor_filter_tokens(predicate_tokens)",
        "parse_single_card_type_card_descriptor_tokens(&descriptor_tokens)",
        "tagged_that_enchantment",
    ] {
        assert!(
            content.contains(required) || helper.contains(required),
            "{relative} should derive demonstrative descriptor filters from captured token ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_single_card_type_card_descriptor(words: &[&str])",
        "let mut descriptor_words = filtered[reference_len..].to_vec()",
        "let demonstrative_reference_len = if IT_WORD_PATTERN.matches_word_at(&filtered, 0)",
        "ITS_WORD_PATTERN.matches_word_at(&filtered, 0)",
        "THAT_WORD_PATTERN.matches_word_at(&filtered, 0)",
        "PREDICATE_REFERENCE_NOUN_WORD_PATTERN.matches_word_at(&filtered, 1)",
        "let (reference_end, tagged_that_enchantment) = if word_refs",
        "IT_WORD_PATTERN.matches_word(word)",
        "word_refs.first().copied() == Some(\"its\")",
        "matches!(word_refs.get(0..2), Some([\"it\", \"s\"]))",
        "THAT_WORD_PATTERN.matches_word(word_refs[0])",
        "PREDICATE_REFERENCE_NOUN_WORD_PATTERN.matches_word(word_refs[1])",
        "word_refs[1] == \"enchantment\"",
        "let words = LexedClause::new(tokens).word_refs()",
        "let word_refs = words.word_refs()",
        "CARD_WORD_PATTERN.matches_word(word)",
        "matches!(head[0], \"doesnt\" | \"doesn't\") && HAVE_WORD_PATTERN.matches_word(head[1])",
        "head[0] == \"does\"",
        "NOT_WORD_PATTERN.matches_word(head[1])",
        "HAVE_WORD_PATTERN.matches_word(head[2])",
        "IS_OR_ARE_WORD_PATTERN.matches_word(word)",
        "NOT_TOKEN_PREFIX_PATTERN.matches_words(head)",
        "let subject_words = subject.word_refs()",
        "IT_WORD_PATTERN.matches_word(word) || THAT_WORD_PATTERN.matches_word(word) || *word == \"its\"",
        "CREATURE_WORD_PATTERN.matches_word(word)",
        "descriptor_words.drain(0..2)",
        "descriptor_words.remove(0)",
        "MANA_VALUE_HEAD_PATTERN",
        "COLORS_SPENT_TO_CAST_SOURCE_TAIL_PATTERN.matches_words(&comparison_words)",
        "TOTAL_POWER_TOUGHNESS_HEAD_PATTERN",
        "parse_filter_comparison_tokens(\"mana value\", mana_value_tail, &filtered)",
        "parse_filter_comparison_tokens(\"power\", &filtered[5..], &filtered)",
        "parse_filter_comparison_tokens(axis, value_tail, &filtered)",
        "HAS_OR_HAVE_TOXIC_PATTERN",
        "HAS_OR_HAVE_TOXIC_PATTERN.matches_words(&descriptor_words)",
        "CREATURE_WORD_PATTERN.matches_word_at(&filtered, 1)",
        "matches!(\n            descriptor_words.as_slice()",
        "MOST_COMMON_COLOR_AMONG_ALL_PERMANENTS_PATTERN\n            .find_exact_window_range(&filtered, 6, 6)",
        "crate::runtime_backend::lexer::synthetic_word_tokens(descriptor_words)",
        "THAT_ENCHANTMENT_PREFIX_PATTERN.matches_words(&filtered)",
        "THAT_ENCHANTMENT_PREFIX_PATTERN.matches_non_article_tokens(predicate_tokens)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild demonstrative descriptor filters from raw descriptor words: found `{forbidden}`"
        );
    }
}

#[test]
fn predicate_or_parser_uses_token_slices_for_split_and_prefix_fallback() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_or_predicate",
        "fn parse_attacking_you_own_control_predicate",
    );

    for required in [
        "fn parse_or_predicate(tokens: &[OwnedLexToken])",
        "token_word_is(token, OR_WORD)",
        "token_word_is_any(token, OR_COMPARISON_TAIL_WORDS)",
        "let left_tokens = &tokens[..or_idx]",
        "let right_tokens = &tokens[or_idx + 1..]",
        "parse_predicate(left_tokens)",
        "parse_predicate(right_tokens)",
        "predicate_reference_prefix_tokens(left_tokens)",
        "predicate_tokens_start_with_reference(right_tokens)",
        "token_word_is_any(token, PREDICATE_REFERENCE_START_WORDS)",
        "prefixed_tokens.extend_from_slice(right_tokens)",
        "parse_or_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required) || parser.contains(required),
            "{relative} should split OR predicates with token slices and preserve reference-prefix fallback: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_or_predicate(filtered: &[&str])",
        "parse_or_predicate(&filtered)",
        "predicate_reference_prefix(left_words)",
        "predicate_words_start_with_reference(right_words)",
        "let first_word = LexedClause::new(tokens).word_refs().first().copied()",
        "predicate_tokens_from_words(left_words)",
        "predicate_tokens_from_words(right_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild OR predicate branches from filtered raw words: found `{forbidden}`"
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
fn anthem_grant_lines_top_shape_helpers_use_direct_word_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn anthem_shape_matches_words",
        "fn anthem_token_offset",
    );
    let prefix_lookup = function_source(
        &content,
        "fn anthem_find_prefix_shape_start",
        "fn anthem_find_slash_word",
    );
    let first_spell = function_source(
        &content,
        "fn first_spell_each_turn_subject",
        "fn first_spell_each_turn_subject_tokens",
    );
    let subject_parser = function_source(
        &content,
        "pub(crate) fn parse_anthem_subject",
        "fn anthem_subject_filter",
    );
    let granted_keyword = function_source(
        &content,
        "pub(crate) fn parse_granted_keyword_static",
        "fn parse_color_filtered_keyword_grants",
    );
    let alternative_tails = function_source(
        &content,
        "fn is_granted_blitz_cost_tail",
        "fn normalize_granted_alternative_spell_filter",
    );
    let granted_emerge_subject = function_source(
        &content,
        "fn granted_emerge_abilities_from_subject",
        "pub(crate) fn find_source_reference_start",
    );
    let best_object_filter_suffix = function_source(
        &content,
        "pub(crate) fn parse_best_object_filter_suffix",
        "fn parse_enchanted_player_controls_subject",
    );
    let attached_subject_helpers = function_source(
        &content,
        "fn parse_enchanted_player_controls_subject",
        "pub(crate) fn parse_anthem_prefix_condition",
    );
    let continuing_segments = function_source(
        &content,
        "fn parse_continuing_anthem_granted_segment",
        "fn attached_object_anthem_subject_uses_tagged_constraints",
    );
    let static_condition = function_source(
        &content,
        "pub(crate) fn parse_static_condition_clause",
        "fn parse_devotion_static_condition",
    );
    let strip_static_condition_intro = function_source(
        &content,
        "fn strip_static_condition_intro",
        "pub(crate) fn parse_static_condition_clause",
    );
    let for_each_expr = function_source(
        &content,
        "pub(crate) fn parse_anthem_for_each_expression",
        "pub(crate) fn parse_anthem_prefix_condition",
    );
    let lose_abilities = function_source(
        &content,
        "pub(crate) fn parse_all_creatures_lose_flying_line",
        "fn is_granted_blitz_cost_tail",
    );
    let base_pt_setters = function_source(
        &content,
        "pub(crate) fn parse_has_base_power_toughness_static_line",
        "pub(crate) fn parse_filter_has_granted_ability_line",
    );
    let soulbond_shared = function_source(
        &content,
        "pub(crate) fn parse_soulbond_shared_line",
        "pub(crate) fn parse_anthem_and_type_color_addition_line",
    );
    let type_color_addition = function_source(
        &content,
        "pub(crate) fn parse_type_color_addition_clause",
        "pub(crate) fn is_type_scope_qualifier_word",
    );
    let equipment_equip = function_source(
        &content,
        "pub(crate) fn parse_equipment_you_control_have_equip_line",
        "fn parsed_exploit_ability",
    );
    let trailing_segment_split = function_source(
        &content,
        "fn split_anthem_trailing_segments_preserving_granted_abilities",
        "fn parsed_triggered_ability_is_empty",
    );
    let filter_has_granted = function_source(
        &content,
        "pub(crate) fn parse_filter_has_granted_ability_line",
        "fn attached_object_anthem_subject_uses_tagged_constraints",
    );
    let conditional_blocking = function_source(
        &content,
        "pub(crate) fn parse_conditional_all_creatures_able_to_block_line",
        "pub(crate) fn parse_source_can_attack_as_though_no_defender_as_long_as_line",
    );
    let no_defender = function_source(
        &content,
        "pub(crate) fn parse_source_can_attack_as_though_no_defender_as_long_as_line",
        "pub(crate) fn parse_anthem_line",
    );
    let permanent_anthem_guards = function_source(
        &content,
        "pub(crate) fn parse_anthem_line",
        "fn trim_multi_anthem_subject_segment",
    );
    let multi_subject_anthem = function_source(
        &content,
        "pub(crate) fn parse_multi_subject_anthem_line",
        "pub(crate) fn parse_has_base_power_toughness_static_line",
    );
    let has_base_pt_static = function_source(
        &content,
        "pub(crate) fn parse_has_base_power_toughness_static_line",
        "fn is_negated_creature_tail",
    );
    let isnt_creature_static = function_source(
        &content,
        "fn is_negated_creature_tail",
        "pub(crate) fn parse_has_base_power_toughness_and_granted_keywords_static_line",
    );
    let has_base_pt_and_keywords = function_source(
        &content,
        "pub(crate) fn parse_has_base_power_toughness_and_granted_keywords_static_line",
        "pub(crate) fn parse_filter_has_granted_ability_line",
    );

    for required in [
        "fn anthem_shape_matches_words",
        "fn anthem_shape_matches_word",
        "fn anthem_token_matches_shape",
        "fn anthem_shape_matches_last_word",
        "shape.matches_word_slice(words)",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose a direct word-matching anthem shape helper: missing `{required}`"
        );
    }
    for forbidden in [
        "synthetic_word_tokens(words)",
        "shape.matches(crate::runtime_backend::lexer::LexedClause::new(&tokens))",
        ".matches_word(",
        ".matches_token(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not call ClauseShape word/token adapter methods directly: found `{forbidden}`"
        );
    }
    assert!(
        prefix_lookup.contains("anthem_shape_matches_words(&words[idx..], *shape)"),
        "{relative} should route anthem prefix-shape lookup through token-backed matching"
    );
    for required in
        ["anthem_shape_matches_words(filter_words, FIRST_SPELL_EACH_TURN_SUBJECT_PATTERN)"]
    {
        assert!(
            first_spell.contains(required),
            "{relative} should route first-spell subject gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&subject_words, FIRST_SPELL_EACH_TURN_SUBJECT_PATTERN)",
        "anthem_shape_matches_words(&subject_words, SOURCE_IT_PATTERN)",
    ] {
        assert!(
            subject_parser.contains(required),
            "{relative} should route anthem subject gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&subject_words, ANTHEM_SUBJECT_ATTACHED_MARKER_PATTERN)",
        "anthem_shape_matches_words(&subject_words, ANTHEM_MANA_WORD_MARKER_PATTERN)",
        "anthem_shape_matches_words(&subject_words, ANTHEM_MANA_VALUE_MARKER_PATTERN)",
        "anthem_shape_matches_words(window, ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN)",
        "anthem_shape_matches_words(&keyword_words, ANTHEM_BLITZ_KEYWORD_PATTERN)",
        "anthem_shape_matches_words(&keyword_words, ANTHEM_EMERGE_KEYWORD_PATTERN)",
        "anthem_shape_matches_words(&keyword_words, ANTHEM_IGNORED_REMINDER_KEYWORD_PATTERN)",
        "ANTHEM_EXPLOIT_KEYWORD_PATTERN",
        "anthem_shape_matches_words(\n                    &trailing_word_refs,\n                    ANTHEM_FLASHBACK_COST_EQUALS_MANA_COST_PATTERN,\n                )",
    ] {
        assert!(
            granted_keyword.contains(required),
            "{relative} should route granted-keyword shape gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&trailing_word_refs, ANTHEM_BLITZ_COST_EQUALS_MANA_COST_PATTERN)",
        "anthem_shape_matches_words(&trailing_word_refs, ANTHEM_EMERGE_COST_EQUALS_MANA_COST_PATTERN)",
    ] {
        assert!(
            alternative_tails.contains(required),
            "{relative} should route granted alternative-cost tail gates through token-backed matching: missing `{required}`"
        );
    }
    assert!(
        granted_emerge_subject.contains(
            "anthem_shape_matches_words(&subject_words, ANTHEM_SPELL_CAST_SUBJECT_PATTERN)"
        ),
        "{relative} should route granted-emerge spell subject gates through token-backed matching"
    );
    assert!(
        best_object_filter_suffix
            .contains("anthem_shape_matches_words(&candidate_words, ANTHEM_IT_OR_THEM_PATTERN)"),
        "{relative} should route best-filter pronoun skips through token-backed matching"
    );
    for required in [
        "anthem_shape_matches_words(&words, ENCHANTED_PLAYER_CONTROLS_SUFFIX_PATTERN)",
        "anthem_shape_matches_words(&condition_words, ATTACHED_CONDITION_SUBJECT_PREFIX_PATTERN)",
        "anthem_shape_matches_words(&crate::runtime_backend::token_word_refs(tokens), SOURCE_IT_PATTERN)",
    ] {
        assert!(
            attached_subject_helpers.contains(required),
            "{relative} should route attached-subject helper gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(\n                &segment_words,\n                ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN,\n            )",
        "anthem_shape_matches_words(&attack_tail, ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN)",
        "anthem_shape_matches_words(&ability_words, ANTHEM_EMERGE_KEYWORD_PATTERN)",
    ] {
        assert!(
            continuing_segments.contains(required),
            "{relative} should route continuing granted segment gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&clause_words, OPPONENT_LOST_LIFE_THIS_TURN_CONDITION_PATTERN)",
        "YOU_NOT_CAST_SPELL_THIS_TURN_CONDITION_PATTERN",
        "anthem_shape_matches_words(&clause_words, YOU_CAST_SPELL_THIS_TURN_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, NO_CARDS_IN_YOUR_LIBRARY_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, SOURCE_IS_ON_BATTLEFIELD_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, SOURCE_DEVOURED_CREATURES_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, SOURCE_IS_SOULBOND_PAIRED_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, SOURCE_ATTACKED_THIS_TURN_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, YOU_ATTACKED_THIS_TURN_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, SOURCE_ENTERED_THIS_TURN_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, YOUR_TURN_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, SOURCE_POWER_EVEN_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, SOURCE_POWER_ODD_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, NOT_YOUR_TURN_CONDITION_PATTERN)",
        "anthem_shape_matches_words(&clause_words, YOUR_LIFE_HALF_STARTING_CONDITION_PATTERN)",
        "anthem_shape_matches_words(subject_words, ANTHEM_SOURCE_PRONOUN_SUBJECT_PATTERN)",
        "anthem_shape_matches_words(remainder_words, SOURCE_IN_GRAVEYARD_TAIL_PATTERN)",
        "anthem_shape_matches_words(&clause_words, THERE_IS_OR_ARE_PREFIX_PATTERN)",
        "anthem_shape_matches_words(zone_tail, SOURCE_IN_GRAVEYARD_TAIL_PATTERN)",
        "anthem_shape_matches_words(&clause_words, ANTHEM_ENTERED_WORD_MARKER_PATTERN)",
        "anthem_shape_matches_words(&clause_words, YOU_COMMITTED_CRIME_THIS_TURN_CONDITION_PATTERN)",
        "anthem_shape_matches_words(tail, ON_SOURCE_COUNTER_TAIL_PATTERN)",
        "anthem_shape_matches_words(\n            &crate::runtime_backend::token_word_refs(filter_tokens),\n            IN_YOUR_GRAVEYARD_TAIL_PATTERN,\n        )",
    ] {
        assert!(
            static_condition.contains(required),
            "{relative} should route static condition shape gates through token-backed matching: missing `{required}`"
        );
    }
    assert!(
        strip_static_condition_intro.contains(
            "anthem_shape_matches_words(&words, CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN)"
        ),
        "{relative} should route static-condition intro stripping through token-backed matching"
    );
    for required in [
        "anthem_shape_matches_words(&token_words, ANTHEM_FOR_EACH_PREFIX_PATTERN)",
        "anthem_shape_matches_words(&rest_words, ANTHEM_AFFECTED_ATTACKED_THIS_TURN_PATTERN)",
        "anthem_shape_matches_words(&rest_words, ANTHEM_AFFECTED_COLORS_PATTERN)",
        "Value::BasicLandTypesAmong(filter) => {",
        "Value::CreatureTypesAmong(filter) => {",
        "anthem_shape_matches_words(&tail_words, ANTHEM_ATTACHED_TO_SOURCE_TAIL_PATTERN)",
        "anthem_shape_matches_words(&rest_words, ANTHEM_UNSPENT_GREEN_MANA_YOU_HAVE_PATTERN)",
        "anthem_shape_matches_words(tail_words, ON_SOURCE_COUNTER_TAIL_PATTERN)",
        "ANTHEM_GRAVEYARD_CONJUNCTION_SPLIT_MARKER_PATTERN",
    ] {
        assert!(
            for_each_expr.contains(required),
            "{relative} should route anthem for-each expression gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&words, ALL_CREATURES_LOSE_FLYING_PATTERN)",
        "anthem_shape_matches_words(&after_words, CANT_GAIN_ABILITY_TAIL_PATTERN)",
        "anthem_shape_matches_words(&words, ANTHEM_LOSE_ALL_ABILITIES_PATTERN)",
        "anthem_shape_matches_words(&words, ANTHEM_EXCEPT_MANA_ABILITIES_PATTERN)",
        "anthem_shape_matches_words(&words[lose_idx + 1..], ANTHEM_ALL_ABILITIES_TAIL_PATTERN)",
        "anthem_shape_matches_words(&words, ANTHEM_UNTIL_WORD_PATTERN)",
        "anthem_shape_matches_words(&words, ANTHEM_BECOMES_WORD_PATTERN)",
        "anthem_shape_matches_words(after_have, ANTHEM_BASE_POWER_TOUGHNESS_PREFIX_PATTERN)",
        "anthem_shape_matches_words(&words[..have_idx], ANTHEM_GET_OR_GETS_CONTAINS_PATTERN)",
    ] {
        assert!(
            lose_abilities.contains(required),
            "{relative} should route lose/base-P/T shape gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(rest_words, ANTHEM_BASE_POWER_TOUGHNESS_PREFIX_PATTERN)",
        "anthem_shape_matches_words(&rest_words, ANTHEM_BASE_POWER_TOUGHNESS_PREFIX_PATTERN)",
    ] {
        assert!(
            base_pt_setters.contains(required),
            "{relative} should route base-P/T setter gates through token-backed matching: missing `{required}`"
        );
    }
    assert!(
        soulbond_shared.contains(
            "anthem_shape_matches_words(&clause_words, CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN)"
        ),
        "{relative} should route soulbond shared-line gates through token-backed matching"
    );
    for required in [
        "anthem_shape_matches_words(subject_words, SOULBOND_SOURCE_SUBJECT_PATTERN)",
        "anthem_shape_matches_words(&rest_words, SOULBOND_BOTH_CREATURES_GET_PREFIX_PATTERN)",
        "anthem_shape_matches_words(\n            &rest_words,\n            SOULBOND_EACH_OF_THOSE_CREATURES_GETS_PREFIX_PATTERN,\n        )",
        "anthem_shape_matches_words(&rest_words, SOULBOND_BOTH_CREATURES_HAVE_PREFIX_PATTERN)",
        "anthem_shape_matches_words(\n            &rest_words,\n            SOULBOND_EACH_OF_THOSE_CREATURES_HAS_PREFIX_PATTERN,\n        )",
    ] {
        assert!(
            soulbond_shared.contains(required),
            "{relative} should route soulbond shared-line detail gates through token-backed matching: missing `{required}`"
        );
    }
    assert!(
        type_color_addition
            .contains("anthem_shape_matches_words(segment, ANTHEM_COLOR_OR_COLORS_WORD_PATTERN)"),
        "{relative} should route type/color scope gates through token-backed matching"
    );
    for required in [
        "anthem_shape_matches_words(&[label.as_str()], METALCRAFT_LABEL_PATTERN)",
        "anthem_shape_matches_words(&words, EQUIPMENT_YOU_CONTROL_HAVE_EQUIP_PREFIX_PATTERN)",
    ] {
        assert!(
            equipment_equip.contains(required),
            "{relative} should route equipment-equip gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&segment_word_refs, ANTHEM_TRIGGERED_SEGMENT_START_PATTERN)",
        "anthem_shape_matches_words(\n                    &segment_word_refs,\n                    ANTHEM_AND_TRIGGERED_SEGMENT_START_PATTERN,\n                )",
    ] {
        assert!(
            trailing_segment_split.contains(required),
            "{relative} should route trailing triggered segment gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&all_words, CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN)",
        "anthem_shape_matches_words(&remainder_words, ALL_CREATURES_BLOCK_THIS_CREATURE_TAIL_PATTERN)",
        "anthem_shape_matches_words(\n        &remainder_words,\n        ALL_CREATURES_BLOCK_ENCHANTED_CREATURE_TAIL_PATTERN,\n    )",
    ] {
        assert!(
            conditional_blocking.contains(required),
            "{relative} should route conditional blocking gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&normalized, CAN_ATTACK_AS_NO_DEFENDER_AS_LONG_AS_PATTERN)",
        "anthem_shape_matches_words(&normalized, CAN_ATTACK_AS_NO_DEFENDER_PATTERN)",
        "anthem_shape_matches_words(&normalized, CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN)",
        "anthem_shape_matches_words(&tail_words, CANT_BE_BLOCKED_WORDS_PATTERN)",
        "anthem_shape_matches_words(&all_words, CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN)",
    ] {
        assert!(
            no_defender.contains(required),
            "{relative} should route no-defender and unblockable gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&ability_word_refs, ANTHEM_WARD_PAY_LIFE_PATTERN)",
        "anthem_shape_matches_words(&tail_words, ANTHEM_AND_HAVE_OR_HAS_TAIL_PATTERN)",
        "anthem_shape_matches_words(&tail_words, ANTHEM_HAVE_OR_HAS_TAIL_PATTERN)",
        "anthem_shape_matches_words(&segment_words, ANTHEM_CANT_ATTACK_ALONE_PATTERN)",
        "anthem_shape_matches_words(&segment_words, ANTHEM_CANT_BLOCK_PATTERN)",
    ] {
        assert!(
            continuing_segments.contains(required),
            "{relative} should route trailing anthem segment gates through token-backed matching: missing `{required}`"
        );
    }
    for required in ["anthem_shape_matches_words(&words, ANTHEM_TARGET_CONTAINS_PATTERN)"] {
        assert!(
            permanent_anthem_guards.contains(required),
            "{relative} should route permanent anthem guard gates through token-backed matching: missing `{required}`"
        );
    }
    assert!(
        multi_subject_anthem
            .contains("anthem_shape_matches_words(&words, ANTHEM_TARGET_CONTAINS_PATTERN)"),
        "{relative} should route multi-subject target guards through token-backed matching"
    );
    for required in [
        "anthem_shape_matches_words(&subject_words, ANTHEM_TARGET_CONTAINS_PATTERN)",
        "anthem_shape_matches_words(&subject_words, UNTIL_YOUR_NEXT_TURN_PREFIX_PATTERN)",
    ] {
        assert!(
            has_base_pt_static.contains(required),
            "{relative} should route base-P/T static subject guards through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(\n                &crate::runtime_backend::token_word_refs(&leading),\n                ANTHEM_BLITZ_KEYWORD_PATTERN,\n            )",
        "anthem_shape_matches_words(\n                &crate::runtime_backend::token_word_refs(&leading),\n                ANTHEM_EMERGE_KEYWORD_PATTERN,\n            )",
    ] {
        assert!(
            filter_has_granted.contains(required),
            "{relative} should route filter-granted sentence probes through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&words[1..], ANTHEM_NO_LONGER_PREFIX_PATTERN)",
        "anthem_shape_matches_words(&all_words, ANTHEM_TARGET_CONTAINS_PATTERN)",
        "anthem_shape_matches_words(&all_words, CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN)",
    ] {
        assert!(
            isnt_creature_static.contains(required),
            "{relative} should route isn't-creature gates through token-backed matching: missing `{required}`"
        );
    }
    for required in [
        "anthem_shape_matches_words(&subject_words, ANTHEM_TARGET_CONTAINS_PATTERN)",
        "anthem_shape_matches_words(&subject_words, UNTIL_YOUR_NEXT_TURN_PREFIX_PATTERN)",
        "anthem_shape_matches_words(&subject_words, THIS_CREATURE_PREFIX_PATTERN)",
    ] {
        assert!(
            has_base_pt_and_keywords.contains(required),
            "{relative} should route base-P/T plus keyword gates through token-backed matching: missing `{required}`"
        );
    }
    for forbidden in [
        "shape.matches_words(&words[idx..])",
        "FIRST_SPELL_EACH_TURN_SUBJECT_PATTERN.matches_words(filter_words)",
        "FIRST_SPELL_EACH_TURN_SUBJECT_PATTERN.matches_words(&subject_words)",
        "SOURCE_IT_PATTERN.matches_words(&subject_words)",
        "ANTHEM_SUBJECT_ATTACHED_MARKER_PATTERN.matches_words(&subject_words)",
        "ANTHEM_MANA_WORD_MARKER_PATTERN.matches_words(&subject_words)",
        "ANTHEM_MANA_VALUE_MARKER_PATTERN.matches_words(&subject_words)",
        "ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches_words(window)",
        "ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches_words(&segment_words)",
        "ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches_words(&attack_tail)",
        "ANTHEM_BLITZ_KEYWORD_PATTERN.matches_words(&keyword_words)",
        "ANTHEM_EMERGE_KEYWORD_PATTERN.matches_words(&keyword_words)",
        "ANTHEM_EMERGE_KEYWORD_PATTERN.matches_words(&ability_words)",
        "ANTHEM_IGNORED_REMINDER_KEYWORD_PATTERN.matches_words(&keyword_words)",
        "ANTHEM_EXPLOIT_KEYWORD_PATTERN\n        .matches_words(&crate::runtime_backend::token_word_refs(&keyword_tokens))",
        "ANTHEM_BLITZ_COST_EQUALS_MANA_COST_PATTERN.matches_words(&trailing_word_refs)",
        "ANTHEM_EMERGE_COST_EQUALS_MANA_COST_PATTERN.matches_words(&trailing_word_refs)",
        "ANTHEM_FLASHBACK_COST_EQUALS_MANA_COST_PATTERN\n                    .matches_words(&trailing_word_refs)",
        "ANTHEM_FLASHBACK_COST_EQUALS_MANA_COST_PATTERN.matches_words(&trailing_word_refs)",
        "EACH_CREATURE_SUBJECT_PREFIX_PATTERN.matches_words(&subject_words)",
        "ANTHEM_SPELL_CAST_SUBJECT_PATTERN.matches_words(&subject_words)",
        "ANTHEM_IT_OR_THEM_PATTERN.matches_words(&candidate_words)",
        "ENCHANTED_PLAYER_CONTROLS_SUFFIX_PATTERN.matches_words(&words)",
        "ATTACHED_CONDITION_SUBJECT_PREFIX_PATTERN\n        .matches_words(&condition_words)",
        "ATTACHED_CONDITION_SUBJECT_PREFIX_PATTERN.matches_words(&condition_words)",
        "SOURCE_IT_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens))",
        "PERMANENT_CARD_PREFIX_PATTERN.matches_words(&token_words)",
        "IN_YOUR_GRAVEYARD_TAIL_PATTERN\n            .matches_words(&crate::runtime_backend::token_word_refs(filter_tokens))",
        "IN_YOUR_GRAVEYARD_TAIL_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(filter_tokens))",
        "ANTHEM_COLOR_OR_COLORS_WORD_PATTERN.matches_words(segment)",
        "SOULBOND_SOURCE_SUBJECT_PATTERN.matches_words(subject_words)",
        "SOULBOND_BOTH_CREATURES_GET_PREFIX_PATTERN.matches_words(&rest_words)",
        "SOULBOND_EACH_OF_THOSE_CREATURES_GETS_PREFIX_PATTERN.matches_words(&rest_words)",
        "SOULBOND_BOTH_CREATURES_HAVE_PREFIX_PATTERN.matches_words(&rest_words)",
        "SOULBOND_EACH_OF_THOSE_CREATURES_HAS_PREFIX_PATTERN.matches_words(&rest_words)",
        "METALCRAFT_LABEL_PATTERN.matches_words(&[label.as_str()])",
        "EQUIPMENT_YOU_CONTROL_HAVE_EQUIP_PREFIX_PATTERN.matches_words(&words)",
        "ANTHEM_TRIGGERED_SEGMENT_START_PATTERN.matches_words(&segment_word_refs)",
        "ANTHEM_AND_TRIGGERED_SEGMENT_START_PATTERN.matches_words(&segment_word_refs)",
        "ANTHEM_WARD_PAY_LIFE_PATTERN.matches_words(&ability_word_refs)",
        "ANTHEM_AND_HAVE_OR_HAS_TAIL_PATTERN.matches_words(&tail_words)",
        "ANTHEM_HAVE_OR_HAS_TAIL_PATTERN.matches_words(&tail_words)",
        "ANTHEM_CANT_ATTACK_ALONE_PATTERN.matches_words(&segment_words)",
        "ANTHEM_CANT_BLOCK_PATTERN.matches_words(&segment_words)",
        "ANTHEM_BLITZ_KEYWORD_PATTERN\n                .matches_words(&crate::runtime_backend::token_word_refs(&leading))",
        "ANTHEM_BLITZ_KEYWORD_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(&leading))",
        "ANTHEM_EMERGE_KEYWORD_PATTERN\n                .matches_words(&crate::runtime_backend::token_word_refs(&leading))",
        "ANTHEM_EMERGE_KEYWORD_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(&leading))",
        "OPPONENT_LOST_LIFE_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words)",
        "YOU_NOT_CAST_SPELL_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words)",
        "YOU_CAST_SPELL_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words)",
        "NO_CARDS_IN_YOUR_LIBRARY_CONDITION_PATTERN.matches_words(&clause_words)",
        "SOURCE_IS_ON_BATTLEFIELD_CONDITION_PATTERN.matches_words(&clause_words)",
        "SOURCE_DEVOURED_CREATURES_CONDITION_PATTERN.matches_words(&clause_words)",
        "SOURCE_IS_SOULBOND_PAIRED_CONDITION_PATTERN.matches_words(&clause_words)",
        "SOURCE_ATTACKED_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words)",
        "YOU_ATTACKED_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words)",
        "SOURCE_ENTERED_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words)",
        "YOUR_TURN_CONDITION_PATTERN.matches_words(&clause_words)",
        "SOURCE_POWER_EVEN_CONDITION_PATTERN.matches_words(&clause_words)",
        "SOURCE_POWER_ODD_CONDITION_PATTERN.matches_words(&clause_words)",
        "NOT_YOUR_TURN_CONDITION_PATTERN.matches_words(&clause_words)",
        "YOUR_LIFE_HALF_STARTING_CONDITION_PATTERN.matches_words(&clause_words)",
        "ANTHEM_SOURCE_PRONOUN_SUBJECT_PATTERN.matches_words(subject_words)",
        "SOURCE_IN_GRAVEYARD_TAIL_PATTERN.matches_words(remainder_words)",
        "THERE_IS_OR_ARE_PREFIX_PATTERN.matches_words(&clause_words)",
        "SOURCE_IN_GRAVEYARD_TAIL_PATTERN.matches_words(zone_tail)",
        "ANTHEM_ENTERED_WORD_MARKER_PATTERN.matches_words(&clause_words)",
        "YOU_COMMITTED_CRIME_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words)",
        "ON_SOURCE_COUNTER_TAIL_PATTERN.matches_words(tail)",
        "ANTHEM_FOR_EACH_PREFIX_PATTERN.matches_words(&token_words)",
        "ANTHEM_AFFECTED_ATTACKED_THIS_TURN_PATTERN.matches_words(&rest_words)",
        "ANTHEM_AFFECTED_COLORS_PATTERN.matches_words(&rest_words)",
        "ANTHEM_BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(&rest_words)",
        "ANTHEM_CREATURE_TYPES_AMONG_PREFIX_PATTERN.matches_words(&rest_words)",
        "ANTHEM_ATTACHED_TO_SOURCE_TAIL_PATTERN.matches_words(&tail_words)",
        "ANTHEM_UNSPENT_GREEN_MANA_YOU_HAVE_PATTERN.matches_words(&rest_words)",
        "ON_SOURCE_COUNTER_TAIL_PATTERN.matches_words(tail_words)",
        "ANTHEM_GRAVEYARD_CONJUNCTION_SPLIT_MARKER_PATTERN.matches_words(&filter_words)",
        "ALL_CREATURES_LOSE_FLYING_PATTERN.matches_words(&words)",
        "CANT_GAIN_ABILITY_TAIL_PATTERN.matches_words(&after_words)",
        "ANTHEM_LOSE_ALL_ABILITIES_PATTERN.matches_words(&words)",
        "ANTHEM_EXCEPT_MANA_ABILITIES_PATTERN.matches_words(&words)",
        "ANTHEM_ALL_ABILITIES_TAIL_PATTERN.matches_words(&words[lose_idx + 1..])",
        "ANTHEM_UNTIL_WORD_PATTERN.matches_words(&words)",
        "ANTHEM_BECOMES_WORD_PATTERN.matches_words(&words)",
        "ANTHEM_BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches_words(after_have)",
        "ANTHEM_GET_OR_GETS_CONTAINS_PATTERN.matches_words(&words[..have_idx])",
        "ANTHEM_BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches_words(rest_words)",
        "ANTHEM_BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches_words(&rest_words)",
        "CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN.matches_words(&all_words)",
        "CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN.matches_words(&normalized)",
        "CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN.matches_words(&words)",
        "CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN.matches_words(&clause_words)",
        "ALL_CREATURES_BLOCK_THIS_CREATURE_TAIL_PATTERN.matches_words(&remainder_words)",
        "ALL_CREATURES_BLOCK_ENCHANTED_CREATURE_TAIL_PATTERN.matches_words(&remainder_words)",
        "CAN_ATTACK_AS_NO_DEFENDER_AS_LONG_AS_PATTERN.matches_words(&normalized)",
        "CAN_ATTACK_AS_NO_DEFENDER_PATTERN.matches_words(&normalized)",
        "CANT_BE_BLOCKED_WORDS_PATTERN.matches_words(&tail_words)",
        "ANTHEM_TARGET_CONTAINS_PATTERN.matches_words(&words)",
        "ANTHEM_TARGET_CONTAINS_PATTERN.matches_words(&all_words)",
        "ANTHEM_TARGET_CONTAINS_PATTERN.matches_words(&subject_words)",
        "UNTIL_YOUR_NEXT_TURN_PREFIX_PATTERN.matches_words(&subject_words)",
        "ANTHEM_NO_LONGER_PREFIX_PATTERN.matches_words(&words[1..])",
        "THIS_CREATURE_PREFIX_PATTERN.matches_words(&subject_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route migrated anthem shape gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn trigger_subject_filters_shape_gates_use_direct_word_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/trigger_subject_filters.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn trigger_subject_shape_matches_words",
        "fn find_token_shape",
    );

    for required in [
        "fn trigger_subject_shape_matches_words",
        "shape.matches_word_slice(words)",
        "fn trigger_subject_word_is_any",
        "fn trigger_subject_token_word_is",
        "fn find_trigger_subject_token_word",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose a direct word-matching trigger-subject shape helper: missing `{required}`"
        );
    }
    assert!(
        content.contains("trigger_subject_shape_matches_words("),
        "{relative} should route trigger-subject shape gates through the direct word-matching helper"
    );
    assert!(
        !helper.contains("synthetic_word_tokens(words)")
            && !helper.contains(
                "shape.matches(crate::runtime_backend::lexer::LexedClause::new(&tokens))"
            )
            && !content.contains(".matches_word(")
            && !content.contains(".matches_token("),
        "{relative} should not route trigger-subject shape gates through synthetic token bridges or singleton ClauseShape probes"
    );
}

#[test]
fn activation_restriction_clauses_shape_gates_use_direct_word_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activation_restriction_clauses.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn activation_restriction_shape_matches_words",
        "fn simple_negated_object_restriction",
    );

    for required in [
        "fn activation_restriction_shape_matches_words",
        "shape.matches_word_slice(words)",
        "fn activation_restriction_word_is_any",
        "fn activation_restriction_token_word_is",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose a direct word-matching activation-restriction shape helper: missing `{required}`"
        );
    }
    assert!(
        content.contains("activation_restriction_shape_matches_words("),
        "{relative} should route activation-restriction shape gates through the direct word-matching helper"
    );
    assert!(
        !content.contains("synthetic_word_tokens(words)")
            && !content.contains(
                "shape.matches(crate::runtime_backend::lexer::LexedClause::new(&tokens))"
            )
            && !content.contains(".matches_word(")
            && !content.contains(".matches_token("),
        "{relative} should not route activation-restriction shape gates through raw word refs or singleton ClauseShape probes"
    );
}

#[test]
fn activation_costs_shape_gates_use_direct_word_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activation_costs.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn activation_cost_shape_matches_words",
        "fn cant_attack_unless_tail",
    );

    for required in [
        "fn activation_cost_shape_matches_words",
        "shape.matches_word_slice(words)",
        "fn parse_activation_count_words(words: &[&str]) -> Option<(u32, usize)>",
        "ironsmith_core::parse_cardinal_words(words)",
        "parse_activation_count_words(words.get(2..).unwrap_or_default())",
    ] {
        assert!(
            content.contains(required) || helper.contains(required),
            "{relative} should expose a direct word-matching activation-cost shape helper: missing `{required}`"
        );
    }
    assert!(
        content.contains("activation_cost_shape_matches_words("),
        "{relative} should route activation-cost shape gates through the direct word-matching helper"
    );
    assert!(
        !helper.contains("synthetic_word_tokens(words)")
            && !helper.contains(
                "shape.matches(crate::runtime_backend::lexer::LexedClause::new(&tokens))"
            )
            && !content.contains(
                "let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words)"
            ),
        "{relative} should not route activation-cost shape/count gates through synthetic token bridges"
    );
}

#[test]
fn activated_line_core_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activated_line_core.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn find_token_word_exact",
        "pub(crate) fn joined_activation_clause_text",
    );

    for required in [
        "fn find_token_word_exact",
        "fn activated_words_start_with",
        "fn activated_words_start_with_any",
        "fn activated_words_equal",
        "fn activated_words_equal_any",
        "fn activated_words_contain_all",
        "fn activated_words_contain_phrase",
        "word_slice_starts_with(words, prefix)",
        "word_slice_contains_all_words(words, required)",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose token-backed activated-line word helpers: missing `{required}`"
        );
    }
    assert!(
        content.contains("PRIMARY_ADD_MANA_CLAUSE_PREFIXES")
            && content.contains("activated_words_start_with_any(")
            && content.contains("activated_words_contain_phrase(")
            && content.contains("activated_words_equal_any("),
        "{relative} should route activated-line shape gates through token-backed word helpers"
    );
    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains(".matches_words("),
        "{relative} should not route activated-line shape gates through ClauseShape/raw word refs"
    );
}

#[test]
fn keyword_action_costs_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/keyword_action_costs.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn keyword_action_words_start_with",
        "const SIMPLE_HEAD_KEYWORD_ACTIONS",
    );

    for required in [
        "fn keyword_action_words_start_with",
        "fn keyword_action_words_start_with_any",
        "fn keyword_action_words_contain_any_phrase",
        "fn keyword_action_word_at_is_any",
        "word_slice_starts_with(words, prefix)",
        "word_slice_contains_any_phrase(words, phrases)",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose token-backed keyword-action-cost word helpers: missing `{required}`"
        );
    }
    assert!(
        content.contains("keyword_action_words_start_with(")
            && content.contains("keyword_action_words_contain_any_word(")
            && content.contains("keyword_action_word_at_is("),
        "{relative} should route keyword-action-cost shape gates through token-backed word helpers"
    );
    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains(".matches_words("),
        "{relative} should not route keyword-action-cost shape gates through ClauseShape/raw word refs"
    );
}

#[test]
fn object_filters_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/object_filters.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn object_filter_words_contain_phrase",
        "fn word_slice_match",
    );

    for required in [
        "fn object_filter_words_contain_phrase",
        "word_slice_contains_phrase(words, phrase)",
        "fn object_filter_words_equal_any",
        "word_slice_eq_any(words, expected)",
        "fn object_filter_word_is_other_or_another",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose token-backed object-filter word helpers: missing `{required}`"
        );
    }
    assert!(
        content.contains("object_filter_words_contain_any_phrase(")
            && content.contains("object_filter_word_is_other_or_another(")
            && content.contains("object_filter_words_equal_any("),
        "{relative} should route object-filter shape gates through token-backed word helpers"
    );
    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains(".matches_words("),
        "{relative} should not route object-filter shape gates through ClauseShape/raw word refs"
    );
}

#[test]
fn choice_object_clauses_shape_gates_use_direct_word_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/choice_object_clauses.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn choice_object_shape_matches_words",
        "fn strip_chosen_player_prefix",
    );

    for required in [
        "fn choice_object_shape_matches_words",
        "shape.matches_word_slice(words)",
        "fn choice_word_is_any",
        "fn choice_token_word_is_any",
        "fn find_choice_token_word",
        "fn find_choice_token_word_any",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose a direct word-matching choice-object shape helper: missing `{required}`"
        );
    }
    assert!(
        content.contains("choice_object_shape_matches_words("),
        "{relative} should route choice-object shape gates through the direct word-matching helper"
    );
    assert!(
        !content.contains("synthetic_word_tokens(words)")
            && !content.contains(
                "shape.matches(crate::runtime_backend::lexer::LexedClause::new(&tokens))"
            )
            && !content.contains(".matches_word(")
            && !content.contains(".matches_token("),
        "{relative} should not route choice-object shape gates through raw word refs or singleton ClauseShape probes"
    );
}

#[test]
fn attached_object_static_lines_shape_gates_use_direct_word_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/attached_object_static_lines.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn attached_shape_matches_words",
        "fn attached_find_prefix_shape_start",
    );

    for required in [
        "fn attached_shape_matches_words",
        "shape.matches_word_slice(words)",
        "fn attached_word_is_any",
        "fn attached_token_word_is",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose a direct word-matching attached-object shape helper: missing `{required}`"
        );
    }
    assert!(
        content.contains("attached_shape_matches_words("),
        "{relative} should route attached-object shape gates through the direct word-matching helper"
    );
    assert!(
        !content.contains("synthetic_word_tokens(words)")
            && !content.contains(
                "shape.matches(crate::runtime_backend::lexer::LexedClause::new(&tokens))"
            )
            && !content.contains(".matches_word(")
            && !content.contains(".matches_token("),
        "{relative} should not route attached-object shape gates through raw word refs or singleton ClauseShape probes"
    );
}

#[test]
fn naming_and_reference_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/naming_and_reference.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "fn find_any_phrase_start(words: &[&str], phrases: &[&[&str]]) -> Option<usize>",
        "fn contains_any_phrase(words: &[&str], phrases: &[&[&str]]) -> bool",
        "fn words_contain_all(words: &[&str], expected: &[&str]) -> bool",
        "fn token_word_is_any(token: &OwnedLexToken, expected: &[&str]) -> bool",
        "fn parse_min_color_count_quantity_prefix(words: &[&str]) -> Option<(u32, usize)>",
        "let (count, used) = parse_min_color_count_quantity_prefix(words)?",
    ] {
        assert!(
            content.contains(required),
            "{relative} should expose word/phrase helpers for naming/reference gates: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape!",
        "naming_reference_shape_matches_words",
        ".matches_words(",
        ".matches_word(",
        ".matches_token(",
        "find_exact_window",
        "synthetic_word_tokens(words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route naming/reference gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn grammar_structure_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/structure.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn structure_token_is",
        "#[derive(Debug, Clone, PartialEq)]",
    );

    for required in [
        "fn structure_token_is",
        "fn structure_token_is_any",
        "fn find_token_word",
        "fn structure_words_equal",
        "fn structure_words_equal_any",
        "fn structure_words_start_with_any",
        "fn structure_words_end_with_any",
        "word_slice_eq(words, expected)",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose token-backed grammar-structure word helpers: missing `{required}`"
        );
    }
    assert!(
        content.contains("structure_words_equal_any(")
            && content.contains("structure_words_start_with_any(")
            && content.contains("structure_token_is("),
        "{relative} should route grammar-structure gates through token-backed word helpers"
    );
    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains(".matches_words("),
        "{relative} should not route grammar-structure shape gates through ClauseShape/raw word refs"
    );
}

#[test]
fn unsupported_shapes_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/unsupported_shapes.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn unsupported_words_contain_phrase",
        "pub(crate) fn is_enters_as_copy_clause_lexed",
    );

    for required in [
        "fn unsupported_words_contain_phrase",
        "word_slice_contains_phrase(words, phrase)",
        "fn unsupported_words_contain_all",
        "word_slice_contains_all_words(words, required)",
        "fn unsupported_words_contain_any",
        "word_slice_contains_any_word(words, candidates)",
        "fn unsupported_prefix_start",
        "word_slice_starts_with_any(&words[*idx..], prefixes)",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose token-backed unsupported-shape word helpers: missing `{required}`"
        );
    }
    assert!(
        content.contains("unsupported_words_contain_phrase(")
            && content.contains("unsupported_words_contain_any_phrase(")
            && content.contains("unsupported_prefix_start("),
        "{relative} should route unsupported-shape gates through token-backed word helpers"
    );
    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains(".matches_words("),
        "{relative} should not route unsupported-shape gates through ClauseShape/raw word refs"
    );
}

#[test]
fn grammar_search_library_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/search_library.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn search_library_token_is_any_word",
        "pub(crate) fn last_non_article_parser_word_token_idx",
    );

    for required in [
        "fn search_library_token_is_any_word",
        "fn search_library_words_equal_any",
        "fn search_library_words_start_with_any",
        "fn search_library_words_contain_phrase",
        "fn search_library_words_contain_all",
        "fn search_library_words_are_default_card_selector",
        "crate::word_primitives::contains_phrase(words, phrase)",
        "required\n        .iter()\n        .all(|required_word| words.iter().any(|word| word == required_word))",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose grammar search-library word helper shapes without synthetic tokens: missing `{required}`"
        );
    }
    assert!(
        content.contains("search_library_words_start_with_any(")
            && content.contains("search_library_words_equal_any(")
            && content.contains("search_library_words_contain_all("),
        "{relative} should route grammar search-library shape gates through token/phrase helpers"
    );
    assert!(
        !helper.contains("synthetic_word_tokens(words)"),
        "{relative} should not rebuild search-library helper tokens from word slices"
    );
    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains(".matches_words("),
        "{relative} should not route grammar search-library shape gates through ClauseShape/raw word refs"
    );
}

#[test]
fn shared_value_helpers_shape_gates_use_word_ref_matching() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/value_helpers.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn value_helper_words_match_pattern",
        "fn value_helper_find_any_phrase_start",
    );

    for required in [
        "use super::lex_patterns::LexPattern",
        "fn value_helper_words_match_pattern",
        "fn value_helper_words_start_with_pattern",
        "fn value_helper_words_contain_any",
        "fn value_helper_words_equal_any",
        "fn value_helper_find_exact_phrase",
        "pattern.match_word_refs(words).is_some()",
        "pattern.match_prefix_word_refs(words).is_some()",
        ".any(|expected_word| words.iter().any(|word| word == expected_word))",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should expose direct word-ref shared value-helper pattern helpers: missing `{required}`"
        );
    }
    assert!(
        content.contains("value_helper_words_start_with_pattern(")
            && content.contains("value_helper_find_exact_phrase(")
            && content.contains("value_helper_words_equal_any("),
        "{relative} should route shared value-helper shape gates through direct word-ref/phrase helpers"
    );
    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains(".matches_words(")
            && !content.contains("synthetic_word_tokens(words)"),
        "{relative} should not route shared value-helper shape gates through ClauseShape/raw word refs"
    );
}

#[test]
fn family_clause_support_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/clause_support.rs";
    let content = read_repo_file(&root, relative);
    let protection_helpers = function_source(
        &content,
        "const PROTECTION_FROM_COLORED_SPELLS_PATTERN",
        "fn parse_protection_chain",
    );

    for required in [
        "use super::lex_patterns::{LexCaptureKind, LexPattern}",
        "const PROTECTION_EACH_MANA_VALUE_AMONG_PATTERN",
        "const EACH_MANA_VALUE_AMONG_PATTERN",
        "LexPattern::object(\"filter\", LexCaptureKind::Rest)",
        "fn protection_from_colored_spells_action(tokens: &[OwnedLexToken])",
        "fn protection_from_each_mana_value_among_action(tokens: &[OwnedLexToken])",
        "fn protection_from_each_mana_value_among_tail_action",
        "fn protection_each_mana_value_among_filter",
        "pattern.match_clause(clause)",
        "matched.capture_clause(\"filter\", clause)",
        "parse_object_filter_lexed(&filter_tokens, false)",
    ] {
        assert!(
            content.contains(required) || protection_helpers.contains(required),
            "{relative} should expose captured LexPattern protection helpers: missing `{required}`"
        );
    }
    assert!(
        content.contains("protection_from_colored_spells_action(tokens)")
            && content.contains("protection_from_each_mana_value_among_action(tokens)")
            && content.contains(
                "protection_from_each_mana_value_among_tail_action(&tokens[target_start..])"
            ),
        "{relative} should route protection shape gates through token-backed LexPattern capture helpers"
    );
    for forbidden in [
        "PROTECTION_EACH_MANA_VALUE_AMONG_PREFIX_PATTERN",
        "EACH_MANA_VALUE_AMONG_PREFIX_PATTERN",
        "fn clause_support_words_match_pattern",
        "token_index_for_word_index(idx + 5)",
        "protection_from_colored_spells_action(&words)",
        "protection_from_each_mana_value_among_action(&words",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep bespoke protection prefix splitting: `{forbidden}`"
        );
    }
    assert!(
        content.contains("fn clause_support_words_start_with_pattern")
            && content.contains("pattern.match_prefix_word_refs(words).is_some()")
            && content.contains("fn clause_support_words_contain_all"),
        "{relative} still carries shared family word helpers while protection tails move to captures"
    );
    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains(".matches_words(")
            && !content.contains("synthetic_word_tokens(words)"),
        "{relative} should not route family clause-support shape gates through ClauseShape/raw word refs"
    );
}

#[test]
fn modal_helpers_result_predicates_use_captured_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/modal_helpers.rs";
    let content = read_repo_file(&root, relative);
    let shape_parser = function_source(
        &content,
        "fn parse_modal_result_shape_from_clause",
        "pub(crate) fn parse_if_result_predicate",
    );
    let short_parser = function_source(
        &content,
        "pub(crate) fn parse_if_result_predicate",
        "pub(crate) fn parse_if_result_predicate_lexed",
    );
    let lexed_parser = function_source(
        &content,
        "pub(crate) fn parse_if_result_predicate_lexed",
        "None\n}",
    );

    for required in [
        "enum ModalResultSubject",
        "enum ModalResultShape",
        "fn modal_non_article_word_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken>",
        "fn parse_modal_result_shape_from_clause(clause: LexedClause<'_>) -> Option<ModalResultShape>",
        "const THIS_WAY_RESULT_PATTERN: LexPattern<'static>",
        "const CONTRACTED_NEGATED_THIS_WAY_RESULT_PATTERN: LexPattern<'static>",
        "const SPLIT_NEGATED_THIS_WAY_RESULT_PATTERN: LexPattern<'static>",
        "const CONTRACTED_EXACT_NEGATED_RESULT_PATTERN: LexPattern<'static>",
        "const SPLIT_EXACT_NEGATED_RESULT_PATTERN: LexPattern<'static>",
        "LexPattern::subject(\"subject\", LexCaptureKind::OneOf(MODAL_RESULT_SUBJECT_WORDS))",
        "LexPattern::action(\"result\", LexCaptureKind::OneOf(RESULT_VERB_WORDS))",
        "LexPattern::modifier(\"negation\", LexCaptureKind::OneOf(CONTRACTED_NEGATION_WORDS))",
        "capture_clause_by_role(LexCaptureRole::Subject, clause)",
        "capture_clause_by_role(LexCaptureRole::Modifier, clause)",
        "capture_clause_by_role(LexCaptureRole::Action, clause)",
    ] {
        assert!(
            content.contains(required) || shape_parser.contains(required),
            "{relative} should classify modal result predicates through captured shapes: missing `{required}`"
        );
    }

    for required in [
        "let normalized_tokens = modal_non_article_word_tokens(tokens)",
        "let clause = LexedClause::new(&normalized_tokens)",
        "match parse_modal_result_shape_from_clause(clause)?",
        "let modal_result_shape = parse_modal_result_shape_from_clause(clause)",
        "ModalResultShape::ThisWay",
        "ModalResultShape::ExactNegated",
        "ModalResultSubject::If | ModalResultSubject::When",
        "ModalResultSubject::You | ModalResultSubject::They",
        "fn modal_clause_matches_pattern(clause: LexedClause<'_>, pattern: LexPattern<'static>) -> bool",
        "fn modal_clause_matches_prefix(clause: LexedClause<'_>, pattern: LexPattern<'static>) -> bool",
        "fn modal_words_end_this_way(words: &[&str]) -> bool",
        "modal_clause_matches_pattern(clause, YOU_DO_PATTERN)",
        "modal_clause_matches_prefix(clause, YOU_WIN_PREFIX_PATTERN)",
        "words.contains(&\"clash\")",
        "modal_clause_matches_prefix(clause, YOU_SEARCHED_PREFIX_PATTERN)",
        "modal_clause_matches_prefix(clause, SPELL_COUNTERED_SUBJECT_PATTERN)",
        "words.contains(&\"countered\")",
        "modal_clause_matches_prefix(clause, EXCESS_DAMAGE_WAS_DEALT_PREFIX_PATTERN)",
        "words.contains(&\"creature\")",
        "modal_clause_matches_prefix(clause, POWER_BECOMES_PREFIX_PATTERN)",
    ] {
        assert!(
            content.contains(required)
                || short_parser.contains(required)
                || lexed_parser.contains(required),
            "{relative} should consume captured modal result shapes in predicate parsers: missing `{required}`"
        );
    }

    for forbidden in [
        "RESULT_VERB_WORD_PATTERN",
        "THIS_WAY_SUFFIX_PATTERN",
        "RESULT_QUALIFIER_PATTERN",
        "CONTRACTED_NEGATION_WORD_PATTERN",
        "SPLIT_NEGATION_FIRST_WORD_PATTERN",
        "NOT_WORD_PATTERN",
        "is_unqualified_this_way_result",
        "is_exact_negated_result",
        "is_negated_this_way_result",
        "ClauseShape",
        "clause_shape!",
        "modal_words_match_shape",
        "modal_words_match_clause_pattern",
        "modal_words_match_prefix_pattern",
        "parse_modal_result_shape_from_words",
        "synthetic_word_tokens(words)",
        "YOU_SEARCHED_THIS_WAY_PATTERN",
        "SPELL_COUNTERED_THIS_WAY_PATTERN",
        "EXCESS_DAMAGE_WAS_DEALT_THIS_WAY_PATTERN",
        "POWER_BECOMES_THIS_WAY_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep modal result predicate logic as bespoke word closures or one-off patterns: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_families_fallback_dispatch_uses_captured_prefix_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_families.rs";
    let content = read_repo_file(&root, relative);
    let fallback_parser = function_source(
        &content,
        "fn keyword_fallback_kind(tokens: &[OwnedLexToken]) -> Option<KeywordFallbackKind>",
        "mod additional_costs",
    );
    let dispatcher = function_source(
        &content,
        "pub(super) fn parse_keyword_dispatch_hint",
        "None\n}",
    );

    for required in [
        "enum KeywordFallbackKind",
        "fn keyword_fallback_kind(tokens: &[OwnedLexToken]) -> Option<KeywordFallbackKind>",
        "const KEYWORD_FALLBACK_PREFIX_PATTERN: LexPattern<'static>",
        "LexPattern::action(\n            \"keyword\",",
        "LexCaptureKind::OneOfPhrase(&[\n                &[\"basic\", \"landcycling\"],",
        "KEYWORD_FALLBACK_PREFIX_PATTERN.match_prefix(clause)",
        "capture_clause_by_role(LexCaptureRole::Action, clause)",
    ] {
        assert!(
            content.contains(required) || fallback_parser.contains(required),
            "{relative} should classify fallback keyword prefixes through captured shapes: missing `{required}`"
        );
    }

    for required in [
        "let fallback_kind = keyword_fallback_kind(tokens)",
        "KeywordFallbackKind::BasicLandcycling",
        "KeywordFallbackKind::Encore | KeywordFallbackKind::JumpStart",
    ] {
        assert!(
            dispatcher.contains(required),
            "{relative} should dispatch fallback keyword prefixes from the captured classifier: missing `{required}`"
        );
    }

    for forbidden in [
        "BASIC_LANDCYCLING_FALLBACK_PATTERN",
        "ENCORE_FALLBACK_PATTERN",
        "JUMP_START_FALLBACK_PATTERN",
        "use super::effect_sentences::clause_pattern_helpers",
        "ClauseShape",
        "clause_shape!",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep fallback keyword prefixes as one-off ClauseShape probes: found `{forbidden}`"
        );
    }
}

#[test]
fn parser_semantic_lowering_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "IF_YOU_DO_PATTERN.matches_word_slice(&token_word_refs(tokens))",
        "SELF_X_COUNTER_ETB_PATTERN.matches_word_slice(&token_word_refs(tokens))",
        "word_slice_ends_with_any(&token_word_refs(attack_tokens), ATTACK_ACTION_SUFFIXES)",
        "word_slice_starts_with(&parser_token_word_refs(tokens), PARTNER_WITH_PREFIX)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route parser-semantic lowering gates through token-backed clause-shape predicates: missing `{required}`"
        );
    }
    assert!(
        !content.contains("parser_semantic_shape_matches_words("),
        "{relative} should no longer keep a synthetic-token shape helper for boolean lowering gates"
    );
    for forbidden in [".matches_words("] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route parser-semantic lowering gates through the banned synthetic-token matches_words adapter: found `{forbidden}`"
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
    for required in [
        "let clause = LexedClause::new(tokens)",
        "CONDITIONAL_DRAW_REPLACEMENT_A_CARD_PREFIX_PATTERN.matches(clause)",
        "CONDITIONAL_DRAW_REPLACEMENT_CARD_PREFIX_PATTERN.matches(clause)",
        ".is_some_and(|tail| CONDITIONAL_DRAW_LIFE_LOSS_TAIL_PATTERN.matches(tail))",
    ] {
        assert!(
            draw_replacement_parser.contains(required),
            "{static_relative} should parse conditional draw replacement shape gates through token clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "CONDITIONAL_DRAW_REPLACEMENT_A_CARD_PREFIX_PATTERN.matches_words(&words)",
        "CONDITIONAL_DRAW_REPLACEMENT_CARD_PREFIX_PATTERN.matches_words(&words)",
        "CONDITIONAL_DRAW_LIFE_LOSS_TAIL_PATTERN.matches_words(tail)",
    ] {
        assert!(
            !draw_replacement_parser.contains(forbidden),
            "{static_relative} should not parse conditional draw replacement shape gates through raw word slices: found `{forbidden}`"
        );
    }
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
fn this_spell_cost_conditions_use_clause_shapes_and_life_change_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_this_spell_target_condition",
        "fn parse_conjoined_this_spell_cost_condition",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "IT_TARGETS_PREFIX_PATTERN.matches(clause)",
        "THIS_SPELL_TARGETS_PREFIX_PATTERN.matches(clause)",
        "let target_clause = clause.after_words(target_start)?",
        "parse_player_life_change_this_turn_condition(tokens)",
        "this_spell_cost_condition_from_life_change_this_turn",
        "LIFE_TOTAL_LESS_THAN_STARTING_PATTERN.matches(clause)",
        "YOU_ATTACKED_THIS_TURN_PATTERN.matches(clause)",
        "OPPONENT_CONTROLS_PREFIX_PATTERN.matches(clause)",
        "ASSASSIN_OR_COMMANDER_COMBAT_DAMAGE_PATTERN.matches(clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse this-spell cost condition gates through token clauses and shared captures: missing `{required}`"
        );
    }
    for forbidden in [
        "IT_TARGETS_PREFIX_PATTERN.matches_words(&w)",
        "THIS_SPELL_TARGETS_PREFIX_PATTERN.matches_words(&w)",
        "YOU_GAINED_LIFE_THIS_TURN_PATTERN.matches_words(&w)",
        "YOU_GAINED_PREFIX_PATTERN.matches_words(&w)",
        "LIFE_THIS_TURN_SUFFIX_PATTERN.matches_words(&w)",
        "LIFE_THIS_TURN_SUFFIX_PATTERN.matches_words(&w[rest_start..])",
        "OPPONENT_CONTROLS_PREFIX_PATTERN.matches_words(&w)",
        "ASSASSIN_OR_COMMANDER_COMBAT_DAMAGE_PATTERN.matches_words(&w)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse this-spell cost condition gates through raw word vectors: found `{forbidden}`"
        );
    }
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

    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_families.rs";
    let content = read_repo_file(&root, relative);
    let dispatcher = function_source(&content, "fn keyword_fallback_kind", "mod additional_costs");
    for required in [
        "let clause = LexedClause::new(tokens)",
        "KEYWORD_FALLBACK_PREFIX_PATTERN.match_prefix(clause)",
        "matched.capture_clause_by_role(LexCaptureRole::Action, clause)",
        "keyword_clause.word_refs().as_slice()",
    ] {
        assert!(
            dispatcher.contains(required),
            "{relative} should route keyword-family fallback gates through token clauses: missing `{required}`"
        );
    }
    for forbidden in [
        "BASIC_LANDCYCLING_FALLBACK_PATTERN.matches_words(&word_refs)",
        "ENCORE_FALLBACK_PATTERN.matches_words(&word_refs)",
        "JUMP_START_FALLBACK_PATTERN.matches_words(&word_refs)",
    ] {
        assert!(
            !dispatcher.contains(forbidden),
            "{relative} should not route keyword-family fallback gates through raw word refs: found `{forbidden}`"
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
        "let clause = LexedClause::new(tokens)",
        "TOUGHNESS_CREWS_VEHICLES_MARKER_PATTERN.matches(clause)",
        "POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches(clause)",
        "LOYALTY_COUNTER_INSTEAD_OF_CREW_COST_MARKER_PATTERN.matches(clause)",
    ] {
        assert!(
            marker_support.contains(expected),
            "{relative} should keep supported keyword-static marker routing on ClauseShape `{expected}`"
        );
    }
    for forbidden in [
        "let words = parser_token_word_refs(tokens)",
        "TOUGHNESS_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)",
        "POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)",
        "LOYALTY_COUNTER_INSTEAD_OF_CREW_COST_MARKER_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !marker_support.contains(forbidden),
            "{relative} should not route supported keyword-static markers through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_early_parser_routes_full_line_markers_through_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_static_ability_ast_line_early_lexed",
        "pub(crate) fn parse_damage_doubling_mana_value_marker_line",
    );

    for required in [
        "X_CANT_EXCEED_PLAYER_COUNT_PATTERN.matches_non_article_tokens(tokens)",
        "EXHAUST_AS_THOUGH_UNACTIVATED_PATTERN.matches_non_article_tokens(tokens)",
        "CANT_ATTACK_UNLESS_CAST_CREATURE_SPELL_PATTERN.matches_non_article_tokens(tokens)",
        "CANT_ATTACK_UNLESS_CAST_NONCREATURE_SPELL_PATTERN.matches_non_article_tokens(tokens)",
        "parse_can_block_additional_creature_each_combat_line(tokens)?",
        "let clause = LexedClause::new(tokens)",
        "DAY_NIGHT_AS_ENTERS_CONTAINS_PATTERN.matches(clause)",
        "DAY_NIGHT_AS_ENTERS_PATTERN.matches(clause)",
        "TOUGHNESS_CREWS_VEHICLES_MARKER_PATTERN.matches(clause)",
        "POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches(clause)",
        "LOYALTY_COUNTER_INSTEAD_OF_CREW_COST_MARKER_PATTERN.matches(clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route early full-line static markers through token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "let rendered_words = parser_token_word_refs(tokens)",
        "X_CANT_EXCEED_PLAYER_COUNT_PATTERN.matches_words(&rendered_words)",
        "EXHAUST_AS_THOUGH_UNACTIVATED_PATTERN.matches_words(&rendered_words)",
        "CANT_ATTACK_UNLESS_CAST_CREATURE_SPELL_PATTERN.matches_words(&rendered_words)",
        "CANT_ATTACK_UNLESS_CAST_NONCREATURE_SPELL_PATTERN.matches_words(&rendered_words)",
        "SOURCE_CAN_BLOCK_PREFIX_PATTERN.matches_words(&words)",
        "BLOCK_ADDITIONAL_DURATION_TAIL_PATTERN.matches_words(&words[idx + 1..])",
        "DAY_NIGHT_AS_ENTERS_CONTAINS_PATTERN.matches_words(&words)",
        "DAY_NIGHT_AS_ENTERS_PATTERN.matches_words(&words)",
        "TOUGHNESS_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)",
        "POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)",
        "LOYALTY_COUNTER_INSTEAD_OF_CREW_COST_MARKER_PATTERN.matches_words(&words)",
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
fn keyword_static_text_markers_use_token_shapes_for_simple_lines() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_static_text_marker_line",
        "pub(crate) fn parse_filter_dont_untap_during_controllers_untap_steps_line",
    );

    for required in [
        "BANDING_MARKER_PATTERN.matches_non_article_tokens(tokens)",
        "YOU_HAVE_HEXPROOF_PATTERN.matches_non_article_tokens(tokens)",
        "YOU_HAVE_PROTECTION_FROM_OPPONENTS_PATTERN.matches_non_article_tokens(tokens)",
        "let clause = LexedClause::new(tokens)",
        "OPPONENTS_CAST_ONLY_AS_SORCERY_PATTERN.matches(clause)",
        "DOUBLE_DAMAGE_TO_ENCHANTED_PLAYER_PATTERN.matches(clause)",
        "AFFINITY_FOR_FILTER_PATTERN.match_clause(clause)",
        "AFFINITY_FOR_ARTIFACTS_PATTERN.matches(clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route simple static text markers through token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "BANDING_MARKER_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens))",
        "YOU_HAVE_HEXPROOF_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens))",
        "YOU_HAVE_PROTECTION_FROM_OPPONENTS_PATTERN\n        .matches_words(&crate::runtime_backend::token_word_refs(tokens))",
        "let words = parser_token_word_refs(tokens)",
        "OPPONENTS_CAST_ONLY_AS_SORCERY_PATTERN.matches_words(&words)",
        "DOUBLE_DAMAGE_TO_ENCHANTED_PLAYER_PATTERN.matches_words(&words)",
        "let words = parser_token_word_refs(&core_tokens)",
        "AFFINITY_FOR_ARTIFACTS_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild raw word vectors for simple static text markers: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_damage_doubling_marker_uses_lexed_clause_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_damage_doubling_mana_value_marker_line",
        "pub(crate) fn parse_static_ability_ast_line_lexed",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "DAMAGE_DOUBLING_MANA_VALUE_MARKER_PATTERN.matches(clause)",
        "DAMAGE_DOUBLING_TO_TARGET_PATTERN.matches(clause)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route damage-doubling marker through LexedClause shape matching: missing `{required}`"
        );
    }
    for forbidden in [
        "let clause_words = crate::runtime_backend::token_word_refs(tokens)",
        "DAMAGE_DOUBLING_MANA_VALUE_MARKER_PATTERN.matches_words(&clause_words)",
        "DAMAGE_DOUBLING_TO_TARGET_PATTERN.matches_words(&clause_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild raw words for damage-doubling marker: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_static_damage_cleanup_marker_uses_token_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_damage_not_removed_cleanup_line",
        "fn parse_as_enters_choice_subject_clause",
    );

    assert!(
        parser.contains("DAMAGE_NOT_REMOVED_CLEANUP_PATTERN.matches_non_article_tokens(tokens)"),
        "{relative} should route damage cleanup marker through token shape"
    );
    assert!(
        !parser.contains("DAMAGE_NOT_REMOVED_CLEANUP_PATTERN.matches_words(&words)"),
        "{relative} should not rebuild raw words for damage cleanup marker"
    );
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
        helper.contains("EXHAUST_ONCE_RESTRICTION_PATTERN.matches(LexedClause::new(tokens))"),
        "{scanner_relative} should detect exhaust-once restrictions through LexedClause matching"
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
fn leaf_shape_gates_use_token_backed_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/leaf.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with(lowered.as_slice(), LEAF_ONE_OR_MORE_PREFIX)",
        "word_slice_ends_with(tail, FROM_YOUR_HAND_SUFFIX)",
        "word_slice_eq_any(target, LEAF_COUNTER_REMOVAL_SELF_TARGET_PHRASES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route leaf parser shape gates through token-slice helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "leaf_shape_matches_words",
        ".matches_words(",
        ".matches_word(",
        ".matches_token(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route leaf parser shape gates through retired shape helpers: found `{forbidden}`"
        );
    }
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
    assert!(
        helper.contains("fn line_words_start_with(line: &str, prefix: &[&str]) -> bool")
            && helper
                .contains("line_words_start_with(line, ADDITIONAL_COST_TO_CAST_THIS_SPELL_PREFIX)"),
        "{relative} should route generic preprocess line shape checks through token word matching"
    );
    assert!(
        !helper.contains("ClauseShape")
            && !helper.contains("clause_shape")
            && !helper.contains(".matches_words("),
        "{relative} should not route generic preprocess line shape checks through ClauseShape/raw word refs"
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
    assert!(
        helper.contains("word_slice_contains_phrase(&words, ITS_AN_ENCHANTMENT_PHRASE)"),
        "{relative} should classify enchantment parentheticals through token word matching"
    );
    assert!(
        !helper.contains("ITS_AN_ENCHANTMENT_PATTERN")
            && !helper.contains("ClauseShape")
            && !helper.contains(".matches_words("),
        "{relative} should not classify enchantment parentheticals through ClauseShape/raw word refs"
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
    assert!(
        helper.contains(
            "word_slice_starts_with(&parser_token_word_refs(&tokens), AS_LONG_AS_PREFIX)"
        ),
        "{relative} should route borrowed static as-long-as gates through token word matching"
    );
    assert!(
        !helper.contains("AS_LONG_AS_PREFIX_PATTERN")
            && !helper.contains("ClauseShape")
            && !helper.contains(".matches_words("),
        "{relative} should not route borrowed static as-long-as gates through ClauseShape/raw word refs"
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
    assert!(
        helper.contains("word_slice_contains_any_phrase(&parser_token_word_refs(&tokens), VOTE_CHOICE_CLAUSE_PHRASES)"),
        "{relative} should route vote-choice context detection through token word matching"
    );
    assert!(
        !helper.contains("VOTE_CHOICE_CLAUSE_PATTERN")
            && !helper.contains("ClauseShape")
            && !helper.contains(".matches_words("),
        "{relative} should not route vote-choice context detection through ClauseShape/raw word refs"
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
    for required in [
        "let words = parser_token_word_refs(&tokens)",
        "word_slice_contains_word(&words, \"exile\")",
        "word_slice_contains_phrase(&words, UNTIL_THIS_PHRASE)",
    ] {
        assert!(
            helper.contains(required),
            "{relative} should route exile/until preprocess gates through token word matching: missing `{required}`"
        );
    }
    for forbidden in [
        "EXILE_WORD_PATTERN",
        "UNTIL_THIS_PATTERN",
        "ClauseShape",
        "EXILE_WORD_PATTERN.matches_words(&words)",
        "UNTIL_THIS_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not route exile/until preprocess gates through ClauseShape/raw word refs: found `{forbidden}`"
        );
    }
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
    for required in [
        "let words = token_word_refs(&tokens)",
        "word_slice_contains_any_phrase(&words, NEXT_UPKEEP_PHRASES)",
        "word_slice_contains_any_phrase(&words, THAT_TURN_DELAYED_STEP_PHRASES)",
        "word_slice_contains_any_phrase(&words, NEXT_END_STEP_PHRASES)",
        "let ability_words = token_word_refs(&ability_tokens)",
        "word_slice_contains_phrase(&ability_words, YOUR_NEXT_UPKEEP_PHRASE)",
        "word_slice_contains_phrase(&ability_words, YOUR_NEXT_DRAW_STEP_PHRASE)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route delayed trigger postpass gates through token word matching: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "NEXT_UPKEEP_PATTERN",
        "THAT_TURN_DELAYED_STEP_PATTERN",
        "NEXT_END_STEP_PATTERN",
        "YOUR_NEXT_UPKEEP_PATTERN",
        "YOUR_NEXT_DRAW_STEP_PATTERN",
        "NEXT_UPKEEP_PATTERN.matches_words(&words)",
        "THAT_TURN_DELAYED_STEP_PATTERN.matches_words(&words)",
        "NEXT_END_STEP_PATTERN.matches_words(&words)",
        "YOUR_NEXT_UPKEEP_PATTERN.matches_words(&ability_words)",
        "YOUR_NEXT_DRAW_STEP_PATTERN.matches_words(&ability_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route delayed trigger postpass gates through raw word refs: found `{forbidden}`"
        );
    }
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
fn dispatch_entry_direct_shape_gates_use_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_entry.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_contains_any_phrase(&token_words, THAT_OBJECT_POWER_DAMAGE_PHRASES)",
        "word_slice_contains_any_phrase(&token_words, THIS_OBJECT_DAMAGE_TARGET_PHRASES)",
        "TO_THAT_PLAYER_PHRASE",
        "LEARN_WORDS",
        "OUTSIDE_GAME_ART_RATING_PHRASES",
        "word_slice_contains_any_phrase(&clause_words, DEAL_X_DAMAGE_PHRASES)",
        "word_slice_contains_any_phrase(&clause_words, X_LIFE_CHANGE_PHRASES)",
        "NONSEMANTIC_X_CANT_BE_ZERO_PHRASES",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct dispatch-entry shape gates through word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "dispatch_entry_shape_matches_words",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "THAT_OBJECT_POWER_DAMAGE_PATTERN\n        .matches_words(&words)",
        "THAT_OBJECT_POWER_DAMAGE_PATTERN.matches_words(&words)",
        "THIS_OBJECT_DAMAGE_TARGET_PATTERN.matches_words(&words)",
        "TO_THAT_PLAYER_PATTERN.matches_words(&words)",
        "LEARN_WORD_PATTERN\n            .matches_words(&crate::runtime_backend::token_word_refs(&sentence_tokens))",
        "OUTSIDE_GAME_ART_RATING_PATTERN.matches_words(&words)",
        "DEAL_X_DAMAGE_PATTERN.matches_words(&clause_words)",
        "X_LIFE_CHANGE_PATTERN.matches_words(&clause_words)",
        "NONSEMANTIC_X_CANT_BE_ZERO_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct dispatch-entry shape gates through ClauseShape adapters or raw word refs: found `{forbidden}`"
        );
    }
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
            source_contains_required(parser, required),
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
            source_contains_required(shape, required) || source_contains_required(parser, required),
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
            source_contains_required(shape, required) || source_contains_required(parser, required),
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
            source_contains_required(&content, required),
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
fn sentence_shape_predicates_route_direct_sentence_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/sentence_shape_predicates.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_contains_phrase(\n        &crate::runtime_backend::token_word_refs(stripped),\n        SENTENCE_TO_THE_PLAYER_PHRASE,",
        "word_slice_starts_with_any(&sentence_words, SENTENCE_SACRIFICE_COUNTED_PREFIXES)",
        "word_slice_contains_any_phrase(&sentence_words, SENTENCE_DELAYED_LIFECYCLE_PHRASES)",
        "fn parse_it_is_aura_enchantment_sentence_lexed(",
        "word_slice_matching_prefix(&words, SENTENCE_ITS_AN_PREFIXES)",
        "word_slice_starts_with(&words, SENTENCE_IT_IS_AN_PREFIX)",
        "word_slice_starts_with(&tail.word_refs(), SENTENCE_AURA_ENCHANT_CREATURE_PREFIX)",
        "SENTENCE_YOU_CONTROL_PREFIX",
        "word_slice_contains_any_phrase(&tail.word_refs(), SENTENCE_LOSES_ALL_ABILITIES_PHRASES)",
        "parse_it_is_aura_enchantment_sentence_lexed(tokens)",
        "word_slice_starts_with(&sentence_words, SENTENCE_AT_THIS_PREFIX)",
        "word_slice_find_phrase_start(&sentence_words, SENTENCE_END_OF_COMBAT_PREFIX)",
        "SENTENCE_NEXT_WORD",
        "word_slice_contains_word(&sentence_words, SENTENCE_WOULD_WORD)",
        "SENTENCE_TARGET_WORD",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct sentence-shape gates through word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "fn sentence_find_phrase_start_lexed(",
        "fn sentence_shape_matches_words(",
        "_PATTERN",
        "SENTENCE_TO_THE_PLAYER_PATTERN.matches_words(stripped_words)",
        "SENTENCE_SACRIFICE_COUNTED_PREFIX_PATTERN.matches_words(&sentence_words)",
        "SENTENCE_DELAYED_LIFECYCLE_MARKER_PATTERN.matches_words(&sentence_words)",
        "fn parse_it_is_aura_enchantment_sentence(words: &[&str])",
        "SENTENCE_ITS_AN_PREFIX_PATTERN.matches_words(words)",
        "SENTENCE_IT_IS_AN_PREFIX_PATTERN.matches_words(words)",
        "SENTENCE_AURA_ENCHANT_CREATURE_PREFIX_PATTERN.matches_words(tail)",
        "SENTENCE_YOU_CONTROL_PREFIX_PATTERN.matches_words(&tail[5..])",
        "SENTENCE_LOSES_ALL_ABILITIES_PATTERN.matches_words(tail)",
        "parse_it_is_aura_enchantment_sentence(sentence_words.as_slice())",
        "SENTENCE_AT_THIS_PREFIX_PATTERN.matches_words(&sentence_words)",
        "sentence_find_phrase_start(sentence_words.as_slice(), SENTENCE_END_OF_COMBAT_PREFIX_PATTERN)",
        "SENTENCE_NEXT_MARKER_PATTERN.matches_words(&sentence_words[..end_idx])",
        "SENTENCE_WOULD_MARKER_PATTERN.matches_words(&sentence_words)",
        "SENTENCE_TARGET_MARKER_PATTERN.matches_words(&stripped_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct sentence-shape gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn top_cards_hand_remainder_parser_uses_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs";
    let content = read_repo_file(&root, relative);
    let shape = function_source(
        &content,
        "const PUT_COUNTED_TOP_CARDS_OBJECT_PHRASES",
        "pub(crate) fn parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb",
    );
    let parser = function_source(
        &content,
        "pub(crate) fn parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb",
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
            source_contains_required(shape, required) || source_contains_required(parser, required),
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
        "word_slice_contains_word(&clause.word_refs(), \"except\")",
        "word_slice_contains_word(&words, \"until\")",
        "word_slice_ends_with(&words, &[\"leaves\", \"the\", \"battlefield\"])",
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
        "REPLACE_EXILE_ALL_CARDS_FROM_PREFIX_PATTERN",
        "REPLACE_EXCEPT_MARKER_PATTERN",
        "REPLACE_EXCEPT_MARKER_PATTERN.matches_words(&words)",
        "REPLACE_HAND_MARKER_PATTERN.matches_words(&words)",
        "REPLACE_GRAVEYARD_MARKER_PATTERN.matches_words(&words)",
        "REPLACE_HAND_MARKER_PATTERN",
        "REPLACE_GRAVEYARD_MARKER_PATTERN",
        "let zone_words = zones_clause.word_refs()",
        "REPLACE_AND_MARKER_PATTERN",
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
fn etb_where_x_value_binding_uses_clause_shape_gate() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/etb_static_lines.rs";
    let content = read_repo_file(&root, relative);
    let value_binding = function_source(
        &content,
        "pub(crate) fn parse_value_binding_clause",
        "fn parse_where_x_life_gained_this_turn_value",
    );
    let source_stat = function_source(
        &content,
        "pub(crate) fn parse_where_x_source_stat_value",
        "fn parse_enters_with_fallback_counter_value",
    );

    for required in [
        "let clause = LexedClause::new(tokens)",
        "ETB_WHERE_X_IS_PREFIX_PATTERN.matches(clause)",
        "ETB_DEVOTION_VALUE_PATTERN.matches(clause)",
        "ETB_ALL_PLAYERS_HAND_COUNT_VALUE_PATTERN.matches(clause)",
        "ETB_SAME_NAME_AS_TRIGGERING_SPELL_GRAVEYARD_VALUE_PATTERN.matches(tail)",
        "ETB_EXILED_CARD_MANA_VALUE_PATTERN.matches(tail)",
        "ETB_TRIGGERING_SPELL_MANA_VALUE_PATTERN.matches(tail)",
        "ETB_YOUR_HAND_COUNT_VALUE_PATTERN.matches(clause)",
        "ETB_YOUR_PARTY_SIZE_VALUE_PATTERN.matches(clause)",
    ] {
        assert!(
            value_binding.contains(required),
            "{relative} should parse where-X value binding recognizers through token clause shapes: missing `{required}`"
        );
    }
    assert!(
        source_stat.contains("ETB_WHERE_X_IS_PREFIX_PATTERN.matches(clause)")
            && source_stat.contains("let tail_clause = clause.after_words(3)?")
            && source_stat.contains("ETB_MANA_VALUE_TAIL_PATTERN.matches(tail_clause)"),
        "{relative} should gate source-stat where-X value parsing with a token clause shape"
    );
    for forbidden in [
        "ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&words)",
        "ETB_DEVOTION_VALUE_PATTERN.matches_words(&words)",
        "ETB_ALL_PLAYERS_HAND_COUNT_VALUE_PATTERN.matches_words(&words)",
        "ETB_SAME_NAME_AS_TRIGGERING_SPELL_GRAVEYARD_VALUE_PATTERN.matches_words(tail)",
        "ETB_EXILED_CARD_MANA_VALUE_PATTERN.matches_words(tail)",
        "ETB_TRIGGERING_SPELL_MANA_VALUE_PATTERN.matches_words(tail)",
        "ETB_MANA_VALUE_TAIL_PATTERN.matches_words(tail)",
        "ETB_YOUR_HAND_COUNT_VALUE_PATTERN.matches_words(&words)",
        "ETB_YOUR_PARTY_SIZE_VALUE_PATTERN.matches_words(&words)",
        "let word_view = crate::runtime_backend::grammar::primitives::TokenWordView::new(tokens);\n    let words = word_view.word_refs();\n    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !value_binding.contains(forbidden) && !source_stat.contains(forbidden),
            "{relative} should not gate where-X value binding parsers through raw word refs: found `{forbidden}`"
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
        leading_conditional_helper
            .contains("word_slice_starts_with(&parser_token_word_refs(tokens), &[\"if\"])"),
        "{relative} should classify leading conditional self-replacement followups through token word helpers"
    );

    for (helper, required) in [
        (
            "fn tokens_mention_morbid_search_to_battlefield_replacement",
            "token_words_contain_all_phrases(\n        tokens,\n        &[\n            PUT_THAT_CARD_ONTO_BATTLEFIELD_INSTEAD_OF_HAND_PHRASE,\n            CREATURE_DIED_THIS_TURN_PHRASE,\n        ],\n    )",
        ),
        (
            "fn tokens_mention_bargained_return_to_battlefield_replacement",
            "token_words_contain_all_phrases(\n        tokens,\n        &[\n            IF_THIS_SPELL_WAS_BARGAINED_PHRASE,\n            ONE_OF_THOSE_CARDS_MV_FOUR_OR_LESS_PHRASE,\n            ONTO_BATTLEFIELD_INSTEAD_OF_HAND_PHRASE,\n        ],\n    )",
        ),
        (
            "fn tokens_mention_kicked_count_override_replacement",
            "token_words_contain_all_phrases(\n        tokens,\n        &[\n            PUT_TWO_OF_THOSE_CARDS_INTO_YOUR_HAND_INSTEAD_PHRASE,\n            PUT_ONE_OF_THOSE_CARDS_INTO_YOUR_HAND_PHRASE,\n        ],\n    )",
        ),
        (
            "fn tokens_mention_kicked_multi_zone_to_battlefield_followup",
            "token_words_contain_all_phrases(\n        tokens,\n        &[\n            IF_THIS_SPELL_WAS_KICKED_PHRASE,\n            PUT_THOSE_CARDS_ONTO_BATTLEFIELD_INSTEAD_OF_HAND_PHRASE,\n        ],\n    )",
        ),
        (
            "fn tokens_mention_clash_win_top_replacement",
            "token_words_contain_all_phrases(\n        tokens,\n        &[\n            CLASH_WITH_AN_OPPONENT_PHRASE,\n            IF_YOU_WIN_PHRASE,\n            ON_TOP_OF_OWNERS_LIBRARY_INSTEAD_PHRASE,\n        ],\n    )",
        ),
    ] {
        let source = function_source(&content, helper, "\n}\n\n");
        assert!(
            source.contains(required),
            "{relative} should implement {helper} with token word helpers"
        );
        for forbidden in [
            "ClauseShape",
            "LexedClause::new(tokens)",
            ".matches_words(",
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
        block_first_strike_helper.contains(
            "word_slice_starts_with(&words, BLOCKS_OR_BECOMES_BLOCKED_FIRST_STRIKE_PREFIX)"
        ) && block_first_strike_helper.contains(
            "word_slice_ends_with(&words, BLOCKS_OR_BECOMES_BLOCKED_FIRST_STRIKE_SUFFIX)"
        ),
        "{lower_mod_relative} should classify blocks/becomes-blocked first-strike lines through token word matching"
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
        "if keyword_prefix_shape(tokens) == Some(KeywordPrefixShape::Sneak)",
        "if keyword_special_form_shape(tokens) == Some(KeywordSpecialFormShape::BlitzFromGraveyard)",
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
        "enum KeywordPrefixShape",
        "enum KeywordSpecialFormShape",
        "const KEYWORD_PREFIX_PATTERN: LexPattern<'static>",
        "LexPattern::action(",
        "\"keyword\"",
        "LexCaptureKind::OneOf(&[\"surge\", \"freerunning\", \"sneak\", \"exploit\"])",
        "fn keyword_prefix_shape(tokens: &[OwnedLexToken]) -> Option<KeywordPrefixShape>",
        "capture_clause_by_role(LexCaptureRole::Action, clause)",
        "fn keyword_special_form_shape(tokens: &[OwnedLexToken]) -> Option<KeywordSpecialFormShape>",
        "SNEAK_SPELL_FORM_PATTERN.find_in_clause(clause)",
        "SNEAK_PERMANENT_FORM_PATTERN",
        "BLITZ_FROM_GRAVEYARD_MARKER_PATTERN",
        ".find_in_clause(clause)",
        "EXERT_ATTACK_PREFIX_PATTERN.match_prefix(clause)",
        "keyword_prefix_shape(tokens) == Some(KeywordPrefixShape::Surge)",
        "keyword_prefix_shape(tokens) == Some(KeywordPrefixShape::Freerunning)",
        "keyword_prefix_shape(tokens) == Some(KeywordPrefixShape::Sneak)",
        "keyword_prefix_shape(tokens) == Some(KeywordPrefixShape::Exploit)",
        "keyword_special_form_shape(tokens) == Some(KeywordSpecialFormShape::BlitzFromGraveyard)",
        "keyword_special_form_shape(tokens) == Some(KeywordSpecialFormShape::ExertAttack)",
        "full_parse_tokens",
    ] {
        assert!(
            content.contains(required),
            "{relative} should preserve Sneak classification through token-backed captured keyword shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "use super::effect_sentences::clause_pattern_helpers",
        "ClauseShape",
        "clause_shape!",
        "SURGE_KEYWORD_PREFIX_PATTERN",
        "FREERUNNING_KEYWORD_PREFIX_PATTERN",
        "SNEAK_KEYWORD_PREFIX_PATTERN",
        "EXPLOIT_KEYWORD_PREFIX_PATTERN",
        "is_supported_spell_sneak_line(line.info.raw_line.as_str())",
        "fn is_supported_spell_sneak_line(raw_line: &str)",
        "raw_line.to_ascii_lowercase()",
        ".contains(\"you may cast this spell for\")",
        ".contains(\"enters tapped and attacking\")",
        "token_slice_first_is(tokens, \"sneak\")",
        "SNEAK_SPELL_FORM_PATTERN.matches_words(&words)",
        "SNEAK_PERMANENT_FORM_PATTERN.matches_words(&words)",
        "SURGE_KEYWORD_PREFIX_PATTERN.matches_words(&parser_token_word_refs(tokens))",
        "FREERUNNING_KEYWORD_PREFIX_PATTERN.matches_words(&parser_token_word_refs(tokens))",
        "SNEAK_KEYWORD_PREFIX_PATTERN.matches_words",
        "BLITZ_FROM_GRAVEYARD_MARKER_PATTERN.matches_words(&parser_token_word_refs(tokens))",
        "EXPLOIT_KEYWORD_PREFIX_PATTERN.matches_words(&parser_token_word_refs(tokens))",
        "EXERT_ATTACK_PREFIX_PATTERN.matches_words(&words)",
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
        "CHARACTER_SELECT_PREFIX_PATTERN.matches_word_slice(&words)",
        "PARTNER_WITH_PATTERN.matches_word_slice(&words)",
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
        "KRRRIK_BLACK_MANA_LIFE_PAYMENT_STATIC_PATTERN.matches_word_slice(&parse_words)",
        "is_minimum_spell_total_mana_three_line_lexed(parse_tokens)",
        "is_players_cant_pay_life_or_sacrifice_line_lexed(parse_tokens)",
        "BOAST_TWICE_STATIC_PATTERN.matches_word_slice(&parse_words)",
        "is_first_equip_cost_alternative_lowering_line(parse_tokens)",
        "EQUIP_ABILITIES_INSTANT_SPEED_PATTERN.matches_word_slice(&parse_words)",
        "VOTE_ADDITIONAL_TIME_PATTERN.matches_word_slice(&parse_words)",
        "VOTE_ADDITIONAL_VOTE_PATTERN.matches_word_slice(&parse_words)",
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
        skip_helper.contains("CANT_BE_BLOCKED_LINE_PATTERN.matches_word_slice(&words)")
            && skip_helper.contains("THIS_OR_IT_PREFIX_PATTERN.matches_word_slice(&words)")
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
            .contains("SELF_X_COUNTER_ETB_PATTERN.matches_word_slice(&token_word_refs(tokens))"),
        "{relative} should classify variable ETB counter prefixes through token clause shapes"
    );
    let revealed_value_helper = function_source(
        &content,
        "fn revealed_cards_total_mana_value_x_value_tokens",
        "fn single_plus_one_counter_enters_static_chunk",
    );
    assert!(
        revealed_value_helper.contains("REVEALED_CARDS_TOTAL_MANA_VALUE_X_PATTERN")
            && revealed_value_helper.contains("matches_word_slice(&token_word_refs(tokens))")
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
                "FULL_PARTY_CONDITION_PATTERN.matches_word_slice(&token_word_refs(effect_parse_tokens))"
            ),
        "{relative} should detect full-party effect tails from parse tokens"
    );
    let classifier = function_source(
        &content,
        "fn full_parse_tokens_contain_full_party_instead",
        "fn looks_like_combined_spell_and_activation_tax",
    );
    assert!(
        classifier
            .contains("FULL_PARTY_INSTEAD_PATTERN.matches_word_slice(&token_word_refs(tokens))"),
        "{relative} should classify full-party replacement triggers from full parse tokens through clause shapes"
    );
    assert!(
        helper.contains(
            "FULL_PARTY_CONDITION_PATTERN.matches_word_slice(&token_word_refs(effect_parse_tokens))"
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
            && helper.contains("IF_YOU_DO_PATTERN.matches_word_slice(&token_word_refs(tokens))")
            && helper.contains("IF_YOU_DONT_PATTERN.matches_word_slice(&token_word_refs(tokens))"),
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
                "EFFECT_STARTS_IF_PATTERN.matches_word_slice(&token_word_refs(effect_parse_tokens))"
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

#[test]
fn permission_helpers_route_clause_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/permission_helpers.rs";
    let content = read_repo_file(&root, relative);
    let spec_parser = function_source(
        &content,
        "pub(crate) fn parse_permission_clause_spec_lexed",
        "pub(crate) fn parse_unsupported_play_cast_permission_clause_lexed",
    );
    let unsupported_parser = function_source(
        &content,
        "pub(crate) fn parse_unsupported_play_cast_permission_clause_lexed",
        "pub(crate) fn parse_until_end_of_turn_may_play_tagged_clause",
    );
    let additional_land = function_source(
        &content,
        "pub(crate) fn parse_additional_land_plays_clause_lexed",
        "pub(crate) fn parse_cast_spells_as_though_they_had_flash_clause",
    );

    for required in [
        "enum TaggedPermissionTargetSurface",
        "enum UnsupportedPermissionShape",
        "struct AdditionalLandPlayClause<'a>",
        "fn tagged_permission_target_surface(tokens: &[OwnedLexToken])",
        "fn unsupported_permission_shape(tokens: &[OwnedLexToken])",
        "fn parse_additional_land_play_clause(\n    tokens: &[OwnedLexToken],\n) -> Option<AdditionalLandPlayClause<'_>>",
        "const SINGLE_TAGGED_TARGET_PATTERN: LexPattern<'static>",
        "const PLURAL_TAGGED_CARDS_PATTERN: LexPattern<'static>",
        "const ADDITIONAL_LAND_EACH_TURN_PATTERN: LexPattern<'static>",
        "const FOR_AS_LONG_AS_PERMISSION_PATTERN: LexPattern<'static>",
        "const ONCE_EACH_TURN_PERMISSION_PATTERN: LexPattern<'static>",
        "const ADDITIONAL_LAND_PLAY_PATTERN: LexPattern<'static>",
        "LexCaptureKind::OneOfPhrase(&[&[\"it\"], &[\"that\", \"card\"], &[\"that\", \"spell\"]])",
        "LexCaptureKind::OneOfPhrase(&[&[\"those\", \"cards\"]])",
        "LexCaptureKind::UntilAnyPhrase(&[\n                &[\"additional\", \"land\", \"this\", \"turn\"],",
        "LexPattern::tail(\"permission\", LexCaptureKind::Rest)",
        "LexPattern::any_phrase(&[\n            &[\"additional\", \"land\", \"this\", \"turn\"],",
        "SINGLE_TAGGED_TARGET_PATTERN.match_clause(clause)",
        "PLURAL_TAGGED_CARDS_PATTERN.match_clause(clause)",
        "matched.capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "matched.capture_clause_by_role(LexCaptureRole::Amount, clause)",
        "clause_matches_any_phrase(permission_clause, &[&[\"may\", \"play\"], &[\"may\", \"cast\"]])",
        "permission_clause.contains_word(\"graveyard\")",
    ] {
        assert!(
            content.contains(required),
            "{relative} should classify permission surfaces through captured LexPattern shapes: missing `{required}`"
        );
    }

    for required in [
        "let target_surface = tagged_permission_target_surface(target_tokens)",
        "TaggedPermissionTargetSurface::SingleTaggedObject",
        "TaggedPermissionTargetSurface::PluralTaggedCards",
        "match unsupported_permission_shape(tokens)",
        "UnsupportedPermissionShape::AdditionalLandEachTurn",
        "UnsupportedPermissionShape::ForAsLongAsPlayCast",
        "UnsupportedPermissionShape::OnceEachTurnGraveyard",
        "let Some(parsed) = parse_additional_land_play_clause(tokens)",
        "let count_tokens = parsed.count_tokens",
        "let count_words = token_word_refs(count_tokens)",
        "parse_value_from_lexed(count_tokens)",
    ] {
        assert!(
            spec_parser.contains(required)
                || unsupported_parser.contains(required)
                || additional_land.contains(required),
            "{relative} should route permission helper shape gates through lexed clauses: missing `{required}`"
        );
    }

    for forbidden in [
        "SINGLE_TAGGED_PERMISSION_TARGET_PATTERN",
        "PLURAL_TAGGED_PERMISSION_CARDS_TARGET_PATTERN",
        "single_tagged_target",
        "plural_tagged_cards_target",
        "SINGLE_TAGGED_PERMISSION_TARGET_PATTERN.matches_words(&target_words)",
        "PLURAL_TAGGED_PERMISSION_CARDS_TARGET_PATTERN.matches_words(&target_words)",
        "PLAY_ANY_NUMBER_OF_LANDS_EACH_TURN_PATTERN",
        "FOR_AS_LONG_AS_PREFIX_PATTERN",
        "MAY_PLAY_OR_CAST_PERMISSION_PATTERN",
        "ONCE_GRAVEYARD_PERMISSION_PATTERN",
        "PLAY_WORD_PATTERN",
        "ADDITIONAL_LAND_THIS_TURN_TAIL_PATTERN",
        "use super::effect_sentences::clause_pattern_helpers",
        "PLAY_ANY_NUMBER_OF_LANDS_EACH_TURN_PATTERN.matches_words(&clause_refs)",
        "FOR_AS_LONG_AS_PREFIX_PATTERN.matches_words(&clause_refs)",
        "MAY_PLAY_OR_CAST_PERMISSION_PATTERN.matches_words(&clause_refs)",
        "ONCE_GRAVEYARD_PERMISSION_PATTERN.matches_words(&clause_refs)",
        "PLAY_WORD_PATTERN.matches_words(&[first_word])",
        "ADDITIONAL_LAND_THIS_TURN_TAIL_PATTERN.matches_words(tail)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route permission helper shape gates through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn activation_helpers_route_clause_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/activation_helpers.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "fn activation_find_phrase_start(clause: LexedClause<'_>, phrase: &[&str])",
        "word_slice_eq(window, phrase)",
        "fn activation_token_slice_prefix_at_matches_phrase",
        "word_slice_starts_with(&words, prefix)",
        "fn activation_words_contain_all",
        "word_slice_contains_all_words(words, required)",
        "fn activation_words_contain_phrase",
        "word_slice_contains_phrase(words, phrase)",
        "enum AddManaChoiceKind",
        "struct AddManaChoiceClause<'a>",
        "fn parse_add_mana_choice_clause(tokens: &[OwnedLexToken])",
        "const MANA_CHOICE_PATTERN: LexPattern<'static>",
        "LexCaptureKind::OneOfPhrase(&[",
        "&[\"any\", \"one\", \"color\"]",
        "matched.capture_clause_by_role(LexCaptureRole::Object, clause)",
        "matched.capture_clause_by_role(LexCaptureRole::Tail, clause)",
        "activation_words_contain_all(&clause_words, &[\"exiled\", \"colors\"])",
        "activation_words_contain_any(&clause_words, &[\"commander\", \"commanders\"])",
        "activation_words_contain_phrase(&clause_words, &[\"different\", \"colors\"])",
        "activation_find_phrase_start(",
        "CHOSEN_COLOR_PHRASE",
        "FOR_EACH_COLOR_AMONG_PHRASE",
        "ADD_ONE_MANA_OF_THAT_COLOR_PHRASE",
        "MANA_OF_CHOSEN_COLOR_SUFFIXES",
        "ADD_MANA_ONE_THAT_COLOR_PREFIX",
        "ADD_MANA_THAT_COLOR_AMOUNT_PREFIX",
        "if let Some(mana_choice) = parse_add_mana_choice_clause(tokens)",
        "let any_one = mana_choice.kind.any_one()",
        "let any_type = mana_choice.kind.allow_colorless()",
        "let tail_tokens = mana_choice.tail_tokens",
        "FOR_EACH_REMOVED_THIS_WAY_PREFIX",
        "CHOSEN_COLOR_MANA_TAIL_PREFIX",
        "word_slice_eq(&trailing_words, &[\"instead\"])",
        "activation_words_contain_all(&clause_words, &[\"or\"])",
        "MANA_POOL_START_PREFIX",
        "LAND_PRODUCE_SUBJECT_PREFIX",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route activation helper gates through token-backed phrase/word helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "shape.matches_words(window)",
        "shape.matches(",
        ".matches_words(",
        "shape.matches_words(&TokenWordView::new(tail).to_word_refs())",
        "ADD_MANA_IMPRINTED_COLORS_PATTERN",
        "ADD_MANA_COMMANDER_IDENTITY_PATTERN",
        "ADD_MANA_DIFFERENT_COLORS_PATTERN",
        "MANA_OF_CHOSEN_COLOR_PREFIX_PATTERN",
        "ADD_MANA_ONE_THAT_COLOR_PATTERN",
        "ADD_MANA_THAT_COLOR_AMOUNT_PATTERN",
        "ADD_MANA_IMPRINTED_COLORS_PATTERN.matches_words(&clause_words)",
        "ADD_MANA_COMMANDER_IDENTITY_PATTERN.matches_words(&clause_words)",
        "ADD_MANA_DIFFERENT_COLORS_PATTERN.matches_words(&clause_words)",
        "MANA_OF_CHOSEN_COLOR_PREFIX_PATTERN.matches_words(prefix)",
        "ADD_MANA_ONE_THAT_COLOR_PATTERN.matches_words(&clause_words)",
        "ADD_MANA_THAT_COLOR_AMOUNT_PATTERN.matches_words(&clause_words)",
        "ADD_MANA_ANY_ONE_COLOR_OR_TYPE_PATTERN",
        "ADD_MANA_ANY_COLOR_PATTERN",
        "ADD_MANA_ANY_TYPE_PATTERN",
        "COLOR_OR_TYPE_WORD_PATTERN",
        "COLOR_WORD_PATTERN",
        "TYPE_WORD_PATTERN",
        "ADD_MANA_ANY_ONE_COLOR_OR_TYPE_PATTERN.matches_words(&clause_words)",
        "ADD_MANA_ANY_COLOR_PATTERN.matches_words(&clause_words)",
        "ADD_MANA_ANY_TYPE_PATTERN.matches_words(&clause_words)",
        "FOR_EACH_REMOVED_THIS_WAY_PATTERN.matches_words(&tail_words)",
        "CHOSEN_COLOR_MANA_TAIL_PATTERN.matches_words(&trailing_words)",
        "INSTEAD_TAIL_PATTERN.matches_words(&trailing_words)",
        "OR_MARKER_PATTERN.matches_words(&clause_words)",
        "MANA_POOL_START_PATTERN.matches_words(&words)",
        "SIMPLE_MANA_POOL_TAIL_PATTERN.matches_words(&words)",
        "LAND_PRODUCE_SUBJECT_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route activation helper gates through ClauseShape/raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
fn mana_actions_route_direct_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/mana_actions.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_contains_all_words(&clause_words, ADD_MANA_IMPRINTED_COLOR_WORDS)",
        "word_slice_contains_any_word(&clause_words, ADD_MANA_COMMANDER_WORDS)",
        "word_slice_contains_phrase(&clause_words, DIFFERENT_COLORS_PHRASE)",
        "word_slice_starts_with(&clause_words, ADD_MANA_ONE_THAT_COLOR_PREFIX)",
        "word_slice_starts_with(&clause_words, ADD_MANA_THAT_COLOR_AMOUNT_PREFIX)",
        "word_slice_contains_any_phrase(&clause_words, ANY_ONE_COLOR_OR_TYPE_PHRASES)",
        "word_slice_contains_any_phrase(&clause_words, ANY_COLOR_PHRASES)",
        "word_slice_contains_any_phrase(&clause_words, ANY_TYPE_PHRASES)",
        "word_slice_ends_with_any(&prefix_clause, MANA_OF_CHOSEN_COLOR_SUFFIXES)",
        "word_slice_eq_any(",
        "CHOSEN_BY_PLAYER_TAILS",
        "word_slice_starts_with(&tail_words, FOR_EACH_REMOVED_THIS_WAY_PREFIX)",
        "word_slice_eq(\n            &crate::runtime_backend::token_word_refs(window),\n            FOR_EACH_PHRASE,",
        "word_slice_starts_with(&trailing_words, CHOSEN_COLOR_TAIL_PREFIX)",
        "word_slice_eq(&trailing_words, &[INSTEAD_WORD])",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct mana gates through word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "LexedClause",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        ".matches(",
        "ADD_MANA_IMPRINTED_COLORS_PATTERN.matches_words(&clause_words)",
        "ADD_MANA_COMMANDER_IDENTITY_PATTERN.matches_words(&clause_words)",
        "DIFFERENT_COLORS_PATTERN.matches_words(&clause_words)",
        "ADD_MANA_ONE_THAT_COLOR_PATTERN.matches_words(&clause_words)",
        "ADD_MANA_THAT_COLOR_AMOUNT_PATTERN.matches_words(&clause_words)",
        "ANY_ONE_COLOR_OR_TYPE_PATTERN.matches_words(&clause_words)",
        "ANY_COLOR_PATTERN.matches_words(&clause_words)",
        "ANY_TYPE_PATTERN.matches_words(&clause_words)",
        "MANA_OF_CHOSEN_COLOR_PREFIX_PATTERN.matches_words(prefix)",
        "CHOSEN_BY_PLAYER_TAIL_PATTERN.matches_words(&tail_words)",
        "FOR_EACH_REMOVED_THIS_WAY_PATTERN.matches_words(&tail_words)",
        "FOR_EACH_WORD_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(window))",
        "CHOSEN_COLOR_TAIL_PATTERN.matches_words(&trailing_words)",
        "INSTEAD_WORD_PATTERN.matches_words(&trailing_words)",
        "_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct mana gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn clause_dispatch_helpers_route_singleton_shape_probes_through_matches_word() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/clause_dispatch/helpers.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_eq_any(\n        &subject_words,\n        ENCHANTED_CREATURE_CONTROLLER_OR_OWNER_CLAUSES,",
        "word_slice_starts_with(&subject_words, THE_CONTROLLER_OF_PREFIX)",
        "word_slice_starts_with(&subject_words, CONTROLLER_OF_PREFIX)",
        "word_slice_starts_with(&subject_words, THE_OWNER_OF_PREFIX)",
        "word_slice_starts_with(&subject_words, OWNER_OF_PREFIX)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route controller/owner subject gates through reusable word helpers: missing `{required}`"
        );
    }
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "ENCHANTED_CREATURE_CONTROLLER_OR_OWNER_PATTERN.matches(subject_clause)",
        "THE_CONTROLLER_OF_PREFIX_PATTERN.matches(subject_clause)",
        "CONTROLLER_OF_PREFIX_PATTERN.matches(subject_clause)",
        "THE_OWNER_OF_PREFIX_PATTERN.matches(subject_clause)",
        "OWNER_OF_PREFIX_PATTERN.matches(subject_clause)",
        "ENCHANTED_CREATURE_CONTROLLER_OR_OWNER_PATTERN.matches_words(&subject_words)",
        "THE_CONTROLLER_OF_PREFIX_PATTERN.matches_words(&subject_words)",
        "CONTROLLER_OF_PREFIX_PATTERN.matches_words(&subject_words)",
        "THE_OWNER_OF_PREFIX_PATTERN.matches_words(&subject_words)",
        "OWNER_OF_PREFIX_PATTERN.matches_words(&subject_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route controller/owner subject gates through raw word refs: found `{forbidden}`"
        );
    }
    assert!(
        content.contains("become_words.iter().position(|word| *word == \"with\")"),
        "{relative} should route singleton clause-dispatch helper probes through direct word predicates"
    );
    for required in [
        "word_slice_eq_any(window, COUNTER_ON_PRONOUN_PHRASES)",
        "word_slice_contains_phrase(subject_words, BASE_POWER_TOUGHNESS_WORDS)",
        "position(|window| word_slice_eq(window, BASE_POWER_TOUGHNESS_WORDS))",
        "tail.len() != 5 || !word_slice_eq(&tail[..4], BASE_POWER_TOUGHNESS_WORDS)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route helper shape gates through reusable word helpers: missing `{required}`"
        );
    }
    assert!(
        !content.contains("WITH_WORD_PATTERN"),
        "{relative} should not keep singleton clause-dispatch helper probes as one-off shapes"
    );
    assert!(
        !content.contains(".matches_words("),
        "{relative} should not route clause-dispatch helper shape gates through raw word refs"
    );
}

#[test]
fn become_clause_routes_direct_shape_gates_through_word_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/clause_dispatch/become_clause.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_eq(become_words, MONARCH_WORDS)",
        "word_slice_eq_any(&target_subject_words, IT_THEY_THEM_CLAUSES)",
        "word_slice_eq_any(&target_subject_words, THIS_SOURCE_SUBJECT_CLAUSES)",
        "word_slice_eq(become_words, BASIC_LAND_TYPE_CHOICE_WORDS)",
        "word_slice_eq_any(become_words, COLOR_CHOICE_CLAUSES)",
        "word_slice_eq(become_words, CREATURE_TYPE_CHOICE_WORDS)",
        "word_slice_eq(become_words, COLORLESS_WORDS)",
        "word_slice_starts_with(become_words, AURA_ENCHANTMENT_WITH_ENCHANT_CREATURE_PREFIX)",
        "word_slice_starts_with(become_words, AURA_WITH_ENCHANT_CREATURE_PREFIX)",
        "word_slice_eq(become_words, SADDLED_WORDS)",
        "word_slice_starts_with(&aura_tail_clause.word_refs(), YOU_CONTROL_PREFIX)",
        "word_slice_starts_with(become_words, EQUAL_TO_PREFIX)",
        "word_slice_eq_any(&rhs.word_refs(), SOURCE_POWER_TOUGHNESS_CLAUSES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct become-clause shape gates through reusable word helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "_PATTERN",
        ".matches(",
        ".matches_word(",
        ".matches_token(",
        "MONARCH_PATTERN.matches_words(become_words)",
        "IT_THEY_THEM_PATTERN.matches_words(&target_subject_words)",
        "THIS_SOURCE_SUBJECT_PATTERN.matches_words(&target_subject_words)",
        "BASIC_LAND_TYPE_CHOICE_PATTERN.matches_words(become_words)",
        "COLOR_CHOICE_PATTERN.matches_words(become_words)",
        "CREATURE_TYPE_CHOICE_PATTERN.matches_words(become_words)",
        "COLORLESS_PATTERN.matches_words(become_words)",
        "AURA_ENCHANTMENT_WITH_ENCHANT_CREATURE_PREFIX_PATTERN.matches_words(become_words)",
        "AURA_WITH_ENCHANT_CREATURE_PREFIX_PATTERN.matches_words(become_words)",
        "SADDLED_PATTERN.matches_words(become_words)",
        "YOU_CONTROL_PREFIX_PATTERN.matches_words(aura_tail_words)",
        "EQUAL_TO_PREFIX_PATTERN.matches_words(become_words)",
        "SOURCE_POWER_TOUGHNESS_PATTERN.matches_words(rhs)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct become-clause shape gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn subject_verb_special_recognizers_route_direct_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_special_recognizers.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with(&words, UNTIL_END_OF_TURN_PREFIX)",
        "word_slice_starts_with(&words, UNTIL_YOUR_NEXT_TURN_PREFIX)",
        "word_slice_starts_with(&words, UNTIL_END_OF_COMBAT_PREFIX)",
        "word_slice_starts_with(&tail.word_refs(), AND_SO_ON_FOR_PREFIX)",
        "word_slice_eq_any(&words, EMPTY_MANA_POOL_PHRASES)",
        "word_slice_starts_with(&words, THAT_PLAYER_PREFIX)",
        "word_slice_starts_with(&words, TARGET_OPPONENT_PREFIX)",
        "word_slice_starts_with(&words, YOU_PREFIX)",
        "KEYWORD_BUNDLE_IF_IT_HAS_PREFIX",
        "word_slice_eq(&tail.word_refs(), POWER_AND_TOUGHNESS_WORDS)",
        "word_slice_contains_word(&filter_words, TOKEN_WORD)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct subject-verb recognizer shape gates through word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "_PATTERN",
        "UNTIL_END_OF_TURN_PREFIX_PATTERN.matches_words(&words)",
        "UNTIL_YOUR_NEXT_TURN_PREFIX_PATTERN.matches_words(&words)",
        "UNTIL_END_OF_COMBAT_PREFIX_PATTERN.matches_words(&words)",
        "AND_SO_ON_FOR_PREFIX_PATTERN.matches_words(&words[cursor..])",
        "EMPTY_MANA_POOL_PATTERNS.matches_words(&words)",
        "THAT_PLAYER_PREFIX_PATTERN.matches_words(&words)",
        "TARGET_OPPONENT_PREFIX_PATTERN.matches_words(&words)",
        "YOU_PREFIX_PATTERN.matches_words(&words)",
        "KEYWORD_BUNDLE_IF_IT_HAS_PREFIX_PATTERN.matches_words(&words[start + 1..])",
        "POWER_AND_TOUGHNESS_PATTERN.matches_words(&words[subject_end - 3..subject_end])",
        "TOKEN_MARKER_PATTERN.matches_words(&filter_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct subject-verb recognizer shape gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn consult_family_routes_direct_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/consult_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_eq(\n            &crate::runtime_backend::token_word_refs(&consult_tokens[..consult_verb_idx]),\n            &[\"they\"],\n        )",
        "word_slice_starts_with(&consult_prefix_words, CONSULT_TOP_LIBRARY_PREFIX)",
        "word_slice_starts_with(\n            &crate::runtime_backend::token_word_refs(&filter_tokens),\n            CONSULT_THAT_MANY_PREFIX,\n        )",
        "word_slice_eq_any(\n        &crate::runtime_backend::token_word_refs(tokens),\n        CONSULT_THIS_POWER_CLAUSES,\n    )",
        "is_some_and(|word| *word == \"play\")",
        "word_slice_eq(&remainder, CONSULT_THIS_TURN_CLAUSE)",
        "word_slice_eq(&remainder, CONSULT_PAY_LIFE_MANA_VALUE_CLAUSE)",
        "word_slice_contains_any_phrase(\n        &crate::runtime_backend::token_word_refs(tokens),\n        CONSULT_NOT_CAST_THIS_MARKER_PHRASES,\n    )",
        "word_slice_eq_any(\n        &crate::runtime_backend::token_word_refs(tokens),\n        CONSULT_PUT_MATCH_INTO_HAND_CLAUSES,\n    )",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct consult-family shape gates through reusable word helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "CONSULT_THEY_SUBJECT_PATTERN.matches_words(&subject_words)",
        "CONSULT_TOP_LIBRARY_PREFIX_PATTERN.matches_words(&prefix_words)",
        "CONSULT_THAT_MANY_PREFIX_PATTERN.matches_words(&filter_words)",
        "CONSULT_THIS_POWER_PATTERN.matches_words(&word_refs)",
        "CONSULT_PLAY_WORD_PATTERN.matches_words(&[*word])",
        "CONSULT_THIS_TURN_PATTERN.matches_words(&remainder)",
        "CONSULT_PAY_LIFE_MANA_VALUE_PATTERN.matches_words(&remainder)",
        "CONSULT_NOT_CAST_THIS_MARKER_PATTERN.matches_words(&clause_words)",
        "CONSULT_PUT_MATCH_INTO_HAND_PATTERN.matches_words(&clause_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct consult-family shape gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn next_spell_family_routes_clause_gates_through_word_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/next_spell_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with(&clause_words, &[\"when\"])",
        "word_slice_starts_with(after_turn, IT_GAINS_PREFIX)",
        "word_slice_starts_with(after_turn, IT_HAS_PREFIX)",
        "word_slice_starts_with(&clause_words, &[\"the\", \"next\"])",
        "word_slice_ends_with(&subject_clause.word_refs(), THIS_TURN_PHRASE)",
        "word_slice_eq_any(words, CANT_BE_COUNTERED_CLAUSES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route next-spell clause gates through reusable word helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "WHEN_PREFIX_PATTERN.matches(clause)",
        "IT_GAINS_PREFIX_PATTERN.matches(after_turn_clause)",
        "IT_HAS_PREFIX_PATTERN.matches(after_turn_clause)",
        "THE_NEXT_PREFIX_PATTERN.matches(clause)",
        "THIS_TURN_SUFFIX_PATTERN.matches(subject_clause)",
        "CANT_BE_COUNTERED_PATTERN.matches(LexedClause::new(&tokens))",
        "WHEN_PREFIX_PATTERN.matches_words(clause_words)",
        "IT_GAINS_PREFIX_PATTERN.matches_words(after_turn)",
        "IT_HAS_PREFIX_PATTERN.matches_words(after_turn)",
        "THE_NEXT_PREFIX_PATTERN.matches_words(&clause_words)",
        "THIS_TURN_SUFFIX_PATTERN.matches_words(subject_words)",
        "CANT_BE_COUNTERED_PATTERN.matches_words(words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route next-spell clause gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn looked_cards_family_routes_clause_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/looked_cards_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with(&tail.word_refs(), LOOKED_NUMBER_OF_PREFIX)",
        "word_slice_starts_with_any(&tail_clause.word_refs(), LOOKED_CARD_OF_YOUR_LIBRARY_PREFIXES)",
        "word_slice_starts_with(&where_tail.word_refs(), LOOKED_WHERE_X_IS_PREFIX)",
        "word_slice_starts_with_any(&value_word_refs, LOOKED_GREATEST_MANA_VALUE_PREFIXES)",
        "looked_words_start_into_hand(&tail.word_refs())",
        "token_words_non_article_eq_any(trimmed.tokens(), LOOKED_IF_DONT_PUT_AMONG_INTO_HAND_PHRASES)",
        "LOOKED_SAME_NAME_SUFFIXES\n            .iter()\n            .any(|suffix| raw_word_refs.ends_with(suffix))",
        "LOOKED_WITH_THE_CHOSEN_NAME_SUFFIX",
        "token_words_non_article_eq_any(&filter_tokens, LOOKED_CHOSEN_CARD_PHRASES)",
        "LOOKED_CARD_WORDS.contains(word)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route looked-card shape gates through word-slice helpers and reusable phrase constants: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "matches_non_article_tokens",
        "LOOKED_NUMBER_OF_PREFIX_PATTERN.matches_words(&words[idx..])",
        "LOOKED_CARD_OF_YOUR_LIBRARY_PREFIX_PATTERN.matches_words(&tail_words)",
        "LOOKED_WHERE_X_IS_PREFIX_PATTERN.matches_words(&tail_words[4..])",
        "LOOKED_GREATEST_MANA_VALUE_PREFIX_PATTERN\n            .matches_words(&value_word_refs)",
        "LOOKED_INTO_HAND_PATTERN.matches_words(&tail_words[idx..])",
        "LOOKED_INTO_HAND_PATTERN.matches_words(after_from_words)",
        "LOOKED_IF_DONT_PUT_AMONG_INTO_HAND_PATTERN.matches_words(&words)",
        "LOOKED_SAME_NAME_SUFFIX_PATTERN.matches_words(&raw_word_refs)",
        "LOOKED_WITH_THE_CHOSEN_NAME_SUFFIX_PATTERN.matches_words(&raw_word_refs)",
        "LOOKED_CHOSEN_CARD_PATTERN.matches_words(&non_article_words)",
        "LOOKED_CARD_PATTERN.matches_words(&non_article_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route looked-card shape gates through ClauseShape adapters or raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn clause_primitives_routes_direct_shape_gates_through_word_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/clause_primitives.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_contains_any_word(&words, ABILITY_OR_ABILITIES_WORDS)",
        "word_slice_contains_any_word(&words, SPELL_OR_SPELLS_WORDS)",
        "filter_words.is_empty() || word_slice_eq(&filter_words, &[\"any\"])",
        "word_slice_eq(&pre_equal_clause.word_refs(), &[\"damage\"])",
        "word_slice_eq_any(\n            &normalized_target_clause.word_refs(),\n            EACH_PLAYER_TARGET_CLAUSES,",
        "word_slice_eq_any(\n            &normalized_target_clause.word_refs(),\n            EACH_OPPONENT_TARGET_CLAUSES,",
        "word_slice_eq_any(&target_clause.word_refs(), EACH_PLAYER_TARGET_CLAUSES)",
        "word_slice_eq_any(&target_clause.word_refs(), EACH_OPPONENT_TARGET_CLAUSES)",
        "word_slice_eq_any(&target_clause.word_refs(), ITSELF_OR_IT_CLAUSES)",
        "word_slice_eq_any(&right_clause.word_refs(), FIGHT_TAGGED_OTHER_CLAUSES)",
        "word_slice_eq(&tail_words, CLASH_OPPONENT_WORDS)",
        "word_slice_eq(&tail_words, CLASH_TARGET_OPPONENT_WORDS)",
        "word_slice_eq(&tail_words, CLASH_DEFENDING_PLAYER_WORDS)",
        "word_slice_starts_with_any(words, POWER_REF_TWO_WORD_PREFIXES)",
        "word_slice_starts_with_any(words, POWER_REF_THREE_WORD_PREFIXES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct clause-primitive shape gates through reusable word helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "fn clause_primitive_shape_matches_words(",
        "shape.matches(LexedClause::new(&tokens))",
        "clause_primitive_shape_matches_words(words, POWER_REF_TWO_WORD_PATTERN)",
        "clause_primitive_shape_matches_words(words, POWER_REF_THREE_WORD_PATTERN)",
        ".matches_word(",
        ".matches_token(",
        ".matches_non_article_tokens(",
        "ABILITY_OR_ABILITIES_MARKER_PATTERN.matches_words(&words)",
        "SPELL_OR_SPELLS_MARKER_PATTERN.matches_words(&words)",
        "ANY_WORD_PATTERN.matches_words(&filter_words)",
        "DAMAGE_WORD_PATTERN.matches_words(&pre_equal_words)",
        "EACH_PLAYER_TARGET_PATTERN.matches_words(&normalized_target_words)",
        "EACH_OPPONENT_TARGET_PATTERN.matches_words(&normalized_target_words)",
        "EACH_PLAYER_TARGET_PATTERN.matches_words(&target_words)",
        "EACH_OPPONENT_TARGET_PATTERN.matches_words(&target_words)",
        "ITSELF_OR_IT_PATTERN.matches_words(&target_words)",
        "FIGHT_TAGGED_OTHER_PATTERN.matches_words(&right_words)",
        "CLASH_OPPONENT_PATTERN.matches_words(&tail_words)",
        "CLASH_TARGET_OPPONENT_PATTERN.matches_words(&tail_words)",
        "CLASH_DEFENDING_PLAYER_PATTERN.matches_words(&tail_words)",
        "POWER_REF_TWO_WORD_PATTERN.matches_words(words)",
        "POWER_REF_THREE_WORD_PATTERN.matches_words(words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct clause-primitive shape gates through raw word refs: found `{forbidden}`"
        );
    }
    assert!(
        !content.contains(".matches_words("),
        "{relative} should not route clause-primitive shape gates through raw word refs"
    );
}

#[test]
fn exile_actions_route_direct_shape_gates_through_word_helpers() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/exile_actions.rs";
    let content = read_repo_file(&root, relative);
    let resource_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/resource_verbs.rs";
    let resource = read_repo_file(&root, resource_relative);

    for required in [
        "word_slice_eq(&crate::runtime_backend::token_word_refs(attachment_target_tokens), &[\"it\"])",
        "word_slice_contains_phrase(\n            &crate::runtime_backend::token_word_refs(tokens),\n            EXILE_WITH_THAT_NAME_PHRASE,",
        "word_slice_eq(\n            &crate::runtime_backend::token_word_refs(&tokens[end - 2..end]),\n            EXILE_FACE_DOWN_TAIL,",
        "word_slice_starts_with(&words, EXILE_THE_TOP_PREFIX)",
        "word_slice_starts_with_any(\n        &crate::runtime_backend::token_word_refs(&owner_tokens),\n        EXILE_EACH_OPPONENT_LIBRARY_PREFIXES,",
        "const GRAVEYARD_OWNER_PREFIXES: &[OwnerPrefixEntry]",
        "const LIBRARY_OWNER_PREFIXES: &[OwnerPrefixEntry]",
        "fn parse_zone_owner_prefix_lexed(",
        "LexPattern::object(\n            \"owner\",\n            LexCaptureKind::OneOfPhrase(entry.phrases),\n        )",
        "matched.capture_word_range(\"owner\")",
        "consumed_words: owner_range.end",
        "parse_graveyard_owner_prefix_lexed(&tokens)",
        "parse_library_owner_prefix_lexed(&owner_tokens, default_player)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct exile-action shape gates through LexedClause matching: missing `{required}`"
        );
    }
    assert!(
        resource.contains("parse_graveyard_owner_prefix_lexed(tokens)"),
        "{resource_relative} should route reorder graveyard-owner parsing through the token-backed owner prefix parser"
    );

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "_PATTERN",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        ".matches(LexedClause",
        "EXILE_IT_REFERENCE_PATTERN.matches(LexedClause::new(attachment_target_tokens))",
        "EXILE_WITH_THAT_NAME_PATTERN.matches(LexedClause::new(tokens))",
        "EXILE_FACE_DOWN_TAIL_PATTERN.matches(LexedClause::new(&tokens[end - 2..end]))",
        "EXILE_THE_TOP_PREFIX_PATTERN.matches(LexedClause::new(&tokens))",
        "EXILE_EACH_OPPONENT_LIBRARY_PATTERN.matches(LexedClause::new(&owner_tokens))",
        "EXILE_IT_REFERENCE_PATTERN.matches_words(&attachment_target_words)",
        "EXILE_WITH_THAT_NAME_PATTERN.matches_words(&clause_words)",
        "EXILE_FACE_DOWN_TAIL_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(\n            &tokens[end - 2..end],\n        ))",
        "EXILE_THE_TOP_PREFIX_PATTERN.matches_words(&words)",
        "EXILE_EACH_OPPONENT_LIBRARY_PATTERN.matches_words(&owner_words)",
        "EXILE_GRAVEYARD_OWNER_YOU_PATTERN",
        "EXILE_LIBRARY_OWNER_DEFAULT_PATTERN",
        "LexPattern::any_phrase(entry.phrases)",
        "fn parse_library_owner_prefix(\n    words: &[&str],",
        "matches_words(words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct exile-action shape gates through raw word refs: found `{forbidden}`"
        );
    }
    assert!(
        !resource.contains("parse_graveyard_owner_prefix(&clause_words)"),
        "{resource_relative} should not route reorder graveyard-owner parsing through raw word refs"
    );
}

#[test]
fn resource_look_owner_prefixes_use_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/resource_verbs.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "const LOOK_HAND_OWNER_PREFIXES: &[LookZoneOwnerEntry]",
        "const LOOK_LIBRARY_OWNER_PREFIXES: &[LookZoneOwnerEntry]",
        "fn parse_look_zone_owner_lexed(",
        "LexPattern::object(\n            \"owner\",\n            LexCaptureKind::OneOfPhrase(entry.phrases),\n        )",
        "matched.capture_word_range(\"owner\")",
        "parse_look_zone_owner_lexed(&hand_tokens, LOOK_HAND_OWNER_PREFIXES)",
        "parse_look_zone_owner_lexed(owner_tokens, LOOK_LIBRARY_OWNER_PREFIXES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route look owner prefixes through LexPattern captures: missing `{required}`"
        );
    }

    for forbidden in [
        "fn parse_hand_owner(words: &[&str])",
        "fn parse_library_owner(words: &[&str])",
        "LOOK_YOUR_HAND_PATTERN",
        "LOOK_EACH_PLAYER_HAND_PATTERN",
        "LOOK_THEIR_HAND_PATTERN",
        "LOOK_THAT_PLAYER_HAND_PATTERN",
        "LOOK_TARGET_PLAYER_HAND_PATTERN",
        "LOOK_TARGET_OPPONENT_HAND_PATTERN",
        "LOOK_OPPONENT_HAND_PATTERN",
        "LOOK_HIS_OR_HER_HAND_PATTERN",
        "LOOK_YOUR_LIBRARY_PATTERN",
        "LOOK_EACH_PLAYER_LIBRARY_PATTERN",
        "LOOK_THEIR_LIBRARY_PATTERN",
        "LOOK_THAT_PLAYER_LIBRARY_PATTERN",
        "LOOK_TARGET_PLAYER_LIBRARY_PATTERN",
        "LOOK_TARGET_OPPONENT_LIBRARY_PATTERN",
        "LOOK_ITS_OWNER_LIBRARY_PATTERN",
        "LOOK_HIS_OR_HER_LIBRARY_PATTERN",
        "parse_hand_owner(&hand_words)",
        "parse_library_owner(&owner_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not parse look owner prefixes through one-off raw word ladders: found `{forbidden}`"
        );
    }
}

#[test]
fn resource_simple_verb_shape_gates_use_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/resource_verbs.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_eq(&words, NOTE_YOUR_LIFE_TOTAL_WORDS)",
        "word_slice_eq(&words, TAKE_EXTRA_TURN_AFTER_THIS_ONE_WORDS)",
        "PROLIFERATE_TRAILING_OK_PHRASES",
        "word_slice_eq(tail, NTH_FROM_TOP_DESTINATION_TAIL_WORDS)",
        "THAT_LIBRARY_AMOUNT_TAIL_WORDS",
        "word_slice_eq(&clause_words, RESOURCE_PLAY_THOSE_EXILED_WORDS)",
        "RESOURCE_AS_YOU_CHOOSE_WORDS",
        "RESOURCE_IT_OR_THEM_WORDS",
        "word_slice_starts_with(&tail[1..], RESOURCE_CHOSEN_NAME_TAIL_PREFIX)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route simple resource verb shape gates through word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "_PATTERN",
        "NOTE_YOUR_LIFE_TOTAL_PATTERN.matches_words(&words)",
        "TAKE_EXTRA_TURN_AFTER_THIS_ONE_PATTERN.matches_words(&words)",
        "PROLIFERATE_TRAILING_OK_PATTERN.matches_words(&trailing_words)",
        "NTH_FROM_TOP_DESTINATION_TAIL_PATTERN.matches_words(tail)",
        "THAT_LIBRARY_AMOUNT_TAIL_PATTERN.matches_words(&amount_words[used + 1..])",
        "RESOURCE_PLAY_THOSE_EXILED_PATTERN.matches_words(&clause_words)",
        "RESOURCE_AS_YOU_CHOOSE_PATTERN.matches_words(rest)",
        "RESOURCE_IT_OR_THEM_PATTERN.matches_words(&target_words)",
        "RESOURCE_CHOSEN_NAME_TAIL_PATTERN.matches_words(&tail[1..])",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route simple resource verb shape gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn resource_shuffle_phrase_families_use_lex_pattern_captures() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/resource_verbs.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "const SHUFFLE_LIBRARY_DESTINATION_PREFIXES: &[ResourceLibraryDestinationEntry]",
        "fn parse_library_destination_player_lexed(",
        "LexPattern::object(\n                \"destination\",\n                LexCaptureKind::OneOfPhrase(entry.phrases),\n            )",
        "matched.capture_word_range(\"destination\")",
        "fn is_tagged_shuffle_target_lexed(",
        "LexCaptureKind::OneOfPhrase(SHUFFLE_TAGGED_TARGET_PHRASES)",
        "fn is_supported_shuffle_source_tail_lexed(",
        "LexCaptureKind::OneOfPhrase(SUPPORTED_SHUFFLE_SOURCE_TAIL_PHRASES)",
        "is_simple_library_phrase_lexed(tokens)",
        "parse_library_destination_player_lexed(&destination_tokens, player)",
        "is_supported_shuffle_source_tail_lexed(&destination_tokens[trailing_token_idx..])",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route shuffle phrase families through LexPattern captures: missing `{required}`"
        );
    }

    for forbidden in [
        "enum LibraryDestinationPlayer",
        "const LIBRARY_DESTINATION_PLAYER_PHRASES",
        "const SUPPORTED_SHUFFLE_SOURCE_TAILS",
        "fn parse_library_destination_player(\n        words: &[&str]",
        "words.starts_with(phrase)",
        ".any(|tail| *tail == words)",
        ".any(|(phrase, _)| *phrase == words)",
        "matches!(\n            target_words,",
        "non_article_word_refs(&clause_words[into_idx + 1..])",
        "RESOURCE_THE_REST_PREFIX_PATTERN\n            .matches_words(&target_words)",
        "RESOURCE_ALL_OTHER_REVEALED_OR_EXILED_CARDS_PATTERN.matches_words(&target_words)",
        "RESOURCE_ITS_OWNER_LIBRARY_TARGET_PATTERN.matches_words(&clause_words)",
        "RESOURCE_UNSUPPORTED_SHUFFLE_MARKER_PATTERN.matches_words(&clause_words)",
        "RESOURCE_THE_REST_PREFIX_PATTERN",
        "RESOURCE_ALL_OTHER_REVEALED_OR_EXILED_CARDS_PATTERN",
        "RESOURCE_ITS_OWNER_LIBRARY_TARGET_PATTERN",
        "RESOURCE_UNSUPPORTED_SHUFFLE_MARKER_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not parse shuffle phrase families through raw word tables: found `{forbidden}`"
        );
    }
}

#[test]
fn control_copy_attach_verbs_route_shape_gates_through_word_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/control_copy_attach_verbs.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "fn cca_destination_player_from_tokens(",
        "const CCA_REST_TARGET_PHRASES: &[&[&str]]",
        "fn cca_token_is(",
        "fn cca_tokens_contain_all(",
        "fn cca_tokens_contain_phrase(",
        "word_slice_contains_word(&words, CCA_YOUR_WORD)",
        "word_slice_starts_with_any(&words, CCA_THAT_PLAYER_PREFIXES)",
        "word_slice_eq(&clause_words, CCA_THE_GAME_WORDS)",
        "cca_tokens_contain_any_phrase(tokens, CCA_ITS_SOURCE_STAT_PHRASES)",
        "cca_tokens_contain_phrase(tokens, CCA_FOR_AS_LONG_AS_PHRASE)",
        "cca_words_contain_all(&clause_words, CCA_BACK_ANY_ORDER_WORDS)",
        "cca_words_contain_all(&clause_words, CCA_REST_TOP_BOTTOM_LIBRARY_WORDS)",
        "token_slice_starts_with(&tokens[idx..], CCA_FACE_DOWN_PREFIX)",
        "token_slice_starts_with(\n                &destination_tail[from_idx + 1..],\n                CCA_COMMAND_ZONE_TAIL_PREFIX,\n            )",
        "token_slice_starts_with(&destination_tail, CCA_ATTACHED_TO_PREFIX)",
        "cca_tokens_contain_phrase(&target_tokens[1..], CCA_FROM_IT_PHRASE)",
        "cca_tokens_contain_all(tokens, CCA_AMONG_THEM_WORDS)",
        "contains_word(tokens, CCA_STICKER_WORD)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route control/copy/attach verb shape gates through word and token helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "_PATTERN",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        ".matches(LexedClause",
        "fn cca_destination_player_from_words(",
        "CCA_YOUR_MARKER_PATTERN",
        "CCA_THEIR_MARKER_PATTERN",
        "CCA_THAT_PLAYER_PREFIX_PATTERN",
        "CCA_THE_GAME_PATTERN",
        "CCA_ITS_SOURCE_STAT_MARKER_PATTERN",
        "CCA_FOR_AS_LONG_AS_MARKER_PATTERN",
        "CCA_FACE_DOWN_PREFIX_PATTERN",
        "CCA_COMMAND_ZONE_TAIL_PATTERN",
        "CCA_ATTACHED_TO_PREFIX_PATTERN",
        "CCA_STICKER_MARKER_PATTERN",
        "CCA_BACK_ANY_ORDER_MARKER_PATTERN.matches_words(&clause_words)",
        "CCA_FROM_AMONG_HAND_MARKER_PATTERN.matches_words(&clause_words)",
        "CCA_REST_TOP_BOTTOM_LIBRARY_MARKER_PATTERN.matches_words(&clause_words)",
        "CCA_AND_OR_THEN_WORD_PATTERN.matches_words(&clause_words)",
        "CCA_FACE_DOWN_PREFIX_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(&tokens[idx..]))",
        "CCA_UNDER_YOUR_CONTROL_PATTERN.matches_words(&controller_words)",
        "CCA_OWNER_CONTROL_TAIL_PATTERN.matches_words(&destination_tail_words)",
        "CCA_STICKER_MARKER_PATTERN.matches_words(&clause_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep retired raw control/copy/attach shape gate `{forbidden}`"
        );
    }
}

#[test]
fn counter_stat_verbs_route_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/counter_stat_verbs.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "counter_words_start_with_any_at(&clause_words, idx, COUNTER_YOU_CONTROL_PREFIXES)",
        "counter_words_contain_any(&words, COUNTER_ABILITY_MARKER_WORDS)",
        "word_slice_starts_with(&filter_words, FOR_EACH_PREFIX)",
        "counter_words_contain_all(&words, REVEAL_FROM_AMONG_WORDS)",
        "word_slice_eq_any(&words, REVEAL_FULL_HAND_PHRASES)",
        "word_slice_starts_with(tail, PARTY_SIZE_EQUAL_TO_PREFIX)",
        "word_slice_eq_any(&words, EXPLICIT_TOP_CARD_PHRASES)",
        "counter_words_start_with_any(&after_count_words, TOP_LIBRARY_TAIL_PREFIXES)",
        "word_slice_starts_with(&words, WHERE_X_IS_PREFIX)",
        "word_slice_starts_with(&words[idx..], NUMBER_OF_PREFIX)",
        "word_slice_contains_phrase(object_words, THIS_WAY_PHRASE)",
        "word_slice_contains_word(object_words, CHOSEN_WORD)",
        "word_slice_eq(&clause_words, THAT_MUCH_LIFE_WORDS)",
        "word_slice_starts_with(&clause_words, LIFE_EQUAL_TO_PREFIX)",
        "word_slice_starts_with(&words, COUNTER_FOR_EACH_PREFIX)",
        "word_slice_contains_phrase(&clause_words, ROUNDED_DOWN_PHRASE)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route counter/stat verb gates through token-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "LexedClause",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        ".matches(LexedClause::new",
        "matched_prefix_len",
        "find_exact_window",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep retired counter/stat shape gate `{forbidden}`"
        );
    }
}

#[test]
fn zone_move_verbs_route_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/zone_move_verbs.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "fn token_is_word(token: &OwnedLexToken, expected: &str) -> bool",
        "fn token_is_any_word(token: &OwnedLexToken, expected: &[&str]) -> bool",
        "fn token_words(tokens: &[OwnedLexToken]) -> Vec<&str>",
        "word_slice_eq(&token_words(&trailing), ZONE_MOVE_MINUS_ONE_WORDS)",
        "word_slice_eq(&token_words(&trailing), ZONE_MOVE_PLUS_ONE_WORDS)",
        "word_slice_contains_phrase(&token_words(&trailing), ZONE_MOVE_FOR_EACH_PHRASE)",
        "token_is_any_word(token, ZONE_MOVE_CARD_OR_CARDS_WORDS)",
        "word_slice_starts_with(\n                &crate::runtime_backend::token_word_refs(&rounded_tail_tokens),\n                ZONE_MOVE_ROUNDED_DOWN_PREFIX,",
        "word_slice_eq(&tail_words, DRAW_TRAILING_INSTEAD_WORDS)",
        "word_slice_starts_with(&tail_words, DRAW_TRAILING_THEN_PUT_PREFIX)",
        "word_slice_starts_with_any(&words, DRAW_AS_MANY_CARDS_AS_PREFIXES)",
        "word_slice_contains_phrase(&words, ZONE_MOVE_THIS_WAY_PHRASE)",
        "word_slice_starts_with(&token_words, DRAW_EQUAL_TO_PREFIX)",
        "word_slice_starts_with(&value_words, stat_words)",
        "word_slice_eq_any(&clause_words, COUNTER_TARGET_SECOND_SPELL_THIS_TURN_PHRASES)",
        "COUNTER_DYNAMIC_PAYMENT_TAIL_WORDS.contains(word)",
        "word_slice_contains_any_phrase(\n                    &trailing_words,\n                    COUNTER_SAME_NAME_AS_SPELL_PHRASES,",
        "word_slice_contains_any_phrase(\n                        &where_words,\n                        COUNTER_SAME_NAME_AS_SPELL_PHRASES,",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route zone-move verb shape gates through word-slice helpers and token-word helpers: missing `{required}`"
        );
    }

    assert!(
        !content.contains(".matches_words("),
        "{relative} should not route zone-move verb shape gates through raw word refs"
    );

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "LexedClause",
        ".matches_word(",
        ".matches_token(",
        ".matches(",
        "ZONE_MOVE_MINUS_ONE_PATTERN.matches_words(&trailing_words)",
        "ZONE_MOVE_CARD_OR_CARDS_WORD_PATTERN.matches_words(&[card_word])",
        "ZONE_MOVE_ROUNDED_DOWN_PREFIX_PATTERN.matches_words(&words[idx + 1..])",
        "DRAW_TRAILING_INSTEAD_PATTERN.matches_words(&tail_words)",
        "DRAW_AS_MANY_CARDS_AS_PREFIX_PATTERN.matches_words(&clause_words)",
        "DRAW_EQUAL_TO_PREFIX_PATTERN.matches_words(&token_words)",
        "COUNTER_TARGET_SECOND_SPELL_THIS_TURN_PATTERN.matches_words(&clause_words)",
        "COUNTER_DYNAMIC_PAYMENT_TAIL_WORD_PATTERN.matches_words(&[*word])",
        "COUNTER_SAME_NAME_AS_SPELL_PATTERN.matches_words(&trailing_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep retired raw zone-move shape gate `{forbidden}`"
        );
    }
}

#[test]
fn combat_verbs_route_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/combat_verbs.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_eq_any(&object_words, ATTACH_TAGGED_OBJECT_WORDS)",
        "word_slice_eq_any(&object_words, ATTACH_TAGGED_EQUIPMENT_WORDS)",
        "word_slice_eq(&object_words, ATTACH_IT_WORDS)",
        "word_slice_starts_with(&clause_words, DAMAGE_EACH_OPPONENT_HAND_SIZE_PREFIX)",
        "word_slice_eq(\n                &crate::runtime_backend::token_word_refs(&tokens[equal_token_idx + 2..]),\n                COMBAT_THE_RESULT_WORDS,",
        "word_slice_eq_any(&target_words, COMBAT_EACH_PLAYER_TARGET_WORDS)",
        "word_slice_eq_any(&target_words, COMBAT_EACH_OTHER_OPPONENT_TARGET_WORDS)",
        "word_slice_contains_phrase(&target_words, COMBAT_THIS_WAY_PHRASE)",
        "word_slice_contains_phrase(&normalized_target_words, COMBAT_MAX_SPEED_PHRASE)",
        "word_slice_contains_any_phrase(&target_words, COMBAT_ITERATED_PLAYER_CONTROL_PHRASES)",
        "word_slice_eq_any(\n                &crate::runtime_backend::token_word_refs(&target_tokens[at_idx..]),\n                COMBAT_END_OF_COMBAT_TIMINGS,",
        "word_slice_ends_with_any(\n        &crate::runtime_backend::token_word_refs(&filter_tokens),\n        COMBAT_WITH_DIFFERENT_POWER_SUFFIXES,",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route combat verb gates through direct token word-slice predicates: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape!",
        "LexedClause",
        "fn combat_shape_matches_words(",
        "fn combat_find_exact_window(",
        "fn combat_words_start_with_shape(",
        "_PATTERN",
        ".matches(",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "find_window_by(words, width, |window| shape.matches_words(window))",
        "ATTACH_TAGGED_OBJECT_PATTERN.matches_words(&object_words)",
        "DAMAGE_EACH_OPPONENT_HAND_SIZE_PATTERN.matches_words(&clause_words)",
        "COMBAT_THE_RESULT_PATTERN.matches_words(&tail_words)",
        "COMBAT_EACH_PLAYER_TARGET_PATTERN.matches_words(&target_words)",
        "COMBAT_THIS_WAY_MARKER_PATTERN.matches_words(&target_words)",
        "COMBAT_END_OF_COMBAT_TIMING_PATTERN.matches_words(&timing_words)",
        "COMBAT_WITH_DIFFERENT_POWER_SUFFIX_PATTERN.matches_words(&filter_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep retired combat shape gate `{forbidden}`"
        );
    }
}

#[test]
fn verb_handlers_do_not_use_raw_clause_shape_word_matching() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers";
    let dir = root.join(relative);
    let mut files = Vec::new();
    collect_rust_files(&dir, &mut files);

    for file in files {
        let content = fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", file.display()));
        assert!(
            !content.contains(".matches_words("),
            "{} should route verb-handler shape gates through LexedClause/token matching",
            repo_relative(&root, &file)
        );
    }
}

#[test]
fn creation_handlers_route_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/creation_handlers.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "create_words_contains_any_phrase(&words, CREATE_CARD_TYPES_AMONG_PHRASES)",
        "create_words_contains_any_word(modifier_words, CREATE_LOSE_OR_LOSES_WORDS)",
        "create_find_phrase(modifier_words, CREATE_TOKEN_GETS_FOR_EACH_WORDS)",
        "create_words_contains_any_phrase(tail_words, CREATE_BEGINNING_NEXT_END_STEP_PHRASES)",
        "create_words_starts_with_any(&source_words[idx..], CREATE_INLINE_MODIFIER_START_PREFIXES)",
        "create_find_any_word(&remaining_words, CREATE_TOKEN_OR_TOKENS_WORDS)",
        "create_words_contains_any_word(&tail_words, CREATE_COPY_OR_COPIES_MARKER_WORDS)",
        "create_words_contains_any_phrase(&tail_words, CREATE_UNBLOCKABLE_RULES_PHRASES)",
        "create_words_eq_any(reference, CREATE_SOURCE_COUNTER_REFERENCE_PHRASES)",
        "create_words_contains_any_word(&clause_words, CREATE_SPELL_OR_SPELLS_WORDS)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route create/investigate shape gates through token-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "create_shape_matches_words",
        "create_find_phrase_shape",
        "synthetic_word_tokens",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route creation shape gates through retired shape helpers: found `{forbidden}`"
        );
    }
}

#[test]
fn bundle_rules_route_shape_gates_through_word_slice_helpers() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/bundle_rules.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with(&first_words, BUNDLE_MAY_CAST_SPELL_WITH_PREFIX)",
        "word_slice_eq(&sentence_word_refs, EMPTY_LABORATORY_WORDS)",
        "word_slice_eq(&sentence_words, THASSAS_ORACLE_BUNDLE_WORDS)",
        "word_slice_contains_all_words(&words, SOURCE_LEAVES_RETURN_FOLLOWUP_REQUIRED_WORDS)",
        "word_slice_starts_with_any(&second_words, FOR_EACH_OF_THOSE_PREFIXES)",
        "word_slice_find_phrase_start(&first_words, LIFE_BID_CONTROL_OF_PREFIX)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route bundle shape gates through word-slice helpers: missing `{required}`"
        );
    }

    assert!(
        !content.contains(".matches_words("),
        "{relative} should not route bundle shape gates through raw word refs"
    );
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "bundle_shape_matches_words",
        "bundle_find_word",
        "bundle_find_phrase_start",
        "synthetic_word_tokens",
        "LexedClause",
        ".matches_word(",
        ".matches_token(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep retired bundle shape gate `{forbidden}`"
        );
    }
}

#[test]
fn clause_dispatch_routes_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/clause_dispatch.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with(&clause_words, CAST_ANY_NUMBER_OF_SPELLS_PREFIX)",
        "word_slice_contains_phrase(&clause_words, FROM_AMONG_NONLAND_EXILED_THIS_WAY_PHRASE)",
        "word_slice_starts_with(&clause_words, ALL_ABILITIES_AND_PREFIX)",
        "word_slice_eq(&clause_words, RING_TEMPTS_YOU_WORDS)",
        "word_slice_eq_any(&normalized_subject_words, PRONOUN_TAGGED_SUBJECT_PHRASES)",
        "word_slice_eq_any(attached_subject_words, EQUIPPED_OBJECT_SUBJECT_PHRASES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route dispatch gates through token-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape!",
        ".matches_word(",
        ".matches_token(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route migrated dispatch gates through old shape helpers: found `{forbidden}`"
        );
    }
}

#[test]
fn dispatch_entry_routes_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_entry.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with_any(&words, DESTROYED_THIS_WAY_SUBJECT_PREFIXES)",
        "words.ends_with(BE_REGENERATED_SUFFIX)",
        "CANT_WORDS.contains(word)",
        "word_slice_contains_phrase(&words, CANT_BE_REGENERATED_SPLIT_PHRASE)",
        "word_slice_eq_any(&words, SIMPLE_CANT_BE_REGENERATED_PHRASES)",
        "word_slice_eq_any(&words, CANT_BE_REGENERATED_THIS_TURN_PHRASES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route dispatch-entry regeneration gates through word-slice helpers: missing `{required}`"
        );
    }

    assert!(
        !content.contains(".matches_words("),
        "{relative} should not route dispatch-entry shape gates through raw word refs"
    );
    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains("dispatch_entry_shape_matches_words"),
        "{relative} should not keep dispatch-entry ClauseShape adapters"
    );
}

#[test]
fn generic_subject_verb_pairs_route_shape_gates_through_word_slice_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/sequence_rules/generic_subject_verb_sequences/pairs.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with(&first_words, YOU_MAY_CAST_TARGET_PREFIX)",
        "word_slice_contains_phrase(&first_words, WITHOUT_PAYING_ITS_MANA_COST_PHRASE)",
        "word_slice_contains_phrase(after_revealed, PUT_MATCHING_INTO_HAND_PHRASE)",
        "word_slice_starts_with_any(&second_words, PUT_MATCHED_CARD_INTO_HAND_PREFIXES)",
        "word_slice_contains_any_phrase(&second_words, OTHER_REVEALED_CARD_PHRASES)",
        "word_slice_starts_with_any(&second_words, PUT_MATCHED_CARD_ONTO_BATTLEFIELD_PREFIXES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route pair sequence shape gates through word-slice helpers: missing `{required}`"
        );
    }

    assert!(
        !content.contains(".matches_words("),
        "{relative} should not route pair sequence shape gates through raw word refs"
    );
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "pairs_shape_matches_words",
        "pairs_find_word",
        "pairs_find_phrase_start",
        "synthetic_word_tokens",
        ".matches_word(",
        ".matches_token(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep retired pair sequence shape gate `{forbidden}`"
        );
    }
}

#[test]
fn generic_subject_verb_triples_route_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/sequence_rules/generic_subject_verb_sequences/triples.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with_any(&third_words, PUT_MATCH_INTO_HAND_PREFIXES)",
        "triple_revealed_pick_into_hand(after_from_words)",
        "triple_revealed_pick_on_top_library(after_from_words)",
        "triple_words_start_put_or_puts_and_contain_all(",
        "word_slice_eq_any(&third_words, THEN_THAT_PLAYER_SHUFFLES_WORDS)",
        "word_slice_contains_any_phrase(&content_words, OTHER_SEARCH_RESULT_GRAVEYARD_PHRASES)",
        "IF_CAST_NON_HAND_PUT_EACH_LOOKED_INTO_HAND_INSTEAD_WORDS",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route triple sequence gates through direct word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "fn triples_shape_matches_words(",
        "fn triple_find_prefix_shape_start(",
        "synthetic_word_tokens",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "LexedClause",
        "_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route triple sequence gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn delayed_step_family_routes_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/delayed_step_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "LexPattern::role_capture(\n        \"subject\",\n        LexCaptureRole::Subject,",
        "word_slice_eq_any(&lose_words, DELAYED_LOSE_GAME_UNLESS_PAID_WORDS)",
        "word_slice_eq(prefix, DELAYED_PLAYER_PREFIX_YOU)",
        "word_slice_starts_with_any(&rest_words, DELAYED_NEGATED_BE_PREFIXES)",
        "word_slice_eq_any(tail, DELAYED_CREATURE_TYPES_EOT_TAILS)",
        "word_slice_starts_with(body_words, DELAYED_LOSE_DRAW_CLASH_PREFIX)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should keep capture LexPatterns while routing boolean delayed-step gates through direct word-slice predicates: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape!",
        "fn delayed_step_shape_matches_words(",
        "fn delayed_find_phrase_start(",
        "synthetic_word_tokens",
        ".matches_words(",
        ".matches_word(",
        ".matches_token(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route delayed-step boolean gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn lex_chain_helpers_route_shape_gates_through_word_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/lex_chain_helpers.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "struct VerbShapeEntry {\n    words: &'static [&'static str],\n    verb: Verb,\n}",
        "fn words_contain_all(",
        "fn words_contain_any(",
        "fn find_word_matching_any(",
        "word_slice_contains_any_phrase(words, INLINE_TOKEN_RULES_CONTEXT_PHRASES)",
        "words_contain_any(&current_words, CURRENT_CARD_TYPE_LIST_MARKER_WORDS)",
        "word_slice_starts_with(words, PREVENT_NEXT_DAMAGE_CORE_PREFIX)",
        "word_slice_eq_any(&token_words, REPEAT_THIS_PROCESS_PHRASES)",
        "word_slice_starts_with(&after_words, RETURN_WITH_COUNTER_FOLLOWUP_PREFIX)",
        "word_slice_contains_phrase(&before_words, AT_THE_PHRASE)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route lex-chain shape gates through word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "synthetic_word_tokens",
        "LexedClause",
        "_PATTERN",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        ".matches(LexedClause",
        "fn lex_chain_shape_matches_words(",
        "token_word_matches_shape(",
        "find_word_matching_shape(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route lex-chain shape gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn labeled_prefixes_route_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/labeled_prefixes.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with(sentence_words.as_slice(), LABELED_ROUND_UP_EACH_TIME_PREFIX)",
        "labeled_cast_from_among_free(sentence_words.as_slice())",
        "word_slice_contains_word(sentence_words.as_slice(), LABELED_UNLESS_WORD)",
        "word_slice_starts_with(&clause_words, LABELED_THE_NEXT_PREFIX)",
        "labeled_contains_any_word(ability_words, LABELED_SIMPLE_ABILITY_WORDS)",
        "word_slice_starts_with_any(filtered, LABELED_TOKEN_SACRIFICE_PREFIXES)",
        "word_slice_starts_with_any(filtered, LABELED_DELAYED_END_STEP_SACRIFICE_PREFIXES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route labeled-prefix shape gates through word-slice helpers and reusable constants: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "fn labeled_shape_matches_words(",
        "synthetic_word_tokens",
        "_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route labeled-prefix shape gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn search_library_routes_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/search_library.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_contains_phrase(&token_word_refs(tokens), SEARCH_THIS_TURN_PHRASE)",
        "word_slice_starts_with_any(&subject_words, SEARCH_EACH_PLAYER_PREFIXES)",
        "word_slice_eq_any(&owner_words, SEARCH_TARGET_OR_YOUR_OWNER_PHRASES)",
        "word_slice_contains_phrase(&token_words, SEARCH_EXILED_WITH_THIS_PHRASE)",
        "words[0] != SEARCH_ENCHANT_WORD",
        "word_slice_find_word_where(&clause_words",
        "word_slice_contains_any_phrase(&token_words, SEARCH_PUT_INTO_GRAVEYARD_THIS_WAY_PHRASES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route search-library gates through direct word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "fn search_shape_matches_words(",
        "synthetic_word_tokens",
        "_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route search-library gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn gain_ability_routes_shape_gates_through_word_slice_helpers() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/gain_ability.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with_any(&ability_words, CANT_BE_BLOCKED_EXCEPT_BY_HASTE_PREFIXES)",
        "word_slice_contains_phrase(&word_list, CAN_ATTACK_PHRASE)",
        "word_slice_eq_any(&subject_word_refs, PRONOUN_TAGGED_SUBJECT_PHRASES)",
        "word_slice_eq_any(&real_subject_words, PLAYER_SUBJECT_PHRASES)",
        "word_slice_eq(&real_subject_words, YOU_AND_PERMANENTS_YOU_CONTROL_WORDS)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route gain-ability shape gates through word-slice helpers: missing `{required}`"
        );
    }

    assert!(
        !content.contains(".matches_words("),
        "{relative} should not route gain-ability shape gates through raw word refs"
    );
    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "gain_shape_matches_words",
        ".matches_word(",
        ".matches_token(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep retired gain-ability shape gate `{forbidden}`"
        );
    }
}

#[test]
fn return_exchange_routes_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/return_exchange.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_contains_any_word(&destination_words, RETURN_HAND_WORDS)",
        "word_slice_eq_any(words, RETURN_EXILED_CARD_REFERENCES)",
        "word_slice_starts_with(&target_words, RETURN_THAT_MANY_PREFIX)",
        "word_slice_eq(&clause_words, EXCHANGE_LIFE_TOTALS_WORDS)",
        "word_slice_starts_with_any(&clause_words, EXCHANGE_TARGET_PLAYER_PREFIXES)",
        "word_slice_eq_any(window, EXCHANGE_THAT_SHARE_RELS)",
        "word_slice_starts_with_any(share_head, EXCHANGE_CARD_TYPE_SHARE_HEAD_PREFIXES)",
        "return_find_prefix_start(&destination_words, RETURN_EXCEPT_FOR_PREFIX)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route return/exchange gates through direct word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        "fn return_shape_matches_words(",
        "return_find_phrase_start",
        "synthetic_word_tokens",
        "LexedClause",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route return/exchange gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn for_each_helpers_route_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/for_each_helpers.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_contains_any_phrase(words, DEMONSTRATIVE_OBJECT_REFERENCE_PHRASES)",
        "word_slice_contains_phrase(words, FOR_EACH_FOR_MANA_PHRASE)",
        "word_slice_starts_with(rest_words, FOR_EACH_BASE_POWER_TOUGHNESS_PREFIX)",
        "word_slice_contains_any_word(&tail_words, FOR_EACH_COMBAT_KEYWORD_WORDS)",
        "word_slice_starts_with(&tail_clause.word_refs(), FOR_EACH_WHERE_X_IS_PREFIX)",
        "word_slice_eq_any(words, FOR_EACH_POISON_COUNTERS_WORDS)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route for-each boolean gates through direct word-slice predicates: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape!",
        "synthetic_word_tokens",
        "fn for_each_shape_matches_words(",
        "fn for_each_strip_prefix_shape_clause(",
        "_PATTERN",
        ".matches_words(",
        ".matches_word(",
        ".matches_token(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route for-each boolean gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn consult_family_routes_shape_gates_through_word_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/consult_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_starts_with(&consult_prefix_words, CONSULT_TOP_LIBRARY_PREFIX)",
        "word_slice_ends_with(&consult_prefix_words, CONSULT_TOP_LIBRARY_SUFFIX)",
        "word_slice_contains_word(words, \"bottom\")",
        "word_slice_contains_phrase(words, &[\"random\", \"order\"])",
        "word_slice_contains_phrase(words, &[\"any\", \"order\"])",
        "word_slice_eq_any(\n        &crate::runtime_backend::token_word_refs(tokens),\n        CONSULT_THIS_POWER_CLAUSES,",
        "word_slice_eq(&remainder, CONSULT_THIS_TURN_CLAUSE)",
        "word_slice_eq(&remainder, CONSULT_PAY_LIFE_MANA_VALUE_CLAUSE)",
        "word_slice_contains_any_phrase(\n        &crate::runtime_backend::token_word_refs(tokens),\n        CONSULT_NOT_CAST_THIS_MARKER_PHRASES,",
        "word_slice_eq_any(\n        &crate::runtime_backend::token_word_refs(tokens),\n        CONSULT_PUT_MATCH_INTO_HAND_CLAUSES,",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route consult-family shape gates through reusable word helpers: missing `{required}`"
        );
    }

    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains("consult_shape_matches_words")
            && !content.contains(".matches_words("),
        "{relative} should not keep consult-family shape adapters/raw word refs"
    );
}

#[test]
fn next_spell_family_does_not_keep_shape_gate_adapter() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/next_spell_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_ends_with_any(words, SHARED_CAST_SUFFIXES)",
        "word_slice_find_phrase_start(shared_prefix, AND_THE_NEXT_PHRASE)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route next-spell shape gates through reusable word helpers: missing `{required}`"
        );
    }

    assert!(
        !content.contains("fn next_spell_shape_matches_words(")
            && !content.contains("shape.matches(LexedClause::new(&tokens))")
            && !content.contains("SHARED_CAST_SUFFIX_PATTERN")
            && !content.contains(".matches_words("),
        "{relative} should not keep next-spell shape adapter/raw word refs"
    );
}

#[test]
fn fanout_family_routes_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/fanout_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "first_word == DEAL_WORD",
        "word_slice_starts_with(&deal_words[1..], THAT_MUCH_PREFIXES[0])",
        "FANOUT_VERBS.contains(&verb)",
        "word_slice_ends_with_any(&words, CONTROLLER_CONTROLS_TAILS)",
        "fanout_words_are_one_of(&words, PLAYER_OPPONENT_DAMAGE_PART_WORDS)",
        "word_slice_eq(&words, &[YOU_WORD])",
        "word_slice_contains_phrase(&words_all, YOUR_GRAVEYARD_PHRASE)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route fanout shape gates through word-slice helpers and reusable constants: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "fn fanout_shape_matches_words(",
        "synthetic_word_tokens",
        "_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route fanout shape gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn choice_damage_family_routes_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/choice_damage_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "fn choice_damage_clause_first_is(",
        "word_slice_contains_phrase(&drain_clause.word_refs(), YOU_GAIN_X_LIFE_PHRASE)",
        "UP_TO_ONE_TARGET_WORDS",
        "word_slice_eq(",
        "word_slice_at_is_any(&descriptor_words, idx, CARD_OR_CARDS_WORDS)",
        "word_slice_contains_word(&descriptor_words, CARD_WORD)",
        "word_slice_contains_phrase(&random_descriptor_clause.word_refs(), AT_RANDOM_PHRASE)",
        "word_slice_eq_any(&hand_words, HAND_REFERENCE_PHRASES)",
        "word_slice_eq_any(&alt_target_words, THEM_OR_THAT_PLAYER_PHRASES)",
        "ENCHANTED_ATTACKED_THIS_TURN_PHRASES",
        "word_slice_eq_any(&subject_clause.word_refs(), DAMAGE_SOURCE_SUBJECT_PHRASES)",
        "word_slice_eq(&target_clause.word_refs(), THAT_PLAYER_WORDS)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route choice-damage shape gates through word-slice helpers and reusable recognizers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        "fn choice_damage_shape_matches_words(",
        "synthetic_word_tokens(words)",
        "shape.matches(LexedClause::new(&tokens))",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route choice-damage shape gates through ClauseShape adapters: found `{forbidden}`"
        );
    }

    assert!(
        !content.contains(".matches_words("),
        "{relative} should not route choice-damage shape gates through raw word refs"
    );
}

#[test]
fn counter_marker_family_routes_shape_gates_through_word_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/counter_marker_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "fn counter_marker_control_tail_controller(words: &[&str]) -> Option<ReturnControllerAst>",
        "word_slice_contains_word(&base_destination_words, BATTLEFIELD_WORD)",
        "word_slice_contains_word(&base_destination_words, TAPPED_WORD)",
        "counter_marker_control_tail_controller(&destination_tail)",
        "word_slice_starts_with(&base_destination_words, BATTLEFIELD_PREFIX)",
        "word_slice_eq_any(&on_target_words, IT_OR_THEM_PREFIXES)",
        "descriptor_clause.contains_word(ADDITIONAL_WORD)",
        "word_slice_eq_any(&predicate_words, ENTERS_AS_CREATURE_PREDICATE_CLAUSES)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route counter-marker shape gates through reusable word helpers: missing `{required}`"
        );
    }

    assert!(
        !content.contains("ClauseShape")
            && !content.contains("clause_shape")
            && !content.contains("counter_marker_shape_matches_words")
            && !content.contains(".matches_words("),
        "{relative} should not keep counter-marker shape adapters/raw word refs"
    );
}

#[test]
fn sentence_shape_predicates_route_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/sentence_shape_predicates.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_eq(tail, &[SENTENCE_NO_WORD])",
        "word_slice_starts_with(&words[where_idx..], SENTENCE_WHERE_X_IS_ITS_POWER_PREFIX)",
        "word_slice_eq_any(object_words, SENTENCE_EXILED_CARD_REFERENCE_PHRASES)",
        "sentence_removed_counters_this_way(object_words)",
        "word_slice_eq(&tail, SENTENCE_COMMANDER_MANA_VALUE_CHOICE_WORDS)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route sentence-shape predicates through word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "sentence_shape_matches_words",
        "sentence_find_phrase_start",
        "_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route sentence-shape predicates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn zone_counter_helpers_route_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/zone_counter_helpers.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_contains_any_word(&clause_words, ZONE_COUNTER_SPELL_WORDS)",
        "word_slice_contains_phrase(&clause_words, ZONE_COUNTER_THIS_TURN_WORDS)",
        "word_slice_eq(&words[1..], ZONE_COUNTER_POWER_WORDS)",
        "word_slice_eq_any(&target_words, &[ZONE_COUNTER_HIM_OR_HER_WORDS])",
        "word_slice_ends_with(&tail_words, ZONE_COUNTER_SOURCE_LEAVES_BATTLEFIELD_TAIL)",
        "word_slice_eq_any(&clause_words, HALF_YOUR_STARTING_LIFE_TOTAL_PHRASES)",
        "HALF_TARGET_PLAYER_STARTING_LIFE_TOTAL_PHRASES,",
        "HALF_OPPONENT_STARTING_LIFE_TOTAL_PHRASES,",
        "word_slice_ends_with(&clause_words, ZONE_COUNTER_ROUNDED_DOWN_TAIL)",
        "word_slice_starts_with_any(&target_words, ZONE_COUNTER_ALL_OR_EACH_PREFIXES)",
        "word_slice_eq_any(&target_words, ZONE_COUNTER_SELF_REFERENCE_TARGETS)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route zone-counter gates through direct word-slice helpers: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "zone_counter_shape_matches_words",
        "token_slice_matches_shape",
        "synthetic_word_tokens",
        "_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route zone-counter gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn chain_carry_routes_direct_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/chain_carry.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "word_slice_eq_any(&words, CHAIN_CHOOSE_BASIC_LAND_TYPE_WORDS)",
        "word_slice_eq_any(&words, CHAIN_YOU_CHOOSE_BASIC_LAND_TYPE_WORDS)",
        "word_slice_contains_any_word(&words, CHAIN_TOKEN_WORDS)",
        "word_slice_eq(&words, CHAIN_OWNER_YOUR_WORDS)",
        "word_slice_eq_any(&words, CHAIN_OWNER_TARGET_PLAYER_WORDS)",
        "word_slice_eq_any(&words, CHAIN_OWNER_TARGET_OPPONENT_WORDS)",
        "word_slice_starts_with(\n        &token_word_refs(&clause_tokens[after_library_start..]),\n        CHAIN_FACE_DOWN_SHUFFLE_FROM_PREFIX,",
        "word_slice_starts_with(&clause_words, CHAIN_EXILE_THEM_PREFIX)",
        "word_slice_find_phrase_start(&clause_words, CHAIN_THEN_MELD_THEM_INTO_PREFIX)",
        "word_slice_starts_with(&token_word_refs(&stripped), CHAIN_CHOOSE_TO_PREFIX)",
        "word_slice_starts_with_any(&clause_words, CHAIN_TAP_ALL_OR_EACH_PREFIXES)",
        "word_slice_contains_any_phrase(&clause_words, CHAIN_OR_UNTAP_ALL_EACH_PHRASES)",
        "word_slice_contains_word(&words, \"target\")",
        "word_slice_starts_with(&segment_words, CHAIN_ALL_ABILITIES_AND_PREFIX)",
        "word_slice_eq(&segment_words, CHAIN_ROUNDED_UP_WORDS)",
        "word_slice_contains_phrase(&token_word_refs(previous), CHAIN_WHERE_X_IS_HALF_PHRASE)",
        "word_slice_contains_phrase(&chain_words, CHAIN_NEXT_END_STEP_REPEAT_MARKER_PHRASE)",
        "word_slice_contains_any_phrase(\n            &token_word_refs(&segment),\n            CHAIN_TOKEN_RULES_TAIL_MARKER_PHRASES,",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct chain-carry gates through direct token word-slice predicates: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape!",
        "LexedClause",
        "synthetic_word_tokens",
        "fn chain_find_phrase_start_lexed(",
        "fn chain_shape_matches_words(",
        "fn chain_find_word(",
        "_PATTERN",
        ".matches(",
        ".matches_word(",
        ".matches_token(",
        ".matches_first_word(",
        ".matches_word_at(",
        "fn chain_find_phrase_start(words: &[&str], shape: &ClauseShape<'static>)",
        "shape.matches_words(&words[*idx..])",
        "CHAIN_CHOOSE_BASIC_LAND_TYPE_PATTERN.matches_words(&words)",
        "CHAIN_YOU_CHOOSE_BASIC_LAND_TYPE_PATTERN.matches_words(&words)",
        "CHAIN_TOKEN_OR_TOKENS_PATTERN.matches_words(&words)",
        "CHAIN_FACE_DOWN_SHUFFLE_FROM_PATTERN.matches_words(&clause_words[library_idx + 1..])",
        "CHAIN_EXILE_THEM_PREFIX_PATTERN.matches_words(&clause_words)",
        "chain_find_phrase_start(&clause_words, &CHAIN_THEN_MELD_THEM_INTO_PREFIX_PATTERN)",
        "CHAIN_CHOOSE_TO_PREFIX_PATTERN.matches_words(&token_word_refs(&stripped))",
        "CHAIN_TAP_ALL_OR_EACH_PREFIX_PATTERN.matches_words(&clause_words)",
        "CHAIN_OR_UNTAP_ALL_EACH_PATTERN.matches_words(&clause_words)",
        "CHAIN_TARGET_WITH_CARD_TYPE_WINDOW_PATTERN.matches_words(&words)",
        "CHAIN_ALL_ABILITIES_AND_PATTERN.matches_words(&segment_words)",
        "CHAIN_ROUNDED_UP_PATTERN.matches_words(&segment_words)",
        "CHAIN_WHERE_X_IS_HALF_MARKER_PATTERN.matches_words(&token_word_refs(previous))",
        "CHAIN_NEXT_END_STEP_REPEAT_MARKER_PATTERN.matches_words(&chain_words)",
        "CHAIN_TOKEN_RULES_TAIL_MARKER_PATTERN.matches_words(&segment_words)",
        "CHAIN_OWNER_YOUR_PATTERN.matches_words(&words)",
        "CHAIN_OWNER_TARGET_PLAYER_PATTERN.matches_words(&words)",
        "CHAIN_OWNER_TARGET_OPPONENT_PATTERN.matches_words(&words)",
        "CHAIN_UNTIL_EOT_TRIGGER_PREFIX_PATTERN.matches_words(clause_words)",
        "CHAIN_WOULD_ENTER_INSTEAD_PATTERN.matches_words(clause_words)",
        "CHAIN_BEGINNING_END_STEP_PATTERN.matches_words(words)",
        "CHAIN_END_OF_COMBAT_PATTERN.matches_words(words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct chain-carry shape gates through raw word refs: found `{forbidden}`"
        );
    }

    assert!(
        !content.contains(".matches_words("),
        "{relative} should not route chain-carry shape gates through raw word refs"
    );
}

/// Parse the production source into `(fn_name, window)` pairs, where `window`
/// is the source starting at the `fn NAME` declaration and extending for up to
/// `WINDOW_LINES` lines. This mirrors the line-window detector that originally
/// found the ~50 imperative cursor-walk matchers.
fn function_windows(source: &str) -> Vec<(String, String)> {
    const WINDOW_LINES: usize = 60;
    let lines: Vec<&str> = source.lines().collect();
    let mut windows = Vec::new();
    for (start, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("fn ") else {
            continue;
        };
        // Extract the bare function name (up to `(`, `<`, or whitespace).
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            continue;
        }
        let end = (start + WINDOW_LINES).min(lines.len());
        let window = lines[start..end].join("\n");
        windows.push((name, window));
    }
    windows
}

const CURSOR_WORD_VARS: [&str; 7] = [
    "words",
    "word_refs",
    "clause_words",
    "tail_words",
    "sentence_words",
    "content_words",
    "second_words",
];

const CURSOR_INDEX_VARS: [&str; 3] = ["idx", "cursor", "word_idx"];

/// Detect the cursor-walk-loop signature inside a single function window.
///
/// A window is flagged only if it contains ALL THREE of:
///   (a) a mutable cursor binding (`let mut idx`/`let mut cursor`/`let mut word_idx`),
///   (b) that cursor being advanced (`idx +=` / `cursor +=` / `word_idx +=`), and
///   (c) word-slice indexing into a known word var by that cursor, i.e.
///       `<word_var> ... .get(<cursor>)` on a single line, or `<word_var>[<cursor>]`.
fn window_is_cursor_walk(window: &str) -> bool {
    let has_binding = CURSOR_INDEX_VARS
        .iter()
        .any(|v| window.contains(&format!("let mut {v}")));
    let has_advance = CURSOR_INDEX_VARS
        .iter()
        .any(|v| window.contains(&format!("{v} +=")));
    if !(has_binding && has_advance) {
        return false;
    }

    // (c) word-slice indexing into a word var by a cursor var.
    for line in window.lines() {
        for word_var in CURSOR_WORD_VARS {
            // Find a position where the word var appears as a whole token.
            let mut search = line;
            while let Some(pos) = search.find(word_var) {
                let after = &search[pos + word_var.len()..];
                let before_ok = pos == 0
                    || !search.as_bytes()[pos - 1].is_ascii_alphanumeric()
                        && search.as_bytes()[pos - 1] != b'_';
                let after_ok = after
                    .chars()
                    .next()
                    .map(|c| !(c.is_alphanumeric() || c == '_'))
                    .unwrap_or(true);
                if before_ok && after_ok {
                    // `<word_var>[<cursor>]` directly after the token (optionally `&`).
                    let direct = after.trim_start_matches('&');
                    for cur in CURSOR_INDEX_VARS {
                        if direct.starts_with(&format!("[{cur}]")) {
                            return true;
                        }
                    }
                    // `<word_var> ... .get(<cursor>)` later on the same line.
                    for cur in CURSOR_INDEX_VARS {
                        if after.contains(&format!(".get({cur})")) {
                            return true;
                        }
                    }
                }
                search = &search[pos + word_var.len()..];
            }
        }
    }
    false
}

/// Known-debt ratchet allowlist for imperative word-index cursor-walk matchers
/// in `runtime_backend`.
///
/// A refactor (`git show 7ec4331bf bcff8d1e6`) converted the bulk of imperative
/// word-index cursor-walk matchers to declarative forms (LexPattern /
/// word_slice / `.position()`). The functions listed below are the genuinely
/// irreducible remainder: semantic scans that validate each word via parse
/// fns, perform unbounded accumulation or token rewrites, plus a few
/// lint-pinned word_slice gates.
///
/// This is a RATCHET, not an aspiration: the set may only shrink.
///   * Adding a NEW cursor-walk-loop matcher is forbidden. Convert it to a
///     declarative form (LexPattern / word_slice / `.position()`) instead.
///   * If you legitimately added an irreducible semantic scan, add its
///     `relative_path::fn_name` here WITH A COMMENT explaining why it cannot be
///     made declarative.
///   * If you removed/converted one, delete its entry here.
fn cursor_walk_allowlist() -> BTreeSet<String> {
    [
        // LexPattern matcher internals: these ARE the declarative replacement
        // primitive (the word/atom matcher everything else is converted to),
        // so they cannot themselves be expressed as a higher-level pattern.
        "crates/ironsmith-compiler/src/runtime_backend/front_end/lex_patterns.rs::match_atoms",
        "crates/ironsmith-compiler/src/runtime_backend/front_end/lex_patterns.rs::match_words",
        // Semantic spell-restriction scans: validate each word via parse fns
        // while accumulating a subject filter.
        "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activation_restriction_clauses.rs::damage_cause_life_loss_restriction_from_tail",
        "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activation_restriction_clauses.rs::parse_spell_restriction_subject_filter",
        "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activation_restriction_clauses.rs::parse_spell_subject_cant_be_cast_filter",
        // "as ~ enters, choose" subject scan: walks the choice clause word by
        // word, validating each token.
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs::parse_as_enters_choice_subject_clause",
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs::parse_as_enters_choice_subject_tokens",
        // looked-cards family: prefix stripping / token rewrites and unbounded
        // semantic scans over the looked-card clause.
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/looked_cards_family.rs::looked_clause_first_is",
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/looked_cards_family.rs::looked_words_start_into_hand",
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/looked_cards_family.rs::parse_prior_effect_number_value",
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/looked_cards_family.rs::strip_up_to_one_looked_card_choice_prefix",
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/looked_cards_family.rs::token_is_word",
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/looked_cards_family.rs::token_words_non_article_eq_any",
        // control/copy/attach verb handlers: token rewrites and zone-constraint
        // scans that consume a variable-length tail.
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/control_copy_attach_verbs.rs::apply",
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/control_copy_attach_verbs.rs::apply_source_zone_constraint",
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/control_copy_attach_verbs.rs::is_top_or_bottom_choice_destination",
        // prior-effect count binding: unbounded semantic scan over the clause.
        "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/counter_stat_verbs.rs::parse_prior_effect_count_binding_clause",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[test]
fn runtime_backend_word_cursor_walks_are_allowlisted() {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_rust_files(
        &root.join("crates/ironsmith-compiler/src/runtime_backend"),
        &mut files,
    );
    files.sort();

    let mut offenders = BTreeSet::new();
    for path in files {
        let relative = repo_relative(&root, &path);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let source = production_source(&content);
        for (name, window) in function_windows(source) {
            if window_is_cursor_walk(&window) {
                offenders.insert(format!("{relative}::{name}"));
            }
        }
    }

    let allowlist = cursor_walk_allowlist();
    assert_eq!(
        offenders, allowlist,
        "runtime_backend cursor-walk-loop matchers changed.\n\
         New imperative cursor-walk-loop matchers are FORBIDDEN: convert them to a \
         declarative form (LexPattern / word_slice / `.position()`).\n\
         If you legitimately added an irreducible semantic scan, add its \
         `relative_path::fn_name` to `cursor_walk_allowlist()` WITH A COMMENT \
         explaining why.\n\
         If you removed/converted one, delete its entry from `cursor_walk_allowlist()`."
    );
}
