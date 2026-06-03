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
use crate::effect::{Comparison, Value};
use crate::target::PlayerFilter;
use ironsmith_core::ValueSurfaceHint;

use super::super::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::super::lexer::{
    LexStream, LexToken, OwnedLexToken, TokenKind, TokenWordView, contains_token_kind,
    token_slice_first_is_any, token_slice_starts_with_at, trim_lexed_commas,
    word_slice_strip_prefix,
};
use super::super::token_primitives::{find_index_with, rfind_index_with};
use super::super::util::{parse_card_type, parse_color, parse_subtype_flexible};
use super::{primitives, values};

const TRIGGER_LIST_CONJUNCTION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["or"], &["and"], &["and/or"]]);
const DISCARD_TRIGGER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["discard", "discards"]]);
const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["card", "cards"]]);
const SPELL_OR_SPELLS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["spell", "spells"]]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["or"]);
const ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["the"]]);
const TYPEISH_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["artifact"], &["artifacts"], &["creature"], &["creatures"]]);
const FIRST_TIME_EACH_TURN_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["for", "the", "first", "time", "each", "turn"],
            &["for", "the", "first", "time", "this", "turn"],
        ]
);
const ENCHANTED_CONTROLLER_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["enchanted", "creature", "controller"],
            &["enchanted", "creatures", "controller"],
            &["enchanted", "permanent", "controller"],
            &["enchanted", "permanents", "controller"],
            &["enchanted", "artifact", "controller"],
            &["enchanted", "artifacts", "controller"],
            &["enchanted", "enchantment", "controller"],
            &["enchanted", "enchantments", "controller"],
            &["enchanted", "land", "controller"],
            &["enchanted", "lands", "controller"],
        ]
);
const RESULT_VERB_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["remove"],
            &["removed"],
            &["sacrifice"],
            &["sacrificed"],
            &["discard"],
            &["discarded"],
            &["exile"],
            &["exiled"],
            &["mill"],
            &["milled"],
        ]
);
const SEARCH_RESULT_VERB_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["search"], &["searches"], &["searched"]]);
const THIS_WAY_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["this", "way"]);
const UNQUALIFIED_THIS_WAY_RESULT_QUALIFIER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it"],
            &["them"],
            &["that"],
            &["card"],
            &["a", "card"],
            &["a", "creature", "card"],
            &["creature", "card"],
        ]
);
const SHORT_NEGATED_RESULT_VERB_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["dont"], &["didnt"], &["cant"]]);
const SHORT_DID_NOT_RESULT_VERB_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["dont"], &["didnt"]]);
const SPLIT_NEGATED_RESULT_VERB_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["do", "not"], &["did", "not"], &["can", "not"]]);
const SPLIT_DID_NOT_RESULT_VERB_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["do", "not"], &["did", "not"]]);
const YOU_DO_RESULT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you", "do"]);
const IT_CONNIVES_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "connives", "this", "way"],
            &["it", "connive", "this", "way"],
        ]
);
const YOU_WIN_RESULT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you", "win"], &["you", "won"]]);
const CLASH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["clash"]);
const RESULT_IS_VALUE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["result", "is"], &["result", "was"]]);
const FLIP_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["flip"]);
const THEY_DO_RESULT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["they", "do"]);
const PLAYER_DO_RESULT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["player", "do"],
            &["player", "does"],
            &["players", "do"],
            &["players", "does"]
        ]
);
const THAT_PLAYER_DO_RESULT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that", "player", "do"], &["that", "player", "does"]]);
const FIRST_PLAYER_DO_RESULT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["first", "player", "do"], &["first", "player", "does"]]);
const YOU_SEARCHED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "searched"]; suffix & ["this", "way"]);
const ONE_OR_MORE_THIS_WAY_RESULT_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["one", "or", "more"]; suffix & ["this", "way"]);
const RESULT_OBJECT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["card"],
            &["cards"],
            &["creature"],
            &["creatures"],
            &["permanent"],
            &["permanents"],
        ]
);
const IS_OR_ARE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["is"], &["are"]]);
const PLAYER_DEALT_DAMAGE_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "player", "is", "dealt", "damage", "this", "way"],
            &["player", "is", "dealt", "damage", "this", "way"],
        ]
);
const SPELL_COUNTERED_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(prefix_any & [&["that", "spell"], &["it", "spell"]]; suffix & ["this", "way"]; contains_words & ["countered"]);
const OBJECT_DIES_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "creature", "dies", "this", "way"],
            &["that", "permanent", "dies", "this", "way"],
            &["that", "card", "dies", "this", "way"],
            &["it", "creature", "dies", "this", "way"],
            &["it", "permanent", "dies", "this", "way"],
            &["it", "card", "dies", "this", "way"],
        ]
);
const OBJECT_DAMAGE_WOULD_DIE_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "creature", "dealt", "damage", "this", "way", "would", "die", "this"
            ],
            &[
                "permanent",
                "dealt",
                "damage",
                "this",
                "way",
                "would",
                "die",
                "this"
            ],
            &[
                "card", "dealt", "damage", "this", "way", "would", "die", "this"
            ],
        ]
);
const EXCESS_DAMAGE_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["it", "deals", "excess", "damage", "this", "way"]);
const EXCESS_DAMAGE_WAS_DEALT_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["excess", "damage", "was", "dealt", "to"];
    suffix & ["this", "way"];
    contains_words & ["creature"]
);
const YOU_LOSE_FLIP_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you", "lose"], &["you", "lost"]]; contains_words & ["flip"]);
const PLAYER_SHORT_NEGATED_RESULT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["player", "dont"],
            &["player", "doesnt"],
            &["player", "didnt"],
            &["player", "cant"],
            &["players", "dont"],
            &["players", "doesnt"],
            &["players", "didnt"],
            &["players", "cant"],
        ]
);
const PLAYER_SPLIT_NEGATED_RESULT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["player", "do", "not"],
            &["player", "does", "not"],
            &["player", "did", "not"],
            &["player", "can", "not"],
            &["players", "do", "not"],
            &["players", "does", "not"],
            &["players", "did", "not"],
            &["players", "can", "not"],
        ]
);
const THAT_PLAYER_SHORT_NEGATED_RESULT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "player", "dont"],
            &["that", "player", "doesnt"],
            &["that", "player", "didnt"],
            &["that", "player", "cant"],
        ]
);
const THAT_PLAYER_SPLIT_NEGATED_RESULT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "player", "do", "not"],
            &["that", "player", "does", "not"],
            &["that", "player", "did", "not"],
            &["that", "player", "can", "not"],
        ]
);
const TURN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["turn"]);
const INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const WHEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["when"]);
const WHO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["who"]);
const WHILE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["while"]);
const CONTROL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["control"]);
const AS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["as"]);
const WHEN_OR_WHENEVER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["when"], &["whenever"]]);
const AND_OR_THEN_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["then"]]);
const CHOOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["choose"]);

fn token_matches_dynamic_word(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|word| word.eq_ignore_ascii_case(expected))
}

fn find_token_shape(tokens: &[OwnedLexToken], shape: &ClauseShape<'static>) -> Option<usize> {
    tokens.iter().position(|token| shape.matches_token(token))
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
    pub(crate) same_mode_more_than_once: bool,
    pub(crate) mode_must_be_unchosen: bool,
    pub(crate) mode_must_be_unchosen_this_turn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailingModalGateSpec<'a> {
    pub(crate) prefix_tokens: &'a [OwnedLexToken],
    pub(crate) predicate: IfResultPredicate,
    pub(crate) remove_mode_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MetadataLineKind {
    ManaCost,
    TypeLine,
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

fn normalized_parser_word(token: &LexToken) -> Option<String> {
    match token.kind {
        TokenKind::Word | TokenKind::Number | TokenKind::Tilde | TokenKind::Half => Some(
            token
                .parser_text()
                .chars()
                .filter(|ch| !matches!(*ch, '\'' | '’' | '‘'))
                .collect(),
        ),
        _ => None,
    }
}

fn parser_text_non_article_words(tokens: &[LexToken]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(normalized_parser_word)
        .filter(|word| !ARTICLE_WORD_PATTERN.matches_word(word))
        .collect()
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
    if primitives::contains_phrase(
        tokens,
        &["at", "the", "beginning", "of", "your", "next", "upkeep"],
    ) && primitives::contains_phrase(tokens, &["lose", "the", "game"])
        && (primitives::contains_phrase(tokens, &["if", "you", "dont"])
            || primitives::contains_phrase(tokens, &["if", "you", "don't"])
            || primitives::contains_phrase(tokens, &["if", "you", "do", "not"]))
    {
        return Some(StatementLineFamily::PactNextUpkeep);
    }

    if primitives::contains_any_phrase(
        tokens,
        &[
            &["during", "that", "player's", "next", "turn"],
            &["during", "that", "players", "next", "turn"],
        ],
    ) && primitives::contains_any_phrase(
        tokens,
        &[
            &["can't", "cast"],
            &["cant", "cast"],
            &["can", "not", "cast"],
        ],
    ) {
        return Some(StatementLineFamily::NextTurnCantCast);
    }

    if primitives::contains_any_phrase(
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

    if primitives::contains_phrase(
        tokens,
        &[
            "ask", "a", "person", "outside", "the", "game", "to", "rate", "its", "new", "art",
            "on", "a", "scale", "from", "1", "to", "5",
        ],
    ) {
        return Some(StatementLineFamily::ArtRating);
    }

    if primitives::contains_any_phrase(tokens, &[&["become"], &["becomes"]])
        && primitives::contains_phrase(tokens, &["until", "end", "of", "turn"])
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

    if primitives::parse_prefix(tokens, primitives::phrase(&["starting", "with", "you"])).is_some()
        && primitives::contains_phrase(tokens, &["each", "player"])
        && primitives::contains_word(tokens, "pay")
    {
        return Some(StatementLineFamily::Generic);
    }

    if primitives::contains_phrase(tokens, &["bid", "life"])
        && primitives::contains_phrase(tokens, &["high", "bid"])
        && primitives::contains_phrase(tokens, &["high", "bidder"])
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

pub(crate) fn classify_static_line_family_lexed(
    tokens: &[OwnedLexToken],
) -> Option<StaticLineFamily> {
    if super::abilities::split_untap_each_other_players_untap_step_line_lexed(tokens).is_some() {
        return Some(StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep);
    }

    if primitives::contains_any_phrase(
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
    ) {
        return Some(StaticLineFamily::Generic);
    }

    if let Some(quote_idx) = primitives::find_token_index(tokens, |token| token.is_quote()) {
        let head = trim_lexed_commas(&tokens[..quote_idx]);
        if !head.is_empty()
            && !contains_token_kind(head, TokenKind::Period)
            && primitives::words_match_any_prefix(head, &[&["this"], &["it"], &["all"], &["each"]])
                .is_some()
        {
            let words = TokenWordView::new(head);
            if words.find_word("has").is_some() || words.find_word("have").is_some() {
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
        || primitives::contains_phrase(tokens, &["maximum", "hand", "size"]))
    .then_some(StaticLineFamily::Generic)
}

fn parse_modeled_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_predicate_with_grammar_entrypoint_lexed(tokens).ok()
}

fn classify_if_result_predicate(words: &[&str]) -> Option<IfResultPredicate> {
    let is_result_verb = |word: &str| RESULT_VERB_WORD_PATTERN.matches_word(word);
    let is_unqualified_this_way_result = |subject: &str| {
        if words.len() < 4
            || words[0] != subject
            || !is_result_verb(words[1])
            || !THIS_WAY_SUFFIX_PATTERN.matches_words(words)
        {
            return false;
        }
        let qualifiers = &words[2..words.len() - 2];
        qualifiers.is_empty()
            || UNQUALIFIED_THIS_WAY_RESULT_QUALIFIER_PATTERN.matches_words(qualifiers)
    };
    let is_exact_negated_result = |subject: &str| {
        if words.first().copied() != Some(subject) {
            return false;
        }
        SHORT_NEGATED_RESULT_VERB_PATTERN.matches_words(&words[1..])
            || SPLIT_NEGATED_RESULT_VERB_PATTERN.matches_words(&words[1..])
    };
    let is_negated_this_way_result = |subject: &str| {
        if words.first().copied() != Some(subject) {
            return false;
        }
        let action_idx = if words.len() >= 5
            && SHORT_DID_NOT_RESULT_VERB_PATTERN.matches_words(&words[1..2])
        {
            2
        } else if words.len() >= 6 && SPLIT_DID_NOT_RESULT_VERB_PATTERN.matches_words(&words[1..3])
        {
            3
        } else {
            return false;
        };
        if !is_result_verb(words[action_idx]) || !THIS_WAY_SUFFIX_PATTERN.matches_words(words) {
            return false;
        }
        let qualifiers = &words[action_idx + 1..words.len() - 2];
        qualifiers.is_empty() || matches!(qualifiers, ["it"] | ["them"] | ["that"])
    };
    let is_searched_library_this_way = || {
        if words.len() < 5 || !THIS_WAY_SUFFIX_PATTERN.matches_words(words) {
            return false;
        }
        let subject_len = match words {
            ["you", ..] | ["they", ..] | ["player", ..] | ["players", ..] => 1,
            ["that", "player", ..] | ["first", "player", ..] => 2,
            _ => return false,
        };
        let Some(verb) = words.get(subject_len) else {
            return false;
        };
        if !SEARCH_RESULT_VERB_WORD_PATTERN.matches_word(verb) {
            return false;
        }
        matches!(
            &words[subject_len + 1..words.len() - 2],
            ["your", "library"] | ["their", "library"] | ["library"]
        )
    };

    if YOU_DO_RESULT_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::Did);
    }
    if IT_CONNIVES_THIS_WAY_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::Did);
    }
    if YOU_WIN_RESULT_PREFIX_PATTERN.matches_words(words)
        && (words.len() == 2 || CLASH_WORD_PATTERN.matches_words(words))
    {
        return Some(IfResultPredicate::Value(
            crate::effect::Comparison::GreaterThan(0),
        ));
    }
    if words.len() == 3
        && RESULT_IS_VALUE_PREFIX_PATTERN.matches_words(words)
        && let Some(value) = ironsmith_core::parse_cardinal_word(words[2])
            .and_then(|value| i32::try_from(value).ok())
    {
        return Some(IfResultPredicate::Value(Comparison::Equal(value)));
    }
    if YOU_WIN_RESULT_PREFIX_PATTERN.matches_words(words) && FLIP_WORD_PATTERN.matches_words(words)
    {
        return Some(IfResultPredicate::Did);
    }
    if THEY_DO_RESULT_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::Did);
    }
    if PLAYER_DO_RESULT_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::Did);
    }
    if THAT_PLAYER_DO_RESULT_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::Did);
    }
    if FIRST_PLAYER_DO_RESULT_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::Did);
    }
    if YOU_SEARCHED_THIS_WAY_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::Did);
    }
    if is_searched_library_this_way() {
        return Some(IfResultPredicate::SearchedLibrary);
    }
    if is_unqualified_this_way_result("you") {
        return Some(IfResultPredicate::Did);
    }
    if is_unqualified_this_way_result("they") {
        return Some(IfResultPredicate::Did);
    }
    let is_one_or_more_result = words.len() >= 6
        && ONE_OR_MORE_THIS_WAY_RESULT_PATTERN.matches_words(words)
        && RESULT_OBJECT_WORD_PATTERN.matches_word(words[3])
        && ((IS_OR_ARE_WORD_PATTERN.matches_word(words[4])
            && words.len() >= 7
            && is_result_verb(words[5]))
            || is_result_verb(words[4]));
    if is_one_or_more_result {
        return Some(IfResultPredicate::Did);
    }
    if PLAYER_DEALT_DAMAGE_THIS_WAY_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::Did);
    }

    if SPELL_COUNTERED_THIS_WAY_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::Did);
    }

    if OBJECT_DIES_THIS_WAY_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::DiesThisWay);
    }
    if OBJECT_DAMAGE_WOULD_DIE_THIS_WAY_PATTERN.matches_words(words)
        && words
            .get(8)
            .is_some_and(|word| TURN_WORD_PATTERN.matches_word(word))
    {
        return Some(IfResultPredicate::DiesThisWay);
    }

    if matches!(
        words,
        ["its", "power", "becomes", _, "this", "way"]
            | ["it", "power", "becomes", _, "this", "way"]
    ) {
        return Some(IfResultPredicate::Did);
    }
    if EXCESS_DAMAGE_WAS_DEALT_THIS_WAY_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::ExcessDamageDealt);
    }
    if EXCESS_DAMAGE_THIS_WAY_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::Did);
    }

    if is_exact_negated_result("you") || is_negated_this_way_result("you") {
        return Some(IfResultPredicate::DidNot);
    }
    if YOU_LOSE_FLIP_PREFIX_PATTERN.matches_words(words) {
        return Some(IfResultPredicate::DidNot);
    }
    if is_exact_negated_result("they") || is_negated_this_way_result("they") {
        return Some(IfResultPredicate::DidNot);
    }
    if PLAYER_SHORT_NEGATED_RESULT_PATTERN.matches_words(words)
        || PLAYER_SPLIT_NEGATED_RESULT_PATTERN.matches_words(words)
    {
        return Some(IfResultPredicate::DidNot);
    }
    if THAT_PLAYER_SHORT_NEGATED_RESULT_PATTERN.matches_words(words)
        || THAT_PLAYER_SPLIT_NEGATED_RESULT_PATTERN.matches_words(words)
    {
        return Some(IfResultPredicate::DidNot);
    }

    None
}

fn parse_if_result_predicate_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<IfResultPredicate, ErrMode<ContextError>> {
    let tokens = input.peek_finish();
    let words = parser_text_non_article_words(tokens);
    let word_refs: Vec<&str> = words.iter().map(String::as_str).collect();
    let Some(predicate) = classify_if_result_predicate(&word_refs) else {
        return Err(primitives::backtrack_err(
            "if-result predicate",
            "result predicate clause",
        ));
    };

    input.finish();
    Ok(predicate)
}

pub(crate) fn parse_if_result_predicate(tokens: &[OwnedLexToken]) -> Option<IfResultPredicate> {
    primitives::parse_prefix(tokens, parse_if_result_predicate_inner)
        .and_then(|(predicate, rest)| rest.is_empty().then_some(predicate))
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

fn looks_like_trigger_objectish_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || parse_subtype_flexible(word).is_some()
        || word.strip_suffix('s').is_some_and(|stem| {
            parse_card_type(stem).is_some() || parse_subtype_flexible(stem).is_some()
        })
}

fn looks_like_trigger_object_list_tail_lexed(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words_view = TokenWordView::new(tokens);
    let words = words_view.word_refs();
    if words.is_empty() {
        return false;
    }
    let starts_with_conjunction = words
        .first()
        .is_some_and(|word| TRIGGER_LIST_CONJUNCTION_WORD_PATTERN.matches_word(word));
    let first_candidate = if starts_with_conjunction {
        words.get(1).copied()
    } else {
        words.first().copied()
    };
    let Some(first_word) = first_candidate else {
        return false;
    };
    looks_like_trigger_objectish_word(first_word) && contains_token_kind(tokens, TokenKind::Comma)
}

fn looks_like_trigger_discard_qualifier_tail_lexed(
    trigger_prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
) -> bool {
    if tail_tokens.is_empty() {
        return false;
    }

    let prefix_words_view = TokenWordView::new(trigger_prefix_tokens);
    let prefix_words = prefix_words_view.word_refs();
    if !DISCARD_TRIGGER_WORD_PATTERN.matches_words(&prefix_words) {
        return false;
    }

    let tail_words_view = TokenWordView::new(tail_tokens);
    let tail_words = tail_words_view.word_refs();
    if tail_words.is_empty() {
        return false;
    }

    let Some(first_word) = tail_words.first().copied() else {
        return false;
    };
    let typeish = parse_card_type(first_word).is_some()
        || TYPEISH_WORD_PATTERN.matches_word(first_word)
        || TRIGGER_LIST_CONJUNCTION_WORD_PATTERN.matches_word(first_word);
    if !typeish {
        return false;
    }

    primitives::find_token_index(tail_tokens, |token| token.kind == TokenKind::Comma).is_some_and(
        |comma_idx| {
            let before_words_view = TokenWordView::new(&tail_tokens[..comma_idx]);
            let before_words = before_words_view.word_refs();
            CARD_OR_CARDS_WORD_PATTERN.matches_words(&before_words)
        },
    )
}

fn looks_like_trigger_type_list_tail_lexed(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words_view = TokenWordView::new(tokens);
    let words = words_view.word_refs();
    if words.is_empty() {
        return false;
    }
    let first_is_card_type = parse_card_type(words[0]).is_some()
        || parse_subtype_flexible(words[0]).is_some()
        || words[0].strip_suffix('s').is_some_and(|word| {
            parse_card_type(word).is_some() || parse_subtype_flexible(word).is_some()
        });
    first_is_card_type
        && SPELL_OR_SPELLS_WORD_PATTERN.matches_words(&words)
        && OR_WORD_PATTERN.matches_words(&words)
        && contains_token_kind(tokens, TokenKind::Comma)
}

fn looks_like_trigger_color_list_tail_lexed(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words_view = TokenWordView::new(tokens);
    let words = words_view.word_refs();
    if words.is_empty() {
        return false;
    }
    parse_color(words[0]).is_some()
        && OR_WORD_PATTERN.matches_words(&words)
        && contains_token_kind(tokens, TokenKind::Comma)
}

fn looks_like_trigger_numeric_list_tail_lexed(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words_view = TokenWordView::new(tokens);
    let words = words_view.word_refs();
    if words.len() < 3 || words[0].parse::<i32>().is_err() {
        return false;
    }
    words.iter().skip(1).any(|word| word.parse::<i32>().is_ok())
        && OR_WORD_PATTERN.matches_words(&words)
}

pub(crate) fn find_trigger_effect_list_tail_split_lexed(
    trigger_prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
) -> Option<usize> {
    let looks_like_discard_qualifier_tail =
        looks_like_trigger_discard_qualifier_tail_lexed(trigger_prefix_tokens, tail_tokens);
    if !looks_like_trigger_type_list_tail_lexed(tail_tokens)
        && !looks_like_trigger_color_list_tail_lexed(tail_tokens)
        && !looks_like_trigger_object_list_tail_lexed(tail_tokens)
        && !looks_like_trigger_numeric_list_tail_lexed(tail_tokens)
        && !looks_like_discard_qualifier_tail
    {
        return None;
    }

    if looks_like_discard_qualifier_tail {
        return find_index_with(tail_tokens, |idx, token| {
            if token.kind != TokenKind::Comma {
                return false;
            }
            let before_words_view = TokenWordView::new(&tail_tokens[..idx]);
            let before_words = before_words_view.word_refs();
            CARD_OR_CARDS_WORD_PATTERN.matches_words(&before_words)
        });
    }

    if looks_like_trigger_numeric_list_tail_lexed(tail_tokens) {
        return rfind_index_with(tail_tokens, |_, token| token.kind == TokenKind::Comma);
    }

    find_index_with(tail_tokens, |idx, token| {
        if token.kind != TokenKind::Comma {
            return false;
        }
        let before_words_view = TokenWordView::new(&tail_tokens[..idx]);
        let before_words = before_words_view.word_refs();
        SPELL_OR_SPELLS_WORD_PATTERN.matches_words(&before_words)
    })
    .or_else(|| {
        if looks_like_trigger_color_list_tail_lexed(tail_tokens)
            || looks_like_trigger_object_list_tail_lexed(tail_tokens)
        {
            find_index_with(tail_tokens, |idx, token| {
                if token.kind != TokenKind::Comma {
                    return false;
                }
                let Some(next_word) = tail_tokens.get(idx + 1).and_then(OwnedLexToken::as_word)
                else {
                    return false;
                };
                if TRIGGER_LIST_CONJUNCTION_WORD_PATTERN.matches_word(next_word) {
                    return false;
                }

                let next_is_list_item = if looks_like_trigger_color_list_tail_lexed(tail_tokens) {
                    parse_color(next_word).is_some()
                } else {
                    looks_like_trigger_objectish_word(next_word)
                };
                if next_is_list_item {
                    return false;
                }
                true
            })
        } else {
            None
        }
    })
}

pub(crate) fn split_first_time_each_turn_trigger_suffix_lexed(
    trigger_tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<u32>) {
    let trigger_words = TokenWordView::new(trigger_tokens);
    let words = trigger_words.word_refs();
    if FIRST_TIME_EACH_TURN_SUFFIX_PATTERN.matches_words(&words) {
        let trimmed_word_len = words.len().saturating_sub(6);
        let trimmed_token_len = trigger_words
            .token_index_for_word_index(trimmed_word_len)
            .unwrap_or(trigger_tokens.len());
        return (&trigger_tokens[..trimmed_token_len], Some(1));
    }
    (trigger_tokens, None)
}

pub(crate) fn rewrite_attached_controller_trigger_effect_tokens_lexed(
    trigger_tokens: &[OwnedLexToken],
    effects_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let trigger_words_view = TokenWordView::new(trigger_tokens);
    let trigger_words = trigger_words_view.word_refs();
    let references_enchanted_controller = (0..trigger_words.len())
        .any(|idx| ENCHANTED_CONTROLLER_TRIGGER_PATTERN.matches_words(&trigger_words[idx..]));
    if !references_enchanted_controller {
        return effects_tokens.to_vec();
    }

    let mut rewritten = Vec::with_capacity(effects_tokens.len());
    let mut idx = 0usize;
    while idx < effects_tokens.len() {
        if token_slice_starts_with_at(effects_tokens, idx, &["that", "creature"]) {
            let mut enchanted = effects_tokens[idx].clone();
            let _ = enchanted.replace_word("enchanted");
            rewritten.push(enchanted);
            rewritten.push(effects_tokens[idx + 1].clone());
            idx += 2;
            continue;
        }
        if token_slice_starts_with_at(effects_tokens, idx, &["that", "permanent"]) {
            let mut enchanted = effects_tokens[idx].clone();
            let _ = enchanted.replace_word("enchanted");
            rewritten.push(enchanted);
            rewritten.push(effects_tokens[idx + 1].clone());
            idx += 2;
            continue;
        }
        rewritten.push(effects_tokens[idx].clone());
        idx += 1;
    }

    rewritten
}

pub(crate) fn scan_modal_header_flags(tokens: &[OwnedLexToken]) -> ModalHeaderFlags {
    let mode_must_be_unchosen_this_turn = primitives::contains_any_phrase(
        tokens,
        &[
            &["that", "hasnt", "been", "chosen", "this", "turn"],
            &["that", "hasn't", "been", "chosen", "this", "turn"],
            &["that", "has", "not", "been", "chosen", "this", "turn"],
        ],
    );
    let mode_must_be_unchosen = mode_must_be_unchosen_this_turn
        || primitives::contains_any_phrase(
            tokens,
            &[
                &["that", "hasnt", "been", "chosen"],
                &["that", "hasn't", "been", "chosen"],
                &["that", "has", "not", "been", "chosen"],
            ],
        );

    let choose_both_control_card_types = scan_choose_both_control_card_types(tokens);

    ModalHeaderFlags {
        commander_allows_both: primitives::contains_word(tokens, "commander")
            && primitives::contains_word(tokens, "both"),
        choose_both_control_card_types,
        same_mode_more_than_once: primitives::contains_phrase(
            tokens,
            &["same", "mode", "more", "than", "once"],
        ),
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
    }
}

fn scan_choose_both_control_card_types(tokens: &[OwnedLexToken]) -> Vec<crate::types::CardType> {
    if !primitives::contains_phrase(tokens, &["you", "may", "choose", "both", "instead"]) {
        return Vec::new();
    }
    let Some(if_idx) = find_token_shape(tokens, &IF_WORD_PATTERN) else {
        return Vec::new();
    };
    let Some(control_idx) =
        find_token_shape(&tokens[if_idx + 1..], &CONTROL_WORD_PATTERN).map(|idx| if_idx + 1 + idx)
    else {
        return Vec::new();
    };
    let Some(as_idx) = find_token_shape(&tokens[control_idx + 1..], &AS_WORD_PATTERN)
        .map(|idx| control_idx + 1 + idx)
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
        if let Some(card_type) = parse_card_type(token.parser_text())
            && !card_types.contains(&card_type)
        {
            card_types.push(card_type);
        }
    }
    card_types
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
        .is_some_and(|token| IF_WORD_PATTERN.matches_token(token))
    {
        LeadingResultPrefixKind::If
    } else if trimmed
        .first()
        .is_some_and(|token| WHEN_WORD_PATTERN.matches_token(token))
    {
        LeadingResultPrefixKind::When
    } else {
        return None;
    };

    let comma_idx = primitives::find_token_index(trimmed, |token| token.kind == TokenKind::Comma)?;
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
    let second = tokens.get(1)?;
    let third = tokens.get(2)?;
    let pipe_idx = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Pipe)?;
    if pipe_idx < 3 {
        return None;
    }

    let min = match first.kind {
        TokenKind::Number => first.parser_text().parse::<i32>().ok()?,
        _ => return None,
    };
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

    let trailing_tokens = trim_lexed_commas(&tokens[pipe_idx + 1..]);
    if trailing_tokens.is_empty() {
        return None;
    }

    Some((
        IfResultPredicate::Value(Comparison::BetweenInclusive(min, max)),
        trailing_tokens,
    ))
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
    let parse_effects_with_leading_instead =
        |effect_tokens: &[OwnedLexToken],
         parse_effects: &mut dyn FnMut(
            &[OwnedLexToken],
        ) -> Result<Vec<EffectAst>, CardTextError>| {
            let trimmed = trim_lexed_commas(effect_tokens);
            let without_instead = if trimmed
                .first()
                .is_some_and(|token| INSTEAD_WORD_PATTERN.matches_token(token))
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
                && let Ok(effects) = parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
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
                if let Ok(effects) = parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
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
            let comma_fragment_looks_like_effect = if comma_indices.len() > 1 {
                let fragment_tokens = &tokens[first_comma_idx + 1..comma_indices[1]];
                parse_effects_with_leading_instead(fragment_tokens, &mut parse_effects)
                    .map(|effects| !effects.is_empty())
                    .unwrap_or(false)
            } else {
                true
            };
            let comma_fragment_looks_like_delayed_trigger = effect_tokens
                .first()
                .is_some_and(|token| token.is_word("when") || token.is_word("whenever"));
            if (comma_fragment_looks_like_effect || comma_fragment_looks_like_delayed_trigger)
                && let Ok(effects) = parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
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
    let predicate_words = TokenWordView::new(predicate_tokens).to_word_refs();
    if !words_contain_phrase(&predicate_words, &["fewer", "than"])
        && !words_contain_phrase(&predicate_words, &["less", "than"])
    {
        return None;
    }

    let effect_words = TokenWordView::new(trim_lexed_commas(effect_tokens)).to_word_refs();
    let subject = match effect_words.as_slice() {
        ["draw", "cards", "equal", "to", "the", "difference"]
        | ["draw", "cards", "equal", "to", "difference"] => PlayerAst::Implicit,
        ["you", "draw", "cards", "equal", "to", "the", "difference"]
        | ["you", "draw", "cards", "equal", "to", "difference"] => PlayerAst::You,
        _ => return None,
    };
    let hand_player = match player {
        PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
        _ => return None,
    };
    let threshold = (*count as i32) + 1;
    let difference = Value::Add(
        Box::new(Value::Fixed(threshold)),
        Box::new(Value::Scaled(
            Box::new(Value::CardsInHand(hand_player)),
            -1,
        )),
    )
    .with_surface_hint(ValueSurfaceHint::Difference);

    Some(vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        subject,
        SubjectVerbActionAst::Draw { count: difference },
    )])
}

fn words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool {
    !phrase.is_empty() && words.windows(phrase.len()).any(|window| window == phrase)
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
        .is_some_and(|token| IF_WORD_PATTERN.matches_token(token))
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
        .is_some_and(|token| INSTEAD_WORD_PATTERN.matches_token(token))
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
    let split_idx =
        primitives::rfind_token_index(tokens, |token| token_matches_dynamic_word(token, keyword))?;
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

pub(crate) fn parse_who_player_predicate_lexed(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let trimmed = trim_lexed_commas(tokens);
    if !trimmed
        .first()
        .is_some_and(|token| WHO_WORD_PATTERN.matches_token(token))
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
        .is_some_and(|token| INSTEAD_WORD_PATTERN.matches_token(token))
        || !trimmed
            .get(1)
            .is_some_and(|token| IF_WORD_PATTERN.matches_token(token))
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

    if let Some(while_idx) = find_token_shape(trigger_tokens, &WHILE_WORD_PATTERN)
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
            .is_some_and(|token| AND_OR_THEN_WORD_PATTERN.matches_token(token))
        {
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
        .is_some_and(|token| WHEN_OR_WHENEVER_WORD_PATTERN.matches_token(token))
    {
        return None;
    }

    let trigger_tokens = &tokens[start_idx..split_idx];
    let effects_tokens = trim_lexed_commas(&tokens[split_idx + 1..]);
    if effects_tokens.is_empty() {
        return None;
    }

    let predicate = if let Some(comma_idx) = trigger_tokens
        .iter()
        .position(|token| token.kind == TokenKind::Comma)
        && trigger_tokens
            .get(comma_idx + 1)
            .is_some_and(|token| IF_WORD_PATTERN.matches_token(token))
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
    let sentence_start =
        primitives::rfind_token_index(tokens, |token| token.kind == TokenKind::Period)
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
        .filter_map(|(idx, token)| CHOOSE_WORD_PATTERN.matches_token(token).then_some(idx))
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
