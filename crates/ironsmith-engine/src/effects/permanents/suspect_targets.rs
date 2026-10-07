// Exact-object references for designation state transitions.

fn exact_designation_filter(
    filter: &crate::target::ObjectFilter,
    ctx: &ExecutionContext,
) -> Result<crate::target::ObjectFilter, ExecutionError> {
    let mut exact = filter.clone();
    for constraint in &mut exact.tagged_constraints {
        if matches!(constraint.relation,
            crate::target::TaggedOpbjectRelation::IsTaggedObject
                | crate::target::TaggedOpbjectRelation::SameObjectId)
        {
            if ctx.get_tagged_all(constraint.tag.as_str()).is_none() {
                return Err(ExecutionError::IncompleteEvidence(format!(
                    "designation change has no exact object reference {}", constraint.tag,
                )));
            }
            constraint.relation = crate::target::TaggedOpbjectRelation::SameObjectId;
        }
    }
    exact.any_of = exact.any_of.iter().map(|filter| exact_designation_filter(filter, ctx)).collect::<Result<_, _>>()?;
    Ok(exact)
}

fn exact_designation_spec(spec: &ChooseSpec, ctx: &ExecutionContext) -> Result<ChooseSpec, ExecutionError> {
    Ok(match spec {
        ChooseSpec::Tagged(tag) => ChooseSpec::All(exact_designation_filter(
            &crate::target::ObjectFilter::tagged(tag.clone()).in_zone(Zone::Battlefield), ctx,
        )?),
        ChooseSpec::Object(filter) => ChooseSpec::Object(exact_designation_filter(filter, ctx)?),
        ChooseSpec::All(filter) => ChooseSpec::All(exact_designation_filter(filter, ctx)?),
        ChooseSpec::SurfaceHinted { spec, hints } => ChooseSpec::SurfaceHinted {
            spec: Box::new(exact_designation_spec(spec, ctx)?), hints: hints.clone(),
        },
        ChooseSpec::Target(inner) => ChooseSpec::Target(Box::new(exact_designation_spec(inner, ctx)?)),
        ChooseSpec::WithCount(inner, count) => ChooseSpec::WithCount(Box::new(exact_designation_spec(inner, ctx)?), *count),
        ChooseSpec::WithCountValue(inner, count, value) => ChooseSpec::WithCountValue(Box::new(exact_designation_spec(inner, ctx)?), *count, value.clone()),
        other => other.clone(),
    })
}

fn resolve_suspected_designation_objects(
    game: &mut GameState, ctx: &mut ExecutionContext, spec: &ChooseSpec,
) -> Result<Vec<crate::ids::ObjectId>, ExecutionError> {
    let exact = exact_designation_spec(spec, ctx)?;
    resolve_objects_for_effect(game, ctx, &exact)
}
