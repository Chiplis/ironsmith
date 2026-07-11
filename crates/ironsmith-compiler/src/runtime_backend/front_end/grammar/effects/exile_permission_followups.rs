use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::take_till;

use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{LexStream, LexedClause, OwnedLexToken, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExilePermissionFollowupKind {
    ReflexiveExileNonland,
    DelayedPlayCard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExilePermissionFollowupShape<'a> {
    pub(crate) kind: ExilePermissionFollowupKind,
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

fn followup_kind(input: &mut LexStream<'_>) -> WResult<ExilePermissionFollowupKind> {
    primitives::phrase(&["when", "you"]).parse_next(input)?;
    alt((
        primitives::phrase(&["exile", "a", "nonland", "card", "this", "way"])
            .value(ExilePermissionFollowupKind::ReflexiveExileNonland),
        primitives::phrase(&["play", "a", "card", "this", "way"])
            .value(ExilePermissionFollowupKind::DelayedPlayCard),
    ))
    .parse_next(input)
}

fn exile_permission_followup<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExilePermissionFollowupShape<'a>> {
    let kind = followup_kind.parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let effect_tokens = take_till(1.., |token: &OwnedLexToken| token.kind == TokenKind::Period)
        .parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;
    let effect_tokens = LexedClause::new(effect_tokens).trimmed().tokens();
    if effect_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "exile/play followup",
            "effect clause",
        ));
    }
    Ok(ExilePermissionFollowupShape {
        kind,
        effect_tokens,
    })
}

pub(crate) fn parse_exile_permission_followup_shape(
    tokens: &[OwnedLexToken],
) -> Option<ExilePermissionFollowupShape<'_>> {
    primitives::parse_all(
        LexedClause::new(tokens).trimmed().tokens(),
        exile_permission_followup,
        "exile/play permission followup",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{lex_line, render_token_slice};

    #[test]
    fn parses_reflexive_exile_and_delayed_play_followups() {
        let exile = lex_line(
            "When you exile a nonland card this way, this creature deals damage equal to its mana value to any target.",
            0,
        )
        .expect("lex");
        let exile_shape = parse_exile_permission_followup_shape(&exile).expect("exile followup");
        assert_eq!(
            exile_shape.kind,
            ExilePermissionFollowupKind::ReflexiveExileNonland
        );
        assert_eq!(
            render_token_slice(exile_shape.effect_tokens),
            "this creature deals damage equal to its mana value to any target"
        );

        let play = lex_line(
            "When you play a card this way, this enchantment deals 2 damage to each player.",
            0,
        )
        .expect("lex");
        let play_shape = parse_exile_permission_followup_shape(&play).expect("play followup");
        assert_eq!(
            play_shape.kind,
            ExilePermissionFollowupKind::DelayedPlayCard
        );
        assert_eq!(
            render_token_slice(play_shape.effect_tokens),
            "this enchantment deals 2 damage to each player"
        );
    }
}
