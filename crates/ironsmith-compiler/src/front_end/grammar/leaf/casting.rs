use winnow::combinator::alt;
use winnow::error::{ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;

use crate::filter::AlternativeCastKind;

use super::super::primitives::{self, WordSliceInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeafAlternativeCastPrefix {
    pub kind: AlternativeCastKind,
    pub consumed: usize,
}

pub fn parse_leaf_alternative_cast_prefix_words(
    words: &[&str],
) -> Option<LeafAlternativeCastPrefix> {
    let mut input: WordSliceInput<'_> = words;
    let Ok(kind) = parse_leaf_alternative_cast_kind.parse_next(&mut input) else {
        return None;
    };
    Some(LeafAlternativeCastPrefix {
        kind,
        consumed: words.len().checked_sub(input.len())?,
    })
}

fn parse_leaf_alternative_cast_kind(
    input: &mut WordSliceInput<'_>,
) -> WResult<AlternativeCastKind> {
    alt((
        (
            primitives::word_slice_exact("jump"),
            primitives::word_slice_exact("start"),
        )
            .value(AlternativeCastKind::JumpStart),
        primitives::word_slice_exact("jumpstart").value(AlternativeCastKind::JumpStart),
        primitives::word_slice_exact("flashback").value(AlternativeCastKind::Flashback),
        primitives::word_slice_exact("suspend").value(AlternativeCastKind::Suspend),
        primitives::word_slice_exact("madness").value(AlternativeCastKind::Madness),
        primitives::word_slice_exact("miracle").value(AlternativeCastKind::Miracle),
        primitives::word_slice_exact("escape").value(AlternativeCastKind::Escape),
        primitives::word_slice_exact("blitz").value(AlternativeCastKind::Blitz),
        primitives::word_slice_exact("dash").value(AlternativeCastKind::Dash),
    ))
    .context(StrContext::Label("alternative-cast kind"))
    .context(StrContext::Expected(StrContextValue::Description(
        "named alternative-casting mechanic",
    )))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_jump_start_surface_consumes_two_words() {
        assert_eq!(
            parse_leaf_alternative_cast_prefix_words(&["jump", "start", "cost"]),
            Some(LeafAlternativeCastPrefix {
                kind: AlternativeCastKind::JumpStart,
                consumed: 2,
            })
        );
    }

    #[test]
    fn compact_and_single_word_surfaces_are_typed() {
        assert_eq!(
            parse_leaf_alternative_cast_prefix_words(&["jumpstart"])
                .unwrap()
                .kind,
            AlternativeCastKind::JumpStart
        );
        assert_eq!(
            parse_leaf_alternative_cast_prefix_words(&["flashback", "three"])
                .unwrap()
                .consumed,
            1
        );
        assert!(parse_leaf_alternative_cast_prefix_words(&["kicker"]).is_none());
    }
}
