//! Modify power/toughness for all creatures effect implementation.

use crate::continuous::{EffectSourceType, EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, Until, Value};
use crate::effects::helpers::resolve_value;
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ObjectFilter;
use crate::zone::Zone;

/// Effect that modifies power and toughness for all creatures matching a filter.
///
/// Creates a continuous effect that applies to all matching creatures for the specified duration.
///
/// # Fields
///
/// * `filter` - Which creatures to modify
/// * `power` - Power modifier
/// * `toughness` - Toughness modifier
///
/// # Example
///
/// ```ignore
/// // Creatures you control get +1/+1 until end of turn
/// let effect = ModifyPowerToughnessAllEffect::new(
///     ObjectFilter::creature().you_control(),
///     1,
///     1,
///     Until::EndOfTurn,
/// );
///
/// // All creatures get -1/-1 until end of turn
/// let effect = ModifyPowerToughnessAllEffect::new(
///     ObjectFilter::creature(),
///     -1,
///     -1,
///     Until::EndOfTurn,
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModifyPowerToughnessAllEffect {
    /// Which creatures to modify.
    pub filter: ObjectFilter,
    /// Power modifier.
    pub power: Value,
    /// Toughness modifier.
    pub toughness: Value,
    /// Duration for the modification.
    pub duration: Until,
}

impl ModifyPowerToughnessAllEffect {
    /// Create a new modify power/toughness all effect with explicit duration.
    pub fn new(
        filter: ObjectFilter,
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        duration: Until,
    ) -> Self {
        Self {
            filter,
            power: power.into(),
            toughness: toughness.into(),
            duration,
        }
    }

    /// All creatures get +X/+X with explicit duration.
    pub fn all_creatures(
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        duration: Until,
    ) -> Self {
        Self::new(ObjectFilter::creature(), power, toughness, duration)
    }

    /// Creatures you control get +X/+X with explicit duration.
    pub fn your_creatures(
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        duration: Until,
    ) -> Self {
        Self::new(
            ObjectFilter::creature().you_control(),
            power,
            toughness,
            duration,
        )
    }
}

impl EffectExecutor for ModifyPowerToughnessAllEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let power_mod = resolve_value(game, &self.power, ctx)?;
        let toughness_mod = resolve_value(game, &self.toughness, ctx)?;

        // Per MTG Rule 611.2c, effects from resolving spells/abilities lock their
        // targets at resolution time. We capture which objects match the filter NOW,
        // and the effect will only apply to those specific objects.
        let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
        let locked_targets: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.zone == Zone::Battlefield)
            .filter(|obj| self.filter.matches(obj, &filter_ctx, game))
            .map(|obj| obj.id)
            .collect();

        // Register a continuous effect with locked targets (Resolution source type).
        let apply = ApplyContinuousEffect::new(
            EffectTarget::Filter(self.filter.clone()),
            Modification::ModifyPowerToughness {
                power: power_mod,
                toughness: toughness_mod,
            },
            self.duration.clone(),
        )
        .with_source_type(EffectSourceType::Resolution { locked_targets });

        execute_effect(game, &Effect::new(apply), ctx)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_card(
        card_id: u32,
        name: &str,
        power: i32,
        toughness: i32,
    ) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        power: i32,
        toughness: i32,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name, power, toughness);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_modify_all_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let _c1 = create_creature(&mut game, "Creature 1", 2, 2, alice);
        let _c2 = create_creature(&mut game, "Creature 2", 3, 3, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ModifyPowerToughnessAllEffect::all_creatures(1, 1, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.continuous_effects.effects_sorted().len(), 1);
    }

    #[test]
    fn test_modify_your_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let _alice_creature = create_creature(&mut game, "Alice's Creature", 2, 2, alice);
        let _bob_creature = create_creature(&mut game, "Bob's Creature", 2, 2, bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ModifyPowerToughnessAllEffect::your_creatures(2, 2, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // The continuous effect will only apply to creatures matching the filter
        assert_eq!(game.continuous_effects.effects_sorted().len(), 1);
    }

    #[test]
    fn test_modify_negative() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let _c1 = create_creature(&mut game, "Creature 1", 2, 2, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ModifyPowerToughnessAllEffect::all_creatures(-1, -1, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_modify_variable_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let _c1 = create_creature(&mut game, "Creature", 2, 2, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_x(4);
        let effect = ModifyPowerToughnessAllEffect::new(
            ObjectFilter::creature(),
            Value::X,
            Value::X,
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_modify_no_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ModifyPowerToughnessAllEffect::all_creatures(1, 1, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should succeed even with no creatures (creates the effect, but it won't apply to anything)
        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_modify_clone_box() {
        let effect = ModifyPowerToughnessAllEffect::all_creatures(1, 1, Until::EndOfTurn);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ModifyPowerToughnessAllEffect"));
    }

    #[test]
    fn test_target_locking_at_resolution() {
        // Per MTG Rule 611.2c, effects from resolving spells lock their targets
        // at resolution time. New creatures entering afterward shouldn't be affected.
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two creatures BEFORE the effect
        let c1 = create_creature(&mut game, "Creature 1", 2, 2, alice);
        let c2 = create_creature(&mut game, "Creature 2", 3, 3, alice);
        let source = game.new_object_id();

        // Execute the effect - should lock to existing creatures
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ModifyPowerToughnessAllEffect::all_creatures(1, 1, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Resolved);

        // Verify the effect has locked targets
        let effects = game.continuous_effects.effects_sorted();
        assert_eq!(effects.len(), 1);
        match &effects[0].source_type {
            EffectSourceType::Resolution { locked_targets } => {
                // Should have captured both creatures
                assert_eq!(locked_targets.len(), 2);
                assert!(locked_targets.contains(&c1));
                assert!(locked_targets.contains(&c2));
            }
            _ => panic!("Expected Resolution source type with locked targets"),
        }

        // Create a creature AFTER the effect resolved
        let c3 = create_creature(&mut game, "Creature 3", 4, 4, alice);

        // The new creature should NOT be in the locked targets
        let effects = game.continuous_effects.effects_sorted();
        match &effects[0].source_type {
            EffectSourceType::Resolution { locked_targets } => {
                assert!(!locked_targets.contains(&c3));
            }
            _ => panic!("Expected Resolution source type"),
        }
    }
}
