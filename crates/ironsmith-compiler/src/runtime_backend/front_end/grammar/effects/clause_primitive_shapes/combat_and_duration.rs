use super::*;
use crate::runtime_backend::front_end::lexer::LexStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatRequirementKind {
    AttackOrBlock,
    Attack,
    MustBeBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CombatRequirementShape<'a> {
    pub(crate) kind: CombatRequirementKind,
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

fn attack_or_block_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        alt((
            primitives::phrase(&["attack", "or", "block", "this", "turn", "if", "able"]),
            primitives::phrase(&["attacks", "or", "blocks", "this", "turn", "if", "able"]),
            primitives::phrase(&["attacks", "or", "block", "this", "turn", "if", "able"]),
            primitives::phrase(&["attack", "or", "blocks", "this", "turn", "if", "able"]),
        )),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

fn attack_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        alt((
            primitives::phrase(&["attack", "this", "turn", "if", "able"]),
            primitives::phrase(&["attacks", "this", "turn", "if", "able"]),
        )),
        primitives::sentence_end(),
    )
        .void()
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
                .map(|((), ())| ())
                .take()
                .parse_next(input)?;
            attack_or_block_suffix.parse_next(input)?;
            Ok(CombatRequirementShape {
                kind: CombatRequirementKind::AttackOrBlock,
                subject_tokens: trim_shape_edges(subject_tokens),
            })
        },
        |input: &mut LexStream<'a>| {
            let subject_tokens = repeat_till(0.., any.void(), peek(attack_suffix))
                .map(|((), ())| ())
                .take()
                .parse_next(input)?;
            attack_suffix.parse_next(input)?;
            Ok(CombatRequirementShape {
                kind: CombatRequirementKind::Attack,
                subject_tokens: trim_shape_edges(subject_tokens),
            })
        },
        |input: &mut LexStream<'a>| {
            let subject_tokens = repeat_till(1.., any.void(), peek(must_be_blocked_suffix))
                .map(|((), ())| ())
                .take()
                .parse_next(input)?;
            must_be_blocked_suffix.parse_next(input)?;
            Ok(CombatRequirementShape {
                kind: CombatRequirementKind::MustBeBlocked,
                subject_tokens: trim_shape_edges(subject_tokens),
            })
        },
    ))
    .parse_next(input)
}

pub(crate) fn parse_combat_requirement_shape(
    tokens: &[OwnedLexToken],
) -> Option<CombatRequirementShape<'_>> {
    primitives::parse_all(
        trim_shape_edges(tokens),
        combat_requirement,
        "combat requirement clause",
    )
    .ok()
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
        .map(|((), ())| ())
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
