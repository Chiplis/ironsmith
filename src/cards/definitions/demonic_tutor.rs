//! Demonic Tutor card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Demonic Tutor
/// {1}{B}
/// Sorcery
/// Search your library for a card, put that card into your hand, then shuffle.
pub fn demonic_tutor() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Demonic Tutor")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Search your library for a card, put that card into your hand, then shuffle.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::effect::EffectResult;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::object::Object;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn add_card_to_library(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        card_type: CardType,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![card_type])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Library);
        game.add_object(obj);
        id
    }

    // ========================================
    // Basic Properties Tests
    // ========================================

    #[test]
    fn test_demonic_tutor_basic_properties() {
        let def = demonic_tutor();
        assert_eq!(def.name(), "Demonic Tutor");
        assert!(def.is_spell());
        assert!(def.card.is_sorcery());
        assert!(!def.card.is_instant());
        assert_eq!(def.card.mana_value(), 2);
    }

    #[test]
    fn test_demonic_tutor_is_black() {
        let def = demonic_tutor();
        assert!(def.card.colors().contains(Color::Black));
        assert!(!def.card.colors().contains(Color::White));
        assert!(!def.card.colors().contains(Color::Blue));
        assert!(!def.card.colors().contains(Color::Red));
        assert!(!def.card.colors().contains(Color::Green));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_demonic_tutor_mana_cost() {
        let def = demonic_tutor();
        // {1}{B} = 2 mana value
        assert_eq!(def.card.mana_value(), 2);
    }

    #[test]
    fn test_demonic_tutor_has_spell_effect() {
        let def = demonic_tutor();
        assert!(def.spell_effect.is_some());
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_demonic_tutor_no_abilities() {
        let def = demonic_tutor();
        // Sorceries don't have permanent abilities
        assert!(def.abilities.is_empty());
    }

    #[test]
    fn test_demonic_tutor_is_not_permanent() {
        let def = demonic_tutor();
        assert!(!def.is_permanent());
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_demonic_tutor_finds_any_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add various cards to library
        let _creature = add_card_to_library(&mut game, "Zombie", alice, CardType::Creature);
        let _land = add_card_to_library(&mut game, "Forest", alice, CardType::Land);
        let _instant = add_card_to_library(&mut game, "Lightning Bolt", alice, CardType::Instant);

        // Verify hand is empty
        assert!(game.player(alice).unwrap().hand.is_empty());

        // Execute the search effect with SelectFirstDecisionMaker
        let def = demonic_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        // Should find a card (auto-selects first one)
        assert!(matches!(result.result, EffectResult::Objects(_)));

        // Hand should now have 1 card
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    }

    #[test]
    fn test_demonic_tutor_can_find_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add only a creature
        let _creature = add_card_to_library(&mut game, "Zombie", alice, CardType::Creature);

        let def = demonic_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        assert!(matches!(result.result, EffectResult::Objects(_)));
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    }

    #[test]
    fn test_demonic_tutor_can_find_land() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add only a land
        let _land = add_card_to_library(&mut game, "Forest", alice, CardType::Land);

        let def = demonic_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        assert!(matches!(result.result, EffectResult::Objects(_)));
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    }

    #[test]
    fn test_demonic_tutor_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Library is empty

        let def = demonic_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        // Should find nothing
        assert_eq!(result.result, EffectResult::Count(0));
        assert!(game.player(alice).unwrap().hand.is_empty());
    }

    #[test]
    fn test_demonic_tutor_shuffles_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add multiple cards
        let _c1 = add_card_to_library(&mut game, "Card A", alice, CardType::Creature);
        let _c2 = add_card_to_library(&mut game, "Card B", alice, CardType::Creature);
        let _c3 = add_card_to_library(&mut game, "Card C", alice, CardType::Creature);

        let library_size_before = game.player(alice).unwrap().library.len();

        let def = demonic_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        // Library should have 1 less card (one moved to hand)
        assert_eq!(
            game.player(alice).unwrap().library.len(),
            library_size_before - 1
        );
    }

    #[test]
    fn test_demonic_tutor_tracks_library_search() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add a card
        let _c1 = add_card_to_library(&mut game, "Card", alice, CardType::Creature);

        assert!(!game.library_searches_this_turn.contains(&alice));

        let def = demonic_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        // Should be tracked for Archive Trap
        assert!(game.library_searches_this_turn.contains(&alice));
    }

    #[test]
    fn test_demonic_tutor_prevented_when_cant_search() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add a card
        let _c1 = add_card_to_library(&mut game, "Card", alice, CardType::Creature);

        // Simulate Leonin Arbiter effect
        game.cant_effects.cant_search.insert(alice);

        let def = demonic_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Prevented);
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_demonic_tutor_oracle_text() {
        let def = demonic_tutor();
        assert!(def.card.oracle_text.contains("Search your library"));
        assert!(
            def.card
                .oracle_text
                .contains("put that card into your hand")
        );
        assert!(def.card.oracle_text.contains("shuffle"));
    }

    #[test]
    fn test_demonic_tutor_no_reveal() {
        let def = demonic_tutor();
        // Demonic Tutor doesn't require revealing the card
        assert!(!def.card.oracle_text.contains("reveal"));
    }

    // ========================================
    // Characteristic Tests
    // ========================================

    #[test]
    fn test_demonic_tutor_is_sorcery_speed() {
        let def = demonic_tutor();
        assert!(def.card.is_sorcery());
        assert!(!def.card.is_instant());
    }

    #[test]
    fn test_demonic_tutor_has_no_subtypes() {
        let def = demonic_tutor();
        assert!(def.card.subtypes.is_empty());
    }

    #[test]
    fn test_demonic_tutor_has_no_supertypes() {
        let def = demonic_tutor();
        assert!(def.card.supertypes.is_empty());
    }

    #[test]
    fn test_demonic_tutor_is_not_creature() {
        let def = demonic_tutor();
        assert!(!def.card.is_creature());
        assert!(def.card.power_toughness.is_none());
    }

    #[test]
    fn test_demonic_tutor_no_alternative_costs() {
        let def = demonic_tutor();
        assert!(def.alternative_casts.is_empty());
    }

    #[test]
    fn test_demonic_tutor_no_optional_costs() {
        let def = demonic_tutor();
        assert!(def.optional_costs.is_empty());
    }
}
