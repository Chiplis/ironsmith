use winnow::Parser;
use winnow::combinator::alt;

use crate::grammar::primitives;
use crate::lexer::OwnedLexToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyCardReferenceShape {
    It,
    That,
    ThatCard,
}

pub fn parse_copy_card_reference_shape(tokens: &[OwnedLexToken]) -> Option<CopyCardReferenceShape> {
    primitives::parse_all(
        tokens,
        (
            primitives::kw("copy"),
            alt((
                primitives::kw("it").value(CopyCardReferenceShape::It),
                primitives::phrase(&["that", "card"]).value(CopyCardReferenceShape::ThatCard),
                primitives::kw("that").value(CopyCardReferenceShape::That),
            )),
            primitives::sentence_end(),
        )
            .map(|(_, shape, ())| shape),
        "copy card reference",
    )
    .ok()
}

pub fn parse_atomic_token_copy_exception_shape(tokens: &[OwnedLexToken]) -> bool {
    if primitives::parse_prefix(
        tokens,
        alt((primitives::kw("create"), primitives::kw("creates"))).void(),
    )
    .is_none()
    {
        return false;
    }
    [
        &["token", "tokens"][..],
        &["copy", "copies"][..],
        &["except"][..],
    ]
    .into_iter()
    .all(|words| {
        let alternatives = [words];
        primitives::find_prefix(tokens, || primitives::any_phrase(&alternatives).void()).is_some()
    })
}
