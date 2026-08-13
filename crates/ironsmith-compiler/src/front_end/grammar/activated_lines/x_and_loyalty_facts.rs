use winnow::ascii::digit1;
use winnow::combinator::{alt, eof, peek, repeat_till, separated};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, literal, take_till};

use crate::lexer::{OwnedLexToken, TokenKind};

use super::super::primitives::{self, TokenWordView, WordSliceInput};
use super::{word_phrase, word_phrase_present, word_present};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivatedCostXFact {
    Mentioned,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ActivatedManaXClauseSpec<'a> {
    pub(crate) where_clause_tokens: Option<&'a [OwnedLexToken]>,
    pub(crate) removed_counters_this_way: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivatedLoyaltyShorthand {
    Add(u32),
    Remove(u32),
    RemoveX,
}

fn word_phrase_offset(words: &[&str], expected: &'static [&'static str]) -> Option<usize> {
    let mut input: WordSliceInput<'_> = words;
    let skipped: &[&str] = repeat_till(0.., any.void(), peek(word_phrase(expected)))
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input)
        .ok()?;
    Some(skipped.len())
}

fn parse_x_surface(input: &mut &str) -> WResult<()> {
    alt((literal("+x"), literal("-x"), literal("x")))
        .void()
        .parse_next(input)
}

fn parse_text_end(input: &mut &str) -> WResult<()> {
    eof.void().parse_next(input)
}

fn parse_segmented_x_surface(input: &mut &str) -> WResult<ActivatedCostXFact> {
    let segments: Vec<&str> =
        separated(1.., take_till(1.., '/'), literal('/')).parse_next(input)?;
    eof.parse_next(input)?;
    for segment in segments {
        let mut segment_input = segment;
        if (parse_x_surface, parse_text_end)
            .void()
            .parse_next(&mut segment_input)
            .is_ok()
        {
            return Ok(ActivatedCostXFact::Mentioned);
        }
    }
    Err(primitives::backtrack_err(
        "activation X surface",
        "X, +X, or -X",
    ))
}

pub(crate) fn parse_activation_cost_x_fact_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedCostXFact> {
    for token in tokens {
        if token.as_word().is_none() {
            continue;
        }
        let mut input = token.parser_text();
        if let Ok(parsed) = parse_segmented_x_surface.parse_next(&mut input) {
            return Some(parsed);
        }
    }
    None
}

pub(crate) fn parse_activated_mana_x_clause_tokens(
    tokens: &[OwnedLexToken],
) -> ActivatedManaXClauseSpec<'_> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let where_clause_tokens = word_phrase_offset(&words, &["where", "x", "is"])
        .and_then(|first| view.token_span_for_words(first, first + 1))
        .map(|range| &tokens[range.start..]);
    let removed_counters_this_way = word_phrase_present(&words, &["this", "way"])
        && word_present(&words, "removed")
        && (word_present(&words, "counter") || word_present(&words, "counters"));
    ActivatedManaXClauseSpec {
        where_clause_tokens,
        removed_counters_this_way,
    }
}

fn parse_loyalty_number(input: &mut &str) -> WResult<u32> {
    digit1
        .try_map(|digits: &str| digits.parse::<u32>())
        .parse_next(input)
}

fn parse_loyalty_shorthand_word(input: &mut &str) -> WResult<ActivatedLoyaltyShorthand> {
    let mut zero = *input;
    if (literal("0"), parse_text_end)
        .void()
        .parse_next(&mut zero)
        .is_ok()
    {
        *input = zero;
        return Ok(ActivatedLoyaltyShorthand::Add(0));
    }

    let sign = alt((
        literal("+").value(1i8),
        literal("-").value(-1i8),
        literal("−").value(-1i8),
    ))
    .parse_next(input)?;
    if sign < 0 {
        let mut x = *input;
        if (literal("x"), parse_text_end)
            .void()
            .parse_next(&mut x)
            .is_ok()
        {
            *input = x;
            return Ok(ActivatedLoyaltyShorthand::RemoveX);
        }
    }
    let amount = parse_loyalty_number.parse_next(input)?;
    eof.parse_next(input)?;
    if sign > 0 {
        Ok(ActivatedLoyaltyShorthand::Add(amount))
    } else {
        Ok(ActivatedLoyaltyShorthand::Remove(amount))
    }
}

fn trim_loyalty_brackets(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut first = 0usize;
    let mut end = tokens.len();
    if tokens
        .get(first)
        .is_some_and(|token| token.kind == TokenKind::LBracket)
    {
        first += 1;
    }
    if end > first
        && tokens
            .get(end - 1)
            .is_some_and(|token| token.kind == TokenKind::RBracket)
    {
        end -= 1;
    }
    &tokens[first..end]
}

pub(crate) fn parse_loyalty_shorthand_activation_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedLoyaltyShorthand> {
    let tokens = trim_loyalty_brackets(tokens);
    match tokens {
        [token] if token.as_word().is_some() => {
            let mut input = token.parser_text();
            parse_loyalty_shorthand_word.parse_next(&mut input).ok()
        }
        [sign, value]
            if matches!(sign.kind, TokenKind::Plus | TokenKind::Dash)
                && value.as_word().is_some() =>
        {
            let mut amount = value.parser_text();
            if sign.kind == TokenKind::Dash {
                let mut x = amount;
                if (parse_x_surface, parse_text_end)
                    .void()
                    .parse_next(&mut x)
                    .is_ok()
                {
                    return Some(ActivatedLoyaltyShorthand::RemoveX);
                }
            }
            let parsed = (parse_loyalty_number, parse_text_end)
                .map(|(amount, ())| amount)
                .parse_next(&mut amount)
                .ok()?;
            if sign.kind == TokenKind::Plus {
                Some(ActivatedLoyaltyShorthand::Add(parsed))
            } else {
                Some(ActivatedLoyaltyShorthand::Remove(parsed))
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn owns_activation_x_and_where_x_facts() {
        for raw in ["X", "W/X", "-X"] {
            let tokens = lex_line(raw, 0).unwrap();
            assert_eq!(
                parse_activation_cost_x_fact_tokens(&tokens),
                Some(ActivatedCostXFact::Mentioned),
                "{raw}"
            );
        }
        let tokens = lex_line(
            "Add X, where X is the number of counters removed this way.",
            0,
        )
        .unwrap();
        let facts = parse_activated_mana_x_clause_tokens(&tokens);
        assert!(facts.where_clause_tokens.is_some());
        assert!(facts.removed_counters_this_way);
    }

    #[test]
    fn owns_loyalty_shorthand_surfaces() {
        for (raw, expected) in [
            ("0", ActivatedLoyaltyShorthand::Add(0)),
            ("+2", ActivatedLoyaltyShorthand::Add(2)),
            ("-3", ActivatedLoyaltyShorthand::Remove(3)),
            ("[-X]", ActivatedLoyaltyShorthand::RemoveX),
        ] {
            let tokens = lex_line(raw, 0).unwrap();
            assert_eq!(
                parse_loyalty_shorthand_activation_tokens(&tokens),
                Some(expected),
                "{raw}"
            );
        }
    }
}
