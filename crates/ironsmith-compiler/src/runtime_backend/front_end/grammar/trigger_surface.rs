use super::super::ast::TriggerIntroSurfaceAst;
use super::super::lexer::{OwnedLexToken, TokenKind};
use super::primitives;
use winnow::Parser;
use winnow::combinator::alt;

mod frequency;
pub(crate) use frequency::{
    BecomesTappedDuringYourTurn, TriggerFrequencySurface,
    parse_becomes_tapped_during_your_turn_tokens, parse_do_this_only_each_turn_limit_tokens,
    parse_trigger_frequency_condition_tokens, parse_trigger_frequency_tokens,
};

pub(crate) fn parse_trigger_intro_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TriggerIntroSurfaceAst> {
    primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("when").value(TriggerIntroSurfaceAst::When),
            primitives::kw("whenever").value(TriggerIntroSurfaceAst::Whenever),
            primitives::kw("at").value(TriggerIntroSurfaceAst::At),
        )),
    )
    .map(|(intro, _)| intro)
}
pub(crate) fn parse_trigger_intro_surface_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TriggerIntroSurfaceAst> {
    if let Some(intro) = parse_trigger_intro_prefix_tokens(tokens) {
        return Some(intro);
    }
    let (_, (_, intro), _) = primitives::find_prefix(tokens, || {
        (
            alt((
                primitives::colon().void(),
                primitives::token_kind(TokenKind::Dash).void(),
                primitives::token_kind(TokenKind::EmDash).void(),
            )),
            alt((
                primitives::kw("when").value(TriggerIntroSurfaceAst::When),
                primitives::kw("whenever").value(TriggerIntroSurfaceAst::Whenever),
            )),
        )
    })?;
    Some(intro)
}
#[cfg(test)]
#[path = "trigger_surface/tests.rs"]
mod tests;
