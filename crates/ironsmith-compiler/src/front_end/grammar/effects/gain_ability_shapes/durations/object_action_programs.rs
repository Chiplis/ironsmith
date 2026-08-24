use super::*;

pub(super) fn you_control(input: &mut WordSliceInput<'_>) -> WResult<()> {
    (
        for_as_long_as,
        primitives::word_slice_exact("you"),
        primitives::word_slice_exact("control"),
    )
        .void()
        .parse_next(input)
}
