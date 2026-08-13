use winnow::combinator::{alt, opt, peek};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::Comparison;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ConditionQuantityPrefix<'a> {
    pub(super) comparison: Comparison,
    pub(super) consumed: usize,
    pub(super) rest: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuantitySuffix {
    AtLeast,
    AtMost,
}

pub(super) fn parse_condition_quantity_prefix(
    tokens: &[OwnedLexToken],
    allow_default_one: bool,
    article_implies_min_one: bool,
) -> Option<ConditionQuantityPrefix<'_>> {
    // The legacy quantity parser diagnosed an empty input even when the
    // caller permitted an implicit one. Preserve that distinction here.
    if tokens.is_empty() {
        return None;
    }

    if let Some((comparison, rest)) =
        primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
            parse_condition_quantity_lexed(input, article_implies_min_one)
        })
    {
        return Some(ConditionQuantityPrefix {
            comparison,
            consumed: tokens.len().saturating_sub(rest.len()),
            rest,
        });
    }

    allow_default_one.then_some(ConditionQuantityPrefix {
        comparison: Comparison::GreaterThanOrEqual(1),
        consumed: 0,
        rest: tokens,
    })
}

fn parse_condition_quantity_lexed(
    input: &mut LexStream<'_>,
    article_implies_min_one: bool,
) -> WResult<Comparison> {
    alt((
        (
            primitives::kw("exactly"),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, value)| Comparison::Equal(value as i32)),
        (
            primitives::kw("no"),
            alt((primitives::kw("more"), primitives::kw("greater"))),
            primitives::kw("than"),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, _, _, value)| Comparison::LessThanOrEqual(value as i32)),
        primitives::kw("no").value(Comparison::LessThanOrEqual(0)),
        (
            primitives::phrase(&["at", "least"]),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, value)| Comparison::GreaterThanOrEqual(value as i32)),
        (
            primitives::phrase(&["at", "most"]),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, value)| Comparison::LessThanOrEqual(value as i32)),
        (
            alt((primitives::kw("fewer"), primitives::kw("less"))),
            primitives::kw("than"),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, _, value)| Comparison::LessThan(value as i32)),
        (
            alt((primitives::kw("more"), primitives::kw("greater"))),
            primitives::kw("than"),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, _, value)| Comparison::GreaterThan(value as i32)),
        |input: &mut LexStream<'_>| parse_bare_quantity(input, article_implies_min_one),
    ))
    .parse_next(input)
}

fn parse_bare_quantity(
    input: &mut LexStream<'_>,
    article_implies_min_one: bool,
) -> WResult<Comparison> {
    let starts_with_article = peek(alt((primitives::kw("a"), primitives::kw("an"))))
        .parse_next(input)
        .is_ok();
    let value = leaf::parse_leaf_number_prefix_lexed.parse_next(input)? as i32;
    if article_implies_min_one && starts_with_article {
        return Ok(Comparison::GreaterThanOrEqual(1));
    }

    let suffix = opt((
        primitives::kw("or"),
        alt((
            alt((primitives::kw("more"), primitives::kw("greater"))).value(QuantitySuffix::AtLeast),
            alt((primitives::kw("less"), primitives::kw("fewer"))).value(QuantitySuffix::AtMost),
        )),
    ))
    .map(|suffix| suffix.map(|(_, kind)| kind))
    .parse_next(input)?;

    Ok(match suffix {
        Some(QuantitySuffix::AtLeast) => Comparison::GreaterThanOrEqual(value),
        Some(QuantitySuffix::AtMost) => Comparison::LessThanOrEqual(value),
        None => Comparison::Equal(value),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn parses_typed_comparison_prefixes_and_remainders() {
        let tokens = lex("three or more creatures");
        let parsed = parse_condition_quantity_prefix(&tokens, false, true).expect("quantity");
        assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(3));
        assert_eq!(parsed.consumed, 3);
        assert!(
            parsed
                .rest
                .first()
                .is_some_and(|token| token.is_word("creatures"))
        );

        let tokens = lex("a creature");
        let parsed = parse_condition_quantity_prefix(&tokens, false, true).expect("article");
        assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(1));
    }

    #[test]
    fn implicit_one_requires_nonempty_input() {
        let tokens = lex("creatures");
        let parsed = parse_condition_quantity_prefix(&tokens, true, true).expect("default one");
        assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(1));
        assert_eq!(parsed.consumed, 0);
        assert!(parse_condition_quantity_prefix(&[], true, true).is_none());
    }
}
