use logos::Logos;
use winnow::stream::{Location, TokenSlice};

use crate::diagnostics::{CardTextError, TextSpan};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LexerError {
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
        _ => normalize_parser_fragment(slice),
    }
}

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"[ \t\r\n\f]+", error = LexerError)]
pub enum TokenKind {
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
    #[token("~")]
    Tilde,
    #[token("-")]
    #[token("−")]
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
        r"(?:\+[0-9xX]+|-[0-9xX]+|[\p{L}0-9]+)(?:(?:['’‘](?:[\p{L}0-9]+)?)|(?:[-−/](?:\+[0-9xX]+|-[0-9xX]+|[\p{L}0-9]+)))*"
    )]
    Word,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedLexToken {
    pub kind: TokenKind,
    pub slice: String,
    pub parser_text: String,
    parser_word_pieces: Box<[TokenWordPiece]>,
    pub span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenWordPiece {
    pub text: String,
    pub span: TextSpan,
}

fn build_token_word_pieces(
    kind: TokenKind,
    slice: &str,
    parser_text: &str,
    span: TextSpan,
) -> Box<[TokenWordPiece]> {
    let mut pieces = Vec::new();
    match kind {
        TokenKind::Word | TokenKind::Number => {
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

pub type LexToken = OwnedLexToken;

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
    pub fn new(kind: TokenKind, slice: impl Into<String>, span: TextSpan) -> Self {
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

    pub fn word(slice: impl Into<String>, span: TextSpan) -> Self {
        Self::new(TokenKind::Word, slice, span)
    }

    pub fn comma(span: TextSpan) -> Self {
        Self::new(TokenKind::Comma, ",", span)
    }

    pub fn period(span: TextSpan) -> Self {
        Self::new(TokenKind::Period, ".", span)
    }

    pub fn colon(span: TextSpan) -> Self {
        Self::new(TokenKind::Colon, ":", span)
    }

    pub fn semicolon(span: TextSpan) -> Self {
        Self::new(TokenKind::Semicolon, ";", span)
    }

    pub fn quote(span: TextSpan) -> Self {
        Self::new(TokenKind::Quote, "\"", span)
    }

    #[allow(dead_code)]
    pub fn synthetic_word(slice: impl Into<String>) -> Self {
        Self::word(slice, TextSpan::synthetic())
    }

    #[allow(dead_code)]
    pub fn synthetic_comma() -> Self {
        Self::comma(TextSpan::synthetic())
    }

    pub fn as_word(&self) -> Option<&str> {
        match self.kind {
            TokenKind::Word | TokenKind::Number => Some(self.slice.as_str()),
            TokenKind::Tilde => Some("this"),
            _ => None,
        }
    }

    pub fn parser_text(&self) -> &str {
        self.parser_text.as_str()
    }

    pub fn parser_word_pieces(&self) -> &[TokenWordPiece] {
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

    pub fn replace_word(&mut self, slice: impl Into<String>) -> bool {
        match self.kind {
            TokenKind::Word | TokenKind::Number => {
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

    pub fn lowercase_word(&mut self) -> bool {
        match self.kind {
            TokenKind::Word | TokenKind::Number => {
                let lowered = self.slice.to_ascii_lowercase();
                self.replace_word(lowered)
            }
            TokenKind::Tilde => true,
            _ => false,
        }
    }

    pub fn is_word(&self, expected: &str) -> bool {
        matches!(
            self.kind,
            TokenKind::Word | TokenKind::Number | TokenKind::Tilde
        ) && self.parser_text == normalize_parser_fragment(expected)
    }

    pub fn is_any_word(&self, expected: &[&str]) -> bool {
        expected.iter().any(|word| self.is_word(word))
    }

    pub fn is_comma(&self) -> bool {
        self.kind == TokenKind::Comma
    }

    pub fn is_period(&self) -> bool {
        self.kind == TokenKind::Period
    }

    pub fn is_colon(&self) -> bool {
        self.kind == TokenKind::Colon
    }

    pub fn is_semicolon(&self) -> bool {
        self.kind == TokenKind::Semicolon
    }

    pub fn is_quote(&self) -> bool {
        self.kind == TokenKind::Quote
    }

    pub fn span(&self) -> TextSpan {
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

pub fn token_word_pieces_for_token(token: &OwnedLexToken) -> &[TokenWordPiece] {
    token.parser_word_pieces()
}

pub fn word_slice_find_phrase_start(words: &[&str], expected: &[&str]) -> Option<usize> {
    crate::word_primitives::find_phrase_start(words, expected)
}

pub fn word_slice_find_phrase_start_or_zero(words: &[&str], expected: &[&str]) -> Option<usize> {
    crate::word_primitives::find_phrase_start_or_zero(words, expected)
}

pub fn word_slice_find_any_phrase_start<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    crate::word_primitives::find_any_phrase_start(words, expected)
}

pub fn word_slice_find_any_phrase_span(
    words: &[&str],
    expected: &[&[&str]],
) -> Option<(usize, usize)> {
    word_slice_find_any_phrase_start(words, expected).map(|(phrase, idx)| (idx, phrase.len()))
}

pub fn word_slice_find_phrase_value<T: Clone>(
    words: &[&str],
    expected: &[(&[&str], T)],
) -> Option<(T, usize)> {
    crate::word_primitives::find_phrase_value(words, expected)
}

pub fn word_slice_find_any_phrase_start_or_zero<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    crate::word_primitives::find_any_phrase_start_or_zero(words, expected)
}

pub fn word_slice_find_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> Option<usize> {
    crate::word_primitives::find_window_by(words, window_len, predicate)
}

pub fn word_slice_contains_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> bool {
    crate::word_primitives::contains_window_by(words, window_len, predicate)
}

pub fn word_slice_contains_phrase(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::contains_phrase(words, expected)
}

pub fn word_slice_contains_phrase_or_empty(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::contains_phrase_or_empty(words, expected)
}

pub fn word_slice_contains_any_phrase(words: &[&str], expected: &[&[&str]]) -> bool {
    crate::word_primitives::contains_any_phrase(words, expected)
}

pub fn word_slice_contains_any_phrase_or_empty(words: &[&str], expected: &[&[&str]]) -> bool {
    crate::word_primitives::contains_any_phrase_or_empty(words, expected)
}

pub fn word_slice_eq(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::equals(words, expected)
}

pub fn word_slice_eq_any(words: &[&str], expected: &[&[&str]]) -> bool {
    crate::word_primitives::equals_any(words, expected)
}

pub fn word_slice_matching_phrase<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<&'p [&'p str]> {
    crate::word_primitives::matching_phrase(words, expected)
}

pub fn word_slice_matching_value<T: Clone>(words: &[&str], expected: &[(&[&str], T)]) -> Option<T> {
    crate::word_primitives::matching_value(words, expected)
}

pub fn word_slice_ends_with(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::ends_with(words, expected)
}

pub fn word_slice_ends_with_any(words: &[&str], expected: &[&[&str]]) -> bool {
    crate::word_primitives::ends_with_any(words, expected)
}

pub fn word_slice_starts_with(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::starts_with(words, expected)
}

pub fn word_slice_starts_with_any(words: &[&str], expected: &[&[&str]]) -> bool {
    crate::word_primitives::starts_with_any(words, expected)
}

pub fn word_slice_strip_prefix<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    crate::word_primitives::strip_prefix(words, expected)
}

pub fn word_slice_strip_suffix<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    crate::word_primitives::strip_suffix(words, expected)
}

pub fn word_slice_strip_any_prefix<'a, 'p>(
    words: &'a [&'a str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [&'a str])> {
    crate::word_primitives::strip_any_prefix(words, expected)
}

pub fn word_slice_strip_prefix_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&[&str], T)],
) -> Option<(T, &'w [&'a str])> {
    crate::word_primitives::strip_prefix_value(words, expected)
}

pub fn word_slice_strip_first_word<'w, 'a>(
    words: &'w [&'a str],
    expected: &str,
) -> Option<&'w [&'a str]> {
    crate::word_primitives::strip_first_word(words, expected)
}

pub fn word_slice_strip_first_word_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&str, T)],
) -> Option<(T, &'w [&'a str])> {
    crate::word_primitives::strip_first_word_value(words, expected)
}

pub fn word_slice_strip_any_suffix<'a, 'p>(
    words: &'a [&'a str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [&'a str])> {
    crate::word_primitives::strip_any_suffix(words, expected)
}

pub fn word_slice_strip_suffix_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&[&str], T)],
) -> Option<(T, &'w [&'a str])> {
    crate::word_primitives::strip_suffix_value(words, expected)
}

pub fn word_slice_contains_word(words: &[&str], expected: &str) -> bool {
    crate::word_primitives::contains_word(words, expected)
}

pub fn word_slice_find_word(words: &[&str], expected: &str) -> Option<usize> {
    crate::word_primitives::find_word(words, expected)
}

pub fn word_slice_find_any_word(words: &[&str], expected: &[&str]) -> Option<usize> {
    crate::word_primitives::find_any_word(words, expected)
}

pub fn word_slice_find_word_where(
    words: &[&str],
    predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    crate::word_primitives::find_word_where(words, predicate)
}

pub fn word_slice_rfind_word_where(
    words: &[&str],
    predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    crate::word_primitives::rfind_word_where(words, predicate)
}

pub fn word_slice_contains_any_word(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::contains_any_word(words, expected)
}

pub fn word_slice_contains_all_words(words: &[&str], expected: &[&str]) -> bool {
    crate::word_primitives::contains_all_words(words, expected)
}

pub fn find_token_word_sequence(tokens: &[OwnedLexToken], expected: &[&str]) -> Option<usize> {
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

pub fn find_token_word_sequence_span(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<(usize, usize)> {
    find_token_word_sequence(tokens, expected).map(|start| (start, start + expected.len()))
}

pub fn find_any_token_word_sequence_span<'p>(
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

pub fn find_token_word_sequence_value<T: Clone>(
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

pub fn contains_token_word_sequence(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    find_token_word_sequence(tokens, expected).is_some()
}

pub fn find_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    crate::slice_primitives::find_index(tokens, |token| token.is_word(expected))
}

pub fn find_token_any_word(tokens: &[OwnedLexToken], expected: &[&str]) -> Option<usize> {
    crate::slice_primitives::find_index(tokens, |token| token.is_any_word(expected))
}

pub fn rfind_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    crate::slice_primitives::rfind_index(tokens, |token| token.is_word(expected))
}

pub fn contains_token_word(tokens: &[OwnedLexToken], expected: &str) -> bool {
    find_token_word(tokens, expected).is_some()
}

pub fn contains_token_any_word(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    find_token_any_word(tokens, expected).is_some()
}

pub fn find_token_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> Option<usize> {
    crate::slice_primitives::find_index(tokens, |token| token.kind == expected)
}

pub fn contains_token_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    find_token_kind(tokens, expected).is_some()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenWordView<'a> {
    words: Vec<&'a str>,
    token_start_indices: Vec<usize>,
    token_end_indices: Vec<usize>,
}

impl<'a> TokenWordView<'a> {
    pub fn new(tokens: &'a [OwnedLexToken]) -> Self {
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
        }
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn get(&self, idx: usize) -> Option<&'a str> {
        self.words.get(idx).copied()
    }

    pub fn starts_with(&self, expected: &[&str]) -> bool {
        word_slice_starts_with(&self.words, expected)
    }

    pub fn starts_with_any(&self, expected: &[&[&str]]) -> bool {
        word_slice_starts_with_any(&self.words, expected)
    }

    pub fn slice_eq(&self, start: usize, expected: &[&str]) -> bool {
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

    pub fn find_phrase_start(&self, expected: &[&str]) -> Option<usize> {
        word_slice_find_phrase_start(&self.words, expected)
    }

    pub fn find_any_phrase_start<'p>(
        &self,
        expected: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], usize)> {
        word_slice_find_any_phrase_start(&self.words, expected)
    }

    pub fn find_any_phrase_span(&self, expected: &[&[&str]]) -> Option<(usize, usize)> {
        word_slice_find_any_phrase_span(&self.words, expected)
    }

    pub fn find_phrase_value<T: Clone>(&self, expected: &[(&[&str], T)]) -> Option<(T, usize)> {
        word_slice_find_phrase_value(&self.words, expected)
    }

    pub fn find_window_by(
        &self,
        window_len: usize,
        predicate: impl FnMut(&[&str]) -> bool,
    ) -> Option<usize> {
        word_slice_find_window_by(&self.words, window_len, predicate)
    }

    pub fn contains_window_by(
        &self,
        window_len: usize,
        predicate: impl FnMut(&[&str]) -> bool,
    ) -> bool {
        word_slice_contains_window_by(&self.words, window_len, predicate)
    }

    pub fn has_phrase(&self, expected: &[&str]) -> bool {
        word_slice_contains_phrase(&self.words, expected)
    }

    pub fn has_any_phrase(&self, expected: &[&[&str]]) -> bool {
        word_slice_contains_any_phrase(&self.words, expected)
    }

    pub fn find_word(&self, expected: &str) -> Option<usize> {
        word_slice_find_word(&self.words, expected)
    }

    pub fn find_any_word(&self, expected: &[&str]) -> Option<usize> {
        word_slice_find_any_word(&self.words, expected)
    }

    pub fn rfind_word(&self, expected: &str) -> Option<usize> {
        word_slice_rfind_word_where(&self.words, |word| word == expected)
    }

    pub fn contains_word(&self, expected: &str) -> bool {
        word_slice_contains_word(&self.words, expected)
    }

    pub fn contains_any_word(&self, expected: &[&str]) -> bool {
        word_slice_contains_any_word(&self.words, expected)
    }

    pub fn contains_all_words(&self, expected: &[&str]) -> bool {
        word_slice_contains_all_words(&self.words, expected)
    }

    pub fn matching_phrase<'p>(&self, expected: &'p [&'p [&'p str]]) -> Option<&'p [&'p str]> {
        word_slice_matching_phrase(&self.words, expected)
    }

    pub fn matching_value<T: Clone>(&self, expected: &[(&[&str], T)]) -> Option<T> {
        word_slice_matching_value(&self.words, expected)
    }

    pub fn strip_prefix_value<'w, T: Clone>(
        &'w self,
        expected: &[(&[&str], T)],
    ) -> Option<(T, &'w [&'a str])> {
        word_slice_strip_prefix_value(&self.words, expected)
    }

    pub fn strip_suffix_value<'w, T: Clone>(
        &'w self,
        expected: &[(&[&str], T)],
    ) -> Option<(T, &'w [&'a str])> {
        word_slice_strip_suffix_value(&self.words, expected)
    }

    pub fn strip_first_word_value<'w, T: Clone>(
        &'w self,
        expected: &[(&str, T)],
    ) -> Option<(T, &'w [&'a str])> {
        word_slice_strip_first_word_value(&self.words, expected)
    }

    pub fn first(&self) -> Option<&str> {
        self.get(0)
    }

    pub fn word_refs(&self) -> Vec<&'a str> {
        self.words.clone()
    }

    pub fn join(&self, separator: &str) -> String {
        self.words.join(separator)
    }

    pub fn owned_words(&self) -> Vec<String> {
        self.words.iter().map(|word| (*word).to_string()).collect()
    }

    pub fn to_word_refs(&self) -> Vec<&'a str> {
        self.word_refs()
    }

    pub fn token_index_for_word_index(&self, word_idx: usize) -> Option<usize> {
        self.token_start_indices.get(word_idx).copied()
    }

    pub fn token_start_indices(&self) -> &[usize] {
        &self.token_start_indices
    }

    pub fn token_index_after_words(&self, word_count: usize) -> Option<usize> {
        if word_count == 0 {
            return Some(0);
        }
        if word_count > self.len() {
            return None;
        }
        self.token_end_indices.get(word_count - 1).copied()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LexCursor<'a> {
    tokens: &'a [OwnedLexToken],
    pos: usize,
}

pub type LexStream<'a> = TokenSlice<'a, LexToken>;

impl<'a> LexCursor<'a> {
    pub fn new(tokens: &'a [OwnedLexToken]) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn peek(&self) -> Option<&'a OwnedLexToken> {
        self.tokens.get(self.pos)
    }

    pub fn peek_n(&self, offset: usize) -> Option<&'a OwnedLexToken> {
        self.tokens.get(self.pos + offset)
    }

    pub fn advance(&mut self) -> Option<&'a OwnedLexToken> {
        let token = self.peek()?;
        self.pos += 1;
        Some(token)
    }

    pub fn remaining(&self) -> &'a [OwnedLexToken] {
        self.tokens.get(self.pos..).unwrap_or_default()
    }

    pub fn position(&self) -> usize {
        self.pos
    }
}

pub fn token_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str> {
    tokens.iter().filter_map(OwnedLexToken::as_word).collect()
}

pub fn parser_token_word_refs<'a>(tokens: &'a [OwnedLexToken]) -> Vec<&'a str> {
    let mut words = Vec::new();
    for token in tokens {
        for piece in token.parser_word_pieces() {
            words.push(piece.text.as_str());
        }
    }
    words
}

pub fn parser_token_word_positions<'a>(tokens: &'a [OwnedLexToken]) -> Vec<(usize, &'a str)> {
    let mut positions = Vec::new();
    for (token_idx, token) in tokens.iter().enumerate() {
        for piece in token.parser_word_pieces() {
            positions.push((token_idx, piece.text.as_str()));
        }
    }
    positions
}

pub fn render_token_slice(tokens: &[OwnedLexToken]) -> String {
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
pub fn trim_lexed_commas(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
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

pub fn split_lexed_sentences(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut sentences = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0u32;
    let mut inside_quotes = false;

    for (idx, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LParen => paren_depth = paren_depth.saturating_add(1),
            TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::Quote => inside_quotes = !inside_quotes,
            TokenKind::Period if paren_depth == 0 && !inside_quotes => {
                if start < idx {
                    sentences.push(&tokens[start..idx]);
                }
                start = idx + 1;
            }
            _ => {}
        }
    }

    if start < tokens.len() {
        sentences.push(&tokens[start..]);
    }

    sentences
}

pub fn lex_line(line: &str, line_index: usize) -> Result<Vec<OwnedLexToken>, CardTextError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_line_normalizes_tilde_and_curly_punctuation() {
        let tokens = lex_line("~ deals ½ damage — it’s fine.", 0).expect("line should lex");

        assert_eq!(tokens[0].parser_text(), "this");
        assert_eq!(tokens[2].parser_text(), "1/2");
        assert_eq!(tokens[4].kind, TokenKind::EmDash);
        assert_eq!(tokens[5].parser_text(), "it's");
    }

    #[test]
    fn split_lexed_sentences_ignores_periods_inside_quotes() {
        let tokens =
            lex_line("Gain \"Draw a card.\" Then scry 1. Untap this creature.", 0).expect("lex");
        let sentences = split_lexed_sentences(&tokens);

        assert_eq!(sentences.len(), 2);
        assert_eq!(
            render_token_slice(sentences[0]),
            "Gain \"Draw a card.\"Then scry 1"
        );
        assert_eq!(render_token_slice(sentences[1]), "Untap this creature");
    }

    #[test]
    fn token_word_view_tracks_split_mana_words() {
        let tokens = lex_line("{W/U} and target non-Human creature", 0).expect("lex");
        let view = TokenWordView::new(&tokens);

        assert_eq!(
            view.word_refs(),
            vec!["w/u", "and", "target", "non", "human", "creature"]
        );
        assert_eq!(view.find_phrase_start(&["target", "non", "human"]), Some(2));
    }
}
