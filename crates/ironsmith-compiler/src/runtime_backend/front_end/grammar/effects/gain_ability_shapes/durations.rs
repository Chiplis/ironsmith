use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::ConditionExpr;
use crate::effect::Until;
use crate::runtime_backend::front_end::grammar::primitives::{self, WordSliceInput};
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, TokenWordView, trim_lexed_commas};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GainAbilityDurationShape {
    pub(crate) start: usize,
    pub(crate) len: usize,
    pub(crate) duration: Until,
    pub(crate) condition: Option<ConditionExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LeadingGainDurationShape {
    pub(crate) consumed_words: usize,
    pub(crate) duration: Until,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct QuotedGainDurationShape {
    pub(crate) close_quote_token: usize,
    pub(crate) duration: Until,
}

fn until_end_of_turn(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        primitives::word_slice_exact("until"),
        primitives::word_slice_exact("end"),
        primitives::word_slice_exact("of"),
        primitives::word_slice_exact("turn"),
    )
        .value(Until::EndOfTurn)
        .parse_next(input)
}

fn until_next_turn(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        primitives::word_slice_exact("until"),
        primitives::word_slice_exact("your"),
        primitives::word_slice_exact("next"),
        alt((
            primitives::word_slice_exact("turn"),
            primitives::word_slice_exact("upkeep"),
        )),
    )
        .value(Until::YourNextTurn)
        .parse_next(input)
}

fn next_untap_step(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        alt((
            primitives::word_slice_exact("until"),
            primitives::word_slice_exact("during"),
        )),
        primitives::word_slice_exact("your"),
        primitives::word_slice_exact("next"),
        primitives::word_slice_exact("untap"),
        primitives::word_slice_exact("step"),
    )
        .value(Until::YourNextTurn)
        .parse_next(input)
}

fn simple_turn_duration(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    alt((until_end_of_turn, until_next_turn, next_untap_step)).parse_next(input)
}

fn for_as_long_as(input: &mut WordSliceInput<'_>) -> WResult<()> {
    (
        primitives::word_slice_exact("for"),
        primitives::word_slice_exact("as"),
        primitives::word_slice_exact("long"),
        primitives::word_slice_exact("as"),
    )
        .void()
        .parse_next(input)
}

fn you_control(input: &mut WordSliceInput<'_>) -> WResult<()> {
    (
        for_as_long_as,
        primitives::word_slice_exact("you"),
        primitives::word_slice_exact("control"),
    )
        .void()
        .parse_next(input)
}

fn word_occurs(words: &[&str], expected: &'static str) -> bool {
    let mut offset = 0usize;
    while offset < words.len() {
        let mut input = &words[offset..];
        if primitives::word_slice_exact(expected)
            .parse_next(&mut input)
            .is_ok()
        {
            return true;
        }
        offset += 1;
    }
    false
}

fn find_source_tapped_duration(words: &[&str]) -> Option<GainAbilityDurationShape> {
    let mut start = 0usize;
    while start < words.len() {
        let mut input = &words[start..];
        if for_as_long_as.parse_next(&mut input).is_ok() {
            let tail = &words[start..];
            if word_occurs(tail, "this")
                && word_occurs(tail, "remains")
                && word_occurs(tail, "tapped")
            {
                return Some(GainAbilityDurationShape {
                    start,
                    len: words.len().saturating_sub(start),
                    duration: Until::SourceUntaps,
                    condition: Some(ConditionExpr::SourceIsTapped),
                });
            }
        }
        start += 1;
    }
    None
}

pub(crate) fn parse_simple_ability_duration_shape(
    words: &[&str],
) -> Option<GainAbilityDurationShape> {
    let mut start = 0usize;
    while start < words.len() {
        let mut input = &words[start..];
        if let Ok(duration) = simple_turn_duration.parse_next(&mut input) {
            return Some(GainAbilityDurationShape {
                start,
                len: words[start..].len().saturating_sub(input.len()),
                duration,
                condition: None,
            });
        }
        let mut input = &words[start..];
        if you_control.parse_next(&mut input).is_ok() {
            return Some(GainAbilityDurationShape {
                start,
                len: words.len().saturating_sub(start),
                duration: Until::YouStopControllingThis,
                condition: None,
            });
        }
        start += 1;
    }
    None
}

pub(crate) fn parse_gain_ability_duration_shape(
    words: &[&str],
) -> Option<GainAbilityDurationShape> {
    find_source_tapped_duration(words).or_else(|| parse_simple_ability_duration_shape(words))
}

pub(crate) fn parse_leading_gain_duration_shape(
    words: &[&str],
) -> Option<LeadingGainDurationShape> {
    let mut input: WordSliceInput<'_> = words;
    let duration = simple_turn_duration.parse_next(&mut input).ok()?;
    Some(LeadingGainDurationShape {
        consumed_words: words.len().saturating_sub(input.len()),
        duration,
    })
}

pub(crate) fn parse_quoted_gain_duration_shape(
    tokens: &[OwnedLexToken],
    gain_token_idx: usize,
) -> Option<QuotedGainDurationShape> {
    let open_quote_idx = gain_token_idx.checked_add(1)?;
    primitives::parse_prefix(tokens.get(open_quote_idx..)?, primitives::quote())?;
    let after_open = tokens.get(open_quote_idx + 1..)?;
    let (relative_close, _, _) = primitives::find_prefix(after_open, primitives::quote)?;
    let close_quote_token = open_quote_idx + 1 + relative_close;
    let tail_tokens = trim_lexed_commas(tokens.get(close_quote_token + 1..)?);
    if tail_tokens.is_empty() {
        return None;
    }
    let tail_words = TokenWordView::new(tail_tokens).to_word_refs();
    let parsed = parse_simple_ability_duration_shape(&tail_words)?;
    if parsed.start != 0 || parsed.len != tail_words.len() {
        return None;
    }
    Some(QuotedGainDurationShape {
        close_quote_token,
        duration: parsed.duration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_turn_conditional_and_quoted_durations() {
        let duration =
            parse_simple_ability_duration_shape(&["flying", "until", "your", "next", "upkeep"])
                .unwrap();
        assert_eq!(duration.start, 1);
        assert_eq!(duration.len, 4);
        assert_eq!(duration.duration, Until::YourNextTurn);

        let tapped = parse_gain_ability_duration_shape(&[
            "flying", "for", "as", "long", "as", "this", "remains", "tapped",
        ])
        .unwrap();
        assert_eq!(tapped.duration, Until::SourceUntaps);
        assert_eq!(tapped.condition, Some(ConditionExpr::SourceIsTapped));

        let tokens = lex_line(
            "Target creature gains \"Whenever it attacks, draw a card.\" until end of turn.",
            0,
        )
        .unwrap();
        let gain_token = primitives::find_prefix(&tokens, || primitives::kw("gains"))
            .map(|(offset, _, _)| offset)
            .unwrap();
        assert_eq!(
            parse_quoted_gain_duration_shape(&tokens, gain_token)
                .unwrap()
                .duration,
            Until::EndOfTurn
        );
    }
}
