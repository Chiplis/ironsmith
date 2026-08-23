use winnow::combinator::opt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{rest, take_till};

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelLineSpec<'a> {
    pub body_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReconfigureLineSpec<'a> {
    pub cost_tokens: &'a [OwnedLexToken],
}

pub fn parse_channel_line_spec_tokens(tokens: &[OwnedLexToken]) -> Option<ChannelLineSpec<'_>> {
    primitives::parse_prefix(tokens, parse_channel_line_spec_lexed).map(|(spec, _)| spec)
}

pub fn parse_reconfigure_line_spec_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ReconfigureLineSpec<'_>> {
    primitives::parse_prefix(tokens, parse_reconfigure_line_spec_lexed).map(|(spec, _)| spec)
}

fn parse_channel_line_spec_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ChannelLineSpec<'a>> {
    primitives::kw("channel").parse_next(input)?;
    let body_tokens = rest.parse_next(input)?;
    Ok(ChannelLineSpec { body_tokens })
}

fn parse_reconfigure_line_spec_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ReconfigureLineSpec<'a>> {
    primitives::kw("reconfigure").parse_next(input)?;
    opt(primitives::token_kind(TokenKind::EmDash)).parse_next(input)?;
    let cost_tokens = take_till(0.., is_reconfigure_suffix_boundary).parse_next(input)?;
    Ok(ReconfigureLineSpec {
        cost_tokens: trim_lexed_commas(cost_tokens),
    })
}

fn is_reconfigure_suffix_boundary(token: &OwnedLexToken) -> bool {
    matches!(token.kind, TokenKind::LParen | TokenKind::Period)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn parses_channel_body_and_reconfigure_cost() {
        let channel = lex_line("Channel {2}{U}", 0).unwrap();
        let spec = parse_channel_line_spec_tokens(&channel).unwrap();
        assert_eq!(render_token_slice(spec.body_tokens), "{2}{U}");

        let reconfigure = lex_line(
            "Reconfigure {2}{U} ({2}{U}: Attach to target creature you control.)",
            0,
        )
        .unwrap();
        let spec = parse_reconfigure_line_spec_tokens(&reconfigure).unwrap();
        assert_eq!(render_token_slice(spec.cost_tokens), "{2}{U}");
    }
}
