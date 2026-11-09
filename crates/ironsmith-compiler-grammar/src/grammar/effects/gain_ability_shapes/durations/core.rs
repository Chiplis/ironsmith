use super::*;

pub(super) fn continuous_duration(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    alt((
        source_cast_from_exile,
        simple_turn_duration,
        source_remains_on_battlefield,
        affected_object_tapped,
        beginning_of_your_next_end_step,
    ))
    .parse_next(input)
}

/// "until the beginning of your next end step" (Fhinon, the Dirgechanter).
fn beginning_of_your_next_end_step(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    for word in ["until", "the", "beginning", "of", "your", "next", "end", "step"] {
        primitives::word_slice_exact(word).parse_next(input)?;
    }
    Ok(Until::YourNextEndStep)
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

fn duration_reference(input: &mut WordSliceInput<'_>) -> WResult<ironsmith_core::ContinuousDurationObject> {
    use ironsmith_core::ContinuousDurationObject as Object;
    use crate::tag::CompilerReferenceTag as Reference;
    alt((
        (primitives::word_slice_exact("this"),primitives::word_slice_exact("card")).value(Object::SourceInZone(crate::zone::Zone::Exile)),
        (primitives::word_slice_exact("the"),primitives::word_slice_exact("iterated"),alt((primitives::word_slice_exact("card"),primitives::word_slice_exact("object"))))
            .value(Object::Tagged(Reference::CurrentIteratedObject.key())),
        (primitives::word_slice_exact("that"),primitives::word_slice_exact("creature")).value(Object::Tagged(Reference::DurationReferencedCreature.key())),
        (primitives::word_slice_exact("that"),primitives::word_slice_exact("artifact")).value(Object::Tagged(Reference::DurationReferencedArtifact.key())),
        (primitives::word_slice_exact("that"),primitives::word_slice_exact("permanent")).value(Object::Tagged(Reference::DurationReferencedPermanent.key())),
    )).parse_next(input)
}
fn duration_zone(input: &mut WordSliceInput<'_>) -> WResult<crate::zone::Zone> {
    alt((
        primitives::word_slice_exact("exiled").value(crate::zone::Zone::Exile),
        (primitives::word_slice_exact("in"),primitives::word_slice_exact("exile")).value(crate::zone::Zone::Exile),
        (primitives::word_slice_exact("on"),opt(primitives::word_slice_exact("the")),primitives::word_slice_exact("battlefield")).value(crate::zone::Zone::Battlefield),
    )).parse_next(input)
}
fn duration_zone_atom(input: &mut WordSliceInput<'_>) -> WResult<ironsmith_core::ContinuousDurationPredicate> {
    let object=duration_reference.parse_next(input)?;
    alt((primitives::word_slice_exact("remains"),primitives::word_slice_exact("is"))).parse_next(input)?;
    let zone=duration_zone.parse_next(input)?;
    // Existing source-BF duration keeps its established exact-incarnation
    // owner. The new cross-zone source atom is only an explicit exile reference.
    if matches!(object,ironsmith_core::ContinuousDurationObject::SourceInZone(_)) && zone!=crate::zone::Zone::Exile {
        return Err(primitives::backtrack_err("source exile duration","explicit source remains exiled"));
    }
    Ok(ironsmith_core::ContinuousDurationPredicate::ObjectInZone{object,zone})
}
pub(super) fn object_zone_duration(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    for_as_long_as.parse_next(input)?;
    let mut predicates=vec![duration_zone_atom.parse_next(input)?];
    while input.first().copied()==Some("and") {
        primitives::word_slice_exact("and").parse_next(input)?;
        predicates.push(duration_zone_atom.parse_next(input)?);
    }
    Ok(Until::ForAsLongAs(ironsmith_core::ContinuousDurationPredicate::all(predicates)))
}

#[cfg(test)]
mod iteration_duration_reference_tests {
    use super::*;
    #[test]
    fn iterated_card_and_chosen_creature_are_independent_duration_operands() {
        let mut words:&[&str]=&["for","as","long","as","the","iterated","card","remains","exiled","and","that","creature","remains","on","the","battlefield"];
        let duration=object_zone_duration(&mut words).unwrap();assert!(words.is_empty());
        let Until::ForAsLongAs(ironsmith_core::ContinuousDurationPredicate::All(atoms))=duration else {panic!("conjoined duration")};
        assert!(matches!(&atoms[0],ironsmith_core::ContinuousDurationPredicate::ObjectInZone{object:ironsmith_core::ContinuousDurationObject::Tagged(tag),zone:crate::zone::Zone::Exile} if tag==&crate::tag::CompilerReferenceTag::CurrentIteratedObject.key()));
        assert!(matches!(&atoms[1],ironsmith_core::ContinuousDurationPredicate::ObjectInZone{object:ironsmith_core::ContinuousDurationObject::Tagged(tag),zone:crate::zone::Zone::Battlefield} if tag==&crate::tag::CompilerReferenceTag::DurationReferencedCreature.key()));
    }
}
