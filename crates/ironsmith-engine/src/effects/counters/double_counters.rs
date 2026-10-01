//! Double counters on selected objects.

use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_players_from_spec};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;
pub use ironsmith_core::DoubleCountersEffect;

fn double_counters_targets_players(target: &ChooseSpec) -> bool {
    matches!(
        target.base(),
        ChooseSpec::Player(_)
            | ChooseSpec::SpecificPlayer(_)
            | ChooseSpec::EachPlayer(_)
            | ChooseSpec::SourceController
            | ChooseSpec::SourceOwner
    )
}

fn player_counter_counts(
    game: &GameState,
    player_id: crate::ids::PlayerId,
    counter_type: Option<CounterType>,
) -> Result<Vec<(CounterType, u32)>, ExecutionError> {
    let player = game
        .player(player_id)
        .ok_or(ExecutionError::PlayerNotFound(player_id))?;
    Ok(player
        .counter_types_with_counters()
        .into_iter()
        .filter(|candidate| counter_type.is_none_or(|wanted| wanted == *candidate))
        .map(|candidate| (candidate, player.counter_count(candidate)))
        .collect())
}

impl EffectExecutor for DoubleCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            if double_counters_targets_players(&self.target) {
                let player_ids = resolve_players_from_spec(game, &self.target, ctx)?;
                let mut outcomes = Vec::new();
                for player_id in player_ids {
                    for (counter_type, count) in
                        player_counter_counts(game, player_id, self.counter_type)?
                    {
                        let event = crate::events::Event::put_player_counters(
                            player_id,
                            counter_type,
                            count,
                            ctx.cause.clone(),
                        )
                        .with_provenance(ctx.provenance);
                        outcomes.push(crate::effects::counters::execute_player_counter_placement(
                            game, ctx, event,
                        )?);
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::aggregate_summing_counts(outcomes));
                        }
                    }
                }

                return Ok(if outcomes.is_empty() {
                    EffectOutcome::resolved()
                } else {
                    EffectOutcome::aggregate_summing_counts(outcomes)
                });
            }

            let target_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
            if target_ids.is_empty() {
                return Ok(EffectOutcome::resolved());
            }

            let mut outcomes = Vec::new();
            // One doubling is one simultaneous counter-placing event (CR 603.2c).
            let mut counter_batch: Option<crate::provenance::ProvNodeId> = None;
            for target_id in target_ids {
                let counters = game
                    .object(target_id)
                    .map(|object| {
                        object
                            .counters
                            .iter()
                            .filter_map(|(counter_type, count)| {
                                (*count > 0
                                    && self
                                        .counter_type
                                        .is_none_or(|wanted| wanted == *counter_type))
                                .then_some((*counter_type, *count))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                for (counter_type, count) in counters {
                    let event = crate::events::Event::put_counters(
                        target_id,
                        counter_type,
                        count,
                        ctx.cause.clone(),
                    )
                    .with_provenance(ctx.provenance);
                    let mut outcome = super::execute_object_counter_placement(game, ctx, event)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    for event in &mut outcome.events {
                        if event.kind() == crate::events::EventKind::MarkersChanged {
                            let batch = *counter_batch.get_or_insert_with(|| {
                                game.alloc_child_event_provenance(
                                    ctx.provenance,
                                    crate::events::EventKind::MarkersChanged,
                                )
                            });
                            *event = event.clone().with_simultaneous_batch(batch);
                        }
                    }
                    outcomes.push(outcome);
                }
            }

            let outcome = if outcomes.is_empty() {
                EffectOutcome::resolved()
            } else {
                EffectOutcome::aggregate_summing_counts(outcomes)
            };
            Ok(outcome)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice());
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() && result.is_ok() {
                return Ok(EffectOutcome::count(0));
            }
        }
        result
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
        assert_eq!(
            game.counter_count(unselected, CounterType::PlusOnePlusOne),
            4
        );
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

    #[test]
    fn aetheric_amplifier_target_mode_doubles_each_counter_kind_on_target_permanent() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let target = create_permanent(&mut game, "Aetheric Amplifier target", alice);
        let other = create_permanent(&mut game, "Aetheric Amplifier bystander", alice);

        game.add_counters(target, CounterType::PlusOnePlusOne, 2);
        game.add_counters(target, CounterType::Charge, 1);
        game.add_counters(other, CounterType::Charge, 3);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![ResolvedTarget::Object(target)];

        let effect = DoubleCountersEffect::new(
            None,
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::permanent())),
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("Aetheric Amplifier target mode should resolve");

        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 4);
        assert_eq!(game.counter_count(target, CounterType::Charge), 2);
        assert_eq!(game.counter_count(other, CounterType::Charge), 3);
    }

    #[test]
    fn aetheric_amplifier_player_mode_doubles_each_player_counter_kind_you_have() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        {
            let alice_player = game.player_mut(alice).expect("alice exists");
            alice_player.poison_counters = 1;
            alice_player.energy_counters = 2;
            alice_player.experience_counters = 3;
        }
        game.player_mut(bob).expect("bob exists").energy_counters = 5;

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DoubleCountersEffect::new(None, ChooseSpec::SourceController);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("Aetheric Amplifier player mode should resolve");

        let alice_player = game.player(alice).expect("alice exists");
        assert_eq!(alice_player.poison_counters, 2);
        assert_eq!(alice_player.energy_counters, 4);
        assert_eq!(alice_player.experience_counters, 6);
        assert_eq!(game.player(bob).expect("bob exists").energy_counters, 5);
        assert_eq!(outcome.events.len(), 3);
    }

    #[test]
    fn aetheric_amplifier_player_mode_does_not_create_absent_counter_kinds() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.player_mut(alice)
            .expect("alice exists")
            .energy_counters = 2;

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DoubleCountersEffect::new(None, ChooseSpec::SourceController);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("Aetheric Amplifier player mode should resolve");

        let alice_player = game.player(alice).expect("alice exists");
        assert_eq!(alice_player.poison_counters, 0);
        assert_eq!(alice_player.energy_counters, 4);
        assert_eq!(alice_player.experience_counters, 0);
        assert_eq!(outcome.events.len(), 1);
    }
}
