//! Exchange control effect implementation.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, EffectResult, Until};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Effect that exchanges control of two permanents.
///
/// Creates continuous effects that swap the controllers of two permanents.
///
/// # Fields
///
/// * `permanent1` - First permanent
/// * `permanent2` - Second permanent
///
/// # Example
///
/// ```ignore
/// // Exchange control of two target creatures
/// let effect = ExchangeControlEffect::new(
///     ChooseSpec::creature(),
///     ChooseSpec::creature(),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeControlEffect {
    /// First permanent to exchange.
    pub permanent1: ChooseSpec,
    /// Second permanent to exchange.
    pub permanent2: ChooseSpec,
}

impl ExchangeControlEffect {
    /// Create a new exchange control effect.
    pub fn new(permanent1: ChooseSpec, permanent2: ChooseSpec) -> Self {
        Self {
            permanent1,
            permanent2,
        }
    }

    /// Exchange control of two creatures.
    pub fn creatures() -> Self {
        Self::new(ChooseSpec::creature(), ChooseSpec::creature())
    }

    /// Exchange control of two permanents.
    pub fn permanents() -> Self {
        Self::new(ChooseSpec::permanent(), ChooseSpec::permanent())
    }
}

impl EffectExecutor for ExchangeControlEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.targets.len() < 2 {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        let perm1_id = match &ctx.targets[0] {
            ResolvedTarget::Object(id) => *id,
            _ => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };

        let perm2_id = match &ctx.targets[1] {
            ResolvedTarget::Object(id) => *id,
            _ => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };

        // Get current controllers
        let controller1 = game.object(perm1_id).map(|o| o.controller);
        let controller2 = game.object(perm2_id).map(|o| o.controller);

        if let (Some(c1), Some(c2)) = (controller1, controller2) {
            let effect1 = ApplyContinuousEffect::new(
                EffectTarget::Specific(perm1_id),
                Modification::ChangeController(c2),
                Until::Forever,
            );

            let effect2 = ApplyContinuousEffect::new(
                EffectTarget::Specific(perm2_id),
                Modification::ChangeController(c1),
                Until::Forever,
            );

            let outcomes = vec![
                execute_effect(game, &Effect::new(effect1), ctx)?,
                execute_effect(game, &Effect::new(effect2), ctx)?,
            ];

            Ok(EffectOutcome::aggregate(outcomes))
        } else {
            Ok(EffectOutcome::from_result(EffectResult::TargetInvalid))
        }
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
    fn test_exchange_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature(&mut game, "Alice's Creature", alice);
        let creature2 = create_creature(&mut game, "Bob's Creature", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(creature1),
            ResolvedTarget::Object(creature2),
        ]);

        let effect = ExchangeControlEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Two continuous effects should be created
        assert_eq!(game.continuous_effects.effects_sorted().len(), 2);
    }

    #[test]
    fn test_exchange_control_insufficient_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature1 = create_creature(&mut game, "Creature", alice);
        let source = game.new_object_id();

        // Only one target
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature1)]);

        let effect = ExchangeControlEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_exchange_control_invalid_first_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature2 = create_creature(&mut game, "Creature", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Player(alice), // Invalid - should be object
            ResolvedTarget::Object(creature2),
        ]);

        let effect = ExchangeControlEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_exchange_control_invalid_second_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature(&mut game, "Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(creature1),
            ResolvedTarget::Player(bob), // Invalid - should be object
        ]);

        let effect = ExchangeControlEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_exchange_control_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature(&mut game, "Permanent 1", alice);
        let creature2 = create_creature(&mut game, "Permanent 2", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(creature1),
            ResolvedTarget::Object(creature2),
        ]);

        let effect = ExchangeControlEffect::permanents();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_exchange_control_clone_box() {
        let effect = ExchangeControlEffect::creatures();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ExchangeControlEffect"));
    }
}
