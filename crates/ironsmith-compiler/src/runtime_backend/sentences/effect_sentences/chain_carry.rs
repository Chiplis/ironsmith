#![allow(dead_code)]

use winnow::Parser;
use winnow::combinator::{alt, repeat};
use winnow::error::{ContextError, ErrMode};

use super::super::compile_support::effects_reference_it_tag;
use super::super::effect_ast_traversal::for_each_nested_effects_mut;
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::{
    LeadingResultPrefixKind, split_leading_result_prefix_lexed, split_trailing_if_clause_lexed,
};
use super::super::lexer::{
    OwnedLexToken, TokenKind, token_slice_at_is, token_slice_first_is, token_word_refs,
    trim_lexed_commas, word_slice_starts_with,
};
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::{
    PermissionClauseSpec, PermissionLifetime, parse_additional_land_plays_clause_lexed,
    parse_cast_or_play_tagged_clause, parse_permission_clause_spec_lexed,
    parse_unsupported_play_cast_permission_clause_lexed,
};
use super::super::rule_engine::{LexClauseView, LexRuleDef, LexRuleIndex};
use super::super::token_primitives::{
    rfind_index as find_last_token_index, str_contains as string_contains,
    str_contains_char as string_contains_char,
};
use super::super::util::{
    is_source_reference_words, remove_first_word as remove_first_word_tokens,
    remove_through_first_word as remove_through_first_word_tokens, strip_leading_word_refs_any,
};
use super::super::value_helpers::{parse_number_from_lexed, parse_value_from_lexed};
use super::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::dispatch_inner::parse_subject_verb_extension_sentence;
use super::lex_chain_helpers::{
    find_verb_lexed, has_effect_head_without_verb_lexed, segment_has_effect_head_lexed,
    split_effect_chain_on_and_lexed, split_segments_on_comma_effect_head_lexed,
    split_segments_on_comma_then_lexed, strip_leading_instead_prefix_lexed,
};
use super::sentence_helpers::*;
use super::{
    parse_cant_effect_sentence_lexed, parse_effect_clause_lexed, parse_effect_sentence_lexed,
    parse_predicate_lexed, parse_search_library_sentence_lexed,
    parse_sentence_exile_source_with_counters_lexed,
    parse_sentence_put_onto_battlefield_with_counters_on_it_lexed,
    parse_sentence_return_with_counters_on_it_lexed, parse_simple_gain_ability_clause_lexed,
    parse_simple_lose_ability_clause_lexed, parse_token_copy_followup_sentence_lexed,
    try_apply_token_copy_followup,
};

const ENCHANTED_TAG_NAME: &str = "enchanted";
const SENTENCE_HELPER_REVEALED_TAG_PREFIX: &str = "__sentence_helper_revealed";
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, PlayerAst, PredicateAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbSubjectAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::types::Subtype;
use crate::zone::Zone;

const EACH_OPPONENT_PREFIXES: &[&[&str]] = &[&["each", "opponent"], &["each", "opponents"]];
const EACH_PLAYER_PREFIXES: &[&[&str]] = &[&["each", "player"], &["each", "players"]];
const UNTIL_YOUR_NEXT_TURN_PREFIXES: &[&[&str]] = &[
    &["until", "your", "next", "turn"],
    &["until", "your", "next", "upkeep"],
];
const UNTIL_YOUR_NEXT_UNTAP_PREFIXES: &[&[&str]] = &[
    &["until", "your", "next", "untap", "step"],
    &["during", "your", "next", "untap", "step"],
];
const CHAIN_CHOOSE_BASIC_LAND_TYPE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["choose", "a", "land", "of", "each", "basic", "land", "type"],
            &["choose", "land", "of", "each", "basic", "land", "type"],
        ]
);
const CHAIN_YOU_CHOOSE_BASIC_LAND_TYPE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "you", "choose", "a", "land", "of", "each", "basic", "land", "type",
            ],
            &[
                "you", "choose", "land", "of", "each", "basic", "land", "type"
            ],
        ]
);
const CHAIN_TOKEN_OR_TOKENS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["token", "tokens"]]);
const X_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["x"]);
const THEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["then"]);
const LIBRARY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["library"]);
const GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["graveyard"], &["graveyards"]]);
const INTO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["into"]);
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const WHEN_WHENEVER_AT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["when"], &["whenever"], &["at"]]);
const TAP_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["tap"]);
const UNTAP_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["untap"]);
const ALL_OR_EACH_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["all"], &["each"]]);
const GAIN_OR_GAINS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["gain"], &["gains"]]);
const DRAW_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["draw"]);
const CHAIN_OWNER_YOUR_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["your"]);
const CHAIN_OWNER_TARGET_PLAYER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["target", "player"], &["target", "players"]]);
const CHAIN_OWNER_TARGET_OPPONENT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["target", "opponent"], &["target", "opponents"]]);
const CHAIN_FACE_DOWN_SHUFFLE_FROM_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["face", "down", "then", "shuffle", "all", "cards", "from"]);
const CHAIN_EXILE_THEM_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["exile", "them"]);
const CHAIN_THEN_MELD_THEM_INTO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["then", "meld", "them", "into"]);
const CHAIN_HAVE_OR_HAS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["have"], &["has"]]);
const CHAIN_TAP_ALL_OR_EACH_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["tap", "all"], &["tap", "each"]]);
const CHAIN_OR_UNTAP_ALL_EACH_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["or", "untap", "all"], &["or", "untap", "each"]]]);
const CHAIN_UNTIL_EOT_TRIGGER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["until", "end", "of", "turn"],
            &["until", "the", "end", "of", "turn"]
        ]
);
const CHAIN_WOULD_ENTER_INSTEAD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["would", "instead"]; contains_any_words & [&["enter", "enters"]]);
const CHAIN_OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const CHAIN_AND_OR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["or"]]);
const CHAIN_COMPARISON_OR_TAIL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["less"], &["greater"], &["more"], &["fewer"]]);
const CHAIN_THAN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["than"]);
const CHAIN_EQUAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["equal"]);
const CHAIN_TARGET_WITH_CARD_TYPE_WINDOW_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["target"]);
const CHAIN_ALL_ABILITIES_AND_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["all", "abilities", "and"]);
const CHAIN_ROUNDED_UP_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["rounded", "up"]);
const CHAIN_END_OF_COMBAT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["end", "of", "combat"]]);
const CHAIN_BEGINNING_END_STEP_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["beginning", "of", "the", "end", "step"],
            &["beginning", "of", "next", "end", "step"],
            &["beginning", "of", "the", "next", "end", "step"],
        ]]
);

fn chain_find_phrase_start(words: &[&str], shape: &ClauseShape<'static>) -> Option<usize> {
    (0..words.len()).find(|idx| shape.matches_words(&words[*idx..]))
}

fn chain_find_word(words: &[&str], shape: &ClauseShape<'static>) -> Option<usize> {
    words.iter().position(|word| shape.matches_word(word))
}

fn chain_has_card_type_or_card_type_window(words: &[&str]) -> bool {
    words.windows(3).any(|window| {
        matches!(
            window,
            [
                "artifact"
                    | "battle"
                    | "creature"
                    | "enchantment"
                    | "instant"
                    | "land"
                    | "planeswalker"
                    | "sorcery",
                "or",
                "artifact"
                    | "battle"
                    | "creature"
                    | "enchantment"
                    | "instant"
                    | "land"
                    | "planeswalker"
                    | "sorcery"
            ]
        )
    })
}

fn synthetic_lexed_word(word: &str) -> OwnedLexToken {
    OwnedLexToken::word(word, TextSpan::synthetic())
}

fn parse_choose_land_of_each_basic_land_type_segment(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    let choice_words = if CHAIN_CHOOSE_BASIC_LAND_TYPE_PATTERN.matches_words(&words) {
        words.as_slice()
    } else if CHAIN_YOU_CHOOSE_BASIC_LAND_TYPE_PATTERN.matches_words(&words) {
        &words[1..]
    } else {
        return None;
    };
    if choice_words.is_empty() {
        return None;
    }

    let basic_land_types = [
        Subtype::Plains,
        Subtype::Island,
        Subtype::Swamp,
        Subtype::Mountain,
        Subtype::Forest,
    ];
    Some(
        basic_land_types
            .into_iter()
            .map(|subtype| {
                let mut filter = ObjectFilter::land().with_subtype(subtype);
                filter.controller = Some(PlayerFilter::Any);
                EffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::Implicit,
                    tag: TagKey::from(crate::cards::builders::IT_TAG),
                }
            })
            .collect(),
    )
}

#[derive(Clone, Copy)]
enum RestAction {
    Destroy,
    Exile,
    Sacrifice,
}

const REST_ACTION_SEGMENT_PHRASES: &[(&[&str], RestAction)] = &[
    (&["destroy", "the", "rest"], RestAction::Destroy),
    (&["destroy", "rest"], RestAction::Destroy),
    (&["exile", "the", "rest"], RestAction::Exile),
    (&["exile", "rest"], RestAction::Exile),
    (&["sacrifice", "the", "rest"], RestAction::Sacrifice),
    (&["sacrifice", "rest"], RestAction::Sacrifice),
    (&["sacrifices", "the", "rest"], RestAction::Sacrifice),
    (&["sacrifices", "rest"], RestAction::Sacrifice),
];

fn parse_rest_action_segment_lexed(tokens: &[OwnedLexToken]) -> Option<RestAction> {
    let words = token_word_refs(tokens);
    let words = if THEN_WORD_PATTERN.matches_first_word(&words) {
        &words[1..]
    } else {
        words.as_slice()
    };
    REST_ACTION_SEGMENT_PHRASES
        .iter()
        .find_map(|(phrase, action)| (*phrase == words).then_some(*action))
}

fn rest_action_effect(action: RestAction, filter: ObjectFilter, player: PlayerAst) -> EffectAst {
    match action {
        RestAction::Destroy => EffectAst::subject_verb_destroy_all(filter),
        RestAction::Exile => EffectAst::subject_verb_exile_all(filter, false),
        RestAction::Sacrifice => EffectAst::subject_verb_sacrifice_all(player, filter),
    }
}

fn try_apply_rest_action_followup(effects: &mut Vec<EffectAst>, action: RestAction) -> bool {
    if let Some(EffectAst::ChooseObjects {
        filter,
        tag,
        player,
        ..
    }) = effects.last()
    {
        let rest_filter = filter.clone().not_tagged(tag.clone());
        let player = *player;
        effects.push(rest_action_effect(action, rest_filter, player));
        return true;
    }

    let Some(last) = effects.last_mut() else {
        return false;
    };
    match last {
        EffectAst::ForEachPlayer {
            effects: inner_effects,
        }
        | EffectAst::ForEachOpponent {
            effects: inner_effects,
        } => {
            let Some(EffectAst::ChooseObjects {
                filter,
                tag,
                player,
                ..
            }) = inner_effects.last()
            else {
                return false;
            };
            let rest_filter = filter.clone().not_tagged(tag.clone());
            let player = *player;
            inner_effects.push(rest_action_effect(action, rest_filter, player));
            true
        }
        _ => false,
    }
}

fn contains_char(text: &str, expected: char) -> bool {
    string_contains_char(text, expected)
}

fn starts_like_create_fragment_lexed(tokens: &[OwnedLexToken]) -> bool {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    let Some(first_word) = words.first().copied() else {
        return false;
    };
    let starts_like_count = parse_number_from_lexed(tokens).is_some()
        || contains_char(first_word, '/')
        || X_WORD_PATTERN.matches_word(first_word);
    starts_like_count && CHAIN_TOKEN_OR_TOKENS_PATTERN.matches_words(&words)
}

pub(super) fn parse_effect_chain_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_effect_chain_lexed(view.tokens).map(Some)
}

pub(super) const FALLBACK_POST_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 1] =
    [LexRuleDef {
        id: "effect-chain",
        priority: 170,
        heads: &[],
        shape_mask: 0,
        run: parse_effect_chain_rule_lexed,
    }];

pub(super) const FALLBACK_POST_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&FALLBACK_POST_DIAGNOSTIC_RULES_LEXED);

fn parse_exile_library_then_shuffle_graveyard_chain_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn parse_owner(words: &[&str]) -> Option<(PlayerFilter, PlayerAst)> {
        let words = crate::runtime_backend::util::possessive_normalized_word_refs(words);
        if CHAIN_OWNER_YOUR_PATTERN.matches_words(&words) {
            Some((PlayerFilter::You, PlayerAst::You))
        } else if CHAIN_OWNER_TARGET_PLAYER_PATTERN.matches_words(&words) {
            Some((PlayerFilter::target_player(), PlayerAst::Target))
        } else if CHAIN_OWNER_TARGET_OPPONENT_PATTERN.matches_words(&words) {
            Some((PlayerFilter::target_opponent(), PlayerAst::TargetOpponent))
        } else {
            None
        }
    }

    let clause_tokens = trim_lexed_commas(tokens);
    if grammar::words_match_prefix(clause_tokens, &["exile", "all", "cards", "from"]).is_none() {
        return Ok(None);
    }

    let clause_words = token_word_refs(clause_tokens);
    let Some(library_idx) = chain_find_word(&clause_words, &LIBRARY_WORD_PATTERN) else {
        return Ok(None);
    };
    if library_idx <= 4 {
        return Ok(None);
    }

    let (owner_filter, owner_player) = match parse_owner(&clause_words[4..library_idx]) {
        Some(owner) => owner,
        None => return Ok(None),
    };
    if !CHAIN_FACE_DOWN_SHUFFLE_FROM_PATTERN.matches_words(&clause_words[library_idx + 1..]) {
        return Ok(None);
    }

    let graveyard_tail = &clause_words[library_idx + 8..];
    let Some(graveyard_idx) =
        chain_find_word(graveyard_tail, &GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN)
    else {
        return Ok(None);
    };
    let Some((graveyard_owner_filter, _graveyard_owner_player)) =
        parse_owner(&graveyard_tail[..graveyard_idx])
    else {
        return Ok(None);
    };
    if graveyard_owner_filter != owner_filter {
        return Ok(None);
    }
    let library_tail = &graveyard_tail[graveyard_idx + 1..];
    if !INTO_WORD_PATTERN.matches_first_word(library_tail)
        || !library_tail
            .last()
            .is_some_and(|word| LIBRARY_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    let Some((destination_owner_filter, _destination_owner_player)) =
        parse_owner(&library_tail[1..library_tail.len() - 1])
    else {
        return Ok(None);
    };
    if destination_owner_filter != owner_filter {
        return Ok(None);
    }

    let mut filter = crate::target::ObjectFilter::default().in_zone(Zone::Library);
    filter.owner = Some(owner_filter.clone());
    Ok(Some(vec![
        EffectAst::subject_verb_exile_all(filter, true),
        EffectAst::subject_verb_shuffle_graveyard_into_library(owner_player),
    ]))
}

pub(crate) fn looks_like_multi_create_chain_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches!(find_verb_lexed(tokens), Some((Verb::Create, _)))
        && token_word_refs(tokens)
            .iter()
            .filter(|word| CHAIN_TOKEN_OR_TOKENS_PATTERN.matches_word(word))
            .count()
            >= 2
}

pub(crate) fn parse_effect_chain_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn immediate_tagged_permission_spec(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
        Ok(matches!(
            parse_permission_clause_spec_lexed(tokens)?,
            Some(PermissionClauseSpec::Tagged {
                lifetime: PermissionLifetime::Immediate,
                ..
            })
        ))
    }

    if let Some(effects) = parse_exile_library_then_shuffle_graveyard_chain_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) =
        super::dispatch_inner::parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(tokens)
    {
        return Ok(effects);
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if CHAIN_EXILE_THEM_PREFIX_PATTERN.matches_words(&clause_words)
        && let Some(meld_idx) =
            chain_find_phrase_start(&clause_words, &CHAIN_THEN_MELD_THEM_INTO_PREFIX_PATTERN)
    {
        let result_words = &clause_words[meld_idx + 4..];
        if result_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing meld result name (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(vec![EffectAst::subject_verb_meld(
            result_words.join(" "),
            false,
            false,
        )]);
    }

    if let Some(stripped) = strip_leading_instead_prefix_lexed(tokens) {
        return parse_effect_chain_lexed(stripped);
    }
    let starts_with_each_opponent =
        grammar::words_match_any_prefix(tokens, EACH_OPPONENT_PREFIXES).is_some();
    let starts_with_each_player =
        grammar::words_match_any_prefix(tokens, EACH_PLAYER_PREFIXES).is_some();

    if let Some(player) = parse_leading_player_may_lexed(tokens) {
        let mut stripped = remove_through_first_word(tokens, "may");
        if stripped
            .first()
            .is_some_and(|token| CHAIN_HAVE_OR_HAS_WORD_PATTERN.matches_token(token))
        {
            stripped.remove(0);
        }
        if word_slice_starts_with(&token_word_refs(&stripped), &["choose", "to"]) {
            stripped = remove_through_first_word_tokens(&stripped, "to");
        }
        let mut effects = parse_effect_chain_lexed(&stripped)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        if leading_may_is_permission_clause_lexed(&stripped)? {
            if immediate_tagged_permission_spec(&stripped)? {
                return Ok(vec![EffectAst::MayByPlayer { player, effects }]);
            }
            return Ok(effects);
        }
        return Ok(vec![EffectAst::MayByPlayer { player, effects }]);
    }

    if token_slice_first_is(tokens, "may") && !starts_with_each_opponent && !starts_with_each_player
    {
        let stripped = remove_first_word(tokens, "may");
        let effects = parse_effect_chain_lexed(&stripped)?;
        if leading_may_is_permission_clause_lexed(&stripped)? {
            if immediate_tagged_permission_spec(&stripped)? {
                return Ok(vec![EffectAst::May { effects }]);
            }
            return Ok(effects);
        }
        return Ok(vec![EffectAst::May { effects }]);
    }

    if CHAIN_TAP_ALL_OR_EACH_PREFIX_PATTERN.matches_words(&clause_words)
        && CHAIN_OR_UNTAP_ALL_EACH_PATTERN.matches_words(&clause_words)
    {
        return parse_effect_chain_with_subject_verb_primitives_lexed(tokens);
    }

    if let Some(unless_action) = parse_or_action_clause_lexed(tokens)? {
        return Ok(vec![unless_action]);
    }

    if clause_may_contain_cast_or_play_permission_lexed(tokens)
        && let Some(effect) = parse_cast_or_play_tagged_clause(tokens)?
    {
        return Ok(vec![effect]);
    }

    parse_effect_chain_with_subject_verb_primitives_lexed(tokens)
}

fn clause_may_contain_cast_or_play_permission_lexed(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .any(|word| {
            matches!(
                word,
                "may" | "cast" | "casts" | "casting" | "play" | "plays" | "playing" | "played"
            )
        })
}

fn leading_may_is_permission_clause_lexed(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    Ok(parse_additional_land_plays_clause_lexed(tokens)?.is_some()
        || parse_permission_clause_spec_lexed(tokens)?.is_some()
        || parse_unsupported_play_cast_permission_clause_lexed(tokens)?.is_some())
}

fn starts_with_until_end_of_turn_trigger_clause(clause_words: &[&str]) -> bool {
    CHAIN_UNTIL_EOT_TRIGGER_PREFIX_PATTERN.matches_words(clause_words)
        && WHEN_WHENEVER_AT_WORD_PATTERN.matches_word_at(
            clause_words,
            if THE_WORD_PATTERN.matches_word_at(clause_words, 1) {
                5
            } else {
                4
            },
        )
}

fn is_would_enter_replacement_clause(clause_words: &[&str]) -> bool {
    CHAIN_WOULD_ENTER_INSTEAD_PATTERN.matches_words(clause_words)
}

fn is_comparison_or_delimiter_lexed(tokens: &[OwnedLexToken], idx: usize) -> bool {
    if !token_slice_at_is(tokens, idx, "or") {
        return false;
    }
    let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
    let next_word = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
    if next_word.is_some_and(|word| CHAIN_COMPARISON_OR_TAIL_WORD_PATTERN.matches_word(word)) {
        return true;
    }
    previous_word.is_some_and(|word| CHAIN_THAN_WORD_PATTERN.matches_word(word))
        && next_word.is_some_and(|word| CHAIN_EQUAL_WORD_PATTERN.matches_word(word))
}

fn action_separator_indices_lexed(tokens: &[OwnedLexToken]) -> Vec<usize> {
    fn is_card_type_word(word: &str) -> bool {
        matches!(
            word,
            "artifact"
                | "battle"
                | "creature"
                | "enchantment"
                | "instant"
                | "land"
                | "planeswalker"
                | "sorcery"
        )
    }

    let mut inside_quotes = false;
    let mut indices = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes {
            continue;
        }
        if CHAIN_OR_WORD_PATTERN.matches_token(token)
            && tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(is_card_type_word)
            && (tokens
                .get(idx.saturating_sub(1))
                .and_then(OwnedLexToken::as_word)
                .is_some_and(is_card_type_word)
                || tokens
                    .get(idx.saturating_sub(1))
                    .is_some_and(|token| matches!(token.kind, TokenKind::Comma)))
        {
            continue;
        }
        if matches!(token.kind, TokenKind::Comma) {
            let before = &tokens[..idx];
            let after = trim_lexed_commas(&tokens[idx + 1..]);
            let after_words = token_word_refs(after);
            if grammar::contains_word(before, "target")
                && (after_words
                    .first()
                    .is_some_and(|word| is_card_type_word(word))
                    || (CHAIN_OR_WORD_PATTERN.matches_first_word(&after_words)
                        && after_words
                            .get(1)
                            .is_some_and(|word| is_card_type_word(word))))
            {
                continue;
            }
        }
        let is_separator = token.kind == TokenKind::Comma
            || (CHAIN_OR_WORD_PATTERN.matches_token(token)
                && !is_comparison_or_delimiter_lexed(tokens, idx));
        if is_separator {
            indices.push(idx);
        }
    }
    indices
}

fn normalize_or_action_option_lexed(mut option: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while option
        .first()
        .is_some_and(|token| CHAIN_AND_OR_WORD_PATTERN.matches_token(token))
    {
        option = &option[1..];
    }
    trim_lexed_commas(option)
}

pub(crate) fn parse_or_action_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if !grammar::contains_word(tokens, "or") {
        return Ok(None);
    }
    let words = token_word_refs(tokens);
    if CHAIN_TARGET_WITH_CARD_TYPE_WINDOW_PATTERN.matches_words(&words)
        && chain_has_card_type_or_card_type_window(&words)
    {
        return Ok(None);
    }

    for separator_idx in action_separator_indices_lexed(tokens) {
        let first = normalize_or_action_option_lexed(&tokens[..separator_idx]);
        let second = normalize_or_action_option_lexed(&tokens[separator_idx + 1..]);
        if first.is_empty() || second.is_empty() {
            continue;
        }

        let first_words = crate::runtime_backend::token_word_refs(first);
        let second_words = crate::runtime_backend::token_word_refs(second);
        if TAP_WORD_PATTERN.matches_first_word(&first_words)
            && UNTAP_WORD_PATTERN.matches_first_word(&second_words)
            && ALL_OR_EACH_WORD_PATTERN.matches_word_at(&first_words, 1)
            && ALL_OR_EACH_WORD_PATTERN.matches_word_at(&second_words, 1)
        {
            continue;
        }

        let first_starts_effect = find_verb_lexed(first).is_some_and(|(_, verb_idx)| verb_idx == 0)
            || has_effect_head_without_verb_lexed(first);
        let second_starts_effect = find_verb_lexed(second)
            .is_some_and(|(_, verb_idx)| verb_idx == 0)
            || has_effect_head_without_verb_lexed(second);
        if !first_starts_effect || !second_starts_effect {
            continue;
        }

        let first_effects = match parse_effect_chain_with_subject_verb_primitives_lexed(first) {
            Ok(effects) if !effects.is_empty() => effects,
            _ => continue,
        };
        let second_effects = match parse_effect_chain_with_subject_verb_primitives_lexed(second) {
            Ok(effects) if !effects.is_empty() => effects,
            _ => continue,
        };

        return Ok(Some(EffectAst::UnlessAction {
            effects: first_effects,
            alternative: second_effects,
            player: PlayerAst::Implicit,
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::{
        CardDefinitionBuilder, EffectAst, PlayerAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    };
    use crate::ids::CardId;

    use super::super::super::lexer::lex_line;
    use super::{
        parse_effect_chain_lexed, parse_leading_player_may_lexed, starts_like_create_fragment_lexed,
    };

    #[test]
    fn leading_may_land_play_permission_does_not_lower_to_may_effect() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
            .parse_text("You may play an additional land this turn.\nDraw a card.")
            .expect("explore-style text should parse");

        let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
        assert!(
            super::string_contains(&spell_debug, "AdditionalLandPlaysEffect")
                || super::string_contains(&spell_debug, "additional_land_plays"),
            "expected Explore-style permission text to lower to additional land plays, got {spell_debug}"
        );
    }

    #[test]
    fn create_fragment_probe_accepts_capitalized_pt_token_clauses() {
        let tokens = lex_line("Two 1/1 white Soldier creature tokens", 0)
            .expect("rewrite lexer should classify create-fragment text");

        assert!(starts_like_create_fragment_lexed(&tokens));
    }

    #[test]
    fn parses_target_card_type_list_with_lte_mana_value_reference() {
        let tokens = lex_line(
            "Exile target enchantment, instant, or sorcery card with equal or lesser mana value than that spell from an opponent's graveyard",
            0,
        )
        .expect("target list clause should lex");

        parse_effect_chain_lexed(&tokens).expect("target list clause should parse");
    }

    #[test]
    fn leading_player_may_probe_accepts_capitalized_opponent_clauses() {
        let tokens = lex_line("An opponent may cast it", 0)
            .expect("rewrite lexer should classify player-may text");

        assert_eq!(
            parse_leading_player_may_lexed(&tokens),
            Some(PlayerAst::Opponent)
        );
    }

    #[test]
    fn leading_player_may_probe_accepts_then_target_player_clauses() {
        let tokens = lex_line("Then target player may draw a card", 0)
            .expect("rewrite lexer should classify target-player may text");

        assert_eq!(
            parse_leading_player_may_lexed(&tokens),
            Some(PlayerAst::Target)
        );
    }

    #[test]
    fn leading_player_may_probe_accepts_possessive_controller_clauses() {
        let tokens = lex_line("That creature's controller may cast it", 0)
            .expect("rewrite lexer should classify possessive controller text");

        assert_eq!(
            parse_leading_player_may_lexed(&tokens),
            Some(PlayerAst::ItsController)
        );
    }

    #[test]
    fn leading_player_may_probe_accepts_that_attacking_player_clauses() {
        let tokens = lex_line("That attacking player may create a tapped Zombie token", 0)
            .expect("rewrite lexer should classify attacking-player may text");

        assert_eq!(
            parse_leading_player_may_lexed(&tokens),
            Some(PlayerAst::Attacking)
        );
    }

    #[test]
    fn leading_player_may_probe_accepts_that_player_or_target_controller_clauses() {
        let tokens = lex_line(
            "That player or that permanent's controller may draw a card",
            0,
        )
        .expect("rewrite lexer should classify split controller text");

        assert_eq!(
            parse_leading_player_may_lexed(&tokens),
            Some(PlayerAst::ThatPlayerOrTargetController)
        );
    }

    #[test]
    fn top_cards_then_put_counted_into_hand_rest_graveyard_chain_parses() {
        let tokens = lex_line(
            "Look at the top three cards of your library, then put one of them into your hand and the rest into your graveyard",
            0,
        )
        .expect("looked-cards split clause should lex");

        let effects =
            parse_effect_chain_lexed(&tokens).expect("looked-cards split clause should parse");

        match effects.as_slice() {
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::LookAtTopCards { .. },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject,
                    action: SubjectVerbActionAst::PutSomeIntoHandRestIntoGraveyard { count },
                }),
            ] => {
                assert_eq!(subject.player, PlayerAst::You);
                assert_eq!(*count, crate::effect::ChoiceCount::exactly(1));
            }
            other => panic!("expected looked-cards split effects, got {other:?}"),
        }
    }

    #[test]
    fn exile_then_shuffle_graveyard_chain_keeps_both_effects() {
        let tokens = lex_line(
            "Exile all cards from your library face down, then shuffle all cards from your graveyard into your library.",
            0,
        )
        .expect("rewrite lexer should classify exile-then-shuffle text");
        let effects = parse_effect_chain_lexed(&tokens).expect("chain should parse");
        let debug = format!("{effects:?}");

        assert!(
            debug.contains("ExileAll")
                && debug.contains("face_down: true")
                && debug.contains("ShuffleGraveyardIntoLibrary"),
            "expected exile-all face-down and graveyard shuffle effects, got {debug}"
        );
        assert!(
            effects.iter().any(|effect| matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ExileAll {
                        face_down: true,
                        ..
                    },
                    ..
                })
            )),
            "expected a face-down exile-all effect in the parsed chain: {debug}"
        );
        assert!(
            effects.iter().any(|effect| {
                matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::ShuffleGraveyardIntoLibrary,
                        ..
                    })
                )
            }),
            "expected a graveyard shuffle effect in the parsed chain: {debug}"
        );
    }

    #[test]
    fn or_action_clause_preserves_secondary_or_inside_sacrifice_filter() {
        let tokens = lex_line(
            "Discard two cards or sacrifice a creature or planeswalker of your choice",
            0,
        )
        .expect("or-action text should lex");

        let parsed = super::parse_or_action_clause_lexed(&tokens)
            .expect("or-action parse should succeed")
            .expect("or-action clause should be recognized");

        let debug = format!("{parsed:?}");
        assert!(
            debug.contains("UnlessAction"),
            "expected or-action lowering to use unless-action AST, got {debug}"
        );
        assert!(
            debug.contains("Discard"),
            "expected discard branch in or-action AST, got {debug}"
        );
        assert!(
            debug.contains("Sacrifice"),
            "expected sacrifice branch in or-action AST, got {debug}"
        );
        assert!(
            debug.contains("Planeswalker"),
            "expected sacrifice filter to keep planeswalker branch, got {debug}"
        );
    }
}

pub(crate) fn parse_effect_chain_with_subject_verb_primitives_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if token_slice_first_is(tokens, "and") {
        return parse_effect_chain_with_subject_verb_primitives_lexed(&tokens[1..]);
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if starts_with_until_end_of_turn_trigger_clause(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported until-end-of-turn permission clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_would_enter_replacement_clause(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported would-enter replacement clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(effects) = run_subject_verb_primitives_lexed(
        tokens,
        PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
        &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
    )? {
        return Ok(effects);
    }
    if let Some(effects) = parse_subject_verb_extension_sentence(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = run_subject_verb_primitives_lexed(
        tokens,
        POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
        &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
    )? {
        return Ok(effects);
    }
    parse_effect_chain_inner_lexed(tokens)
}

pub(crate) fn parse_effect_chain_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(stripped) = strip_leading_instead_prefix_lexed(tokens) {
        return parse_effect_chain_inner_lexed(stripped);
    }

    if let Some(effects) = parse_search_library_sentence_lexed(tokens)? {
        return Ok(effects);
    }

    let mut effects = Vec::new();
    let raw_segments = split_effect_chain_on_and_lexed(tokens);
    let mut lexed_segments = Vec::new();
    for segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        lexed_segments.push(segment);
    }

    let mut merged_lexed_segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    for lexed_segment in lexed_segments {
        let segment = lexed_segment.to_vec();
        if merged_lexed_segments.is_empty() {
            merged_lexed_segments.push(segment);
            continue;
        }
        if !super::lex_chain_helpers::segment_has_effect_head_lexed(&segment) {
            if let Some(previous) = merged_lexed_segments.last()
                && let Some(expanded) = expand_missing_verb_segment_lexed(previous, &segment)
            {
                merged_lexed_segments.push(expanded);
                continue;
            }
            let last = merged_lexed_segments
                .last_mut()
                .expect("non-empty segments");
            last.push(synthetic_lexed_word("and"));
            last.extend(segment);
            continue;
        }
        merged_lexed_segments.push(segment);
    }
    while merged_lexed_segments.len() > 1
        && !super::lex_chain_helpers::segment_has_effect_head_lexed(&merged_lexed_segments[0])
    {
        let mut first = merged_lexed_segments.remove(0);
        first.push(synthetic_lexed_word("and"));
        let mut next = merged_lexed_segments.remove(0);
        first.append(&mut next);
        merged_lexed_segments.insert(0, first);
    }
    let merged_segment_slices = merged_lexed_segments
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let mut segments: Vec<Vec<OwnedLexToken>> = split_segments_on_comma_effect_head_lexed(
        split_segments_on_comma_then_lexed(merged_segment_slices),
    )
    .into_iter()
    .map(|segment| segment.to_vec())
    .collect();
    segments = expand_segments_with_comma_action_clauses_lexed(segments);
    segments = expand_segments_with_multi_create_clauses_lexed(segments);
    let mut carried_context: Option<CarryContext> = None;
    let leading_duration = leading_duration_for_followup_carry(tokens);
    let mut carried_duration: Option<Until> = leading_duration.clone();
    let mut previous_segment: Option<Vec<OwnedLexToken>> = None;
    for segment in segments {
        let mut segment = segment;
        if is_orphan_rounded_up_where_x_tail(&segment, previous_segment.as_deref(), effects.last())
        {
            continue;
        }
        if let Some(previous) = &previous_segment
            && let Some(expanded) = expand_gain_lose_followup_segment_lexed(previous, &segment)
        {
            segment = expanded;
        }

        let carry_gain_duration = find_verb_lexed(&segment).is_some_and(|(verb, verb_idx)| {
            verb_idx == 0 && matches!(verb, Verb::Gain | Verb::Lose)
        });
        let carry_leading_duration = leading_duration.is_some();
        let segment_effects =
            if let Some(effects) = parse_sentence_return_with_counters_on_it_lexed(&segment)? {
                Some(effects)
            } else if let Some(effects) =
                parse_sentence_put_onto_battlefield_with_counters_on_it_lexed(&segment)?
            {
                Some(effects)
            } else if let Some(prefix) = split_leading_result_prefix_lexed(&segment) {
                Some(vec![match prefix.kind {
                    LeadingResultPrefixKind::If => EffectAst::IfResult {
                        predicate: prefix.predicate,
                        effects: parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
                    },
                    LeadingResultPrefixKind::When => EffectAst::WhenResult {
                        predicate: prefix.predicate,
                        effects: parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
                    },
                }])
            } else {
                parse_sentence_exile_source_with_counters_lexed(&segment)?
            };
        if let Some(segment_effects) = segment_effects {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(effect);
            }
            continue;
        }
        if let Some(segment_effects) = parse_search_library_sentence_lexed(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(effect);
            }
            continue;
        }
        if let Some(segment_effects) = parse_cant_effect_sentence_lexed(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(effect);
            }
            continue;
        }
        if let Some(segment_effects) = parse_subject_verb_extension_sentence(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(effect);
            }
            previous_segment = Some(segment);
            continue;
        }
        let primitive_segment_effects = if let Some(effects) = run_subject_verb_primitives_lexed(
            &segment,
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )? {
            Some(effects)
        } else {
            run_subject_verb_primitives_lexed(
                &segment,
                POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
                &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
            )?
        };
        if let Some(segment_effects) = primitive_segment_effects {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(effect);
            }
            previous_segment = Some(segment);
            continue;
        }
        if let Some(followup) = parse_token_copy_followup_sentence_lexed(&segment)
            && try_apply_token_copy_followup(&mut effects, followup)?
        {
            continue;
        }
        if let Some(segment_effects) = parse_choose_land_of_each_basic_land_type_segment(&segment) {
            effects.extend(segment_effects);
            previous_segment = Some(segment);
            continue;
        }
        if let Some(action) = parse_rest_action_segment_lexed(&segment)
            && try_apply_rest_action_followup(&mut effects, action)
        {
            previous_segment = Some(segment);
            continue;
        }
        let segment_words = token_word_refs(&segment);
        if CHAIN_ALL_ABILITIES_AND_PATTERN.matches_words(&segment_words)
            && GAIN_OR_GAINS_WORD_PATTERN.matches_word_at(&segment_words, 3)
        {
            let Some(gain_idx) =
                crate::runtime_backend::lexer::find_token_any_word(&segment, &["gain", "gains"])
            else {
                continue;
            };
            let mut gain_tokens = Vec::new();
            gain_tokens.push(synthetic_lexed_word("it"));
            gain_tokens.extend(segment[gain_idx..].iter().cloned());
            if let Some(mut effect) = parse_simple_gain_ability_clause_lexed(&gain_tokens)? {
                if let Some(duration) = &carried_duration {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                effects.push(effect);
                previous_segment = Some(segment);
                continue;
            }
        }
        let mut effect = parse_effect_clause_with_trailing_if_lexed(&segment)?;
        if let Some(context) = carried_context {
            maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
        }
        if (carry_gain_duration || carry_leading_duration)
            && let Some(duration) = &carried_duration
        {
            apply_carried_effect_duration(&mut effect, duration);
        }
        if let Some(context) = explicit_player_for_carry(&effect) {
            carried_context = Some(context);
        }
        if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
            carried_duration = Some(duration);
        }
        effects.push(effect);
        previous_segment = Some(segment);
    }
    collapse_for_each_player_it_tag_followups(&mut effects);
    collapse_for_each_object_it_tag_followups(&mut effects);
    collapse_token_copy_next_end_step_exile_followup_lexed(&mut effects, tokens);
    collapse_token_copy_next_end_step_sacrifice_followup_lexed(&mut effects, tokens);
    collapse_token_copy_end_of_combat_exile_followup_lexed(&mut effects, tokens);
    Ok(effects)
}

fn is_orphan_rounded_up_where_x_tail(
    segment: &[OwnedLexToken],
    previous: Option<&[OwnedLexToken]>,
    previous_effect: Option<&EffectAst>,
) -> bool {
    let segment_words = token_word_refs(segment);
    if !CHAIN_ROUNDED_UP_PATTERN.matches_words(&segment_words) {
        return false;
    }
    if previous.is_none() && previous_effect.is_none() {
        return true;
    }
    previous.is_some_and(|previous| {
        grammar::words_find_phrase(previous, &["where", "x", "is", "half"]).is_some()
    }) || previous_effect.is_some_and(effect_uses_half_life_total_value)
}

fn effect_uses_half_life_total_value(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenWithMods {
                    dynamic_power_toughness,
                    ..
                },
            ..
        }) => dynamic_power_toughness
            .as_ref()
            .is_some_and(|(power, toughness)| {
                value_is_half_life_total(power) || value_is_half_life_total(toughness)
            }),
        EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::WhenResult { effects, .. }
        | EffectAst::ManaRestricted { effects, .. } => {
            effects.iter().any(effect_uses_half_life_total_value)
        }
        _ => false,
    }
}

fn value_is_half_life_total(value: &Value) -> bool {
    matches!(value.unhinted(), Value::HalfLifeTotalRoundedUp(_))
}

fn leading_duration_for_followup_carry(tokens: &[OwnedLexToken]) -> Option<Until> {
    let words = token_word_refs(tokens);
    if crate::runtime_backend::util::starts_with_until_end_of_turn(&words) {
        return Some(Until::EndOfTurn);
    }
    if grammar::words_match_any_prefix(tokens, UNTIL_YOUR_NEXT_TURN_PREFIXES).is_some() {
        return Some(Until::YourNextTurn);
    }
    if grammar::words_match_any_prefix(tokens, UNTIL_YOUR_NEXT_UNTAP_PREFIXES).is_some() {
        return Some(Until::ControllersNextUntapStep);
    }
    None
}

fn effect_duration_for_gain_followup_carry(effect: &EffectAst) -> Option<Until> {
    let duration = match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GainControl { duration, .. }
                | SubjectVerbActionAst::Pump { duration, .. }
                | SubjectVerbActionAst::PumpAll { duration, .. }
                | SubjectVerbActionAst::SetBasePowerToughness { duration, .. }
                | SubjectVerbActionAst::SetBasePower { duration, .. }
                | SubjectVerbActionAst::BecomeBasePtCreature { duration, .. }
                | SubjectVerbActionAst::AddCardTypes { duration, .. }
                | SubjectVerbActionAst::RemoveCardTypes { duration, .. }
                | SubjectVerbActionAst::AddSubtypes { duration, .. }
                | SubjectVerbActionAst::AddColors { duration, .. }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily { duration, .. }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { duration, .. }
                | SubjectVerbActionAst::SetColors { duration, .. }
                | SubjectVerbActionAst::MakeColorless { duration, .. }
                | SubjectVerbActionAst::BecomeBasicLandType { duration, .. }
                | SubjectVerbActionAst::BecomeBasicLandTypeChoice { duration, .. }
                | SubjectVerbActionAst::BecomeColorChoice { duration, .. }
                | SubjectVerbActionAst::BecomeCreatureTypeChoice { duration, .. }
                | SubjectVerbActionAst::BecomeCopy { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesToTarget { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesAll { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { duration, .. }
                | SubjectVerbActionAst::RemoveAbilitiesFromTarget { duration, .. }
                | SubjectVerbActionAst::RemoveAbilitiesAll { duration, .. },
            ..
        }) => duration,
        _ => return None,
    };

    if matches!(duration, Until::Forever) {
        None
    } else {
        Some(duration.clone())
    }
}

fn apply_carried_effect_duration(effect: &mut EffectAst, duration: &Until) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GainControl {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::Pump {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PumpAll {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetBasePowerToughness {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetBasePower {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasePtCreature {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddSubtypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::MakeColorless {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasicLandType {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasicLandTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeColorChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCreatureTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCopy {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesToTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesAll {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAbilitiesFromTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAbilitiesAll {
                    duration: effect_duration,
                    ..
                },
            ..
        }) if matches!(effect_duration, Until::Forever) => {
            *effect_duration = duration.clone();
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            for nested in if_true.iter_mut().chain(if_false.iter_mut()) {
                apply_carried_effect_duration(nested, duration);
            }
        }
        _ => {}
    }
}

pub(crate) fn collapse_for_each_player_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::ForEachPlayer { .. },
                EffectAst::ForEachPlayer {
                    effects: followup_effects,
                },
            ) => effects_reference_it_tag(followup_effects),
            _ => false,
        };

        if !should_merge {
            idx += 1;
            continue;
        }

        let followup = effects.remove(idx + 1);
        match (&mut effects[idx], followup) {
            (
                EffectAst::ForEachPlayer {
                    effects: first_effects,
                },
                EffectAst::ForEachPlayer {
                    effects: mut followup_effects,
                },
            ) => {
                first_effects.append(&mut followup_effects);
            }
            _ => {
                // Defensive: should be unreachable given should_merge checks.
            }
        }
        // Re-check this index in case we have a longer chain of followups.
    }
}

pub(crate) fn collapse_for_each_object_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (EffectAst::ForEachObject { filter, .. }, followup) => {
                effects_reference_it_tag(std::slice::from_ref(followup))
                    || (for_each_revealed_this_way_filter(filter)
                        && is_revealed_this_way_scalar_reward(followup))
            }
            _ => false,
        };

        if !should_merge {
            idx += 1;
            continue;
        }

        let followup = effects.remove(idx + 1);
        match (&mut effects[idx], followup) {
            (EffectAst::ForEachObject { effects: inner, .. }, followup) => {
                inner.push(followup);
            }
            _ => {
                // Defensive: should be unreachable given should_merge checks.
            }
        }
        // Re-check this index in case we have a longer chain of followups.
    }
}

fn for_each_revealed_this_way_filter(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && (constraint.tag.as_str() == IT_TAG
                || sentence_helper_revealed_tag(constraint.tag.as_str()))
    })
}

fn sentence_helper_revealed_tag(tag: &str) -> bool {
    tag.starts_with(SENTENCE_HELPER_REVEALED_TAG_PREFIX)
}

fn is_revealed_this_way_scalar_reward(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. },
            ..
        })
    )
}

pub(crate) fn parse_effect_clause_with_trailing_if_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) else {
        return parse_effect_clause_lexed(tokens);
    };
    let predicate = trailing_if.predicate;
    if !trailing_if_predicate_supported(&predicate) {
        return parse_effect_clause_lexed(tokens);
    }

    let base_effect = if let Ok(effect) = parse_effect_clause_lexed(trailing_if.leading_tokens) {
        effect
    } else {
        if let Some(effect) = parse_simple_lose_ability_clause_lexed(trailing_if.leading_tokens)? {
            effect
        } else if let Some(effect) =
            parse_simple_gain_ability_clause_lexed(trailing_if.leading_tokens)?
        {
            effect
        } else {
            return parse_effect_clause_lexed(tokens);
        }
    };

    Ok(EffectAst::Conditional {
        predicate,
        if_true: vec![base_effect],
        if_false: Vec::new(),
    })
}

fn trailing_if_predicate_supported(predicate: &PredicateAst) -> bool {
    matches!(
        predicate,
        PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
            | PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(_)
            | PredicateAst::ItMatches(_)
            | PredicateAst::TargetMatches(_)
            | PredicateAst::PlayerControlsMoreThanYou { .. }
            | PredicateAst::PlayerControls { .. }
            | PredicateAst::PlayerControlsAtLeast { .. }
            | PredicateAst::PlayerControlsExactly { .. }
            | PredicateAst::PlayerControlsAtLeastWithDifferentPowers { .. }
            | PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { .. }
            | PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { .. }
            | PredicateAst::PlayerHasMoreLifeThanYou { .. }
            | PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { .. }
            | PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { .. }
            | PredicateAst::PlayerIsMonarch { .. }
            | PredicateAst::PlayerHasInitiative { .. }
            | PredicateAst::PlayerHasCitysBlessing { .. }
            | PredicateAst::PlayerHasMoreCardsInHandThanYou { .. }
            | PredicateAst::PlayerHasCardTypesInGraveyardOrMore { .. }
    ) || matches!(predicate, PredicateAst::TaggedMatches(tag, _) if tag.as_str() == ENCHANTED_TAG_NAME)
}

pub(crate) fn is_beginning_of_end_step_words(words: &[&str]) -> bool {
    CHAIN_BEGINNING_END_STEP_PATTERN.matches_words(words)
}

pub(crate) fn is_end_of_combat_words(words: &[&str]) -> bool {
    CHAIN_END_OF_COMBAT_PATTERN.matches_words(words)
}

pub(crate) fn target_is_generic_token_filter(target: &TargetAst) -> bool {
    let TargetAst::Object(filter, _, _) = target else {
        return false;
    };
    filter.token
        && filter.zone.is_none()
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.tagged_constraints.is_empty()
        && filter.controller.is_none()
        && filter.owner.is_none()
}

pub(crate) fn collapse_token_copy_next_end_step_exile_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let chain_words = token_word_refs(tokens);
    if !grammar::contains_word(tokens, "exile")
        || !grammar::contains_word(tokens, "token")
        || !is_beginning_of_end_step_words(&chain_words)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. },
                    ..
                }),
                EffectAst::SubjectVerb(subject_verb),
            ) => match &subject_verb.action {
                SubjectVerbActionAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                }
                | SubjectVerbActionAst::Exile { target, .. } => {
                    target_is_generic_token_filter(target)
                }
                _ => false,
            },
            _ => false,
        };

        if !mark_next_end_step_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy {
                        exile_at_next_end_step,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource {
                        exile_at_next_end_step,
                        ..
                    },
                ..
            }) => {
                *exile_at_next_end_step = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub(crate) fn collapse_token_copy_next_end_step_sacrifice_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let chain_words = token_word_refs(tokens);
    if !grammar::contains_word(tokens, "sacrifice")
        || !grammar::contains_word(tokens, "token")
        || grammar::words_find_phrase(tokens, &["next", "end", "step", "repeat"]).is_none()
            && !is_beginning_of_end_step_words(&chain_words)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_sacrifice = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Sacrifice { filter, count, .. },
                    ..
                }),
            ) => *count == 1 && filter.token,
            _ => false,
        };

        if !mark_next_end_step_sacrifice {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy {
                        sacrifice_at_next_end_step,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource {
                        sacrifice_at_next_end_step,
                        ..
                    },
                ..
            }) => {
                *sacrifice_at_next_end_step = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub(crate) fn collapse_token_copy_end_of_combat_exile_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let chain_words = token_word_refs(tokens);
    if !grammar::contains_word(tokens, "exile")
        || !grammar::contains_word(tokens, "token")
        || !is_end_of_combat_words(&chain_words)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_end_of_combat_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. }
                        | SubjectVerbActionAst::CreateTokenWithMods { .. },
                    ..
                }),
                EffectAst::SubjectVerb(subject_verb),
            ) => match &subject_verb.action {
                SubjectVerbActionAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                }
                | SubjectVerbActionAst::Exile { target, .. } => {
                    target_is_generic_token_filter(target)
                }
                _ => false,
            },
            _ => false,
        };

        if !mark_end_of_combat_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy {
                        exile_at_end_of_combat,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource {
                        exile_at_end_of_combat,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenWithMods {
                        exile_at_end_of_combat,
                        ..
                    },
                ..
            }) => {
                *exile_at_end_of_combat = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

fn split_on_comma_or_semicolon_lexed(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut inside_quotes = false;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes || !matches!(token.kind, TokenKind::Comma | TokenKind::Semicolon) {
            continue;
        }
        let current = trim_lexed_commas(&tokens[start..idx]);
        if !current.is_empty() {
            segments.push(current.to_vec());
        }
        start = idx + 1;
    }
    let tail = trim_lexed_commas(&tokens[start..]);
    if !tail.is_empty() {
        segments.push(tail.to_vec());
    }
    segments
}

pub(crate) fn expand_segments_with_comma_action_clauses_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let looks_like_sac_discard_chain = (grammar::contains_word(&segment, "sacrifice")
            || grammar::contains_word(&segment, "sacrifices"))
            && (grammar::contains_word(&segment, "discard")
                || grammar::contains_word(&segment, "discards"));
        if !looks_like_sac_discard_chain {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon_lexed(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<OwnedLexToken>> = Vec::new();
        let mut valid_split = true;

        for raw_part in comma_parts {
            let mut part = trim_lexed_commas(&raw_part).to_vec();
            while token_slice_first_is(&part, "and") {
                part.remove(0);
            }
            if part.is_empty() {
                continue;
            }

            if segment_has_effect_head_lexed(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment_lexed(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }

            valid_split = false;
            break;
        }

        if valid_split && local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub(crate) fn expand_segments_with_multi_create_clauses_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let Some((Verb::Create, _)) = find_verb_lexed(&segment) else {
            expanded.push(segment);
            continue;
        };
        let has_token_rules_tail = grammar::words_find_phrase(&segment, &["when", "this", "token"])
            .is_some()
            || grammar::words_find_phrase(&segment, &["whenever", "this", "token"]).is_some()
            || grammar::words_find_phrase(&segment, &["this", "token"]).is_some()
            || grammar::words_find_phrase(&segment, &["that", "token"]).is_some()
            || grammar::words_find_phrase(&segment, &["those", "tokens"]).is_some()
            || grammar::words_find_phrase(&segment, &["it", "has"]).is_some()
            || grammar::words_find_phrase(&segment, &["they", "have"]).is_some();
        if has_token_rules_tail {
            expanded.push(segment);
            continue;
        }
        let segment_words = token_word_refs(&segment);
        let token_mentions = segment_words
            .iter()
            .filter(|word| CHAIN_TOKEN_OR_TOKENS_PATTERN.matches_word(word))
            .count();
        if token_mentions < 2 {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon_lexed(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<OwnedLexToken>> = Vec::new();
        for raw_part in comma_parts {
            let mut part = trim_lexed_commas(&raw_part).to_vec();
            while token_slice_first_is(&part, "and") {
                part.remove(0);
            }
            if part.is_empty() {
                continue;
            }
            if let Some(previous) = local_parts.last()
                && is_token_creation_context(previous)
                && starts_with_inline_token_rules_tail(&part)
            {
                if let Some(last) = local_parts.last_mut() {
                    last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                    last.extend(part);
                }
                continue;
            }
            if segment_has_effect_head_lexed(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment_lexed(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }
            if let Some(last) = local_parts.last_mut() {
                last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                last.extend(part);
            } else {
                local_parts.push(part);
            }
        }

        if local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub(crate) fn expand_missing_verb_segment_lexed(
    previous: &[OwnedLexToken],
    segment: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let (verb, verb_idx) = find_verb_lexed(previous)?;
    match verb {
        Verb::Deal => {
            if parse_value_from_lexed(segment).is_none()
                || !grammar::contains_word(segment, "damage")
            {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Sacrifice => {
            let segment_words = token_word_refs(segment);
            let starts_like_object_phrase = matches!(
                segment_words.first().copied(),
                Some("a" | "an" | "another" | "target")
            ) || parse_number_from_lexed(segment).is_some();
            if !starts_like_object_phrase {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Create => {
            if !starts_like_create_fragment_lexed(segment) {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        _ => None,
    }
}

fn strip_leading_gain_duration_prefix(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let words = token_word_refs(tokens);
    if crate::runtime_backend::util::starts_with_until_end_of_turn(&words) {
        return trim_lexed_commas(&tokens[4..]);
    }
    if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, UNTIL_YOUR_NEXT_TURN_PREFIXES)
    {
        return trim_lexed_commas(&tokens[prefix.len()..]);
    }
    if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, UNTIL_YOUR_NEXT_UNTAP_PREFIXES)
    {
        return trim_lexed_commas(&tokens[prefix.len()..]);
    }
    trim_lexed_commas(tokens)
}

fn previous_segment_has_carryable_subject(previous: &[OwnedLexToken]) -> bool {
    let Some((_, verb_idx)) = find_verb_lexed(previous) else {
        return false;
    };
    if verb_idx == 0 {
        return false;
    }

    let prefix = trim_lexed_commas(&previous[..verb_idx]);
    let subject_tokens = strip_leading_gain_duration_prefix(prefix);
    if subject_tokens.is_empty() {
        return false;
    }

    let subject_words = token_word_refs(subject_tokens);
    is_source_reference_words(&subject_words)
        || starts_with_target_indicator(&subject_tokens)
        || parse_object_filter(subject_tokens, false).is_ok()
}

fn expand_gain_lose_followup_segment_lexed(
    previous: &[OwnedLexToken],
    segment: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let (verb, verb_idx) = find_verb_lexed(segment)?;
    if verb_idx != 0 || !matches!(verb, Verb::Gain | Verb::Lose) {
        return None;
    }
    if !previous_segment_has_carryable_subject(previous) {
        return None;
    }

    let previous_verb_idx = find_verb_lexed(previous)?.1;
    let mut expanded = Vec::new();
    expanded.extend(previous.iter().take(previous_verb_idx).cloned());
    expanded.extend(segment.iter().cloned());
    Some(expanded)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarryContext {
    Player(PlayerAst),
    ForEachPlayer,
    ForEachTargetPlayers(ChoiceCount),
    ForEachOpponent,
}

pub(crate) fn player_ast_from_filter_for_carry(filter: &PlayerFilter) -> Option<PlayerAst> {
    match filter {
        PlayerFilter::You => Some(PlayerAst::You),
        PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        PlayerFilter::Any => Some(PlayerAst::Any),
        PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        PlayerFilter::Target(inner) => {
            if matches!(inner.as_ref(), PlayerFilter::Opponent) {
                Some(PlayerAst::TargetOpponent)
            } else {
                Some(PlayerAst::Target)
            }
        }
        _ => None,
    }
}

pub(crate) fn player_owner_filter_from_target_for_carry(target: &TargetAst) -> Option<PlayerAst> {
    match target {
        TargetAst::Player(filter, _) => player_ast_from_filter_for_carry(filter),
        TargetAst::Object(filter, _, _) => {
            if !matches!(
                filter.zone,
                Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
            ) {
                return None;
            }
            filter
                .owner
                .as_ref()
                .and_then(player_ast_from_filter_for_carry)
        }
        TargetAst::WithCount(inner, _) => player_owner_filter_from_target_for_carry(inner),
        _ => None,
    }
}

fn player_target_carry_context(target: &TargetAst) -> Option<CarryContext> {
    match target {
        TargetAst::Player(filter, _) => {
            player_ast_from_filter_for_carry(filter).map(CarryContext::Player)
        }
        TargetAst::WithCount(inner, count) => {
            let inner_context = player_target_carry_context(inner.as_ref())?;
            if count.min > 1 && count.max == Some(count.min) {
                Some(CarryContext::ForEachTargetPlayers(*count))
            } else {
                Some(inner_context)
            }
        }
        _ => None,
    }
}

pub(crate) fn explicit_player_for_carry(effect: &EffectAst) -> Option<CarryContext> {
    if matches!(effect, EffectAst::ForEachPlayer { .. }) {
        return Some(CarryContext::ForEachPlayer);
    }
    if let EffectAst::ForEachTargetPlayers { count, .. } = effect {
        return Some(CarryContext::ForEachTargetPlayers(*count));
    }
    if matches!(effect, EffectAst::ForEachOpponent { .. }) {
        return Some(CarryContext::ForEachOpponent);
    }
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::TargetOnly { target } = &subject_verb.action
        && let Some(context) = player_target_carry_context(target)
    {
        return Some(context);
    }
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::Exile { target, .. } = &subject_verb.action
        && let Some(player) = player_owner_filter_from_target_for_carry(target)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. },
        ..
    }) = effect
        && let Some(player) = player_owner_filter_from_target_for_carry(target)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ExileAll { filter, .. },
        ..
    }) = effect
        && let Some(owner) = filter.owner.as_ref()
        && let Some(player) = player_ast_from_filter_for_carry(owner)
    {
        return Some(CarryContext::Player(player));
    }
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ChoosePlayer { .. },
            ..
        })
    ) {
        return Some(CarryContext::Player(PlayerAst::That));
    }

    let player = match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => {
            if !matches!(player, PlayerAst::Implicit) {
                *player
            } else if !matches!(chooser, PlayerAst::Implicit) {
                *chooser
            } else {
                return None;
            }
        }
        EffectAst::SubjectVerb(_) => subject_verb_player_action_player(effect)?,
        EffectAst::ChooseObjects { player, .. } => *player,
        _ => return None,
    };

    if matches!(player, PlayerAst::Implicit) {
        None
    } else {
        Some(CarryContext::Player(player))
    }
}

pub(crate) fn effect_uses_implicit_player(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => matches!(*chooser, PlayerAst::Implicit) || matches!(*player, PlayerAst::Implicit),
        EffectAst::SubjectVerb(_) => {
            matches!(
                subject_verb_player_action_player(effect),
                Some(PlayerAst::Implicit)
            )
        }
        EffectAst::ChooseObjects { player, .. } => {
            matches!(*player, PlayerAst::Implicit)
        }
        _ => false,
    }
}

fn subject_verb_player_action_player_mut(effect: &mut EffectAst) -> Option<&mut PlayerAst> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenCopy { player, .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. }
                | SubjectVerbActionAst::CreateTokenWithMods { player, .. },
            ..
        }) => Some(player),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action:
                SubjectVerbActionAst::Draw { .. }
                | SubjectVerbActionAst::LoseLife { .. }
                | SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::RevealHand
                | SubjectVerbActionAst::RevealTop
                | SubjectVerbActionAst::RevealCardsFromHand { .. }
                | SubjectVerbActionAst::Mill { .. }
                | SubjectVerbActionAst::Scry { .. }
                | SubjectVerbActionAst::Surveil { .. }
                | SubjectVerbActionAst::Discard { .. }
                | SubjectVerbActionAst::DiscardHand
                | SubjectVerbActionAst::PoisonCounters { .. }
                | SubjectVerbActionAst::EnergyCounters { .. }
                | SubjectVerbActionAst::TicketCounters { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayAnyEnergy { .. }
                | SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::DoubleManaPool
                | SubjectVerbActionAst::EmptyManaPool
                | SubjectVerbActionAst::SetLifeTotal { .. }
                | SubjectVerbActionAst::EndTurn
                | SubjectVerbActionAst::SkipTurn
                | SubjectVerbActionAst::SkipCombatPhases
                | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
                | SubjectVerbActionAst::SkipMainPhasesThisTurn
                | SubjectVerbActionAst::SkipCombatPhasesThisTurn
                | SubjectVerbActionAst::SkipDrawStep
                | SubjectVerbActionAst::RingTemptsYou
                | SubjectVerbActionAst::VentureIntoDungeon { .. }
                | SubjectVerbActionAst::BecomeMonarch
                | SubjectVerbActionAst::TakeInitiative
                | SubjectVerbActionAst::CreateEmblem { .. }
                | SubjectVerbActionAst::LoseGame
                | SubjectVerbActionAst::WinGame
                | SubjectVerbActionAst::FlipCoin
                | SubjectVerbActionAst::RollDie { .. }
                | SubjectVerbActionAst::RollDiceChooseResult { .. }
                | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
                | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary
                | SubjectVerbActionAst::ReorderGraveyard
                | SubjectVerbActionAst::ChooseColor
                | SubjectVerbActionAst::ChooseCardType { .. }
                | SubjectVerbActionAst::ChooseNamedOption { .. }
                | SubjectVerbActionAst::ChooseCreatureType { .. }
                | SubjectVerbActionAst::ChooseCardName { .. }
                | SubjectVerbActionAst::ChoosePlayer { .. }
                | SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::PutIntoHand { .. }
                | SubjectVerbActionAst::AdditionalLandPlays { .. }
                | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
                | SubjectVerbActionAst::Sacrifice { .. }
                | SubjectVerbActionAst::ShuffleLibrary,
        }) => Some(player),
        _ => None,
    }
}

fn subject_verb_player_action_player(effect: &EffectAst) -> Option<PlayerAst> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenCopy { player, .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. }
                | SubjectVerbActionAst::CreateTokenWithMods { player, .. },
            ..
        }) => Some(*player),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action:
                SubjectVerbActionAst::Draw { .. }
                | SubjectVerbActionAst::LoseLife { .. }
                | SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::RevealHand
                | SubjectVerbActionAst::RevealTop
                | SubjectVerbActionAst::RevealCardsFromHand { .. }
                | SubjectVerbActionAst::Mill { .. }
                | SubjectVerbActionAst::Scry { .. }
                | SubjectVerbActionAst::Surveil { .. }
                | SubjectVerbActionAst::Discard { .. }
                | SubjectVerbActionAst::DiscardHand
                | SubjectVerbActionAst::PoisonCounters { .. }
                | SubjectVerbActionAst::EnergyCounters { .. }
                | SubjectVerbActionAst::TicketCounters { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayAnyEnergy { .. }
                | SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::DoubleManaPool
                | SubjectVerbActionAst::EmptyManaPool
                | SubjectVerbActionAst::SetLifeTotal { .. }
                | SubjectVerbActionAst::EndTurn
                | SubjectVerbActionAst::SkipTurn
                | SubjectVerbActionAst::SkipCombatPhases
                | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
                | SubjectVerbActionAst::SkipMainPhasesThisTurn
                | SubjectVerbActionAst::SkipCombatPhasesThisTurn
                | SubjectVerbActionAst::SkipDrawStep
                | SubjectVerbActionAst::RingTemptsYou
                | SubjectVerbActionAst::VentureIntoDungeon { .. }
                | SubjectVerbActionAst::BecomeMonarch
                | SubjectVerbActionAst::TakeInitiative
                | SubjectVerbActionAst::CreateEmblem { .. }
                | SubjectVerbActionAst::LoseGame
                | SubjectVerbActionAst::WinGame
                | SubjectVerbActionAst::FlipCoin
                | SubjectVerbActionAst::RollDie { .. }
                | SubjectVerbActionAst::RollDiceChooseResult { .. }
                | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
                | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary
                | SubjectVerbActionAst::ReorderGraveyard
                | SubjectVerbActionAst::ChooseColor
                | SubjectVerbActionAst::ChooseCardType { .. }
                | SubjectVerbActionAst::ChooseNamedOption { .. }
                | SubjectVerbActionAst::ChooseCreatureType { .. }
                | SubjectVerbActionAst::ChooseCardName { .. }
                | SubjectVerbActionAst::ChoosePlayer { .. }
                | SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::PutIntoHand { .. }
                | SubjectVerbActionAst::AdditionalLandPlays { .. }
                | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
                | SubjectVerbActionAst::Sacrifice { .. }
                | SubjectVerbActionAst::ShuffleLibrary,
        }) => Some(*player),
        _ => None,
    }
}

pub(crate) fn maybe_apply_carried_player(effect: &mut EffectAst, carried_context: CarryContext) {
    match carried_context {
        CarryContext::Player(carried_player) => {
            // When carrying an explicit target player/opponent into an implicit clause,
            // bind to the previously selected target ("that player") instead of creating
            // a fresh explicit target. This preserves shared-target semantics for chains
            // like "Target player mills..., draws..., and loses...".
            let carried_player = match carried_player {
                PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
                other => other,
            };
            match effect {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::SearchLibrary {
                            chooser, player, ..
                        },
                    ..
                }) => {
                    if matches!(*chooser, PlayerAst::Implicit) {
                        *chooser = carried_player;
                    }
                    if matches!(*player, PlayerAst::Implicit) {
                        *player = carried_player;
                    }
                }
                EffectAst::SubjectVerb(_) => {
                    if let Some(player) = subject_verb_player_action_player_mut(effect)
                        && *player == PlayerAst::Implicit
                    {
                        *player = carried_player;
                    }
                }
                EffectAst::ChooseObjects { player, .. } => {
                    if matches!(*player, PlayerAst::Implicit) {
                        *player = carried_player;
                    }
                }
                _ => {}
            }
        }
        CarryContext::ForEachPlayer => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachPlayer {
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachTargetPlayers(count) => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachTargetPlayers {
                    count,
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachOpponent => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachOpponent {
                    effects: vec![wrapped],
                };
            }
        }
    }
}

pub(crate) fn clause_words_for_carry_lexed(tokens: &[OwnedLexToken]) -> Vec<&str> {
    let clause_words = token_word_refs(tokens);
    strip_leading_word_refs_any(&clause_words, &["then", "and"]).to_vec()
}

pub(crate) fn maybe_apply_carried_player_with_clause_lexed(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[OwnedLexToken],
) {
    let clause_words = clause_words_for_carry_lexed(clause_tokens);
    let should_skip = match carried_context {
        CarryContext::Player(_) => {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::Implicit,
                        ..
                    },
                    action: SubjectVerbActionAst::Draw { .. },
                })
            ) && DRAW_WORD_PATTERN.matches_first_word(&clause_words)
        }
        CarryContext::ForEachPlayer
        | CarryContext::ForEachTargetPlayers(_)
        | CarryContext::ForEachOpponent => {
            let is_implicit_vision_effect = matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::Implicit,
                        ..
                    },
                    action: SubjectVerbActionAst::Draw { .. }
                        | SubjectVerbActionAst::Scry { .. }
                        | SubjectVerbActionAst::Surveil { .. },
                })
            );
            is_implicit_vision_effect
                && matches!(
                    clause_words.first().copied(),
                    Some("draw" | "scry" | "surveil")
                )
        }
    };
    if should_skip {
        return;
    }
    maybe_apply_carried_player(effect, carried_context);
}

pub(crate) fn bind_implicit_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::RetargetStackObject { .. },
        }) => {
            if matches!(subject.player, PlayerAst::Implicit) {
                subject.player = player;
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CopySpell {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::CopySpellForEachTarget {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::CastTagged {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player: effect_player,
                    ..
                },
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    player: effect_player,
                    ..
                },
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        EffectAst::SubjectVerb(_) => {
            if let Some(effect_player) = subject_verb_player_action_player_mut(effect)
                && matches!(*effect_player, PlayerAst::Implicit)
            {
                *effect_player = player;
            }
        }
        EffectAst::ChooseObjects {
            player: effect_player,
            ..
        }
        | EffectAst::ChooseObjectsAcrossZones {
            player: effect_player,
            ..
        } => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                bind_implicit_player_context(nested_effect, player);
            }
        }),
    }
}

fn parse_leading_player_may_words(words: &[&str]) -> Option<PlayerAst> {
    type WordInput<'a> = grammar::WordSliceInput<'a>;
    use grammar::word_slice_exact as word_eq;

    fn player_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((word_eq("player"), word_eq("players"))).void()
    }

    fn opponent_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((word_eq("opponent"), word_eq("opponents"))).void()
    }

    fn controller_subject_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((
            word_eq("creatures"),
            word_eq("permanents"),
            word_eq("planeswalkers"),
            word_eq("sources"),
            word_eq("spells"),
        ))
        .void()
    }

    fn controller_or_owner_subject_word<'a>()
    -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((
            word_eq("creatures"),
            word_eq("permanents"),
            word_eq("sources"),
            word_eq("spells"),
        ))
        .void()
    }

    fn leading_conjunctions<'a>(input: &mut WordInput<'a>) -> Result<(), ErrMode<ContextError>> {
        repeat::<_, _, (), _, _>(0.., alt((word_eq("then"), word_eq("and")))).parse_next(input)
    }

    fn parse_player_may_prefix<'a>(
        input: &mut WordInput<'a>,
    ) -> Result<PlayerAst, ErrMode<ContextError>> {
        (
            leading_conjunctions,
            alt((
                alt((
                    (word_eq("you"), word_eq("may")).value(PlayerAst::You),
                    (word_eq("target"), opponent_word(), word_eq("may"))
                        .value(PlayerAst::TargetOpponent),
                    (word_eq("target"), player_word(), word_eq("may")).value(PlayerAst::Target),
                    (word_eq("that"), player_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("that"), opponent_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("they"), word_eq("may")).value(PlayerAst::That),
                    (
                        word_eq("that"),
                        word_eq("player"),
                        word_eq("or"),
                        word_eq("that"),
                        controller_subject_word(),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ThatPlayerOrTargetController),
                    (
                        word_eq("that"),
                        controller_or_owner_subject_word(),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsController),
                    (
                        word_eq("that"),
                        controller_or_owner_subject_word(),
                        word_eq("owner"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsOwner),
                )),
                alt((
                    (
                        word_eq("the"),
                        player_word(),
                        word_eq("whose"),
                        word_eq("turn"),
                        word_eq("it"),
                        word_eq("is"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::Active),
                    (word_eq("the"), player_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("defending"), word_eq("player"), word_eq("may"))
                        .value(PlayerAst::Defending),
                    alt((
                        (word_eq("attacking"), word_eq("player"), word_eq("may"))
                            .value(PlayerAst::Attacking),
                        (
                            word_eq("that"),
                            word_eq("attacking"),
                            word_eq("player"),
                            word_eq("may"),
                        )
                            .value(PlayerAst::Attacking),
                        (
                            word_eq("the"),
                            word_eq("attacking"),
                            word_eq("player"),
                            word_eq("may"),
                        )
                            .value(PlayerAst::Attacking),
                    )),
                    (
                        alt((word_eq("its"), word_eq("their"))),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsController),
                    (
                        alt((word_eq("its"), word_eq("their"))),
                        word_eq("owner"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsOwner),
                    alt((
                        (opponent_word(), word_eq("may")).value(PlayerAst::Opponent),
                        (word_eq("an"), word_eq("opponent"), word_eq("may"))
                            .value(PlayerAst::Opponent),
                    )),
                )),
            )),
        )
            .map(|(_, player)| player)
            .parse_next(input)
    }

    let mut input = words;
    parse_player_may_prefix(&mut input).ok()
}

pub(crate) fn parse_leading_player_may_lexed(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    parse_leading_player_may_words(&words)
}

pub(crate) fn find_verb(tokens: &[OwnedLexToken]) -> Option<(Verb, usize)> {
    find_verb_lexed(tokens)
}

pub(crate) fn parse_effect_chain(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_lexed(tokens)
}

pub(crate) fn parse_or_action_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_or_action_clause_lexed(tokens)
}

pub(crate) fn parse_effect_chain_with_subject_verb_primitives(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_with_subject_verb_primitives_lexed(tokens)
}

pub(crate) fn parse_effect_chain_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_inner_lexed(tokens)
}

pub(crate) fn parse_effect_clause_with_trailing_if(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_clause_with_trailing_if_lexed(tokens)
}

pub(crate) fn collapse_token_copy_next_end_step_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_next_end_step_exile_followup_lexed(effects, tokens);
}

pub(crate) fn collapse_token_copy_next_end_step_sacrifice_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_next_end_step_sacrifice_followup_lexed(effects, tokens);
}

pub(crate) fn collapse_token_copy_end_of_combat_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_end_of_combat_exile_followup_lexed(effects, tokens);
}

pub(crate) fn maybe_apply_carried_player_with_clause(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[OwnedLexToken],
) {
    maybe_apply_carried_player_with_clause_lexed(effect, carried_context, clause_tokens);
}

pub(crate) fn parse_leading_player_may(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    parse_leading_player_may_lexed(tokens)
}

pub(crate) fn remove_first_word(tokens: &[OwnedLexToken], word: &str) -> Vec<OwnedLexToken> {
    remove_first_word_tokens(tokens, word)
}

pub(crate) fn remove_through_first_word(
    tokens: &[OwnedLexToken],
    word: &str,
) -> Vec<OwnedLexToken> {
    remove_through_first_word_tokens(tokens, word)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb {
    Add,
    Move,
    Deal,
    Draw,
    Counter,
    Destroy,
    Exile,
    Untap,
    Scry,
    Discard,
    Transform,
    Convert,
    Flip,
    Roll,
    Regenerate,
    Mill,
    Get,
    Reveal,
    Look,
    Lose,
    Gain,
    Put,
    Sacrifice,
    Create,
    Investigate,
    Proliferate,
    Tap,
    Attach,
    Remove,
    Return,
    Exchange,
    Become,
    Switch,
    Skip,
    Surveil,
    Incubate,
    Shuffle,
    Reorder,
    Pay,
    Take,
    Detain,
    Goad,
    Suspect,
    End,
}
