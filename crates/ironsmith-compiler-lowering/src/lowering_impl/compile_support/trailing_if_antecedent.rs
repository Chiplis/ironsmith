//! "Put a +1/+1 counter on the creature you control if it has power 4 or
//! greater" (Hog-Monkey Rampage): a postcondition's "it" names the object the
//! consequence acts on. When that object is a qualified member of a broader
//! remembered set ("the creature you control" among both chosen creatures),
//! the condition must test that member, not "any member of the set".
use super::*;

pub(super) fn narrow_trailing_condition_to_consequence_object(
    condition: Condition,
    if_true: &[Effect],
) -> Condition {
    let Condition::TaggedObjectMatches(tag, mut filter) = condition else {
        return condition;
    };
    let [consequence] = if_true else {
        return Condition::TaggedObjectMatches(tag, filter);
    };
    let Some(spec) = consequence.target_spec() else {
        return Condition::TaggedObjectMatches(tag, filter);
    };
    let (ChooseSpec::Object(object) | ChooseSpec::All(object)) = spec.base() else {
        return Condition::TaggedObjectMatches(tag, filter);
    };
    let references_tag = object.tagged_constraints.iter().any(|constraint| {
        constraint.tag == tag
            && matches!(
                constraint.relation,
                TaggedOpbjectRelation::IsTaggedObject | TaggedOpbjectRelation::SameObjectId
            )
    });
    if !references_tag {
        return Condition::TaggedObjectMatches(tag, filter);
    }
    if filter.controller.is_none() {
        filter.controller = object.controller.clone();
    }
    if filter.card_types.is_empty() {
        filter.card_types = object.card_types.clone();
    }
    Condition::TaggedObjectMatches(tag, filter)
}
