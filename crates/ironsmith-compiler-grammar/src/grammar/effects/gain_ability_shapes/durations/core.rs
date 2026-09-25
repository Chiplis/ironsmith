use super::*;

pub(super) fn continuous_duration(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    alt((
        simple_turn_duration,
        source_remains_on_battlefield,
        affected_object_tapped,
    ))
    .parse_next(input)
}

fn affected_object_tapped(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    for_as_long_as.parse_next(input)?;
    primitives::word_slice_exact("it").parse_next(input)?;
    primitives::word_slice_exact("remains").parse_next(input)?;
    primitives::word_slice_exact("tapped").parse_next(input)?;
    Ok(Until::ForAsLongAs(
        ironsmith_core::ContinuousDurationPredicate::ObjectTapped(
            ironsmith_core::ContinuousDurationObject::AffectedObject,
        ),
    ))
}
