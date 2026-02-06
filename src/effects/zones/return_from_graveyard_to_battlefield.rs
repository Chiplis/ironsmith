//! Return from graveyard to battlefield effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that returns a target card from a graveyard to the battlefield.
///
/// This is used for reanimation spells like Animate Dead, Reanimate, etc.
///
/// # Fields
///
/// * `target` - Which card to return (resolved from ctx.targets)
/// * `tapped` - Whether the permanent enters tapped
///
/// # Example
///
/// ```ignore
/// // Return target creature card from your graveyard to the battlefield
/// let effect = ReturnFromGraveyardToBattlefieldEffect::new(
///     ChooseSpec::creature_card_in_graveyard(),
///     false
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardToBattlefieldEffect {
    /// The targeting specification (for UI/validation purposes).
    pub target: ChooseSpec,
    /// Whether the permanent enters tapped.
    pub tapped: bool,
}

impl ReturnFromGraveyardToBattlefieldEffect {
    /// Create a new return from graveyard to battlefield effect.
    pub fn new(target: ChooseSpec, tapped: bool) -> Self {
        Self { target, tapped }
    }

    /// Create an effect that returns a creature untapped.
    pub fn creature() -> Self {
        Self::new(
            ChooseSpec::Object(crate::target::ObjectFilter::creature().in_zone(Zone::Graveyard)),
            false,
        )
    }

    /// Create an effect that returns a creature tapped.
    pub fn creature_tapped() -> Self {
        Self::new(
            ChooseSpec::Object(crate::target::ObjectFilter::creature().in_zone(Zone::Graveyard)),
            true,
        )
    }

    /// Create an effect targeting any card in a graveyard, entering untapped.
    pub fn any_card() -> Self {
        Self::new(ChooseSpec::card_in_zone(Zone::Graveyard), false)
    }

    /// Create an effect targeting any card in a graveyard, entering tapped.
    pub fn any_card_tapped() -> Self {
        Self::new(ChooseSpec::card_in_zone(Zone::Graveyard), true)
    }
}

impl EffectExecutor for ReturnFromGraveyardToBattlefieldEffect {
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

        // Move to battlefield with ETB event processing (handles replacement effects)
        if let Some(result) = game.move_object_with_etb_processing_with_dm(
            target_id,
            Zone::Battlefield,
            &mut ctx.decision_maker,
        ) {
            let new_id = result.new_id;

            // Apply tapped from effect (in addition to any ETB replacement effects)
            if self.tapped && !result.enters_tapped {
                game.tap(new_id);
            }

            Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                new_id,
            ])))
        } else {
            // ETB was prevented entirely
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
    fn test_reanimate_creature_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Griselbrand", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return Objects with new ID
        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Creature should be on battlefield and untapped
            assert!(game.battlefield.contains(&new_id));
            assert!(!game.is_tapped(new_id));
        } else {
            panic!("Expected Objects result");
        }
        // Graveyard should be empty
        assert!(game.players[0].graveyard.is_empty());
    }

    #[test]
    fn test_reanimate_creature_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Griselbrand", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature_tapped();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return Objects with new ID
        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Creature should be on battlefield and tapped
            assert!(game.battlefield.contains(&new_id));
            assert!(game.is_tapped(new_id));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_reanimate_from_wrong_zone_fails() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        // Create creature on battlefield, not in graveyard
        let creature_id = create_creature_on_battlefield(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should fail - target not in graveyard
        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_reanimate_opponent_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_creature_in_graveyard(&mut game, "Massacre Wurm", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should succeed
        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Creature enters under owner's (Bob's) control by default
            // (Reanimate effects that give you control need additional logic)
            assert!(game.battlefield.contains(&new_id));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_reanimate_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let result = effect.execute(&mut game, &mut ctx);

        // Should return error - no target
        assert!(result.is_err());
    }

    #[test]
    fn test_reanimate_clone_box() {
        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ReturnFromGraveyardToBattlefieldEffect"));
    }

    #[test]
    fn test_reanimate_get_target_spec() {
        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        assert!(effect.get_target_spec().is_some());
    }
}
