use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::runtime_backend::lexer::{LexStream, OwnedLexToken, TokenKind};

use super::super::primitives;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmblemPayloadShape<'a> {
    pub(crate) explicit_you: bool,
    pub(crate) ability_groups: Vec<&'a [OwnedLexToken]>,
    pub(crate) requires_whole_sentence_dispatch: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EmblemGroupSpan {
    first: usize,
    end: usize,
}

fn emblem_with_prefix<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    alt((
        primitives::phrase(&["you", "get", "an", "emblem", "with"]).value(true),
        primitives::phrase(&["get", "an", "emblem", "with"]).value(false),
        primitives::phrase(&["gets", "an", "emblem", "with"]).value(false),
        primitives::phrase(&["an", "emblem", "with"]).value(false),
        primitives::phrase(&["emblem", "with"]).value(false),
    ))
    .parse_next(input)
}

fn quoted_emblem_payload<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<Vec<EmblemGroupSpan>> {
    primitives::quote().parse_next(input)?;
    let mut groups = Vec::new();
    let mut inside_quote = true;
    let mut first = initial_len.saturating_sub(input.len());
    while !input.is_empty() {
        let token_idx = initial_len.saturating_sub(input.len());
        let token: &OwnedLexToken = any.parse_next(input)?;
        if token.kind != TokenKind::Quote {
            continue;
        }
        if inside_quote {
            if token_idx > first {
                groups.push(EmblemGroupSpan {
                    first,
                    end: token_idx,
                });
            }
            inside_quote = false;
        } else {
            first = initial_len.saturating_sub(input.len());
            inside_quote = true;
        }
    }
    if inside_quote && initial_len > first {
        groups.push(EmblemGroupSpan {
            first,
            end: initial_len,
        });
    }
    eof.parse_next(input)?;
    if groups.is_empty() {
        return Err(primitives::backtrack_err(
            "emblem ability payload",
            "one or more quoted ability groups",
        ));
    }
    Ok(groups)
}

fn unquoted_emblem_payload<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<Vec<EmblemGroupSpan>> {
    let first = initial_len.saturating_sub(input.len());
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
        .void()
        .parse_next(input)?;
    let end = initial_len.saturating_sub(input.len());
    primitives::sentence_end().parse_next(input)?;
    eof.parse_next(input)?;
    Ok(vec![EmblemGroupSpan { first, end }])
}

fn emblem_payload<'a>(input: &mut LexStream<'a>) -> WResult<(bool, Vec<EmblemGroupSpan>)> {
    let initial_len = input.len();
    let explicit_you = emblem_with_prefix.parse_next(input)?;
    let spans = alt((
        |input: &mut LexStream<'a>| quoted_emblem_payload(input, initial_len),
        |input: &mut LexStream<'a>| unquoted_emblem_payload(input, initial_len),
    ))
    .parse_next(input)?;
    Ok((explicit_you, spans))
}

pub(crate) fn parse_emblem_payload_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EmblemPayloadShape<'_>> {
    let (explicit_you, spans) =
        primitives::parse_all(tokens, emblem_payload, "emblem typed ability payload").ok()?;
    let ability_groups = spans
        .into_iter()
        .map(|span| tokens.get(span.first..span.end))
        .collect::<Option<Vec<_>>>()?;
    let requires_whole_sentence_dispatch = ability_groups.len() > 1
        || ability_groups
            .iter()
            .any(|group| group.iter().any(|token| token.kind == TokenKind::Colon));
    (!ability_groups.is_empty()).then_some(EmblemPayloadShape {
        explicit_you,
        ability_groups,
        requires_whole_sentence_dispatch,
    })
}

#[cfg(test)]
#[path = "emblem_shapes/tests.rs"]
mod tests;
