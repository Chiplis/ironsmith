use super::super::*;

use crate::ExtraTurnAnchorAst;
use crate::effects::AdditionalPhase;
use crate::runtime_backend::front_end::grammar::leaf;
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MonstrosityShape {
    pub(crate) amount: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CounterRemovedPumpShape {
    pub(crate) power: i32,
    pub(crate) toughness: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenEndCombatActionShape {
    Exile,
    Sacrifice,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExtraTurnShape {
    pub(crate) player: PlayerAst,
    pub(crate) anchor: ExtraTurnAnchorAst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdditionalPhasesShape {
    pub(crate) phases: Vec<AdditionalPhase>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VotedWithYouScryShape {
    pub(crate) count: Value,
}

fn monstrosity<'a>(input: &mut LexStream<'a>) -> WResult<Value> {
    primitives::kw("monstrosity").parse_next(input)?;
    let amount = leaf::parse_leaf_number_or_x_prefix_lexed
        .parse_next(input)?
        .into_value()
        .ok_or_else(|| primitives::backtrack_err("monstrosity amount", "runtime value"))?;
    primitives::sentence_end().parse_next(input)?;
    Ok(amount)
}

pub(crate) fn parse_monstrosity_shape(tokens: &[OwnedLexToken]) -> Option<MonstrosityShape> {
    primitives::parse_all(tokens, monstrosity, "monstrosity shape")
        .ok()
        .map(|amount| MonstrosityShape { amount })
}

fn fixed_modifier(token: &OwnedLexToken) -> Option<(i32, i32)> {
    let (power, toughness) =
        leaf::parse_leaf_pt_modifier_values_complete(token.parser_text()).ok()?;
    match (power, toughness) {
        (Value::Fixed(power), Value::Fixed(toughness)) => Some((power, toughness)),
        _ => None,
    }
}

pub(crate) fn parse_counter_removed_pump_shape(
    tokens: &[OwnedLexToken],
) -> Option<CounterRemovedPumpShape> {
    let (_, body) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["for", "each", "counter", "removed", "this", "way"]),
    )?;
    let (subject_tokens, modifier_tokens) =
        primitives::split_lexed_once_on_separator(body, || {
            alt((primitives::kw("get"), primitives::kw("gets"))).void()
        })?;
    if parse_subject(trim_lexed_commas(subject_tokens)) != SubjectAst::This {
        return None;
    }
    let modifier_tokens = trim_lexed_commas(modifier_tokens);
    let (power, toughness) = fixed_modifier(modifier_tokens.first()?)?;
    Some(CounterRemovedPumpShape { power, toughness })
}

fn token_end_combat<'a>(input: &mut LexStream<'a>) -> WResult<TokenEndCombatActionShape> {
    let action = alt((
        primitives::kw("exile").value(TokenEndCombatActionShape::Exile),
        primitives::kw("sacrifice").value(TokenEndCombatActionShape::Sacrifice),
    ))
    .parse_next(input)?;
    primitives::any_phrase(&[
        &["that", "token"],
        &["that", "tokens"],
        &["the", "token"],
        &["the", "tokens"],
        &["those", "token"],
        &["those", "tokens"],
        &["it"],
    ])
    .parse_next(input)?;
    primitives::kw("at").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["end", "of", "combat"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(action)
}

pub(crate) fn parse_token_end_combat_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<TokenEndCombatActionShape> {
    primitives::parse_all(tokens, token_end_combat, "token end-of-combat action").ok()
}

fn extra_turn<'a>(input: &mut LexStream<'a>) -> WResult<ExtraTurnShape> {
    alt((
        (
            primitives::phrase(&["take", "an", "extra", "turn", "after", "this", "one"]),
            primitives::sentence_end(),
        )
            .value(ExtraTurnShape {
                player: PlayerAst::You,
                anchor: ExtraTurnAnchorAst::CurrentTurn,
            }),
        (
            primitives::phrase(&[
                "the", "chosen", "player", "takes", "an", "extra", "turn", "after", "this", "one",
            ]),
            primitives::sentence_end(),
        )
            .value(ExtraTurnShape {
                player: PlayerAst::Chosen,
                anchor: ExtraTurnAnchorAst::CurrentTurn,
            }),
        (
            (
                primitives::phrase(&["after", "that", "turn"]),
                opt(primitives::comma()),
                primitives::phrase(&["that", "player", "takes", "an", "extra", "turn"]),
            ),
            primitives::sentence_end(),
        )
            .value(ExtraTurnShape {
                player: PlayerAst::That,
                anchor: ExtraTurnAnchorAst::ReferencedTurn,
            }),
    ))
    .parse_next(input)
}

pub(crate) fn parse_extra_turn_shape(tokens: &[OwnedLexToken]) -> Option<ExtraTurnShape> {
    primitives::parse_all(tokens, extra_turn, "extra turn shape").ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhaseIntroShape {
    Empty,
    AfterThisPhase,
    AfterThisCombat,
    AfterThisMain,
    IfYourMain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhaseTailShape {
    Empty,
    AfterThisPhase,
    FollowedByMain,
    AfterThisPhaseFollowedByMain,
}

fn phase_intro<'a>(input: &mut LexStream<'a>) -> WResult<PhaseIntroShape> {
    opt(alt((
        primitives::phrase(&["after", "this", "combat", "phase"])
            .value(PhaseIntroShape::AfterThisCombat),
        primitives::phrase(&["after", "this", "main", "phase"])
            .value(PhaseIntroShape::AfterThisMain),
        primitives::phrase(&["after", "this", "phase"]).value(PhaseIntroShape::AfterThisPhase),
        primitives::any_phrase(&[
            &["if", "it", "your", "main", "phase"],
            &["if", "it's", "your", "main", "phase"],
            &["if", "its", "your", "main", "phase"],
        ])
        .value(PhaseIntroShape::IfYourMain),
    )))
    .map(Option::unwrap_or_default)
    .parse_next(input)
}

impl Default for PhaseIntroShape {
    fn default() -> Self {
        Self::Empty
    }
}

fn phase_tail<'a>(input: &mut LexStream<'a>) -> WResult<PhaseTailShape> {
    opt(alt((
        primitives::phrase(&[
            "after",
            "this",
            "phase",
            "followed",
            "by",
            "an",
            "additional",
            "main",
            "phase",
        ])
        .value(PhaseTailShape::AfterThisPhaseFollowedByMain),
        primitives::phrase(&["followed", "by", "an", "additional", "main", "phase"])
            .value(PhaseTailShape::FollowedByMain),
        primitives::phrase(&["after", "this", "phase"]).value(PhaseTailShape::AfterThisPhase),
    )))
    .map(|tail| tail.unwrap_or(PhaseTailShape::Empty))
    .parse_next(input)
}

fn additional_phases<'a>(input: &mut LexStream<'a>) -> WResult<AdditionalPhasesShape> {
    let intro = phase_intro.parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::any_phrase(&[
        &["there", "is"],
        &["there's"],
        &["theres"],
        &["there", "are"],
        &["there're"],
        &["therere"],
    ])
    .parse_next(input)?;
    let count = alt((
        primitives::kw("an").value(1u8),
        primitives::kw("two").value(2u8),
    ))
    .parse_next(input)?;
    primitives::kw("additional").parse_next(input)?;
    if count == 1 {
        primitives::phrase(&["combat", "phase"]).parse_next(input)?;
    } else {
        primitives::phrase(&["combat", "phases"]).parse_next(input)?;
    }
    let tail = phase_tail.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let followed_by_main = match (intro, tail) {
        (
            PhaseIntroShape::AfterThisPhase | PhaseIntroShape::AfterThisMain,
            PhaseTailShape::FollowedByMain,
        )
        | (
            PhaseIntroShape::Empty | PhaseIntroShape::IfYourMain,
            PhaseTailShape::AfterThisPhaseFollowedByMain,
        ) => true,
        (
            PhaseIntroShape::AfterThisPhase
            | PhaseIntroShape::AfterThisCombat
            | PhaseIntroShape::AfterThisMain,
            PhaseTailShape::Empty,
        )
        | (PhaseIntroShape::Empty, PhaseTailShape::AfterThisPhase) => false,
        _ => {
            return Err(primitives::backtrack_err(
                "additional phases",
                "supported phase order",
            ));
        }
    };
    let phases = if followed_by_main {
        if count != 1 {
            return Err(primitives::backtrack_err(
                "additional phases",
                "one combat before main",
            ));
        }
        vec![AdditionalPhase::Combat, AdditionalPhase::Main]
    } else if count == 2 {
        vec![AdditionalPhase::Combat, AdditionalPhase::Combat]
    } else {
        vec![AdditionalPhase::Combat]
    };
    Ok(AdditionalPhasesShape { phases })
}

pub(crate) fn parse_additional_phases_shape(
    tokens: &[OwnedLexToken],
) -> Option<AdditionalPhasesShape> {
    primitives::parse_all(tokens, additional_phases, "additional phases shape").ok()
}

fn voted_with_you_scry<'a>(input: &mut LexStream<'a>) -> WResult<Value> {
    primitives::phrase(&[
        "you", "and", "each", "opponent", "who", "voted", "for", "a", "choice", "you", "voted",
        "for",
    ])
    .parse_next(input)?;
    primitives::kw("may").parse_next(input)?;
    primitives::kw("scry").parse_next(input)?;
    let count = leaf::parse_leaf_number_or_x_prefix_lexed
        .parse_next(input)?
        .into_value()
        .ok_or_else(|| primitives::backtrack_err("vote scry count", "runtime value"))?;
    primitives::sentence_end().parse_next(input)?;
    Ok(count)
}

pub(crate) fn parse_voted_with_you_scry_shape(
    tokens: &[OwnedLexToken],
) -> Option<VotedWithYouScryShape> {
    primitives::parse_all(tokens, voted_with_you_scry, "voted-with-you scry")
        .ok()
        .map(|count| VotedWithYouScryShape { count })
}

#[cfg(test)]
#[path = "actions/tests.rs"]
mod tests;
