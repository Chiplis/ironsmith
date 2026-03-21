#![allow(dead_code)]

use logos::Logos;

use crate::cards::builders::{CardTextError, TextSpan};

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"[ \t\r\n\f]+")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LexToken<'a> {
    pub(crate) kind: TokenKind,
    pub(crate) slice: &'a str,
    pub(crate) span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnedLexToken {
    pub(crate) kind: TokenKind,
    pub(crate) slice: String,
    pub(crate) span: TextSpan,
}

impl OwnedLexToken {
    pub(crate) fn as_borrowed(&self) -> LexToken<'_> {
        LexToken {
            kind: self.kind,
            slice: self.slice.as_str(),
            span: self.span,
        }
    }
}

pub(crate) fn lex_line(line: &str, line_index: usize) -> Result<Vec<OwnedLexToken>, CardTextError> {
    let mut lexer = TokenKind::lexer(line);
    let mut tokens = Vec::new();

    while let Some(kind_result) = lexer.next() {
        let span = lexer.span();
        let start = span.start;
        let end = span.end;
        let slice = &line[start..end];
        let span = TextSpan {
            line: line_index,
            start,
            end,
        };

        let Ok(kind) = kind_result else {
            return Err(CardTextError::ParseError(format!(
                "rewrite lexer could not classify token '{}' at {}..{}",
                slice, start, end
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
