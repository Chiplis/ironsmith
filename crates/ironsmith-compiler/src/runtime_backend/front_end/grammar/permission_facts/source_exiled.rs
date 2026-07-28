//! Typed facts for spells cast from a source-linked exiled-card pool.

use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken};

use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceExiledSpellKind {
    Any,
    Creature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceExiledReference {
    pub(crate) surface: ironsmith_core::SourceReferenceSurface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellFromSourceExiledFact<'a> {
    pub(crate) kind: SourceExiledSpellKind,
    pub(crate) reference: SourceExiledReference,
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_spell_from_source_exiled_tokens(
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

fn parse_spell_from_source_exiled_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(SourceExiledSpellKind, SourceExiledReference)> {
    primitives::kw("a").parse_next(input)?;
    let kind = alt((
        primitives::phrase(&["creature", "spell"]).value(SourceExiledSpellKind::Creature),
        primitives::kw("spell").value(SourceExiledSpellKind::Any),
    ))
    .parse_next(input)?;
    primitives::phrase(&["from", "among", "cards", "exiled", "with", "this"]).parse_next(input)?;
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
        kind,
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
    use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

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
}
