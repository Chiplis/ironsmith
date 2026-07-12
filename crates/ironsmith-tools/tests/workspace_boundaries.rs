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
fn compiled_text_renderer_does_not_read_source_only_fields() {
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
fn winnow_leaf_parser_patterns_are_kept_in_repo() {
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
fn compiler_parser_does_not_depend_on_nom() {
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
fn cardinal_recognition_has_one_leaf_grammar_owner() {
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
fn migrated_leaf_namespaces_are_typed_winnow_grammar() {
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
fn migrated_line_families_consume_typed_grammar_results() {
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
fn migrated_effect_families_consume_typed_grammar_results() {
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
fn parser_audits_point_at_runtime_backend() {
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
fn lowering_lower_has_no_raw_text_checks_or_migration_allowlist() {
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
fn document_line_family_handlers_have_no_raw_text_checks() {
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
fn document_partner_parenthetical_trims_use_token_kinds() {
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
fn instead_followup_classifier_uses_tokens_not_raw_oracle_text() {
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
fn labeled_keyword_prefix_preservation_is_front_end_grammar_owned() {
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
fn token_pt_parsing_is_front_end_leaf_grammar_owned() {
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
fn activated_sentence_classification_is_front_end_grammar_owned() {
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
fn activated_display_text_uses_typed_presentation_kind_not_raw_scan() {
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
fn filter_player_relation_core_shapes_use_typed_winnow_grammar() {
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
fn keyword_static_source_is_chosen_color_uses_token_word_view() {
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
fn keyword_static_source_damage_prevention_uses_token_slices() {
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
fn keyword_static_minimum_damage_replacement_prefixes_use_clause_shapes() {
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
fn keyword_static_exile_replacement_shape_gates_use_clause_shapes() {
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
fn keyword_static_cost_target_specs_use_clause_shapes() {
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
fn keyword_static_heterogeneous_animation_attached_probe_uses_token_word_view() {
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
fn keyword_static_exile_counter_permission_grant_uses_lexed_ranges() {
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
fn keyword_static_you_may_static_grant_uses_clause_shapes() {
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
        parser.contains("late_static_facts::parse_play_permission_with_haste_followup(tokens)")
            && parser.contains("parse_permission_clause_spec(permission_sentence)?"),
        "{relative} should consume a typed permission sentence after the grammar validates the haste follow-up"
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
        "let words = LexedClause::new(tokens).word_refs()",
        "early_static_facts::parse_count_as_card_named_shape_words(&words)",
        "words.get(shape.spell_name_words)?",
        "words.get(shape.counted_name_words)?",
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
    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/keyword_static_lines/nearby_primitives.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    let marker = function_source(
        &grammar,
        "pub(crate) fn parse_ward_abilities_dont_trigger_marker_tokens",
        "pub(crate) fn parse_dont_untap_during_controllers_step_tokens",
    );
    for required in [
        "semantic_all(",
        "semantic_phrase(&[\"ward\", \"abilities\", \"of\", \"those\", \"creatures\"])",
        "alt((semantic_kw(\"dont\"), semantic_kw(\"don't\")))",
        "semantic_kw(\"trigger\")",
    ] {
        assert!(
            marker.contains(required),
            "{grammar_relative} should own the multiword ward-suppression marker with typed winnow grammar: missing `{required}`"
        );
    }

    let family_relative =
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_static/mod.rs";
    let family = read_repo_file(&root, family_relative);
    let consumer = function_source(
        &family,
        "pub(crate) fn parse_ward_static_ability_line",
        "pub(crate) fn parse_ward_discard_card_type_cost",
    );

    for required in [
        "tokens.first().is_some_and(|token| token.is_word(\"ward\"))",
        "keyword_static_lines::parse_ward_abilities_dont_trigger_marker_tokens(tokens)",
        "keyword_static_lines::parse_ward_cost_tokens(tokens)",
        "let cost_tokens = trim_commas(ward.cost_tokens)",
        "parse_payment_clause_as_total_cost(&cost_tokens)",
        "render_token_slice(tokens)",
    ] {
        assert!(
            consumer.contains(required),
            "{family_relative} should consume typed ward tokens and lower only their cost/result: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs",
        "parser_token_word_refs",
        "word_slice_",
        ".matches_words(",
        "raw_line",
        "normalized_text",
        "split_once",
        "str_contains",
        "str_starts_with",
        ".join(\" \")",
    ] {
        assert!(
            !consumer.contains(forbidden),
            "{family_relative} should not rediscover ward structure through raw text or detached word vectors: found `{forbidden}`"
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
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/activation_costs/cant_shapes/attack_unless.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn parse_controller_control_requirement_inner",
        "fn parse_minimum_count_lexed",
    );

    assert!(
        helper.contains("conditions::parse_control_condition")
            && helper.contains("conditions::ControlConditionOptions")
            && helper.contains("parsed.has_explicit_quantity()")
            && helper.contains(".at_least_count()")
            && helper.contains("ConditionExpr::PlayerHasAtLeast"),
        "{relative} should parse combat-restriction control tails into a typed condition through the shared capture parser"
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
        "pub(crate) fn parse_this_spell_cost_condition",
        "fn parse_conjoined_this_spell_cost_condition",
    );

    for required in [
        "parse_player_life_change_this_turn_condition(tokens)",
        "this_spell_cost_condition_from_life_change_this_turn",
        "static_mid_facts::parse_known_spell_cost_condition(tokens)",
        "Fact::LifeTotalLessThanStarting",
        "Fact::AttackedThisTurn",
        "Fact::Target(target)",
        "Fact::OpponentControlsLandsOrMore(count)",
        "Fact::AssassinOrCommanderDealtCombatDamage",
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
        "is_core_keyword_marker_text(&text)",
        "early_static_facts::parse_early_keyword_marker_tokens(tokens).is_some()",
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
        "early_static_facts::parse_damage_doubling_mana_value_marker_tokens(tokens)",
        "keyword_static_marker(tokens)",
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
fn sacrifice_filter_article_normalization_is_typed_grammar_owned() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/activation_costs/object_segments.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(crate) fn parse_sacrifice_segment_tokens",
        "pub(crate) fn parse_discard_segment_tokens",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "single-choice sacrifice filter normalization should strip articles through tokens, not raw text prefixes"
    );
    assert!(
        helper.contains("parse_sacrifice_cost_shape_lexed")
            && helper.contains("SacrificeCostShape::Chosen")
            && helper.contains("parse_object_filter_with_grammar_entrypoint_lexed"),
        "{relative} should lower a typed winnow sacrifice shape directly into the cost CST"
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
fn future_zone_replacement_recognizer_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_entry.rs";
    let content = read_repo_file(&root, relative);
    let recognizer = function_source(
        &content,
        "fn future_zone_replacement_from_sentence_tokens",
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
fn sentence_shape_predicates_route_direct_sentence_gates_through_typed_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/sentence_shape_predicates.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "sentence_shapes::parses_cant_gain_life_replacement_tokens(tokens)",
        "sentence_shapes::parse_delayed_sentence_tokens(tokens)",
        "sentence_shapes::parse_quoted_ability_sentence_tokens(tokens)",
        "sentence_shapes::parse_immediate_sacrifice_sentence_tokens(tokens)",
        "sentence_shapes::parse_leading_if_sentence_tokens(tokens)",
        "fn parse_it_is_aura_enchantment_sentence_lexed(",
        "sentence_shapes::parse_aura_enchantment_tokens(tokens)",
        "parse_it_is_aura_enchantment_sentence_lexed(tokens)",
        "sentence_shapes::DelayedSentenceShape::EndOfCombat",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct sentence-shape gates through typed grammar: missing `{required}`"
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
        "SENTENCE_SACRIFICE_COUNTED_PREFIXES",
        "SENTENCE_DELAYED_LIFECYCLE_PHRASES",
        "SENTENCE_AURA_ENCHANT_CREATURE_PREFIX",
        "contains_token_kind(tokens, TokenKind::Quote)",
        "word_slice_find_phrase_start(&sentence_words",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct sentence-shape gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
fn replacement_and_prevention_routes_shape_recognition_through_typed_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser_body = function_source(
        &content,
        "pub(crate) fn parse_monstrosity_sentence",
        "#[cfg(test)]",
    );

    for required in [
        "replacement_grammar::parse_monstrosity_shape(tokens)",
        "replacement_grammar::parse_counter_removed_pump_shape(tokens)",
        "replacement_grammar::parse_token_end_combat_action_shape(tokens)",
        "replacement_grammar::parse_extra_turn_shape(tokens)",
        "replacement_grammar::parse_additional_phases_shape(tokens)",
        "replacement_grammar::parse_split_all_shape(tokens)",
        "replacement_grammar::parse_exile_return_same_shape(tokens)",
        "replacement_grammar::parse_exile_each_target_type_shape(tokens)",
        "replacement_grammar::parse_look_hand_shape(tokens)",
        "replacement_grammar::parse_look_top_exile_one_shape(tokens)",
        "replacement_grammar::parse_voted_with_you_scry_shape(tokens)",
    ] {
        assert!(
            parser_body.contains(required),
            "{relative} should delegate parser-owned recognition to typed grammar: missing `{required}`"
        );
    }

    for forbidden in [
        "LexPattern",
        "LexCapture",
        ".match_clause(",
        ".matches_clause(",
        ".matches_prefix(",
        ".word_refs(",
        "word_slice_",
        "token_slice_",
        "replace_up_to_one_target",
        "strip_lexed_suffix",
        "REPLACE_",
    ] {
        assert!(
            !parser_body.contains(forbidden) && !content.contains(forbidden),
            "{relative} should not rediscover Oracle shapes in the sentence caller via `{forbidden}`"
        );
    }

    for (grammar_relative, required_type) in [
        (
            "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/replacement_prevention_shapes/actions.rs",
            "pub(crate) struct AdditionalPhasesShape",
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/replacement_prevention_shapes/zones.rs",
            "pub(crate) struct ExileReturnSameShape",
        ),
        (
            "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/replacement_prevention_shapes/look.rs",
            "pub(crate) struct LookTopExileOneShape",
        ),
    ] {
        let grammar = read_repo_file(&root, grammar_relative);
        assert!(
            grammar.contains("winnow::") && grammar.contains(required_type),
            "{grammar_relative} should expose typed winnow grammar output `{required_type}`"
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
fn clause_pattern_helpers_delegate_migrated_families_to_typed_grammar() {
    let root = workspace_root();
    let caller_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/clause_pattern_helpers.rs";
    let caller = read_repo_file(&root, caller_relative);
    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/clause_pattern_shapes.rs";
    let grammar = read_repo_file(&root, grammar_relative);

    for required in [
        "clause_shapes::parse_prevent_next_damage_tokens(tokens)",
        "clause_shapes::parse_prevent_next_time_damage_tokens(tokens)",
        "clause_shapes::parse_redirect_next_damage_tokens(tokens)",
        "clause_shapes::parse_counter_ability_target_tokens(tokens)",
        "clause_shapes::parse_keyword_mechanic_tokens(tokens)",
    ] {
        assert!(
            caller.contains(required),
            "{caller_relative} should consume the typed clause grammar result `{required}`"
        );
    }
    for required in [
        "mod counter_ability;",
        "mod damage;",
        "mod keywords;",
        "pub(crate) use counter_ability::*;",
        "pub(crate) use damage::*;",
        "pub(crate) use keywords::*;",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should expose migrated typed clause grammar: missing `{required}`"
        );
    }
    let redirect = function_source(
        &caller,
        "pub(crate) fn parse_redirect_next_damage_sentence",
        "pub(crate) fn parse_can_block_additional_creature_this_turn_clause",
    );
    for forbidden in [
        ".starts_with(&[",
        "find_phrase_start",
        "CLAUSE_REDIRECT_DAMAGE_PREFIX_PATTERN",
        "CLAUSE_THAT_DAMAGE_IS_DEALT_TO_PREFIX_PATTERN",
    ] {
        assert!(
            !redirect.contains(forbidden),
            "{caller_relative} should not rediscover redirect Oracle shape with `{forbidden}`"
        );
    }
}

#[test]
fn prevent_all_damage_clause_parser_uses_clause_shapes() {
    let root = workspace_root();
    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/clause_pattern_shapes/typed_clauses.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    for required in [
        "pub(crate) enum PreventAllDamageSourceShape<'a>",
        "pub(crate) enum PreventAllDamageShape<'a>",
        "FromSource",
        "ToTarget",
        "ToTargetFromSource",
        "fn parse_duration_first_source",
        "fn parse_duration_first_target",
        "fn parse_target_first_source",
        "fn parse_target_first",
        "pub(crate) fn parse_prevent_all_damage_shape_tokens",
        "primitives::parse_all(",
        "repeat_till",
        "primitives::sentence_end()",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should own typed prevent-all-damage variants with all-consuming winnow parsers: missing `{required}`"
        );
    }

    let caller_relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/clause_pattern_helpers.rs";
    let caller = read_repo_file(&root, caller_relative);
    let consumer = function_source(
        &caller,
        "pub(crate) fn parse_prevent_all_damage_clause",
        "pub(crate) fn parse_can_attack_as_though_no_defender_clause",
    );
    for required in [
        "clause_shapes::parse_prevent_all_damage_shape_tokens(tokens)",
        "clause_shapes::PreventAllDamageShape::FromSource",
        "clause_shapes::PreventAllDamageShape::ToTarget",
        "clause_shapes::PreventAllDamageShape::ToTargetFromSource",
        "clause_shapes::PreventAllDamageSourceShape::Choice",
        "clause_shapes::PreventAllDamageSourceShape::Filter",
    ] {
        assert!(
            consumer.contains(required),
            "{caller_relative} should lower typed prevent-all-damage variants: missing `{required}`"
        );
    }
    for forbidden in [
        "classify_prevent_all_damage_clause",
        "CLAUSE_PREVENT_ALL_DAMAGE",
        "CLAUSE_SOURCES_SUFFIX",
        "CLAUSE_THIS_TURN_PATTERN",
        "token_word_refs",
        "word_slice_",
        ".starts_with(",
        ".ends_with(",
        ".matches_words(",
        "split_once",
    ] {
        assert!(
            !consumer.contains(forbidden),
            "{caller_relative} should not rediscover prevent-all-damage Oracle shapes after typed parsing: found `{forbidden}`"
        );
    }
}

#[test]
fn keyword_payload_additional_cost_recognition_uses_lexed_tail_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/families/keyword_payloads.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(super) fn parse_additional_cost(",
        "pub(super) fn parse_alternative_cast(",
    );

    assert!(
        helper.contains("additional_cost_tail_tokens_lexed(tokens)"),
        "{relative} should recognize additional-cost effects from parse tokens"
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

    let registry = read_repo_file(
        &root,
        "crates/ironsmith-compiler/src/runtime_backend/families/keyword_registry.rs",
    );
    assert!(
        registry.contains("(rule.parse)(line, &tokens, &full_parse_tokens)?")
            && registry.contains("payload.to_line_ast()"),
        "keyword recognition must carry its typed payload through CST instead of pairing a boolean match with a lowering reparse"
    );
    assert!(
        !registry.contains("__split_kicker_label:"),
        "split kicker labels must use the typed keyword payload"
    );
}

#[test]
fn triggered_label_source_selection_uses_lexed_dash_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/document/mod.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn trigger_presentation_from_line_tokens",
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

    let legacy_lowering_relative =
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/parser_semantic_lowering.rs";
    assert!(
        !root.join(legacy_lowering_relative).exists(),
        "trigger presentation recognition must not return to deleted parser-owned lowering module {legacy_lowering_relative}"
    );

    let semantic_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/semantic_line_parsing/lines.rs";
    let semantic = read_repo_file(&root, semantic_relative);
    assert!(
        !semantic.contains("presentation_label_from_raw_trigger_line")
            && !semantic.contains(".or_else(|| presentation_label_from_raw"),
        "{semantic_relative} should consume trigger presentation facts from CST/IR, not re-read raw Oracle text"
    );
}

#[test]
fn chosen_option_context_flow_uses_typed_cst_ir_fact() {
    let root = workspace_root();
    let ir_relative = "crates/ironsmith-compiler/src/runtime_backend/model/ir.rs";
    let ir = read_repo_file(&root, ir_relative);
    assert!(
        ir.contains("enum ChosenOptionContext")
            && ir.contains("StationThreshold(i32)")
            && ir.contains("ControlsSubtypePermanent(Subtype)")
            && ir.contains("ControlsEitherColorPermanent"),
        "{ir_relative} should carry typed chosen-option and threshold facts"
    );

    let relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/semantic_line_parsing/chosen_options.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(crate) fn condition_for_chosen_option",
        "pub(crate) fn wrap_chosen_option_static_chunk",
    );

    assert!(
        helper.contains("ChosenOptionContext::")
            && !helper.contains("strip_prefix")
            && !helper.contains("split_once")
            && !helper.contains("raw_line"),
        "{relative} should consume typed chosen-option contexts without decoding label strings"
    );

    for lowering_relative in [
        "crates/ironsmith-compiler/src/runtime_backend/front_end/semantic_line_parsing/lines.rs",
        "crates/ironsmith-compiler/src/runtime_backend/front_end/semantic_line_parsing/activated.rs",
        "crates/ironsmith-compiler/src/runtime_backend/lowering/lower/rewrite_text_helpers.rs",
    ] {
        let lowering = read_repo_file(&root, lowering_relative);
        assert!(
            !lowering.contains("__max_speed_condition")
                && !lowering.contains("__station_threshold_")
                && !lowering.contains("__control_subtype_permanent_")
                && !lowering.contains("__control_color_pair_permanent_"),
            "{lowering_relative} should not decode chosen-option semantics from magic strings"
        );
    }
}

#[test]
fn partner_parenthetical_trims_are_typed_grammar_owned() {
    let root = workspace_root();
    let grammar_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/keyword_special_lines.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    let entry = function_source(
        &grammar,
        "pub(crate) fn parse_partner_with_name_shape_tokens",
        "pub(crate) fn parse_partner_visible_label_tokens",
    );
    let recognizer = function_source(
        &grammar,
        "fn parse_partner_with_name_shape_lexed",
        "fn parse_partner_visible_label_lexed",
    );

    for required in [
        "pub(crate) struct PartnerWithNameShape<'a>",
        "pub(crate) name_tokens: &'a [OwnedLexToken]",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should define a typed partner-name capture: missing `{required}`"
        );
    }
    for required in [
        "primitives::parse_prefix(tokens, parse_partner_with_name_shape_lexed)",
        "Some(PartnerWithNameShape { name_tokens })",
        "render_token_slice(shape.name_tokens)",
    ] {
        assert!(
            entry.contains(required),
            "{grammar_relative} should expose the captured partner-name token range as a typed grammar result: missing `{required}`"
        );
    }
    for required in [
        "WResult<&'a [OwnedLexToken]>",
        "primitives::phrase(&[\"partner\", \"with\"])",
        "repeat_till",
        "primitives::token_kind(TokenKind::LParen)",
        "primitives::token_kind(TokenKind::Period)",
        "eof.value(())",
    ] {
        assert!(
            recognizer.contains(required),
            "{grammar_relative} should own partner parenthetical/terminal boundaries with winnow token kinds: missing `{required}`"
        );
    }
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
            !entry.contains(forbidden) && !recognizer.contains(forbidden),
            "{grammar_relative} should not trim partner parentheticals with raw string branch `{forbidden}`"
        );
    }

    let semantic_relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/semantic_line_parsing/lines.rs";
    let semantic = read_repo_file(&root, semantic_relative);
    let adapter = function_source(
        &semantic,
        "fn try_lower_partner_with_tokens",
        "pub(crate) fn try_parse_optional_cost_with_cast_trigger",
    );
    assert!(
        adapter.contains("keyword_special_grammar::parse_partner_with_name_tokens(tokens)"),
        "{semantic_relative} should consume the typed partner-name grammar result"
    );
    for forbidden in [
        "TokenKind::LParen",
        "split_once",
        "str_split_once_char",
        "trim_end_matches",
        "raw_line",
        "normalized_text",
    ] {
        assert!(
            !adapter.contains(forbidden),
            "{semantic_relative} should not rediscover partner-name boundaries after typed parsing: found `{forbidden}`"
        );
    }
}

#[test]
fn semantic_line_hideaway_special_case_uses_token_words() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler/src/runtime_backend/front_end/semantic_line_parsing/lines.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn try_lower_hideaway_keyword",
        "fn hideaway_line_ast",
    );

    assert!(
        helper.contains("try_lower_hideaway_tokens(parse_tokens")
            && helper.contains("semantic_grammar::parse_hideaway_keyword_tokens(parse_tokens)?")
            && helper.contains("hideaway_line_ast(shape.count)"),
        "{relative} should lower hideaway from the typed grammar capture"
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
fn clause_dispatch_routes_shape_recognition_through_typed_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/clause_dispatch.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "grammar::effects::clause_dispatch_shapes as clause_grammar",
        "clause_grammar::parse_clause_subject_verb_shape(tokens)",
        "clause_grammar::parse_direct_clause_shape(tokens)",
        "clause_grammar::parse_pump_subject_shape(subject_tokens)",
        "clause_grammar::parse_cast_any_tagged_shape(tokens)",
        "clause_grammar::parse_passive_sacrifice_shape(tokens)",
        "clause_grammar::parse_hexproof_targeting_override_shape(&clause_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should lower typed clause grammar results: missing `{required}`"
        );
    }

    for forbidden in [
        "clause_shape!",
        "LexPattern",
        ".matches_word(",
        ".matches_token(",
        "strip_leading_pump_subject_duration",
        "dispatch_words_eq",
        "rest_starts_all_abilities_shared_gain",
        "is_tagged_object_reference",
        "CAST_ANY_NUMBER_OF_SPELLS_PREFIX",
        "RING_TEMPTS_YOU_WORDS",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rediscover migrated clause shapes: found `{forbidden}`"
        );
    }

    for grammar_relative in [
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/clause_dispatch_shapes/core.rs",
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/clause_dispatch_shapes/direct.rs",
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/clause_dispatch_shapes/permissions.rs",
        "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/clause_dispatch_shapes/relational.rs",
    ] {
        let grammar_content = read_repo_file(&root, grammar_relative);
        assert!(
            grammar_content.contains("winnow::"),
            "{grammar_relative} should recognize clause shapes with winnow"
        );
        assert!(
            !grammar_content.contains("nom::"),
            "{grammar_relative} must not introduce nom"
        );
    }
}

#[test]
fn return_exchange_routes_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/return_exchange.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "grammar::effects::parse_return_clause_shape(tokens)",
        "grammar::effects::ReturnTargetShape::All",
        "grammar::effects::ReturnTargetShape::Singular",
        "grammar::effects::parse_exchange_clause_shape(tokens)",
        "ExchangeClauseShape::LifeTotalsWith",
        "ExchangeClauseShape::Values",
        "grammar::effects::parse_exchange_value_operands",
        "grammar::effects::parse_return_timing_words_shape(words)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route return/exchange gates through typed grammar shapes: missing `{required}`"
        );
    }

    for forbidden in [
        "fn return_shape_matches_words(",
        "return_find_phrase_start",
        "return_find_prefix_start",
        "return_word_is_any",
        "word_slice_eq(",
        "word_slice_eq_any(",
        "words_start_with(",
        "words_start_with_any(",
        "locate_index(",
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
fn counter_marker_family_routes_shape_gates_through_typed_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/counter_marker_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "grammar::effects::counter_marker_shapes as counter_shapes",
        "counter_shapes::parse_return_with_counters_tokens(clause.tokens())",
        "counter_shapes::parse_put_onto_battlefield_with_counters_tokens(clause.tokens())",
        "counter_shapes::parse_if_enters_additional_tokens(clause.tokens())",
        "counter_shapes::parse_tagged_enters_additional_tokens(clause.tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route counter-marker shape gates through typed grammar: missing `{required}`"
        );
    }

    for forbidden in [
        "LexPattern",
        "LexCaptureRole",
        "counter_marker_control_tail_controller",
        "counter_marker_matches_accepted_target",
        "word_slice_eq_any",
        ".matches_words(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not retain parser-shape helpers after typed grammar migration: found `{forbidden}`"
        );
    }

    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/counter_marker_shapes.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    for required in [
        "struct CounterDescriptorShape",
        "struct MoveWithCountersShape",
        "enum CounterMarkerTimingShape",
        "fn parse_return_with_counters_lexed",
        "fn parse_if_enters_additional_lexed",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should own typed counter-marker recognition: missing `{required}`"
        );
    }
}

#[test]
fn sentence_shape_predicates_route_shape_gates_through_typed_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/dispatch_inner/sentence_shape_predicates.rs";
    let content = read_repo_file(&root, relative);
    let grammar_relative = "crates/ironsmith-compiler/src/runtime_backend/front_end/grammar/effects/sentence_predicate_shapes.rs";
    let grammar = read_repo_file(&root, grammar_relative);

    for required in [
        "sentence_shapes::parse_trailing_counter_constraint_tokens(tokens)",
        "sentence_shapes::parse_power_damage_self_tokens(tokens)",
        "sentence_shapes::parse_tapped_this_way_binding_tokens",
        "sentence_shapes::parse_where_x_sentence_tokens(tokens)",
        "sentence_shapes::parse_where_x_value_shape_tokens",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route sentence-shape predicates through typed grammar: missing `{required}`"
        );
    }

    for required in [
        "enum WhereXValueShape",
        "struct WhereXSentenceShape",
        "struct AuraEnchantmentShape",
        "enum DelayedSentenceShape",
        "fn parse_where_x_value_shape_tokens",
        "use winnow::",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should own typed sentence-shape recognition: missing `{required}`"
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
        "SENTENCE_WHERE_X_IS_PREFIX",
        "sentence_removed_counters_this_way",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route sentence-shape predicates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
fn token_copy_control_uses_typed_grammar_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/runtime_backend/sentences/effect_sentences/subject_verb_primitives/token_copy_control_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "effect_grammar::parse_each_player_reveal_permanents_shape(clause.tokens())",
        "effect_grammar::parse_return_same_subtypes_shape(clause.tokens())",
        "effect_grammar::parse_choose_same_filter_shape(clause.tokens())",
        "effect_grammar::parse_choose_sequence_shape(clause.tokens())",
        "effect_grammar::parse_sacrifice_choice_shape(clause.tokens())",
        "effect_grammar::parse_then_sequence_shape(clause.tokens())",
        "effect_grammar::parse_return_create_shape(clause.tokens())",
        "effect_grammar::parse_exile_may_put_shape(clause.tokens())",
        "effect_grammar::parse_exile_shuffle_shape(clause.tokens())",
        "effect_grammar::parse_exile_source_counter_shape(clause.tokens())",
        "effect_grammar::parse_comma_then_special_shape(clause.tokens())",
        "effect_grammar::parse_destroy_land_damage_shape(clause.tokens())",
        "effect_grammar::parse_destroy_attached_shape(clause.tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should lower typed token/copy/control grammar facts: missing `{required}`"
        );
    }

    for forbidden in [
        "LexPattern",
        "LexCapture",
        ".match_pattern(",
        ".word_refs()",
        "word_slice_",
        "words_start_with(",
        "find_phrase_start(",
        "rfind_token_word(",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not retain parser-shape probe `{forbidden}`"
        );
    }
}
