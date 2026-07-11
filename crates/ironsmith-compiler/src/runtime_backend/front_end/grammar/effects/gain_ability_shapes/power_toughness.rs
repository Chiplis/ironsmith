use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::Value;
use crate::runtime_backend::front_end::grammar::{leaf, primitives};
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, TokenWordView};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GainBasePtShapeError {
    InvalidValue,
    UnsupportedTail,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GainBasePtShape {
    pub(crate) power: Value,
    pub(crate) toughness: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LeadingGainBasePtShape {
    pub(crate) has_offset: usize,
    pub(crate) power: Value,
    pub(crate) toughness: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GainPumpHeadShape {
    pub(crate) power: Value,
    pub(crate) toughness: Value,
    pub(crate) has_local_duration: bool,
}

fn base_pt_head(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    (
        primitives::word_slice_exact("base"),
        primitives::word_slice_exact("power"),
        primitives::word_slice_exact("and"),
        primitives::word_slice_exact("toughness"),
    )
        .void()
        .parse_next(input)
}

fn has_word(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("has"),
        primitives::word_slice_exact("have"),
    ))
    .void()
    .parse_next(input)
}

fn end_of_turn(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    (
        primitives::word_slice_exact("until"),
        primitives::word_slice_exact("end"),
        primitives::word_slice_exact("of"),
        primitives::word_slice_exact("turn"),
    )
        .void()
        .parse_next(input)
}

fn valid_trailing_words(words: &[&str], leading: bool) -> bool {
    if words.is_empty() {
        return true;
    }
    if primitives::parse_full_word_slice(words, end_of_turn).is_some() {
        return true;
    }
    if !leading {
        return false;
    }
    primitives::parse_full_word_slice(words, primitives::word_slice_exact("and").void()).is_some()
        || primitives::parse_full_word_slice(
            words,
            (end_of_turn, primitives::word_slice_exact("and")).void(),
        )
        .is_some()
}

fn parse_base_pt_tail(
    words: &[&str],
    leading: bool,
) -> Result<Option<GainBasePtShape>, GainBasePtShapeError> {
    let mut input: primitives::WordSliceInput<'_> = words;
    if base_pt_head.parse_next(&mut input).is_err() {
        return Ok(None);
    }
    let Some(value_word) = input.first().copied() else {
        return Ok(None);
    };
    input = &input[1..];
    let (power, toughness) = leaf::parse_leaf_pt_modifier_values_complete(value_word)
        .map_err(|_| GainBasePtShapeError::InvalidValue)?;
    if !valid_trailing_words(input, leading) {
        return Err(GainBasePtShapeError::UnsupportedTail);
    }
    Ok(Some(GainBasePtShape { power, toughness }))
}

pub(crate) fn parse_gain_base_pt_after_has_shape(
    words_after_has: &[&str],
) -> Result<Option<GainBasePtShape>, GainBasePtShapeError> {
    parse_base_pt_tail(words_after_has, false)
}

pub(crate) fn parse_leading_gain_base_pt_shape(
    words: &[&str],
) -> Result<Option<LeadingGainBasePtShape>, GainBasePtShapeError> {
    let mut offset = 0usize;
    while offset < words.len() {
        let mut input = &words[offset..];
        if has_word.parse_next(&mut input).is_ok() {
            if offset == 0 {
                return Ok(None);
            }
            return parse_base_pt_tail(input, true).map(|shape| {
                shape.map(|shape| LeadingGainBasePtShape {
                    has_offset: offset,
                    power: shape.power,
                    toughness: shape.toughness,
                })
            });
        }
        offset += 1;
    }
    Ok(None)
}

pub(crate) fn subject_contains_gain_base_pt(words: &[&str]) -> bool {
    let mut offset = 0usize;
    while offset < words.len() {
        let mut input = &words[offset..];
        if has_word.parse_next(&mut input).is_ok() && base_pt_head.parse_next(&mut input).is_ok() {
            return true;
        }
        offset += 1;
    }
    false
}

pub(crate) fn parse_gain_pump_head_shape(
    modifier_tokens: &[OwnedLexToken],
) -> Option<GainPumpHeadShape> {
    let first = modifier_tokens.first()?;
    let (power, toughness) =
        leaf::parse_leaf_pt_modifier_values_complete(first.parser_text()).ok()?;
    let modifier_words = TokenWordView::new(modifier_tokens).to_word_refs();
    let has_local_duration = modifier_words
        .iter()
        .copied()
        .any(|word| matches!(word, "until" | "during"));
    Some(GainPumpHeadShape {
        power,
        toughness,
        has_local_duration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_base_pt_and_pump_heads() {
        assert_eq!(
            parse_leading_gain_base_pt_shape(&[
                "it",
                "has",
                "base",
                "power",
                "and",
                "toughness",
                "0/3",
                "and",
            ])
            .unwrap()
            .unwrap()
            .has_offset,
            1
        );
        assert!(subject_contains_gain_base_pt(&[
            "it",
            "has",
            "base",
            "power",
            "and",
            "toughness",
            "2/2",
        ]));
        let tokens = lex_line("+2/+1 until end of turn", 0).unwrap();
        let pump = parse_gain_pump_head_shape(&tokens).unwrap();
        assert_eq!(pump.power, Value::Fixed(2));
        assert!(pump.has_local_duration);
    }
}
