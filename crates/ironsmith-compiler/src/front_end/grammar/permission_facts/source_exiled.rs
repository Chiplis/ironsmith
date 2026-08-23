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

pub fn parse_spell_from_source_exiled_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SpellFromSourceExiledFact<'_>> {
    let ((kind, reference), tail_tokens) =
        primitives::parse_prefix(tokens, parse_spell_from_source_exiled_lexed)?;
    Some(SpellFromSourceExiledFact {
        kind,
        reference,
        tail_tokens,
    })
}

pub fn parse_spells_from_source_exiled_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SpellsFromSourceExiledFact<'_>> {
    let (scope_start, _, after_cards) =
        primitives::find_prefix(tokens, || primitives::phrase(&["from", "among", "cards"]))?;
    let subject_tokens = trim_lexed_commas(&tokens[..scope_start]);
    if subject_tokens.is_empty() {
        return None;
    }
    let ((owned_by_you, reference), tail_tokens) =
        primitives::parse_prefix(after_cards, parse_source_exiled_tail_lexed)?;
    Some(SpellsFromSourceExiledFact {
        subject_tokens,
        owned_by_you,
        reference,
        tail_tokens,
    })
}

fn parse_spell_from_source_exiled_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(SourceExiledSpellKind, SourceExiledReference)> {
    primitives::kw("a").parse_next(input)?;
    let kind = alt((
        primitives::phrase(&["creature", "spell"]).value(SourceExiledSpellKind::Creature),
        primitives::kw("spell").value(SourceExiledSpellKind::Any),
    ))
    .parse_next(input)?;
    primitives::phrase(&["from", "among", "cards"]).parse_next(input)?;
    let (_, reference) = parse_source_exiled_tail_lexed.parse_next(input)?;
    Ok((kind, reference))
}

fn parse_source_exiled_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(bool, SourceExiledReference)> {
    let owned_by_you = opt(primitives::phrase(&["you", "own"]))
        .parse_next(input)?
        .is_some();
    primitives::phrase(&["exiled", "with", "this"]).parse_next(input)?;
    let source_kind = alt((
        primitives::kw("enchantment").value("enchantment"),
        primitives::kw("artifact").value("artifact"),
        primitives::kw("creature").value("creature"),
        primitives::kw("permanent").value("permanent"),
        primitives::kw("card").value("card"),
        primitives::kw("land").value("land"),
    ))
    .parse_next(input)?;
    Ok((
        owned_by_you,
        SourceExiledReference {
            surface: ironsmith_core::SourceReferenceSurface::ThisPermanentType(format!(
                "this {source_kind}"
            )),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{TokenWordView, lex_line};

    #[test]
    fn source_exiled_spell_fact_is_typed_and_preserves_tail() {
        let tokens = lex_line(
            "a creature spell from among cards exiled with this enchantment this turn",
            0,
        )
        .unwrap();
        let parsed = parse_spell_from_source_exiled_tokens(&tokens).unwrap();
        assert_eq!(parsed.kind, SourceExiledSpellKind::Creature);
        assert_eq!(
            TokenWordView::new(parsed.tail_tokens).word_refs(),
            ["this", "turn"]
        );
        assert_eq!(
            parsed.reference.surface,
            ironsmith_core::SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string()
            )
        );
    }

    #[test]
    fn plural_source_exiled_spell_fact_preserves_filter_owner_and_source_surface() {
        let tokens = lex_line(
            "Dinosaur creature spells from among cards you own exiled with this creature this turn",
            0,
        )
        .unwrap();
        let parsed = parse_spells_from_source_exiled_tokens(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(parsed.subject_tokens).word_refs(),
            ["Dinosaur", "creature", "spells"]
        );
        assert!(parsed.owned_by_you);
        assert_eq!(
            TokenWordView::new(parsed.tail_tokens).word_refs(),
            ["this", "turn"]
        );
        assert_eq!(
            parsed.reference.surface,
            ironsmith_core::SourceReferenceSurface::ThisPermanentType("this creature".to_string())
        );
    }
}
