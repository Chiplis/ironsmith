#![allow(unused_imports)]
use super::shard_01::*;
use super::shard_02::*;
use super::*;

#[test]
pub(super) fn debug_safe_is_mechanical_only() {
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
pub(super) fn aggregate_compiled_card_models_are_core_owned() {
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
pub(super) fn compiled_text_renderer_does_not_read_source_only_fields() {
    let root = workspace_root();
    let renderer_files = [
        "crates/ironsmith-runtime/src/compiled_text/mod.rs",
        "crates/ironsmith-runtime/src/compiled_text/ast_render.rs",
        "crates/ironsmith-runtime/src/compiled_text/render_effects.rs",
        "crates/ironsmith-runtime/src/compiled_text/normalize_common.rs",
        "crates/ironsmith-runtime/src/compiled_text/oracle_style.rs",
    ];
    let forbidden = [
        ".source_text",
        ".source_label",
        ".presentation_label.as_deref()",
        "mode.description",
        "prompt.description",
        "RepeatProcessPromptEffect { pub description",
    ];

    for relative in renderer_files {
        let content = read_repo_file(&root, relative);
        let production_content = content
            .split("\n#[cfg(test)]")
            .next()
            .unwrap_or(content.as_str());
        for fragment in forbidden {
            assert!(
                !production_content.contains(fragment),
                "{relative} must render from AST/typed presentation metadata, not source-only field `{fragment}`"
            );
        }
    }

    let repeat_prompt = read_repo_file(
        &root,
        "crates/ironsmith-runtime/src/effects/composition/repeat_process_prompt.rs",
    );
    assert!(
        !repeat_prompt.contains("pub description"),
        "RepeatProcessPromptEffect must not store renderer-ready source text"
    );
}

#[test]
pub(super) fn migrated_effect_payloads_are_core_owned() {
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
pub(super) fn migrated_static_ability_model_is_core_owned() {
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
pub(super) fn compiler_boundary_adapter_has_no_semantic_conversion_tables() {
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
pub(super) fn runtime_public_surface_does_not_export_legacy_executor_or_game_event_modules() {
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
pub(super) fn runtime_gameplay_code_does_not_call_global_registry_singletons_directly() {
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
pub(super) fn compiler_runtime_backend_does_not_import_ironsmith_runtime_directly() {
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
pub(super) fn runtime_code_does_not_import_legacy_executor_module_paths() {
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
pub(super) fn parser_lowering_dry_checklist_is_kept_in_repo() {
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
pub(super) fn winnow_leaf_parser_patterns_are_kept_in_repo() {
    let root = workspace_root();
    let patterns = read_repo_file(&root, "architecture/winnow-leaf-parser-patterns.md");

    for required in [
        "Recognition lives in `crates/ironsmith-compiler/src/runtime_backend/front_end/grammar`.",
        "New parser code uses `winnow`. Do not add `nom`.",
        "fn parse_count(input: &mut &str) -> WResult<u32>",
        "Raw `.split_once`, `.find`, and token-window searches",
        "--fail-on-findings",
    ] {
        assert!(
            patterns.contains(required),
            "winnow leaf-parser pattern guide is missing required guidance: {required}"
        );
    }
}

#[test]
pub(super) fn compiler_parser_does_not_depend_on_nom() {
    let root = workspace_root();
    let compiler_manifest = read_repo_file(&root, "crates/ironsmith-compiler/Cargo.toml");

    assert!(
        !compiler_manifest
            .lines()
            .map(str::trim)
            .any(|line| line.starts_with("nom ") || line.starts_with("nom=")),
        "ironsmith-compiler parser dependencies must use winnow, not nom"
    );
}

#[test]
pub(super) fn cardinal_recognition_has_one_leaf_grammar_owner() {
    let root = workspace_root();
    let runtime_backend = root.join("crates/ironsmith-compiler/src/runtime_backend");
    let expected_owner =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/leaf/numbers.rs";
    let mut files = Vec::new();
    collect_rust_files(&runtime_backend, &mut files);

    let mut owners = files
        .into_iter()
        .filter_map(|path| {
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            (content.contains("ironsmith_core::parse_cardinal_word(")
                || content.contains("ironsmith_core::parse_cardinal_words("))
            .then(|| repo_relative(&root, &path))
        })
        .collect::<Vec<_>>();
    owners.sort();

    assert_eq!(
        owners,
        vec![expected_owner.to_string()],
        "generic number recognition must remain owned by the typed leaf grammar"
    );
}

#[test]
pub(super) fn migrated_leaf_namespaces_are_typed_winnow_grammar() {
    let root = workspace_root();
    let leaf_root = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/leaf";
    let leaf_facade = read_repo_file(
        &root,
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/leaf.rs",
    );

    for module in [
        "articles.rs",
        "condition_prefixes.rs",
        "counts.rs",
        "durations.rs",
        "filter_atoms.rs",
        "mana.rs",
        "numbers.rs",
        "player_subjects.rs",
        "power_toughness.rs",
        "references.rs",
        "source_references.rs",
        "targets.rs",
    ] {
        let relative = format!("{leaf_root}/{module}");
        let content = read_repo_file(&root, &relative);
        assert!(
            content.contains("winnow::") && content.contains("pub(crate) fn parse_"),
            "{relative} must own typed winnow parser entry points"
        );

        let module_name = module.trim_end_matches(".rs");
        assert!(
            leaf_facade.contains(&format!("mod {module_name};")),
            "leaf grammar facade must register {module_name}"
        );
    }

    for (module, typed_result) in [
        ("articles.rs", "pub(crate) enum LeafArticle"),
        ("counts.rs", "pub(crate) struct LeafCountRange"),
        ("durations.rs", "pub(crate) enum LeafDurationPhrase"),
        ("mana.rs", "pub(crate) struct LeafManaCostPrefix"),
        ("numbers.rs", "pub(crate) enum LeafNumber"),
        (
            "player_subjects.rs",
            "pub(crate) enum LeafPlayerReferenceMode",
        ),
        ("targets.rs", "pub(crate) struct LeafTargetHead"),
    ] {
        let relative = format!("{leaf_root}/{module}");
        let content = read_repo_file(&root, &relative);
        assert!(
            content.contains(typed_result),
            "{relative} must own typed parser result `{typed_result}`"
        );
    }
}

#[test]
pub(super) fn migrated_line_families_consume_typed_grammar_results() {
    let root = workspace_root();
    let cases: &[(&str, &[&str])] = &[
        (
            "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activation_costs.rs",
            &[
                "grammar::activation_costs::cant_shapes::{",
                "cant_shapes::parse_direct_cant_fact_tokens(tokens)",
                "cant_shapes::parse_attack_unless_condition_tokens(tokens)",
            ],
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/activation_restriction_clauses.rs",
            &[
                "grammar::activation_restrictions::{",
                "parse_static_restriction_condition_shape_tokens(tokens)",
                "restriction_grammar::parse_cant_cast_restriction_fact_words(words)",
            ],
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/families/activation_and_restrictions/keyword_activated_lines.rs",
            &[
                "grammar::keyword_activated_lines::{",
                "keyword_activated_grammar::parse_cycling_keyword_cost_groups_tokens(tokens)",
                "keyword_activated_grammar::parse_craft_line_spec_tokens(tokens)",
                "keyword_activated_grammar::parse_equip_line_spec_tokens(tokens)",
            ],
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/anthem_grant_lines.rs",
            &[
                "anthem_grant_grammar::parse_first_spell_each_turn_clause(tokens)",
                "anthem_grant_grammar::parse_granted_keyword_verb_facts(tokens)",
                "anthem_grant_grammar::parse_source_counter_condition(&tokens)",
            ],
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs",
            &[
                "use super::grammar::static_keyword_shapes;",
                "static_keyword_shapes::parse_rule_id_head(rule_id)",
                "use super::grammar::anthem_grants as anthem_grant_grammar;",
            ],
        ),
    ];

    for (relative, required_routes) in cases {
        let content = read_repo_file(&root, relative);
        for required in *required_routes {
            assert!(
                content.contains(required),
                "{relative} must consume typed grammar result `{required}`"
            );
        }
    }
}

#[test]
pub(super) fn migrated_effect_families_consume_typed_grammar_results() {
    let root = workspace_root();
    let cases: &[(&str, &[&str])] = &[
        (
            "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/verb_handlers/resource_verbs.rs",
            &[
                "resource_grammar::parse_resource_look_shape(tokens, subject_player)",
                "resource_grammar::parse_resource_shuffle_shape(tokens, player)",
                "resource_grammar::parse_resource_chosen_name_target_shape(target_tokens)",
            ],
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/registry.rs",
            &[
                "grammar::effects::subject_verb_registry_shapes as registry_shapes",
                "registry_shapes::parse_joint_draw_shape(clause.tokens())",
                "registry_shapes::parse_registry_next_end_step_shape(clause.tokens())",
            ],
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/combat_and_damage_family.rs",
            &[
                "grammar::effects::combat_damage_family_shapes as combat_shapes",
                "combat_shapes::parse_put_sticker_shape(clause.tokens())",
                "combat_shapes::parse_return_multiple_targets_shape(clause.tokens())",
            ],
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/delayed_step_family.rs",
            &[
                "grammar::effects::delayed_step_shapes as delayed_grammar",
                "delayed_grammar::parse_delayed_creature_types_shape(",
                "delayed_grammar::parse_delayed_losing_pump_shape(",
            ],
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/for_each_helpers.rs",
            &[
                "grammar::effects::for_each_shapes::{",
                "for_each_shapes::parse_for_each_object_subject_shape(subject_tokens)",
                "for_each_shapes::parse_participant_clause_shape(tokens)",
            ],
        ),
    ];

    for (relative, required_routes) in cases {
        let content = read_repo_file(&root, relative);
        for required in *required_routes {
            assert!(
                content.contains(required),
                "{relative} must consume typed grammar result `{required}`"
            );
        }
    }

    for relative in [
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/resource_shapes.rs",
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/subject_verb_registry_shapes/joint.rs",
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/clause_dispatch_shapes/core.rs",
    ] {
        let content = read_repo_file(&root, relative);
        assert!(
            content.contains("winnow::") && content.contains("pub(crate) fn parse_"),
            "{relative} must expose typed winnow grammar entry points"
        );
    }
}

#[test]
pub(super) fn parser_audits_point_at_runtime_backend() {
    let root = workspace_root();
    let manual_audit = read_repo_file(
        &root,
        "crates/ironsmith-tools/src/bin/audit_manual_parser_sections.rs",
    );
    let size_audit = read_repo_file(
        &root,
        "crates/ironsmith-tools/src/bin/audit_parser_module_sizes.rs",
    );

    for audit in [&manual_audit, &size_audit] {
        assert!(
            audit.contains("crates/ironsmith-compiler/src/runtime_backend"),
            "parser audits must scan the runtime_backend compiler tree"
        );
        assert!(
            !audit.contains("src/cards/builders/compiler"),
            "parser audits must not scan the stale compiler tree"
        );
    }

    assert!(
        manual_audit.contains("--enforce-prefix") && manual_audit.contains("--fail-on-findings"),
        "manual parser audit must support staged and global enforcement"
    );

    for bypass_alias in [
        "words_have_phrase(",
        "words_start_with(",
        "tokens_start_with(",
        "items_have(",
        "locate_index(",
        "token_start_for_word(",
        "str_contains(",
        "str_split_once(",
    ] {
        assert!(
            manual_audit.contains(bypass_alias),
            "manual parser audit must detect legacy-helper alias `{bypass_alias}`"
        );
    }

    assert!(
        !manual_audit.contains("Some(\"migration_audit.rs\")"),
        "manual parser audit must not provide a filename-based enforcement bypass"
    );
    assert!(
        !root
            .join("crates/ironsmith-compiler/src/runtime_backend/migration_audit_allowlist")
            .exists(),
        "the temporary parser migration allowlist must be deleted"
    );
}

#[test]
pub(super) fn parse_annotations_stay_diagnostic_only() {
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
pub(super) fn condition_antecedent_binding_has_single_lowering_owner() {
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

pub(super) fn production_source(content: &str) -> &str {
    content.split("\n#[cfg(test)]").next().unwrap_or(content)
}

pub(super) fn non_test_raw_text_check_literals(content: &str) -> Vec<String> {
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

pub(super) fn function_source<'a>(
    content: &'a str,
    start_marker: &str,
    end_marker: &str,
) -> &'a str {
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
pub(super) fn lowering_lower_has_no_raw_text_checks_or_migration_allowlist() {
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
        "lowering/lower must not recognize Oracle text; move parser facts into typed front-end grammar"
    );

    for relative in [
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/activated_lowering.rs",
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs",
    ] {
        assert!(
            !root.join(relative).exists(),
            "legacy parser-owned lowering module must stay deleted: {relative}"
        );
    }

    assert!(
        !root
            .join("crates/ironsmith-compiler/src/runtime_backend/migration_audit_allowlist")
            .exists(),
        "the parser migration must not regain a temporary allowlist"
    );
}

#[test]
pub(super) fn runtime_backend_matches_words_is_clause_shape_primitive_only() {
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
pub(super) fn document_line_family_handlers_have_no_raw_text_checks() {
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
        "document line-family routing must hand Oracle recognition to typed token/shape grammar"
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
pub(super) fn document_partner_parenthetical_trims_use_token_kinds() {
    let root = workspace_root();
    let caller_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/document/line_family_handlers.rs";
    let caller = read_repo_file(&root, caller_relative);
    let adapter = function_source(
        &caller,
        "fn partner_with_name_from_line",
        "pub(super) fn run_combined_static_line_family",
    );

    assert!(
        adapter
            .contains("keyword_special_lines::parse_partner_with_name_shape_tokens(&line.tokens)")
            && adapter.contains("render_original_text_for_token_slice(line, shape.name_tokens)")
            && adapter.contains("render_token_slice(shape.name_tokens)"),
        "{caller_relative} should consume the typed partner-name token span and preserve its original surface"
    );
    for forbidden in [
        "raw_line",
        "normalized_text",
        "TokenKind::LParen",
        "split_once",
        "str_split_once_char",
        "\"partner with \".len()",
        "trim_end_matches('.')",
    ] {
        assert!(
            !adapter.contains(forbidden),
            "{caller_relative} should not rediscover partner-name boundaries with `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn document_prefix_punctuation_checks_use_char_helpers() {
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
        numeric_result_prefix
            .contains("document_grammar::parse_numeric_result_prefix_tokens(&tokens)"),
        "{relative} should classify numeric result prefixes through the typed document grammar"
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
pub(super) fn document_ability_word_label_detection_uses_label_tokens() {
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
pub(super) fn grammar_effect_labeled_prefix_classifiers_use_parser_token_words() {
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
pub(super) fn strict_unsupported_preflight_uses_tokens_not_raw_text() {
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
pub(super) fn instead_followup_classifier_uses_tokens_not_raw_oracle_text() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/instead.rs";
    let content = read_repo_file(&root, relative);
    let classifier = function_source(
        &content,
        "pub(crate) fn parse_instead_followup_semantics_lexed",
        "#[cfg(test)]",
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
    assert!(
        classifier.contains("WResult<InsteadSemantics>")
            && classifier.contains("pub(crate) fn parse_instead_followup_shape_tokens")
            && classifier.contains("InsteadFollowupShape"),
        "{relative} should return typed instead-followup semantics and shape facts from winnow grammar"
    );

    let adapter_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/shared/util.rs";
    let adapter = read_repo_file(&root, adapter_relative);
    let adapter = function_source(
        &adapter,
        "pub(crate) fn classify_instead_followup_tokens",
        "pub(crate) fn find_first_sacrifice_cost_choice_tag",
    );
    assert!(
        adapter.contains("grammar::effects::classify_instead_followup_semantics_tokens(tokens)"),
        "{adapter_relative} should only forward typed tokens to the grammar owner"
    );
}

#[test]
pub(super) fn shared_util_cost_tag_lookup_uses_named_tag_helpers() {
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
pub(super) fn shared_util_level_header_parser_uses_tokens_not_raw_prefixes() {
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
pub(super) fn shared_util_power_toughness_parser_uses_char_helpers() {
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
pub(super) fn labeled_keyword_prefix_preservation_is_front_end_grammar_owned() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn preserve_labeled_ability_prefix_for_parse_text",
        "fn is_generic_ability_label_prefix_text",
    );

    assert!(
        parser.contains("labeled_prefix_tokens(prefix)"),
        "{relative} should route keyword-prefix text through the front-end lexer"
    );
    assert!(
        parser.contains("parser_token_word_refs(&tokens)"),
        "{relative} should classify keyword prefixes from parser token words"
    );
    assert!(
        content.contains("fn labeled_prefix_tokens(prefix: &str)")
            && content.contains("lex_line(prefix.trim(), 0).ok()"),
        "{relative} should centralize the text-to-token boundary before semantic prefix classification"
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
pub(super) fn token_pt_parsing_is_front_end_leaf_grammar_owned() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/token_definitions/surface.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn token_pt",
        "fn parse_token_definition_pt_token",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "token P/T parsing should delegate to typed leaf grammar, not raw string probes"
    );

    assert!(
        helper.contains("leaf::parse_leaf_unsigned_pt_complete(word)"),
        "{relative} should route token P/T recognition through the typed leaf parser"
    );

    let lowering_relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/compile_support.rs";
    let lowering = read_repo_file(&root, lowering_relative);
    assert!(
        !lowering.contains("fn parse_token_pt"),
        "{lowering_relative} must not regain parser-owned token P/T recognition"
    );
}

#[test]
pub(super) fn activated_sentence_classification_is_front_end_grammar_owned() {
    let root = workspace_root();
    let grammar_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/activated_lowering.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    let classifier = function_source(
        &grammar,
        "pub(crate) fn classify_activated_restriction_sentence",
        "fn x_definition_intro",
    );
    let actual = non_test_raw_text_check_literals(classifier)
        .into_iter()
        .map(|literal| format!("{grammar_relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "activated sentence classification should use token grammar, not rendered raw text probes"
    );

    let semantic_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/semantic_line_parsing/activated.rs";
    let semantic = read_repo_file(&root, semantic_relative);
    let dispatcher = function_source(
        &semantic,
        "fn finalize_rewrite_activated_effect_sentences",
        "fn split_rewrite_activated_effect_text",
    );
    assert!(
        dispatcher.contains("activated_grammar::classify_activated_restriction_sentence(&tokens)")
            && dispatcher.contains("parse_mana_restriction_tokens(&tokens)"),
        "{semantic_relative} should delegate activated sentence recognition to typed front-end grammar"
    );
    assert!(
        !grammar.contains("align_activated_parse_sentences")
            && !semantic.contains("align_rewrite_activated_parse_sentences"),
        "dead activated sentence-alignment adapters must not be restored"
    );
}

#[test]
pub(super) fn activated_display_text_uses_typed_presentation_kind_not_raw_scan() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/semantic_line_parsing/activated.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn rewrite_activated_display_text",
        "fn infer_rewrite_activated_functional_zones",
    );

    assert!(
        helper.contains("line.presentation_kind?.display()")
            && helper.contains("render_token_slice(&line.cost_parse_tokens)")
            && helper.contains("render_token_slice(&line.effect_parse_tokens)"),
        "{relative} should build activated presentation text from the typed presentation kind and parsed cost/effect tokens"
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
pub(super) fn filter_player_relation_core_shapes_use_typed_winnow_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/filters/player_relations.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "use winnow::combinator::{alt, opt};",
        "use winnow::error::ModalResult as WResult;",
        "fn relation_phrase<'a>(",
        "fn parse_relation_axis_word_slice(",
        "fn parse_relation_verb_word_slice(",
        "fn parse_relation_subject_word_slice(",
        "fn parse_negated_you_relation_word_slice(",
        "fn parse_chosen_player_graveyard_word_slice(",
        "fn parse_owner_controller_pair_word_slice(",
        "fn parse_entered_battlefield_this_turn_word_slice(",
        "let mut input: primitives::WordSliceInput<'_> = words;",
        "parse_relation_subject_shape(words, pronoun_player_filter)",
        "parse_negated_you_relation_shape(words)?",
        "parse_chosen_player_graveyard_shape(words)?",
    ] {
        assert!(
            content.contains(required),
            "{relative} must own typed winnow player-relation recognition: missing `{required}`"
        );
    }

    for forbidden in [
        "LexPattern",
        "LexCapture",
        "relation_captured_prefix",
        "match_prefix_word_refs",
        ".match_pattern(",
        "ClauseShape",
        "clause_shape!",
        "synthetic_word_tokens(words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} must not retain transitional player-relation matcher `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_own_control_and_conjoined_shapes_use_predicate_tokens() {
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
pub(super) fn predicate_final_filtered_adapters_use_predicate_tokens() {
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
pub(super) fn reference_tag_stage_uses_parser_token_words_for_legacy_word_core() {
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
pub(super) fn object_filter_tap_activated_ability_qualifier_uses_token_mirror() {
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
pub(super) fn keyword_static_source_is_chosen_color_uses_token_word_view() {
    let root = workspace_root();
    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/keyword_static_lines/color_choice_shapes.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    let entry = function_source(
        &grammar,
        "pub(crate) fn parse_source_is_chosen_color_tokens",
        "fn parse_pregame_choose_color_lexed",
    );
    let recognizer = function_source(
        &grammar,
        "fn parse_source_is_chosen_color_lexed",
        "fn classify_chosen_color_subject",
    );
    for required in [
        "primitives::parse_all(",
        "parse_source_is_chosen_color_lexed",
        "Option<(ChosenColorSubjectSurface, bool)>",
    ] {
        assert!(
            entry.contains(required),
            "{grammar_relative} should expose a typed all-consuming chosen-color parser: missing `{required}`"
        );
    }
    for required in [
        "repeat_till",
        "peek(primitives::kw(\"is\"))",
        "primitives::phrase(&[\"chosen\", \"color\"])",
        "primitives::sentence_end()",
        "classify_chosen_color_subject",
    ] {
        assert!(
            recognizer.contains(required),
            "{grammar_relative} should own chosen-color Oracle recognition with winnow: missing `{required}`"
        );
    }

    let family_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let family = read_repo_file(&root, family_relative);
    let consumer = function_source(
        &family,
        "pub(crate) fn parse_source_is_chosen_color_line",
        "pub(crate) fn parse_choose_creature_type_as_enters_line",
    );
    assert!(
        consumer.contains("keyword_static_lines::parse_source_is_chosen_color_tokens(tokens)"),
        "{family_relative} should lower the typed chosen-color grammar result"
    );
    for forbidden in [
        "LexedClause",
        "TokenWordView",
        "token_word_refs",
        "word_slice_",
        "ClauseShape",
        ".matches_words(",
    ] {
        assert!(
            !consumer.contains(forbidden),
            "{family_relative} should not rediscover chosen-color Oracle text after typed parsing: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_source_damage_prevention_uses_token_slices() {
    let root = workspace_root();
    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/keyword_static_lines/damage_combat.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    let recognizer = function_source(
        &grammar,
        "fn parse_prevent_damage_to_you_lexed",
        "fn parse_damage_amount_tail_lexed",
    );
    for required in [
        "WResult<PreventDamageToYouSpec<'a>>",
        "repeat_till",
        "peek(primitives::phrase(&[",
        "\"would\", \"deal\", \"damage\", \"to\", \"you\"",
        "leaf::parse_leaf_number_prefix_lexed",
        "primitives::phrase(&[\"of\", \"that\", \"damage\"])",
        "source_tokens: trim_lexed_commas(source_tokens)",
    ] {
        assert!(
            recognizer.contains(required),
            "{grammar_relative} should return typed source/amount facts from winnow recognition: missing `{required}`"
        );
    }

    let family_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let family = read_repo_file(&root, family_relative);
    let consumer = function_source(
        &family,
        "pub(crate) fn parse_prevent_damage_to_you_from_source_filter_line",
        "pub(crate) fn parse_replace_damage_with_counters_instead_line",
    );
    for required in [
        "keyword_static_lines::parse_prevent_damage_to_you_tokens(tokens)",
        "parse_damage_source_filter_tokens(spec.source_tokens)",
        "spec.amount",
    ] {
        assert!(
            consumer.contains(required),
            "{family_relative} should only lower the typed prevention spec: missing `{required}`"
        );
    }
    for forbidden in [
        "LexedClause",
        "TokenWordView",
        "token_word_refs",
        "word_slice_",
        "find_phrase",
        "would deal damage to you",
        "of that damage",
    ] {
        assert!(
            !consumer.contains(forbidden),
            "{family_relative} should not reparse the prevention sentence after grammar recognition: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_minimum_damage_replacement_prefixes_use_clause_shapes() {
    let root = workspace_root();
    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/keyword_static_lines/damage_combat.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    let entry = function_source(
        &grammar,
        "pub(crate) fn parse_minimum_red_noncombat_damage_tokens",
        "pub(crate) fn parse_prevent_damage_to_you_tokens",
    );
    let recognizer = function_source(
        &grammar,
        "fn parse_minimum_red_noncombat_damage_lexed",
        "fn parse_prevent_damage_to_you_lexed",
    );
    assert!(
        entry.contains("primitives::parse_all(")
            && entry.contains("parse_minimum_red_noncombat_damage_lexed"),
        "{grammar_relative} should expose an all-consuming typed minimum-damage recognizer"
    );
    for required in [
        "WResult<()>",
        "primitives::phrase(&[",
        "repeat_till",
        "primitives::kw(\"power\")",
        "primitives::phrase(&[\"to\", \"an\", \"opponent\"])",
        "primitives::phrase(&[\"power\", \"instead\"])",
        "primitives::sentence_end()",
    ] {
        assert!(
            recognizer.contains(required),
            "{grammar_relative} should own the complete minimum-damage surface: missing `{required}`"
        );
    }

    let family_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let family = read_repo_file(&root, family_relative);
    let consumer = function_source(
        &family,
        "pub(crate) fn parse_minimum_damage_amount_replacement_line",
        "pub(crate) fn parse_enter_as_copy_as_enters_line",
    );
    assert!(
        consumer
            .contains("keyword_static_lines::parse_minimum_red_noncombat_damage_tokens(&tokens)"),
        "{family_relative} should consume the typed minimum-damage recognition result"
    );
    for forbidden in [
        "LexedClause",
        "TokenWordView",
        "token_word_refs",
        "word_slice_",
        "ClauseShape",
        "find_phrase",
        "noncombat damage less than",
    ] {
        assert!(
            !consumer.contains(forbidden),
            "{family_relative} should not rediscover the minimum-damage sentence after grammar recognition: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_draw_replacement_shape_gates_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_draw_replacement_exile_top_and_play_line",
        "pub(crate) fn parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line",
    );

    for required in [
        "late_static_facts::parse_draw_replacement_exile_top_and_play_count(tokens)",
        "StaticAbility::draw_replacement_exile_top_and_play(",
        "count,",
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
pub(super) fn keyword_static_exile_replacement_shape_gates_use_clause_shapes() {
    let root = workspace_root();
    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/keyword_static_lines/exile_replacement_shapes.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    for required in [
        "pub(crate) struct ExileToGraveyardReplacementSpec<'a>",
        "pub(crate) enum ExileWouldDieSpec",
        "pub(crate) fn parse_exile_to_graveyard_replacement_tokens",
        "pub(crate) fn parse_exile_would_die_tokens",
        "primitives::parse_all(",
        "parse_exile_to_graveyard_replacement_lexed",
        "parse_nontoken_exile_would_die_lexed",
        "parse_damaged_by_exile_would_die_lexed",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should own typed exile-replacement recognition: missing `{required}`"
        );
    }
    let graveyard_recognizer = function_source(
        &grammar,
        "fn parse_exile_to_graveyard_replacement_lexed",
        "fn classify_exile_graveyard_filter",
    );
    for required in [
        "WResult<ExileToGraveyardReplacementSpec<'a>>",
        "repeat_till",
        "peek(primitives::phrase(&[\"would\", \"be\", \"put\", \"into\"]))",
        "parse_graveyard_owner_lexed",
        "primitives::kw(\"instead\")",
        "primitives::sentence_end()",
    ] {
        assert!(
            graveyard_recognizer.contains(required),
            "{grammar_relative} should recognize the complete graveyard-replacement shape with winnow: missing `{required}`"
        );
    }

    let family_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let family = read_repo_file(&root, family_relative);
    let graveyard_consumer = function_source(
        &family,
        "pub(crate) fn parse_exile_to_exile_instead_of_graveyard_line",
        "pub(crate) fn parse_exile_would_die_instead_line",
    );
    let would_die_consumer = function_source(
        &family,
        "pub(crate) fn parse_exile_would_die_instead_line",
        "pub(crate) fn parse_pay_life_or_enter_tapped_line",
    );
    for required in [
        "keyword_static_lines::parse_exile_to_graveyard_replacement_tokens(tokens)",
        "spec.graveyard_owner",
        "spec.filter_kind",
        "spec.exclude_cycled",
    ] {
        assert!(
            graveyard_consumer.contains(required),
            "{family_relative} should lower the typed graveyard-replacement spec: missing `{required}`"
        );
    }
    assert!(
        would_die_consumer.contains("keyword_static_lines::parse_exile_would_die_tokens(tokens)")
            && would_die_consumer.contains("keyword_static_lines::ExileWouldDieSpec::"),
        "{family_relative} should lower typed would-die replacement variants"
    );
    for forbidden in [
        "LexedClause",
        "TokenWordView",
        "token_word_refs",
        "word_slice_",
        "find_phrase",
        "ClauseShape",
        ".matches_words(",
        "would die",
        "instead of graveyard",
    ] {
        assert!(
            !graveyard_consumer.contains(forbidden) && !would_die_consumer.contains(forbidden),
            "{family_relative} should not rediscover exile-replacement Oracle text after typed parsing: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_cost_target_specs_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_this_spell_cost_condition",
        "fn parse_conjoined_this_spell_cost_condition",
    );

    for required in [
        "static_mid_facts::parse_known_spell_cost_condition(tokens)",
        "Fact::Target(target)",
        "static_mid_facts::CostTargetFact::You",
        "static_mid_facts::CostTargetFact::Opponent",
        "static_mid_facts::CostTargetFact::AnyPlayer",
        "static_mid_facts::CostTargetFact::Object(filter)",
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
pub(super) fn keyword_static_this_spell_cost_condition_quantity_tails_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_this_spell_cost_condition",
        "fn parse_conjoined_this_spell_cost_condition",
    );

    for required in [
        "static_mid_facts::parse_known_spell_cost_condition(tokens)",
        "Fact::OpponentHasPoisonCountersOrMore(count)",
        "Fact::OpponentHasCardsInGraveyardOrMore(count)",
        "Fact::OpponentControlsLandsOrMore(count)",
        "Fact::OpponentControlsMoreCreaturesThanYou(count)",
        "Fact::TotalCreatureCardsInAllGraveyardsOrMore(count)",
        "Fact::OpponentCastSpellsThisTurnOrMore(count)",
        "Fact::OpponentDrewCardsThisTurnOrMore(count)",
        "Fact::YouWereDealtDamageByCreaturesThisTurnOrMore(count)",
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
pub(super) fn keyword_static_all_creatures_are_color_uses_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_all_creatures_are_color_line",
        "pub(crate) fn parse_subjects_are_basic_line",
    );

    for required in [
        "type_and_color_facts::parse_subject_color_tokens(tokens)",
        "parse_object_filter_lexed(fact.subject_tokens, false)",
        "StaticAbility::set_colors(filter, fact.color)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should keep subject and color boundaries in the lexed token coordinate space: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs",
        "word_slice_",
        ".matches_words(",
        "raw_line",
        "normalized_text",
        "split_once",
        "str_contains",
        "str_starts_with",
        "let subject_tokens = &tokens[..are_idx]",
        "parse_object_filter(subject_tokens, false)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rediscover color-assignment structure through raw text or word-as-token offsets: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_subjects_are_basic_uses_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_subjects_are_basic_line",
        "pub(crate) fn parse_nonbasic_lands_are_basic_land_type_line",
    );

    for required in [
        "type_and_color_facts::parse_subjects_are_basic_tokens(tokens)",
        "split_lexed_slices_on_and(fact.subject_tokens)",
        "parse_object_filter_lexed(fact.subject_tokens, false)",
        "StaticAbility::add_supertypes(",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should retain typed token ranges through conjunction and object-filter lowering: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs",
        "word_slice_",
        ".matches_words(",
        "raw_line",
        "normalized_text",
        "split_once",
        "str_contains",
        "str_starts_with",
        "let subject_tokens = trim_lexed_commas(&tokens[..be_idx])",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not reconstruct subjects-are-basic structure from raw text or detached word vectors: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_land_type_parsers_use_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_nonbasic_lands_are_basic_land_type_line",
        "pub(crate) fn parse_lands_are_pt_creatures_still_lands_line",
    );

    for required in [
        "type_and_color_facts::parse_basic_land_subtype_tokens(tokens)",
        "parse_object_filter_lexed(fact.subject_tokens, false)",
        "StaticAbility::set_land_subtypes(",
        "type_and_color_facts::parse_land_type_addition_tokens(tokens)",
        "type_and_color_facts::LandTypeAdditionFact::EveryBasic",
        "type_and_color_facts::LandTypeAdditionFact::One",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve lexed subject/subtype/tail boundaries for land-type statics: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs",
        "word_slice_",
        ".matches_words(",
        "raw_line",
        "normalized_text",
        "split_once",
        "str_contains",
        "str_starts_with",
        "let subtype_words = &words[subtype_idx..]",
        "let subject_tokens = &tokens[..be_idx]",
        "let filter_tokens = &tokens[..be_idx]",
        "let filter = parse_object_filter(filter_tokens, false)",
        "let filter = parse_object_filter(subject_tokens, false)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not reconstruct land-type statics from raw text or detached word offsets: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_land_animation_uses_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_lands_are_pt_creatures_still_lands_line",
        "pub(crate) fn parse_filter_is_pt_creature_in_addition_and_has_line",
    );

    for required in [
        "type_and_color_facts::parse_land_animation_tokens(tokens)",
        "parse_object_filter_lexed(fact.subject_tokens, false)",
        "StaticAbility::set_base_power_toughness(filter, fact.power, fact.toughness)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should carry land-animation subject and tail boundaries as lexed token ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs",
        "word_slice_",
        ".matches_words(",
        "raw_line",
        "normalized_text",
        "split_once",
        "str_contains",
        "str_starts_with",
        "let filter_tokens = &tokens[..be_idx]",
        "parse_object_filter(filter_tokens, false)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not reconstruct land-animation structure from raw text or word-as-token offsets: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_heterogeneous_animation_attached_probe_uses_token_word_view() {
    let root = workspace_root();
    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/static_keyword_line_shapes.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    for required in [
        "pub(crate) struct AnimationVerbShape",
        "pub(crate) fn parse_animation_verbs",
        "first_token_word(tokens, &[\"is\", \"are\"])",
        "first_token_word(tail, &[\"have\", \"has\"])",
        "pub(crate) fn parse_animation_creature_word",
        "first_word(words, &[\"creature\", \"creatures\"])",
        "let mut input = LexStream::new(tokens)",
        "let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input)",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should own typed animation boundaries via winnow token traversal: missing `{required}`"
        );
    }

    let family_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let family = read_repo_file(&root, family_relative);
    let consumer = function_source(
        &family,
        "pub(crate) fn parse_filter_is_pt_creature_in_addition_and_has_line",
        "pub(crate) fn parse_subject_is_subtype_with_base_pt_and_granted_abilities_line",
    );

    for required in [
        "static_keyword_line_shapes::parse_animation_verbs(tokens)",
        "let be_idx = animation_verbs.be.token",
        "let has_idx = animation_verbs.has.token",
        "static_keyword_line_shapes::parse_animation_creature_word(&before_has_words)",
        "let clause_words = LexedClause::new(tokens).word_refs()",
        "let attached_subject = LexedClause::new(&subject_tokens)",
        "let before_has_clause = LexedClause::new(&before_has)",
        "let raw_before_has_words = before_has_clause.word_refs()",
        "type_and_color_facts::parse_other_type_addition_tail_tokens(",
        ".between_word_range(tail_start_word, tail_end_word)",
        ".map(|tail_clause| tail_clause.tokens())",
    ] {
        assert!(
            consumer.contains(required),
            "{family_relative} should consume typed animation boundaries while retaining token-backed subject ranges: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs",
        "word_slice_find",
        ".matches_words(",
        "raw_line",
        "normalized_text",
        "split_once",
        "str_contains",
        "str_starts_with",
    ] {
        assert!(
            !consumer.contains(forbidden),
            "{family_relative} should not rediscover typed animation boundaries with `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_pay_life_etb_gates_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_pay_life_or_enter_tapped_line",
        "pub(crate) fn parse_copy_activated_abilities_line",
    );

    for required in [
        "late_static_facts::parse_pay_life_or_enter_tapped_tokens(tokens)",
        "late_static_facts::PayLifeOrEnterTappedError::MissingPay",
        "late_static_facts::PayLifeOrEnterTappedError::UnsupportedTail",
        "StaticAbility::pay_life_or_enter_tapped(fact.amount)",
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
pub(super) fn keyword_static_copy_activated_abilities_gates_use_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_copy_activated_abilities_line",
        "pub(crate) fn parse_spend_mana_as_any_color_line",
    );

    for required in [
        "late_static_facts::parse_copy_activated_abilities_tokens(tokens)",
        "tokens[fact.filter_start_token..fact.filter_end_token]",
        "fact.once_each_turn_word_start.is_some()",
        "fact.exclude_source_name",
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
pub(super) fn keyword_static_choose_not_untap_uses_token_word_ranges() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_may_choose_not_to_untap_during_untap_step_line",
        "pub(crate) fn parse_untap_during_each_other_players_untap_step_line",
    );

    for required in [
        "late_static_facts::parse_may_choose_not_untap_tokens(tokens)",
        "parser_token_word_refs(fact.subject_tokens)",
        "fact.simple_source_subject",
        "render_token_slice(fact.subject_tokens)",
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
pub(super) fn keyword_static_search_attack_land_and_retrace_gates_use_clause_shapes() {
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
        "pub(crate) fn parse_cast_spells_from_hand_without_paying_mana_costs_line",
    );

    for required in [
        "late_static_facts::is_control_opponents_while_searching(tokens)",
        "late_static_facts::is_opponent_search_exile_found_cards(tokens)",
        "late_static_facts::is_cast_this_card_from_library_while_searching(tokens)",
        "late_static_facts::parse_attack_each_combat_if_able_tokens(tokens)",
        "late_static_facts::AttackEachCombatFact::AttachedController",
        "late_static_facts::parse_additional_land_play_count(tokens)",
        "late_static_facts::parse_retrace_grant_tokens(tokens)",
        "card_types: fact.card_types",
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
pub(super) fn keyword_static_activation_restriction_wrapper_uses_lexed_spec() {
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
pub(super) fn keyword_static_exile_counter_permission_grant_uses_lexed_ranges() {
    let root = workspace_root();
    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/keyword_static_lines/permission_counter_shapes.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    for required in [
        "pub(crate) enum ExileCounterPermissionFamily",
        "pub(crate) enum ExileCounterPermissionOwner",
        "pub(crate) enum ExileCounterManaPermission",
        "pub(crate) struct ExileCounterPermissionSpec",
        "pub(crate) fn parse_exile_counter_permission_tokens",
        "primitives::parse_all(",
        "parse_cast_countered_exile_cards_lexed",
        "parse_play_source_exiled_countered_cards_lexed",
        "parse_counter_type_before_on_them",
        "parse_countered_exile_mana_permission",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should own a typed, all-consuming exile-counter permission grammar: missing `{required}`"
        );
    }

    let family_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let family = read_repo_file(&root, family_relative);
    let consumer = function_source(
        &family,
        "pub(crate) fn parse_you_may_cast_exile_counter_cards_with_mana_permission_line",
        "pub(crate) fn parse_surveilled_graveyard_play_life_cost_line",
    );

    for required in [
        "keyword_static_lines::parse_exile_counter_permission_tokens(tokens)",
        "spec.family",
        "spec.owner",
        "spec.counter_type",
        "spec.mana_permission",
    ] {
        assert!(
            consumer.contains(required),
            "{family_relative} should lower typed exile-counter permission facts: missing `{required}`"
        );
    }
    for forbidden in [
        "LexedClause",
        "TokenWordView",
        "token_word_refs",
        "parser_token_word_refs",
        "word_slice_",
        "find_phrase",
        "ClauseShape",
        ".matches_words(",
        "spend mana as though",
        "counters on them",
    ] {
        assert!(
            !consumer.contains(forbidden),
            "{family_relative} should not rediscover exile-counter permission Oracle text after typed parsing: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_surveilled_graveyard_permission_uses_clause_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_surveilled_graveyard_play_life_cost_line",
        "pub(crate) fn parse_you_may_static_grant_line",
    );

    assert!(
        parser.contains("late_static_facts::is_surveilled_graveyard_play_life_cost(tokens)"),
        "{relative} should consume the typed surveilled graveyard permission fact"
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
pub(super) fn keyword_static_you_may_static_grant_uses_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_you_may_static_grant_line",
        "pub(crate) fn parse_as_you_cascade_land_drop_line",
    );

    for required in [
        "late_static_facts::is_source_linked_exile_cast_with_any_mana(tokens)",
        "parse_permission_clause_spec(tokens)?",
        "late_static_facts::contains_singular_cast_spell(tokens)",
        "crate::grant::Grantable::AlternativeCast(method)",
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
