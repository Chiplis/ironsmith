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
    #[token("]")]
    RBracket,
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
    #[regex(r#""|'|‘|’|“|”"#)]
    Quote,
    #[regex(r"\{[^}\r\n]+\}")]
    ManaGroup,
    #[regex(r"(?:\+[\p{L}0-9][\p{L}0-9/'’+\-−]*|[\p{L}0-9][\p{L}0-9/'’+\-−]*)")]
    Word,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnedLexToken {
    pub(crate) kind: TokenKind,
    pub(crate) slice: String,
    pub(crate) span: TextSpan,
}

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
    pub(crate) fn word(slice: impl Into<String>, span: TextSpan) -> Self {
        Self {
            kind: TokenKind::Word,
            slice: slice.into(),
            span,
        }
    }

    pub(crate) fn comma(span: TextSpan) -> Self {
        Self {
            kind: TokenKind::Comma,
            slice: ",".to_string(),
            span,
        }
    }

    pub(crate) fn period(span: TextSpan) -> Self {
        Self {
            kind: TokenKind::Period,
            slice: ".".to_string(),
            span,
        }
    }

    pub(crate) fn colon(span: TextSpan) -> Self {
        Self {
            kind: TokenKind::Colon,
            slice: ":".to_string(),
            span,
        }
    }

    pub(crate) fn semicolon(span: TextSpan) -> Self {
        Self {
            kind: TokenKind::Semicolon,
            slice: ";".to_string(),
            span,
        }
    }

    pub(crate) fn quote(span: TextSpan) -> Self {
        Self {
            kind: TokenKind::Quote,
            slice: "\"".to_string(),
            span,
        }
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
            TokenKind::Word => Some(self.slice.as_str()),
            TokenKind::Tilde => Some("this"),
            _ => None,
        }
    }

    pub(crate) fn word_mut(&mut self) -> Option<&mut String> {
        match self.kind {
            TokenKind::Word => Some(&mut self.slice),
            _ => None,
        }
    }

    pub(crate) fn is_word(&self, expected: &str) -> bool {
        self.as_word()
            .is_some_and(|word| word.eq_ignore_ascii_case(expected))
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

pub(crate) type LexStream<'a> = TokenSlice<'a, OwnedLexToken>;

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

pub(crate) fn lexed_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    tokens.iter().filter_map(OwnedLexToken::as_word).collect()
}

pub(crate) fn render_lexed_tokens(tokens: &[OwnedLexToken]) -> String {
    fn needs_space(prev: TokenKind, current: TokenKind) -> bool {
        if matches!(
            current,
            TokenKind::Comma
                | TokenKind::Period
                | TokenKind::Colon
                | TokenKind::Semicolon
                | TokenKind::Question
                | TokenKind::Bang
                | TokenKind::RBracket
        ) {
            return false;
        }

        !matches!(
            prev,
            TokenKind::LBracket | TokenKind::Quote | TokenKind::Plus | TokenKind::Dash
        )
    }

    let mut rendered = String::new();
    let mut previous_kind = None;

    for token in tokens {
        if let Some(previous_kind) = previous_kind
            && needs_space(previous_kind, token.kind)
        {
            rendered.push(' ');
        }
        rendered.push_str(&token.slice);
        previous_kind = Some(token.kind);
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
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut quote_depth = 0u32;

    for (idx, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Quote => {
                if !matches!(token.slice.as_str(), "\"" | "“" | "”") {
                    continue;
                }
                let closing_quote = quote_depth != 0;
                quote_depth = if quote_depth == 0 { 1 } else { 0 };
                if closing_quote
                    && idx > start
                    && tokens
                        .get(idx.saturating_sub(1))
                        .is_some_and(|previous| previous.kind == TokenKind::Period)
                {
                    segments.push(&tokens[start..=idx]);
                    start = idx + 1;
                }
            }
            TokenKind::Period if quote_depth == 0 => {
                if start < idx {
                    segments.push(&tokens[start..idx]);
                }
                start = idx + 1;
            }
            _ => {}
        }
    }

    if start < tokens.len() {
        segments.push(&tokens[start..]);
    }

    segments
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

        tokens.push(OwnedLexToken {
            kind,
            slice: slice.to_string(),
            span,
        });
    }

    Ok(tokens)
}
