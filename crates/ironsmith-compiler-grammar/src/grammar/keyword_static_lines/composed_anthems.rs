use winnow::combinator::peek;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposedAnthemSegment<'a> {
    pub body_tokens: &'a [OwnedLexToken],
    pub omitted_subject: bool,
}

pub fn parse_composed_anthem_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ComposedAnthemSegment<'_>> {
    let tokens = trim_lexed_commas(tokens);
    if tokens.is_empty() {
        return None;
    }

    let body_tokens = primitives::parse_prefix(tokens, parse_and_action_prefix_lexed)
        .map(|(_, rest)| trim_lexed_commas(rest))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(tokens);
    let omitted_subject =
        primitives::parse_prefix(body_tokens, peek(parse_anthem_action_lexed)).is_some();
    Some(ComposedAnthemSegment {
        body_tokens,
        omitted_subject,
    })
}

pub fn parse_where_x_value_prefix_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(tokens, primitives::phrase(&["where", "x", "is"]))
        .map(|(_, rest)| rest)
}

fn parse_and_action_prefix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("and").parse_next(input)?;
    peek(parse_anthem_action_lexed).parse_next(input)
}

fn parse_anthem_action_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    winnow::combinator::alt((
        primitives::kw("get"),
        primitives::kw("gets"),
        primitives::kw("have"),
        primitives::kw("has"),
    ))
    .void()
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn classifies_composed_anthem_segments() {
        let tokens = lex_line("and gets +1/+1", 0).unwrap();
        let parsed = parse_composed_anthem_segment_tokens(&tokens).unwrap();
        assert!(parsed.omitted_subject);
        assert_eq!(render_token_slice(parsed.body_tokens), "gets +1/+1");

        let tokens = lex_line("and flying", 0).unwrap();
        let parsed = parse_composed_anthem_segment_tokens(&tokens).unwrap();
        assert!(!parsed.omitted_subject);
        assert_eq!(render_token_slice(parsed.body_tokens), "and flying");
    }
}
