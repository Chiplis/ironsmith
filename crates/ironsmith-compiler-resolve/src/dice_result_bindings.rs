//! Die result queries use the shared exact-instruction binding owner.
use super::*;
use super::local_random_result_bindings::Family;

pub(super) fn is_die_query(query: &ironsmith_core::PriorEffectMetricQuery) -> bool {
    Family::Die.query(query)
}
pub(super) fn remember_producer(producers: &mut Vec<Option<EffectId>>, id: Option<EffectId>, effect: &EffectAst) {
    if result_gate_surface(effect)
        .is_some_and(|(predicate, _)| matches!(predicate, IfResultPredicate::DieValue(_)))
    {
        return;
    }
    Family::Die.remember(producers, id, effect)
}
pub(super) fn bind_die_query(query: &ironsmith_core::PriorEffectMetricQuery, state: EffectReferenceResolutionState<'_>) -> Result<Value, CardTextError> {
    Family::Die.bind(query, state)
}
pub(super) fn rebound_producer_id(producer: &EffectAst, remaining: &[EffectAst]) -> Option<EffectId> {
    Family::Die.rebound(producer, remaining)
}
