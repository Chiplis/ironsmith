use winnow::combinator::{alt, eof};
use winnow::prelude::*;

use super::super::lexer::OwnedLexToken;
use super::primitives;

#[path = "statement_player_counters.rs"]
mod player_counters;
pub(crate) use player_counters::{
    PlayerCounterKind, PlayerCounterSubject, PlayerGetsCountersShape,
    parse_player_gets_counters_surface_tokens, parse_player_gets_counters_tokens,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DieRollAdjustmentShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnyPlayerNoOneDoesShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EachPlayerChooseBounceDrawShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatementForceShape {
    DivvySelection,
    ExilePlayCost,
    ConditionalInstead,
    GroupTurnDuration,
    PlayerGetsCounters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NextDamagePreventionShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DayNightEntersShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RevealFromHandShape;

fn surface_has_sequence(tokens: &[OwnedLexToken], sequence: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(sequence)).is_some()
}

fn surface_starts_with(tokens: &[OwnedLexToken], sequence: &'static [&'static str]) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(sequence)).is_some()
}

pub(crate) fn parse_die_roll_adjustment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DieRollAdjustmentShape> {
    (surface_starts_with(tokens, &["after", "you", "roll", "a", "die"])
        && surface_has_sequence(tokens, &["you", "may", "pay"])
        && surface_has_sequence(tokens, &["if", "you", "do"])
        && surface_has_sequence(
            tokens,
            &["increase", "or", "decrease", "the", "result", "by"],
        )
        && surface_has_sequence(tokens, &["do", "this", "only", "once", "each", "turn"]))
    .then_some(DieRollAdjustmentShape)
}

pub(crate) fn parse_any_player_no_one_does_sentences(
    sentences: &[Vec<OwnedLexToken>],
) -> Option<AnyPlayerNoOneDoesShape> {
    let [may_sentence, no_one_sentence, _followup] = sentences else {
        return None;
    };
    (surface_starts_with(may_sentence, &["any", "player", "may"])
        && surface_starts_with(no_one_sentence, &["if", "no", "one", "does"]))
    .then_some(AnyPlayerNoOneDoesShape)
}

pub(crate) fn parse_each_player_choose_bounce_draw_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerChooseBounceDrawShape> {
    (surface_starts_with(
        tokens,
        &[
            "each",
            "player",
            "chooses",
            "a",
            "nonland",
            "permanent",
            "they",
            "control",
        ],
    ) && surface_has_sequence(
        tokens,
        &[
            "return",
            "all",
            "nonland",
            "permanents",
            "not",
            "chosen",
            "this",
            "way",
        ],
    ) && surface_has_sequence(
        tokens,
        &[
            "you", "draw", "a", "card", "for", "each", "opponent", "who", "has", "more", "cards",
            "in", "their", "hand", "than", "you",
        ],
    ))
    .then_some(EachPlayerChooseBounceDrawShape)
}

pub(crate) fn parse_statement_force_shape(tokens: &[OwnedLexToken]) -> Option<StatementForceShape> {
    if parse_player_gets_counters_surface_tokens(tokens).is_some() {
        return Some(StatementForceShape::PlayerGetsCounters);
    }
    if surface_has_sequence(tokens, &["chooses", "two", "of", "those", "cards"])
        && surface_has_sequence(tokens, &["shuffle", "the", "chosen", "cards"])
        && surface_has_sequence(
            tokens,
            &["put", "the", "rest", "onto", "the", "battlefield"],
        )
    {
        return Some(StatementForceShape::DivvySelection);
    }
    if surface_has_sequence(
        tokens,
        &[
            "for", "as", "long", "as", "that", "card", "remains", "exiled",
        ],
    ) && surface_has_sequence(tokens, &["more", "to", "cast"])
    {
        return Some(StatementForceShape::ExilePlayCost);
    }
    if surface_starts_with(tokens, &["if"]) && surface_has_sequence(tokens, &["instead"]) {
        return Some(StatementForceShape::ConditionalInstead);
    }
    if (surface_starts_with(tokens, &["each"]) || surface_starts_with(tokens, &["all"]))
        && surface_has_sequence(tokens, &["until", "end", "of", "turn"])
    {
        return Some(StatementForceShape::GroupTurnDuration);
    }
    None
}

pub(crate) fn parse_next_damage_prevention_tokens(
    tokens: &[OwnedLexToken],
) -> Option<NextDamagePreventionShape> {
    (surface_starts_with(tokens, &["the", "next", "time"])
        && surface_has_sequence(tokens, &["source", "of", "your", "choice"])
        && surface_has_sequence(tokens, &["prevent", "that", "damage"])
        && surface_has_sequence(tokens, &["damage", "is", "prevented", "this", "way"]))
    .then_some(NextDamagePreventionShape)
}

pub(crate) fn parse_day_night_enters_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DayNightEntersShape> {
    (surface_starts_with(tokens, &["if"])
        && surface_has_sequence(tokens, &["neither", "day", "nor", "night"])
        && surface_has_sequence(tokens, &["it", "becomes", "day"])
        && (surface_has_sequence(tokens, &["as", "this", "creature", "enters"])
            || surface_has_sequence(tokens, &["as", "this", "permanent", "enters"])
            || surface_has_sequence(tokens, &["as", "this", "object", "enters"])))
    .then_some(DayNightEntersShape)
}

pub(crate) fn parse_reveal_from_hand_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RevealFromHandShape> {
    primitives::parse_all(
        tokens,
        (
            primitives::phrase(&["reveal", "this", "card", "from", "your", "hand"]),
            eof,
        )
            .value(RevealFromHandShape),
        "reveal-this-card-from-hand",
    )
    .ok()
}

pub(crate) fn has_statement_error_prefix(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("choose"),
            primitives::kw("if"),
            primitives::kw("reveal"),
        )),
    )
    .is_some()
}

#[cfg(test)]
#[path = "statement_shapes_tests.rs"]
mod tests;
