#![allow(dead_code)]

use logos::Logos;
use winnow::stream::{Location, TokenSlice};

use crate::cards::builders::{CardTextError, TextSpan};

use super::grammar::primitives as grammar;
use super::lex_patterns::{LexPattern, LexPatternAtom};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum LexerError {
    #[default]
    InvalidToken,
}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexerError::InvalidToken => f.write_str("encountered an unsupported token"),
        }
    }
}

fn normalize_parser_fragment(slice: &str) -> String {
    let mut normalized = String::with_capacity(slice.len());
    for ch in slice.chars() {
        match ch {
            '−' => normalized.push('-'),
            '’' | '‘' => normalized.push('\''),
            '“' | '”' => normalized.push('"'),
            _ => normalized.push(ch.to_ascii_lowercase()),
        }
    }
    normalized
}

fn parser_text_for_token(kind: TokenKind, slice: &str) -> String {
    match kind {
        TokenKind::Tilde => "this".to_string(),
        TokenKind::Half => "1/2".to_string(),
        TokenKind::Ampersand => "and".to_string(),
        _ => normalize_parser_fragment(slice),
    }
}

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"[ \t\r\n\f]+", error = LexerError)]
pub(crate) enum TokenKind {
    #[token("!")]
    Bang,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token("[")]
    LBracket,
    #[token("(")]
    LParen,
    #[token("]")]
    RBracket,
    #[token(")")]
    RParen,
    #[token("?")]
    Question,
    #[token(".")]
    Period,
    #[token("+")]
    Plus,
    #[token("|")]
    Pipe,
    #[token(";")]
    Semicolon,
    #[token("•")]
    #[token("*")]
    Bullet,
    #[token("&")]
    Ampersand,
    #[token("~")]
    Tilde,
    #[token("-")]
    #[token("−")]
    #[token("–")]
    Dash,
    #[token("—")]
    EmDash,
    #[token("½")]
    Half,
    #[token("'")]
    #[token("’")]
    #[token("‘")]
    Apostrophe,
    #[regex(r#""|“|”"#)]
    Quote,
    #[regex(r"\{[^}\r\n]+\}")]
    ManaGroup,
    #[regex(r"[0-9]+", priority = 3)]
    Number,
    #[regex(
        r"(?:\+[0-9xX]+|-[0-9xX]+|[\p{L}0-9]+)(?:(?:['’‘](?:[\p{L}0-9]+)?)|(?:[-−]|/+)(?:\+[0-9xX]+|-[0-9xX]+|[\p{L}0-9]+))*"
    )]
    Word,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnedLexToken {
    pub(crate) kind: TokenKind,
    pub(crate) slice: String,
    pub(crate) parser_text: String,
    parser_word_pieces: Box<[TokenWordPiece]>,
    pub(crate) span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenWordPiece {
    pub(crate) text: String,
    pub(crate) span: TextSpan,
}

fn build_token_word_pieces(
    kind: TokenKind,
    slice: &str,
    parser_text: &str,
    span: TextSpan,
) -> Box<[TokenWordPiece]> {
    let mut pieces = Vec::new();
    match kind {
        TokenKind::Word | TokenKind::Number | TokenKind::Ampersand => {
            push_normalized_token_words(parser_text, span, false, &mut pieces);
        }
        TokenKind::Tilde => pieces.push(TokenWordPiece {
            text: "this".to_string(),
            span,
        }),
        TokenKind::ManaGroup => {
            let inner = slice.trim_start_matches('{').trim_end_matches('}');
            if !inner.is_empty() {
                push_normalized_token_words(
                    inner,
                    TextSpan {
                        line: span.line,
                        start: span.start.saturating_add(1),
                        end: span.end.saturating_sub(1),
                    },
                    true,
                    &mut pieces,
                );
            }
        }
        TokenKind::Half => pieces.push(TokenWordPiece {
            text: "1/2".to_string(),
            span,
        }),
        _ => {}
    }
    pieces.into_boxed_slice()
}

pub(crate) type LexToken = OwnedLexToken;

impl PartialEq<TokenKind> for OwnedLexToken {
    fn eq(&self, other: &TokenKind) -> bool {
        self.kind == *other
    }
}

impl Location for OwnedLexToken {
    fn previous_token_end(&self) -> usize {
        self.span.end
    }

    fn current_token_start(&self) -> usize {
        self.span.start
    }
}

impl OwnedLexToken {
    pub(crate) fn new(kind: TokenKind, slice: impl Into<String>, span: TextSpan) -> Self {
        let slice = slice.into();
        let parser_text = parser_text_for_token(kind, slice.as_str());
        let parser_word_pieces =
            build_token_word_pieces(kind, slice.as_str(), parser_text.as_str(), span);
        Self {
            kind,
            slice,
            parser_text,
            parser_word_pieces,
            span,
        }
    }

    pub(crate) fn word(slice: impl Into<String>, span: TextSpan) -> Self {
        Self::new(TokenKind::Word, slice, span)
    }

    pub(crate) fn comma(span: TextSpan) -> Self {
        Self::new(TokenKind::Comma, ",", span)
    }

    pub(crate) fn period(span: TextSpan) -> Self {
        Self::new(TokenKind::Period, ".", span)
    }

    pub(crate) fn colon(span: TextSpan) -> Self {
        Self::new(TokenKind::Colon, ":", span)
    }

    pub(crate) fn semicolon(span: TextSpan) -> Self {
        Self::new(TokenKind::Semicolon, ";", span)
    }

    pub(crate) fn quote(span: TextSpan) -> Self {
        Self::new(TokenKind::Quote, "\"", span)
    }

    #[allow(dead_code)]
    pub(crate) fn synthetic_word(slice: impl Into<String>) -> Self {
        Self::word(slice, TextSpan::synthetic())
    }

    #[allow(dead_code)]
    pub(crate) fn synthetic_comma() -> Self {
        Self::comma(TextSpan::synthetic())
    }

    pub(crate) fn as_word(&self) -> Option<&str> {
        match self.kind {
            TokenKind::Word | TokenKind::Number | TokenKind::Ampersand => Some(self.slice.as_str()),
            TokenKind::Tilde => Some("this"),
            _ => None,
        }
    }

    pub(crate) fn parser_text(&self) -> &str {
        self.parser_text.as_str()
    }

    pub(crate) fn mana_group_inner(&self) -> Option<&str> {
        if self.kind != TokenKind::ManaGroup {
            return None;
        }
        self.slice
            .strip_prefix('{')
            .and_then(|inner| inner.strip_suffix('}'))
    }

    pub(crate) fn parser_word_pieces(&self) -> &[TokenWordPiece] {
        &self.parser_word_pieces
    }

    fn refresh_parser_word_pieces(&mut self) {
        self.parser_word_pieces = build_token_word_pieces(
            self.kind,
            self.slice.as_str(),
            self.parser_text.as_str(),
            self.span,
        );
    }

    pub(crate) fn replace_word(&mut self, slice: impl Into<String>) -> bool {
        match self.kind {
            TokenKind::Word | TokenKind::Number | TokenKind::Ampersand => {
                let slice = slice.into();
                self.parser_text = parser_text_for_token(self.kind, slice.as_str());
                self.slice = slice;
                self.refresh_parser_word_pieces();
                true
            }
            TokenKind::Tilde => {
                self.parser_text = "this".to_string();
                self.refresh_parser_word_pieces();
                true
            }
            _ => false,
        }
    }

    pub(crate) fn lowercase_word(&mut self) -> bool {
        match self.kind {
            TokenKind::Word | TokenKind::Number | TokenKind::Ampersand => {
                let lowered = self.slice.to_ascii_lowercase();
                self.replace_word(lowered)
            }
            TokenKind::Tilde => true,
            _ => false,
        }
    }

    pub(crate) fn is_word(&self, expected: &str) -> bool {
        matches!(
            self.kind,
            TokenKind::Word | TokenKind::Number | TokenKind::Ampersand | TokenKind::Tilde
        ) && self.parser_text == normalize_parser_fragment(expected)
    }

    pub(crate) fn is_any_word(&self, expected: &[&str]) -> bool {
        expected.iter().any(|word| self.is_word(word))
    }

    pub(crate) fn is_comma(&self) -> bool {
        self.kind == TokenKind::Comma
    }

    pub(crate) fn is_period(&self) -> bool {
        self.kind == TokenKind::Period
    }

    pub(crate) fn is_colon(&self) -> bool {
        self.kind == TokenKind::Colon
    }

    pub(crate) fn is_semicolon(&self) -> bool {
        self.kind == TokenKind::Semicolon
    }

    pub(crate) fn is_quote(&self) -> bool {
        self.kind == TokenKind::Quote
    }

    pub(crate) fn span(&self) -> TextSpan {
        self.span
    }
}

fn push_normalized_token_words(
    slice: &str,
    base_span: TextSpan,
    in_mana_braces: bool,
    out: &mut Vec<TokenWordPiece>,
) {
    let mut buffer = String::new();
    let mut piece_start: Option<usize> = None;
    let mut piece_end = base_span.start;
    let chars: Vec<(usize, char)> = slice.char_indices().collect();

    let flush = |buffer: &mut String,
                 out: &mut Vec<TokenWordPiece>,
                 piece_start: &mut Option<usize>,
                 piece_end: &mut usize| {
        if !buffer.is_empty() {
            out.push(TokenWordPiece {
                text: std::mem::take(buffer),
                span: TextSpan {
                    line: base_span.line,
                    start: piece_start.unwrap_or(base_span.start),
                    end: *piece_end,
                },
            });
        }
        *piece_start = None;
    };

    for (idx, (rel_idx, original_ch)) in chars.iter().copied().enumerate() {
        let mut normalized_ch = original_ch;
        if normalized_ch == '−' {
            normalized_ch = '-';
        }
        let prev = if idx > 0 { chars[idx - 1].1 } else { '\0' };
        let next = if idx + 1 < chars.len() {
            chars[idx + 1].1
        } else {
            '\0'
        };
        let is_counter_char = match normalized_ch {
            '+' | '-' => next.is_ascii_digit() || next == 'x' || next == 'X',
            '/' => {
                (prev.is_ascii_digit() || prev == 'x' || prev == 'X')
                    && (next.is_ascii_digit()
                        || next == '-'
                        || next == '+'
                        || next == 'x'
                        || next == 'X')
            }
            _ => false,
        };
        let is_mana_hybrid_slash = normalized_ch == '/' && in_mana_braces;

        if normalized_ch.is_ascii_alphanumeric() || is_counter_char || is_mana_hybrid_slash {
            if piece_start.is_none() {
                piece_start = Some(base_span.start + rel_idx);
            }
            piece_end = base_span.start + rel_idx + original_ch.len_utf8();
            buffer.push(normalized_ch.to_ascii_lowercase());
            continue;
        }

        if matches!(normalized_ch, '\'' | '’' | '‘') {
            if piece_start.is_some() {
                piece_end = base_span.start + rel_idx + original_ch.len_utf8();
            }
            continue;
        }

        flush(&mut buffer, out, &mut piece_start, &mut piece_end);
    }

    flush(&mut buffer, out, &mut piece_start, &mut piece_end);
}

pub(crate) fn token_word_pieces_for_token(token: &OwnedLexToken) -> &[TokenWordPiece] {
    token.parser_word_pieces()
}

pub(crate) fn word_slice_find_phrase_start(words: &[&str], expected: &[&str]) -> Option<usize> {
    crate::word_primitives::find_phrase_start(words, expected)
}

pub(crate) fn word_slice_find_phrase_start_or_zero(
    words: &[&str],
    expected: &[&str],
) -> Option<usize> {
    crate::word_primitives::find_phrase_start_or_zero(words, expected)
}

pub(crate) fn word_slice_find_any_phrase_start<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    crate::word_primitives::find_any_phrase_start(words, expected)
}

pub(crate) fn word_slice_find_any_phrase_span(
    words: &[&str],
    expected: &[&[&str]],
) -> Option<(usize, usize)> {
    crate::word_primitives::find_any_phrase_span(words, expected).map(|(_, idx, len)| (idx, len))
}

pub(crate) fn word_slice_find_phrase_value<T: Clone>(
    words: &[&str],
    expected: &[(&[&str], T)],
) -> Option<(T, usize)> {
    crate::word_primitives::find_phrase_value(words, expected)
}

pub(crate) fn word_slice_find_any_phrase_start_or_zero<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    crate::word_primitives::find_any_phrase_start_or_zero(words, expected)
}

pub(crate) fn word_slice_find_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> Option<usize> {
    crate::word_primitives::find_window_by(words, window_len, predicate)
}

pub(crate) fn word_slice_contains_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> bool {
    crate::word_primitives::contains_window_by(words, window_len, predicate)
}

pub(crate) fn word_slice_contains_phrase(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::contains_phrase(words, expected)
}

pub(crate) fn word_slice_contains_phrase_or_empty(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::contains_phrase_or_empty(words, expected)
}

pub(crate) fn word_slice_contains_any_phrase(words: &[&str], expected: &[&[&str]]) -> bool {
    crate::word_primitives::contains_any_phrase(words, expected)
}

pub(crate) fn word_slice_contains_any_phrase_or_empty(
    words: &[&str],
    expected: &[&[&str]],
) -> bool {
    crate::word_primitives::contains_any_phrase_or_empty(words, expected)
}

pub(crate) fn word_slice_eq(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::equals(words, expected)
}

pub(crate) fn word_slice_eq_any(words: &[&str], expected: &[&[&str]]) -> bool {
    crate::word_primitives::equals_any(words, expected)
}

pub(crate) fn word_slice_eq_at(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    crate::word_primitives::equals_at(words, idx, expected)
}

pub(crate) fn word_slice_eq_any_at(words: &[&str], idx: usize, expected: &[&[&str]]) -> bool {
    crate::word_primitives::equals_any_at(words, idx, expected)
}

pub(crate) fn word_slice_at_is(words: &[&str], idx: usize, expected: &str) -> bool {
    crate::word_primitives::at_is(words, idx, expected)
}

pub(crate) fn word_slice_at_is_any(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    crate::word_primitives::at_is_any(words, idx, expected)
}

pub(crate) fn word_slice_first_is(words: &[&str], expected: &str) -> bool {
    crate::word_primitives::first_is(words, expected)
}

pub(crate) fn word_slice_first_is_any(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::first_is_any(words, expected)
}

pub(crate) fn word_slice_last_is(words: &[&str], expected: &str) -> bool {
    crate::word_primitives::last_is(words, expected)
}

pub(crate) fn word_slice_last_is_any(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::last_is_any(words, expected)
}

pub(crate) fn word_slice_matching_phrase<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<&'p [&'p str]> {
    crate::word_primitives::matching_phrase(words, expected)
}

pub(crate) fn word_slice_matching_value<T: Clone>(
    words: &[&str],
    expected: &[(&[&str], T)],
) -> Option<T> {
    crate::word_primitives::matching_value(words, expected)
}

pub(crate) fn word_slice_ends_with(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::ends_with(words, expected)
}

pub(crate) fn word_slice_ends_with_any(words: &[&str], expected: &[&[&str]]) -> bool {
    crate::word_primitives::ends_with_any(words, expected)
}

pub(crate) fn word_slice_starts_with(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::starts_with(words, expected)
}

pub(crate) fn word_slice_starts_with_at(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    crate::word_primitives::starts_with_at(words, idx, expected)
}

pub(crate) fn word_slice_starts_with_any(words: &[&str], expected: &[&[&str]]) -> bool {
    crate::word_primitives::starts_with_any(words, expected)
}

pub(crate) fn word_slice_strip_prefix<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    crate::word_primitives::strip_prefix(words, expected)
}

pub(crate) fn word_slice_strip_suffix<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    crate::word_primitives::strip_suffix(words, expected)
}

pub(crate) fn word_slice_strip_any_prefix<'a, 'p>(
    words: &'a [&'a str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [&'a str])> {
    crate::word_primitives::strip_any_prefix(words, expected)
}

pub(crate) fn word_slice_strip_prefix_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&[&str], T)],
) -> Option<(T, &'w [&'a str])> {
    crate::word_primitives::strip_prefix_value(words, expected)
}

pub(crate) fn word_slice_strip_first_word<'w, 'a>(
    words: &'w [&'a str],
    expected: &str,
) -> Option<&'w [&'a str]> {
    crate::word_primitives::strip_first_word(words, expected)
}

pub(crate) fn word_slice_strip_first_word_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&str, T)],
) -> Option<(T, &'w [&'a str])> {
    crate::word_primitives::strip_first_word_value(words, expected)
}

pub(crate) fn word_slice_strip_any_suffix<'a, 'p>(
    words: &'a [&'a str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [&'a str])> {
    crate::word_primitives::strip_any_suffix(words, expected)
}

pub(crate) fn word_slice_strip_suffix_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&[&str], T)],
) -> Option<(T, &'w [&'a str])> {
    crate::word_primitives::strip_suffix_value(words, expected)
}

pub(crate) fn word_slice_contains_word(words: &[&str], expected: &str) -> bool {
    crate::word_primitives::contains_word(words, expected)
}

pub(crate) fn word_slice_find_word(words: &[&str], expected: &str) -> Option<usize> {
    crate::word_primitives::find_word(words, expected)
}

pub(crate) fn word_slice_find_any_word(words: &[&str], expected: &[&str]) -> Option<usize> {
    crate::word_primitives::find_any_word(words, expected)
}

pub(crate) fn word_slice_find_any_word_from(
    words: &[&str],
    expected: &[&str],
    start: usize,
) -> Option<usize> {
    crate::word_primitives::find_any_word_from(words, expected, start)
}

pub(crate) fn word_slice_find_word_where(
    words: &[&str],
    predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    crate::word_primitives::find_word_where(words, predicate)
}

pub(crate) fn word_slice_rfind_word_where(
    words: &[&str],
    predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    crate::word_primitives::rfind_word_where(words, predicate)
}

pub(crate) fn word_slice_contains_any_word(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::contains_any_word(words, expected)
}

pub(crate) fn word_slice_contains_no_words(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::contains_no_words(words, expected)
}

pub(crate) fn word_slice_contains_all_words(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::contains_all_words(words, expected)
}

pub(crate) fn word_slice_all_words_are_any(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::all_words_are_any(words, expected)
}

pub(crate) fn find_token_word_sequence(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<usize> {
    if expected.is_empty() {
        return None;
    }
    crate::slice_primitives::find_window_by(tokens, expected.len(), |window| {
        window
            .iter()
            .zip(expected.iter())
            .all(|(token, expected_word)| token.is_word(expected_word))
    })
}

pub(crate) fn find_token_word_sequence_span(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<(usize, usize)> {
    find_token_word_sequence(tokens, expected).map(|start| (start, start + expected.len()))
}

pub(crate) fn find_any_token_word_sequence_span<'p>(
    tokens: &[OwnedLexToken],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize, usize)> {
    expected
        .iter()
        .filter_map(|phrase| {
            find_token_word_sequence_span(tokens, phrase).map(|(start, end)| (*phrase, start, end))
        })
        .min_by_key(|(_, start, _)| *start)
}

pub(crate) fn find_token_word_sequence_value<T: Clone>(
    tokens: &[OwnedLexToken],
    expected: &[(&[&str], T)],
) -> Option<(T, usize, usize)> {
    expected
        .iter()
        .filter_map(|(phrase, value)| {
            find_token_word_sequence_span(tokens, phrase)
                .map(|(start, end)| (value.clone(), start, end))
        })
        .min_by_key(|(_, start, _)| *start)
}

pub(crate) fn contains_token_word_sequence(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    find_token_word_sequence(tokens, expected).is_some()
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

pub(crate) fn token_slice_at_kind(
    tokens: &[OwnedLexToken],
    idx: usize,
    expected: TokenKind,
) -> bool {
    tokens.get(idx).is_some_and(|token| token.kind == expected)
}

pub(crate) fn token_slice_first_is(tokens: &[OwnedLexToken], expected: &str) -> bool {
    token_slice_at_is(tokens, 0, expected)
}

pub(crate) fn token_slice_first_is_any(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    token_slice_at_is_any(tokens, 0, expected)
}

pub(crate) fn token_slice_first_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    token_slice_at_kind(tokens, 0, expected)
}

pub(crate) fn token_slice_last_is(tokens: &[OwnedLexToken], expected: &str) -> bool {
    tokens.last().is_some_and(|token| token.is_word(expected))
}

pub(crate) fn token_slice_last_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    tokens.last().is_some_and(|token| token.kind == expected)
}

pub(crate) fn token_slice_starts_with(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    tokens.len() >= expected.len()
        && tokens
            .iter()
            .take(expected.len())
            .zip(expected.iter())
            .all(|(token, expected_word)| token.is_word(expected_word))
}

pub(crate) fn token_slice_starts_with_at(
    tokens: &[OwnedLexToken],
    idx: usize,
    expected: &[&str],
) -> bool {
    tokens
        .get(idx..)
        .is_some_and(|tail| token_slice_starts_with(tail, expected))
}

pub(crate) fn token_slice_starts_with_any(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|phrase| token_slice_starts_with(tokens, phrase))
}

pub(crate) fn token_slice_words_eq(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let view = TokenWordView::new(tokens);
    view.len() == expected.len() && view.slice_eq(0, expected)
}

pub(crate) fn token_slice_words_eq_any(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|phrase| token_slice_words_eq(tokens, phrase))
}

pub(crate) fn token_slice_ends_with(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let view = TokenWordView::new(tokens);
    view.len() >= expected.len() && view.slice_eq(view.len() - expected.len(), expected)
}

pub(crate) fn token_slice_ends_with_any(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|phrase| token_slice_ends_with(tokens, phrase))
}

pub(crate) fn token_slice_strip_word_prefix<'a>(
    tokens: &'a [OwnedLexToken],
    expected: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    if expected.is_empty() {
        return Some(tokens);
    }
    let view = TokenWordView::new(tokens);
    if !view.starts_with(expected) {
        return None;
    }
    let token_end = view.token_index_after_words(expected.len())?;
    Some(&tokens[token_end..])
}

pub(crate) fn token_slice_strip_any_word_prefix<'a, 'p>(
    tokens: &'a [OwnedLexToken],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [OwnedLexToken])> {
    expected.iter().find_map(|phrase| {
        token_slice_strip_word_prefix(tokens, phrase).map(|rest| (*phrase, rest))
    })
}

pub(crate) fn find_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    crate::slice_primitives::find_index(tokens, |token| token.is_word(expected))
}

pub(crate) fn find_token_any_word(tokens: &[OwnedLexToken], expected: &[&str]) -> Option<usize> {
    crate::slice_primitives::find_index(tokens, |token| token.is_any_word(expected))
}

pub(crate) fn rfind_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    crate::slice_primitives::rfind_index(tokens, |token| token.is_word(expected))
}

pub(crate) fn contains_token_word(tokens: &[OwnedLexToken], expected: &str) -> bool {
    find_token_word(tokens, expected).is_some()
}

pub(crate) fn contains_token_any_word(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    find_token_any_word(tokens, expected).is_some()
}

pub(crate) fn find_token_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> Option<usize> {
    crate::slice_primitives::find_index(tokens, |token| token.kind == expected)
}

pub(crate) fn contains_token_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    find_token_kind(tokens, expected).is_some()
}

pub(crate) fn token_slice_all_are_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    crate::slice_primitives::all_match(tokens, |token| token.kind == expected)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenWordView<'a> {
    words: Vec<&'a str>,
    token_start_indices: Vec<usize>,
    token_end_indices: Vec<usize>,
    token_len: usize,
}

impl<'a> TokenWordView<'a> {
    pub(crate) fn new(tokens: &'a [OwnedLexToken]) -> Self {
        let mut words = Vec::new();
        let mut token_start_indices = Vec::new();
        let mut token_end_indices = Vec::new();
        let mut token_idx = 0usize;
        while token_idx < tokens.len() {
            let token = &tokens[token_idx];
            let pieces = token_word_pieces_for_token(token);
            if pieces.is_empty() {
                token_idx += 1;
                continue;
            }
            for piece in pieces {
                words.push(piece.text.as_str());
                token_start_indices.push(token_idx);
                token_end_indices.push(token_idx + 1);
            }
            token_idx += 1;
        }
        Self {
            words,
            token_start_indices,
            token_end_indices,
            token_len: tokens.len(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.words.len()
    }

    pub(crate) fn get(&self, idx: usize) -> Option<&'a str> {
        self.words.get(idx).copied()
    }

    pub(crate) fn starts_with(&self, expected: &[&str]) -> bool {
        word_slice_starts_with(&self.words, expected)
    }

    pub(crate) fn starts_with_at(&self, idx: usize, expected: &[&str]) -> bool {
        word_slice_starts_with_at(&self.words, idx, expected)
    }

    pub(crate) fn starts_with_any(&self, expected: &[&[&str]]) -> bool {
        word_slice_starts_with_any(&self.words, expected)
    }

    pub(crate) fn equals_at(&self, idx: usize, expected: &[&str]) -> bool {
        word_slice_eq_at(&self.words, idx, expected)
    }

    pub(crate) fn equals_any_at(&self, idx: usize, expected: &[&[&str]]) -> bool {
        word_slice_eq_any_at(&self.words, idx, expected)
    }

    pub(crate) fn ends_with(&self, expected: &[&str]) -> bool {
        word_slice_ends_with(&self.words, expected)
    }

    pub(crate) fn ends_with_any(&self, expected: &[&[&str]]) -> bool {
        word_slice_ends_with_any(&self.words, expected)
    }

    pub(crate) fn slice_eq(&self, start: usize, expected: &[&str]) -> bool {
        self.words
            .get(start..start.saturating_add(expected.len()))
            .is_some_and(|slice| {
                slice
                    .iter()
                    .copied()
                    .zip(expected.iter().copied())
                    .all(|(actual, expected)| actual == expected)
            })
    }

    pub(crate) fn find_phrase_start(&self, expected: &[&str]) -> Option<usize> {
        word_slice_find_phrase_start(&self.words, expected)
    }

    pub(crate) fn find_any_phrase_start<'p>(
        &self,
        expected: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], usize)> {
        word_slice_find_any_phrase_start(&self.words, expected)
    }

    pub(crate) fn find_phrase_value<T: Clone>(
        &self,
        expected: &[(&[&str], T)],
    ) -> Option<(T, usize)> {
        word_slice_find_phrase_value(&self.words, expected)
    }

    pub(crate) fn find_window_by(
        &self,
        window_len: usize,
        predicate: impl FnMut(&[&str]) -> bool,
    ) -> Option<usize> {
        word_slice_find_window_by(&self.words, window_len, predicate)
    }

    pub(crate) fn contains_window_by(
        &self,
        window_len: usize,
        predicate: impl FnMut(&[&str]) -> bool,
    ) -> bool {
        word_slice_contains_window_by(&self.words, window_len, predicate)
    }

    pub(crate) fn has_phrase(&self, expected: &[&str]) -> bool {
        word_slice_contains_phrase(&self.words, expected)
    }

    pub(crate) fn has_any_phrase(&self, expected: &[&[&str]]) -> bool {
        word_slice_contains_any_phrase(&self.words, expected)
    }

    pub(crate) fn find_word(&self, expected: &str) -> Option<usize> {
        word_slice_find_word(&self.words, expected)
    }

    pub(crate) fn find_any_word(&self, expected: &[&str]) -> Option<usize> {
        word_slice_find_any_word(&self.words, expected)
    }

    pub(crate) fn find_any_word_from(&self, expected: &[&str], start: usize) -> Option<usize> {
        word_slice_find_any_word_from(&self.words, expected, start)
    }

    pub(crate) fn rfind_word(&self, expected: &str) -> Option<usize> {
        word_slice_rfind_word_where(&self.words, |word| word == expected)
    }

    pub(crate) fn contains_word(&self, expected: &str) -> bool {
        word_slice_contains_word(&self.words, expected)
    }

    pub(crate) fn contains_any_word(&self, expected: &[&str]) -> bool {
        word_slice_contains_any_word(&self.words, expected)
    }

    pub(crate) fn contains_no_words(&self, expected: &[&str]) -> bool {
        word_slice_contains_no_words(&self.words, expected)
    }

    pub(crate) fn contains_all_words(&self, expected: &[&str]) -> bool {
        word_slice_contains_all_words(&self.words, expected)
    }

    pub(crate) fn at_is(&self, idx: usize, expected: &str) -> bool {
        word_slice_at_is(&self.words, idx, expected)
    }

    pub(crate) fn at_is_any(&self, idx: usize, expected: &[&str]) -> bool {
        word_slice_at_is_any(&self.words, idx, expected)
    }

    pub(crate) fn first_is(&self, expected: &str) -> bool {
        word_slice_first_is(&self.words, expected)
    }

    pub(crate) fn first_is_any(&self, expected: &[&str]) -> bool {
        word_slice_first_is_any(&self.words, expected)
    }

    pub(crate) fn last_is(&self, expected: &str) -> bool {
        word_slice_last_is(&self.words, expected)
    }

    pub(crate) fn last_is_any(&self, expected: &[&str]) -> bool {
        word_slice_last_is_any(&self.words, expected)
    }

    pub(crate) fn matching_value<T: Clone>(&self, expected: &[(&[&str], T)]) -> Option<T> {
        word_slice_matching_value(&self.words, expected)
    }

    pub(crate) fn strip_prefix_value<'s, T: Clone>(
        &'s self,
        expected: &[(&[&str], T)],
    ) -> Option<(T, &'s [&'a str])> {
        word_slice_strip_prefix_value(&self.words, expected)
    }

    pub(crate) fn strip_first_word_value<'s, T: Clone>(
        &'s self,
        expected: &[(&str, T)],
    ) -> Option<(T, &'s [&'a str])> {
        word_slice_strip_first_word_value(&self.words, expected)
    }

    pub(crate) fn strip_suffix_value<'s, T: Clone>(
        &'s self,
        expected: &[(&[&str], T)],
    ) -> Option<(T, &'s [&'a str])> {
        word_slice_strip_suffix_value(&self.words, expected)
    }

    pub(crate) fn first(&self) -> Option<&str> {
        self.get(0)
    }

    pub(crate) fn word_refs(&self) -> Vec<&'a str> {
        self.words.clone()
    }

    pub(crate) fn join(&self, separator: &str) -> String {
        self.words.join(separator)
    }

    pub(crate) fn owned_words(&self) -> Vec<String> {
        self.words.iter().map(|word| (*word).to_string()).collect()
    }

    pub(crate) fn to_word_refs(&self) -> Vec<&'a str> {
        self.word_refs()
    }

    pub(crate) fn token_index_for_word_index(&self, word_idx: usize) -> Option<usize> {
        self.token_start_indices.get(word_idx).copied()
    }

    pub(crate) fn token_index_for_word_or_end(&self, word_idx: usize) -> Option<usize> {
        if word_idx == self.len() {
            Some(self.token_len)
        } else {
            self.token_index_for_word_index(word_idx)
        }
    }

    pub(crate) fn token_start_indices(&self) -> &[usize] {
        &self.token_start_indices
    }

    pub(crate) fn token_index_after_words(&self, word_count: usize) -> Option<usize> {
        if word_count == 0 {
            return Some(0);
        }
        if word_count > self.len() {
            return None;
        }
        self.token_end_indices.get(word_count - 1).copied()
    }

    pub(crate) fn token_index_after_words_or_end(&self, word_count: usize) -> Option<usize> {
        if word_count == 0 {
            return Some(0);
        }
        if word_count > self.len() {
            return None;
        }
        if word_count == self.len() {
            return Some(self.token_len);
        }
        self.token_index_after_words(word_count)
    }

    pub(crate) fn token_range_for_word_range(
        &self,
        start_word: usize,
        end_word: usize,
    ) -> Option<std::ops::Range<usize>> {
        if start_word > end_word || end_word > self.len() {
            return None;
        }
        let start = if start_word == end_word {
            self.token_index_after_words(start_word)?
        } else {
            self.token_index_for_word_index(start_word)?
        };
        let end = self.token_index_after_words(end_word)?;
        Some(start..end)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LexedClause<'a> {
    tokens: &'a [OwnedLexToken],
}

#[allow(dead_code)]
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

    pub(crate) fn word_refs_where(self, mut predicate: impl FnMut(&str) -> bool) -> Vec<&'a str> {
        self.word_refs()
            .into_iter()
            .filter(|word| predicate(word))
            .collect()
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

    pub(crate) fn first_is_word(self, expected: &str) -> bool {
        let atoms = [LexPatternAtom::Word(expected)];
        LexPattern::new(&atoms).matches_prefix(self)
    }

    pub(crate) fn first_is_any_word(self, expected: &[&str]) -> bool {
        let atoms = [LexPatternAtom::AnyWord(expected)];
        LexPattern::new(&atoms).matches_prefix(self)
    }

    pub(crate) fn first_word(self) -> Option<&'a str> {
        self.tokens
            .iter()
            .flat_map(|token| token.parser_word_pieces().iter())
            .map(|piece| piece.text.as_str())
            .next()
    }

    pub(crate) fn matches_words(self, expected: &[&str]) -> bool {
        let atoms = [LexPatternAtom::Phrase(expected)];
        LexPattern::new(&atoms).matches_clause(self)
    }

    pub(crate) fn matches_any_words(self, phrases: &[&[&str]]) -> bool {
        let atoms = [LexPatternAtom::AnyPhrase(phrases)];
        LexPattern::new(&atoms).matches_clause(self)
    }

    pub(crate) fn matching_word_value<T: Clone>(self, phrases: &[(&[&str], T)]) -> Option<T> {
        let words = self.word_refs();
        word_slice_matching_value(words.as_slice(), phrases)
    }

    pub(crate) fn find_word_value<T: Clone>(self, phrases: &[(&[&str], T)]) -> Option<(T, usize)> {
        let words = self.word_refs();
        word_slice_find_phrase_value(words.as_slice(), phrases)
    }

    pub(crate) fn starts_with(self, expected: &[&str]) -> bool {
        let atoms = [LexPatternAtom::Phrase(expected)];
        LexPattern::new(&atoms).matches_prefix(self)
    }

    pub(crate) fn starts_with_any(self, phrases: &[&[&str]]) -> bool {
        let atoms = [LexPatternAtom::AnyPhrase(phrases)];
        LexPattern::new(&atoms).matches_prefix(self)
    }

    pub(crate) fn ends_with(self, expected: &[&str]) -> bool {
        let words = self.word_refs();
        word_slice_ends_with(&words, expected)
    }

    pub(crate) fn ends_with_any(self, phrases: &[&[&str]]) -> bool {
        let words = self.word_refs();
        word_slice_ends_with_any(&words, phrases)
    }

    pub(crate) fn strip_prefix_clause(self, expected: &[&str]) -> Option<Self> {
        let atoms = [LexPatternAtom::Phrase(expected)];
        let matched = LexPattern::new(&atoms).match_prefix(self)?;
        let token_idx = self
            .words()
            .token_index_after_words(matched.word_range.end)?;
        Some(self.from(token_idx))
    }

    pub(crate) fn strip_any_prefix_clause<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], Self)> {
        let words = self.word_refs();
        let atoms = [LexPatternAtom::AnyPhrase(phrases)];
        let matched = LexPattern::new(&atoms).match_prefix(self)?;
        let phrase = phrases.iter().copied().find(|phrase| {
            words
                .get(0..phrase.len())
                .is_some_and(|head| head == *phrase)
        })?;
        let token_idx = self
            .words()
            .token_index_after_words(matched.word_range.end)?;
        Some((phrase, self.from(token_idx)))
    }

    pub(crate) fn strip_prefix_value_clause<T: Clone>(
        self,
        phrases: &[(&[&str], T)],
    ) -> Option<(T, Self)> {
        let words = self.word_refs();
        word_slice_strip_prefix_value(&words, phrases).and_then(|(value, tail_words)| {
            let consumed = words.len().saturating_sub(tail_words.len());
            let token_idx = self.words().token_index_after_words(consumed)?;
            Some((value, self.from(token_idx)))
        })
    }

    pub(crate) fn strip_suffix_clause(self, expected: &[&str]) -> Option<Self> {
        let words = self.words();
        if words.len() < expected.len() {
            return None;
        }
        if !self.ends_with(expected) {
            return None;
        }
        let word_count = words.len().saturating_sub(expected.len());
        let token_idx = words.token_index_after_words(word_count)?;
        Some(self.before(token_idx))
    }

    pub(crate) fn strip_any_suffix_clause<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], Self)> {
        let words = self.word_refs();
        word_slice_strip_any_suffix(&words, phrases)
            .and_then(|(phrase, _)| self.strip_suffix_clause(phrase).map(|head| (phrase, head)))
    }

    pub(crate) fn strip_suffix_value_clause<T: Clone>(
        self,
        phrases: &[(&[&str], T)],
    ) -> Option<(T, Self)> {
        let words = self.word_refs();
        word_slice_strip_suffix_value(&words, phrases).and_then(|(value, head_words)| {
            let token_idx = self.words().token_index_after_words(head_words.len())?;
            Some((value, self.before(token_idx)))
        })
    }

    pub(crate) fn token_index_for_word_index(self, word_idx: usize) -> Option<usize> {
        self.words().token_index_for_word_index(word_idx)
    }

    pub(crate) fn token_index_for_word_or_end(self, word_idx: usize) -> Option<usize> {
        self.words().token_index_for_word_or_end(word_idx)
    }

    pub(crate) fn token_index_after_words(self, word_count: usize) -> Option<usize> {
        self.words().token_index_after_words(word_count)
    }

    pub(crate) fn before_word(self, word_idx: usize) -> Option<Self> {
        let token_idx = self.token_index_for_word_index(word_idx)?;
        Some(self.before(token_idx))
    }

    pub(crate) fn from_word(self, word_idx: usize) -> Option<Self> {
        let token_idx = self.token_index_for_word_index(word_idx)?;
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
        let range = self.words().token_range_for_word_range(start, end)?;
        Some(self.between(range.start, range.end))
    }

    pub(crate) fn without_word_range_trimmed(
        self,
        word_start: usize,
        word_len: usize,
    ) -> Vec<OwnedLexToken> {
        let token_start = self
            .token_index_for_word_index(word_start)
            .unwrap_or(self.tokens.len());
        let token_end = self
            .token_index_for_word_index(word_start + word_len)
            .unwrap_or(self.tokens.len());
        let mut tokens = self.tokens[..token_start].to_vec();
        tokens.extend_from_slice(&self.tokens[token_end..]);
        LexedClause::new(&tokens).trim()
    }

    pub(crate) fn without_phrase_trimmed(self, phrase: &[&str]) -> Option<Vec<OwnedLexToken>> {
        let word_start = self.find_phrase_start(phrase)?;
        Some(self.without_word_range_trimmed(word_start, phrase.len()))
    }

    pub(crate) fn without_any_phrase_trimmed<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], Vec<OwnedLexToken>)> {
        let (phrase, word_start) = self.find_any_phrase_start(phrases)?;
        Some((
            phrase,
            self.without_word_range_trimmed(word_start, phrase.len()),
        ))
    }

    pub(crate) fn find_word(self, expected: &str) -> Option<usize> {
        let atoms = [LexPatternAtom::Word(expected)];
        LexPattern::new(&atoms)
            .find_in_clause(self)
            .map(|matched| matched.word_range.start)
    }

    pub(crate) fn find_word_any(self, expected: &[&str]) -> Option<usize> {
        let atoms = [LexPatternAtom::AnyWord(expected)];
        LexPattern::new(&atoms)
            .find_in_clause(self)
            .map(|matched| matched.word_range.start)
    }

    pub(crate) fn rfind_word(self, expected: &str) -> Option<usize> {
        self.words().rfind_word(expected)
    }

    pub(crate) fn find_phrase_start(self, expected: &[&str]) -> Option<usize> {
        let atoms = [LexPatternAtom::Phrase(expected)];
        LexPattern::new(&atoms)
            .find_in_clause(self)
            .map(|matched| matched.word_range.start)
    }

    pub(crate) fn find_any_phrase_start<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], usize)> {
        let words = self.word_refs();
        let atoms = [LexPatternAtom::AnyPhrase(phrases)];
        let matched = LexPattern::new(&atoms).find_in_clause(self)?;
        let phrase = phrases.iter().copied().find(|phrase| {
            words
                .get(matched.word_range.start..matched.word_range.start + phrase.len())
                .is_some_and(|window| window == *phrase)
        })?;
        Some((phrase, matched.word_range.start))
    }

    pub(crate) fn find_any_phrase_span<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(usize, usize)> {
        let words = self.word_refs();
        word_slice_find_any_phrase_span(&words, phrases)
    }

    pub(crate) fn contains_word(self, expected: &str) -> bool {
        self.find_word(expected).is_some()
    }

    pub(crate) fn contains_any_word(self, expected: &[&str]) -> bool {
        self.find_word_any(expected).is_some()
    }

    pub(crate) fn contains_no_words(self, expected: &[&str]) -> bool {
        self.words().contains_no_words(expected)
    }

    pub(crate) fn count_word(self, expected: &str) -> usize {
        self.word_refs()
            .into_iter()
            .filter(|word| *word == expected)
            .count()
    }

    pub(crate) fn contains_comma(self) -> bool {
        contains_token_kind(self.tokens, TokenKind::Comma)
    }

    pub(crate) fn contains_comma_or_any_word(self, expected: &[&str]) -> bool {
        self.contains_comma() || self.contains_any_word(expected)
    }

    pub(crate) fn contains_all_words(self, expected: &[&str]) -> bool {
        self.words().contains_all_words(expected)
    }

    pub(crate) fn contains_phrase(self, expected: &[&str]) -> bool {
        self.words().has_phrase(expected)
    }

    pub(crate) fn contains_any_phrase(self, phrases: &[&[&str]]) -> bool {
        self.words().has_any_phrase(phrases)
    }

    pub(crate) fn find_token_word(self, expected: &str) -> Option<usize> {
        find_token_word(self.tokens, expected)
    }

    pub(crate) fn find_token_word_any(self, expected: &[&str]) -> Option<usize> {
        find_token_any_word(self.tokens, expected)
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

    pub(crate) fn rfind_token_word(self, expected: &str) -> Option<usize> {
        rfind_token_word(self.tokens, expected)
    }

    pub(crate) fn split_once_on_word(self, expected: &str) -> Option<(Self, Self)> {
        let idx = self.find_token_word(expected)?;
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

    pub(crate) fn split_once_on_word_any_trimmed(self, expected: &[&str]) -> Option<(Self, Self)> {
        self.split_once_on_word_any(expected)
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    pub(crate) fn rsplit_once_on_word(self, expected: &str) -> Option<(Self, Self)> {
        let idx = self.rfind_token_word(expected)?;
        Some((self.before(idx), self.from(idx + 1)))
    }

    pub(crate) fn rsplit_once_on_word_trimmed(self, expected: &str) -> Option<(Self, Self)> {
        self.rsplit_once_on_word(expected)
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    pub(crate) fn split_once_on_comma(self) -> Option<(Self, Self)> {
        let idx = find_token_kind(self.tokens, TokenKind::Comma)?;
        Some((self.before(idx), self.from(idx + 1)))
    }

    pub(crate) fn split_once_before_word(self, expected: &str) -> Option<(Self, Self)> {
        let idx = self.find_token_word(expected)?;
        Some((self.before(idx), self.from(idx)))
    }

    pub(crate) fn split_once_before_phrase(self, expected: &[&str]) -> Option<(Self, Self)> {
        let word_idx = self.find_phrase_start(expected)?;
        let token_idx = self.token_index_for_word_index(word_idx)?;
        Some((self.before(token_idx), self.from(token_idx)))
    }

    pub(crate) fn split_once_on_phrase(self, expected: &[&str]) -> Option<(Self, Self)> {
        let word_idx = self.find_phrase_start(expected)?;
        let start_token_idx = self.token_index_for_word_index(word_idx)?;
        let end_token_idx = self.token_index_after_words(word_idx + expected.len())?;
        Some((self.before(start_token_idx), self.from(end_token_idx)))
    }

    pub(crate) fn split_once_before_any_phrase(
        self,
        phrases: &[&'static [&'static str]],
    ) -> Option<(&'static [&'static str], Self, Self)> {
        phrases
            .iter()
            .filter_map(|phrase| {
                self.split_once_before_phrase(phrase)
                    .and_then(|(head, tail)| {
                        let word_idx = self.find_phrase_start(phrase)?;
                        Some((*phrase, word_idx, head, tail))
                    })
            })
            .min_by_key(|(_, word_idx, _, _)| *word_idx)
            .map(|(phrase, _, head, tail)| (phrase, head, tail))
    }

    pub(crate) fn split_once_on_any_phrase(
        self,
        phrases: &[&'static [&'static str]],
    ) -> Option<(&'static [&'static str], Self, Self)> {
        phrases
            .iter()
            .filter_map(|phrase| {
                self.split_once_on_phrase(phrase).and_then(|(head, tail)| {
                    let word_idx = self.find_phrase_start(phrase)?;
                    Some((*phrase, word_idx, head, tail))
                })
            })
            .min_by_key(|(_, word_idx, _, _)| *word_idx)
            .map(|(phrase, _, head, tail)| (phrase, head, tail))
    }

    pub(crate) fn take_until_token_matching<F>(self, mut predicate: F) -> Self
    where
        F: FnMut(&OwnedLexToken) -> bool,
    {
        let idx = crate::slice_primitives::find_index(self.tokens, |token| predicate(token))
            .unwrap_or(self.tokens.len());
        self.before(idx)
    }

    pub(crate) fn trimmed(self) -> Self {
        Self::new(trim_lexed_commas(self.tokens))
    }

    pub(crate) fn trim(self) -> Vec<OwnedLexToken> {
        self.trimmed().tokens().to_vec()
    }

    pub(crate) fn trimmed_tokens(self) -> &'a [OwnedLexToken] {
        self.trimmed().tokens()
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

    pub(crate) fn trimmed_and_segments(self) -> Vec<Self> {
        self.and_segments().into_iter().map(Self::trimmed).collect()
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

    pub(crate) fn split_comma_then_trimmed(self) -> Option<(Self, Self)> {
        self.split_comma_then()
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    pub(crate) fn split_once_on_then(self) -> Option<(Self, Self)> {
        self.split_comma_then()
            .or_else(|| self.split_once_on_word("then"))
    }

    pub(crate) fn split_once_on_then_trimmed(self) -> Option<(Self, Self)> {
        self.split_once_on_then()
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    pub(crate) fn comma_then_idx(self) -> Option<usize> {
        self.split_comma_then().map(|(head, _)| head.len())
    }

    pub(crate) fn without_leading_connectors_clause(self) -> Self {
        let trimmed = self.trimmed();
        let mut start = 0usize;
        while start < trimmed.tokens.len()
            && trimmed.tokens[start]
                .as_word()
                .is_some_and(|word| matches!(word, "then" | "and"))
        {
            start += 1;
        }
        trimmed.from(start)
    }

    pub(crate) fn without_trailing_words_clause(self, words: &[&str]) -> Self {
        let trimmed = self.trimmed();
        let mut end = trimmed.tokens.len();
        while end > 0
            && trimmed.tokens[end - 1]
                .as_word()
                .is_some_and(|word| crate::word_primitives::contains_word(words, word))
        {
            end -= 1;
        }
        trimmed.before(end)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LexCursor<'a> {
    tokens: &'a [OwnedLexToken],
    pos: usize,
}

pub(crate) type LexStream<'a> = TokenSlice<'a, LexToken>;

impl<'a> LexCursor<'a> {
    pub(crate) fn new(tokens: &'a [OwnedLexToken]) -> Self {
        Self { tokens, pos: 0 }
    }

    pub(crate) fn peek(&self) -> Option<&'a OwnedLexToken> {
        self.tokens.get(self.pos)
    }

    pub(crate) fn peek_n(&self, offset: usize) -> Option<&'a OwnedLexToken> {
        self.tokens.get(self.pos + offset)
    }

    pub(crate) fn advance(&mut self) -> Option<&'a OwnedLexToken> {
        let token = self.peek()?;
        self.pos += 1;
        Some(token)
    }

    pub(crate) fn remaining(&self) -> &'a [OwnedLexToken] {
        self.tokens.get(self.pos..).unwrap_or_default()
    }

    pub(crate) fn position(&self) -> usize {
        self.pos
    }
}

pub(crate) fn token_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str> {
    tokens.iter().filter_map(OwnedLexToken::as_word).collect()
}

pub(crate) fn synthetic_word_tokens<I, W>(words: I) -> Vec<OwnedLexToken>
where
    I: IntoIterator<Item = W>,
    W: AsRef<str>,
{
    words
        .into_iter()
        .map(|word| OwnedLexToken::synthetic_word(word.as_ref()))
        .collect()
}

pub(crate) fn parser_token_word_refs<'a>(tokens: &'a [OwnedLexToken]) -> Vec<&'a str> {
    let mut words = Vec::new();
    for token in tokens {
        for piece in token.parser_word_pieces() {
            words.push(piece.text.as_str());
        }
    }
    words
}

pub(crate) fn parser_token_word_positions<'a>(
    tokens: &'a [OwnedLexToken],
) -> Vec<(usize, &'a str)> {
    let mut positions = Vec::new();
    for (token_idx, token) in tokens.iter().enumerate() {
        for piece in token.parser_word_pieces() {
            positions.push((token_idx, piece.text.as_str()));
        }
    }
    positions
}

pub(crate) fn render_token_slice(tokens: &[OwnedLexToken]) -> String {
    fn needs_space(prev: &OwnedLexToken, current: &OwnedLexToken) -> bool {
        if prev.span.end == current.span.start {
            return false;
        }

        if matches!(
            current.kind,
            TokenKind::Comma
                | TokenKind::Period
                | TokenKind::Colon
                | TokenKind::Semicolon
                | TokenKind::Question
                | TokenKind::Bang
                | TokenKind::RParen
                | TokenKind::RBracket
        ) {
            return false;
        }

        !matches!(
            prev.kind,
            TokenKind::LBracket
                | TokenKind::LParen
                | TokenKind::Quote
                | TokenKind::Apostrophe
                | TokenKind::Plus
                | TokenKind::Dash
        )
    }

    let mut rendered = String::new();
    let mut previous_token = None;

    for token in tokens {
        if let Some(previous_token) = previous_token
            && needs_space(previous_token, token)
        {
            rendered.push(' ');
        }
        rendered.push_str(&token.slice);
        previous_token = Some(token);
    }

    rendered
}

#[allow(dead_code)]
pub(crate) fn trim_lexed_commas(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && tokens[start].kind == TokenKind::Comma {
        start += 1;
    }
    while end > start && tokens[end - 1].kind == TokenKind::Comma {
        end -= 1;
    }
    &tokens[start..end]
}

pub(crate) fn split_lexed_sentences(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    super::grammar::structure::split_lexed_sentences(tokens)
}

pub(crate) fn lex_line(line: &str, line_index: usize) -> Result<Vec<OwnedLexToken>, CardTextError> {
    let mut tokens = Vec::new();

    for (kind_result, span) in TokenKind::lexer(line).spanned() {
        let start = span.start;
        let end = span.end;
        let slice = &line[start..end];
        let span = TextSpan {
            line: line_index,
            start,
            end,
        };

        let Ok(kind) = kind_result else {
            let display_line = line_index + 1;
            return Err(CardTextError::ParseError(format!(
                "rewrite lexer encountered an unsupported token {slice:?} on line {display_line} at {start}..{end}",
            )));
        };

        tokens.push(OwnedLexToken::new(kind, slice, span));
    }

    Ok(tokens)
}
