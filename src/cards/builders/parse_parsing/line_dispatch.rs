use crate::cards::builders::{
    AdditionalCostChoiceOptionAst, CardTextError, ClauseView, KeywordAction, LineAst,
    ParsedAbility, RULE_SHAPE_HAS_COLON, RuleDef, RuleIndex, StaticAbilityAst, Token, TriggerSpec,
    UnsupportedDiagnoser, UnsupportedRuleDef, dash_labeled_remainder_starts_with_trigger,
    find_verb, is_non_mana_additional_cost_modifier_line,
    is_untap_during_each_other_players_untap_step_words, leading_mana_symbols_to_oracle,
    parse_ability_line, parse_activated_line, parse_activated_line_with_raw, parse_activation_cost,
    parse_buyback_line, parse_cast_this_spell_only_line, parse_channel_line, parse_cycling_line,
    parse_effect_sentences, parse_enters_with_counters_line, parse_entwine_line, parse_equip_line,
    parse_escape_line, parse_if_this_spell_costs_less_to_cast_line, parse_kicker_line,
    parse_level_up_line, parse_loyalty_shorthand_activation_cost, parse_madness_line,
    parse_mana_symbol, parse_mana_symbol_group, parse_morph_keyword_line, parse_multikicker_line,
    parse_reinforce_line, parse_saga_chapter_prefix, parse_scryfall_mana_cost, parse_squad_line,
    parse_static_ability_ast_line, parse_this_spell_cost_condition, parse_transmute_line,
    parse_triggered_line, parser_trace, parser_trace_line, split_on_or,
    starts_with_until_end_of_turn, tokenize_line, trim_commas, unsupported_rule_error_for_view,
    words,
};
use crate::{AlternativeCastingMethod, OptionalCost, TotalCost};

const PRE_TOKEN_DIAGNOSTIC_RULES: [UnsupportedRuleDef; 22] = [
    UnsupportedRuleDef {
        id: "commander-cast-count",
        priority: 100,
        heads: &["for"],
        shape_mask: 0,
        message: "unsupported commander-cast-count clause",
        predicate: line_has_commander_cast_count_clause,
    },
    UnsupportedRuleDef {
        id: "verb-leading-spell",
        priority: 110,
        heads: &["sacrifice"],
        shape_mask: 0,
        message: "unsupported verb-leading spell clause",
        predicate: line_has_verb_leading_spell_clause,
    },
    UnsupportedRuleDef {
        id: "choose-leading-spell",
        priority: 120,
        heads: &["choose"],
        shape_mask: 0,
        message: "unsupported choose-leading spell clause",
        predicate: line_has_choose_leading_spell_clause,
    },
    UnsupportedRuleDef {
        id: "partner-with-keyword-line",
        priority: 125,
        heads: &["partner"],
        shape_mask: 0,
        message: "unsupported partner-with keyword line",
        predicate: line_has_partner_with_keyword_clause,
    },
    UnsupportedRuleDef {
        id: "put-from-among",
        priority: 130,
        heads: &[],
        shape_mask: 0,
        message: "unsupported put-from-among clause",
        predicate: line_has_put_from_among_clause,
    },
    UnsupportedRuleDef {
        id: "standalone-token-reminder",
        priority: 140,
        heads: &["it"],
        shape_mask: 0,
        message: "unsupported standalone token reminder clause",
        predicate: line_has_standalone_token_reminder_clause,
    },
    UnsupportedRuleDef {
        id: "multi-destination-put",
        priority: 150,
        heads: &["put"],
        shape_mask: 0,
        message: "unsupported multi-destination put clause",
        predicate: line_has_multi_destination_put_clause,
    },
    UnsupportedRuleDef {
        id: "marker-keyword-tail",
        priority: 160,
        heads: &["ninjutsu"],
        shape_mask: 0,
        message: "unsupported marker keyword tail clause",
        predicate: line_has_marker_keyword_tail_clause,
    },
    UnsupportedRuleDef {
        id: "aura-copy-attachment-fanout",
        priority: 170,
        heads: &[],
        shape_mask: 0,
        message: "unsupported aura-copy attachment fanout clause",
        predicate: line_has_aura_copy_attachment_fanout_clause,
    },
    UnsupportedRuleDef {
        id: "defending-players-choice",
        priority: 180,
        heads: &["of", "target"],
        shape_mask: 0,
        message: "unsupported defending-players-choice clause",
        predicate: line_has_defending_players_choice_clause,
    },
    UnsupportedRuleDef {
        id: "first-spell-cost-modifier",
        priority: 190,
        heads: &["the"],
        shape_mask: 0,
        message: "unsupported first-spell cost modifier mechanic",
        predicate: line_has_first_spell_cost_modifier_clause,
    },
    UnsupportedRuleDef {
        id: "spent-to-cast-conditional",
        priority: 200,
        heads: &[],
        shape_mask: 0,
        message: "unsupported spent-to-cast conditional clause",
        predicate: line_has_spent_to_cast_conditional_clause,
    },
    UnsupportedRuleDef {
        id: "different-mana-value-target",
        priority: 210,
        heads: &[],
        shape_mask: 0,
        message: "unsupported different-mana-value target clause",
        predicate: line_has_different_mana_value_target_clause,
    },
    UnsupportedRuleDef {
        id: "most-common-color",
        priority: 220,
        heads: &[],
        shape_mask: 0,
        message: "unsupported most-common-color clause",
        predicate: line_has_most_common_color_clause,
    },
    UnsupportedRuleDef {
        id: "power-vs-count-conditional",
        priority: 230,
        heads: &[],
        shape_mask: 0,
        message: "unsupported power-vs-count conditional clause",
        predicate: line_has_power_vs_count_conditional_clause,
    },
    UnsupportedRuleDef {
        id: "graveyards-from-battlefield-count",
        priority: 240,
        heads: &[],
        shape_mask: 0,
        message: "unsupported graveyards-from-battlefield count clause",
        predicate: line_has_put_into_graveyards_from_battlefield_count_clause,
    },
    UnsupportedRuleDef {
        id: "phase-out-until-leaves",
        priority: 250,
        heads: &[],
        shape_mask: 0,
        message: "unsupported phase-out-until-leaves clause",
        predicate: line_has_phase_out_until_leaves_clause,
    },
    UnsupportedRuleDef {
        id: "same-name-as-another-in-hand",
        priority: 260,
        heads: &[],
        shape_mask: 0,
        message: "unsupported same-name-as-another-in-hand clause",
        predicate: line_has_same_name_as_another_in_hand_clause,
    },
    UnsupportedRuleDef {
        id: "for-each-mana-from-spent",
        priority: 270,
        heads: &[],
        shape_mask: 0,
        message: "unsupported for-each-mana-from-spent clause",
        predicate: line_has_for_each_mana_from_spent_clause,
    },
    UnsupportedRuleDef {
        id: "enters-as-copy-except-ability",
        priority: 280,
        heads: &[],
        shape_mask: 0,
        message: "unsupported enters-as-copy except-ability clause",
        predicate: line_has_enters_as_copy_except_ability_clause,
    },
    UnsupportedRuleDef {
        id: "creature-token-player-planeswalker-target",
        priority: 290,
        heads: &[],
        shape_mask: 0,
        message: "unsupported creature-token/player/planeswalker target clause",
        predicate: line_has_creature_token_player_planeswalker_target_clause,
    },
    UnsupportedRuleDef {
        id: "non-mana-additional-cost-modifier",
        priority: 300,
        heads: &[],
        shape_mask: 0,
        message: "unsupported non-mana additional-cost modifier line",
        predicate: line_has_non_mana_additional_cost_modifier_clause,
    },
];

const STATIC_LINE_DIAGNOSTIC_RULES: [UnsupportedRuleDef; 6] = [
    UnsupportedRuleDef {
        id: "activate-only-standalone",
        priority: 200,
        heads: &["activate"],
        shape_mask: 0,
        message: "unsupported standalone activate-only restriction line",
        predicate: line_has_activate_only_standalone_clause,
    },
    UnsupportedRuleDef {
        id: "graveyard-cast-permission",
        priority: 210,
        heads: &["you"],
        shape_mask: 0,
        message: "unsupported graveyard cast-permission static clause",
        predicate: line_has_graveyard_cast_permission_clause,
    },
    UnsupportedRuleDef {
        id: "dynamic-gets-from-counters",
        priority: 220,
        heads: &[],
        shape_mask: 0,
        message: "unsupported dynamic gets-from-counters static clause",
        predicate: line_has_dynamic_gets_from_counters_clause,
    },
    UnsupportedRuleDef {
        id: "foretell-cost-modifier",
        priority: 230,
        heads: &["foretelling"],
        shape_mask: 0,
        message: "unsupported foretell-cost modifier static clause",
        predicate: line_has_foretell_cost_modifier_clause,
    },
    UnsupportedRuleDef {
        id: "trigger-frequency-standalone",
        priority: 240,
        heads: &["this"],
        shape_mask: 0,
        message: "unsupported standalone trigger-frequency restriction line",
        predicate: line_has_trigger_frequency_restriction_clause,
    },
    UnsupportedRuleDef {
        id: "level-marker-static",
        priority: 250,
        heads: &[],
        shape_mask: 0,
        message: "unsupported level marker static clause",
        predicate: line_has_level_marker_clause,
    },
];

const KNOWN_STATIC_LINE_DIAGNOSTIC_RULES: [UnsupportedRuleDef; 16] = [
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 300,
        heads: &["play"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_top_card_revealed_static_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 310,
        heads: &["gain", "when"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_class_level_progression_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 320,
        heads: &["whenever"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_play_a_card_trigger_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 330,
        heads: &["when"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_no_creatures_trigger_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 340,
        heads: &["you", "play", "all"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_play_from_top_of_library_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 350,
        heads: &["you"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_look_top_card_any_time_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 360,
        heads: &["once"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_once_each_turn_play_from_exile_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 370,
        heads: &["creatures"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_generic_attack_tax_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 380,
        heads: &["this"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_this_attack_or_block_restriction_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 390,
        heads: &["players"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_single_artifact_untap_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 400,
        heads: &["as"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_equipped_human_condition_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 410,
        heads: &["while"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_choosing_targets_static_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 420,
        heads: &["it"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_it_enters_with_counter_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 430,
        heads: &["enchanted"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_enchanted_creature_gets_negative_x_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 440,
        heads: &["if"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_plus_one_counter_replacement_clause,
    },
    UnsupportedRuleDef {
        id: "known-static-clause",
        priority: 450,
        heads: &["if"],
        shape_mask: 0,
        message: "unsupported static clause",
        predicate: line_has_token_replacement_clause,
    },
];

const PRE_TOKEN_DIAGNOSER: UnsupportedDiagnoser =
    UnsupportedDiagnoser::new(&PRE_TOKEN_DIAGNOSTIC_RULES);
const STATIC_LINE_DIAGNOSER: UnsupportedDiagnoser =
    UnsupportedDiagnoser::new(&STATIC_LINE_DIAGNOSTIC_RULES);
const KNOWN_STATIC_LINE_DIAGNOSER: UnsupportedDiagnoser =
    UnsupportedDiagnoser::new(&KNOWN_STATIC_LINE_DIAGNOSTIC_RULES);

fn normalized_line<'a>(view: &'a ClauseView<'a>) -> &'a str {
    view.normalized.or(view.raw).unwrap_or_default().trim()
}

fn normalized_line_without_braces<'a>(view: &'a ClauseView<'a>) -> &'a str {
    view.normalized_without_braces
        .unwrap_or_else(|| normalized_line(view))
}

fn diagnose_line_unsupported(view: &ClauseView<'_>) -> Option<CardTextError> {
    PRE_TOKEN_DIAGNOSER
        .diagnose(view, "line")
        .or_else(|| STATIC_LINE_DIAGNOSER.diagnose(view, "line"))
        .or_else(|| KNOWN_STATIC_LINE_DIAGNOSER.diagnose(view, "line"))
}

fn line_has_commander_cast_count_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("for each time")
        && normalized.contains("cast")
        && normalized.contains("commander")
        && normalized.contains("from the command zone")
}

fn line_has_verb_leading_spell_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("sacrifice x lands")
        && normalized.contains("you may play x additional lands this turn")
}

fn line_has_choose_leading_spell_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("choose target land")
        && normalized.contains("create three tokens that are copies of it")
}

fn line_has_partner_with_keyword_clause(view: &ClauseView<'_>) -> bool {
    normalized_line(view).starts_with("partner with ")
}

fn line_has_put_from_among_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    let supported_reveal_or_look_top_sequence = (normalized.starts_with("reveal the top ")
        || normalized.starts_with("look at the top "))
        && normalized.contains("put the rest into")
        && normalized.contains("graveyard");
    let supported_look_battlefield_or_hand_sequence = normalized.starts_with("look at the top ")
        && (normalized.contains("from among them onto the battlefield")
            || normalized.contains("from among those cards onto the battlefield"))
        && (normalized.contains("put a card from among them into your hand")
            || normalized.contains("put a card from among those cards into your hand"))
        && normalized.contains("put the rest on the bottom of your library");
    let supported_mill_into_hand_sequence = normalized.contains("mill")
        && normalized.contains("put a ")
        && (normalized.contains("from among them into your hand")
            || normalized.contains("from among those cards into your hand"));
    normalized.contains("put a ")
        && (normalized.contains("from among them into your hand")
            || normalized.contains("from among those cards into your hand"))
        && !supported_reveal_or_look_top_sequence
        && !supported_look_battlefield_or_hand_sequence
        && !supported_mill_into_hand_sequence
}

fn line_has_standalone_token_reminder_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("it has \"this token gets +1/+1 for each card named")
        && normalized.contains("in each graveyard")
}

fn line_has_multi_destination_put_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("put one of them into your hand and the rest into your graveyard")
}

fn line_has_spent_to_cast_conditional_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains(" was spent to cast this spell")
        && normalized.contains(" if ")
        && normalized.contains(" and ")
}

fn line_has_different_mana_value_target_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("different mana value") && normalized.contains("target creature cards")
}

fn line_has_most_common_color_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("most common color among all permanents")
        || normalized.contains("tied for most common")
}

fn line_has_power_vs_count_conditional_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("power is less than or equal to the number of")
}

fn line_has_put_into_graveyards_from_battlefield_count_clause(view: &ClauseView<'_>) -> bool {
    normalized_line(view).contains("were put into graveyards from the battlefield this turn")
}

fn line_has_phase_out_until_leaves_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("phases out until") && normalized.contains("leaves the battlefield")
}

fn line_has_same_name_as_another_in_hand_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("same name as another card in their hand")
        || normalized.contains("same name as another card in your hand")
}

fn line_has_for_each_mana_from_spent_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("for each mana from") && normalized.contains("spent to cast this spell")
}

fn line_has_enters_as_copy_except_ability_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("enter as a copy of") && normalized.contains("except it has")
}

fn line_has_creature_token_player_planeswalker_target_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("target creature token")
        && normalized.contains("player")
        && normalized.contains("planeswalker")
}

fn line_has_marker_keyword_tail_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("ninjutsu abilities you activate cost")
}

fn line_has_aura_copy_attachment_fanout_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("copy of that aura attached to that creature")
}

fn line_has_defending_players_choice_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("of defending players choice")
        || normalized.contains("of defending player s choice")
        || (normalized.contains("defending player") && normalized.contains("choice"))
}

fn line_has_first_spell_cost_modifier_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("the first creature spell you cast each turn costs")
        && normalized.contains("less to cast")
}

fn line_has_non_mana_additional_cost_modifier_clause(view: &ClauseView<'_>) -> bool {
    is_non_mana_additional_cost_modifier_line(normalized_line(view))
}

fn line_has_activate_only_standalone_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("activate only")
}

fn line_has_graveyard_cast_permission_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("you may cast this card from your graveyard as long as you control")
        || normalized.starts_with("you may cast this from your graveyard as long as you control")
}

fn line_has_dynamic_gets_from_counters_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("gets +x/+x")
        && normalized.contains("where x is the number of counters on this")
}

fn line_has_foretell_cost_modifier_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("foretelling cards from your hand costs")
}

fn line_has_trigger_frequency_restriction_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("this ability triggers only")
}

fn line_has_level_marker_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains(": level ")
}

fn line_has_top_card_revealed_static_clause(view: &ClauseView<'_>) -> bool {
    normalized_line_without_braces(view) == "play with the top card of your library revealed"
}

fn line_has_class_level_progression_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line_without_braces(view);
    normalized.starts_with("gain the next level as a sorcery to add its ability")
        || normalized.starts_with("when this class becomes level")
}

fn line_has_play_a_card_trigger_clause(view: &ClauseView<'_>) -> bool {
    normalized_line_without_braces(view).starts_with("whenever you play a card")
}

fn line_has_no_creatures_trigger_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line_without_braces(view);
    normalized.starts_with("when there are no creatures on the battlefield")
        || normalized.starts_with("when there are no creatures on battlefield")
}

fn line_has_play_from_top_of_library_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line_without_braces(view);
    normalized == "you may play lands and cast spells from the top of your library"
        || normalized == "play lands and cast spells from the top of your library"
        || normalized == "all mountains are plains"
}

fn line_has_look_top_card_any_time_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line_without_braces(view);
    normalized.starts_with("you may look at top card of your library any time")
        || normalized.starts_with("you may look at the top card of your library any time")
}

fn line_has_once_each_turn_play_from_exile_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line_without_braces(view);
    normalized.starts_with("once each turn, you may play a card from exile")
        || normalized.starts_with("once each turn you may play a card from exile")
}

fn is_collective_restraint_domain_attack_tax(normalized: &str) -> bool {
    normalized.starts_with(
        "creatures cant attack you unless their controller pays x for each creature they control thats attacking you",
    ) && normalized.contains("where x is the number of basic land type")
}

fn is_fixed_attack_tax_per_attacker(normalized: &str) -> bool {
    normalized
        .strip_prefix("creatures cant attack you unless their controller pays ")
        .and_then(|rest| rest.strip_suffix(" for each creature they control thats attacking you"))
        .is_some_and(|amount| !amount.is_empty() && amount.chars().all(|ch| ch.is_ascii_digit()))
}

fn line_has_generic_attack_tax_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line_without_braces(view);
    normalized.starts_with("creatures cant attack you unless")
        && !is_collective_restraint_domain_attack_tax(normalized)
        && !is_fixed_attack_tax_per_attacker(normalized)
}

fn line_has_this_attack_or_block_restriction_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line_without_braces(view);
    normalized.starts_with("this creature cant attack unless")
        || normalized.starts_with("this creature cant attack if")
        || normalized.starts_with("this creature cant block unless")
        || normalized.starts_with("this creature cant block if")
        || normalized == "this creature attacks or blocks each combat if able"
}

fn line_has_single_artifact_untap_clause(view: &ClauseView<'_>) -> bool {
    normalized_line_without_braces(view)
        .starts_with("players cant untap more than one artifact during their untap steps")
}

fn line_has_equipped_human_condition_clause(view: &ClauseView<'_>) -> bool {
    normalized_line_without_braces(view).starts_with("as long as equipped creature is a human")
}

fn line_has_choosing_targets_static_clause(view: &ClauseView<'_>) -> bool {
    normalized_line_without_braces(view)
        .starts_with("while an opponent is choosing targets as part of casting a spell")
}

fn line_has_it_enters_with_counter_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line_without_braces(view);
    normalized.starts_with("it enters with") && normalized.contains("+1/+1 counter")
}

fn line_has_enchanted_creature_gets_negative_x_clause(view: &ClauseView<'_>) -> bool {
    normalized_line_without_braces(view).starts_with("enchanted creature gets -x/-x")
}

fn line_has_plus_one_counter_replacement_clause(view: &ClauseView<'_>) -> bool {
    normalized_line_without_braces(view)
        .starts_with("if one or more +1/+1 counters would be put on")
}

fn line_has_token_replacement_clause(view: &ClauseView<'_>) -> bool {
    normalized_line_without_braces(view)
        .starts_with("if an effect would create one or more tokens under your control")
}

fn parse_first_parsed_ability_rule(
    tokens: &[Token],
) -> Result<Option<(&'static str, ParsedAbility)>, CardTextError> {
    fn parse_equip_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_equip_line(view.tokens)
    }

    fn parse_level_up_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_level_up_line(view.tokens)
    }

    fn parse_reinforce_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_reinforce_line(view.tokens)
    }

    fn parse_cycling_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_cycling_line(view.tokens)
    }

    fn parse_morph_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_morph_keyword_line(view.tokens)
    }

    fn parse_transmute_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_transmute_line(view.tokens)
    }

    fn parse_channel_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_channel_line(view.tokens)
    }

    const RULES: [RuleDef<ParsedAbility>; 7] = [
        RuleDef {
            id: "equip",
            priority: 100,
            heads: &["equip"],
            shape_mask: 0,
            run: parse_equip_rule,
        },
        RuleDef {
            id: "level-up",
            priority: 110,
            heads: &["level", "levelup"],
            shape_mask: 0,
            run: parse_level_up_rule,
        },
        RuleDef {
            id: "reinforce",
            priority: 120,
            heads: &["reinforce"],
            shape_mask: 0,
            run: parse_reinforce_rule,
        },
        RuleDef {
            id: "cycling",
            priority: 130,
            heads: &[],
            shape_mask: 0,
            run: parse_cycling_rule,
        },
        RuleDef {
            id: "morph",
            priority: 140,
            heads: &["morph", "megamorph"],
            shape_mask: 0,
            run: parse_morph_rule,
        },
        RuleDef {
            id: "transmute",
            priority: 150,
            heads: &["transmute"],
            shape_mask: 0,
            run: parse_transmute_rule,
        },
        RuleDef {
            id: "channel",
            priority: 160,
            heads: &["channel"],
            shape_mask: RULE_SHAPE_HAS_COLON,
            run: parse_channel_rule,
        },
    ];
    let view = ClauseView::from_tokens(tokens);
    let index = RuleIndex::new(&RULES);
    index.run_first(&view)
}

fn parse_first_optional_cost_rule(
    tokens: &[Token],
) -> Result<Option<(&'static str, OptionalCost)>, CardTextError> {
    fn parse_buyback_rule(view: &ClauseView<'_>) -> Result<Option<OptionalCost>, CardTextError> {
        parse_buyback_line(view.tokens)
    }

    fn parse_kicker_rule(view: &ClauseView<'_>) -> Result<Option<OptionalCost>, CardTextError> {
        parse_kicker_line(view.tokens)
    }

    fn parse_multikicker_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<OptionalCost>, CardTextError> {
        parse_multikicker_line(view.tokens)
    }

    fn parse_entwine_rule(view: &ClauseView<'_>) -> Result<Option<OptionalCost>, CardTextError> {
        parse_entwine_line(view.tokens)
    }

    fn parse_squad_rule(view: &ClauseView<'_>) -> Result<Option<OptionalCost>, CardTextError> {
        parse_squad_line(view.tokens)
    }

    const RULES: [RuleDef<OptionalCost>; 5] = [
        RuleDef {
            id: "buyback",
            priority: 100,
            heads: &["buyback"],
            shape_mask: 0,
            run: parse_buyback_rule,
        },
        RuleDef {
            id: "kicker",
            priority: 110,
            heads: &["kicker"],
            shape_mask: 0,
            run: parse_kicker_rule,
        },
        RuleDef {
            id: "multikicker",
            priority: 120,
            heads: &["multikicker"],
            shape_mask: 0,
            run: parse_multikicker_rule,
        },
        RuleDef {
            id: "entwine",
            priority: 130,
            heads: &["entwine"],
            shape_mask: 0,
            run: parse_entwine_rule,
        },
        RuleDef {
            id: "squad",
            priority: 140,
            heads: &["squad"],
            shape_mask: 0,
            run: parse_squad_rule,
        },
    ];
    let view = ClauseView::from_tokens(tokens);
    let index = RuleIndex::new(&RULES);
    index.run_first(&view)
}

fn parse_first_alternative_cast_rule(
    tokens: &[Token],
    line: &str,
) -> Result<Option<(&'static str, AlternativeCastingMethod)>, CardTextError> {
    fn parse_if_conditional_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_if_conditional_alternative_cost_line(view.tokens, view.raw.unwrap_or_default())
    }

    fn parse_self_free_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        Ok(parse_self_free_cast_alternative_cost_line(view.tokens))
    }

    fn parse_you_may_rather_than_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_you_may_rather_than_spell_cost_line(view.tokens, view.raw.unwrap_or_default())
    }

    fn parse_escape_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_escape_line(view.tokens)
    }

    fn parse_bestow_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_bestow_line(view.tokens)
    }

    fn parse_flashback_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_flashback_line(view.tokens)
    }

    fn parse_madness_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_madness_line(view.tokens)
    }

    const RULES: [RuleDef<AlternativeCastingMethod>; 7] = [
        RuleDef {
            id: "if-conditional-alternative-cost",
            priority: 100,
            heads: &["if"],
            shape_mask: 0,
            run: parse_if_conditional_rule,
        },
        RuleDef {
            id: "self-free-cast-alternative-cost",
            priority: 110,
            heads: &["if", "you"],
            shape_mask: 0,
            run: parse_self_free_rule,
        },
        RuleDef {
            id: "alternative-cost",
            priority: 120,
            heads: &["you"],
            shape_mask: 0,
            run: parse_you_may_rather_than_rule,
        },
        RuleDef {
            id: "escape",
            priority: 130,
            heads: &["escape"],
            shape_mask: 0,
            run: parse_escape_rule,
        },
        RuleDef {
            id: "bestow",
            priority: 140,
            heads: &["bestow"],
            shape_mask: 0,
            run: parse_bestow_rule,
        },
        RuleDef {
            id: "flashback",
            priority: 150,
            heads: &["flashback"],
            shape_mask: 0,
            run: parse_flashback_rule,
        },
        RuleDef {
            id: "madness",
            priority: 160,
            heads: &["madness"],
            shape_mask: 0,
            run: parse_madness_rule,
        },
    ];
    let view = ClauseView::from_line(line, line, line, tokens, 0);
    let index = RuleIndex::new(&RULES);
    index.run_first(&view)
}

fn line_ast_from_static_abilities(abilities: Vec<StaticAbilityAst>) -> LineAst {
    match abilities.as_slice() {
        [ability] => LineAst::StaticAbility(ability.clone()),
        _ => LineAst::StaticAbilities(abilities),
    }
}

fn line_rule_unsupported(
    view: &ClauseView<'_>,
    rule_id: &'static str,
    message: &'static str,
) -> CardTextError {
    unsupported_rule_error_for_view(rule_id, message, "line", view)
}

fn line_is_cost_floor_reminder_clause(normalized: &str) -> bool {
    normalized.starts_with("this effect cant reduce the mana in that cost to less than")
        || normalized.starts_with("this effect cant reduce the mana in those costs to less than")
}

fn line_is_self_enters_with_counters_clause(normalized: &str) -> bool {
    (normalized.starts_with("this creature enters with")
        || normalized.starts_with("this creature enters the battlefield with")
        || normalized.starts_with("it enters with")
        || normalized.starts_with("it enters the battlefield with"))
        && normalized.contains("+1/+1 counter")
}

fn line_is_this_cant_attack_unless_clause(normalized: &str) -> bool {
    normalized.starts_with("this creature cant attack unless")
        || normalized.starts_with("this cant attack unless")
}

fn line_is_cast_this_spell_only_clause(normalized: &str) -> bool {
    normalized.starts_with("cast this spell only")
}

fn line_is_skulk_rules_text_clause(normalized: &str) -> bool {
    normalized.starts_with("creatures with power less than this creatures power cant block it")
}

fn line_has_token_mana_reminder_tail(words: &[&str]) -> bool {
    words.contains(&"create")
        && words.contains(&"sacrifice")
        && words.contains(&"add")
        && words
            .windows(2)
            .any(|window| window == ["it", "has"] || window == ["they", "have"])
}

fn line_starts_with_statement_effect_head(tokens: &[Token], words: &[&str]) -> bool {
    let leading_verb_idx = find_verb(tokens).map(|(_, idx)| idx);
    matches!(leading_verb_idx, Some(0))
        || (matches!(leading_verb_idx, Some(1))
            && tokens.first().is_some_and(|token| {
                token.is_word("this") || token.is_word("it") || token.is_word("that")
            }))
        || tokens
            .first()
            .is_some_and(|token| token.is_word("choose") || token.is_word("if"))
        || starts_with_until_end_of_turn(words)
}

fn line_is_damage_prevent_with_remove_static(words: &[&str]) -> bool {
    words.starts_with(&["if", "damage", "would", "be", "dealt", "to", "this"])
        && words
            .windows(3)
            .any(|window| window == ["prevent", "that", "damage"])
        && words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
        && words.iter().any(|word| *word == "remove")
}

fn line_is_prevent_all_damage_to_source_by_creatures_static(words: &[&str]) -> bool {
    words.starts_with(&[
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to", "this",
    ]) && words.ends_with(&["by", "creatures"])
}

fn line_is_prevent_all_combat_damage_to_source_static(words: &[&str]) -> bool {
    words
        == [
            "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to", "this",
            "creature",
        ]
        || words
            == [
                "prevent",
                "all",
                "combat",
                "damage",
                "that",
                "would",
                "be",
                "dealt",
                "to",
                "this",
                "permanent",
            ]
        || words
            == [
                "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to", "it",
            ]
}

fn parse_cost_floor_reminder_rule(view: &ClauseView<'_>) -> Result<Option<LineAst>, CardTextError> {
    if !line_is_cost_floor_reminder_clause(normalized_line(view)) {
        return Ok(None);
    }
    Ok(Some(LineAst::StaticAbilities(Vec::new())))
}

fn parse_self_enters_with_counters_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    if !line_is_self_enters_with_counters_clause(normalized_line(view)) {
        return Ok(None);
    }
    match parse_enters_with_counters_line(view.tokens) {
        Ok(Some(ability)) => {
            parser_trace("parse_line:branch=self-etb-counters", view.tokens);
            Ok(Some(LineAst::StaticAbility(ability.into())))
        }
        _ => Err(line_rule_unsupported(
            view,
            "self-enters-with-counters",
            "unsupported self-enters-with-counters static clause",
        )),
    }
}

fn parse_saga_line_rule(view: &ClauseView<'_>) -> Result<Option<LineAst>, CardTextError> {
    let Some((chapters, rest)) = parse_saga_chapter_prefix(normalized_line(view)) else {
        return Ok(None);
    };
    let rest_tokens = tokenize_line(rest, view.line_index.unwrap_or_default());
    parser_trace("parse_line:branch=saga", &rest_tokens);
    let effects = parse_effect_sentences(&rest_tokens)?;
    Ok(Some(LineAst::Triggered {
        trigger: TriggerSpec::SagaChapter(chapters),
        effects,
        max_triggers_per_turn: None,
    }))
}

fn parse_replicate_line_rule(view: &ClauseView<'_>) -> Result<Option<LineAst>, CardTextError> {
    if !view
        .tokens
        .first()
        .is_some_and(|token| token.is_word("replicate"))
        || !view.raw.unwrap_or_default().contains('—')
    {
        return Ok(None);
    }

    let cost_tokens = view.tokens.get(1..).unwrap_or_default();
    if cost_tokens.is_empty() {
        let line = view.raw.unwrap_or_default();
        return Err(CardTextError::ParseError(format!(
            "replicate line missing cost (line: '{line}')"
        )));
    }

    parser_trace("parse_line:branch=replicate", view.tokens);
    let cost = parse_activation_cost(cost_tokens)?;
    Ok(Some(LineAst::OptionalCost(
        OptionalCost::custom("Replicate", cost).repeatable(),
    )))
}

fn parse_pact_next_upkeep_line_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let raw_line = view.raw.unwrap_or_default();
    let raw_segments = raw_line
        .split('.')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if raw_segments.len() != 3 {
        return Ok(None);
    }

    let first_tokens = tokenize_line(raw_segments[0], view.line_index.unwrap_or_default());
    let first_effects = parse_effect_sentences(&first_tokens)?;
    if first_effects.is_empty() {
        return Ok(None);
    }

    let upkeep_raw = raw_segments[1].trim();
    let upkeep_segment = tokenize_line(upkeep_raw, view.line_index.unwrap_or_default());
    let upkeep_tokens = trim_commas(&upkeep_segment);
    let upkeep_words = words(&upkeep_tokens);
    let pay_prefix = if upkeep_words.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "your",
        "next",
        "upkeep",
        "pay",
    ]) {
        "at the beginning of your next upkeep, pay"
    } else if upkeep_words.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "upkeep",
        "pay",
    ]) {
        "at the beginning of the next upkeep, pay"
    } else {
        return Ok(None);
    };

    let Some(raw_mana) = upkeep_raw
        .get(pay_prefix.len()..)
        .map(str::trim)
        .filter(|tail| !tail.is_empty())
    else {
        return Ok(None);
    };
    let mana_cost = parse_scryfall_mana_cost(raw_mana)?;
    let mut mana = Vec::new();
    for pip in mana_cost.pips() {
        let [symbol] = pip.as_slice() else {
            return Ok(None);
        };
        mana.push(*symbol);
    }
    if mana.is_empty() {
        return Ok(None);
    }

    let lose_segment = tokenize_line(raw_segments[2], view.line_index.unwrap_or_default());
    let lose_tokens = trim_commas(&lose_segment);
    let lose_words = words(&lose_tokens);
    if lose_words != ["if", "you", "dont", "you", "lose", "the", "game"]
        && lose_words != ["if", "you", "don't", "you", "lose", "the", "game"]
        && lose_words != ["if", "you", "don", "t", "you", "lose", "the", "game"]
        && lose_words != ["if", "you", "do", "not", "you", "lose", "the", "game"]
    {
        return Ok(None);
    }

    let mut effects = first_effects;
    effects.push(crate::cards::builders::EffectAst::DelayedUntilNextUpkeep {
        player: crate::cards::builders::PlayerAst::You,
        effects: vec![crate::cards::builders::EffectAst::UnlessPays {
            effects: vec![crate::cards::builders::EffectAst::LoseGame {
                player: crate::cards::builders::PlayerAst::You,
            }],
            player: crate::cards::builders::PlayerAst::You,
            mana,
        }],
    });
    parser_trace("parse_line:branch=pact-next-upkeep", view.tokens);
    Ok(Some(LineAst::Statement { effects }))
}

fn parse_additional_cost_line_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    if !normalized_line(view).starts_with("as an additional cost to cast this spell") {
        return Ok(None);
    }

    let comma_idx = view
        .tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let effect_start = if let Some(idx) = comma_idx {
        idx + 1
    } else if let Some(idx) = view.tokens.iter().position(|token| token.is_word("spell")) {
        idx + 1
    } else {
        view.tokens.len()
    };
    let effect_tokens = view.tokens.get(effect_start..).unwrap_or_default();
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "additional cost line missing effect clause".to_string(),
        ));
    }

    parser_trace("parse_line:branch=additional-cost", effect_tokens);
    if let Some(options) = parse_additional_cost_choice_options(effect_tokens)? {
        return Ok(Some(LineAst::AdditionalCostChoice { options }));
    }
    let effects = parse_effect_sentences(effect_tokens)?;
    Ok(Some(LineAst::AdditionalCost { effects }))
}

fn parse_alternative_cast_line_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let line = view.raw.unwrap_or_default();
    if let Some((branch, method)) = parse_first_alternative_cast_rule(view.tokens, line)? {
        let stage = format!("parse_line:branch={branch}");
        parser_trace(stage.as_str(), view.tokens);
        return Ok(Some(LineAst::AlternativeCastingMethod(method)));
    }
    Ok(None)
}

fn parse_parsed_ability_line_rule(view: &ClauseView<'_>) -> Result<Option<LineAst>, CardTextError> {
    if let Some((branch, ability)) = parse_first_parsed_ability_rule(view.tokens)? {
        let stage = format!("parse_line:branch={branch}");
        parser_trace(stage.as_str(), view.tokens);
        return Ok(Some(LineAst::Ability(ability)));
    }
    Ok(None)
}

fn parse_optional_cost_line_rule(view: &ClauseView<'_>) -> Result<Option<LineAst>, CardTextError> {
    if let Some((branch, cost)) = parse_first_optional_cost_rule(view.tokens)? {
        let stage = format!("parse_line:branch={branch}");
        parser_trace(stage.as_str(), view.tokens);
        return Ok(Some(LineAst::OptionalCost(cost)));
    }
    Ok(None)
}

fn parse_triggered_line_rule(view: &ClauseView<'_>) -> Result<Option<LineAst>, CardTextError> {
    let line = view.raw.unwrap_or_default();
    let Some((trigger_idx, _)) = view.tokens.iter().enumerate().find(|(idx, token)| {
        token.is_word("whenever") || token.is_word("when") || is_at_trigger_intro(view.tokens, *idx)
    }) else {
        return Ok(None);
    };
    if trigger_idx > 2 && !dash_labeled_remainder_starts_with_trigger(line) {
        return Ok(None);
    }

    parser_trace("parse_line:branch=triggered", &view.tokens[trigger_idx..]);
    parse_triggered_line(&view.tokens[trigger_idx..]).map(Some)
}

fn parse_waterbend_activated_line_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    if !view
        .tokens
        .first()
        .is_some_and(|token| token.is_word("waterbend"))
    {
        return Ok(None);
    }
    if let Some(ability) = parse_activated_line(&view.tokens[1..])? {
        parser_trace("parse_line:branch=waterbend-activated", &view.tokens[1..]);
        return Ok(Some(LineAst::Ability(ability)));
    }
    Ok(None)
}

fn parse_activated_line_rule(view: &ClauseView<'_>) -> Result<Option<LineAst>, CardTextError> {
    let Some(colon_idx) = view
        .tokens
        .iter()
        .position(|token| matches!(token, Token::Colon(_)))
    else {
        return Ok(None);
    };

    let cost_tokens = &view.tokens[..colon_idx];
    let line = view.raw.unwrap_or_default();
    let has_loyalty_shorthand =
        parse_loyalty_shorthand_activation_cost(cost_tokens, Some(line)).is_some();
    if starts_with_activation_cost(cost_tokens) || has_loyalty_shorthand {
        if let Some(ability) = parse_activated_line_with_raw(view.tokens, view.raw)? {
            parser_trace("parse_line:branch=activated", view.tokens);
            return Ok(Some(LineAst::Ability(ability)));
        }
        return Err(line_rule_unsupported(
            view,
            "activated",
            "unsupported activated ability line",
        ));
    }
    if (line.contains('—') || line.contains(" - "))
        && find_activation_cost_start(cost_tokens).is_some()
        && let Some(ability) = parse_activated_line(view.tokens)?
    {
        parser_trace("parse_line:branch=activated-labeled", view.tokens);
        return Ok(Some(LineAst::Ability(ability)));
    }
    Ok(None)
}

fn parse_token_mana_reminder_statement_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    if !line_has_token_mana_reminder_tail(view.words.as_slice()) {
        return Ok(None);
    }
    if let Ok(effects) = parse_effect_sentences(view.tokens)
        && !effects.is_empty()
    {
        parser_trace(
            "parse_line:branch=statement-token-mana-reminder",
            view.tokens,
        );
        return Ok(Some(LineAst::Statement { effects }));
    }
    Ok(None)
}

fn parse_if_this_spell_costs_less_line_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    if !view.tokens.first().is_some_and(|token| token.is_word("if")) {
        return Ok(None);
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line(view.tokens)? {
        parser_trace("parse_line:branch=if-this-spell-costs-less", view.tokens);
        return Ok(Some(LineAst::StaticAbility(ability.into())));
    }
    Ok(None)
}

fn parse_this_cant_attack_unless_static_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    if !line_is_this_cant_attack_unless_clause(normalized_line(view)) {
        return Ok(None);
    }
    match parse_static_ability_ast_line(view.tokens) {
        Ok(Some(abilities)) => {
            parser_trace(
                "parse_line:branch=this-cant-attack-unless-static",
                view.tokens,
            );
            Ok(Some(line_ast_from_static_abilities(abilities)))
        }
        _ => Err(line_rule_unsupported(
            view,
            "this-cant-attack-unless-static",
            "unsupported this-cant-attack-unless static clause",
        )),
    }
}

fn parse_cast_this_spell_only_static_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    if !line_is_cast_this_spell_only_clause(normalized_line(view)) {
        return Ok(None);
    }
    match parse_cast_this_spell_only_line(view.tokens) {
        Ok(Some(ability)) => {
            parser_trace("parse_line:branch=this-spell-cast-only-static", view.tokens);
            Ok(Some(LineAst::StaticAbility(ability.into())))
        }
        _ => Err(line_rule_unsupported(
            view,
            "cast-this-spell-only-static",
            "unsupported cast-this-spell-only static clause",
        )),
    }
}

fn parse_skulk_rules_text_static_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    if !line_is_skulk_rules_text_clause(normalized_line(view)) {
        return Ok(None);
    }
    match parse_static_ability_ast_line(view.tokens) {
        Ok(Some(abilities)) => {
            parser_trace("parse_line:branch=skulk-rules-text-static", view.tokens);
            Ok(Some(line_ast_from_static_abilities(abilities)))
        }
        _ => Err(line_rule_unsupported(
            view,
            "skulk-rules-text-static",
            "unsupported skulk-rules-text static clause",
        )),
    }
}

fn parse_statement_verb_leading_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let words = view.words.as_slice();
    if !line_starts_with_statement_effect_head(view.tokens, words)
        || is_untap_during_each_other_players_untap_step_words(words)
        || line_is_damage_prevent_with_remove_static(words)
        || line_is_prevent_all_damage_to_source_by_creatures_static(words)
        || line_is_prevent_all_combat_damage_to_source_static(words)
    {
        return Ok(None);
    }

    match parse_effect_sentences(view.tokens) {
        Ok(effects) if !effects.is_empty() => {
            parser_trace("parse_line:branch=statement-verb-leading", view.tokens);
            Ok(Some(LineAst::Statement { effects }))
        }
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

fn parse_static_line_rule(view: &ClauseView<'_>) -> Result<Option<LineAst>, CardTextError> {
    if parse_ability_line(view.tokens).is_some()
        && !line_prefers_static_parse(view.words.as_slice())
    {
        return Ok(None);
    }
    let Some(abilities) = parse_static_ability_ast_line(view.tokens)? else {
        return Ok(None);
    };
    parser_trace("parse_line:branch=static", view.tokens);
    Ok(Some(line_ast_from_static_abilities(abilities)))
}

fn parse_keyword_ability_line_rule(
    view: &ClauseView<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    if line_prefers_static_parse(view.words.as_slice()) {
        return Ok(None);
    }
    if let Some(actions) = parse_ability_line(view.tokens) {
        parser_trace("parse_line:branch=keyword-ability-line", view.tokens);
        return Ok(Some(LineAst::Abilities(actions)));
    }
    Ok(None)
}

fn line_prefers_static_parse(words: &[&str]) -> bool {
    words.len() > 3
        && words.ends_with(&["cant", "be", "blocked"])
        && !words.starts_with(&["protection", "from"])
}

fn parse_statement_line_rule(view: &ClauseView<'_>) -> Result<Option<LineAst>, CardTextError> {
    parser_trace("parse_line:branch=statement", view.tokens);
    let effects = parse_effect_sentences(view.tokens)?;
    if effects.is_empty() {
        parser_trace("parse_line:branch=statement-empty", view.tokens);
        let head = view
            .tokens
            .first()
            .and_then(Token::as_word)
            .unwrap_or("unknown-head");
        let line = view.raw.unwrap_or_default();
        return Err(CardTextError::ParseError(format!(
            "unsupported line (no-line-rule-match, head='{head}'): {line}"
        )));
    }
    Ok(Some(LineAst::Statement { effects }))
}

const PRE_DIAGNOSTIC_LINE_PARSE_RULES: [RuleDef<LineAst>; 17] = [
    RuleDef {
        id: "cost-floor-reminder",
        priority: 100,
        heads: &["this"],
        shape_mask: 0,
        run: parse_cost_floor_reminder_rule,
    },
    RuleDef {
        id: "self-enters-with-counters",
        priority: 110,
        heads: &["this", "it"],
        shape_mask: 0,
        run: parse_self_enters_with_counters_rule,
    },
    RuleDef {
        id: "saga",
        priority: 120,
        heads: &[],
        shape_mask: 0,
        run: parse_saga_line_rule,
    },
    RuleDef {
        id: "replicate",
        priority: 130,
        heads: &["replicate"],
        shape_mask: 0,
        run: parse_replicate_line_rule,
    },
    RuleDef {
        id: "additional-cost",
        priority: 140,
        heads: &["as"],
        shape_mask: 0,
        run: parse_additional_cost_line_rule,
    },
    RuleDef {
        id: "alternative-casting-method",
        priority: 150,
        heads: &["if", "you", "escape", "bestow", "flashback", "madness"],
        shape_mask: 0,
        run: parse_alternative_cast_line_rule,
    },
    RuleDef {
        id: "parsed-ability",
        priority: 160,
        heads: &[],
        shape_mask: 0,
        run: parse_parsed_ability_line_rule,
    },
    RuleDef {
        id: "optional-cost",
        priority: 170,
        heads: &["buyback", "kicker", "multikicker", "entwine", "squad"],
        shape_mask: 0,
        run: parse_optional_cost_line_rule,
    },
    RuleDef {
        id: "pact-next-upkeep",
        priority: 175,
        heads: &["search"],
        shape_mask: 0,
        run: parse_pact_next_upkeep_line_rule,
    },
    RuleDef {
        id: "triggered",
        priority: 180,
        heads: &[],
        shape_mask: 0,
        run: parse_triggered_line_rule,
    },
    RuleDef {
        id: "waterbend-activated",
        priority: 190,
        heads: &["waterbend"],
        shape_mask: 0,
        run: parse_waterbend_activated_line_rule,
    },
    RuleDef {
        id: "activated",
        priority: 200,
        heads: &[],
        shape_mask: RULE_SHAPE_HAS_COLON,
        run: parse_activated_line_rule,
    },
    RuleDef {
        id: "statement-token-mana-reminder",
        priority: 210,
        heads: &[],
        shape_mask: 0,
        run: parse_token_mana_reminder_statement_rule,
    },
    RuleDef {
        id: "if-this-spell-costs-less",
        priority: 220,
        heads: &["if"],
        shape_mask: 0,
        run: parse_if_this_spell_costs_less_line_rule,
    },
    RuleDef {
        id: "this-cant-attack-unless-static",
        priority: 230,
        heads: &["this"],
        shape_mask: 0,
        run: parse_this_cant_attack_unless_static_rule,
    },
    RuleDef {
        id: "cast-this-spell-only-static",
        priority: 240,
        heads: &["cast"],
        shape_mask: 0,
        run: parse_cast_this_spell_only_static_rule,
    },
    RuleDef {
        id: "skulk-rules-text-static",
        priority: 250,
        heads: &["creatures"],
        shape_mask: 0,
        run: parse_skulk_rules_text_static_rule,
    },
];

const PRE_DIAGNOSTIC_LINE_PARSE_INDEX: RuleIndex<LineAst> =
    RuleIndex::new(&PRE_DIAGNOSTIC_LINE_PARSE_RULES);

const POST_DIAGNOSTIC_LINE_PARSE_RULES: [RuleDef<LineAst>; 4] = [
    RuleDef {
        id: "statement-verb-leading",
        priority: 260,
        heads: &[],
        shape_mask: 0,
        run: parse_statement_verb_leading_rule,
    },
    RuleDef {
        id: "static",
        priority: 270,
        heads: &[],
        shape_mask: 0,
        run: parse_static_line_rule,
    },
    RuleDef {
        id: "keyword-ability-line",
        priority: 280,
        heads: &[],
        shape_mask: 0,
        run: parse_keyword_ability_line_rule,
    },
    RuleDef {
        id: "statement",
        priority: 290,
        heads: &[],
        shape_mask: 0,
        run: parse_statement_line_rule,
    },
];

const POST_DIAGNOSTIC_LINE_PARSE_INDEX: RuleIndex<LineAst> =
    RuleIndex::new(&POST_DIAGNOSTIC_LINE_PARSE_RULES);

fn run_line_parse_rules(view: &ClauseView<'_>) -> Result<(&'static str, LineAst), CardTextError> {
    if let Some((rule_id, ast)) = PRE_DIAGNOSTIC_LINE_PARSE_INDEX.run_first(view)? {
        return Ok((rule_id, ast));
    }

    // Run explicit unsupported-grammar checks before the broad fallback parsers.
    if let Some(diag) = diagnose_line_unsupported(view) {
        return Err(diag);
    }

    if let Some((rule_id, ast)) = POST_DIAGNOSTIC_LINE_PARSE_INDEX.run_first(view)? {
        return Ok((rule_id, ast));
    }

    Err(CardTextError::InvariantViolation(format!(
        "missing line parse rule for line: '{}'",
        view.display_text()
    )))
}

pub(crate) fn parse_line(line: &str, line_index: usize) -> Result<LineAst, CardTextError> {
    parser_trace_line("parse_line:entry", line);
    let normalized = line
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    let normalized = normalized.replace('\'', "").replace('’', "");
    let normalized_without_braces = normalized.replace('{', "").replace('}', "");
    let normalized_without_braces = normalized_without_braces.trim_end_matches('.');
    let tokens = tokenize_line(line, line_index);
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty line".to_string()));
    }
    let line_view = ClauseView::from_line(
        line,
        &normalized,
        normalized_without_braces,
        &tokens,
        line_index,
    );
    let (_, ast) = run_line_parse_rules(&line_view)?;
    Ok(ast)
}

pub(crate) fn parse_additional_cost_choice_options(
    tokens: &[Token],
) -> Result<Option<Vec<AdditionalCostChoiceOptionAst>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"or") {
        return Ok(None);
    }

    let option_tokens = split_on_or(tokens);
    if option_tokens.len() < 2 {
        return Ok(None);
    }

    let mut normalized_options = Vec::new();
    for mut option in option_tokens {
        while option
            .first()
            .is_some_and(|token| token.is_word("and") || token.is_word("or"))
        {
            option.remove(0);
        }
        let option = trim_commas(&option).to_vec();
        if option.is_empty() {
            continue;
        }
        normalized_options.push(option);
    }

    if normalized_options.len() < 2 {
        return Ok(None);
    }

    // If any branch lacks a verb, this "or" belongs to a noun phrase
    // (for example, "discard a red or green card"), not a cost choice.
    if normalized_options
        .iter()
        .any(|option| find_verb(option).is_none())
    {
        return Ok(None);
    }

    let mut options = Vec::new();
    for option in normalized_options {
        let effects = parse_effect_sentences(&option)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "additional cost option parsed to no effects (clause: '{}')",
                words(&option).join(" ")
            )));
        }
        options.push(AdditionalCostChoiceOptionAst {
            description: words(&option).join(" "),
            effects,
        });
    }

    if options.len() < 2 {
        return Ok(None);
    }

    Ok(Some(options))
}

pub(crate) fn is_at_trigger_intro(tokens: &[Token], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return false;
    }

    let second = tokens.get(idx + 1).and_then(Token::as_word);
    let third = tokens.get(idx + 2).and_then(Token::as_word);
    matches!(
        (second, third),
        (Some("beginning"), _)
            | (Some("end"), _)
            | (Some("the"), Some("beginning"))
            | (Some("the"), Some("end"))
    )
}

pub(crate) fn starts_with_activation_cost(tokens: &[Token]) -> bool {
    let Some(word) = tokens.first().and_then(Token::as_word) else {
        return false;
    };
    if matches!(
        word,
        "tap"
            | "t"
            | "pay"
            | "discard"
            | "mill"
            | "sacrifice"
            | "put"
            | "remove"
            | "exile"
            | "return"
            | "e"
    ) {
        return true;
    }
    if word.contains('/') {
        return parse_mana_symbol_group(word).is_ok();
    }
    parse_mana_symbol(word).is_ok()
}

pub(crate) fn find_activation_cost_start(tokens: &[Token]) -> Option<usize> {
    (0..tokens.len()).find(|idx| starts_with_activation_cost(&tokens[*idx..]))
}

pub(crate) fn parse_flashback_keyword_line(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let words_all = words(tokens);
    if words_all.first().copied() != Some("flashback") {
        return None;
    }
    let (cost, consumed) = leading_mana_symbols_to_oracle(&words_all[1..])?;
    let mut text = format!("Flashback {cost}");
    let tail = &words_all[1 + consumed..];
    if !tail.is_empty() {
        let mut tail_text = tail.join(" ");
        if let Some(first) = tail_text.chars().next() {
            let upper = first.to_ascii_uppercase().to_string();
            let rest = &tail_text[first.len_utf8()..];
            tail_text = format!("{upper}{rest}");
        }
        text.push_str(", ");
        text.push_str(&tail_text);
    }
    Some(vec![KeywordAction::MarkerText(text)])
}

pub(crate) fn parse_flashback_line(
    tokens: &[Token],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("flashback"))
    {
        return Ok(None);
    }

    let cost_start = 1usize;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(
            "flashback keyword missing mana cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(&tokens[cost_start..])?;
    if total_cost.mana_cost().is_none() {
        return Err(CardTextError::ParseError(
            "flashback keyword missing mana symbols".to_string(),
        ));
    }

    Ok(Some(AlternativeCastingMethod::Flashback { total_cost }))
}

pub(crate) fn parse_bestow_line(
    tokens: &[Token],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("bestow")) {
        return Ok(None);
    }

    let words_all = words(tokens);
    let (mana_cost_text, mana_word_count) = leading_mana_symbols_to_oracle(&words_all[1..])
        .ok_or_else(|| CardTextError::ParseError("bestow keyword missing mana cost".to_string()))?;
    let mana_cost = parse_scryfall_mana_cost(&mana_cost_text).map_err(|err| {
        CardTextError::ParseError(format!(
            "invalid bestow mana cost '{mana_cost_text}': {err:?}"
        ))
    })?;
    let mut total_cost = TotalCost::mana(mana_cost.clone());

    let mut consumed_mana_tokens = 0usize;
    for token in tokens.iter().skip(1) {
        let Some(word) = token.as_word() else {
            break;
        };
        if parse_mana_symbol(word).is_ok() {
            consumed_mana_tokens += 1;
            continue;
        }
        break;
    }
    if consumed_mana_tokens == 0 {
        consumed_mana_tokens = mana_word_count;
    }
    consumed_mana_tokens = consumed_mana_tokens.min(tokens.len().saturating_sub(1));

    let mut cost_tokens = tokens
        .get(1..1 + consumed_mana_tokens)
        .unwrap_or_default()
        .to_vec();
    let tail_tokens = tokens.get(1 + consumed_mana_tokens..).unwrap_or_default();
    if tail_tokens
        .first()
        .is_some_and(|token| matches!(token, Token::Comma(_)))
    {
        let clause_end = tail_tokens
            .iter()
            .position(|token| matches!(token, Token::Period(_)))
            .unwrap_or(tail_tokens.len());
        let clause_tokens = trim_commas(&tail_tokens[..clause_end]).to_vec();
        let clause_words = words(&clause_tokens);
        if !clause_words.is_empty() && clause_words[0] != "if" {
            cost_tokens.extend(clause_tokens);
        }
    }

    if let Ok(parsed_total_cost) = parse_activation_cost(&cost_tokens) {
        total_cost = parsed_total_cost;
        if total_cost.mana_cost().is_none() {
            let mut components = total_cost.costs().to_vec();
            components.insert(0, crate::costs::Cost::mana(mana_cost));
            total_cost = TotalCost::from_costs(components);
        }
    }

    Ok(Some(AlternativeCastingMethod::Bestow { total_cost }))
}

fn is_self_free_cast_clause(words: &[&str]) -> bool {
    words
        == [
            "you", "may", "cast", "this", "spell", "without", "paying", "its", "mana", "cost",
        ]
        || words
            == [
                "you", "may", "cast", "this", "spell", "without", "paying", "this", "spells",
                "mana", "cost",
            ]
}

pub(crate) fn parse_self_free_cast_alternative_cost_line(
    tokens: &[Token],
) -> Option<AlternativeCastingMethod> {
    let clause_words = words(tokens);
    if !is_self_free_cast_clause(&clause_words) {
        return None;
    }
    Some(AlternativeCastingMethod::alternative_cost(
        "Parsed alternative cost",
        None,
        Vec::new(),
    ))
}

pub(crate) fn parse_you_may_rather_than_spell_cost_line(
    tokens: &[Token],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !(tokens.first().is_some_and(|token| token.is_word("you"))
        && tokens.get(1).is_some_and(|token| token.is_word("may")))
    {
        return Ok(None);
    }
    let Some(rather_idx) = tokens.iter().position(|token| token.is_word("rather")) else {
        return Ok(None);
    };
    let rather_tail = words(tokens.get(rather_idx + 1..).unwrap_or_default());
    let is_spell_cost_clause = rather_tail.starts_with(&["than", "pay", "this"])
        && rather_tail.contains(&"mana")
        && rather_tail.contains(&"cost")
        && (rather_tail.contains(&"spell") || rather_tail.contains(&"spells"));
    if !is_spell_cost_clause {
        return Ok(None);
    }
    let cost_clause_end = (rather_idx + 1..tokens.len())
        .rfind(|idx| tokens[*idx].is_word("cost") || tokens[*idx].is_word("costs"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "alternative cost line missing terminal cost word (line: '{}')",
                line
            ))
        })?;
    let trailing_words = words(&tokens[cost_clause_end + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing clause after alternative cost (line: '{}', trailing: '{}')",
            line,
            trailing_words.join(" ")
        )));
    }
    let cost_tokens = tokens.get(2..rather_idx).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "alternative cost line missing cost clause".to_string(),
        ));
    }
    let total_cost = parse_activation_cost(cost_tokens)?;
    Ok(Some(AlternativeCastingMethod::Composed {
        name: "Parsed alternative cost",
        total_cost,
        condition: None,
    }))
}

fn trap_condition_from_this_spell_cost_condition(
    condition: &crate::static_abilities::ThisSpellCostCondition,
) -> Option<crate::TrapCondition> {
    match condition {
        crate::static_abilities::ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(
            count,
        ) => Some(crate::TrapCondition::OpponentCastSpells { count: *count }),
        _ => None,
    }
}

fn simple_trap_cost_from_alternative_method(
    method: &AlternativeCastingMethod,
) -> Option<crate::mana::ManaCost> {
    let AlternativeCastingMethod::Composed { total_cost, .. } = method else {
        return None;
    };
    if total_cost.non_mana_costs().next().is_some() {
        return None;
    }
    Some(
        total_cost
            .mana_cost()
            .cloned()
            .unwrap_or_else(crate::mana::ManaCost::new),
    )
}

pub(crate) fn parse_if_conditional_alternative_cost_line(
    tokens: &[Token],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["if"]) {
        return Ok(None);
    }

    let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };
    let condition_tokens = trim_commas(&tokens[1..comma_idx]);
    let tail_tokens = trim_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    let tail_words = words(&tail_tokens);
    if !is_self_free_cast_clause(&tail_words)
        && parse_you_may_rather_than_spell_cost_line(&tail_tokens, line)?.is_none()
    {
        return Ok(None);
    }
    let Some(condition) = parse_this_spell_cost_condition(&condition_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    if is_self_free_cast_clause(&tail_words) {
        let method = AlternativeCastingMethod::alternative_cost_with_condition(
            "Parsed alternative cost",
            None,
            Vec::new(),
            condition,
        );
        if let Some(trap_condition) = method
            .cast_condition()
            .and_then(trap_condition_from_this_spell_cost_condition)
            && let Some(cost) = simple_trap_cost_from_alternative_method(&method)
        {
            return Ok(Some(AlternativeCastingMethod::trap(
                "Trap",
                cost,
                trap_condition,
            )));
        }
        return Ok(Some(method));
    }

    let Some(method) = parse_you_may_rather_than_spell_cost_line(&tail_tokens, line)? else {
        return Ok(None);
    };
    let method = method.with_cast_condition(condition);
    if let Some(trap_condition) = method
        .cast_condition()
        .and_then(trap_condition_from_this_spell_cost_condition)
        && let Some(cost) = simple_trap_cost_from_alternative_method(&method)
    {
        return Ok(Some(AlternativeCastingMethod::trap(
            "Trap",
            cost,
            trap_condition,
        )));
    }
    Ok(Some(method))
}
