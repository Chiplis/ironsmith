use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::rest;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;
use super::subjects::{
    AttachedSubject, parse_attached_subject_lexed, semantic_finish, semantic_kw, semantic_phrase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedCombatRestrictionKind {
    CantAttack,
    CantBlock,
    CantAttackOrBlock,
    CantBeBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedCombatRestrictionSpec {
    pub subject: AttachedSubject,
    pub kind: AttachedCombatRestrictionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedCombatRestrictionGrantSpec<'a> {
    pub subject: AttachedSubject,
    pub subject_tokens: &'a [OwnedLexToken],
    pub kind: AttachedCombatRestrictionKind,
    pub ability_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedTapAbilitySubject {
    EnchantedCreature,
    EnchantedPermanent,
    EquippedCreature,
}

impl AttachedTapAbilitySubject {
    pub fn display(self) -> &'static str {
        match self {
            Self::EnchantedCreature => "enchanted creature",
            Self::EnchantedPermanent => "enchanted permanent",
            Self::EquippedCreature => "equipped creature",
        }
    }
}

pub fn parse_attached_combat_restriction_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedCombatRestrictionSpec> {
    primitives::parse_all(
        tokens,
        parse_attached_combat_restriction_lexed,
        "attached combat restriction",
    )
    .ok()
}

pub fn parse_attached_combat_restriction_grant_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedCombatRestrictionGrantSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_attached_combat_restriction_grant_lexed,
        "attached combat restriction and grant",
    )
    .ok()
}

pub fn parse_all_creatures_block_attached_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedSubject> {
    primitives::parse_all(
        tokens,
        parse_all_creatures_block_attached_lexed,
        "all creatures block attached object",
    )
    .ok()
}

pub fn parse_attached_tap_ability_restriction_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedTapAbilitySubject> {
    primitives::parse_all(
        tokens,
        parse_attached_tap_ability_restriction_lexed,
        "attached tap-ability restriction",
    )
    .ok()
}

pub fn parse_you_control_attached_tokens(tokens: &[OwnedLexToken]) -> Option<AttachedSubject> {
    primitives::parse_all(
        tokens,
        parse_you_control_attached_lexed,
        "control attached object",
    )
    .ok()
}

pub fn parse_attached_restriction_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedCombatRestrictionKind> {
    primitives::parse_all(
        tokens,
        (parse_attached_restriction_tail_lexed, semantic_finish).map(|(kind, ())| kind),
        "attached restriction tail",
    )
    .ok()
}

fn parse_attached_combat_restriction_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedCombatRestrictionSpec> {
    let subject = parse_attached_subject_lexed(input)?;
    if !matches!(
        subject,
        AttachedSubject::EnchantedCreature
            | AttachedSubject::EnchantedPermanent
            | AttachedSubject::EquippedCreature
    ) {
        return Err(primitives::backtrack_err(
            "attached restriction subject",
            "creature or permanent attachment subject",
        ));
    }
    let kind = parse_attached_restriction_tail_lexed(input)?;
    if kind == AttachedCombatRestrictionKind::CantBeBlocked {
        return Err(primitives::backtrack_err(
            "attached restriction",
            "attack or block restriction",
        ));
    }
    semantic_finish(input)?;
    Ok(AttachedCombatRestrictionSpec { subject, kind })
}

fn parse_attached_combat_restriction_grant_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedCombatRestrictionGrantSpec<'a>> {
    let (subject, subject_tokens) = parse_attached_subject_lexed
        .with_taken()
        .parse_next(input)?;
    if !matches!(
        subject,
        AttachedSubject::EnchantedCreature
            | AttachedSubject::EnchantedPermanent
            | AttachedSubject::EquippedCreature
    ) {
        return Err(primitives::backtrack_err(
            "attached restriction subject",
            "creature or permanent attachment subject",
        ));
    }
    let kind = parse_attached_restriction_tail_lexed(input)?;
    if kind == AttachedCombatRestrictionKind::CantBeBlocked {
        return Err(primitives::backtrack_err(
            "attached restriction",
            "attack or block restriction",
        ));
    }
    semantic_kw("and").parse_next(input)?;
    alt((semantic_kw("has"), semantic_kw("have"))).parse_next(input)?;
    let ability_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    if ability_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "attached restriction grant",
            "nonempty granted ability",
        ));
    }
    Ok(AttachedCombatRestrictionGrantSpec {
        subject,
        subject_tokens,
        kind,
        ability_tokens,
    })
}

fn parse_attached_restriction_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedCombatRestrictionKind> {
    semantic_kw("cant").parse_next(input)?;
    alt((
        semantic_phrase(&["attack", "or", "block"])
            .value(AttachedCombatRestrictionKind::CantAttackOrBlock),
        semantic_phrase(&["be", "blocked"]).value(AttachedCombatRestrictionKind::CantBeBlocked),
        semantic_kw("attack").value(AttachedCombatRestrictionKind::CantAttack),
        semantic_kw("block").value(AttachedCombatRestrictionKind::CantBlock),
    ))
    .parse_next(input)
}

fn parse_all_creatures_block_attached_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedSubject> {
    semantic_phrase(&["all", "creatures", "able", "to", "block"]).parse_next(input)?;
    let subject = parse_attached_subject_lexed(input)?;
    semantic_phrase(&["do", "so"]).parse_next(input)?;
    semantic_finish(input)?;
    Ok(subject)
}

fn parse_attached_tap_ability_restriction_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedTapAbilitySubject> {
    let subject = alt((
        (semantic_kw("enchanted"), semantic_kw("creatures"))
            .value(AttachedTapAbilitySubject::EnchantedCreature),
        (semantic_kw("enchanted"), semantic_kw("permanents"))
            .value(AttachedTapAbilitySubject::EnchantedPermanent),
        (semantic_kw("equipped"), semantic_kw("creatures"))
            .value(AttachedTapAbilitySubject::EquippedCreature),
    ))
    .parse_next(input)?;
    semantic_phrase(&[
        "activated",
        "abilities",
        "with",
        "t",
        "in",
        "their",
        "costs",
        "cant",
        "be",
        "activated",
    ])
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(subject)
}

fn parse_you_control_attached_lexed<'a>(input: &mut LexStream<'a>) -> WResult<AttachedSubject> {
    semantic_phrase(&["you", "control"]).parse_next(input)?;
    let subject = parse_attached_subject_lexed(input)?;
    semantic_finish(input)?;
    Ok(subject)
}

#[cfg(test)]
#[path = "restrictions_tests.rs"]
mod tests;
