use std::fs;

mod tooling_paths;

#[derive(Clone, Copy)]
struct Budget {
    path: &'static str,
    max_lines: usize,
}

const RUNTIME_BACKEND: &str = "crates/ironsmith-compiler/src/runtime_backend";

fn main() {
    let repo_root = tooling_paths::repo_root()
        .unwrap_or_else(|err| panic!("failed to locate repo root: {err}"));
    let parser_root = repo_root.join(RUNTIME_BACKEND);
    let budgets = [
        Budget {
            path: "families/activation_and_restrictions/mod.rs",
            max_lines: 900,
        },
        Budget {
            path: "families/activation_and_restrictions/activated_sentence_parsers.rs",
            max_lines: 2500,
        },
        Budget {
            path: "families/activation_and_restrictions/keyword_activated_lines.rs",
            max_lines: 1200,
        },
        Budget {
            path: "families/activation_and_restrictions/activated_line_core.rs",
            max_lines: 1600,
        },
        Budget {
            path: "families/activation_and_restrictions/activation_costs.rs",
            max_lines: 2300,
        },
        Budget {
            path: "families/activation_and_restrictions/activation_restriction_clauses.rs",
            max_lines: 2800,
        },
        Budget {
            path: "families/activation_and_restrictions/keyword_action_costs.rs",
            max_lines: 2400,
        },
        Budget {
            path: "families/keyword_static/attached_object_static_lines.rs",
            max_lines: 1500,
        },
        Budget {
            path: "families/activation_and_restrictions/trigger_clause_core.rs",
            max_lines: 5400,
        },
        Budget {
            path: "families/activation_and_restrictions/trigger_subject_filters.rs",
            max_lines: 2500,
        },
        Budget {
            path: "families/activation_and_restrictions/choice_object_clauses.rs",
            max_lines: 1050,
        },
        Budget {
            path: "families/activation_helpers.rs",
            max_lines: 500,
        },
        Budget {
            path: "families/keyword_families.rs",
            max_lines: 400,
        },
        Budget {
            path: "families/keyword_registry.rs",
            max_lines: 150,
        },
        Budget {
            path: "families/keyword_payloads.rs",
            max_lines: 625,
        },
        Budget {
            path: "families/permission_helpers.rs",
            max_lines: 3200,
        },
        Budget {
            path: "sentences/effect_sentences/dispatch_inner/generic_subject_verb_programs.rs",
            max_lines: 3600,
        },
        Budget {
            path: "sentences/effect_sentences/chain_carry.rs",
            max_lines: 3000,
        },
        Budget {
            path: "sentences/effect_sentences/lex_chain_helpers.rs",
            max_lines: 250,
        },
        Budget {
            path: "sentences/effect_sentences/search_library.rs",
            max_lines: 850,
        },
        Budget {
            path: "sentences/effect_sentences/sacrifice_discard.rs",
            max_lines: 575,
        },
        Budget {
            path: "sentences/effect_sentences/clause_dispatch/become_clause.rs",
            max_lines: 425,
        },
        Budget {
            path: "sentences/effect_sentences/clause_dispatch.rs",
            max_lines: 1800,
        },
        Budget {
            path: "sentences/effect_sentences/looked_cards_family.rs",
            max_lines: 200,
        },
        Budget {
            path: "sentences/effect_sentences/misc_actions.rs",
            max_lines: 825,
        },
        Budget {
            path: "sentences/effect_sentences/emblem_actions.rs",
            max_lines: 175,
        },
        Budget {
            path: "sentences/effect_sentences/gain_ability.rs",
            max_lines: 2650,
        },
        Budget {
            path: "sentences/effect_sentences/clause_pattern_helpers.rs",
            max_lines: 2150,
        },
        Budget {
            path: "sentences/effect_sentences/dispatch_entry/subject_verb_followups.rs",
            max_lines: 1600,
        },
        Budget {
            path: "sentences/effect_sentences/return_exchange.rs",
            max_lines: 500,
        },
        Budget {
            path: "sentences/effect_sentences/sequence_rules/generic_subject_verb_sequences/triples.rs",
            max_lines: 3700,
        },
        Budget {
            path: "front_end/grammar/leaf.rs",
            max_lines: 700,
        },
        Budget {
            path: "front_end/grammar/permission_shapes.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/leaf/activation_heads.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/leaf/articles.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/leaf/common.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/leaf/casting.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/leaf/condition_prefixes.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/leaf/counts.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/leaf/durations.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/leaf/filter_atoms.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/leaf/mana.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/leaf/numbers.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/leaf/player_subjects.rs",
            max_lines: 600,
        },
        Budget {
            path: "front_end/grammar/leaf/power_toughness.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/leaf/references.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/leaf/source_references.rs",
            max_lines: 700,
        },
        Budget {
            path: "front_end/grammar/leaf/targets.rs",
            max_lines: 800,
        },
        Budget {
            path: "front_end/grammar/targets.rs",
            max_lines: 600,
        },
        Budget {
            path: "front_end/grammar/targets/shapes.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/abilities/flashback.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/abilities/spell_countered_trigger.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/abilities.rs",
            max_lines: 2200,
        },
        Budget {
            path: "front_end/grammar/abilities/surface.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/abilities/static_shapes.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/abilities/activation_conditions.rs",
            max_lines: 650,
        },
        Budget {
            path: "front_end/grammar/abilities/mana_usage.rs",
            max_lines: 850,
        },
        Budget {
            path: "front_end/grammar/activation_costs.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/activation_costs/simple_segments.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/activation_costs/cant_shapes.rs",
            max_lines: 650,
        },
        Budget {
            path: "front_end/grammar/activation_costs/selectors.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/activation_costs/counter_segments.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/activation_costs/zone_segments.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/activation_costs/object_segments.rs",
            max_lines: 650,
        },
        Budget {
            path: "front_end/grammar/activation_costs/exile_segments.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/activation_restrictions.rs",
            max_lines: 700,
        },
        Budget {
            path: "front_end/grammar/activation_restrictions/surface_shapes.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/trigger_subjects.rs",
            max_lines: 700,
        },
        Budget {
            path: "front_end/grammar/trigger_subjects/surface_shapes.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/trigger_clauses.rs",
            max_lines: 750,
        },
        Budget {
            path: "front_end/grammar/trigger_clauses/surface_patterns.rs",
            max_lines: 325,
        },
        Budget {
            path: "front_end/grammar/choices.rs",
            max_lines: 750,
        },
        Budget {
            path: "front_end/grammar/choices/object_shapes.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/choices/type_phrases.rs",
            max_lines: 325,
        },
        Budget {
            path: "front_end/grammar/choices/sequence_shapes.rs",
            max_lines: 450,
        },
        Budget {
            path: "front_end/grammar/keyword_action_costs.rs",
            max_lines: 700,
        },
        Budget {
            path: "front_end/grammar/keyword_dispatch.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/keyword_special_lines.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/splice_keyword_lines.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/modal.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/keyword_action_costs/semantic_shapes.rs",
            max_lines: 450,
        },
        Budget {
            path: "front_end/grammar/keyword_activated_lines.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/keyword_activated_lines/craft.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/keyword_activated_lines/cycling.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/keyword_activated_lines/equip.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/keyword_activated_lines/simple.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/activated_lines.rs",
            max_lines: 800,
        },
        Budget {
            path: "front_end/grammar/activated_lowering.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/semantic_line_parsing/mod.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/semantic_line_parsing/activated.rs",
            max_lines: 725,
        },
        Budget {
            path: "front_end/semantic_line_parsing/lines.rs",
            max_lines: 2600,
        },
        Budget {
            path: "front_end/grammar/sentence_markers.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/activated_lines/x_and_loyalty_facts.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/activated_lines/blocking_and_cycling.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/anthem_grants.rs",
            max_lines: 2000,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/addition_shapes.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/clause_shapes.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/condition_quantities.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/condition_shapes.rs",
            max_lines: 875,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/continuing_shapes.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/count_shapes.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/granted_tail_shapes.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/anthem_keyword_shapes.rs",
            max_lines: 550,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/tail_static_shapes.rs",
            max_lines: 450,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/misc_shapes.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/anthem_grants/soulbond_shapes.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/activation_helpers.rs",
            max_lines: 600,
        },
        Budget {
            path: "front_end/grammar/restriction_normalization.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/restriction_facts.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/semantic_lowering.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/semantic_lowering/keyword_shapes.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/semantic_lowering/statement_shapes.rs",
            max_lines: 750,
        },
        Budget {
            path: "front_end/grammar/semantic_lowering/static_shapes.rs",
            max_lines: 375,
        },
        Budget {
            path: "front_end/grammar/semantic_lowering/special_triggered_programs.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/semantic_lowering/special_triggered_programs/tests.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/semantic_lowering/triggered_shapes.rs",
            max_lines: 375,
        },
        Budget {
            path: "front_end/grammar/effects/creation_shapes.rs",
            max_lines: 50,
        },
        Budget {
            path: "front_end/grammar/effects/chain_carry.rs",
            max_lines: 800,
        },
        Budget {
            path: "front_end/grammar/effects/chain_splitting.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/effects/chain_splitting/recognition.rs",
            max_lines: 1000,
        },
        Budget {
            path: "front_end/grammar/effects/chain_splitting/split_rules.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/effects/chain_splitting/verbs.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/effects/combat_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/combat_shapes/attachments.rs",
            max_lines: 275,
        },
        Budget {
            path: "front_end/grammar/effects/combat_shapes/conditions.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/effects/combat_shapes/damage.rs",
            max_lines: 750,
        },
        Budget {
            path: "front_end/grammar/effects/combat_damage_family_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/combat_damage_family_shapes/creature_types.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/combat_damage_family_shapes/for_each.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/effects/combat_damage_family_shapes/returns.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/effects/combat_damage_family_shapes/stickers.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/creation_shapes/copy_modifiers.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/effects/creation_shapes/counts.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/effects/creation_shapes/surface.rs",
            max_lines: 675,
        },
        Budget {
            path: "front_end/grammar/effects/creation_shapes/token_shapes.rs",
            max_lines: 525,
        },
        Budget {
            path: "front_end/grammar/static_line_support.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/statement_shapes.rs",
            max_lines: 275,
        },
        Budget {
            path: "front_end/grammar/token_definitions.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/token_definitions/common.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/token_definitions/equipment.rs",
            max_lines: 275,
        },
        Budget {
            path: "front_end/grammar/token_definitions/equipment_compat.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/token_definitions/names.rs",
            max_lines: 525,
        },
        Budget {
            path: "front_end/grammar/token_definitions/reminder.rs",
            max_lines: 275,
        },
        Budget {
            path: "front_end/grammar/token_definitions/reminder_merge.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/token_definitions/rules.rs",
            max_lines: 425,
        },
        Budget {
            path: "front_end/grammar/token_definitions/surface.rs",
            max_lines: 650,
        },
        Budget {
            path: "front_end/grammar/static_keyword_shapes.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/static_keyword_line_shapes.rs",
            max_lines: 600,
        },
        Budget {
            path: "front_end/grammar/static_keyword_cost_shapes.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/static_keyword_replacement_shapes.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/clause_support.rs",
            max_lines: 600,
        },
        Budget {
            path: "front_end/grammar/clause_support/ability_shapes.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/conditions.rs",
            max_lines: 2200,
        },
        Budget {
            path: "front_end/grammar/conditions/event_shapes.rs",
            max_lines: 750,
        },
        Budget {
            path: "front_end/grammar/conditions/relation_shapes.rs",
            max_lines: 450,
        },
        Budget {
            path: "front_end/grammar/conditions/status_shapes.rs",
            max_lines: 550,
        },
        Budget {
            path: "front_end/grammar/conditions/zone_change_shapes.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/document_shapes.rs",
            max_lines: 550,
        },
        Budget {
            path: "front_end/grammar/document_shapes/labels.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/document_shapes/choice_context.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/preprocess.rs",
            max_lines: 50,
        },
        Budget {
            path: "front_end/grammar/preprocess/borrow_shapes.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/preprocess/line_shapes.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/preprocess/name_shapes.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/preprocess/vote_shapes.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/shared_util.rs",
            max_lines: 50,
        },
        Budget {
            path: "front_end/grammar/shared_util/value_shapes.rs",
            max_lines: 425,
        },
        Budget {
            path: "front_end/grammar/shared_util/count_shapes.rs",
            max_lines: 425,
        },
        Budget {
            path: "front_end/grammar/shared_util/alternative_cost_lines.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/shared_util/additional_cost_choices.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/shared_util/cast_restriction_lines.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/shared_util/header_shapes.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/shared_util/keyword_cost_lines.rs",
            max_lines: 475,
        },
        Budget {
            path: "front_end/grammar/shared_util/reference_shapes.rs",
            max_lines: 650,
        },
        Budget {
            path: "front_end/grammar/shared_util/target_surfaces.rs",
            max_lines: 450,
        },
        Budget {
            path: "front_end/grammar/shared_util/target_semantics.rs",
            max_lines: 850,
        },
        Budget {
            path: "front_end/grammar/shared_util/value_expr.rs",
            max_lines: 700,
        },
        Budget {
            path: "front_end/grammar/shared_util/value_helper_shapes.rs",
            max_lines: 325,
        },
        Budget {
            path: "front_end/grammar/effects.rs",
            max_lines: 2125,
        },
        Budget {
            path: "front_end/grammar/effects/bundle_rules.rs",
            max_lines: 900,
        },
        Budget {
            path: "front_end/grammar/effects/bundle_rules/exact.rs",
            max_lines: 650,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs/cloak.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_quad_shapes.rs",
            max_lines: 450,
        },
        Budget {
            path: "front_end/grammar/effects/generic_sequence_shapes.rs",
            max_lines: 600,
        },
        Budget {
            path: "front_end/grammar/effects/sentence_prelude.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/effects/sentence_predicate_shapes.rs",
            max_lines: 1250,
        },
        Budget {
            path: "front_end/grammar/effects/tap_shapes.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs/consult.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs/consult/cast.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs/consult/remainder.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs/consult/traversal.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs/consult/values.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs/copy.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs/library.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs/misc.rs",
            max_lines: 600,
        },
        Budget {
            path: "front_end/grammar/effects/sequence_pairs/residual.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/control.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/effects/conditional_shapes.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/clause_primitive_shapes.rs",
            max_lines: 550,
        },
        Budget {
            path: "front_end/grammar/effects/damage.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/effects/delayed.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/delayed_sentence_shapes.rs",
            max_lines: 525,
        },
        Budget {
            path: "front_end/grammar/effects/delayed_step_shapes.rs",
            max_lines: 550,
        },
        Budget {
            path: "front_end/grammar/effects/divvy_shapes.rs",
            max_lines: 700,
        },
        Budget {
            path: "front_end/grammar/effects/fixed_mana_output.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/fanout_shapes.rs",
            max_lines: 875,
        },
        Budget {
            path: "front_end/grammar/effects/emblem_shapes.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/effects/emblem_shapes/tests.rs",
            max_lines: 75,
        },
        Budget {
            path: "front_end/grammar/effects/exile_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/exile_shapes/bundles.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/exile_shapes/hand_or_permanent.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/effects/exile_shapes/library.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/exile_shapes/owner.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/effects/followup_shapes.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/followup_shapes/regeneration.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/effects/generic_program_shapes.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/effects/generic_program_shapes/choice_complements.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/generic_program_shapes/triggering_spell_damage.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/effects/counter_stat_shapes.rs",
            max_lines: 650,
        },
        Budget {
            path: "front_end/grammar/effects/counter_marker_shapes.rs",
            max_lines: 1200,
        },
        Budget {
            path: "front_end/grammar/effects/choice_damage_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/choice_damage_shapes/common.rs",
            max_lines: 325,
        },
        Budget {
            path: "front_end/grammar/effects/choice_damage_shapes/destroy.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/effects/choice_damage_shapes/sentences.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/clause_pattern_shapes.rs",
            max_lines: 50,
        },
        Budget {
            path: "front_end/grammar/effects/clause_pattern_shapes/counter_ability.rs",
            max_lines: 325,
        },
        Budget {
            path: "front_end/grammar/effects/clause_pattern_shapes/damage.rs",
            max_lines: 650,
        },
        Budget {
            path: "front_end/grammar/effects/clause_pattern_shapes/keywords.rs",
            max_lines: 625,
        },
        Budget {
            path: "front_end/grammar/effects/clause_pattern_shapes/utility.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/effects/clause_pattern_shapes/utility/tests.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/effects/become_shapes.rs",
            max_lines: 325,
        },
        Budget {
            path: "front_end/grammar/effects/become_shapes/descriptors.rs",
            max_lines: 375,
        },
        Budget {
            path: "front_end/grammar/effects/become_shapes/subjects.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/effects/become_shapes/surface.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/line_families.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/effects/gain_life_shapes.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/gain_ability_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/gain_ability_shapes/components.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/gain_ability_shapes/compound.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/effects/gain_ability_shapes/durations.rs",
            max_lines: 275,
        },
        Budget {
            path: "front_end/grammar/effects/gain_ability_shapes/power_toughness.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/gain_ability_shapes/words.rs",
            max_lines: 525,
        },
        Budget {
            path: "front_end/grammar/effects/misc_action_shapes.rs",
            max_lines: 550,
        },
        Budget {
            path: "front_end/grammar/effects/misc_action_shapes/payment_and_untap.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/effects/remove_destroy_shapes.rs",
            max_lines: 725,
        },
        Budget {
            path: "front_end/grammar/effects/remove_destroy_shapes/tests.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/effects/resource_shapes.rs",
            max_lines: 800,
        },
        Budget {
            path: "front_end/grammar/effects/resource_shapes/tests.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/effects/special_sentence_shapes.rs",
            max_lines: 550,
        },
        Budget {
            path: "front_end/grammar/effects/token_copy_control_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/token_copy_control_shapes/choices.rs",
            max_lines: 325,
        },
        Budget {
            path: "front_end/grammar/effects/token_copy_control_shapes/sequences.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/effects/token_copy_control_shapes/surfaces.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/effects/replacement_prevention_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/replacement_prevention_shapes/actions.rs",
            max_lines: 375,
        },
        Budget {
            path: "front_end/grammar/effects/replacement_prevention_shapes/zones.rs",
            max_lines: 325,
        },
        Budget {
            path: "front_end/grammar/effects/replacement_prevention_shapes/look.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/triple_sequence_shapes.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/effects/triple_sequence_shapes/early.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/triple_sequence_shapes/looked.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/effects/triple_sequence_shapes/late.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/instead.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/mana_replacement.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/next_spell_grants.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/effects/return_exchange.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/effects/return_exchange/return_shapes.rs",
            max_lines: 650,
        },
        Budget {
            path: "front_end/grammar/effects/return_exchange/exchange_shapes.rs",
            max_lines: 475,
        },
        Budget {
            path: "front_end/grammar/effects/rewrite_shapes.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/labeled_dispatch.rs",
            max_lines: 30,
        },
        Budget {
            path: "front_end/grammar/effects/labeled_dispatch/ability_candidates.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/effects/labeled_dispatch/common.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/effects/labeled_dispatch/cost_reduction.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/effects/labeled_dispatch/passive_addition.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/labeled_dispatch/surface.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/labeled_dispatch/token_copy.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/looked_card_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/looked_card_shapes/filters.rs",
            max_lines: 475,
        },
        Budget {
            path: "front_end/grammar/effects/looked_card_shapes/surface.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/effects/looked_card_shapes/values.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/control_copy_attach_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/control_copy_attach_shapes/common.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/control_copy_attach_shapes/control.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/effects/control_copy_attach_shapes/destinations.rs",
            max_lines: 450,
        },
        Budget {
            path: "front_end/grammar/effects/control_copy_attach_shapes/life.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/effects/control_copy_attach_shapes/looked_put.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/for_each_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/for_each_shapes/facts.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/for_each_shapes/modifier.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/effects/for_each_shapes/participants.rs",
            max_lines: 525,
        },
        Budget {
            path: "front_end/grammar/effects/for_each_shapes/power.rs",
            max_lines: 225,
        },
        Budget {
            path: "front_end/grammar/effects/for_each_shapes/subjects.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/effects/clause_dispatch_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/clause_dispatch_shapes/core.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/effects/clause_dispatch_shapes/direct.rs",
            max_lines: 425,
        },
        Budget {
            path: "front_end/grammar/effects/clause_dispatch_shapes/permissions.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/effects/clause_dispatch_shapes/relational.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/subject_verb_registry_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/subject_verb_registry_shapes/clause.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/effects/subject_verb_registry_shapes/delayed.rs",
            max_lines: 150,
        },
        Budget {
            path: "front_end/grammar/effects/subject_verb_registry_shapes/joint.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/effects/subject_verb_registry_shapes/sequences.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/effects/search_library.rs",
            max_lines: 1800,
        },
        Budget {
            path: "front_end/grammar/effects/search_library/duration_shapes.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/effects/search_library/exile_shapes.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/effects/search_library/shuffle_shapes.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/effects/sacrifice_discard_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/sacrifice_discard_shapes/common.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/effects/sacrifice_discard_shapes/discard.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/effects/sacrifice_discard_shapes/sacrifice.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/effects/unsupported_shapes.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/effects/unless_clause.rs",
            max_lines: 125,
        },
        Budget {
            path: "front_end/grammar/effects/unless_clause/tests.rs",
            max_lines: 75,
        },
        Budget {
            path: "front_end/grammar/effects/zone_counter_shapes.rs",
            max_lines: 875,
        },
        Budget {
            path: "front_end/grammar/effects/zone_move_shapes.rs",
            max_lines: 25,
        },
        Budget {
            path: "front_end/grammar/effects/zone_move_shapes/draw.rs",
            max_lines: 700,
        },
        Budget {
            path: "front_end/grammar/effects/zone_move_shapes/counter.rs",
            max_lines: 375,
        },
        Budget {
            path: "front_end/grammar/etb_static_lines.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/etb_static_lines/counter_entry.rs",
            max_lines: 850,
        },
        Budget {
            path: "front_end/grammar/etb_static_lines/entry_shapes.rs",
            max_lines: 400,
        },
        Budget {
            path: "front_end/grammar/etb_static_lines/known_values.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/etb_static_lines/phrase_facts.rs",
            max_lines: 850,
        },
        Budget {
            path: "front_end/grammar/etb_static_lines/value_shapes.rs",
            max_lines: 900,
        },
        Budget {
            path: "front_end/grammar/attached_object_static_lines.rs",
            max_lines: 100,
        },
        Budget {
            path: "front_end/grammar/attached_object_static_lines/grant_shapes.rs",
            max_lines: 425,
        },
        Budget {
            path: "front_end/grammar/attached_object_static_lines/prevention.rs",
            max_lines: 375,
        },
        Budget {
            path: "front_end/grammar/attached_object_static_lines/prevention/prevent_all.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/attached_object_static_lines/restrictions.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/attached_object_static_lines/subjects.rs",
            max_lines: 350,
        },
        Budget {
            path: "front_end/grammar/attached_object_static_lines/transforms.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/primitives.rs",
            max_lines: 1300,
        },
        Budget {
            path: "front_end/grammar/structure.rs",
            max_lines: 1650,
        },
        Budget {
            path: "front_end/grammar/structure/trigger_shapes.rs",
            max_lines: 325,
        },
        Budget {
            path: "front_end/grammar/values.rs",
            max_lines: 900,
        },
        Budget {
            path: "front_end/grammar/filters/counter_constraints.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/filters/decorations.rs",
            max_lines: 700,
        },
        Budget {
            path: "front_end/grammar/filters/domain_unions.rs",
            max_lines: 175,
        },
        Budget {
            path: "front_end/grammar/filters/mod.rs",
            max_lines: 800,
        },
        Budget {
            path: "front_end/grammar/filters/player_relations.rs",
            max_lines: 2000,
        },
        Budget {
            path: "front_end/grammar/filters/naming_and_reference.rs",
            max_lines: 1600,
        },
        Budget {
            path: "front_end/grammar/filters/reference_tag_stage.rs",
            max_lines: 2600,
        },
        Budget {
            path: "front_end/grammar/filters/reference_tag_word_facts.rs",
            max_lines: 500,
        },
        Budget {
            path: "front_end/grammar/filters/simple.rs",
            max_lines: 900,
        },
        Budget {
            path: "front_end/grammar/filters/spell_filters.rs",
            max_lines: 2000,
        },
        Budget {
            path: "front_end/grammar/filters/predicate_phrases.rs",
            max_lines: 10500,
        },
        Budget {
            path: "front_end/grammar/filters/predicate_phrases/capture_shapes.rs",
            max_lines: 700,
        },
        Budget {
            path: "front_end/grammar/filters/predicate_phrases/surface.rs",
            max_lines: 200,
        },
        Budget {
            path: "front_end/grammar/filters/meld_and_special_subjects.rs",
            max_lines: 2000,
        },
        Budget {
            path: "front_end/grammar/functional_zones.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/lowering_surfaces.rs",
            max_lines: 300,
        },
        Budget {
            path: "front_end/grammar/modal_results.rs",
            max_lines: 525,
        },
        Budget {
            path: "front_end/grammar/postpass_surfaces.rs",
            max_lines: 250,
        },
        Budget {
            path: "front_end/grammar/trigger_surface.rs",
            max_lines: 100,
        },
        Budget {
            path: "sentences/effect_sentences/bundle_rules.rs",
            max_lines: 1600,
        },
        Budget {
            path: "sentences/effect_sentences/sequence_rules/generic_subject_verb_sequences/pairs.rs",
            max_lines: 2800,
        },
        Budget {
            path: "sentences/effect_sentences/sequence_rules/generic_subject_verb_sequences/quads.rs",
            max_lines: 950,
        },
        Budget {
            path: "sentences/effect_sentences/sequence_rules/generic_subject_verb_sequences/mod.rs",
            max_lines: 775,
        },
        Budget {
            path: "sentences/effect_sentences/subject_verb_primitives/mod.rs",
            max_lines: 900,
        },
        Budget {
            path: "sentences/effect_sentences/subject_verb_primitives/choice_damage_family.rs",
            max_lines: 775,
        },
        Budget {
            path: "sentences/effect_sentences/subject_verb_primitives/registry.rs",
            max_lines: 2200,
        },
        Budget {
            path: "sentences/effect_sentences/subject_verb_primitives/counter_marker_family.rs",
            max_lines: 900,
        },
        Budget {
            path: "sentences/effect_sentences/subject_verb_primitives/token_copy_control_family.rs",
            max_lines: 700,
        },
        Budget {
            path: "sentences/effect_sentences/subject_verb_primitives/combat_and_damage_family.rs",
            max_lines: 925,
        },
        Budget {
            path: "sentences/effect_sentences/subject_verb_primitives/delayed_step_family.rs",
            max_lines: 1800,
        },
        Budget {
            path: "sentences/effect_sentences/subject_verb_primitives/mechanic_marker_family.rs",
            max_lines: 1800,
        },
        Budget {
            path: "sentences/effect_sentences/verb_handlers/mod.rs",
            max_lines: 700,
        },
        Budget {
            path: "sentences/effect_sentences/verb_handlers/resource_verbs.rs",
            max_lines: 1500,
        },
        Budget {
            path: "sentences/effect_sentences/verb_handlers/combat_verbs.rs",
            max_lines: 750,
        },
        Budget {
            path: "sentences/effect_sentences/verb_handlers/zone_move_verbs.rs",
            max_lines: 1500,
        },
        Budget {
            path: "sentences/effect_sentences/verb_handlers/counter_stat_verbs.rs",
            max_lines: 1500,
        },
        Budget {
            path: "sentences/effect_sentences/verb_handlers/control_copy_attach_verbs.rs",
            max_lines: 1100,
        },
        Budget {
            path: "sentences/effect_sentences/for_each_helpers.rs",
            max_lines: 700,
        },
        Budget {
            path: "sentences/effect_sentences/dispatch_inner/mod.rs",
            max_lines: 650,
        },
        Budget {
            path: "sentences/effect_sentences/dispatch_inner/sentence_shape_predicates.rs",
            max_lines: 1200,
        },
        Budget {
            path: "sentences/effect_sentences/dispatch_inner/labeled_prefixes.rs",
            max_lines: 625,
        },
        Budget {
            path: "sentences/effect_sentences/dispatch_inner/copy_and_next_spell_shapes.rs",
            max_lines: 1200,
        },
        Budget {
            path: "sentences/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs",
            max_lines: 650,
        },
        Budget {
            path: "sentences/effect_sentences/dispatch_inner/unsupported_shape_diagnostics.rs",
            max_lines: 1200,
        },
        Budget {
            path: "lowering/lower/mod.rs",
            max_lines: 2200,
        },
        Budget {
            path: "lowering/compile_support.rs",
            max_lines: 4250,
        },
        Budget {
            path: "lowering/lower/rewrite_text_helpers.rs",
            max_lines: 2200,
        },
        Budget {
            path: "lowering/lower/rewrite_sentence_grouping.rs",
            max_lines: 2200,
        },
        Budget {
            path: "lowering/lower/damage_and_cost_rewrites.rs",
            max_lines: 2200,
        },
        Budget {
            path: "lowering/lower/modal_and_level_lowering.rs",
            max_lines: 2200,
        },
    ];

    let mut failures = Vec::new();
    for budget in budgets {
        let path = parser_root.join(budget.path);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()));
        let line_count = source.lines().count();
        let display_path = format!("{RUNTIME_BACKEND}/{}", budget.path);
        println!(
            "{}: {} lines (budget {})",
            display_path, line_count, budget.max_lines
        );
        if line_count > budget.max_lines {
            failures.push((display_path, line_count, budget.max_lines));
        }
    }

    if !failures.is_empty() {
        eprintln!("\nBudget failures:");
        for (path, line_count, max_lines) in failures {
            eprintln!("  {path}: {line_count} > {max_lines}");
        }
        std::process::exit(1);
    }
}
