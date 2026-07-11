use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken};
use crate::types::CardType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceLinkedExileReferenceKind {
    Permanent,
    CardType(CardType),
}

/// "Each player turns face up all cards they own exiled with this [source],
/// then puts all permanent cards among them onto the battlefield."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RevealSourceExiledPermanentsShape {
    pub(crate) source_kind: SourceLinkedExileReferenceKind,
}

fn source_reference<'a>(input: &mut LexStream<'a>) -> WResult<SourceLinkedExileReferenceKind> {
    primitives::kw("this").parse_next(input)?;
    alt((
        primitives::kw("permanent").value(SourceLinkedExileReferenceKind::Permanent),
        primitives::kw("object").value(SourceLinkedExileReferenceKind::Permanent),
        primitives::kw("artifact")
            .value(SourceLinkedExileReferenceKind::CardType(CardType::Artifact)),
        primitives::kw("creature")
            .value(SourceLinkedExileReferenceKind::CardType(CardType::Creature)),
        primitives::kw("enchantment").value(SourceLinkedExileReferenceKind::CardType(
            CardType::Enchantment,
        )),
        primitives::kw("land").value(SourceLinkedExileReferenceKind::CardType(CardType::Land)),
        primitives::kw("planeswalker").value(SourceLinkedExileReferenceKind::CardType(
            CardType::Planeswalker,
        )),
        primitives::kw("battle").value(SourceLinkedExileReferenceKind::CardType(CardType::Battle)),
    ))
    .parse_next(input)
}

fn reveal_source_exiled_permanents<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RevealSourceExiledPermanentsShape> {
    primitives::phrase(&[
        "each", "player", "turns", "face", "up", "all", "cards", "they", "own", "exiled", "with",
    ])
    .parse_next(input)?;
    let source_kind = source_reference.parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "then",
        "puts",
        "all",
        "permanent",
        "cards",
        "among",
        "them",
        "onto",
        "the",
        "battlefield",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RevealSourceExiledPermanentsShape { source_kind })
}

pub(crate) fn parse_reveal_source_exiled_permanents_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RevealSourceExiledPermanentsShape> {
    primitives::parse_all(
        tokens,
        reveal_source_exiled_permanents,
        "reveal source-exiled permanents",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_each_player_source_linked_exile_sequence() {
        let tokens = lex_line(
            "Each player turns face up all cards they own exiled with this artifact, then puts all permanent cards among them onto the battlefield.",
            0,
        )
        .unwrap();

        assert_eq!(
            parse_reveal_source_exiled_permanents_tokens(&tokens),
            Some(RevealSourceExiledPermanentsShape {
                source_kind: SourceLinkedExileReferenceKind::CardType(CardType::Artifact),
            })
        );
    }
}
