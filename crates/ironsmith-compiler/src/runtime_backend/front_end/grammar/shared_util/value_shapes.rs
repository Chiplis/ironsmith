use winnow::combinator::{alt, opt};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;

use crate::effect::Comparison;
use crate::runtime_backend::lexer::{OwnedLexToken, TokenWordView};

use super::super::{leaf, primitives};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AggregateValueMetric {
    BasicLandTypes,
    CreatureTypes,
    Colors,
    DistinctPowers,
    Counters,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AggregateValueSurface<'a> {
    pub(crate) metric: AggregateValueMetric,
    pub(crate) scope_words: &'a [&'a str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QuantityComparisonWords {
    pub(crate) comparison: Comparison,
    pub(crate) consumed_words: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QuantityComparisonTokens {
    pub(crate) comparison: Comparison,
    pub(crate) consumed_tokens: usize,
}

pub(crate) fn parse_aggregate_value_surface<'a>(
    words: &'a [&'a str],
) -> Option<AggregateValueSurface<'a>> {
    let mut input: primitives::WordSliceInput<'a> = words;
    let surface = parse_aggregate_surface.parse_next(&mut input).ok()?;
    input.is_empty().then_some(surface)
}

pub(crate) fn parse_quantity_comparison_prefix_words(
    words: &[&str],
    allow_default_one: bool,
    article_implies_min_one: bool,
) -> Option<QuantityComparisonWords> {
    if words.is_empty() {
        return None;
    }

    let mut input: primitives::WordSliceInput<'_> = words;
    let comparison = match parse_quantity_comparison(article_implies_min_one).parse_next(&mut input)
    {
        Ok(comparison) => comparison,
        Err(_) if allow_default_one => Comparison::GreaterThanOrEqual(1),
        Err(_) => return None,
    };
    Some(QuantityComparisonWords {
        comparison,
        consumed_words: words.len().checked_sub(input.len())?,
    })
}

pub(crate) fn parse_quantity_comparison_prefix_tokens(
    tokens: &[OwnedLexToken],
    allow_default_one: bool,
    article_implies_min_one: bool,
) -> Option<QuantityComparisonTokens> {
    if tokens.is_empty() {
        return None;
    }

    let view = TokenWordView::new(tokens);
    let parsed = parse_quantity_comparison_prefix_words(
        &view.word_refs(),
        allow_default_one,
        article_implies_min_one,
    )?;
    Some(QuantityComparisonTokens {
        comparison: parsed.comparison,
        consumed_tokens: view.token_index_after_words(parsed.consumed_words)?,
    })
}

fn parse_aggregate_surface<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> WResult<AggregateValueSurface<'a>> {
    let metric = parse_aggregate_metric.parse_next(input)?;
    opt(primitives::word_slice_exact("the"))
        .void()
        .parse_next(input)?;
    let scope_words = take_nonempty_scope.parse_next(input)?;
    Ok(AggregateValueSurface {
        metric,
        scope_words,
    })
}

fn parse_aggregate_metric(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<AggregateValueMetric> {
    alt((
        (
            primitives::word_slice_exact("basic"),
            primitives::word_slice_exact("land"),
            alt((
                primitives::word_slice_exact("type"),
                primitives::word_slice_exact("types"),
            )),
            primitives::word_slice_exact("among"),
        )
            .value(AggregateValueMetric::BasicLandTypes),
        (
            primitives::word_slice_exact("creature"),
            alt((
                primitives::word_slice_exact("type"),
                primitives::word_slice_exact("types"),
            )),
            primitives::word_slice_exact("among"),
        )
            .value(AggregateValueMetric::CreatureTypes),
        (
            alt((
                primitives::word_slice_exact("color"),
                primitives::word_slice_exact("colors"),
            )),
            primitives::word_slice_exact("among"),
        )
            .value(AggregateValueMetric::Colors),
        (
            primitives::word_slice_exact("different"),
            primitives::word_slice_exact("power"),
            primitives::word_slice_exact("values"),
            primitives::word_slice_exact("among"),
        )
            .value(AggregateValueMetric::DistinctPowers),
        (
            primitives::word_slice_exact("different"),
            primitives::word_slice_exact("powers"),
            primitives::word_slice_exact("among"),
        )
            .value(AggregateValueMetric::DistinctPowers),
        (
            primitives::word_slice_exact("different"),
            primitives::word_slice_exact("power"),
            primitives::word_slice_exact("among"),
        )
            .value(AggregateValueMetric::DistinctPowers),
        (
            alt((
                primitives::word_slice_exact("counter"),
                primitives::word_slice_exact("counters"),
            )),
            primitives::word_slice_exact("among"),
        )
            .value(AggregateValueMetric::Counters),
    ))
    .parse_next(input)
}

fn take_nonempty_scope<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<&'a [&'a str]> {
    if input.is_empty() {
        return Err(primitives::backtrack_err(
            "aggregate value scope",
            "one or more scope words",
        ));
    }
    let scope = *input;
    *input = &[];
    Ok(scope)
}

fn parse_quantity_comparison(
    article_implies_min_one: bool,
) -> impl for<'a> Parser<primitives::WordSliceInput<'a>, Comparison, ErrMode<ContextError>> {
    move |input: &mut primitives::WordSliceInput<'_>| {
        alt((
            parse_exact_quantity,
            parse_no_more_than_quantity,
            parse_no_quantity,
            parse_at_least_quantity,
            parse_at_most_quantity,
            parse_strict_less_quantity,
            parse_strict_greater_quantity,
            move |input: &mut primitives::WordSliceInput<'_>| {
                parse_plain_quantity(input, article_implies_min_one)
            },
        ))
        .parse_next(input)
    }
}

fn parse_exact_quantity(input: &mut primitives::WordSliceInput<'_>) -> WResult<Comparison> {
    primitives::word_slice_exact("exactly").parse_next(input)?;
    parse_fixed_number
        .map(|value| Comparison::Equal(value as i32))
        .parse_next(input)
}

fn parse_no_more_than_quantity(input: &mut primitives::WordSliceInput<'_>) -> WResult<Comparison> {
    primitives::word_slice_exact("no").parse_next(input)?;
    alt((
        primitives::word_slice_exact("more"),
        primitives::word_slice_exact("greater"),
    ))
    .parse_next(input)?;
    primitives::word_slice_exact("than").parse_next(input)?;
    parse_fixed_number
        .map(|value| Comparison::LessThanOrEqual(value as i32))
        .parse_next(input)
}

fn parse_no_quantity(input: &mut primitives::WordSliceInput<'_>) -> WResult<Comparison> {
    primitives::word_slice_exact("no")
        .value(Comparison::LessThanOrEqual(0))
        .parse_next(input)
}

fn parse_at_least_quantity(input: &mut primitives::WordSliceInput<'_>) -> WResult<Comparison> {
    (
        primitives::word_slice_exact("at"),
        primitives::word_slice_exact("least"),
    )
        .parse_next(input)?;
    parse_fixed_number
        .map(|value| Comparison::GreaterThanOrEqual(value as i32))
        .parse_next(input)
}

fn parse_at_most_quantity(input: &mut primitives::WordSliceInput<'_>) -> WResult<Comparison> {
    (
        primitives::word_slice_exact("at"),
        primitives::word_slice_exact("most"),
    )
        .parse_next(input)?;
    parse_fixed_number
        .map(|value| Comparison::LessThanOrEqual(value as i32))
        .parse_next(input)
}

fn parse_strict_less_quantity(input: &mut primitives::WordSliceInput<'_>) -> WResult<Comparison> {
    alt((
        primitives::word_slice_exact("fewer"),
        primitives::word_slice_exact("less"),
    ))
    .parse_next(input)?;
    primitives::word_slice_exact("than").parse_next(input)?;
    parse_fixed_number
        .map(|value| Comparison::LessThan(value as i32))
        .parse_next(input)
}

fn parse_strict_greater_quantity(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<Comparison> {
    alt((
        primitives::word_slice_exact("more"),
        primitives::word_slice_exact("greater"),
    ))
    .parse_next(input)?;
    primitives::word_slice_exact("than").parse_next(input)?;
    parse_fixed_number
        .map(|value| Comparison::GreaterThan(value as i32))
        .parse_next(input)
}

fn parse_plain_quantity(
    input: &mut primitives::WordSliceInput<'_>,
    article_implies_min_one: bool,
) -> WResult<Comparison> {
    let mut article_probe = *input;
    let starts_with_article = alt((
        primitives::word_slice_exact("a"),
        primitives::word_slice_exact("an"),
    ))
    .parse_next(&mut article_probe)
    .is_ok();
    let value = parse_fixed_number.parse_next(input)? as i32;
    if article_implies_min_one && starts_with_article {
        return Ok(Comparison::GreaterThanOrEqual(1));
    }

    let mut tail = *input;
    if (
        primitives::word_slice_exact("or"),
        alt((
            primitives::word_slice_exact("more"),
            primitives::word_slice_exact("greater"),
        )),
    )
        .parse_next(&mut tail)
        .is_ok()
    {
        *input = tail;
        return Ok(Comparison::GreaterThanOrEqual(value));
    }

    let mut tail = *input;
    if (
        primitives::word_slice_exact("or"),
        alt((
            primitives::word_slice_exact("less"),
            primitives::word_slice_exact("fewer"),
        )),
    )
        .parse_next(&mut tail)
        .is_ok()
    {
        *input = tail;
        return Ok(Comparison::LessThanOrEqual(value));
    }

    Ok(Comparison::Equal(value))
}

fn parse_fixed_number(input: &mut primitives::WordSliceInput<'_>) -> WResult<u32> {
    let parsed = leaf::parse_leaf_number_prefix_words(input).ok_or_else(|| {
        primitives::backtrack_err("quantity", "numeric, word, or article quantity")
    })?;
    let (value, consumed) = parsed
        .into_fixed()
        .ok_or_else(|| primitives::backtrack_err("quantity", "fixed numeric or word quantity"))?;
    *input = input
        .get(consumed..)
        .ok_or_else(|| primitives::backtrack_err("quantity", "available quantity words"))?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn parses_aggregate_value_surfaces() {
        let words = [
            "different",
            "power",
            "values",
            "among",
            "creatures",
            "you",
            "control",
        ];
        let parsed = parse_aggregate_value_surface(&words).expect("aggregate surface");
        assert_eq!(parsed.metric, AggregateValueMetric::DistinctPowers);
        assert_eq!(parsed.scope_words, ["creatures", "you", "control"]);

        let words = ["colors", "among", "the", "permanents"];
        let parsed = parse_aggregate_value_surface(&words).expect("aggregate surface");
        assert_eq!(parsed.metric, AggregateValueMetric::Colors);
        assert_eq!(parsed.scope_words, ["permanents"]);
    }

    #[test]
    fn parses_quantity_comparison_surfaces() {
        let words = ["no", "more", "than", "three", "creatures"];
        let parsed = parse_quantity_comparison_prefix_words(&words, false, false)
            .expect("quantity comparison");
        assert_eq!(parsed.comparison, Comparison::LessThanOrEqual(3));
        assert_eq!(parsed.consumed_words, 4);

        let words = ["two", "or", "more", "cards"];
        let parsed = parse_quantity_comparison_prefix_words(&words, false, false)
            .expect("quantity comparison");
        assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(2));
        assert_eq!(parsed.consumed_words, 3);

        let words = ["creatures"];
        let parsed = parse_quantity_comparison_prefix_words(&words, true, false)
            .expect("default quantity comparison");
        assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(1));
        assert_eq!(parsed.consumed_words, 0);
    }

    #[test]
    fn converts_consumed_words_to_token_boundaries() {
        let tokens = lex_line("twenty one or more creatures", 0).expect("lex fixture");
        let parsed = parse_quantity_comparison_prefix_tokens(&tokens, false, false)
            .expect("quantity comparison");
        assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(21));
        assert_eq!(parsed.consumed_tokens, 4);
    }
}
