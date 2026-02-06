//! Enlightened Tutor card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Enlightened Tutor
/// {W}
/// Instant
/// Search your library for an artifact or enchantment card, reveal it, then shuffle and put that card on top.
pub fn enlightened_tutor() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Enlightened Tutor")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Search your library for an artifact or enchantment card, reveal it, then shuffle and put that card on top.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Zone;
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::effect::EffectResult;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::object::Object;

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
    fn test_enlightened_tutor_basic_properties() {
        let def = enlightened_tutor();
        assert_eq!(def.name(), "Enlightened Tutor");
        assert!(def.is_spell());
        assert!(def.card.is_instant());
        assert!(!def.card.is_sorcery());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_enlightened_tutor_is_white() {
        let def = enlightened_tutor();
        assert!(def.card.colors().contains(Color::White));
        assert!(!def.card.colors().contains(Color::Black));
        assert!(!def.card.colors().contains(Color::Blue));
        assert!(!def.card.colors().contains(Color::Red));
        assert!(!def.card.colors().contains(Color::Green));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_enlightened_tutor_mana_cost() {
        let def = enlightened_tutor();
        // {W} = 1 mana value
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_enlightened_tutor_has_one_spell_effect() {
        let def = enlightened_tutor();
        assert!(def.spell_effect.is_some());
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_enlightened_tutor_no_abilities() {
        let def = enlightened_tutor();
        assert!(def.abilities.is_empty());
    }

    #[test]
    fn test_enlightened_tutor_is_not_permanent() {
        let def = enlightened_tutor();
        assert!(!def.is_permanent());
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_enlightened_tutor_finds_artifact() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add an artifact to library
        let _artifact = add_card_to_library(&mut game, "Sol Ring", alice, CardType::Artifact);

        let def = enlightened_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();
        assert!(matches!(result.result, EffectResult::Objects(_)));
    }

    #[test]
    fn test_enlightened_tutor_finds_enchantment() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add an enchantment to library
        let _enchantment = add_card_to_library(&mut game, "Land Tax", alice, CardType::Enchantment);

        let def = enlightened_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();
        assert!(matches!(result.result, EffectResult::Objects(_)));
    }

    #[test]
    fn test_enlightened_tutor_does_not_find_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add only creatures
        let _creature = add_card_to_library(&mut game, "Zombie", alice, CardType::Creature);

        let def = enlightened_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();
        // Should not find anything since there are no artifacts or enchantments
        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_enlightened_tutor_puts_on_top_of_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add an artifact to library
        let _artifact = add_card_to_library(&mut game, "Sol Ring", alice, CardType::Artifact);

        let initial_size = game.player(alice).unwrap().library.len();

        let def = enlightened_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::EffectResult::Objects(found) = result.result {
            assert_eq!(found.len(), 1);
            // Card should be on top of library
            let top_of_library = game.player(alice).unwrap().library.last();
            assert!(top_of_library.is_some());
        }

        // Hand should still be empty (card goes to library, not hand)
        assert!(game.player(alice).unwrap().hand.is_empty());

        // Library size should be unchanged
        assert_eq!(game.player(alice).unwrap().library.len(), initial_size);
    }

    #[test]
    fn test_enlightened_tutor_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let def = enlightened_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_enlightened_tutor_tracks_library_search() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let _artifact = add_card_to_library(&mut game, "Sol Ring", alice, CardType::Artifact);

        assert!(!game.library_searches_this_turn.contains(&alice));

        let def = enlightened_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        assert!(game.library_searches_this_turn.contains(&alice));
    }

    #[test]
    fn test_enlightened_tutor_prevented_when_cant_search() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let _artifact = add_card_to_library(&mut game, "Sol Ring", alice, CardType::Artifact);

        game.cant_effects.cant_search.insert(alice);

        let def = enlightened_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Prevented);
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_enlightened_tutor_oracle_text() {
        let def = enlightened_tutor();
        assert!(def.card.oracle_text.contains("Search your library"));
        assert!(def.card.oracle_text.contains("artifact or enchantment"));
        assert!(def.card.oracle_text.contains("reveal"));
        assert!(def.card.oracle_text.contains("put that card on top"));
    }

    // ========================================
    // Comparison Tests
    // ========================================

    #[test]
    fn test_enlightened_tutor_is_instant_speed() {
        let def = enlightened_tutor();
        assert!(def.card.is_instant());
        assert!(!def.card.is_sorcery());
    }

    #[test]
    fn test_enlightened_tutor_has_no_subtypes() {
        let def = enlightened_tutor();
        assert!(def.card.subtypes.is_empty());
    }

    #[test]
    fn test_enlightened_tutor_no_alternative_costs() {
        let def = enlightened_tutor();
        assert!(def.alternative_casts.is_empty());
    }

    #[test]
    fn test_enlightened_tutor_no_optional_costs() {
        let def = enlightened_tutor();
        assert!(def.optional_costs.is_empty());
    }
}
