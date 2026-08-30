//! Typed facts for spells cast from a source-linked exiled-card pool.

use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};

use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceExiledSpellKind {
    Any,
    Creature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceExiledReference {
    pub surface: ironsmith_core::SourceReferenceSurface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellFromSourceExiledFact<'a> {
    pub kind: SourceExiledSpellKind,
    pub reference: SourceExiledReference,
    pub tail_tokens: &'a [OwnedLexToken],
}

/// A static permission whose castable set is a filtered plural spell subject
/// drawn from the cards linked to the source's exile pool.
///
/// This is distinct from [`SpellFromSourceExiledFact`]: singular wording is
/// commonly a one-shot tagged permission, while plural wording such as
/// "Dinosaur creature spells from among cards you own exiled with this
/// creature" describes a persistent grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellsFromSourceExiledFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub owned_by_you: bool,
    pub reference: SourceExiledReference,
    pub tail_tokens: &'a [OwnedLexToken],
}

#[cfg(test)]
#[path = "source_exiled_inline_tests.rs"]
mod tests;

#[path = "source_exiled/reference.rs"]
mod reference_programs;
use reference_programs::{parse_source_exiled_tail_lexed, parse_spell_from_source_exiled_lexed};
pub use reference_programs::{
    parse_spell_from_source_exiled_tokens, parse_spells_from_source_exiled_tokens,
};
