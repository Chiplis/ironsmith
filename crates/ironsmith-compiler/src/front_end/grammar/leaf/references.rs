#[cfg(test)]
use winnow::combinator::alt;
#[cfg(test)]
use winnow::error::{ModalResult as WResult, StrContext, StrContextValue};
#[cfg(test)]
use winnow::prelude::*;

#[cfg(test)]
use crate::cards::builders::CardTextError;

#[cfg(test)]
use super::common::{finish_text_parse, phrase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafPlayerReference {
    You,
    Opponent,
    EachOpponent,
    AnyPlayer,
    #[cfg(test)]
    EachPlayer,
    TargetPlayer,
    TargetOpponent,
    #[cfg(test)]
    ItsController,
    ThatPlayer,
    DefendingPlayer,
    AttackingPlayer,
}

#[cfg(test)]
pub(crate) fn parse_player_reference(input: &mut &str) -> WResult<LeafPlayerReference> {
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

#[cfg(test)]
pub(crate) fn parse_player_reference_complete(
    raw: &str,
) -> Result<LeafPlayerReference, CardTextError> {
    finish_text_parse(raw, parse_player_reference, "leaf-player-reference")
}

#[cfg(test)]
fn parse_target_player_reference(input: &mut &str) -> WResult<LeafPlayerReference> {
    alt((
        phrase("target opponent").value(LeafPlayerReference::TargetOpponent),
        phrase("target player").value(LeafPlayerReference::TargetPlayer),
    ))
    .parse_next(input)
}

#[cfg(test)]
fn parse_opponent_reference(input: &mut &str) -> WResult<LeafPlayerReference> {
    alt((
        phrase("each opponent").value(LeafPlayerReference::EachOpponent),
        phrase("an opponent").value(LeafPlayerReference::Opponent),
        phrase("opponent").value(LeafPlayerReference::Opponent),
    ))
    .parse_next(input)
}

#[cfg(test)]
fn parse_any_player_reference(input: &mut &str) -> WResult<LeafPlayerReference> {
    alt((
        phrase("each player").value(LeafPlayerReference::EachPlayer),
        phrase("a player").value(LeafPlayerReference::AnyPlayer),
        phrase("player").value(LeafPlayerReference::AnyPlayer),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_string_reference_language_remains_context_free() {
        for raw in ["that player", "defending player", "attacking player"] {
            assert!(parse_player_reference_complete(raw).is_err(), "{raw}");
        }
    }
}
