#[cfg(any(test, feature = "test-support"))]
use winnow::combinator::alt;
#[cfg(any(test, feature = "test-support"))]
use winnow::error::{ModalResult as WResult, StrContext, StrContextValue};
#[cfg(any(test, feature = "test-support"))]
use winnow::prelude::*;

#[cfg(any(test, feature = "test-support"))]
use crate::cards::builders::CardTextError;

#[cfg(any(test, feature = "test-support"))]
use super::common::{finish_text_parse, phrase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafPlayerReference {
    You,
    Opponent,
    EachOpponent,
    AnyPlayer,
    #[cfg(any(test, feature = "test-support"))]
    EachPlayer,
    TargetPlayer,
    TargetOpponent,
    #[cfg(any(test, feature = "test-support"))]
    ItsController,
    ThatPlayer,
    DefendingPlayer,
    AttackingPlayer,
}

#[cfg(any(test, feature = "test-support"))]
pub fn parse_player_reference(input: &mut &str) -> WResult<LeafPlayerReference> {
    alt((
        parse_target_player_reference,
        parse_opponent_reference,
        parse_any_player_reference,
        phrase("its controller").value(LeafPlayerReference::ItsController),
        phrase("you").value(LeafPlayerReference::You),
    ))
    .context(StrContext::Label("player reference"))
    .context(StrContext::Expected(StrContextValue::Description(
        "player or controller reference",
    )))
    .parse_next(input)
}

#[cfg(any(test, feature = "test-support"))]
pub fn parse_player_reference_complete(raw: &str) -> Result<LeafPlayerReference, CardTextError> {
    finish_text_parse(raw, parse_player_reference, "leaf-player-reference")
}

#[cfg(any(test, feature = "test-support"))]
fn parse_target_player_reference(input: &mut &str) -> WResult<LeafPlayerReference> {
    alt((
        phrase("target opponent").value(LeafPlayerReference::TargetOpponent),
        phrase("target player").value(LeafPlayerReference::TargetPlayer),
    ))
    .parse_next(input)
}

#[cfg(any(test, feature = "test-support"))]
fn parse_opponent_reference(input: &mut &str) -> WResult<LeafPlayerReference> {
    alt((
        phrase("each opponent").value(LeafPlayerReference::EachOpponent),
        phrase("an opponent").value(LeafPlayerReference::Opponent),
        phrase("opponent").value(LeafPlayerReference::Opponent),
    ))
    .parse_next(input)
}

#[cfg(any(test, feature = "test-support"))]
fn parse_any_player_reference(input: &mut &str) -> WResult<LeafPlayerReference> {
    alt((
        phrase("each player").value(LeafPlayerReference::EachPlayer),
        phrase("a player").value(LeafPlayerReference::AnyPlayer),
        phrase("player").value(LeafPlayerReference::AnyPlayer),
    ))
    .parse_next(input)
}

#[cfg(any(test, feature = "test-support"))]
mod tests {
    use super::*;

    #[test]
    fn general_string_reference_language_remains_context_free() {
        for raw in ["that player", "defending player", "attacking player"] {
            assert!(parse_player_reference_complete(raw).is_err(), "{raw}");
        }
    }
}
