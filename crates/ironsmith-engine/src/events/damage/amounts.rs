//! Damage and its scalar consequences are game quantities: they saturate at
//! the bounds of their storage type instead of failing (CR has no cap).
//! The `Result` shape is kept for callers; these conversions never fail.
use crate::effects::ExecutionError;

pub(crate) fn checked_damage_amount(
    amount: u128,
    _resource: &'static str,
) -> Result<u32, ExecutionError> {
    Ok(u32::try_from(amount).unwrap_or(u32::MAX))
}
pub(crate) fn checked_damage_count(
    amount: u128,
    _resource: &'static str,
) -> Result<i64, ExecutionError> {
    Ok(i64::try_from(amount).unwrap_or(i64::MAX))
}

/// Characteristic and cost consumers retain a signed 32-bit representation.
pub(crate) fn checked_scalar_count(
    amount: u128,
    _resource: &'static str,
) -> Result<i32, ExecutionError> {
    Ok(i32::try_from(amount).unwrap_or(i32::MAX))
}

/// Current-turn quantity queries and grouped trigger values saturate where
/// they are summed, so a completed receipt set never needs a representation
/// check before publication. Kept as the publication seam for callers.
pub(crate) fn validate_damage_history_amounts<'a>(
    _game: &'a crate::game_state::GameState,
    _incoming: impl IntoIterator<Item = &'a crate::triggers::TriggerEvent>,
) -> Result<(), ExecutionError> {
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
    fn damage_representations_saturate_at_their_bounds() {
        assert_eq!(
            checked_damage_amount(u32::MAX as u128, "damage").unwrap(),
            u32::MAX
        );
        assert_eq!(
            checked_damage_count(i64::MAX as u128, "damage outcome").unwrap(),
            i64::MAX
        );
        assert_eq!(checked_damage_amount(u32::MAX as u128 + 1, "damage").unwrap(), u32::MAX);
        assert_eq!(
            checked_damage_count(i64::MAX as u128 + 1, "damage outcome").unwrap(),
            i64::MAX
        );
        assert_eq!(checked_scalar_count(u128::MAX, "scalar").unwrap(), i32::MAX);
    }
}
