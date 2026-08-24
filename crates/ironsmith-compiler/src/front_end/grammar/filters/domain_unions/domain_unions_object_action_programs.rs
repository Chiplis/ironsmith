use super::*;

pub(super) fn propagate_trailing_shared_attachment_scope(branches: &mut [ObjectFilter]) {
    let Some((last, preceding)) = branches.split_last_mut() else {
        return;
    };
    let [constraint] = last.tagged_constraints.as_slice() else {
        return;
    };
    if !matches!(
        constraint.relation,
        TaggedOpbjectRelation::AttachedToTaggedObject
            | TaggedOpbjectRelation::WasAttachedToTaggedObject
    ) || preceding
        .iter()
        .any(|branch| !branch.tagged_constraints.is_empty())
    {
        return;
    }

    for branch in preceding {
        branch.tagged_constraints.push(constraint.clone());
    }
}
