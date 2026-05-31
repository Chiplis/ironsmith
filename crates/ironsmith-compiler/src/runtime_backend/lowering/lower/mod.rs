use crate::Until;
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::builders::{
    CardDefinition, CardDefinitionBuilder, CardTextError, ChoiceCount, EffectAst, GiftTimingAst,
    IT_TAG, InsteadSemantics, LibraryBottomOrderAst, LineAst, LineInfo, NormalizedLine,
    OptionalCost, ParseAnnotations, ParsedAbility, ParsedCardItem, ParsedLevelAbilityAst,
    ParsedLevelAbilityItemAst, ParsedLineAst, ParsedModalAst, ParsedModalModeAst,
    ParsedRestrictions, PlayerAst, PredicateAst, ReferenceImports, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, TriggerSpec,
};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::mana::ManaSymbol;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

mod activated_lowering;
mod damage_and_cost_rewrites;
mod line_lowering;
mod modal_and_level_lowering;
mod normalization_support;
mod parser_semantic_lowering;
mod rewrite_sentence_grouping;
mod rewrite_support;
mod rewrite_text_helpers;

pub(crate) use activated_lowering::lower_rewrite_activated_to_chunk;
use activated_lowering::{LoweredRewriteActivatedLine, align_rewrite_activated_parse_sentences};
use normalization_support::{
    apply_chosen_option_to_triggered_chunk, apply_explicit_intervening_if_to_triggered_chunk,
};
pub(crate) use normalization_support::{
    prepare_parsed_card_ast_for_lowering, rewrite_document_to_normalized_card_ast,
    rewrite_document_to_parsed_card_ast,
};
#[cfg(test)]
pub(crate) use parser_semantic_lowering::lower_rewrite_keyword_to_chunk;
use parser_semantic_lowering::{
    infer_rewrite_triggered_functional_zones, lower_rewrite_modal_to_item,
};
pub(crate) use parser_semantic_lowering::{
    lower_exert_attack_keyword_line, lower_gift_keyword_line, lower_keyword_special_cases,
    lower_rewrite_statement_token_groups_to_chunks, lower_rewrite_static_to_chunk,
    lower_rewrite_triggered_to_chunk,
};
pub(crate) use parser_semantic_lowering::{
    lower_special_rewrite_triggered_chunk, try_lower_optional_behold_additional_cost,
    try_lower_optional_cost_with_cast_trigger,
};
#[cfg(test)]
use parser_semantic_lowering::{
    normalize_exert_followup_source_reference_tokens, parse_single_effect_lexed,
    strip_lexed_suffix_phrase,
};

pub(crate) use damage_and_cost_rewrites::*;
pub(crate) use modal_and_level_lowering::*;
pub(crate) use rewrite_sentence_grouping::*;
use rewrite_support::{
    infer_static_ability_functional_zones, infer_triggered_ability_functional_zones,
    rewrite_finalize_lowered_card, rewrite_normalize_additional_cost_sacrifice_tags,
    runtime_effects_to_costs,
};
pub(crate) use rewrite_text_helpers::*;

use super::activation_and_restrictions::{
    infer_activated_functional_zones_lexed, is_any_player_may_activate_sentence_lexed,
    parse_activation_cost, parse_mana_spend_bonus_sentence_lexed,
    parse_mana_usage_restriction_sentence_lexed,
};
use super::activation_and_restrictions::{
    parse_channel_line_lexed, parse_cycling_line_lexed, parse_equip_line_lexed,
};
use super::clause_support::{
    parse_ability_line_lexed, parse_effect_sentences_lexed, parse_static_ability_ast_line_lexed,
    parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
use super::compile_support::{
    collect_tag_spans_from_effects_with_context, compile_condition_from_predicate_ast_with_env,
    materialize_prepared_effects_with_trigger_context,
    trigger_binds_player_reference_context as rewrite_trigger_binds_player_reference_context,
};
use super::effect_pipeline::{
    NormalizedAdditionalCostChoiceOptionAst, NormalizedCardAst, NormalizedCardItem,
    NormalizedLineAst, NormalizedLineChunk, NormalizedModalAst, NormalizedModalModeAst,
    NormalizedParsedAbility, NormalizedPreparedAbility, ParsedCardAst,
};
use super::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed;
use super::ir::{
    RewriteKeywordLine, RewriteKeywordLineKind, RewriteLevelHeader, RewriteModalBlock,
    RewriteSagaChapterLine, RewriteSemanticDocument, RewriteSemanticItem, RewriteStatementLine,
    RewriteStaticLine, RewriteTriggeredLine,
};
use super::keyword_static::parse_if_this_spell_costs_less_to_cast_line_lexed;
use super::lexer::{
    OwnedLexToken, TokenKind, TokenWordView, contains_token_word_sequence, lex_line,
    render_token_slice, split_lexed_sentences, token_slice_ends_with, token_slice_starts_with,
    token_word_refs, trim_lexed_commas,
};
use super::lexer::{
    word_slice_contains_phrase, word_slice_contains_phrase_or_empty, word_slice_ends_with,
    word_slice_find_phrase_start, word_slice_find_word, word_slice_starts_with,
};
use super::lowering_support::{
    rewrite_apply_instead_followup_statement_to_last_ability, rewrite_lower_prepared_ability,
    rewrite_lower_prepared_additional_cost_choice_modes_with_exports,
    rewrite_lower_prepared_statement_effects, rewrite_lower_static_abilities_ast,
    rewrite_lower_static_ability_ast, rewrite_parsed_triggered_ability,
    rewrite_prepare_effects_for_lowering,
    rewrite_prepare_effects_with_trigger_context_for_lowering,
    rewrite_prepare_triggered_effects_for_lowering, rewrite_static_ability_for_keyword_action,
    rewrite_validate_iterated_player_bindings_in_lowered_effects,
};
use super::modal_support::{parse_modal_header, replace_modal_header_x_in_effects_ast};
use super::parser_support::split_text_for_parse;
use super::reference_model::LoweredEffects;
use super::reference_model::ReferenceEnv;
use super::reference_model::ReferenceExports;
use super::restriction_support::{
    apply_pending_mana_restriction, apply_pending_restrictions_to_ability, is_restrictable_ability,
};
use super::token_primitives::{
    find_index, iter_contains, lexed_tokens_contain_non_prefix_instead,
    remove_copy_exception_type_removal_lexed, rewrite_followup_intro_to_if_lexed, slice_contains,
    slice_ends_with, slice_starts_with, split_em_dash_label_prefix, str_contains, str_find,
    str_split_once, str_split_once_char, str_strip_prefix, str_strip_suffix,
    word_view_has_any_prefix, word_view_has_prefix,
};
use super::util::{
    classify_instead_followup_text, find_first_sacrifice_cost_choice_tag,
    find_last_exile_cost_choice_tag, join_sentences_with_period,
    parse_additional_cost_choice_options_lexed, parse_bargain_line_lexed, parse_bestow_line_lexed,
    parse_buyback_line_lexed, parse_cast_this_spell_only_line_lexed, parse_entwine_line_lexed,
    parse_escape_line_lexed, parse_flashback_line_lexed, parse_harmonize_line_lexed,
    parse_if_conditional_alternative_cost_line_lexed, parse_kicker_line_lexed,
    parse_level_up_line_lexed, parse_madness_line_lexed, parse_mana_symbol,
    parse_morph_keyword_line_lexed, parse_multikicker_line_lexed, parse_number_or_x_value_lexed,
    parse_offspring_line_lexed, parse_reinforce_line_lexed, parse_scryfall_mana_cost,
    parse_self_free_cast_alternative_cost_line_lexed, parse_squad_line_lexed,
    parse_transmute_line_lexed, parse_warp_line_lexed,
    parse_you_may_rather_than_spell_cost_line_lexed, preserve_keyword_prefix_for_parse,
    token_index_for_word_index, trim_commas, words,
};

const BECOMES_TAPPED_DURING_YOUR_TURN_PHRASE: &[&str] =
    &["becomes", "tapped", "during", "your", "turn"];
const THIS_ABILITY_TRIGGERS_ONLY_ONCE_EACH_TURN_PHRASE: &[&str] = &[
    "this", "ability", "triggers", "only", "once", "each", "turn",
];
const THIS_ABILITY_TRIGGERS_ONLY_TWICE_EACH_TURN_PHRASE: &[&str] = &[
    "this", "ability", "triggers", "only", "twice", "each", "turn",
];
const DO_THIS_ONLY_ONCE_EACH_TURN_PHRASE: &[&str] = &["do", "this", "only", "once", "each", "turn"];
const DO_THIS_ONLY_TWICE_EACH_TURN_PHRASE: &[&str] =
    &["do", "this", "only", "twice", "each", "turn"];
const PUT_THAT_CARD_ONTO_BATTLEFIELD_INSTEAD_OF_HAND_PHRASE: &[&str] = &[
    "put",
    "that",
    "card",
    "onto",
    "the",
    "battlefield",
    "instead",
    "of",
    "putting",
    "it",
    "into",
    "your",
    "hand",
];
const CREATURE_DIED_THIS_TURN_PHRASE: &[&str] = &["creature", "died", "this", "turn"];
const IF_THIS_SPELL_WAS_BARGAINED_PHRASE: &[&str] = &["if", "this", "spell", "was", "bargained"];
const ONE_OF_THOSE_CARDS_MV_FOUR_OR_LESS_PHRASE: &[&str] = &[
    "one", "of", "those", "cards", "with", "mana", "value", "4", "or", "less",
];
const ONTO_BATTLEFIELD_INSTEAD_OF_HAND_PHRASE: &[&str] = &[
    "onto",
    "the",
    "battlefield",
    "instead",
    "of",
    "putting",
    "it",
    "into",
    "your",
    "hand",
];
const PUT_TWO_OF_THOSE_CARDS_INTO_YOUR_HAND_INSTEAD_PHRASE: &[&str] = &[
    "put", "two", "of", "those", "cards", "into", "your", "hand", "instead",
];
const PUT_ONE_OF_THOSE_CARDS_INTO_YOUR_HAND_PHRASE: &[&str] =
    &["put", "one", "of", "those", "cards", "into", "your", "hand"];
const SEARCH_LIBRARY_OR_GRAVEYARD_FOR_DOCTORS_PHRASE: &[&str] = &[
    "search",
    "your",
    "library",
    "and/or",
    "graveyard",
    "for",
    "up",
    "to",
    "five",
    "doctor",
    "cards",
];
const IF_THIS_SPELL_WAS_KICKED_PHRASE: &[&str] = &["if", "this", "spell", "was", "kicked"];
const PUT_THOSE_CARDS_ONTO_BATTLEFIELD_INSTEAD_OF_HAND_PHRASE: &[&str] = &[
    "put",
    "those",
    "cards",
    "onto",
    "the",
    "battlefield",
    "instead",
    "of",
    "putting",
    "them",
    "into",
    "your",
    "hand",
];
const CLASH_WITH_AN_OPPONENT_PHRASE: &[&str] = &["clash", "with", "an", "opponent"];
const IF_YOU_WIN_PHRASE: &[&str] = &["if", "you", "win"];
const ON_TOP_OF_OWNERS_LIBRARY_INSTEAD_PHRASE: &[&str] =
    &["on", "top", "of", "its", "owner's", "library", "instead"];
const SELF_ENTERS_WITH_X_PLUS_ONE_COUNTER_PREFIXES: &[&[&str]] = &[
    &[
        "this", "creature", "enters", "with", "x", "+1/+1", "counters", "on", "it",
    ],
    &[
        "this",
        "permanent",
        "enters",
        "with",
        "x",
        "+1/+1",
        "counters",
        "on",
        "it",
    ],
    &["it", "enters", "with", "x", "+1/+1", "counters", "on", "it"],
];
const NEITHER_DAY_NOR_NIGHT_PHRASE: &[&str] = &["neither", "day", "nor", "night"];
const BECOMES_DAY_PHRASE: &[&str] = &["becomes", "day"];
const AS_THIS_ENTERS_PHRASES: &[&[&str]] = &[
    &["as", "this", "creature", "enters"],
    &["as", "this", "permanent", "enters"],
    &["as", "this", "object", "enters"],
];
const THIS_SPELL_COST_PREFIXES: &[&[&str]] =
    &[&["this", "spell", "costs"], &["this", "spell", "cost"]];
const YOU_MAY_PAY_PREFIX: &[&str] = &["you", "may", "pay"];
const PARTNER_WITH_PREFIX: &[&str] = &["partner", "with"];
const FIRST_EQUIP_COST_ALTERNATIVE_PHRASE: &[&str] = &[
    "rather", "than", "pay", "the", "equip", "cost", "of", "the", "first", "equip", "ability",
    "you", "activate",
];
const FIRST_EQUIP_COST_ALTERNATIVE_SUFFIXES: &[&[&str]] = &[
    &["each", "turn"],
    &["during", "each", "of", "your", "turns"],
];
const EXHAUST_PREFIX: &[&str] = &["exhaust"];
const ADD_X_MANA_PHRASE: &[&str] = &["add", "x", "mana"];
const WHERE_X_IS_PHRASE: &[&str] = &["where", "x", "is"];
const ANY_PLAYER_MAY_ACTIVATE_THIS_ABILITY_PHRASE: &[&str] =
    &["any", "player", "may", "activate", "this", "ability"];
const ON_THE_STACK_PHRASE: &[&str] = &["on", "the", "stack"];
const EXILE_SELF_FROM_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["exile", "this", "card", "from", "your", "graveyard"],
    &["exile", "this", "creature", "from", "your", "graveyard"],
    &["exile", "this", "permanent", "from", "your", "graveyard"],
];
const STATIC_LIBRARY_SEARCH_ZONE_PHRASES: &[&[&str]] = &[
    &["while", "youre", "searching", "your", "library"],
    &["while", "you're", "searching", "your", "library"],
];
const FROM_YOUR_LIBRARY_PHRASE: &[&str] = &["from", "your", "library"];
const CAST_OR_PLAY_SELF_FROM_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["cast", "this", "card", "from", "your", "graveyard"],
    &["play", "this", "card", "from", "your", "graveyard"],
];
const CAST_OR_PLAY_SELF_FROM_EXILE_PHRASES: &[&[&str]] = &[
    &["cast", "this", "card", "from", "exile"],
    &["play", "this", "card", "from", "exile"],
];
const STATIC_ZONE_HINT_PHRASES: &[(&[&str], Zone)] = &[
    (&["this", "card", "is", "in", "your", "hand"], Zone::Hand),
    (
        &["there", "is", "this", "card", "in", "your", "hand"],
        Zone::Hand,
    ),
    (
        &["this", "card", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "creature", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "permanent", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "object", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["there", "is", "this", "card", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "card", "is", "in", "your", "library"],
        Zone::Library,
    ),
    (
        &["there", "is", "this", "card", "in", "your", "library"],
        Zone::Library,
    ),
    (&["this", "card", "is", "in", "exile"], Zone::Exile),
    (&["there", "is", "this", "card", "in", "exile"], Zone::Exile),
    (
        &["this", "card", "is", "in", "the", "command", "zone"],
        Zone::Command,
    ),
    (
        &[
            "there", "is", "this", "card", "in", "the", "command", "zone",
        ],
        Zone::Command,
    ),
];
const TRIGGER_ZONE_HINT_PHRASES: &[(&[&str], Zone)] = &[
    (&["if", "this", "is", "in", "your", "hand"], Zone::Hand),
    (
        &["if", "this", "card", "is", "in", "your", "hand"],
        Zone::Hand,
    ),
    (
        &["if", "this", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "card", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "creature", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "permanent", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "object", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "is", "in", "your", "library"],
        Zone::Library,
    ),
    (
        &["if", "this", "card", "is", "in", "your", "library"],
        Zone::Library,
    ),
    (&["if", "this", "is", "in", "exile"], Zone::Exile),
    (&["if", "this", "card", "is", "in", "exile"], Zone::Exile),
    (&["if", "this", "card", "is", "exiled"], Zone::Exile),
    (
        &["if", "this", "is", "in", "the", "command", "zone"],
        Zone::Command,
    ),
    (
        &["if", "this", "card", "is", "in", "the", "command", "zone"],
        Zone::Command,
    ),
];
const RETURN_SELF_FROM_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["return", "this", "from", "your", "graveyard"],
    &["return", "this", "card", "from", "your", "graveyard"],
];
const DISCARD_THIS_CARD_PHRASE: &[&str] = &["discard", "this", "card"];
const THIS_ABILITY_COSTS_PREFIX: &[&str] = &["this", "ability", "costs"];
const CANT_BE_BLOCKED_SUFFIXES: &[&[&str]] =
    &[&["can't", "be", "blocked"], &["cant", "be", "blocked"]];
const IF_YOU_HAVE_FULL_PARTY_PHRASE: &[&str] = &["if", "you", "have", "a", "full", "party"];
const UNTIL_END_OF_TURN_INSTEAD_PHRASE: &[&str] = &["until", "end", "of", "turn", "instead"];
const IF_YOU_DO_PHRASE: &[&str] = &["if", "you", "do"];
const IF_YOU_DONT_PHRASES: &[&[&str]] = &[&["if", "you", "don't"], &["if", "you", "dont"]];
const WHEN_IT_ENTERS_PHRASE: &[&str] = &["when", "it", "enters"];
const REMOVE_PREFIX: &[&str] = &["remove"];
const LEVEL_UP_PREFIX: &[&str] = &["level", "up"];
const DAMAGE_TO_EACH_PLAYER_CREATURES_PHRASES: &[&[&str]] = &[
    &[
        "damage", "to", "each", "player", "and", "each", "creature", "they", "control",
    ],
    &[
        "damage",
        "to",
        "each",
        "player",
        "and",
        "each",
        "creatures",
        "they",
        "control",
    ],
    &[
        "damage", "to", "each", "player", "and", "each", "creature", "that", "player", "controls",
    ],
    &[
        "damage",
        "to",
        "each",
        "player",
        "and",
        "each",
        "creatures",
        "that",
        "player",
        "controls",
    ],
];
const BLOCKS_OR_BECOMES_BLOCKED_PREFIX: &[&str] = &[
    "whenever", "this", "creature", "blocks", "or", "becomes", "blocked", "by", "a", "creature",
];
const THAT_CREATURE_FIRST_STRIKE_SUFFIX: &[&str] = &[
    "that", "creature", "gains", "first", "strike", "until", "end", "of", "turn",
];
const ATTACK_ACTION_SUFFIXES: &[&[&str]] = &[&["attack"], &["attacks"]];

fn token_is_trigger_intro_surface(token: &OwnedLexToken) -> bool {
    token.is_word("when") || token.is_word("whenever") || token.is_word("at")
}

fn tokens_start_with_trigger_intro_surface(tokens: &[OwnedLexToken]) -> bool {
    tokens.first().is_some_and(token_is_trigger_intro_surface)
}

fn text_starts_with_trigger_intro_surface(text: &str) -> bool {
    lex_line(text, 0)
        .ok()
        .is_some_and(|tokens| tokens_start_with_trigger_intro_surface(&tokens))
}

fn text_contains_word_phrase(text: &str, phrase: &[&str]) -> bool {
    lex_line(text, 0)
        .ok()
        .is_some_and(|tokens| contains_token_word_sequence(&tokens, phrase))
}

fn text_contains_any_word_phrase(text: &str, phrases: &[&[&str]]) -> bool {
    lex_line(text, 0).ok().is_some_and(|tokens| {
        phrases
            .iter()
            .any(|phrase| contains_token_word_sequence(&tokens, phrase))
    })
}

fn text_starts_with_any_word_phrase(text: &str, phrases: &[&[&str]]) -> bool {
    lex_line(text, 0).ok().is_some_and(|tokens| {
        phrases
            .iter()
            .any(|phrase| token_slice_starts_with(&tokens, phrase))
    })
}

fn text_mentions_becomes_tapped_during_your_turn(text: &str) -> bool {
    text_contains_word_phrase(text, BECOMES_TAPPED_DURING_YOUR_TURN_PHRASE)
}

fn do_this_frequency_surface_from_text(text: &str) -> Option<u32> {
    if text_contains_word_phrase(text, DO_THIS_ONLY_ONCE_EACH_TURN_PHRASE) {
        Some(1)
    } else if text_contains_word_phrase(text, DO_THIS_ONLY_TWICE_EACH_TURN_PHRASE) {
        Some(2)
    } else {
        None
    }
}

fn trigger_cap_surface_from_text(text: &str) -> Option<u32> {
    if text_contains_word_phrase(text, THIS_ABILITY_TRIGGERS_ONLY_ONCE_EACH_TURN_PHRASE)
        || text_contains_word_phrase(text, DO_THIS_ONLY_ONCE_EACH_TURN_PHRASE)
    {
        Some(1)
    } else if text_contains_word_phrase(text, THIS_ABILITY_TRIGGERS_ONLY_TWICE_EACH_TURN_PHRASE)
        || text_contains_word_phrase(text, DO_THIS_ONLY_TWICE_EACH_TURN_PHRASE)
    {
        Some(2)
    } else {
        None
    }
}

fn text_mentions_morbid_search_to_battlefield_replacement(text: &str) -> bool {
    text_contains_word_phrase(text, PUT_THAT_CARD_ONTO_BATTLEFIELD_INSTEAD_OF_HAND_PHRASE)
        && text_contains_word_phrase(text, CREATURE_DIED_THIS_TURN_PHRASE)
}

fn text_mentions_bargained_return_to_battlefield_replacement(text: &str) -> bool {
    text_contains_word_phrase(text, IF_THIS_SPELL_WAS_BARGAINED_PHRASE)
        && text_contains_word_phrase(text, ONE_OF_THOSE_CARDS_MV_FOUR_OR_LESS_PHRASE)
        && text_contains_word_phrase(text, ONTO_BATTLEFIELD_INSTEAD_OF_HAND_PHRASE)
}

fn text_mentions_kicked_count_override_replacement(text: &str) -> bool {
    text_contains_word_phrase(text, PUT_TWO_OF_THOSE_CARDS_INTO_YOUR_HAND_INSTEAD_PHRASE)
        && text_contains_word_phrase(text, PUT_ONE_OF_THOSE_CARDS_INTO_YOUR_HAND_PHRASE)
}

fn text_mentions_kicked_multi_zone_search_to_battlefield_replacement(text: &str) -> bool {
    text_contains_word_phrase(text, SEARCH_LIBRARY_OR_GRAVEYARD_FOR_DOCTORS_PHRASE)
        && text_contains_word_phrase(text, IF_THIS_SPELL_WAS_KICKED_PHRASE)
        && text_contains_word_phrase(
            text,
            PUT_THOSE_CARDS_ONTO_BATTLEFIELD_INSTEAD_OF_HAND_PHRASE,
        )
}

fn text_mentions_clash_win_top_replacement(text: &str) -> bool {
    text_contains_word_phrase(text, CLASH_WITH_AN_OPPONENT_PHRASE)
        && text_contains_word_phrase(text, IF_YOU_WIN_PHRASE)
        && text_contains_word_phrase(text, ON_TOP_OF_OWNERS_LIBRARY_INSTEAD_PHRASE)
}

fn text_starts_with_self_x_counter_etb(text: &str) -> bool {
    text_starts_with_any_word_phrase(text, SELF_ENTERS_WITH_X_PLUS_ONE_COUNTER_PREFIXES)
}

fn text_starts_with_this_spell_cost(text: &str) -> bool {
    text_starts_with_any_word_phrase(text, THIS_SPELL_COST_PREFIXES)
}

fn text_starts_with_if(text: &str) -> bool {
    text_starts_with_any_word_phrase(text, &[&["if"]])
}

fn text_starts_with_exhaust(text: &str) -> bool {
    text_starts_with_any_word_phrase(text, &[EXHAUST_PREFIX])
}

fn text_mentions_add_x_mana(text: &str) -> bool {
    text_contains_word_phrase(text, ADD_X_MANA_PHRASE)
}

fn text_mentions_where_x_is(text: &str) -> bool {
    text_contains_word_phrase(text, WHERE_X_IS_PHRASE)
}

fn text_mentions_any_player_activate_on_stack(text: &str) -> bool {
    text_contains_word_phrase(text, ANY_PLAYER_MAY_ACTIVATE_THIS_ABILITY_PHRASE)
        && text_contains_word_phrase(text, ON_THE_STACK_PHRASE)
}

fn text_mentions_exile_self_from_graveyard(text: &str) -> bool {
    text_contains_any_word_phrase(text, EXILE_SELF_FROM_GRAVEYARD_PHRASES)
}

fn text_starts_with_this_ability_costs(text: &str) -> bool {
    text_starts_with_any_word_phrase(text, &[THIS_ABILITY_COSTS_PREFIX])
}

fn text_is_unqualified_cant_be_blocked(text: &str) -> bool {
    lex_line(text.trim_end_matches('.'), 0)
        .ok()
        .is_some_and(|tokens| {
            CANT_BE_BLOCKED_SUFFIXES
                .iter()
                .any(|suffix| token_slice_ends_with(&tokens, suffix))
                && !token_slice_starts_with(&tokens, &["this"])
                && !token_slice_starts_with(&tokens, &["it"])
        })
}

fn text_mentions_full_party_instead(text: &str) -> bool {
    text_contains_word_phrase(text, IF_YOU_HAVE_FULL_PARTY_PHRASE)
        && text_contains_word_phrase(text, UNTIL_END_OF_TURN_INSTEAD_PHRASE)
}

fn text_mentions_if_you_do(text: &str) -> bool {
    text_contains_word_phrase(text, IF_YOU_DO_PHRASE)
}

fn text_mentions_if_you_dont(text: &str) -> bool {
    text_contains_any_word_phrase(text, IF_YOU_DONT_PHRASES)
}

fn text_mentions_when_it_enters(text: &str) -> bool {
    text_contains_word_phrase(text, WHEN_IT_ENTERS_PHRASE)
}

fn text_starts_with_remove(text: &str) -> bool {
    text_starts_with_any_word_phrase(text, &[REMOVE_PREFIX])
}

fn tokens_start_with_level_up(tokens: &[OwnedLexToken]) -> bool {
    token_slice_starts_with(tokens, LEVEL_UP_PREFIX)
}

fn tokens_match_each_player_and_their_creatures_damage(tokens: &[OwnedLexToken]) -> bool {
    DAMAGE_TO_EACH_PLAYER_CREATURES_PHRASES
        .iter()
        .any(|phrase| contains_token_word_sequence(tokens, phrase))
}

fn text_matches_blocks_or_blocked_first_strike(text: &str) -> bool {
    lex_line(text.trim_end_matches('.'), 0)
        .ok()
        .is_some_and(|tokens| {
            token_slice_starts_with(&tokens, BLOCKS_OR_BECOMES_BLOCKED_PREFIX)
                && token_slice_ends_with(&tokens, THAT_CREATURE_FIRST_STRIKE_SUFFIX)
        })
}

fn level_number_from_text(text: &str) -> Option<u32> {
    let tokens = lex_line(text, 0).ok()?;
    if !token_slice_starts_with(&tokens, &["level"]) {
        return None;
    }
    tokens.get(1)?.parser_text.parse::<u32>().ok()
}

fn text_is_first_equip_cost_alternative_line(text: &str) -> bool {
    lex_line(text.trim_end_matches('.'), 0)
        .ok()
        .is_some_and(|tokens| {
            token_slice_starts_with(&tokens, YOU_MAY_PAY_PREFIX)
                && contains_token_word_sequence(&tokens, FIRST_EQUIP_COST_ALTERNATIVE_PHRASE)
                && FIRST_EQUIP_COST_ALTERNATIVE_SUFFIXES
                    .iter()
                    .any(|suffix| token_slice_ends_with(&tokens, suffix))
        })
}

fn text_starts_with_partner_with(text: &str) -> bool {
    lex_line(text, 0)
        .ok()
        .is_some_and(|tokens| token_slice_starts_with(&tokens, PARTNER_WITH_PREFIX))
}

fn tokens_start_with_partner_dash_label(tokens: &[OwnedLexToken]) -> bool {
    tokens.first().is_some_and(|token| token.is_word("partner"))
        && tokens
            .get(1)
            .is_some_and(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
}

fn tokens_mention_day_night_starts_day(tokens: &[OwnedLexToken]) -> bool {
    contains_token_word_sequence(tokens, NEITHER_DAY_NOR_NIGHT_PHRASE)
        && contains_token_word_sequence(tokens, BECOMES_DAY_PHRASE)
        && AS_THIS_ENTERS_PHRASES
            .iter()
            .any(|phrase| contains_token_word_sequence(tokens, phrase))
}
