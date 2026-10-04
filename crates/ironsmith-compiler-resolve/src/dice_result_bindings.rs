//! Completed local die-result precedence, separate from ambient triggering dice.
use super::*;
pub(super) fn is_die_query(query: &ironsmith_core::PriorEffectMetricQuery) -> bool {
    query.action == Some(PriorEffectAction::Rolled)
        && query.source == EffectMetricSource::Outcome
        && query.metric == EffectMetric::Count
        && query.filter.is_none()
        && query.player.is_none()
        && query.counter_type.is_none()
}
fn direct_roll(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Random(
                RandomActionAst::RollDie { .. } | RandomActionAst::RollDiceChooseResult { .. }
            ),
            ..
        })
    )
}
fn compatible_producer(effect: &EffectAst) -> bool {
    if direct_roll(effect) {
        return true;
    }
    match effect {
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. } => {
            matches!(effects.as_slice(), [only] if compatible_producer(only))
        }
        _ => false,
    }
}
fn contains_roll(effect: &EffectAst) -> bool {
    if direct_roll(effect) {
        return true;
    }
    let mut found = false;
    for_each_nested_effects(effect, true, |effects| {
        found |= effects.iter().any(contains_roll)
    });
    found
}
pub(super) fn remember_producer(
    producers: &mut Vec<Option<EffectId>>,
    id: Option<EffectId>,
    effect: &EffectAst,
) {
    if compatible_producer(effect) {
        producers.push(id);
    } else if contains_roll(effect) {
        producers.push(None);
    }
}
pub(super) fn bind_die_query(
    query: &ironsmith_core::PriorEffectMetricQuery,
    state: EffectReferenceResolutionState<'_>,
) -> Result<Value, CardTextError> {
    if let Some(id) = state.die_result_producers.last() {
        return id
            .map(|effect_id| Value::PriorEffectMetric {
                effect_id,
                query: query.clone(),
            })
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "the local die result is not exported by its enclosing instruction".into(),
                )
            });
    }
    if state.dice_event_grouped == Some(false) {
        return Ok(Value::EventValue(EventValueSpec::DieResult));
    }
    Err(CardTextError::ParseError(
        "die-result predicate requires a compatible local roll or singular numeric roll trigger"
            .into(),
    ))
}
pub(super) fn rebound_producer_id(
    producer: &EffectAst,
    remaining: &[EffectAst],
) -> Option<EffectId> {
    if !compatible_producer(producer) {
        return None;
    }
    fn collect(value: &Value, ids: &mut Vec<EffectId>) {
        match value {
            Value::PriorEffectMetric { effect_id, query } if is_die_query(query) => {
                if !ids.contains(effect_id) {
                    ids.push(*effect_id);
                }
            }
            Value::SurfaceHinted { value, .. }
            | Value::Scaled(value, _)
            | Value::DividedRoundedDown(value, _)
            | Value::HalfRoundedDown(value) => collect(value, ids),
            Value::Add(a, b) | Value::Min(a, b) => {
                collect(a, ids);
                collect(b, ids);
            }
            _ => {}
        }
    }
    for consumer in remaining {
        if contains_roll(consumer) {
            return None;
        }
        let mut ids = Vec::new();
        visit_effect_values(consumer, &mut |value| collect(value, &mut ids));
        if ids.len() == 1 {
            return Some(ids[0]);
        }
    }
    None
}
