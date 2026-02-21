//! Return from graveyard to hand effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
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
        use rand::seq::SliceRandom;

        let mut returned = Vec::new();

        if self.random {
            let base = self.target.base();
            let ChooseSpec::Object(filter) = base else {
                return Ok(EffectOutcome::from_result(EffectResult::Impossible));
            };
            if filter.zone != Some(Zone::Graveyard) {
                return Ok(EffectOutcome::from_result(EffectResult::Impossible));
            }

            let count = self.target.count();
            let requested = if count.is_dynamic_x() {
                0
            } else {
                count.max.unwrap_or(count.min)
            };
            if requested == 0 {
                return Ok(EffectOutcome::from_result(
                    EffectResult::Objects(Vec::new()),
                ));
            }

            let filter_ctx = ctx.filter_context(game);
            let mut candidates: Vec<_> = game
                .players
                .iter()
                .flat_map(|p| p.graveyard.iter().copied())
                .filter(|id| {
                    game.object(*id)
                        .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
                })
                .collect();

            candidates.shuffle(&mut rand::rng());
            for id in candidates.into_iter().take(requested) {
                if let Some(new_id) = game.move_object(id, Zone::Hand) {
                    returned.push(new_id);
                }
            }

            return Ok(EffectOutcome::from_result(EffectResult::Objects(returned)));
        }

        // Non-random: return all resolved object targets that are still in a graveyard.
        for target in &ctx.targets {
            let crate::executor::ResolvedTarget::Object(target_id) = target else {
                continue;
            };
            let Some(obj) = game.object(*target_id) else {
                continue;
            };
            if obj.zone != Zone::Graveyard {
                continue;
            }
            if let Some(new_id) = game.move_object(*target_id, Zone::Hand) {
                returned.push(new_id);
            }
        }

        if returned.is_empty() {
            Ok(EffectOutcome::from_result(EffectResult::TargetInvalid))
        } else {
            Ok(EffectOutcome::from_result(EffectResult::Objects(returned)))
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.random {
            None
        } else if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.random {
            None
        } else if self.target.is_target() {
            Some(self.target.count())
        } else {
            None
        }
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
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // No resolved objects to return.
        assert_eq!(result.result, EffectResult::TargetInvalid);
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
        assert!(effect.get_target_spec().is_none());

        let targeted = ReturnFromGraveyardToHandEffect::new(
            ChooseSpec::target(ChooseSpec::card_in_zone(Zone::Graveyard)),
            false,
        );
        assert!(targeted.get_target_spec().is_some());
    }
}
