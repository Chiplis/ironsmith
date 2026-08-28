use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::{leaf, primitives};
use crate::lexer::{LexStream, LexedClause, OwnedLexToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageBackReferenceShape {
    Itself,
    ThatPlayerOrPlaneswalker,
    ThatObject,
    ThatObjectController,
}

fn demonstrative_object_head<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| {
        let Some(word) = token.as_word() else {
            return false;
        };
        let word = leaf::strip_leaf_source_possessive_suffix(word);
        leaf::parse_leaf_demonstrative_object_head_complete(word).is_ok()
            || word.as_bytes().last().is_some_and(|byte| *byte == b's')
                && word
                    .get(..word.len().saturating_sub(1))
                    .is_some_and(|singular| {
                        leaf::parse_leaf_demonstrative_object_head_complete(singular).is_ok()
                    })
    })
    .void()
    .parse_next(input)
}

pub fn parse_damage_back_reference_shape(
    tokens: &[OwnedLexToken],
) -> Option<DamageBackReferenceShape> {
    let tokens = LexedClause::new(tokens).trimmed();
    let parser = alt((
        primitives::kw("itself").value(DamageBackReferenceShape::Itself),
        primitives::phrase(&["that", "player", "or", "planeswalker"])
            .value(DamageBackReferenceShape::ThatPlayerOrPlaneswalker),
        (
            primitives::kw("that"),
            demonstrative_object_head,
            primitives::kw("controller"),
        )
            .value(DamageBackReferenceShape::ThatObjectController),
        (primitives::kw("that"), demonstrative_object_head)
            .value(DamageBackReferenceShape::ThatObject),
    ));
    primitives::parse_all(
        tokens.tokens(),
        (parser, primitives::sentence_end()).map(|(shape, ())| shape),
        "damage back-reference",
    )
    .ok()
}
