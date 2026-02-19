//! Return from graveyard to hand effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that returns a target card from a graveyard to its owner's hand.
///
/// This is used for recursion spells like Regrowth, Raise Dead, etc.
///
/// # Fields
///
/// * `target` - Which card to return (resolved from ctx.targets)
///
/// # Example
///
/// ```ignore
/// // Return target creature card from your graveyard to your hand
/// let effect = ReturnFromGraveyardToHandEffect::new(ChooseSpec::creature_card_in_graveyard());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardToHandEffect {
    /// The targeting specification (for UI/validation purposes).
    pub target: ChooseSpec,
    /// Whether the cards are selected at random (text-level semantics).
    pub random: bool,
}

impl ReturnFromGraveyardToHandEffect {
    /// Create a new return from graveyard to hand effect.
    pub fn new(target: ChooseSpec, random: bool) -> Self {
        Self { target, random }
    }

    /// Create an effect targeting any card in a graveyard.
    pub fn any_card() -> Self {
        Self::new(ChooseSpec::card_in_zone(Zone::Graveyard), false)
    }
}

impl EffectExecutor for ReturnFromGraveyardToHandEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = find_target_object(&ctx.targets)?;

        // Verify target is in a graveyard
        let obj = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;

        if obj.zone != Zone::Graveyard {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        if let Some(new_id) = game.move_object(target_id, Zone::Hand) {
            Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                new_id,
            ])))
        } else {
            Ok(EffectOutcome::from_result(EffectResult::Impossible))
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "card in graveyard to return"
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

    fn create_creature_in_graveyard(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Graveyard);
        game.add_object(obj);
        id
    }

    fn create_creature_on_battlefield(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_return_creature_from_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToHandEffect::any_card();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return Objects with new ID
        assert!(matches!(result.result, EffectResult::Objects(_)));
        // Graveyard should be empty
        assert!(game.players[0].graveyard.is_empty());
        // Hand should have the card (with new ID per rule 400.7)
        assert!(!game.players[0].hand.is_empty());
    }

    #[test]
    fn test_return_from_wrong_zone_fails() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        // Create creature on battlefield, not in graveyard
        let creature_id = create_creature_on_battlefield(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToHandEffect::any_card();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should fail - target not in graveyard
        assert_eq!(result.result, EffectResult::TargetInvalid);
        // Creature should still be on battlefield
        assert!(game.battlefield.contains(&creature_id));
    }

    #[test]
    fn test_return_opponent_creature_from_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_creature_in_graveyard(&mut game, "Hill Giant", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToHandEffect::any_card();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should succeed - returns to owner's hand (Bob's)
        assert!(matches!(result.result, EffectResult::Objects(_)));
        // Bob's graveyard should be empty
        assert!(game.players[1].graveyard.is_empty());
        // Bob's hand should have the card
        assert!(!game.players[1].hand.is_empty());
    }

    #[test]
    fn test_return_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ReturnFromGraveyardToHandEffect::any_card();
        let result = effect.execute(&mut game, &mut ctx);

        // Should return error - no target
        assert!(result.is_err());
    }

    #[test]
    fn test_return_from_graveyard_clone_box() {
        let effect = ReturnFromGraveyardToHandEffect::any_card();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ReturnFromGraveyardToHandEffect"));
    }

    #[test]
    fn test_return_from_graveyard_get_target_spec() {
        let effect = ReturnFromGraveyardToHandEffect::any_card();
        assert!(effect.get_target_spec().is_some());
    }
}
