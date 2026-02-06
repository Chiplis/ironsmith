//! Remove up to any counters effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{CounterRemovalSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{find_target_object, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

/// Effect that removes up to a number of counters of ANY type from a target.
///
/// Used by cards like Hex Parasite. The player chooses which counters to remove.
///
/// # Fields
///
/// * `max_count` - Maximum total counters the player can choose to remove
/// * `target` - Which permanent to target
///
/// # Example
///
/// ```ignore
/// // Remove up to X counters from target permanent
/// let effect = RemoveUpToAnyCountersEffect::new(Value::X, ChooseSpec::permanent());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveUpToAnyCountersEffect {
    /// Maximum total counters to remove.
    pub max_count: Value,
    /// Which permanent to target.
    pub target: ChooseSpec,
}

impl RemoveUpToAnyCountersEffect {
    /// Create a new remove up to any counters effect.
    pub fn new(max_count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            max_count: max_count.into(),
            target,
        }
    }
}

impl EffectExecutor for RemoveUpToAnyCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = find_target_object(&ctx.targets)?;
        let max_count = resolve_value(game, &self.max_count, ctx)?.max(0) as u32;

        // Get available counters on the target
        let available_counters: Vec<(CounterType, u32)> = game
            .object(target_id)
            .map(|obj| {
                obj.counters
                    .iter()
                    .filter(|(_, count)| **count > 0)
                    .map(|(ct, count)| (*ct, *count))
                    .collect()
            })
            .unwrap_or_default();

        // Count total counters available
        let total_counters: u32 = available_counters.iter().map(|(_, c)| c).sum();

        // The actual maximum we can remove is the lesser of max_count and total available
        let actual_max = max_count.min(total_counters);

        // If there's nothing to remove, return 0
        if actual_max == 0 {
            return Ok(EffectOutcome::count(0));
        }

        // Ask the player which counters to remove using the spec-based system
        let spec = CounterRemovalSpec::new(
            ctx.source,
            target_id,
            actual_max,
            available_counters.clone(),
        );
        let selections = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
            FallbackStrategy::Maximum,
        );

        // Validate and apply the selections using centralized method
        let mut total_removed = 0u32;
        let mut outcome = EffectOutcome::count(0);

        for (counter_type, to_remove) in selections {
            // Validate: can't remove more than max_total
            if total_removed >= actual_max {
                break;
            }
            let remaining = actual_max - total_removed;
            let amount_to_remove = to_remove.min(remaining);

            if let Some((removed, event)) = game.remove_counters(
                target_id,
                counter_type,
                amount_to_remove,
                Some(ctx.source),
                Some(ctx.controller),
            ) {
                outcome = outcome.with_event(event);
                total_removed += removed;
            }
        }

        outcome.result = crate::effect::EffectResult::Count(total_removed as i32);
        Ok(outcome)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to remove counters from"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
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

        assert_eq!(result.result, EffectResult::Count(4));
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

        assert_eq!(result.result, EffectResult::Count(5)); // Limited by available
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

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_remove_up_to_any_counters_clone_box() {
        let effect = RemoveUpToAnyCountersEffect::new(1, ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("RemoveUpToAnyCountersEffect"));
    }
}
