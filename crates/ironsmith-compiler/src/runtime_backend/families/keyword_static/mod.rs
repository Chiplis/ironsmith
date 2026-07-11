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
    is_prevent_all_noncombat_damage_to_matching_permanents_line_lexed,
    is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed,
    is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed,
    is_protection_mana_value_marker_line_lexed, is_remove_snow_line_lexed,
    is_sab_sunen_cant_attack_or_block_unless_line_lexed,
    is_shuffle_into_library_from_graveyard_line_lexed, is_skulk_rules_text_line_lexed,
    is_this_creature_cant_attack_alone_line_lexed,
    is_this_creature_cant_attack_its_owner_line_lexed, is_this_subject_reference_lexed,
    is_you_assign_combat_damage_of_creatures_attacking_you_line_lexed,
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
use super::grammar::anthem_grants as anthem_grant_grammar;
use super::grammar::conditions::{
    PlayerLifeChangeDirectionAst, PlayerLifeChangeThisTurnConditionAst,
    parse_player_life_change_this_turn_condition,
};
use super::grammar::filters::{
    parse_object_filter_with_grammar_entrypoint, parse_spell_filter_with_grammar_entrypoint,
    parse_spell_filter_with_grammar_entrypoint_lexed,
};
use super::grammar::leaf::parse_leaf_fixed_mana_cost_prefix_tokens;
use super::grammar::primitives::{
    split_lexed_slices_on_and, split_lexed_slices_on_comma,
    split_lexed_slices_on_commas_or_semicolons, split_lexed_slices_on_period,
};
use super::grammar::static_keyword_shapes;
pub(crate) use super::grammar::values::parse_add_mana_equal_amount_value_lexed as parse_add_mana_equal_amount_value;
use super::grammar::values::parse_max_cards_in_hand_value_lexed;
use super::grammar::{
    attached_object_static_lines as attached_grammar, document_shapes as document_grammar,
    keyword_static_lines, static_keyword_cost_shapes, static_keyword_line_shapes,
    static_keyword_replacement_shapes, trigger_surface,
};
use super::keyword_static_helpers::*;
use super::lexer::{
    LexedClause, OwnedLexToken, TokenKind, TokenWordView, contains_token_kind, find_token_kind,
    parser_token_word_refs, render_token_slice, split_lexed_sentences, token_slice_at_is,
    token_slice_at_is_any, token_slice_first_is, token_slice_first_is_any, tokens_start_with,
    trim_lexed_commas, word_slice_at_is, word_slice_at_is_any, word_slice_eq, word_slice_eq_any,
    word_slice_eq_any_at, word_slice_eq_at, word_slice_find_any_phrase_span,
    word_slice_find_any_phrase_start, word_slice_find_phrase_start_or_zero,
    word_slice_find_word_where, word_slice_first_is, word_slice_first_is_any, word_slice_last_is,
    word_slice_last_is_any, word_slice_strip_any_prefix, word_slice_strip_first_word,
    words_end_with, words_end_with_any, words_have_all, words_have_any, words_have_any_phrase,
    words_have_none, words_start_with_any, words_start_with_at,
};
use super::lowering_support::rewrite_parsed_triggered_ability as parsed_triggered_ability;
use super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::rule_engine::{LexRuleHeadHint, LexRuleHintIndex, build_lex_rule_hint_index};
use super::static_ability_helpers::{
    afflict_triggered_ability, lower_granted_abilities_ast_to_object_abilities,
    static_ability_for_keyword_action,
};
use super::token_primitives::{
    is_core_keyword_marker_text, is_ticket_sticker_marker_text, items_have, lexed_head_words,
    slice_strip_prefix, slice_strip_suffix, split_em_dash_label_prefix,
    split_em_dash_label_prefix_tokens,
};
use super::util::{
    comparison_to_at_least_threshold, comparison_to_strict_at_least_threshold,
    is_source_reference_words, leading_mana_cost_from_tokens, mana_pips_from_token,
    parse_alternative_cast_words, parse_card_type, parse_choice_count_token_prefix_consumed,
    parse_color, parse_counter_type_word, parse_counter_type_words,
    parse_filter_counter_constraint_words, parse_flashback_keyword_line,
    parse_for_each_count_value_words, parse_greater_than_or_equal_quantity_prefix,
    parse_greater_than_or_equal_quantity_prefix_words, parse_less_than_or_equal_quantity_prefix,
    parse_mana_symbol_word_flexible, parse_number_word_i32, parse_quantity_comparison_prefix,
    parse_subtype_flexible, parse_value, parse_value_expr_words, parse_zone_word,
    source_reference_surface_for_possessive_words, strip_leading_article_word_refs,
    strip_leading_token_words_any, strip_leading_word_refs_any, trim_commas,
    trim_edge_punctuation_tokens, word_refs_at_is_article, words,
};
use super::util::{source_choose_spec_for_surface, source_reference_surface_for_words};
use super::value_helpers::{
    parse_aggregate_scope_value_lexed, parse_commander_cast_count_player,
    parse_filter_comparison_tokens,
};
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

const LOSES_ALL_OTHER_CREATURE_TYPES_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "loses", "all", "other", "creature", "types"],
            &["this", "loses", "all", "other", "creature", "types"],
        ]
);
const SKIP_YOUR_UPKEEP_STEP_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["skip", "your", "upkeep", "step"]);
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
const WOULD_DEAL_DAMAGE_TO_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["would", "deal", "damage", "to"]);
const WOULD_DEAL_NONCOMBAT_DAMAGE_TO_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["would", "deal", "noncombat", "damage", "to"]);
const IT_DEALS_THAT_MUCH_DAMAGE_PLUS_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["it", "deals", "that", "much", "damage", "plus"]);
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
const LAND_OR_LANDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["land"], &["lands"]]);
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
const CONDITIONAL_DRAW_LIFE_LOSS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["and", "you", "lose"];
    suffix & ["life"]
);
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
const CAST_CREATURE_THIS_WAY_HASTE_SENTENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "if", "you", "cast", "a", "creature", "spell", "this", "way", "it", "gains", "haste",
            "until", "end", "of", "turn",
        ]
);
const CAST_THIS_WAY_ENTERS_TAPPED_SENTENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "if", "you", "cast", "a", "spell", "this", "way", "that", "artifact", "enters",
                "tapped",
            ],
            &[
                "if",
                "you",
                "cast",
                "a",
                "spell",
                "this",
                "way",
                "that",
                "permanent",
                "enters",
                "tapped",
            ],
            &[
                "if", "you", "cast", "a", "spell", "this", "way", "that", "creature", "enters",
                "tapped",
            ],
            &[
                "if", "you", "cast", "a", "spell", "this", "way", "it", "enters", "tapped",
            ],
            &["if", "you", "do", "it", "enters", "tapped"],
        ]
);
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
            &["have", "all", "activated", "abilities", "of"],
            &["has", "all", "loyalty", "abilities", "of"],
            &["have", "all", "loyalty", "abilities", "of"]
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
const AS_LONG_AS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["as", "long", "as"]);
const ENTERS_TAPPED_LINE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this"]; contains_words & ["enters", "tapped"]);
const CHOOSE_CARD_NAME_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["choose", "a", "card", "name"]);
const NOTE_YOUR_LIFE_TOTAL_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["note", "your", "life", "total"]);
const SOURCE_IT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it"]);
const CHOICE_OR_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["or"]);
const AS_THIS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["as", "this"]);
const BECOMES_ATTACHED_TO_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["becomes", "attached", "to"]);
const IS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["is"]);
const BASIC_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["basic"]);
const IS_OR_ARE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["is"], &["are"]]);
const CHOOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["choose"]);
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
const CHOSEN_COLOR_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["the", "chosen", "color"], &["chosen", "color"]]);
const THE_CHOSEN_COLOR_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["the", "chosen", "color"]);
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
const PLAYER_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "player"], &["player"]]);
const YOU_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const OPPONENT_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["an", "opponent"], &["opponent"]]);
const IT_TARGETS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["it", "targets"]);
const THIS_SPELL_TARGETS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "spell", "targets"]);
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
const COUNT_AS_CARD_NAMED_GRAVEYARD_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["if", "this", "card", "is", "in", "a", "graveyard"],
            &["if", "this", "card", "is", "in", "your", "graveyard"],
        ]
);
const EFFECTS_FROM_SPELLS_NAMED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["effects", "from", "spells", "named"]);
const COUNT_IT_AS_A_CARD_NAMED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["count", "it", "as", "a", "card", "named"]);
const ONLY_OTHER_CREATURE_CARDS_NAMED_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "the", "only", "other", "creature", "cards", "in", "your", "hand", "are", "named",
        ]
);
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
// Compared against article-stripped words; do not include "a"/"an"/"the".
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
const FIRST_SPELL_EACH_TURN_COST_MODIFIER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["first", "each", "turn"]; contains_any_words & [&["cost", "costs"]]);
const YOU_CAST_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["you", "cast"]]);
const FROM_YOUR_GRAVEYARD_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["from", "your", "graveyard"]]);
const OPPONENT_WORD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["opponent", "opponents"]]);
const CAST_OR_CASTS_WORD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["cast", "casts"]]);
// Compared against article-stripped words; "the" is removed before matching.
const CAST_A_OR_ONE_SPELL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["cast", "a", "spell"], &["cast", "one", "spell"]]);
const AND_YOU_MAY_SPEND_MANA_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["and", "you", "may", "spend", "mana"]);
const THAT_HAVE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["that", "have"]);
const WOULD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["would"]);
const GRAVEYARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["graveyard"]);
const DEALT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["dealt"]);
const RATHER_THAN_PAY_CYCLING_COSTS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["rather", "than", "pay", "cycling", "costs"]);
const CAST_WORD_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["cast"]);
const YOU_MAY_PAY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "may", "pay"]);
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
const SURVEILLED_GRAVEYARD_PLAY_LIFE_COST_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
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
                "cost",
            ],
            &[
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
                "you've",
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
                "cost",
            ],
            &[
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
                "you’ve",
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
                "cost",
            ],
        ]
);
// Compared against article-stripped words; "the" is removed before matching.
const IF_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["if"]);
const WHILE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["while"]);
const TO_TRIGGER_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["to", "trigger"]);
const TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const WITH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["with"]);
const ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["a"], &["an"]]);
const ENCHANTED_OR_EQUIPPED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enchanted"], &["equipped"]]);
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
const ALL_CARDS_SPELLS_PERMANENTS_CHOSEN_COLOR_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
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
            "colors",
        ]
);
const ARE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["are"]);
const BE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["is"], &["are"]]);
const AND_ARE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["and", "are"]);
const CREATURE_TYPE_SCOPE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["creature", "type"], &["creature", "types"]]);
const TAPPED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["tapped"]);

fn keyword_find_prefix_shape_start(
    clause: LexedClause<'_>,
    shape: &ClauseShape<'static>,
) -> Option<usize> {
    (0..clause.word_len()).find(|&idx| {
        clause
            .after_words(idx)
            .is_some_and(|tail| shape.matches(tail))
    })
}

fn keyword_find_exact_clause_window(
    clause: LexedClause<'_>,
    width: usize,
    shape: ClauseShape<'static>,
) -> Option<usize> {
    let word_len = clause.word_len();
    if width == 0 || word_len < width {
        return None;
    }
    (0..=word_len - width).find(|&idx| {
        clause
            .between_word_range(idx, idx + width)
            .is_some_and(|window| shape.matches(window))
    })
}

fn chosen_name_source_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter.name = Some("{chosen name}".to_string());
    filter
}

fn two_card_type_union_filter(left_type: CardType, right_type: CardType) -> ObjectFilter {
    let mut left_filter = ObjectFilter::default();
    left_filter.zone = Some(Zone::Battlefield);
    left_filter.card_types = vec![left_type];

    let mut right_filter = ObjectFilter::default();
    right_filter.zone = Some(Zone::Battlefield);
    right_filter.card_types = vec![right_type];

    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = vec![left_filter, right_filter];
    disjunction
}

fn activated_ability_subject_special_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    match keyword_static_lines::parse_activated_ability_special_subject_tokens(tokens)? {
        keyword_static_lines::ActivatedAbilitySpecialSubject::ChosenName => {
            Some(chosen_name_source_filter())
        }
        keyword_static_lines::ActivatedAbilitySpecialSubject::TwoCardTypes(left, right) => {
            Some(two_card_type_union_filter(left, right))
        }
    }
}

fn parse_life_total_or_less_spell_cost_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;

    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    if words.len() >= 4 && YOU_HAVE_PREFIX_PATTERN.matches(clause) {
        let tail_words = words.get(2..)?;
        if tail_words.last().copied() == Some("life") {
            let quantity_clause = clause.between_word_range(2, words.len().saturating_sub(1))?;
            let quantity_tokens = quantity_clause.tokens();
            if let Some((amount, used)) = parse_less_than_or_equal_quantity_prefix(
                quantity_tokens,
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

    if words.len() >= 6 && YOUR_LIFE_TOTAL_IS_PREFIX_PATTERN.matches(clause) {
        let quantity_clause = clause.after_words(4)?;
        let quantity_tokens = quantity_clause.tokens();
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
        .any(|word| parse_card_type(word) == Some(CardType::Instant))
    {
        types.push(CardType::Instant);
    }
    if words
        .iter()
        .any(|word| parse_card_type(word) == Some(CardType::Sorcery))
    {
        types.push(CardType::Sorcery);
    }
    types
}

fn count_start_for_optional_an_opponent_prefix(words: &[&str], long_len: usize) -> Option<usize> {
    if words.first().is_some_and(|word| is_article(word)) {
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

fn only_creature_cards_in_hand_named(clause: LexedClause<'_>) -> Option<String> {
    let words = clause.word_refs();
    let matches_named_exception = (YOU_HAVE_NO_OTHER_CREATURE_CARDS_PREFIX_PATTERN.matches(clause)
        && OR_IF_PHRASE_PATTERN.matches(clause))
        || ONLY_OTHER_CREATURE_CARDS_NAMED_PREFIX_PATTERN.matches(clause);
    if !matches_named_exception {
        return None;
    }

    let named_idx = static_keyword_line_shapes::parse_named_word(&words)?.word;
    let name_words = words.get(named_idx + 1..)?;
    let name = name_words.join(" ");
    (!name.is_empty()).then_some(name)
}

pub(crate) fn parse_can_be_attached_only_to_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(attachment) = static_keyword_line_shapes::parse_attachment_restriction_span(tokens)
    else {
        return Ok(None);
    };
    if attachment.start == 0 {
        return Ok(None);
    }
    let target_tokens = trim_commas(&tokens[attachment.end..]);
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

const AS_ENTERS_AURA_SUBJECTS: &[(&str, &str)] = &[("aura", "this aura")];

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
        let clause = LexedClause::new(tokens);
        if POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches(clause)
            && !crate::string_primitives::ends_with_char(&text, '.')
        {
            text.push('.');
        }
        return StaticAbility::keyword_marker(text);
    }
    StaticAbility::keyword_fallback_text(text)
}

fn supported_keyword_marker_tokens(tokens: &[OwnedLexToken], text: &str) -> bool {
    let text = text.trim_start().to_ascii_lowercase();
    let clause = LexedClause::new(tokens);
    is_core_keyword_marker_text(&text)
        || TOUGHNESS_CREWS_VEHICLES_MARKER_PATTERN.matches(clause)
        || POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches(clause)
        || LOYALTY_COUNTER_INSTEAD_OF_CREW_COST_MARKER_PATTERN.matches(clause)
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
        "parse_attached_can_attack_as_though_no_defender_line" => vec![
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Pair("enchanted", "creature"),
            StaticAbilityLineHeadHint::Pair("enchanted", "wall"),
            StaticAbilityLineHeadHint::Single("equipped"),
            StaticAbilityLineHeadHint::Pair("equipped", "creature"),
            StaticAbilityLineHeadHint::Single("attached"),
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
        "parse_as_you_cascade_land_drop_line" => vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "you"),
        ],
        "parse_play_from_permission_with_haste_this_way_line"
        | "parse_play_from_permission_with_enter_counter_this_way_line"
        | "parse_play_from_permission_with_enter_tapped_this_way_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
            StaticAbilityLineHeadHint::Single("once"),
            StaticAbilityLineHeadHint::Pair("once", "during"),
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
        _ => match static_keyword_shapes::parse_rule_id_head(rule_id) {
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
        single_static_ability_ast_rule!(parse_affinity_cost_reduction_line),
        single_static_ability_ast_rule!(parse_pregame_begin_on_battlefield_line),
        single_static_ability_ast_rule!(parse_pregame_mulligan_redraw_line),
        multi_static_ability_ast_rule!(parse_combined_pregame_choose_color_line),
        single_static_ability_ast_rule!(parse_pregame_choose_color_line),
        single_static_ability_ast_rule!(parse_activated_abilities_cost_increase_line),
        single_static_ability_ast_rule!(parse_choose_basic_land_type_as_enters_line),
        single_static_ability_ast_rule!(
            parse_revealed_hand_choose_nonland_card_name_as_enters_line
        ),
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
        single_static_ability_ast_rule!(parse_keyword_action_replacement_line),
        single_static_ability_ast_infallible_rule!(parse_static_text_marker_line),
        multi_static_ability_ast_rule!(parse_enters_tapped_with_choose_color_line),
        single_static_ability_ast_rule!(parse_damage_not_removed_cleanup_line),
        single_static_ability_ast_passthrough_rule!(
            parse_prevent_damage_to_source_remove_counter_line
        ),
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
        single_static_ability_ast_rule!(
            parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line
        ),
        single_static_ability_ast_rule!(parse_conditional_draw_replacement_line),
        single_static_ability_ast_rule!(parse_draw_replacement_double_line),
        single_static_ability_ast_rule!(parse_draw_replacement_skip_empty_library_line),
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
            id: stringify!(parse_anthem_and_keyword_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_anthem_and_keyword_line),
        },
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
            parse_you_assign_combat_damage_of_creatures_attacking_you_line
        ),
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
        multi_static_ability_ast_rule!(parse_source_counter_threshold_keyword_and_subtype_line),
        multi_static_ability_ast_rule!(
            parse_subject_are_card_types_in_addition_to_their_other_types_line
        ),
        single_static_ability_ast_rule!(parse_all_permanents_colorless_line),
        single_static_ability_ast_rule!(parse_all_cards_spells_permanents_colorless_line),
        multi_static_ability_ast_rule!(parse_all_are_pt_color_type_addition_line),
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
        multi_static_ability_ast_passthrough_rule!(parse_attached_land_ability_reset_line),
        multi_static_ability_ast_rule!(parse_attached_type_transform_line),
        multi_static_ability_ast_rule!(parse_attached_has_and_loses_keywords_line),
        single_static_ability_ast_rule!(parse_you_control_attached_creature_line),
        single_static_ability_ast_passthrough_rule!(parse_attached_cant_attack_or_block_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_can_attack_as_though_no_defender_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_damage_dealt_to_and_by_attached_line
        ),
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
        single_static_ability_ast_rule!(parse_enter_as_copy_as_enters_line),
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
        multi_static_ability_ast_rule!(
            parse_you_may_cast_exile_counter_cards_with_mana_permission_line
        ),
        multi_static_ability_ast_rule!(parse_surveilled_graveyard_play_life_cost_line),
        single_static_ability_ast_rule!(parse_as_you_cascade_land_drop_line),
        single_static_ability_ast_rule!(parse_play_from_permission_with_haste_this_way_line),
        single_static_ability_ast_rule!(
            parse_play_from_permission_with_enter_counter_this_way_line
        ),
        single_static_ability_ast_rule!(parse_play_from_permission_with_enter_tapped_this_way_line),
        multi_static_ability_ast_rule!(parse_you_may_static_grant_line),
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
        single_static_ability_ast_rule!(
            parse_prevent_all_noncombat_damage_to_matching_permanents_line
        ),
        single_static_ability_ast_rule!(parse_prevent_all_damage_to_source_by_creatures_line),
        single_static_ability_ast_rule!(
            parse_prevent_damage_to_other_creature_you_control_put_counters_line
        ),
        single_static_ability_ast_rule!(parse_prevent_all_damage_dealt_to_creatures_line),
        single_static_ability_ast_passthrough_rule!(parse_creatures_cant_block_line),
        multi_static_ability_ast_rule!(parse_enters_tapped_with_counters_line),
        multi_static_ability_ast_rule!(parse_enters_with_counters_line),
        single_static_ability_ast_rule!(parse_enters_with_additional_counter_for_filter_line),
        single_static_ability_ast_rule!(parse_as_enters_reveal_from_hand_line),
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

#[rustfmt::skip]
fn parse_count_as_card_named_for_spell_effect_line(tokens: &[OwnedLexToken])
    -> Option<StaticAbility>
{
    let clause = LexedClause::new(tokens);
    if !COUNT_AS_CARD_NAMED_GRAVEYARD_PREFIX_PATTERN.matches(clause) {
        return None;
    }

    let words = clause.word_refs();
    let effects_idx = keyword_find_exact_clause_window(
        clause,
        4,
        EFFECTS_FROM_SPELLS_NAMED_PATTERN,
    )?;
    let spell_name_start = effects_idx + 4;
    let count_idx = static_keyword_line_shapes::parse_count_as_card_count_word(
        words.get(spell_name_start..).unwrap_or_default(),
    )?
    .word
        + spell_name_start;
    let spell_name_words = words.get(spell_name_start..count_idx)?;
    if spell_name_words.is_empty()
        || !clause
            .between_word_range(count_idx, count_idx + 6)
            .is_some_and(|tail| COUNT_IT_AS_A_CARD_NAMED_PATTERN.matches(tail))
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

fn parse_source_characteristics_of_last_exiled_creature_card_line(
    tokens: &[OwnedLexToken],
) -> Option<StaticAbility> {
    let words = parser_token_word_refs(tokens);
    let base = [
        "as",
        "long",
        "as",
        "a",
        "card",
        "exiled",
        "with",
        "this",
        "creature",
        "is",
        "a",
        "creature",
        "card",
        "this",
        "creature",
        "has",
        "the",
        "power",
        "toughness",
        "and",
        "creature",
        "types",
        "of",
        "the",
        "last",
        "creature",
        "card",
        "exiled",
        "with",
        "it",
    ];
    if words.len() < base.len() || words[..base.len()] != base {
        return None;
    }

    let retained_subtypes = match words.get(base.len()..) {
        Some([]) => Vec::new(),
        Some(["it's", "still", "a", subtype]) | Some(["its", "still", "a", subtype]) => {
            vec![parse_subtype_flexible(subtype)?]
        }
        _ => return None,
    };

    let mut filter = ObjectFilter::default();
    filter.card_types.push(CardType::Creature);
    filter.nontoken = true;
    filter.zone = Some(Zone::Exile);
    Some(
        StaticAbility::source_characteristics_of_last_exiled_creature_card(
            filter,
            retained_subtypes,
        ),
    )
}

fn parse_static_ability_ast_line_early_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let marker_text = render_token_slice(tokens);
    if is_ticket_sticker_marker_text(&marker_text) {
        return Ok(Some(vec![keyword_static_marker(tokens).into()]));
    }

    if let Some(ability) = parse_source_characteristics_of_last_exiled_creature_card_line(tokens) {
        return Ok(Some(vec![ability.into()]));
    }

    if let Some(ability) = parse_enter_as_copy_as_enters_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }

    if let Some(marker) = keyword_static_lines::parse_early_static_marker_tokens(tokens) {
        let ability = match marker {
            keyword_static_lines::EarlyStaticMarkerKind::XMaximumPlayerCount => {
                StaticAbility::this_spell_x_maximum(
                    Value::CountPlayers(PlayerFilter::Any),
                    "X can't be greater than the number of players in the game.",
                )
            }
            keyword_static_lines::EarlyStaticMarkerKind::XMinimumOne => {
                StaticAbility::this_spell_x_minimum(Value::Fixed(1), "X can't be 0.")
            }
            keyword_static_lines::EarlyStaticMarkerKind::ExhaustAsUnactivated => {
                StaticAbility::exhaust_abilities_as_though_unactivated_this_turn()
            }
            keyword_static_lines::EarlyStaticMarkerKind::CantAttackWithoutCreatureSpell => {
                StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn()
            }
            keyword_static_lines::EarlyStaticMarkerKind::CantAttackWithoutNoncreatureSpell => {
                StaticAbility::cant_attack_unless_controller_cast_noncreature_spell_this_turn()
            }
            keyword_static_lines::EarlyStaticMarkerKind::DayNightStartsDay => {
                StaticAbility::day_night_starts_day_as_enters()
            }
            keyword_static_lines::EarlyStaticMarkerKind::VehicleRulesMarker => {
                keyword_static_marker(tokens)
            }
        };
        return Ok(Some(vec![ability.into()]));
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

    if let Some(ability) = parse_can_block_additional_creature_each_combat_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }

    if let Some(ability) = parse_count_as_card_named_for_spell_effect_line(tokens) {
        return Ok(Some(vec![ability.into()]));
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
    let clause = LexedClause::new(tokens);
    if !DAMAGE_DOUBLING_MANA_VALUE_MARKER_PATTERN.matches(clause)
        || !DAMAGE_DOUBLING_TO_TARGET_PATTERN.matches(clause)
    {
        return Ok(None);
    }

    Ok(Some(keyword_static_marker(tokens)))
}

pub(crate) fn parse_static_ability_ast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    // Compound "as this enters ... if you do ..." replacement text is one
    // semantic unit even though it contains a sentence boundary.
    if let Some(ability) = parse_enter_as_copy_as_enters_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    if let Some(ability) =
        parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line(tokens)?
    {
        return Ok(Some(vec![StaticAbilityAst::from(ability)]));
    }
    if let Some(abilities) = parse_attached_conditional_keyword_otherwise_line(tokens)? {
        return Ok(Some(abilities));
    }

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
    if let Some(ability) = parse_double_counters_replacement_line(tokens)? {
        return Ok(Some(vec![StaticAbilityAst::Static(ability)]));
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
    let Some(head) = static_keyword_line_shapes::parse_player_counter_gain_head(tokens) else {
        return false;
    };
    if !head.has_counter_resource {
        return false;
    }

    let tail = tokens.get(head.get.token + 1..).unwrap_or_default();
    if parse_value(tail).is_some() {
        return true;
    }

    tail.iter()
        .find_map(OwnedLexToken::as_word)
        .is_some_and(|word| {
            word == "another"
                || crate::runtime_backend::grammar::leaf::parse_number_complete(word).is_ok()
        })
}

pub(crate) fn parse_activated_abilities_cant_be_activated_line(
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

    // "Activated abilities of artifacts and creatures ..." should be a union of types.
    // Our general object filter parser treats type lists joined by "and" as intersection,
    // which is correct for many adjective chains, but incorrect for this rules pattern.
    let subject_words = crate::runtime_backend::util::non_article_token_word_refs(&subject_tokens);

    let filter = if let Some(filter) = activated_ability_subject_special_filter(subject_tokens) {
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

pub(crate) fn parse_activated_abilities_cost_increase_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = static_keyword_cost_shapes::parse_activated_ability_cost_increase(tokens)
    else {
        return Ok(None);
    };
    let clause_display = render_token_slice(tokens);

    let subject_tokens = trim_lexed_commas(spec.subject_tokens);
    let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported activated-ability cost increase subject (clause: '{}')",
            clause_display
        ))
    })?;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }

    let additional_cost_tokens = trim_outer_quotes(trim_lexed_commas(spec.additional_cost_tokens));
    if additional_cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing activated-ability additional cost (clause: '{}')",
            clause_display
        )));
    }
    let total_cost = parse_activation_cost(additional_cost_tokens)?;
    if total_cost.is_free() {
        return Err(CardTextError::ParseError(format!(
            "unsupported activated-ability additional cost (clause: '{}')",
            clause_display
        )));
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

    let filter = if let Some(filter) = activated_ability_subject_special_filter(subject_tokens) {
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
    let clause_display = render_token_slice(tokens);
    let Some(spec) = keyword_static_lines::parse_pregame_begin_on_battlefield_tokens(tokens)?
    else {
        return Ok(None);
    };

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::BeginOnBattlefield(
            crate::static_abilities::PregameBeginOnBattlefieldSpec {
                require_not_starting_player: spec.require_not_starting_player,
                counters: spec.counters,
                exile_cards_from_hand: spec.exile_cards_from_hand,
            },
        ),
        clause_display,
    )))
}

pub(crate) fn parse_pregame_mulligan_redraw_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !PREGAME_MULLIGAN_REDRAW_PATTERN.matches(LexedClause::new(tokens)) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::MulliganExileHandDrawSameCount,
        render_token_slice(tokens),
    )))
}

pub(crate) fn parse_pregame_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !keyword_static_lines::parse_pregame_choose_color_tokens(tokens) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::ChooseColor,
        render_token_slice(tokens),
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
    let Some(additional) = keyword_static_lines::parse_can_block_additional_creature_tokens(tokens)
    else {
        return Ok(None);
    };

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
    if !tokens.first().is_some_and(|token| token.is_word("ward")) {
        return Ok(None);
    }

    if keyword_static_lines::parse_ward_abilities_dont_trigger_marker_tokens(tokens) {
        return Ok(Some(StaticAbility::suppress_matching_triggered_abilities(
            Some(ObjectFilter::creature().opponent_controls()),
            None,
            render_token_slice(tokens),
        )));
    }

    let Some(ward) = keyword_static_lines::parse_ward_cost_tokens(tokens) else {
        return Err(CardTextError::ParseError(
            "ward keyword missing cost".to_string(),
        ));
    };

    let cost_tokens = trim_commas(ward.cost_tokens);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "ward keyword missing cost".to_string(),
        ));
    }

    if let Some(mana) = parse_leaf_fixed_mana_cost_prefix_tokens(&cost_tokens)
        && trim_edge_punctuation_tokens(&cost_tokens[mana.consumed..]).is_empty()
    {
        return Ok(Some(StaticAbility::ward(TotalCost::mana(mana.cost))));
    }

    if let Some(cost) = parse_compact_sacrifice_ward_cost(&cost_tokens)? {
        return Ok(Some(StaticAbility::ward(cost)));
    }

    if let Some(cost) = parse_ward_discard_card_type_cost(&cost_tokens) {
        return Ok(Some(StaticAbility::ward(cost)));
    }

    if let Some(cost) = parse_payment_clause_as_total_cost(&cost_tokens)? {
        return Ok(Some(StaticAbility::ward(cost)));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported ward cost clause (clause: '{}')",
        render_token_slice(tokens)
    )))
}

fn parse_compact_sacrifice_ward_cost(
    tokens: &[OwnedLexToken],
) -> Result<Option<TotalCost>, CardTextError> {
    let cst =
        match crate::runtime_backend::front_end::leaf::parse_activation_cost_tokens_rewrite(tokens)
        {
            Ok(cst) => cst,
            Err(_) => return Ok(None),
        };
    let [crate::runtime_backend::front_end::grammar::activation_costs::ActivationCostSegmentCst::SacrificeChosen {
        count,
        filter,
    }] = cst.segments.as_slice()
    else {
        return Ok(None);
    };
    if !cst.alternative_branches.is_empty()
        || count.dynamic_x
        || count.min != 1
        || count.max != Some(1)
    {
        return Ok(None);
    }

    Ok(Some(TotalCost::from_cost(crate::costs::Cost::sacrifice(
        filter.clone(),
    ))))
}

#[rustfmt::skip]
pub(crate) fn parse_ward_discard_card_type_cost(tokens: &[OwnedLexToken]) -> Option<TotalCost> {
    let words = LexedClause::new(tokens).words();
    if !words
        .first()
        .is_some_and(|word| word == "discard")
    {
        return None;
    }

    let mut idx = 1usize;
    let mut count = 1u32;
    let count_token_idx = static_keyword_shapes::parse_word_token_offset(tokens, idx).unwrap_or(tokens.len());
    if let Some((value, used)) = parse_number(&tokens[count_token_idx..]) {
        count = value;
        let used_end = count_token_idx.saturating_add(used).min(tokens.len());
        idx += LexedClause::new(&tokens[count_token_idx..used_end]).word_len();
    }

    let tail_token_idx = words.token_boundary_for_word_or_end(idx).unwrap_or(tokens.len());
    if WARD_DISCARD_HAND_TAIL_PATTERN.matches(LexedClause::new(&tokens[tail_token_idx..])) {
        return Some(TotalCost::from_cost(crate::costs::Cost::discard_hand()));
    }

    while words
        .get(idx)
        .is_some_and(|word| is_article(word))
    {
        idx += 1;
    }

    let mut card_types = Vec::<CardType>::new();
    while let Some(word) = words.get(idx) {
        if matches!(word, "card" | "cards") {
            idx += 1;
            break;
        }
        if matches!(word, "and" | "or" | "a" | "an") {
            idx += 1;
            continue;
        }
        let parsed = parse_card_type(word)?;
        if !card_types.iter().any(|existing| *existing == parsed) {
            card_types.push(parsed);
        }
        idx += 1;
    }

    if idx != words.len() {
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
    let anthem_head = static_keyword_line_shapes::parse_composed_anthem_head(tokens);
    if matches!(
        anthem_head,
        static_keyword_line_shapes::ComposedAnthemHead::Temporary
    ) {
        return Ok(None);
    }

    let comma_segments = anthem_grant_grammar::split_trailing_grant_segments(tokens);
    if comma_segments.len() < 2 {
        return Ok(None);
    }

    if comma_segments.len() == 2 {
        let where_tail = trim_commas(&comma_segments[1]);
        if keyword_static_lines::parse_where_x_value_prefix_tokens(&where_tail).is_some()
            && let Some(ability) = parse_anthem_line(tokens)?
        {
            return Ok(Some(vec![ability.into()]));
        }
    }

    let first_action_idx = match anthem_head {
        static_keyword_line_shapes::ComposedAnthemHead::Temporary => return Ok(None),
        static_keyword_line_shapes::ComposedAnthemHead::Permanent {
            action: Some(action),
        } => action.token,
        static_keyword_line_shapes::ComposedAnthemHead::Permanent { action: None } => {
            return Ok(None);
        }
    };

    let subject_tokens = trim_commas(&tokens[..first_action_idx]);
    // This is only a speculative shape check.  A later static-line family may
    // own the line, so do not retain recovery diagnostics from this rejected
    // branch.  The committed anthem branches below parse the subject again
    // and retain any genuine recovery loss.
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let (subject_result, _) = crate::parse_loss::capture(|| parse_anthem_subject(&subject_tokens));
    if subject_result.is_err() {
        return Ok(None);
    }

    let mut saw_omitted_subject_clause = false;
    let mut compiled = Vec::new();

    for (idx, raw_segment) in comma_segments.into_iter().enumerate() {
        let Some(parsed) = keyword_static_lines::parse_composed_anthem_segment_tokens(&raw_segment)
        else {
            continue;
        };
        let mut segment = parsed.body_tokens.to_vec();
        if parsed.omitted_subject {
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

    if keyword_static_lines::parse_dungeon_room_trigger_duplication_marker_tokens(tokens) {
        return Some(StaticAbility::dungeon_room_trigger_duplication(
            "Room abilities of dungeons you own trigger an additional time.",
        ));
    }

    if is_once_each_turn_play_from_exile_marker_guard_lexed(tokens) {
        return None;
    }

    if is_doctors_companion_marker_line_lexed(tokens) {
        return Some(StaticAbility::doctors_companion());
    }

    if let Some(marker) = keyword_static_lines::parse_static_text_marker_kind_tokens(tokens) {
        return Some(match marker {
            keyword_static_lines::StaticTextMarkerKind::Banding => StaticAbility::banding(),
            keyword_static_lines::StaticTextMarkerKind::YouHaveHexproof => {
                StaticAbility::restriction(
                    crate::effect::Restriction::be_targeted_player_from(
                        PlayerFilter::You,
                        ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
                    ),
                    "You have hexproof".to_string(),
                )
            }
            keyword_static_lines::StaticTextMarkerKind::YouHaveProtectionFromOpponents => {
                StaticAbility::restriction(
                    crate::effect::Restriction::be_targeted_player_from(
                        PlayerFilter::You,
                        ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
                    ),
                    "You have protection from each of your opponents".to_string(),
                )
            }
            keyword_static_lines::StaticTextMarkerKind::OpponentsCastOnlyAsSorcery => {
                StaticAbility::restriction(
                    crate::effect::Restriction::cast_spells_only_as_sorcery(
                        PlayerFilter::Opponent,
                    ),
                    "Each opponent can cast spells only any time they could cast a sorcery."
                        .to_string(),
                )
            }
            keyword_static_lines::StaticTextMarkerKind::DoubleDamageToEnchantedPlayer => {
                StaticAbility::double_damage_amount_replacement(
                    ObjectFilter::default(),
                    Some(PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted"))),
                    None,
                    "If a source would deal damage to enchanted player, it deals double that damage to that player instead.".to_string(),
                )
            }
        });
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
        let condition = Condition::Not(Box::new(Condition::ObjectEnteredBattlefieldThisTurn(
            ObjectFilter::source(),
        )));
        return Some(StaticAbility::new(
            GrantAbility::source(StaticAbility::can_attack_as_though_haste())
                .with_condition(condition),
        ));
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

    if let Ok(Some(ward)) = parse_ward_static_ability_line(tokens) {
        return Some(ward);
    }

    if is_lands_dont_untap_during_their_controllers_untap_steps_line_lexed(tokens) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::untap(ObjectFilter::land()),
            "Lands don't untap during their controllers' untap steps".to_string(),
        ));
    }

    None
}

pub(crate) fn parse_affinity_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let core_tokens = if let Some(paren_idx) = find_token_kind(tokens, TokenKind::LParen) {
        trim_commas(&tokens[..paren_idx])
    } else {
        trim_commas(tokens)
    };
    let Some(spec) = static_keyword_line_shapes::parse_affinity_for_filter(&core_tokens) else {
        return Ok(None);
    };

    if spec.is_artifacts {
        return Ok(None);
    }

    let filter_tokens = trim_commas(spec.filter_tokens);
    let mut filter = parse_object_filter_lexed(&filter_tokens, false)?;
    if filter.controller.is_none() {
        filter.controller = Some(PlayerFilter::You);
    }
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }

    Ok(Some(StaticAbility::new(
        crate::static_abilities::ThisSpellCostReduction::new(
            Value::Count(filter.clone()),
            crate::static_abilities::ThisSpellCostCondition::Always,
        )
        .with_affinity_filter(filter),
    )))
}

pub(crate) fn parse_filter_dont_untap_during_controllers_untap_steps_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_dont_untap_during_controllers_step_tokens(tokens)
    else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.subject_tokens);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter(&subject_tokens, false)?;
    let subject_text = render_token_slice(&subject_tokens);
    let mut display = if spec.singular_subject {
        format!("{subject_text} doesn't untap during its controller's untap step")
    } else {
        format!("{subject_text} don't untap during their controllers' untap steps")
    };
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
    let Some(quantified) = keyword_static_lines::parse_there_is_or_are_quantified_tokens(tokens)
    else {
        return Ok(None);
    };
    let Ok((comparison, used)) = parse_static_quantity_prefix(quantified, false) else {
        return Ok(None);
    };
    let Some(threshold) = comparison_to_at_least_threshold(&comparison) else {
        return Ok(None);
    };

    let rest = &quantified[used..];
    let metric = match keyword_static_lines::parse_graveyard_metric_tokens(rest) {
        Some(keyword_static_lines::GraveyardMetricKind::CardTypes) => {
            crate::static_abilities::GraveyardCountMetric::CardTypes
        }
        Some(keyword_static_lines::GraveyardMetricKind::ManaValues) => {
            crate::static_abilities::GraveyardCountMetric::ManaValues
        }
        None => return Ok(None),
    };
    Ok(Some((metric, threshold)))
}

pub(crate) fn parse_conditional_source_spell_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = static_keyword_line_shapes::parse_conditional_spell_keyword(tokens) else {
        return Ok(None);
    };
    let keyword = match spec.keyword {
        static_keyword_line_shapes::ConditionalSpellKeyword::Flash => {
            crate::static_abilities::ConditionalSpellKeywordKind::Flash
        }
        static_keyword_line_shapes::ConditionalSpellKeyword::Cascade => {
            crate::static_abilities::ConditionalSpellKeywordKind::Cascade
        }
    };

    let condition_tokens = trim_commas(spec.condition_tokens);
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

#[rustfmt::skip]
pub(crate) fn parse_enters_tapped_with_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if !ENTERS_TAPPED_LINE_PATTERN.matches(clause) {
        return Ok(None);
    }
    let words = clause.words();
    let tapped_word_idx = words
        .find_window_by(1, |window| {
            window
                .first()
                .copied()
                .is_some_and(|word| word == "tapped")
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing tapped keyword in enters-tapped clause (clause: '{}')",
                render_token_slice(tokens)
            ))
        })?;
    let tapped_token_idx = static_keyword_shapes::parse_word_token_offset(tokens, tapped_word_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map tapped keyword in enters-tapped clause (clause: '{}')",
                render_token_slice(tokens)
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
    if keyword_static_lines::parse_damage_not_removed_cleanup_tokens(tokens) {
        return Ok(Some(StaticAbility::damage_not_removed_during_cleanup()));
    }
    Ok(None)
}

fn parse_as_enters_choice_subject_tokens<'a>(
    tokens: &'a [OwnedLexToken],
    this_kind_display_pairs: &[(&str, &'static str)],
) -> Option<(&'a [OwnedLexToken], &'static str)> {
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    let (tail_word_idx, display_subject) =
        parse_as_enters_choice_subject_clause(clause, this_kind_display_pairs)?;
    let tail_token_idx = words.token_boundary_for_word_or_end(tail_word_idx)?;
    Some((&tokens[tail_token_idx..], display_subject))
}

fn parse_as_enters_choice_subject_clause(
    clause: LexedClause<'_>,
    this_kind_display_pairs: &[(&str, &'static str)],
) -> Option<(usize, &'static str)> {
    let word_refs = clause.word_refs();
    let words = word_refs.as_slice();
    let allowed_kinds = this_kind_display_pairs
        .iter()
        .map(|(kind, _)| *kind)
        .collect::<Vec<_>>();
    let shape = static_keyword_line_shapes::parse_as_enters_subject(words, &allowed_kinds)?;
    let display_subject = match shape.subject {
        static_keyword_line_shapes::AsEntersSubject::This(Some(kind)) => {
            let mut display = None;
            for (candidate, candidate_display) in this_kind_display_pairs {
                if *candidate == kind {
                    display = Some(*candidate_display);
                    break;
                }
            }
            display?
        }
        static_keyword_line_shapes::AsEntersSubject::It => "it",
        static_keyword_line_shapes::AsEntersSubject::This(None)
        | static_keyword_line_shapes::AsEntersSubject::SourceReference => "this",
    };
    Some((shape.tail_word, display_subject))
}

pub(crate) fn parse_choose_basic_land_type_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_AURA_SUBJECTS).or_else(|| {
            parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
        })
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    if let Some(consumed) = parse_choose_basic_land_type_phrase_words(&tail_words) {
        if consumed == tail_words.len() {
            return Ok(Some(StaticAbility::choose_basic_land_type_as_enters(
                format!("As {display_subject} enters, choose a basic land type."),
            )));
        }
    }
    let Some(consumed) = parse_choose_land_type_phrase_words(&tail_words) else {
        return Ok(None);
    };
    if consumed != tail_words.len() {
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
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    if !CHOOSE_CARD_NAME_TAIL_PATTERN.matches(LexedClause::new(tail_tokens)) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_card_name_as_enters(format!(
        "As {display_subject} enters, choose a card name."
    ))))
}

pub(crate) fn parse_revealed_hand_choose_nonland_card_name_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() != 2 {
        return Ok(None);
    }

    let first_clause = LexedClause::new(sentences[0]);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_clause(first_clause, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let first_tail = first_clause.after_words(idx).map(LexedClause::tokens);
    if !first_tail.is_some_and(keyword_static_lines::parse_revealed_hand_as_enters_tail_tokens) {
        return Ok(None);
    }

    if !keyword_static_lines::parse_choose_revealed_nonland_name_tail_tokens(sentences[1]) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::choose_revealed_hand_nonland_card_name_as_enters(format!(
            "As {display_subject} enters, each opponent reveals their hand. You choose the name of a nonland card revealed this way."
        )),
    ))
}

pub(crate) fn parse_note_life_total_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    if !NOTE_YOUR_LIFE_TOTAL_TAIL_PATTERN.matches(LexedClause::new(tail_tokens)) {
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
    let Some((subject, has_article)) =
        keyword_static_lines::parse_source_is_chosen_color_tokens(tokens)
    else {
        return Ok(None);
    };
    let display_subject = subject.display();
    let display = if has_article {
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
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS)
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    let Some((consumed, excluded_subtypes)) = parse_choose_creature_type_phrase_words(&tail_words)?
    else {
        return Ok(None);
    };
    if !excluded_subtypes.is_empty() {
        return Ok(None);
    }
    if consumed != tail_words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_creature_type_as_enters(
        format!("As {display_subject} enters, choose a creature type."),
    )))
}

pub(crate) fn parse_choose_named_options_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS)
    else {
        return Ok(None);
    };
    let tail_clause = LexedClause::new(tail_tokens);
    let tail_words = tail_clause.word_refs();
    let Some(choice_offset) =
        static_keyword_line_shapes::parse_choice_word(&tail_words).map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    let choice_words = &tail_words[choice_offset..];
    let Some(choice_clause) = tail_clause.between_word_range(choice_offset, tail_words.len())
    else {
        return Ok(None);
    };
    if choice_words.len() < 4 || !CHOICE_OR_PATTERN.matches(choice_clause) {
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

    let mut card_type_options = Vec::new();
    for word in choice_words.iter().skip(1) {
        if *word == "or" || *word == "," {
            continue;
        }
        let Some(card_type) = parse_card_type(word.trim_end_matches('s')) else {
            card_type_options.clear();
            break;
        };
        crate::slice_primitives::push_unique(&mut card_type_options, card_type);
    }
    if card_type_options.len() >= 2 {
        return Ok(Some(StaticAbility::choose_named_option_as_enters(
            card_type_options
                .iter()
                .map(|card_type| card_type.name().to_string())
                .collect(),
            format!("As {display_subject} enters, {}.", choice_words.join(" ")),
        )));
    }

    let mut options = Vec::new();
    let mut current = Vec::new();
    for word in choice_words.iter().skip(1) {
        if *word == "or" {
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

fn trigger_duplication_tail_matches(tokens: &[OwnedLexToken]) -> bool {
    keyword_static_lines::parse_trigger_duplication_tail_tokens(tokens)
}

fn parse_trigger_duplication_source_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if TRIGGER_DUPLICATION_SOURCE_PATTERN.matches(LexedClause::new(&tokens)) {
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
    let clause = LexedClause::new(&tokens);
    let clause_display = render_token_slice(&tokens);

    let build_filter = |subject_tokens: &[OwnedLexToken]| -> Result<ObjectFilter, CardTextError> {
        parse_object_filter_with_grammar_entrypoint(&trim_edge_punctuation(subject_tokens), false)
    };

    if TURNING_FACE_UP_TRIGGER_DUPLICATION_PATTERN.matches(clause) {
        if tokens.len() <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing turned-face-up subject in trigger-duplication clause (clause: '{}')",
                clause_display
            )));
        }
        let filter = build_filter(&tokens[1..tokens.len() - 2])?;
        return Ok(Trigger::turned_face_up(filter));
    }

    if YOU_CASTING_OR_COPYING_PREFIX_PATTERN.matches(clause) {
        if tokens.len() <= 4 {
            return Err(CardTextError::ParseError(format!(
                "missing spell subject in trigger-duplication clause (clause: '{}')",
                clause_display
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
        if !ClauseShape::new().suffix(suffix).matches(clause) || clause.word_len() <= suffix.len() {
            continue;
        }
        let subject_len = clause.word_len() - suffix.len();
        if suffix == &["drawing", "a", "card"] {
            let Some(subject_clause) = clause.between_word_range(0, subject_len) else {
                return Err(CardTextError::ParseError(format!(
                    "failed to split trigger-duplication subject (clause: '{}')",
                    clause_display
                )));
            };
            if PLAYER_SUBJECT_PATTERN.matches(subject_clause) {
                return Ok(Trigger::player_draws_card(PlayerFilter::Any));
            }
            if YOU_SUBJECT_PATTERN.matches(subject_clause) {
                return Ok(Trigger::player_draws_card(PlayerFilter::You));
            }
            if OPPONENT_SUBJECT_PATTERN.matches(subject_clause) {
                return Ok(Trigger::player_draws_card(PlayerFilter::Opponent));
            }
        }
        let Some(subject_end_token_idx) =
            static_keyword_shapes::parse_word_token_offset(&tokens, subject_len)
        else {
            return Err(CardTextError::ParseError(format!(
                "failed to split trigger-duplication subject (clause: '{}')",
                clause_display
            )));
        };
        let filter = build_filter(&tokens[..subject_end_token_idx])?;
        return Ok(build(filter));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported trigger-duplication cause clause (clause: '{}')",
        clause_display
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

    if !trigger_duplication_tail_matches(&tail_tokens) {
        return Ok(None);
    }

    if !IF_PREFIX_PATTERN.matches(LexedClause::new(&head_tokens)) || head_tokens.len() < 2 {
        return Ok(None);
    }

    let body_tokens = &head_tokens[1..];
    let body_clause = LexedClause::new(body_tokens);
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
        if !ClauseShape::new().prefix(prefix).matches(body_clause)
            || body_tokens.len() <= prefix.len() + 1
        {
            continue;
        }
        let Some(triggers_idx) =
            static_keyword_line_shapes::parse_trigger_duplication_triggers_word(&body_words)
                .map(|boundary| boundary.word)
        else {
            continue;
        };
        if triggers_idx <= prefix.len() {
            continue;
        }

        let condition = if body_clause
            .after_words(triggers_idx + 1)
            .is_some_and(|tail| WHILE_PREFIX_PATTERN.matches(tail))
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
            static_keyword_line_shapes::parse_trigger_duplication_causes_word(&body_words)
                .map(|boundary| boundary.word)
    {
        let cause_tokens = &body_tokens[..causes_idx];
        let source_body_tokens = &body_tokens[causes_idx + 1..];
        let source_body_clause = LexedClause::new(source_body_tokens);
        for prefix in ability_prefixes {
            if !ClauseShape::new()
                .prefix(prefix)
                .matches(source_body_clause)
                || source_body_tokens.len() <= prefix.len() + 2
            {
                continue;
            }
            if !TO_TRIGGER_SUFFIX_PATTERN.matches(source_body_clause) {
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
    let Some(spec) = keyword_static_lines::parse_additive_damage_amount_tokens(&tokens) else {
        return Ok(None);
    };
    let damaged_words = parser_token_word_refs(spec.damaged_tokens);
    let (target_player_filter, target_object_filter) =
        parse_damage_amount_replacement_target_filters(&damaged_words)?;
    if target_player_filter.is_none() && target_object_filter.is_none() {
        return Ok(None);
    }
    if let Some(repeated_target_tokens) = spec.repeated_target_tokens {
        let repeated_words = parser_token_word_refs(repeated_target_tokens);
        let (repeated_player_filter, repeated_object_filter) =
            parse_damage_amount_replacement_target_filters(&repeated_words)?;
        if repeated_player_filter.as_ref() != target_player_filter.as_ref()
            || repeated_object_filter.as_ref() != target_object_filter.as_ref()
        {
            return Ok(None);
        }
    }
    let source_filter = damage_source_filter_from_shape(spec.source)?;

    let mut display = render_token_slice(&tokens).trim().to_string();
    if !crate::string_primitives::ends_with_char(&display, '.') {
        display.push('.');
    }
    Ok(Some(
        StaticAbility::modify_damage_amount_replacement_with_noncombat_only(
            source_filter,
            target_player_filter,
            target_object_filter,
            spec.delta,
            spec.noncombat_only,
            display,
        ),
    ))
}

pub(crate) fn parse_double_damage_amount_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(spec) = keyword_static_lines::parse_damage_multiplier_tokens(&tokens) else {
        return Ok(None);
    };
    let damaged_words = parser_token_word_refs(spec.damaged_tokens);

    let (target_player_filter, target_object_filter) =
        parse_damage_amount_replacement_target_filters(&damaged_words)?;
    if target_player_filter.is_none() && target_object_filter.is_none() {
        return Ok(None);
    }

    let source_filter = damage_source_filter_from_shape(spec.source)?;

    let mut display = render_token_slice(&tokens).trim().to_string();
    if !crate::string_primitives::ends_with_char(&display, '.') {
        display.push('.');
    }

    Ok(Some(StaticAbility::multiply_damage_amount_replacement(
        source_filter,
        target_player_filter,
        target_object_filter,
        spec.factor,
        spec.combat_only,
        display,
    )))
}

fn damage_source_filter_from_shape(
    shape: keyword_static_lines::DamageSourceShape<'_>,
) -> Result<ObjectFilter, CardTextError> {
    let mut filter = if shape.filter_tokens.is_empty() {
        ObjectFilter::default()
    } else {
        parse_object_filter_lexed(shape.filter_tokens, false)?
    };
    match shape.controller {
        keyword_static_lines::DamageSourceControllerKind::None => {}
        keyword_static_lines::DamageSourceControllerKind::You => filter = filter.you_control(),
        keyword_static_lines::DamageSourceControllerKind::Opponent => {
            filter = filter.controlled_by(PlayerFilter::Opponent);
        }
    }
    Ok(filter)
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
    if !keyword_static_lines::parse_minimum_red_noncombat_damage_tokens(&tokens) {
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

pub(crate) fn parse_enter_as_copy_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    fn parse_added_copy_abilities(
        ability_tokens: &[OwnedLexToken],
        clause_words: &[&str],
    ) -> Result<Vec<crate::ability::Ability>, CardTextError> {
        if ability_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported empty enters-as-copy ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let (abilities, _choice) =
            parse_granted_abilities_for_gain_clause(ability_tokens, clause_words, false)?;
        if abilities.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported enters-as-copy ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        lower_granted_abilities_ast_to_object_abilities(&abilities)
    }

    let Some(shape) = keyword_static_lines::parse_enter_as_copy_tokens(tokens) else {
        return Ok(None);
    };
    let clause_words = parser_token_word_refs(tokens);
    let display = render_token_slice(tokens).trim().to_string();

    match shape {
        keyword_static_lines::EnterAsCopyShape::LinkedExilePair(shape) => {
            if shape.exile_count != 2
                || shape.copy_count != 1
                || shape.counter_value
                    != keyword_static_lines::LinkedExileCopyCounterValue::OtherCardPower
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported linked-exile enters-as-copy configuration: exile {}, copy {}, counter value {:?}",
                    shape.exile_count, shape.copy_count, shape.counter_value
                )));
            }
            return Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
                crate::static_abilities::EnterAsCopyAsEntersSpec {
                    filter: shape.filter,
                    affected_filter: None,
                    may: shape.may,
                    enters_tapped_if_chosen: false,
                    linked_exile_pair: Some(
                        crate::static_abilities::EnterAsCopyLinkedExilePairSpec {
                            counter_type: shape.counter_type,
                        },
                    ),
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
        keyword_static_lines::EnterAsCopyShape::Direct {
            affected_tokens,
            copy_source_tokens,
            copy_source_kind,
        } => {
            let affected_filter = parse_object_filter(affected_tokens, false)?;
            let (filter, copy_source_self, copy_source_enchanted) = match copy_source_kind {
                keyword_static_lines::CopySourceKind::Source => {
                    (ObjectFilter::source(), true, false)
                }
                keyword_static_lines::CopySourceKind::Enchanted => {
                    (ObjectFilter::source(), false, true)
                }
                keyword_static_lines::CopySourceKind::Filter => (
                    parse_object_filter(copy_source_tokens, false)?,
                    false,
                    false,
                ),
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
                display,
            )));
        }
        keyword_static_lines::EnterAsCopyShape::May {
            named_subject_tokens,
            enters_tapped,
            filter_tokens,
            exception_display_split,
            exception_tokens,
        } => {
            let display = if let Some(split) = exception_display_split {
                format!(
                    "{} {}",
                    render_token_slice(split.before_separator).trim(),
                    render_token_slice(split.after_separator).trim()
                )
            } else {
                display
            };
            let filter = parse_object_filter(filter_tokens, false)?;
            let named_copy_subject = named_subject_tokens.map(|subject_tokens| {
                parser_token_word_refs(subject_tokens)
                    .into_iter()
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
                    .join(" ")
            });

            let mut name_override = None;
            let mut added_card_types = Vec::new();
            let mut removed_supertypes = Vec::new();
            let mut added_subtypes = Vec::new();
            let mut added_abilities = Vec::new();
            let mut set_base_power_toughness = None;
            let mut set_base_power_toughness_from_self = false;

            if let Some(exception_tokens) = exception_tokens {
                let exception = keyword_static_lines::parse_copy_exception_tokens(exception_tokens)
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported enters-as-copy exception clause (clause: '{}')",
                            clause_words.join(" ")
                        ))
                    })?;
                match exception {
                    keyword_static_lines::CopyExceptionShape::Name {
                        name_tokens,
                        use_named_subject,
                    } => {
                        if use_named_subject {
                            name_override = named_copy_subject.clone();
                        } else {
                            let name_words = parser_token_word_refs(name_tokens);
                            if !name_words.is_empty() {
                                name_override = Some(name_words.join(" "));
                            }
                        }
                    }
                    keyword_static_lines::CopyExceptionShape::Abilities { ability_tokens } => {
                        added_abilities =
                            parse_added_copy_abilities(ability_tokens, &clause_words)?;
                    }
                    keyword_static_lines::CopyExceptionShape::Characteristics {
                        remove_legendary,
                        characteristic_tokens,
                        remainder,
                    } => {
                        if remove_legendary {
                            removed_supertypes.push(crate::types::Supertype::Legendary);
                        }
                        let characteristic_words = parser_token_word_refs(characteristic_tokens);
                        if characteristic_words.is_empty() {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported enters-as-copy exception clause (clause: '{}')",
                                clause_words.join(" ")
                            )));
                        }
                        let mut cursor = 0usize;
                        if let Ok((power, toughness)) =
                            parse_pt_modifier(characteristic_words[cursor])
                        {
                            set_base_power_toughness = Some((power, toughness));
                            cursor += 1;
                        }
                        let mut parsed_type_or_subtype = false;
                        while cursor < characteristic_words.len() {
                            if let Some(card_type) = parse_card_type(characteristic_words[cursor]) {
                                crate::slice_primitives::push_unique(
                                    &mut added_card_types,
                                    card_type,
                                );
                                parsed_type_or_subtype = true;
                                cursor += 1;
                                continue;
                            }
                            if let Some(subtype) = parse_subtype_word(characteristic_words[cursor])
                                .or_else(|| parse_subtype_flexible(characteristic_words[cursor]))
                            {
                                crate::slice_primitives::push_unique(&mut added_subtypes, subtype);
                                parsed_type_or_subtype = true;
                                cursor += 1;
                                continue;
                            }
                            break;
                        }
                        if (!parsed_type_or_subtype && set_base_power_toughness.is_none())
                            || cursor != characteristic_words.len()
                        {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported enters-as-copy type '{}' (clause: '{}')",
                                characteristic_words
                                    [cursor.min(characteristic_words.len().saturating_sub(1))],
                                clause_words.join(" ")
                            )));
                        }
                        match remainder {
                            keyword_static_lines::CopyCharacteristicRemainder::None => {}
                            keyword_static_lines::CopyCharacteristicRemainder::PowerToughnessFromSource => {
                                set_base_power_toughness_from_self = true;
                            }
                            keyword_static_lines::CopyCharacteristicRemainder::Abilities(
                                ability_tokens,
                            ) => {
                                added_abilities =
                                    parse_added_copy_abilities(ability_tokens, &clause_words)?;
                            }
                            keyword_static_lines::CopyCharacteristicRemainder::Unsupported => {
                                return Err(CardTextError::ParseError(format!(
                                    "unsupported enters-as-copy exception clause (clause: '{}')",
                                    clause_words.join(" ")
                                )));
                            }
                        }
                    }
                }
            }

            Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
                crate::static_abilities::EnterAsCopyAsEntersSpec {
                    filter,
                    affected_filter: None,
                    may: true,
                    enters_tapped_if_chosen: enters_tapped,
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
                display,
            )))
        }
    }
}

pub(crate) fn parse_choose_color_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    let Some((consumed, excluded_color_set)) = parse_choose_color_phrase_words(&tail_words)? else {
        return Ok(None);
    };
    if consumed != tail_words.len() {
        return Ok(None);
    }

    let excluded = if let Some(color_set) = excluded_color_set {
        Some(color_from_color_set(color_set).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "ambiguous color choice in choose-color clause (clause: '{}')",
                render_token_slice(tokens)
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
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    if words.len() < 6 || !AS_THIS_PREFIX_PATTERN.matches(clause) {
        return Ok(None);
    }
    let display_subject = match words.get(2) {
        Some("equipment") => "this Equipment",
        Some("aura") => "this Aura",
        Some("permanent") => "this permanent",
        Some("artifact") => "this artifact",
        Some("enchantment") => "this enchantment",
        _ => return Ok(None),
    };
    let Some(attached_tail_idx) = static_keyword_shapes::parse_word_token_offset(tokens, 3) else {
        return Ok(None);
    };
    let attached_tail = LexedClause::new(&tokens[attached_tail_idx..]);
    if !BECOMES_ATTACHED_TO_TAIL_PATTERN.matches(attached_tail.before(3)) {
        return Ok(None);
    }
    let Some(choose_idx) = words.find_window_by(1, |window| {
        window.first().copied().is_some_and(|word| word == "choose")
    }) else {
        return Ok(None);
    };
    if choose_idx <= 6 {
        return Ok(None);
    }
    let word_refs = words.word_refs();
    let Some((consumed, excluded_color_set)) =
        parse_choose_color_phrase_words(&word_refs[choose_idx..])?
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
                render_token_slice(tokens)
            ))
        })?)
    } else {
        None
    };
    if excluded.is_some() {
        return Err(CardTextError::ParseError(format!(
            "excluded color choices for attachment choices are not supported yet (clause: '{}')",
            render_token_slice(tokens)
        )));
    }

    Ok(Some(StaticAbility::choose_color_as_becomes_attached(
        format!("As {display_subject} becomes attached, choose a color."),
    )))
}

pub(crate) fn parse_choose_player_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    let Some(consumed) = parse_choose_player_phrase_words(&tail_words) else {
        return Ok(None);
    };
    if consumed != tail_words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_player_as_enters(format!(
        "As {display_subject} enters, choose a player."
    ))))
}

pub(crate) fn parse_damage_redirect_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    if words.len() != 19 {
        return Ok(None);
    }
    let Some(tail_idx) = static_keyword_shapes::parse_word_token_offset(tokens, 11) else {
        return Ok(None);
    };
    if DAMAGE_REDIRECT_TO_SOURCE_PREFIX_PATTERN.matches(clause)
        && words
            .get(10)
            .is_some_and(|word| matches!(word, "permanent" | "permanents"))
        && DAMAGE_REDIRECT_TO_SOURCE_TAIL_PATTERN.matches(LexedClause::new(&tokens[tail_idx..]))
    {
        return Ok(Some(
            StaticAbility::redirect_damage_from_you_and_other_permanents_to_source(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_damage_redirect_to_source_controller_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_damage_redirect_controller_tokens(tokens) else {
        return Ok(None);
    };
    let source_filter = parse_object_filter_lexed(spec.source_tokens, false)?;
    let mut display = render_token_slice(tokens).trim().to_string();
    if !crate::string_primitives::ends_with_char(&display, '.') {
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
    if tokens.len() < 8 {
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

    let tail = &tokens[used..];
    let Some(kind) = keyword_static_lines::parse_combat_maximum_tail_tokens(tail) else {
        return Ok(None);
    };
    let ability = match kind {
        keyword_static_lines::CombatMaximumKind::AttackYou => {
            StaticAbility::max_attackers_can_attack_you_each_combat(maximum as usize)
        }
        keyword_static_lines::CombatMaximumKind::Attack => {
            StaticAbility::max_attackers_each_combat(maximum as usize)
        }
        keyword_static_lines::CombatMaximumKind::Block => {
            StaticAbility::max_blockers_each_combat(maximum as usize)
        }
    };
    Ok(Some(ability))
}

pub(crate) fn parse_characteristic_defining_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let sentence_tokens = trim_edge_punctuation(tokens);
    if split_lexed_sentences(&sentence_tokens).len() > 1 {
        return Ok(None);
    }

    if let Some(tail_tokens) =
        keyword_static_lines::parse_characteristic_shared_value_tail_tokens(tokens)
    {
        if !tail_tokens.is_empty() {
            let value = parse_characteristic_defining_stat_value(tail_tokens).ok_or_else(|| {
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

    let line_words = LexedClause::new(tokens).words();
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
            if line_words.get(and_idx) != Some("and") {
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

        let Some(value_start_token_idx) =
            static_keyword_shapes::parse_word_token_offset(tokens, value_start_word_idx)
        else {
            break;
        };
        let value_end_token_idx = if value_end_word_idx < line_words.len() {
            static_keyword_shapes::parse_word_token_offset(tokens, value_end_word_idx)
                .unwrap_or(tokens.len())
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
    match keyword_static_lines::parse_characteristic_relative_value_tokens(tokens)? {
        keyword_static_lines::CharacteristicRelativeValue::Same => Some(base.clone()),
        keyword_static_lines::CharacteristicRelativeValue::Plus(amount) => Some(Value::Add(
            Box::new(base.clone()),
            Box::new(Value::Fixed(amount as i32)),
        )),
    }
}

fn parse_characteristic_axis_clause_start<'a>(
    words: &TokenWordView<'a>,
    idx: usize,
) -> Option<(&'a str, usize)> {
    let first = words.get(idx)?;
    if is_characteristic_axis_word(first) && characteristic_equal_to_at(words, idx + 1) {
        return Some((first, idx + 4));
    }

    if first == "creature"
        && words.get(idx + 1).is_some_and(is_characteristic_axis_word)
        && characteristic_equal_to_at(words, idx + 2)
    {
        return Some((words.get(idx + 1)?, idx + 5));
    }

    if !matches!(first, "this" | "thiss" | "its") {
        return None;
    }

    if words.get(idx + 1).is_some_and(is_characteristic_axis_word)
        && characteristic_equal_to_at(words, idx + 2)
    {
        return Some((words.get(idx + 1)?, idx + 5));
    }

    if words.get(idx + 1).is_some_and(|word| word == "creature")
        && words.get(idx + 2).is_some_and(is_characteristic_axis_word)
        && characteristic_equal_to_at(words, idx + 3)
    {
        return Some((words.get(idx + 2)?, idx + 6));
    }

    None
}

fn is_characteristic_axis_word(word: &str) -> bool {
    matches!(word, "power" | "toughness")
}

fn characteristic_equal_to_at(words: &TokenWordView<'_>, idx: usize) -> bool {
    words.get(idx).is_some_and(|word| word == "is")
        && words.get(idx + 1).is_some_and(|word| word == "equal")
        && words.get(idx + 2).is_some_and(|word| word == "to")
}

fn parse_characteristic_defining_stat_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let trimmed = trim_edge_punctuation_tokens(tokens);
    let words = LexedClause::new(trimmed).words();
    if words.is_empty() {
        return None;
    }

    if let Some(kind) = keyword_static_lines::parse_characteristic_source_value_tokens(trimmed) {
        return Some(match kind {
            keyword_static_lines::CharacteristicSourceValueKind::Power => Value::SourcePower,
            keyword_static_lines::CharacteristicSourceValueKind::Toughness => {
                Value::SourceToughness
            }
        });
    }

    let mut equal_prefixed = Vec::with_capacity(trimmed.len() + 2);
    equal_prefixed.push(OwnedLexToken::word(
        "equal".to_string(),
        TextSpan::synthetic(),
    ));
    equal_prefixed.push(OwnedLexToken::word("to".to_string(), TextSpan::synthetic()));
    equal_prefixed.extend(trimmed.iter().cloned());

    if keyword_static_lines::characteristic_tokens_have_card_types_among_marker(trimmed)
        && let Some(value) = parse_characteristic_defining_pt_value(trimmed)
    {
        return Some(value);
    }

    parse_equal_to_aggregate_filter_value(&equal_prefixed)
        .or_else(|| parse_add_mana_equal_amount_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_filter_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(&equal_prefixed))
        .or_else(|| parse_characteristic_defining_pt_value(trimmed))
}

pub(crate) fn parse_characteristic_defining_pt_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = LexedClause::new(tokens).words();
    if words.is_empty() {
        return None;
    }

    let plus_positions: Vec<usize> = words
        .word_refs()
        .into_iter()
        .enumerate()
        .filter_map(|(idx, word)| (word == "plus").then_some(idx))
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

    let start = keyword_static_lines::strip_characteristic_number_of_prefix_tokens(tokens);
    if start.is_empty() {
        return None;
    }

    // "the number of cards in the hand of the opponent with the most cards in hand"
    // (Adamaro, First to Desire)
    if let Some(value) = parse_max_cards_in_hand_value_lexed(start) {
        return Some(value);
    }

    if let Some(aggregate) =
        keyword_static_lines::parse_characteristic_aggregate_prefix_tokens(start)
    {
        if aggregate.kind != keyword_static_lines::CharacteristicAggregateKind::CardTypes {
            return parse_aggregate_scope_value_lexed(start);
        }
        let scope_tokens = trim_commas(aggregate.scope_tokens);
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

pub(crate) fn parse_you_assign_combat_damage_of_creatures_attacking_you_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_assign_combat_damage_of_creatures_attacking_you_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::you_assign_combat_damage_of_creatures_attacking_you(),
        ));
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
    let Some(amount) = keyword_static_lines::parse_starting_life_bonus_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(StaticAbility::starting_life_bonus(amount as i32)))
}

pub(crate) fn parse_buyback_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(amount) = keyword_static_lines::parse_buyback_cost_reduction_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(StaticAbility::buyback_cost_reduction(amount)))
}

pub(crate) fn parse_spell_cost_increase_per_target_beyond_first_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(cost_head) = static_keyword_cost_shapes::parse_spell_cost_increase_head(tokens) else {
        return Ok(None);
    };
    let line_start = cost_head.line_start.token;
    if !keyword_static_lines::parse_cost_increase_per_target_marker_tokens(tokens) {
        return Ok(None);
    }

    let search_tokens = &tokens[line_start..];
    let costs_idx = cost_head.costs.token.saturating_sub(line_start);
    let amount_tokens = &search_tokens[costs_idx + 1..];
    if let Some((cost, used)) = parse_cost_modifier_mana_cost(amount_tokens)
        && keyword_static_lines::parse_more_cost_tail_prefix_tokens(
            amount_tokens.get(used..).unwrap_or_default(),
        )
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

    let costs_idx = static_keyword_cost_shapes::parse_cost_verb(spec.tail_tokens)
        .map(|boundary| boundary.token)
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
    let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
    let remaining_words = parser_token_word_refs(remaining_tokens);
    if parse_cost_modifier_direction(&remaining_words) != Some(CostModifierDirection::Less)
        || !CAST_WORD_MARKER_PATTERN.matches(LexedClause::new(remaining_tokens))
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

    let clause = LexedClause::new(tokens);
    let target_start = if IT_TARGETS_PREFIX_PATTERN.matches(clause) {
        2
    } else if THIS_SPELL_TARGETS_PREFIX_PATTERN.matches(clause) {
        3
    } else {
        return None;
    };
    let target_clause = clause.after_words(target_start)?;
    let target_tokens = trim_commas(target_clause.tokens());
    if target_tokens.is_empty() {
        return None;
    }
    let target_clause = LexedClause::new(&target_tokens);
    if YOU_TARGET_PREFIX_PATTERN.matches(target_clause) {
        return Some(ThisSpellCostCondition::TargetsPlayer(PlayerFilter::You));
    }
    if OPPONENT_TARGET_PREFIX_PATTERN.matches(target_clause) {
        return Some(ThisSpellCostCondition::TargetsPlayer(
            PlayerFilter::Opponent,
        ));
    }
    if PLAYER_TARGET_PREFIX_PATTERN.matches(target_clause) {
        return Some(ThisSpellCostCondition::TargetsPlayer(PlayerFilter::Any));
    }
    parse_object_filter(&target_tokens, false)
        .ok()
        .map(ThisSpellCostCondition::TargetsObject)
}

fn this_spell_cost_condition_from_life_change_this_turn(
    condition: PlayerLifeChangeThisTurnConditionAst,
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    if condition.player != PlayerFilter::You
        || condition.direction != PlayerLifeChangeDirectionAst::Gained
    {
        return None;
    }
    let count = comparison_to_at_least_threshold(&condition.comparison)?;
    Some(crate::static_abilities::ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(count))
}

pub(crate) fn parse_this_spell_cost_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;

    let clause = LexedClause::new(tokens);
    let w = clause.word_refs();
    if w.is_empty() {
        return None;
    }

    if let Some(condition) = parse_life_total_or_less_spell_cost_condition(tokens) {
        return Some(condition);
    }
    if LIFE_TOTAL_LESS_THAN_STARTING_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::LifeTotalLessThanStarting);
    }

    if YOU_ATTACKED_THIS_TURN_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::AttackedThisTurn,
            display: w.join(" "),
        });
    }
    if CREATURE_DIED_THIS_TURN_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::CreatureDiedThisTurn,
            display: w.join(" "),
        });
    }
    if let Some(condition) = parse_player_life_change_this_turn_condition(tokens)
        .and_then(this_spell_cost_condition_from_life_change_this_turn)
    {
        return Some(condition);
    }
    if ITS_NIGHT_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::IsNight);
    }
    if THIS_SPELL_BARGAINED_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::ThisSpellPaidLabel("Bargain".into()),
            display: w.join(" "),
        });
    }
    if YOU_SACRIFICED_ARTIFACT_THIS_TURN_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::YouSacrificedArtifactThisTurn);
    }
    if YOU_COMMITTED_CRIME_THIS_TURN_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::YouCommittedCrimeThisTurn);
    }
    if CREATURE_LEFT_BATTLEFIELD_UNDER_YOUR_CONTROL_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::CreatureLeftBattlefieldUnderYourControlThisTurn);
    }
    if YOU_CAST_ANOTHER_PREFIX_PATTERN.matches(clause) && THIS_TURN_SUFFIX_PATTERN.matches(clause) {
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
    if YOU_CAST_PREFIX_PATTERN.matches(clause) && THIS_TURN_SUFFIX_PATTERN.matches(clause) {
        let types = mentioned_instant_sorcery_card_types(&w);
        if !types.is_empty() {
            return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
                count: 1,
                card_types: types,
            });
        }
    }

    if NOT_STARTING_PLAYER_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::NotStartingPlayer);
    }
    if CREATURE_IS_ATTACKING_YOU_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::CreatureIsAttackingYou);
    }
    if CREATURE_CARD_PUT_INTO_GRAVEYARD_THIS_TURN_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::CreatureCardPutIntoYourGraveyardThisTurn);
    }
    if w.len() >= 11
        && THERE_ARE_PREFIX_PATTERN.matches(clause)
        && CARD_TYPES_GRAVEYARD_COUNT_PATTERN.matches(clause)
        && let Some((n, _)) = parse_static_at_least_quantity_at(tokens, 2)
    {
        return Some(ThisSpellCostCondition::DistinctCardTypesInYourGraveyardOrMore(n));
    }
    if YOU_HAVE_IN_YOUR_GRAVEYARD_PATTERN.matches(clause)
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
    if w.len() >= 7 && OPPONENT_HAS_PREFIX_PATTERN.matches(clause) {
        let count_start = count_start_for_optional_an_opponent_prefix(&w, 3)?;
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, count_start) {
            if clause
                .after_words(rest_start)
                .is_some_and(|tail| POISON_COUNTERS_TAIL_PATTERN.matches(tail))
            {
                return Some(ThisSpellCostCondition::OpponentHasPoisonCountersOrMore(n));
            }
            if clause
                .after_words(rest_start)
                .is_some_and(|tail| CARDS_IN_OPPONENT_GRAVEYARD_TAIL_PATTERN.matches(tail))
            {
                return Some(ThisSpellCostCondition::OpponentHasCardsInGraveyardOrMore(n));
            }
        }
    }

    if THERE_ARE_NO_PREFIX_PATTERN.matches(clause) && IN_YOUR_HAND_SUFFIX_PATTERN.matches(clause) {
        let filter_tokens = trim_commas(tokens.get(3..).unwrap_or_default());
        if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
            return Some(ThisSpellCostCondition::NoCardsInHandMatching {
                filter,
                display: w.join(" "),
            });
        }
    }
    if let Some(name) = only_creature_cards_in_hand_named(clause) {
        return Some(ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(name));
    }

    if THERE_IS_PREFIX_PATTERN.matches(clause) && IN_YOUR_GRAVEYARD_SUFFIX_PATTERN.matches(clause) {
        let filter_tokens = trim_commas(tokens.get(2..).unwrap_or_default());
        if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
            return Some(ThisSpellCostCondition::CardInYourGraveyardMatching {
                filter,
                display: w.join(" "),
            });
        }
    }

    if TARGETS_BIG_CONTROLLED_CREATURE_STACK_OBJECT_PATTERN.matches(clause) {
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
    if OPPONENT_HAS_NO_CARDS_IN_HAND_PATTERN.matches(clause) {
        return Some(ThisSpellCostCondition::OpponentHasNoCardsInHand);
    }

    // an opponent controls seven or more lands
    if w.len() >= 7 && OPPONENT_CONTROLS_PREFIX_PATTERN.matches(clause) {
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, 3) {
            if clause
                .after_words(rest_start)
                .is_some_and(|tail| LANDS_TAIL_PATTERN.matches(tail))
            {
                return Some(ThisSpellCostCondition::OpponentControlsLandsOrMore(n));
            }
        }
    }

    // an opponent controls at least four more creatures than you
    if w.len() >= 10 && OPPONENT_CONTROLS_PREFIX_PATTERN.matches(clause) {
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, 3) {
            if clause
                .after_words(rest_start)
                .is_some_and(|tail| MORE_CREATURES_THAN_YOU_TAIL_PATTERN.matches(tail))
            {
                return Some(
                    ThisSpellCostCondition::OpponentControlsAtLeastNMoreCreaturesThanYou(n),
                );
            }
        }
    }

    // there are ten or more creature cards total in all graveyards
    if w.len() >= 12 && THERE_ARE_PREFIX_PATTERN.matches(clause) {
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, 2) {
            if clause.after_words(rest_start).is_some_and(|tail| {
                TOTAL_CREATURE_CARDS_IN_ALL_GRAVEYARDS_TAIL_PATTERN.matches(tail)
            }) {
                return Some(ThisSpellCostCondition::TotalCreatureCardsInAllGraveyardsOrMore(n));
            }
        }
    }

    // an opponent cast two or more spells this turn
    if w.len() >= 9 && OPPONENT_CAST_PREFIX_PATTERN.matches(clause) {
        let count_start = count_start_for_optional_an_opponent_prefix(&w, 3)?;
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, count_start) {
            if clause
                .after_words(rest_start)
                .is_some_and(|tail| SPELLS_THIS_TURN_TAIL_PATTERN.matches(tail))
            {
                return Some(ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(n));
            }
        }
    }

    // an opponent has drawn four or more cards this turn
    if w.len() >= 10 && OPPONENT_HAS_DRAWN_PREFIX_PATTERN.matches(clause) {
        let count_start = count_start_for_optional_an_opponent_prefix(&w, 4)?;
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, count_start) {
            if clause
                .after_words(rest_start)
                .is_some_and(|tail| CARDS_THIS_TURN_TAIL_PATTERN.matches(tail))
            {
                return Some(ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(n));
            }
        }
    }

    // you've been dealt damage by two or more creatures this turn
    if YOU_WERE_DEALT_DAMAGE_BY_PREFIX_PATTERN.matches(clause) && w.len() >= 11 {
        let count_start = if YOU_HAVE_PREFIX_PATTERN.matches(clause) {
            6
        } else {
            5
        };
        if let Some((n, rest_start)) = parse_static_at_least_quantity_at(tokens, count_start) {
            if clause
                .after_words(rest_start)
                .is_some_and(|tail| CREATURES_THIS_TURN_TAIL_PATTERN.matches(tail))
            {
                return Some(
                    ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(n),
                );
            }
        }
    }

    if ASSASSIN_OR_COMMANDER_COMBAT_DAMAGE_PATTERN.matches(clause) {
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
        .filter_map(|(idx, word)| (*word == "and").then_some(idx))
        .collect::<Vec<_>>();
    for and_word_idx in and_positions {
        let and_token_idx = static_keyword_shapes::parse_word_token_offset(tokens, and_word_idx)?;
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
    let Some(if_idx) =
        static_keyword_cost_shapes::parse_trailing_cost_condition_if(&remaining_words)
            .map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    let condition_token_idx =
        static_keyword_shapes::parse_word_token_offset(remaining_tokens, if_idx + 1).ok_or_else(
            || {
                CardTextError::ParseError(format!(
                    "unable to map this-spell cost condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            },
        )?;
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
        words.iter().any(|word| *word == "less"),
        words.iter().any(|word| *word == "more"),
    ) {
        (true, false) => Some(CostModifierDirection::Less),
        (false, true) => Some(CostModifierDirection::More),
        _ => None,
    }
}

fn parse_cost_modifier_target_spec(
    target_tokens: &[OwnedLexToken],
) -> Result<(Option<PlayerFilter>, Option<Box<ObjectFilter>>), CardTextError> {
    let target_clause = LexedClause::new(target_tokens);
    if YOU_TARGET_PREFIX_PATTERN.matches(target_clause) {
        return Ok((Some(PlayerFilter::You), None));
    }
    if OPPONENT_OR_OPPONENTS_TARGET_PREFIX_PATTERN.matches(target_clause) {
        return Ok((Some(PlayerFilter::Opponent), None));
    }
    if PLAYER_OR_PLAYERS_TARGET_PREFIX_PATTERN.matches(target_clause) {
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
    if let Some(prefix) =
        keyword_static_lines::parse_cost_prefix_condition_tokens(tokens, spells_token_idx)
    {
        match prefix {
            keyword_static_lines::CostPrefixCondition::DuringTurnsOtherThanYours {
                subject_start,
            } => {
                return Ok((
                    Some(crate::ConditionExpr::Not(Box::new(
                        crate::ConditionExpr::YourTurn,
                    ))),
                    subject_start,
                ));
            }
            keyword_static_lines::CostPrefixCondition::DuringYourTurn { subject_start } => {
                return Ok((Some(crate::ConditionExpr::YourTurn), subject_start));
            }
            keyword_static_lines::CostPrefixCondition::AsLongAs {
                condition_tokens,
                subject_start,
            } => {
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after leading 'as long as' clause (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )));
                }
                let condition = match parse_static_condition_clause(&condition_tokens) {
                    Ok(condition) => condition,
                    Err(_) => parse_source_tap_status_condition_lexed(&condition_tokens)
                        .ok_or_else(|| {
                            CardTextError::ParseError(format!(
                                "unsupported static condition clause (clause: '{}')",
                                crate::runtime_backend::token_word_refs(&condition_tokens)
                                    .join(" ")
                            ))
                        })?,
                };
                return Ok((Some(condition), subject_start));
            }
        }
    }

    Ok((None, 0))
}

fn parse_optional_life_additional_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let additional_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(spec) = static_keyword_cost_shapes::parse_additional_cost_spell_filter(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.spell_filter_tokens);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_spell_filter_with_grammar_entrypoint(&subject_tokens);
    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    if static_keyword_cost_shapes::parse_optional_life_subject_is_permanent(&subject_words) {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }
    filter.cast_by = Some(PlayerFilter::You);

    let Some(optional_life_shape) =
        static_keyword_cost_shapes::parse_optional_life_reduction_words(&additional_words)
    else {
        return Ok(None);
    };
    let pay_word_idx = optional_life_shape.pay.word;
    let payment_words = &additional_words[pay_word_idx + 1..];
    let Some(life_cost) = payment_words
        .first()
        .and_then(|word| parse_number_word_i32(word))
        .and_then(|amount| u32::try_from(amount).ok())
    else {
        return Ok(None);
    };
    if !optional_life_shape.payment_has_life {
        return Ok(None);
    }

    if !optional_life_shape.those_spells_paid_life_this_way {
        return Ok(None);
    }
    let costs_word_idx = optional_life_shape.costs.word;
    let Some(costs_idx) = static_keyword_shapes::parse_word_token_offset(tokens, costs_word_idx)
    else {
        return Ok(None);
    };
    let amount_tokens = &tokens[costs_idx + 1..];
    let (_, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let Some((reduction, _)) = parsed_mana_cost else {
        return Ok(None);
    };
    let remaining_words = crate::runtime_backend::token_word_refs(amount_tokens);
    if parse_cost_modifier_direction(&remaining_words) != Some(CostModifierDirection::Less)
        || !static_keyword_cost_shapes::parse_cost_modifier_cast_marker(&remaining_words)
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
    let clause = LexedClause::new(tokens);

    let Some(spells_token_idx) =
        static_keyword_cost_shapes::parse_spells_subject(tokens).map(|boundary| boundary.token)
    else {
        return Ok(None);
    };

    if FIRST_SPELL_EACH_TURN_COST_MODIFIER_PATTERN.matches(clause) {
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
        if !tokens[idx]
            .as_word()
            .is_some_and(|word| matches!(word, "cost" | "costs"))
        {
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
    let between_clause = LexedClause::new(between_tokens);
    if !is_this_spell {
        for (idx, token) in between_tokens.iter().enumerate() {
            if !token
                .as_word()
                .is_some_and(|word| matches!(word, "spell" | "spells"))
            {
                continue;
            }
            let mut start = idx;
            while start > 0 {
                if between_tokens[start - 1]
                    .as_word()
                    .is_some_and(|word| matches!(word, "and" | "or"))
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
        if YOU_CAST_PHRASE_PATTERN.matches(between_clause) {
            filter.cast_by = Some(PlayerFilter::You);
        }
        if FROM_YOUR_GRAVEYARD_PHRASE_PATTERN.matches(between_clause) {
            filter.zone = Some(Zone::Graveyard);
            filter.owner = Some(PlayerFilter::You);
        }
        if OPPONENT_WORD_MARKER_PATTERN.matches(between_clause)
            && CAST_OR_CASTS_WORD_MARKER_PATTERN.matches(between_clause)
        {
            filter.cast_by = Some(PlayerFilter::Opponent);
        }
        let mut targets_idx = None;
        for (idx, token) in between_tokens.iter().enumerate() {
            if token
                .as_word()
                .is_some_and(|word| matches!(word, "target" | "targets"))
            {
                if idx > 0
                    && between_tokens[idx - 1]
                        .as_word()
                        .is_some_and(|word| word == "that")
                {
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
    let direction_words = if let Some(if_idx) =
        static_keyword_cost_shapes::parse_cost_direction_if_boundary(&remaining_words)
            .map(|boundary| boundary.word)
    {
        &remaining_words[..if_idx]
    } else {
        &remaining_words
    };
    let Some(direction) = parse_cost_modifier_direction(direction_words) else {
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
    if WHERE_X_IS_MARKER_PATTERN.matches(LexedClause::new(remaining_tokens)) {
        let clause = clause_words.join(" ");
        let where_word_idx = keyword_find_exact_clause_window(
            LexedClause::new(remaining_tokens),
            3,
            WHERE_X_IS_PREFIX_PATTERN,
        )
        .unwrap_or(0);
        let where_token_idx =
            static_keyword_shapes::parse_word_token_offset(remaining_tokens, where_word_idx)
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
    let Some(and_idx) = static_keyword_cost_shapes::parse_spell_and_abilities_separator(tokens)
        .map(|boundary| boundary.token)
    else {
        return Ok(None);
    };
    let right_start = and_idx + 1;

    let left_tokens = trim_commas(&tokens[..and_idx]);
    let right_tokens = trim_commas(&tokens[right_start..]);
    let Some(spell_cost_ability) = parse_spells_cost_modifier_line(&left_tokens)? else {
        return Ok(None);
    };
    let Some(mut activated_cost_ability) =
        parse_player_activated_ability_cost_modifier_clause(&right_tokens)?
    else {
        return Ok(None);
    };

    if let Some(spells_idx) =
        static_keyword_cost_shapes::parse_spells_subject(tokens).map(|boundary| boundary.token)
    {
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
    let clause = LexedClause::new(tokens);
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 8 {
        return Ok(None);
    }

    let (condition, body_start_word_idx) = if AS_LONG_AS_PREFIX_PATTERN.matches(clause) {
        let Some(body_word_idx) =
            keyword_find_prefix_shape_start(clause, &YOU_MAY_PAY_PREFIX_PATTERN)
                .filter(|idx| *idx >= 3)
        else {
            return Ok(None);
        };
        let condition_start = static_keyword_shapes::parse_word_token_offset(tokens, 3)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map cycling-cost alternative condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let condition_end = static_keyword_shapes::parse_word_token_offset(tokens, body_word_idx)
            .ok_or_else(|| {
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

    if !clause
        .after_words(body_start_word_idx)
        .is_some_and(|body_clause| YOU_MAY_PAY_PREFIX_PATTERN.matches(body_clause))
    {
        return Ok(None);
    }
    let Some(body_clause) = clause.after_words(body_start_word_idx) else {
        return Ok(None);
    };
    let Some(rather_rel_idx) =
        keyword_find_prefix_shape_start(body_clause, &RATHER_THAN_PAY_CYCLING_COSTS_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    let rather_word_idx = body_start_word_idx + rather_rel_idx;
    let pay_token_idx =
        static_keyword_shapes::parse_word_token_offset(tokens, body_start_word_idx + 2)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map cycling-cost alternative cost (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
    let cost_start = pay_token_idx + 1;
    let cost_end = static_keyword_shapes::parse_word_token_offset(tokens, rather_word_idx)
        .ok_or_else(|| {
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
            .is_some_and(|word| *word == "abilities")
    {
        return Ok(None);
    }

    let Some(cost_words) =
        static_keyword_cost_shapes::parse_player_ability_cost_words(&clause_words)
    else {
        return Ok(None);
    };
    let activate_idx = cost_words.activate.word;
    let clause = LexedClause::new(tokens);
    let Some(activator_clause) = clause.between_word_range(1, activate_idx) else {
        return Ok(None);
    };
    let activator = if YOU_SUBJECT_PATTERN.matches(activator_clause) {
        PlayerFilter::You
    } else if YOUR_OPPONENTS_ACTIVATOR_PATTERN.matches(activator_clause) {
        PlayerFilter::Opponent
    } else {
        return Ok(None);
    };

    let cost_idx = cost_words.costs.word;
    let cost_token_idx = static_keyword_shapes::parse_word_token_offset(tokens, cost_idx)
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
    let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
    let remaining_clause = LexedClause::new(remaining_tokens);
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    if parse_cost_modifier_direction(&remaining_words) != Some(CostModifierDirection::More)
        || !TO_ACTIVATE_PHRASE_PATTERN.matches(remaining_clause)
    {
        return Ok(None);
    }

    let non_mana_only = UNLESS_THEYRE_MANA_ABILITIES_PATTERN.matches(remaining_clause);
    Ok(Some(
        StaticAbility::increase_activated_ability_costs_for_activator(
            activator,
            increase,
            non_mana_only,
        ),
    ))
}

fn strip_relative_target_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let Some(target_clause_idx) = static_keyword_cost_shapes::parse_relative_target_clause(tokens)
        .map(|boundary| boundary.token)
    else {
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
    let Some(if_word_idx) =
        static_keyword_cost_shapes::parse_trailing_target_condition_if(&remaining_words)
            .map(|boundary| boundary.word)
    else {
        return Ok(());
    };
    let condition_words = &remaining_words[if_word_idx..];
    let Some(condition_clause) =
        LexedClause::new(remaining_tokens).between_word_range(if_word_idx, remaining_words.len())
    else {
        return Ok(());
    };
    if condition_words.len() < 4
        || !IF_IT_TARGET_OR_TARGETS_PREFIX_PATTERN.matches(condition_clause)
    {
        return Ok(());
    }

    let target_word_idx = if_word_idx + 3;
    let target_token_idx = static_keyword_shapes::parse_word_token_offset(
        remaining_tokens,
        target_word_idx,
    )
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
    let clause = LexedClause::new(tokens);
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
    let Some(cost_idx) =
        static_keyword_cost_shapes::parse_last_cost_verb(tokens).map(|boundary| boundary.token)
    else {
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
    if YOU_PAY_PHRASE_PATTERN.matches(clause) {
        filter.cast_by = Some(PlayerFilter::You);
    } else if OPPONENTS_PAY_PHRASE_PATTERN.matches(clause) {
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
    let Some(head) = keyword_static_lines::parse_equip_cost_modifier_head_tokens(tokens) else {
        return Ok(None);
    };
    let cost_idx = head.cost_token;

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
    match head.payer {
        keyword_static_lines::EquipCostPayer::You => filter.controller = Some(PlayerFilter::You),
        keyword_static_lines::EquipCostPayer::Opponent => {
            filter.controller = Some(PlayerFilter::Opponent)
        }
        keyword_static_lines::EquipCostPayer::Unspecified => {}
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
    let clause = LexedClause::new(tokens);
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 7 {
        return Ok(None);
    }
    if !FORETELLING_CARDS_FROM_HAND_COSTS_PREFIX_PATTERN.matches(clause) {
        return Ok(None);
    }

    let has_any_players_turn = ANY_PLAYER_TURN_PATTERN.matches(clause);
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
    let parsed = parse_leaf_fixed_mana_cost_prefix_tokens(tokens)?;
    Some((parsed.cost, parsed.consumed))
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
        if !tokens[idx - 2].is_word("by")
            || !tokens[idx - 1].is_word("more")
            || !tokens[idx].is_word("than")
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
    use keyword_static_lines::{
        CounterReferenceKind, DynamicCostValueShape, DynamicPlayerKind, DynamicThisWayMetric,
        SpellCastDynamicKind,
    };

    let Some(shape) = keyword_static_lines::parse_dynamic_cost_value_shape_tokens(tokens) else {
        return Ok(None);
    };
    let player_filter = |player| match player {
        DynamicPlayerKind::You => PlayerFilter::You,
        DynamicPlayerKind::Opponent => PlayerFilter::Opponent,
        DynamicPlayerKind::Any => PlayerFilter::Any,
    };
    let value = match shape {
        DynamicCostValueShape::CardsDrawn(player) => {
            Value::MaxCardsDrawnThisTurn(player_filter(player))
        }
        DynamicCostValueShape::KickCount => Value::KickCount,
        DynamicCostValueShape::CreaturesDiedThisTurn => Value::CreaturesDiedThisTurn,
        DynamicCostValueShape::OpponentsLifeLostThisTurn => {
            Value::LifeLostThisTurn(PlayerFilter::Opponent)
        }
        DynamicCostValueShape::ControlledCreaturesDiedThisTurn => {
            Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You)
        }
        DynamicCostValueShape::SpellCast { player, kind } => {
            let player = player_filter(player);
            match kind {
                SpellCastDynamicKind::CardTypes => {
                    let mut filter = ObjectFilter::spell();
                    filter.cast_by = Some(player);
                    Value::CardTypesAmong(filter)
                }
                SpellCastDynamicKind::OtherThanFirst => Value::Add(
                    Box::new(Value::SpellsCastThisTurn(player)),
                    Box::new(Value::Fixed(-1)),
                ),
                SpellCastDynamicKind::MatchingTypes {
                    instant,
                    sorcery,
                    exclude_source,
                } => {
                    let mut filter = ObjectFilter::spell();
                    filter.card_types = match (instant, sorcery) {
                        (true, true) => vec![CardType::Instant, CardType::Sorcery],
                        (true, false) => vec![CardType::Instant],
                        (false, true) => vec![CardType::Sorcery],
                        (false, false) => Vec::new(),
                    };
                    Value::SpellsCastThisTurnMatching {
                        player,
                        filter,
                        exclude_source,
                    }
                }
                SpellCastDynamicKind::Simple => Value::SpellsCastThisTurn(player),
            }
        }
        DynamicCostValueShape::CardTypesInGraveyard(player) => {
            Value::CardTypesInGraveyard(player_filter(player))
        }
        DynamicCostValueShape::ColorsSpentToCastThisSpell => {
            Value::ColorsOfManaSpentToCastThisSpell
        }
        DynamicCostValueShape::PartySize => Value::PartySize(PlayerFilter::You),
        DynamicCostValueShape::AggregateScope => {
            let each_idx = static_keyword_cost_shapes::parse_dynamic_cost_each_word(tokens)
                .map(|boundary| boundary.token)
                .unwrap_or(0);
            let filter_tokens = tokens.get(each_idx + 1..).unwrap_or_default();
            let Some(value) = parse_aggregate_scope_value_lexed(filter_tokens) else {
                return Ok(None);
            };
            value
        }
        DynamicCostValueShape::CardTypesAmong { scope_tokens } => {
            let Ok(filter) = parse_object_filter(scope_tokens, false) else {
                return Ok(None);
            };
            Value::CardTypesAmong(filter)
        }
        DynamicCostValueShape::UnsupportedCardTypesAmong => {
            return Err(CardTextError::ParseError(format!(
                "unsupported card-types-among dynamic value (clause: '{}')",
                parser_token_word_refs(tokens).join(" ")
            )));
        }
        DynamicCostValueShape::CountersRemovedThisWay => Value::X,
        DynamicCostValueShape::PlayerCounters(counter_type) => {
            Value::PlayerCounters(PlayerFilter::You, counter_type)
        }
        DynamicCostValueShape::ThisWayMetric(metric) => match metric {
            DynamicThisWayMetric::Destroyed | DynamicThisWayMetric::Sacrificed => {
                Value::PendingEffectMetric {
                    source: EffectMetricSource::AffectedObjects,
                    metric: EffectMetric::Count,
                }
            }
            DynamicThisWayMetric::Discarded => Value::PendingEffectMetric {
                source: EffectMetricSource::Outcome,
                metric: EffectMetric::Count,
            },
            DynamicThisWayMetric::Exiled => Value::Count(
                ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile),
            ),
        },
        DynamicCostValueShape::RevealedPublic => {
            Value::Count(ObjectFilter::tagged(TagKey::from("__public_revealed")))
        }
        DynamicCostValueShape::RevealedOther => {
            let words = parser_token_word_refs(tokens);
            let Some((value, used_words)) = parse_for_each_count_value_words(&words) else {
                return Ok(None);
            };
            if used_words != words.len() {
                return Ok(None);
            }
            value
        }
        DynamicCostValueShape::CounterReference(reference) => {
            let counter_type = reference.counter_type;
            match reference.reference_kind {
                CounterReferenceKind::Source => match counter_type {
                    Some(counter_type) => Value::CountersOnSource(counter_type),
                    None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
                },
                CounterReferenceKind::Tagged => Value::CountersOn(
                    Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                    counter_type,
                ),
                CounterReferenceKind::Other => {
                    let words = parser_token_word_refs(reference.reference_tokens);
                    let Some(surface) = source_reference_surface_for_words(&words) else {
                        return Ok(None);
                    };
                    Value::CountersOn(
                        Box::new(source_choose_spec_for_surface(surface)),
                        counter_type,
                    )
                }
            }
        }
        DynamicCostValueShape::UnsupportedThisWay => {
            return Err(CardTextError::ParseError(format!(
                "unsupported this-way dynamic value (clause: '{}')",
                parser_token_word_refs(tokens).join(" ")
            )));
        }
        DynamicCostValueShape::Other { filter_tokens } => {
            if let Some(player) = parse_commander_cast_count_player(filter_tokens) {
                Value::CommanderCastCount(player)
            } else if let Ok(filter) = parse_object_filter(filter_tokens, false) {
                Value::Count(filter)
            } else {
                return Ok(None);
            }
        }
    };
    Ok(Some(value))
}

pub(crate) fn parse_add_mana_that_much_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    if keyword_static_lines::parse_that_much_value_marker_tokens(tokens) {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }
    None
}

pub(crate) fn parse_players_skip_upkeep_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let tokens =
        super::grammar::effects::split_labeled_effect_prefix_lexed(&tokens).unwrap_or(&tokens);
    let clause = LexedClause::new(tokens);
    if SKIP_YOUR_UPKEEP_STEP_PATTERN.matches(clause) {
        let mut ability = StaticAbility::player_skips_upkeep(crate::target::PlayerFilter::You);
        let words = clause.words();
        if words.len() > 4 {
            if words.get(4) != Some("if") || words.len() <= 5 {
                return Err(CardTextError::ParseError(format!(
                    "unsupported skip-upkeep tail (clause: '{}')",
                    render_token_slice(tokens)
                )));
            }
            let condition_idx = words.token_boundary_for_word_or_end(5).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported skip-upkeep tail (clause: '{}')",
                    render_token_slice(tokens)
                ))
            })?;
            let condition = parse_static_condition_clause(&tokens[condition_idx..])?;
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
    if keyword_static_lines::parse_legend_rule_doesnt_apply_tokens(tokens) {
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
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(be_idx) = words.find_window_by(1, |window| {
        window
            .first()
            .copied()
            .is_some_and(|word| matches!(word, "is" | "are"))
    }) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 2 >= words.len() {
        return Ok(None);
    }

    let Some(addition_idx) =
        keyword_find_exact_clause_window(clause, 5, IN_ADDITION_TO_OTHER_PATTERN)
    else {
        return Ok(None);
    };
    if addition_idx <= be_idx + 1 {
        return Ok(None);
    }

    if !words
        .get(addition_idx + 5)
        .is_some_and(|word| matches!(word, "type" | "types"))
    {
        return Ok(None);
    }

    let Some(subject_tokens) = clause
        .between_word_range(0, be_idx)
        .map(|clause| clause.tokens())
    else {
        return Ok(None);
    };
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let Some(added_clause) = clause.between_word_range(be_idx + 1, addition_idx) else {
        return Ok(None);
    };
    if CHOSEN_TYPE_PATTERN.matches(added_clause) {
        let filter = parse_object_filter_lexed(subject_tokens, false)?;
        if filter
            .card_types
            .iter()
            .any(|card_type| *card_type == CardType::Land)
        {
            return Ok(Some(vec![StaticAbility::add_chosen_basic_land_type(
                filter,
                render_token_slice(tokens),
            )]));
        }
        return Ok(Some(vec![StaticAbility::add_chosen_creature_type(
            filter,
            render_token_slice(tokens),
        )]));
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in added_clause.tokens() {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if matches!(descriptor, "a" | "an" | "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }

        let Some(subtype) = parse_subtype_flexible(descriptor) else {
            return Ok(None);
        };
        crate::slice_primitives::push_unique(&mut subtypes, subtype);
    }
    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter_lexed(subject_tokens, false)?;

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
    if keyword_static_lines::parse_all_cards_spells_permanents_colorless_tokens(tokens) {
        return Ok(Some(StaticAbility::make_colorless(ObjectFilter::default())));
    }
    Ok(None)
}

pub(crate) fn parse_all_cards_spells_permanents_add_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if ALL_CARDS_SPELLS_PERMANENTS_CHOSEN_COLOR_PATTERN.matches(LexedClause::new(tokens)) {
        return Ok(Some(StaticAbility::add_chosen_color(
            ObjectFilter::default(),
            render_token_slice(tokens),
        )));
    }

    Ok(None)
}

fn parse_conjoined_subject_filter(tokens: &[OwnedLexToken]) -> Result<ObjectFilter, CardTextError> {
    let subject_tokens = trim_lexed_commas(tokens);
    let subject_segments = split_lexed_slices_on_and(subject_tokens);
    if subject_segments.len() <= 1 {
        return parse_object_filter_lexed(subject_tokens, false);
    }

    let mut branches = Vec::with_capacity(subject_segments.len());
    for segment in subject_segments {
        let segment = trim_lexed_commas(segment);
        if segment.is_empty() {
            return parse_object_filter_lexed(subject_tokens, false);
        }
        branches.push(parse_object_filter_lexed(segment, false)?);
    }
    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    Ok(filter)
}

pub(crate) fn parse_all_are_pt_color_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    if words.len() < 10 {
        return Ok(None);
    }

    let Some(be_idx) = words.find_window_by(1, |window| {
        window
            .first()
            .copied()
            .is_some_and(|word| matches!(word, "is" | "are"))
    }) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 1 >= words.len() {
        return Ok(None);
    }

    let Some(pt_word) = words.get(be_idx + 1) else {
        return Ok(None);
    };
    let (power, toughness) = match parse_pt_modifier(pt_word) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };

    let Some(addition_idx) =
        keyword_find_exact_clause_window(clause, 5, IN_ADDITION_TO_THEIR_OTHER_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    if addition_idx <= be_idx + 2 {
        return Ok(None);
    }
    if !words
        .get(addition_idx + 5)
        .is_some_and(|word| matches!(word, "type" | "types"))
    {
        return Ok(None);
    }

    let Some(descriptor_clause) = clause.between_word_range(be_idx + 2, addition_idx) else {
        return Ok(None);
    };
    let mut colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in descriptor_clause.tokens() {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if is_article(descriptor) || matches!(descriptor, "and" | "or" | "and/or") {
            continue;
        }
        if let Some(color) = parse_color(descriptor) {
            colors = colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in pt-color-type-addition clause (clause: '{}')",
            descriptor,
            render_token_slice(tokens)
        )));
    }

    if colors.is_empty() && card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let Some(subject_tokens) = clause
        .between_word_range(0, be_idx)
        .map(|clause| clause.tokens())
    else {
        return Ok(None);
    };
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_conjoined_subject_filter(subject_tokens)?;

    let mut abilities = Vec::new();
    if !colors.is_empty() {
        abilities.push(StaticAbility::set_colors(filter.clone(), colors));
    }
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter.clone(), subtypes));
    }
    abilities.push(StaticAbility::set_base_power_toughness(
        filter, power, toughness,
    ));
    Ok(Some(abilities))
}

pub(crate) fn parse_all_are_color_and_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    if words.len() < 10 {
        return Ok(None);
    }
    let Some(are_idx) = words.find_window_by(1, |window| {
        window.first().copied().is_some_and(|word| word == "are")
    }) else {
        return Ok(None);
    };
    if are_idx == 0 || are_idx + 4 >= words.len() {
        return Ok(None);
    }

    let Some(base_color) = words.get(are_idx + 1).and_then(|word| parse_color(word)) else {
        return Ok(None);
    };

    // Pattern: "<subject> are <color> and are <subtype>... in addition to their other creature types"
    let Some(and_are_token_idx) =
        static_keyword_shapes::parse_word_token_offset(tokens, are_idx + 2)
    else {
        return Ok(None);
    };
    if !AND_ARE_PREFIX_PATTERN.matches(LexedClause::new(&tokens[and_are_token_idx..])) {
        return Ok(None);
    }

    let descriptor_start = are_idx + 4;
    let Some(addition_idx) =
        keyword_find_exact_clause_window(clause, 5, IN_ADDITION_TO_THEIR_OTHER_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    if addition_idx <= descriptor_start {
        return Ok(None);
    }

    let Some(scope_clause) = clause.between_word_range(addition_idx + 5, words.len()) else {
        return Ok(None);
    };
    if !CREATURE_TYPE_SCOPE_PATTERN.matches(scope_clause) {
        return Ok(None);
    }

    let Some(descriptor_clause) = clause.between_word_range(descriptor_start, addition_idx) else {
        return Ok(None);
    };
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in descriptor_clause.tokens() {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if matches!(descriptor, "a" | "an" | "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in are-color-and-type-addition clause (clause: '{}')",
            descriptor,
            render_token_slice(tokens)
        )));
    }

    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let Some(subject_tokens) = clause
        .between_word_range(0, are_idx)
        .map(|clause| clause.tokens())
    else {
        return Ok(None);
    };
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter_lexed(subject_tokens, false)?;

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
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    if words.len() < 4 {
        return Ok(None);
    }
    let Some(are_idx) = words.find_window_by(1, |window| {
        window
            .first()
            .copied()
            .is_some_and(|word| matches!(word, "is" | "are"))
    }) else {
        return Ok(None);
    };
    if are_idx == 0 {
        return Ok(None);
    }

    let Some(color_clause) = clause.between_word_range(are_idx + 1, words.len()) else {
        return Ok(None);
    };
    let mut color_words = color_clause
        .tokens()
        .iter()
        .filter_map(OwnedLexToken::as_word);
    let color = match (color_words.next(), color_words.next(), color_words.next()) {
        (Some("all"), Some("colors"), None) => {
            crate::color::Color::ALL.into_iter().collect::<ColorSet>()
        }
        (Some(color_word), None, None) => {
            let Some(color) = parse_color(color_word) else {
                return Ok(None);
            };
            color
        }
        _ => return Ok(None),
    };

    let Some(subject_tokens) = clause
        .between_word_range(0, are_idx)
        .map(|clause| clause.tokens())
    else {
        return Ok(None);
    };
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter_lexed(subject_tokens, false)?;

    Ok(Some(StaticAbility::set_colors(filter, color)))
}

pub(crate) fn parse_subjects_are_basic_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    let Some(be_idx) = words.find_window_by(1, |window| {
        window
            .first()
            .copied()
            .is_some_and(|word| matches!(word, "is" | "are"))
    }) else {
        return Ok(None);
    };
    if be_idx == 0 {
        return Ok(None);
    }

    let Some(tail_clause) = clause.between_word_range(be_idx + 1, words.len()) else {
        return Ok(None);
    };
    if !BASIC_TAIL_PATTERN.matches(tail_clause) {
        return Ok(None);
    }

    let Some(subject_tokens) = clause
        .between_word_range(0, be_idx)
        .map(|clause| trim_lexed_commas(clause.tokens()))
    else {
        return Ok(None);
    };
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
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    let Some(be_idx) = words.find_window_by(1, |window| {
        window
            .first()
            .copied()
            .is_some_and(|word| matches!(word, "is" | "are"))
    }) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 1 >= words.len() {
        return Ok(None);
    }

    let mut subtype_idx = be_idx + 1;
    if words.get(subtype_idx).is_some_and(|word| is_article(word)) {
        subtype_idx += 1;
    }
    let Some(subtype_clause) = clause.between_word_range(subtype_idx, words.len()) else {
        return Ok(None);
    };
    let mut subtype_words = subtype_clause
        .tokens()
        .iter()
        .filter_map(OwnedLexToken::as_word);
    let Some(subtype_word) = subtype_words.next() else {
        return Ok(None);
    };
    if subtype_words.next().is_some() {
        return Ok(None);
    }

    let Some(subtype) = parse_subtype_flexible(subtype_word) else {
        return Ok(None);
    };
    if !matches!(
        subtype,
        Subtype::Plains | Subtype::Island | Subtype::Swamp | Subtype::Mountain | Subtype::Forest
    ) {
        return Ok(None);
    }

    let Some(subject_clause) = clause.between_word_range(0, be_idx) else {
        return Ok(None);
    };
    let subject_tokens = subject_clause.tokens();
    let filter = parse_object_filter_lexed(subject_tokens, false)?;
    if !subject_clause.words().contains_window_by(1, |window| {
        window
            .first()
            .copied()
            .is_some_and(|word| matches!(word, "land" | "lands"))
    }) {
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
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    if words.len() < 10 {
        return Ok(None);
    }

    let Some(be_idx) = words.find_window_by(1, |window| {
        window
            .first()
            .copied()
            .is_some_and(|word| matches!(word, "is" | "are"))
    }) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 1 >= words.len() {
        return Ok(None);
    }

    let Some(after_be_clause) = clause.between_word_range(be_idx + 1, words.len()) else {
        return Ok(None);
    };
    if EVERY_BASIC_LAND_TYPE_ADDITION_TAIL_PATTERN.matches(after_be_clause) {
        let Some(filter_tokens) = clause
            .between_word_range(0, be_idx)
            .map(|clause| clause.tokens())
        else {
            return Ok(None);
        };
        if filter_tokens.is_empty() {
            return Ok(None);
        }
        let filter = parse_object_filter_lexed(filter_tokens, false)?;
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
    let Some(subtype_word) = words.get(subtype_word_idx) else {
        return Ok(None);
    };
    let Some(subtype) = parse_subtype_flexible(subtype_word) else {
        return Ok(None);
    };
    if !is_land_subtype(subtype) {
        return Ok(None);
    }

    let Some(tail_clause) = clause.between_word_range(subtype_word_idx + 1, words.len()) else {
        return Ok(None);
    };
    if !LAND_TYPE_ADDITION_TAIL_PATTERN.matches(tail_clause) {
        return Ok(None);
    }

    let Some(filter_tokens) = clause
        .between_word_range(0, be_idx)
        .map(|clause| clause.tokens())
    else {
        return Ok(None);
    };
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter_lexed(filter_tokens, false)?;

    Ok(Some(StaticAbility::add_subtypes(filter, vec![subtype])))
}

pub(crate) fn parse_lands_are_pt_creatures_still_lands_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(be_idx) = words.find_window_by(1, |window| {
        window
            .first()
            .copied()
            .is_some_and(|word| matches!(word, "is" | "are"))
    }) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 2 >= words.len() {
        return Ok(None);
    }
    let Some(pt_word) = words.get(be_idx + 1) else {
        return Ok(None);
    };
    let (power, toughness) = match parse_pt_modifier(pt_word) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };

    let Some(creature_word) = words.get(be_idx + 2) else {
        return Ok(None);
    };
    if !matches!(creature_word, "creature" | "creatures") {
        return Ok(None);
    }

    let Some(tail_clause) = clause.between_word_range(be_idx + 3, words.len()) else {
        return Ok(None);
    };
    if !STILL_LAND_ANIMATION_TAIL_PATTERN.matches(tail_clause) {
        return Ok(None);
    }

    let Some(filter_tokens) = clause
        .between_word_range(0, be_idx)
        .map(|clause| clause.tokens())
    else {
        return Ok(None);
    };
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter_lexed(filter_tokens, false)?;

    Ok(Some(vec![
        StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature]),
        StaticAbility::set_base_power_toughness(filter, power, toughness),
    ]))
}

fn parse_static_base_power_toughness_value_tail(
    tail_tokens: &[OwnedLexToken],
) -> Option<(Value, Value)> {
    if !keyword_static_lines::parse_iterated_mana_value_base_pt_tail_tokens(tail_tokens) {
        return None;
    }
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    Some((value.clone(), value))
}

pub(crate) fn parse_filter_is_pt_creature_in_addition_and_has_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = LexedClause::new(tokens).word_refs();
    let Some(animation_verbs) = static_keyword_line_shapes::parse_animation_verbs(tokens) else {
        return Ok(None);
    };
    let be_idx = animation_verbs.be.token;
    let has_idx = animation_verbs.has.token;

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
    let attached_subject = LexedClause::new(&subject_tokens)
        .words()
        .first()
        .is_some_and(|word| matches!(word, "enchanted" | "equipped"));

    let before_has = trim_commas(&tokens[be_idx + 1..has_idx]);
    if before_has.is_empty() {
        return Ok(None);
    }
    let before_has_clause = LexedClause::new(&before_has);
    let raw_before_has_words = before_has_clause.word_refs();
    let before_has_words = strip_leading_article_word_refs(&raw_before_has_words);
    let skipped_article_words = raw_before_has_words
        .len()
        .saturating_sub(before_has_words.len());
    let Some(creature_idx) =
        static_keyword_line_shapes::parse_animation_creature_word(&before_has_words)
            .map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    let (base_power_toughness, subtype_start_word, granted_tail) = match before_has_words
        .first()
        .and_then(|word| parse_pt_modifier(word).ok())
    {
        Some((power, toughness)) => {
            if creature_idx == 0 {
                return Ok(None);
            }
            let Some(granted_tail) = parse_heterogeneous_granted_tail(
                &tokens[has_idx + 1..],
                &clause_words,
                attached_subject,
            )?
            else {
                return Ok(None);
            };
            (
                (Value::Fixed(power), Value::Fixed(toughness)),
                1usize,
                granted_tail,
            )
        }
        None => {
            let Some((power, toughness)) =
                parse_static_base_power_toughness_value_tail(&tokens[has_idx + 1..])
            else {
                return Ok(None);
            };
            ((power, toughness), 0usize, ParsedGrantedTailAst::default())
        }
    };
    let subtype_words = &before_has_words[subtype_start_word..creature_idx];
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
    let tail_start_word = skipped_article_words + creature_idx + 1;
    let mut tail_end_word = skipped_article_words + before_has_words.len();
    let tail_ends_with_and = before_has_words[creature_idx + 1..]
        .last()
        .is_some_and(|word| *word == "and");
    if tail_ends_with_and {
        tail_end_word = tail_end_word.saturating_sub(1);
    }
    if !before_has_clause
        .between_word_range(tail_start_word, tail_end_word)
        .is_some_and(|tail_clause| OTHER_TYPE_ADDITION_TAIL_PATTERN.matches(tail_clause))
    {
        return Ok(None);
    }

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::Add,
            base_power_toughness: Some(base_power_toughness),
            granted_tail,
        },
    )))
}

pub(crate) fn parse_subject_is_subtype_with_base_pt_and_granted_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens =
        if let Some((label_tokens, body_tokens)) = split_em_dash_label_prefix_tokens(tokens) {
            if document_grammar::parse_preserved_keyword_label_tokens(label_tokens).is_some() {
                tokens
            } else {
                body_tokens
            }
        } else {
            tokens
        };
    let Some(grant_verbs) = static_keyword_line_shapes::parse_subtype_grant_verbs(tokens) else {
        return Ok(None);
    };
    let be_idx = grant_verbs.be.token;
    let with_idx = grant_verbs.with.token;

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
    let attached_subject = LexedClause::new(&subject_tokens)
        .words()
        .first()
        .is_some_and(|word| matches!(word, "enchanted" | "equipped"));

    let type_tokens = trim_commas(&tokens[be_idx + 1..with_idx]);
    if type_tokens.is_empty() {
        return Ok(None);
    }
    let type_words = LexedClause::new(&type_tokens).word_refs();
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
        let mut note_start = None;
        let mut idx = 0usize;
        let word_len = LexedClause::new(&after_with).word_len();
        while idx + 6 <= word_len {
            let Some(window_clause) =
                LexedClause::new(&after_with).between_word_range(idx, idx + 6)
            else {
                return Ok(None);
            };
            if LOSES_ALL_OTHER_CREATURE_TYPES_PATTERN.matches(window_clause) {
                note_start = Some(idx);
                break;
            }
            idx += 1;
        }
        if let Some(note_start) = note_start {
            let Some(token_idx) =
                static_keyword_shapes::parse_word_token_offset(&after_with, note_start)
            else {
                return Ok(None);
            };
            after_with.truncate(token_idx);
            true
        } else {
            false
        }
    };

    let after_with = trim_edge_punctuation_tokens(&after_with);
    let after_with_clause = LexedClause::new(after_with);
    let after_with_words = after_with_clause.word_refs();
    if after_with_words.len() < 5 || !BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches(after_with_clause)
    {
        return Ok(None);
    }
    let (power, toughness) = match parse_pt_modifier(after_with_words[4]) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };

    let ability_start_word_idx = if after_with_words.get(5).copied() == Some(",") {
        6
    } else {
        5
    };
    if ability_start_word_idx >= after_with_words.len() {
        return Ok(None);
    }
    let Some(ability_start_idx) =
        static_keyword_shapes::parse_word_token_offset(after_with, ability_start_word_idx)
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
            base_power_toughness: Some((Value::Fixed(power), Value::Fixed(toughness))),
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

fn parse_damage_source_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let words = LexedClause::new(tokens).word_refs();
    parse_damage_source_filter_words(&words)
}

pub(crate) fn parse_prevent_damage_to_you_from_source_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_prevent_damage_to_you_tokens(tokens) else {
        return Ok(None);
    };
    let Some(source_filter) = parse_damage_source_filter_tokens(spec.source_tokens) else {
        return Ok(None);
    };
    let display = render_token_slice(tokens);

    Ok(Some(
        StaticAbility::prevent_damage_to_you_from_source_filter(
            spec.amount,
            source_filter,
            display,
        ),
    ))
}

pub(crate) fn parse_replace_damage_with_counters_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !keyword_static_lines::parse_noncombat_damage_minus_counter_replacement_tokens(tokens) {
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
    let Some(shape) = keyword_static_lines::parse_counter_replacement_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(match shape {
        keyword_static_lines::CounterReplacementShape::GenericUnderYourControl => {
            StaticAbility::double_counters_replacement(
                ObjectFilter::permanent().controlled_by(PlayerFilter::You),
                None,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::EnergyYouGet => {
            StaticAbility::double_player_counters_replacement(
                PlayerFilter::You,
                Some(CounterType::Energy),
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::PlusOneAdd {
            filter_tokens,
            additional,
        } => StaticAbility::add_counters_placement_replacement(
            parse_object_filter_lexed(filter_tokens, false)?,
            Some(CounterType::PlusOnePlusOne),
            additional,
            display_text_for_tokens(tokens, true),
        ),
        keyword_static_lines::CounterReplacementShape::PlusOneDouble { filter_tokens } => {
            StaticAbility::double_counters_replacement(
                parse_object_filter_lexed(filter_tokens, false)?,
                Some(CounterType::PlusOnePlusOne),
                display_text_for_tokens(tokens, true),
            )
        }
    }))
}

pub(crate) fn parse_double_token_creation_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_token_creation_replacement_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(match shape {
        keyword_static_lines::TokenCreationReplacementShape::GenericUnderYourControl => {
            StaticAbility::double_token_creation_replacement(
                PlayerFilter::You,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::TokenCreationReplacementShape::AddTreasure { descriptor_tokens } => {
            let mut token_filter = ObjectFilter::default().token();
            for word in parser_token_word_refs(descriptor_tokens) {
                if let Some(card_type) = parse_card_type(word) {
                    token_filter = token_filter.with_type(card_type);
                } else if let Some(subtype) = parse_subtype_flexible(word) {
                    token_filter = token_filter.with_subtype(subtype);
                } else {
                    return Ok(None);
                }
            }
            StaticAbility::add_token_creation_replacement(
                PlayerFilter::You,
                token_filter,
                ironsmith_core::AdditionalTokenKind::Treasure,
                1,
                display_text_for_tokens(tokens, true),
            )
        }
    }))
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
    let Some(prevention) =
        static_keyword_replacement_shapes::parse_combat_prevention_prefix(tokens)
    else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&tokens[prevention.end..]);
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
            StaticAbility::prevent_all_damage_to_self().with_condition(
                crate::ConditionExpr::ActivationTiming(
                    crate::ability::ActivationTiming::DuringYourTurn,
                ),
            ),
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

pub(crate) fn parse_prevent_all_noncombat_damage_to_matching_permanents_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_all_noncombat_damage_to_matching_permanents_line_lexed(tokens) {
        return Ok(None);
    }

    let Some(prevention) =
        static_keyword_replacement_shapes::parse_noncombat_prevention_prefix(tokens)
    else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&tokens[prevention.end..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all noncombat damage target filter: {}",
            render_token_slice(tokens)
        )));
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    Ok(Some(
        StaticAbility::prevent_all_noncombat_damage_to_permanents_matching(filter),
    ))
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
    let clause = LexedClause::new(tokens);
    if !YOU_MAY_CHOOSE_NOT_TO_UNTAP_PREFIX_PATTERN.matches(clause) {
        return Ok(None);
    }
    if !DURING_YOUR_UNTAP_STEP_TAIL_PATTERN.matches(clause) {
        return Ok(None);
    }
    let words = clause.words();
    if words.len() <= 10 {
        return Ok(None);
    }

    let Some(subject_clause) = clause.between_word_range(6, words.len() - 4) else {
        return Ok(None);
    };
    let subject_words = subject_clause.words().to_word_refs();
    if !MAY_CHOOSE_NOT_UNTAP_SOURCE_SUBJECT_PATTERN.matches(subject_clause)
        && !is_source_reference_words(&subject_words)
    {
        return Ok(None);
    }

    let subject = subject_clause.text();
    let subject = source_reference_surface_for_words(&subject_words)
        .map(|surface| surface.display_text())
        .unwrap_or(subject);
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
        return Err(CardTextError::ParseError(format!(
            "missing subject in other-players untap ability (clause: '{}')",
            render_token_slice(tokens)
        )));
    }

    let filter = parse_object_filter(&subject_tokens, false)?;
    let subject_text = render_token_slice(&subject_tokens);
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
            let clause_display = render_token_slice(tokens);
            let tail_tokens = trim_commas(tail_tokens);
            if tail_tokens.is_empty() {
                return Ok(Some(
                    StaticAbilityAst::Static(StaticAbility::doesnt_untap()),
                ));
            }
            if tail_tokens.first().is_some_and(|token| token.is_word("if")) {
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after untap-step if-clause (clause: '{}')",
                        clause_display
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
                clause_display
            )))
        }
        Some(DoesntUntapDuringUntapStepSpec::Attached {
            subject_tokens,
            tail_tokens,
        }) => {
            let subject = render_token_slice(subject_tokens);
            let text = format!("{subject} doesnt untap during its controllers untap step");
            let condition = if tail_tokens.is_empty() {
                None
            } else {
                let clause_display = render_token_slice(tokens);
                if !tail_tokens
                    .first()
                    .is_some_and(|token| token.as_word() == Some("unless"))
                {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing attached untap-step clause (clause: '{}')",
                        clause_display
                    )));
                }
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after attached untap-step unless-clause (clause: '{}')",
                        clause_display
                    )));
                }
                Some(crate::ConditionExpr::Not(Box::new(
                    parse_static_condition_clause(&condition_tokens)?,
                )))
            };
            Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                display: text,
                condition,
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
    let Some(spec) = keyword_static_lines::parse_mana_value_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    Ok(Some(StaticAbility::grants(crate::grant::GrantSpec::new(
        crate::grant::Grantable::mana_value_as_generic_from_hand(),
        filter,
        Zone::Hand,
    ))))
}

pub(crate) fn parse_life_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_life_mana_value_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    let usage_limit = match spec.usage_limit {
        keyword_static_lines::LifeManaValueGrantUsageLimit::OnceDuringEachOfYourTurns => {
            crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns
        }
    };
    Ok(Some(StaticAbility::grants(crate::grant::GrantSpec::new(
        crate::grant::Grantable::life_equal_mana_value_from_hand(Some(usage_limit)),
        filter,
        Zone::Hand,
    ))))
}

pub(crate) fn parse_fixed_mana_cost_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_fixed_mana_cost_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    Ok(Some(StaticAbility::grants(
        crate::grant::GrantSpec::cast_from_hand_for_alternative_mana_cost_matching(
            filter,
            spec.mana_cost,
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
    let Some(spec) = keyword_static_lines::parse_exile_counter_permission_tokens(tokens) else {
        return Ok(None);
    };
    let is_play_lands_and_cast_noncreature_family = matches!(
        spec.family,
        keyword_static_lines::ExileCounterPermissionFamily::PlayLandsAndCastNoncreatureCardsExiledBySource
    );
    let owner = match spec.owner {
        keyword_static_lines::ExileCounterPermissionOwner::Any => None,
        keyword_static_lines::ExileCounterPermissionOwner::Opponent => Some(PlayerFilter::Opponent),
    };

    let uses_snow_sources = matches!(
        spec.mana_permission,
        keyword_static_lines::ExileCounterManaPermission::SnowSources
    );

    let mut base_filter = ObjectFilter {
        zone: Some(Zone::Exile),
        owner,
        with_counter: Some(crate::filter::CounterConstraint::Typed(spec.counter_type)),
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
    if !SURVEILLED_GRAVEYARD_PLAY_LIFE_COST_PATTERN.matches(LexedClause::new(tokens)) {
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
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if SOURCE_LINKED_EXILE_CAST_PREFIX_PATTERN.matches(clause)
        && ANY_MANA_CAST_SUFFIX_PATTERN.matches(clause)
        && clause.word_len() > 19 + 11
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.owner = Some(PlayerFilter::NotYou);
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        let grant = StaticAbility::grants(
            crate::grant::GrantSpec::new(
                crate::grant::Grantable::play_from(),
                filter.clone(),
                Zone::Exile,
            )
            .with_beneficiary(PlayerFilter::Any),
        );
        let mana_permission = StaticAbility::mana_spend_permission(
            crate::effect::ManaSpendPermission::any_color_for_casting_matching(
                PlayerFilter::Any,
                filter,
            ),
            "Mana of any type can be spent to cast it",
        );
        return Ok(Some(vec![grant, mana_permission]));
    }

    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) => {
            let singular_spell = CAST_SINGLE_SPELL_PATTERN.matches(clause);
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
                .map(|beneficiary| vec![StaticAbility::grants(spec.with_beneficiary(beneficiary))]))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_as_you_cascade_land_drop_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_cascade_land_drop_tokens(tokens) {
        return Ok(Some(StaticAbility::cascade_land_drop()));
    }
    Ok(None)
}

pub(crate) fn parse_play_from_permission_with_haste_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [permission_sentence, haste_sentence] = sentences.as_slice() else {
        return Ok(None);
    };

    if !CAST_CREATURE_THIS_WAY_HASTE_SENTENCE_PATTERN.matches(LexedClause::new(haste_sentence)) {
        return Ok(None);
    }

    match parse_permission_clause_spec(permission_sentence)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(spec.grantable, crate::grant::Grantable::PlayFrom)
            && spec.filter.card_types.len() == 1
            && spec
                .filter
                .card_types
                .iter()
                .any(|card_type| *card_type == CardType::Creature) =>
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

pub(crate) fn parse_play_from_permission_with_enter_counter_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(parsed) = keyword_static_lines::parse_play_permission_enter_counter_tokens(tokens)
    else {
        return Ok(None);
    };

    match parse_permission_clause_spec(parsed.permission_tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(spec.grantable, crate::grant::Grantable::PlayFrom) => {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(spec.with_beneficiary(beneficiary).with_cast_this_way_grant(
                    StaticAbility::enters_with_counters_value(parsed.counter_type, Value::Fixed(1)),
                ))
            }))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_play_from_permission_with_enter_tapped_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [permission_sentence, tapped_sentence] = sentences.as_slice() else {
        return Ok(None);
    };

    if !CAST_THIS_WAY_ENTERS_TAPPED_SENTENCE_PATTERN.matches(LexedClause::new(tapped_sentence)) {
        return Ok(None);
    }

    match parse_permission_clause_spec(permission_sentence)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(
            spec.grantable,
            crate::grant::Grantable::PlayFrom
                | crate::grant::Grantable::AlternativeCast(_)
                | crate::grant::Grantable::DerivedAlternativeCast(_)
        ) =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(
                    spec.with_beneficiary(beneficiary)
                        .with_cast_this_way_grant(StaticAbility::enters_tapped_ability()),
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
    if CONTROL_OPPONENTS_WHILE_SEARCHING_PATTERN.matches(LexedClause::new(tokens)) {
        return Ok(Some(
            StaticAbility::control_opponents_while_searching_libraries(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_opponent_search_exile_found_cards_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if OPPONENT_SEARCH_EXILE_FOUND_CARDS_PATTERN.matches(LexedClause::new(tokens)) {
        return Ok(Some(StaticAbility::opponent_search_exile_found_cards()));
    }
    Ok(None)
}

pub(crate) fn parse_cast_this_card_from_library_while_searching_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if CAST_THIS_CARD_FROM_LIBRARY_WHILE_SEARCHING_PATTERN.matches(LexedClause::new(tokens)) {
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
    let clause = LexedClause::new(tokens);
    let words = crate::runtime_backend::token_word_refs(tokens);
    if ATTACHED_CONTROLLER_ATTACK_EACH_COMBAT_PATTERN.matches(clause) {
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::all_creatures_attack_attached_controller_each_combat_if_able(),
        )));
    }

    let Some(attack_idx) =
        static_keyword_line_shapes::parse_attack_word(&words).map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    if !clause
        .after_words(attack_idx)
        .is_some_and(|tail| ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches(tail))
    {
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
    let clause = LexedClause::new(tokens);
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !YOU_MAY_PLAY_PREFIX_PATTERN.matches(clause) {
        return Ok(None);
    }

    let mut count_word_idx = 3;
    if clause
        .after_words(count_word_idx)
        .is_some_and(|tail| UP_TO_PREFIX_PATTERN.matches(tail))
    {
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
    if !clause
        .after_words(rest_word_idx)
        .is_some_and(|tail| ADDITIONAL_LAND_PLAY_TAIL_PATTERN.matches(tail))
    {
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
    let clause = LexedClause::new(tokens);
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(have_idx) =
        static_keyword_line_shapes::parse_retrace_have_word(&words).map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    if !clause
        .after_words(have_idx + 1)
        .is_some_and(|tail| RETRACE_TAIL_PATTERN.matches(tail))
    {
        return Ok(None);
    }
    let prefix_start = if words.first().is_some_and(|word| *word == "each") {
        1
    } else {
        0
    };
    if have_idx <= prefix_start + 3 {
        return Ok(None);
    }
    let Some(prefix_clause) = clause.between_word_range(prefix_start, have_idx) else {
        return Ok(None);
    };
    if !STATIC_IN_YOUR_GRAVEYARD_SUFFIX_PATTERN.matches(prefix_clause) {
        return Ok(None);
    }

    let subject = &words[prefix_start..have_idx - 3];
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
    if keyword_find_prefix_shape_start(LexedClause::new(tokens), &CAST_A_OR_ONE_SPELL_PATTERN)
        .is_some()
    {
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
    static_keyword_shapes::parse_pt_components(raw)
        .map(|components| (components.power, components.toughness))
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
    let Some(spec) = keyword_static_lines::parse_hand_size_line_tokens(tokens) else {
        return Ok(None);
    };
    let player = match spec.player {
        keyword_static_lines::HandSizePlayerKind::You => PlayerFilter::You,
        keyword_static_lines::HandSizePlayerKind::Opponent => PlayerFilter::Opponent,
        keyword_static_lines::HandSizePlayerKind::Any => PlayerFilter::Any,
    };
    let min_card_types_condition = if let Some(condition_tokens) = spec.condition_tokens {
        let Some((metric, threshold)) =
            parse_graveyard_metric_threshold_condition(condition_tokens)?
        else {
            return Ok(None);
        };
        if metric != crate::static_abilities::GraveyardCountMetric::CardTypes {
            return Ok(None);
        }
        threshold
    } else {
        0
    };
    Ok(Some(match spec.operation {
        keyword_static_lines::HandSizeOperation::Reduce(amount) => {
            StaticAbility::reduce_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::Increase(amount) => {
            StaticAbility::increase_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::Set(amount) => {
            StaticAbility::set_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::SevenMinusGraveyardCardTypes => {
            StaticAbility::max_hand_size_seven_minus_your_graveyard_card_types(
                player,
                min_card_types_condition,
            )
        }
    }))
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
    let clause = LexedClause::new(tokens);
    let words = parser_token_word_refs(tokens);
    if words.len() < 20 {
        return Ok(None);
    }

    if !DRAW_REPLACEMENT_EXILE_TOP_PLAY_PREFIX_PATTERN.matches(clause) {
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
        .is_some_and(|word| matches!(*word, "card" | "cards"))
    {
        return Ok(None);
    }

    if !clause
        .after_words(11)
        .is_some_and(|tail| DRAW_REPLACEMENT_EXILE_TOP_PLAY_TAIL_PATTERN.matches(tail))
    {
        return Ok(None);
    }

    Ok(Some(StaticAbility::draw_replacement_exile_top_and_play(
        count,
    )))
}

pub(crate) fn parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) =
        static_keyword_replacement_shapes::parse_draw_reveal_matching_rest_bottom(tokens)
    else {
        return Ok(None);
    };
    let Some(card_type) = parse_card_type(spec.card_type_word) else {
        return Ok(None);
    };
    let order = match spec.order {
        static_keyword_replacement_shapes::LibraryBottomOrderShape::Chosen => {
            ironsmith_core::LibraryBottomOrder::ChooserChooses
        }
        static_keyword_replacement_shapes::LibraryBottomOrderShape::Random => {
            ironsmith_core::LibraryBottomOrder::Random
        }
    };

    let mut filter = ObjectFilter::default();
    filter.card_types.push(card_type);

    Ok(Some(
        StaticAbility::draw_replacement_reveal_top_matching_to_hand_rest_bottom(
            spec.count,
            filter,
            order,
            render_token_slice(tokens),
        ),
    ))
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
    let clause = LexedClause::new(tokens);
    let words = parser_token_word_refs(tokens);
    let draw_subject_len = if CONDITIONAL_DRAW_REPLACEMENT_A_CARD_PREFIX_PATTERN.matches(clause) {
        7
    } else if CONDITIONAL_DRAW_REPLACEMENT_CARD_PREFIX_PATTERN.matches(clause) {
        6
    } else {
        return Ok(None);
    };

    let Some(instead_idx) =
        static_keyword_line_shapes::parse_draw_replacement_instead_word(&words[draw_subject_len..])
            .map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    let instead_idx = draw_subject_len + instead_idx;
    let Some(condition_token_start) =
        static_keyword_shapes::parse_word_token_offset(tokens, draw_subject_len)
    else {
        return Ok(None);
    };
    let Some(condition_token_end) =
        static_keyword_shapes::parse_word_token_offset(tokens, instead_idx)
    else {
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
    let draw_idx = if effect_words.first().is_some_and(|word| *word == "you") {
        1
    } else {
        0
    };
    if !effect_words
        .get(draw_idx)
        .is_some_and(|word| *word == "draw")
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
        .is_some_and(|word| matches!(*word, "card" | "cards"))
    {
        return Ok(None);
    }

    let mut next_idx = draw_idx + 3;
    if effect_words
        .get(next_idx)
        .is_some_and(|word| *word == "instead")
    {
        next_idx += 1;
    }

    let mut replacement_effects = vec![Effect::draw(draw_count as i32)];
    let mut life_loss = None;
    if next_idx < effect_words.len() {
        let tail = &effect_words[next_idx..];
        if tail.len() != 5
            || !clause
                .after_words(instead_idx + 1 + next_idx)
                .is_some_and(|tail| CONDITIONAL_DRAW_LIFE_LOSS_TAIL_PATTERN.matches(tail))
        {
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

pub(crate) fn parse_keyword_action_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_keyword_action_replacement_tokens(tokens) else {
        return Ok(None);
    };
    let display = render_token_slice(tokens);
    Ok(Some(match shape {
        keyword_static_lines::KeywordActionReplacementShape::ProliferateYouTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Proliferate,
                ObjectFilter::default().controlled_by(PlayerFilter::You),
                vec![Effect::proliferate(2)],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ProliferateOpponentTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Proliferate,
                ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
                vec![Effect::proliferate(2)],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ExploreTwice => {
            let explored_creature = ChooseSpec::tagged(IT_TAG);
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Explore,
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                vec![
                    Effect::explore(explored_creature.clone()),
                    Effect::explore(explored_creature),
                ],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ExploreAfterScry { value_tokens } => {
            let value_words = parser_token_word_refs(value_tokens);
            let (count, used) = parse_value_expr_words(&value_words).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported scry amount in keyword-action replacement (clause: '{}')",
                    render_token_slice(tokens)
                ))
            })?;
            if used != value_words.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported scry amount in keyword-action replacement (clause: '{}')",
                    render_token_slice(tokens)
                )));
            }
            let explored_creature = ChooseSpec::tagged(IT_TAG);
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Explore,
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                vec![Effect::scry(count), Effect::explore(explored_creature)],
                display,
            )
        }
    }))
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
    let Some(spec) = keyword_static_lines::parse_exile_to_graveyard_replacement_tokens(tokens)
    else {
        return Ok(None);
    };
    let graveyard_owner = match spec.graveyard_owner {
        keyword_static_lines::ReplacementPlayerKind::Any => PlayerFilter::Any,
        keyword_static_lines::ReplacementPlayerKind::You => PlayerFilter::You,
        keyword_static_lines::ReplacementPlayerKind::Opponent => PlayerFilter::Opponent,
    };
    let filter = match spec.filter_kind {
        keyword_static_lines::ExileGraveyardFilterKind::Source => ObjectFilter::source(),
        keyword_static_lines::ExileGraveyardFilterKind::AnyCard => ObjectFilter::default(),
        keyword_static_lines::ExileGraveyardFilterKind::CreatureCard => ObjectFilter::creature(),
        keyword_static_lines::ExileGraveyardFilterKind::CyclingCard => {
            ObjectFilter::default().with_ability_marker("cycling")
        }
        keyword_static_lines::ExileGraveyardFilterKind::ObjectFilter => {
            parse_object_filter(spec.filter_tokens, false)?
        }
    };
    let ability = if spec.exclude_cycled {
        StaticAbility::exile_to_exile_instead_of_graveyard_unless_cycled(filter, graveyard_owner)
    } else {
        StaticAbility::exile_to_exile_instead_of_graveyard(filter, graveyard_owner)
    };
    Ok(Some(ability))
}

pub(crate) fn parse_exile_would_die_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_exile_would_die_tokens(tokens) else {
        return Ok(None);
    };
    let ability = match spec {
        keyword_static_lines::ExileWouldDieSpec::NontokenCreature {
            controller,
            exile_counter,
            create_zombie,
        } => {
            let matched_filter = match controller {
                keyword_static_lines::ReplacementPlayerKind::Any => {
                    ObjectFilter::creature().nontoken()
                }
                keyword_static_lines::ReplacementPlayerKind::You => ObjectFilter::creature()
                    .nontoken()
                    .controlled_by(PlayerFilter::You),
                keyword_static_lines::ReplacementPlayerKind::Opponent => ObjectFilter::creature()
                    .nontoken()
                    .controlled_by(PlayerFilter::Opponent),
            };
            let exile_with_counters = exile_counter
                .map(|counter_type| vec![(counter_type, 1)])
                .unwrap_or_default();
            let follow_up = create_zombie
                .then(|| vec![Effect::create_tokens(kalitas_zombie_token(), 1)])
                .unwrap_or_default();
            StaticAbility::exile_would_die_instead_with_damage_source_counters_and_follow_up(
                matched_filter,
                None,
                exile_with_counters,
                follow_up,
            )
        }
        keyword_static_lines::ExileWouldDieSpec::DamagedBy { victim, damaged_by } => {
            let victim = match victim {
                keyword_static_lines::ExileWouldDieVictimKind::Creature => ObjectFilter::creature(),
                keyword_static_lines::ExileWouldDieVictimKind::Permanent => {
                    ObjectFilter::permanent()
                }
            };
            StaticAbility::exile_would_die_instead_with_damage_source(victim, Some(damaged_by))
        }
        keyword_static_lines::ExileWouldDieSpec::SimpleSource(kind) => {
            let filter = match kind {
                keyword_static_lines::SimpleSourceReplacementKind::Any => ObjectFilter::source(),
                keyword_static_lines::SimpleSourceReplacementKind::Creature => {
                    ObjectFilter::source().with_type(CardType::Creature)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Artifact => {
                    ObjectFilter::source().with_type(CardType::Artifact)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Enchantment => {
                    ObjectFilter::source().with_type(CardType::Enchantment)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Permanent => {
                    ObjectFilter::source()
                }
            };
            StaticAbility::exile_would_die_instead(filter)
        }
        keyword_static_lines::ExileWouldDieSpec::SimpleCreature(player) => {
            let player = match player {
                keyword_static_lines::ReplacementPlayerKind::Any => PlayerFilter::Any,
                keyword_static_lines::ReplacementPlayerKind::You => PlayerFilter::You,
                keyword_static_lines::ReplacementPlayerKind::Opponent => PlayerFilter::Opponent,
            };
            StaticAbility::exile_would_die_instead(ObjectFilter::creature().controlled_by(player))
        }
    };
    Ok(Some(ability))
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
    Ok(
        static_keyword_replacement_shapes::parse_discard_or_redirect_replacement(tokens).map(
            |shape| {
                StaticAbility::discard_or_redirect_replacement(
                    ObjectFilter::default().with_type(shape.discard_type),
                    shape.redirect_zone,
                )
            },
        ),
    )
}

pub(crate) fn parse_pay_life_or_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    if words.len() < 8 {
        return Ok(None);
    }

    if !AS_THIS_CONTAINS_PAY_LIFE_PATTERN.matches(clause) {
        return Ok(None);
    }

    let Some(pay_shape) = static_keyword_line_shapes::parse_pay_life_etb_shape(tokens) else {
        return Err(CardTextError::ParseError(format!(
            "missing 'pay' keyword in pay-life ETB clause (clause: '{}')",
            words.join(" ")
        )));
    };
    let pay_idx = pay_shape.pay.token;
    if !pay_shape.saw_enter {
        return Ok(None);
    }
    if !pay_shape.saw_may {
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

    let if_dont_idx = keyword_find_prefix_shape_start(clause, &IF_YOU_DONT_PREFIX_PATTERN)
        .filter(|_| IF_YOU_DONT_PHRASE_PATTERN.matches(clause))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported pay-life ETB trailing clause (expected 'if you don't ...') (clause: '{}')",
                words.join(" ")
            ))
        })?;

    if !clause
        .after_words(if_dont_idx + 3)
        .is_some_and(|trailing| PAY_LIFE_ENTER_TAPPED_TAIL_PATTERN.matches(trailing))
    {
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
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    if clause_words.len() < 6 {
        return Ok(None);
    }

    let mut has_idx = None;
    for idx in 0..clause_words.len().saturating_sub(4) {
        if clause
            .after_words(idx)
            .is_some_and(|tail| HAS_ALL_ACTIVATED_ABILITIES_OF_PATTERN.matches(tail))
        {
            has_idx = Some(idx);
            break;
        }
    }
    let Some(has_idx) = has_idx else {
        return Ok(None);
    };
    let only_loyalty = clause_words
        .get(has_idx + 2)
        .is_some_and(|word| *word == "loyalty");
    let Some(has_token_idx) = static_keyword_shapes::parse_word_token_offset(tokens, has_idx)
    else {
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

    let Some(filter_start_idx) =
        static_keyword_shapes::parse_word_token_offset(tokens, has_idx + 5)
    else {
        return Ok(None);
    };
    let filter_tokens = trim_edge_punctuation(&tokens[filter_start_idx..]);
    let mut filter_tokens =
        strip_leading_token_words_any(&filter_tokens, &["all", "each"]).to_vec();
    let filter_clause = LexedClause::new(&filter_tokens);
    let filter_word_len = filter_clause.word_len();
    let mut once_each_turn_start = None;
    for idx in 0..filter_word_len.saturating_sub(10) {
        if filter_clause
            .between_word_range(idx, idx + 11)
            .is_some_and(|window| ACTIVATE_EACH_OF_THOSE_ONCE_TAIL_PATTERN.matches(window))
        {
            once_each_turn_start = Some(idx);
            break;
        }
    }
    let force_once_each_turn = once_each_turn_start.is_some();
    if let Some(start) = once_each_turn_start
        && let Some(token_idx) =
            static_keyword_shapes::parse_word_token_offset(&filter_tokens, start)
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

    let counter = match filter.with_counter {
        Some(crate::filter::CounterConstraint::Typed(counter_type)) => Some(counter_type),
        _ => None,
    };

    let exclude_source_name = (0..clause_words.len().saturating_sub(4)).any(|idx| {
        clause
            .between_word_range(idx, idx + 5)
            .is_some_and(|window| SAME_NAME_AS_SOURCE_CREATURE_PATTERN.matches(window))
    });
    let display_words = copy_activated_abilities_display_words(&clause_words);
    let display = if force_once_each_turn {
        let mut display_tail_start = None;
        for idx in 0..clause_words.len().saturating_sub(10) {
            if clause
                .between_word_range(idx, idx + 11)
                .is_some_and(|window| ACTIVATE_EACH_OF_THOSE_ONCE_TAIL_PATTERN.matches(window))
            {
                display_tail_start = Some(copy_activated_display_index_for_original_word(
                    &clause_words,
                    idx,
                ));
                break;
            }
        }
        if let Some(start) = display_tail_start {
            format!(
                "{}. You may activate each of those abilities only once each turn",
                display_words[..start].join(" ").trim()
            )
        } else {
            display_words.join(" ")
        }
    } else {
        display_words.join(" ")
    };

    let mut ability = crate::static_abilities::CopyActivatedAbilities::new(filter)
        .with_exclude_source_name(exclude_source_name)
        .with_exclude_source_id(true)
        .with_display(display);
    if let Some(counter) = counter {
        ability = ability.with_counter(counter);
    }
    if only_loyalty {
        ability = ability.with_only_loyalty();
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

fn copy_activated_abilities_display_words<'a>(clause_words: &[&'a str]) -> Vec<&'a str> {
    let mut display_words = Vec::with_capacity(clause_words.len());
    for (idx, word) in clause_words.iter().copied().enumerate() {
        if copy_activated_should_skip_display_word(clause_words, idx, word) {
            continue;
        }
        display_words.push(word);
    }
    display_words
}

fn copy_activated_display_index_for_original_word(
    clause_words: &[&str],
    original_idx: usize,
) -> usize {
    clause_words
        .iter()
        .enumerate()
        .take(original_idx)
        .filter(|(idx, word)| !copy_activated_should_skip_display_word(clause_words, *idx, word))
        .count()
}

fn copy_activated_should_skip_display_word(clause_words: &[&str], idx: usize, word: &str) -> bool {
    idx >= 2
        && word == clause_words[idx - 1]
        && clause_words[idx - 2] == "this"
        && copy_activated_display_source_noun(word)
}

fn copy_activated_display_source_noun(word: &str) -> bool {
    matches!(word, "card" | "permanent" | "source" | "spell")
        || parse_card_type(word).is_some()
        || parse_subtype_flexible(word).is_some()
}

pub(crate) fn parse_spend_mana_as_any_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_mana_spend_permission_tokens(tokens) else {
        return Ok(None);
    };
    let clause_words = parser_token_word_refs(tokens);
    let (permission, display) = match shape {
        keyword_static_lines::ManaSpendPermissionShape::SymbolAsAnyColorOtherAsColorless {
            symbol,
        } => {
            let symbol_text = match symbol {
                ManaSymbol::White => "white",
                ManaSymbol::Blue => "blue",
                ManaSymbol::Black => "black",
                ManaSymbol::Red => "red",
                ManaSymbol::Green => "green",
                _ => unreachable!("typed grammar only returns colored mana symbols"),
            };
            (
                crate::effect::ManaSpendPermission::mana_symbol_as_any_color_other_as_colorless(
                    PlayerFilter::You,
                    symbol,
                ),
                format!(
                    "You may spend {symbol_text} mana as though it were mana of any color. You may spend other mana only as though it were colorless mana"
                ),
            )
        }
        keyword_static_lines::ManaSpendPermissionShape::AnyTypeToCast { filter_tokens } => {
            let filter = parse_object_filter(filter_tokens, false)
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
            (
                crate::effect::ManaSpendPermission::any_color_for_casting_matching(
                    PlayerFilter::You,
                    filter,
                ),
                clause_words.join(" "),
            )
        }
        keyword_static_lines::ManaSpendPermissionShape::AnyColor {
            player,
            activation_filter_tokens,
            source_activation_only,
        } => {
            let player = match player {
                keyword_static_lines::ManaSpendPlayerKind::You => PlayerFilter::You,
                keyword_static_lines::ManaSpendPlayerKind::Any => PlayerFilter::Any,
            };
            let permission = if source_activation_only {
                crate::effect::ManaSpendPermission::any_color_for_activation(
                    player.clone(),
                    ObjectFilter::source(),
                )
            } else if let Some(filter_tokens) = activation_filter_tokens {
                let filter = match parse_object_filter(filter_tokens, false) {
                    Ok(filter) => filter,
                    Err(_) => return Ok(None),
                };
                crate::effect::ManaSpendPermission::any_color_for_activation(player.clone(), filter)
            } else {
                crate::effect::ManaSpendPermission::any_color(player.clone())
            };
            let display = if player == PlayerFilter::Any {
                "Players may spend mana as though it were mana of any color".to_string()
            } else {
                clause_words.join(" ")
            };
            (permission, display)
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
    use crate::static_abilities::StaticAbilityId;

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

    #[test]
    fn parse_keyword_action_replacement_static_line() {
        let tokens =
            lex_line("If you would proliferate, proliferate twice instead.", 0).expect("lex");
        let parsed = parse_keyword_action_replacement_line(&tokens)
            .expect("keyword-action replacement parser should not hard-error");
        assert!(
            parsed
                .as_ref()
                .is_some_and(|ability| ability.id() == StaticAbilityId::KeywordActionReplacement),
            "expected keyword-action replacement static ability, got {parsed:?}"
        );
        let parsed = parse_static_ability_ast_line_lexed(&tokens)
            .expect("static ability line parser should not hard-error");
        assert!(
            parsed
                .as_ref()
                .is_some_and(|abilities| abilities.iter().any(
                    |ability| matches!(ability, StaticAbilityAst::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::KeywordActionReplacement)
                )),
            "expected static line parser to preserve keyword-action replacement, got {parsed:?}"
        );
    }
}
