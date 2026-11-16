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
    let condition = retarget_pronoun_characteristic_to_consequence_object(condition, if_true);
    let Condition::TaggedObjectMatches(tag, mut filter) = condition else {
        return condition;
    };
    let [consequence] = if_true else {
        return Condition::TaggedObjectMatches(tag, filter);
    };
    let Some(spec) = consequence.target_spec() else {
        return Condition::TaggedObjectMatches(tag, filter);
    };
    if tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        && let ChooseSpec::Tagged(consequence_tag) = spec.base()
    {
        // A definite description can already have resolved to one announced
        // target. Test that member instead of the complete chosen pair.
        return Condition::TaggedObjectMatches(consequence_tag.clone(), filter);
    }
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

/// "Mill two cards, then destroy the chosen permanent if its mana value is
/// less than or equal to the number of cards in your graveyard" (Quag Feast):
/// the postcondition's "its" names the object the gated action acts on, not
/// the most recently remembered set (the milled cards).
fn retarget_pronoun_characteristic_to_consequence_object(
    condition: Condition,
    if_true: &[Effect],
) -> Condition {
    let Condition::ValueComparison {
        left,
        operator,
        right,
    } = condition
    else {
        return condition;
    };
    let consequence_tag = match if_true {
        [consequence] => consequence.target_spec().and_then(|spec| match spec.base() {
            ChooseSpec::Tagged(tag) => Some(tag.clone()),
            _ => None,
        }),
        _ => None,
    };
    let left = match consequence_tag {
        Some(tag) => retarget_pronoun_characteristic(left, &tag),
        None => left,
    };
    Condition::ValueComparison {
        left,
        operator,
        right,
    }
}

fn retarget_pronoun_characteristic(value: Value, consequence_tag: &TagKey) -> Value {
    let (spec, rebuild): (Box<ChooseSpec>, fn(Box<ChooseSpec>) -> Value) = match value {
        Value::PowerOf(spec) => (spec, Value::PowerOf),
        Value::ToughnessOf(spec) => (spec, Value::ToughnessOf),
        Value::ManaValueOf(spec) => (spec, Value::ManaValueOf),
        other => return other,
    };
    let names_it = matches!(
        spec.source_reference_surface(),
        Some(ironsmith_core::SourceReferenceSurface::ThisPermanentType(text)) if text == "it"
    );
    let retarget = names_it
        && matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag != consequence_tag);
    if !retarget {
        return rebuild(spec);
    }
    let hints = spec.surface_hints().to_vec();
    rebuild(Box::new(ChooseSpec::SurfaceHinted {
        spec: Box::new(ChooseSpec::Tagged(consequence_tag.clone())),
        hints,
    }))
}
