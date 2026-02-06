//! Modify power/toughness effect implementation.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, EffectResult, Until, Value};
use crate::effects::helpers::{find_target_object, resolve_value};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::types::CardType;

/// Effect that modifies a target creature's power and toughness.
///
/// Creates a continuous effect that lasts for the specified duration.
///
/// # Fields
///
/// * `target` - Which creature to modify
/// * `power` - Power modifier (can be negative)
/// * `toughness` - Toughness modifier (can be negative)
///
/// # Example
///
/// ```ignore
/// // Target creature gets +3/+3 until end of turn (Giant Growth)
/// let effect = ModifyPowerToughnessEffect::new(ChooseSpec::creature(), 3, 3, Until::EndOfTurn);
///
/// // Target creature gets -2/-2 until end of turn
/// let effect = ModifyPowerToughnessEffect::new(ChooseSpec::creature(), -2, -2, Until::EndOfTurn);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModifyPowerToughnessEffect {
    /// Which creature to modify.
    pub target: ChooseSpec,
    /// Power modifier.
    pub power: Value,
    /// Toughness modifier.
    pub toughness: Value,
    /// Duration for the modification.
    pub duration: Until,
}

impl ModifyPowerToughnessEffect {
    /// Create a new modify power/toughness effect with explicit duration.
    pub fn new(
        target: ChooseSpec,
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        duration: Until,
    ) -> Self {
        Self {
            target,
            power: power.into(),
            toughness: toughness.into(),
            duration,
        }
    }

    /// Create a pump effect (+X/+X) with explicit duration.
    pub fn pump(target: ChooseSpec, amount: impl Into<Value>, duration: Until) -> Self {
        let val = amount.into();
        Self::new(target, val.clone(), val, duration)
    }

    /// Create a shrink effect (-X/-X) with explicit duration.
    pub fn shrink(target: ChooseSpec, amount: i32, duration: Until) -> Self {
        Self::new(target, -amount, -amount, duration)
    }

    /// Modify the source creature with explicit duration.
    pub fn source(power: impl Into<Value>, toughness: impl Into<Value>, duration: Until) -> Self {
        Self::new(ChooseSpec::Source, power, toughness, duration)
    }
}

impl EffectExecutor for ModifyPowerToughnessEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let power_mod = resolve_value(game, &self.power, ctx)?;
        let toughness_mod = resolve_value(game, &self.toughness, ctx)?;

        // Determine target - either from resolved targets or from Source
        let target_id = match &self.target {
            ChooseSpec::Source => ctx.source,
            _ => find_target_object(&ctx.targets)?,
        };

        // Verify the target exists and is a creature
        let target = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;

        if !target.has_card_type(CardType::Creature) {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        // Register the continuous effect via the shared primitive.
        let apply = ApplyContinuousEffect::new(
            EffectTarget::Specific(target_id),
            Modification::ModifyPowerToughness {
                power: power_mod,
                toughness: toughness_mod,
            },
            self.duration.clone(),
        );

        execute_effect(game, &Effect::new(apply), ctx)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature"
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
    fn test_pump_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature = create_creature(&mut game, "Bear", 2, 2, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let effect =
            ModifyPowerToughnessEffect::new(ChooseSpec::creature(), 3, 3, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Should have added a continuous effect
        assert_eq!(game.continuous_effects.effects_sorted().len(), 1);
    }

    #[test]
    fn test_shrink_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature = create_creature(&mut game, "Bear", 2, 2, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let effect =
            ModifyPowerToughnessEffect::shrink(ChooseSpec::creature(), 2, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.continuous_effects.effects_sorted().len(), 1);
    }

    #[test]
    fn test_pump_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature = create_creature(&mut game, "Pumping Creature", 2, 2, alice);

        let mut ctx = ExecutionContext::new_default(creature, alice);

        let effect = ModifyPowerToughnessEffect::source(2, 2, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.continuous_effects.effects_sorted().len(), 1);
    }

    #[test]
    fn test_pump_variable_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature = create_creature(&mut game, "Bear", 2, 2, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)])
            .with_x(5);

        let effect = ModifyPowerToughnessEffect::new(
            ChooseSpec::creature(),
            Value::X,
            Value::X,
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_pump_non_creature_fails() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a non-creature artifact
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), "Artifact")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Artifact])
            .build();
        let obj = Object::from_card(id, &card, alice, Zone::Battlefield);
        game.add_object(obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(id)]);

        let effect =
            ModifyPowerToughnessEffect::new(ChooseSpec::creature(), 3, 3, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_pump_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            ModifyPowerToughnessEffect::new(ChooseSpec::creature(), 3, 3, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_pump_clone_box() {
        let effect = ModifyPowerToughnessEffect::pump(ChooseSpec::creature(), 3, Until::EndOfTurn);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ModifyPowerToughnessEffect"));
    }
}
