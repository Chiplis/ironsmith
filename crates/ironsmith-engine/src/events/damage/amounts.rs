//! Checked host representations for damage and its scalar consequences.
//! These limits describe the current engine representation, never a Magic cap.
use crate::effects::ExecutionError;

pub(crate) fn checked_damage_amount(
    amount: u128,
    resource: &'static str,
) -> Result<u32, ExecutionError> {
    u32::try_from(amount).map_err(|_| ExecutionError::ResourceLimitExceeded {
        resource,
        requested: amount,
        maximum: u32::MAX as u128,
    })
}
pub(crate) fn checked_damage_count(
    amount: u128,
    resource: &'static str,
) -> Result<i64, ExecutionError> {
    i64::try_from(amount).map_err(|_| ExecutionError::ResourceLimitExceeded {
        resource,
        requested: amount,
        maximum: i64::MAX as u128,
    })
}

/// Characteristic and cost consumers retain a signed 32-bit representation.
pub(crate) fn checked_scalar_count(
    amount: u128,
    resource: &'static str,
) -> Result<i32, ExecutionError> {
    i32::try_from(amount).map_err(|_| ExecutionError::ResourceLimitExceeded {
        resource,
        requested: amount,
        maximum: i32::MAX as u128,
    })
}

/// Current-turn quantity queries and grouped trigger values use signed scalar
/// results. Validate the completed receipt set before publication or projection,
/// including earlier actions, rather than allow their sums to wrap later.
pub(crate) fn validate_damage_history_amounts<'a>(
    game: &'a crate::game_state::GameState,
    incoming: impl IntoIterator<Item = &'a crate::triggers::TriggerEvent>,
) -> Result<(), ExecutionError> {
    let mut seen_occurrences = std::collections::HashSet::new();
    let mut totals = [0u128; 3];
    let mut add = |event: &crate::triggers::TriggerEvent| {
        let (index, amount) = if let Some(damage) = event.downcast::<crate::events::DamageEvent>() {
            (0, damage.amount)
        } else if let Some(loss) = event.downcast::<crate::events::LifeLossEvent>() {
            (1, loss.amount)
        } else if let Some(gain) = event.downcast::<crate::events::LifeGainEvent>() {
            (2, gain.amount)
        } else {
            return;
        };
        if !seen_occurrences.insert(event.occurrence_key()) {
            return;
        }
        totals[index] += u128::from(amount);
    };
    for record in game.turn_store.turn_history.projected_records() {
        add(&record.event);
    }
    for event in game
        .effect_store
        .pending_trigger_events
        .iter()
        .chain(incoming)
    {
        if !event.triggers_captured() {
            add(event);
        }
    }
    for (amount, resource) in totals.into_iter().zip([
        "current-turn damage quantity",
        "current-turn life-loss quantity",
        "current-turn life-gain quantity",
    ]) {
        checked_damage_count(amount, resource)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prior_damage_projection_is_counted_once_and_larger_history_stays_wide() {
        let mut game = crate::GameState::new(vec!["A".into(), "B".into()], 30);
        let provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::Damage);
        let mut prior = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                crate::ObjectId::from_raw(1),
                crate::events::DamageTarget::Player(crate::PlayerId(1)),
                i32::MAX as u32,
                false,
                crate::events::cause::EventCause::effect(),
            ),
            provenance,
        );
        game.stage_turn_history_event(&prior);
        validate_damage_history_amounts(&game, [&prior]).unwrap();
        game.record_turn_history_event(&prior);
        prior.mark_triggers_captured();
        validate_damage_history_amounts(&game, [&prior]).unwrap();
        let next = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                crate::ObjectId::from_raw(1),
                crate::events::DamageTarget::Player(crate::PlayerId(1)),
                1,
                false,
                crate::events::cause::EventCause::effect(),
            ),
            Default::default(),
        );
        validate_damage_history_amounts(&game, [&next]).unwrap();
        assert_eq!(
            game.turn_store
                .turn_history
                .event_kind_count(crate::events::EventKind::Damage),
            1
        );
    }
    #[test]
    fn distinct_damage_siblings_are_not_aliases_even_with_the_same_parent_provenance() {
        let mut game = crate::GameState::new(vec!["A".into(), "B".into()], 30);
        let prior_id = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::Damage);
        let sibling_id = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::Damage);
        let event = |amount, provenance| {
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::DamageEvent::with_cause(
                    crate::ObjectId::from_raw(1),
                    crate::events::DamageTarget::Player(crate::PlayerId(1)),
                    amount,
                    false,
                    crate::events::cause::EventCause::effect(),
                ),
                provenance,
            )
        };
        let prior = event(i32::MAX as u32 - 1, prior_id);
        game.record_turn_history_event(&prior);
        let first = event(1, sibling_id);
        let second = event(1, sibling_id);
        validate_damage_history_amounts(&game, [&first, &first]).unwrap();
        validate_damage_history_amounts(&game, [&first, &second]).unwrap();
        assert_ne!(first.occurrence_key(), second.occurrence_key());
    }

    #[test]
    fn exact_boundaries_and_incomplete_execution_classification() {
        assert_eq!(
            checked_damage_amount(u32::MAX as u128, "damage").unwrap(),
            u32::MAX
        );
        assert_eq!(
            checked_damage_count(i64::MAX as u128, "damage outcome").unwrap(),
            i64::MAX
        );
        for error in [
            checked_damage_amount(u32::MAX as u128 + 1, "damage").unwrap_err(),
            checked_damage_count(i64::MAX as u128 + 1, "damage outcome").unwrap_err(),
        ] {
            assert!(error.is_incomplete_execution());
            assert!(matches!(
                error,
                ExecutionError::ResourceLimitExceeded { .. }
            ));
        }
    }
}
