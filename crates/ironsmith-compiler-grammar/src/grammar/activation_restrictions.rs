use winnow::combinator::{alt, eof};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, literal, rest, take_till};

use crate::ability::ActivationTiming;
use crate::color::Color;
use crate::target::ObjectFilter;

use super::super::lexer::{LexStream, OwnedLexToken};
use super::{leaf, primitives};

#[path = "activation_restrictions/cast_facts.rs"]
mod cast_facts;
pub use cast_facts::*;

#[path = "activation_restrictions/clause_facts.rs"]
mod clause_facts;
pub use clause_facts::*;

#[path = "activation_restrictions/object_facts.rs"]
mod object_facts;
pub use object_facts::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivationNegationSpan {
    pub first: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivationWordSpan {
    pub first: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CantRestrictionOrSplit {
    pub first: Vec<OwnedLexToken>,
    pub second: Vec<OwnedLexToken>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivationCastLimitQualifier {
    pub filter: ObjectFilter,
    pub consumed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticRestrictionConditionKind {
    If,
    AsLongAs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticRestrictionConditionShape {
    Timing {
        timing: ActivationTiming,
        remainder_first: usize,
        remainder_end: usize,
    },
    Condition {
        kind: StaticRestrictionConditionKind,
        condition: ActivationWordSpan,
        remainder_first: usize,
    },
    ExtraTurn {
        remainder_first: usize,
        remainder_end: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaRetentionSubject {
    You,
    AnyPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnspentManaRetentionTail {
    pub color: Option<Color>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnspentManaRetentionStatic {
    pub subject: ManaRetentionSubject,
    pub color: Option<Color>,
}

pub fn parse_unspent_mana_retention_tail_words(words: &[&str]) -> Option<UnspentManaRetentionTail> {
    primitives::parse_full_word_slice(words, parse_unspent_mana_retention_tail_word_slice)
}

pub fn parse_unspent_mana_retention_static_words(
    words: &[&str],
) -> Option<UnspentManaRetentionStatic> {
    primitives::parse_full_word_slice(words, parse_unspent_mana_retention_static_word_slice)
}

fn parse_unspent_mana_retention_static_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<UnspentManaRetentionStatic> {
    let subject = alt((
        primitives::word_slice_exact("you").value(ManaRetentionSubject::You),
        primitives::word_slice_exact("players").value(ManaRetentionSubject::AnyPlayer),
        (
            primitives::word_slice_exact("each"),
            primitives::word_slice_exact("player"),
        )
            .value(ManaRetentionSubject::AnyPlayer),
    ))
    .parse_next(input)?;
    alt((
        primitives::word_slice_exact("dont").void(),
        primitives::word_slice_exact("don't").void(),
        (
            primitives::word_slice_exact("do"),
            primitives::word_slice_exact("not"),
        )
            .void(),
    ))
    .parse_next(input)?;
    let tail = parse_unspent_mana_retention_tail_word_slice.parse_next(input)?;
    Ok(UnspentManaRetentionStatic {
        subject,
        color: tail.color,
    })
}

fn parse_unspent_mana_retention_tail_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<UnspentManaRetentionTail> {
    primitives::word_slice_exact("lose").parse_next(input)?;
    primitives::word_slice_exact("unspent").parse_next(input)?;
    let color = winnow::combinator::opt(parse_retained_mana_color_word).parse_next(input)?;
    primitives::word_slice_exact("mana").parse_next(input)?;
    (
        primitives::word_slice_exact("as"),
        primitives::word_slice_exact("steps"),
    )
        .parse_next(input)?;
    winnow::combinator::opt((
        primitives::word_slice_exact("and"),
        primitives::word_slice_exact("phases"),
        primitives::word_slice_exact("end"),
    ))
    .parse_next(input)?;
    Ok(UnspentManaRetentionTail { color })
}

fn parse_retained_mana_color_word(input: &mut primitives::WordSliceInput<'_>) -> WResult<Color> {
    let word: &str = any.parse_next(input)?;
    let color_set = super::leaf::parse_leaf_color_complete(word)
        .map_err(|_| primitives::backtrack_err("retained mana color", "Magic color"))?;
    for color in Color::ALL {
        if color_set == crate::color::ColorSet::from_color(color) {
            return Ok(color);
        }
    }
    Err(primitives::backtrack_err(
        "retained mana color",
        "single Magic color",
    ))
}

pub fn parse_activation_negation_span_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivationNegationSpan> {
    let mut input = LexStream::new(tokens);
    parse_activation_negation_span_lexed
        .parse_next(&mut input)
        .ok()
}

pub fn parse_activation_cast_limit_qualifier_words(
    words: &[&str],
) -> Option<ActivationCastLimitQualifier> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let filter = parse_cast_limit_qualifier_word_slice
        .parse_next(&mut input)
        .ok()?;
    Some(ActivationCastLimitQualifier {
        filter,
        consumed: words.len().checked_sub(input.len())?,
    })
}

pub fn parse_activation_possessive_owner_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    if let Some(last) = normalized.last_mut()
        && let Some(word) = last.as_word()
        && let Ok(stem) = parse_possessive_owner_stem.parse(word)
    {
        last.replace_word(&stem);
    }
    normalized
}

pub fn parse_static_restriction_condition_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StaticRestrictionConditionShape> {
    for parser in [
        parse_during_extra_turns_prefix_lexed,
        parse_during_your_turn_prefix_lexed,
        parse_if_condition_prefix_lexed,
        parse_during_combat_prefix_lexed,
        parse_during_your_turn_suffix_lexed,
        parse_during_combat_suffix_lexed,
        parse_during_extra_turns_suffix_lexed,
        parse_as_long_as_condition_prefix_lexed,
    ] {
        if let Ok(shape) = primitives::parse_all(tokens, parser, "static-restriction-condition") {
            return Some(shape);
        }
    }
    None
}

fn parse_during_extra_turns_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<StaticRestrictionConditionShape> {
    let initial_len = input.len();
    primitives::phrase(&["during", "extra", "turns"]).parse_next(input)?;
    while input.peek_token().is_some_and(|token| token.is_comma()) {
        any.parse_next(input)?;
    }
    let remainder_first = initial_len.saturating_sub(input.len());
    let _: Vec<&OwnedLexToken> = winnow::combinator::repeat(0.., any).parse_next(input)?;
    Ok(StaticRestrictionConditionShape::ExtraTurn {
        remainder_first,
        remainder_end: initial_len,
    })
}

pub fn parse_source_attached_to_creature_condition_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_source_attached_to_creature_condition_lexed,
        "source-attached-condition",
    )
    .is_ok()
}

pub fn parse_cant_restriction_or_split_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CantRestrictionOrSplit> {
    let negation = parse_activation_negation_span_tokens(tokens)?;
    let subject = trim_commas(&tokens[..negation.first]);
    let remainder = trim_commas(&tokens[negation.end..]);
    if primitives::parse_prefix(remainder, primitives::phrase(&["attack", "or", "block"])).is_some()
    {
        return None;
    }
    let or_offset = find_restriction_or_lexed(remainder)?;
    let tail = trim_commas(&remainder[or_offset + 1..]);
    primitives::parse_prefix(tail, parse_restriction_verb_lexed)?;

    let negation_tokens = tokens[negation.first..negation.end].to_vec();
    let mut first = subject.to_vec();
    first.extend(negation_tokens.iter().cloned());
    first.extend(trim_commas(&remainder[..or_offset]).iter().cloned());

    let mut second = subject.to_vec();
    second.extend(negation_tokens);
    second.extend(tail.iter().cloned());
    Some(CantRestrictionOrSplit { first, second })
}

fn parse_activation_negation_span_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationNegationSpan> {
    let initial_len = input.len();
    let mut previous_words = Vec::new();
    let mut inside_quotes = false;
    loop {
        let first = initial_len.saturating_sub(input.len());
        if !inside_quotes {
            let mut candidate = input.clone();
            if let Some(end) =
                parse_negation_candidate(&mut candidate, initial_len, &previous_words)
            {
                *input = candidate;
                return Ok(ActivationNegationSpan { first, end });
            }
        }
        let token: &OwnedLexToken = any.parse_next(input)?;
        if token.is_quote() {
            inside_quotes = !inside_quotes;
            previous_words.clear();
        } else if !inside_quotes && let Some(word) = token.as_word() {
            previous_words.push(word);
            if previous_words.len() > 2 {
                previous_words.remove(0);
            }
        }
    }
}

fn parse_during_your_turn_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<StaticRestrictionConditionShape> {
    parse_timing_prefix_lexed(
        input,
        &["during", "your", "turn"],
        ActivationTiming::DuringYourTurn,
    )
}

fn parse_during_combat_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<StaticRestrictionConditionShape> {
    parse_timing_prefix_lexed(input, &["during", "combat"], ActivationTiming::DuringCombat)
}

fn parse_timing_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
    phrase: &'static [&'static str],
    timing: ActivationTiming,
) -> WResult<StaticRestrictionConditionShape> {
    let initial_len = input.len();
    primitives::phrase(phrase).parse_next(input)?;
    let fallback = initial_len.saturating_sub(input.len());
    let mut remainder_first = fallback;
    loop {
        let mut comma = input.clone();
        if primitives::comma().parse_next(&mut comma).is_ok() {
            *input = comma;
            remainder_first = initial_len.saturating_sub(input.len());
            break;
        }
        if input.peek_token().is_none() {
            break;
        }
        any.parse_next(input)?;
    }
    let _: Vec<&OwnedLexToken> = winnow::combinator::repeat(0.., any).parse_next(input)?;
    Ok(StaticRestrictionConditionShape::Timing {
        timing,
        remainder_first,
        remainder_end: initial_len,
    })
}

fn parse_if_condition_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<StaticRestrictionConditionShape> {
    parse_condition_prefix_lexed(input, &["if"], StaticRestrictionConditionKind::If)
}

fn parse_as_long_as_condition_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<StaticRestrictionConditionShape> {
    parse_condition_prefix_lexed(
        input,
        &["as", "long", "as"],
        StaticRestrictionConditionKind::AsLongAs,
    )
}

fn parse_condition_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
    phrase: &'static [&'static str],
    kind: StaticRestrictionConditionKind,
) -> WResult<StaticRestrictionConditionShape> {
    let initial_len = input.len();
    primitives::phrase(phrase).parse_next(input)?;
    let condition_first = initial_len.saturating_sub(input.len());
    let mut condition_end = condition_first;
    loop {
        let mut comma = input.clone();
        if primitives::comma().parse_next(&mut comma).is_ok() {
            if condition_end == condition_first {
                return Err(primitives::backtrack_err(
                    "restriction condition",
                    "condition before comma",
                ));
            }
            *input = comma;
            break;
        }
        any.parse_next(input)?;
        condition_end += 1;
    }
    let remainder_first = initial_len.saturating_sub(input.len());
    let _: Vec<&OwnedLexToken> = winnow::combinator::repeat(0.., any).parse_next(input)?;
    Ok(StaticRestrictionConditionShape::Condition {
        kind,
        condition: ActivationWordSpan {
            first: condition_first,
            end: condition_end,
        },
        remainder_first,
    })
}

fn parse_during_your_turn_suffix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<StaticRestrictionConditionShape> {
    parse_timing_suffix_lexed(
        input,
        &["during", "your", "turn"],
        ActivationTiming::DuringYourTurn,
    )
}

fn parse_during_combat_suffix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<StaticRestrictionConditionShape> {
    parse_timing_suffix_lexed(input, &["during", "combat"], ActivationTiming::DuringCombat)
}

fn parse_during_extra_turns_suffix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<StaticRestrictionConditionShape> {
    let initial_len = input.len();
    loop {
        let remainder_end = initial_len.saturating_sub(input.len());
        let mut suffix = input.clone();
        if primitives::phrase(&["during", "extra", "turns"])
            .parse_next(&mut suffix)
            .is_ok()
        {
            while suffix
                .peek_token()
                .is_some_and(|token| token.is_comma() || token.is_period())
            {
                any.parse_next(&mut suffix)?;
            }
            if suffix.peek_token().is_none() {
                *input = suffix;
                return Ok(StaticRestrictionConditionShape::ExtraTurn {
                    remainder_first: 0,
                    remainder_end,
                });
            }
        }
        any.parse_next(input)?;
    }
}

fn parse_timing_suffix_lexed<'a>(
    input: &mut LexStream<'a>,
    phrase: &'static [&'static str],
    timing: ActivationTiming,
) -> WResult<StaticRestrictionConditionShape> {
    let initial_len = input.len();
    loop {
        let remainder_end = initial_len.saturating_sub(input.len());
        let mut suffix = input.clone();
        if primitives::phrase(phrase).parse_next(&mut suffix).is_ok() {
            while suffix
                .peek_token()
                .is_some_and(|token| token.is_comma() || token.is_period())
            {
                any.parse_next(&mut suffix)?;
            }
            if suffix.peek_token().is_none() {
                *input = suffix;
                return Ok(StaticRestrictionConditionShape::Timing {
                    timing,
                    remainder_first: 0,
                    remainder_end,
                });
            }
        }
        any.parse_next(input)?;
    }
}

fn parse_source_attached_to_creature_condition_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["this", "equipment", "is", "attached", "to", "a", "creature"]),
        primitives::phrase(&["this", "equipment", "is", "attached", "to", "creature"]),
        primitives::phrase(&["this", "permanent", "is", "attached", "to", "a", "creature"]),
        primitives::phrase(&["this", "permanent", "is", "attached", "to", "creature"]),
        primitives::phrase(&["this", "is", "attached"]),
    ))
    .void()
    .parse_next(input)
}

fn parse_cast_limit_qualifier_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<ObjectFilter> {
    let mut compound_non = *input;
    if let Ok(word) = take_word_slice_any(&mut compound_non)
        && let Some(term) = parse_compound_non_term(word)
        && let Some(filter) = classify_cast_limit_term(term, true)
    {
        *input = compound_non;
        return Ok(filter);
    }

    let mut separated_non = *input;
    if primitives::word_slice_exact("non")
        .parse_next(&mut separated_non)
        .is_ok()
        && let Ok(term) = take_word_slice_any(&mut separated_non)
        && let Some(filter) = classify_cast_limit_term(term, true)
    {
        *input = separated_non;
        return Ok(filter);
    }

    let first = take_word_slice_any(input)?;
    let Some(first_filter) = classify_cast_limit_term(first, false) else {
        return Err(primitives::backtrack_err(
            "cast-limit qualifier",
            "card type or subtype",
        ));
    };
    let mut filters = vec![first_filter];
    loop {
        let mut connector = *input;
        let Ok(word) = take_word_slice_any(&mut connector) else {
            break;
        };
        if !matches!(word, "and" | "or") {
            break;
        }
        let Ok(term) = take_word_slice_any(&mut connector) else {
            break;
        };
        let Some(filter) = classify_cast_limit_term(term, false) else {
            break;
        };
        filters.push(filter);
        *input = connector;
    }
    if filters.len() == 1 {
        return Ok(filters.pop().expect("single cast-limit filter"));
    }
    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = filters;
    Ok(disjunction)
}

fn parse_compound_non_term(raw: &str) -> Option<&str> {
    let mut input = raw;
    parse_compound_non_term_text.parse_next(&mut input).ok()
}

fn parse_compound_non_term_text<'a>(input: &mut &'a str) -> WResult<&'a str> {
    alt((literal("non-"), literal("non"))).parse_next(input)?;
    let term: &str = rest.parse_next(input)?;
    if term.is_empty() {
        return Err(primitives::backtrack_err(
            "non qualifier",
            "term following non",
        ));
    }
    Ok(term)
}

fn take_word_slice_any<'word>(input: &mut &[&'word str]) -> WResult<&'word str> {
    any.parse_next(input)
}

fn classify_cast_limit_term(term: &str, negated: bool) -> Option<ObjectFilter> {
    if let Ok(card_type) = leaf::parse_leaf_card_type_complete(term) {
        return Some(if negated {
            ObjectFilter::default().without_type(card_type)
        } else {
            ObjectFilter::default().with_type(card_type)
        });
    }
    let subtype = leaf::parse_leaf_subtype_flexible_complete(term).ok()?;
    Some(if negated {
        ObjectFilter::default().without_subtype(subtype)
    } else {
        ObjectFilter::default().with_subtype(subtype)
    })
}

fn parse_possessive_owner_stem(input: &mut &str) -> WResult<String> {
    let stem: &str =
        take_till(1.., |character: char| matches!(character, '\'' | '’')).parse_next(input)?;
    let plural = alt((
        literal("'s").value(false),
        literal("’s").value(false),
        literal("'").value(true),
        literal("’").value(true),
    ))
    .parse_next(input)?;
    eof.parse_next(input)?;
    if plural {
        if stem.as_bytes().last().copied() != Some(b's') {
            return Err(primitives::backtrack_err(
                "possessive owner",
                "plural s before apostrophe",
            ));
        }
        return Ok(stem[..stem.len().saturating_sub(1)].to_string());
    }
    Ok(stem.to_string())
}

fn parse_negation_candidate<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
    previous_words: &[&str],
) -> Option<usize> {
    let first = primitives::word_parser_text.parse_next(input).ok()?;
    if matches!(first, "cant" | "can't" | "cannot") {
        return Some(initial_len.saturating_sub(input.len()));
    }
    if first == "can" {
        let mut contraction = input.clone();
        if primitives::kw("t").parse_next(&mut contraction).is_ok() {
            *input = contraction;
            return Some(initial_len.saturating_sub(input.len()));
        }
    }

    let after_if_you =
        primitives::parse_word_sequence_complete(previous_words, &["if", "you"]).is_some();
    if matches!(first, "doesnt" | "dont" | "doesn't" | "don't") {
        if after_if_you || next_is_control_or_own(input) {
            return None;
        }
        return Some(initial_len.saturating_sub(input.len()));
    }
    if matches!(first, "does" | "do" | "can") {
        let mut phrase = input.clone();
        if primitives::kw("not").parse_next(&mut phrase).is_err() {
            return None;
        }
        if after_if_you || (matches!(first, "does" | "do") && next_is_control_or_own(&phrase)) {
            return None;
        }
        *input = phrase;
        return Some(initial_len.saturating_sub(input.len()));
    }
    None
}

fn next_is_control_or_own(input: &LexStream<'_>) -> bool {
    let mut probe = input.clone();
    primitives::word_parser_text
        .parse_next(&mut probe)
        .is_ok_and(|word| matches!(word, "control" | "controls" | "own" | "owns"))
}

fn find_restriction_or_lexed(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if primitives::kw("or").parse_next(&mut candidate).is_ok() {
            return Some(offset);
        }
        consume_any_lexed(&mut input).ok()?;
    }
}

fn consume_any_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.void().parse_next(input)
}

fn parse_restriction_verb_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    let verb = primitives::word_parser_text.parse_next(input)?;
    if matches!(verb, "cast" | "activate" | "attack" | "block" | "be") {
        Ok(())
    } else {
        Err(primitives::backtrack_err(
            "restriction verb",
            "cast, activate, attack, block, or be",
        ))
    }
}

fn trim_commas(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[1..];
    }
    while tokens.last().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

#[cfg(test)]
mod tests;
