//! Retained announcement packets are authoritative for target cardinality.
use crate::effects::ExecutionContext;
use crate::target::ChooseSpec;
fn matches(assignment:&crate::game_state::TargetAssignment,spec:&ChooseSpec)->bool {
    assignment.spec==*spec || assignment.spec.base()==spec.base() || crate::targeting::target_spec_matches_chooser_assignment(spec,&assignment.spec)
}
pub(crate) fn retained(ctx:&ExecutionContext,spec:&ChooseSpec)->bool {ctx.target_assignments.iter().any(|assignment|matches(assignment,spec))}
pub(crate) fn multiple(ctx:&ExecutionContext,spec:&ChooseSpec)->bool {ctx.target_assignments.iter().filter(|assignment|matches(assignment,spec)).map(|assignment|assignment.range.len()).sum::<usize>()>1}
