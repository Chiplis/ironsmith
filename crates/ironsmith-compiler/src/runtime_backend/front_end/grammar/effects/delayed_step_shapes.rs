use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::PlayerAst;
use crate::runtime_backend::front_end::grammar::{leaf, primitives};
use crate::runtime_backend::front_end::lexer::{LexStream, LexedClause, OwnedLexToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedActionShape {
    Pay,
    Draw,
    Discard,
    Sacrifice,
    CastOrPlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedCreatureTypesShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) gain: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedLosingPumpShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) modifier: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LoseDrawClashShape {
    pub(crate) life_count: i32,
    pub(crate) draw_count: i32,
    pub(crate) repeat_if_win: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImplicitBecomePrefixShape {
    pub(crate) consumed: usize,
    pub(crate) negated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedUpkeepPaymentShape<'a> {
    pub(crate) mana_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedPaymentActionSplit<'a> {
    pub(crate) player_tokens: &'a [OwnedLexToken],
    pub(crate) action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImplicitBecomeSubjectKind {
    Source,
    Tagged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImplicitBecomeSubjectShape<'a> {
    pub(crate) kind: ImplicitBecomeSubjectKind,
    pub(crate) remainder_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedTimingStepShape {
    Upkeep,
    DrawStep,
    EndStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedTimingMarkerShape {
    pub(crate) start_word: usize,
    pub(crate) end_word: usize,
    pub(crate) step: DelayedTimingStepShape,
    pub(crate) player: PlayerAst,
}

const KNOWN_FALLBACK_MARKER_PREFIXES: &[&[&str]] = &[
    &[
        "chooses",
        "any",
        "number",
        "of",
        "creatures",
        "they",
        "control",
    ],
    &[
        "each",
        "player",
        "chooses",
        "any",
        "number",
        "of",
        "creatures",
        "they",
        "control",
    ],
    &["an", "opponent", "chooses", "one", "of", "those", "piles"],
    &["put", "that", "pile", "into", "your", "hand"],
    &["cast", "that", "card", "for", "as", "long", "as"],
    &[
        "until", "end", "of", "turn", "this", "creature", "loses", "prevent", "all", "damage",
    ],
    &[
        "until",
        "end",
        "of",
        "turn",
        "target",
        "creature",
        "loses",
        "all",
        "abilities",
        "and",
        "has",
        "base",
        "power",
        "and",
        "toughness",
    ],
    &["for", "each", "1", "damage", "prevented", "this", "way"],
    &[
        "for", "each", "card", "less", "than", "two", "a", "player", "draws", "this", "way",
    ],
    &["this", "deals", "4", "damage", "if", "there", "are"],
    &[
        "this", "deals", "4", "damage", "instead", "if", "there", "are",
    ],
    &[
        "the", "next", "spell", "you", "cast", "this", "turn", "costs",
    ],
    &[
        "that",
        "creature",
        "attacks",
        "during",
        "its",
        "controllers",
        "next",
        "combat",
        "phase",
        "if",
        "able",
    ],
    &[
        "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to", "target",
        "creature", "you", "control", "by", "a", "source", "of", "your", "choice", "is", "dealt",
        "to", "another", "target", "creature", "instead",
    ],
];

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    LexedClause::new(tokens).trimmed().tokens()
}

fn punctuation<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::comma(),
        primitives::period(),
        primitives::semicolon(),
    ))
    .void()
    .parse_next(input)
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    (
        repeat::<_, _, (), _, _>(0.., punctuation),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn exact_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::parse_all(
        trimmed(tokens),
        (
            semantic_phrase(phrase),
            repeat::<_, _, (), _, _>(0.., punctuation),
            eof,
        )
            .void(),
        "delayed-step phrase",
    )
    .is_ok()
}

fn words_to_tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    crate::runtime_backend::front_end::lexer::synthetic_word_tokens(words.iter().copied())
}

#[path = "delayed_step_shapes/timing_and_subjects.rs"]
mod timing_and_subjects;
pub(crate) use timing_and_subjects::*;

pub(crate) fn is_delayed_lose_game_unless_paid_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_phrase(tokens, &["if", "you", "dont", "you", "lose", "the", "game"])
        || exact_phrase(
            tokens,
            &["if", "you", "do", "not", "you", "lose", "the", "game"],
        )
}

fn action_parser<'a>(
    kind: DelayedActionShape,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| match kind {
        DelayedActionShape::Pay => alt((primitives::kw("pay"), primitives::kw("pays")))
            .void()
            .parse_next(input),
        DelayedActionShape::Draw => alt((primitives::kw("draw"), primitives::kw("draws")))
            .void()
            .parse_next(input),
        DelayedActionShape::Discard => alt((primitives::kw("discard"), primitives::kw("discards")))
            .void()
            .parse_next(input),
        DelayedActionShape::Sacrifice => {
            alt((primitives::kw("sacrifice"), primitives::kw("sacrifices")))
                .void()
                .parse_next(input)
        }
        DelayedActionShape::CastOrPlay => alt((
            primitives::kw("may"),
            primitives::kw("cast"),
            primitives::kw("casts"),
            primitives::kw("casting"),
            primitives::kw("play"),
            primitives::kw("plays"),
            primitives::kw("playing"),
            primitives::kw("played"),
        ))
        .void()
        .parse_next(input),
    }
}

pub(crate) fn delayed_action_shape(
    tokens: &[OwnedLexToken],
    kind: DelayedActionShape,
    must_start: bool,
) -> bool {
    if must_start {
        primitives::parse_prefix(trimmed(tokens), action_parser(kind)).is_some()
    } else {
        primitives::find_prefix(trimmed(tokens), || action_parser(kind)).is_some()
    }
}

pub(crate) fn delayed_mentions_mana_cost_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        (
            primitives::kw("mana"),
            alt((primitives::kw("cost"), primitives::kw("costs"))),
        )
            .void()
    })
    .is_some()
}

pub(crate) fn delayed_exact_shape(
    tokens: &[OwnedLexToken],
    phrase: &'static [&'static str],
) -> bool {
    exact_phrase(tokens, phrase)
}

pub(crate) fn delayed_starts_any_shape(
    tokens: &[OwnedLexToken],
    phrases: &'static [&'static [&'static str]],
) -> bool {
    phrases
        .iter()
        .any(|phrase| primitives::parse_prefix(trimmed(tokens), semantic_phrase(phrase)).is_some())
}

pub(crate) fn delayed_mentions_remains_tapped_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || primitives::kw("remains")).is_some()
        && primitives::find_prefix(tokens, || primitives::kw("tapped")).is_some()
}

pub(crate) fn delayed_referential_sacrifice_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_phrase(tokens, &["sacrifice", "it"])
        || exact_phrase(tokens, &["sacrifice", "that", "card"])
        || exact_phrase(tokens, &["sacrifice", "that", "token"])
}

pub(crate) fn parse_implicit_become_prefix_words(words: &[&str]) -> ImplicitBecomePrefixShape {
    let tokens = words_to_tokens(words);
    let tokens = trimmed(&tokens);
    let mut consumed = 0usize;
    let mut rest = tokens;
    if let Some(((), after)) = primitives::parse_prefix(rest, primitives::kw("still").void()) {
        consumed += rest.len().saturating_sub(after.len());
        rest = after;
    }
    if let Some(((), after)) = primitives::parse_prefix(
        rest,
        alt((
            primitives::phrase(&["is", "not"]),
            primitives::phrase(&["are", "not"]),
        ))
        .void(),
    ) {
        consumed += rest.len().saturating_sub(after.len());
        return ImplicitBecomePrefixShape {
            consumed,
            negated: true,
        };
    }
    if let Some(((), after)) = primitives::parse_prefix(
        rest,
        alt((semantic_kw("isnt"), semantic_kw("arent"))).void(),
    ) {
        consumed += rest.len().saturating_sub(after.len());
        return ImplicitBecomePrefixShape {
            consumed,
            negated: true,
        };
    }
    if let Some(((), after)) = primitives::parse_prefix(
        rest,
        alt((
            primitives::kw("is"),
            primitives::kw("are"),
            primitives::kw("s"),
        ))
        .void(),
    ) {
        consumed += rest.len().saturating_sub(after.len());
    }
    ImplicitBecomePrefixShape {
        consumed,
        negated: false,
    }
}

fn suffix_len(words: &[&str], phrases: &'static [&'static [&'static str]]) -> Option<usize> {
    let tokens = words_to_tokens(words);
    for phrase in phrases {
        if primitives::split_lexed_once_before_suffix(&tokens, 0, || {
            (semantic_phrase(phrase), eof).void()
        })
        .is_some()
        {
            return Some(phrase.len());
        }
    }
    None
}

pub(crate) fn delayed_until_eot_suffix_len(words: &[&str]) -> Option<usize> {
    suffix_len(words, &[&["until", "end", "of", "turn"]])
}

pub(crate) fn delayed_addition_other_types_suffix_len(words: &[&str]) -> Option<usize> {
    suffix_len(
        words,
        &[
            &["in", "addition", "to", "its", "other", "types"],
            &["in", "addition", "to", "their", "other", "types"],
            &["in", "addition", "to", "its", "other", "type"],
            &["in", "addition", "to", "their", "other", "type"],
        ],
    )
}

pub(crate) fn delayed_article_shape(word: &str) -> bool {
    matches!(word, "a" | "an" | "the")
}

pub(crate) fn delayed_negative_type_prefix_len(
    words: &[&str],
    already_negated: bool,
) -> Option<usize> {
    if already_negated {
        return Some(usize::from(
            words
                .first()
                .is_some_and(|word| delayed_article_shape(word)),
        ));
    }
    let tokens = words_to_tokens(words);
    if primitives::parse_prefix(
        &tokens,
        alt((
            primitives::phrase(&["not", "a"]),
            primitives::phrase(&["not", "an"]),
        )),
    )
    .is_some()
    {
        return Some(2);
    }
    primitives::parse_prefix(&tokens, primitives::kw("not")).map(|_| 1)
}

pub(crate) fn parse_delayed_creature_types_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedCreatureTypesShape<'_>> {
    let tokens = trimmed(tokens);
    let (verb_idx, gain, after_verb) = primitives::find_prefix(tokens, || {
        alt((
            alt((primitives::kw("gain"), primitives::kw("gains"))).value(true),
            alt((primitives::kw("lose"), primitives::kw("loses"))).value(false),
        ))
    })?;
    let tail_ok = primitives::parse_all(
        trimmed(after_verb),
        (
            alt((
                primitives::phrase(&["all", "creature", "types", "until", "end", "of", "turn"]),
                primitives::phrase(&["every", "creature", "type", "until", "end", "of", "turn"]),
            )),
            primitives::sentence_end(),
        )
            .void(),
        "all creature types duration",
    )
    .is_ok();
    (tail_ok && verb_idx > 0).then_some(DelayedCreatureTypesShape {
        subject_tokens: trimmed(&tokens[..verb_idx]),
        gain,
    })
}

pub(crate) fn parse_delayed_losing_pump_shape(
    subject_tokens: &[OwnedLexToken],
) -> Option<DelayedLosingPumpShape<'_>> {
    let (get_idx, _, after_get) = primitives::find_prefix(subject_tokens, || {
        alt((primitives::kw("get"), primitives::kw("gets")))
    })?;
    let modifier = after_get.first()?.as_word()?;
    let target_tokens = trimmed(&subject_tokens[..get_idx]);
    (!target_tokens.is_empty()).then_some(DelayedLosingPumpShape {
        target_tokens,
        modifier,
    })
}

pub(crate) fn delayed_tagged_creature_reference_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_phrase(tokens, &["it"]) || exact_phrase(tokens, &["that", "creature"])
}

fn parse_lose_draw_clash<'a>(input: &mut LexStream<'a>) -> WResult<LoseDrawClashShape> {
    primitives::phrase(&["you", "lose"]).parse_next(input)?;
    let life_count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)? as i32;
    primitives::phrase(&["life", "and", "draw"]).parse_next(input)?;
    let draw_count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)? as i32;
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["then", "clash", "with", "an", "opponent"]).parse_next(input)?;
    let repeat_if_win = opt((
        opt(primitives::period()),
        primitives::phrase(&["if", "you", "win"]),
        opt(primitives::comma()),
        primitives::phrase(&["repeat", "this", "process"]),
    ))
    .parse_next(input)?
    .is_some();
    primitives::sentence_end().parse_next(input)?;
    Ok(LoseDrawClashShape {
        life_count,
        draw_count,
        repeat_if_win,
    })
}

pub(crate) fn parse_lose_draw_clash_shape(tokens: &[OwnedLexToken]) -> Option<LoseDrawClashShape> {
    primitives::parse_all(
        trimmed(tokens),
        parse_lose_draw_clash,
        "lose draw clash repeat",
    )
    .ok()
}

#[cfg(test)]
#[path = "delayed_step_shapes/tests.rs"]
mod tests;
