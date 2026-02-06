//! Move counters effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_value;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

/// Effect that moves counters from one permanent to another.
///
/// # Fields
///
/// * `counter_type` - The type of counter to move
/// * `count` - How many counters to move
/// * `from` - Source permanent (first target)
/// * `to` - Destination permanent (second target)
///
/// # Example
///
/// ```ignore
/// // Move two +1/+1 counters from one creature to another
/// let effect = MoveCountersEffect::new(
///     CounterType::PlusOnePlusOne,
///     2,
///     ChooseSpec::creature(),
///     ChooseSpec::creature(),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MoveCountersEffect {
    /// The type of counter to move.
    pub counter_type: CounterType,
    /// How many counters to move.
    pub count: Value,
    /// Source permanent (first target).
    pub from: ChooseSpec,
    /// Destination permanent (second target).
    pub to: ChooseSpec,
}

impl MoveCountersEffect {
    /// Create a new move counters effect.
    pub fn new(
        counter_type: CounterType,
        count: impl Into<Value>,
        from: ChooseSpec,
        to: ChooseSpec,
    ) -> Self {
        Self {
            counter_type,
            count: count.into(),
            from,
            to,
        }
    }

    /// Create an effect that moves +1/+1 counters between creatures.
    pub fn plus_one_counters(count: impl Into<Value>) -> Self {
        Self::new(
            CounterType::PlusOnePlusOne,
            count,
            ChooseSpec::creature(),
            ChooseSpec::creature(),
        )
    }
}

impl EffectExecutor for MoveCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        // Get from and to targets from resolved targets
        if ctx.targets.len() < 2 {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        let from_id = match &ctx.targets[0] {
            ResolvedTarget::Object(id) => *id,
            _ => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };

        let to_id = match &ctx.targets[1] {
            ResolvedTarget::Object(id) => *id,
            _ => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };

        // Get current counter count on source
        let available = game
            .object(from_id)
            .and_then(|obj| obj.counters.get(&self.counter_type).copied())
            .unwrap_or(0);

        let to_move = count.min(available);

        if to_move == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcome = EffectOutcome::count(to_move as i32);

        // Remove from source using centralized method
        if let Some((_, remove_event)) = game.remove_counters(
            from_id,
            self.counter_type,
            to_move,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            outcome = outcome.with_event(remove_event);
        }

        // Add to target using centralized method
        if let Some(add_event) = game.add_counters_with_source(
            to_id,
            self.counter_type,
            to_move,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            outcome = outcome.with_event(add_event);
        }

        Ok(outcome)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.from)
    }

    fn target_description(&self) -> &'static str {
        "creature to move counters from"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
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

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_move_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_counters(
            &mut game,
            "Source Creature",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveCountersEffect::plus_one_counters(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));

        let from_obj = game.object(from_id).unwrap();
        assert_eq!(
            from_obj.counters.get(&CounterType::PlusOnePlusOne),
            Some(&2)
        ); // 5 - 3

        let to_obj = game.object(to_id).unwrap();
        assert_eq!(to_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3));
    }

    #[test]
    fn test_move_counters_limited_by_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_counters(
            &mut game,
            "Source Creature",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        // Request 5 but only 2 available
        let effect = MoveCountersEffect::plus_one_counters(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2)); // Limited by available

        // When all counters are removed, the entry is removed from the HashMap
        assert_eq!(game.counter_count(from_id, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(to_id, CounterType::PlusOnePlusOne), 2);
    }

    #[test]
    fn test_move_counters_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature(&mut game, "Source Creature", alice);
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveCountersEffect::plus_one_counters(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_move_counters_insufficient_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_counters(
            &mut game,
            "Source Creature",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let source = game.new_object_id();

        // Only one target provided
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(from_id)]);

        let effect = MoveCountersEffect::plus_one_counters(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_move_counters_clone_box() {
        let effect = MoveCountersEffect::plus_one_counters(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MoveCountersEffect"));
    }
}
