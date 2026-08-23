use crate::effect::{Effect, EffectOutcome, OutcomeStatus, OutcomeValue};
use crate::effects::{EffectExecutor, SequenceEffect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::ZoneChangeEvent;
use crate::filter::Comparison;
use crate::game_state::GameState;
use crate::resolve_value;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

pub type RepeatEffectsEffect = ironsmith_core::RepeatEffectsEffect<Effect>;

fn coalesce_vote_token_entry_events(
    game: &mut GameState,
    pending_start: usize,
    object_ids: &[crate::ids::ObjectId],
    ctx: &ExecutionContext,
) {
    if object_ids.len() <= 1 {
        return;
    }

    let removed = game.remove_pending_trigger_events_matching_from(pending_start, |event| {
        event
            .downcast::<ZoneChangeEvent>()
            .is_some_and(|zone_change| {
                zone_change.from == Zone::Command
                    && zone_change.to == Zone::Battlefield
                    && zone_change
                        .objects
                        .iter()
                        .all(|object_id| object_ids.contains(object_id))
            })
    });
    if removed.is_empty() {
        return;
    }

    let cause = removed
        .iter()
        .find_map(|event| {
            event
                .downcast::<ZoneChangeEvent>()
                .map(|zone_change| zone_change.cause.clone())
        })
        .unwrap_or_else(crate::events::EventCause::effect);
    let snapshots = removed
        .iter()
        .filter_map(|event| event.downcast::<ZoneChangeEvent>())
        .flat_map(|zone_change| zone_change.snapshots().iter().cloned())
        .collect();
    let event = ZoneChangeEvent::batch_with_snapshots(
        object_ids.to_vec(),
        Zone::Command,
        Zone::Battlefield,
        cause,
        snapshots,
    );
    game.queue_trigger_event(
        ctx.provenance,
        TriggerEvent::new_with_provenance(event, ctx.provenance),
    );
}

impl EffectExecutor for RepeatEffectsEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // A repeated one-object choice over `DistinctPowers` means one choice
        // from each power class, rather than N unconstrained choices from the
        // same pool. Keep the selected objects accumulated under the original
        // tag so a later "not chosen this way" filter sees the full set.
        if let crate::effect::Value::DistinctPowers(power_filter) = self.count.unhinted()
            && let [choice_effect] = self.effects.as_slice()
            && let Some(choice) =
                choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && choice.count.is_single()
            && &choice.filter == power_filter
        {
            let powers =
                crate::effects::helpers::distinct_power_values_for_filter(game, power_filter, ctx);
            let mut selected = Vec::new();
            let mut all_events = Vec::new();
            let mut all_execution_facts = Vec::new();
            ctx.clear_object_tag(&choice.tag);

            for power in powers {
                let mut power_choice = choice.clone();
                power_choice.filter.power = Some(Comparison::Equal(power));
                let outcome =
                    SequenceEffect::new(vec![Effect::new(power_choice)]).execute(game, ctx)?;
                all_events.extend(outcome.events.clone());
                all_execution_facts.extend(outcome.execution_facts.clone());
                if let Some(current) = ctx.get_tagged_all(&choice.tag) {
                    for snapshot in current {
                        if !selected
                            .iter()
                            .any(|existing: &crate::snapshot::ObjectSnapshot| {
                                existing.object_id == snapshot.object_id
                            })
                        {
                            selected.push(snapshot.clone());
                        }
                    }
                }
                if outcome.status.is_failure() {
                    ctx.set_tagged_objects(choice.tag.clone(), selected);
                    return Ok(EffectOutcome::with_details(
                        outcome.status,
                        outcome.value,
                        all_events,
                        all_execution_facts,
                    ));
                }
            }

            ctx.set_tagged_objects(choice.tag.clone(), selected);
            return Ok(EffectOutcome::with_details(
                OutcomeStatus::Succeeded,
                OutcomeValue::None,
                all_events,
                all_execution_facts,
            ));
        }

        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        let sequence = SequenceEffect::new(self.effects.clone());
        let mut all_events = Vec::new();
        let mut all_execution_facts = Vec::new();
        let mut all_output_objects = Vec::new();
        // A voter-independent "for each [option] vote, create a token" clause
        // lowers to RepeatEffectsEffect rather than living inside VoteEffect.
        // It is still one token-creation instruction, so all of those tokens
        // enter as one event. Keyword actions such as investigate remain
        // repeated actions and intentionally do not take this path.
        let batch_vote_tokens = matches!(self.count.unhinted(), crate::effect::Value::VoteCount(_))
            && matches!(
                self.effects.as_slice(),
                [effect]
                    if effect
                        .downcast_ref::<crate::effects::CreateTokenEffect>()
                        .is_some()
            );
        let pending_token_event_start = game.effect_store.pending_trigger_events.len();

        for _ in 0..count {
            let outcome = sequence.execute(game, ctx)?;
            all_events.extend(outcome.events.clone());
            all_execution_facts.extend(outcome.execution_facts.clone());
            if let Some(objects) = outcome.objects() {
                for object in objects {
                    if !all_output_objects.contains(object) {
                        all_output_objects.push(*object);
                    }
                }
            }
            if outcome.status.is_failure() {
                if batch_vote_tokens {
                    coalesce_vote_token_entry_events(
                        game,
                        pending_token_event_start,
                        &all_output_objects,
                        ctx,
                    );
                }
                let value = if all_output_objects.is_empty() {
                    outcome.value
                } else {
                    OutcomeValue::Objects(all_output_objects)
                };
                return Ok(EffectOutcome::with_details(
                    outcome.status,
                    value,
                    all_events,
                    all_execution_facts,
                ));
            }
        }

        if batch_vote_tokens {
            coalesce_vote_token_entry_events(
                game,
                pending_token_event_start,
                &all_output_objects,
                ctx,
            );
        }
        Ok(EffectOutcome::with_details(
            OutcomeStatus::Succeeded,
            if all_output_objects.is_empty() {
                OutcomeValue::None
            } else {
                OutcomeValue::Objects(all_output_objects)
            },
            all_events,
            all_execution_facts,
        ))
    }
}
