use super::*;

pub(super) fn parse_graveyard_word<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("graveyard"),
        primitives::word_slice_exact("graveyards"),
    ))
    .void()
    .parse_next(input)
}

pub(super) fn parse_hand_word<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("hand"),
        primitives::word_slice_exact("hands"),
    ))
    .void()
    .parse_next(input)
}
