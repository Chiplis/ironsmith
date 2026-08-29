use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::any;

use crate::lexer::{LexStream, OwnedLexToken, TokenKind};

use super::super::primitives;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmblemPayloadShape<'a> {
    pub explicit_you: bool,
    pub ability_groups: Vec<&'a [OwnedLexToken]>,
    pub requires_whole_sentence_dispatch: bool,
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

fn quoted_emblem_group_span<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<EmblemGroupSpan> {
    primitives::quote().parse_next(input)?;
    let first = initial_len.saturating_sub(input.len());
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::quote()))
        .void()
        .parse_next(input)?;
    let end = initial_len.saturating_sub(input.len());
    primitives::quote().parse_next(input)?;
    Ok(EmblemGroupSpan { first, end })
}

fn quoted_emblem_payload<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<Vec<EmblemGroupSpan>> {
    let mut groups = vec![quoted_emblem_group_span(input, initial_len)?];
    loop {
        let checkpoint = input.checkpoint();
        if primitives::kw("and").parse_next(input).is_err() {
            input.reset(&checkpoint);
            break;
        }
        match quoted_emblem_group_span(input, initial_len) {
            Ok(group) => groups.push(group),
            Err(_) => {
                input.reset(&checkpoint);
                break;
            }
        }
    }
    // Statement grouping appends one synthetic sentence terminator after a
    // sentence whose real terminator lives inside the closing quote. Accept
    // only that optional terminator here; an unquoted continuation (for
    // example Kiora's `Then create ...`) must remain outside the emblem.
    // Subject/verb dispatch hands `parse_get` the action tail without the
    // sentence terminator (and, for quoted text, without the synthetic outer
    // period).  The whole-sentence entry point still supplies that period.
    // Both are complete emblem payloads; only an unquoted continuation must
    // be rejected by the outer shape.
    alt((primitives::sentence_end().void(), eof.void())).parse_next(input)?;
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
    let spans = if input
        .peek_token()
        .is_some_and(|token| token.kind == TokenKind::Quote)
    {
        quoted_emblem_payload(input, initial_len)?
    } else {
        unquoted_emblem_payload(input, initial_len)?
    };
    Ok((explicit_you, spans))
}

pub fn parse_emblem_payload_tokens(tokens: &[OwnedLexToken]) -> Option<EmblemPayloadShape<'_>> {
    let (explicit_you, spans) =
        primitives::parse_all(tokens, emblem_payload, "emblem typed ability payload").ok()?;
    let ability_groups = spans
        .into_iter()
        .map(|span| tokens.get(span.first..span.end))
        .collect::<Option<Vec<_>>>()?;
    // Generic sentence dispatch trims terminal punctuation, including the
    // closing quote, before subject/verb routing. Every complete emblem shape
    // therefore needs the quote-preserving whole-sentence path, not just
    // triggered or activated emblem abilities.
    let requires_whole_sentence_dispatch = true;
    (!ability_groups.is_empty()).then_some(EmblemPayloadShape {
        explicit_you,
        ability_groups,
        requires_whole_sentence_dispatch,
    })
}

#[cfg(test)]
#[path = "emblem_shapes/tests.rs"]
mod tests;
