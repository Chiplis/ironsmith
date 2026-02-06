//! Remove up to counters effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{NumberSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{find_target_object, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

/// Effect that removes up to a number of counters from a target permanent.
///
/// The player chooses how many counters to remove (0 to max).
///
/// # Fields
///
/// * `counter_type` - The type of counter to remove
/// * `max_count` - Maximum counters the player can choose to remove
/// * `target` - Which permanent to target
///
/// # Example
///
/// ```ignore
/// // Remove up to two +1/+1 counters from target creature
/// let effect = RemoveUpToCountersEffect::new(
///     CounterType::PlusOnePlusOne,
///     2,
///     ChooseSpec::creature(),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveUpToCountersEffect {
    /// The type of counter to remove.
    pub counter_type: CounterType,
    /// Maximum counters to remove.
    pub max_count: Value,
    /// Which permanent to target.
    pub target: ChooseSpec,
}

impl RemoveUpToCountersEffect {
    /// Create a new remove up to counters effect.
    pub fn new(counter_type: CounterType, max_count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            counter_type,
            max_count: max_count.into(),
            target,
        }
    }

    /// Create an effect that removes up to N +1/+1 counters.
    pub fn plus_one_counters(max_count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(CounterType::PlusOnePlusOne, max_count, target)
    }
}

impl EffectExecutor for RemoveUpToCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = find_target_object(&ctx.targets)?;
        let max_count = resolve_value(game, &self.max_count, ctx)?.max(0) as u32;

        // Get the current count of counters on the target
        let available = game
            .object(target_id)
            .map(|obj| obj.counters.get(&self.counter_type).copied().unwrap_or(0))
            .unwrap_or(0);

        // The actual maximum we can remove is the lesser of max_count and available
        let actual_max = max_count.min(available);

        // If there's nothing to remove, return 0
        if actual_max == 0 {
            return Ok(EffectOutcome::count(0));
        }

        // Ask the player how many counters to remove (0 to actual_max)
        let description = format!(
            "Choose how many {} counters to remove (0-{})",
            format!("{:?}", self.counter_type).to_lowercase(),
            actual_max
        );
        let spec = NumberSpec::up_to(ctx.source, actual_max, description);
        let chosen_count = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
            FallbackStrategy::Maximum,
        )
        .min(actual_max);

        // Remove the chosen number of counters using centralized method
        match game.remove_counters(
            target_id,
            self.counter_type,
            chosen_count,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            Some((removed, event)) => Ok(EffectOutcome::count(removed as i32).with_event(event)),
            None => Ok(EffectOutcome::count(0)),
        }
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

    fn create_creature_with_counters(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        counter_type: CounterType,
        count: u32,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.counters.insert(counter_type, count);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_remove_up_to_counters_default_max() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // No decision maker, so defaults to removing maximum
        let effect = RemoveUpToCountersEffect::plus_one_counters(3, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&2)); // 5 - 3
    }

    #[test]
    fn test_remove_up_to_limited_by_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Request up to 5, but only 2 available
        let effect = RemoveUpToCountersEffect::plus_one_counters(5, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2)); // Limited by available
        // When all counters are removed, the entry is removed from the HashMap
        assert_eq!(
            game.counter_count(creature_id, CounterType::PlusOnePlusOne),
            0
        );
    }

    #[test]
    fn test_remove_up_to_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Grizzly Bears",
            alice,
            CounterType::PlusOnePlusOne,
            0,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RemoveUpToCountersEffect::plus_one_counters(3, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_remove_up_to_counters_clone_box() {
        let effect = RemoveUpToCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("RemoveUpToCountersEffect"));
    }
}
