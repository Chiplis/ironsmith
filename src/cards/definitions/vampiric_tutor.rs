//! Vampiric Tutor card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

/// Vampiric Tutor
/// {B}
/// Instant
/// Search your library for a card, then shuffle and put that card on top. You lose 2 life.
pub fn vampiric_tutor() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Vampiric Tutor")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![
            Effect::search_library(
                ObjectFilter::default(),
                Zone::Library,
                PlayerFilter::You,
                false,
            ),
            Effect::lose_life(2),
        ])
        .oracle_text(
            "Search your library for a card, then shuffle and put that card on top. You lose 2 life.",
        )
        .build()
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
    fn test_vampiric_tutor_basic_properties() {
        let def = vampiric_tutor();
        assert_eq!(def.name(), "Vampiric Tutor");
        assert!(def.is_spell());
        assert!(def.card.is_instant());
        assert!(!def.card.is_sorcery());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_vampiric_tutor_is_black() {
        let def = vampiric_tutor();
        assert!(def.card.colors().contains(Color::Black));
        assert!(!def.card.colors().contains(Color::White));
        assert!(!def.card.colors().contains(Color::Blue));
        assert!(!def.card.colors().contains(Color::Red));
        assert!(!def.card.colors().contains(Color::Green));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_vampiric_tutor_mana_cost() {
        let def = vampiric_tutor();
        // {B} = 1 mana value
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_vampiric_tutor_has_two_spell_effects() {
        let def = vampiric_tutor();
        assert!(def.spell_effect.is_some());
        // Search effect + lose life effect
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_vampiric_tutor_no_abilities() {
        let def = vampiric_tutor();
        // Instants don't have permanent abilities
        assert!(def.abilities.is_empty());
    }

    #[test]
    fn test_vampiric_tutor_is_not_permanent() {
        let def = vampiric_tutor();
        assert!(!def.is_permanent());
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_vampiric_tutor_finds_any_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add cards to library
        let _creature = add_card_to_library(&mut game, "Zombie", alice, CardType::Creature);

        let def = vampiric_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Execute search effect
        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();
        assert!(matches!(result.result, EffectResult::Objects(_)));
    }

    #[test]
    fn test_vampiric_tutor_puts_on_top_of_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add a card to library
        let _card_id = add_card_to_library(&mut game, "Target Card", alice, CardType::Creature);

        // Remember initial library size
        let initial_size = game.player(alice).unwrap().library.len();

        let def = vampiric_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Execute search effect (puts card on top)
        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::EffectResult::Objects(found) = result.result {
            assert_eq!(found.len(), 1);
            // The card should be on top of the library
            let top_of_library = game.player(alice).unwrap().library.last();
            assert!(top_of_library.is_some());
        } else {
            panic!("Expected Objects result");
        }

        // Hand should still be empty (card goes to library, not hand)
        assert!(game.player(alice).unwrap().hand.is_empty());

        // Library size should be the same (card moved within library)
        assert_eq!(game.player(alice).unwrap().library.len(), initial_size);
    }

    #[test]
    fn test_vampiric_tutor_causes_life_loss() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Verify starting life total
        let initial_life = game.player(alice).unwrap().life;
        assert_eq!(initial_life, 20);

        let def = vampiric_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Execute lose life effect
        let result = effects[1].0.execute(&mut game, &mut ctx).unwrap();

        // Should lose 2 life
        let final_life = game.player(alice).unwrap().life;
        assert_eq!(final_life, 18);
        assert!(matches!(result.result, EffectResult::Count(2)));
    }

    #[test]
    fn test_vampiric_tutor_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Library is empty

        let def = vampiric_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Execute search effect
        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        // Should find nothing
        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_vampiric_tutor_tracks_library_search() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add a card
        let _c1 = add_card_to_library(&mut game, "Card", alice, CardType::Creature);

        assert!(!game.library_searches_this_turn.contains(&alice));

        let def = vampiric_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let _ = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        // Should be tracked for Archive Trap
        assert!(game.library_searches_this_turn.contains(&alice));
    }

    #[test]
    fn test_vampiric_tutor_prevented_when_cant_search() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add a card
        let _c1 = add_card_to_library(&mut game, "Card", alice, CardType::Creature);

        // Simulate Leonin Arbiter effect
        game.cant_effects.cant_search.insert(alice);

        let def = vampiric_tutor();
        let effects = def.spell_effect.as_ref().unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effects[0].0.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Prevented);
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_vampiric_tutor_oracle_text() {
        let def = vampiric_tutor();
        assert!(def.card.oracle_text.contains("Search your library"));
        assert!(def.card.oracle_text.contains("put that card on top"));
        assert!(def.card.oracle_text.contains("shuffle"));
        assert!(def.card.oracle_text.contains("lose 2 life"));
    }

    #[test]
    fn test_vampiric_tutor_no_reveal() {
        let def = vampiric_tutor();
        // Vampiric Tutor doesn't require revealing
        assert!(!def.card.oracle_text.contains("reveal"));
    }

    // ========================================
    // Characteristic Tests
    // ========================================

    #[test]
    fn test_vampiric_tutor_is_instant_speed() {
        let def = vampiric_tutor();
        assert!(def.card.is_instant());
        assert!(!def.card.is_sorcery());
    }

    #[test]
    fn test_vampiric_tutor_has_no_subtypes() {
        let def = vampiric_tutor();
        assert!(def.card.subtypes.is_empty());
    }

    #[test]
    fn test_vampiric_tutor_has_no_supertypes() {
        let def = vampiric_tutor();
        assert!(def.card.supertypes.is_empty());
    }

    #[test]
    fn test_vampiric_tutor_no_alternative_costs() {
        let def = vampiric_tutor();
        assert!(def.alternative_casts.is_empty());
    }

    #[test]
    fn test_vampiric_tutor_no_optional_costs() {
        let def = vampiric_tutor();
        assert!(def.optional_costs.is_empty());
    }

    // ========================================
    // Comparison with Demonic Tutor
    // ========================================

    #[test]
    fn test_vampiric_tutor_cheaper_than_demonic_tutor() {
        let vamp = vampiric_tutor();
        // Vampiric Tutor costs {B} = 1, Demonic Tutor costs {1}{B} = 2
        assert_eq!(vamp.card.mana_value(), 1);
    }

    #[test]
    fn test_vampiric_tutor_is_instant_unlike_demonic() {
        let vamp = vampiric_tutor();
        // Vampiric Tutor is instant, Demonic Tutor is sorcery
        assert!(vamp.card.is_instant());
        assert!(!vamp.card.is_sorcery());
    }

    #[test]
    fn test_vampiric_tutor_goes_to_library_not_hand() {
        let def = vampiric_tutor();
        // Vampiric puts on top of library, not hand
        // This is tested in the effect execution tests above
        assert!(def.card.oracle_text.contains("on top"));
        assert!(!def.card.oracle_text.contains("into your hand"));
    }
}
