use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::any;

use crate::cards::TextSpan;
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, PlayerAst, PredicateAst, SubjectVerbActionAst,
    SubjectVerbRoleAst,
};
use crate::effect::{Comparison, Value, ValueComparisonOperator};
use crate::target::PlayerFilter;
use ironsmith_core::ValueSurfaceHint;

use super::super::lexer::{
    LexStream, LexToken, OwnedLexToken, TokenKind, TokenWordView, contains_token_kind,
    trim_lexed_commas, word_slice_strip_prefix,
};
use super::super::util::{parse_card_type, parse_subtype_flexible};
use super::{leaf, primitives, values};

const QUOTED_GRANT_EXPLICIT_HEAD_PREFIXES: &[&[&str]] = &[&["this"], &["it"], &["all"], &["each"]];
const QUOTED_GRANT_OBJECT_FILTER_WORDS: &[&str] = &[
    "commander",
    "commanders",
    "permanent",
    "permanents",
    "spell",
    "spells",
];
#[path = "structure/trigger_shapes.rs"]
mod trigger_shapes;

fn token_matches_dynamic_word(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|word| word.eq_ignore_ascii_case(expected))
}

fn structure_token_is(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|_| token.parser_text() == expected)
}

fn structure_token_is_any(token: &OwnedLexToken, candidates: &[&str]) -> bool {
    token
        .as_word()
        .is_some_and(|_| structure_word_is_any(token.parser_text(), candidates))
}

fn find_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if structure_token_is(&tokens[idx], expected) {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn structure_token_kind_index(tokens: &[OwnedLexToken], kind: TokenKind) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if tokens[idx].kind == kind {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn structure_token_kind_rindex(tokens: &[OwnedLexToken], kind: TokenKind) -> Option<usize> {
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        if tokens[idx].kind == kind {
            return Some(idx);
        }
    }
    None
}

fn structure_word_is_any(word: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| word == *candidate)
}

fn phrase_occurs(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

fn word_phrase_occurs(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    if phrase.is_empty() {
        return true;
    }
    let words = tokens
        .iter()
        .filter_map(|token| token.as_word().map(|_| token.parser_text()))
        .collect::<Vec<_>>();
    words.windows(phrase.len()).any(|window| window == phrase)
}

fn predicate_candidate_contains_search_action(tokens: &[OwnedLexToken]) -> bool {
    phrase_occurs(tokens, &["search", "your", "library"])
}

fn one_of_phrases_occurs(
    tokens: &[OwnedLexToken],
    phrases: &'static [&'static [&'static str]],
) -> bool {
    primitives::find_prefix(tokens, || primitives::any_phrase(phrases)).is_some()
}

fn structure_word_index_any(words: &[&str], candidates: &[&str]) -> Option<usize> {
    let mut idx = 0usize;
    while idx < words.len() {
        if structure_word_is_any(words[idx], candidates) {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn structure_push_unique<T: PartialEq>(items: &mut Vec<T>, value: T) {
    let mut idx = 0usize;
    while idx < items.len() {
        if items[idx] == value {
            return;
        }
        idx += 1;
    }
    items.push(value);
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModalHeaderChooseSpec {
    pub(crate) choose_idx: usize,
    pub(crate) min: Value,
    pub(crate) max: Option<Value>,
    pub(crate) random: bool,
    pub(crate) x_clause_start: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModalHeaderFlags {
    pub(crate) commander_allows_both: bool,
    pub(crate) choose_both_control_card_types: Vec<crate::types::CardType>,
    pub(crate) choose_both_exact_life_total: Option<i32>,
    pub(crate) same_mode_more_than_once: bool,
    pub(crate) mode_must_be_unchosen: bool,
    pub(crate) mode_must_be_unchosen_this_turn: bool,
    pub(crate) distinct_player_targets_per_mode: bool,
    pub(crate) if_kicked_choose_any_number: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrailingModalGateSpec<'a> {
    pub(crate) prefix_tokens: &'a [OwnedLexToken],
    pub(crate) predicate: IfResultPredicate,
    pub(crate) remove_mode_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MetadataLineKind {
    ManaCost,
    TypeLine,
    FirstPrintedSet,
    PowerToughness,
    Loyalty,
    Defense,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MetadataLineSpec<'a> {
    pub(crate) kind: MetadataLineKind,
    pub(crate) value_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatementLineFamily {
    Emblem,
    PactNextUpkeep,
    NextTurnCantCast,
    Divvy,
    ArtRating,
    ExilePlayCostsMore,
    BidLife,
    Vote,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaticLineFamily {
    UntapAllDuringEachOtherPlayersUntapStep,
    GrantedQuotedAbility,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeadingResultPrefixKind {
    If,
    When,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LeadingResultPrefixSpec<'a> {
    pub(crate) kind: LeadingResultPrefixKind,
    pub(crate) predicate: IfResultPredicate,
    pub(crate) trailing_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrailingIfClauseSpec<'a> {
    pub(crate) leading_tokens: &'a [OwnedLexToken],
    pub(crate) predicate: PredicateAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum IfClausePredicateSpec {
    Conditional(PredicateAst),
    Result(IfResultPredicate),
}

#[derive(Debug, Clone)]
pub(crate) struct IfClauseSplitSpec {
    pub(crate) predicate: IfClausePredicateSpec,
    pub(crate) effects: Vec<EffectAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ConditionalPredicateTailSpec {
    Plain(PredicateAst),
    InsteadIf {
        base_predicate: PredicateAst,
        outer_predicate: PredicateAst,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TriggeredConditionalClauseSpec<'a> {
    pub(crate) trigger_tokens: &'a [OwnedLexToken],
    pub(crate) predicate: PredicateAst,
    pub(crate) effects_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StateTriggeredClauseSpec<'a> {
    pub(crate) trigger_tokens: &'a [OwnedLexToken],
    pub(crate) display_tokens: &'a [OwnedLexToken],
    pub(crate) predicate: PredicateAst,
    pub(crate) effects_tokens: &'a [OwnedLexToken],
}

fn is_sentence_quote(token: &LexToken) -> bool {
    token.kind == TokenKind::Quote && matches!(token.slice.as_str(), "\"" | "“" | "”")
}

fn parse_remove_mode_only_prefix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        alt((primitives::kw("you"), primitives::kw("they"))),
        alt((primitives::kw("remove"), primitives::kw("removed"))),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn split_metadata_line_lexed(tokens: &[OwnedLexToken]) -> Option<MetadataLineSpec<'_>> {
    fn match_metadata_prefix<'a>(
        tokens: &'a [OwnedLexToken],
        phrase: &'static [&'static str],
        kind: MetadataLineKind,
    ) -> Option<MetadataLineSpec<'a>> {
        let (_, value_tokens) =
            primitives::parse_prefix(tokens, (primitives::phrase(phrase), primitives::colon()))?;
        Some(MetadataLineSpec { kind, value_tokens })
    }

    match_metadata_prefix(tokens, &["mana", "cost"], MetadataLineKind::ManaCost)
        .or_else(|| match_metadata_prefix(tokens, &["type", "line"], MetadataLineKind::TypeLine))
        .or_else(|| match_metadata_prefix(tokens, &["type"], MetadataLineKind::TypeLine))
        .or_else(|| {
            match_metadata_prefix(
                tokens,
                &["first", "printed", "set"],
                MetadataLineKind::FirstPrintedSet,
            )
        })
        .or_else(|| {
            match_metadata_prefix(
                tokens,
                &["power/toughness"],
                MetadataLineKind::PowerToughness,
            )
        })
        .or_else(|| match_metadata_prefix(tokens, &["loyalty"], MetadataLineKind::Loyalty))
        .or_else(|| match_metadata_prefix(tokens, &["defense"], MetadataLineKind::Defense))
}

pub(crate) fn classify_statement_line_family_lexed(
    tokens: &[OwnedLexToken],
) -> Option<StatementLineFamily> {
    if super::effects::emblem_shapes::parse_emblem_payload_tokens(tokens).is_some() {
        return Some(StatementLineFamily::Emblem);
    }

    if (primitives::parse_prefix(tokens, primitives::phrase(&["starting", "with"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["each", "player", "votes"]))
            .is_some()
        || primitives::parse_prefix(
            tokens,
            primitives::phrase(&["each", "player", "secretly", "votes"]),
        )
        .is_some())
        && (primitives::contains_word(tokens, "vote")
            || primitives::contains_word(tokens, "votes")
            || primitives::contains_word(tokens, "voting"))
    {
        return Some(StatementLineFamily::Vote);
    }

    if super::effects::clause_dispatch_shapes::parse_copular_animation_shape(tokens).is_some() {
        return Some(StatementLineFamily::Generic);
    }

    // Quantified object subjects can contain an arbitrary descriptor between
    // the quantifier and the verb (for example, `Each non-Vampire creature
    // gets ...`).  Let the typed subject/verb grammar recognize that shape
    // instead of guessing the verb from a fixed word offset.
    if super::effects::clause_dispatch_shapes::parse_clause_subject_verb_shape(tokens).is_some_and(
        |shape| {
            matches!(
                shape.kind,
                super::effects::chain_splitting::ChainVerbKind::Get
            ) && !shape.subject_tokens.is_empty()
        },
    ) {
        return Some(StatementLineFamily::Generic);
    }

    if phrase_occurs(
        tokens,
        &["at", "the", "beginning", "of", "your", "next", "upkeep"],
    ) && phrase_occurs(tokens, &["lose", "the", "game"])
        && (phrase_occurs(tokens, &["if", "you", "dont"])
            || phrase_occurs(tokens, &["if", "you", "don't"])
            || phrase_occurs(tokens, &["if", "you", "do", "not"]))
    {
        return Some(StatementLineFamily::PactNextUpkeep);
    }

    if one_of_phrases_occurs(
        tokens,
        &[
            &["during", "that", "player's", "next", "turn"],
            &["during", "that", "players", "next", "turn"],
        ],
    ) && one_of_phrases_occurs(
        tokens,
        &[
            &["can't", "cast"],
            &["cant", "cast"],
            &["can", "not", "cast"],
        ],
    ) {
        return Some(StatementLineFamily::NextTurnCantCast);
    }

    if one_of_phrases_occurs(
        tokens,
        &[
            &["into", "two", "piles"],
            &["into", "three", "piles"],
            &["chooses", "two", "of", "those", "cards"],
            &["chooses", "one", "of", "those", "piles"],
            &["pile", "of", "your", "choice"],
            &["pile", "of", "that", "player's", "choice"],
            &["chosen", "pile"],
            &["chosen", "piles"],
        ],
    ) {
        return Some(StatementLineFamily::Divvy);
    }

    if phrase_occurs(
        tokens,
        &[
            "ask", "a", "person", "outside", "the", "game", "to", "rate", "its", "new", "art",
            "on", "a", "scale", "from", "1", "to", "5",
        ],
    ) {
        return Some(StatementLineFamily::ArtRating);
    }

    if one_of_phrases_occurs(tokens, &[&["become"], &["becomes"]])
        && phrase_occurs(tokens, &["until", "end", "of", "turn"])
    {
        return Some(StatementLineFamily::Generic);
    }

    if primitives::parse_prefix(
        tokens,
        primitives::phrase(&["after", "you", "roll", "a", "die"]),
    )
    .is_some()
        && phrase_occurs(tokens, &["you", "may", "pay"])
        && phrase_occurs(tokens, &["increase", "or", "decrease", "the", "result"])
    {
        return Some(StatementLineFamily::Generic);
    }

    let sentence_words_match = |sentence_tokens: &[OwnedLexToken], expected: &[&str]| {
        let words = TokenWordView::new(sentence_tokens);
        words.len() == expected.len() && words.slice_eq(0, expected)
    };
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if matches!(
        sentences.as_slice(),
        [first, second, third]
            if sentence_words_match(first, &["exile", "target", "nonland", "permanent"])
                && sentence_words_match(
                    second,
                    &[
                        "for", "as", "long", "as", "that", "card", "remains", "exiled", "its",
                        "owner", "may", "play", "it",
                    ],
                )
                && sentence_words_match(
                    third,
                    &[
                        "a", "spell", "cast", "by", "an", "opponent", "this", "way", "costs",
                        "2", "more", "to", "cast",
                    ],
                )
    ) {
        return Some(StatementLineFamily::ExilePlayCostsMore);
    }

    if primitives::parse_prefix(tokens, primitives::phrase(&["starting", "with", "you"])).is_some()
        && phrase_occurs(tokens, &["each", "player"])
        && primitives::contains_word(tokens, "pay")
    {
        return Some(StatementLineFamily::Generic);
    }

    if phrase_occurs(tokens, &["bid", "life"])
        && phrase_occurs(tokens, &["high", "bid"])
        && phrase_occurs(tokens, &["high", "bidder"])
    {
        return Some(StatementLineFamily::BidLife);
    }

    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let word_refs = words.iter().map(String::as_str).collect::<Vec<_>>();
    if word_refs.is_empty() {
        return None;
    }

    let starts_with_each_player_statement =
        word_slice_strip_prefix(&word_refs, &["each", "player"])
            .and_then(|rest| rest.first())
            .is_some_and(|word| is_statement_verb_word(word));
    let starts_with_each_other_player_statement =
        word_slice_strip_prefix(&word_refs, &["each", "other", "player"])
            .or_else(|| word_slice_strip_prefix(&word_refs, &["each", "other", "players"]))
            .and_then(|rest| rest.first())
            .is_some_and(|word| is_statement_verb_word(word));
    let starts_with_all_quantified_statement = word_slice_strip_prefix(&word_refs, &["all"])
        .is_some_and(|rest| rest.iter().any(|word| is_statement_verb_word(word)));
    let starts_with_quantified_target_player_statement = word_refs
        .get(1..)
        .and_then(|tail| {
            word_slice_strip_prefix(tail, &["target", "player"])
                .or_else(|| word_slice_strip_prefix(tail, &["target", "players"]))
        })
        .and_then(|rest| rest.first())
        .is_some_and(|word| is_statement_verb_word(word));
    let starts_with_this_spell_statement = word_slice_strip_prefix(&word_refs, &["this", "spell"])
        .and_then(|rest| rest.first())
        .is_some_and(|word| is_statement_verb_word(word));

    (starts_with_each_player_statement
        || starts_with_each_other_player_statement
        || starts_with_all_quantified_statement
        || starts_with_quantified_target_player_statement
        || is_statement_verb_word(word_refs[0])
        || starts_with_this_spell_statement
        || word_refs
            .get(1)
            .is_some_and(|word| is_statement_verb_word(word))
        || word_slice_strip_prefix(&word_refs, &["target"])
            .is_some_and(|rest| rest.iter().any(|word| is_statement_verb_word(word))))
    .then_some(StatementLineFamily::Generic)
}

fn is_statement_verb_word(word: &str) -> bool {
    matches!(
        word,
        "add"
            | "adds"
            | "choose"
            | "chooses"
            | "counter"
            | "counters"
            | "create"
            | "creates"
            | "deal"
            | "deals"
            | "destroy"
            | "destroys"
            | "discard"
            | "discards"
            | "draw"
            | "draws"
            | "become"
            | "becomes"
            | "bid"
            | "bids"
            | "enchant"
            | "enchants"
            | "exchange"
            | "exchanges"
            | "exile"
            | "exiles"
            | "gain"
            | "gains"
            | "get"
            | "gets"
            | "look"
            | "looks"
            | "lose"
            | "loses"
            | "mill"
            | "mills"
            | "note"
            | "notes"
            | "put"
            | "puts"
            | "return"
            | "returns"
            | "reveal"
            | "reveals"
            | "roll"
            | "rolls"
            | "sacrifice"
            | "sacrifices"
            | "search"
            | "searches"
            | "shuffle"
            | "shuffles"
            | "surveil"
            | "tap"
            | "taps"
            | "until"
            | "untap"
            | "untaps"
    )
}

fn quoted_grant_head_looks_like_object_filter(head: &[OwnedLexToken]) -> bool {
    if primitives::parse_prefix(
        head,
        primitives::any_phrase(QUOTED_GRANT_EXPLICIT_HEAD_PREFIXES),
    )
    .is_some()
    {
        return true;
    }
    let words = TokenWordView::new(head).to_word_refs();
    let Some(have_idx) = structure_word_index_any(&words, &["has", "have"]) else {
        return false;
    };
    words[..have_idx].iter().any(|word| {
        structure_word_is_any(word, QUOTED_GRANT_OBJECT_FILTER_WORDS)
            || parse_card_type(word).is_some()
            || parse_subtype_flexible(word).is_some()
    })
}

pub(crate) fn classify_static_line_family_lexed(
    tokens: &[OwnedLexToken],
) -> Option<StaticLineFamily> {
    if super::abilities::split_untap_each_other_players_untap_step_line_lexed(tokens).is_some() {
        return Some(StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep);
    }

    if one_of_phrases_occurs(
        tokens,
        &[
            &[
                "you",
                "may",
                "cast",
                "this",
                "card",
                "from",
                "your",
                "graveyard",
            ],
            &[
                "you",
                "may",
                "cast",
                "this",
                "spell",
                "from",
                "your",
                "graveyard",
            ],
            &[
                "once", "each", "turn", "you", "may", "cast", "a", "spell", "from", "the", "top",
                "of", "your", "library",
            ],
            &[
                "once", "each", "turn", "you", "may", "cast", "spells", "from", "the", "top", "of",
                "your", "library",
            ],
            &["if", "you", "would", "draw", "a", "card"],
        ],
    ) || contains_characteristic_equal_to_shape(tokens)
    {
        return Some(StaticLineFamily::Generic);
    }

    if let Some(quote_idx) = structure_token_kind_index(tokens, TokenKind::Quote) {
        let head = trim_lexed_commas(&tokens[..quote_idx]);
        if !head.is_empty()
            && !contains_token_kind(head, TokenKind::Period)
            && quoted_grant_head_looks_like_object_filter(head)
        {
            if primitives::contains_word(head, "has") || primitives::contains_word(head, "have") {
                return Some(StaticLineFamily::GrantedQuotedAbility);
            }
        }
    }

    (primitives::parse_prefix(tokens, primitives::phrase(&["this"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["enchanted"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["equipped"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["fortified"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["spells"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["creatures"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["other"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["each"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"])).is_some()
        || primitives::contains_word(tokens, "can't")
        || primitives::contains_word(tokens, "can")
        || primitives::contains_word(tokens, "has")
        || primitives::contains_word(tokens, "have")
        || phrase_occurs(tokens, &["maximum", "hand", "size"]))
    .then_some(StaticLineFamily::Generic)
}

fn contains_characteristic_equal_to_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::has_phrase(tokens, &["power", "is", "equal", "to"])
        || primitives::has_phrase(tokens, &["toughness", "is", "equal", "to"])
}

fn parse_modeled_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_predicate_with_grammar_entrypoint_lexed(tokens).ok()
}

pub(crate) fn parse_if_result_predicate(tokens: &[OwnedLexToken]) -> Option<IfResultPredicate> {
    super::modal_results::parse_if_result_predicate_lexed_tokens(tokens)
}

fn parse_sentence_segment_len<'a>(
    input: &mut LexStream<'a>,
) -> Result<usize, ErrMode<ContextError>> {
    fn quoted_period_continues_sentence(next: Option<&LexToken>) -> bool {
        match next {
            Some(token) if token.kind == TokenKind::Comma => true,
            Some(token)
                if token.kind == TokenKind::Word
                    && matches!(
                        token.parser_text(),
                        "and" | "during" | "for" | "this" | "until" | "where" | "with" | "without"
                    ) =>
            {
                true
            }
            _ => false,
        }
    }

    let initial_len = input.len();
    let mut inside_quotes = false;
    let mut last_inner_token_was_period = false;

    while let Some(token) = input.peek_token() {
        if is_sentence_quote(token) {
            primitives::quote().parse_next(input)?;
            if inside_quotes
                && last_inner_token_was_period
                && !quoted_period_continues_sentence(input.peek_token())
            {
                let consumed = initial_len - input.len();
                return Ok(consumed);
            }
            inside_quotes = !inside_quotes;
            last_inner_token_was_period = false;
            continue;
        }

        if token.kind == TokenKind::Period {
            primitives::period().parse_next(input)?;
            if inside_quotes {
                last_inner_token_was_period = true;
                continue;
            }

            let consumed = initial_len - input.len();
            return Ok(consumed.saturating_sub(1));
        }

        any.parse_next(input)?;
        last_inner_token_was_period = false;
    }

    Ok(initial_len - input.len())
}

pub(crate) fn split_lexed_sentences<'a>(tokens: &'a [OwnedLexToken]) -> Vec<&'a [OwnedLexToken]> {
    let mut segments = Vec::new();
    let mut remaining = tokens;

    while !remaining.is_empty() {
        let Some((segment_len, rest)) =
            primitives::parse_prefix(remaining, parse_sentence_segment_len)
        else {
            break;
        };

        if segment_len > 0 {
            segments.push(&remaining[..segment_len]);
        }

        if rest.len() == remaining.len() {
            break;
        }
        remaining = rest;
    }

    segments
}

pub(crate) fn find_trigger_effect_list_tail_split_lexed(
    trigger_prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
) -> Option<usize> {
    trigger_shapes::parse_trigger_effect_list_tail_split_lexed(trigger_prefix_tokens, tail_tokens)
        .map(|split| split.split_token_idx)
}

pub(crate) fn split_first_time_each_turn_trigger_suffix_lexed(
    trigger_tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<u32>) {
    trigger_shapes::parse_first_time_each_turn_trigger_suffix_lexed(trigger_tokens)
        .map(|split| (split.trigger_tokens, Some(split.limit)))
        .unwrap_or((trigger_tokens, None))
}

pub(crate) fn rewrite_attached_controller_trigger_effect_tokens_lexed(
    trigger_tokens: &[OwnedLexToken],
    effects_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    trigger_shapes::rewrite_attached_controller_effect_tokens_lexed(trigger_tokens, effects_tokens)
}
pub(crate) fn scan_modal_header_flags(tokens: &[OwnedLexToken]) -> ModalHeaderFlags {
    let mode_must_be_unchosen_this_turn = one_of_phrases_occurs(
        tokens,
        &[
            &["that", "hasnt", "been", "chosen", "this", "turn"],
            &["that", "hasn't", "been", "chosen", "this", "turn"],
            &["that", "has", "not", "been", "chosen", "this", "turn"],
        ],
    );
    let mode_must_be_unchosen = mode_must_be_unchosen_this_turn
        || one_of_phrases_occurs(
            tokens,
            &[
                &["that", "hasnt", "been", "chosen"],
                &["that", "hasn't", "been", "chosen"],
                &["that", "has", "not", "been", "chosen"],
            ],
        );

    let choose_both_control_card_types = scan_choose_both_control_card_types(tokens);
    let choose_both_exact_life_total = scan_choose_both_exact_life_total(tokens);

    ModalHeaderFlags {
        commander_allows_both: primitives::contains_word(tokens, "commander")
            && primitives::contains_word(tokens, "both"),
        choose_both_control_card_types,
        choose_both_exact_life_total,
        same_mode_more_than_once: phrase_occurs(tokens, &["same", "mode", "more", "than", "once"]),
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
        distinct_player_targets_per_mode: phrase_occurs(
            tokens,
            &["each", "mode", "must", "target", "a", "different", "player"],
        ),
        if_kicked_choose_any_number: word_phrase_occurs(
            tokens,
            &[
                "if", "this", "spell", "was", "kicked", "choose", "any", "number", "instead",
            ],
        ),
    }
}

fn scan_choose_both_control_card_types(tokens: &[OwnedLexToken]) -> Vec<crate::types::CardType> {
    if !phrase_occurs(tokens, &["you", "may", "choose", "both", "instead"]) {
        return Vec::new();
    }
    let Some(if_idx) = find_token_word(tokens, "if") else {
        return Vec::new();
    };
    let Some(control_idx) =
        find_token_word(&tokens[if_idx + 1..], "control").map(|idx| if_idx + 1 + idx)
    else {
        return Vec::new();
    };
    let Some(as_idx) =
        find_token_word(&tokens[control_idx + 1..], "as").map(|idx| control_idx + 1 + idx)
    else {
        return Vec::new();
    };
    if as_idx <= control_idx + 1 {
        return Vec::new();
    }

    let mut card_types = Vec::new();
    for token in &tokens[control_idx + 1..as_idx] {
        if token.kind != TokenKind::Word {
            continue;
        }
        if let Some(card_type) = parse_card_type(token.parser_text()) {
            structure_push_unique(&mut card_types, card_type);
        }
    }
    card_types
}

fn scan_choose_both_exact_life_total(tokens: &[OwnedLexToken]) -> Option<i32> {
    if !phrase_occurs(tokens, &["you", "may", "choose", "both", "instead"]) {
        return None;
    }

    let mut idx = 0usize;
    while idx + 4 < tokens.len() {
        if tokens[idx].is_word("you")
            && tokens[idx + 1].is_word("have")
            && tokens[idx + 2].is_word("exactly")
            && tokens[idx + 4].is_word("life")
        {
            return match tokens[idx + 3].kind {
                TokenKind::Number | TokenKind::Word => {
                    super::leaf::parse_number_i32_complete(tokens[idx + 3].parser_text()).ok()
                }
                _ => None,
            };
        }
        idx += 1;
    }

    None
}

pub(crate) fn split_leading_result_prefix_lexed<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LeadingResultPrefixSpec<'a>> {
    let trimmed = trim_lexed_commas(tokens);
    if let Some((predicate, trailing_tokens)) = split_leading_numeric_result_prefix_lexed(trimmed) {
        return Some(LeadingResultPrefixSpec {
            kind: LeadingResultPrefixKind::If,
            predicate,
            trailing_tokens,
        });
    }
    let kind = if trimmed
        .first()
        .is_some_and(|token| structure_token_is(token, "if"))
    {
        LeadingResultPrefixKind::If
    } else if trimmed
        .first()
        .is_some_and(|token| structure_token_is(token, "when"))
    {
        LeadingResultPrefixKind::When
    } else {
        return None;
    };

    let comma_idx = structure_token_kind_index(trimmed, TokenKind::Comma)?;
    if comma_idx <= 1 || comma_idx + 1 >= trimmed.len() {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&trimmed[1..comma_idx]);
    if predicate_tokens.is_empty() {
        return None;
    }
    let predicate = parse_if_result_predicate(predicate_tokens)?;

    let trailing_tokens = trim_lexed_commas(&trimmed[comma_idx + 1..]);
    if trailing_tokens.is_empty() {
        return None;
    }

    Some(LeadingResultPrefixSpec {
        kind,
        predicate,
        trailing_tokens,
    })
}

fn split_leading_numeric_result_prefix_lexed<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(IfResultPredicate, &'a [OwnedLexToken])> {
    let first = tokens.first()?;
    let pipe_idx = structure_token_kind_index(tokens, TokenKind::Pipe)?;
    if pipe_idx != 1 && pipe_idx < 3 {
        return None;
    }

    let compact_range = compact_ascii_numeric_range(first);
    let predicate = if pipe_idx == 1 {
        if let Some((min, max)) = compact_range {
            IfResultPredicate::Value(Comparison::BetweenInclusive(min, max))
        } else {
            let min = match first.kind {
                TokenKind::Number => first.parser_text().parse::<i32>().ok()?,
                _ => return None,
            };
            IfResultPredicate::Value(Comparison::Equal(min))
        }
    } else {
        let min = match first.kind {
            TokenKind::Number => first.parser_text().parse::<i32>().ok()?,
            _ => return None,
        };
        let second = tokens.get(1)?;
        let third = tokens.get(2)?;
        if !matches!(second.kind, TokenKind::Dash | TokenKind::EmDash) {
            return None;
        }
        let max = match third.kind {
            TokenKind::Number => third.parser_text().parse::<i32>().ok()?,
            _ => return None,
        };
        if min > max {
            return None;
        }
        IfResultPredicate::Value(Comparison::BetweenInclusive(min, max))
    };

    let trailing_tokens = trim_lexed_commas(&tokens[pipe_idx + 1..]);
    if trailing_tokens.is_empty() {
        return None;
    }

    Some((predicate, trailing_tokens))
}

fn compact_ascii_numeric_range(token: &OwnedLexToken) -> Option<(i32, i32)> {
    if token.kind != TokenKind::Word {
        return None;
    }
    let (min, max) = token.parser_text().split_once('-')?;
    if min.is_empty()
        || max.is_empty()
        || !min.bytes().all(|byte| byte.is_ascii_digit())
        || !max.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let min = min.parse::<i32>().ok()?;
    let max = max.parse::<i32>().ok()?;
    (min <= max).then_some((min, max))
}

pub(crate) fn split_trailing_if_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<TrailingIfClauseSpec<'a>> {
    split_trailing_predicate_clause_lexed(tokens, "if")
}

pub(crate) fn parse_predicate_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> Result<PredicateAst, CardTextError> {
    super::filters::parse_predicate(tokens)
}

pub(crate) fn split_if_clause_lexed(
    tokens: &[OwnedLexToken],
    mut parse_effects: impl FnMut(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<IfClauseSplitSpec, CardTextError> {
    if let Some(tie_choice) = super::conditions::parse_player_life_tie_choice_condition(tokens) {
        let tied_players = tie_choice.tied_players;
        let mut effects = vec![EffectAst::subject_verb_choose_player(
            PlayerAst::You,
            tied_players.clone(),
            crate::tag::TagKey::from(crate::cards::builders::IT_TAG),
            false,
            0,
        )];
        effects.extend(parse_effects(tie_choice.consequence_tokens)?);
        return Ok(IfClauseSplitSpec {
            predicate: IfClausePredicateSpec::Conditional(PredicateAst::ValueComparison {
                left: Value::CountPlayers(tied_players),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(tie_choice.minimum_players as i32),
            }),
            effects,
        });
    }

    fn split_leaves_player_may_search_subject(tokens: &[OwnedLexToken], split_idx: usize) -> bool {
        const PLAYER_MAY_SUFFIXES: &[&[&str]] = &[
            &["its", "controller", "may"],
            &["its", "controllers", "may"],
            &["its", "owner", "may"],
            &["its", "owners", "may"],
            &["that", "player", "may"],
            &["target", "player", "may"],
            &["that", "opponent", "may"],
            &["target", "opponent", "may"],
        ];
        let effect_tokens = trim_lexed_commas(&tokens[split_idx..]);
        if primitives::parse_prefix(
            effect_tokens,
            alt((primitives::kw("search"), primitives::kw("searches"))).void(),
        )
        .is_none()
        {
            return false;
        }

        let predicate_tokens = trim_lexed_commas(&tokens[..split_idx]);
        primitives::strip_lexed_suffix_phrases(predicate_tokens, PLAYER_MAY_SUFFIXES).is_some()
    }

    let parse_effects_with_leading_instead =
        |effect_tokens: &[OwnedLexToken],
         parse_effects: &mut dyn FnMut(
            &[OwnedLexToken],
        ) -> Result<Vec<EffectAst>, CardTextError>| {
            let trimmed = trim_lexed_commas(effect_tokens);
            let without_instead = if trimmed
                .first()
                .is_some_and(|token| structure_token_is(token, "instead"))
            {
                trim_lexed_commas(&trimmed[1..])
            } else {
                trimmed
            };
            parse_effects(without_instead)
        };

    if let Some(effect_token_idx) =
        primitives::find_phrase_start(tokens, &["exile", "them", "then", "meld", "them", "into"])
    {
        let predicate_tokens = trim_lexed_commas(&tokens[1..effect_token_idx]);
        let predicate_tokens_without_commas = predicate_tokens
            .iter()
            .filter(|token| !token.is_comma())
            .cloned()
            .collect::<Vec<_>>();
        let effect_tokens = &tokens[effect_token_idx..];
        if !predicate_tokens_without_commas.is_empty() {
            if let Ok(predicate) =
                parse_predicate_with_grammar_entrypoint_lexed(&predicate_tokens_without_commas)
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
            if let Some(predicate) = parse_if_result_predicate(&predicate_tokens_without_commas)
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Result(predicate),
                    effects,
                });
            }
        }
    }

    let comma_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_comma().then_some(idx))
        .collect::<Vec<_>>();
    if comma_indices.is_empty() {
        for split_idx in (2..tokens.len()).rev() {
            if split_leaves_player_may_search_subject(tokens, split_idx) {
                continue;
            }
            let predicate_tokens = &tokens[1..split_idx];
            let effect_tokens = trim_lexed_commas(&tokens[split_idx..]);
            if effect_tokens.is_empty() {
                continue;
            }
            if let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens) {
                if let Some(effects) = parse_cards_in_hand_difference_draw_effect(
                    &predicate,
                    predicate_tokens,
                    effect_tokens,
                ) {
                    return Ok(IfClauseSplitSpec {
                        predicate: IfClausePredicateSpec::Conditional(predicate),
                        effects,
                    });
                }
                if let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                    && !effects.is_empty()
                {
                    return Ok(IfClauseSplitSpec {
                        predicate: IfClausePredicateSpec::Conditional(predicate),
                        effects,
                    });
                }
            }
            if let Some(predicate) = parse_if_result_predicate(predicate_tokens)
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Result(predicate),
                    effects,
                });
            }
        }
        return Err(CardTextError::ParseError(
            "missing comma in if clause".to_string(),
        ));
    }

    let first_comma_idx = comma_indices[0];
    if first_comma_idx > 1 {
        let predicate_tokens = &tokens[1..first_comma_idx];
        if let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens) {
            let effect_tokens = &tokens[first_comma_idx + 1..];
            if let Some(effects) = parse_cards_in_hand_difference_draw_effect(
                &predicate,
                predicate_tokens,
                effect_tokens,
            ) {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
            if effect_tokens
                .first()
                .is_some_and(|token| token.is_word("search") || token.is_word("searches"))
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
            let comma_fragment_looks_like_effect = if comma_indices.len() > 1 {
                let fragment_tokens = &tokens[first_comma_idx + 1..comma_indices[1]];
                super::effects::for_each_shapes::parse_for_each_object_subject_shape(
                    fragment_tokens,
                )
                .is_some()
                    || parse_effects_with_leading_instead(fragment_tokens, &mut parse_effects)
                        .map(|effects| !effects.is_empty())
                        .unwrap_or(false)
            } else {
                true
            };
            let comma_fragment_looks_like_delayed_trigger = effect_tokens
                .first()
                .is_some_and(|token| token.is_word("when") || token.is_word("whenever"));
            if (comma_fragment_looks_like_effect || comma_fragment_looks_like_delayed_trigger)
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
        }
        if let Some(predicate) = parse_if_result_predicate(predicate_tokens) {
            let effect_tokens = &tokens[first_comma_idx + 1..];
            let effects = parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)?;
            return Ok(IfClauseSplitSpec {
                predicate: IfClausePredicateSpec::Result(predicate),
                effects,
            });
        }
    }

    let mut split: Option<(usize, Vec<EffectAst>)> = None;
    for idx in comma_indices.iter().rev().copied() {
        let effect_tokens = &tokens[idx + 1..];
        if effect_tokens.is_empty() {
            continue;
        }
        if let Ok(effects) = parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
            && !effects.is_empty()
        {
            split = Some((idx, effects));
            break;
        }
    }

    let (comma_idx, effects) = if let Some(split) = split {
        split
    } else {
        let first_idx = comma_indices[0];
        let effect_tokens = &tokens[first_idx + 1..];
        (
            first_idx,
            parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)?,
        )
    };
    let predicate_tokens = &tokens[1..comma_idx];

    if let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens) {
        return Ok(IfClauseSplitSpec {
            predicate: IfClausePredicateSpec::Conditional(predicate),
            effects,
        });
    }
    let Some(predicate) = parse_if_result_predicate(predicate_tokens) else {
        let predicate = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens)?;
        return Ok(IfClauseSplitSpec {
            predicate: IfClausePredicateSpec::Conditional(predicate),
            effects,
        });
    };

    Ok(IfClauseSplitSpec {
        predicate: IfClausePredicateSpec::Result(predicate),
        effects,
    })
}

fn parse_cards_in_hand_difference_draw_effect(
    predicate: &PredicateAst,
    predicate_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let PredicateAst::PlayerCardsInHandOrFewer { player, count } = predicate else {
        return None;
    };
    if !one_of_phrases_occurs(predicate_tokens, &[&["fewer", "than"], &["less", "than"]]) {
        return None;
    }

    let effect_words = TokenWordView::new(trim_lexed_commas(effect_tokens)).to_word_refs();
    let implicit_phrases: &[&[&str]] = &[
        &["draw", "cards", "equal", "to", "the", "difference"],
        &["draw", "cards", "equal", "to", "difference"],
    ];
    let you_phrases: &[&[&str]] = &[
        &["you", "draw", "cards", "equal", "to", "the", "difference"],
        &["you", "draw", "cards", "equal", "to", "difference"],
    ];
    let subject =
        if implicit_phrases.iter().any(|expected| {
            primitives::parse_word_sequence_complete(&effect_words, expected).is_some()
        }) {
            PlayerAst::Implicit
        } else if you_phrases.iter().any(|expected| {
            primitives::parse_word_sequence_complete(&effect_words, expected).is_some()
        }) {
            PlayerAst::You
        } else {
            return None;
        };
    let hand_player = match player {
        PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
        _ => return None,
    };
    let threshold = (*count as i32) + 1;
    let difference = Value::Add(
        Box::new(Value::Fixed(threshold)),
        Box::new(Value::Scaled(Box::new(Value::CardsInHand(hand_player)), -1)),
    )
    .with_surface_hint(ValueSurfaceHint::Difference);

    Some(vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        subject,
        SubjectVerbActionAst::Draw { count: difference },
    )])
}

pub(crate) fn split_trailing_unless_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<TrailingIfClauseSpec<'a>> {
    split_trailing_predicate_clause_lexed(tokens, "unless")
}

pub(crate) fn parse_trailing_if_predicate_lexed(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let trimmed = trim_lexed_commas(tokens);
    if !trimmed
        .first()
        .is_some_and(|token| structure_token_is(token, "if"))
    {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&trimmed[1..]);
    if predicate_tokens.is_empty() {
        return None;
    }

    parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens).ok()
}

pub(crate) fn parse_conditional_predicate_tail_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalPredicateTailSpec> {
    let mut trimmed = trim_lexed_commas(tokens).to_vec();
    while trimmed
        .last()
        .is_some_and(|token| structure_token_is(token, "instead"))
    {
        trimmed.pop();
    }
    let trimmed = trim_lexed_commas(&trimmed);
    if trimmed.is_empty() {
        return None;
    }

    let mut instead_if_idx = None;
    let mut idx = 0usize;
    while idx < trimmed.len() {
        if primitives::parse_prefix(
            &trimmed[idx..],
            (primitives::kw("instead"), primitives::kw("if")),
        )
        .is_some()
        {
            instead_if_idx = Some(idx);
            break;
        }
        idx += 1;
    }

    if let Some(instead_if_idx) = instead_if_idx {
        let base_predicate_tokens = trim_lexed_commas(&trimmed[..instead_if_idx]);
        let outer_predicate_tokens = trim_lexed_commas(&trimmed[instead_if_idx + 2..]);
        if base_predicate_tokens.is_empty() || outer_predicate_tokens.is_empty() {
            return None;
        }

        let base_predicate =
            parse_predicate_with_grammar_entrypoint_lexed(base_predicate_tokens).ok()?;
        let outer_predicate =
            parse_predicate_with_grammar_entrypoint_lexed(outer_predicate_tokens).ok()?;
        return Some(ConditionalPredicateTailSpec::InsteadIf {
            base_predicate,
            outer_predicate,
        });
    }

    let predicate = parse_predicate_with_grammar_entrypoint_lexed(trimmed).ok()?;
    Some(ConditionalPredicateTailSpec::Plain(predicate))
}

fn split_trailing_predicate_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    keyword: &'static str,
) -> Option<TrailingIfClauseSpec<'a>> {
    let split_idx = rfind_unquoted_dynamic_word(tokens, keyword)?;
    if split_idx == 0 || split_idx + 1 >= tokens.len() {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&tokens[split_idx + 1..]);
    if predicate_tokens.is_empty() {
        return None;
    }
    let predicate = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens).ok()?;

    let leading_tokens = trim_lexed_commas(&tokens[..split_idx]);
    if leading_tokens.is_empty() {
        return None;
    }

    Some(TrailingIfClauseSpec {
        leading_tokens,
        predicate,
    })
}

fn rfind_unquoted_dynamic_word(tokens: &[OwnedLexToken], word: &'static str) -> Option<usize> {
    let mut inside_quotes = false;
    let mut result = None;

    for (idx, token) in tokens.iter().enumerate() {
        if is_sentence_quote(token) {
            inside_quotes = !inside_quotes;
            continue;
        }
        if !inside_quotes && token_matches_dynamic_word(token, word) {
            result = Some(idx);
        }
    }

    result
}

pub(crate) fn parse_who_player_predicate_lexed(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let trimmed = trim_lexed_commas(tokens);
    if !trimmed
        .first()
        .is_some_and(|token| structure_token_is(token, "who"))
    {
        return None;
    }

    let predicate_tail = trim_lexed_commas(&trimmed[1..]);
    if predicate_tail.is_empty() {
        return None;
    }

    let mut predicate_tokens = Vec::with_capacity(predicate_tail.len() + 2);
    predicate_tokens.push(OwnedLexToken::word(
        "that".to_string(),
        TextSpan::synthetic(),
    ));
    predicate_tokens.push(OwnedLexToken::word(
        "player".to_string(),
        TextSpan::synthetic(),
    ));
    predicate_tokens.extend(predicate_tail.iter().cloned());

    parse_predicate_with_grammar_entrypoint_lexed(&predicate_tokens).ok()
}

pub(crate) fn parse_trailing_instead_if_predicate_lexed(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let trimmed = trim_lexed_commas(tokens);
    if !trimmed
        .first()
        .is_some_and(|token| structure_token_is(token, "instead"))
        || !trimmed
            .get(1)
            .is_some_and(|token| structure_token_is(token, "if"))
    {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&trimmed[2..]);
    if predicate_tokens.is_empty() {
        return None;
    }

    parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens).ok()
}

pub(crate) fn split_triggered_conditional_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    start_idx: usize,
) -> Option<TriggeredConditionalClauseSpec<'a>> {
    let (leading_tokens, after_first_comma) = primitives::split_lexed_once_on_comma(tokens)?;
    if leading_tokens.len() <= start_idx {
        return None;
    }

    let trigger_tokens = &leading_tokens[start_idx..];
    let after_first_comma = trim_lexed_commas(after_first_comma);

    if let Some(while_idx) = find_token_word(trigger_tokens, "while")
        && while_idx > 0
    {
        let predicate_tokens = trim_lexed_commas(&trigger_tokens[while_idx + 1..]);
        if let Some(predicate) = parse_modeled_predicate(predicate_tokens) {
            return Some(TriggeredConditionalClauseSpec {
                trigger_tokens: trim_lexed_commas(&trigger_tokens[..while_idx]),
                predicate,
                effects_tokens: after_first_comma,
            });
        }
    }

    let (_, after_if) = primitives::parse_prefix(after_first_comma, primitives::kw("if"))?;

    let mut comma_indices = Vec::new();
    let mut inside_quotes = false;
    for (comma_idx, token) in after_if.iter().enumerate() {
        if is_sentence_quote(token) {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes || !token.is_comma() {
            continue;
        }
        comma_indices.push(comma_idx);
    }

    for comma_idx in comma_indices.into_iter().rev() {
        let predicate_tokens = trim_lexed_commas(&after_if[..comma_idx]);
        let effects_tokens = trim_lexed_commas(&after_if[comma_idx + 1..]);
        if predicate_tokens.is_empty() || effects_tokens.is_empty() {
            continue;
        }
        if contains_token_kind(predicate_tokens, TokenKind::Period) {
            continue;
        }
        if effects_tokens
            .first()
            .is_some_and(|token| structure_token_is_any(token, &["and", "then"]))
        {
            continue;
        }
        // Search effects commonly contain follow-up commas. When candidates are
        // examined from right to left, a later comma can otherwise absorb the
        // search action into a permissively modeled predicate and leave only a
        // put/reveal/shuffle follow-up as the effect.
        if predicate_candidate_contains_search_action(predicate_tokens) {
            continue;
        }
        // A duration following a comma belongs to the effect clause. Reject
        // this later split candidate so the preceding comma can preserve the
        // duration at the head of `effects_tokens`. A predicate that itself
        // ends in "this turn" has no separating comma and remains valid.
        if leaf::parse_leaf_restriction_duration_suffix_tokens(predicate_tokens).is_some_and(
            |shape| {
                shape
                    .rest
                    .last()
                    .is_some_and(|token| token.kind == TokenKind::Comma)
            },
        ) {
            continue;
        }
        if let Some(predicate) = parse_modeled_predicate(predicate_tokens) {
            return Some(TriggeredConditionalClauseSpec {
                trigger_tokens,
                predicate,
                effects_tokens,
            });
        }
    }

    None
}

pub(crate) fn split_state_triggered_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    start_idx: usize,
    split_idx: usize,
) -> Option<StateTriggeredClauseSpec<'a>> {
    if split_idx <= start_idx || split_idx >= tokens.len() {
        return None;
    }
    if !tokens
        .first()
        .is_some_and(|token| structure_token_is_any(token, &["when", "whenever"]))
    {
        return None;
    }

    let trigger_tokens = &tokens[start_idx..split_idx];
    let effects_tokens = trim_lexed_commas(&tokens[split_idx + 1..]);
    if effects_tokens.is_empty() {
        return None;
    }

    let predicate = if let Some(comma_idx) =
        structure_token_kind_index(trigger_tokens, TokenKind::Comma)
        && trigger_tokens
            .get(comma_idx + 1)
            .is_some_and(|token| structure_token_is(token, "if"))
    {
        let state_predicate =
            parse_modeled_predicate(trim_lexed_commas(&trigger_tokens[..comma_idx]))?;
        let gate_predicate =
            parse_modeled_predicate(trim_lexed_commas(&trigger_tokens[comma_idx + 2..]))?;
        PredicateAst::And(Box::new(state_predicate), Box::new(gate_predicate))
    } else {
        parse_modeled_predicate(trigger_tokens)?
    };

    Some(StateTriggeredClauseSpec {
        trigger_tokens,
        display_tokens: &tokens[..split_idx],
        predicate,
        effects_tokens,
    })
}

pub(crate) fn split_trailing_modal_gate_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<TrailingModalGateSpec<'a>> {
    let sentence_start = structure_token_kind_rindex(tokens, TokenKind::Period)
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let sentence_tokens = trim_lexed_commas(&tokens[sentence_start..]);
    if sentence_tokens.is_empty() {
        return None;
    }
    let (_, predicate_tail) = primitives::parse_prefix(
        sentence_tokens,
        alt((primitives::kw("if"), primitives::kw("when"))),
    )?;
    let (predicate_tokens, trailing_tokens) = if let Some((predicate_tokens, trailing_tokens)) =
        primitives::split_lexed_once_on_comma(predicate_tail)
    {
        (
            trim_lexed_commas(predicate_tokens),
            trim_lexed_commas(trailing_tokens),
        )
    } else {
        (trim_lexed_commas(predicate_tail), &[][..])
    };
    if predicate_tokens.is_empty() || !trailing_tokens.is_empty() {
        return None;
    }

    let mut prefix_end = sentence_start;
    while prefix_end > 0 && tokens[prefix_end - 1].kind == TokenKind::Comma {
        prefix_end -= 1;
    }

    let predicate = parse_if_result_predicate(predicate_tokens)?;

    Some(TrailingModalGateSpec {
        prefix_tokens: &tokens[..prefix_end],
        predicate,
        remove_mode_only: primitives::parse_prefix(predicate_tokens, parse_remove_mode_only_prefix)
            .is_some(),
    })
}

fn parse_modal_header_choose_spec_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<Option<ModalHeaderChooseSpec>, ErrMode<ContextError>> {
    let tokens = input.peek_finish();
    let choose_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| structure_token_is(token, "choose").then_some(idx))
        .collect::<Vec<_>>();
    if choose_indices.is_empty() {
        input.finish();
        return Ok(None);
    }

    for choose_idx in choose_indices.iter().copied() {
        let choose_tail = &tokens[choose_idx + 1..];
        let Some((Some(min), max)) = values::parse_modal_choose_range(choose_tail).ok().flatten()
        else {
            continue;
        };
        let x_clause_start = primitives::find_phrase_start(choose_tail, &["x", "is"])
            .map(|idx| choose_idx + 1 + idx);
        let random = primitives::find_phrase_start(choose_tail, &["at", "random"]).is_some();

        input.finish();
        return Ok(Some(ModalHeaderChooseSpec {
            choose_idx,
            min,
            max,
            random,
            x_clause_start,
        }));
    }

    let choose_idx = *choose_indices.last().expect("checked non-empty");
    input.next_slice(choose_idx + 1);
    Err(primitives::cut_err_ctx(
        "modal header choose clause",
        "modal choice range",
    ))
}

pub(crate) fn parse_modal_header_choose_spec<'a>(
    input: &mut LexStream<'a>,
) -> Result<Option<ModalHeaderChooseSpec>, ErrMode<ContextError>> {
    parse_modal_header_choose_spec_inner
        .context(StrContext::Label("modal header"))
        .context(StrContext::Expected(StrContextValue::Description(
            "modal header line",
        )))
        .parse_next(input)
}
