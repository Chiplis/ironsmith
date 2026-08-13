use std::ops::Range;

use winnow::combinator::opt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::lexer::{LexStream, OwnedLexToken};
use super::primitives;

#[derive(Debug, Clone, Copy)]
pub(crate) struct OtherwiseAbilityClause<'a> {
    pub(crate) ability_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LeadingIfClause<'a> {
    pub(crate) condition_tokens: &'a [OwnedLexToken],
    pub(crate) remainder_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenDelimiterSpan {
    pub(crate) delimiter: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CounterKeyword {
    pub(crate) index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CounterTailPrefix {
    ForEach,
    EqualTo,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CounterTailFacts {
    pub(crate) has_words: bool,
    pub(crate) prefix: CounterTailPrefix,
}

pub(crate) fn parse_otherwise_ability_clause(
    tokens: &[OwnedLexToken],
) -> Option<OtherwiseAbilityClause<'_>> {
    let ((), ability_tokens) = primitives::parse_prefix(tokens, parse_otherwise_prefix_lexed)?;
    Some(OtherwiseAbilityClause { ability_tokens })
}

pub(crate) fn parse_leading_if_clause(tokens: &[OwnedLexToken]) -> Option<LeadingIfClause<'_>> {
    let ((condition_start, condition_end, remainder_start), _) =
        primitives::parse_prefix(tokens, parse_leading_if_boundaries_lexed)?;
    Some(LeadingIfClause {
        condition_tokens: &tokens[condition_start..condition_end],
        remainder_tokens: &tokens[remainder_start..],
    })
}

pub(crate) fn parse_and_with_delimiter(tokens: &[OwnedLexToken]) -> Option<TokenDelimiterSpan> {
    let (delimiter, _) = primitives::parse_prefix(tokens, parse_and_with_delimiter_lexed)?;
    Some(TokenDelimiterSpan { delimiter })
}

pub(crate) fn parse_counter_keyword(tokens: &[OwnedLexToken]) -> Option<CounterKeyword> {
    let (index, _) = primitives::parse_prefix(tokens, parse_counter_keyword_lexed)?;
    Some(CounterKeyword { index })
}

pub(crate) fn parse_counter_tail_facts(tokens: &[OwnedLexToken]) -> CounterTailFacts {
    let prefix = if primitives::parse_prefix(tokens, primitives::phrase(&["for", "each"])).is_some()
    {
        CounterTailPrefix::ForEach
    } else if primitives::parse_prefix(tokens, primitives::phrase(&["equal", "to"])).is_some() {
        CounterTailPrefix::EqualTo
    } else {
        CounterTailPrefix::Other
    };
    let has_words = primitives::parse_prefix(tokens, parse_any_word_lexed).is_some();
    CounterTailFacts { has_words, prefix }
}

fn parse_leading_if_boundaries_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(usize, usize, usize)> {
    let initial_len = input.len();
    primitives::kw("if").parse_next(input)?;
    let condition_start = initial_len.saturating_sub(input.len());
    loop {
        let condition_end = initial_len.saturating_sub(input.len());
        let mut comma = input.clone();
        if primitives::comma().parse_next(&mut comma).is_ok() {
            *input = comma;
            let remainder_start = initial_len.saturating_sub(input.len());
            return Ok((condition_start, condition_end, remainder_start));
        }
        any.parse_next(input)?;
    }
}

fn parse_otherwise_prefix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("otherwise").parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "has"]).parse_next(input)
}

fn parse_and_with_delimiter_lexed<'a>(input: &mut LexStream<'a>) -> WResult<Range<usize>> {
    let initial_len = input.len();
    loop {
        let start = initial_len.saturating_sub(input.len());
        let mut delimiter = input.clone();
        if primitives::phrase(&["and", "with"])
            .parse_next(&mut delimiter)
            .is_ok()
        {
            *input = delimiter;
            let end = initial_len.saturating_sub(input.len());
            return Ok(start..end);
        }
        any.parse_next(input)?;
    }
}

fn parse_counter_keyword_lexed<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let initial_len = input.len();
    loop {
        let index = initial_len.saturating_sub(input.len());
        let token: &'a OwnedLexToken = any.parse_next(input)?;
        if token
            .as_word()
            .is_some_and(|word| matches!(word, "counter" | "counters"))
        {
            return Ok(index);
        }
    }
}

fn parse_any_word_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    loop {
        let token: &'a OwnedLexToken = any.parse_next(input)?;
        if token.as_word().is_some() {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::lexer::{TokenWordView, lex_line};
    use super::*;

    #[test]
    fn parses_static_grant_clause_boundaries() {
        let tokens = lex_line("Otherwise, it has flying.", 0).unwrap();
        let clause = parse_otherwise_ability_clause(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(clause.ability_tokens).word_refs(),
            ["flying"]
        );

        let tokens = lex_line("If you paid life, it enters with a counter.", 0).unwrap();
        let clause = parse_leading_if_clause(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(clause.condition_tokens).word_refs(),
            ["you", "paid", "life"]
        );
        assert_eq!(
            TokenWordView::new(clause.remainder_tokens).word_refs(),
            ["it", "enters", "with", "a", "counter"]
        );
    }

    #[test]
    fn parses_counter_delimiters_and_tail_kinds() {
        let tokens = lex_line("two counters and with flying", 0).unwrap();
        let delimiter = parse_and_with_delimiter(&tokens).unwrap();
        assert_eq!(delimiter.delimiter, 2..4);
        assert_eq!(parse_counter_keyword(&tokens).unwrap().index, 1);

        let tail = lex_line("for each creature you control", 0).unwrap();
        assert_eq!(
            parse_counter_tail_facts(&tail).prefix,
            CounterTailPrefix::ForEach
        );
        let tail = lex_line("equal to your life total", 0).unwrap();
        assert_eq!(
            parse_counter_tail_facts(&tail).prefix,
            CounterTailPrefix::EqualTo
        );
    }
}
