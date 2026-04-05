#![allow(dead_code)]

use logos::Logos;
use winnow::stream::{Location, TokenSlice};

use crate::cards::builders::{CardTextError, TextSpan};

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
            TokenKind::Word | TokenKind::Number => Some(self.slice.as_str()),
            TokenKind::Tilde => Some("this"),
            _ => None,
        }
    }

    pub(crate) fn parser_text(&self) -> &str {
        self.parser_text.as_str()
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

    pub(crate) fn lowercase_word(&mut self) -> bool {
        match self.kind {
            TokenKind::Word | TokenKind::Number => {
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
            TokenKind::Word | TokenKind::Number | TokenKind::Tilde
        ) && self.parser_text == normalize_parser_fragment(expected)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenWordView {
    words: Vec<String>,
    token_start_indices: Vec<usize>,
    token_end_indices: Vec<usize>,
}

impl TokenWordView {
    pub(crate) fn new(tokens: &[OwnedLexToken]) -> Self {
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
                words.push(piece.text.clone());
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

    pub(crate) fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.words.len()
    }

    pub(crate) fn get(&self, idx: usize) -> Option<&str> {
        self.words.get(idx).map(String::as_str)
    }

    pub(crate) fn starts_with(&self, expected: &[&str]) -> bool {
        self.slice_eq(0, expected)
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

    pub(crate) fn find_phrase_start(&self, expected: &[&str]) -> Option<usize> {
        if expected.is_empty() || self.words.len() < expected.len() {
            return None;
        }
        let mut idx = 0usize;
        let last_start = self.words.len() - expected.len();
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

    pub(crate) fn find_word(&self, expected: &str) -> Option<usize> {
        let mut idx = 0usize;
        while idx < self.words.len() {
            if self.words[idx] == expected {
                return Some(idx);
            }
            idx += 1;
        }

        None
    }

    pub(crate) fn first(&self) -> Option<&str> {
        self.get(0)
    }

    pub(crate) fn word_refs(&self) -> Vec<&str> {
        self.words.iter().map(String::as_str).collect()
    }

    pub(crate) fn join(&self, separator: &str) -> String {
        self.words.join(separator)
    }

    pub(crate) fn owned_words(&self) -> Vec<String> {
        self.words.clone()
    }

    pub(crate) fn to_word_refs(&self) -> Vec<&str> {
        self.word_refs()
    }

    pub(crate) fn token_index_for_word_index(&self, word_idx: usize) -> Option<usize> {
        self.token_start_indices.get(word_idx).copied()
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
