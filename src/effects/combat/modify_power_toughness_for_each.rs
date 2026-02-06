//! Modify power/toughness for each effect implementation.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, Until, Value};
use crate::effects::helpers::{find_target_object, resolve_value};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Effect that modifies a creature's power/toughness based on a count.
///
/// The final modification is power_per * count and toughness_per * count.
///
/// # Fields
///
/// * `target` - Which creature to modify
/// * `power_per` - Power modifier per count
/// * `toughness_per` - Toughness modifier per count
/// * `count` - Value determining the multiplier
///
/// # Example
///
/// ```ignore
/// // Target creature gets +1/+1 for each creature you control
/// let effect = ModifyPowerToughnessForEachEffect::new(
///     ChooseSpec::creature(),
///     1,
///     1,
///     Value::Count(ObjectFilter::creature().you_control()),
///     Until::EndOfTurn,
/// );
///
/// // Target creature gets +2/+0 for each card in your hand
/// let effect = ModifyPowerToughnessForEachEffect::new(
///     ChooseSpec::creature(),
///     2,
///     0,
///     Value::CardsInHand(PlayerFilter::You),
///     Until::EndOfTurn,
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModifyPowerToughnessForEachEffect {
    /// Which creature to modify.
    pub target: ChooseSpec,
    /// Power modifier per count.
    pub power_per: i32,
    /// Toughness modifier per count.
    pub toughness_per: i32,
    /// Value determining the multiplier.
    pub count: Value,
    /// Duration for the modification.
    pub duration: Until,
}

impl ModifyPowerToughnessForEachEffect {
    /// Create a new modify power/toughness for each effect with explicit duration.
    pub fn new(
        target: ChooseSpec,
        power_per: i32,
        toughness_per: i32,
        count: Value,
        duration: Until,
    ) -> Self {
        Self {
            target,
            power_per,
            toughness_per,
            count,
            duration,
        }
    }

    /// Create a symmetric pump (+N/+N per count) with explicit duration.
    pub fn symmetric(target: ChooseSpec, per: i32, count: Value, duration: Until) -> Self {
        Self::new(target, per, per, count, duration)
    }
}

impl EffectExecutor for ModifyPowerToughnessForEachEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let multiplier = resolve_value(game, &self.count, ctx)?;
        let power_mod = self.power_per * multiplier;
        let toughness_mod = self.toughness_per * multiplier;

        let target_id = find_target_object(&ctx.targets)?;

        // Verify the target exists
        let _target = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;

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
    use crate::effect::EffectResult;
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ObjectFilter;
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
    fn test_pump_for_each_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 3 creatures
        let target = create_creature(&mut game, "Target", 1, 1, alice);
        let _c1 = create_creature(&mut game, "Creature 1", 2, 2, alice);
        let _c2 = create_creature(&mut game, "Creature 2", 2, 2, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        // +1/+1 for each creature you control (should be 3 creatures = +3/+3)
        let effect = ModifyPowerToughnessForEachEffect::symmetric(
            ChooseSpec::creature(),
            1,
            Value::Count(ObjectFilter::creature().you_control()),
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.continuous_effects.effects_sorted().len(), 1);
    }

    #[test]
    fn test_pump_for_x() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let target = create_creature(&mut game, "Target", 1, 1, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)])
            .with_x(5);

        // +1/+1 for each X (X=5 = +5/+5)
        let effect = ModifyPowerToughnessForEachEffect::symmetric(
            ChooseSpec::creature(),
            1,
            Value::X,
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_pump_asymmetric() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let target = create_creature(&mut game, "Target", 1, 1, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)])
            .with_x(3);

        // +2/+0 for each X (X=3 = +6/+0)
        let effect = ModifyPowerToughnessForEachEffect::new(
            ChooseSpec::creature(),
            2,
            0,
            Value::X,
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_pump_zero_count() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let target = create_creature(&mut game, "Target", 1, 1, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)])
            .with_x(0);

        let effect = ModifyPowerToughnessForEachEffect::symmetric(
            ChooseSpec::creature(),
            1,
            Value::X,
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Still creates the effect (with +0/+0)
        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_pump_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ModifyPowerToughnessForEachEffect::symmetric(
            ChooseSpec::creature(),
            1,
            Value::Fixed(3),
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_pump_clone_box() {
        let effect = ModifyPowerToughnessForEachEffect::symmetric(
            ChooseSpec::creature(),
            1,
            Value::X,
            Until::EndOfTurn,
        );
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ModifyPowerToughnessForEachEffect"));
    }
}
