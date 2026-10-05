//! Exact life direction/participant and lexical producer selection. This does
//! not reuse the generic last-result ID, which may belong to a payment gate.
use super::*;
use ironsmith_compiler_semantic::trigger_references::LifeAmountProducer;

pub(super) fn is_life_query(query: &ironsmith_core::PriorEffectMetricQuery) -> bool {
    query.source == EffectMetricSource::Outcome
        && matches!(
            query.metric,
            EffectMetric::LifeGained | EffectMetric::LifeLost
        )
        && query.action.is_none()
        && query.filter.is_none()
        && query.counter_type.is_none()
}
fn pending_life_metrics(effect: &EffectAst) -> Vec<EffectMetric> {
    fn collect(value: &Value, metrics: &mut Vec<EffectMetric>) {
        match value {
            Value::PendingPriorEffectMetric(query) if is_life_query(query) => {
                if !metrics.contains(&query.metric) {
                    metrics.push(query.metric);
                }
            }
            Value::SurfaceHinted { value, .. }
            | Value::Scaled(value, _)
            | Value::DividedRoundedDown(value, _)
            | Value::HalfRoundedDown(value) => collect(value, metrics),
            Value::Add(left, right) | Value::Min(left, right) => {
                collect(left, metrics);
                collect(right, metrics);
            }
            _ => {}
        }
    }
    let mut metrics = Vec::new();
    visit_effect_values(effect, &mut |value| collect(value, &mut metrics));
    metrics
}
fn direct_life_producer(effect: &EffectAst) -> Option<(EffectMetric, PlayerFilter)> {
    let EffectAst::SubjectVerb(subject) = effect else {
        return None;
    };
    let metric = match &subject.action {
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { .. }) => {
            EffectMetric::LifeGained
        }
        SubjectVerbActionAst::LifeResources(
            LifeResourceActionAst::LoseLife { .. } | LifeResourceActionAst::PayLife { .. },
        ) => EffectMetric::LifeLost,
        _ => return None,
    };
    let player = match subject.subject.player {
        PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
        PlayerAst::That => PlayerFilter::IteratedPlayer,
        PlayerAst::Opponent => PlayerFilter::Opponent,
        PlayerAst::Any => PlayerFilter::Any,
        // A target/choice needs its own resolved identity, not an invented
        // equivalence with the event participant.
        _ => return None,
    };
    Some((metric, player))
}
pub(super) fn life_producer(effect: &EffectAst) -> Option<(EffectMetric, PlayerFilter)> {
    if let Some(producer) = direct_life_producer(effect) {
        return Some(producer);
    }
    if let EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::MayByPlayer {
        player,
        effects,
    }) = effect
    {
        let mut producers = effects.iter().filter_map(life_producer);
        let (metric, participant) = producers.next()?;
        if producers.next().is_some() {
            return None;
        }
        let actor = match player {
            PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            PlayerAst::Opponent => PlayerFilter::Opponent,
            _ => return None,
        };
        return Some((
            metric,
            if participant == PlayerFilter::You {
                actor
            } else {
                participant
            },
        ));
    }
    // Transparent and optional wrappers can export the one instruction they
    // contain. Do not aggregate multiple life instructions or cross a delayed
    // program/player-iteration boundary and call that the latest instruction.
    let nested = match effect {
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::May { effects }) => {
            effects
        }
        _ => return None,
    };
    let mut producers = nested.iter().filter_map(life_producer);
    let first = producers.next()?;
    producers.next().is_none().then_some(first)
}
pub(super) fn supplies_requested_life_metric(
    producer: &EffectAst,
    consumer: &EffectAst,
) -> Option<bool> {
    let metrics = pending_life_metrics(consumer);
    if metrics.is_empty() {
        return None;
    }
    Some(life_producer(producer).is_some_and(|(metric, _)| metrics.contains(&metric)))
}
/// Transparent bodies can be annotated twice. A resolved query retains its
/// producer's exact ID; reuse it instead of creating an unobserved new result.
pub(super) fn rebound_producer_id(
    producer: &EffectAst,
    remaining: &[EffectAst],
) -> Option<EffectId> {
    let role = life_producer(producer)?;
    fn collect(value: &Value, role: &(EffectMetric, PlayerFilter), ids: &mut Vec<EffectId>) {
        match value {
            Value::PriorEffectMetric { effect_id, query }
                if is_life_query(query)
                    && query.metric == role.0
                    && query.player.as_ref() == Some(&role.1) =>
            {
                if !ids.contains(effect_id) {
                    ids.push(*effect_id);
                }
            }
            Value::SurfaceHinted { value, .. }
            | Value::Scaled(value, _)
            | Value::DividedRoundedDown(value, _)
            | Value::HalfRoundedDown(value) => collect(value, role, ids),
            Value::Add(left, right) | Value::Min(left, right) => {
                collect(left, role, ids);
                collect(right, role, ids);
            }
            _ => {}
        }
    }
    for next in remaining {
        if life_producer(next).as_ref() == Some(&role) {
            return None;
        }
        let mut ids = Vec::new();
        visit_effect_values(next, &mut |value| collect(value, &role, &mut ids));
        if ids.len() == 1 {
            return Some(ids[0]);
        }
        if !ids.is_empty() {
            return None;
        }
    }
    None
}

pub(super) fn remember_life_producer(
    producers: &mut Vec<LifeAmountProducer>,
    id: EffectId,
    effect: &EffectAst,
) {
    if let Some((metric, player)) = life_producer(effect) {
        producers.retain(|producer| producer.effect_id != id);
        producers.push(LifeAmountProducer {
            effect_id: id,
            metric,
            player,
        });
    }
}
pub(super) fn bind_life_query(
    query: &ironsmith_core::PriorEffectMetricQuery,
    state: EffectReferenceResolutionState<'_>,
) -> Result<Value, CardTextError> {
    let any_player = PlayerFilter::Any;
    let participant = query.player.as_ref().unwrap_or(&any_player);
    let same_player = |producer: &PlayerFilter| {
        producer == participant
            || (participant == &PlayerFilter::IteratedPlayer
                && producer == &PlayerFilter::You
                && state
                    .life_event_binding
                    .is_some_and(|event| event.player == PlayerFilter::You))
    };
    if let Some(producer) = state
        .life_amount_producers
        .iter()
        .rev()
        .find(|producer| producer.metric == query.metric && same_player(&producer.player))
    {
        return Ok(Value::PriorEffectMetric {
            effect_id: producer.effect_id,
            query: query.clone(),
        });
    }
    if let Some(event) = state.life_event_binding
        && event.metric == query.metric
        && (participant == &PlayerFilter::IteratedPlayer
            || (participant == &PlayerFilter::You && event.player == PlayerFilter::You))
    {
        return Ok(Value::EventValue(EventValueSpec::LifeChange {
            gained: query.metric == EffectMetric::LifeGained,
            for_controller: participant == &PlayerFilter::You,
        }));
    }
    Err(CardTextError::ParseError("life quantity requires a same-direction, same-participant life instruction or triggering event".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn query(metric: EffectMetric, player: PlayerFilter) -> ironsmith_core::PriorEffectMetricQuery {
        let mut query =
            ironsmith_core::PriorEffectMetricQuery::new(EffectMetricSource::Outcome, metric);
        query.player = Some(player);
        query
    }
    #[test]
    fn payment_results_cannot_replace_a_typed_life_event_but_explicit_life_producers_win() {
        let mut env = ReferenceEnv::default();
        env.life_event_binding =
            ironsmith_compiler_semantic::trigger_references::trigger_life_event_binding(
                &TriggerSpec::YouGainLife,
            );
        env.last_effect_id = RefState::Known(EffectId(77));
        let gained = query(EffectMetric::LifeGained, PlayerFilter::You);
        assert!(matches!(
            bind_life_query(&gained, effect_reference_resolution_state(&env)).unwrap(),
            Value::EventValue(EventValueSpec::LifeChange {
                gained: true,
                for_controller: true
            })
        ));
        env.life_amount_producers = std::sync::Arc::new(vec![LifeAmountProducer {
            effect_id: EffectId(9),
            metric: EffectMetric::LifeGained,
            player: PlayerFilter::You,
        }]);
        assert!(matches!(
            bind_life_query(&gained, effect_reference_resolution_state(&env)).unwrap(),
            Value::PriorEffectMetric {
                effect_id: EffectId(9),
                ..
            }
        ));
        let restored = ReferenceEnv::from_frame(&env.to_frame(false, false));
        assert_eq!(restored.life_event_binding, env.life_event_binding);
        assert_eq!(restored.life_amount_producers, env.life_amount_producers);
    }
    #[test]
    fn life_direction_participant_and_alternative_branches_must_all_be_proven() {
        let mut env = ReferenceEnv::default();
        env.allow_life_event_value = true; // Generic numeric capability is insufficient.
        env.life_event_binding =
            ironsmith_compiler_semantic::trigger_references::trigger_life_event_binding(
                &TriggerSpec::PlayerGainsLife {
                    player: PlayerFilter::Opponent,
                    during_turn: None,
                },
            );
        assert!(
            bind_life_query(
                &query(EffectMetric::LifeGained, PlayerFilter::You),
                effect_reference_resolution_state(&env)
            )
            .is_err()
        );
        assert!(
            bind_life_query(
                &query(EffectMetric::LifeLost, PlayerFilter::IteratedPlayer),
                effect_reference_resolution_state(&env)
            )
            .is_err()
        );
        assert!(matches!(
            bind_life_query(
                &query(EffectMetric::LifeGained, PlayerFilter::IteratedPlayer),
                effect_reference_resolution_state(&env)
            )
            .unwrap(),
            Value::EventValue(EventValueSpec::LifeChange {
                gained: true,
                for_controller: false
            })
        ));
        let mixed = TriggerSpec::Either(
            Box::new(TriggerSpec::YouGainLife),
            Box::new(TriggerSpec::PlayerLosesLife(PlayerFilter::You)),
        );
        assert!(
            ironsmith_compiler_semantic::trigger_references::trigger_life_event_binding(&mixed)
                .is_none()
        );
        env.life_event_binding = None;
        assert!(
            bind_life_query(
                &query(EffectMetric::LifeGained, PlayerFilter::IteratedPlayer),
                effect_reference_resolution_state(&env)
            )
            .is_err()
        );
    }
}
