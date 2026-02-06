//! Put onto battlefield effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{find_target_object, resolve_player_filter};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectRef, PlayerFilter};
use crate::zone::Zone;

/// Effect that puts a target card onto the battlefield.
///
/// This is a general-purpose effect for putting cards from any zone onto
/// the battlefield. Used by various spells and abilities that cheat permanents
/// into play.
///
/// # Fields
///
/// * `target` - Which card to put onto the battlefield (resolved from ctx.targets)
/// * `tapped` - Whether the permanent enters tapped
/// * `controller` - Who controls the permanent when it enters
///
/// # Example
///
/// ```ignore
/// // Put target creature card from your hand onto the battlefield tapped
/// let effect = PutOntoBattlefieldEffect::new(
///     ChooseSpec::creature_card_in_hand(),
///     true,
///     PlayerFilter::You,
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PutOntoBattlefieldEffect {
    /// The targeting specification (for UI/validation purposes).
    pub target: ChooseSpec,
    /// Whether the permanent enters tapped.
    pub tapped: bool,
    /// Who controls the permanent when it enters.
    pub controller: PlayerFilter,
}

impl PutOntoBattlefieldEffect {
    /// Create a new put onto battlefield effect.
    pub fn new(target: ChooseSpec, tapped: bool, controller: PlayerFilter) -> Self {
        Self {
            target,
            tapped,
            controller,
        }
    }

    /// Create an effect that puts a card onto the battlefield under your control.
    pub fn you_control(target: ChooseSpec, tapped: bool) -> Self {
        Self::new(target, tapped, PlayerFilter::You)
    }

    /// Create an effect that puts a card onto the battlefield under its owner's control.
    pub fn owner_control(target: ChooseSpec, tapped: bool) -> Self {
        Self::new(target, tapped, PlayerFilter::OwnerOf(ObjectRef::Target))
    }
}

impl EffectExecutor for PutOntoBattlefieldEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id = resolve_player_filter(game, &self.controller, ctx)?;
        let target_id = find_target_object(&ctx.targets)?;

        // Check if object exists
        let _obj = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;

        // Move to battlefield with ETB event processing (handles replacement effects)
        if let Some(result) = game.move_object_with_etb_processing_with_dm(
            target_id,
            Zone::Battlefield,
            &mut ctx.decision_maker,
        ) {
            let new_id = result.new_id;

            // Set controller
            if let Some(obj) = game.object_mut(new_id) {
                obj.controller = controller_id;
            }

            // Apply tapped from effect (in addition to any ETB replacement effects)
            if self.tapped && !result.enters_tapped {
                game.tap(new_id);
            }

            Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                new_id,
            ])))
        } else {
            // ETB was prevented entirely (e.g., "if this would enter, exile it instead")
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
        "card to put onto battlefield"
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
    use crate::target::ObjectFilter;
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

    fn create_creature_in_hand(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Hand);
        game.add_object(obj);
        id
    }

    fn create_creature_in_graveyard(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Graveyard);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_put_onto_battlefield_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_hand(&mut game, "Emrakul", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature()),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Creature should be on battlefield and untapped
            assert!(game.battlefield.contains(&new_id));
            assert!(!game.is_tapped(new_id));
            assert_eq!(game.object(new_id).unwrap().controller, alice);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_put_onto_battlefield_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_hand(&mut game, "Emrakul", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature()),
            true,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

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
    fn test_put_onto_battlefield_from_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Griselbrand", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature().in_zone(Zone::Graveyard)),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            assert!(game.battlefield.contains(&new_id));
        } else {
            panic!("Expected Objects result");
        }
        // Graveyard should be empty now
        assert!(game.players[0].graveyard.is_empty());
    }

    #[test]
    fn test_put_onto_battlefield_opponent_creature_you_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_creature_in_graveyard(&mut game, "Wurmcoil", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature()),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Alice controls it even though Bob owns it
            let obj = game.object(new_id).unwrap();
            assert_eq!(obj.controller, alice);
            assert_eq!(obj.owner, bob);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_put_onto_battlefield_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature()),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx);

        // Should return error - no target
        assert!(result.is_err());
    }

    #[test]
    fn test_put_onto_battlefield_clone_box() {
        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature()),
            false,
        );
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("PutOntoBattlefieldEffect"));
    }

    #[test]
    fn test_put_onto_battlefield_get_target_spec() {
        let effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature()),
            false,
        );
        assert!(effect.get_target_spec().is_some());
    }
}
