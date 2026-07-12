use crate::cards::builders::PlayerAst;
use crate::target::PlayerFilter;

use super::super::super::lexer::{LexStream, LexedClause, OwnedLexToken};
use super::super::permission_shapes;
pub(crate) use super::super::permission_shapes::{
    PermissionAtom as EffectAtom, PermissionCaptureKind as EffectCaptureKind,
    PermissionCaptureRole as EffectCaptureRole, PermissionSequence as EffectSequence,
};
use super::super::permission_shapes::{
    PermissionCaptureKind, PermissionCaptureRole, PermissionSequence,
};
use super::super::primitives;
use winnow::Parser as _;
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult};
use winnow::prelude::*;
use winnow::token::any;

#[path = "generic_program_shapes/voting.rs"]
mod voting;
pub(crate) use voting::*;

#[path = "generic_program_shapes/choice_complements.rs"]
mod choice_complements;
#[path = "generic_program_shapes/semantic_sequences.rs"]
mod semantic_sequences;
#[path = "generic_program_shapes/triggering_spell_damage.rs"]
mod triggering_spell_damage;

pub(crate) use choice_complements::*;
pub(crate) use semantic_sequences::*;
pub(crate) use triggering_spell_damage::*;

#[derive(Debug, Clone)]
pub(crate) struct AnyPlayerSourceDamageShape<'a> {
    pub(crate) player: PlayerAst,
    pub(crate) player_filter: PlayerFilter,
    pub(crate) damage_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_any_player_source_damage(
    tokens: &[OwnedLexToken],
) -> Option<AnyPlayerSourceDamageShape<'_>> {
    let atoms = [
        PermissionSequence::subject(
            "player",
            PermissionCaptureKind::OneOfPhrase(&[
                &["any", "opponent", "may", "have"],
                &["any", "player", "may", "have"],
            ]),
        ),
        PermissionSequence::capture(
            "source",
            PermissionCaptureKind::UntilAnyPhrase(&[&["deal"], &["deals"]]),
        ),
        PermissionSequence::action("deal", PermissionCaptureKind::OneOf(&["deal", "deals"])),
        PermissionSequence::tail("damage", PermissionCaptureKind::Rest),
    ];
    let clause = LexedClause::new(tokens).trimmed();
    let parsed = PermissionSequence::new(&atoms).parse_full(clause)?;
    let source = parsed.capture_clause("source", clause)?;
    if source.word_refs().is_empty() {
        return None;
    }
    let player_clause = parsed.capture_clause_by_role(PermissionCaptureRole::Subject, clause)?;
    let (player, player_filter) = if permission_shapes::exact_tokens(
        player_clause.tokens(),
        &["any", "opponent", "may", "have"],
    ) {
        (PlayerAst::Opponent, PlayerFilter::Opponent)
    } else {
        (PlayerAst::Any, PlayerFilter::Any)
    };
    let damage = parsed.capture_clause_by_role(PermissionCaptureRole::Tail, clause)?;
    Some(AnyPlayerSourceDamageShape {
        player,
        player_filter,
        damage_tokens: damage.tokens(),
    })
}

pub(crate) fn parse_choice_complement_clause(tokens: &[OwnedLexToken]) -> Option<LexedClause<'_>> {
    let atoms = [
        PermissionSequence::phrase(&["each", "player"]),
        PermissionSequence::action(
            "choose",
            PermissionCaptureKind::OneOf(&["choose", "chooses"]),
        ),
        PermissionSequence::object("choice", PermissionCaptureKind::UntilPhrase(&["then"])),
        PermissionSequence::word("then"),
        PermissionSequence::action(
            "sacrifice",
            PermissionCaptureKind::OneOf(&["sacrifice", "sacrifices"]),
        ),
        PermissionSequence::phrase(&["the", "rest"]),
    ];
    let clause = LexedClause::new(tokens).trimmed();
    let parsed = PermissionSequence::new(&atoms).parse_full(clause)?;
    parsed
        .capture_clause_by_role(PermissionCaptureRole::Object, clause)
        .map(LexedClause::trimmed)
}

pub(crate) fn exact_any_words(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

pub(crate) fn prefix_words(words: &[&str], expected: &[&str]) -> bool {
    permission_shapes::prefix_words(words, expected)
}

pub(crate) fn exact_any_tokens(tokens: &[OwnedLexToken], alternatives: &[&[&str]]) -> bool {
    permission_shapes::exact_tokens_any(tokens, alternatives)
}

#[cfg(test)]
#[path = "generic_program_shapes/tests.rs"]
mod tests;
