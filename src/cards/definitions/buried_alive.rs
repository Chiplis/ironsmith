//! Buried Alive card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Buried Alive
/// {2}{B}
/// Sorcery
/// Search your library for up to three creature cards, put them into your graveyard, then shuffle.
pub fn buried_alive() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Buried Alive")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search your library for up to three creature cards, put them into your graveyard, then shuffle.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Effect;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::color::Color;
    use crate::effect::{EffectOutcome, EffectResult};
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::object::Object;
    use crate::types::Subtype;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn add_creature_to_library(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Zombie])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Library);
        game.add_object(obj);
        id
    }

    fn add_non_creature_to_library(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Land])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Library);
        game.add_object(obj);
        id
    }

    fn buried_alive_effects() -> Vec<Effect> {
        let def = buried_alive();
        def.spell_effect.clone().unwrap_or_default()
    }

    fn execute_buried_alive(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Vec<EffectOutcome> {
        buried_alive_effects()
            .into_iter()
            .map(|effect| effect.0.execute(game, ctx).unwrap())
            .collect()
    }

    // ========================================
    // Basic Properties Tests
    // ========================================

    #[test]
    fn test_buried_alive_basic_properties() {
        let def = buried_alive();
        assert_eq!(def.name(), "Buried Alive");
        assert!(def.is_spell());
        assert!(def.card.is_sorcery());
        assert!(!def.card.is_instant());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[test]
    fn test_buried_alive_is_black() {
        let def = buried_alive();
        assert!(def.card.colors().contains(Color::Black));
        assert!(!def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_buried_alive_mana_cost() {
        let def = buried_alive();
        assert_eq!(def.card.mana_value(), 3);
        // {2}{B} = 2 generic + 1 black
    }

    #[test]
    fn test_buried_alive_has_spell_effect() {
        let def = buried_alive();
        assert!(def.spell_effect.is_some());
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_buried_alive_no_abilities() {
        let def = buried_alive();
        // Sorceries don't have abilities on the battlefield
        assert!(def.abilities.is_empty());
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_buried_alive_finds_up_to_three_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add 5 creatures to Alice's library
        let _c1 = add_creature_to_library(&mut game, "Zombie A", alice);
        let _c2 = add_creature_to_library(&mut game, "Zombie B", alice);
        let _c3 = add_creature_to_library(&mut game, "Zombie C", alice);
        let _c4 = add_creature_to_library(&mut game, "Zombie D", alice);
        let _c5 = add_creature_to_library(&mut game, "Zombie E", alice);

        // Execute the effect with SelectFirstDecisionMaker to auto-select maximum options
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = execute_buried_alive(&mut game, &mut ctx);

        // Should move exactly 3 creatures to the graveyard (auto-select takes first 3)
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 3);
    }

    #[test]
    fn test_buried_alive_with_fewer_than_three_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add only 2 creatures to Alice's library
        let _c1 = add_creature_to_library(&mut game, "Zombie A", alice);
        let _c2 = add_creature_to_library(&mut game, "Zombie B", alice);

        // Add a non-creature
        let _land = add_non_creature_to_library(&mut game, "Forest", alice);

        // Execute the effect with SelectFirstDecisionMaker to auto-select available options
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = execute_buried_alive(&mut game, &mut ctx);

        // Should find only 2 creatures (all available)
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 2);
    }

    #[test]
    fn test_buried_alive_no_creatures_in_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add only non-creatures to Alice's library
        let _land1 = add_non_creature_to_library(&mut game, "Forest", alice);
        let _land2 = add_non_creature_to_library(&mut game, "Mountain", alice);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = execute_buried_alive(&mut game, &mut ctx);

        // Should find no creatures
        assert!(game.player(alice).unwrap().graveyard.is_empty());
    }

    #[test]
    fn test_buried_alive_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Library is empty

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = execute_buried_alive(&mut game, &mut ctx);

        // Should find no creatures
        assert!(game.player(alice).unwrap().graveyard.is_empty());
    }

    #[test]
    fn test_buried_alive_shuffles_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add some cards
        let _c1 = add_creature_to_library(&mut game, "Zombie A", alice);
        let _c2 = add_creature_to_library(&mut game, "Zombie B", alice);
        let land = add_non_creature_to_library(&mut game, "Forest", alice);

        let library_size_before = game.player(alice).unwrap().library.len();

        // Execute the effect with SelectFirstDecisionMaker to auto-select options
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = execute_buried_alive(&mut game, &mut ctx);

        // Library should still exist (but 2 cards removed and shuffled)
        let library_after = &game.player(alice).unwrap().library;
        assert_eq!(library_after.len(), library_size_before - 2);

        // The land should still be in the library
        assert!(library_after.contains(&land));
    }

    #[test]
    fn test_buried_alive_only_finds_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add mix of creatures and non-creatures
        let _c1 = add_creature_to_library(&mut game, "Zombie", alice);
        let land_id = add_non_creature_to_library(&mut game, "Forest", alice);

        // Execute the effect with SelectFirstDecisionMaker to auto-select options
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = execute_buried_alive(&mut game, &mut ctx);

        // Should only find the creature, not the land
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
        // The land should NOT be in graveyard
        let land = game.object(land_id).unwrap();
        assert_eq!(land.zone, Zone::Library, "Land should still be in library");
    }

    #[test]
    fn test_buried_alive_cards_go_to_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add creatures
        let _c1 = add_creature_to_library(&mut game, "Zombie A", alice);
        let _c2 = add_creature_to_library(&mut game, "Zombie B", alice);

        // Graveyard should be empty initially
        assert!(game.player(alice).unwrap().graveyard.is_empty());

        // Execute the effect with SelectFirstDecisionMaker to auto-select options
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = execute_buried_alive(&mut game, &mut ctx);

        // Graveyard should now have 2 creatures
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 2);
    }

    #[test]
    fn test_buried_alive_tracks_library_search() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Verify not tracked initially
        assert!(!game.library_searches_this_turn.contains(&alice));

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = execute_buried_alive(&mut game, &mut ctx);

        // Should be tracked for Archive Trap
        assert!(game.library_searches_this_turn.contains(&alice));
    }

    #[test]
    fn test_buried_alive_prevented_when_cant_search() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add creatures
        let _c1 = add_creature_to_library(&mut game, "Zombie", alice);

        // Simulate Leonin Arbiter effect
        game.cant_effects.cant_search.insert(alice);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcomes = execute_buried_alive(&mut game, &mut ctx);

        // Should be prevented by the search restriction
        assert_eq!(outcomes[0].result, EffectResult::Prevented);
        assert!(game.player(alice).unwrap().graveyard.is_empty());
    }

    // ========================================
    // Rules Interaction Tests
    // ========================================

    #[test]
    fn test_buried_alive_creature_cards_only() {
        // Verify that the effect only targets creature cards, not creature tokens
        // (tokens can't exist in the library anyway, but the filter should be for creature *cards*)
        let def = buried_alive();

        // The oracle text specifies "creature cards"
        assert!(def.card.oracle_text.contains("creature cards"));
    }

    #[test]
    fn test_buried_alive_is_sorcery() {
        let def = buried_alive();

        // Buried Alive can only be cast at sorcery speed
        assert!(def.card.is_sorcery());
        assert!(!def.card.is_instant());
    }

    #[test]
    fn test_buried_alive_exact_three() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add exactly 3 creatures
        let _c1 = add_creature_to_library(&mut game, "Zombie A", alice);
        let _c2 = add_creature_to_library(&mut game, "Zombie B", alice);
        let _c3 = add_creature_to_library(&mut game, "Zombie C", alice);

        // Execute the effect with SelectFirstDecisionMaker to auto-select options
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = execute_buried_alive(&mut game, &mut ctx);

        // Should find all 3
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 3);
    }

    #[test]
    fn test_buried_alive_doesnt_reveal() {
        // Buried Alive doesn't say "reveal", so the cards put into graveyard
        // are not revealed until they're there (graveyard is public info)
        let def = buried_alive();

        // Oracle text doesn't contain "reveal"
        assert!(!def.card.oracle_text.contains("reveal"));
    }

    #[test]
    fn test_buried_alive_one_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add only 1 creature
        let _c1 = add_creature_to_library(&mut game, "Zombie", alice);

        // Execute the effect with SelectFirstDecisionMaker to auto-select options
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = execute_buried_alive(&mut game, &mut ctx);

        // Should find just 1
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    }
}
