use std::ops::Range;

use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::color::{Color, ColorSet};
use crate::target::ObjectFilter;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::util::starts_filter_keyword_list_continuation_words;
use super::primitives::{self, TokenWordView, WordSliceInput};

#[path = "clause_support/ability_shapes.rs"]
mod ability_shapes;
pub use ability_shapes::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionTargetKind {
    EachManaValueAmong { filter_word_first: usize },
    Spell,
    PermanentCastThisTurn,
    ManaValue { comparison_word_first: usize },
    PermanentWithCounter { counter_word_first: usize },
    ChosenPlayer,
    ChosenColor,
    Colorless,
    Everything,
    AllColors,
    Named,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtectionTarget<'a> {
    pub value: &'a str,
    pub target_word: usize,
    pub target_token_first: usize,
    pub kind: ProtectionTargetKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectionChain<'a> {
    pub targets: Vec<ProtectionTarget<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenSpan {
    pub first: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerIntro {
    pub body_first: usize,
    pub is_non_at_intro: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTriggerKind {
    BecomesBlocked,
    LeavesBattlefield,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceTriggerPrefix {
    pub kind: SourceTriggerKind,
    pub effect_first: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerDelimiterKind {
    Comma,
    Then,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerDelimiter {
    pub index: usize,
    pub kind: TriggerDelimiterKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerDelimiterFacts {
    pub first_comma: Option<usize>,
    pub first_comma_or_then: Option<TriggerDelimiter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackWithShape {
    pub subject_words: Range<usize>,
    pub attacked_words: Option<Range<usize>>,
    pub object_token_first: usize,
}

pub fn parse_color_only_hexproof_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    primitives::parse_full_word_slice(words, parse_color_only_hexproof_filter_word_slice)
}

fn parse_color_only_hexproof_filter_word_slice(
    input: &mut WordSliceInput<'_>,
) -> WResult<ObjectFilter> {
    let mut each_color = *input;
    if (
        primitives::word_slice_exact("each"),
        primitives::word_slice_exact("color"),
        primitives::word_slice_eof,
    )
        .parse_next(&mut each_color)
        .is_ok()
    {
        *input = each_color;
        let colors = Color::ALL
            .into_iter()
            .fold(ColorSet::new(), |set, color| set.with(color));
        let mut filter = ObjectFilter::default();
        filter.colors = Some(colors);
        return Ok(filter);
    }

    let mut filters = Vec::new();
    while !input.is_empty() {
        let word: &str = any.parse_next(input)?;
        if matches!(word, "and" | "from") {
            continue;
        }
        let color = super::leaf::parse_leaf_color_complete(word)
            .map_err(|_| primitives::backtrack_err("hexproof color", "Magic color"))?;
        let mut filter = ObjectFilter::default();
        filter.colors = Some(color);
        filters.push(filter);
    }
    match filters.len() {
        0 => Err(primitives::backtrack_err(
            "hexproof color",
            "one or more colors",
        )),
        1 => Ok(filters.pop().expect("single parsed color")),
        _ => {
            let mut filter = ObjectFilter::default();
            filter.any_of = filters;
            Ok(filter)
        }
    }
}

pub fn parse_protection_chain_tokens(tokens: &[OwnedLexToken]) -> Option<ProtectionChain<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let from_words = parse_protection_from_words(&words)?;
    let mut targets = Vec::with_capacity(from_words.len());
    for from_word in from_words {
        let target_word = from_word + 1;
        let value = *words.get(target_word)?;
        let target_token_first = *view.token_start_indices().get(target_word)?;
        targets.push(ProtectionTarget {
            value,
            target_word,
            target_token_first,
            kind: classify_protection_target(&words, target_word),
        });
    }
    (!targets.is_empty()).then_some(ProtectionChain { targets })
}

pub fn parse_ability_segments_tokens(tokens: &[OwnedLexToken]) -> Vec<TokenSpan> {
    parse_token_segments(tokens, SegmentDelimiter::CommaOrSemicolon)
}

pub fn parse_conjoined_segments_tokens(tokens: &[OwnedLexToken]) -> Vec<TokenSpan> {
    parse_token_segments(tokens, SegmentDelimiter::And)
}

pub fn parse_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> TriggerIntro {
    let mut input = LexStream::new(tokens);
    let intro = crate::grammar::primitives::take_leaf(&mut input, parse_trigger_intro_lexed);
    TriggerIntro {
        body_first: intro.map_or(0, |(_, first)| first),
        is_non_at_intro: intro.is_some_and(|(non_at, _)| non_at),
    }
}

pub fn parse_monstrous_damage_hand_trigger_tokens(tokens: &[OwnedLexToken]) -> bool {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: WordSliceInput<'_> = &words;
    if parse_word_phrase(
        &mut input,
        &[
            "when",
            "this",
            "becomes",
            "monstrous",
            "it",
            "deals",
            "damage",
            "to",
            "each",
            "opponent",
            "equal",
            "to",
        ],
    )
    .is_err()
    {
        return false;
    }
    word_occurs(&words, &["number"])
        && word_occurs(&words, &["cards"])
        && word_occurs(&words, &["hand"])
}

pub fn parse_combined_x_cost_trigger_tokens(tokens: &[OwnedLexToken]) -> Option<usize> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    phrase_occurs_normalized(
        &words,
        &[
            "you", "cast", "an", "instant", "or", "sorcery", "spell", "or", "activate", "an",
            "ability",
        ],
    )
    .then_some(())?;
    phrase_occurs_normalized(
        &words,
        &[
            "that",
            "spells",
            "mana",
            "cost",
            "or",
            "that",
            "abilitys",
            "activation",
            "cost",
            "contains",
        ],
    )
    .then_some(())?;
    let copy_word = normalized_phrase_offset(&words, &["copy", "that", "spell", "or", "ability"])?;
    view.token_start_indices().get(copy_word).copied()
}

pub fn parse_source_trigger_prefix_tokens(tokens: &[OwnedLexToken]) -> Option<SourceTriggerPrefix> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let (kind, word_count) =
        if word_phrase_prefix(&words, &["this", "creature", "becomes", "blocked"]) {
            (SourceTriggerKind::BecomesBlocked, 4)
        } else if word_phrase_prefix(&words, &["this", "becomes", "blocked"]) {
            (SourceTriggerKind::BecomesBlocked, 3)
        } else if word_phrase_prefix(
            &words,
            &["this", "creature", "leaves", "the", "battlefield"],
        ) {
            (SourceTriggerKind::LeavesBattlefield, 5)
        } else if word_phrase_prefix(&words, &["this", "leaves", "the", "battlefield"]) {
            (SourceTriggerKind::LeavesBattlefield, 4)
        } else {
            return None;
        };
    let effect_first = view.token_index_after_words_or_end(word_count)?;
    Some(SourceTriggerPrefix { kind, effect_first })
}

pub fn parse_blocked_damage_effect_tokens(tokens: &[OwnedLexToken]) -> bool {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: WordSliceInput<'_> = &words;
    parse_word_phrase(
        &mut input,
        &[
            "it",
            "deals",
            "2",
            "damage",
            "to",
            "each",
            "attacking",
            "creature",
            "and",
            "each",
            "blocking",
            "creature",
        ],
    )
    .is_ok()
        && input.is_empty()
}

pub fn parse_trigger_delimiters_tokens(tokens: &[OwnedLexToken]) -> TriggerDelimiterFacts {
    let mut input = LexStream::new(tokens);
    parse_trigger_delimiter_facts_lexed(&mut input)
}

pub fn parse_attack_with_shape_tokens(tokens: &[OwnedLexToken]) -> Option<AttackWithShape> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let (attack_word, with_word) = parse_attack_with_words(&words)?;
    let object_word = with_word + 1;
    let object_token_first = *view.token_start_indices().get(object_word)?;
    Some(AttackWithShape {
        subject_words: 0..attack_word,
        attacked_words: (with_word > attack_word + 1).then_some(attack_word + 1..with_word),
        object_token_first,
    })
}

fn parse_protection_from_words(words: &[&str]) -> Option<Vec<usize>> {
    let mut input: WordSliceInput<'_> = words;
    let initial_len = input.len();
    let mut from_words = vec![crate::grammar::primitives::take_leaf(
        &mut input,
        |input: &mut _| parse_protection_head_word_stream(input, initial_len),
    )?];
    while let Ok(word) = take_word(&mut input) {
        if word == "from" {
            from_words.push(initial_len.saturating_sub(input.len() + 1));
        }
    }
    Some(from_words)
}

fn parse_protection_head_word_stream(
    input: &mut WordSliceInput<'_>,
    initial_len: usize,
) -> WResult<usize> {
    if input.first().copied() == Some("and") {
        primitives::word_slice_exact("and").parse_next(input)?;
    }
    primitives::word_slice_exact("protection").parse_next(input)?;
    let from_word = initial_len.saturating_sub(input.len());
    primitives::word_slice_exact("from").parse_next(input)?;
    Ok(from_word)
}

fn classify_protection_target(words: &[&str], target_word: usize) -> ProtectionTargetKind {
    let tail = words.get(target_word..).unwrap_or_default();
    if word_phrase_prefix(tail, &["each", "mana", "value", "among"]) {
        return ProtectionTargetKind::EachManaValueAmong {
            filter_word_first: target_word + 4,
        };
    }
    if word_phrase_prefix(tail, &["spell"]) || word_phrase_prefix(tail, &["spells"]) {
        return ProtectionTargetKind::Spell;
    }
    if word_phrase_prefix(tail, &["permanent", "that", "were", "cast", "this", "turn"])
        || word_phrase_prefix(
            tail,
            &["permanents", "that", "were", "cast", "this", "turn"],
        )
    {
        return ProtectionTargetKind::PermanentCastThisTurn;
    }
    if word_phrase_prefix(tail, &["mana", "value"]) {
        return ProtectionTargetKind::ManaValue {
            comparison_word_first: target_word + 2,
        };
    }
    if (word_phrase_prefix(tail, &["permanent", "with"])
        || word_phrase_prefix(tail, &["permanents", "with"]))
        && tail.len() > 2
    {
        return ProtectionTargetKind::PermanentWithCounter {
            counter_word_first: target_word + 2,
        };
    }
    if word_phrase_prefix(tail, &["the", "chosen", "player"]) {
        return ProtectionTargetKind::ChosenPlayer;
    }
    if word_phrase_prefix(tail, &["the", "chosen", "color"])
        || word_phrase_prefix(tail, &["the", "last", "chosen", "color"])
    {
        return ProtectionTargetKind::ChosenColor;
    }
    if word_phrase_prefix(tail, &["all", "color"]) || word_phrase_prefix(tail, &["all", "colors"]) {
        return ProtectionTargetKind::AllColors;
    }
    match words.get(target_word).copied() {
        Some("colorless") => ProtectionTargetKind::Colorless,
        Some("everything") => ProtectionTargetKind::Everything,
        _ => ProtectionTargetKind::Named,
    }
}

#[derive(Debug, Clone, Copy)]
enum SegmentDelimiter {
    CommaOrSemicolon,
    And,
}

fn parse_token_segments(tokens: &[OwnedLexToken], delimiter: SegmentDelimiter) -> Vec<TokenSpan> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    let mut first = 0usize;
    let mut spans = Vec::new();
    while let Ok(token) = take_token(&mut input) {
        let end = initial_len.saturating_sub(input.len() + 1);
        let matches = match delimiter {
            SegmentDelimiter::CommaOrSemicolon => {
                matches!(token.kind, TokenKind::Comma | TokenKind::Semicolon)
            }
            SegmentDelimiter::And => token.is_word("and"),
        };
        if matches {
            if first < end {
                spans.push(TokenSpan { first, end });
            }
            first = end + 1;
        }
    }
    if first < tokens.len() {
        spans.push(TokenSpan {
            first,
            end: tokens.len(),
        });
    }
    spans
}

fn parse_trigger_intro_lexed(input: &mut LexStream<'_>) -> WResult<(bool, usize)> {
    let token = take_token(input)?;
    if !token.is_any_word(&["whenever", "when", "at"]) {
        return Err(backtrack());
    }
    Ok((!token.is_word("at"), 1))
}

fn parse_trigger_delimiter_facts_lexed(input: &mut LexStream<'_>) -> TriggerDelimiterFacts {
    let initial_len = input.len();
    let mut first_comma = None;
    let mut first_comma_or_then = None;
    while let Ok(token) = take_token(input) {
        let index = initial_len.saturating_sub(input.len() + 1);
        let kind = if token.kind == TokenKind::Comma {
            let continuation_words = TokenWordView::new(input.as_ref()).to_word_refs();
            if starts_filter_keyword_list_continuation_words(&continuation_words) {
                continue;
            }
            first_comma.get_or_insert(index);
            Some(TriggerDelimiterKind::Comma)
        } else if token.is_word("then") {
            Some(TriggerDelimiterKind::Then)
        } else {
            None
        };
        if first_comma_or_then.is_none()
            && let Some(kind) = kind
        {
            first_comma_or_then = Some(TriggerDelimiter { index, kind });
        }
    }
    TriggerDelimiterFacts {
        first_comma,
        first_comma_or_then,
    }
}

fn parse_attack_with_words(words: &[&str]) -> Option<(usize, usize)> {
    let mut input: WordSliceInput<'_> = words;
    let initial_len = input.len();
    let attack_word = loop {
        let index = initial_len.saturating_sub(input.len());
        let word = crate::grammar::primitives::take_leaf(&mut input, take_word)?;
        if matches!(word, "attack" | "attacks") {
            break index;
        }
    };
    let with_word = loop {
        let index = initial_len.saturating_sub(input.len());
        let word = crate::grammar::primitives::take_leaf(&mut input, take_word)?;
        if word == "with" {
            break index;
        }
    };
    Some((attack_word, with_word))
}

fn word_phrase_prefix(words: &[&str], expected: &'static [&'static str]) -> bool {
    let mut input: WordSliceInput<'_> = words;
    parse_word_phrase(&mut input, expected).is_ok()
}

fn parse_word_phrase(
    input: &mut WordSliceInput<'_>,
    expected: &'static [&'static str],
) -> WResult<()> {
    for word in expected {
        primitives::word_slice_exact(word)
            .void()
            .parse_next(input)?;
    }
    Ok(())
}

fn word_occurs(words: &[&str], expected: &'static [&'static str]) -> bool {
    let mut input: WordSliceInput<'_> = words;
    while let Ok(word) = take_word(&mut input) {
        if expected.contains(&word) {
            return true;
        }
    }
    false
}

fn phrase_occurs_normalized(words: &[&str], expected: &[&str]) -> bool {
    normalized_phrase_offset(words, expected).is_some()
}

fn normalized_phrase_offset(words: &[&str], expected: &[&str]) -> Option<usize> {
    let mut input: WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input;
        if parse_normalized_phrase(&mut candidate, expected).is_ok() {
            return Some(offset);
        }
        crate::grammar::primitives::take_leaf(&mut input, take_word)?;
    }
}

fn parse_normalized_phrase(input: &mut WordSliceInput<'_>, expected: &[&str]) -> WResult<()> {
    for expected_word in expected {
        let word = take_word(input)?;
        let normalized = word.replace(['\'', '’'], "");
        if normalized != *expected_word {
            return Err(backtrack());
        }
    }
    Ok(())
}

fn backtrack() -> ErrMode<ContextError> {
    ErrMode::Backtrack(ContextError::new())
}

fn take_word<'a>(input: &mut WordSliceInput<'a>) -> WResult<&'a str> {
    any.parse_next(input)
}

fn take_token<'a>(input: &mut LexStream<'a>) -> WResult<&'a OwnedLexToken> {
    any.parse_next(input)
}

#[cfg(test)]
#[path = "clause_support/tests.rs"]
mod tests;
