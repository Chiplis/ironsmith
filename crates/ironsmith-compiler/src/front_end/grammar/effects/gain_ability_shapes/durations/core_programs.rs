use super::*;

pub(super) fn continuous_duration(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    alt((simple_turn_duration, source_remains_on_battlefield)).parse_next(input)
}
