use super::*;

pub(super) fn continuous_duration(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    alt((
        source_cast_from_exile,
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


fn source_cast_from_exile(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    for word in ["until", "this", "card", "is", "cast", "from", "exile"] {
        primitives::word_slice_exact(word).parse_next(input)?;
    }
    Ok(Until::ObjectIsCast {
        object: ironsmith_core::ContinuousDurationObject::Tagged(
            crate::tag::CompilerReferenceTag::SourceExiledSelf.bind().into()),
        from_zone: crate::zone::Zone::Exile,
    })
}
