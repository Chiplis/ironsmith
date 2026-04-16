//! Modify power/toughness for each effect implementation.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, Until, Value};
use crate::effects::helpers::{resolve_single_object_for_effect, resolve_value};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::ModifyPowerToughnessForEachEffect;

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
impl EffectExecutor for ModifyPowerToughnessForEachEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let multiplier = resolve_value(game, &self.count, ctx)?;
        let power_mod = self.power_per * multiplier;
        let toughness_mod = self.toughness_per * multiplier;

        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;

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
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(
            game.effect_store.continuous_effects.effects_sorted().len(),
            1
        );
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

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
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

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
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
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
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
    fn test_pump_for_each_tagged_target_without_ctx_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let target = create_creature(&mut game, "Tagged Target", 1, 1, alice);
        let _other = create_creature(&mut game, "Other", 2, 2, alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let snapshot = ObjectSnapshot::from_object(game.object(target).unwrap(), &game);
        ctx.tag_object("tagged_target", snapshot);

        let effect = ModifyPowerToughnessForEachEffect::symmetric(
            ChooseSpec::Tagged("tagged_target".into()),
            1,
            Value::Count(ObjectFilter::creature().you_control()),
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(
            game.effect_store.continuous_effects.effects_sorted().len(),
            1
        );
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
