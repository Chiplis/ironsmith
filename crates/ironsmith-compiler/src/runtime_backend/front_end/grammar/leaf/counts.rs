use winnow::combinator::{alt, dispatch, opt, peek};
use winnow::error::{ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;
use crate::effect::{ChoiceCount, Comparison, Value};

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;
use super::numbers::{
    LeafNumber, parse_leaf_number_or_x_prefix_lexed, parse_leaf_number_prefix_lexed,
    parse_leaf_number_prefix_word_slice,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LeafCountRange {
    pub(crate) min: Option<Value>,
    pub(crate) max: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeafChoiceCountPrefix {
    pub(crate) count: ChoiceCount,
    pub(crate) consumed: usize,
}

impl LeafCountRange {
    pub(crate) fn exact(value: Value) -> Self {
        Self {
            min: Some(value.clone()),
            max: Some(value),
        }
    }

    pub(crate) fn between(min: Value, max: Value) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
        }
    }

    pub(crate) fn at_least(min: Value) -> Self {
        Self {
            min: Some(min),
            max: None,
        }
    }

    pub(crate) fn up_to(max: Value) -> Self {
        Self {
            min: Some(Value::Fixed(0)),
            max: Some(max),
        }
    }

    pub(crate) fn any_number() -> Self {
        Self {
            min: Some(Value::Fixed(0)),
            max: None,
        }
    }

    pub(crate) fn into_min_max(self) -> (Option<Value>, Option<Value>) {
        (self.min, self.max)
    }
}

pub(crate) fn parse_leaf_modal_value_token<'a>(input: &mut LexStream<'a>) -> WResult<Value> {
    let checkpoint = input.checkpoint();
    let number = parse_leaf_number_or_x_prefix_lexed.parse_next(input)?;
    if let Some(value) = number.into_value() {
        return Ok(value);
    }

    input.reset(&checkpoint);
    Err(primitives::backtrack_err(
        "modal value",
        "number or X representable as a runtime value",
    ))
}

pub(crate) fn parse_leaf_count_range_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LeafCountRange> {
    dispatch! {peek(primitives::word_parser_text);
        "one" => alt((
            primitives::phrase(&["one", "or", "more"])
                .value(LeafCountRange::at_least(Value::Fixed(1))),
            primitives::phrase(&["one", "or", "both"])
                .value(LeafCountRange::between(Value::Fixed(1), Value::Fixed(2))),
            primitives::kw("one").value(LeafCountRange::exact(Value::Fixed(1))),
        )),
        "up" => (
            primitives::kw("up"),
            primitives::kw("to"),
            parse_leaf_modal_value_token,
        )
            .map(|(_, _, value)| LeafCountRange::up_to(value)),
        _ => parse_leaf_modal_value_token.map(LeafCountRange::exact),
    }
    .context(StrContext::Label("count range prefix"))
    .context(StrContext::Expected(StrContextValue::Description(
        "count range prefix",
    )))
    .parse_next(input)
}

pub(crate) fn parse_leaf_target_count_range_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ChoiceCount> {
    let checkpoint = input.checkpoint();
    let first = parse_leaf_number_prefix_lexed.parse_next(input)?;
    let mut after_first = input.clone();
    if primitives::kw("or").parse_next(&mut after_first).is_ok() {
        *input = after_first;
        let second = parse_leaf_number_prefix_lexed.parse_next(input)?;
        if second >= first {
            return Ok(choice_count_range(first, second));
        }
        input.reset(&checkpoint);
        return Err(primitives::backtrack_err(
            "target count range",
            "ascending count range",
        ));
    }

    if primitives::comma().parse_next(input).is_err() {
        input.reset(&checkpoint);
        return Err(primitives::backtrack_err(
            "target count range",
            "count range",
        ));
    }
    let second = parse_leaf_number_prefix_lexed.parse_next(input)?;
    let mut after_second = input.clone();
    if primitives::comma().parse_next(&mut after_second).is_ok() {
        *input = after_second;
    }
    primitives::kw("or").parse_next(input)?;
    let third = parse_leaf_number_prefix_lexed.parse_next(input)?;
    if second > first && third >= second {
        return Ok(choice_count_range(first, third));
    }

    input.reset(&checkpoint);
    Err(primitives::backtrack_err(
        "target count range",
        "ascending count range",
    ))
}

pub(crate) fn parse_leaf_another_event_count_comparison_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<Comparison> {
    primitives::kw("another")
        .value(Comparison::GreaterThanOrEqual(2))
        .context(StrContext::Label("another event count"))
        .context(StrContext::Expected(StrContextValue::Description(
            "another as second-or-later event count",
        )))
        .parse_next(input)
}

pub(crate) fn parse_leaf_choice_count_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ChoiceCount> {
    alt((
        primitives::phrase(&["one", "or", "more"]).value(ChoiceCount::at_least(1)),
        (
            primitives::phrase(&["any", "number"]),
            opt(primitives::kw("of")),
        )
            .value(ChoiceCount::any_number()),
        (
            primitives::phrase(&["up", "to"]),
            parse_leaf_number_or_x_prefix_lexed,
        )
            .map(|(_, number)| choice_count_from_leaf_number(number, true)),
        parse_leaf_number_or_x_prefix_lexed
            .map(|number| choice_count_from_leaf_number(number, false)),
    ))
    .context(StrContext::Label("choice count prefix"))
    .context(StrContext::Expected(StrContextValue::Description(
        "any number, up to a number, X, or a fixed number",
    )))
    .parse_next(input)
}

pub(crate) fn parse_leaf_choice_count_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeafChoiceCountPrefix> {
    let (count, rest) = primitives::parse_prefix(tokens, parse_leaf_choice_count_prefix_lexed)?;
    Some(LeafChoiceCountPrefix {
        count,
        consumed: tokens.len().checked_sub(rest.len())?,
    })
}

pub(crate) fn parse_leaf_choice_count_prefix_words(
    words: &[&str],
) -> Option<LeafChoiceCountPrefix> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let count = parse_leaf_choice_count_prefix_word_slice
        .parse_next(&mut input)
        .ok()?;
    Some(LeafChoiceCountPrefix {
        count,
        consumed: words.len().checked_sub(input.len())?,
    })
}

pub(crate) fn parse_leaf_another_event_count_comparison_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<Comparison>, CardTextError> {
    primitives::parse_all_or_none(
        tokens,
        parse_leaf_another_event_count_comparison_lexed,
        "leaf-another-event-count",
    )
}

pub(crate) fn parse_leaf_modal_choose_range_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<LeafCountRange>, CardTextError> {
    if primitives::parse_prefix(tokens, primitives::phrase(&["any", "number"])).is_some() {
        return Ok(Some(LeafCountRange::any_number()));
    }

    if let Some((range, _)) = primitives::parse_prefix(tokens, parse_leaf_count_range_prefix_lexed)
    {
        return Ok(Some(range));
    }

    if primitives::parse_prefix(tokens, parse_leaf_or_word_anywhere).is_some() {
        return Ok(Some(LeafCountRange::exact(Value::Fixed(1))));
    }

    Ok(None)
}

fn parse_leaf_or_word_anywhere<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    while input.peek_token().is_some() {
        let checkpoint = input.checkpoint();
        if primitives::kw("or").parse_next(input).is_ok() {
            return Ok(());
        }
        input.reset(&checkpoint);
        any.parse_next(input)?;
    }

    Err(primitives::backtrack_err("or word", "or"))
}

fn parse_leaf_choice_count_prefix_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<ChoiceCount> {
    alt((
        (
            primitives::word_slice_exact("one"),
            primitives::word_slice_exact("or"),
            primitives::word_slice_exact("more"),
        )
            .value(ChoiceCount::at_least(1)),
        (
            (
                primitives::word_slice_exact("any"),
                primitives::word_slice_exact("number"),
            ),
            opt(primitives::word_slice_exact("of")),
        )
            .value(ChoiceCount::any_number()),
        (
            (
                primitives::word_slice_exact("up"),
                primitives::word_slice_exact("to"),
            ),
            parse_leaf_number_or_x_prefix_word_slice,
        )
            .map(|(_, number)| choice_count_from_leaf_number(number, true)),
        parse_leaf_number_or_x_prefix_word_slice
            .map(|number| choice_count_from_leaf_number(number, false)),
    ))
    .parse_next(input)
}

fn parse_leaf_number_or_x_prefix_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<LeafNumber> {
    alt((
        primitives::word_slice_exact("x").value(LeafNumber::X),
        parse_leaf_number_prefix_word_slice.map(LeafNumber::Fixed),
    ))
    .parse_next(input)
}

fn choice_count_from_leaf_number(number: LeafNumber, up_to: bool) -> ChoiceCount {
    match (number, up_to) {
        (LeafNumber::X, true) => ChoiceCount::up_to_dynamic_x(),
        (LeafNumber::X, false) => ChoiceCount::dynamic_x(),
        (LeafNumber::Fixed(value), true) => ChoiceCount::up_to(value as usize),
        (LeafNumber::Fixed(value), false) => ChoiceCount::exactly(value as usize),
    }
}

fn choice_count_range(first: u32, last: u32) -> ChoiceCount {
    ChoiceCount {
        min: first as usize,
        max: Some(last as usize),
        dynamic_x: false,
        up_to_x: false,
        random: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn choice_count_prefixes_are_typed_for_tokens_and_words() {
        for (raw, consumed, min, max, dynamic_x, up_to_x) in [
            ("any number of targets", 3, 0, None, false, false),
            ("up to x targets", 3, 0, None, true, true),
            ("up to three targets", 3, 0, Some(3), false, false),
            ("x targets", 1, 0, None, true, false),
            ("two targets", 1, 2, Some(2), false, false),
        ] {
            let tokens = lex_line(raw, 0).unwrap();
            let parsed = parse_leaf_choice_count_prefix_tokens(&tokens).unwrap();
            assert_eq!(parsed.consumed, consumed, "{raw}");
            assert_eq!(parsed.count.min, min, "{raw}");
            assert_eq!(parsed.count.max, max, "{raw}");
            assert_eq!(parsed.count.dynamic_x, dynamic_x, "{raw}");
            assert_eq!(parsed.count.up_to_x, up_to_x, "{raw}");

            let words = raw.split_whitespace().collect::<Vec<_>>();
            let parsed_words = parse_leaf_choice_count_prefix_words(&words).unwrap();
            assert_eq!(parsed_words, parsed, "{raw}");
        }
    }
}
