use winnow::combinator::{alt, eof};
use winnow::error::{ContextError, ErrMode, ParseError, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::{any, literal};

use crate::cards::builders::{CardTextError, TextSpan};

use super::super::lexer::{LexStream, LexToken, TokenKind};
use super::super::native_tokens::compat_word_pieces_for_token;

fn failure_location<'a>(
    tokens: &'a LexStream<'a>,
    offset: usize,
) -> (TextSpan, Option<&'a LexToken>) {
    if let Some(token) = tokens.get(offset) {
        return (token.span(), Some(token));
    }

    if let Some(last) = tokens.last() {
        return (
            TextSpan {
                line: last.span.line,
                start: last.span.end,
                end: last.span.end,
            },
            None,
        );
    }

    (TextSpan::synthetic(), None)
}

fn format_parse_error(
    label: &str,
    err: ParseError<LexStream<'_>, ContextError>,
    display_line_index: Option<usize>,
) -> CardTextError {
    let (span, token) = failure_location(err.input(), err.offset());
    let display_line = display_line_index.unwrap_or(span.line) + 1;
    let location = if span.start == span.end {
        format!("line {display_line} at {}", span.start)
    } else {
        format!("line {display_line} at {}..{}", span.start, span.end)
    };
    let found = token
        .map(|token| format!(" near {:?}", token.slice))
        .unwrap_or_else(|| " at end of input".to_string());

    CardTextError::ParseError(format!(
        "rewrite {label} parse failed on {location}{found}: {}",
        err.inner()
    ))
}

pub(crate) fn parse_all<'a, O>(
    tokens: &'a [LexToken],
    mut parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
) -> Result<O, CardTextError> {
    parser
        .parse(LexStream::new(tokens))
        .map_err(|err| format_parse_error(label, err, None))
}

pub(crate) fn parse_all_with_display_line<'a, O>(
    tokens: &'a [LexToken],
    mut parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
    display_line_index: usize,
) -> Result<O, CardTextError> {
    parser
        .parse(LexStream::new(tokens))
        .map_err(|err| format_parse_error(label, err, Some(display_line_index)))
}

pub(crate) fn parse_prefix<'a, O>(
    tokens: &'a [LexToken],
    mut parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
) -> Option<(O, &'a [LexToken])> {
    let mut input = LexStream::new(tokens);
    let parsed = parser.parse_next(&mut input).ok()?;
    let consumed = tokens.len().checked_sub(input.len())?;
    Some((parsed, &tokens[consumed..]))
}

#[derive(Debug, Clone)]
pub(crate) struct CompatWordIndex {
    words: Vec<String>,
    token_start_indices: Vec<usize>,
    token_end_indices: Vec<usize>,
}

impl CompatWordIndex {
    pub(crate) fn new(tokens: &[LexToken]) -> Self {
        let mut words = Vec::new();
        let mut token_start_indices = Vec::new();
        let mut token_end_indices = Vec::new();

        for (token_idx, token) in tokens.iter().enumerate() {
            for piece in compat_word_pieces_for_token(token) {
                words.push(piece.text);
                token_start_indices.push(token_idx);
                token_end_indices.push(token_idx + 1);
            }
        }

        Self {
            words,
            token_start_indices,
            token_end_indices,
        }
    }

    pub(crate) fn word_refs(&self) -> Vec<&str> {
        self.words.iter().map(String::as_str).collect()
    }

    pub(crate) fn owned_words(&self) -> Vec<String> {
        self.words.clone()
    }

    pub(crate) fn to_word_refs(&self) -> Vec<&str> {
        self.word_refs()
    }

    pub(crate) fn len(&self) -> usize {
        self.words.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub(crate) fn get(&self, idx: usize) -> Option<&str> {
        self.words.get(idx).map(String::as_str)
    }

    pub(crate) fn first(&self) -> Option<&str> {
        self.get(0)
    }

    pub(crate) fn slice_eq(&self, start: usize, expected: &[&str]) -> bool {
        self.words
            .get(start..start.saturating_add(expected.len()))
            .is_some_and(|slice| {
                slice
                    .iter()
                    .map(String::as_str)
                    .zip(expected.iter().copied())
                    .all(|(actual, expected)| actual == expected)
            })
    }

    pub(crate) fn find_word(&self, expected: &str) -> Option<usize> {
        self.words.iter().position(|word| word == expected)
    }

    pub(crate) fn find_phrase_start(&self, expected: &[&str]) -> Option<usize> {
        if expected.is_empty() || self.words.len() < expected.len() {
            return None;
        }

        let last_start = self.words.len() - expected.len();
        let mut idx = 0usize;
        while idx <= last_start {
            if self.slice_eq(idx, expected) {
                return Some(idx);
            }
            idx += 1;
        }

        None
    }

    pub(crate) fn has_phrase(&self, expected: &[&str]) -> bool {
        self.find_phrase_start(expected).is_some()
    }

    pub(crate) fn has_word(&self, expected: &str) -> bool {
        self.find_word(expected).is_some()
    }

    pub(crate) fn has_any_word(&self, expected: &[&str]) -> bool {
        expected.iter().any(|word| self.has_word(word))
    }

    pub(crate) fn token_index_for_word_index(&self, word_idx: usize) -> Option<usize> {
        self.token_start_indices.get(word_idx).copied()
    }

    pub(crate) fn token_start_indices(&self) -> Vec<usize> {
        self.token_start_indices.clone()
    }

    pub(crate) fn token_index_after_words(&self, word_count: usize) -> Option<usize> {
        if word_count == 0 {
            return Some(0);
        }
        self.token_end_indices.get(word_count - 1).copied()
    }
}

pub(crate) fn token_kind<'a>(
    expected: TokenKind,
) -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    literal(expected)
        .map(|tokens: &'a [LexToken]| &tokens[0])
        .context(StrContext::Expected(StrContextValue::Description("token")))
}

fn punctuation<'a>(
    expected: TokenKind,
    label: &'static str,
) -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    token_kind(expected)
        .context(StrContext::Label(label))
        .context(StrContext::Expected(StrContextValue::Description(label)))
}

pub(crate) fn word_text<'a>(input: &mut LexStream<'a>) -> Result<&'a str, ErrMode<ContextError>> {
    let token: &'a LexToken = any.parse_next(input)?;
    token.as_word().ok_or_else(|| {
        let mut err = ContextError::new();
        err.push(StrContext::Label("word"));
        err.push(StrContext::Expected(StrContextValue::Description("word")));
        ErrMode::Backtrack(err)
    })
}

pub(crate) fn kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        let Some(token) = input.peek_token() else {
            let mut err = ContextError::new();
            err.push(StrContext::Label("keyword"));
            err.push(StrContext::Expected(expected.into()));
            return Err(ErrMode::Backtrack(err));
        };

        if !token.is_word(expected) {
            let mut err = ContextError::new();
            err.push(StrContext::Label("keyword"));
            err.push(StrContext::Expected(expected.into()));
            return Err(ErrMode::Backtrack(err));
        }

        Ok(&input.next_slice(1)[0])
    }
}

pub(crate) fn comma<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Comma, "comma")
}

pub(crate) fn period<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Period, "period")
}

pub(crate) fn colon<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Colon, "colon")
}

pub(crate) fn semicolon<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Semicolon, "semicolon")
}

pub(crate) fn lparen<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::LParen, "left parenthesis")
}

pub(crate) fn rparen<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::RParen, "right parenthesis")
}

pub(crate) fn quote<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Quote, "quote")
}

pub(crate) fn end_of_sentence<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    period()
        .map(|_| ())
        .context(StrContext::Label("end of sentence"))
        .context(StrContext::Expected(StrContextValue::Description("period")))
}

pub(crate) fn end_of_block<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    eof.value(())
        .context(StrContext::Label("end of block"))
        .context(StrContext::Expected(StrContextValue::Description(
            "end of token block",
        )))
}

pub(crate) fn end_of_sentence_or_block<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>>
{
    alt((end_of_sentence(), end_of_block()))
        .context(StrContext::Label("end of sentence or block"))
        .context(StrContext::Expected(StrContextValue::Description(
            "end of sentence or block",
        )))
}

pub(crate) fn phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            if let Err(err) = kw(*word).parse_next(input) {
                return Err(err.map(|mut inner| {
                    inner.push(StrContext::Label("phrase"));
                    inner.push(StrContext::Expected(StrContextValue::Description(
                        "word phrase",
                    )));
                    inner
                }));
            }
        }
        Ok(())
    }
}

fn split_lexed_slices_on_separator<'a, P, F>(
    tokens: &'a [LexToken],
    make_separator: F,
) -> Vec<&'a [LexToken]>
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, (), ErrMode<ContextError>>,
{
    let mut segments = Vec::new();
    let mut segment_start = 0usize;
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let tail = &tokens[cursor..];
        if let Some((_, rest)) = parse_prefix(tail, make_separator()) {
            if segment_start < cursor {
                segments.push(&tokens[segment_start..cursor]);
            }

            let consumed = tail.len().saturating_sub(rest.len());
            if consumed == 0 {
                cursor += 1;
            } else {
                cursor += consumed;
            }
            segment_start = cursor;
        } else {
            cursor += 1;
        }
    }

    if segment_start < tokens.len() {
        segments.push(&tokens[segment_start..]);
    }

    segments
}

pub(crate) fn split_lexed_slices_on_and<'a>(tokens: &'a [LexToken]) -> Vec<&'a [LexToken]> {
    split_lexed_slices_on_separator(tokens, || phrase(&["and"]))
}

pub(crate) fn split_lexed_slices_on_comma<'a>(tokens: &'a [LexToken]) -> Vec<&'a [LexToken]> {
    split_lexed_slices_on_separator(tokens, || comma().map(|_| ()))
}

fn is_comparison_or_delimiter(tokens: &[LexToken], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("or")) {
        return false;
    }

    let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
    let next_word = tokens.get(idx + 1).and_then(LexToken::as_word);

    if matches!(next_word, Some("less" | "greater" | "more" | "fewer")) {
        return true;
    }

    previous_word == Some("than") && next_word == Some("equal")
}

pub(crate) fn split_lexed_slices_on_or<'a>(tokens: &'a [LexToken]) -> Vec<&'a [LexToken]> {
    let mut segments = Vec::new();
    let mut segment_start = 0usize;

    for (idx, token) in tokens.iter().enumerate() {
        let is_separator =
            token.is_comma() || (token.is_word("or") && !is_comparison_or_delimiter(tokens, idx));
        if !is_separator {
            continue;
        }

        if segment_start < idx {
            segments.push(&tokens[segment_start..idx]);
        }
        segment_start = idx + 1;
    }

    if segment_start < tokens.len() {
        segments.push(&tokens[segment_start..]);
    }

    segments
}

pub(crate) fn split_lexed_slices_on_commas_or_semicolons<'a>(
    tokens: &'a [LexToken],
) -> Vec<&'a [LexToken]> {
    split_lexed_slices_on_separator(tokens, || {
        alt((comma().map(|_| ()), semicolon().map(|_| ())))
    })
}

pub(crate) fn split_lexed_slices_on_period<'a>(tokens: &'a [LexToken]) -> Vec<&'a [LexToken]> {
    let mut segments = Vec::new();
    let mut segment_start = 0usize;
    let mut quote_depth = 0u32;

    for (idx, token) in tokens.iter().enumerate() {
        if token.is_quote() {
            quote_depth = if quote_depth == 0 { 1 } else { 0 };
            continue;
        }

        if token.is_period() && quote_depth == 0 {
            if segment_start < idx {
                segments.push(&tokens[segment_start..idx]);
            }
            segment_start = idx + 1;
        }
    }

    if segment_start < tokens.len() {
        segments.push(&tokens[segment_start..]);
    }

    segments
}

pub(crate) fn strip_lexed_prefix_phrase<'a>(
    tokens: &'a [LexToken],
    phrase_words: &'static [&'static str],
) -> Option<&'a [LexToken]> {
    parse_prefix(tokens, phrase(phrase_words)).map(|(_, rest)| rest)
}

pub(crate) fn strip_lexed_suffix_phrase<'a>(
    tokens: &'a [LexToken],
    phrase: &[&str],
) -> Option<&'a [LexToken]> {
    let words = CompatWordIndex::new(tokens);
    let word_refs = words.word_refs();
    if word_refs.len() < phrase.len() {
        return None;
    }

    let suffix_start = word_refs.len() - phrase.len();
    if word_refs[suffix_start..]
        .iter()
        .copied()
        .ne(phrase.iter().copied())
    {
        return None;
    }

    let keep_word_count = word_refs.len().checked_sub(phrase.len())?;
    let keep_until = if keep_word_count == 0 {
        0
    } else {
        words.token_index_for_word_index(keep_word_count)?
    };
    Some(&tokens[..keep_until])
}

pub(crate) fn strip_lexed_suffix_phrases<'a, 'b>(
    tokens: &'a [LexToken],
    phrases: &'b [&'b [&'b str]],
) -> Option<(&'b [&'b str], &'a [LexToken])> {
    phrases
        .iter()
        .find_map(|phrase| strip_lexed_suffix_phrase(tokens, phrase).map(|rest| (*phrase, rest)))
}
