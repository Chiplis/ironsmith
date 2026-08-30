use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::primitives;
use crate::lexer::{LexStream, LexedClause, OwnedLexToken};
use crate::target::PlayerFilter;

#[path = "delayed_sentence_shapes/schedule.rs"]
mod schedule;
pub use schedule::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayedObjectKind {
    Creature,
    Permanent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayedLeavesObjectKind {
    Creature,
    Permanent,
    Token,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayedTaggedLeavesShape<'a> {
    pub kind: DelayedLeavesObjectKind,
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayedNextCombatShape<'a> {
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub struct DelayedEndStepShape<'a> {
    pub player: PlayerFilter,
    pub start_next_turn: bool,
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayedThisTurnPlacement {
    LeadingDuration,
    TrailingDuration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayedThisTurnShape<'a> {
    pub placement: DelayedThisTurnPlacement,
    pub trigger_tokens: &'a [OwnedLexToken],
    pub effect_tokens: &'a [OwnedLexToken],
    pub references_previous_creature: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayedTaggedDamageShape {
    pub kind: DelayedObjectKind,
    pub combat: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyTwiceShape {
    pub may_choose_new_targets: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayedDiesShape<'a> {
    ThatReference {
        effect_tokens: &'a [OwnedLexToken],
    },
    DefinitePriorTarget {
        subject_tokens: &'a [OwnedLexToken],
        effect_tokens: &'a [OwnedLexToken],
    },
    ThisWay {
        subject_tokens: &'a [OwnedLexToken],
        effect_tokens: &'a [OwnedLexToken],
    },
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    LexedClause::new(tokens).trimmed().tokens()
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    (
        repeat::<_, _, (), _, _>(0.., semantic_noise),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

fn semantic_noise<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::comma(),
        primitives::period(),
        primitives::semicolon(),
    ))
    .void()
    .parse_next(input)
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn trigger_intro<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("when"), primitives::kw("whenever")))
        .void()
        .parse_next(input)
}

fn dies_intro<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("when"),
        primitives::kw("whenever"),
        primitives::kw("if"),
    ))
    .void()
    .parse_next(input)
}

fn object_kind<'a>(input: &mut LexStream<'a>) -> WResult<DelayedObjectKind> {
    alt((
        primitives::kw("creature").value(DelayedObjectKind::Creature),
        primitives::kw("permanent").value(DelayedObjectKind::Permanent),
    ))
    .parse_next(input)
}

fn leaves_object_kind<'a>(input: &mut LexStream<'a>) -> WResult<DelayedLeavesObjectKind> {
    alt((
        primitives::kw("creature").value(DelayedLeavesObjectKind::Creature),
        primitives::kw("permanent").value(DelayedLeavesObjectKind::Permanent),
        primitives::kw("token").value(DelayedLeavesObjectKind::Token),
    ))
    .parse_next(input)
}

/// Parses a delayed follow-up that watches the object selected or created by
/// the preceding effect: `When that creature/token/permanent leaves the
/// battlefield, ...`.
pub fn parse_delayed_tagged_leaves_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedTaggedLeavesShape<'_>> {
    let tokens = trimmed(tokens);
    let (header_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let kind = primitives::parse_all(
        trimmed(header_tokens),
        (
            trigger_intro,
            primitives::kw("that"),
            leaves_object_kind,
            primitives::phrase(&["leaves", "the", "battlefield"]),
            eof,
        )
            .map(|(_, _, kind, _, _)| kind),
        "delayed tagged-object leaves trigger",
    )
    .ok()?;
    let effect_tokens = trimmed(effect_tokens);
    (!effect_tokens.is_empty()).then_some(DelayedTaggedLeavesShape {
        kind,
        effect_tokens,
    })
}

pub fn parse_delayed_next_combat_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedNextCombatShape<'_>> {
    let tokens = trimmed(tokens);
    let (_, after_header) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "combat"]),
            opt(primitives::kw("phase")),
            primitives::phrase(&["this", "turn"]),
            primitives::comma(),
        ),
    )?;
    let effect_tokens = trimmed(after_header);
    (!effect_tokens.is_empty()).then_some(DelayedNextCombatShape { effect_tokens })
}

fn end_step_owner<'a>(input: &mut LexStream<'a>) -> WResult<PlayerFilter> {
    alt((
        semantic_kw("your").value(PlayerFilter::You),
        (
            primitives::kw("that"),
            alt((semantic_kw("player"), semantic_kw("players"))),
        )
            .value(PlayerFilter::IteratedPlayer),
        (
            primitives::kw("target"),
            alt((semantic_kw("player"), semantic_kw("players"))),
        )
            .value(PlayerFilter::Target(Box::new(PlayerFilter::Any))),
    ))
    .parse_next(input)
}

fn end_step_header<'a>(input: &mut LexStream<'a>) -> WResult<(PlayerFilter, bool)> {
    primitives::kw("at").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["beginning", "of"]).parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    let step_owner = opt(semantic_kw("your").value(PlayerFilter::You)).parse_next(input)?;
    alt((
        primitives::phrase(&["next", "end", "step"]),
        primitives::phrase(&["end", "step"]),
    ))
    .parse_next(input)?;
    let turn_owner = opt((
        primitives::kw("of"),
        end_step_owner,
        primitives::phrase(&["next", "turn"]),
    ))
    .parse_next(input)?;
    let start_next_turn = turn_owner.is_some();
    let player = turn_owner
        .map(|(_, player, _)| player)
        .or(step_owner)
        .unwrap_or(PlayerFilter::Any);
    Ok((player, start_next_turn))
}

pub fn parse_delayed_end_step_shape(tokens: &[OwnedLexToken]) -> Option<DelayedEndStepShape<'_>> {
    let tokens = trimmed(tokens);
    let ((player, start_next_turn), after_comma) = primitives::parse_prefix(
        tokens,
        (end_step_header, primitives::comma()).map(|(header, _)| header),
    )?;
    let effect_tokens = trimmed(after_comma);
    (!effect_tokens.is_empty()).then_some(DelayedEndStepShape {
        player,
        start_next_turn,
        effect_tokens,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DelayedTriggerCore<'a> {
    tokens: &'a [OwnedLexToken],
    references_previous_creature: bool,
}

fn parse_intro_and_trigger(tokens: &[OwnedLexToken]) -> Option<DelayedTriggerCore<'_>> {
    let (_, trigger_tokens) = primitives::parse_prefix(trimmed(tokens), trigger_intro)?;
    let trigger_tokens = trimmed(trigger_tokens);
    if trigger_tokens.is_empty() {
        return None;
    }
    let references_previous_creature =
        primitives::parse_prefix(trigger_tokens, semantic_phrase(&["that", "creature"])).is_some();
    Some(DelayedTriggerCore {
        tokens: trigger_tokens,
        references_previous_creature,
    })
}

pub fn parse_delayed_this_turn_shape(tokens: &[OwnedLexToken]) -> Option<DelayedThisTurnShape<'_>> {
    let tokens = trimmed(tokens);

    if let Some(((), after_duration)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["this", "turn"]),
            opt(primitives::comma()),
        )
            .void(),
    ) && let Some((trigger_header, effect_tokens)) =
        primitives::split_lexed_once_on_separator(after_duration, || primitives::comma().void())
        && let Some(trigger) = parse_intro_and_trigger(trigger_header)
    {
        let effect_tokens = trimmed(effect_tokens);
        if !effect_tokens.is_empty() {
            return Some(DelayedThisTurnShape {
                placement: DelayedThisTurnPlacement::LeadingDuration,
                trigger_tokens: trigger.tokens,
                effect_tokens,
                references_previous_creature: trigger.references_previous_creature,
            });
        }
    }

    let (intro_and_trigger, effect_tokens) =
        primitives::split_lexed_once_before_suffix(tokens, 1, || {
            (
                primitives::phrase(&["this", "turn"]),
                primitives::comma(),
                repeat::<_, _, (), _, _>(1.., any.void()).take(),
                eof,
            )
                .map(|(_, _, effect_tokens, _)| effect_tokens)
        })?;
    let trigger = parse_intro_and_trigger(intro_and_trigger)?;
    let effect_tokens = trimmed(effect_tokens);
    (!effect_tokens.is_empty()).then_some(DelayedThisTurnShape {
        placement: DelayedThisTurnPlacement::TrailingDuration,
        trigger_tokens: trigger.tokens,
        effect_tokens,
        references_previous_creature: trigger.references_previous_creature,
    })
}

pub fn parse_delayed_attack_unblocked_subject(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let tokens = trimmed(tokens);
    let (_, after_target) = primitives::parse_prefix(tokens, primitives::kw("target"))?;
    let (subject_tokens, ()) = primitives::split_lexed_once_before_suffix(after_target, 1, || {
        (
            primitives::phrase(&["attacks", "and"]),
            alt((primitives::kw("isn't"), primitives::kw("isnt"))),
            primitives::kw("blocked"),
            eof,
        )
            .void()
    })?;
    let subject_tokens = trimmed(subject_tokens);
    (!subject_tokens.is_empty()).then_some(subject_tokens)
}

pub fn parse_delayed_tagged_damage_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedTaggedDamageShape> {
    primitives::parse_all(
        trimmed(tokens),
        (
            primitives::kw("that"),
            object_kind,
            primitives::phrase(&["is", "dealt"]),
            opt(primitives::kw("combat")),
            primitives::kw("damage"),
            eof,
        )
            .map(|(_, kind, _, combat, _, _)| DelayedTaggedDamageShape {
                kind,
                combat: combat.is_some(),
            }),
        "delayed tagged damage trigger",
    )
    .ok()
}

pub fn parse_delayed_deals_combat_damage_kind(
    tokens: &[OwnedLexToken],
) -> Option<DelayedObjectKind> {
    primitives::parse_all(
        trimmed(tokens),
        (
            primitives::kw("that"),
            object_kind,
            primitives::phrase(&["deals", "combat", "damage", "to", "a", "player"]),
            eof,
        )
            .map(|(_, kind, _, _)| kind),
        "delayed combat-damage trigger",
    )
    .ok()
}

/// Parse the object-kind portion of a delayed "target ... dies" trigger.
///
/// The target is chosen while the enclosing spell or ability resolves; the
/// delayed trigger must subsequently watch that exact object rather than every
/// object matching the descriptive filter.
pub fn parse_delayed_target_dies_subject(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let tokens = trimmed(tokens);
    let (_, after_target) = primitives::parse_prefix(tokens, primitives::kw("target"))?;
    let (subject_tokens, ()) = primitives::split_lexed_once_before_suffix(after_target, 1, || {
        (primitives::kw("dies"), eof).void()
    })?;
    let subject_tokens = trimmed(subject_tokens);
    (!subject_tokens.is_empty()).then_some(subject_tokens)
}

pub struct DelayedTargetCombatDamageShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub recipient_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayedDiesAfterDamageByPreviousCreatureShape<'a> {
    pub victim_tokens: &'a [OwnedLexToken],
}

/// Parse the event core from a delayed watcher such as
/// `Whenever a creature dealt damage by that creature dies this turn, ...`.
///
/// `parse_delayed_this_turn_shape` owns the trailing duration and comma, so
/// this parser deliberately receives only the trigger core. The demonstrative
/// damager refers to the creature selected by the preceding instruction; the
/// victim remains the independently dying object.
pub fn parse_delayed_dies_after_damage_by_previous_creature_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedDiesAfterDamageByPreviousCreatureShape<'_>> {
    let (victim_tokens, ()) =
        primitives::split_lexed_once_before_suffix(trimmed(tokens), 1, || {
            (
                semantic_phrase(&["dealt", "damage", "by", "that", "creature", "dies"]),
                eof,
            )
                .void()
        })?;
    let victim_tokens = trimmed(victim_tokens);
    (!victim_tokens.is_empty())
        .then_some(DelayedDiesAfterDamageByPreviousCreatureShape { victim_tokens })
}

/// Parse "target <subject> deals combat damage to <recipient>" for a delayed
/// this-turn registration. The target is chosen while the enclosing ability
/// resolves; the trigger must watch that exact object.
pub fn parse_delayed_target_deals_combat_damage_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedTargetCombatDamageShape<'_>> {
    let tokens = trimmed(tokens);
    let (_, after_target) = primitives::parse_prefix(tokens, primitives::kw("target"))?;
    let (subject_tokens, recipient_tokens) =
        primitives::split_lexed_once_on_separator(after_target, || {
            primitives::phrase(&["deals", "combat", "damage", "to"]).void()
        })?;
    let subject_tokens = trimmed(subject_tokens);
    let recipient_tokens = trimmed(recipient_tokens);
    (!subject_tokens.is_empty() && !recipient_tokens.is_empty()).then_some(
        DelayedTargetCombatDamageShape {
            subject_tokens,
            recipient_tokens,
        },
    )
}

#[cfg(test)]
#[path = "delayed_sentence_shapes_inline_tests.rs"]
mod tests;

#[path = "delayed_sentence_shapes/trigger.rs"]
mod trigger_programs;
pub use trigger_programs::{
    is_delayed_prior_object_put_into_a_graveyard, parse_delayed_dies_shape,
    parse_delayed_target_put_into_your_graveyard_subject,
};
#[path = "delayed_sentence_shapes/core.rs"]
mod core_programs;
use core_programs::dies_this_way_suffix;
#[path = "delayed_sentence_shapes/counter.rs"]
mod counter_programs;
pub use counter_programs::{
    delayed_trigger_has_first_time_marker, delayed_trigger_has_next_marker,
    is_next_cast_spell_or_loyalty_shape,
};
#[path = "delayed_sentence_shapes/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::parse_copy_twice_shape;
