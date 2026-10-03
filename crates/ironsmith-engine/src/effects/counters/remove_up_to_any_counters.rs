//! Remove up to any counters effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{CounterRemovalSpec, DecisionSpec as _, make_decision_with_fallback};
use crate::effect::EffectOutcome;
use crate::effects::helpers::{
    resolve_single_object_for_effect, resolve_single_target_from_spec, resolve_value,
};
use crate::effects::{EffectExecutor, RemoveAnyCountersAmongEffect};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::{GameState, Target};
use crate::object::CounterType;
use crate::target::ChooseSpec;
pub use ironsmith_core::RemoveUpToAnyCountersEffect;

/// Effect that removes up to a number of counters of ANY type from a target.
///
/// Used by cards like Hex Parasite. The player chooses which counters to remove.
///
/// # Fields
///
/// * `max_count` - Maximum total counters the player can choose to remove
/// * `target` - Which permanent or player to target
///
/// # Example
///
/// ```ignore
/// // Remove up to X counters from target permanent
/// let effect = RemoveUpToAnyCountersEffect::new(Value::X, ChooseSpec::permanent());
/// ```
impl EffectExecutor for RemoveUpToAnyCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = execute_up_to_any_counter_removal(self, game, ctx);
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice());
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to remove counters from"
    }
}

fn execute_up_to_any_counter_removal(effect: &RemoveUpToAnyCountersEffect, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        let max_count = resolve_value(game, &effect.max_count, ctx)?.max(0) as u32;
        if let ChooseSpec::All(filter) = effect.target.unhinted() {
            let min_count = if effect.up_to { 0 } else { max_count };
            let distributed =
                RemoveAnyCountersAmongEffect::dynamic(min_count, max_count, filter.clone(), false);
            return distributed.execute(game, ctx);
        }
        let target = match effect.target.base() {
            ChooseSpec::Player(_)
            | ChooseSpec::SpecificPlayer(_)
            | ChooseSpec::AnyTarget
            | ChooseSpec::AnyOtherTarget
            | ChooseSpec::ObjectOrPlayer(_, _)
            | ChooseSpec::PlayerOrPlaneswalker(_)
            | ChooseSpec::AttackedPlayerOrPlaneswalker
            | ChooseSpec::SourceController
            | ChooseSpec::SourceOwner
            | ChooseSpec::EachPlayer(_) => {
                resolve_single_target_from_spec(game, &effect.target, ctx)?
            }
            _ => ResolvedTarget::Object(resolve_single_object_for_effect(game, ctx, &effect.target)?),
        };

        // Get available counters on the target
        let available_counters: Vec<(CounterType, u32)> = match target {
            ResolvedTarget::Object(target_id) => game.object(target_id).map(|object| {
                object
                    .counters
                    .iter()
                    .filter(|(_, count)| **count > 0)
                    .map(|(counter_type, count)| (*counter_type, *count))
                    .collect()
            }),
            ResolvedTarget::Player(target_player) => game.player(target_player).map(|player| {
                player
                    .counter_types_with_counters()
                    .into_iter()
                    .map(|counter_type| (counter_type, player.counter_count(counter_type)))
                    .collect()
            }),
        }
        .unwrap_or_default();

        // Count total counters available
        let total_counters: u32 = available_counters.iter().try_fold(0u32, |total, (_, count)| total.checked_add(*count)
            .ok_or_else(|| ExecutionError::InternalError("available counter total overflow".into())))?;

        // The actual maximum we can remove is the lesser of max_count and total available
        let actual_max = max_count.min(total_counters);

        // If there's nothing to remove, return 0
        if actual_max == 0 {
            return Ok(EffectOutcome::count(0));
        }

        // Ask the player which counters to remove using the spec-based system
        let min_count = if effect.up_to { 0 } else { actual_max };
        let decision_target = match target {
            ResolvedTarget::Object(id) => Target::Object(id),
            ResolvedTarget::Player(id) => Target::Player(id),
        };
        let spec = CounterRemovalSpec::for_target(
            ctx.source,
            decision_target,
            actual_max,
            available_counters.clone(),
        )
        .with_min_total(min_count);
        let mandatory_fallback = spec.default_response(FallbackStrategy::Maximum);
        let mut selections = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
            FallbackStrategy::Maximum,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if selections.iter().try_fold(0u32, |total, (_, count)| total.checked_add(*count)
            .ok_or_else(|| ExecutionError::InternalError("selected counter total overflow".into())))? < min_count {
            selections = mandatory_fallback;
        }

        let mut selected_total = 0u32;
        let mut removed_total = 0u32;
        let mut outcomes = Vec::new();
        for (counter_type, requested) in selections {
            if selected_total >= actual_max { break; }
            let amount = requested.min(actual_max - selected_total);
            if amount == 0 { continue; }
            let outcome = match target {
                ResolvedTarget::Object(target_id) => {
                    let event = crate::events::Event::remove_counters(target_id, counter_type, amount)
                        .with_provenance(ctx.provenance);
                    super::remove_counters::execute_counter_removal_event(game, ctx, event)?
                }
                // Player-counter removal still lacks a replaceable player carrier.
                // Preserve its existing primitive path until that model is migrated.
                ResolvedTarget::Player(player) => {
                    if let Some((removed, event)) = game.remove_player_counters_with_source(player, counter_type, amount, Some(ctx.source), Some(ctx.controller)) {
                        let count = i32::try_from(removed).map_err(|_| ExecutionError::InternalError("player removal exceeds the supported outcome range".into()))?;
                        EffectOutcome::count(count).with_event(event)
                    } else { EffectOutcome::count(0) }
                }
            };
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            let removed = u32::try_from(outcome.count_or_zero()).map_err(|_| ExecutionError::InternalError("invalid counter-removal count".into()))?;
            removed_total = removed_total.checked_add(removed).ok_or_else(|| ExecutionError::InternalError("counter-removal total overflow".into()))?;
            selected_total += amount;
            outcomes.push(outcome);
        }
        let count = i32::try_from(removed_total).map_err(|_| ExecutionError::InternalError("counter-removal total exceeds outcome range".into()))?;
        let mut outcome = EffectOutcome::aggregate(outcomes);
        outcome.set_value(crate::effect::OutcomeValue::Count(count));
        Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_with_multiple_counters(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.counters.insert(CounterType::PlusOnePlusOne, 3);
        obj.counters.insert(CounterType::MinusOneMinusOne, 2);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_remove_up_to_any_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_multiple_counters(&mut game, "Test Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Remove up to 4 counters of any type
        let effect = RemoveUpToAnyCountersEffect::new(4, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(4));
        let obj = game.object(creature_id).unwrap();
        // Default removes from first types in order
        let total_remaining: u32 = obj.counters.values().sum();
        assert_eq!(total_remaining, 1); // Started with 5, removed 4
    }

    #[test]
    fn test_remove_up_to_any_limited_by_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_multiple_counters(&mut game, "Test Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Request up to 10, but only 5 available (3 + 2)
        let effect = RemoveUpToAnyCountersEffect::new(10, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5)); // Limited by available
        let obj = game.object(creature_id).unwrap();
        let total_remaining: u32 = obj.counters.values().sum();
        assert_eq!(total_remaining, 0);
    }

    #[test]
    fn test_remove_up_to_any_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, "Empty Creature");
        let obj = Object::from_card(id, &card, alice, Zone::Battlefield);
        game.add_object(obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(id)]);

        let effect = RemoveUpToAnyCountersEffect::new(5, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_remove_up_to_any_counters_from_target_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let bob_state = game.player_mut(bob).expect("Bob should exist");
        bob_state.poison_counters = 3;
        bob_state.energy_counters = 4;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let target = ChooseSpec::target(ChooseSpec::ObjectOrPlayer(
            crate::target::ObjectFilter::default()
                .with_type(CardType::Artifact)
                .with_type(CardType::Creature)
                .with_type(CardType::Planeswalker),
            crate::target::PlayerFilter::Opponent,
        ));
        let effect = RemoveUpToAnyCountersEffect::new(5, target);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5));
        let bob_state = game.player(bob).expect("Bob should remain in the game");
        assert_eq!(bob_state.poison_counters, 0);
        assert_eq!(bob_state.energy_counters, 2);
    }

    #[test]
    fn test_remove_up_to_any_counters_clone_box() {
        let effect = RemoveUpToAnyCountersEffect::new(1, ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("RemoveUpToAnyCountersEffect"));
    }
}
