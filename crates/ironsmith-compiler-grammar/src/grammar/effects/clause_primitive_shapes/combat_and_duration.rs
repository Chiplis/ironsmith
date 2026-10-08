use super::*;
use crate::lexer::LexStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatRequirementKind {
    AttackOrBlock,
    Attack,
    MustBeBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatRequirementDuration {
    Turn,
    Combat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatRequirementShape<'a> {
    pub kind: CombatRequirementKind,
    pub duration: CombatRequirementDuration,
    pub subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MustBlockShape<'a> {
    SubjectThisTurn {
        subject_tokens: &'a [OwnedLexToken],
    },
    AllCreatures {
        /// "creatures your opponents control" when the blockers are
        /// qualified (You Look Upon the Tarrasque); unqualified otherwise.
        blocker_filter_tokens: Option<&'a [OwnedLexToken]>,
        attacker_and_duration_tokens: &'a [OwnedLexToken],
    },
    SubjectAgainstAttacker {
        subject_tokens: &'a [OwnedLexToken],
        attacker_and_duration_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationTriggerPrefixShape {
    UntilEndOfTurn,
    UntilYourNextTurn,
    /// "until (the) end of your next turn" (Season of the Bold).
    UntilEndOfYourNextTurn,
    UntilYourNextUpkeep,
    UntilYourNextUntapStep,
    DuringYourNextUntapStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerClauseIntroShape {
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

fn must_be_blocked_suffix<'a>(input: &mut LexStream<'a>) -> WResult<CombatRequirementDuration> {
    (
        alt((
            primitives::phrase(&["must", "be", "blocked", "if", "able"])
                .value(CombatRequirementDuration::Turn),
            primitives::phrase(&["must", "be", "blocked", "this", "turn", "if", "able"])
                .value(CombatRequirementDuration::Turn),
            primitives::phrase(&[
                "must", "be", "blocked", "each", "combat", "this", "turn", "if", "able",
            ])
            .value(CombatRequirementDuration::Turn),
            primitives::phrase(&["must", "be", "blocked", "this", "combat", "if", "able"])
                .value(CombatRequirementDuration::Combat),
        )),
        primitives::sentence_end(),
    )
        .map(|(duration, _)| duration)
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
            // Keep the newly reachable source subject raw. Its complete
            // owner must see punctuation immediately before `must`, including
            // a period that the sentence-boundary check below must reject.
            // Other requirement families retain their existing normalization.
            let subject_tokens = if subject_tokens.first().is_some_and(|token| token.is_word("this")) {
                subject_tokens
            } else {
                trim_shape_edges(subject_tokens)
            };
            let duration = must_be_blocked_suffix.parse_next(input)?;
            Ok(CombatRequirementShape {
                kind: CombatRequirementKind::MustBeBlocked,
                duration,
                subject_tokens,
            })
        },
    ))
    .parse_next(input)
}

pub fn parse_combat_requirement_shape(
    tokens: &[OwnedLexToken],
) -> Option<CombatRequirementShape<'_>> {
    let shape = crate::grammar::primitives::probe_all(
        trim_shape_edges(tokens),
        combat_requirement,
        "combat requirement clause",
    )?;
    // A combat requirement's subject belongs to the same sentence as its
    // suffix. Without this boundary, the suffix parser can scan backward
    // across prior instructions and reinterpret an entire animation sentence
    // as the target of a later “It must be blocked” follow-up.
    (!shape.subject_tokens.iter().any(|token| token.is_period())).then_some(shape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackRequirementPlayer {
    /// "that player": the player the preceding instruction named.
    ThatPlayer,
    You,
    /// "attacks a player": any player rather than a planeswalker or battle.
    APlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackPlayerRequirementDuration {
    Turn,
    Combat,
    /// "each combat": every combat within an explicitly stated duration.
    EachCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackPlayerRequirementShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub player: AttackRequirementPlayer,
    pub duration: AttackPlayerRequirementDuration,
}

fn attack_player_suffix<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(AttackRequirementPlayer, AttackPlayerRequirementDuration)> {
    (
        alt((primitives::kw("attack"), primitives::kw("attacks"))),
        alt((
            primitives::phrase(&["that", "player"]).value(AttackRequirementPlayer::ThatPlayer),
            // "attacks you this turn" is the typed this-turn requirement
            // (effect_sentences::attack_player_requirement) only.
            primitives::phrase(&["a", "player"]).value(AttackRequirementPlayer::APlayer),
        )),
        alt((
            primitives::phrase(&["this", "turn"]).value(AttackPlayerRequirementDuration::Turn),
            primitives::phrase(&["this", "combat"]).value(AttackPlayerRequirementDuration::Combat),
            primitives::phrase(&["each", "combat"])
                .value(AttackPlayerRequirementDuration::EachCombat),
        )),
        primitives::phrase(&["if", "able"]),
        primitives::sentence_end(),
    )
        .map(|(_, player, duration, _, _)| (player, duration))
        .parse_next(input)
}

fn attack_player_requirement<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttackPlayerRequirementShape<'a>> {
    let subject_tokens = repeat_till(1.., any.void(), peek(attack_player_suffix))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let (player, duration) = attack_player_suffix.parse_next(input)?;
    Ok(AttackPlayerRequirementShape {
        subject_tokens: trim_shape_edges(subject_tokens),
        player,
        duration,
    })
}

/// "This creature attacks that player this combat if able." (Ruhan of the
/// Fomori): a requirement to attack one specific player (CR 508.1d).
pub fn parse_attack_player_requirement_shape(
    tokens: &[OwnedLexToken],
) -> Option<AttackPlayerRequirementShape<'_>> {
    let shape = crate::grammar::primitives::probe_all(
        trim_shape_edges(tokens),
        attack_player_requirement,
        "attack player requirement clause",
    )?;
    (!shape.subject_tokens.is_empty()
        && !shape.subject_tokens.iter().any(|token| token.is_period()))
    .then_some(shape)
}

fn time_travel_once<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["time", "travel"]).parse_next(input)
}

fn time_travel_count<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    opt(primitives::kw("then")).parse_next(input)?;
    time_travel_once.parse_next(input)?;
    let count = alt((
        (
            opt(primitives::comma()),
            opt(primitives::kw("then")),
            time_travel_once,
        )
            .value(2u32),
        primitives::kw("twice").value(2u32),
        (
            crate::grammar::leaf::parse_leaf_number_prefix_lexed,
            primitives::kw("times"),
        )
            .map(|(count, _)| count),
        winnow::combinator::empty.value(1u32),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(count)
}

/// "time travel", "time travel twice / three times", "time travel, then time
/// travel": how many times the keyword action is performed.
pub fn parse_time_travel_count_shape(tokens: &[OwnedLexToken]) -> Option<u32> {
    crate::grammar::primitives::probe_all(
        trim_shape_edges(tokens),
        time_travel_count,
        "time travel clause",
    )
    .filter(|count| *count > 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReselectAttackTargetShape<'a> {
    pub attacker_tokens: &'a [OwnedLexToken],
    pub players_only: bool,
}

fn reselect_attack_target<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ReselectAttackTargetShape<'a>> {
    primitives::phrase(&["reselect", "which"]).parse_next(input)?;
    let players_only = alt((
        primitives::phrase(&["player", "or", "permanent"]).value(false),
        primitives::kw("player").value(true),
    ))
    .parse_next(input)?;
    let attacker_tokens = repeat_till(
        1..,
        any.void(),
        peek((primitives::phrase(&["is", "attacking"]), primitives::sentence_end())),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["is", "attacking"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ReselectAttackTargetShape {
        attacker_tokens: trim_shape_edges(attacker_tokens),
        players_only,
    })
}

pub fn parse_reselect_attack_target_shape(
    tokens: &[OwnedLexToken],
) -> Option<ReselectAttackTargetShape<'_>> {
    crate::grammar::primitives::probe_all(
        trim_shape_edges(tokens),
        reselect_attack_target,
        "reselect attack target clause",
    )
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
        blocker_filter_tokens: None,
        attacker_and_duration_tokens: trim_shape_edges(attacker_and_duration_tokens),
    })
}

/// "All creatures your opponents control able to block that creature this
/// turn do so.": the Lure requirement over a qualified blocker set.
fn all_filtered_creatures_block<'a>(input: &mut LexStream<'a>) -> WResult<MustBlockShape<'a>> {
    primitives::kw("all").parse_next(input)?;
    let able = || primitives::phrase(&["able", "to", "block"]);
    let blocker_filter_tokens = (
        primitives::kw("creatures"),
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(able())),
    )
        .take()
        .parse_next(input)?;
    able().parse_next(input)?;
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
        blocker_filter_tokens: Some(trim_shape_edges(blocker_filter_tokens)),
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

pub fn parse_must_block_shape(tokens: &[OwnedLexToken]) -> Option<MustBlockShape<'_>> {
    crate::grammar::primitives::probe_all(
        trim_shape_edges(tokens),
        alt((
            subject_blocks_this_turn,
            all_creatures_block,
            all_filtered_creatures_block,
            subject_blocks_attacker,
        )),
        "must block clause",
    )
}

pub fn parse_duration_trigger_prefix_shape(
    tokens: &[OwnedLexToken],
) -> Option<DurationTriggerPrefixShape> {
    primitives::parse_prefix(
        trim_shape_edges(tokens),
        alt((
            primitives::phrase(&["until", "end", "of", "turn"])
                .value(DurationTriggerPrefixShape::UntilEndOfTurn),
            primitives::phrase(&["until", "your", "next", "turn"])
                .value(DurationTriggerPrefixShape::UntilYourNextTurn),
            primitives::phrase(&["until", "the", "end", "of", "your", "next", "turn"])
                .value(DurationTriggerPrefixShape::UntilEndOfYourNextTurn),
            primitives::phrase(&["until", "end", "of", "your", "next", "turn"])
                .value(DurationTriggerPrefixShape::UntilEndOfYourNextTurn),
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

pub fn parse_trigger_clause_intro_shape(
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
