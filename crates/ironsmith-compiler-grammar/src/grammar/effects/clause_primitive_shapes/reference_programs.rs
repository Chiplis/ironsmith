use super::*;

pub fn parse_retarget_reference_shape(tokens: &[OwnedLexToken]) -> Option<RetargetReferenceShape> {
    let tokens = trim_shape_edges(tokens);
    if primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["the", "copy"]),
            primitives::phrase(&["the", "copies"]),
            primitives::phrase(&["that", "copy"]),
            primitives::phrase(&["those", "copies"]),
        )),
    )
    .is_some()
    {
        Some(RetargetReferenceShape::Copy)
    } else if primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("it").void(),
            primitives::kw("them").void(),
            primitives::phrase(&["the", "spell"]).void(),
            primitives::phrase(&["that", "spell"]).void(),
        )),
    )
    .is_some()
    {
        Some(RetargetReferenceShape::Other)
    } else {
        None
    }
}

pub fn parse_retarget_constraint_shapes(tokens: &[OwnedLexToken]) -> Vec<RetargetConstraintShape> {
    let tokens = trim_shape_edges(tokens);
    let mut constraints = Vec::new();
    let candidates: &'static [(&'static [&'static str], RetargetConstraintShape)] = &[
        (
            &["with", "a", "single", "target"],
            RetargetConstraintShape::SingleTarget,
        ),
        (
            &["targets", "only", "a", "single", "creature"],
            RetargetConstraintShape::SingleCreatureTarget,
        ),
        (
            &["targets", "only", "this", "creature"],
            RetargetConstraintShape::SourceOnlyTarget,
        ),
        (
            &["targets", "only", "this", "permanent"],
            RetargetConstraintShape::SourceOnlyTarget,
        ),
        (
            &["targets", "only", "you"],
            RetargetConstraintShape::YouOnlyTarget,
        ),
        (
            &["targets", "only", "a", "player"],
            RetargetConstraintShape::AnyPlayerTarget,
        ),
        (
            &["if", "that", "target", "is", "you"],
            RetargetConstraintShape::YouOnlyTarget,
        ),
    ];
    for &(phrase, constraint) in candidates {
        if primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some() {
            constraints.push(constraint);
        }
    }
    constraints
}
