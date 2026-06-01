//! Double counters on selected objects.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::processing::process_put_counters_with_event;
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::DoubleCountersEffect;

impl EffectExecutor for DoubleCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        if target_ids.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let mut outcomes = Vec::new();
        let mut affected_objects = Vec::new();
        for target_id in target_ids {
            let counters = game
                .object(target_id)
                .map(|object| {
                    object
                        .counters
                        .iter()
                        .filter_map(|(counter_type, count)| {
                            (*count > 0
                                && self.counter_type.is_none_or(|wanted| wanted == *counter_type))
                            .then_some((*counter_type, *count))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            for (counter_type, count) in counters {
                let final_count = process_put_counters_with_event(
                    game,
                    target_id,
                    counter_type,
                    count,
                    ctx.cause.clone(),
                );
                if final_count == 0 {
                    outcomes.push(EffectOutcome::prevented());
                    continue;
                }
                if let Some(event) = game.add_counters_with_source(
                    target_id,
                    counter_type,
                    final_count,
                    Some(ctx.source),
                    Some(ctx.controller),
                ) {
                    affected_objects.push(target_id);
                    outcomes.push(EffectOutcome::count(final_count as i32).with_event(event));
                }
            }
        }

        let mut outcome = if outcomes.is_empty() {
            EffectOutcome::resolved()
        } else {
            EffectOutcome::aggregate_summing_counts(outcomes)
        };
        if !affected_objects.is_empty() {
            affected_objects.sort_unstable();
            affected_objects.dedup();
            outcome = outcome.with_affected_objects_from_game(game, affected_objects);
        }
        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target for doubled counters"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::ChoiceCount;
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::{CounterType, Object};
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_permanent(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Artifact])
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn deepglow_skate_target_spec() -> ChooseSpec {
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::permanent()))
            .with_count(ChoiceCount::any_number())
    }

    #[test]
    fn deepglow_skate_doubles_each_counter_kind_on_selected_permanents() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let selected = create_permanent(&mut game, "Deepglow Skate target", alice);
        let selected_without_counters =
            create_permanent(&mut game, "Deepglow Skate empty target", alice);
        let unselected = create_permanent(&mut game, "Unselected permanent", alice);

        game.add_counters(selected, CounterType::PlusOnePlusOne, 2);
        game.add_counters(selected, CounterType::Charge, 3);
        game.add_counters(unselected, CounterType::PlusOnePlusOne, 4);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![
            ResolvedTarget::Object(selected),
            ResolvedTarget::Object(selected_without_counters),
        ];

        let effect = DoubleCountersEffect::new(None, deepglow_skate_target_spec());
        effect
            .execute(&mut game, &mut ctx)
            .expect("Deepglow Skate counter doubling should resolve");

        assert_eq!(game.counter_count(selected, CounterType::PlusOnePlusOne), 4);
        assert_eq!(game.counter_count(selected, CounterType::Charge), 6);
        assert_eq!(
            game.counter_count(selected_without_counters, CounterType::PlusOnePlusOne),
            0
        );
        assert_eq!(game.counter_count(unselected, CounterType::PlusOnePlusOne), 4);
    }

    #[test]
    fn deepglow_skate_any_number_targets_allows_zero_choices() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let unchosen = create_permanent(&mut game, "Unchosen Deepglow Skate permanent", alice);
        game.add_counters(unchosen, CounterType::Charge, 2);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DoubleCountersEffect::new(None, deepglow_skate_target_spec());
        effect
            .execute(&mut game, &mut ctx)
            .expect("choosing zero Deepglow Skate targets should resolve");

        assert_eq!(game.counter_count(unchosen, CounterType::Charge), 2);
    }
}
