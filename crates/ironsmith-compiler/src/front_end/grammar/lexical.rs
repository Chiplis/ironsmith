use winnow::combinator::repeat_till;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::{any, take};

use crate::cards::builders::TextSpan;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordPiece, trim_lexed_commas};
use super::primitives as grammar;

type WordInput<'a> = &'a [&'a str];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedWordSequence<'p> {
    sequence: &'p [&'p str],
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedTokenSequence {
    start: usize,
    end: usize,
}

fn same_word_sequence(actual: &[&str], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .copied()
            .zip(expected.iter().copied())
            .all(|(actual_word, expected_word)| actual_word == expected_word)
}

fn word_sequence_parser<'a, 'p>(
    expected: &'p [&'p str],
) -> impl Parser<WordInput<'a>, ParsedWordSequence<'p>, ErrMode<ContextError>> + 'p {
    move |input: &mut WordInput<'a>| {
        let actual: &[&str] = take(expected.len()).parse_next(input)?;
        if !same_word_sequence(actual, expected) {
            return Err(grammar::backtrack_err(
                "word sequence",
                "expected word sequence",
            ));
        }
        Ok(ParsedWordSequence {
            sequence: expected,
            start: 0,
            end: expected.len(),
        })
    }
}

fn parse_word_sequence_at<'p>(
    words: &[&str],
    start: usize,
    expected: &'p [&'p str],
) -> Option<ParsedWordSequence<'p>> {
    let mut input = words.get(start..)?;
    let mut parsed = word_sequence_parser(expected).parse_next(&mut input).ok()?;
    parsed.start = start;
    parsed.end = start + expected.len();
    Some(parsed)
}

fn parse_word_choice_at<'p>(
    words: &[&str],
    start: usize,
    expected: &'p [&'p str],
) -> Option<&'p str> {
    let actual = *words.get(start)?;
    expected
        .iter()
        .find(|&candidate| actual == *candidate)
        .map(|v| v as _)
}

fn parse_first_word_choice<'p>(
    words: &[&str],
    expected: &'p [&'p str],
) -> Option<(usize, &'p str)> {
    let mut input = words;
    let mut index = 0usize;
    while !input.is_empty() {
        let parsed: Result<&str, ErrMode<ContextError>> = any.parse_next(&mut input);
        let word = parsed.ok()?;
        for candidate in expected {
            if word == *candidate {
                return Some((index, candidate));
            }
        }
        index += 1;
    }
    None
}

fn parse_last_word_boundary_by(
    words: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    let mut input = words;
    let mut index = 0usize;
    let mut last = None;
    while !input.is_empty() {
        let parsed: Result<&str, ErrMode<ContextError>> = any.parse_next(&mut input);
        let word = parsed.ok()?;
        if predicate(word) {
            last = Some(index);
        }
        index += 1;
    }
    last
}

fn parse_word_window_boundary(
    words: &[&str],
    window_len: usize,
    mut predicate: impl FnMut(&[&str]) -> bool,
) -> Option<usize> {
    if window_len == 0 || window_len > words.len() {
        return None;
    }
    let mut start = 0usize;
    while start + window_len <= words.len() {
        let mut input = &words[start..];
        let parsed: Result<&[&str], ErrMode<ContextError>> =
            take(window_len).parse_next(&mut input);
        if parsed.ok().is_some_and(&mut predicate) {
            return Some(start);
        }
        start += 1;
    }
    None
}

fn same_token_sequence(actual: &[OwnedLexToken], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter().copied())
            .all(|(token, expected_word)| token.is_word(expected_word))
}

fn token_sequence_parser<'a, 'p>(
    expected: &'p [&'p str],
) -> impl Parser<LexStream<'a>, ParsedTokenSequence, ErrMode<ContextError>> + 'p {
    move |input: &mut LexStream<'a>| {
        let actual: &[OwnedLexToken] = take(expected.len()).parse_next(input)?;
        if !same_token_sequence(actual, expected) {
            return Err(grammar::backtrack_err(
                "token sequence",
                "expected token sequence",
            ));
        }
        Ok(ParsedTokenSequence {
            start: 0,
            end: expected.len(),
        })
    }
}

fn parse_first_token_sequence(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<ParsedTokenSequence> {
    if expected.is_empty() {
        return None;
    }
    let mut input = LexStream::new(tokens);
    let mut parsed = repeat_till(0.., any.void(), token_sequence_parser(expected))
        .map(|((), parsed)| parsed)
        .parse_next(&mut input)
        .ok()?;
    parsed.end = tokens.len().saturating_sub(input.len());
    parsed.start = parsed.end.saturating_sub(expected.len());
    Some(parsed)
}

fn parse_token_boundary_by(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    let mut index = 0usize;
    while !input.is_empty() {
        let parsed: Result<&OwnedLexToken, ErrMode<ContextError>> = any.parse_next(&mut input);
        let token = parsed.ok()?;
        if predicate(token) {
            return Some(index);
        }
        index += 1;
    }
    None
}

pub(crate) fn token_word_pieces_for_token(token: &OwnedLexToken) -> &[TokenWordPiece] {
    token.parser_word_pieces()
}

pub(crate) fn word_slice_find_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> Option<usize> {
    parse_word_window_boundary(words, window_len, predicate)
}

pub(crate) fn word_slice_last_is_any(words: &[&str], expected: &[&str]) -> bool {
    words
        .len()
        .checked_sub(1)
        .and_then(|idx| parse_word_choice_at(words, idx, expected))
        .is_some()
}

pub(crate) fn word_prefix_present(words: &[&str], expected: &[&str]) -> bool {
    parse_word_sequence_at(words, 0, expected).is_some()
}

pub(crate) fn word_slice_strip_prefix<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    parse_word_sequence_at(words, 0, expected)?;
    Some(&words[expected.len()..])
}

pub(crate) fn word_slice_find_any_word_from(
    words: &[&str],
    expected: &[&str],
    start: usize,
) -> Option<usize> {
    let tail = words.get(start..)?;
    parse_first_word_choice(tail, expected).map(|(idx, _)| start + idx)
}

pub(crate) fn locate_last_word_by(
    words: &[&str],
    predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    parse_last_word_boundary_by(words, predicate)
}

pub(crate) fn find_token_word_sequence(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<usize> {
    parse_first_token_sequence(tokens, expected).map(|parsed| parsed.start)
}

pub(crate) fn find_token_word_sequence_span(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<(usize, usize)> {
    parse_first_token_sequence(tokens, expected).map(|parsed| (parsed.start, parsed.end))
}

pub(crate) fn contains_token_word_sequence(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    parse_first_token_sequence(tokens, expected).is_some()
}

pub(crate) fn token_slice_at_is(tokens: &[OwnedLexToken], idx: usize, expected: &str) -> bool {
    tokens.get(idx).is_some_and(|token| token.is_word(expected))
}

pub(crate) fn token_slice_at_is_any(
    tokens: &[OwnedLexToken],
    idx: usize,
    expected: &[&str],
) -> bool {
    tokens
        .get(idx)
        .is_some_and(|token| token.is_any_word(expected))
}

pub(crate) fn token_slice_first_is(tokens: &[OwnedLexToken], expected: &str) -> bool {
    token_slice_at_is(tokens, 0, expected)
}

pub(crate) fn token_slice_last_is(tokens: &[OwnedLexToken], expected: &str) -> bool {
    tokens.last().is_some_and(|token| token.is_word(expected))
}

pub(crate) fn locate_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    parse_token_boundary_by(tokens, |token| token.is_word(expected))
}

pub(crate) fn locate_token_word_choice(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<usize> {
    parse_token_boundary_by(tokens, |token| token.is_any_word(expected))
}

pub(crate) fn contains_token_word(tokens: &[OwnedLexToken], expected: &str) -> bool {
    locate_token_word(tokens, expected).is_some()
}

pub(crate) fn contains_token_any_word(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    locate_token_word_choice(tokens, expected).is_some()
}

pub(crate) fn locate_token_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> Option<usize> {
    parse_token_boundary_by(tokens, |token| token.kind == expected)
}

pub(crate) fn contains_token_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    locate_token_kind(tokens, expected).is_some()
}

pub(crate) fn token_slice_all_are_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    let mut input = LexStream::new(tokens);
    while !input.is_empty() {
        let parsed: Result<&OwnedLexToken, ErrMode<ContextError>> = any.parse_next(&mut input);
        if parsed.ok().is_none_or(|token| token.kind != expected) {
            return false;
        }
    }
    true
}

pub(crate) use crate::lexer::TokenWordView;

#[derive(Debug, Clone, Copy)]
pub(crate) struct LexedClause<'a> {
    tokens: &'a [OwnedLexToken],
}

impl<'a> LexedClause<'a> {
    pub(crate) fn new(tokens: &'a [OwnedLexToken]) -> Self {
        Self { tokens }
    }

    pub(crate) fn tokens(self) -> &'a [OwnedLexToken] {
        self.tokens
    }

    pub(crate) fn len(self) -> usize {
        self.tokens.len()
    }

    pub(crate) fn is_empty(self) -> bool {
        self.tokens.is_empty()
    }

    pub(crate) fn token(self, idx: usize) -> Option<&'a OwnedLexToken> {
        self.tokens.get(idx)
    }

    pub(crate) fn before(self, idx: usize) -> Self {
        Self::new(&self.tokens[..idx.min(self.tokens.len())])
    }

    pub(crate) fn from(self, idx: usize) -> Self {
        Self::new(&self.tokens[idx.min(self.tokens.len())..])
    }

    pub(crate) fn between(self, start: usize, end: usize) -> Self {
        let start = start.min(self.tokens.len());
        let end = end.min(self.tokens.len()).max(start);
        Self::new(&self.tokens[start..end])
    }

    pub(crate) fn words(self) -> TokenWordView<'a> {
        TokenWordView::new(self.tokens)
    }

    pub(crate) fn word_refs(self) -> Vec<&'a str> {
        self.words().word_refs()
    }

    pub(crate) fn word_len(self) -> usize {
        self.words().len()
    }

    pub(crate) fn text(self) -> String {
        self.words().join(" ")
    }

    pub(crate) fn span(self) -> Option<TextSpan> {
        let first = self.tokens.first()?;
        let last = self.tokens.last()?;
        Some(TextSpan {
            line: first.span.line,
            start: first.span.start,
            end: last.span.end,
        })
    }

    pub(crate) fn first_word(self) -> Option<&'a str> {
        self.tokens
            .iter()
            .flat_map(|token| token.parser_word_pieces().iter())
            .map(|piece| piece.text.as_str())
            .next()
    }

    pub(crate) fn token_boundary_for_word(self, word_idx: usize) -> Option<usize> {
        self.words().token_boundary_for_word(word_idx)
    }

    pub(crate) fn token_index_after_words(self, word_count: usize) -> Option<usize> {
        self.words().token_index_after_words(word_count)
    }

    pub(crate) fn before_word(self, word_idx: usize) -> Option<Self> {
        let token_idx = self.token_boundary_for_word(word_idx)?;
        Some(self.before(token_idx))
    }

    pub(crate) fn from_word(self, word_idx: usize) -> Option<Self> {
        let token_idx = self.token_boundary_for_word(word_idx)?;
        Some(self.from(token_idx))
    }

    pub(crate) fn after_words(self, word_count: usize) -> Option<Self> {
        let token_idx = self.token_index_after_words(word_count)?;
        Some(self.from(token_idx))
    }

    pub(crate) fn between_words_trimmed(self, start: usize, end: usize) -> Self {
        self.between_word_range(start, end)
            .unwrap_or_else(|| self.between(self.tokens.len(), self.tokens.len()))
            .trimmed()
    }

    pub(crate) fn between_word_range(self, start: usize, end: usize) -> Option<Self> {
        let range = self.words().token_span_for_words(start, end)?;
        Some(self.between(range.start, range.end))
    }

    pub(crate) fn rfind_word(self, expected: &str) -> Option<usize> {
        self.words().rfind_word(expected)
    }

    pub(crate) fn find_token_word(self, expected: &str) -> Option<usize> {
        locate_token_word(self.tokens, expected)
    }

    pub(crate) fn find_token_word_any(self, expected: &[&str]) -> Option<usize> {
        locate_token_word_choice(self.tokens, expected)
    }

    pub(crate) fn find_token_word_where(
        self,
        expected: &str,
        mut predicate: impl FnMut(usize, Self) -> bool,
    ) -> Option<usize> {
        self.tokens.iter().enumerate().find_map(|(idx, token)| {
            if token.is_word(expected) && predicate(idx, self.from(idx + 1)) {
                Some(idx)
            } else {
                None
            }
        })
    }

    pub(crate) fn find_unquoted_token_word(self, expected: &str) -> Option<usize> {
        let mut inside_quotes = false;
        for (idx, token) in self.tokens.iter().enumerate() {
            if token.is_quote() {
                inside_quotes = !inside_quotes;
                continue;
            }
            if !inside_quotes && token.is_word(expected) {
                return Some(idx);
            }
        }
        None
    }

    pub(crate) fn split_once_on_word(self, expected: &str) -> Option<(Self, Self)> {
        let idx = locate_token_word(self.tokens, expected)?;
        Some((self.before(idx), self.from(idx + 1)))
    }

    pub(crate) fn split_once_on_word_trimmed(self, expected: &str) -> Option<(Self, Self)> {
        self.split_once_on_word(expected)
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    pub(crate) fn split_once_on_word_any(self, expected: &[&str]) -> Option<(Self, Self)> {
        let idx = self.find_token_word_any(expected)?;
        Some((self.before(idx), self.from(idx + 1)))
    }

    pub(crate) fn split_once_on_comma(self) -> Option<(Self, Self)> {
        let idx = locate_token_kind(self.tokens, TokenKind::Comma)?;
        Some((self.before(idx), self.from(idx + 1)))
    }

    pub(crate) fn trimmed(self) -> Self {
        Self::new(trim_lexed_commas(self.tokens))
    }

    pub(crate) fn trim(self) -> Vec<OwnedLexToken> {
        self.trimmed().tokens().to_vec()
    }

    pub(crate) fn trimmed_word_refs(self) -> Vec<&'a str> {
        self.trimmed().word_refs()
    }

    pub(crate) fn comma_segments(self) -> Vec<Self> {
        grammar::split_lexed_slices_on_comma(self.tokens)
            .into_iter()
            .map(Self::new)
            .collect()
    }

    pub(crate) fn trimmed_comma_segments(self) -> Vec<Self> {
        self.comma_segments()
            .into_iter()
            .map(Self::trimmed)
            .collect()
    }

    pub(crate) fn and_segments(self) -> Vec<Self> {
        grammar::split_lexed_slices_on_and(self.tokens)
            .into_iter()
            .map(Self::new)
            .collect()
    }

    pub(crate) fn trimmed_and_comma_segments(self) -> Vec<Self> {
        self.and_segments()
            .into_iter()
            .flat_map(Self::trimmed_comma_segments)
            .filter(|segment| !segment.is_empty())
            .collect()
    }

    pub(crate) fn period_segments(self) -> Vec<Self> {
        grammar::split_lexed_slices_on_period(self.tokens)
            .into_iter()
            .map(Self::new)
            .collect()
    }

    pub(crate) fn trimmed_period_segments(self) -> Vec<Self> {
        self.period_segments()
            .into_iter()
            .map(Self::trimmed)
            .collect()
    }

    pub(crate) fn split_comma_then(self) -> Option<(Self, Self)> {
        grammar::split_lexed_once_on_separator(self.tokens, || {
            use winnow::Parser as _;
            (grammar::comma(), grammar::kw("then")).void()
        })
        .map(|(head, tail)| (Self::new(head), Self::new(tail)))
    }

    pub(crate) fn split_once_on_then(self) -> Option<(Self, Self)> {
        self.split_comma_then()
            .or_else(|| self.split_once_on_word("then"))
    }

    pub(crate) fn split_once_on_then_trimmed(self) -> Option<(Self, Self)> {
        self.split_once_on_then()
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    /// If this clause's trailing tokens exactly spell `phrase` (word for word),
    /// return the clause with that phrase removed; otherwise return it unchanged.
    /// This requires the exact ordered phrase and does not trim, so callers can
    /// detect a match by token count.
    pub(crate) fn without_trailing_phrase(self, phrase: &[&str]) -> Self {
        let len = self.tokens.len();
        if phrase.is_empty() || len < phrase.len() {
            return self;
        }
        let tail = &self.tokens[len - phrase.len()..];
        let matches = tail
            .iter()
            .zip(phrase)
            .all(|(token, expected)| token.as_word().is_some_and(|word| word == *expected));
        if matches {
            self.before(len - phrase.len())
        } else {
            self
        }
    }
}
