use super::*;

pub(super) fn parse_card_word<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("card"),
        primitives::word_slice_exact("cards"),
    ))
    .void()
    .parse_next(input)
}
