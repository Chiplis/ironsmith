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
    pub(crate) span: TextSpan,
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
        Self {
            kind,
            slice,
            parser_text,
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

    pub(crate) fn replace_word(&mut self, slice: impl Into<String>) -> bool {
        match self.kind {
            TokenKind::Word | TokenKind::Number => {
                let slice = slice.into();
                self.parser_text = parser_text_for_token(self.kind, slice.as_str());
                self.slice = slice;
                true
            }
            TokenKind::Tilde => {
                self.parser_text = "this".to_string();
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
