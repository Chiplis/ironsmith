//! Double counters on selected objects.

use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_players_from_spec};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
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
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        game.clear_pending_decision_controllers();
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                // Capture every authored amount before any placement or its
                // replacement additions can change a later recipient's counters.
                let mut requests = Vec::new();
                if double_counters_targets_players(&self.target) {
                    for player_id in resolve_players_from_spec(game, &self.target, ctx)? {
                        for (counter_type, count) in
                            player_counter_counts(game, player_id, self.counter_type)?
                        {
                            requests.push(
                                crate::events::Event::put_player_counters(
                                    player_id,
                                    counter_type,
                                    count,
                                    ctx.cause.clone(),
                                )
                                .with_provenance(ctx.provenance),
                            );
                        }
                    }
                } else {
                    for target_id in resolve_objects_for_effect(game, ctx, &self.target)? {
                        if let Some(object) = game.object(target_id) {
                            for (&counter_type, &count) in &object.counters {
                                if count > 0
                                    && self
                                        .counter_type
                                        .is_none_or(|wanted| wanted == counter_type)
                                {
                                    requests.push(
                                        crate::events::Event::put_counters(
                                            target_id,
                                            counter_type,
                                            count,
                                            ctx.cause.clone(),
                                        )
                                        .with_provenance(ctx.provenance),
                                    );
                                }
                            }
                        }
                    }
                }
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                // The shared owner prepares replacements, commits and groups
                // originals, freezes observations, then runs deferred additions.
                let children = super::execute_counter_batch_with_outputs(game, ctx, requests)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let outcome = if children.is_empty() {
                    EffectOutcome::resolved()
                } else {
                    EffectOutcome::aggregate_summing_counts(
                        children.iter().map(|child| child.outcome.clone()),
                    )
                };
                let mut outputs = CompletedEffectOutputs::aggregate_only(outcome);
                for child in children {
                    outputs.retain_owned_child(child);
                }
                Ok(outputs)
            },
        )
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
