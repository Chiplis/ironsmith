//! Checked projections of already-matched counter receipts. The caller owns
//! the simultaneous boundary; this never infers simultaneity from a recipient.
use crate::effects::ExecutionError;
use crate::triggers::TriggeredAbilityEntry;
use crate::triggers::matcher_trait::SimultaneousTriggerKey;
use std::collections::HashMap;

/// Stage the entire batch before publishing any queue entry. Per-event
/// occurrence indices preserve identical ability instances independently.
pub(crate) fn coalesce_counter_recipient_groups(
    groups: Vec<Vec<TriggeredAbilityEntry>>,
) -> Result<Vec<Vec<TriggeredAbilityEntry>>, ExecutionError> {
    let mut staged: Vec<Vec<TriggeredAbilityEntry>> = Vec::with_capacity(groups.len());
    let mut recipients: HashMap<_, (usize, usize)> = HashMap::new();
    for entries in groups {
        let mut occurrences = HashMap::new();
        let group_index = staged.len();
        staged.push(Vec::new());
        for entry in entries {
            if let Some(group @ SimultaneousTriggerKey::CounterRecipient { .. }) =
                entry.ability.trigger.simultaneous_trigger_key(&entry.triggering_event)
            {
                let key = (entry.source_stable_id, entry.trigger_identity, group);
                let occurrence = occurrences.entry(key).or_insert(0usize);
                let instance_key = (key, *occurrence);
                *occurrence += 1;
                if let Some(&(previous_group, previous_index)) = recipients.get(&instance_key) {
                    let previous: &mut TriggeredAbilityEntry = &mut staged[previous_group][previous_index];
                    previous.triggering_event.accumulate_counter_trigger_amount(&entry.triggering_event)?;
                    crate::triggers::merge_trigger_group_tags(&mut previous.tagged_objects, &entry.tagged_objects);
                    continue;
                }
                recipients.insert(instance_key, (group_index, staged[group_index].len()));
            }
            staged[group_index].push(entry);
        }
    }
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{EffectExecutor, ExecutionContext, ProliferateEffect};
    use crate::triggers::{CountMode, CounterPutOnTrigger, Trigger, TriggerQueue};
    use crate::{CardId, CardType, CounterType, GameState, PlayerId, Zone};

    /// The real producer supplies two kinds and their real batch identity.
    /// Only the private numeric projection is amplified to reach an i64
    /// boundary without allocating billions of physical counter receipts.
    fn large_producer_group() -> (GameState, crate::ObjectId, Vec<crate::triggers::TriggerEvent>) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let card = crate::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Counter watcher")
            .card_types(vec![CardType::Artifact])
            .with_trigger(Trigger::new(CounterPutOnTrigger::new(crate::target::ObjectFilter::permanent())
                .count(CountMode::OneOrMore)), Vec::new()).build();
        let source = game.create_object_from_definition(&card, PlayerId(0), Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge, 1);
        game.object_mut(source).unwrap().counters.insert(CounterType::Time, 1);
        let outcome = ProliferateEffect::new(1).execute(&mut game,
            &mut ExecutionContext::new_default(source, PlayerId(0))).unwrap();
        let mut events: Vec<_> = outcome.events.into_iter()
            .filter(|event| event.downcast::<crate::events::MarkersChangedEvent>().is_some()).collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].simultaneous_batch(), events[1].simultaneous_batch());
        for event in &mut events {
            for _ in 0..62 {
                event.accumulate_counter_trigger_amount(&event.clone()).unwrap();
            }
        }
        (game, source, events)
    }

    fn expected_error() -> ExecutionError {
        ExecutionError::ResourceLimitExceeded {
            resource: "counter trigger group amount", requested: 1u128 << 63,
            maximum: i64::MAX as u128,
        }
    }

    #[test]
    fn typed_capture_rejects_counter_group_overflow_before_publishing_any_entry() {
        let (mut game, source, mut events) = large_producer_group();
        let context = ExecutionContext::new_default(source, PlayerId(0));
        let result = crate::effects::capture_triggers_before_added_program(
            &mut game, &context, None, events.iter_mut(),
        );
        assert_eq!(result, Err(expected_error()));
        assert!(game.take_pending_trigger_entries().is_empty());
        assert!(events.iter().all(|event| !event.triggers_captured()));
        assert!(events.iter().all(|event| event.counter_trigger_amount() == Ok(1i64 << 62)));
    }

    #[test]
    fn typed_drain_preserves_the_queue_and_pending_group_on_counter_overflow() {
        for (delayed, with_decision_channel) in [(false, false), (false, true), (true, false), (true, true)] {
            let (mut game, source, events) = large_producer_group();
            if delayed {
                let trigger = match &game.object(source).unwrap().abilities[0].kind {
                    crate::ability::AbilityKind::Triggered(ability) => ability.clone(),
                    _ => panic!("expected counter trigger"),
                };
                game.object_mut(source).unwrap().abilities_mut().clear();
                crate::effects::delayed::queue_delayed_trigger(&mut game,
                    crate::effects::delayed::DelayedTriggerConfig::new(
                        trigger.trigger, trigger.effects, true, Vec::new(), PlayerId(0),
                    ).with_ability_source(Some(source)));
            }
            for event in events { game.queue_trigger_event(event.provenance(), event); }
            let mut queue = TriggerQueue::new();
            let result = if with_decision_channel {
                crate::game_loop::drain_pending_trigger_events_with_dm(
                    &mut game, &mut queue, &mut crate::decision::SelectFirstDecisionMaker,
                )
            } else {
                crate::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue)
            };
            assert_eq!(result, Err(expected_error()));
            assert!(queue.entries.is_empty());
            let pending = game.take_pending_trigger_events();
            assert_eq!(pending.len(), 2);
            assert!(pending.iter().all(|event| event.counter_trigger_amount() == Ok(1i64 << 62)));
            assert_eq!(game.effect_store.delayed_triggers.len(), usize::from(delayed));
        }
    }

    fn pending_large_group() -> (GameState, crate::ObjectId) {
        let (mut game, source, events) = large_producer_group();
        for event in events { game.queue_trigger_event(event.provenance(), event); }
        assert!(game.token_resource_failure().is_none(), "the producing effect's resource scope has closed");
        (game, source)
    }

    #[test]
    fn turn_advance_and_full_turn_driver_return_the_exact_group_failure() {
        use crate::game_loop::GameLoopError;
        let (mut game, _) = pending_large_group();
        let mut queue = TriggerQueue::new();
        let previous_step = game.turn.step;
        let mut runner = crate::turn_runner::TurnRunner::from_state_for_sync(crate::turn_runner::TurnState::Upkeep);
        assert!(matches!(runner.advance(&mut game, &mut queue),
            Err(GameLoopError::ExecutionFailed(error)) if error == expected_error()));
        assert!(matches!(runner.state(), crate::turn_runner::TurnState::Upkeep));
        assert_eq!(game.turn.step, previous_step);
        assert!(queue.entries.is_empty());
        assert_eq!(game.take_pending_trigger_events().len(), 2);

        let (mut game, _) = pending_large_group();
        let mut queue = TriggerQueue::new();
        let mut combat = crate::combat_state::new_combat();
        assert!(matches!(crate::game_loop::execute_turn_with(
            &mut game, &mut combat, &mut queue, &mut crate::decision::SelectFirstDecisionMaker,
        ), Err(GameLoopError::ExecutionFailed(error)) if error == expected_error()));
        assert!(queue.entries.is_empty());
        assert_eq!(game.take_pending_trigger_events().len(), 2);
    }

    #[test]
    fn priority_land_action_rolls_back_its_mutation_when_group_matching_is_incomplete() {
        use crate::game_loop::{GameLoopError, PriorityLoopState, PriorityResponse};
        let (mut game, _) = pending_large_group();
        game.turn.turn_number = 3;
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = PlayerId(0);
        game.turn.priority_player = Some(PlayerId(0));
        let land = game.create_object_from_definition(
            &crate::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Failure boundary land")
                .card_types(vec![CardType::Land]).build(), PlayerId(0), Zone::Hand,
        );
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        assert!(matches!(crate::game_loop::apply_priority_response_with_dm(
            &mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(crate::decision::LegalAction::PlayLand { land_id: land }),
            &mut crate::decision::SelectFirstDecisionMaker,
        ), Err(GameLoopError::ExecutionFailed(error)) if error == expected_error()));
        assert_eq!(game.object(land).unwrap().zone, Zone::Hand);
        assert!(queue.entries.is_empty());
        assert_eq!(game.take_pending_trigger_events().len(), 2);
    }

    #[test]
    fn special_action_instruction_boundary_preserves_typed_failure_after_effect_scope_closes() {
        let (mut game, source) = pending_large_group();
        game.turn.priority_player = Some(PlayerId(0));
        let expires = game.turn.turn_number;
        game.effect_store.repeatable_mana_payment_actions.push(crate::game_state::RepeatableManaPaymentAction {
            player: PlayerId(0), source, controller: PlayerId(0),
            cost: crate::mana::ManaCost::from_pips(Vec::new()),
            effects: vec![crate::effect::Effect::new(ProliferateEffect::new(0))],
            targets: Vec::new(), tagged_objects: Default::default(), tagged_players: Default::default(),
            expires_end_of_turn: expires, ends_continuous_effects: Vec::new(),
        });
        let result = crate::special_actions::perform(
            crate::special_actions::SpecialAction::PerformRepeatableManaPaymentAction { action_index: 0 },
            &mut game, PlayerId(0), &mut crate::decision::SelectFirstDecisionMaker,
        );
        assert_eq!(result, Err(crate::special_actions::ActionError::ExecutionFailure {
            source, error: expected_error(),
        }));
        assert_eq!(game.take_pending_trigger_events().len(), 2);
        assert!(game.take_pending_trigger_entries().is_empty());
    }
}
