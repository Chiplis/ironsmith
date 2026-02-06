//! Gain control effect implementation.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, Until};
use crate::effects::helpers::find_target_object;
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Effect that gains control of a target permanent.
///
/// Creates a continuous effect that changes the controller of the target.
///
/// # Fields
///
/// * `target` - The permanent to gain control of
/// * `duration` - How long the control change lasts
///
/// # Example
///
/// ```ignore
/// // Gain control of target creature until end of turn
/// let effect = GainControlEffect::until_end_of_turn(ChooseSpec::creature());
///
/// // Gain control of target permanent permanently
/// let effect = GainControlEffect::permanent(ChooseSpec::permanent());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GainControlEffect {
    /// The permanent to gain control of.
    pub target: ChooseSpec,
    /// How long the control change lasts.
    pub duration: Until,
}

impl GainControlEffect {
    /// Create a new gain control effect.
    pub fn new(target: ChooseSpec, duration: Until) -> Self {
        Self { target, duration }
    }

    /// Gain control until end of turn.
    pub fn until_end_of_turn(target: ChooseSpec) -> Self {
        Self::new(target, Until::EndOfTurn)
    }

    /// Gain control permanently.
    pub fn permanent(target: ChooseSpec) -> Self {
        Self::new(target, Until::Forever)
    }

    /// Gain control until source leaves the battlefield.
    pub fn while_source_remains(target: ChooseSpec) -> Self {
        Self::new(target, Until::ThisLeavesTheBattlefield)
    }
}

impl EffectExecutor for GainControlEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = find_target_object(&ctx.targets)?;

        // Verify target is on the battlefield
        let _obj = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;

        let apply = ApplyContinuousEffect::new(
            EffectTarget::Specific(target_id),
            Modification::ChangeController(ctx.controller),
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
        "permanent to gain control of"
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
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
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
    fn test_gain_control_until_end_of_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature = create_creature(&mut game, "Opponent's Creature", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let effect = GainControlEffect::until_end_of_turn(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.continuous_effects.effects_sorted().len(), 1);
    }

    #[test]
    fn test_gain_control_permanent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature = create_creature(&mut game, "Stolen Creature", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let effect = GainControlEffect::permanent(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_gain_control_while_source_remains() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature = create_creature(&mut game, "Target Creature", bob);
        let source = create_creature(&mut game, "Control Source", alice);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let effect = GainControlEffect::while_source_remains(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_gain_control_target_not_found() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let fake_id = ObjectId(9999);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(fake_id)]);

        let effect = GainControlEffect::until_end_of_turn(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_gain_control_clone_box() {
        let effect = GainControlEffect::until_end_of_turn(ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("GainControlEffect"));
    }

    #[test]
    fn test_gain_control_get_target_spec() {
        let effect = GainControlEffect::until_end_of_turn(ChooseSpec::creature());
        assert!(effect.get_target_spec().is_some());
    }
}
