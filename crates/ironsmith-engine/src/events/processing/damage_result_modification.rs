// Damage and its scalar result events share checked numeric replacement rules.
fn apply_damage_result_modification(
    game: &GameState,
    event: &Event,
    modification: &EventModification,
    effect: &ReplacementEffect,
) -> Result<Option<Event>, crate::effects::ExecutionError> {
    use crate::events::damage::checked_damage_amount;
    use crate::events::{
        DamageEvent, LifeGainEvent, LifeLossEvent, PutCountersEvent, downcast_event,
    };
    let amount = if let Some(event) = downcast_event::<DamageEvent>(event.inner()) {
        event.amount
    } else if let Some(event) = downcast_event::<LifeGainEvent>(event.inner()) {
        event.amount
    } else if let Some(event) = downcast_event::<LifeLossEvent>(event.inner()) {
        event.amount
    } else if let Some(event) = downcast_event::<PutCountersEvent>(event.inner()) {
        event.count
    } else {
        return Ok(None);
    };
    let next: u128 = match modification {
        EventModification::Multiply(factor) => u128::from(amount) * u128::from(*factor),
        EventModification::Add(delta) => (i128::from(amount) + i128::from(*delta)).max(0) as u128,
        EventModification::Subtract(delta) => u128::from(amount.saturating_sub(*delta)),
        EventModification::SetTo(value) => u128::from(*value),
        EventModification::SetToAtLeast(value) => u128::from(amount.max(
            resolve_value_for_replacement_checked(value, game, effect.source)?,
        )),
        EventModification::ReduceToZero => 0,
        EventModification::AddDynamic(value) => {
            let delta = resolve_signed_value_for_replacement(value, game, effect)?;
            (i128::from(amount) + i128::from(delta)).max(0) as u128
        },
        EventModification::Halve { round_up } => {
            u128::from(if *round_up { amount.div_ceil(2) } else { amount / 2 })
        }
    };
    // A rules-imposed maximum (an event-local player-counter lock) is
    // applied mathematically before asking whether the final event fits.
    let next = if let Some(counter) = downcast_event::<PutCountersEvent>(event.inner()) {
        counter
            .maximum_count
            .map_or(next, |maximum| next.min(u128::from(maximum)))
    } else {
        next
    };
    let next = checked_damage_amount(next, "damage/result replacement amount")?;
    Ok(Some(
        if let Some(damage) = downcast_event::<DamageEvent>(event.inner()) {
            event.rewrap(damage.with_amount(next))
        } else if let Some(gain) = downcast_event::<LifeGainEvent>(event.inner()) {
            event.rewrap(gain.with_amount(next))
        } else if let Some(loss) = downcast_event::<LifeLossEvent>(event.inner()) {
            event.rewrap(loss.with_amount(next))
        } else {
            event.rewrap(
                downcast_event::<PutCountersEvent>(event.inner())
                    .expect("checked event kind")
                    .with_count(next),
            )
        },
    ))
}

#[cfg(test)]
mod damage_result_modification_tests {
    use super::*;
    fn effect() -> ReplacementEffect {
        ReplacementEffect::with_matcher(
            crate::ObjectId::from_raw(1),
            crate::PlayerId(0),
            crate::events::WouldGainLifeMatcher::any_player(),
            ReplacementAction::Double,
        )
    }
    #[test]
    fn checked_modifiers_preserve_zero_floor_and_real_counter_limit() {
        let game = GameState::new(vec!["A".into(), "B".into()], 30);
        let damage = Event::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                crate::ObjectId::from_raw(1),
                crate::events::DamageTarget::Player(crate::PlayerId(1)),
                3,
                false,
                crate::events::cause::EventCause::effect(),
            ),
            Default::default(),
        );
        for modification in [EventModification::Add(-10), EventModification::Subtract(10)] {
            let result = apply_damage_result_modification(&game, &damage, &modification, &effect())
                .unwrap()
                .unwrap();
            assert_eq!(
                crate::events::downcast_event::<crate::events::DamageEvent>(result.inner())
                    .unwrap()
                    .amount,
                0
            );
        }
        let counters = crate::events::PutCountersEvent::with_cause(
            crate::game_state::Target::Player(crate::PlayerId(0)),
            CounterType::Poison,
            u32::MAX,
            crate::events::cause::EventCause::effect(),
        )
        .with_count_limit(1, 1);
        let event = Event::new_with_provenance(counters, Default::default());
        let result = apply_damage_result_modification(
            &game,
            &event,
            &EventModification::Multiply(u32::MAX),
            &effect(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            crate::events::downcast_event::<crate::events::PutCountersEvent>(result.inner())
                .unwrap()
                .count,
            1
        );
    }
}
