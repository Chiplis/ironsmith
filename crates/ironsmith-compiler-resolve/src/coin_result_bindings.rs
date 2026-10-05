//! Coin result queries use the shared exact-instruction binding owner.
use super::*;
use super::local_random_result_bindings::Family;

pub(super) fn is_coin_query(query: &ironsmith_core::PriorEffectMetricQuery) -> bool {
    Family::Coin.query(query)
}
pub(super) fn remember_producer(producers: &mut Vec<Option<EffectId>>, id: Option<EffectId>, effect: &EffectAst) {
    Family::Coin.remember(producers, id, effect)
}
pub(super) fn bind_coin_query(query: &ironsmith_core::PriorEffectMetricQuery, state: EffectReferenceResolutionState<'_>) -> Result<Value, CardTextError> {
    Family::Coin.bind(query, state)
}
pub(super) fn rebound_producer_id(producer: &EffectAst, remaining: &[EffectAst]) -> Option<EffectId> {
    Family::Coin.rebound(producer, remaining)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> ironsmith_core::PriorEffectMetricQuery {
        ironsmith_core::PriorEffectMetricQuery::new(EffectMetricSource::Outcome, EffectMetric::CoinFlipsWon)
            .with_action(PriorEffectAction::Flipped)
    }

    #[test]
    fn coin_binding_survives_frames_and_ignores_unrelated_numeric_and_die_results() {
        let env = ReferenceEnv {
            coin_result_producers: std::sync::Arc::new(vec![Some(EffectId(4))]),
            die_result_producers: std::sync::Arc::new(vec![Some(EffectId(8))]),
            last_effect_id: RefState::Known(EffectId(9)),
            ..Default::default()
        };
        let restored = ReferenceEnv::from_frame(&ReferenceFrame::from_lowering_frame(&env.to_lowering_frame(false, false)));
        assert_eq!(bind_coin_query(&query(), effect_reference_resolution_state(&restored)).unwrap(),
            Value::PriorEffectMetric { effect_id: EffectId(4), query: query() });
    }

    #[test]
    fn absent_and_unexported_coin_results_never_fall_back_to_an_old_number() {
        for producers in [vec![], vec![Some(EffectId(4)), None]] {
            let env = ReferenceEnv {
                coin_result_producers: std::sync::Arc::new(producers),
                last_effect_id: RefState::Known(EffectId(9)),
                ..Default::default()
            };
            assert!(bind_coin_query(&query(), effect_reference_resolution_state(&env)).is_err());
        }
    }
}
