use crate::cards::builders::PlayerAst;
use crate::target::PlayerFilter;

use super::super::super::lexer::{LexStream, LexedClause, OwnedLexToken};
use super::super::permission_shapes;
pub use super::super::permission_shapes::{
    PermissionAtom as EffectAtom, PermissionCaptureKind as EffectCaptureKind,
    PermissionCaptureRole as EffectCaptureRole, PermissionSequence as EffectSequence,
};
use super::super::permission_shapes::{
    PermissionCaptureKind, PermissionCaptureRole, PermissionSequence,
};
use super::super::primitives;
use winnow::Parser as _;
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult;
use winnow::token::any;

#[path = "generic_program_shapes/voting.rs"]
mod voting;
pub use voting::*;

#[path = "generic_program_shapes/choice_complements.rs"]
mod choice_complements;
#[path = "generic_program_shapes/semantic_sequences.rs"]
mod semantic_sequences;
#[path = "generic_program_shapes/triggering_spell_damage.rs"]
mod triggering_spell_damage;

pub use choice_complements::*;
pub use semantic_sequences::*;
pub use triggering_spell_damage::*;

#[derive(Debug, Clone)]
pub struct AnyPlayerSourceDamageShape<'a> {
    pub player: PlayerAst,
    pub player_filter: PlayerFilter,
    pub damage_tokens: &'a [OwnedLexToken],
}

pub fn parse_any_player_source_damage(
    tokens: &[OwnedLexToken],
) -> Option<AnyPlayerSourceDamageShape<'_>> {
    let atoms = [
        PermissionSequence::subject(
            "player",
            PermissionCaptureKind::OneOfPhrase(&[
                &["any", "opponent", "may", "have"],
                &["any", "player", "may", "have"],
                &["target", "opponent", "may", "have"],
                &["target", "player", "may", "have"],
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
    let (player, player_filter) = match player_clause.word_refs().as_slice() {
        ["any", "opponent", "may", "have"] => (PlayerAst::Opponent, PlayerFilter::Opponent),
        ["any", "player", "may", "have"] => (PlayerAst::Any, PlayerFilter::Any),
        ["target", "opponent", "may", "have"] => {
            (PlayerAst::TargetOpponent, PlayerFilter::target_opponent())
        }
        ["target", "player", "may", "have"] => (PlayerAst::Target, PlayerFilter::target_player()),
        _ => return None,
    };
    let damage = parsed.capture_clause_by_role(PermissionCaptureRole::Tail, clause)?;
    Some(AnyPlayerSourceDamageShape {
        player,
        player_filter,
        damage_tokens: damage.tokens(),
    })
}

/// A source-subject damage clause whose recipient is the player currently
/// making a surrounding choice: `<source> deal N damage to them`.
#[derive(Debug, Clone)]
pub struct SourceDamageToDeciderShape<'a> {
    pub damage_tokens: &'a [OwnedLexToken],
}

pub fn parse_source_damage_to_decider(
    tokens: &[OwnedLexToken],
) -> Option<SourceDamageToDeciderShape<'_>> {
    let atoms = [
        PermissionSequence::capture(
            "source",
            PermissionCaptureKind::UntilAnyPhrase(&[&["deal"], &["deals"]]),
        ),
        PermissionSequence::action("deal", PermissionCaptureKind::OneOf(&["deal", "deals"])),
        PermissionSequence::tail("damage", PermissionCaptureKind::Rest),
    ];
    let clause = LexedClause::new(tokens).trimmed();
    let parsed = PermissionSequence::new(&atoms).parse_full(clause)?;
    if parsed
        .capture_clause("source", clause)?
        .word_refs()
        .is_empty()
    {
        return None;
    }
    let damage = parsed.capture_clause_by_role(PermissionCaptureRole::Tail, clause)?;
    let words = damage.word_refs();
    let recipient_is_decider = crate::word_primitives::parse_any_sequence_suffix(
        &words,
        &[&["to", "them"], &["to", "that", "player"]],
    );
    recipient_is_decider.then_some(SourceDamageToDeciderShape {
        damage_tokens: damage.tokens(),
    })
}

pub fn parse_choice_complement_clause(tokens: &[OwnedLexToken]) -> Option<LexedClause<'_>> {
    let then_atoms = [
        PermissionSequence::any_phrase(&[&["each", "player"], &["each", "opponent"]]),
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
    if let Some(parsed) = PermissionSequence::new(&then_atoms).parse_full(clause) {
        return parsed
            .capture_clause_by_role(PermissionCaptureRole::Object, clause)
            .map(LexedClause::trimmed);
    }

    // Participant-scoped Oracle commonly coordinates the same operation with
    // "and" ("chooses five lands they control and sacrifices the rest").
    // Only try this surface when no `then` exists, so an `and` inside a
    // multi-slot choice list is never mistaken for the action boundary.
    if tokens.iter().any(|token| token.is_word("then")) {
        return None;
    }
    let and_atoms = [
        PermissionSequence::any_phrase(&[&["each", "player"], &["each", "opponent"]]),
        PermissionSequence::action(
            "choose",
            PermissionCaptureKind::OneOf(&["choose", "chooses"]),
        ),
        PermissionSequence::object("choice", PermissionCaptureKind::UntilPhrase(&["and"])),
        PermissionSequence::word("and"),
        PermissionSequence::action(
            "sacrifice",
            PermissionCaptureKind::OneOf(&["sacrifice", "sacrifices"]),
        ),
        PermissionSequence::phrase(&["the", "rest"]),
    ];
    let parsed = PermissionSequence::new(&and_atoms).parse_full(clause)?;
    parsed
        .capture_clause_by_role(PermissionCaptureRole::Object, clause)
        .map(LexedClause::trimmed)
}

pub fn exact_any_words(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

pub fn prefix_words(words: &[&str], expected: &[&str]) -> bool {
    permission_shapes::prefix_words(words, expected)
}

pub fn exact_any_tokens(tokens: &[OwnedLexToken], alternatives: &[&[&str]]) -> bool {
    permission_shapes::exact_tokens_any(tokens, alternatives)
}

#[cfg(test)]
#[path = "generic_program_shapes/tests.rs"]
mod tests;
