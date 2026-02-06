//! Put counters effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_from_spec, resolve_value};
use crate::event_processor::process_put_counters_with_event;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

/// Effect that puts counters on a target permanent.
///
/// Supports replacement effects like Doubling Season and Hardened Scales.
///
/// # Fields
///
/// * `counter_type` - The type of counter to put
/// * `count` - How many counters to put
/// * `target` - Which permanent to target
/// * `target_count` - How many targets (for "up to" effects)
///
/// # Example
///
/// ```ignore
/// // Put two +1/+1 counters on target creature
/// let effect = PutCountersEffect::new(
///     CounterType::PlusOnePlusOne,
///     2,
///     ChooseSpec::creature(),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PutCountersEffect {
    /// The type of counter to put.
    pub counter_type: CounterType,
    /// How many counters to put.
    pub count: Value,
    /// Which permanent to target.
    pub target: ChooseSpec,
    /// How many targets. None defaults to exactly 1.
    pub target_count: Option<ChoiceCount>,
}

impl PutCountersEffect {
    /// Create a new put counters effect.
    pub fn new(counter_type: CounterType, count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            counter_type,
            count: count.into(),
            target,
            target_count: None,
        }
    }

    /// Create a put counters effect with target count specification.
    pub fn with_target_count(mut self, target_count: ChoiceCount) -> Self {
        self.target_count = Some(target_count);
        self
    }

    /// Create an effect that puts +1/+1 counters on target creature.
    pub fn plus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(CounterType::PlusOnePlusOne, count, target)
    }

    /// Create an effect that puts -1/-1 counters on target creature.
    pub fn minus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(CounterType::MinusOneMinusOne, count, target)
    }

    /// Create an effect that puts counters on the source.
    pub fn on_source(counter_type: CounterType, count: impl Into<Value>) -> Self {
        Self::new(counter_type, count, ChooseSpec::Source)
    }
}

impl EffectExecutor for PutCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle Source target specially (for abilities like level-up that target themselves)
        let target_id = match &self.target {
            ChooseSpec::Source => ctx.source,
            _ => match resolve_objects_from_spec(game, &self.target, ctx) {
                Ok(objects) if !objects.is_empty() => objects[0],
                _ => {
                    // No target chosen (valid for "up to" effects)
                    return Ok(EffectOutcome::resolved());
                }
            },
        };

        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        // Process through replacement effects (e.g., Melira, Doubling Season)
        let final_count =
            process_put_counters_with_event(game, target_id, self.counter_type, count);

        if final_count == 0 {
            // Counters were prevented
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }

        // Use centralized method which handles counter addition, timestamp recording, and event creation
        match game.add_counters_with_source(
            target_id,
            self.counter_type,
            final_count,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            Some(event) => Ok(EffectOutcome::count(final_count as i32).with_event(event)),
            None => Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target for counters"
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        self.target_count
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

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_put_plus_one_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutCountersEffect::plus_one_counters(2, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&2));
    }

    #[test]
    fn test_put_counters_on_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Walking Ballista", alice);

        let mut ctx = ExecutionContext::new_default(creature_id, alice);

        let effect = PutCountersEffect::on_source(CounterType::PlusOnePlusOne, 3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3));
    }

    #[test]
    fn test_put_minus_one_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutCountersEffect::minus_one_counters(1, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::MinusOneMinusOne), Some(&1));
    }

    #[test]
    fn test_put_counters_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // For "up to" effects, no target is valid
        let effect = PutCountersEffect::plus_one_counters(1, ChooseSpec::creature())
            .with_target_count(ChoiceCount::up_to(1));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should resolve without doing anything
        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_put_counters_adds_to_existing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);

        // Give it 2 counters initially
        if let Some(obj) = game.object_mut(creature_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 2);
        }

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutCountersEffect::plus_one_counters(3, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&5)); // 2 + 3
    }

    #[test]
    fn test_put_counters_clone_box() {
        let effect = PutCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("PutCountersEffect"));
    }

    #[test]
    fn test_put_counters_get_target_spec() {
        let effect = PutCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        assert!(effect.get_target_spec().is_some());
    }

    #[test]
    fn test_put_counters_returns_event() {
        use crate::events::EventKind;
        use crate::events::other::MarkersChangedEvent;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutCountersEffect::plus_one_counters(2, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind(), EventKind::MarkersChanged);

        // Verify event contains correct data
        let event = result.events[0].downcast::<MarkersChangedEvent>().unwrap();
        assert_eq!(event.amount, 2);
    }
}
