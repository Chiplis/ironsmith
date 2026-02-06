//! Remove counters effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{find_target_object, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

/// Effect that removes counters from a target permanent.
///
/// # Fields
///
/// * `counter_type` - The type of counter to remove
/// * `count` - How many counters to remove
/// * `target` - Which permanent to target
///
/// # Example
///
/// ```ignore
/// // Remove two +1/+1 counters from target creature
/// let effect = RemoveCountersEffect::new(
///     CounterType::PlusOnePlusOne,
///     2,
///     ChooseSpec::creature(),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveCountersEffect {
    /// The type of counter to remove.
    pub counter_type: CounterType,
    /// How many counters to remove.
    pub count: Value,
    /// Which permanent to target.
    pub target: ChooseSpec,
}

impl RemoveCountersEffect {
    /// Create a new remove counters effect.
    pub fn new(counter_type: CounterType, count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            counter_type,
            count: count.into(),
            target,
        }
    }

    /// Create an effect that removes +1/+1 counters from target creature.
    pub fn plus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(CounterType::PlusOnePlusOne, count, target)
    }

    /// Create an effect that removes -1/-1 counters from target creature.
    pub fn minus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(CounterType::MinusOneMinusOne, count, target)
    }
}

impl EffectExecutor for RemoveCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = find_target_object(&ctx.targets)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        // Verify the object exists
        if game.object(target_id).is_none() {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        // Use centralized method which emits events and returns actual count removed
        match game.remove_counters(
            target_id,
            self.counter_type,
            count,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            Some((removed, event)) => Ok(EffectOutcome::count(removed as i32).with_event(event)),
            None => {
                // No counters were removed (object had 0 of this counter type)
                Ok(EffectOutcome::count(0))
            }
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
    fn test_remove_counters() {
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

        let effect = RemoveCountersEffect::plus_one_counters(2, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3)); // 5 - 2
    }

    #[test]
    fn test_remove_more_than_available() {
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

        let effect = RemoveCountersEffect::plus_one_counters(5, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only 2 counters were available to remove
        assert_eq!(result.result, EffectResult::Count(2));
        // When all counters are removed, the entry is removed from the HashMap
        assert_eq!(
            game.counter_count(creature_id, CounterType::PlusOnePlusOne),
            0
        );
    }

    #[test]
    fn test_remove_from_no_counters() {
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

        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // No counters to remove
        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_remove_counters_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_remove_counters_clone_box() {
        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("RemoveCountersEffect"));
    }

    #[test]
    fn test_remove_counters_get_target_spec() {
        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        assert!(effect.get_target_spec().is_some());
    }
}
