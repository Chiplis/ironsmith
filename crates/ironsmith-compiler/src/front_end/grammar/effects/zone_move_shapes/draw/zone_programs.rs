use super::*;

pub fn same_name_graveyard_count_value() -> Value {
    Value::Count(
        crate::target::ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .match_tagged(
                crate::tag::CompilerReferenceTag::Triggering.key(),
                TaggedOpbjectRelation::SameNameAsTagged,
            ),
    )
}
