use super::activation_and_restrictions::parse_cycling_line;
use super::activation_and_restrictions::{
    normalize_cant_words, parse_ability_phrase, parse_activated_line, parse_activation_cost,
    parse_choose_land_type_phrase_words, parse_payment_clause_as_total_cost,
    parse_single_word_keyword_action,
};
use super::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::effect_sentences::parse_granted_abilities_for_gain_clause;
use super::grammar::abilities::{
    CombatDamageUsingToughnessSubject, DoesntUntapDuringUntapStepSpec, FlyingBlockRestrictionKind,
    is_all_permanents_colorless_line_lexed,
    is_as_long_as_power_odd_or_even_flash_marker_line_lexed,
    is_attack_as_haste_unless_entered_this_turn_marker_line_lexed,
    is_can_be_your_commander_line_lexed, is_can_block_only_flying_line_lexed,
    is_cast_this_spell_as_though_it_had_flash_line_lexed, is_companion_marker_line_lexed,
    is_creatures_cant_block_line_lexed,
    is_creatures_entering_dont_cause_abilities_to_trigger_line_lexed,
    is_creatures_without_flying_cant_attack_line_lexed,
    is_discard_or_redirect_replacement_line_lexed, is_doctors_companion_marker_line_lexed,
    is_double_damage_from_sources_you_control_of_chosen_type_line_lexed,
    is_draw_replace_exile_top_face_down_line_lexed, is_draw_replacement_double_line_lexed,
    is_draw_replacement_skip_empty_library_line_lexed,
    is_during_your_turn_prevent_all_damage_to_source_line_lexed,
    is_effect_discard_to_library_replacement_line_lexed,
    is_enchanted_land_is_chosen_type_line_lexed,
    is_if_source_you_control_with_mana_value_double_instead_marker_line_lexed,
    is_krrik_black_mana_life_payment_line_lexed,
    is_lands_dont_untap_during_their_controllers_untap_steps_line_lexed,
    is_lethal_damage_to_creatures_you_control_uses_power_line_lexed,
    is_mana_group_slash_marker_line_lexed, is_may_assign_damage_as_unblocked_line_lexed,
    is_minimum_spell_total_mana_three_line_lexed, is_more_than_meets_the_eye_marker_line_lexed,
    is_no_maximum_hand_size_line_lexed, is_once_each_turn_play_from_exile_marker_guard_lexed,
    is_opponent_effect_discard_this_to_battlefield_replacement_line_lexed,
    is_permanents_enter_tapped_line_lexed, is_play_lands_from_graveyard_line_lexed,
    is_play_top_card_your_library_revealed_line_lexed, is_players_cant_cycle_line_lexed,
    is_players_cant_pay_life_or_sacrifice_line_lexed,
    is_players_play_top_card_libraries_revealed_line_lexed, is_players_skip_upkeep_line_lexed,
    is_prevent_all_combat_damage_to_matching_permanents_line_lexed,
    is_prevent_all_combat_damage_to_source_line_lexed,
    is_prevent_all_damage_dealt_to_creatures_line_lexed,
    is_prevent_all_damage_to_source_by_creatures_line_lexed,
    is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed,
    is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed,
    is_protection_mana_value_marker_line_lexed, is_remove_snow_line_lexed,
    is_sab_sunen_cant_attack_or_block_unless_line_lexed,
    is_shuffle_into_library_from_graveyard_line_lexed, is_skulk_rules_text_line_lexed,
    is_this_creature_cant_attack_alone_line_lexed,
    is_this_creature_cant_attack_its_owner_line_lexed, is_this_subject_reference_lexed,
    is_you_have_shroud_line_lexed,
    is_you_may_look_face_down_creatures_you_dont_control_any_time_line_lexed,
    is_you_may_look_top_card_any_time_line_lexed,
    is_your_opponents_play_with_hands_revealed_line_lexed,
    parse_activated_abilities_cant_be_activated_spec_lexed,
    parse_can_block_subtype_as_though_reach_line_lexed,
    parse_creatures_assign_combat_damage_using_toughness_line_lexed,
    parse_doesnt_untap_during_untap_step_spec_lexed,
    parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed,
    parse_flying_block_restriction_line_lexed,
    parse_reveal_first_card_you_draw_each_turn_spec_lexed,
    parse_source_is_chosen_type_in_addition_line_lexed, parse_source_tap_status_condition_lexed,
    parse_trigger_suppression_spec_lexed, parse_ward_pay_life_amount_lexed,
    split_as_long_as_condition_prefix_lexed, split_if_this_spell_costs_line_lexed,
    split_untap_each_other_players_untap_step_line_lexed,
};
use super::grammar::filters::{
    parse_object_filter_with_grammar_entrypoint, parse_spell_filter_with_grammar_entrypoint,
    parse_spell_filter_with_grammar_entrypoint_lexed,
};
use super::grammar::primitives::{
    split_lexed_slices_on_and, split_lexed_slices_on_comma,
    split_lexed_slices_on_commas_or_semicolons, split_lexed_slices_on_period,
};
pub(crate) use super::grammar::values::parse_add_mana_equal_amount_value_lexed as parse_add_mana_equal_amount_value;
use super::grammar::values::parse_max_cards_in_hand_value_lexed;
use super::keyword_static_helpers::*;
use super::lexer::{
    LexedClause, OwnedLexToken, TokenKind, contains_token_kind, find_token_kind,
    find_token_word_sequence_span, parser_token_word_refs, render_token_slice,
    split_lexed_sentences, token_slice_at_is, token_slice_at_is_any, token_slice_first_is,
    token_slice_first_is_any, token_slice_starts_with, trim_lexed_commas, word_slice_at_is,
    word_slice_at_is_any, word_slice_contains_all_words, word_slice_contains_any_phrase,
    word_slice_contains_any_word, word_slice_contains_no_words, word_slice_contains_phrase,
    word_slice_contains_word, word_slice_ends_with, word_slice_ends_with_any, word_slice_eq,
    word_slice_eq_any, word_slice_eq_any_at, word_slice_eq_at, word_slice_find_any_phrase_span,
    word_slice_find_any_phrase_start, word_slice_find_phrase_start,
    word_slice_find_phrase_start_or_zero, word_slice_find_word, word_slice_find_word_where,
    word_slice_first_is, word_slice_first_is_any, word_slice_last_is, word_slice_last_is_any,
    word_slice_starts_with, word_slice_starts_with_any, word_slice_starts_with_at,
    word_slice_strip_any_prefix,
};
use super::lowering_support::rewrite_parsed_triggered_ability as parsed_triggered_ability;
use super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::rule_engine::{LexRuleHeadHint, LexRuleHintIndex, build_lex_rule_hint_index};
use super::static_ability_helpers::{
    afflict_triggered_ability, lower_granted_abilities_ast_to_object_abilities,
    static_ability_for_keyword_action,
};
use super::token_primitives::{
    find_index, find_window_by, is_core_keyword_marker_text, is_ticket_sticker_marker_text,
    lexed_head_words, rfind_index, slice_contains, slice_strip_prefix, slice_strip_suffix,
    split_em_dash_label_prefix, str_split_once_char, str_strip_prefix, str_strip_suffix,
};
use super::util::{
    comparison_to_at_least_threshold, comparison_to_strict_at_least_threshold,
    is_source_reference_words, leading_mana_cost_from_tokens, mana_pips_from_token,
    parse_alternative_cast_words, parse_card_type, parse_color, parse_counter_type_from_tokens,
    parse_counter_type_word, parse_filter_counter_constraint_words, parse_flashback_keyword_line,
    parse_for_each_count_value_words, parse_greater_than_or_equal_quantity_prefix,
    parse_less_than_or_equal_quantity_prefix, parse_mana_symbol_word_flexible, parse_number_word_i32,
    parse_quantity_comparison_prefix, parse_subtype_flexible, parse_value, parse_value_expr_words,
    parse_zone_word, preserve_keyword_prefix_for_parse,
    source_reference_surface_for_possessive_words, strip_leading_article_word_refs,
    strip_leading_token_words_any, strip_leading_word_refs_any, trim_commas,
    word_refs_at_is_article, words,
};
use super::util::{source_choose_spec_for_surface, source_reference_surface_for_words};
use super::value_helpers::{parse_commander_cast_count_player, parse_filter_comparison_tokens};
#[allow(unused_imports)]
use crate::ability::{Ability, AbilityKind, TriggeredAbility};
#[allow(unused_imports)]
use crate::alternative_cast::AlternativeCastingMethod;
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, GrantedAbilityAst, IT_TAG, KeywordAction, LineAst, ParsedAbility,
    ReferenceImports, StaticAbilityAst, TagKey, TextSpan,
};
#[allow(unused_imports)]
use crate::color::{Color, ColorSet};
#[allow(unused_imports)]
use crate::cost::TotalCost;
#[allow(unused_imports)]
use crate::effect::{Condition, Effect, EventValueSpec, Value};
#[allow(unused_imports)]
use crate::mana::{ManaCost, ManaSymbol};
#[allow(unused_imports)]
use crate::object::CounterType;
#[allow(unused_imports)]
use crate::static_abilities::{
    Anthem, AnthemCountExpression, AnthemValue, GrantAbility, PowerToughnessChoiceOption,
    StaticAbility,
};
#[allow(unused_imports)]
use crate::target::{ChooseSpec, ChooseSpecSurfaceHint, ObjectFilter, PlayerFilter};
#[allow(unused_imports)]
use crate::triggers::Trigger;
#[allow(unused_imports)]
use crate::types::{CardType, Subtype, Supertype};
#[allow(unused_imports)]
use crate::zone::Zone;
use ironsmith_core::{EffectMetric, EffectMetricSource};
use std::sync::LazyLock;

const AS_ENTERS_AURA_SUBJECTS: &[(&str, &str)] = &[("aura", "this Aura")];
const SOURCE_CAN_BLOCK_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "creature", "can", "block"]);
const BLOCK_ADDITIONAL_DURATION_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["each", "combat"], &["this", "turn"]]);
const BLOCK_ADDITIONAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["additional"]);
const BLOCK_CREATURE_OR_CREATURES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["creature"], &["creatures"]]);
const DAY_NIGHT_AS_ENTERS_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "if", "its", "neither", "day", "nor", "night", "it", "becomes", "day", "as",
                "this", "creature", "enters",
            ],
            &[
                "if",
                "its",
                "neither",
                "day",
                "nor",
                "night",
                "it",
                "becomes",
                "day",
                "as",
                "this",
                "permanent",
                "enters",
            ],
            &[
                "if", "its", "neither", "day", "nor", "night", "it", "becomes", "day", "as",
                "this", "object", "enters",
            ],
        ]
);
const DAY_NIGHT_AS_ENTERS_CONTAINS_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_phrases(&[&["it", "becomes", "day"]])
    .contains_any_phrases(&[
        &[
            &["its", "neither", "day", "nor", "night"],
            &["it's", "neither", "day", "nor", "night"],
        ],
        &[
            &["as", "this", "creature", "enters"],
            &["as", "this", "permanent", "enters"],
            &["as", "this", "object", "enters"],
        ],
    ]);
const TOUGHNESS_CREWS_VEHICLES_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "this",
                "creature",
                "crews",
                "vehicles",
                "using",
                "its",
                "toughness",
                "rather",
                "than",
                "its",
                "power",
            ],
            &[
                "this",
                "creature",
                "saddles",
                "mounts",
                "and",
                "crews",
                "vehicles",
                "using",
                "its",
                "toughness",
                "rather",
                "than",
                "its",
                "power",
            ],
        ]
);
const POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "crews", "vehicles", "as", "though", "its", "power", "were"],
            &[
                "this",
                "creature",
                "saddles",
                "mounts",
                "and",
                "crews",
                "vehicles",
                "as",
                "though",
                "its",
                "power",
                "were",
            ],
            &[
                "this",
                "token",
                "saddles",
                "mounts",
                "and",
                "crews",
                "vehicles",
                "as",
                "though",
                "its",
                "power",
                "were",
            ],
        ];
    suffix & ["greater"]
);
const LOYALTY_COUNTER_INSTEAD_OF_CREW_COST_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "may", "remove", "a", "loyalty", "counter", "from", "a", "planeswalker", "you", "control", "rather", "than", "pay"]; suffix & ["crew", "cost"]);
const DAMAGE_DOUBLING_MANA_VALUE_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["if", "a", "source", "you", "control", "with"];
    suffix & ["instead"];
    contains_words & ["mana", "value", "double"]
);
const DAMAGE_DOUBLING_TO_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["would", "deal", "damage", "to", "a"],
            &["would", "deal", "damage", "to", "target"],
        ]]
);
const DAMAGE_PLUS_REPLACEMENT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "if",
            "a",
            "source",
            "you",
            "control",
            "would",
            "deal",
            "damage",
            "to",
            "an",
            "opponent",
            "or",
            "a",
            "permanent",
            "an",
            "opponent",
            "controls",
            "it",
            "deals",
            "that",
            "much",
            "damage",
            "plus",
        ];
    suffix & ["instead"]
);
const WOULD_DEAL_DAMAGE_TO_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["would", "deal", "damage", "to"]);
const WOULD_DEAL_DAMAGE_TO_YOU_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["would", "deal", "damage", "to", "you"]);
const IT_DEALS_MULTIPLE_THAT_DAMAGE_TO_PHRASE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "deals", "double", "that", "damage", "to"],
            &["it", "deals", "triple", "that", "damage", "to"],
        ]
);
const THAT_SOURCE_DEALS_DAMAGE_EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "source", "deals", "damage", "equal", "to"]);
const DAMAGE_REDIRECT_TO_SOURCE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "all", "damage", "that", "would", "be", "dealt", "to", "you", "and", "other",
        ]
);
const MINIMUM_RED_NONCOMBAT_DAMAGE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "if",
            "a",
            "red",
            "source",
            "you",
            "control",
            "would",
            "deal",
            "an",
            "amount",
            "of",
            "noncombat",
            "damage",
            "less",
            "than",
        ]
);
const AN_OPPONENT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["an", "opponent"]);
const SOURCE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["source"]);
const FORETELLING_CARDS_FROM_HAND_COSTS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["foretelling", "cards", "from", "your", "hand", "costs"]);
const ANY_PLAYER_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["on", "any", "players", "turn"],
            &["on", "any", "player", "turn"],
            &["on", "any", "player", "s", "turn"],
        ]]
);
const PERMANENT_OR_PERMANENTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["permanent"], &["permanents"]]);
const LAND_OR_LANDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["land"], &["lands"]]);
const STATIC_CREATURE_OR_CREATURES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["creature"], &["creatures"]]);
const BASE_POWER_TOUGHNESS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["base", "power", "and", "toughness"]);
const EVERY_BASIC_LAND_TYPE_ADDITION_TAIL_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .exact_any(&[
        &[
            "every", "basic", "land", "type", "in", "addition", "to", "its", "other", "type",
        ],
        &[
            "every", "basic", "land", "type", "in", "addition", "to", "its", "other", "types",
        ],
        &[
            "every", "basic", "land", "types", "in", "addition", "to", "its", "other", "type",
        ],
        &[
            "every", "basic", "land", "types", "in", "addition", "to", "its", "other", "types",
        ],
        &[
            "every", "basic", "land", "type", "in", "addition", "to", "their", "other", "type",
        ],
        &[
            "every", "basic", "land", "type", "in", "addition", "to", "their", "other", "types",
        ],
        &[
            "every", "basic", "land", "types", "in", "addition", "to", "their", "other", "type",
        ],
        &[
            "every", "basic", "land", "types", "in", "addition", "to", "their", "other", "types",
        ],
    ]);
const LAND_TYPE_ADDITION_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["in", "addition", "to", "its", "other", "land", "type"],
            &["in", "addition", "to", "its", "other", "land", "types"],
            &["in", "addition", "to", "their", "other", "land", "type"],
            &["in", "addition", "to", "their", "other", "land", "types"],
        ]
);
const STILL_LAND_ANIMATION_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "are", "still", "land"],
            &["that", "are", "still", "lands"],
            &["that", "is", "still", "land"],
            &["that", "is", "still", "a", "land"],
        ]
);
const OTHER_TYPE_ADDITION_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["in", "addition", "to", "its", "other", "type"],
            &["in", "addition", "to", "its", "other", "types"],
            &["in", "addition", "to", "their", "other", "type"],
            &["in", "addition", "to", "their", "other", "types"],
        ]
);
const MANA_VALUE_INSTEAD_OF_MANA_COST_GRANT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "you", "may", "pay", "x", "rather", "than", "pay", "the", "mana", "cost", "for",
        ]
);
const LIFE_MANA_VALUE_INSTEAD_OF_MANA_COST_GRANT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "once", "during", "each", "of", "your", "turns", "you", "may", "cast",
        ]
);
const LIFE_MANA_VALUE_INSTEAD_OF_MANA_COST_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "paying", "life", "equal", "to", "its", "mana", "value", "rather", "than",
                "paying", "its", "mana", "cost",
            ],
            &[
                "pay", "life", "equal", "to", "its", "mana", "value", "rather", "than", "pay",
                "its", "mana", "cost",
            ],
        ]
);
const RATHER_THAN_PAY_MANA_COST_FOR_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["rather", "than", "pay", "the", "mana", "cost", "for",]);
const SPELL_OR_SPELLS_CONTAINS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["spell", "spells"]]);
const DRAW_REPLACEMENT_EXILE_TOP_PLAY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "if", "you", "would", "draw", "a", "card", "exile", "the", "top",
        ]
);
const DRAW_REPLACEMENT_EXILE_TOP_PLAY_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "of", "your", "library", "instead", "you", "may", "play", "those", "cards", "this",
            "turn",
        ]
);
const CONDITIONAL_DRAW_REPLACEMENT_A_CARD_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "you", "would", "draw", "a", "card", "while"]);
const CONDITIONAL_DRAW_REPLACEMENT_CARD_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "you", "would", "draw", "card", "while"]);
const YOU_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const DRAW_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["draw"]);
const INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const CONDITIONAL_DRAW_LIFE_LOSS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["and", "you", "lose"];
    suffix & ["life"]
);
const YOU_PROLIFERATE_TWICE_INSTEAD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "if",
            "you",
            "would",
            "proliferate",
            "proliferate",
            "twice",
            "instead",
        ]
);
const OPPONENT_PROLIFERATES_TWICE_INSTEAD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "if",
            "an",
            "opponent",
            "would",
            "proliferate",
            "that",
            "player",
            "proliferates",
            "twice",
            "instead",
        ]
);
const CONTROLLED_CREATURE_EXPLORE_REPLACEMENT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "if", "a", "creature", "you", "control", "would", "explore", "instead"
        ]
);
const YOU_SCRY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "scry"]);
const EXPLORES_TWICE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["it", "explores", "then", "it", "explores", "again"]);
const EXPLORE_REPLACEMENT_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it", "explores"], &["that", "creature", "explores"]]);
const SOURCE_LINKED_EXILE_CAST_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "during", "each", "players", "turn", "that", "player", "may", "cast", "a", "spell",
            "from", "among", "the", "cards", "they", "dont", "own", "exiled", "with",
        ]
);
const ANY_MANA_CAST_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix
        & [
            "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "it",
        ]
);
const CAST_SINGLE_SPELL_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["cast", "a", "spell"], &["cast", "one", "spell"]]]);
const CONTROL_OPPONENTS_WHILE_SEARCHING_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "you",
                "control",
                "your",
                "opponents",
                "while",
                "theyre",
                "searching",
                "their",
                "libraries",
            ],
            &[
                "you",
                "control",
                "your",
                "opponents",
                "while",
                "they're",
                "searching",
                "their",
                "libraries",
            ],
        ]
);
const OPPONENT_SEARCH_EXILE_FOUND_CARDS_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "while",
            "an",
            "opponent",
            "is",
            "searching",
            "their",
            "library",
            "they",
            "exile",
            "each",
            "card",
            "they",
            "find",
            "you",
            "may",
            "play",
            "those",
            "cards",
            "for",
            "as",
            "long",
            "as",
            "they",
            "remain",
            "exiled",
            "and",
            "you",
            "may",
            "spend",
            "mana",
            "as",
            "though",
            "it",
            "were",
            "mana",
            "of",
            "any",
            "color",
            "to",
            "cast",
            "them",
        ]
);
const CAST_THIS_CARD_FROM_LIBRARY_WHILE_SEARCHING_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "while",
                "youre",
                "searching",
                "your",
                "library",
                "you",
                "may",
                "cast",
                "this",
                "card",
                "from",
                "your",
                "library",
            ],
            &[
                "while",
                "you're",
                "searching",
                "your",
                "library",
                "you",
                "may",
                "cast",
                "this",
                "card",
                "from",
                "your",
                "library",
            ],
        ]
);
const ATTACHED_CONTROLLER_ATTACK_EACH_COMBAT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "all",
                "creatures",
                "attack",
                "enchanted",
                "creatures",
                "controller",
                "each",
                "combat",
                "if",
                "able",
            ],
            &[
                "all",
                "creatures",
                "attack",
                "enchanted",
                "creature",
                "controller",
                "each",
                "combat",
                "if",
                "able",
            ],
            &[
                "all",
                "creatures",
                "attack",
                "enchanted",
                "creature's",
                "controller",
                "each",
                "combat",
                "if",
                "able",
            ],
        ]
);
const ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["attacks", "each", "combat", "if", "able"],
            &["attack", "each", "combat", "if", "able"],
        ]
);
const ATTACK_OR_ATTACKS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["attack"], &["attacks"]]);
const YOU_MAY_PLAY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "may", "play"]);
const UP_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["up", "to"]);
const ADDITIONAL_LAND_PLAY_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["additional", "land", "on", "each", "of", "your", "turns"],
            &["additional", "lands", "on", "each", "of", "your", "turns"],
        ]
);
const RETRACE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["retrace"]);
const STATIC_IN_YOUR_GRAVEYARD_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["in", "your", "graveyard"]);
const EXILE_TO_EXILE_INSTEAD_OF_GRAVEYARD_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["if"];
    suffix & ["exile", "it", "instead"];
    contains_words & ["would", "graveyard", "anywhere"]
);
const WASNT_CYCLED_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["and", "it", "wasnt", "cycled"],
            &["and", "it", "wasn't", "cycled"]
        ]]
);
const CARD_OR_TOKEN_FILTER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "card", "or", "token"], &["card", "or", "token"]]);
const CARD_FILTER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "card"], &["card"]]);
const CREATURE_CARD_FILTER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "creature", "card"], &["creature", "card"]]);
const CYCLING_CARD_FILTER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "card", "that", "has", "a", "cycling", "ability"],
            &["card", "that", "has", "a", "cycling", "ability"],
        ]
);
const AS_THIS_CONTAINS_PAY_LIFE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["as", "this"]; contains_words & ["pay", "life"]);
const PAY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["pay"]);
const MAY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["may"]);
const IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const IF_YOU_DONT_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["if", "you", "dont"]]);
const IF_YOU_DONT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "you", "dont"]);
const PAY_LIFE_ENTER_TAPPED_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["it", "enters", "tapped"],
            &["it", "enter", "tapped"],
            &["it", "enters", "the", "battlefield", "tapped"],
            &["it", "enter", "the", "battlefield", "tapped"],
        ]
);
const HAS_ALL_ACTIVATED_ABILITIES_OF_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["has", "all", "activated", "abilities", "of"],
            &["have", "all", "activated", "abilities", "of"]
        ]
);
const SAME_NAME_AS_SOURCE_CREATURE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["same", "name", "as", "this", "creature"],
            &["same", "name", "as", "thiss", "creature"],
        ]
);
const ACTIVATE_EACH_OF_THOSE_ONCE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "you",
            "may",
            "activate",
            "each",
            "of",
            "those",
            "abilities",
            "only",
            "once",
            "each",
            "turn",
        ]
);
const DAMAGE_REDIRECT_TO_SOURCE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "you", "control", "is", "dealt", "to", "this", "creature", "instead",
        ]
);
const IT_DEALS_DAMAGE_TO_ITS_CONTROLLER_INSTEAD_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "it",
            "deals",
            "that",
            "damage",
            "to",
            "its",
            "controller",
            "instead",
        ]
);
const ACTIVATED_ABILITIES_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["activated", "abilities", "of"]);
const CANT_BE_ACTIVATED_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["cant", "be", "activated"]);
const MANA_ABILITIES_EXCEPTION_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["unless", "theyre", "mana", "abilities"]]);
const ADDITIONAL_COST_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["an", "additional"], &["a", "additional"]]);
const TO_ACTIVATE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["to", "activate"]);
const PREGAME_BEGIN_ON_BATTLEFIELD_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [
            &["your", "opening", "hand"],
            &["you", "may", "begin", "the", "game", "with"],
            &["on", "the", "battlefield"],
        ]
);
const ON_THE_BATTLEFIELD_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["on", "the", "battlefield"]);
const IF_YOU_DO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["if", "you", "do"]);
const THIS_CARD_IS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "card", "is"]);
const PREGAME_COUNTER_ON_IT_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["on", "it"]);
const PREGAME_EXILE_FROM_HAND_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["card", "from", "your", "hand"],
            &["cards", "from", "your", "hand"],
        ]
);
const PREGAME_MULLIGAN_REDRAW_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [
            &["any", "time", "you", "could", "mulligan"],
            &["is", "in", "your", "hand"],
            &[
                "you", "may", "exile", "all", "the", "cards", "from", "your", "hand",
            ],
            &["then", "draw", "that", "many", "cards"],
        ]
);
const BEFORE_GAME_BEGINS_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["before", "the", "game", "begins"]);
const WARD_DISCARD_HAND_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["your", "hand"]);
const WARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["ward"]);
const DISCARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["discard"]);
const WHERE_X_IS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["where", "x", "is"]);
const WHERE_X_IS_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["where", "x", "is"]]);
const BANDING_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["banding"]);
const YOU_HAVE_HEXPROOF_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "have", "hexproof"]);
const YOU_HAVE_PROTECTION_FROM_OPPONENTS_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "you",
            "have",
            "protection",
            "from",
            "each",
            "of",
            "your",
            "opponents",
        ]
);
const OPPONENTS_CAST_ONLY_AS_SORCERY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "each", "opponent", "can", "cast", "spells", "only", "any", "time", "they", "could",
            "cast", "a", "sorcery",
        ]
);
const DOUBLE_DAMAGE_TO_ENCHANTED_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "if",
            "a",
            "source",
            "would",
            "deal",
            "damage",
            "to",
            "enchanted",
            "player",
            "it",
            "deals",
            "double",
            "that",
            "damage",
            "to",
            "that",
            "player",
            "instead",
        ]
);
const CONTROLLERS_UNTAP_STEP_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["during", "their", "controllers", "untap"],
            &["during", "its", "controllers", "untap"],
        ];
    contains_any_words & [&["step", "steps"]]
);
const THERE_IS_OR_ARE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["there", "are"], &["there", "is"]]);
const CARD_TYPES_IN_YOUR_GRAVEYARD_METRIC_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["card", "type", "among", "cards", "in", "your", "graveyard"],
            &["card", "types", "among", "cards", "in", "your", "graveyard"],
        ]
);
const MANA_VALUES_IN_YOUR_GRAVEYARD_METRIC_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["mana", "value", "among", "cards", "in", "your", "graveyard"],
            &[
                "mana",
                "values",
                "among",
                "cards",
                "in",
                "your",
                "graveyard"
            ],
        ]
);
const AS_LONG_AS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["as", "long", "as"]);
const ENTERS_TAPPED_LINE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this"]; contains_words & ["enters", "tapped"]);
const THE_BATTLEFIELD_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "battlefield"]);
const BATTLEFIELD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["battlefield"]);
const CHOOSE_CARD_NAME_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["choose", "a", "card", "name"]);
const SOURCE_IT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it"]);
const CHOICE_OR_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["or"]);
const AS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["as"]);
const THIS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this"]);
const AS_THIS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["as", "this"]);
const BECOMES_ATTACHED_TO_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["becomes", "attached", "to"]);
const ENTERS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["enters"]);
const ENTER_OR_ENTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enter"], &["enters"]]);
const IS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["is"]);
const ISNT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["isn't"]);
const IS_OR_ARE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["is"], &["are"]]);
const HAVE_OR_HAS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["have"], &["has"]]);
const HAS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["has"]);
const CHOOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["choose"]);
const POWER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["power"]);
const POWER_OR_TOUGHNESS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["power"], &["toughness"]]);
const SELF_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["thiss"], &["its"]]);
const CREATURE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["creature"]);
const CHARACTERISTIC_POWER_TOUGHNESS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["power", "and", "toughness"]]);
const CHARACTERISTIC_EQUAL_TO_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["equal", "to"]]);
const IS_EQUAL_TO_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["is", "equal", "to"]);
const THAT_NUMBER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["that", "number"]);
const PLUS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["plus"]);
const EXCEPT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["except"]);
const RESPECTIVELY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["respectively"]);
const SOURCE_POWER_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["source", "power"],
            &["sources", "power"],
            &["its", "power"],
            &["this", "power"],
            &["thiss", "power"],
            &["its", "creature", "power"],
            &["this", "creature", "power"],
            &["thiss", "creature", "power"],
        ]
);
const SOURCE_TOUGHNESS_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["its", "toughness"],
            &["this", "toughness"],
            &["thiss", "toughness"],
            &["its", "creature", "toughness"],
            &["this", "creature", "toughness"],
            &["thiss", "creature", "toughness"],
        ]
);
const CHOSEN_COLOR_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["the", "chosen", "color"], &["chosen", "color"]]);
const THE_CHOSEN_COLOR_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["the", "chosen", "color"]);
const DAMAGE_NOT_REMOVED_CLEANUP_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "damage", "isnt", "removed", "from", "this", "creature", "during", "cleanup",
                "steps",
            ],
            &[
                "damage", "isn't", "removed", "from", "this", "creature", "during", "cleanup",
                "steps",
            ],
        ]
);
const CREATURES_CAN_ATTACK_EACH_COMBAT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["creature", "can", "attack", "each", "combat"],
            &["creatures", "can", "attack", "each", "combat"],
        ]
);
const CREATURES_CAN_ATTACK_YOU_EACH_COMBAT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["creature", "can", "attack", "you", "each", "combat"],
            &["creatures", "can", "attack", "you", "each", "combat"],
        ]
);
const CREATURES_CAN_BLOCK_EACH_COMBAT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["creature", "can", "block", "each", "combat"],
            &["creatures", "can", "block", "each", "combat"],
        ]
);
const TRIGGER_DUPLICATION_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "triggers", "an", "additional", "time"],
            &["that", "ability", "triggers", "an", "additional", "time"],
        ]
);
const TRIGGER_DUPLICATION_SOURCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "or", "an", "emblem", "you", "own"],
            &["this", "creature", "or", "emblem", "you", "own"],
        ]
);
const TURNING_FACE_UP_TRIGGER_DUPLICATION_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["turning"]; suffix & ["face", "up"]);
const YOU_CASTING_OR_COPYING_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "casting", "or", "copying"]);
const DRAWING_A_CARD_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["drawing", "a", "card"]);
const PLAYER_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "player"], &["player"]]);
const YOU_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const OPPONENT_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["an", "opponent"], &["opponent"]]);
const IT_TARGETS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["it", "targets"]);
const THIS_SPELL_TARGETS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "spell", "targets"]);
const THIS_SPELL_HAS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "spell", "has"]);
const CONDITIONAL_SPELL_KEYWORD_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["flash"], &["cascade"]]);
const YOU_MAY_HAVE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "may", "have"]);
const AS_A_COPY_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["as", "a", "copy", "of"]);
const THIS_COPY_SOURCE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["this"]);
const ENCHANTED_COPY_SOURCE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["enchanted"]);
const ENTER_AS_COPY_EXILE_TWO_CREATURE_CARDS_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "you",
            "may",
            "exile",
            "two",
            "creature",
            "cards",
            "from",
            "graveyards"
        ]
);
const ENTER_AS_COPY_IF_YOU_DO_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [&[
            "if", "you", "do", "it", "enters", "as", "a", "copy", "of", "one", "of", "those",
            "cards",
        ]]
);
const ENTER_AS_COPY_COUNTER_POWER_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["additional", "+1/+1", "counters", "power", "other"]);
const ITS_NAME_IS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["its", "name", "is"]);
const IT_HAS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["it", "has"]);
const NOT_LEGENDARY_COPY_EXCEPTION_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["it", "isn't", "legendary"],
            &["it", "is", "not", "legendary"],
        ]
);
const IT_IS_OR_ITS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["it", "is"], &["it", "s"]]);
const IN_ADDITION_TO_ITS_OTHER_TYPES_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["in", "addition", "to", "its", "other", "types"]);
const IN_ADDITION_TO_ITS_OTHER_CREATURE_TYPES_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["in", "addition", "to", "its", "other", "creature", "types"]);
const COPY_POWER_TOUGHNESS_FROM_SELF_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "and",
                "its",
                "power",
                "and",
                "toughness",
                "are",
                "equal",
                "to",
                "this",
            ],
            &[
                "its",
                "power",
                "and",
                "toughness",
                "are",
                "equal",
                "to",
                "this",
            ],
        ]
);
const AND_IT_HAS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["and", "it", "has"], &["and", "has"]]);
const YOU_TARGET_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you"]);
const OPPONENT_TARGET_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["an", "opponent"], &["opponent"]]);
const PLAYER_TARGET_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["a", "player"], &["player"]]);
const YOU_HAVE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "have"]);
const YOUR_LIFE_TOTAL_IS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["your", "life", "total", "is"]);
const LIFE_TOTAL_LESS_THAN_STARTING_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "your", "life", "total", "is", "less", "than", "your", "starting", "life", "total",
        ]
);
const YOU_ATTACKED_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "attacked", "this", "turn"],
            &["youve", "attacked", "this", "turn"]
        ]
);
const CREATURE_DIED_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "creature", "died", "this", "turn"],
            &["creature", "died", "this", "turn"],
        ]
);
const YOU_GAINED_LIFE_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "gained", "life", "this", "turn"],
            &["youve", "gained", "life", "this", "turn"],
        ]
);
const YOU_GAINED_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["youve", "gained"], &["you", "gained"]]);
const LIFE_THIS_TURN_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["life", "this", "turn"]);
const ITS_NIGHT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["its", "night"], &["it", "is", "night"]]);
const THIS_SPELL_BARGAINED_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["its", "bargained"],
            &["it's", "bargained"],
            &["it", "is", "bargained"],
            &["this", "spell", "is", "bargained"],
            &["this", "spell", "was", "bargained"],
        ]
);
const YOU_SACRIFICED_ARTIFACT_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["youve", "sacrificed", "an", "artifact", "this", "turn"],
            &["you", "sacrificed", "an", "artifact", "this", "turn"],
        ]
);
const YOU_COMMITTED_CRIME_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["youve", "committed", "a", "crime", "this", "turn"],
            &["you", "committed", "a", "crime", "this", "turn"],
        ]
);
const CREATURE_LEFT_BATTLEFIELD_UNDER_YOUR_CONTROL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "a",
            "creature",
            "left",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ]
);
const YOU_CAST_ANOTHER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["youve", "cast", "another"],
            &["you've", "cast", "another"],
            &["you", "cast", "another"],
            &["you", "ve", "cast", "another"],
        ]
);
const YOU_CAST_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["youve", "cast"],
            &["you've", "cast"],
            &["you", "cast"],
            &["you", "ve", "cast"],
        ]
);
const THIS_TURN_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["this", "turn"]);
const INSTANT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instant"]);
const SORCERY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["sorcery"]);
const NOT_STARTING_PLAYER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "werent", "the", "starting", "player"]);
const CREATURE_IS_ATTACKING_YOU_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["a", "creature", "is", "attacking", "you"]);
const CREATURE_CARD_PUT_INTO_GRAVEYARD_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "a",
            "creature",
            "card",
            "was",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
            "this",
            "turn",
        ]
);
const THERE_ARE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["there", "are"]);
const CARD_TYPES_GRAVEYARD_COUNT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["card", "types", "graveyard"]);
const YOU_HAVE_IN_YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "have"]; suffix & ["in", "your", "graveyard"]);
const OPPONENT_HAS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["an", "opponent", "has"], &["opponent", "has"]]);
const POISON_COUNTERS_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["poison", "counters"], &["poison", "counter"],]);
const CARDS_IN_OPPONENT_GRAVEYARD_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["cards", "in", "their", "graveyard"],
            &["cards", "in", "his", "graveyard"],
            &["cards", "in", "her", "graveyard"],
            &["card", "in", "their", "graveyard"],
        ]
);
const THERE_ARE_NO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["there", "are", "no"]);
const IN_YOUR_HAND_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["in", "your", "hand"]);
const THERE_IS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["there", "is"]);
const IN_YOUR_GRAVEYARD_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["in", "your", "graveyard"]);
const OPPONENT_HAS_NO_CARDS_IN_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["an", "opponent", "has", "no", "cards", "in", "hand"],
            &["opponent", "has", "no", "cards", "in", "hand"],
        ]
);
const OPPONENT_CONTROLS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["an", "opponent", "controls"]);
const LANDS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["lands"], &["land"]]);
const MORE_CREATURES_THAN_YOU_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["more", "creatures", "than", "you"],
            &["more", "creature", "than", "you"],
        ]
);
const TOTAL_CREATURE_CARDS_IN_ALL_GRAVEYARDS_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["creature", "cards", "total", "in", "all", "graveyards",]);
const OPPONENT_CAST_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["an", "opponent", "cast"], &["opponent", "cast"]]);
const SPELLS_THIS_TURN_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["spells", "this", "turn"], &["spell", "this", "turn"],]);
const OPPONENT_HAS_DRAWN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["an", "opponent", "has", "drawn"],
            &["opponent", "has", "drawn"],
        ]
);
const CARDS_THIS_TURN_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["cards", "this", "turn"], &["card", "this", "turn"],]);
const YOU_WERE_DEALT_DAMAGE_BY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["youve", "been", "dealt", "damage", "by"],
            &["you", "have", "been", "dealt", "damage", "by"],
        ]
);
const DAMAGE_BY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["damage", "by"]);
const WOULD_DIE_EXILE_INSTEAD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["this", "turn", "would", "die", "exile", "it", "instead"]);
const NONTOKEN_OPPONENT_WOULD_DIE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "if", "a", "nontoken", "creature", "an", "opponent", "controls", "would", "die",
            ],
            &[
                "if", "nontoken", "creature", "opponent", "controls", "would", "die",
            ],
        ]
);
const NONTOKEN_ANY_WOULD_DIE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["if", "a", "nontoken", "creature", "would", "die"],
            &["if", "nontoken", "creature", "would", "die"],
        ]
);
const SIMPLE_WOULD_DIE_EXILE_PLAYER_FILTERS: &[(&[&str], PlayerFilter)] = &[
    (
        &[
            "if", "a", "creature", "an", "opponent", "controls", "would", "die", "exile", "it",
            "instead",
        ],
        PlayerFilter::Opponent,
    ),
    (
        &[
            "if", "creature", "an", "opponent", "controls", "would", "die", "exile", "it",
            "instead",
        ],
        PlayerFilter::Opponent,
    ),
    (
        &[
            "if", "a", "creature", "you", "control", "would", "die", "exile", "it", "instead",
        ],
        PlayerFilter::You,
    ),
    (
        &[
            "if", "creature", "you", "control", "would", "die", "exile", "it", "instead",
        ],
        PlayerFilter::You,
    ),
    (
        &[
            "if", "a", "creature", "would", "die", "exile", "it", "instead",
        ],
        PlayerFilter::Any,
    ),
    (
        &["if", "creature", "would", "die", "exile", "it", "instead"],
        PlayerFilter::Any,
    ),
];
const THIS_DAMAGED_BY_SOURCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "source"],
            &["this"],
        ]
);
const EQUIPPED_CREATURE_DAMAGED_BY_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["equipped", "creature"]);
const ENCHANTED_CREATURE_DAMAGED_BY_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["enchanted", "creature"]);
const CREATURES_THIS_TURN_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["creatures", "this", "turn"],
            &["creature", "this", "turn"],
        ]
);
const YOU_HAVE_NO_OTHER_CREATURE_CARDS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "have", "no", "other", "creature", "cards"]);
const OR_IF_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["or", "if"]]);
const ONLY_OTHER_CREATURE_CARDS_NAMED_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "the", "only", "other", "creature", "cards", "in", "your", "hand", "are", "named",
        ]
);
const NAMED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["named"]);
const TARGETS_BIG_CONTROLLED_CREATURE_STACK_OBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "it", "targets", "a", "spell", "or", "ability", "that", "targets", "a", "creature",
            "you", "control", "with", "power", "7", "or", "greater",
        ]
);
const ASSASSIN_OR_COMMANDER_COMBAT_DAMAGE_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "you",
            "dealt",
            "combat",
            "damage",
            "to",
            "a",
            "player",
            "this",
            "turn",
            "with",
            "an",
            "assassin",
            "or",
            "commander",
        ]
);
const OPPONENT_OR_OPPONENTS_TARGET_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["opponent"], &["opponents"]]);
const PLAYER_OR_PLAYERS_TARGET_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["player"], &["players"]]);
const LESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["less"]);
const MORE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["more"]);
const SPELL_OR_SPELLS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["spell"], &["spells"]]);
const TOKEN_OR_TOKENS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["token"], &["tokens"]]);
const GENERIC_DOUBLE_COUNTERS_UNDER_YOUR_CONTROL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "if",
            "an",
            "effect",
            "would",
            "put",
            "one",
            "or",
            "more",
            "counters",
            "on",
            "a",
            "permanent",
            "you",
            "control",
            "it",
            "puts",
            "twice",
            "that",
            "many",
            "of",
            "those",
            "counters",
            "on",
            "that",
            "permanent",
            "instead",
        ]
);
const PLUS_ONE_COUNTERS_WOULD_BE_PUT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "if", "one", "or", "more", "+1/+1", "counters", "would", "be", "put", "on",
        ]
);
const TWICE_THAT_MANY_PLUS_ONE_COUNTERS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["twice", "that", "many", "are", "put", "on", "it", "instead"],
            &[
                "twice", "that", "many", "are", "put", "on", "that", "creature", "instead",
            ],
            &[
                "twice", "that", "many", "+1/+1", "counters", "are", "put", "on", "it", "instead",
            ],
            &[
                "twice", "that", "many", "+1/+1", "counters", "are", "put", "on", "that",
                "creature", "instead",
            ],
        ]
);
const DOUBLE_TOKEN_CREATION_UNDER_YOUR_CONTROL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "if", "an", "effect", "would", "create", "one", "or", "more", "tokens", "under",
                "your", "control", "it", "creates", "twice", "that", "many", "of", "those",
                "tokens", "instead",
            ],
            &[
                "if", "one", "or", "more", "tokens", "would", "be", "created", "under", "your",
                "control", "twice", "that", "many", "of", "those", "tokens", "are", "created",
                "instead",
            ],
        ]
);
const YOU_CREATE_ONE_OR_MORE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "you", "would", "create", "one", "or", "more"]);
const ADDITIONAL_TOKEN_REPLACEMENT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "instead",
            "create",
            "those",
            "tokens",
            "plus",
            "an",
            "additional"
        ]
);
const TREASURE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["treasure"]);
const YOU_MAY_CHOOSE_NOT_TO_UNTAP_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "may", "choose", "not", "to", "untap"]);
const DURING_YOUR_UNTAP_STEP_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["during", "your", "untap", "step"]);
const MAY_CHOOSE_NOT_UNTAP_SOURCE_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this"],
            &["it"],
            &["this", "artifact"],
            &["this", "creature"],
            &["this", "land"],
            &["this", "permanent"],
            &["this", "card"],
        ]
);
const DURING_TURNS_OTHER_THAN_YOURS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["during", "turns", "other", "than", "yours"]);
const DURING_YOUR_TURN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["during", "your", "turn"]);
const FIRST_SPELL_EACH_TURN_COST_MODIFIER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["first", "each", "turn"]; contains_any_words & [&["cost", "costs"]]);
const YOU_CAST_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["you", "cast"]]);
const FROM_YOUR_GRAVEYARD_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["from", "your", "graveyard"]]);
const OPPONENT_WORD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["opponent", "opponents"]]);
const CAST_OR_CASTS_WORD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["cast", "casts"]]);
const TARGET_OR_TARGETS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["target"], &["targets"]]);
const THAT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that"]);
const THAT_MUCH_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["that", "much"]);
const LEGEND_RULE_APPLY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["legend", "rule", "apply"]);
const DOESNT_WORD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["doesnt", "doesn"]]);
const DOES_NOT_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["does", "not"]]);
const YOU_START_THE_GAME_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "start", "the", "game"]);
const ADDITIONAL_LIFE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["additional", "life"]);
const BUYBACK_COSTS_COST_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["buyback", "costs", "cost"]);
const THIS_SPELL_COSTS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "spell", "costs"]);
const CAST_A_OR_ONE_SPELL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["cast", "a", "spell"], &["cast", "one", "spell"]]);
const AND_YOU_MAY_SPEND_MANA_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["and", "you", "may", "spend", "mana"]);
const THAT_HAVE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["that", "have"]);
const THEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["then"]);
const WOULD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["would"]);
const GRAVEYARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["graveyard"]);
const DEALT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["dealt"]);
const EQUIP_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["equip"]);
const RATHER_THAN_PAY_CYCLING_COSTS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["rather", "than", "pay", "cycling", "costs"]);
const TARGET_BEYOND_MORE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["more", "target", "beyond"]);
const CAST_WORD_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["cast"]);
const AND_ABILITIES_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["and", "abilities"]);
const YOU_MAY_PAY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "may", "pay"]);
const ABILITIES_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["abilities"]);
const ACTIVATE_OR_ACTIVATES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["activate"], &["activates"]]);
const YOUR_OPPONENTS_ACTIVATOR_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["your", "opponents"], &["opponents"]]);
const TO_ACTIVATE_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["to", "activate"]]);
const UNLESS_THEYRE_MANA_ABILITIES_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["unless", "theyre", "mana", "abilities"],
            &["unless", "they're", "mana", "abilities"],
        ]]
);
const THAT_TARGET_OR_TARGETS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["that", "target"], &["that", "targets"]]);
const IF_IT_TARGET_OR_TARGETS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["if", "it", "target"], &["if", "it", "targets"]]);
const YOU_PAY_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["you", "pay"]]);
const OPPONENTS_PAY_PHRASE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["your", "opponents", "pay"],
            &["opponents", "pay"],
            &["opponent", "pays"],
        ]]
);
const BY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["by"]);
const FOR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["for"]);
const CAST_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["cast"]);
const WHERE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["where"]);
const THAN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["than"]);
const EACH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["each"]);
const DRAWN_THIS_TURN_CARD_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_any_words & [&["card", "cards"]]; contains_words & ["drawn", "this", "turn"]);
const YOU_WORD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["you", "your", "youve", "you've"]]);
const CREATURES_DIED_THIS_TURN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["creature", "that", "died", "this", "turn"],
            &["creatures", "that", "died", "this", "turn"],
        ]
);
const KICK_COUNT_DYNAMIC_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["time", "this", "was", "kicked"],
            &["time", "this", "spell", "was", "kicked"],
        ]
);
const LIFE_OPPONENTS_LOST_THIS_TURN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "1",
                "life",
                "your",
                "opponents",
                "have",
                "lost",
                "this",
                "turn"
            ],
            &["life", "your", "opponents", "have", "lost", "this", "turn"],
            &["1", "life", "opponents", "have", "lost", "this", "turn"],
            &["life", "opponents", "have", "lost", "this", "turn"],
        ]
);
const CREATURES_DIED_UNDER_YOUR_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["creature", "that", "died", "under", "your", "control"],
            &["creatures", "that", "died", "under", "your", "control"],
        ]
);
const THIS_TURN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["this", "turn"]);
const SPELL_CAST_THIS_TURN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["spell", "spells"]]; contains_words & ["this", "turn"]);
const CAST_OR_CASTS_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["cast", "casts"]]);
const CARD_TYPE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["card", "type"], &["card", "types"]]]);
const CARD_TYPES_IN_GRAVEYARD_DYNAMIC_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["card", "type"], &["card", "types"]]];
    contains_words & ["graveyard"]
);
const YOUR_GRAVEYARD_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["your", "graveyard"]]);
const OPPONENT_GRAVEYARD_PHRASE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["opponents", "graveyard"], &["opponent", "graveyard"]]]
);
const CAST_EXILE_COUNTER_CARDS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "you", "may", "cast", "spells", "from", "among", "cards", "in", "exile"
        ]
);
const PLAY_LANDS_CAST_NONCREATURE_EXILED_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "you",
            "may",
            "play",
            "lands",
            "and",
            "cast",
            "noncreature",
            "spells",
            "from",
            "among",
            "cards",
            "you",
            "exiled",
        ]
);
const ON_THEM_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["on", "them"]);
const OPPONENT_OWNED_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["your", "opponents", "own"],
            &["your", "opponent", "owns"],
            &["opponents", "own"],
            &["opponent", "owns"],
        ]
);
const SPEND_SNOW_MANA_AS_ANY_COLOR_FOR_THOSE_SPELLS_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "and", "you", "may", "spend", "mana", "from", "snow", "sources", "as", "though", "it",
            "were", "mana", "of", "any", "color", "to", "cast", "those", "spells",
        ]
);
const SPEND_MANA_AS_ANY_COLOR_FOR_THOSE_SPELLS_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of",
            "any", "color", "to", "cast", "those", "spells",
        ]
);
const SPEND_MANA_ANY_TYPE_CAST_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "you", "can", "spend", "mana", "of", "any", "type", "to", "cast"
            ],
            &[
                "you", "may", "spend", "mana", "of", "any", "type", "to", "cast"
            ],
        ]
);
const PLAYERS_MAY_SPEND_MANA_ANY_COLOR_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "players", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
            "color",
        ]
);
const YOU_MAY_SPEND_MANA_ANY_COLOR_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
            "color",
        ]
);
const PAY_ACTIVATION_COSTS_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["to", "pay", "the", "activation", "costs", "of"]);
const ACTIVATE_ABILITIES_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["to", "activate", "abilities", "of"]);
const ABILITY_OR_ABILITIES_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["ability", "abilities"]]);
const OTHER_THAN_FIRST_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["other", "than", "the", "first"]]);
const OTHER_WORD_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["other"]);
const YOU_DREW_CARDS_DYNAMIC_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["card", "youve", "drawn", "this", "turn"],
            &["cards", "youve", "drawn", "this", "turn"],
            &["card", "you", "have", "drawn", "this", "turn"],
            &["cards", "you", "have", "drawn", "this", "turn"],
            &["card", "you", "ve", "drawn", "this", "turn"],
            &["cards", "you", "ve", "drawn", "this", "turn"],
        ]
);
const SIMPLE_YOU_CAST_SPELLS_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["spell", "youve", "cast", "this", "turn"],
            &["spells", "youve", "cast", "this", "turn"],
            &["spell", "you", "cast", "this", "turn"],
            &["spells", "you", "cast", "this", "turn"],
            &["spell", "your", "cast", "this", "turn"],
            &["spells", "your", "cast", "this", "turn"],
        ]
);
const IF_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["if"]);
const TRIGGERS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["triggers"]);
const CAUSES_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["causes"]);
const TWICE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["twice"]);
const LOSES_ALL_OTHER_CREATURE_TYPES_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "loses", "all", "other", "creature", "types"],
            &["this", "loses", "all", "other", "creature", "types"],
        ]
);
const WHILE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["while"]);
const TO_TRIGGER_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["to", "trigger"]);
const COLORS_OF_MANA_CAST_THIS_SPELL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "color", "of", "mana", "spent", "to", "cast", "this", "spell"
            ],
            &[
                "colors", "of", "mana", "spent", "to", "cast", "this", "spell"
            ],
            &["color", "of", "mana", "used", "to", "cast", "this", "spell"],
            &[
                "colors", "of", "mana", "used", "to", "cast", "this", "spell"
            ],
        ]
);
const CREATURES_IN_YOUR_PARTY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["creature", "in", "your", "party"],
            &["creatures", "in", "your", "party"]
        ]
);
const BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["basic", "land", "type", "among"],
            &["basic", "land", "types", "among"],
        ]
);
const COLORS_AMONG_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["color", "among"], &["colors", "among"]]);
const CREATURE_TYPES_AMONG_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["creature", "type", "among"],
            &["creature", "types", "among"]
        ]
);
const CARD_TYPES_AMONG_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["card", "type", "among"], &["card", "types", "among"]]);
const CARD_TYPES_AMONG_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["card", "type", "among"], &["card", "types", "among"]]]
);
const COUNTERS_REMOVED_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases & [&["this", "way"]];
    contains_words & ["removed"];
    contains_any_words & [&["counter", "counters"]]
);
const DESTROYED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["this", "way"]]; contains_words & ["destroyed"]);
const DISCARDED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["this", "way"]]; contains_words & ["discarded"]);
const REVEALED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["this", "way"]]; contains_words & ["revealed"]);
const THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(contains_phrases & [&["this", "way"]]);
const X_CANT_EXCEED_PLAYER_COUNT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "x", "cant", "be", "greater", "than", "the", "number", "of", "players", "in", "the",
            "game",
        ]
);
const EXHAUST_AS_THOUGH_UNACTIVATED_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "during",
            "your",
            "turn",
            "as",
            "long",
            "as",
            "you",
            "havent",
            "activated",
            "an",
            "exhaust",
            "ability",
            "this",
            "turn",
            "you",
            "may",
            "activate",
            "exhaust",
            "abilities",
            "as",
            "though",
            "they",
            "havent",
            "been",
            "activated",
        ]
);
const CANT_ATTACK_UNLESS_CAST_CREATURE_SPELL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "this", "creature", "cant", "attack", "unless", "youve", "cast", "a", "creature",
                "spell", "this", "turn",
            ],
            &[
                "this", "cant", "attack", "unless", "youve", "cast", "a", "creature", "spell",
                "this", "turn",
            ],
        ]
);
const CANT_ATTACK_UNLESS_CAST_NONCREATURE_SPELL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "this",
                "creature",
                "cant",
                "attack",
                "unless",
                "youve",
                "cast",
                "a",
                "noncreature",
                "spell",
                "this",
                "turn",
            ],
            &[
                "this",
                "cant",
                "attack",
                "unless",
                "youve",
                "cast",
                "a",
                "noncreature",
                "spell",
                "this",
                "turn",
            ],
        ]
);
const GET_OR_GETS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["get"], &["gets"]]);
const GET_GETS_HAVE_HAS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["get"], &["gets"], &["have"], &["has"]]);
const PLAYER_COUNTER_RESOURCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["energy"], &["poison"], &["ticket"], &["e"], &["tk"]]);
const ANOTHER_OR_CARDINAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["another"]);
const CANT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["cant"]);
const DONT_OR_DOESNT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["dont"], &["doesnt"]]);
const COST_OR_COSTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["cost"], &["costs"]]);
const TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const COMMA_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & [","]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const UNTAP_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["untap"]);
const WITH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["with"]);
const NUMBER_OF_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["number", "of"]);
const ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["a"], &["an"]]);
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const ITS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["its"]);
const IT_APOSTROPHE_S_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it's"], &["it’s"]]);
const COPY_NAME_BOUNDARY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it's"], &["it’s"], &["and"]]);
const EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["equal", "to"]);
const SOURCES_WITH_CHOSEN_NAME_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["sources", "with", "chosen", "name"],
            &["sources", "with", "the", "chosen", "name"],
        ]
);
const COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const ENCHANTED_OR_EQUIPPED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enchanted"], &["equipped"]]);
const SOURCE_COUNTER_LEADING_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["one"], &["another"]]);
const ON_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["on"]);
const SOURCE_COUNTER_REFERENCE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["it"],
            &["this"],
            &["that", "object"],
            &["that", "permanent"],
        ]
);
const EXILE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["exile"]);
const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const TYPE_OR_TYPES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["type"], &["types"]]);
const IN_ADDITION_TO_OTHER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["in", "addition", "to", "its", "other"],
            &["in", "addition", "to", "their", "other"],
        ]
);
const IN_ADDITION_TO_THEIR_OTHER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["in", "addition", "to", "their", "other"]);
const CHOSEN_TYPE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["the", "chosen", "type"], &["chosen", "type"]]);
const TYPE_ADDITION_IGNORED_DESCRIPTOR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["and"], &["or"], &["and/or"]]);
const ALL_CARDS_SPELLS_PERMANENTS_COLORLESS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["colorless", "cards", "spells", "permanents"]);
const ARE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["are"]);
const BE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["is"], &["are"]]);
const AND_ARE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["and", "are"]);
const CREATURE_TYPE_SCOPE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["creature", "type"], &["creature", "types"]]);
const DISCARD_COST_IGNORED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["or"], &["a"], &["an"]]);
const TAPPED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["tapped"]);
const NOT_STARTING_PLAYER_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["youre", "not", "playing", "first"],
            &["you're", "not", "playing", "first"],
            &["you", "re", "not", "playing", "first"],
            &["you", "are", "not", "playing", "first"],
            &["youre", "not", "the", "starting", "player"],
            &["you're", "not", "the", "starting", "player"],
            &["you", "re", "not", "the", "starting", "player"],
            &["you", "are", "not", "the", "starting", "player"],
            &["youre", "not", "starting", "the", "game"],
            &["you're", "not", "starting", "the", "game"],
            &["you", "re", "not", "starting", "the", "game"],
            &["you", "are", "not", "starting", "the", "game"],
        ]]
);
const MAX_HAND_SIZE_AS_LONG_AS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["as", "long", "as"]);
const MAX_HAND_SIZE_YOU_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["your"], &["you"]]);
const MAX_HAND_SIZE_EACH_OPPONENT_POSSESSIVE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["each", "opponent's"], &["each", "opponent", "s"]]);
const MAX_HAND_SIZE_EACH_OPPONENT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["each", "opponent"],
            &["each", "opponents"],
            &["each", "opponent", "s"],
        ]
);
const MAX_HAND_SIZE_OPPONENT_POSSESSIVE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["opponent's"], &["opponent", "s"]]);
const MAX_HAND_SIZE_OPPONENT_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["opponent"], &["opponents"], &["opponent", "s"]]);
const MAX_HAND_SIZE_EACH_PLAYER_POSSESSIVE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["each", "player's"], &["each", "player", "s"]]);
const MAX_HAND_SIZE_EACH_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["each", "player"],
            &["each", "players"],
            &["each", "player", "s"],
        ]
);
const MAX_HAND_SIZE_PLAYER_POSSESSIVE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["player's"], &["player", "s"]]);
const MAX_HAND_SIZE_PLAYER_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["player"], &["players"], &["player", "s"]]);
const MAX_HAND_SIZE_IS_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["maximum", "hand", "size", "is"]);
const MAX_HAND_SIZE_REDUCED_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["maximum", "hand", "size", "is", "reduced"]);
const MAX_HAND_SIZE_SEVEN_MINUS_CARD_TYPES_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "equal", "to", "seven", "minus", "the", "number", "of", "those", "card", "types",
            ],
            &[
                "equal", "to", "seven", "minus", "the", "number", "of", "those", "card", "type",
            ],
        ]
);

fn keyword_find_prefix_shape_start(words: &[&str], shape: &ClauseShape<'static>) -> Option<usize> {
    (0..words.len()).find(|&idx| shape.matches_words(&words[idx..]))
}

fn simple_would_die_exile_player_filter(words: &[&str]) -> Option<PlayerFilter> {
    SIMPLE_WOULD_DIE_EXILE_PLAYER_FILTERS
        .iter()
        .find_map(|(phrase, player)| (*phrase == words).then(|| player.clone()))
}

fn max_hand_size_subject_prefix(words: &[&str]) -> Option<(PlayerFilter, usize)> {
    if MAX_HAND_SIZE_YOU_SUBJECT_PATTERN.matches_words(words) {
        Some((PlayerFilter::You, 1))
    } else if let Some(used) =
        MAX_HAND_SIZE_EACH_OPPONENT_POSSESSIVE_PATTERN.matched_prefix_len(words)
    {
        Some((PlayerFilter::Opponent, used))
    } else if let Some(used) = MAX_HAND_SIZE_EACH_OPPONENT_PATTERN.matched_prefix_len(words) {
        Some((PlayerFilter::Opponent, used))
    } else if let Some(used) = MAX_HAND_SIZE_OPPONENT_POSSESSIVE_PATTERN.matched_prefix_len(words) {
        Some((PlayerFilter::Opponent, used))
    } else if let Some(used) = MAX_HAND_SIZE_OPPONENT_PATTERN.matched_prefix_len(words) {
        Some((PlayerFilter::Opponent, used))
    } else if let Some(used) =
        MAX_HAND_SIZE_EACH_PLAYER_POSSESSIVE_PATTERN.matched_prefix_len(words)
    {
        Some((PlayerFilter::Any, used))
    } else if let Some(used) = MAX_HAND_SIZE_EACH_PLAYER_PATTERN.matched_prefix_len(words) {
        Some((PlayerFilter::Any, used))
    } else if let Some(used) = MAX_HAND_SIZE_PLAYER_POSSESSIVE_PATTERN.matched_prefix_len(words) {
        Some((PlayerFilter::Any, used))
    } else if let Some(used) = MAX_HAND_SIZE_PLAYER_PATTERN.matched_prefix_len(words) {
        Some((PlayerFilter::Any, used))
    } else {
        None
    }
}

fn chosen_name_source_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter.name = Some("{chosen name}".to_string());
    filter
}

fn card_type_word(word: &str) -> Option<CardType> {
    parse_card_type(str_strip_suffix(word, "s").unwrap_or(word))
}

fn two_card_type_union_filter_from_words(words: &[&str]) -> Option<ObjectFilter> {
    let [left, and_word, right] = words else {
        return None;
    };
    if !AND_WORD_PATTERN.matches_word(and_word) {
        return None;
    }
    let (Some(left_type), Some(right_type)) = (card_type_word(left), card_type_word(right)) else {
        return None;
    };

    let mut left_filter = ObjectFilter::default();
    left_filter.zone = Some(Zone::Battlefield);
    left_filter.card_types = vec![left_type];

    let mut right_filter = ObjectFilter::default();
    right_filter.zone = Some(Zone::Battlefield);
    right_filter.card_types = vec![right_type];

    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = vec![left_filter, right_filter];
    Some(disjunction)
}

fn activated_ability_subject_special_filter(words: &[&str]) -> Option<ObjectFilter> {
    if SOURCES_WITH_CHOSEN_NAME_PATTERN.matches_words(words) {
        return Some(chosen_name_source_filter());
    }
    two_card_type_union_filter_from_words(words)
}

fn parse_life_total_or_less_spell_cost_condition(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;

    if words.len() >= 4 && YOU_HAVE_PREFIX_PATTERN.matches_words(words) {
        let tail_words = words.get(2..)?;
        if tail_words.last().copied() == Some("life") {
            let quantity_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(
                &tail_words[..tail_words.len() - 1],
            );
            if let Some((amount, used)) = parse_less_than_or_equal_quantity_prefix(
                &quantity_tokens,
                false,
                false,
                "life total cost condition",
            )
            .ok()
            .flatten()
                && used == tail_words.len() - 1
            {
                return Some(ThisSpellCostCondition::YouLifeTotalOrLess(amount as i32));
            }
        }
    }

    if words.len() >= 6 && YOUR_LIFE_TOTAL_IS_PREFIX_PATTERN.matches_words(words) {
        let quantity_tokens = tokens.get(4..).unwrap_or_default();
        if let Some((amount, used)) = parse_less_than_or_equal_quantity_prefix(
            quantity_tokens,
            false,
            false,
            "life total cost condition",
        )
        .ok()
        .flatten()
            && used == quantity_tokens.len()
        {
            return Some(ThisSpellCostCondition::YouLifeTotalOrLess(amount as i32));
        }
    }

    None
}

fn mentioned_instant_sorcery_card_types(words: &[&str]) -> Vec<CardType> {
    let mut types = Vec::new();
    if words
        .iter()
        .any(|word| INSTANT_WORD_PATTERN.matches_word(word))
    {
        types.push(CardType::Instant);
    }
    if words
        .iter()
        .any(|word| SORCERY_WORD_PATTERN.matches_word(word))
    {
        types.push(CardType::Sorcery);
    }
    types
}

fn count_start_for_optional_an_opponent_prefix(words: &[&str], long_len: usize) -> Option<usize> {
    if words
        .first()
        .is_some_and(|word| ARTICLE_WORD_PATTERN.matches_word(word))
    {
        Some(long_len)
    } else {
        Some(long_len.saturating_sub(1))
    }
}

fn parse_static_at_least_quantity_at(
    tokens: &[OwnedLexToken],
    start: usize,
) -> Option<(u32, usize)> {
    let (comparison, used) = parse_quantity_comparison_prefix(
        tokens.get(start..).unwrap_or_default(),
        false,
        false,
        "spell cost condition",
    )
    .ok()?;
    let count = comparison_to_strict_at_least_threshold(&comparison)?;
    Some((count, start + used))
}

fn only_creature_cards_in_hand_named(words: &[&str]) -> Option<String> {
    let matches_named_exception = (YOU_HAVE_NO_OTHER_CREATURE_CARDS_PREFIX_PATTERN
        .matches_words(words)
        && OR_IF_PHRASE_PATTERN.matches_words(words))
        || ONLY_OTHER_CREATURE_CARDS_NAMED_PREFIX_PATTERN.matches_words(words);
    if !matches_named_exception {
        return None;
    }

    let named_idx = find_index(words, |word| NAMED_WORD_PATTERN.matches_word(word))?;
    let name_words = words.get(named_idx + 1..)?;
    let name = name_words.join(" ");
    (!name.is_empty()).then_some(name)
}

fn dynamic_cards_drawn_this_turn_player(words: &[&str]) -> Option<PlayerFilter> {
    if !DRAWN_THIS_TURN_CARD_MARKER_PATTERN.matches_words(words) {
        return None;
    }
    if YOU_WORD_MARKER_PATTERN.matches_words(words) {
        Some(PlayerFilter::You)
    } else if OPPONENT_WORD_MARKER_PATTERN.matches_words(words) {
        Some(PlayerFilter::Opponent)
    } else {
        None
    }
}

fn dynamic_spell_cast_this_turn_player(words: &[&str]) -> Option<PlayerFilter> {
    if !SPELL_CAST_THIS_TURN_MARKER_PATTERN.matches_words(words)
        || !CAST_OR_CASTS_MARKER_PATTERN.matches_words(words)
    {
        return None;
    }
    if YOU_WORD_MARKER_PATTERN.matches_words(words) {
        Some(PlayerFilter::You)
    } else if OPPONENT_WORD_MARKER_PATTERN.matches_words(words) {
        Some(PlayerFilter::Opponent)
    } else {
        Some(PlayerFilter::Any)
    }
}

pub(crate) fn parse_can_be_attached_only_to_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some((phrase_idx, phrase_end)) =
        find_token_word_sequence_span(tokens, &["can", "be", "attached", "only", "to"])
    else {
        return Ok(None);
    };
    if phrase_idx == 0 {
        return Ok(None);
    }
    let target_tokens = trim_commas(&tokens[phrase_end..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "attachment restriction missing target filter (clause: '{}')",
            render_token_slice(tokens)
        )));
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    Ok(Some(StaticAbilityAst::AttachmentRestriction {
        filter: filter.into(),
        display: render_token_slice(tokens),
    }))
}

const AS_ENTERS_STANDARD_SUBJECTS: &[(&str, &str)] = &[
    ("land", "this land"),
    ("creature", "this creature"),
    ("artifact", "this artifact"),
    ("enchantment", "this enchantment"),
    ("permanent", "this permanent"),
];

const AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA: &[(&str, &str)] = &[
    ("land", "this land"),
    ("creature", "this creature"),
    ("artifact", "this artifact"),
    ("enchantment", "this enchantment"),
    ("aura", "this aura"),
    ("permanent", "this permanent"),
];

fn keyword_static_clause_text(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens).trim().to_string()
}

fn keyword_static_marker(tokens: &[OwnedLexToken]) -> StaticAbility {
    let mut text = keyword_static_clause_text(tokens);
    if supported_keyword_marker_tokens(tokens, &text) {
        let words = parser_token_word_refs(tokens);
        if POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words) && !text.ends_with('.')
        {
            text.push('.');
        }
        return StaticAbility::keyword_marker(text);
    }
    StaticAbility::keyword_fallback_text(text)
}

fn supported_keyword_marker_tokens(tokens: &[OwnedLexToken], text: &str) -> bool {
    let text = text.trim_start().to_ascii_lowercase();
    let words = parser_token_word_refs(tokens);
    is_core_keyword_marker_text(&text)
        || TOUGHNESS_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)
        || POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)
        || LOYALTY_COUNTER_INSTEAD_OF_CREW_COST_MARKER_PATTERN.matches_words(&words)
}

fn trim_outer_quotes(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && tokens[start].is_quote() {
        start += 1;
    }
    while end > start && tokens[end - 1].is_quote() {
        end -= 1;
    }
    &tokens[start..end]
}

fn looks_like_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        tokens.first().map(|token| token.parser_text()),
        Some("when" | "whenever" | "at")
    )
}

fn looks_like_trigger_intro_after_label(tokens: &[OwnedLexToken]) -> bool {
    split_em_dash_label_prefix(tokens)
        .is_some_and(|(_, body_tokens)| looks_like_trigger_intro_tokens(body_tokens))
}

#[derive(Clone, Copy)]
enum StaticAbilityLineRuleAst {
    Single(fn(&[OwnedLexToken]) -> Result<Option<StaticAbilityAst>, CardTextError>),
    SingleInfallible(fn(&[OwnedLexToken]) -> Option<StaticAbilityAst>),
    Multi(fn(&[OwnedLexToken]) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError>),
}

#[derive(Clone, Copy)]
struct StaticAbilityLineRuleDef {
    id: &'static str,
    rule: StaticAbilityLineRuleAst,
}

type StaticAbilityLineHeadHint = LexRuleHeadHint;

fn run_static_ability_ast_line_rule(
    rule: StaticAbilityLineRuleAst,
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    match rule {
        StaticAbilityLineRuleAst::Single(parse) => Ok(parse(tokens)?.map(|ability| vec![ability])),
        StaticAbilityLineRuleAst::SingleInfallible(parse) => {
            Ok(parse(tokens).map(|ability| vec![ability]))
        }
        StaticAbilityLineRuleAst::Multi(parse) => parse(tokens),
    }
}

fn try_static_ability_ast_line_rule_indices(
    rules: &'static [StaticAbilityLineRuleDef],
    tokens: &[OwnedLexToken],
    tried: &mut [bool],
    deferred_error: &mut Option<CardTextError>,
    candidate_indices: &[usize],
) -> Option<Vec<StaticAbilityAst>> {
    for &idx in candidate_indices {
        tried[idx] = true;
        match run_static_ability_ast_line_rule(rules[idx].rule, tokens) {
            Ok(Some(abilities)) => return Some(abilities),
            Ok(None) => {}
            Err(err) => {
                deferred_error.get_or_insert(err);
            }
        }
    }

    None
}

fn static_ability_rule_head_hints(rule_id: &'static str) -> Vec<StaticAbilityLineHeadHint> {
    match rule_id {
        "parse_characteristic_defining_pt_line" => Vec::new(),
        "parse_reduced_maximum_hand_size_line" => vec![
            StaticAbilityLineHeadHint::Single("your"),
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Single("each"),
            StaticAbilityLineHeadHint::Single("opponent"),
            StaticAbilityLineHeadHint::Single("opponents"),
            StaticAbilityLineHeadHint::Single("player"),
            StaticAbilityLineHeadHint::Single("players"),
            StaticAbilityLineHeadHint::Single("as"),
        ],
        "parse_can_be_attached_only_to_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "equipment"),
        ],
        "parse_conditional_source_spell_keyword_line" => vec![
            StaticAbilityLineHeadHint::Single("if"),
            StaticAbilityLineHeadHint::Pair("if", "this"),
        ],
        "parse_conditional_all_creatures_able_to_block_line" => vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "long"),
        ],
        "parse_subject_has_keywords_and_cant_be_blocked_line" => vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "long"),
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "creature"),
        ],
        "parse_landwalk_as_though_block_override_line" => vec![
            StaticAbilityLineHeadHint::Single("creatures"),
            StaticAbilityLineHeadHint::Pair("creatures", "with"),
        ],
        "parse_multi_subject_anthem_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "creature"),
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Pair("enchanted", "creature"),
            StaticAbilityLineHeadHint::Single("equipped"),
            StaticAbilityLineHeadHint::Pair("equipped", "creature"),
        ],
        "parse_spell_cost_increase_per_target_beyond_first_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "spell"),
        ],
        "parse_double_damage_from_sources_you_control_of_chosen_type_line" => vec![
            StaticAbilityLineHeadHint::Single("double"),
            StaticAbilityLineHeadHint::Pair("double", "damage"),
        ],
        "parse_source_can_attack_as_though_no_defender_as_long_as_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "can"),
        ],
        "parse_no_maximum_hand_size_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "have"),
        ],
        "parse_untap_during_each_other_players_untap_step_line" => vec![
            StaticAbilityLineHeadHint::Single("untap"),
            StaticAbilityLineHeadHint::Pair("untap", "all"),
        ],
        "parse_you_may_static_grant_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
            StaticAbilityLineHeadHint::Single("during"),
            StaticAbilityLineHeadHint::Pair("during", "each"),
        ],
        "parse_you_may_cast_exile_counter_cards_with_mana_permission_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_surveilled_graveyard_play_life_cost_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_fixed_mana_cost_instead_of_mana_cost_grant_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_enter_as_copy_as_enters_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_you_may_look_top_card_any_time_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_you_may_look_face_down_creatures_you_dont_control_any_time_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_players_play_top_card_libraries_revealed_line" => vec![
            StaticAbilityLineHeadHint::Single("players"),
            StaticAbilityLineHeadHint::Pair("players", "play"),
        ],
        "parse_play_top_card_your_library_revealed_line" => vec![
            StaticAbilityLineHeadHint::Single("play"),
            StaticAbilityLineHeadHint::Pair("play", "with"),
        ],
        "parse_your_opponents_play_with_hands_revealed_line" => vec![
            StaticAbilityLineHeadHint::Single("your"),
            StaticAbilityLineHeadHint::Pair("your", "opponents"),
        ],
        "parse_control_opponents_while_searching_libraries_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "control"),
        ],
        "parse_opponent_search_exile_found_cards_line" => vec![
            StaticAbilityLineHeadHint::Single("while"),
            StaticAbilityLineHeadHint::Pair("while", "an"),
        ],
        "parse_cast_this_card_from_library_while_searching_line" => vec![
            StaticAbilityLineHeadHint::Single("while"),
            StaticAbilityLineHeadHint::Pair("while", "youre"),
        ],
        "parse_additional_land_play_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_play_lands_from_graveyard_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_graveyard_cards_have_retrace_line" => vec![
            StaticAbilityLineHeadHint::Single("instant"),
            StaticAbilityLineHeadHint::Single("instants"),
            StaticAbilityLineHeadHint::Single("sorcery"),
            StaticAbilityLineHeadHint::Single("sorceries"),
            StaticAbilityLineHeadHint::Single("each"),
        ],
        "parse_pregame_choose_color_line" => vec![
            StaticAbilityLineHeadHint::Single("if"),
            StaticAbilityLineHeadHint::Single("choose"),
            StaticAbilityLineHeadHint::Pair("choose", "a"),
        ],
        "parse_pregame_mulligan_redraw_line" => vec![
            StaticAbilityLineHeadHint::Single("any"),
            StaticAbilityLineHeadHint::Pair("any", "time"),
        ],
        "parse_legend_rule_doesnt_apply_line" => vec![
            StaticAbilityLineHeadHint::Single("the"),
            StaticAbilityLineHeadHint::Pair("the", "legend"),
        ],
        _ => match str_strip_prefix(rule_id, "parse_").and_then(|id| id.split('_').next()) {
            Some("ward") => vec![StaticAbilityLineHeadHint::Single("ward")],
            Some("skulk") => vec![StaticAbilityLineHeadHint::Single("skulk")],
            Some("if") => vec![StaticAbilityLineHeadHint::Single("if")],
            Some("choose") => vec![StaticAbilityLineHeadHint::Single("choose")],
            Some("enchanted") => vec![StaticAbilityLineHeadHint::Single("enchanted")],
            Some("enters") => vec![StaticAbilityLineHeadHint::Single("enters")],
            Some("damage") => vec![StaticAbilityLineHeadHint::Single("damage")],
            Some("pay") => vec![StaticAbilityLineHeadHint::Single("pay")],
            Some("copy") => vec![StaticAbilityLineHeadHint::Single("copy")],
            Some("players") => vec![StaticAbilityLineHeadHint::Single("players")],
            Some("shuffle") => vec![StaticAbilityLineHeadHint::Single("shuffle")],
            Some("permanents") => vec![StaticAbilityLineHeadHint::Single("permanents")],
            Some("creatures") => vec![StaticAbilityLineHeadHint::Single("creatures")],
            Some("buyback") => vec![StaticAbilityLineHeadHint::Single("buyback")],
            Some("flashback") => vec![StaticAbilityLineHeadHint::Single("flashback")],
            Some("spells") => vec![StaticAbilityLineHeadHint::Single("spells")],
            Some("foretelling") => vec![StaticAbilityLineHeadHint::Single("foretelling")],
            Some("all") => vec![StaticAbilityLineHeadHint::Single("all")],
            Some("blood") => vec![StaticAbilityLineHeadHint::Single("blood")],
            Some("land") => vec![StaticAbilityLineHeadHint::Single("land")],
            Some("lands") => vec![StaticAbilityLineHeadHint::Single("lands")],
            Some("remove") => vec![StaticAbilityLineHeadHint::Single("remove")],
            Some("attached") => vec![StaticAbilityLineHeadHint::Single("attached")],
            Some("soulbond") => vec![StaticAbilityLineHeadHint::Single("soulbond")],
            Some("may") => vec![StaticAbilityLineHeadHint::Single("may")],
            Some("warp") => vec![StaticAbilityLineHeadHint::Single("warp")],
            Some("melee") => vec![StaticAbilityLineHeadHint::Single("melee")],
            Some("equipped") => vec![StaticAbilityLineHeadHint::Single("equipped")],
            Some("as") => vec![StaticAbilityLineHeadHint::Single("as")],
            Some("prevent") => vec![StaticAbilityLineHeadHint::Single("prevent")],
            Some("reveal") => vec![StaticAbilityLineHeadHint::Single("reveal")],
            Some("activated") => vec![StaticAbilityLineHeadHint::Single("activated")],
            _ => Vec::new(),
        },
    }
}

macro_rules! single_static_ability_ast_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: stringify!($parse),
            rule: StaticAbilityLineRuleAst::Single(|tokens| {
                Ok($parse(tokens)?.map(StaticAbilityAst::from))
            }),
        }
    };
}

macro_rules! single_static_ability_ast_infallible_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: stringify!($parse),
            rule: StaticAbilityLineRuleAst::SingleInfallible(|tokens| {
                $parse(tokens).map(StaticAbilityAst::from)
            }),
        }
    };
}

macro_rules! multi_static_ability_ast_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: stringify!($parse),
            rule: StaticAbilityLineRuleAst::Multi(|tokens| {
                Ok($parse(tokens)?.map(|abilities| {
                    abilities
                        .into_iter()
                        .map(StaticAbilityAst::from)
                        .collect::<Vec<_>>()
                }))
            }),
        }
    };
}

macro_rules! single_static_ability_ast_passthrough_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: stringify!($parse),
            rule: StaticAbilityLineRuleAst::Single($parse),
        }
    };
}

macro_rules! multi_static_ability_ast_passthrough_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: stringify!($parse),
            rule: StaticAbilityLineRuleAst::Multi($parse),
        }
    };
}

fn static_ability_ast_line_rules() -> &'static [StaticAbilityLineRuleDef] {
    &[
        StaticAbilityLineRuleDef {
            id: stringify!(parse_soulbond_shared_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_soulbond_shared_line),
        },
        single_static_ability_ast_rule!(parse_ward_static_ability_line),
        single_static_ability_ast_rule!(parse_skulk_rules_text_line),
        single_static_ability_ast_rule!(
            parse_filter_dont_untap_during_controllers_untap_steps_line
        ),
        single_static_ability_ast_rule!(parse_damage_doubling_mana_value_marker_line),
        single_static_ability_ast_rule!(parse_conditional_source_spell_keyword_line),
        single_static_ability_ast_rule!(parse_pregame_begin_on_battlefield_line),
        single_static_ability_ast_rule!(parse_pregame_mulligan_redraw_line),
        multi_static_ability_ast_rule!(parse_combined_pregame_choose_color_line),
        single_static_ability_ast_rule!(parse_pregame_choose_color_line),
        single_static_ability_ast_rule!(parse_activated_abilities_cost_increase_line),
        single_static_ability_ast_rule!(parse_choose_basic_land_type_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_card_name_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_creature_type_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_named_options_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_player_as_enters_line),
        single_static_ability_ast_rule!(parse_note_life_total_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_color_as_becomes_attached_line),
        single_static_ability_ast_rule!(parse_enchanted_land_is_chosen_type_line),
        single_static_ability_ast_rule!(parse_source_is_chosen_type_in_addition_line),
        single_static_ability_ast_rule!(parse_source_is_chosen_color_line),
        single_static_ability_ast_rule!(parse_double_token_creation_replacement_line),
        single_static_ability_ast_rule!(parse_double_counters_replacement_line),
        single_static_ability_ast_infallible_rule!(parse_static_text_marker_line),
        multi_static_ability_ast_rule!(parse_enters_tapped_with_choose_color_line),
        single_static_ability_ast_rule!(parse_damage_not_removed_cleanup_line),
        single_static_ability_ast_rule!(parse_prevent_damage_to_source_remove_counter_line),
        single_static_ability_ast_passthrough_rule!(
            parse_prevent_damage_to_source_put_counters_line
        ),
        single_static_ability_ast_rule!(parse_prevent_damage_to_you_from_source_filter_line),
        single_static_ability_ast_rule!(parse_replace_damage_with_counters_instead_line),
        single_static_ability_ast_rule!(parse_choose_color_as_enters_line),
        single_static_ability_ast_rule!(parse_damage_redirect_to_source_controller_line),
        single_static_ability_ast_rule!(parse_damage_redirect_to_source_line),
        single_static_ability_ast_rule!(
            parse_no_more_than_creatures_can_attack_or_block_each_combat_line
        ),
        single_static_ability_ast_rule!(parse_characteristic_defining_pt_line),
        single_static_ability_ast_rule!(parse_no_maximum_hand_size_line),
        single_static_ability_ast_rule!(parse_can_be_your_commander_line),
        single_static_ability_ast_rule!(parse_reduced_maximum_hand_size_line),
        single_static_ability_ast_rule!(parse_effect_discard_to_library_replacement_line),
        single_static_ability_ast_rule!(parse_draw_replace_exile_top_face_down_line),
        single_static_ability_ast_rule!(parse_draw_replacement_exile_top_and_play_line),
        single_static_ability_ast_rule!(parse_conditional_draw_replacement_line),
        single_static_ability_ast_rule!(parse_draw_replacement_double_line),
        single_static_ability_ast_rule!(parse_draw_replacement_skip_empty_library_line),
        single_static_ability_ast_rule!(parse_keyword_action_replacement_line),
        single_static_ability_ast_rule!(parse_exile_to_exile_instead_of_graveyard_line),
        single_static_ability_ast_rule!(parse_exile_to_countered_exile_instead_of_graveyard_line),
        single_static_ability_ast_rule!(parse_exile_would_die_instead_line),
        single_static_ability_ast_rule!(parse_discard_or_redirect_replacement_line),
        single_static_ability_ast_rule!(parse_pay_life_or_enter_tapped_line),
        single_static_ability_ast_passthrough_rule!(parse_copy_activated_abilities_line),
        single_static_ability_ast_passthrough_rule!(parse_spend_mana_as_any_color_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_enchanted_has_activated_ability_line),
            rule: StaticAbilityLineRuleAst::Single(parse_enchanted_has_activated_ability_line),
        },
        multi_static_ability_ast_passthrough_rule!(
            parse_has_base_power_toughness_and_granted_keywords_static_line
        ),
        multi_static_ability_ast_passthrough_rule!(
            parse_subject_is_subtype_with_base_pt_and_granted_abilities_line
        ),
        multi_static_ability_ast_passthrough_rule!(
            parse_filter_is_pt_creature_in_addition_and_has_line
        ),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_filter_has_granted_ability_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_filter_has_granted_ability_line),
        },
        StaticAbilityLineRuleDef {
            id: stringify!(parse_equipped_gets_and_has_activated_ability_line),
            rule: StaticAbilityLineRuleAst::Multi(
                parse_equipped_gets_and_has_activated_ability_line,
            ),
        },
        single_static_ability_ast_rule!(parse_shuffle_into_library_from_graveyard_line),
        single_static_ability_ast_rule!(parse_permanents_enter_tapped_line),
        single_static_ability_ast_rule!(
            parse_creatures_entering_dont_cause_abilities_to_trigger_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_trigger_suppression_line_ast),
        single_static_ability_ast_rule!(parse_creatures_assign_combat_damage_using_toughness_line),
        single_static_ability_ast_rule!(
            parse_lethal_damage_to_creatures_you_control_uses_power_line
        ),
        single_static_ability_ast_rule!(parse_players_cant_cycle_line),
        single_static_ability_ast_rule!(parse_starting_life_bonus_line),
        single_static_ability_ast_rule!(parse_buyback_cost_reduction_line),
        single_static_ability_ast_passthrough_rule!(parse_can_be_attached_only_to_line),
        single_static_ability_ast_rule!(parse_spell_cost_increase_per_target_beyond_first_line),
        single_static_ability_ast_rule!(parse_equip_cost_modifier_line),
        single_static_ability_ast_rule!(parse_flashback_cost_modifier_line),
        multi_static_ability_ast_rule!(parse_spell_and_player_activated_ability_cost_modifier_line),
        single_static_ability_ast_rule!(parse_spells_cost_modifier_line),
        single_static_ability_ast_passthrough_rule!(parse_trigger_duplication_line_ast),
        single_static_ability_ast_rule!(
            parse_double_damage_from_sources_you_control_of_chosen_type_line
        ),
        single_static_ability_ast_rule!(parse_double_damage_amount_replacement_line),
        single_static_ability_ast_rule!(parse_minimum_damage_amount_replacement_line),
        single_static_ability_ast_rule!(parse_damage_amount_replacement_line),
        single_static_ability_ast_rule!(parse_foretelling_cards_cost_modifier_line),
        single_static_ability_ast_rule!(parse_players_skip_upkeep_line),
        single_static_ability_ast_rule!(parse_legend_rule_doesnt_apply_line),
        multi_static_ability_ast_rule!(
            parse_subject_are_card_types_in_addition_to_their_other_types_line
        ),
        single_static_ability_ast_rule!(parse_all_permanents_colorless_line),
        single_static_ability_ast_rule!(parse_all_cards_spells_permanents_colorless_line),
        multi_static_ability_ast_rule!(parse_all_are_color_and_type_addition_line),
        single_static_ability_ast_rule!(parse_all_cards_spells_permanents_add_chosen_color_line),
        single_static_ability_ast_rule!(parse_all_creatures_are_color_line),
        single_static_ability_ast_rule!(parse_subjects_are_basic_line),
        single_static_ability_ast_rule!(parse_protection_from_colored_spells_line),
        single_static_ability_ast_rule!(parse_nonbasic_lands_are_basic_land_type_line),
        single_static_ability_ast_rule!(parse_land_type_addition_line),
        multi_static_ability_ast_rule!(parse_lands_are_pt_creatures_still_lands_line),
        single_static_ability_ast_rule!(parse_remove_snow_line),
        multi_static_ability_ast_rule!(parse_attached_is_legendary_gets_and_has_keywords_line),
        single_static_ability_ast_rule!(parse_landwalk_as_though_block_override_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_granted_keyword_static_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_granted_keyword_static_line),
        },
        multi_static_ability_ast_rule!(parse_equipment_you_control_have_equip_line),
        multi_static_ability_ast_rule!(parse_lose_all_abilities_and_transform_base_pt_line),
        multi_static_ability_ast_rule!(parse_lose_all_abilities_and_base_pt_line),
        multi_static_ability_ast_passthrough_rule!(parse_subject_loses_keywords_line),
        single_static_ability_ast_passthrough_rule!(parse_all_creatures_lose_flying_line),
        single_static_ability_ast_passthrough_rule!(
            parse_each_creature_cant_be_blocked_by_more_than_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_each_creature_can_block_additional_creature_each_combat_line
        ),
        multi_static_ability_ast_rule!(parse_anthem_and_type_color_addition_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_anthem_and_keyword_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_anthem_and_keyword_line),
        },
        multi_static_ability_ast_rule!(parse_anthem_and_goaded_line),
        multi_static_ability_ast_passthrough_rule!(parse_anthem_and_granted_ability_line),
        multi_static_ability_ast_passthrough_rule!(
            parse_subject_has_keywords_and_cant_be_blocked_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_subject_is_every_subtype_family_line),
        single_static_ability_ast_passthrough_rule!(parse_all_have_indestructible_line),
        single_static_ability_ast_passthrough_rule!(
            parse_subject_cant_be_blocked_as_long_as_defending_player_controls_card_type_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_subject_cant_be_blocked_as_long_as_condition_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_subject_cant_be_blocked_line),
        single_static_ability_ast_rule!(parse_may_choose_not_to_untap_during_untap_step_line),
        single_static_ability_ast_rule!(parse_untap_during_each_other_players_untap_step_line),
        single_static_ability_ast_passthrough_rule!(parse_doesnt_untap_during_untap_step_line),
        multi_static_ability_ast_rule!(parse_equipped_creature_has_line),
        multi_static_ability_ast_rule!(parse_enchanted_creature_has_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_tap_abilities_cant_be_activated_line
        ),
        multi_static_ability_ast_rule!(parse_attached_type_transform_line),
        multi_static_ability_ast_rule!(parse_attached_has_and_loses_keywords_line),
        single_static_ability_ast_rule!(parse_you_control_attached_creature_line),
        single_static_ability_ast_passthrough_rule!(parse_attached_cant_attack_or_block_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_damage_dealt_by_attached_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_combat_damage_dealt_by_attached_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_damage_dealt_to_attached_line
        ),
        multi_static_ability_ast_passthrough_rule!(parse_attached_gets_and_cant_block_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_attached_has_keywords_and_triggered_ability_line),
            rule: StaticAbilityLineRuleAst::Multi(
                parse_attached_has_keywords_and_triggered_ability_line,
            ),
        },
        StaticAbilityLineRuleDef {
            id: stringify!(parse_attached_gets_and_has_ability_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_attached_gets_and_has_ability_line),
        },
        StaticAbilityLineRuleDef {
            id: stringify!(parse_anthem_with_trailing_segments_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_anthem_with_trailing_segments_line),
        },
        multi_static_ability_ast_passthrough_rule!(parse_gets_and_attacks_each_combat_if_able_line),
        single_static_ability_ast_passthrough_rule!(
            parse_conditional_all_creatures_able_to_block_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_as_long_as_condition_can_attack_as_though_no_defender_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_source_can_attack_as_though_no_defender_as_long_as_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_attacks_each_combat_if_able_line),
        single_static_ability_ast_rule!(parse_source_must_be_blocked_if_able_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_composed_anthem_effects_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_composed_anthem_effects_line),
        },
        single_static_ability_ast_rule!(parse_has_base_power_toughness_static_line),
        single_static_ability_ast_rule!(parse_isnt_creature_line),
        multi_static_ability_ast_rule!(parse_multi_subject_anthem_line),
        single_static_ability_ast_rule!(parse_anthem_line),
        single_static_ability_ast_rule!(parse_flying_restriction_line),
        single_static_ability_ast_rule!(parse_can_block_only_flying_line),
        single_static_ability_ast_rule!(parse_can_block_subtype_as_though_reach_line),
        single_static_ability_ast_rule!(parse_assign_damage_as_unblocked_line),
        single_static_ability_ast_rule!(parse_fixed_mana_cost_instead_of_mana_cost_grant_line),
        single_static_ability_ast_rule!(parse_mana_value_instead_of_mana_cost_grant_line),
        single_static_ability_ast_rule!(parse_life_mana_value_instead_of_mana_cost_grant_line),
        single_static_ability_ast_rule!(parse_as_enters_or_turns_face_up_pt_choice_line),
        single_static_ability_ast_rule!(parse_as_enters_becomes_characteristics_for_filter_line),
        single_static_ability_ast_rule!(parse_enter_as_copy_as_enters_line),
        multi_static_ability_ast_rule!(
            parse_you_may_cast_exile_counter_cards_with_mana_permission_line
        ),
        multi_static_ability_ast_rule!(parse_surveilled_graveyard_play_life_cost_line),
        single_static_ability_ast_rule!(parse_play_from_permission_with_haste_this_way_line),
        single_static_ability_ast_rule!(parse_you_may_static_grant_line),
        single_static_ability_ast_rule!(parse_grant_flash_to_noncreature_spells_line),
        single_static_ability_ast_rule!(parse_cast_this_spell_as_though_it_had_flash_line),
        single_static_ability_ast_rule!(parse_during_your_turn_prevent_all_damage_to_source_line),
        single_static_ability_ast_rule!(parse_prevent_all_combat_damage_to_source_line),
        single_static_ability_ast_rule!(
            parse_prevent_all_combat_damage_to_matching_permanents_line
        ),
        single_static_ability_ast_rule!(
            parse_prevent_all_noncombat_damage_to_other_creatures_you_control_line
        ),
        single_static_ability_ast_rule!(parse_prevent_all_damage_to_source_by_creatures_line),
        single_static_ability_ast_rule!(
            parse_prevent_damage_to_other_creature_you_control_put_counters_line
        ),
        single_static_ability_ast_rule!(parse_prevent_all_damage_dealt_to_creatures_line),
        single_static_ability_ast_passthrough_rule!(parse_creatures_cant_block_line),
        multi_static_ability_ast_rule!(parse_enters_tapped_with_counters_line),
        single_static_ability_ast_rule!(parse_enters_with_counters_line),
        single_static_ability_ast_rule!(parse_enters_with_additional_counter_for_filter_line),
        single_static_ability_ast_rule!(parse_reveal_from_hand_or_enters_tapped_line),
        single_static_ability_ast_rule!(parse_conditional_enters_tapped_unless_line),
        single_static_ability_ast_rule!(parse_enters_untapped_for_filter_line),
        single_static_ability_ast_rule!(parse_enters_tapped_for_filter_line),
        single_static_ability_ast_rule!(parse_enters_tapped_line),
        multi_static_ability_ast_rule!(parse_additional_land_play_line),
        single_static_ability_ast_rule!(parse_you_may_look_top_card_any_time_line),
        single_static_ability_ast_rule!(
            parse_you_may_look_face_down_creatures_you_dont_control_any_time_line
        ),
        single_static_ability_ast_rule!(parse_players_play_top_card_libraries_revealed_line),
        single_static_ability_ast_rule!(parse_play_top_card_your_library_revealed_line),
        single_static_ability_ast_rule!(parse_your_opponents_play_with_hands_revealed_line),
        single_static_ability_ast_rule!(parse_control_opponents_while_searching_libraries_line),
        single_static_ability_ast_rule!(parse_opponent_search_exile_found_cards_line),
        single_static_ability_ast_rule!(parse_cast_this_card_from_library_while_searching_line),
        single_static_ability_ast_rule!(parse_play_lands_from_graveyard_line),
        single_static_ability_ast_rule!(parse_graveyard_cards_have_retrace_line),
        single_static_ability_ast_rule!(parse_cast_spells_from_hand_without_paying_mana_costs_line),
        single_static_ability_ast_rule!(parse_cost_reduction_line),
        single_static_ability_ast_rule!(parse_can_block_additional_creature_each_combat_line),
        single_static_ability_ast_passthrough_rule!(parse_all_creatures_able_to_block_source_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_all_creatures_able_to_block_line
        ),
        single_static_ability_ast_rule!(parse_activated_abilities_cant_be_activated_line),
        multi_static_ability_ast_rule!(parse_cant_clauses),
    ]
}

static STATIC_ABILITY_AST_LINE_RULE_INDEX: LazyLock<LexRuleHintIndex> = LazyLock::new(|| {
    let rules = static_ability_ast_line_rules();
    build_lex_rule_hint_index(rules.len(), |idx| {
        static_ability_rule_head_hints(rules[idx].id)
    })
});

fn parse_static_ability_ast_line_lowered(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let rules = static_ability_ast_line_rules();
    let (head, second) = lexed_head_words(tokens).unwrap_or(("", None));
    let mut tried = vec![false; rules.len()];
    let mut deferred_error: Option<CardTextError> = None;

    let candidate_indices = STATIC_ABILITY_AST_LINE_RULE_INDEX.candidate_indices(head, second);
    if !candidate_indices.is_empty() {
        if let Some(abilities) = try_static_ability_ast_line_rule_indices(
            rules,
            tokens,
            &mut tried,
            &mut deferred_error,
            &candidate_indices,
        ) {
            return Ok(Some(abilities));
        }
    }

    for (idx, rule) in rules.iter().enumerate() {
        if tried[idx] {
            continue;
        }
        match run_static_ability_ast_line_rule(rule.rule, tokens) {
            Ok(Some(abilities)) => return Ok(Some(abilities)),
            Ok(None) => {}
            Err(err) => {
                deferred_error.get_or_insert(err);
            }
        }
    }

    if let Some(err) = deferred_error {
        return Err(err);
    }

    Ok(None)
}

fn title_case_count_as_card_name(words: &[&str]) -> String {
    words
        .iter()
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                let mut out = first.to_ascii_uppercase().to_string();
                out.push_str(chars.as_str());
                out
            } else {
                String::new()
            }
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_count_as_card_named_for_spell_effect_line(words: &[&str]) -> Option<StaticAbility> {
    const GRAVEYARD_PREFIXES: &[&[&str]] = &[
        &["if", "this", "card", "is", "in", "a", "graveyard"],
        &["if", "this", "card", "is", "in", "your", "graveyard"],
    ];
    if !GRAVEYARD_PREFIXES
        .iter()
        .any(|prefix| word_slice_starts_with(words, prefix))
    {
        return None;
    }

    let effects_idx = word_slice_find_phrase_start(words, &["effects", "from", "spells", "named"])?;
    let spell_name_start = effects_idx + 4;
    let count_idx =
        word_slice_find_word(words.get(spell_name_start..).unwrap_or_default(), "count")?
            + spell_name_start;
    let spell_name_words = words.get(spell_name_start..count_idx)?;
    if spell_name_words.is_empty()
        || !words
            .get(count_idx..count_idx + 6)
            .is_some_and(|tail| tail == ["count", "it", "as", "a", "card", "named"])
    {
        return None;
    }

    let counted_name_words = words.get(count_idx + 6..)?;
    if counted_name_words.is_empty() {
        return None;
    }

    Some(StaticAbility::count_as_card_named_for_spell_effect(
        title_case_count_as_card_name(spell_name_words),
        title_case_count_as_card_name(counted_name_words),
    ))
}

fn parse_static_ability_ast_line_early_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let rendered_words = parser_token_word_refs(tokens);
    let marker_text = render_token_slice(tokens);
    if is_ticket_sticker_marker_text(&marker_text) {
        return Ok(Some(vec![keyword_static_marker(tokens).into()]));
    }

    if X_CANT_EXCEED_PLAYER_COUNT_PATTERN.matches_words(&rendered_words) {
        return Ok(Some(vec![
            StaticAbility::this_spell_x_maximum(
                Value::CountPlayers(PlayerFilter::Any),
                "X can't be greater than the number of players in the game.",
            )
            .into(),
        ]));
    }
    if EXHAUST_AS_THOUGH_UNACTIVATED_PATTERN.matches_words(&rendered_words) {
        return Ok(Some(vec![
            StaticAbility::exhaust_abilities_as_though_unactivated_this_turn().into(),
        ]));
    }
    if CANT_ATTACK_UNLESS_CAST_CREATURE_SPELL_PATTERN.matches_words(&rendered_words) {
        return Ok(Some(vec![
            StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn().into(),
        ]));
    }
    if CANT_ATTACK_UNLESS_CAST_NONCREATURE_SPELL_PATTERN.matches_words(&rendered_words) {
        return Ok(Some(vec![
            StaticAbility::cant_attack_unless_controller_cast_noncreature_spell_this_turn().into(),
        ]));
    }

    if let Some(spec) = parse_reveal_first_card_you_draw_each_turn_spec_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::reveal_first_card_you_draw_each_turn(
                spec.optional,
                spec.your_turns_only,
            )
            .into(),
        ]));
    }

    let words = parser_token_word_refs(tokens);
    if let Some(ability) = parse_count_as_card_named_for_spell_effect_line(&words) {
        return Ok(Some(vec![ability.into()]));
    }
    if SOURCE_CAN_BLOCK_PREFIX_PATTERN.matches_words(&words) {
        let mut idx = 4usize;
        if words
            .get(idx)
            .is_some_and(|word| ARTICLE_WORD_PATTERN.matches_word(word))
        {
            idx += 1;
        }

        if words
            .get(idx)
            .is_some_and(|word| BLOCK_ADDITIONAL_WORD_PATTERN.matches_word(word))
        {
            idx += 1;
            let mut additional = 1usize;
            if let Some((count, used)) = parse_number(&tokens[idx..]) {
                additional = count as usize;
                idx += used;
            }

            if words
                .get(idx)
                .is_some_and(|word| BLOCK_CREATURE_OR_CREATURES_WORD_PATTERN.matches_word(word))
                && (words.get(idx + 1..).unwrap_or_default().is_empty()
                    || BLOCK_ADDITIONAL_DURATION_TAIL_PATTERN.matches_words(&words[idx + 1..]))
            {
                return Ok(Some(vec![
                    StaticAbility::can_block_additional_creature_each_combat(additional).into(),
                ]));
            }
        }
    }

    if DAY_NIGHT_AS_ENTERS_CONTAINS_PATTERN.matches_words(&words) {
        return Ok(Some(vec![
            StaticAbility::day_night_starts_day_as_enters().into(),
        ]));
    }
    if DAY_NIGHT_AS_ENTERS_PATTERN.matches_words(&words) {
        return Ok(Some(vec![
            StaticAbility::day_night_starts_day_as_enters().into(),
        ]));
    }
    if TOUGHNESS_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)
        || POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)
        || LOYALTY_COUNTER_INSTEAD_OF_CREW_COST_MARKER_PATTERN.matches_words(&words)
    {
        return Ok(Some(vec![keyword_static_marker(tokens).into()]));
    }
    if is_minimum_spell_total_mana_three_line_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::minimum_spell_total_mana(3).into(),
        ]));
    }
    if is_players_cant_pay_life_or_sacrifice_line_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate().into(),
        ]));
    }
    if is_krrik_black_mana_life_payment_line_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::krrik_black_mana_may_be_paid_with_life().into(),
        ]));
    }
    if let Some(ability) = parse_cycling_cost_alternative_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    if let Some(abilities) = parse_spell_and_player_activated_ability_cost_modifier_line(tokens)? {
        return Ok(Some(
            abilities.into_iter().map(StaticAbilityAst::from).collect(),
        ));
    }

    if let Some(spec) = split_untap_each_other_players_untap_step_line_lexed(tokens) {
        let subject_tokens = trim_commas(spec.subject_tokens);
        let filter = parse_object_filter(&subject_tokens, false)?;
        let subject_text = crate::runtime_backend::token_word_refs(&subject_tokens).join(" ");
        let display = if spec.untap_all {
            format!("Untap all {subject_text} during each other player's untap step")
        } else {
            format!("Untap {subject_text} during each other player's untap step")
        };
        return Ok(Some(vec![
            StaticAbility::untap_during_each_other_players_untap_step(filter, display).into(),
        ]));
    }

    if let Some(ability) = parse_activated_abilities_cant_be_activated_line_lexed(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    if let Some(ability) = parse_legend_rule_doesnt_apply_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }

    Ok(None)
}

pub(crate) fn parse_damage_doubling_mana_value_marker_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !DAMAGE_DOUBLING_MANA_VALUE_MARKER_PATTERN.matches_words(&clause_words)
        || !DAMAGE_DOUBLING_TO_TARGET_PATTERN.matches_words(&clause_words)
    {
        return Ok(None);
    }

    Ok(Some(keyword_static_marker(tokens)))
}

pub(crate) fn parse_static_ability_ast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() > 1 {
        let mut combined = Vec::new();
        for sentence in sentences {
            match parse_static_ability_ast_line_lexed_single(sentence) {
                Ok(Some(mut parsed)) => combined.append(&mut parsed),
                Ok(None) | Err(_) => return parse_static_ability_ast_line_lexed_single(tokens),
            }
        }
        return Ok((!combined.is_empty()).then_some(combined));
    }

    parse_static_ability_ast_line_lexed_single(tokens)
}

fn parse_static_ability_ast_line_lexed_single(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    if looks_like_trigger_intro_tokens(tokens) || looks_like_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    if looks_like_player_counter_gain_effect_tokens(tokens) {
        return Ok(None);
    }

    if let Some(abilities) = parse_static_ability_ast_line_early_lexed(tokens)? {
        return Ok(Some(abilities));
    }

    parse_static_ability_ast_line_lowered(tokens)
}

fn looks_like_player_counter_gain_effect_tokens(tokens: &[OwnedLexToken]) -> bool {
    let Some(get_idx) = find_index(tokens, |token| {
        GET_OR_GETS_WORD_PATTERN.matches_token(token)
    }) else {
        return false;
    };

    let has_counter_resource = tokens.iter().any(|token| {
        PLAYER_COUNTER_RESOURCE_WORD_PATTERN.matches_token(token)
            || (token.kind == TokenKind::ManaGroup
                && token.mana_group_inner().is_some_and(|inner| {
                    inner.eq_ignore_ascii_case("e") || inner.eq_ignore_ascii_case("tk")
                }))
    });
    if !has_counter_resource {
        return false;
    }

    let tail = tokens.get(get_idx + 1..).unwrap_or_default();
    if parse_value(tail).is_some() {
        return true;
    }

    tail.iter()
        .find_map(OwnedLexToken::as_word)
        .is_some_and(|word| {
            ANOTHER_OR_CARDINAL_WORD_PATTERN.matches_word(word)
                || ironsmith_core::parse_cardinal_word(word).is_some()
        })
}

pub(crate) fn parse_activated_abilities_cant_be_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    use crate::effect::Restriction;

    let normalized = crate::runtime_backend::token_word_refs(tokens);
    if normalized.len() < 6 || !ACTIVATED_ABILITIES_OF_PREFIX_PATTERN.matches_words(&normalized) {
        return Ok(None);
    }

    let Some(cant_idx) = find_index(&normalized, |word| CANT_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    if cant_idx <= 3 {
        return Ok(None);
    }

    let tail = &normalized[cant_idx..];
    if !CANT_BE_ACTIVATED_TAIL_PATTERN.matches_words(tail) {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[3..cant_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    // "Activated abilities of artifacts and creatures ..." should be a union of types.
    // Our general object filter parser treats type lists joined by "and" as intersection,
    // which is correct for many adjective chains, but incorrect for this rules pattern.
    let subject_words = crate::runtime_backend::util::non_article_token_word_refs(&subject_tokens);

    let filter = if let Some(filter) = activated_ability_subject_special_filter(&subject_words) {
        filter
    } else {
        parse_object_filter(&subject_tokens, false)?
    };

    let non_mana_only = MANA_ABILITIES_EXCEPTION_PATTERN.matches_words(&normalized);

    let restriction = if non_mana_only {
        Restriction::activate_non_mana_abilities_of(filter)
    } else {
        Restriction::activate_abilities_of(filter)
    };

    let display_subject = subject_words.join(" ");
    let display = if non_mana_only {
        format!(
            "Activated abilities of {display_subject} can't be activated unless they're mana abilities."
        )
    } else {
        format!("Activated abilities of {display_subject} can't be activated.")
    };

    Ok(Some(StaticAbility::restriction(restriction, display)))
}

pub(crate) fn parse_activated_abilities_cost_increase_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 8 || !ACTIVATED_ABILITIES_OF_PREFIX_PATTERN.matches_words(&clause_words)
    {
        return Ok(None);
    }

    let Some(cost_idx) = find_index(&clause_words, |word| {
        COST_OR_COSTS_WORD_PATTERN.matches_word(word)
    }) else {
        return Ok(None);
    };
    if cost_idx <= 3 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[3..cost_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported activated-ability cost increase subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }

    let amount_tokens = trim_commas(&tokens[cost_idx + 1..]);
    let amount_words = crate::runtime_backend::token_word_refs(&amount_tokens);
    if !ADDITIONAL_COST_PREFIX_PATTERN.matches_words(&amount_words) {
        return Ok(None);
    }

    let cost_tokens = trim_commas(&amount_tokens[2..]);
    let Some(to_idx) = find_index(
        &crate::runtime_backend::token_word_refs(&cost_tokens),
        |word| TO_WORD_PATTERN.matches_word(word),
    ) else {
        return Ok(None);
    };
    let to_token_idx = crate::runtime_backend::token_index_for_word_index(&cost_tokens, to_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing activated-ability additional cost terminator (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let additional_cost_tokens = trim_commas(&cost_tokens[..to_token_idx]);
    let additional_cost_tokens = trim_outer_quotes(&additional_cost_tokens);
    if additional_cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing activated-ability additional cost (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let total_cost = parse_activation_cost(&additional_cost_tokens)?;
    if total_cost.is_free() {
        return Err(CardTextError::ParseError(format!(
            "unsupported activated-ability additional cost (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let tail_words = crate::runtime_backend::token_word_refs(&cost_tokens[to_token_idx..]);
    if !TO_ACTIVATE_PREFIX_PATTERN.matches_words(&tail_words) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::increase_activated_ability_costs(
        filter, total_cost,
    )))
}

pub(crate) fn parse_activated_abilities_cant_be_activated_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    use crate::effect::Restriction;

    let Some(spec) = parse_activated_abilities_cant_be_activated_spec_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = spec.subject_tokens;
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = crate::runtime_backend::util::non_article_token_word_refs(subject_tokens);

    let filter = if let Some(filter) = activated_ability_subject_special_filter(&subject_words) {
        filter
    } else {
        parse_object_filter_lexed(subject_tokens, false)?
    };

    let non_mana_only = spec.non_mana_only;

    let restriction = if non_mana_only {
        Restriction::activate_non_mana_abilities_of(filter)
    } else {
        Restriction::activate_abilities_of(filter)
    };

    let display_subject = subject_words.join(" ");
    let display = if non_mana_only {
        format!(
            "Activated abilities of {display_subject} can't be activated unless they're mana abilities."
        )
    } else {
        format!("Activated abilities of {display_subject} can't be activated.")
    };

    Ok(Some(StaticAbility::restriction(restriction, display)))
}

pub(crate) fn parse_pregame_begin_on_battlefield_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !IF_PREFIX_PATTERN.matches_words(&clause_words)
        || !PREGAME_BEGIN_ON_BATTLEFIELD_PATTERN.matches_words(&clause_words)
    {
        return Ok(None);
    }

    let source_ref_start = find_source_reference_start(&tokens[1..]).map(|idx| idx + 1);
    if source_ref_start.is_none() && !THIS_CARD_IS_PREFIX_PATTERN.matches_words(&clause_words[1..])
    {
        return Ok(None);
    }

    let require_not_starting_player =
        NOT_STARTING_PLAYER_CONDITION_PATTERN.matches_words(&clause_words);

    let battlefield_end_word_idx =
        keyword_find_prefix_shape_start(&clause_words, &ON_THE_BATTLEFIELD_PREFIX_PATTERN)
            .map(|idx| idx + 3)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing battlefield destination in pregame line (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
    let if_you_do_word_idx =
        keyword_find_prefix_shape_start(&clause_words, &IF_YOU_DO_PREFIX_PATTERN);

    let mut counters = Vec::new();
    let counter_tail_start =
        token_index_for_word_index(tokens, battlefield_end_word_idx).unwrap_or(tokens.len());
    let counter_tail_end = if_you_do_word_idx
        .and_then(|idx| token_index_for_word_index(tokens, idx))
        .unwrap_or(tokens.len());
    let counter_tail = trim_commas(&tokens[counter_tail_start..counter_tail_end]);
    if !counter_tail.is_empty() {
        if !counter_tail
            .first()
            .is_some_and(|token| WITH_WORD_PATTERN.matches_token(token))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported pregame battlefield modifier (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let after_with = &counter_tail[1..];
        let (count, used) = if after_with
            .first()
            .is_some_and(|token| ARTICLE_WORD_PATTERN.matches_token(token))
        {
            (1u32, 1usize)
        } else {
            parse_number(after_with).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter count in pregame line (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
        };
        let counter_type =
            parse_counter_type_from_tokens(&after_with[used..]).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported counter type in pregame line (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let counter_word_idx = find_index(after_with, |token| {
            COUNTER_OR_COUNTERS_WORD_PATTERN.matches_token(token)
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter keyword in pregame line (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let trailing = crate::runtime_backend::token_word_refs(&after_with[counter_word_idx + 1..]);
        if !PREGAME_COUNTER_ON_IT_TAIL_PATTERN.matches_words(&trailing) {
            return Err(CardTextError::ParseError(format!(
                "unsupported counter placement tail in pregame line (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        counters.push((counter_type, count));
    }

    let exile_cards_from_hand = if let Some(if_you_do_word_idx) = if_you_do_word_idx {
        let exile_start =
            token_index_for_word_index(tokens, if_you_do_word_idx + 3).unwrap_or(tokens.len());
        let exile_tail = trim_commas(&tokens[exile_start..]);
        if !exile_tail
            .first()
            .is_some_and(|token| EXILE_WORD_PATTERN.matches_token(token))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported pregame follow-up clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let after_exile = &exile_tail[1..];
        let (count, used) = if after_exile
            .first()
            .is_some_and(|token| ARTICLE_WORD_PATTERN.matches_token(token))
        {
            (1u32, 1usize)
        } else {
            parse_number(after_exile).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing exile count in pregame follow-up (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
        };
        let trailing = crate::runtime_backend::token_word_refs(&after_exile[used..]);
        if !PREGAME_EXILE_FROM_HAND_TAIL_PATTERN.matches_words(&trailing) {
            return Err(CardTextError::ParseError(format!(
                "unsupported pregame exile tail (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        count as usize
    } else {
        0
    };

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::BeginOnBattlefield(
            crate::static_abilities::PregameBeginOnBattlefieldSpec {
                require_not_starting_player,
                counters,
                exile_cards_from_hand,
            },
        ),
        clause_words.join(" "),
    )))
}

pub(crate) fn parse_pregame_mulligan_redraw_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !PREGAME_MULLIGAN_REDRAW_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::MulliganExileHandDrawSameCount,
        clause_words.join(" "),
    )))
}

pub(crate) fn parse_pregame_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(choose_idx) = find_index(&clause_words, |word| CHOOSE_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    let Some((consumed, excluded)) = parse_choose_color_phrase_words(&clause_words[choose_idx..])?
    else {
        return Ok(None);
    };
    if excluded.is_some() {
        return Ok(None);
    }
    let tail = &clause_words[choose_idx + consumed..];
    if !BEFORE_GAME_BEGINS_TAIL_PATTERN.matches_words(tail) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::ChooseColor,
        clause_words.join(" "),
    )))
}

pub(crate) fn parse_combined_pregame_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let sentences = split_lexed_slices_on_period(tokens);
    if sentences.len() < 2 {
        return Ok(None);
    }

    let Some(first) = parse_pregame_choose_color_line(sentences[0])? else {
        return Ok(None);
    };
    let Some(second) = parse_source_is_chosen_color_line(sentences[1])? else {
        return Ok(None);
    };
    Ok(Some(vec![first, second]))
}

pub(crate) fn parse_can_block_additional_creature_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = crate::runtime_backend::token_word_refs(tokens);
    if !SOURCE_CAN_BLOCK_PREFIX_PATTERN.matches_words(&normalized)
        || !matches!(
            normalized.last().copied(),
            Some("combat") | Some("turn") | Some("creature") | Some("creatures")
        )
    {
        return Ok(None);
    }

    let mut idx = 4usize;
    if normalized
        .get(idx)
        .is_some_and(|word| ARTICLE_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }

    if !normalized
        .get(idx)
        .is_some_and(|word| BLOCK_ADDITIONAL_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    idx += 1;

    let mut additional = 1usize;
    if let Some((count, used)) = parse_number(&tokens[idx..]) {
        additional = count as usize;
        idx += used;
    }

    if !normalized
        .get(idx)
        .is_some_and(|word| BLOCK_CREATURE_OR_CREATURES_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    idx += 1;

    let tail = &normalized[idx..];
    if !tail.is_empty() && !BLOCK_ADDITIONAL_DURATION_TAIL_PATTERN.matches_words(tail) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::can_block_additional_creature_each_combat(additional),
    ))
}

pub(crate) fn parse_skulk_rules_text_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_skulk_rules_text_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::cant_be_blocked_by_lower_power_than_source(),
    ))
}

pub(crate) fn parse_ward_static_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !WARD_WORD_PATTERN.matches_first_word(&clause_words) {
        return Ok(None);
    }

    let cost_tokens = trim_commas(&tokens[1..]);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "ward keyword missing cost".to_string(),
        ));
    }

    if let Some(cost) = parse_ward_discard_card_type_cost(&cost_tokens) {
        return Ok(Some(StaticAbility::ward(cost)));
    }

    if let Some(cost) = parse_payment_clause_as_total_cost(&cost_tokens)? {
        return Ok(Some(StaticAbility::ward(cost)));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported ward cost clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_ward_discard_card_type_cost(tokens: &[OwnedLexToken]) -> Option<TotalCost> {
    let cost_words = crate::runtime_backend::token_word_refs(tokens);
    if !DISCARD_WORD_PATTERN.matches_first_word(&cost_words) {
        return None;
    }

    let mut idx = 1usize;
    let mut count = 1u32;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }

    let words_tail = &cost_words[idx..];
    if WARD_DISCARD_HAND_TAIL_PATTERN.matches_words(words_tail) {
        return Some(TotalCost::from_cost(crate::costs::Cost::discard_hand()));
    }

    while cost_words
        .get(idx)
        .is_some_and(|word| ARTICLE_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }

    let mut card_types = Vec::<CardType>::new();
    while let Some(word) = cost_words.get(idx) {
        if CARD_OR_CARDS_WORD_PATTERN.matches_word(word) {
            idx += 1;
            break;
        }
        if DISCARD_COST_IGNORED_WORD_PATTERN.matches_word(word) {
            idx += 1;
            continue;
        }
        let parsed = parse_card_type(word)?;
        if !card_types.iter().any(|existing| *existing == parsed) {
            card_types.push(parsed);
        }
        idx += 1;
    }

    if idx != cost_words.len() {
        return None;
    }

    let cost = if card_types.len() > 1 {
        crate::costs::Cost::discard_types(count, card_types)
    } else if let Some(card_type) = card_types.first().copied() {
        crate::costs::Cost::discard(count, Some(card_type))
    } else {
        crate::costs::Cost::discard(count, None)
    };
    Some(TotalCost::from_cost(cost))
}

pub(crate) fn parse_composed_anthem_effects_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if contains_until_end_of_turn(&clause_words) {
        return Ok(None);
    }

    let comma_segments = split_anthem_trailing_segments_preserving_granted_abilities(tokens);
    if comma_segments.len() < 2 {
        return Ok(None);
    }

    if comma_segments.len() == 2 {
        let where_tail = trim_commas(&comma_segments[1]);
        let where_words = crate::runtime_backend::token_word_refs(&where_tail);
        if WHERE_X_IS_PREFIX_PATTERN.matches_words(&where_words)
            && let Some(ability) = parse_anthem_line(tokens)?
        {
            return Ok(Some(vec![ability.into()]));
        }
    }

    let Some(first_action_idx) = find_index(tokens, |token| {
        GET_GETS_HAVE_HAS_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(&tokens[..first_action_idx]);
    if subject_tokens.is_empty() || parse_anthem_subject(&subject_tokens).is_err() {
        return Ok(None);
    }

    let mut saw_omitted_subject_clause = false;
    let mut compiled = Vec::new();

    for (idx, raw_segment) in comma_segments.into_iter().enumerate() {
        let mut segment = trim_commas(&raw_segment).to_vec();
        if segment.is_empty() {
            continue;
        }

        if token_slice_first_is(&segment, "and") {
            let trimmed = trim_commas(&segment[1..]);
            if token_slice_first_is_any(&trimmed, &["get", "gets", "have", "has"]) {
                segment = trimmed.to_vec();
            }
        }

        let starts_with_action =
            token_slice_first_is_any(&segment, &["get", "gets", "have", "has"]);
        if starts_with_action {
            if idx > 0 {
                saw_omitted_subject_clause = true;
            }
            let mut expanded = subject_tokens.clone();
            expanded.extend(segment);
            segment = expanded;
        }

        let parsed_segment =
            if let Some(abilities) = parse_anthem_and_type_color_addition_line(&segment)? {
                abilities.into_iter().map(StaticAbilityAst::from).collect()
            } else if let Some(abilities) = parse_anthem_and_keyword_line(&segment)? {
                abilities
            } else if let Some(abilities) = parse_granted_keyword_static_line(&segment)? {
                abilities
            } else if let Some(ability) = parse_anthem_line(&segment)? {
                vec![ability.into()]
            } else {
                return Ok(None);
            };
        compiled.extend(parsed_segment);
    }

    if !saw_omitted_subject_clause || compiled.len() < 2 {
        return Ok(None);
    }

    Ok(Some(compiled))
}

pub(crate) fn parse_static_text_marker_line(tokens: &[OwnedLexToken]) -> Option<StaticAbility> {
    if tokens.is_empty() {
        return None;
    }

    if is_once_each_turn_play_from_exile_marker_guard_lexed(tokens) {
        return None;
    }

    if is_doctors_companion_marker_line_lexed(tokens) {
        return Some(StaticAbility::doctors_companion());
    }

    if BANDING_MARKER_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens)) {
        return Some(StaticAbility::banding());
    }

    if is_companion_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_more_than_meets_the_eye_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_protection_mana_value_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_mana_group_slash_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_if_source_you_control_with_mana_value_double_instead_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_as_long_as_power_odd_or_even_flash_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_attack_as_haste_unless_entered_this_turn_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_sab_sunen_cant_attack_or_block_unless_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_you_have_shroud_line_lexed(tokens) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::be_targeted_player(PlayerFilter::You),
            "You have shroud".to_string(),
        ));
    }

    if YOU_HAVE_HEXPROOF_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens)) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::be_targeted_player_from(
                PlayerFilter::You,
                ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
            ),
            "You have hexproof".to_string(),
        ));
    }

    if YOU_HAVE_PROTECTION_FROM_OPPONENTS_PATTERN
        .matches_words(&crate::runtime_backend::token_word_refs(tokens))
    {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::be_targeted_player_from(
                PlayerFilter::You,
                ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
            ),
            "You have protection from each of your opponents".to_string(),
        ));
    }

    let words = parser_token_word_refs(tokens);
    if OPPONENTS_CAST_ONLY_AS_SORCERY_PATTERN.matches_words(&words) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::cast_spells_only_as_sorcery(PlayerFilter::Opponent),
            "Each opponent can cast spells only any time they could cast a sorcery.".to_string(),
        ));
    }

    if DOUBLE_DAMAGE_TO_ENCHANTED_PLAYER_PATTERN.matches_words(&words) {
        return Some(StaticAbility::double_damage_amount_replacement(
            ObjectFilter::default(),
            Some(PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted"))),
            None,
            "If a source would deal damage to enchanted player, it deals double that damage to that player instead.".to_string(),
        ));
    }

    if is_creatures_without_flying_cant_attack_line_lexed(tokens) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::attack(
                ObjectFilter::creature()
                    .without_static_ability(crate::static_abilities::StaticAbilityId::Flying),
            ),
            "Creatures without flying can't attack".to_string(),
        ));
    }

    if is_this_creature_cant_attack_alone_line_lexed(tokens) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "This creature can't attack alone".to_string(),
        ));
    }

    if is_this_creature_cant_attack_its_owner_line_lexed(tokens) {
        return Some(StaticAbility::cant_attack_its_owner());
    }

    if let Some(amount) = parse_ward_pay_life_amount_lexed(tokens) {
        return Some(StaticAbility::ward(TotalCost::from_cost(
            crate::costs::Cost::life(amount),
        )));
    }

    if is_lands_dont_untap_during_their_controllers_untap_steps_line_lexed(tokens) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::untap(ObjectFilter::land()),
            "Lands don't untap during their controllers' untap steps".to_string(),
        ));
    }

    None
}

pub(crate) fn parse_filter_dont_untap_during_controllers_untap_steps_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(dont_word_idx) = find_index(&line_words, |word| {
        DONT_OR_DOESNT_WORD_PATTERN.matches_word(word)
    }) else {
        return Ok(None);
    };
    if !line_words
        .get(dont_word_idx + 1)
        .is_some_and(|word| UNTAP_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let tail = line_words.get(dont_word_idx + 2..).unwrap_or_default();
    if !CONTROLLERS_UNTAP_STEP_TAIL_PATTERN.matches_words(tail) {
        return Ok(None);
    }

    let dont_token_idx = token_index_for_word_index(tokens, dont_word_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map negated untap subject (clause: '{}')",
            line_words.join(" ")
        ))
    })?;
    let subject_tokens = trim_commas(&tokens[..dont_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter(&subject_tokens, false)?;
    let subject_text = crate::runtime_backend::token_word_refs(&subject_tokens).join(" ");
    let mut display = format!("{subject_text} don't untap during their controllers' untap steps");
    if let Some(first) = display
        .chars()
        .next()
        .map(|ch| ch.to_ascii_uppercase().to_string())
    {
        display.replace_range(0..1, &first);
    }

    Ok(Some(StaticAbility::restriction(
        crate::effect::Restriction::untap(filter),
        display,
    )))
}

fn parse_graveyard_metric_threshold_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::static_abilities::GraveyardCountMetric, u32)>, CardTextError> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if !THERE_IS_OR_ARE_PREFIX_PATTERN.matches_words(&words_all) {
        return Ok(None);
    }

    let quantified = &tokens[2..];
    let Ok((comparison, used)) = parse_static_quantity_prefix(quantified, false) else {
        return Ok(None);
    };
    let Some(threshold) = comparison_to_at_least_threshold(&comparison) else {
        return Ok(None);
    };

    let mut rest = &quantified[used..];
    if rest
        .first()
        .is_some_and(|token| CARD_OR_CARDS_WORD_PATTERN.matches_token(token))
        && !rest
            .get(1)
            .is_some_and(|token| TYPE_OR_TYPES_WORD_PATTERN.matches_token(token))
    {
        rest = &rest[1..];
    }
    let rest_words = crate::runtime_backend::token_word_refs(rest);
    if CARD_TYPES_IN_YOUR_GRAVEYARD_METRIC_PATTERN.matches_words(&rest_words) {
        return Ok(Some((
            crate::static_abilities::GraveyardCountMetric::CardTypes,
            threshold,
        )));
    }

    if MANA_VALUES_IN_YOUR_GRAVEYARD_METRIC_PATTERN.matches_words(&rest_words) {
        return Ok(Some((
            crate::static_abilities::GraveyardCountMetric::ManaValues,
            threshold,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_conditional_source_spell_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 10 {
        return Ok(None);
    }

    let Some(this_idx) =
        keyword_find_prefix_shape_start(&clause_words, &THIS_SPELL_HAS_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    let Some(keyword_word) = clause_words.get(this_idx + 3).copied() else {
        return Ok(None);
    };
    if !CONDITIONAL_SPELL_KEYWORD_WORD_PATTERN.matches_word(keyword_word) {
        return Ok(None);
    }
    let keyword = match keyword_word {
        "flash" => crate::static_abilities::ConditionalSpellKeywordKind::Flash,
        "cascade" => crate::static_abilities::ConditionalSpellKeywordKind::Cascade,
        _ => return Ok(None),
    };

    if !AS_LONG_AS_PREFIX_PATTERN.matches_words(&clause_words[this_idx + 4..]) {
        return Ok(None);
    }

    let condition_start = token_index_for_word_index(tokens, this_idx + 7).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map conditional spell keyword condition (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let condition_tokens = trim_commas(&tokens[condition_start..]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let Some((metric, threshold)) = parse_graveyard_metric_threshold_condition(&condition_tokens)?
    else {
        return Ok(None);
    };

    let spec = crate::static_abilities::ConditionalSpellKeywordSpec {
        keyword,
        metric,
        threshold,
    };
    Ok(Some(StaticAbility::conditional_spell_keyword(spec)))
}

pub(crate) fn parse_enters_tapped_with_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ENTERS_TAPPED_LINE_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }
    let tapped_word_idx = find_index(&clause_words, |word| TAPPED_WORD_PATTERN.matches_word(word))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing tapped keyword in enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let tapped_token_idx =
        token_index_for_word_index(tokens, tapped_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map tapped keyword in enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let trailing = &tokens[tapped_token_idx + 1..];
    if trailing.is_empty() {
        return Ok(None);
    }
    let Some(color_choice) = parse_choose_color_as_enters_line(trailing)? else {
        return Ok(None);
    };
    Ok(Some(vec![
        StaticAbility::enters_tapped_ability(),
        color_choice,
    ]))
}

pub(crate) fn parse_damage_not_removed_cleanup_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if DAMAGE_NOT_REMOVED_CLEANUP_PATTERN.matches_words(&words) {
        return Ok(Some(StaticAbility::damage_not_removed_during_cleanup()));
    }
    Ok(None)
}

fn parse_as_enters_choice_subject_words<'a>(
    words: &'a [&'a str],
    this_kind_display_pairs: &[(&str, &'static str)],
) -> Option<(usize, &'static str)> {
    if !words
        .first()
        .is_some_and(|word| AS_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let mut idx = 1usize;
    let display_subject = if words
        .get(idx)
        .is_some_and(|word| THIS_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
        if let Some(pair_idx) = find_index(this_kind_display_pairs, |(kind, _)| {
            words.get(idx) == Some(kind)
        }) {
            let (_, display) = this_kind_display_pairs[pair_idx];
            idx += 1;
            display
        } else {
            "this"
        }
    } else if words
        .get(idx)
        .is_some_and(|word| SOURCE_IT_PATTERN.matches_word(word))
    {
        idx += 1;
        "it"
    } else {
        let mut source_end = None;
        let mut scan = idx + 1;
        while scan < words.len() {
            if words
                .get(scan)
                .is_some_and(|word| ENTERS_WORD_PATTERN.matches_word(word))
                && source_reference_surface_for_words(&words[idx..scan]).is_some()
            {
                source_end = Some(scan);
                break;
            }
            scan += 1;
        }
        idx = source_end?;
        "this"
    };

    if !words
        .get(idx)
        .is_some_and(|word| ENTERS_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    idx += 1;

    if THE_BATTLEFIELD_PREFIX_PATTERN.matches_words(&words[idx..]) {
        idx += 2;
    }

    Some((idx, display_subject))
}

pub(crate) fn parse_choose_basic_land_type_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_AURA_SUBJECTS)
    else {
        return Ok(None);
    };
    if let Some(consumed) = parse_choose_basic_land_type_phrase_words(&words[idx..]) {
        if idx + consumed == words.len() {
            return Ok(Some(StaticAbility::choose_basic_land_type_as_enters(
                format!("As {display_subject} enters, choose a basic land type."),
            )));
        }
    }
    let Some(consumed) = parse_choose_land_type_phrase_words(&words[idx..]) else {
        return Ok(None);
    };
    if idx + consumed != words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_land_type_as_enters(format!(
        "As {display_subject} enters, choose a land type."
    ))))
}

pub(crate) fn parse_enchanted_land_is_chosen_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_enchanted_land_is_chosen_type_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::enchanted_land_is_chosen_type(
        "Enchanted land is the chosen type.".to_string(),
    )))
}

pub(crate) fn parse_choose_card_name_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    if !CHOOSE_CARD_NAME_TAIL_PATTERN.matches_words(&words[idx..]) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_card_name_as_enters(format!(
        "As {display_subject} enters, choose a card name."
    ))))
}

pub(crate) fn parse_note_life_total_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    if words.get(idx..) != Some(&["note", "your", "life", "total"][..]) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::note_life_total_as_enters(format!(
        "As {display_subject} enters, note your life total."
    ))))
}

pub(crate) fn parse_source_is_chosen_type_in_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(display) = parse_source_is_chosen_type_in_addition_line_lexed(tokens) else {
        return Ok(None);
    };

    Ok(Some(StaticAbility::add_chosen_creature_type(
        ObjectFilter::source(),
        display.to_string(),
    )))
}

pub(crate) fn parse_source_is_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(is_idx) = find_index(&words, |word| IS_WORD_PATTERN.matches_word(word)) else {
        return Ok(None);
    };
    let subject_words = &words[..is_idx];
    let is_source =
        is_source_reference_words(subject_words) || SOURCE_IT_PATTERN.matches_words(subject_words);
    if !is_source {
        return Ok(None);
    }

    let display_subject = match subject_words {
        ["this", "creature"] => "This creature",
        ["this", "permanent"] => "This permanent",
        ["this", "card"] => "This card",
        ["this"] => "This",
        ["it"] => "It",
        _ => "This",
    };

    let chosen_color_tail = &words[is_idx + 1..];
    if !CHOSEN_COLOR_TAIL_PATTERN.matches_words(chosen_color_tail) {
        return Ok(None);
    };
    let display = if THE_CHOSEN_COLOR_TAIL_PATTERN.matches_words(chosen_color_tail) {
        format!("{display_subject} is the chosen color.")
    } else {
        format!("{display_subject} is chosen color.")
    };

    Ok(Some(StaticAbility::set_chosen_color(
        ObjectFilter::source(),
        display,
    )))
}

pub(crate) fn parse_choose_creature_type_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS)
    else {
        return Ok(None);
    };
    let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(&words[idx..])?
    else {
        return Ok(None);
    };
    if !excluded_subtypes.is_empty() {
        return Ok(None);
    }
    if idx + consumed != words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_creature_type_as_enters(
        format!("As {display_subject} enters, choose a creature type."),
    )))
}

pub(crate) fn parse_choose_named_options_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS)
    else {
        return Ok(None);
    };
    let Some(choice_offset) =
        find_index(&words[idx..], |word| CHOOSE_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    let choice_idx = idx + choice_offset;
    let choice_words = &words[choice_idx..];
    if choice_words.len() < 4 || !CHOICE_OR_PATTERN.matches_words(choice_words) {
        return Ok(None);
    }
    if parse_choose_color_phrase_words(choice_words)?.is_some()
        || parse_choose_player_phrase_words(choice_words).is_some()
        || parse_choose_basic_land_type_phrase_words(choice_words).is_some()
        || parse_choose_land_type_phrase_words(choice_words).is_some()
        || parse_choose_creature_type_phrase_words(choice_words)?.is_some()
    {
        return Ok(None);
    }

    let mut options = Vec::new();
    let mut current = Vec::new();
    for word in choice_words.iter().skip(1) {
        if OR_WORD_PATTERN.matches_word(word) {
            if current.is_empty() {
                return Ok(None);
            }
            options.push(current.join(" "));
            current.clear();
        } else {
            current.push((*word).to_string());
        }
    }
    if current.is_empty() {
        return Ok(None);
    }
    options.push(current.join(" "));
    if options.len() < 2 {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_named_option_as_enters(
        options,
        format!("As {display_subject} enters, {}.", choice_words.join(" ")),
    )))
}

fn trigger_duplication_tail_matches(words: &[&str]) -> bool {
    TRIGGER_DUPLICATION_TAIL_PATTERN.matches_words(words)
}

fn parse_trigger_duplication_source_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let filter_words = crate::runtime_backend::token_word_refs(&tokens);
    if TRIGGER_DUPLICATION_SOURCE_PATTERN.matches_words(&filter_words) {
        let mut emblem = ObjectFilter::default();
        emblem.zone = Some(Zone::Command);
        emblem.owner = Some(PlayerFilter::You);

        let mut filter = ObjectFilter::default();
        filter.any_of = vec![ObjectFilter::source(), emblem];
        return Ok(filter);
    }

    parse_object_filter_with_grammar_entrypoint(&tokens, false)
}

fn parse_trigger_duplication_event_matcher(
    tokens: &[OwnedLexToken],
) -> Result<Trigger, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let phrase_words = crate::runtime_backend::token_word_refs(&tokens);

    let build_filter = |subject_tokens: &[OwnedLexToken]| -> Result<ObjectFilter, CardTextError> {
        parse_object_filter_with_grammar_entrypoint(&trim_edge_punctuation(subject_tokens), false)
    };

    if TURNING_FACE_UP_TRIGGER_DUPLICATION_PATTERN.matches_words(&phrase_words) {
        if tokens.len() <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing turned-face-up subject in trigger-duplication clause (clause: '{}')",
                phrase_words.join(" ")
            )));
        }
        let filter = build_filter(&tokens[1..tokens.len() - 2])?;
        return Ok(Trigger::turned_face_up(filter));
    }

    if YOU_CASTING_OR_COPYING_PREFIX_PATTERN.matches_words(&phrase_words) {
        if tokens.len() <= 4 {
            return Err(CardTextError::ParseError(format!(
                "missing spell subject in trigger-duplication clause (clause: '{}')",
                phrase_words.join(" ")
            )));
        }
        let filter = build_filter(&tokens[4..])?;
        return Ok(Trigger::either(
            Trigger::spell_cast_qualified(
                Some(filter.clone()),
                PlayerFilter::You,
                None,
                None,
                None,
                false,
            ),
            Trigger::spell_copied(Some(filter), PlayerFilter::You),
        ));
    }

    let suffixes: &[(&[&str], fn(ObjectFilter) -> Trigger)] = &[
        (
            &["dealing", "combat", "damage", "to", "a", "player"],
            |filter| Trigger::deals_combat_damage_to_player(filter, PlayerFilter::Any),
        ),
        (
            &[
                "becoming", "the", "target", "of", "a", "spell", "or", "ability",
            ],
            |filter| Trigger::becomes_targeted_object(filter),
        ),
        (&["being", "dealt", "damage"], |filter| {
            Trigger::is_dealt_damage(ChooseSpec::Object(filter))
        }),
        (
            &["entering", "or", "leaving", "the", "battlefield"],
            |filter| {
                Trigger::either(
                    Trigger::enters_battlefield(filter.clone(), None),
                    Trigger::leaves_battlefield(filter),
                )
            },
        ),
        (&["entering", "the", "battlefield"], |filter| {
            Trigger::enters_battlefield(filter, None)
        }),
        (&["leaving", "the", "battlefield"], |filter| {
            Trigger::leaves_battlefield(filter)
        }),
        (&["drawing", "a", "card"], |_filter| {
            Trigger::player_draws_card(PlayerFilter::Any)
        }),
        (&["attacking"], |filter| Trigger::attacks(filter)),
        (&["dying"], |filter| Trigger::dies(filter)),
        (&["entering"], |filter| {
            Trigger::enters_battlefield(filter, None)
        }),
    ];

    for (suffix, build) in suffixes {
        if !ClauseShape::new()
            .suffix(suffix)
            .matches_words(&phrase_words)
            || phrase_words.len() <= suffix.len()
        {
            continue;
        }
        let subject_len = phrase_words.len() - suffix.len();
        if DRAWING_A_CARD_SUFFIX_PATTERN.matches_words(suffix) {
            let subject_words = &phrase_words[..subject_len];
            if PLAYER_SUBJECT_PATTERN.matches_words(subject_words) {
                return Ok(Trigger::player_draws_card(PlayerFilter::Any));
            }
            if YOU_SUBJECT_PATTERN.matches_words(subject_words) {
                return Ok(Trigger::player_draws_card(PlayerFilter::You));
            }
            if OPPONENT_SUBJECT_PATTERN.matches_words(subject_words) {
                return Ok(Trigger::player_draws_card(PlayerFilter::Opponent));
            }
        }
        let Some(subject_end_token_idx) = token_index_for_word_index(&tokens, subject_len) else {
            return Err(CardTextError::ParseError(format!(
                "failed to split trigger-duplication subject (clause: '{}')",
                phrase_words.join(" ")
            )));
        };
        let filter = build_filter(&tokens[..subject_end_token_idx])?;
        return Ok(build(filter));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported trigger-duplication cause clause (clause: '{}')",
        phrase_words.join(" ")
    )))
}

fn parse_trigger_duplication_core(
    tokens: &[OwnedLexToken],
) -> Result<Option<(StaticAbility, Option<crate::ConditionExpr>)>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let segments = split_lexed_slices_on_comma(&tokens);
    if segments.len() != 2 {
        return Ok(None);
    }

    let head_tokens = trim_commas(segments[0]);
    let tail_tokens = trim_commas(segments[1]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
    if !trigger_duplication_tail_matches(&tail_words) {
        return Ok(None);
    }

    let head_words = crate::runtime_backend::token_word_refs(&head_tokens);
    if !IF_PREFIX_PATTERN.matches_words(&head_words) || head_tokens.len() < 2 {
        return Ok(None);
    }

    let body_tokens = &head_tokens[1..];
    let body_words = crate::runtime_backend::token_word_refs(body_tokens);

    let ability_prefixes: &[&[&str]] = &[
        &["a", "triggered", "ability", "of"],
        &["an", "ability", "of"],
    ];

    let mut parsed: Option<(
        Option<ObjectFilter>,
        Option<Trigger>,
        Option<crate::ConditionExpr>,
    )> = None;

    for prefix in ability_prefixes {
        if !ClauseShape::new().prefix(prefix).matches_words(&body_words)
            || body_tokens.len() <= prefix.len() + 1
        {
            continue;
        }
        let Some(triggers_idx) =
            find_index(&body_words, |word| TRIGGERS_WORD_PATTERN.matches_word(word))
        else {
            continue;
        };
        if triggers_idx <= prefix.len() {
            continue;
        }

        let condition = if body_words
            .get(triggers_idx + 1..)
            .is_some_and(|tail| WHILE_PREFIX_PATTERN.matches_words(tail))
        {
            Some(parse_static_condition_clause(
                &body_tokens[triggers_idx + 2..],
            )?)
        } else if triggers_idx + 1 == body_tokens.len() {
            None
        } else {
            continue;
        };

        let source_filter =
            parse_trigger_duplication_source_filter(&body_tokens[prefix.len()..triggers_idx])?;
        parsed = Some((Some(source_filter), None, condition));
        break;
    }

    if parsed.is_none()
        && let Some(causes_idx) =
            find_index(&body_words, |word| CAUSES_WORD_PATTERN.matches_word(word))
    {
        let cause_tokens = &body_tokens[..causes_idx];
        let source_body_tokens = &body_tokens[causes_idx + 1..];
        let source_words = crate::runtime_backend::token_word_refs(source_body_tokens);
        for prefix in ability_prefixes {
            if !ClauseShape::new()
                .prefix(prefix)
                .matches_words(&source_words)
                || source_body_tokens.len() <= prefix.len() + 2
            {
                continue;
            }
            if !TO_TRIGGER_SUFFIX_PATTERN.matches_words(&source_words) {
                continue;
            }
            let source_filter = parse_trigger_duplication_source_filter(
                &source_body_tokens[prefix.len()..source_body_tokens.len() - 2],
            )?;
            let event_matcher = parse_trigger_duplication_event_matcher(cause_tokens)?;
            parsed = Some((Some(source_filter), Some(event_matcher), None));
            break;
        }
    }

    Ok(parsed.map(|(source_filter, event_matcher, condition)| {
        (
            StaticAbility::duplicate_matching_triggered_abilities(
                source_filter,
                event_matcher,
                1,
                crate::runtime_backend::token_word_refs(&tokens).join(" "),
            ),
            condition,
        )
    }))
}

pub(crate) fn parse_trigger_duplication_line_ast(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if let Some(spec) = split_as_long_as_condition_prefix_lexed(&tokens) {
        let condition = parse_static_condition_clause(spec.condition_tokens)?;
        let Some(inner) = parse_trigger_duplication_line_ast(spec.remainder_tokens)? else {
            return Ok(None);
        };
        return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(inner),
            condition,
        }));
    }

    let Some((ability, condition)) = parse_trigger_duplication_core(&tokens)? else {
        return Ok(None);
    };
    let ast = StaticAbilityAst::Static(ability);
    Ok(Some(if let Some(condition) = condition {
        StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(ast),
            condition,
        }
    } else {
        ast
    }))
}

pub(crate) fn parse_trigger_suppression_line_ast(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(spec) = parse_trigger_suppression_spec_lexed(&tokens) else {
        return Ok(None);
    };
    let source_filter = match spec.source_filter_tokens {
        Some(source_filter_tokens) => Some(parse_trigger_duplication_source_filter(
            source_filter_tokens,
        )?),
        None => None,
    };
    let event_matcher = parse_trigger_duplication_event_matcher(spec.cause_tokens)?;
    let display = crate::runtime_backend::token_word_refs(&tokens).join(" ");

    Ok(Some(StaticAbilityAst::from(
        StaticAbility::suppress_matching_triggered_abilities(
            source_filter,
            Some(event_matcher),
            display,
        ),
    )))
}

pub(crate) fn parse_double_damage_from_sources_you_control_of_chosen_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_double_damage_from_sources_you_control_of_chosen_type_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::double_damage_from_sources_you_control_of_chosen_type(
            "Double all damage that sources you control of the chosen type would deal.".to_string(),
        ),
    ))
}

pub(crate) fn parse_damage_amount_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let words = parser_token_word_refs(&tokens);
    const DAMAGE_PLUS_REPLACEMENT_PREFIX_LEN: usize = 23;
    if !DAMAGE_PLUS_REPLACEMENT_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    let Some(delta_word) = words.get(DAMAGE_PLUS_REPLACEMENT_PREFIX_LEN) else {
        return Ok(None);
    };
    let Some(delta) = parse_number_word_i32(delta_word) else {
        return Ok(None);
    };
    if words.len() != DAMAGE_PLUS_REPLACEMENT_PREFIX_LEN + 2 {
        return Ok(None);
    }

    let display = format!(
        "If a source you control would deal damage to an opponent or a permanent an opponent controls, it deals that much damage plus {delta} instead."
    );
    Ok(Some(StaticAbility::modify_damage_amount_replacement(
        ObjectFilter::default().you_control(),
        Some(PlayerFilter::Opponent),
        Some(ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent)),
        delta,
        display,
    )))
}

pub(crate) fn parse_double_damage_amount_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let words = parser_token_word_refs(&tokens);
    if !IF_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let Some(would_idx) = find_window_by(&words, 4, |window| {
        WOULD_DEAL_DAMAGE_TO_PHRASE_PATTERN.matches_words(window)
    }) else {
        return Ok(None);
    };
    let Some(source_idx) = SOURCE_WORD_PATTERN.find_word(&words[..would_idx]) else {
        return Ok(None);
    };
    if source_idx <= 1 {
        return Ok(None);
    }

    let Some(tail_idx) = find_window_by(&words[would_idx + 4..], 6, |window| {
        IT_DEALS_MULTIPLE_THAT_DAMAGE_TO_PHRASE_PATTERN.matches_words(window)
    }) else {
        return Ok(None);
    };
    let replacement_start = would_idx + 4 + tail_idx;
    let factor = match words.get(replacement_start + 2).copied() {
        Some("double") => 2,
        Some("triple") => 3,
        _ => return Ok(None),
    };
    let damaged_words = &words[would_idx + 4..replacement_start];
    let replacement_target_words = &words[replacement_start + 6..];
    if damaged_words.is_empty()
        || replacement_target_words.len() < 2
        || !THAT_WORD_PATTERN.matches_first_word(replacement_target_words)
        || !INSTEAD_WORD_PATTERN.matches_last_word(replacement_target_words)
    {
        return Ok(None);
    }

    let (target_player_filter, target_object_filter) =
        parse_damage_amount_replacement_target_filters(damaged_words)?;
    if target_player_filter.is_none() && target_object_filter.is_none() {
        return Ok(None);
    }

    let source_tokens = trim_lexed_commas(
        LexedClause::new(&tokens)
            .between_word_range(1, source_idx)
            .unwrap_or_else(|| LexedClause::new(&tokens).between(tokens.len(), tokens.len()))
            .tokens(),
    );
    let source_words = parser_token_word_refs(source_tokens);
    let mut source_filter =
        if source_tokens.is_empty() || ARTICLE_WORD_PATTERN.matches_words(&source_words) {
            ObjectFilter::default()
        } else {
            parse_object_filter_lexed(source_tokens, false)?
        };

    match &words[source_idx + 1..would_idx] {
        ["you", "control"] => source_filter = source_filter.you_control(),
        ["an", "opponent", "controls"] | ["opponent", "controls"] => {
            source_filter = source_filter.controlled_by(PlayerFilter::Opponent);
        }
        [] => {}
        _ => return Ok(None),
    }

    let mut display = render_token_slice(&tokens).trim().to_string();
    if !display.ends_with('.') {
        display.push('.');
    }

    Ok(Some(StaticAbility::multiply_damage_amount_replacement(
        source_filter,
        target_player_filter,
        target_object_filter,
        factor,
        display,
    )))
}

fn parse_damage_amount_replacement_target_filters(
    words: &[&str],
) -> Result<(Option<PlayerFilter>, Option<ObjectFilter>), CardTextError> {
    #[derive(Clone, Copy)]
    enum DamageReplacementTargetKind {
        You,
        Opponent,
        AnyPlayer,
        EnchantedPlayer,
        Permanent,
        PermanentOrPlayer,
        OpponentOrPermanentOpponentControls,
    }

    const DAMAGE_REPLACEMENT_TARGET_PHRASES: &[(&[&str], DamageReplacementTargetKind)] = &[
        (&["you"], DamageReplacementTargetKind::You),
        (&["opponent"], DamageReplacementTargetKind::Opponent),
        (&["player"], DamageReplacementTargetKind::AnyPlayer),
        (
            &["enchanted", "player"],
            DamageReplacementTargetKind::EnchantedPlayer,
        ),
        (&["permanent"], DamageReplacementTargetKind::Permanent),
        (
            &["permanent", "or", "player"],
            DamageReplacementTargetKind::PermanentOrPlayer,
        ),
        (
            &["permanent", "or", "a", "player"],
            DamageReplacementTargetKind::PermanentOrPlayer,
        ),
        (
            &["player", "or", "permanent"],
            DamageReplacementTargetKind::PermanentOrPlayer,
        ),
        (
            &["player", "or", "a", "permanent"],
            DamageReplacementTargetKind::PermanentOrPlayer,
        ),
        (
            &[
                "opponent",
                "or",
                "a",
                "permanent",
                "an",
                "opponent",
                "controls",
            ],
            DamageReplacementTargetKind::OpponentOrPermanentOpponentControls,
        ),
        (
            &["opponent", "or", "permanent", "an", "opponent", "controls"],
            DamageReplacementTargetKind::OpponentOrPermanentOpponentControls,
        ),
    ];

    let words = strip_leading_word_refs_any(words, &["a", "an"]);
    let Some((_, kind)) = DAMAGE_REPLACEMENT_TARGET_PHRASES
        .iter()
        .find(|(phrase, _)| *phrase == words)
    else {
        return Ok((None, None));
    };

    match kind {
        DamageReplacementTargetKind::You => Ok((Some(PlayerFilter::You), None)),
        DamageReplacementTargetKind::Opponent => Ok((Some(PlayerFilter::Opponent), None)),
        DamageReplacementTargetKind::AnyPlayer => Ok((Some(PlayerFilter::Any), None)),
        DamageReplacementTargetKind::EnchantedPlayer => Ok((
            Some(PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted"))),
            None,
        )),
        DamageReplacementTargetKind::Permanent => Ok((None, Some(ObjectFilter::permanent()))),
        DamageReplacementTargetKind::PermanentOrPlayer => {
            Ok((Some(PlayerFilter::Any), Some(ObjectFilter::permanent())))
        }
        DamageReplacementTargetKind::OpponentOrPermanentOpponentControls => Ok((
            Some(PlayerFilter::Opponent),
            Some(ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent)),
        )),
    }
}

pub(crate) fn parse_minimum_damage_amount_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let words = parser_token_word_refs(&tokens);
    const MINIMUM_RED_NONCOMBAT_DAMAGE_PREFIX_LEN: usize = 15;
    if !MINIMUM_RED_NONCOMBAT_DAMAGE_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let Some(to_idx) = TO_WORD_PATTERN
        .find_word(&words[MINIMUM_RED_NONCOMBAT_DAMAGE_PREFIX_LEN..])
        .map(|offset| MINIMUM_RED_NONCOMBAT_DAMAGE_PREFIX_LEN + offset)
    else {
        return Ok(None);
    };
    if !AN_OPPONENT_PREFIX_PATTERN.matches_words(&words[to_idx + 1..]) {
        return Ok(None);
    }

    let Some(source_deals_idx) =
        keyword_find_prefix_shape_start(&words, &THAT_SOURCE_DEALS_DAMAGE_EQUAL_TO_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    if source_deals_idx <= to_idx {
        return Ok(None);
    }
    if !INSTEAD_WORD_PATTERN.matches_last_word(&words) {
        return Ok(None);
    }

    let floor_words = &words[MINIMUM_RED_NONCOMBAT_DAMAGE_PREFIX_LEN..to_idx];
    let replacement_floor_words = &words[source_deals_idx + 6..words.len() - 1];
    if !damage_floor_value_words_match(floor_words)
        || !damage_floor_value_words_match(replacement_floor_words)
    {
        return Ok(None);
    }

    let display = render_token_slice(&tokens);
    Ok(Some(StaticAbility::minimum_damage_amount_replacement(
        ObjectFilter::default()
            .you_control()
            .with_colors(ColorSet::from_color(Color::Red)),
        Some(PlayerFilter::Opponent),
        None,
        Value::SourcePower,
        true,
        display,
    )))
}

fn damage_floor_value_words_match(words: &[&str]) -> bool {
    SOURCE_POWER_VALUE_PATTERN.matches_words(words)
        || words.len() >= 2 && POWER_WORD_PATTERN.matches_last_word(words)
}

pub(crate) fn parse_enter_as_copy_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    fn parse_added_copy_abilities(
        tokens: &[OwnedLexToken],
        clause_words: &[&str],
        has_word_idx: usize,
    ) -> Result<Vec<crate::ability::Ability>, CardTextError> {
        let ability_start_token_idx = token_index_for_word_index(tokens, has_word_idx)
            .map(|idx| idx + 1)
            .unwrap_or(tokens.len());
        let ability_tokens = trim_commas(&tokens[ability_start_token_idx..]);
        if ability_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported empty enters-as-copy ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let (abilities, _choice) =
            parse_granted_abilities_for_gain_clause(&ability_tokens, clause_words, false)?;
        if abilities.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported enters-as-copy ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        lower_granted_abilities_ast_to_object_abilities(&abilities)
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words
        .first()
        .is_some_and(|word| AS_WORD_PATTERN.matches_word(word))
        && let Some(enter_idx) = find_index(&clause_words, |word| {
            ENTER_OR_ENTERS_WORD_PATTERN.matches_word(word)
        })
        && ENTER_AS_COPY_EXILE_TWO_CREATURE_CARDS_PATTERN
            .matches_words(&clause_words[enter_idx + 1..])
        && ENTER_AS_COPY_IF_YOU_DO_PATTERN.matches_words(&clause_words)
        && ENTER_AS_COPY_COUNTER_POWER_MARKER_PATTERN.matches_words(&clause_words)
    {
        return Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
            crate::static_abilities::EnterAsCopyAsEntersSpec {
                filter: ObjectFilter::creature().in_zone(Zone::Graveyard).nontoken(),
                affected_filter: None,
                may: true,
                enters_tapped_if_chosen: false,
                linked_exile_pair: Some(crate::static_abilities::EnterAsCopyLinkedExilePairSpec {
                    counter_type: CounterType::PlusOnePlusOne,
                }),
                copy_source_self: false,
                copy_source_enchanted: false,
                name_override: None,
                added_card_types: Vec::new(),
                removed_supertypes: Vec::new(),
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
                set_base_power_toughness: None,
                set_base_power_toughness_from_self: false,
            },
            render_token_slice(tokens).trim().to_string(),
        )));
    }
    if !YOU_MAY_HAVE_PREFIX_PATTERN.matches_words(&clause_words)
        && let Some(enter_idx) = find_index(&clause_words, |word| {
            ENTER_OR_ENTERS_WORD_PATTERN.matches_word(word)
        })
        && enter_idx > 0
    {
        let mut after_enter = enter_idx + 1;
        if THE_WORD_PATTERN.matches_word_at(&clause_words, after_enter)
            && BATTLEFIELD_WORD_PATTERN.matches_word_at(&clause_words, after_enter + 1)
        {
            after_enter += 2;
        }
        if AS_A_COPY_OF_PREFIX_PATTERN.matches_words(&clause_words[after_enter..]) {
            let filter_end_token_idx =
                token_index_for_word_index(tokens, enter_idx).unwrap_or(tokens.len());
            let affected_tokens = trim_commas(&tokens[..filter_end_token_idx]);
            let affected_filter = parse_object_filter(&affected_tokens, false)?;
            let copy_source_words = &clause_words[after_enter + 4..];
            let (filter, copy_source_self, copy_source_enchanted) =
                if THIS_COPY_SOURCE_PREFIX_PATTERN.matches_words(copy_source_words) {
                    (ObjectFilter::source(), true, false)
                } else if ENCHANTED_COPY_SOURCE_PREFIX_PATTERN.matches_words(copy_source_words) {
                    (ObjectFilter::source(), false, true)
                } else {
                    let copy_start_word_idx = after_enter + 4;
                    let copy_start_token_idx =
                        token_index_for_word_index(tokens, copy_start_word_idx)
                            .unwrap_or(tokens.len());
                    let copy_tokens = trim_commas(&tokens[copy_start_token_idx..]);
                    (parse_object_filter(&copy_tokens, false)?, false, false)
                };

            return Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
                crate::static_abilities::EnterAsCopyAsEntersSpec {
                    filter,
                    affected_filter: Some(affected_filter),
                    may: false,
                    enters_tapped_if_chosen: false,
                    linked_exile_pair: None,
                    copy_source_self,
                    copy_source_enchanted,
                    name_override: None,
                    added_card_types: Vec::new(),
                    removed_supertypes: Vec::new(),
                    added_subtypes: Vec::new(),
                    added_abilities: Vec::new(),
                    set_base_power_toughness: None,
                    set_base_power_toughness_from_self: false,
                },
                clause_words.join(" "),
            )));
        }
    }

    if clause_words.len() < 11 || !YOU_MAY_HAVE_PREFIX_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }

    let mut idx = 3usize;
    let mut named_copy_subject: Option<String> = None;
    if THIS_WORD_PATTERN.matches_word_at(&clause_words, idx) {
        idx += 1;
    } else if let Some(enter_idx) = find_index(&clause_words[idx..], |word| {
        ENTER_OR_ENTERS_WORD_PATTERN.matches_word(word)
    }) {
        named_copy_subject = Some(
            clause_words[idx..idx + enter_idx]
                .iter()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first) => {
                            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
                        }
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        );
        idx += enter_idx;
    } else {
        return Ok(None);
    }
    if clause_words.get(idx).is_some_and(|word| {
        matches!(
            *word,
            "land" | "creature" | "artifact" | "enchantment" | "permanent"
        )
    }) {
        idx += 1;
    }

    if !clause_words
        .get(idx)
        .is_some_and(|word| ENTER_OR_ENTERS_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    idx += 1;

    if THE_WORD_PATTERN.matches_word_at(&clause_words, idx)
        && BATTLEFIELD_WORD_PATTERN.matches_word_at(&clause_words, idx + 1)
    {
        idx += 2;
    }

    let mut enters_tapped_if_chosen = false;
    if TAPPED_WORD_PATTERN.matches_word_at(&clause_words, idx) {
        enters_tapped_if_chosen = true;
        idx += 1;
    }

    if !AS_A_COPY_OF_PREFIX_PATTERN.matches_words(&clause_words[idx..]) {
        return Ok(None);
    }
    idx += 4;

    let except_idx = find_index(&clause_words, |word| EXCEPT_WORD_PATTERN.matches_word(word));
    let filter_end_word_idx = except_idx.unwrap_or(clause_words.len());
    let filter_tokens = trim_commas(
        LexedClause::new(tokens)
            .between_word_range(idx, filter_end_word_idx)
            .unwrap_or_else(|| LexedClause::new(tokens).between(tokens.len(), tokens.len()))
            .tokens(),
    );
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&filter_tokens, false)?;

    let mut name_override = None;
    let mut added_card_types = Vec::new();
    let mut removed_supertypes = Vec::new();
    let mut added_subtypes = Vec::new();
    let mut added_abilities = Vec::new();
    let mut set_base_power_toughness = None;
    let mut set_base_power_toughness_from_self = false;
    if let Some(except_idx) = except_idx {
        let tail = &clause_words[except_idx + 1..];
        if tail.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported enters-as-copy exception clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        if ITS_NAME_IS_PREFIX_PATTERN.matches_words(tail) {
            let mut name_words = Vec::new();
            for word in &tail[3..] {
                if COPY_NAME_BOUNDARY_WORD_PATTERN.matches_word(word) {
                    break;
                }
                name_words.push(*word);
            }
            if THIS_COPY_SOURCE_PREFIX_PATTERN.matches_words(&name_words) {
                name_override = named_copy_subject.clone();
            } else if !name_words.is_empty() {
                name_override = Some(name_words.join(" "));
            }
            // Name/legendary exception riders such as Sakashima's do not change
            // the copied object's functional abilities for current engine use.
        } else if IT_HAS_PREFIX_PATTERN.matches_words(tail) {
            added_abilities = parse_added_copy_abilities(tokens, &clause_words, except_idx + 2)?;
        } else {
            let mut tail_idx = 0usize;
            if NOT_LEGENDARY_COPY_EXCEPTION_PREFIX_PATTERN.matches_words(tail) {
                removed_supertypes.push(crate::types::Supertype::Legendary);
                tail_idx = if ISNT_WORD_PATTERN.matches_word_at(tail, 1) {
                    3
                } else {
                    4
                };
            }

            let tail_word_offset = tail_idx;
            let tail = &tail[tail_idx..];
            let type_idx = if ITS_WORD_PATTERN.matches_first_word(tail)
                && ARTICLE_WORD_PATTERN.matches_word_at(tail, 1)
            {
                2usize
            } else if IS_WORD_PATTERN.matches_first_word(tail)
                && ARTICLE_WORD_PATTERN.matches_word_at(tail, 1)
            {
                2usize
            } else if IT_IS_OR_ITS_PREFIX_PATTERN.matches_words(tail)
                && ARTICLE_WORD_PATTERN.matches_word_at(tail, 2)
            {
                3usize
            } else if IT_APOSTROPHE_S_WORD_PATTERN.matches_first_word(tail)
                && ARTICLE_WORD_PATTERN.matches_word_at(tail, 1)
            {
                2usize
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported enters-as-copy exception clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            };

            let mut cursor = type_idx;
            if let Ok((power, toughness)) = parse_pt_modifier(tail[cursor]) {
                set_base_power_toughness = Some((power, toughness));
                cursor += 1;
            }

            let mut parsed_type_or_subtype = false;
            while cursor < tail.len() {
                if let Some(card_type) = parse_card_type(tail[cursor]) {
                    crate::slice_primitives::push_unique(&mut added_card_types, card_type);
                    parsed_type_or_subtype = true;
                    cursor += 1;
                    continue;
                }
                if let Some(subtype) = parse_subtype_word(tail[cursor])
                    .or_else(|| parse_subtype_flexible(tail[cursor]))
                {
                    crate::slice_primitives::push_unique(&mut added_subtypes, subtype);
                    parsed_type_or_subtype = true;
                    cursor += 1;
                    continue;
                }
                break;
            }

            if !parsed_type_or_subtype && set_base_power_toughness.is_none() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported enters-as-copy type '{}' (clause: '{}')",
                    tail[type_idx],
                    clause_words.join(" ")
                )));
            }

            let mut remainder_start = cursor;
            if IN_ADDITION_TO_ITS_OTHER_CREATURE_TYPES_PREFIX_PATTERN
                .matches_words(&tail[remainder_start..])
            {
                remainder_start += 7;
            } else if IN_ADDITION_TO_ITS_OTHER_TYPES_PREFIX_PATTERN
                .matches_words(&tail[remainder_start..])
            {
                remainder_start += 6;
            }

            if !tail[remainder_start..].is_empty() {
                if COPY_POWER_TOUGHNESS_FROM_SELF_TAIL_PATTERN
                    .matches_words(&tail[remainder_start..])
                {
                    set_base_power_toughness_from_self = true;
                } else if !AND_IT_HAS_PREFIX_PATTERN.matches_words(&tail[remainder_start..]) {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported enters-as-copy exception clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                } else if HAS_WORD_PATTERN.matches_word_at(&tail[remainder_start..], 1) {
                    added_abilities = parse_added_copy_abilities(
                        tokens,
                        &clause_words,
                        except_idx + 1 + tail_word_offset + remainder_start + 1,
                    )?;
                } else {
                    added_abilities = parse_added_copy_abilities(
                        tokens,
                        &clause_words,
                        except_idx + 1 + tail_word_offset + remainder_start + 2,
                    )?;
                }
            }
        }
    }

    Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
        crate::static_abilities::EnterAsCopyAsEntersSpec {
            filter,
            affected_filter: None,
            may: true,
            enters_tapped_if_chosen,
            linked_exile_pair: None,
            copy_source_self: false,
            copy_source_enchanted: false,
            name_override,
            added_card_types,
            added_subtypes,
            added_abilities,
            set_base_power_toughness,
            set_base_power_toughness_from_self,
            removed_supertypes,
        },
        clause_words.join(" "),
    )))
}

pub(crate) fn parse_choose_color_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let Some((consumed, excluded_color_set)) = parse_choose_color_phrase_words(&words[idx..])?
    else {
        return Ok(None);
    };
    if idx + consumed != words.len() {
        return Ok(None);
    }

    let excluded = if let Some(color_set) = excluded_color_set {
        Some(color_from_color_set(color_set).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "ambiguous color choice in choose-color clause (clause: '{}')",
                words.join(" ")
            ))
        })?)
    } else {
        None
    };
    let display = match excluded {
        Some(color) => format!(
            "As {display_subject} enters, choose a color other than {}.",
            color.name().to_string()
        ),
        None => format!("As {display_subject} enters, choose a color."),
    };

    Ok(Some(StaticAbility::choose_color_as_enters(
        excluded, display,
    )))
}

pub(crate) fn parse_choose_color_as_becomes_attached_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 6 || !AS_THIS_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    let display_subject = match words.get(2).copied() {
        Some("equipment") => "this Equipment",
        Some("aura") => "this Aura",
        Some("permanent") => "this permanent",
        Some("artifact") => "this artifact",
        Some("enchantment") => "this enchantment",
        _ => return Ok(None),
    };
    if !BECOMES_ATTACHED_TO_TAIL_PATTERN.matches_words(&words[3..6]) {
        return Ok(None);
    }
    let Some(choose_idx) = words
        .iter()
        .position(|word| CHOOSE_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    if choose_idx <= 6 {
        return Ok(None);
    }
    let Some((consumed, excluded_color_set)) =
        parse_choose_color_phrase_words(&words[choose_idx..])?
    else {
        return Ok(None);
    };
    if choose_idx + consumed != words.len() {
        return Ok(None);
    }
    let excluded = if let Some(color_set) = excluded_color_set {
        Some(color_from_color_set(color_set).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "ambiguous color choice in choose-color attached clause (clause: '{}')",
                words.join(" ")
            ))
        })?)
    } else {
        None
    };
    if excluded.is_some() {
        return Err(CardTextError::ParseError(format!(
            "excluded color choices for attachment choices are not supported yet (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(Some(StaticAbility::choose_color_as_becomes_attached(
        format!("As {display_subject} becomes attached, choose a color."),
    )))
}

pub(crate) fn parse_choose_player_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let Some(consumed) = parse_choose_player_phrase_words(&words[idx..]) else {
        return Ok(None);
    };
    if idx + consumed != words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_player_as_enters(format!(
        "As {display_subject} enters, choose a player."
    ))))
}

pub(crate) fn parse_damage_redirect_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() != 19 {
        return Ok(None);
    }
    let matches = DAMAGE_REDIRECT_TO_SOURCE_PREFIX_PATTERN.matches_words(&words)
        && words
            .get(10)
            .is_some_and(|word| PERMANENT_OR_PERMANENTS_WORD_PATTERN.matches_word(word))
        && DAMAGE_REDIRECT_TO_SOURCE_TAIL_PATTERN.matches_words(&words[11..]);
    if matches {
        return Ok(Some(
            StaticAbility::redirect_damage_from_you_and_other_permanents_to_source(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_damage_redirect_to_source_controller_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !IF_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let Some(would_idx) = find_window_by(&words, 5, |window| {
        WOULD_DEAL_DAMAGE_TO_YOU_PHRASE_PATTERN.matches_words(window)
    }) else {
        return Ok(None);
    };
    let tail = &words[would_idx + 5..];
    if !IT_DEALS_DAMAGE_TO_ITS_CONTROLLER_INSTEAD_TAIL_PATTERN.matches_words(tail) {
        return Ok(None);
    }
    if would_idx <= 1 {
        return Ok(None);
    }

    let source_tokens = trim_lexed_commas(
        LexedClause::new(tokens)
            .between_word_range(1, would_idx)
            .unwrap_or_else(|| LexedClause::new(tokens).between(tokens.len(), tokens.len()))
            .tokens(),
    );
    let source_filter = parse_object_filter_lexed(source_tokens, false)?;
    let mut display = render_token_slice(tokens).trim().to_string();
    if !display.ends_with('.') {
        display.push('.');
    }

    Ok(Some(StaticAbility::redirect_damage_to_source_controller(
        source_filter,
        PlayerFilter::You,
        display,
    )))
}

pub(crate) fn parse_no_more_than_creatures_can_attack_or_block_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::runtime_backend::token_word_refs(tokens);
    if line_words.len() < 8 {
        return Ok(None);
    }

    let Some((maximum, used)) = parse_less_than_or_equal_quantity_prefix(
        tokens,
        false,
        false,
        "combat maximum restriction",
    )
    .ok()
    .flatten() else {
        return Ok(None);
    };

    let tail = crate::runtime_backend::token_word_refs(&tokens[used..]);
    let ability = if CREATURES_CAN_ATTACK_YOU_EACH_COMBAT_TAIL_PATTERN.matches_words(&tail) {
        StaticAbility::max_attackers_can_attack_you_each_combat(maximum as usize)
    } else if CREATURES_CAN_ATTACK_EACH_COMBAT_TAIL_PATTERN.matches_words(&tail) {
        StaticAbility::max_attackers_each_combat(maximum as usize)
    } else if CREATURES_CAN_BLOCK_EACH_COMBAT_TAIL_PATTERN.matches_words(&tail) {
        StaticAbility::max_blockers_each_combat(maximum as usize)
    } else {
        return Ok(None);
    };
    Ok(Some(ability))
}

pub(crate) fn parse_characteristic_defining_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if split_lexed_sentences(tokens).len() > 1 {
        return Ok(None);
    }

    let line_words = crate::runtime_backend::token_word_refs(tokens);
    let has_pt_axes = CHARACTERISTIC_POWER_TOUGHNESS_PATTERN.matches_words(&line_words);
    if has_pt_axes
        && CHARACTERISTIC_EQUAL_TO_PATTERN.matches_words(&line_words)
        && let Some(equal_word_idx) =
            keyword_find_prefix_shape_start(&line_words, &EQUAL_TO_PREFIX_PATTERN)
    {
        let start_word_idx = equal_word_idx + 2;
        if let Some(start_token_idx) = token_index_for_word_index(tokens, start_word_idx) {
            let mut tail_tokens = &tokens[start_token_idx..];
            while tail_tokens.last().is_some_and(|token| {
                RESPECTIVELY_WORD_PATTERN.matches_token(token) || token.is_period()
            }) {
                tail_tokens = &tail_tokens[..tail_tokens.len().saturating_sub(1)];
            }
            if !tail_tokens.is_empty() {
                let value =
                    parse_characteristic_defining_stat_value(tail_tokens).ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported characteristic defining P/T value (value: '{}')",
                            crate::runtime_backend::token_word_refs(tail_tokens).join(" ")
                        ))
                    })?;
                return Ok(Some(StaticAbility::characteristic_defining_pt(
                    value.clone(),
                    value,
                )));
            }
        }
    }

    let mut parsed_power: Option<Value> = None;
    let mut parsed_toughness: Option<Value> = None;
    let mut previous_value: Option<Value> = None;
    let mut idx = 0usize;
    while idx < line_words.len() {
        let Some((axis, value_start_word_idx)) =
            parse_characteristic_axis_clause_start(&line_words, idx)
        else {
            idx += 1;
            continue;
        };

        let mut value_end_word_idx = line_words.len();
        let mut next_clause_word_idx = None;
        for and_idx in value_start_word_idx..line_words.len() {
            if !AND_WORD_PATTERN.matches_word(line_words[and_idx]) {
                continue;
            }
            if let Some((_next_axis, _)) =
                parse_characteristic_axis_clause_start(&line_words, and_idx + 1)
            {
                value_end_word_idx = and_idx;
                next_clause_word_idx = Some(and_idx + 1);
                break;
            }
        }

        let Some(value_start_token_idx) = token_index_for_word_index(tokens, value_start_word_idx)
        else {
            break;
        };
        let value_end_token_idx = if value_end_word_idx < line_words.len() {
            token_index_for_word_index(tokens, value_end_word_idx).unwrap_or(tokens.len())
        } else {
            tokens.len()
        };
        let value_tokens =
            trim_edge_punctuation(&tokens[value_start_token_idx..value_end_token_idx]);
        if value_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing characteristic defining {} value (line: '{}')",
                axis,
                line_words.join(" ")
            )));
        }

        let value = parse_characteristic_defining_stat_value(&value_tokens)
            .or_else(|| {
                previous_value.as_ref().and_then(|base| {
                    parse_characteristic_defining_relative_value(&value_tokens, base)
                })
            })
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported characteristic defining {} value (value: '{}')",
                    axis,
                    crate::runtime_backend::token_word_refs(&value_tokens).join(" ")
                ))
            })?;

        match axis {
            "power" => parsed_power = Some(value.clone()),
            "toughness" => parsed_toughness = Some(value.clone()),
            _ => {}
        }
        previous_value = Some(value);

        if let Some(next_idx) = next_clause_word_idx {
            idx = next_idx;
        } else {
            break;
        }
    }

    if parsed_power.is_none() && parsed_toughness.is_none() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::characteristic_defining_pt(
        parsed_power.unwrap_or(Value::SourcePower),
        parsed_toughness.unwrap_or(Value::SourceToughness),
    )))
}

fn parse_characteristic_defining_relative_value(
    tokens: &[OwnedLexToken],
    base: &Value,
) -> Option<Value> {
    let trimmed = trim_edge_punctuation(tokens);
    let words = crate::runtime_backend::token_word_refs(&trimmed);
    if !THAT_NUMBER_PREFIX_PATTERN.matches_words(&words) {
        return None;
    }
    if words.len() == 2 {
        return Some(base.clone());
    }
    if words.len() == 4 && PLUS_WORD_PATTERN.matches_word(words[2]) {
        let (amount, used) = parse_number(&trimmed[3..])?;
        if used == trimmed[3..].len() {
            return Some(Value::Add(
                Box::new(base.clone()),
                Box::new(Value::Fixed(amount as i32)),
            ));
        }
    }
    None
}

fn parse_characteristic_axis_clause_start<'a>(
    words: &'a [&'a str],
    idx: usize,
) -> Option<(&'a str, usize)> {
    let first = words.get(idx).copied()?;
    if POWER_OR_TOUGHNESS_WORD_PATTERN.matches_word(first)
        && words
            .get(idx + 1..idx + 4)
            .is_some_and(|tail| IS_EQUAL_TO_TAIL_PATTERN.matches_words(tail))
    {
        return Some((first, idx + 4));
    }

    if CREATURE_WORD_PATTERN.matches_word(first)
        && words
            .get(idx + 1)
            .is_some_and(|word| POWER_OR_TOUGHNESS_WORD_PATTERN.matches_word(word))
        && words
            .get(idx + 2..idx + 5)
            .is_some_and(|tail| IS_EQUAL_TO_TAIL_PATTERN.matches_words(tail))
    {
        return Some((words[idx + 1], idx + 5));
    }

    if !SELF_REFERENCE_WORD_PATTERN.matches_word(first) {
        return None;
    }

    if words
        .get(idx + 1)
        .is_some_and(|word| POWER_OR_TOUGHNESS_WORD_PATTERN.matches_word(word))
        && words
            .get(idx + 2..idx + 5)
            .is_some_and(|tail| IS_EQUAL_TO_TAIL_PATTERN.matches_words(tail))
    {
        return Some((words[idx + 1], idx + 5));
    }

    if words
        .get(idx + 1)
        .is_some_and(|word| CREATURE_WORD_PATTERN.matches_word(word))
        && words
            .get(idx + 2)
            .is_some_and(|word| POWER_OR_TOUGHNESS_WORD_PATTERN.matches_word(word))
        && words
            .get(idx + 3..idx + 6)
            .is_some_and(|tail| IS_EQUAL_TO_TAIL_PATTERN.matches_words(tail))
    {
        return Some((words[idx + 2], idx + 6));
    }

    None
}

fn parse_characteristic_defining_stat_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let trimmed = trim_edge_punctuation(tokens);
    let trimmed_words = crate::runtime_backend::token_word_refs(&trimmed);
    if trimmed_words.is_empty() {
        return None;
    }

    if SOURCE_POWER_VALUE_PATTERN.matches_words(&trimmed_words) {
        return Some(Value::SourcePower);
    }
    if SOURCE_TOUGHNESS_VALUE_PATTERN.matches_words(&trimmed_words) {
        return Some(Value::SourceToughness);
    }

    let mut equal_prefixed = Vec::with_capacity(trimmed.len() + 2);
    equal_prefixed.push(OwnedLexToken::word(
        "equal".to_string(),
        TextSpan::synthetic(),
    ));
    equal_prefixed.push(OwnedLexToken::word("to".to_string(), TextSpan::synthetic()));
    equal_prefixed.extend(trimmed.iter().cloned());

    parse_equal_to_aggregate_filter_value(&equal_prefixed)
        .or_else(|| parse_add_mana_equal_amount_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_filter_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(&equal_prefixed))
        .or_else(|| parse_characteristic_defining_pt_value(&trimmed))
}

pub(crate) fn parse_characteristic_defining_pt_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.is_empty() {
        return None;
    }

    let plus_positions: Vec<usize> = words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| PLUS_WORD_PATTERN.matches_word(word).then_some(idx))
        .collect();
    if plus_positions.is_empty() {
        return parse_characteristic_defining_pt_term(tokens);
    }

    let mut values = Vec::new();
    let mut start_word_idx = 0usize;
    let clause = LexedClause::new(tokens);
    for plus_word_idx in plus_positions {
        values.push(parse_characteristic_defining_pt_term(
            clause
                .between_word_range(start_word_idx, plus_word_idx)?
                .tokens(),
        )?);
        start_word_idx = plus_word_idx + 1;
    }
    values.push(parse_characteristic_defining_pt_term(
        clause.from_word(start_word_idx)?.tokens(),
    )?);

    let mut iter = values.into_iter();
    let mut acc = iter.next()?;
    for value in iter {
        acc = Value::Add(Box::new(acc), Box::new(value));
    }
    Some(acc)
}

pub(crate) fn parse_characteristic_defining_pt_term(tokens: &[OwnedLexToken]) -> Option<Value> {
    if tokens.is_empty() {
        return None;
    }

    if let Some((number, used)) = parse_number(tokens) {
        if tokens.len() == used {
            return Some(Value::Fixed(number as i32));
        }
    }

    let mut start = tokens;
    while start
        .first()
        .is_some_and(|token| token.as_word().is_some_and(is_article))
    {
        start = &start[1..];
    }
    if start.is_empty() {
        return None;
    }

    let initial_start_words = crate::runtime_backend::token_word_refs(start);
    if NUMBER_OF_PREFIX_PATTERN.matches_words(&initial_start_words) {
        start = &start[2..];
    }
    if start.is_empty() {
        return None;
    }

    // "the number of cards in the hand of the opponent with the most cards in hand"
    // (Adamaro, First to Desire)
    if let Some(value) = parse_max_cards_in_hand_value_lexed(start) {
        return Some(value);
    }

    let start_words = crate::runtime_backend::token_word_refs(start);
    if BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(&start_words) {
        let lands_token_idx = token_index_for_word_index(start, 4)?;
        let lands_tokens = trim_commas(&start[lands_token_idx..]);
        if let Ok(filter) = parse_object_filter(&lands_tokens, false) {
            return Some(Value::BasicLandTypesAmong(filter));
        }
    }

    if COLORS_AMONG_PREFIX_PATTERN.matches_words(&start_words) {
        let mut scope_word_idx = 2usize;
        if THE_WORD_PATTERN.matches_word_at(&start_words, scope_word_idx) {
            scope_word_idx += 1;
        }
        let scope_token_idx = token_index_for_word_index(start, scope_word_idx)?;
        let scope_tokens = trim_commas(&start[scope_token_idx..]);
        if let Ok(filter) = parse_object_filter(&scope_tokens, false) {
            return Some(Value::ColorsAmong(filter));
        }
    }

    if CARD_TYPES_AMONG_PREFIX_PATTERN.matches_words(&start_words) {
        let mut scope_word_idx = 3usize;
        if THE_WORD_PATTERN.matches_word_at(&start_words, scope_word_idx) {
            scope_word_idx += 1;
        }
        let scope_token_idx = token_index_for_word_index(start, scope_word_idx)?;
        let scope_tokens = trim_commas(&start[scope_token_idx..]);
        if let Ok(filter) = parse_object_filter(&scope_tokens, false)
            && filter.zone == Some(Zone::Graveyard)
        {
            let player = match filter.owner.clone() {
                Some(player) => player,
                None if !filter.single_graveyard => PlayerFilter::Any,
                None => PlayerFilter::You,
            };
            return Some(Value::CardTypesInGraveyard(player));
        }
    }

    let filter = parse_object_filter(start, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_shuffle_into_library_from_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_shuffle_into_library_from_graveyard_line_lexed(tokens) {
        return Ok(Some(StaticAbility::shuffle_into_library_from_graveyard()));
    }

    Ok(None)
}

pub(crate) fn parse_permanents_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_permanents_enter_tapped_line_lexed(tokens) {
        return Ok(Some(StaticAbility::permanents_enter_tapped()));
    }
    Ok(None)
}

pub(crate) fn parse_creatures_entering_dont_cause_abilities_to_trigger_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_creatures_entering_dont_cause_abilities_to_trigger_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::creatures_entering_dont_cause_abilities_to_trigger(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_creatures_assign_combat_damage_using_toughness_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    match parse_creatures_assign_combat_damage_using_toughness_line_lexed(tokens) {
        Some(CombatDamageUsingToughnessSubject::ThisCreature) => {
            return Ok(Some(
                StaticAbility::this_creature_assigns_combat_damage_using_toughness(),
            ));
        }
        Some(CombatDamageUsingToughnessSubject::EachCreature) => {
            return Ok(Some(
                StaticAbility::creatures_assign_combat_damage_using_toughness(),
            ));
        }
        Some(CombatDamageUsingToughnessSubject::EachCreatureYouControl) => {
            return Ok(Some(
                StaticAbility::creatures_you_control_assign_combat_damage_using_toughness(),
            ));
        }
        None => {}
    }
    Ok(None)
}

pub(crate) fn parse_lethal_damage_to_creatures_you_control_uses_power_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_lethal_damage_to_creatures_you_control_uses_power_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::lethal_damage_to_creatures_you_control_uses_power(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_players_cant_cycle_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_players_cant_cycle_line_lexed(tokens) {
        return Ok(Some(StaticAbility::players_cant_cycle()));
    }
    Ok(None)
}

pub(crate) fn parse_starting_life_bonus_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !YOU_START_THE_GAME_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    if !ADDITIONAL_LIFE_MARKER_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    let mut amount = None;
    for (idx, _token) in tokens.iter().enumerate() {
        if let Some((value, _)) = parse_number(&tokens[idx..]) {
            amount = Some(value);
            break;
        }
    }
    let amount = amount
        .ok_or_else(|| CardTextError::ParseError("missing starting life amount".to_string()))?;
    Ok(Some(StaticAbility::starting_life_bonus(amount as i32)))
}

pub(crate) fn parse_buyback_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !BUYBACK_COSTS_COST_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    let (amount, _) = parse_number(&tokens[3..])
        .ok_or_else(|| CardTextError::ParseError("missing buyback reduction amount".to_string()))?;
    if !words
        .iter()
        .any(|word| LESS_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    Ok(Some(StaticAbility::buyback_cost_reduction(amount)))
}

pub(crate) fn parse_spell_cost_increase_per_target_beyond_first_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let line_start = if THIS_SPELL_COSTS_PREFIX_PATTERN.matches_words(&words) {
        0
    } else if let Some(idx) =
        keyword_find_prefix_shape_start(&words, &THIS_SPELL_COSTS_PREFIX_PATTERN)
    {
        idx
    } else {
        return Ok(None);
    };
    if !TARGET_BEYOND_MORE_MARKER_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let search_tokens = &tokens[line_start..];
    let costs_idx = find_index(search_tokens, |token| {
        COST_OR_COSTS_WORD_PATTERN.matches_token(token)
    })
    .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = &search_tokens[costs_idx + 1..];
    if let Some((cost, used)) = parse_cost_modifier_mana_cost(amount_tokens)
        && amount_tokens
            .get(used)
            .is_some_and(|token| MORE_WORD_PATTERN.matches_token(token))
    {
        return Ok(Some(
            StaticAbility::cost_increase_mana_cost_per_target_beyond_first(cost),
        ));
    }
    let (amount_value, _) =
        parse_cost_modifier_amount(amount_tokens).unwrap_or((Value::Fixed(1), 0));
    let amount = if let Value::Fixed(v) = amount_value {
        v.max(0) as u32
    } else {
        1
    };

    Ok(Some(StaticAbility::cost_increase_per_target_beyond_first(
        amount,
    )))
}

#[allow(dead_code)]
pub(crate) fn parse_if_this_spell_costs_less_to_cast_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if !IF_PREFIX_PATTERN.matches_words(&words_all) {
        return Ok(None);
    }

    let Some(comma_idx) = find_index(tokens, |t| t.is_comma()) else {
        return Ok(None);
    };
    let condition_tokens = trim_commas(&tokens[1..comma_idx]);
    let tail_tokens = trim_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
    if !THIS_SPELL_COSTS_PREFIX_PATTERN.matches_words(&tail_words) {
        return Ok(None);
    }

    let condition = parse_this_spell_cost_condition(&condition_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let costs_idx = find_index(&tail_tokens, |token: &OwnedLexToken| {
        COST_OR_COSTS_WORD_PATTERN.matches_token(token)
    })
    .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = tail_tokens.get(costs_idx + 1..).unwrap_or_default();
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (amount_value, used) = parsed_amount
        .clone()
        .unwrap_or_else(|| (Value::Fixed(0), 0));
    let used = if used > 0 {
        used
    } else if let Some((_, used)) = parsed_mana_cost {
        used
    } else {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    };
    let remaining_words =
        crate::runtime_backend::token_word_refs(amount_tokens.get(used..).unwrap_or_default());
    if parse_cost_modifier_direction(&remaining_words) != Some(CostModifierDirection::Less)
        || !CAST_WORD_MARKER_PATTERN.matches_words(&remaining_words)
    {
        return Ok(None);
    }

    if let Some((reduction, _)) = parsed_mana_cost {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(reduction, condition),
        )));
    }

    Ok(Some(StaticAbility::new(
        crate::static_abilities::ThisSpellCostReduction::new(amount_value, condition),
    )))
}

pub(crate) fn parse_if_this_spell_costs_less_to_cast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = split_if_this_spell_costs_line_lexed(tokens) else {
        return Ok(None);
    };

    let condition = parse_this_spell_cost_condition(spec.condition_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;

    let costs_idx = find_index(spec.tail_tokens, |token| {
        COST_OR_COSTS_WORD_PATTERN.matches_token(token)
    })
    .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = spec.tail_tokens.get(costs_idx + 1..).unwrap_or_default();
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (amount_value, used) = parsed_amount
        .clone()
        .unwrap_or_else(|| (Value::Fixed(0), 0));
    let used = if used > 0 {
        used
    } else if let Some((_, used)) = parsed_mana_cost {
        used
    } else {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    };
    let remaining_words = parser_token_word_refs(amount_tokens.get(used..).unwrap_or_default());
    if parse_cost_modifier_direction(&remaining_words) != Some(CostModifierDirection::Less)
        || !CAST_WORD_MARKER_PATTERN.matches_words(&remaining_words)
    {
        return Ok(None);
    }

    if let Some((reduction, _)) = parsed_mana_cost {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(reduction, condition),
        )));
    }

    Ok(Some(StaticAbility::new(
        crate::static_abilities::ThisSpellCostReduction::new(amount_value, condition),
    )))
}

pub(crate) fn parse_this_spell_target_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;

    let w = crate::runtime_backend::token_word_refs(tokens);
    let target_start = if IT_TARGETS_PREFIX_PATTERN.matches_words(&w) {
        2
    } else if THIS_SPELL_TARGETS_PREFIX_PATTERN.matches_words(&w) {
        3
    } else {
        return None;
    };
    let target_tokens = trim_commas(tokens.get(target_start..).unwrap_or_default());
    if target_tokens.is_empty() {
        return None;
    }
    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if YOU_TARGET_PREFIX_PATTERN.matches_words(&target_words) {
        return Some(ThisSpellCostCondition::TargetsPlayer(PlayerFilter::You));
    }
    if OPPONENT_TARGET_PREFIX_PATTERN.matches_words(&target_words) {
        return Some(ThisSpellCostCondition::TargetsPlayer(
            PlayerFilter::Opponent,
        ));
    }
    if PLAYER_TARGET_PREFIX_PATTERN.matches_words(&target_words) {
        return Some(ThisSpellCostCondition::TargetsPlayer(PlayerFilter::Any));
    }
    parse_object_filter(&target_tokens, false)
        .ok()
        .map(ThisSpellCostCondition::TargetsObject)
}

pub(crate) fn parse_this_spell_cost_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;

    let w = crate::runtime_backend::token_word_refs(tokens);
    if w.is_empty() {
        return None;
    }

    if let Some(condition) = parse_life_total_or_less_spell_cost_condition(tokens, &w) {
        return Some(condition);
    }
    if LIFE_TOTAL_LESS_THAN_STARTING_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::LifeTotalLessThanStarting);
    }

    if YOU_ATTACKED_THIS_TURN_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::AttackedThisTurn,
            display: w.join(" "),
        });
    }
    if CREATURE_DIED_THIS_TURN_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::CreatureDiedThisTurn,
            display: w.join(" "),
        });
    }
    if YOU_GAINED_LIFE_THIS_TURN_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(1));
    }
    if YOU_GAINED_PREFIX_PATTERN.matches_words(&w)
        && w.len() >= 7
        && LIFE_THIS_TURN_SUFFIX_PATTERN.matches_words(&w)
        && let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, 2)
        && LIFE_THIS_TURN_SUFFIX_PATTERN.matches_words(&w[rest_start..])
    {
        return Some(ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(n));
    }
    if ITS_NIGHT_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::IsNight);
    }
    if THIS_SPELL_BARGAINED_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::ThisSpellPaidLabel("Bargain".to_string()),
            display: w.join(" "),
        });
    }
    if YOU_SACRIFICED_ARTIFACT_THIS_TURN_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::YouSacrificedArtifactThisTurn);
    }
    if YOU_COMMITTED_CRIME_THIS_TURN_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::YouCommittedCrimeThisTurn);
    }
    if CREATURE_LEFT_BATTLEFIELD_UNDER_YOUR_CONTROL_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::CreatureLeftBattlefieldUnderYourControlThisTurn);
    }
    if YOU_CAST_ANOTHER_PREFIX_PATTERN.matches_words(&w)
        && THIS_TURN_SUFFIX_PATTERN.matches_words(&w)
    {
        let types = mentioned_instant_sorcery_card_types(&w);
        if !types.is_empty() {
            return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
                count: 1,
                card_types: types,
            });
        }
        return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
            count: 1,
            card_types: Vec::new(),
        });
    }
    if YOU_CAST_PREFIX_PATTERN.matches_words(&w) && THIS_TURN_SUFFIX_PATTERN.matches_words(&w) {
        let types = mentioned_instant_sorcery_card_types(&w);
        if !types.is_empty() {
            return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
                count: 1,
                card_types: types,
            });
        }
    }

    if NOT_STARTING_PLAYER_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::NotStartingPlayer);
    }
    if CREATURE_IS_ATTACKING_YOU_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::CreatureIsAttackingYou);
    }
    if CREATURE_CARD_PUT_INTO_GRAVEYARD_THIS_TURN_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::CreatureCardPutIntoYourGraveyardThisTurn);
    }
    if w.len() >= 11
        && THERE_ARE_PREFIX_PATTERN.matches_words(&w)
        && CARD_TYPES_GRAVEYARD_COUNT_PATTERN.matches_words(&w)
        && let Some((n, _)) = parse_static_at_least_quantity_at(tokens, 2)
    {
        return Some(ThisSpellCostCondition::DistinctCardTypesInYourGraveyardOrMore(n));
    }
    if YOU_HAVE_IN_YOUR_GRAVEYARD_PATTERN.matches_words(&w)
        && let Some((n, _)) = parse_static_at_least_quantity_at(tokens, 2)
    {
        let types = mentioned_instant_sorcery_card_types(&w);
        if !types.is_empty() {
            return Some(
                ThisSpellCostCondition::YouHaveCardsOfTypesInYourGraveyardOrMore {
                    count: n,
                    card_types: types,
                },
            );
        }
        return Some(ThisSpellCostCondition::YouHaveCardsInYourGraveyardOrMore(n));
    }
    if w.len() >= 7 && OPPONENT_HAS_PREFIX_PATTERN.matches_words(&w) {
        let count_start = count_start_for_optional_an_opponent_prefix(&w, 3)?;
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, count_start) {
            let tail = &w[rest_start..];
            if POISON_COUNTERS_TAIL_PATTERN.matches_words(tail) {
                return Some(ThisSpellCostCondition::OpponentHasPoisonCountersOrMore(n));
            }
            if CARDS_IN_OPPONENT_GRAVEYARD_TAIL_PATTERN.matches_words(tail) {
                return Some(ThisSpellCostCondition::OpponentHasCardsInGraveyardOrMore(n));
            }
        }
    }

    if THERE_ARE_NO_PREFIX_PATTERN.matches_words(&w)
        && IN_YOUR_HAND_SUFFIX_PATTERN.matches_words(&w)
    {
        let filter_tokens = trim_commas(tokens.get(3..).unwrap_or_default());
        if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
            return Some(ThisSpellCostCondition::NoCardsInHandMatching {
                filter,
                display: w.join(" "),
            });
        }
    }
    if let Some(name) = only_creature_cards_in_hand_named(&w) {
        return Some(ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(name));
    }

    if THERE_IS_PREFIX_PATTERN.matches_words(&w)
        && IN_YOUR_GRAVEYARD_SUFFIX_PATTERN.matches_words(&w)
    {
        let filter_tokens = trim_commas(tokens.get(2..).unwrap_or_default());
        if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
            return Some(ThisSpellCostCondition::CardInYourGraveyardMatching {
                filter,
                display: w.join(" "),
            });
        }
    }

    if TARGETS_BIG_CONTROLLED_CREATURE_STACK_OBJECT_PATTERN.matches_words(&w) {
        let mut protected = ObjectFilter::creature().you_control();
        protected.power = Some(crate::filter::Comparison::GreaterThanOrEqual(7));
        let mut stack_target = ObjectFilter::default();
        stack_target.zone = Some(Zone::Stack);
        stack_target.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
        stack_target.targets_object = Some(Box::new(protected));
        return Some(ThisSpellCostCondition::TargetsObject(stack_target));
    }

    if let Some(target_condition) = parse_this_spell_target_condition(tokens) {
        return Some(target_condition);
    }

    // an opponent has no cards in hand
    if OPPONENT_HAS_NO_CARDS_IN_HAND_PATTERN.matches_words(&w) {
        return Some(ThisSpellCostCondition::OpponentHasNoCardsInHand);
    }

    // an opponent controls seven or more lands
    if w.len() >= 7 && OPPONENT_CONTROLS_PREFIX_PATTERN.matches_words(&w) {
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, 3) {
            let tail = &w[rest_start..];
            if LANDS_TAIL_PATTERN.matches_words(tail) {
                return Some(ThisSpellCostCondition::OpponentControlsLandsOrMore(n));
            }
        }
    }

    // an opponent controls at least four more creatures than you
    if w.len() >= 10 && OPPONENT_CONTROLS_PREFIX_PATTERN.matches_words(&w) {
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, 3) {
            let tail = &w[rest_start..];
            if MORE_CREATURES_THAN_YOU_TAIL_PATTERN.matches_words(tail) {
                return Some(
                    ThisSpellCostCondition::OpponentControlsAtLeastNMoreCreaturesThanYou(n),
                );
            }
        }
    }

    // there are ten or more creature cards total in all graveyards
    if w.len() >= 12 && THERE_ARE_PREFIX_PATTERN.matches_words(&w) {
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, 2) {
            let tail = &w[rest_start..];
            if TOTAL_CREATURE_CARDS_IN_ALL_GRAVEYARDS_TAIL_PATTERN.matches_words(tail) {
                return Some(ThisSpellCostCondition::TotalCreatureCardsInAllGraveyardsOrMore(n));
            }
        }
    }

    // an opponent cast two or more spells this turn
    if w.len() >= 9 && OPPONENT_CAST_PREFIX_PATTERN.matches_words(&w) {
        let count_start = count_start_for_optional_an_opponent_prefix(&w, 3)?;
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, count_start) {
            let tail = &w[rest_start..];
            if SPELLS_THIS_TURN_TAIL_PATTERN.matches_words(tail) {
                return Some(ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(n));
            }
        }
    }

    // an opponent has drawn four or more cards this turn
    if w.len() >= 10 && OPPONENT_HAS_DRAWN_PREFIX_PATTERN.matches_words(&w) {
        let count_start = count_start_for_optional_an_opponent_prefix(&w, 4)?;
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, count_start) {
            let tail = &w[rest_start..];
            if CARDS_THIS_TURN_TAIL_PATTERN.matches_words(tail) {
                return Some(ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(n));
            }
        }
    }

    // you've been dealt damage by two or more creatures this turn
    if YOU_WERE_DEALT_DAMAGE_BY_PREFIX_PATTERN.matches_words(&w) && w.len() >= 11 {
        let count_start = if YOU_HAVE_PREFIX_PATTERN.matches_words(&w) {
            6
        } else {
            5
        };
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, count_start) {
            let tail = &w[rest_start..];
            if CREATURES_THIS_TURN_TAIL_PATTERN.matches_words(tail) {
                return Some(
                    ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(n),
                );
            }
        }
    }

    if ASSASSIN_OR_COMMANDER_COMBAT_DAMAGE_PATTERN.matches_words(&w) {
        return Some(
            ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
                Subtype::Assassin,
            ),
        );
    }

    if let Some(condition_expr) = parse_conjoined_this_spell_cost_condition(tokens) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: condition_expr,
            display: w.join(" "),
        });
    }

    if let Ok(condition_expr) = parse_static_condition_clause(tokens) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: condition_expr,
            display: w.join(" "),
        });
    }

    None
}

fn parse_conjoined_this_spell_cost_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let and_positions = words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| AND_WORD_PATTERN.matches_word(word).then_some(idx))
        .collect::<Vec<_>>();
    for and_word_idx in and_positions {
        let and_token_idx = token_index_for_word_index(tokens, and_word_idx)?;
        let left_tokens = trim_commas(&tokens[..and_token_idx]);
        let right_tokens = trim_commas(&tokens[and_token_idx + 1..]);
        if left_tokens.is_empty() || right_tokens.is_empty() {
            continue;
        }
        let Ok(left) = parse_static_condition_clause(&left_tokens) else {
            continue;
        };
        let right = parse_conjoined_this_spell_cost_condition(&right_tokens)
            .or_else(|| parse_static_condition_clause(&right_tokens).ok());
        if let Some(right) = right {
            return Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)));
        }
    }
    None
}

pub(crate) fn parse_trailing_this_spell_cost_condition(
    remaining_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<crate::static_abilities::ThisSpellCostCondition>, CardTextError> {
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let Some(if_idx) = find_index(&remaining_words, |word| {
        IF_PREFIX_PATTERN.matches_word(word)
    }) else {
        return Ok(None);
    };
    let condition_token_idx =
        token_index_for_word_index(remaining_tokens, if_idx + 1).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map this-spell cost condition (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let condition_tokens = trim_commas(&remaining_tokens[condition_token_idx..]);
    if condition_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing this-spell cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let Some(condition) = parse_this_spell_cost_condition(&condition_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    Ok(Some(condition))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CostModifierDirection {
    Less,
    More,
}

fn parse_cost_modifier_direction(words: &[&str]) -> Option<CostModifierDirection> {
    match (
        words
            .iter()
            .any(|word| LESS_WORD_PATTERN.matches_word(word)),
        words
            .iter()
            .any(|word| MORE_WORD_PATTERN.matches_word(word)),
    ) {
        (true, false) => Some(CostModifierDirection::Less),
        (false, true) => Some(CostModifierDirection::More),
        _ => None,
    }
}

fn parse_cost_modifier_target_spec(
    target_tokens: &[OwnedLexToken],
) -> Result<(Option<PlayerFilter>, Option<Box<ObjectFilter>>), CardTextError> {
    let target_words = crate::runtime_backend::token_word_refs(target_tokens);
    if YOU_TARGET_PREFIX_PATTERN.matches_words(&target_words) {
        return Ok((Some(PlayerFilter::You), None));
    }
    if OPPONENT_OR_OPPONENTS_TARGET_PREFIX_PATTERN.matches_words(&target_words) {
        return Ok((Some(PlayerFilter::Opponent), None));
    }
    if PLAYER_OR_PLAYERS_TARGET_PREFIX_PATTERN.matches_words(&target_words) {
        return Ok((Some(PlayerFilter::Any), None));
    }

    Ok((
        None,
        Some(Box::new(parse_object_filter(target_tokens, false)?)),
    ))
}

pub(crate) fn parse_cost_modifier_prefix_condition(
    tokens: &[OwnedLexToken],
    spells_token_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    let subject_end = spells_token_idx.min(tokens.len());
    let head_tokens = &tokens[..subject_end];
    let all_words = crate::runtime_backend::token_word_refs(tokens);

    if DURING_TURNS_OTHER_THAN_YOURS_PREFIX_PATTERN.matches_words(&all_words) {
        let subject_start = find_index(head_tokens, |token| token.is_comma())
            .map(|idx| idx + 1)
            .unwrap_or(5);
        return Ok((
            Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            subject_start,
        ));
    }

    if DURING_YOUR_TURN_PREFIX_PATTERN.matches_words(&all_words) {
        let subject_start = find_index(head_tokens, |token| token.is_comma())
            .map(|idx| idx + 1)
            .unwrap_or(3);
        return Ok((Some(crate::ConditionExpr::YourTurn), subject_start));
    }

    if AS_LONG_AS_PREFIX_PATTERN.matches_words(&all_words) {
        let subject_start = find_index(head_tokens, |token| token.is_comma())
            .map(|idx| idx + 1)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subject boundary in leading static condition clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
        if subject_start <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        let condition = match parse_static_condition_clause(&condition_tokens) {
            Ok(condition) => condition,
            Err(_) => {
                parse_source_tap_status_condition_lexed(&condition_tokens).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported static condition clause (clause: '{}')",
                        crate::runtime_backend::token_word_refs(&condition_tokens).join(" ")
                    ))
                })?
            }
        };
        return Ok((Some(condition), subject_start));
    }

    Ok((None, 0))
}

fn parse_optional_life_additional_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let additional_words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_starts_with(
        &additional_words,
        &["as", "an", "additional", "cost", "to", "cast"],
    ) {
        return Ok(None);
    }

    let Some(additional_spells_word_idx) = find_index(&additional_words, |word| {
        *word == "spell" || *word == "spells"
    }) else {
        return Ok(None);
    };
    let Some(cost_subject_start) = token_index_for_word_index(tokens, 6) else {
        return Ok(None);
    };
    let Some(additional_spells_idx) =
        token_index_for_word_index(tokens, additional_spells_word_idx)
    else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[cost_subject_start..additional_spells_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_spell_filter_with_grammar_entrypoint(&subject_tokens);
    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    if word_slice_contains_word(&subject_words, "permanent") {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }
    filter.cast_by = Some(PlayerFilter::You);

    let Some(pay_word_idx) = find_index(&additional_words, |word| *word == "pay") else {
        return Ok(None);
    };
    let payment_words = &additional_words[pay_word_idx + 1..];
    let Some(life_cost) = payment_words
        .first()
        .and_then(|word| parse_number_word_i32(word))
        .and_then(|amount| u32::try_from(amount).ok())
    else {
        return Ok(None);
    };
    if !word_slice_contains_word(&payment_words, "life") {
        return Ok(None);
    }

    let Some(those_spells_idx) =
        word_slice_find_phrase_start(&additional_words, &["those", "spells"])
    else {
        return Ok(None);
    };
    if !word_slice_contains_phrase(
        &additional_words[those_spells_idx..],
        &["paid", "life", "this", "way"],
    ) {
        return Ok(None);
    }
    let Some(costs_word_idx) = find_index(&additional_words[those_spells_idx..], |word| {
        *word == "cost" || *word == "costs"
    }) else {
        return Ok(None);
    };
    let costs_word_idx = those_spells_idx + costs_word_idx;
    let Some(costs_idx) = token_index_for_word_index(tokens, costs_word_idx) else {
        return Ok(None);
    };
    let amount_tokens = &tokens[costs_idx + 1..];
    let (_, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let Some((reduction, _)) = parsed_mana_cost else {
        return Ok(None);
    };
    let remaining_words = crate::runtime_backend::token_word_refs(amount_tokens);
    if parse_cost_modifier_direction(&remaining_words) != Some(CostModifierDirection::Less)
        || !word_slice_contains_word(&remaining_words, "cast")
    {
        return Ok(None);
    }

    let label_end = find_token_kind(tokens, TokenKind::Period)
        .map(|idx| idx + 1)
        .unwrap_or(costs_idx);
    let label = render_token_slice(&tokens[..label_end])
        .trim()
        .trim_end_matches('.')
        .to_string();
    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostReductionManaCost::new(filter, reduction)
            .with_optional_life_additional_cost(label, life_cost),
    )))
}

pub(crate) fn parse_spells_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some(ability) = parse_optional_life_additional_cost_reduction_line(tokens)? {
        return Ok(Some(ability));
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let Some(spells_token_idx) = find_index(tokens, |token| {
        SPELL_OR_SPELLS_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };

    if FIRST_SPELL_EACH_TURN_COST_MODIFIER_PATTERN.matches_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported first-spell-each-turn cost modifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (prefix_condition, subject_start) =
        parse_cost_modifier_prefix_condition(tokens, spells_token_idx)?;
    if subject_start > spells_token_idx {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[subject_start..spells_token_idx]);
    let is_this_spell = is_this_subject_reference_lexed(&subject_tokens);

    let mut cost_token_idx = None;
    for idx in spells_token_idx + 1..tokens.len() {
        if !COST_OR_COSTS_WORD_PATTERN.matches_token(&tokens[idx]) {
            continue;
        }
        let amount_tokens = &tokens[idx + 1..];
        let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
        if parsed_amount.is_some() || parsed_mana_cost.is_some() {
            cost_token_idx = Some(idx);
            break;
        }
    }
    let Some(cost_token_idx) = cost_token_idx else {
        return Ok(None);
    };
    if cost_token_idx <= spells_token_idx {
        return Ok(None);
    }

    let mut filter = if is_this_spell {
        ObjectFilter::default()
    } else {
        parse_spell_filter_with_grammar_entrypoint(&subject_tokens)
    };

    let between_tokens = &tokens[spells_token_idx + 1..cost_token_idx];
    let between_words = crate::runtime_backend::token_word_refs(between_tokens);
    if !is_this_spell {
        for (idx, token) in between_tokens.iter().enumerate() {
            if !SPELL_OR_SPELLS_WORD_PATTERN.matches_token(token) {
                continue;
            }
            let mut start = idx;
            while start > 0 {
                if AND_WORD_PATTERN.matches_token(&between_tokens[start - 1])
                    || OR_WORD_PATTERN.matches_token(&between_tokens[start - 1])
                    || between_tokens[start - 1].is_comma()
                {
                    break;
                }
                start -= 1;
            }
            let descriptor_tokens = trim_commas(&between_tokens[start..idx]);
            if descriptor_tokens.is_empty() {
                continue;
            }
            let extra_filter = parse_spell_filter_with_grammar_entrypoint(
                strip_relative_target_clause(&descriptor_tokens),
            );
            if spell_filter_has_identity(&extra_filter) {
                merge_spell_filters(&mut filter, extra_filter);
            }
        }
        let between_filter = parse_spell_filter_with_grammar_entrypoint(
            strip_relative_target_clause(between_tokens),
        );
        if spell_filter_has_identity(&between_filter) {
            merge_spell_filters(&mut filter, between_filter);
        }
        if YOU_CAST_PHRASE_PATTERN.matches_words(&between_words) {
            filter.cast_by = Some(PlayerFilter::You);
        }
        if FROM_YOUR_GRAVEYARD_PHRASE_PATTERN.matches_words(&between_words) {
            filter.zone = Some(Zone::Graveyard);
            filter.owner = Some(PlayerFilter::You);
        }
        if OPPONENT_WORD_MARKER_PATTERN.matches_words(&between_words)
            && CAST_OR_CASTS_WORD_MARKER_PATTERN.matches_words(&between_words)
        {
            filter.cast_by = Some(PlayerFilter::Opponent);
        }
        let mut targets_idx = None;
        for (idx, token) in between_tokens.iter().enumerate() {
            if TARGET_OR_TARGETS_WORD_PATTERN.matches_token(token) {
                if idx > 0 && THAT_WORD_PATTERN.matches_token(&between_tokens[idx - 1]) {
                    targets_idx = Some(idx);
                    break;
                }
            }
        }
        if let Some(targets_idx) = targets_idx {
            let target_tokens = &between_tokens[targets_idx + 1..];
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing target in spells-cost modifier clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let (target_player, target_object) = parse_cost_modifier_target_spec(target_tokens)?;
            filter.targets_player = target_player;
            filter.targets_object = target_object;
        }
    }

    let amount_tokens = &tokens[cost_token_idx + 1..];
    let (parsed_amount, mut parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let mut parsed_mana_cost_repetitions = None;
    let (mut amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or_else(|| {
            if let Some((_, used)) = &parsed_mana_cost {
                (Value::Fixed(1), *used)
            } else {
                (Value::Fixed(1), 0)
            }
        });
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let Some(direction) = parse_cost_modifier_direction(&remaining_words) else {
        return Ok(None);
    };

    if let Some(dynamic_value) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
        if parsed_mana_cost.is_some() && is_this_spell {
            parsed_mana_cost_repetitions = Some(dynamic_value);
        } else {
            if parsed_mana_cost.is_some() {
                parsed_mana_cost = None;
            }
            let multiplier = parsed_amount
                .as_ref()
                .and_then(|(value, _)| match value {
                    Value::Fixed(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(1);
            amount_value = scale_dynamic_cost_modifier_value(dynamic_value, multiplier);
        }
    } else if parsed_amount.is_none() && parsed_mana_cost.is_none() {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    }

    // Handle trailing "where X is ..." clauses, e.g.
    // "This spell costs {X} less to cast, where X is the number of differently named lands you control."
    if WHERE_X_IS_MARKER_PATTERN.matches_words(&remaining_words) {
        let clause = clause_words.join(" ");
        let where_word_idx = find_window_by(&remaining_words, 3, |window| {
            WHERE_X_IS_PREFIX_PATTERN.matches_words(window)
        })
        .unwrap_or(0);
        let where_token_idx = token_index_for_word_index(remaining_tokens, where_word_idx)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map where-x clause in spells-cost modifier (clause: '{clause}')"
                ))
            })?;
        let where_tokens = trim_commas(&remaining_tokens[where_token_idx..]);
        let x_value = parse_value_binding_clause(&where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in spells-cost modifier (clause: '{clause}')"
            ))
        })?;
        if !value_contains_unbound_x(&amount_value) {
            return Err(CardTextError::ParseError(format!(
                "missing where-x clause in spells-cost modifier (clause: '{clause}')"
            )));
        }
        amount_value = replace_unbound_x_with_value(amount_value, &x_value, &clause)?;
    }
    if direction == CostModifierDirection::Less
        && let Some(cap) = parse_cost_reduction_cap(remaining_tokens)
    {
        amount_value = Value::Min(Box::new(amount_value), Box::new(Value::Fixed(cap)));
    }

    if !is_this_spell {
        parse_trailing_targets_condition_in_cost_modifier(
            &mut filter,
            remaining_tokens,
            &clause_words,
        )?;
    }

    let this_spell_condition = if is_this_spell {
        if let Some(condition) =
            parse_trailing_this_spell_cost_condition(remaining_tokens, &clause_words)?
        {
            condition
        } else if let Some(prefix) = &prefix_condition {
            match prefix {
                crate::ConditionExpr::YourTurn => {
                    crate::static_abilities::ThisSpellCostCondition::YourTurn
                }
                crate::ConditionExpr::Not(inner)
                    if matches!(inner.as_ref(), crate::ConditionExpr::YourTurn) =>
                {
                    crate::static_abilities::ThisSpellCostCondition::NotYourTurn
                }
                other => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported leading this-spell cost condition (clause: '{}'; condition: {other:?})",
                        clause_words.join(" ")
                    )));
                }
            }
        } else {
            crate::static_abilities::ThisSpellCostCondition::Always
        }
    } else {
        crate::static_abilities::ThisSpellCostCondition::Always
    };

    let non_this_condition = if is_this_spell {
        None
    } else {
        prefix_condition.clone()
    };

    if direction == CostModifierDirection::Less {
        // "This spell costs {N} less to cast" is a self-only modifier that should not
        // apply from the permanent on the battlefield after it resolves.
        if is_this_spell && parsed_mana_cost.is_none() {
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    amount_value,
                    this_spell_condition,
                ),
            )));
        }
        if is_this_spell && let Some((cost, _)) = parsed_mana_cost.clone() {
            let mut ability = crate::static_abilities::ThisSpellCostReductionManaCost::new(
                cost,
                this_spell_condition,
            );
            if let Some(repetitions) = parsed_mana_cost_repetitions {
                ability = ability.with_repetitions(repetitions);
            }
            return Ok(Some(StaticAbility::new(ability)));
        }
        if let Some((cost, _)) = parsed_mana_cost {
            let mut ability = crate::static_abilities::CostReductionManaCost::new(filter, cost);
            if let Some(condition) = non_this_condition.clone() {
                ability = ability.with_condition(condition);
            }
            return Ok(Some(StaticAbility::new(ability)));
        }
        let mut ability = crate::static_abilities::CostReduction::new(filter, amount_value);
        if let Some(condition) = non_this_condition.clone() {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(ability)));
    }

    if let Some((cost, _)) = parsed_mana_cost {
        let mut ability = crate::static_abilities::CostIncreaseManaCost::new(filter, cost);
        if let Some(condition) = non_this_condition.clone() {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(ability)));
    }

    let mut ability = crate::static_abilities::CostIncrease::new(filter, amount_value);
    if let Some(condition) = non_this_condition.clone() {
        ability = ability.with_condition(condition);
    }
    Ok(Some(StaticAbility::new(ability)))
}

pub(crate) fn parse_spell_and_player_activated_ability_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(and_idx) = find_window_by(tokens, 2, |window| {
        AND_ABILITIES_PREFIX_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(window))
    }) else {
        return Ok(None);
    };

    let left_tokens = trim_commas(&tokens[..and_idx]);
    let right_tokens = trim_commas(&tokens[and_idx + 1..]);
    let Some(spell_cost_ability) = parse_spells_cost_modifier_line(&left_tokens)? else {
        return Ok(None);
    };
    let Some(mut activated_cost_ability) =
        parse_player_activated_ability_cost_modifier_clause(&right_tokens)?
    else {
        return Ok(None);
    };

    if let Some(spells_idx) = find_index(tokens, |token| {
        SPELL_OR_SPELLS_WORD_PATTERN.matches_token(token)
    }) {
        let (prefix_condition, _) = parse_cost_modifier_prefix_condition(tokens, spells_idx)?;
        if let Some(condition) = prefix_condition {
            activated_cost_ability = activated_cost_ability.with_condition(condition);
        }
    }

    Ok(Some(vec![spell_cost_ability, activated_cost_ability]))
}

fn parse_cycling_cost_alternative_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 8 {
        return Ok(None);
    }

    let (condition, body_start_word_idx) = if AS_LONG_AS_PREFIX_PATTERN.matches_words(&clause_words)
    {
        let Some(body_rel_idx) =
            keyword_find_prefix_shape_start(&clause_words[3..], &YOU_MAY_PAY_PREFIX_PATTERN)
        else {
            return Ok(None);
        };
        let body_word_idx = body_rel_idx + 3;
        let condition_start = token_index_for_word_index(tokens, 3).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map cycling-cost alternative condition (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let condition_end = token_index_for_word_index(tokens, body_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map cycling-cost alternative body (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let condition_tokens = trim_commas(&tokens[condition_start..condition_end]);
        let condition = parse_static_condition_clause(&condition_tokens)?;
        (Some(condition), body_word_idx)
    } else {
        (None, 0)
    };

    let body_words = &clause_words[body_start_word_idx..];
    if !YOU_MAY_PAY_PREFIX_PATTERN.matches_words(body_words) {
        return Ok(None);
    }
    let Some(rather_rel_idx) =
        keyword_find_prefix_shape_start(body_words, &RATHER_THAN_PAY_CYCLING_COSTS_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    let rather_word_idx = body_start_word_idx + rather_rel_idx;
    let pay_token_idx =
        token_index_for_word_index(tokens, body_start_word_idx + 2).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map cycling-cost alternative cost (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let cost_start = pay_token_idx + 1;
    let cost_end = token_index_for_word_index(tokens, rather_word_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map cycling-cost alternative tail (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let replacement_mana_cost = if cost_end <= cost_start {
        ManaCost::new()
    } else {
        let replacement_cost_tokens = trim_commas(&tokens[cost_start..cost_end]);
        let replacement_total_cost = parse_activation_cost(&replacement_cost_tokens)?;
        if replacement_total_cost.has_non_mana_costs() {
            return Err(CardTextError::ParseError(format!(
                "unsupported non-mana cycling alternative cost (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        replacement_total_cost.mana_cost().cloned().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing cycling alternative mana cost (clause: '{}')",
                clause_words.join(" ")
            ))
        })?
    };

    let mut filter = ObjectFilter::default().with_ability_marker("cycling");
    filter.zone = Some(Zone::Hand);
    let display = format!(
        "You may pay {} rather than pay cycling costs",
        replacement_mana_cost.to_oracle()
    );
    let mut ability =
        StaticAbility::replace_activated_ability_mana_cost(filter, replacement_mana_cost, display);
    if let Some(condition) = condition {
        ability = ability.with_condition(condition);
    }
    Ok(Some(ability))
}

fn parse_player_activated_ability_cost_modifier_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 7
        || !clause_words
            .first()
            .is_some_and(|word| ABILITIES_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let Some(activate_idx) = find_index(&clause_words, |word| {
        ACTIVATE_OR_ACTIVATES_WORD_PATTERN.matches_word(word)
    }) else {
        return Ok(None);
    };
    let activator_words = &clause_words[1..activate_idx];
    let activator = if YOU_SUBJECT_PATTERN.matches_words(activator_words) {
        PlayerFilter::You
    } else if YOUR_OPPONENTS_ACTIVATOR_PATTERN.matches_words(activator_words) {
        PlayerFilter::Opponent
    } else {
        return Ok(None);
    };

    let Some(cost_idx) = find_index(&clause_words[activate_idx + 1..], |word| {
        COST_OR_COSTS_WORD_PATTERN.matches_word(word)
    })
    .map(|idx| idx + activate_idx + 1) else {
        return Ok(None);
    };
    let cost_token_idx = crate::runtime_backend::token_index_for_word_index(tokens, cost_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map activated-ability cost modifier amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let amount_tokens = &tokens[cost_token_idx + 1..];
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (increase, used) = if let Some((mana_cost, used)) = parsed_mana_cost {
        (TotalCost::mana(mana_cost), used)
    } else if let Some((Value::Fixed(amount), used)) = parsed_amount {
        if amount < 0 {
            return Ok(None);
        }
        let generic = amount.min(u8::MAX as i32) as u8;
        (
            TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(generic)])),
            used,
        )
    } else {
        return Ok(None);
    };
    let remaining_words = crate::runtime_backend::token_word_refs(&amount_tokens[used..]);
    if parse_cost_modifier_direction(&remaining_words) != Some(CostModifierDirection::More)
        || !TO_ACTIVATE_PHRASE_PATTERN.matches_words(&remaining_words)
    {
        return Ok(None);
    }

    let non_mana_only = UNLESS_THEYRE_MANA_ABILITIES_PATTERN.matches_words(&remaining_words);
    Ok(Some(
        StaticAbility::increase_activated_ability_costs_for_activator(
            activator,
            increase,
            non_mana_only,
        ),
    ))
}

fn strip_relative_target_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let Some(target_clause_idx) = find_window_by(tokens, 2, |window| {
        THAT_TARGET_OR_TARGETS_PREFIX_PATTERN
            .matches_words(&crate::runtime_backend::token_word_refs(window))
    }) else {
        return tokens;
    };

    &tokens[..target_clause_idx]
}

pub(crate) fn parse_trailing_targets_condition_in_cost_modifier(
    filter: &mut ObjectFilter,
    remaining_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(), CardTextError> {
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let Some(if_word_idx) = find_index(&remaining_words, |word| {
        IF_PREFIX_PATTERN.matches_word(word)
    }) else {
        return Ok(());
    };
    let condition_words = &remaining_words[if_word_idx..];
    if condition_words.len() < 4
        || !IF_IT_TARGET_OR_TARGETS_PREFIX_PATTERN.matches_words(condition_words)
    {
        return Ok(());
    }

    let target_word_idx = if_word_idx + 3;
    let target_token_idx = token_index_for_word_index(remaining_tokens, target_word_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map trailing target condition in spells-cost modifier (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_tokens = &remaining_tokens[target_token_idx..];
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in trailing spells-cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (targets_player, targets_object) = parse_cost_modifier_target_spec(target_tokens)?;
    filter.targets_player = targets_player;
    filter.targets_object = targets_object;
    Ok(())
}

pub(crate) fn parse_flashback_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some((kind, consumed)) = parse_alternative_cast_words(&clause_words) else {
        return Ok(None);
    };
    if clause_words.len() < consumed + 5 {
        return Ok(None);
    }
    if clause_words.get(consumed).copied() != Some("costs") {
        return Ok(None);
    }
    let cost_idx = rfind_index(tokens, |token| {
        COST_OR_COSTS_WORD_PATTERN.matches_token(token)
    });
    let Some(cost_idx) = cost_idx else {
        return Ok(None);
    };
    let amount_tokens = &tokens[cost_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or((Value::Fixed(1), 0));
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let Some(direction) = parse_cost_modifier_direction(&remaining_words) else {
        return Ok(None);
    };
    if parsed_amount.is_none() {
        return Err(CardTextError::ParseError(
            "missing flashback cost modifier amount".to_string(),
        ));
    }

    let mut filter = ObjectFilter::default();
    filter.alternative_cast = Some(kind);
    if YOU_PAY_PHRASE_PATTERN.matches_words(&clause_words) {
        filter.cast_by = Some(PlayerFilter::You);
    } else if OPPONENTS_PAY_PHRASE_PATTERN.matches_words(&clause_words) {
        filter.cast_by = Some(PlayerFilter::Opponent);
    }

    if direction == CostModifierDirection::Less {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, amount_value),
        )));
    }
    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostIncrease::new(filter, amount_value),
    )))
}

pub(crate) fn parse_equip_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 6 || !EQUIP_WORD_PATTERN.matches_first_word(&clause_words) {
        return Ok(None);
    }
    if clause_words.get(1).copied() != Some("costs") {
        return Ok(None);
    }
    let Some(cost_idx) = rfind_index(tokens, |token| {
        COST_OR_COSTS_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };

    let amount_tokens = &tokens[cost_idx + 1..];
    let Some((amount_value, used)) = parse_cost_modifier_amount(amount_tokens) else {
        return Ok(None);
    };
    let Value::Fixed(amount) = amount_value else {
        return Ok(None);
    };
    if amount < 0 {
        return Ok(None);
    }

    let remaining_words = crate::runtime_backend::token_word_refs(&amount_tokens[used..]);
    let Some(direction) = parse_cost_modifier_direction(&remaining_words) else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::default().with_ability_marker("equip");
    if YOU_PAY_PHRASE_PATTERN.matches_words(&clause_words) {
        filter.controller = Some(PlayerFilter::You);
    } else if OPPONENTS_PAY_PHRASE_PATTERN.matches_words(&clause_words) {
        filter.controller = Some(PlayerFilter::Opponent);
    }

    if direction == CostModifierDirection::Less {
        let amount_text = format!("{{{amount}}}");
        let display = if filter.controller == Some(PlayerFilter::Opponent) {
            format!("Equip costs your opponents pay cost {amount_text} less")
        } else {
            format!("Equip costs you pay cost {amount_text} less")
        };
        return Ok(Some(
            StaticAbility::reduce_activated_ability_costs_with_display(
                filter,
                amount as u32,
                None,
                display,
            ),
        ));
    }

    let increase = TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(
        amount.min(u8::MAX as i32) as u8,
    )]));
    Ok(Some(StaticAbility::increase_activated_ability_costs(
        filter, increase,
    )))
}

pub(crate) fn parse_foretelling_cards_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 7 {
        return Ok(None);
    }
    if !FORETELLING_CARDS_FROM_HAND_COSTS_PREFIX_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }

    let has_any_players_turn = ANY_PLAYER_TURN_PATTERN.matches_words(&clause_words);
    if parse_cost_modifier_direction(&clause_words) != Some(CostModifierDirection::Less)
        || !has_any_players_turn
    {
        return Ok(None);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported foretelling cost modifier clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_cost_modifier_amount(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    if let Some((amount, used)) = parse_number(tokens) {
        return Some((Value::Fixed(amount as i32), used));
    }

    let first_token = tokens.first()?;
    let group = mana_pips_from_token(first_token)?;
    if group.len() != 1 {
        return None;
    }
    let symbol = group[0];
    if let ManaSymbol::Generic(amount) = symbol {
        return Some((Value::Fixed(amount as i32), 1));
    }
    if symbol == ManaSymbol::X {
        return Some((Value::X, 1));
    }
    None
}

pub(crate) fn parse_cost_modifier_mana_cost(
    tokens: &[OwnedLexToken],
) -> Option<(crate::mana::ManaCost, usize)> {
    use crate::mana::{ManaCost, ManaSymbol};

    let mut pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut used = 0usize;
    while let Some(token) = tokens.get(used) {
        let Some(group) = mana_pips_from_token(token) else {
            break;
        };
        if group.iter().any(|symbol| {
            matches!(
                symbol,
                ManaSymbol::X | ManaSymbol::Snow | ManaSymbol::Life(_)
            )
        }) {
            break;
        }
        pips.push(group);
        used += 1;
    }
    if used == 0 {
        return None;
    }
    Some((ManaCost::from_pips(pips), used))
}

pub(crate) fn parse_cost_modifier_components(
    amount_tokens: &[OwnedLexToken],
) -> (
    Option<(Value, usize)>,
    Option<(crate::mana::ManaCost, usize)>,
) {
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let parsed_mana_cost = parse_cost_modifier_mana_cost(amount_tokens);

    let amount_used = parsed_amount.as_ref().map(|(_, used)| *used).unwrap_or(0);
    let mana_used = parsed_mana_cost
        .as_ref()
        .map(|(_, used)| *used)
        .unwrap_or(0);

    // Prefer mana-symbol parsing when it consumes a longer contiguous mana sequence
    // (e.g. "{2}{U}{U}" should stay a single mana-cost reduction component).
    if mana_used > amount_used {
        return (None, parsed_mana_cost);
    }

    (parsed_amount, None)
}

fn parse_cost_reduction_cap(tokens: &[OwnedLexToken]) -> Option<i32> {
    for idx in 2..tokens.len().saturating_sub(1) {
        if !BY_WORD_PATTERN.matches_token(&tokens[idx - 2])
            || !MORE_WORD_PATTERN.matches_token(&tokens[idx - 1])
            || !THAN_WORD_PATTERN.matches_token(&tokens[idx])
        {
            continue;
        }
        let group = mana_pips_from_token(tokens.get(idx + 1)?)?;
        if group.len() != 1 {
            return None;
        }
        return match group[0] {
            ManaSymbol::Generic(amount) => Some(amount as i32),
            _ => None,
        };
    }
    None
}

pub(crate) fn parse_dynamic_cost_modifier_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if let Some(player) = dynamic_cards_drawn_this_turn_player(&words_all) {
        return Ok(Some(Value::MaxCardsDrawnThisTurn(player)));
    }

    let Some(each_idx) = find_index(&words_all, |word| EACH_WORD_PATTERN.matches_word(word)) else {
        return Ok(None);
    };

    let filter_tokens = &tokens[each_idx + 1..];
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    if filter_words.is_empty() {
        return Ok(None);
    }
    if KICK_COUNT_DYNAMIC_PREFIX_PATTERN.matches_words(&filter_words) {
        return Ok(Some(Value::KickCount));
    }
    if CREATURES_DIED_THIS_TURN_PREFIX_PATTERN.matches_words(&filter_words) {
        return Ok(Some(Value::CreaturesDiedThisTurn));
    }
    if LIFE_OPPONENTS_LOST_THIS_TURN_PREFIX_PATTERN.matches_words(&filter_words) {
        return Ok(Some(Value::LifeLostThisTurn(PlayerFilter::Opponent)));
    }
    if CREATURES_DIED_UNDER_YOUR_CONTROL_PREFIX_PATTERN.matches_words(&filter_words) {
        if THIS_TURN_MARKER_PATTERN.matches_words(&filter_words) {
            return Ok(Some(Value::CreaturesDiedThisTurnControlledBy(
                PlayerFilter::You,
            )));
        }
    }
    // "for each spell you've cast this turn" (and limited variants like "instant and sorcery spell")
    if let Some(player) = dynamic_spell_cast_this_turn_player(&filter_words) {
        if CARD_TYPE_MARKER_PATTERN.matches_words(&filter_words) {
            let mut filter = ObjectFilter::spell();
            filter.cast_by = Some(player);
            return Ok(Some(Value::CardTypesAmong(filter)));
        }

        if OTHER_THAN_FIRST_PATTERN.matches_words(&filter_words) {
            return Ok(Some(Value::Add(
                Box::new(Value::SpellsCastThisTurn(player)),
                Box::new(Value::Fixed(-1)),
            )));
        }

        let exclude_source = OTHER_WORD_MARKER_PATTERN.matches_words(&filter_words);
        let has_instant = filter_words
            .iter()
            .any(|word| INSTANT_WORD_PATTERN.matches_word(word));
        let has_sorcery = filter_words
            .iter()
            .any(|word| SORCERY_WORD_PATTERN.matches_word(word));
        if has_instant || has_sorcery {
            let mut filter = ObjectFilter::spell();
            filter.card_types = if has_instant && has_sorcery {
                vec![CardType::Instant, CardType::Sorcery]
            } else if has_instant {
                vec![CardType::Instant]
            } else {
                vec![CardType::Sorcery]
            };
            return Ok(Some(Value::SpellsCastThisTurnMatching {
                player,
                filter,
                exclude_source,
            }));
        }

        if SIMPLE_YOU_CAST_SPELLS_THIS_TURN_PATTERN.matches_words(&filter_words) {
            return Ok(Some(Value::SpellsCastThisTurn(player)));
        }
    }

    if YOU_DREW_CARDS_DYNAMIC_PREFIX_PATTERN.matches_words(&filter_words) {
        return Ok(Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::You)));
    }

    if let Some(player) = dynamic_cards_drawn_this_turn_player(&filter_words) {
        return Ok(Some(Value::MaxCardsDrawnThisTurn(player)));
    }

    if CARD_TYPES_IN_GRAVEYARD_DYNAMIC_PATTERN.matches_words(&filter_words) {
        let player = if YOUR_GRAVEYARD_PHRASE_PATTERN.matches_words(&filter_words) {
            PlayerFilter::You
        } else if OPPONENT_GRAVEYARD_PHRASE_PATTERN.matches_words(&filter_words) {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::You
        };
        return Ok(Some(Value::CardTypesInGraveyard(player)));
    }

    if COLORS_OF_MANA_CAST_THIS_SPELL_PREFIX_PATTERN.matches_words(&filter_words) {
        return Ok(Some(Value::ColorsOfManaSpentToCastThisSpell));
    }
    if CREATURES_IN_YOUR_PARTY_PREFIX_PATTERN.matches_words(&filter_words) {
        return Ok(Some(Value::PartySize(PlayerFilter::You)));
    }
    if BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words) {
        let lands_tokens = &filter_tokens[4..];
        if let Ok(filter) = parse_object_filter(lands_tokens, false) {
            return Ok(Some(Value::BasicLandTypesAmong(filter)));
        }
    }
    if CREATURE_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words) {
        let Some(after_among_token_idx) = token_index_for_word_index(filter_tokens, 3) else {
            return Ok(None);
        };
        let mut end_token_idx = filter_tokens.len();
        if let Some(period_idx) =
            find_token_kind(&filter_tokens[after_among_token_idx..], TokenKind::Period)
        {
            end_token_idx = after_among_token_idx + period_idx;
        }
        let creature_scope_tokens =
            trim_commas(&filter_tokens[after_among_token_idx..end_token_idx]);
        if let Ok(filter) = parse_object_filter(&creature_scope_tokens, false) {
            return Ok(Some(Value::CreatureTypesAmong(filter)));
        }
    }
    if CARD_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words) {
        let Some(after_among_token_idx) = token_index_for_word_index(filter_tokens, 3) else {
            return Ok(None);
        };
        let mut end_token_idx = filter_tokens.len();
        if let Some(period_idx) =
            find_token_kind(&filter_tokens[after_among_token_idx..], TokenKind::Period)
        {
            end_token_idx = after_among_token_idx + period_idx;
        }
        let card_scope_tokens = trim_commas(&filter_tokens[after_among_token_idx..end_token_idx]);
        if let Ok(filter) = parse_object_filter(&card_scope_tokens, false) {
            return Ok(Some(Value::CardTypesAmong(filter)));
        }
    }
    if word_slice_starts_with_any(
        &filter_words,
        &[
            &["different", "powers", "among"],
            &["different", "power", "values", "among"],
            &["different", "power", "among"],
        ],
    ) {
        let scope_start_word_idx = if word_slice_at_is(&filter_words, 2, "among") {
            3
        } else {
            4
        };
        let Some(scope_start_token_idx) =
            token_index_for_word_index(filter_tokens, scope_start_word_idx)
        else {
            return Ok(None);
        };
        let mut end_token_idx = filter_tokens.len();
        if let Some(period_idx) =
            find_token_kind(&filter_tokens[scope_start_token_idx..], TokenKind::Period)
        {
            end_token_idx = scope_start_token_idx + period_idx;
        }
        let scope_tokens = trim_commas(&filter_tokens[scope_start_token_idx..end_token_idx]);
        if let Ok(filter) = parse_object_filter(&scope_tokens, false) {
            return Ok(Some(Value::DistinctPowers(filter)));
        }
    }

    let has_card_type_among = CARD_TYPES_AMONG_MARKER_PATTERN.matches_words(&filter_words);
    if has_card_type_among {
        return Err(CardTextError::ParseError(format!(
            "unsupported card-types-among dynamic value (clause: '{}')",
            words_all.join(" ")
        )));
    }

    // "for each <counter> counter removed this way" (storage lands, mana batteries, etc.)
    // The remove-counters cost plumbs the removed total through `CostContext.x_value`,
    // so model the dynamic amount as `X`.
    if COUNTERS_REMOVED_THIS_WAY_PATTERN.matches_words(&filter_words) {
        return Ok(Some(Value::X));
    }
    if DESTROYED_THIS_WAY_PATTERN.matches_words(&filter_words) {
        return Ok(Some(Value::PendingEffectMetric {
            source: EffectMetricSource::AffectedObjects,
            metric: EffectMetric::Count,
        }));
    }
    if DISCARDED_THIS_WAY_PATTERN.matches_words(&filter_words) {
        return Ok(Some(Value::PendingEffectMetric {
            source: EffectMetricSource::Outcome,
            metric: EffectMetric::Count,
        }));
    }
    if REVEALED_THIS_WAY_PATTERN.matches_words(&filter_words)
        && let Some((value, used_words)) = parse_for_each_count_value_words(&words_all)
        && used_words == words_all.len()
    {
        return Ok(Some(value));
    }

    let mut source_counter_words = filter_words.as_slice();
    if source_counter_words
        .first()
        .is_some_and(|word| SOURCE_COUNTER_LEADING_WORD_PATTERN.matches_word(word))
    {
        source_counter_words = &source_counter_words[1..];
    }
    let source_counter_match = if source_counter_words.len() >= 3
        && COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(source_counter_words[0])
        && ON_WORD_PATTERN.matches_word(source_counter_words[1])
    {
        Some((None, 1usize))
    } else if source_counter_words.len() >= 4
        && parse_counter_type_word(source_counter_words[0]).is_some()
        && COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(source_counter_words[1])
        && ON_WORD_PATTERN.matches_word(source_counter_words[2])
    {
        Some((parse_counter_type_word(source_counter_words[0]), 2usize))
    } else {
        None
    };
    if let Some((counter_type, on_idx)) = source_counter_match {
        let tail = &source_counter_words[on_idx + 1..];
        let on_source = SOURCE_COUNTER_REFERENCE_PREFIX_PATTERN.matches_words(tail);
        if on_source {
            return Ok(Some(match counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            }));
        }
        if let Some(surface) = source_reference_surface_for_words(tail) {
            return Ok(Some(Value::CountersOn(
                Box::new(source_choose_spec_for_surface(surface)),
                counter_type,
            )));
        }
    }

    if let Some(player) = parse_commander_cast_count_player(filter_tokens) {
        return Ok(Some(Value::CommanderCastCount(player)));
    }

    if THIS_WAY_PATTERN.matches_words(&filter_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-way dynamic value (clause: '{}')",
            words_all.join(" ")
        )));
    }

    if let Ok(filter) = parse_object_filter(filter_tokens, false) {
        return Ok(Some(Value::Count(filter)));
    }

    Ok(None)
}

pub(crate) fn parse_add_mana_that_much_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if THAT_MUCH_PREFIX_PATTERN.matches_words(&words_all) {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }
    None
}

pub(crate) fn parse_players_skip_upkeep_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let tokens = super::grammar::effects::split_labeled_effect_prefix_lexed(&tokens)
        .unwrap_or(&tokens);
    let words = parser_token_word_refs(tokens);
    if token_slice_starts_with(tokens, &["skip", "your", "upkeep", "step"]) {
        let mut ability = StaticAbility::player_skips_upkeep(crate::target::PlayerFilter::You);
        if words.len() > 4 {
            if words.get(4) != Some(&"if") || tokens.len() <= 5 {
                return Err(CardTextError::ParseError(format!(
                    "unsupported skip-upkeep tail (clause: '{}')",
                    words.join(" ")
                )));
            }
            let condition = parse_static_condition_clause(&tokens[5..])?;
            ability = ability.with_condition(condition);
        }
        return Ok(Some(ability));
    }
    if is_players_skip_upkeep_line_lexed(tokens) {
        return Ok(Some(StaticAbility::players_skip_upkeep()));
    }
    Ok(None)
}

pub(crate) fn parse_legend_rule_doesnt_apply_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let has_negative = DOESNT_WORD_MARKER_PATTERN.matches_words(&words)
        || DOES_NOT_PHRASE_PATTERN.matches_words(&words);
    if LEGEND_RULE_APPLY_MARKER_PATTERN.matches_words(&words) && has_negative {
        return Ok(Some(StaticAbility::legend_rule_doesnt_apply()));
    }
    Ok(None)
}

pub(crate) fn parse_all_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_all_permanents_colorless_line_lexed(tokens) {
        return Ok(Some(StaticAbility::make_colorless(
            ObjectFilter::permanent(),
        )));
    }
    Ok(None)
}

pub(crate) fn parse_subject_are_card_types_in_addition_to_their_other_types_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(be_idx) = find_index(&words, |word| BE_WORD_PATTERN.matches_word(word)) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 2 >= words.len() {
        return Ok(None);
    }

    let tail = &words[be_idx + 1..];
    let Some(addition_idx) = find_window_by(tail, 5, |window| {
        IN_ADDITION_TO_OTHER_PATTERN.matches_words(window)
    }) else {
        return Ok(None);
    };
    if addition_idx == 0 {
        return Ok(None);
    }

    if !tail
        .get(addition_idx + 5)
        .is_some_and(|word| TYPE_OR_TYPES_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let subject_tokens = &tokens[..be_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let added_words = &tail[..addition_idx];
    if CHOSEN_TYPE_PATTERN.matches_words(added_words) {
        let filter = parse_object_filter(subject_tokens, false)?;
        return Ok(Some(vec![StaticAbility::add_chosen_creature_type(
            filter,
            render_token_slice(tokens),
        )]));
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in &tail[..addition_idx] {
        if TYPE_ADDITION_IGNORED_DESCRIPTOR_WORD_PATTERN.matches_word(descriptor) {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !slice_contains(&card_types, &card_type) {
                card_types.push(card_type);
            }
            continue;
        }

        let Some(subtype) = parse_subtype_flexible(descriptor) else {
            return Ok(None);
        };
        if !slice_contains(&subtypes, &subtype) {
            subtypes.push(subtype);
        }
    }
    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter(subject_tokens, false)?;

    let mut abilities = Vec::new();
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

pub(crate) fn parse_all_cards_spells_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if ALL_CARDS_SPELLS_PERMANENTS_COLORLESS_PATTERN.matches_words(&words) {
        return Ok(Some(StaticAbility::make_colorless(ObjectFilter::default())));
    }
    Ok(None)
}

pub(crate) fn parse_all_cards_spells_permanents_add_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if matches!(
        words.as_slice(),
        [
            "all",
            "cards",
            "that",
            "arent",
            "on",
            "the",
            "battlefield",
            "spells",
            "and",
            "permanents",
            "are",
            "the",
            "chosen",
            "color",
            "in",
            "addition",
            "to",
            "their",
            "other",
            "colors"
        ]
    ) {
        return Ok(Some(StaticAbility::add_chosen_color(
            ObjectFilter::default(),
            render_token_slice(tokens),
        )));
    }

    Ok(None)
}

pub(crate) fn parse_all_are_color_and_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 10 {
        return Ok(None);
    }
    let Some(are_idx) = find_index(&words, |word| ARE_WORD_PATTERN.matches_word(word)) else {
        return Ok(None);
    };
    if are_idx == 0 || are_idx + 4 >= words.len() {
        return Ok(None);
    }

    let Some(base_color) = words.get(are_idx + 1).and_then(|word| parse_color(word)) else {
        return Ok(None);
    };

    // Pattern: "<subject> are <color> and are <subtype>... in addition to their other creature types"
    if !words
        .get(are_idx + 2..)
        .is_some_and(|tail| AND_ARE_PREFIX_PATTERN.matches_words(tail))
    {
        return Ok(None);
    }

    let tail = &words[are_idx + 4..];
    let Some(addition_idx) = (0..tail.len())
        .find(|idx| IN_ADDITION_TO_THEIR_OTHER_PREFIX_PATTERN.matches_words(&tail[*idx..]))
    else {
        return Ok(None);
    };
    if addition_idx == 0 {
        return Ok(None);
    }

    let scope = &tail[addition_idx + 5..];
    if !CREATURE_TYPE_SCOPE_PATTERN.matches_words(scope) {
        return Ok(None);
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in &tail[..addition_idx] {
        if TYPE_ADDITION_IGNORED_DESCRIPTOR_WORD_PATTERN.matches_word(descriptor) {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !slice_contains(&card_types, &card_type) {
                card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            if !slice_contains(&subtypes, &subtype) {
                subtypes.push(subtype);
            }
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in are-color-and-type-addition clause (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let subject_tokens = &tokens[..are_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(subject_tokens, false)?;

    let mut abilities = vec![StaticAbility::set_colors(filter.clone(), base_color)];
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

pub(crate) fn parse_all_creatures_are_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 4 {
        return Ok(None);
    }
    let Some(are_idx) = find_index(&words, |word| BE_WORD_PATTERN.matches_word(word)) else {
        return Ok(None);
    };
    if are_idx == 0 {
        return Ok(None);
    }
    let color = match &words[are_idx + 1..] {
        ["all", "colors"] => crate::color::Color::ALL.into_iter().collect::<ColorSet>(),
        [color_word] => {
            let Some(color) = parse_color(color_word) else {
                return Ok(None);
            };
            color
        }
        _ => return Ok(None),
    };

    let subject_tokens = &tokens[..are_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(subject_tokens, false)?;

    Ok(Some(StaticAbility::set_colors(filter, color)))
}

pub(crate) fn parse_subjects_are_basic_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(be_idx) = find_index(&words, |word| matches!(*word, "is" | "are")) else {
        return Ok(None);
    };
    if be_idx == 0 || !word_slice_eq(&words[be_idx + 1..], &["basic"]) {
        return Ok(None);
    }

    let subject_tokens = trim_lexed_commas(&tokens[..be_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_segments = split_lexed_slices_on_and(subject_tokens);
    let filter = if subject_segments.len() > 1 {
        let mut branches = Vec::with_capacity(subject_segments.len());
        for segment in subject_segments {
            let segment = trim_lexed_commas(segment);
            if segment.is_empty() {
                return Ok(None);
            }
            branches.push(parse_object_filter_lexed(segment, false)?);
        }
        let mut filter = ObjectFilter::default();
        filter.any_of = branches;
        filter
    } else {
        parse_object_filter_lexed(subject_tokens, false)?
    };

    Ok(Some(StaticAbility::add_supertypes(
        filter,
        vec![Supertype::Basic],
    )))
}

pub(crate) fn parse_nonbasic_lands_are_basic_land_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(be_idx) = IS_OR_ARE_WORD_PATTERN.find_word(&words) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 1 >= words.len() {
        return Ok(None);
    }

    let mut subtype_idx = be_idx + 1;
    if word_refs_at_is_article(&words, subtype_idx) {
        subtype_idx += 1;
    }
    let subtype_words = &words[subtype_idx..];
    if subtype_words.len() != 1 {
        return Ok(None);
    }

    let Some(subtype) = parse_subtype_flexible(subtype_words[0]) else {
        return Ok(None);
    };
    if !matches!(
        subtype,
        Subtype::Plains | Subtype::Island | Subtype::Swamp | Subtype::Mountain | Subtype::Forest
    ) {
        return Ok(None);
    }

    let subject_tokens = &tokens[..be_idx];
    let filter = parse_object_filter(subject_tokens, false)?;
    let filter_words = &words[..be_idx];
    if !filter_words
        .iter()
        .any(|word| LAND_OR_LANDS_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    Ok(Some(StaticAbility::set_land_subtypes(
        filter,
        vec![subtype],
    )))
}

pub(crate) fn parse_remove_snow_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_remove_snow_line_lexed(tokens) {
        return Ok(Some(StaticAbility::remove_supertypes(
            ObjectFilter::land(),
            vec![Supertype::Snow],
        )));
    }
    Ok(None)
}

pub(crate) fn parse_land_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 10 {
        return Ok(None);
    }

    let Some(be_idx) = find_index(&words, |word| IS_OR_ARE_WORD_PATTERN.matches_word(word)) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 1 >= words.len() {
        return Ok(None);
    }

    if EVERY_BASIC_LAND_TYPE_ADDITION_TAIL_PATTERN.matches_words(&words[be_idx + 1..]) {
        let filter_tokens = &tokens[..be_idx];
        if filter_tokens.is_empty() {
            return Ok(None);
        }
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(Some(StaticAbility::add_subtypes(
            filter,
            vec![
                Subtype::Plains,
                Subtype::Island,
                Subtype::Swamp,
                Subtype::Mountain,
                Subtype::Forest,
            ],
        )));
    }

    let mut subtype_word_idx = be_idx + 1;
    if words
        .get(subtype_word_idx)
        .is_some_and(|word| is_article(word))
    {
        subtype_word_idx += 1;
    }
    let Some(subtype_word) = words.get(subtype_word_idx).copied() else {
        return Ok(None);
    };
    let Some(subtype) = parse_subtype_flexible(subtype_word) else {
        return Ok(None);
    };
    if !is_land_subtype(subtype) {
        return Ok(None);
    }

    let tail = &words[subtype_word_idx + 1..];
    if !LAND_TYPE_ADDITION_TAIL_PATTERN.matches_words(tail) {
        return Ok(None);
    }

    let filter_tokens = &tokens[..be_idx];
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(filter_tokens, false)?;

    Ok(Some(StaticAbility::add_subtypes(filter, vec![subtype])))
}

pub(crate) fn parse_lands_are_pt_creatures_still_lands_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(be_idx) = find_index(&words, |word| IS_OR_ARE_WORD_PATTERN.matches_word(word)) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 2 >= words.len() {
        return Ok(None);
    }
    let (power, toughness) = match parse_pt_modifier(words[be_idx + 1]) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };

    if !STATIC_CREATURE_OR_CREATURES_WORD_PATTERN.matches_word(words[be_idx + 2]) {
        return Ok(None);
    }

    let tail = &words[be_idx + 3..];
    if !STILL_LAND_ANIMATION_TAIL_PATTERN.matches_words(tail) {
        return Ok(None);
    }

    let filter_tokens = &tokens[..be_idx];
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(filter_tokens, false)?;

    Ok(Some(vec![
        StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature]),
        StaticAbility::set_base_power_toughness(filter, power, toughness),
    ]))
}

pub(crate) fn parse_filter_is_pt_creature_in_addition_and_has_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(be_idx) = find_index(tokens, |token| IS_OR_ARE_WORD_PATTERN.matches_token(token))
    else {
        return Ok(None);
    };
    let Some(has_idx) = find_index(&tokens[be_idx + 1..], |token| {
        HAVE_OR_HAS_WORD_PATTERN.matches_token(token)
    })
    .map(|offset| be_idx + 1 + offset) else {
        return Ok(None);
    };

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..be_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let attached_subject = crate::runtime_backend::token_word_refs(&subject_tokens)
        .first()
        .is_some_and(|word| ENCHANTED_OR_EQUIPPED_WORD_PATTERN.matches_word(word));

    let before_has = trim_commas(&tokens[be_idx + 1..has_idx]);
    if before_has.is_empty() {
        return Ok(None);
    }
    let before_has_words = crate::runtime_backend::token_word_refs(&before_has);
    let before_has_words = strip_leading_article_word_refs(&before_has_words);
    if before_has_words.len() < 8 {
        return Ok(None);
    }

    let (power, toughness) = match parse_pt_modifier(before_has_words[0]) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let Some(creature_idx) = find_index(&before_has_words, |word| {
        STATIC_CREATURE_OR_CREATURES_WORD_PATTERN.matches_word(word)
    }) else {
        return Ok(None);
    };
    if creature_idx == 0 {
        return Ok(None);
    }
    let subtype_words = &before_has_words[1..creature_idx];
    let mut subtypes = Vec::new();
    for word in subtype_words {
        if is_article(word) {
            continue;
        }
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }
    let mut tail = &before_has_words[creature_idx + 1..];
    if tail
        .last()
        .is_some_and(|word| AND_WORD_PATTERN.matches_word(word))
    {
        tail = &tail[..tail.len().saturating_sub(1)];
    }
    if !OTHER_TYPE_ADDITION_TAIL_PATTERN.matches_words(tail) {
        return Ok(None);
    }

    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(&tokens[has_idx + 1..], &clause_words, attached_subject)?
    else {
        return Ok(None);
    };

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::Add,
            base_power_toughness: Some((power, toughness)),
            granted_tail,
        },
    )))
}

pub(crate) fn parse_subject_is_subtype_with_base_pt_and_granted_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = if let Some((label, body_tokens)) = split_em_dash_label_prefix(tokens) {
        if preserve_keyword_prefix_for_parse(label.as_str()) {
            tokens
        } else {
            body_tokens
        }
    } else {
        tokens
    };
    let Some(be_idx) = find_index(tokens, |token| IS_OR_ARE_WORD_PATTERN.matches_token(token))
    else {
        return Ok(None);
    };
    let Some(with_idx) = find_index(&tokens[be_idx + 1..], |token| {
        WITH_WORD_PATTERN.matches_token(token)
    })
    .map(|offset| be_idx + 1 + offset) else {
        return Ok(None);
    };

    let (_condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..be_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let attached_subject = crate::runtime_backend::token_word_refs(&subject_tokens)
        .first()
        .is_some_and(|word| ENCHANTED_OR_EQUIPPED_WORD_PATTERN.matches_word(word));

    let type_tokens = trim_commas(&tokens[be_idx + 1..with_idx]);
    if type_tokens.is_empty() {
        return Ok(None);
    }
    let type_words = crate::runtime_backend::token_word_refs(&type_tokens);
    let type_words = strip_leading_article_word_refs(&type_words);
    if type_words.is_empty() {
        return Ok(None);
    }
    let mut subtypes = Vec::new();
    for word in type_words {
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }

    let mut after_with = trim_commas(&tokens[with_idx + 1..]).to_vec();
    if after_with.is_empty() {
        return Ok(None);
    }

    let _loses_other_creature_types = {
        let words = crate::runtime_backend::token_word_refs(&after_with)
            .into_iter()
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        let mut note_start = None;
        let mut idx = 0usize;
        while idx + 6 <= words.len() {
            let window = &words[idx..idx + 6];
            let window_refs = window.iter().map(String::as_str).collect::<Vec<_>>();
            if LOSES_ALL_OTHER_CREATURE_TYPES_PATTERN.matches_words(&window_refs) {
                note_start = Some(idx);
                break;
            }
            idx += 1;
        }
        if let Some(note_start) = note_start {
            let Some(token_idx) = token_index_for_word_index(&after_with, note_start) else {
                return Ok(None);
            };
            after_with.truncate(token_idx);
            true
        } else {
            false
        }
    };

    let after_with = trim_edge_punctuation(&after_with);
    let after_with_words = crate::runtime_backend::token_word_refs(&after_with);
    if after_with_words.len() < 5
        || !BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches_words(&after_with_words)
    {
        return Ok(None);
    }
    let (power, toughness) = match parse_pt_modifier(after_with_words[4]) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };

    let ability_start_word_idx = if COMMA_WORD_PATTERN.matches_word_at(&after_with_words, 5) {
        6
    } else {
        5
    };
    if ability_start_word_idx >= after_with_words.len() {
        return Ok(None);
    }
    let Some(ability_start_idx) = token_index_for_word_index(&after_with, ability_start_word_idx)
    else {
        return Ok(None);
    };
    let ability_tokens = trim_commas(&after_with[ability_start_idx..]);
    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(&ability_tokens, &after_with_words, attached_subject)?
    else {
        return Ok(None);
    };

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition: _condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::ReplaceCreatureTypes,
            base_power_toughness: Some((power, toughness)),
            granted_tail,
        },
    )))
}

pub(crate) fn parse_creatures_cant_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if is_creatures_cant_block_line_lexed(tokens) {
        return Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter: ObjectFilter::creature(),
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::cant_block())),
            condition: None,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_prevent_all_damage_dealt_to_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_damage_dealt_to_creatures_line_lexed(tokens) {
        return Ok(Some(StaticAbility::prevent_all_damage_dealt_to_creatures()));
    }
    Ok(None)
}

pub(crate) fn parse_prevent_damage_to_other_creature_you_control_put_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::prevent_damage_to_other_creature_you_control_put_counters_instead(
            crate::object::CounterType::PlusOnePlusOne,
            display_text_for_tokens(tokens, true),
        ),
    ))
}

fn parse_damage_source_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut words = strip_leading_article_word_refs(words).to_vec();
    if word_slice_last_is_any(&words, &["source", "sources"]) {
        words.pop();
    }
    if words.is_empty() {
        return Some(ObjectFilter::default());
    }

    let mut filter = ObjectFilter::default();
    let mut colors: Option<ColorSet> = None;
    for word in words {
        if matches!(word, "and" | "or") {
            continue;
        }
        if let Some(color) = parse_color(word) {
            colors = Some(colors.unwrap_or_else(ColorSet::new).union(color));
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            filter.card_types.push(card_type);
            continue;
        }
        return None;
    }
    if let Some(colors) = colors {
        filter.colors = Some(colors);
    }
    Some(filter)
}

pub(crate) fn parse_prevent_damage_to_you_from_source_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_first_is(&words, "if") {
        return Ok(None);
    }
    let Some(would_idx) =
        word_slice_find_phrase_start(&words, &["would", "deal", "damage", "to", "you"])
    else {
        return Ok(None);
    };
    if would_idx <= 1 {
        return Ok(None);
    }
    let tail = &words[would_idx + 5..];
    if tail.len() != 5
        || !word_slice_first_is(tail, "prevent")
        || !word_slice_eq(&tail[2..], &["of", "that", "damage"])
    {
        return Ok(None);
    }
    let Some(amount) = parse_number_word_i32(tail[1]).filter(|amount| *amount > 0) else {
        return Ok(None);
    };
    let Some(source_filter) = parse_damage_source_filter_words(&words[1..would_idx]) else {
        return Ok(None);
    };
    let display = format!(
        "If {}, prevent {} of that damage.",
        words[1..would_idx + 5].join(" "),
        tail[1]
    );

    Ok(Some(
        StaticAbility::prevent_damage_to_you_from_source_filter(
            amount as u32,
            source_filter,
            display,
        ),
    ))
}

pub(crate) fn parse_replace_damage_with_counters_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let soul_scar_words = [
        "if",
        "a",
        "source",
        "you",
        "control",
        "would",
        "deal",
        "noncombat",
        "damage",
        "to",
        "a",
        "creature",
        "an",
        "opponent",
        "controls",
        "put",
        "that",
        "many",
        "-1/-1",
        "counters",
        "on",
        "that",
        "creature",
        "instead",
    ];
    let soul_scar_words_no_articles = [
        "if",
        "source",
        "you",
        "control",
        "would",
        "deal",
        "noncombat",
        "damage",
        "to",
        "creature",
        "opponent",
        "controls",
        "put",
        "that",
        "many",
        "-1/-1",
        "counters",
        "on",
        "that",
        "creature",
        "instead",
    ];
    if words.as_slice() != soul_scar_words && words.as_slice() != soul_scar_words_no_articles {
        return Ok(None);
    }

    Ok(Some(StaticAbility::replace_damage_with_counters_instead(
        CounterType::MinusOneMinusOne,
        ObjectFilter::default().controlled_by(PlayerFilter::You),
        ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        Some(false),
        display_text_for_tokens(tokens, true),
    )))
}

pub(crate) fn parse_double_counters_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::runtime_backend::token_word_refs(tokens);
    if GENERIC_DOUBLE_COUNTERS_UNDER_YOUR_CONTROL_PATTERN.matches_words(&line_words) {
        return Ok(Some(StaticAbility::double_counters_replacement(
            ObjectFilter::permanent().controlled_by(PlayerFilter::You),
            None,
            display_text_for_tokens(tokens, true),
        )));
    }

    let prefix_len = 10usize;
    if !PLUS_ONE_COUNTERS_WOULD_BE_PUT_PREFIX_PATTERN.matches_words(&line_words)
        || !TWICE_THAT_MANY_PLUS_ONE_COUNTERS_TAIL_PATTERN.matches_words(&line_words)
    {
        return Ok(None);
    }

    let Some(twice_idx) = find_index(&line_words, |word| TWICE_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    if twice_idx <= prefix_len {
        return Ok(None);
    }

    let filter_tokens =
        crate::runtime_backend::lexer::synthetic_word_tokens(&line_words[prefix_len..twice_idx]);
    let filter = parse_object_filter_lexed(&filter_tokens, false)?;

    Ok(Some(StaticAbility::double_counters_replacement(
        filter,
        Some(crate::object::CounterType::PlusOnePlusOne),
        display_text_for_tokens(tokens, true),
    )))
}

pub(crate) fn parse_double_token_creation_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::runtime_backend::token_word_refs(tokens);
    if DOUBLE_TOKEN_CREATION_UNDER_YOUR_CONTROL_PATTERN.matches_words(&line_words) {
        return Ok(Some(StaticAbility::double_token_creation_replacement(
            PlayerFilter::You,
            display_text_for_tokens(tokens, true),
        )));
    }

    let add_one_prefix_len = 7usize;
    if YOU_CREATE_ONE_OR_MORE_PREFIX_PATTERN.matches_words(&line_words)
        && let Some(token_idx) = line_words[add_one_prefix_len..]
            .iter()
            .position(|word| TOKEN_OR_TOKENS_WORD_PATTERN.matches_word(word))
            .map(|idx| idx + add_one_prefix_len)
    {
        let descriptor = &line_words[add_one_prefix_len..token_idx];
        let additional_prefix_len = 7usize;
        let after_token = &line_words[token_idx + 1..];
        if !descriptor.is_empty()
            && ADDITIONAL_TOKEN_REPLACEMENT_PREFIX_PATTERN.matches_words(after_token)
            && TOKEN_OR_TOKENS_WORD_PATTERN.matches_last_word(after_token)
            && &after_token[additional_prefix_len..after_token.len() - 1] == descriptor
        {
            if !TREASURE_WORD_PATTERN.matches_words(descriptor) {
                return Ok(None);
            }
            let mut token_filter = ObjectFilter::default().token();
            for word in descriptor {
                if let Some(card_type) = parse_card_type(word) {
                    token_filter = token_filter.with_type(card_type);
                } else if let Some(subtype) = parse_subtype_flexible(word) {
                    token_filter = token_filter.with_subtype(subtype);
                } else {
                    return Ok(None);
                }
            }
            return Ok(Some(StaticAbility::add_token_creation_replacement(
                PlayerFilter::You,
                token_filter,
                ironsmith_core::AdditionalTokenKind::Treasure,
                1,
                display_text_for_tokens(tokens, true),
            )));
        }
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_combat_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_combat_damage_to_source_line_lexed(tokens) {
        return Ok(Some(StaticAbility::prevent_all_combat_damage_to_self()));
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_combat_damage_to_matching_permanents_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_all_combat_damage_to_matching_permanents_line_lexed(tokens) {
        return Ok(None);
    }
    let Some((_phrase_idx, phrase_end)) = find_token_word_sequence_span(
        tokens,
        &[
            "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to",
        ],
    ) else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&tokens[phrase_end..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "prevent-all combat damage static line missing target filter (clause: '{}')",
            render_token_slice(tokens)
        )));
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    Ok(Some(
        StaticAbility::prevent_all_combat_damage_to_permanents_matching(filter),
    ))
}

pub(crate) fn parse_during_your_turn_prevent_all_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_during_your_turn_prevent_all_damage_to_source_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self()
                .with_condition(crate::ConditionExpr::YourTurn),
        ));
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_noncombat_damage_to_other_creatures_you_control_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_noncombat_damage_to_other_creatures_you_control(),
        ));
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_damage_to_source_by_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_damage_to_source_by_creatures_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self_by_creatures(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_may_choose_not_to_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !YOU_MAY_CHOOSE_NOT_TO_UNTAP_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    if !DURING_YOUR_UNTAP_STEP_TAIL_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    if words.len() <= 10 {
        return Ok(None);
    }

    let subject_words = &words[6..words.len() - 4];
    let subject_allowed = MAY_CHOOSE_NOT_UNTAP_SOURCE_SUBJECT_PATTERN.matches_words(subject_words);
    if !subject_allowed {
        return Ok(None);
    }

    let subject = subject_words.join(" ");
    Ok(Some(
        StaticAbility::may_choose_not_to_untap_during_untap_step(subject),
    ))
}

pub(crate) fn parse_untap_during_each_other_players_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = split_untap_each_other_players_untap_step_line_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.subject_tokens);
    if subject_tokens.is_empty() {
        let line_words = crate::runtime_backend::token_word_refs(tokens);
        return Err(CardTextError::ParseError(format!(
            "missing subject in other-players untap ability (clause: '{}')",
            line_words.join(" ")
        )));
    }

    let filter = parse_object_filter(&subject_tokens, false)?;
    let subject_text = crate::runtime_backend::token_word_refs(&subject_tokens).join(" ");
    Ok(Some(
        StaticAbility::untap_during_each_other_players_untap_step(
            filter,
            format!("Untap all {subject_text} during each other player's untap step"),
        ),
    ))
}

pub(crate) fn parse_doesnt_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    match parse_doesnt_untap_during_untap_step_spec_lexed(tokens) {
        Some(DoesntUntapDuringUntapStepSpec::Source { tail_tokens }) => {
            let clause_words = crate::runtime_backend::token_word_refs(tokens);
            let tail_tokens = trim_commas(tail_tokens);
            if tail_tokens.is_empty() {
                return Ok(Some(
                    StaticAbilityAst::Static(StaticAbility::doesnt_untap()),
                ));
            }
            let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
            if IF_WORD_PATTERN.matches_first_word(&tail_words) {
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after untap-step if-clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                let condition = parse_static_condition_clause(&condition_tokens)?;
                return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                    condition,
                }));
            }

            Err(CardTextError::ParseError(format!(
                "unsupported trailing untap-step clause (clause: '{}')",
                clause_words.join(" ")
            )))
        }
        Some(DoesntUntapDuringUntapStepSpec::Attached { subject_tokens }) => {
            let subject = crate::runtime_backend::token_word_refs(subject_tokens).join(" ");
            let text = format!("{subject} doesnt untap during its controllers untap step");
            Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                display: text,
                condition: None,
            }))
        }
        None => Ok(None),
    }
}

pub(crate) fn parse_flying_restriction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(match parse_flying_block_restriction_line_lexed(tokens) {
        Some(FlyingBlockRestrictionKind::FlyingOnly) => {
            Some(StaticAbility::flying_only_restriction())
        }
        Some(FlyingBlockRestrictionKind::FlyingOrReach) => {
            Some(StaticAbility::flying_restriction())
        }
        None => None,
    })
}

pub(crate) fn parse_can_block_only_flying_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_can_block_only_flying_line_lexed(tokens) {
        return Ok(Some(StaticAbility::can_block_only_flying()));
    }

    Ok(None)
}

pub(crate) fn parse_can_block_subtype_as_though_reach_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(parse_can_block_subtype_as_though_reach_line_lexed(tokens)
        .map(StaticAbility::can_block_subtype_as_though_reach))
}

pub(crate) fn parse_assign_damage_as_unblocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_may_assign_damage_as_unblocked_line_lexed(tokens) {
        return Ok(Some(StaticAbility::may_assign_damage_as_unblocked()));
    }

    Ok(None)
}

pub(crate) fn parse_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    };
    let Some((_, head_tokens)) = super::grammar::primitives::strip_lexed_suffix_phrases(
        tokens,
        &[
            &["where", "x", "is", "that", "spell's", "mana", "value"],
            &["where", "x", "is", "that", "spells", "mana", "value"],
        ],
    ) else {
        return Ok(None);
    };
    let head_tokens = trim_lexed_commas(head_tokens);
    let words = super::lexer::parser_token_word_refs(head_tokens);
    if !MANA_VALUE_INSTEAD_OF_MANA_COST_GRANT_PREFIX_PATTERN.matches_words(words.as_slice()) {
        return Ok(None);
    }

    let Some(for_idx) = find_index(head_tokens, |token| FOR_WORD_PATTERN.matches_token(token))
    else {
        return Ok(None);
    };
    let subject_tokens = trim_lexed_commas(head_tokens.get(for_idx + 1..).unwrap_or_default());
    let subject_words = crate::runtime_backend::token_word_refs(subject_tokens);
    if subject_tokens.is_empty() || !SPELL_OR_SPELLS_CONTAINS_PATTERN.matches_words(&subject_words)
    {
        return Ok(None);
    }

    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(subject_tokens);
    Ok(Some(StaticAbility::grants(crate::grant::GrantSpec::new(
        crate::grant::Grantable::mana_value_as_generic_from_hand(),
        filter,
        Zone::Hand,
    ))))
}

pub(crate) fn parse_life_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    };

    let words = super::lexer::parser_token_word_refs(tokens);
    if !LIFE_MANA_VALUE_INSTEAD_OF_MANA_COST_GRANT_PREFIX_PATTERN.matches_words(words.as_slice()) {
        return Ok(None);
    }

    let Some(cast_idx) = find_index(tokens, |token| CAST_WORD_PATTERN.matches_token(token)) else {
        return Ok(None);
    };
    let Some(by_idx) = find_index(tokens, |token| BY_WORD_PATTERN.matches_token(token)) else {
        return Ok(None);
    };
    if by_idx <= cast_idx + 1 {
        return Ok(None);
    }
    let subject_tokens = trim_lexed_commas(tokens.get(cast_idx + 1..by_idx).unwrap_or_default());
    let subject_words = crate::runtime_backend::token_word_refs(subject_tokens);
    if subject_tokens.is_empty() || !SPELL_OR_SPELLS_CONTAINS_PATTERN.matches_words(&subject_words)
    {
        return Ok(None);
    }

    let tail_words =
        crate::runtime_backend::token_word_refs(tokens.get(by_idx + 1..).unwrap_or_default());
    if !LIFE_MANA_VALUE_INSTEAD_OF_MANA_COST_TAIL_PATTERN.matches_words(&tail_words) {
        return Ok(None);
    }

    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(subject_tokens);
    Ok(Some(StaticAbility::grants(crate::grant::GrantSpec::new(
        crate::grant::Grantable::life_equal_mana_value_from_hand(Some(
            crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns,
        )),
        filter,
        Zone::Hand,
    ))))
}

pub(crate) fn parse_fixed_mana_cost_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    };
    let words = super::lexer::parser_token_word_refs(tokens);
    if !YOU_MAY_PAY_PREFIX_PATTERN.matches_words(words.as_slice())
        || words
            .iter()
            .any(|word| WHERE_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let Some((mana_cost, consumed)) =
        leading_mana_cost_from_tokens(tokens.get(3..).unwrap_or_default())
    else {
        return Ok(None);
    };

    let tail_tokens = tokens.get(3 + consumed..).unwrap_or_default();
    let tail_words = super::lexer::parser_token_word_refs(tail_tokens);
    if !RATHER_THAN_PAY_MANA_COST_FOR_PREFIX_PATTERN.matches_words(tail_words.as_slice()) {
        return Ok(None);
    }

    let subject_tokens = trim_lexed_commas(tail_tokens.get(7..).unwrap_or_default());
    let subject_words = crate::runtime_backend::token_word_refs(subject_tokens);
    if subject_tokens.is_empty() || !SPELL_OR_SPELLS_CONTAINS_PATTERN.matches_words(&subject_words)
    {
        return Ok(None);
    }

    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(subject_tokens);
    Ok(Some(StaticAbility::grants(
        crate::grant::GrantSpec::cast_from_hand_for_alternative_mana_cost_matching(
            filter, mana_cost,
        ),
    )))
}

pub(crate) fn parse_grant_flash_to_noncreature_spells_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if spec == crate::grant::GrantSpec::flash_to_noncreature_spells() => {
            Ok(Some(StaticAbility::grants(spec)))
        }
        _ => Ok(None),
    }
}

fn static_grant_beneficiary(player: crate::cards::builders::PlayerAst) -> Option<PlayerFilter> {
    match player {
        crate::cards::builders::PlayerAst::You | crate::cards::builders::PlayerAst::Implicit => {
            Some(PlayerFilter::You)
        }
        crate::cards::builders::PlayerAst::Any => Some(PlayerFilter::Any),
        _ => None,
    }
}

pub(crate) fn parse_you_may_cast_exile_counter_cards_with_mana_permission_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let cast_prefix_len = CAST_EXILE_COUNTER_CARDS_PREFIX_PATTERN.matched_prefix_len(&words);
    let is_cast_from_exile_family = cast_prefix_len.is_some();
    let is_play_lands_and_cast_noncreature_family =
        PLAY_LANDS_CAST_NONCREATURE_EXILED_PREFIX_PATTERN.matches_words(&words);
    if !is_cast_from_exile_family && !is_play_lands_and_cast_noncreature_family {
        return Ok(None);
    }

    let Some(and_idx) =
        keyword_find_prefix_shape_start(&words, &AND_YOU_MAY_SPEND_MANA_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    let (counter_start_idx, counters_idx) =
        if let Some(with_idx) = WITH_WORD_PATTERN.find_word(&words[..and_idx]) {
            let Some(counters_idx) = words[with_idx + 1..and_idx]
                .iter()
                .position(|word| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word))
                .map(|offset| with_idx + 1 + offset)
            else {
                return Ok(None);
            };
            (with_idx + 1, counters_idx)
        } else if is_play_lands_and_cast_noncreature_family {
            let Some(that_have_idx) =
                keyword_find_prefix_shape_start(&words[..and_idx], &THAT_HAVE_PREFIX_PATTERN)
            else {
                return Ok(None);
            };
            let that_have_prefix_len = 2usize;
            let Some(counters_idx) = words[that_have_idx + that_have_prefix_len..and_idx]
                .iter()
                .position(|word| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word))
                .map(|offset| that_have_idx + that_have_prefix_len + offset)
            else {
                return Ok(None);
            };
            (that_have_idx + that_have_prefix_len, counters_idx)
        } else {
            return Ok(None);
        };
    if counters_idx + 3 > and_idx
        || !ON_THEM_PREFIX_PATTERN.matches_words(&words[counters_idx + 1..])
    {
        return Ok(None);
    }

    let owner = if is_play_lands_and_cast_noncreature_family {
        None
    } else {
        let owner_words =
            &words[cast_prefix_len.unwrap_or_default()..counter_start_idx.saturating_sub(1)];
        if owner_words.is_empty() {
            None
        } else if OPPONENT_OWNED_PREFIX_PATTERN.matches_words(owner_words) {
            Some(PlayerFilter::Opponent)
        } else {
            return Ok(None);
        }
    };

    let counter_tokens = &tokens[counter_start_idx..counters_idx + 1];
    let Some(counter_type) = parse_counter_type_from_tokens(counter_tokens) else {
        return Ok(None);
    };

    let spend_words = &words[and_idx..];
    let uses_snow_sources =
        SPEND_SNOW_MANA_AS_ANY_COLOR_FOR_THOSE_SPELLS_PATTERN.matches_words(spend_words);
    let valid_spend_suffix = uses_snow_sources
        || SPEND_MANA_AS_ANY_COLOR_FOR_THOSE_SPELLS_PATTERN.matches_words(spend_words);
    if !valid_spend_suffix {
        return Ok(None);
    }

    let mut base_filter = ObjectFilter {
        zone: Some(Zone::Exile),
        owner,
        with_counter: Some(crate::filter::CounterConstraint::Typed(counter_type)),
        ..ObjectFilter::default()
    };
    if is_play_lands_and_cast_noncreature_family {
        base_filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
    }

    let mut filter = if is_play_lands_and_cast_noncreature_family {
        ObjectFilter {
            any_of: vec![
                ObjectFilter {
                    card_types: vec![CardType::Land],
                    ..base_filter.clone()
                },
                ObjectFilter {
                    excluded_card_types: vec![CardType::Creature, CardType::Land],
                    ..base_filter.clone()
                },
            ],
            ..ObjectFilter::default()
        }
    } else {
        ObjectFilter {
            excluded_card_types: vec![CardType::Land],
            ..base_filter
        }
    };
    filter.has_mana_cost = false;

    let grant = StaticAbility::grants(
        crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            filter.clone(),
            Zone::Exile,
        )
        .with_beneficiary(PlayerFilter::You),
    );
    let permission = if uses_snow_sources {
        crate::effect::ManaSpendPermission::any_color_from_sources_for_casting_matching(
            PlayerFilter::You,
            filter,
            ObjectFilter::default().with_supertype(Supertype::Snow),
        )
    } else {
        crate::effect::ManaSpendPermission::any_color_for_casting_matching(
            PlayerFilter::You,
            filter,
        )
    };
    let mana_permission =
        StaticAbility::mana_spend_permission(permission, render_token_slice(tokens));

    Ok(Some(vec![grant, mana_permission]))
}

pub(crate) fn parse_surveilled_graveyard_play_life_cost_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let normalized = words
        .iter()
        .map(|word| word.replace(['\'', '’'], ""))
        .collect::<Vec<_>>();
    let refs = normalized.iter().map(String::as_str).collect::<Vec<_>>();
    if !matches!(
        refs.as_slice(),
        [
            "you",
            "may",
            "play",
            "lands",
            "and",
            "cast",
            "spells",
            "from",
            "among",
            "cards",
            "in",
            "your",
            "graveyard",
            "youve",
            "surveilled",
            "this",
            "turn",
            "if",
            "you",
            "cast",
            "a",
            "spell",
            "this",
            "way",
            "you",
            "pay",
            "life",
            "equal",
            "to",
            "its",
            "mana",
            "value",
            "rather",
            "than",
            "paying",
            "its",
            "mana",
            "cost"
        ]
    ) {
        return Ok(None);
    }

    let base_filter = ObjectFilter {
        zone: Some(Zone::Graveyard),
        owner: Some(PlayerFilter::You),
        surveilled_this_turn: true,
        ..ObjectFilter::default()
    };
    let mut spell_filter = base_filter.clone();
    spell_filter.excluded_card_types.push(CardType::Land);

    Ok(Some(vec![
        StaticAbility::grants(
            crate::grant::GrantSpec::new(
                crate::grant::Grantable::play_from(),
                base_filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        ),
        StaticAbility::grants(
            crate::grant::GrantSpec::new(
                crate::grant::Grantable::life_equal_mana_value_from_zone(Zone::Graveyard, None),
                spell_filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        ),
    ]))
}

pub(crate) fn parse_you_may_static_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if SOURCE_LINKED_EXILE_CAST_PREFIX_PATTERN.matches_words(&words)
        && ANY_MANA_CAST_SUFFIX_PATTERN.matches_words(&words)
        && words.len() > 19 + 11
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.owner = Some(PlayerFilter::NotYou);
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(Some(StaticAbility::grants(
            crate::grant::GrantSpec::new(crate::grant::Grantable::play_from(), filter, Zone::Exile)
                .with_beneficiary(PlayerFilter::Any),
        )));
    }

    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) => {
            let clause_words = crate::runtime_backend::token_word_refs(tokens);
            let singular_spell = CAST_SINGLE_SPELL_PATTERN.matches_words(&clause_words);
            if singular_spell
                && spec.zone == Zone::Hand
                && matches!(
                    &spec.grantable,
                    crate::grant::Grantable::AlternativeCast(method)
                        if method.cast_from_zone() == Zone::Hand
                            && method.mana_cost().is_none()
                            && method.non_mana_costs().is_empty()
                )
            {
                return Ok(None);
            }
            Ok(static_grant_beneficiary(player)
                .map(|beneficiary| StaticAbility::grants(spec.with_beneficiary(beneficiary))))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_play_from_permission_with_haste_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [permission_sentence, haste_sentence] = sentences.as_slice() else {
        return Ok(None);
    };

    let haste_words = parser_token_word_refs(haste_sentence);
    if haste_words
        != [
            "if", "you", "cast", "a", "creature", "spell", "this", "way", "it", "gains", "haste",
            "until", "end", "of", "turn",
        ]
    {
        return Ok(None);
    }

    match parse_permission_clause_spec(permission_sentence)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(spec.grantable, crate::grant::Grantable::PlayFrom)
            && spec.filter.card_types.as_slice() == [CardType::Creature] =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(
                    spec.with_beneficiary(beneficiary)
                        .with_cast_this_way_grant(StaticAbility::haste()),
                )
            }))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_you_may_look_top_card_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_may_look_top_card_any_time_line_lexed(tokens) {
        return Ok(Some(StaticAbility::look_at_top_card_of_library()));
    }
    Ok(None)
}

pub(crate) fn parse_you_may_look_face_down_creatures_you_dont_control_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_may_look_face_down_creatures_you_dont_control_any_time_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::look_at_face_down_creatures_you_dont_control(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_players_play_top_card_libraries_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_players_play_top_card_libraries_revealed_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::all_players_look_at_top_cards_of_libraries(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_play_top_card_your_library_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_play_top_card_your_library_revealed_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::all_players_look_at_your_top_library_card(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_your_opponents_play_with_hands_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_your_opponents_play_with_hands_revealed_line_lexed(tokens) {
        return Ok(Some(StaticAbility::opponents_play_with_hands_revealed()));
    }
    Ok(None)
}

pub(crate) fn parse_control_opponents_while_searching_libraries_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if CONTROL_OPPONENTS_WHILE_SEARCHING_PATTERN.matches_words(&words) {
        return Ok(Some(
            StaticAbility::control_opponents_while_searching_libraries(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_opponent_search_exile_found_cards_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if OPPONENT_SEARCH_EXILE_FOUND_CARDS_PATTERN.matches_words(&words) {
        return Ok(Some(StaticAbility::opponent_search_exile_found_cards()));
    }
    Ok(None)
}

pub(crate) fn parse_cast_this_card_from_library_while_searching_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if CAST_THIS_CARD_FROM_LIBRARY_WHILE_SEARCHING_PATTERN.matches_words(&words) {
        return Ok(Some(
            StaticAbility::cast_this_card_from_library_while_searching(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_cast_this_spell_as_though_it_had_flash_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_cast_this_spell_as_though_it_had_flash_line_lexed(tokens) {
        return Ok(Some(StaticAbility::flash()));
    }
    Ok(None)
}

pub(crate) fn parse_attacks_each_combat_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if ATTACHED_CONTROLLER_ATTACK_EACH_COMBAT_PATTERN.matches_words(&words) {
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::all_creatures_attack_attached_controller_each_combat_if_able(),
        )));
    }

    let Some(attack_idx) = find_index(&words, |word| {
        ATTACK_OR_ATTACKS_WORD_PATTERN.matches_word(word)
    }) else {
        return Ok(None);
    };
    if !ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches_words(&words[attack_idx..]) {
        return Ok(None);
    }

    if attack_idx == 0 {
        return Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())));
    }

    let subject_tokens = trim_commas(&tokens[..attack_idx]);
    if subject_tokens.is_empty() {
        return Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())));
    }
    let subject = parse_anthem_subject(&subject_tokens)?;
    match subject {
        AnthemSubjectAst::Source => {
            Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())))
        }
        AnthemSubjectAst::Filter(filter) => Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
            condition: None,
        })),
    }
}

pub(crate) fn parse_additional_land_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !YOU_MAY_PLAY_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let mut count_word_idx = 3;
    if UP_TO_PREFIX_PATTERN.matches_words(&words[count_word_idx..]) {
        count_word_idx += 2;
    }

    let mut count_token_idx = None;
    let mut seen_word_idx = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_none() {
            continue;
        }
        if seen_word_idx == count_word_idx {
            count_token_idx = Some(idx);
            break;
        }
        seen_word_idx += 1;
    }
    let Some(count_token_idx) = count_token_idx else {
        return Ok(None);
    };
    let Some((count, used)) = parse_number(&tokens[count_token_idx..]) else {
        return Ok(None);
    };
    let rest_word_idx = count_word_idx + used;
    if rest_word_idx >= words.len() {
        return Ok(None);
    }
    let rest_words = &words[rest_word_idx..];
    if !ADDITIONAL_LAND_PLAY_TAIL_PATTERN.matches_words(rest_words) {
        return Ok(None);
    }
    if count == 0 {
        return Ok(None);
    }

    Ok(Some(vec![StaticAbility::additional_land_plays(count)]))
}

pub(crate) fn parse_play_lands_from_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_play_lands_from_graveyard_line_lexed(tokens) {
        let spec = crate::grant::GrantSpec::play_lands_from_graveyard();
        return Ok(Some(StaticAbility::grants(spec)));
    }
    Ok(None)
}

pub(crate) fn parse_graveyard_cards_have_retrace_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(have_idx) = find_index(&words, |word| HAVE_OR_HAS_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    if !RETRACE_TAIL_PATTERN.matches_words(&words[have_idx + 1..]) {
        return Ok(None);
    }
    let mut prefix = &words[..have_idx];
    if prefix
        .first()
        .is_some_and(|word| EACH_WORD_PATTERN.matches_word(word))
    {
        prefix = &prefix[1..];
    }
    if !STATIC_IN_YOUR_GRAVEYARD_SUFFIX_PATTERN.matches_words(prefix) || prefix.len() <= 3 {
        return Ok(None);
    }

    let subject = &prefix[..prefix.len() - 3];
    let Some(card_types) = parse_retrace_grant_card_types(subject) else {
        return Ok(None);
    };
    let mut filter = ObjectFilter {
        card_types,
        owner: Some(PlayerFilter::You),
        ..ObjectFilter::default()
    };
    filter.zone = Some(Zone::Graveyard);
    let spec = crate::grant::GrantSpec::new(
        crate::grant::Grantable::retrace_from_cards_mana_cost(),
        filter,
        Zone::Graveyard,
    );
    Ok(Some(StaticAbility::grants(spec)))
}

fn parse_retrace_grant_card_types(words: &[&str]) -> Option<Vec<CardType>> {
    let mut card_types = Vec::new();
    for word in words {
        match *word {
            "instant" | "instants" => {
                if !card_types.contains(&CardType::Instant) {
                    card_types.push(CardType::Instant);
                }
            }
            "sorcery" | "sorceries" => {
                if !card_types.contains(&CardType::Sorcery) {
                    card_types.push(CardType::Sorcery);
                }
            }
            "and" | "or" | "card" | "cards" => {}
            _ => return None,
        }
    }
    (!card_types.is_empty()).then_some(card_types)
}

pub(crate) fn parse_cast_spells_from_hand_without_paying_mana_costs_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if keyword_find_prefix_shape_start(&clause_words, &CAST_A_OR_ONE_SPELL_PATTERN).is_some() {
        return Ok(None);
    }
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if spec.zone == Zone::Hand
            && matches!(
                &spec.grantable,
                crate::grant::Grantable::AlternativeCast(method)
                    if method.cast_from_zone() == Zone::Hand
                        && method.mana_cost().is_none()
                        && method.non_mana_costs().is_empty()
            ) =>
        {
            Ok(Some(StaticAbility::grants(spec)))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_pt_modifier(raw: &str) -> Result<(i32, i32), CardTextError> {
    let (power_raw, toughness_raw) = split_pt_modifier_components(raw)?;
    let power_str = strip_leading_plus_char(power_raw);
    let toughness_str = strip_leading_plus_char(toughness_raw);
    let power = power_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power modifier".to_string()))?;
    let toughness = toughness_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid toughness modifier".to_string()))?;
    Ok((power, toughness))
}

pub(crate) fn parse_signed_pt_component(raw: &str) -> Result<Value, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CardTextError::ParseError(
            "missing power/toughness component".to_string(),
        ));
    }

    let (sign, value_text) = split_signed_pt_component(trimmed);

    if pt_component_is_x(value_text) {
        return Ok(match sign {
            1 => Value::X,
            -1 => Value::XTimes(-1),
            _ => Value::XTimes(sign),
        });
    }

    let parsed = value_text
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power/toughness component".to_string()))?;
    Ok(Value::Fixed(parsed * sign))
}

pub(crate) fn parse_pt_modifier_values(raw: &str) -> Result<(Value, Value), CardTextError> {
    let (power_raw, toughness_raw) = split_pt_modifier_components(raw)?;
    let power = parse_signed_pt_component(power_raw)?;
    let toughness = parse_signed_pt_component(toughness_raw)?;
    Ok((power, toughness))
}

fn split_pt_modifier_components(raw: &str) -> Result<(&str, &str), CardTextError> {
    str_split_once_char(raw, '/')
        .ok_or_else(|| CardTextError::ParseError("missing power/toughness modifier".to_string()))
}

fn strip_leading_plus_char(raw: &str) -> &str {
    let trimmed = raw.trim();
    let mut chars = trimmed.chars();
    if chars.next().is_some_and(|ch| ch == '+') {
        chars.as_str()
    } else {
        trimmed
    }
}

fn split_signed_pt_component(trimmed: &str) -> (i32, &str) {
    let mut chars = trimmed.chars();
    match chars.next() {
        Some('+') => (1, chars.as_str()),
        Some('-' | '−') => (-1, chars.as_str()),
        _ => (1, trimmed),
    }
}

fn pt_component_is_x(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|ch| matches!(ch, 'x' | 'X')) && chars.next().is_none()
}

pub(crate) fn parse_no_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_no_maximum_hand_size_line_lexed(tokens) {
        return Ok(Some(StaticAbility::no_maximum_hand_size()));
    }
    Ok(None)
}

pub(crate) fn parse_can_be_your_commander_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_can_be_your_commander_line_lexed(tokens) {
        return Ok(Some(StaticAbility::can_be_commander()));
    }
    Ok(None)
}

pub(crate) fn parse_reduced_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let mut min_card_types_condition: Option<u32> = None;
    let mut line_words = crate::runtime_backend::token_word_refs(tokens);
    if line_words.is_empty() {
        return Ok(None);
    }

    let working_tokens_storage =
        if MAX_HAND_SIZE_AS_LONG_AS_PREFIX_PATTERN.matches_words(&line_words) {
            let (condition_end_idx, remainder_start_idx) =
                if let Some(comma_idx) = find_index(tokens, |token| token.is_comma()) {
                    if comma_idx <= 3 {
                        return Ok(None);
                    }
                    (comma_idx, comma_idx + 1)
                } else {
                    let mut split_word_idx = None;
                    for word_idx in 4..line_words.len() {
                        let tail = &line_words[word_idx..];
                        let Some((_, prefix_len)) = max_hand_size_subject_prefix(tail) else {
                            continue;
                        };
                        if tail
                            .get(prefix_len..)
                            .is_some_and(|tail| MAX_HAND_SIZE_IS_PATTERN.matches_words(tail))
                        {
                            split_word_idx = Some(word_idx);
                            break;
                        }
                    }
                    let Some(split_word_idx) = split_word_idx else {
                        return Ok(None);
                    };
                    let split_token_idx = token_index_for_word_index(tokens, split_word_idx)
                        .ok_or_else(|| {
                            CardTextError::ParseError(format!(
                                "unable to map delirium hand-size subject split (clause: '{}')",
                                line_words.join(" ")
                            ))
                        })?;
                    (split_token_idx, split_token_idx)
                };

            let condition_tokens = trim_commas(&tokens[3..condition_end_idx]);
            let Some((metric, threshold)) =
                parse_graveyard_metric_threshold_condition(&condition_tokens)?
            else {
                return Ok(None);
            };
            if metric != crate::static_abilities::GraveyardCountMetric::CardTypes {
                return Ok(None);
            }
            min_card_types_condition = Some(threshold);
            Some(trim_commas(&tokens[remainder_start_idx..]))
        } else {
            None
        };
    let working_tokens = working_tokens_storage.as_deref().unwrap_or(tokens);
    line_words = crate::runtime_backend::token_word_refs(working_tokens);
    if line_words.is_empty() {
        return Ok(None);
    }

    let Some((player, mut idx)) = max_hand_size_subject_prefix(&line_words) else {
        return Ok(None);
    };

    if line_words
        .get(idx..)
        .is_some_and(|tail| MAX_HAND_SIZE_REDUCED_PATTERN.matches_words(tail))
    {
        idx += 5;
        if !line_words
            .get(idx)
            .is_some_and(|word| BY_WORD_PATTERN.matches_word(word))
        {
            return Ok(None);
        }
        idx += 1;

        let Some(amount_word) = line_words.get(idx) else {
            return Err(CardTextError::ParseError(format!(
                "missing maximum-hand-size reduction amount (clause: '{}')",
                line_words.join(" ")
            )));
        };
        let Some(amount) = parse_named_number(amount_word) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported maximum-hand-size reduction amount '{}' (clause: '{}')",
                amount_word,
                line_words.join(" ")
            )));
        };
        idx += 1;

        if idx != line_words.len() {
            return Ok(None);
        }

        return Ok(Some(StaticAbility::reduce_maximum_hand_size(
            player, amount,
        )));
    }

    if line_words
        .get(idx..)
        .is_some_and(|tail| MAX_HAND_SIZE_IS_PATTERN.matches_words(tail))
    {
        idx += 4;

        if line_words
            .get(idx..idx + 10)
            .is_some_and(|tail| MAX_HAND_SIZE_SEVEN_MINUS_CARD_TYPES_PATTERN.matches_words(tail))
        {
            idx += 10;
            if idx != line_words.len() {
                return Ok(None);
            }
            return Ok(Some(
                StaticAbility::max_hand_size_seven_minus_your_graveyard_card_types(
                    player,
                    min_card_types_condition.unwrap_or(0),
                ),
            ));
        }

        let Some(amount_word) = line_words.get(idx) else {
            return Err(CardTextError::ParseError(format!(
                "missing maximum-hand-size value (clause: '{}')",
                line_words.join(" ")
            )));
        };
        let Some(amount) = parse_named_number(amount_word) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported maximum-hand-size value '{}' (clause: '{}')",
                amount_word,
                line_words.join(" ")
            )));
        };
        idx += 1;
        if idx != line_words.len() {
            return Ok(None);
        }

        return Ok(Some(StaticAbility::set_maximum_hand_size(player, amount)));
    }
    Ok(None)
}

pub(crate) fn parse_effect_discard_to_library_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_effect_discard_to_library_replacement_line_lexed(tokens) {
        return Ok(Some(StaticAbility::effect_discard_to_library_replacement()));
    }

    if is_opponent_effect_discard_this_to_battlefield_replacement_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::opponent_effect_discard_this_to_battlefield_replacement(),
        ));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replace_exile_top_face_down_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replace_exile_top_face_down_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_exile_top_face_down()));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replacement_exile_top_and_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if words.len() < 20 {
        return Ok(None);
    }

    if !DRAW_REPLACEMENT_EXILE_TOP_PLAY_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let Some(count_word) = words.get(9).copied() else {
        return Ok(None);
    };
    let Some(count) = parse_named_number(count_word) else {
        return Ok(None);
    };

    if !words
        .get(10)
        .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    if !DRAW_REPLACEMENT_EXILE_TOP_PLAY_TAIL_PATTERN.matches_words(&words[11..]) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::draw_replacement_exile_top_and_play(
        count,
    )))
}

pub(crate) fn parse_draw_replacement_double_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replacement_double_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_double()));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replacement_skip_empty_library_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replacement_skip_empty_library_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_skip_empty_library()));
    }

    Ok(None)
}

fn parse_conditional_draw_replacement_amount(word: &str) -> Option<u32> {
    parse_named_number(word)
}

pub(crate) fn parse_conditional_draw_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let draw_subject_len =
        if CONDITIONAL_DRAW_REPLACEMENT_A_CARD_PREFIX_PATTERN.matches_words(&words) {
            7
        } else if CONDITIONAL_DRAW_REPLACEMENT_CARD_PREFIX_PATTERN.matches_words(&words) {
            6
        } else {
            return Ok(None);
        };

    let Some(instead_idx) = find_index(&words[draw_subject_len..], |word| {
        INSTEAD_WORD_PATTERN.matches_word(word)
    }) else {
        return Ok(None);
    };
    let instead_idx = draw_subject_len + instead_idx;
    let Some(condition_token_start) = token_index_for_word_index(tokens, draw_subject_len) else {
        return Ok(None);
    };
    let Some(condition_token_end) = token_index_for_word_index(tokens, instead_idx) else {
        return Ok(None);
    };
    let Some(no_cards_condition) =
        crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_condition(
            tokens
                .get(condition_token_start..condition_token_end)
                .unwrap_or_default(),
        )
    else {
        return Ok(None);
    };
    if no_cards_condition.player != PlayerFilter::You || !no_cards_condition.is_no_cards_in_hand() {
        return Ok(None);
    }

    let effect_words = &words[instead_idx + 1..];
    let draw_idx = if effect_words
        .first()
        .is_some_and(|word| YOU_WORD_PATTERN.matches_word(word))
    {
        1
    } else {
        0
    };
    if !effect_words
        .get(draw_idx)
        .is_some_and(|word| DRAW_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    let Some(draw_count_word) = effect_words.get(draw_idx + 1).copied() else {
        return Ok(None);
    };
    let Some(draw_count) = parse_conditional_draw_replacement_amount(draw_count_word) else {
        return Ok(None);
    };
    if !effect_words
        .get(draw_idx + 2)
        .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let mut next_idx = draw_idx + 3;
    if effect_words
        .get(next_idx)
        .is_some_and(|word| INSTEAD_WORD_PATTERN.matches_word(word))
    {
        next_idx += 1;
    }

    let mut replacement_effects = vec![Effect::draw(draw_count as i32)];
    let mut life_loss = None;
    if next_idx < effect_words.len() {
        let tail = &effect_words[next_idx..];
        if tail.len() != 5 || !CONDITIONAL_DRAW_LIFE_LOSS_TAIL_PATTERN.matches_words(tail) {
            return Ok(None);
        }
        let Some(amount) = parse_conditional_draw_replacement_amount(tail[3]) else {
            return Ok(None);
        };
        life_loss = Some(amount);
        replacement_effects.push(Effect::lose_life(amount as i32));
    }

    let draw_amount_text = match draw_count {
        1 => "a".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        _ => draw_count.to_string(),
    };
    let draw_card_text = if draw_count == 1 { "card" } else { "cards" };
    let mut display = format!(
        "If you would draw a card while you have no cards in hand, instead you draw {draw_amount_text} {draw_card_text}"
    );
    if let Some(amount) = life_loss {
        display.push_str(&format!(" and you lose {amount} life"));
    }
    display.push('.');

    Ok(Some(StaticAbility::conditional_draw_replacement(
        Condition::Not(Box::new(Condition::CardsInHandOrMore(1))),
        replacement_effects,
        display,
    )))
}

fn keyword_action_replacement_subject_explores(words: &[&str]) -> bool {
    EXPLORE_REPLACEMENT_SUBJECT_PATTERN.matches_words(words)
}

pub(crate) fn parse_keyword_action_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = parser_token_word_refs(tokens);
    let proliferate_display = render_token_slice(tokens);
    if YOU_PROLIFERATE_TWICE_INSTEAD_PATTERN.matches_words(&line_words) {
        return Ok(Some(StaticAbility::keyword_action_replacement(
            crate::events::KeywordActionKind::Proliferate,
            ObjectFilter::default().controlled_by(PlayerFilter::You),
            vec![Effect::proliferate(2)],
            proliferate_display,
        )));
    }
    if OPPONENT_PROLIFERATES_TWICE_INSTEAD_PATTERN.matches_words(&line_words) {
        return Ok(Some(StaticAbility::keyword_action_replacement(
            crate::events::KeywordActionKind::Proliferate,
            ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
            vec![Effect::proliferate(2)],
            proliferate_display,
        )));
    }

    if !CONTROLLED_CREATURE_EXPLORE_REPLACEMENT_PREFIX_PATTERN.matches_words(&line_words) {
        return Ok(None);
    }
    let tail = &line_words[8..];
    let explored_creature = ChooseSpec::tagged(IT_TAG);
    let explore_effect = || Effect::explore(explored_creature.clone());
    let source_filter = ObjectFilter::creature().controlled_by(PlayerFilter::You);
    let display = render_token_slice(tokens);

    if let Some(then_idx) = THEN_WORD_PATTERN.find_word(tail)
        && YOU_SCRY_PREFIX_PATTERN.matches_words(tail)
        && keyword_action_replacement_subject_explores(&tail[then_idx + 1..])
    {
        let value_tokens = &tail[2..then_idx];
        let (count, used) = parse_value_expr_words(value_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported scry amount in keyword-action replacement (clause: '{}')",
                render_token_slice(tokens)
            ))
        })?;
        if used != value_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported scry amount in keyword-action replacement (clause: '{}')",
                render_token_slice(tokens)
            )));
        }
        return Ok(Some(StaticAbility::keyword_action_replacement(
            crate::events::KeywordActionKind::Explore,
            source_filter,
            vec![Effect::scry(count), explore_effect()],
            display,
        )));
    }

    if EXPLORES_TWICE_TAIL_PATTERN.matches_words(tail) {
        return Ok(Some(StaticAbility::keyword_action_replacement(
            crate::events::KeywordActionKind::Explore,
            source_filter,
            vec![explore_effect(), explore_effect()],
            display,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_exile_to_countered_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(tokens) else {
        return Ok(None);
    };

    Ok(Some(
        StaticAbility::exile_to_countered_exile_instead_of_graveyard(
            spec.player,
            spec.counter_type,
        ),
    ))
}

pub(crate) fn parse_exile_to_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if !EXILE_TO_EXILE_INSTEAD_OF_GRAVEYARD_MARKER_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let Some(would_idx) = WOULD_WORD_PATTERN.find_word(&words) else {
        return Ok(None);
    };
    let Some(graveyard_idx) = GRAVEYARD_WORD_PATTERN.find_word(&words) else {
        return Ok(None);
    };
    let Some(graveyard_owner) = words
        .get(would_idx..graveyard_idx)
        .and_then(parse_would_be_put_into_graveyard_owner_words)
    else {
        return Ok(None);
    };
    let exclude_cycled = words
        .get(graveyard_idx + 1..)
        .is_some_and(|tail| WASNT_CYCLED_TAIL_PATTERN.matches_words(tail));

    let filter_tokens = trim_lexed_commas(&tokens[1..would_idx]);
    let filter_words = &words[1..would_idx];
    let filter = if is_source_reference_words(filter_words) {
        ObjectFilter::source()
    } else if CARD_OR_TOKEN_FILTER_PATTERN.matches_words(filter_words) {
        ObjectFilter::default()
    } else {
        parse_object_filter(filter_tokens, false).or_else(|_| {
            if CARD_FILTER_PATTERN.matches_words(filter_words) {
                Ok(ObjectFilter::default())
            } else if CREATURE_CARD_FILTER_PATTERN.matches_words(filter_words) {
                Ok(ObjectFilter::creature())
            } else if CYCLING_CARD_FILTER_PATTERN.matches_words(filter_words) {
                Ok(ObjectFilter::default().with_ability_marker("cycling"))
            } else {
                Err(CardTextError::ParseError(format!(
                    "unsupported exile-to-graveyard replacement subject (subject: '{}')",
                    filter_words.join(" ")
                )))
            }
        })?
    };
    let ability = if exclude_cycled {
        StaticAbility::exile_to_exile_instead_of_graveyard_unless_cycled(filter, graveyard_owner)
    } else {
        StaticAbility::exile_to_exile_instead_of_graveyard(filter, graveyard_owner)
    };
    Ok(Some(ability))
}

fn parse_would_be_put_into_graveyard_owner_words(words: &[&str]) -> Option<PlayerFilter> {
    const WOULD_BE_PUT_INTO_GRAVEYARD_OWNER_PHRASES: &[(&[&str], PlayerFilter)] = &[
        (&["would", "be", "put", "into", "a"], PlayerFilter::Any),
        (
            &["would", "be", "put", "into", "a", "players"],
            PlayerFilter::Any,
        ),
        (
            &["would", "be", "put", "into", "a", "player's"],
            PlayerFilter::Any,
        ),
        (&["would", "be", "put", "into", "your"], PlayerFilter::You),
        (
            &["would", "be", "put", "into", "an", "opponents"],
            PlayerFilter::Opponent,
        ),
        (
            &["would", "be", "put", "into", "an", "opponent's"],
            PlayerFilter::Opponent,
        ),
        (
            &["would", "be", "put", "into", "opponents"],
            PlayerFilter::Opponent,
        ),
        (
            &["would", "be", "put", "into", "opponent's"],
            PlayerFilter::Opponent,
        ),
    ];

    WOULD_BE_PUT_INTO_GRAVEYARD_OWNER_PHRASES
        .iter()
        .find_map(|(phrase, owner)| (*phrase == words).then(|| owner.clone()))
}

pub(crate) fn parse_exile_would_die_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let matched_nontoken_filter = if let Some(prefix_len) =
        NONTOKEN_OPPONENT_WOULD_DIE_PREFIX_PATTERN.matched_prefix_len(&words)
    {
        Some((
            ObjectFilter::creature()
                .nontoken()
                .controlled_by(PlayerFilter::Opponent),
            prefix_len,
        ))
    } else if let Some(prefix_len) =
        NONTOKEN_ANY_WOULD_DIE_PREFIX_PATTERN.matched_prefix_len(&words)
    {
        Some((ObjectFilter::creature().nontoken(), prefix_len))
    } else {
        None
    };
    if let Some((matched_filter, prefix_len)) = matched_nontoken_filter {
        let tail = &words[prefix_len..];
        let (exile_with_counters, follow_up) = match tail {
            [
                "exile",
                "that",
                "card",
                "instead",
                "and",
                "create",
                "a",
                "2/2",
                "black",
                "zombie",
                "creature",
                "token",
            ]
            | [
                "instead",
                "exile",
                "that",
                "card",
                "and",
                "create",
                "a",
                "2/2",
                "black",
                "zombie",
                "creature",
                "token",
            ] => (
                Vec::new(),
                vec![Effect::create_tokens(kalitas_zombie_token(), 1)],
            ),
            [
                "exile",
                "that",
                "card",
                "with",
                "an",
                "ice",
                "counter",
                "on",
                "it",
                "instead",
            ] => (vec![(CounterType::Ice, 1)], Vec::new()),
            ["exile", "that", "card", "instead"] | ["exile", "it", "instead"] => {
                (Vec::new(), Vec::new())
            }
            [
                "instead",
                "exile",
                "that",
                "card",
                "with",
                "a",
                counter_word,
                "counter",
                "on",
                "it",
            ]
            | [
                "instead",
                "exile",
                "it",
                "with",
                "a",
                counter_word,
                "counter",
                "on",
                "it",
            ]
            | [
                "exile",
                "that",
                "card",
                "with",
                "a",
                counter_word,
                "counter",
                "on",
                "it",
                "instead",
            ]
            | [
                "exile",
                "it",
                "with",
                "a",
                counter_word,
                "counter",
                "on",
                "it",
                "instead",
            ] => {
                let Some(counter_type) = parse_counter_type_word(counter_word) else {
                    return Ok(None);
                };
                (vec![(counter_type, 1)], Vec::new())
            }
            _ => return Ok(None),
        };
        return Ok(Some(
            StaticAbility::exile_would_die_instead_with_damage_source_counters_and_follow_up(
                matched_filter,
                None,
                exile_with_counters,
                follow_up,
            ),
        ));
    }

    if let Some(dealt_idx) = DEALT_WORD_PATTERN.find_word(&words)
        && DAMAGE_BY_PREFIX_PATTERN.matches_words(&words[dealt_idx + 1..])
        && WOULD_DIE_EXILE_INSTEAD_TAIL_PATTERN.matches_words(&words)
    {
        let victim_words = &words[1..dealt_idx];
        let victim = match victim_words {
            ["a", "creature"] | ["creature"] => ObjectFilter::creature(),
            ["a", "permanent"] | ["permanent"] => ObjectFilter::permanent(),
            _ => return Ok(None),
        };

        let damager_words = &words[dealt_idx + 3..words.len() - 7];
        let has_named_source_words = !damager_words.is_empty()
            && !matches!(
                damager_words.first().copied(),
                Some("a" | "an" | "the" | "target" | "that" | "this" | "equipped" | "enchanted")
            )
            && !damager_words.iter().any(|word| {
                matches!(
                    *word,
                    "creature" | "creatures" | "permanent" | "permanents" | "source" | "sources"
                )
            });
        let damaged_by = if THIS_DAMAGED_BY_SOURCE_PATTERN.matches_words(&damager_words)
            || has_named_source_words
        {
            Some(ironsmith_core::DamagedBySource::ThisCreature)
        } else if EQUIPPED_CREATURE_DAMAGED_BY_PATTERN.matches_words(&damager_words) {
            Some(ironsmith_core::DamagedBySource::EquippedCreature)
        } else if ENCHANTED_CREATURE_DAMAGED_BY_PATTERN.matches_words(&damager_words) {
            Some(ironsmith_core::DamagedBySource::EnchantedCreature)
        } else {
            None
        };

        if let Some(damaged_by) = damaged_by {
            return Ok(Some(
                StaticAbility::exile_would_die_instead_with_damage_source(victim, Some(damaged_by)),
            ));
        }
    }

    let Some(player) = simple_would_die_exile_player_filter(&words) else {
        return Ok(None);
    };

    Ok(Some(StaticAbility::exile_would_die_instead(
        ObjectFilter::creature().controlled_by(player),
    )))
}

fn kalitas_zombie_token() -> crate::cards::CardDefinition {
    crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zombie")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .color_indicator(ColorSet::BLACK)
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build()
}

pub(crate) fn parse_discard_or_redirect_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_discard_or_redirect_replacement_line_lexed(tokens) {
        return Ok(Some(StaticAbility::discard_or_redirect_replacement(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_pay_life_or_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized_storage = crate::runtime_backend::token_word_refs(tokens)
        .into_iter()
        .map(|word| match word.replace('’', "'").as_str() {
            "don't" => "dont".to_string(),
            _ => word.to_string(),
        })
        .collect::<Vec<_>>();
    let words = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if words.len() < 8 {
        return Ok(None);
    }

    if !AS_THIS_CONTAINS_PAY_LIFE_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let Some(pay_idx) = find_index(tokens, |token| PAY_WORD_PATTERN.matches_token(token)) else {
        return Err(CardTextError::ParseError(format!(
            "missing 'pay' keyword in pay-life ETB clause (clause: '{}')",
            words.join(" ")
        )));
    };
    if !words[..pay_idx]
        .iter()
        .any(|word| ENTER_OR_ENTERS_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    if !words[..pay_idx]
        .iter()
        .any(|word| MAY_WORD_PATTERN.matches_word(word))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported pay-life ETB prefix (clause: '{}')",
            words.join(" ")
        )));
    }

    let Some((value, _)) = parse_number(&tokens[pay_idx + 1..]) else {
        return Err(CardTextError::ParseError(format!(
            "missing life payment amount in pay-life ETB clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let if_dont_idx = keyword_find_prefix_shape_start(&words, &IF_YOU_DONT_PREFIX_PATTERN)
        .filter(|_| IF_YOU_DONT_PHRASE_PATTERN.matches_words(&words))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported pay-life ETB trailing clause (expected 'if you don't ...') (clause: '{}')",
                words.join(" ")
            ))
        })?;

    let trailing = &words[if_dont_idx + 3..];
    if !PAY_LIFE_ENTER_TAPPED_TAIL_PATTERN.matches_words(trailing) {
        return Err(CardTextError::ParseError(format!(
            "unsupported pay-life ETB trailing clause (clause: '{}')",
            words.join(" ")
        )));
    };

    parser_trace("parse_static:pay-life-etb:matched", tokens);
    Ok(Some(StaticAbility::pay_life_or_enter_tapped(value)))
}

pub(crate) fn parse_copy_activated_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 6 {
        return Ok(None);
    }

    let mut has_idx = None;
    for idx in 0..clause_words.len().saturating_sub(4) {
        if HAS_ALL_ACTIVATED_ABILITIES_OF_PATTERN.matches_words(&clause_words[idx..]) {
            has_idx = Some(idx);
            break;
        }
    }
    let Some(has_idx) = has_idx else {
        return Ok(None);
    };
    let Some(has_token_idx) = token_index_for_word_index(tokens, has_idx) else {
        return Ok(None);
    };

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, has_token_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..has_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let Some(filter_start_idx) = token_index_for_word_index(tokens, has_idx + 5) else {
        return Ok(None);
    };
    let filter_tokens = trim_edge_punctuation(&tokens[filter_start_idx..]);
    let mut filter_tokens =
        strip_leading_token_words_any(&filter_tokens, &["all", "each"]).to_vec();
    let after_of_words = crate::runtime_backend::lexer::parser_token_word_refs(&filter_tokens);
    let once_each_turn_start = after_of_words
        .windows(11)
        .position(|window| ACTIVATE_EACH_OF_THOSE_ONCE_TAIL_PATTERN.matches_words(window));
    let force_once_each_turn = once_each_turn_start.is_some();
    if let Some(start) = once_each_turn_start
        && let Some(token_idx) = token_index_for_word_index(&filter_tokens, start)
    {
        filter_tokens.truncate(token_idx);
        filter_tokens = trim_edge_punctuation(&filter_tokens);
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    let counter = parse_counter_type_from_tokens(&filter_tokens);

    let exclude_source_name = find_window_by(&clause_words, 5, |window| {
        SAME_NAME_AS_SOURCE_CREATURE_PATTERN.matches_words(window)
    })
    .is_some();
    let display = if force_once_each_turn {
        if let Some(start) = find_window_by(&clause_words, 11, |window| {
            ACTIVATE_EACH_OF_THOSE_ONCE_TAIL_PATTERN.matches_words(window)
        }) {
            format!(
                "{}. You may activate each of those abilities only once each turn",
                clause_words[..start].join(" ").trim()
            )
        } else {
            clause_words.join(" ")
        }
    } else {
        clause_words.join(" ")
    };

    let mut ability = crate::static_abilities::CopyActivatedAbilities::new(filter)
        .with_exclude_source_name(exclude_source_name)
        .with_exclude_source_id(true)
        .with_display(display);
    if let Some(counter) = counter {
        ability = ability.with_counter(counter);
    }
    if force_once_each_turn {
        ability = ability.with_once_each_turn();
    }

    let ability = StaticAbility::copy_activated_abilities(ability);
    let ast = match subject {
        AnthemSubjectAst::Source => match condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition,
            },
            None => StaticAbilityAst::Static(ability),
        },
        AnthemSubjectAst::Filter(subject_filter) => StaticAbilityAst::GrantStaticAbility {
            filter: subject_filter,
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition,
        },
    };

    Ok(Some(ast))
}

pub(crate) fn parse_spend_mana_as_any_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() == 25
        && word_slice_eq(&clause_words[0..3], &["you", "may", "spend"])
        && word_slice_eq(&clause_words[4..25], &[
            "mana", "as", "though", "it", "were", "mana", "of", "any", "color", "you", "may",
            "spend", "other", "mana", "only", "as", "though", "it", "were", "colorless",
            "mana",
        ])
        && let Some(symbol) = parse_mana_symbol_word_flexible(clause_words[3])
        && !matches!(symbol, ManaSymbol::Colorless)
    {
        let display = format!(
            "You may spend {} mana as though it were mana of any color. You may spend other mana only as though it were colorless mana",
            clause_words[3]
        );
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::mana_spend_permission(
                crate::effect::ManaSpendPermission::mana_symbol_as_any_color_other_as_colorless(
                    PlayerFilter::You,
                    symbol,
                ),
                display,
            ),
        )));
    }

    if SPEND_MANA_ANY_TYPE_CAST_PREFIX_PATTERN.matches_words(&clause_words) {
        let filter_start = 9usize;
        let filter_tokens = trim_edge_punctuation(&tokens[filter_start..]);
        if filter_tokens.is_empty() {
            return Ok(None);
        }
        let filter = parse_object_filter(&filter_tokens, false)
            .map(|mut filter| {
                filter.zone = None;
                filter.stack_kind = None;
                filter.has_mana_cost = false;
                filter
            })
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported mana spend cast filter (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::mana_spend_permission(
                crate::effect::ManaSpendPermission::any_color_for_casting_matching(
                    PlayerFilter::You,
                    filter,
                ),
                clause_words.join(" "),
            ),
        )));
    }

    let (player, tail_start, display) =
        if PLAYERS_MAY_SPEND_MANA_ANY_COLOR_PREFIX_PATTERN.matches_words(&clause_words) {
            (
                PlayerFilter::Any,
                12usize,
                "Players may spend mana as though it were mana of any color".to_string(),
            )
        } else if YOU_MAY_SPEND_MANA_ANY_COLOR_PREFIX_PATTERN.matches_words(&clause_words) {
            (PlayerFilter::You, 12usize, clause_words.join(" "))
        } else {
            return Ok(None);
        };

    let tail_tokens = trim_edge_punctuation(&tokens[tail_start..]);
    let permission = if tail_tokens.is_empty() {
        crate::effect::ManaSpendPermission::any_color(player)
    } else {
        let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
        if PAY_ACTIVATION_COSTS_OF_PREFIX_PATTERN.matches_words(&tail_words) {
            let ability_words = crate::runtime_backend::token_word_refs(&tail_tokens[6..]);
            if !ABILITY_OR_ABILITIES_MARKER_PATTERN.matches_words(&ability_words) {
                return Ok(None);
            }
            crate::effect::ManaSpendPermission::any_color_for_activation(
                player,
                ObjectFilter::source(),
            )
        } else if ACTIVATE_ABILITIES_OF_PREFIX_PATTERN.matches_words(&tail_words) {
            let filter_tokens = trim_edge_punctuation(&tail_tokens[4..]);
            if filter_tokens.is_empty() {
                return Ok(None);
            }
            let filter = match parse_object_filter(&filter_tokens, false) {
                Ok(filter) => filter,
                Err(_) => return Ok(None),
            };
            crate::effect::ManaSpendPermission::any_color_for_activation(player, filter)
        } else {
            return Ok(None);
        }
    };

    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::mana_spend_permission(permission, display),
    )))
}
include!("keyword_lines.rs");
include!("anthem_grant_lines.rs");
include!("etb_static_lines.rs");
include!("attached_object_static_lines.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn supported_keyword_marker_uses_token_shapes_for_crew_markers() {
        for line in [
            "This creature crews Vehicles using its toughness rather than its power.",
            "This token saddles Mounts and crews Vehicles as though its power were 2 greater.",
            "You may remove a loyalty counter from a planeswalker you control rather than pay this creature's crew cost.",
        ] {
            let tokens = lex_line(line, 0).expect("marker line should lex");
            let text = render_token_slice(&tokens);
            assert!(
                supported_keyword_marker_tokens(&tokens, &text),
                "{line} should be recognized through token shapes"
            );
        }
    }

    #[test]
    fn pt_modifier_parsers_use_char_signs() {
        assert_eq!(parse_pt_modifier("+2/-1").unwrap(), (2, -1));
        assert_eq!(parse_pt_modifier("2/+3").unwrap(), (2, 3));
        assert_eq!(
            parse_pt_modifier_values("−X/+2").unwrap(),
            (Value::XTimes(-1), Value::Fixed(2))
        );
    }

    #[test]
    fn early_static_ability_parser_uses_parser_token_words() {
        for line in [
            "X can't be greater than the number of players in the game.",
            "This creature can't attack unless you've cast a creature spell this turn.",
            "During your turn, as long as you haven't activated an exhaust ability this turn, you may activate exhaust abilities as though they haven't been activated.",
        ] {
            let tokens = lex_line(line, 0).expect("static line should lex");
            assert!(
                parse_static_ability_ast_line_early_lexed(&tokens)
                    .expect("early static line should parse")
                    .is_some(),
                "{line} should match through parser token words"
            );
        }
    }
}
