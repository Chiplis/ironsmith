use crate::effect::{Effect, EffectOutcome, OutcomeStatus, OutcomeValue};
use crate::effects::{EffectExecutor, SequenceEffect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::Comparison;
use crate::game_state::GameState;
use crate::resolve_value;

pub type RepeatEffectsEffect = ironsmith_core::RepeatEffectsEffect<Effect>;

impl EffectExecutor for RepeatEffectsEffect {
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

        for _ in 0..count {
            let outcome = sequence.execute(game, ctx)?;
            all_events.extend(outcome.events.clone());
            all_execution_facts.extend(outcome.execution_facts.clone());
            if outcome.status.is_failure() {
                return Ok(EffectOutcome::with_details(
                    outcome.status,
                    outcome.value,
                    all_events,
                    all_execution_facts,
                ));
            }
        }

        Ok(EffectOutcome::with_details(
            OutcomeStatus::Succeeded,
            OutcomeValue::None,
            all_events,
            all_execution_facts,
        ))
    }
}
