//! "If a creature would enter the battlefield under an opponent's control
//! this turn, it enters under your control instead." (Gather Specimens) and
//! "each token that would be created under an opponent's control this turn is
//! created under your control instead" (Crafty Cutpurse): a resolving
//! instruction creating a control-changing entry replacement for the rest of
//! the turn (CR 614.1c, 614.12, 616.1b). Every matching entry this turn is
//! replaced, not only the next one.

use crate::cards::builders::{EffectAst, ZoneReplacementDurationAst};
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};
use crate::target::{ObjectFilter, PlayerFilter};
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

/// "under an opponent's control"
fn under_an_opponents_control<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["under", "an"]).parse_next(input)?;
    alt((
        primitives::kw("opponent's"),
        primitives::kw("opponents"),
        primitives::kw("opponent’s"),
    ))
    .parse_next(input)?;
    primitives::kw("control").parse_next(input)?;
    Ok(())
}

/// "If a creature would enter [the battlefield] under an opponent's control
/// this turn, it enters under your control instead."
fn creature_entry<'a>(input: &mut LexStream<'a>) -> WResult<ObjectFilter> {
    primitives::phrase(&["if", "a", "creature", "would", "enter"]).parse_next(input)?;
    opt(primitives::phrase(&["the", "battlefield"])).parse_next(input)?;
    under_an_opponents_control.parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "enters"]).parse_next(input)?;
    opt(primitives::phrase(&["the", "battlefield"])).parse_next(input)?;
    primitives::phrase(&["under", "your", "control", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ObjectFilter::creature())
}

/// "each token that would be created under an opponent's control this turn
/// is created under your control instead"
fn token_creation<'a>(input: &mut LexStream<'a>) -> WResult<ObjectFilter> {
    primitives::phrase(&["each", "token", "that", "would", "be", "created"])
        .parse_next(input)?;
    under_an_opponents_control.parse_next(input)?;
    primitives::phrase(&["this", "turn", "is", "created", "under", "your", "control", "instead"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ObjectFilter::default().token())
}

pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let mut filter =
        primitives::probe_all(tokens, alt((creature_entry, token_creation)), "turn control entry")?;
    filter.controller = Some(PlayerFilter::Opponent);
    Some(EffectAst::subject_verb_register_enter_under_control_replacement(
        filter,
        ZoneReplacementDurationAst::UntilEndOfTurn,
    ))
}
