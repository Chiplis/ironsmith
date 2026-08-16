use super::*;
use crate::lexer::LexStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatRequirementKind {
    AttackOrBlock,
    Attack,
    MustBeBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatRequirementDuration {
    Turn,
    Combat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CombatRequirementShape<'a> {
    pub(crate) kind: CombatRequirementKind,
    pub(crate) duration: CombatRequirementDuration,
    pub(crate) subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MustBlockShape<'a> {
    SubjectThisTurn {
        subject_tokens: &'a [OwnedLexToken],
    },
    AllCreatures {
        attacker_and_duration_tokens: &'a [OwnedLexToken],
    },
    SubjectAgainstAttacker {
        subject_tokens: &'a [OwnedLexToken],
        attacker_and_duration_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DurationTriggerPrefixShape {
    UntilEndOfTurn,
    UntilYourNextTurn,
    UntilYourNextUpkeep,
    UntilYourNextUntapStep,
    DuringYourNextUntapStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerClauseIntroShape {
    Event,
    Step,
}

fn attack_or_block_suffix<'a>(input: &mut LexStream<'a>) -> WResult<CombatRequirementDuration> {
    (
        alt((
            primitives::phrase(&["attack", "or", "block"]),
            primitives::phrase(&["attacks", "or", "blocks"]),
            primitives::phrase(&["attacks", "or", "block"]),
            primitives::phrase(&["attack", "or", "blocks"]),
        )),
        alt((
            primitives::phrase(&["this", "turn"]).value(CombatRequirementDuration::Turn),
            primitives::phrase(&["this", "combat"]).value(CombatRequirementDuration::Combat),
        )),
        primitives::phrase(&["if", "able"]),
        primitives::sentence_end(),
    )
        .map(|(_, duration, _, _)| duration)
        .parse_next(input)
}

fn attack_suffix<'a>(input: &mut LexStream<'a>) -> WResult<CombatRequirementDuration> {
    (
        alt((
            primitives::phrase(&["attack"]),
            primitives::phrase(&["attacks"]),
        )),
        alt((
            primitives::phrase(&["this", "turn"]).value(CombatRequirementDuration::Turn),
            primitives::phrase(&["this", "combat"]).value(CombatRequirementDuration::Combat),
        )),
        primitives::phrase(&["if", "able"]),
        primitives::sentence_end(),
    )
        .map(|(_, duration, _, _)| duration)
        .parse_next(input)
}

fn must_be_blocked_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        alt((
            primitives::phrase(&["must", "be", "blocked", "if", "able"]),
            primitives::phrase(&["must", "be", "blocked", "this", "turn", "if", "able"]),
            primitives::phrase(&[
                "must", "be", "blocked", "each", "combat", "this", "turn", "if", "able",
            ]),
        )),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

fn combat_requirement<'a>(input: &mut LexStream<'a>) -> WResult<CombatRequirementShape<'a>> {
    alt((
        |input: &mut LexStream<'a>| {
            let subject_tokens = repeat_till(0.., any.void(), peek(attack_or_block_suffix))
                .map(|((), _)| ())
                .take()
                .parse_next(input)?;
            let duration = attack_or_block_suffix.parse_next(input)?;
            Ok(CombatRequirementShape {
                kind: CombatRequirementKind::AttackOrBlock,
                duration,
                subject_tokens: trim_shape_edges(subject_tokens),
            })
        },
        |input: &mut LexStream<'a>| {
            let subject_tokens = repeat_till(0.., any.void(), peek(attack_suffix))
                .map(|((), _)| ())
                .take()
                .parse_next(input)?;
            let duration = attack_suffix.parse_next(input)?;
            Ok(CombatRequirementShape {
                kind: CombatRequirementKind::Attack,
                duration,
                subject_tokens: trim_shape_edges(subject_tokens),
            })
        },
        |input: &mut LexStream<'a>| {
            let subject_tokens = repeat_till(1.., any.void(), peek(must_be_blocked_suffix))
                .map(|((), _duration)| ())
                .take()
                .parse_next(input)?;
            must_be_blocked_suffix.parse_next(input)?;
            Ok(CombatRequirementShape {
                kind: CombatRequirementKind::MustBeBlocked,
                duration: CombatRequirementDuration::Turn,
                subject_tokens: trim_shape_edges(subject_tokens),
            })
        },
    ))
    .parse_next(input)
}

pub(crate) fn parse_combat_requirement_shape(
    tokens: &[OwnedLexToken],
) -> Option<CombatRequirementShape<'_>> {
    let shape = primitives::parse_all(
        trim_shape_edges(tokens),
        combat_requirement,
        "combat requirement clause",
    )
    .ok()?;
    // A combat requirement's subject belongs to the same sentence as its
    // suffix. Without this boundary, the suffix parser can scan backward
    // across prior instructions and reinterpret an entire animation sentence
    // as the target of a later “It must be blocked” follow-up.
    (!shape.subject_tokens.iter().any(|token| token.is_period())).then_some(shape)
}

fn subject_blocks_this_turn<'a>(input: &mut LexStream<'a>) -> WResult<MustBlockShape<'a>> {
    let suffix = || {
        (
            alt((primitives::kw("block"), primitives::kw("blocks"))),
            primitives::phrase(&["this", "turn", "if", "able"]),
            primitives::sentence_end(),
        )
            .void()
    };
    let subject_tokens = repeat_till(1.., any.void(), peek(suffix()))
        .map(|((), _duration)| ())
        .take()
        .parse_next(input)?;
    suffix().parse_next(input)?;
    Ok(MustBlockShape::SubjectThisTurn {
        subject_tokens: trim_shape_edges(subject_tokens),
    })
}

fn all_creatures_block<'a>(input: &mut LexStream<'a>) -> WResult<MustBlockShape<'a>> {
    primitives::phrase(&["all", "creatures", "able", "to", "block"]).parse_next(input)?;
    let suffix = || {
        (
            primitives::phrase(&["do", "so"]),
            primitives::sentence_end(),
        )
            .void()
    };
    let attacker_and_duration_tokens = repeat_till(1.., any.void(), peek(suffix()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    suffix().parse_next(input)?;
    Ok(MustBlockShape::AllCreatures {
        attacker_and_duration_tokens: trim_shape_edges(attacker_and_duration_tokens),
    })
}

fn subject_blocks_attacker<'a>(input: &mut LexStream<'a>) -> WResult<MustBlockShape<'a>> {
    let block = || alt((primitives::kw("block"), primitives::kw("blocks"))).void();
    let subject_tokens = repeat_till(1.., any.void(), peek(block()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    block().parse_next(input)?;
    let suffix = || {
        (
            primitives::phrase(&["if", "able"]),
            primitives::sentence_end(),
        )
            .void()
    };
    let attacker_and_duration_tokens = repeat_till(1.., any.void(), peek(suffix()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    suffix().parse_next(input)?;
    Ok(MustBlockShape::SubjectAgainstAttacker {
        subject_tokens: trim_shape_edges(subject_tokens),
        attacker_and_duration_tokens: trim_shape_edges(attacker_and_duration_tokens),
    })
}

pub(crate) fn parse_must_block_shape(tokens: &[OwnedLexToken]) -> Option<MustBlockShape<'_>> {
    primitives::parse_all(
        trim_shape_edges(tokens),
        alt((
            subject_blocks_this_turn,
            all_creatures_block,
            subject_blocks_attacker,
        )),
        "must block clause",
    )
    .ok()
}

pub(crate) fn parse_duration_trigger_prefix_shape(
    tokens: &[OwnedLexToken],
) -> Option<DurationTriggerPrefixShape> {
    primitives::parse_prefix(
        trim_shape_edges(tokens),
        alt((
            primitives::phrase(&["until", "end", "of", "turn"])
                .value(DurationTriggerPrefixShape::UntilEndOfTurn),
            primitives::phrase(&["until", "your", "next", "turn"])
                .value(DurationTriggerPrefixShape::UntilYourNextTurn),
            primitives::phrase(&["until", "your", "next", "upkeep"])
                .value(DurationTriggerPrefixShape::UntilYourNextUpkeep),
            primitives::phrase(&["until", "your", "next", "untap", "step"])
                .value(DurationTriggerPrefixShape::UntilYourNextUntapStep),
            primitives::phrase(&["during", "your", "next", "untap", "step"])
                .value(DurationTriggerPrefixShape::DuringYourNextUntapStep),
        )),
    )
    .map(|(shape, _)| shape)
}

pub(crate) fn parse_trigger_clause_intro_shape(
    tokens: &[OwnedLexToken],
) -> Option<TriggerClauseIntroShape> {
    primitives::parse_prefix(
        trim_shape_edges(tokens),
        alt((
            alt((primitives::kw("when"), primitives::kw("whenever")))
                .value(TriggerClauseIntroShape::Event),
            primitives::phrase(&["at", "the"]).value(TriggerClauseIntroShape::Step),
        )),
    )
    .map(|(shape, _)| shape)
}
