//! Monstrosity effect implementation.

use crate::effect::{Effect, EffectOutcome, EffectResult, Value};
use crate::effects::helpers::resolve_value;
use crate::effects::{EffectExecutor, PutCountersEffect};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

/// Effect that makes a creature monstrous.
///
/// Monstrosity N is an activated ability that, when resolved:
/// 1. Checks if the creature is already monstrous (if so, does nothing)
/// 2. Puts N +1/+1 counters on the creature
/// 3. Marks the creature as monstrous
///
/// This enables "When this creature becomes monstrous" triggered abilities.
///
/// # Fields
///
/// * `n` - The number of +1/+1 counters to put on the creature
///
/// # Example
///
/// ```ignore
/// // Monstrosity 3
/// let effect = MonstrosityEffect::new(3);
///
/// // Monstrosity X (where X was chosen when activating)
/// let effect = MonstrosityEffect::new(Value::X);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MonstrosityEffect {
    /// The number of +1/+1 counters to add.
    pub n: Value,
}

impl MonstrosityEffect {
    /// Create a new monstrosity effect.
    pub fn new(n: impl Into<Value>) -> Self {
        Self { n: n.into() }
    }
}

impl EffectExecutor for MonstrosityEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let n_value = resolve_value(game, &self.n, ctx)?.max(0) as u32;

        // Monstrosity targets the source (the creature with the ability)
        let source_id = ctx.source;

        // Check if already monstrous
        if game.object(source_id).is_none() {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }
        if game.is_monstrous(source_id) {
            // Already monstrous - do nothing
            return Ok(EffectOutcome::count(0));
        }

        // Put N +1/+1 counters on it and mark as monstrous
        if n_value > 0 {
            let original_targets = ctx.targets.clone();
            ctx.targets = vec![ResolvedTarget::Object(source_id)];
            let counters_effect =
                PutCountersEffect::new(CounterType::PlusOnePlusOne, n_value, ChooseSpec::AnyTarget);
            let counters_outcome = execute_effect(game, &Effect::new(counters_effect), ctx)?;
            ctx.targets = original_targets;

            if let EffectResult::Count(n) = counters_outcome.result
                && n > 0
            {
                game.continuous_effects.record_counter_change(source_id);
            }
        }
        game.set_monstrous(source_id);

        // Return a special result that indicates monstrosity happened
        // The game loop will need to generate the BecameMonstrous event
        Ok(EffectOutcome::from_result(
            EffectResult::MonstrosityApplied {
                creature: source_id,
                n: n_value,
            },
        ))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
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
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
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
    fn test_monstrosity_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Polukranos", alice);

        let mut ctx = ExecutionContext::new_default(creature_id, alice);

        let effect = MonstrosityEffect::new(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return MonstrosityApplied
        match result.result {
            EffectResult::MonstrosityApplied { creature, n } => {
                assert_eq!(creature, creature_id);
                assert_eq!(n, 5);
            }
            _ => panic!("Expected MonstrosityApplied result"),
        }

        // Creature should have 5 +1/+1 counters
        let obj = game.object(creature_id).unwrap();
        assert_eq!(
            *obj.counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0),
            5
        );

        // Creature should be monstrous
        assert!(game.is_monstrous(creature_id));
    }

    #[test]
    fn test_monstrosity_already_monstrous() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Polukranos", alice);

        // Mark as already monstrous with some counters
        game.set_monstrous(creature_id);
        if let Some(obj) = game.object_mut(creature_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 3);
        }

        let mut ctx = ExecutionContext::new_default(creature_id, alice);

        let effect = MonstrosityEffect::new(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return Count(0) - nothing happened
        assert_eq!(result.result, EffectResult::Count(0));

        // Counters should be unchanged
        let obj = game.object(creature_id).unwrap();
        assert_eq!(
            *obj.counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0),
            3
        );
    }

    #[test]
    fn test_monstrosity_with_x() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Mistcutter Hydra", alice);

        let mut ctx = ExecutionContext::new_default(creature_id, alice).with_x(7);

        let effect = MonstrosityEffect::new(Value::X);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should have 7 counters
        match result.result {
            EffectResult::MonstrosityApplied { n, .. } => {
                assert_eq!(n, 7);
            }
            _ => panic!("Expected MonstrosityApplied result"),
        }

        let obj = game.object(creature_id).unwrap();
        assert_eq!(
            *obj.counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0),
            7
        );
    }

    #[test]
    fn test_monstrosity_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Weird Creature", alice);

        let mut ctx = ExecutionContext::new_default(creature_id, alice);

        let effect = MonstrosityEffect::new(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should still become monstrous even with 0 counters
        match result.result {
            EffectResult::MonstrosityApplied { n, .. } => {
                assert_eq!(n, 0);
            }
            _ => panic!("Expected MonstrosityApplied result"),
        }

        assert!(game.is_monstrous(creature_id));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(
            *obj.counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0),
            0
        );
    }

    #[test]
    fn test_monstrosity_source_not_found() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let nonexistent_id = ObjectId::from_raw(99999);

        let mut ctx = ExecutionContext::new_default(nonexistent_id, alice);

        let effect = MonstrosityEffect::new(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Source doesn't exist - target invalid
        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_monstrosity_clone_box() {
        let effect = MonstrosityEffect::new(5);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MonstrosityEffect"));
    }
}
