//! Put onto battlefield effect implementation.

use super::battlefield_entry::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectRef, PlayerFilter};

/// Effect that puts a target card onto the battlefield.
///
/// This is a general-purpose effect for putting cards from any zone onto
/// the battlefield. Used by various spells and abilities that cheat permanents
/// into play.
///
/// # Fields
///
/// * `target` - Which card to put onto the battlefield
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
        let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        if object_ids.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        let mut moved_ids = Vec::new();
        let mut prevented = false;

        for object_id in object_ids {
            if game.object(object_id).is_none() {
                continue;
            }
            let outcome = move_to_battlefield_with_options(
                game,
                ctx,
                object_id,
                BattlefieldEntryOptions::specific(controller_id, self.tapped),
            );
            match outcome {
                BattlefieldEntryOutcome::Moved(new_id) => moved_ids.push(new_id),
                BattlefieldEntryOutcome::Prevented => prevented = true,
            }
        }

        if !moved_ids.is_empty() {
            Ok(EffectOutcome::with_objects(moved_ids))
        } else if prevented {
            Ok(EffectOutcome::impossible())
        } else {
            Ok(EffectOutcome::target_invalid())
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "card to put onto battlefield"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{ChoiceCount, Effect};
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

    fn create_creature_in_library(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Library);
        game.add_object(obj);
        id
    }

    fn create_land_in_library(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Land])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Library);
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

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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
    fn test_put_onto_battlefield_iterated_object_from_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_library(&mut game, "Library Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.iteration.iterated_object = Some(creature_id);

        let effect = PutOntoBattlefieldEffect::you_control(ChooseSpec::Iterated, true);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            assert!(game.battlefield.contains(&new_id));
            assert!(game.is_tapped(new_id));
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
        assert_eq!(result, Err(ExecutionError::InvalidTarget));
    }

    #[test]
    fn test_map_the_frontier_style_sequence_puts_chosen_cards_onto_battlefield_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_land_in_library(&mut game, "Forest", alice);
        create_land_in_library(&mut game, "Desert", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = crate::effects::SequenceEffect::new(vec![
            Effect::choose_objects(
                ObjectFilter::land().in_zone(Zone::Library),
                ChoiceCount::up_to(2),
                PlayerFilter::You,
                "searched_0",
            ),
            Effect::for_each_tagged(
                "searched_0",
                vec![Effect::put_onto_battlefield(
                    ChooseSpec::Iterated,
                    true,
                    PlayerFilter::You,
                )],
            ),
            Effect::shuffle_library_player(PlayerFilter::You),
        ]);

        effect.execute(&mut game, &mut ctx).unwrap();

        assert!(
            game.player(alice).unwrap().library.is_empty(),
            "chosen cards should leave the library"
        );
        assert_eq!(game.battlefield.len(), 2);
        for object_id in game.battlefield.clone() {
            assert!(game.is_tapped(object_id));
            assert_eq!(game.object(object_id).unwrap().zone, Zone::Battlefield);
        }
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
        let targeted_effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            false,
        );
        assert!(targeted_effect.get_target_spec().is_some());

        let non_target_effect = PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(ObjectFilter::creature()),
            false,
        );
        assert!(non_target_effect.get_target_spec().is_none());
    }
}
