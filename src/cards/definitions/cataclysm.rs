//! Cataclysm card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Cataclysm
/// {2}{W}{W}
/// Sorcery
/// Each player chooses from among the permanents they control an artifact,
/// a creature, an enchantment, and a land, then sacrifices the rest.
pub fn cataclysm() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Cataclysm")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each player chooses from among the permanents they control an artifact, \
             a creature, an enchantment, and a land, then sacrifices the rest.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::color::Color;
    use crate::effect::{Effect, EffectResult};
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::object::Object;
    use crate::types::Subtype;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Soldier])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_artifact(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Artifact])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_enchantment(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Enchantment])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_land(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Land])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn cataclysm_effects() -> Vec<Effect> {
        let def = cataclysm();
        def.spell_effect.clone().unwrap_or_default()
    }

    fn create_artifact_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Golem])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    // ========================================
    // Basic Properties Tests
    // ========================================

    #[test]
    fn test_cataclysm_basic_properties() {
        let def = cataclysm();
        assert_eq!(def.name(), "Cataclysm");
        assert!(def.is_spell());
        assert!(def.card.is_sorcery());
        assert!(!def.card.is_instant());
        assert_eq!(def.card.mana_value(), 4);
    }

    #[test]
    fn test_cataclysm_is_white() {
        let def = cataclysm();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_cataclysm_mana_cost() {
        let def = cataclysm();
        assert_eq!(def.card.mana_value(), 4);
        // {2}{W}{W} = 2 generic + 2 white
    }

    #[test]
    fn test_cataclysm_has_spell_effect() {
        let def = cataclysm();
        assert!(def.spell_effect.is_some());
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 1);
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_cataclysm_keeps_one_of_each_type() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Alice has multiple permanents of each type
        let _artifact1 = create_artifact(&mut game, "Sol Ring", alice);
        let artifact2 = create_artifact(&mut game, "Mox", alice);
        let _creature1 = create_creature(&mut game, "Soldier A", alice);
        let creature2 = create_creature(&mut game, "Soldier B", alice);
        let _enchantment1 = create_enchantment(&mut game, "O-Ring", alice);
        let _land1 = create_land(&mut game, "Plains", alice);
        let land2 = create_land(&mut game, "Forest", alice);

        let battlefield_before = game.battlefield.len();
        assert_eq!(battlefield_before, 7);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        for effect in cataclysm_effects() {
            let _ = effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // Alice should have exactly 4 permanents (one of each type)
        let alice_permanents: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(_, obj)| obj.controller == alice)
            .collect();

        assert_eq!(alice_permanents.len(), 4, "Alice should have 4 permanents");

        // Verify one of each type
        let has_artifact = alice_permanents
            .iter()
            .any(|(_, obj)| obj.has_card_type(CardType::Artifact));
        let has_creature = alice_permanents
            .iter()
            .any(|(_, obj)| obj.has_card_type(CardType::Creature));
        let has_enchantment = alice_permanents
            .iter()
            .any(|(_, obj)| obj.has_card_type(CardType::Enchantment));
        let has_land = alice_permanents
            .iter()
            .any(|(_, obj)| obj.has_card_type(CardType::Land));

        assert!(has_artifact, "Should have an artifact");
        assert!(has_creature, "Should have a creature");
        assert!(has_enchantment, "Should have an enchantment");
        assert!(has_land, "Should have a land");

        // The second artifact, creature, and land should be sacrificed (moved to graveyard)
        // Note: Objects get new IDs when they move zones, so we check they're not on battlefield
        assert!(
            !game.battlefield.contains(&artifact2),
            "artifact2 should be sacrificed"
        );
        assert!(
            !game.battlefield.contains(&creature2),
            "creature2 should be sacrificed"
        );
        assert!(
            !game.battlefield.contains(&land2),
            "land2 should be sacrificed"
        );
    }

    #[test]
    fn test_cataclysm_affects_both_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Alice's permanents
        let _a_creature = create_creature(&mut game, "Alice Creature", alice);
        let _a_land = create_land(&mut game, "Alice Land", alice);

        // Bob's permanents
        let _b_creature = create_creature(&mut game, "Bob Creature", bob);
        let _b_land = create_land(&mut game, "Bob Land", bob);
        let _b_extra_creature = create_creature(&mut game, "Bob Extra", bob);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        for effect in cataclysm_effects() {
            let _ = effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // Alice should have 2 permanents (creature and land)
        let alice_count = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == alice)
            .count();
        assert_eq!(alice_count, 2);

        // Bob should have 2 permanents (creature and land - extra creature sacrificed)
        let bob_count = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == bob)
            .count();
        assert_eq!(bob_count, 2);
    }

    #[test]
    fn test_cataclysm_artifact_creature_counts_as_one() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Alice has an artifact creature and other permanents
        let golem = create_artifact_creature(&mut game, "Golem", alice);
        let _creature = create_creature(&mut game, "Soldier", alice);
        let _artifact = create_artifact(&mut game, "Sol Ring", alice);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        for effect in cataclysm_effects() {
            let _ = effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // The golem can only be chosen for ONE type (artifact OR creature)
        // So Alice should have 2 permanents total (golem as artifact, soldier as creature)
        // OR (golem as creature, sol ring as artifact)
        let alice_permanents: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == alice)
            .collect();

        assert_eq!(alice_permanents.len(), 2, "Should have 2 permanents");

        // Golem should survive (it gets picked first for artifact)
        assert!(game.battlefield.contains(&golem), "Golem should survive");
    }

    #[test]
    fn test_cataclysm_player_with_no_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Only Alice has permanents
        let _creature = create_creature(&mut game, "Creature", alice);
        let _land = create_land(&mut game, "Land", alice);

        // Bob has nothing

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        for effect in cataclysm_effects() {
            let _ = effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // Alice should have 2 permanents
        let alice_count = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == alice)
            .count();
        assert_eq!(alice_count, 2);

        // Bob should still have nothing
        let bob_count = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == bob)
            .count();
        assert_eq!(bob_count, 0);
    }

    #[test]
    fn test_cataclysm_player_with_only_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Alice only has creatures
        let _creature1 = create_creature(&mut game, "Soldier A", alice);
        let _creature2 = create_creature(&mut game, "Soldier B", alice);
        let _creature3 = create_creature(&mut game, "Soldier C", alice);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        for effect in cataclysm_effects() {
            let _ = effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // Alice should have exactly 1 creature (no artifacts, enchantments, or lands)
        let alice_permanents: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == alice)
            .collect();

        assert_eq!(alice_permanents.len(), 1);
        assert!(alice_permanents[0].has_card_type(CardType::Creature));
    }

    #[test]
    fn test_cataclysm_sacrificed_permanents_go_to_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Alice has two creatures
        let _creature1 = create_creature(&mut game, "Survivor", alice);
        let creature2 = create_creature(&mut game, "Victim", alice);

        // Graveyard should be empty
        assert!(game.player(alice).unwrap().graveyard.is_empty());

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        for effect in cataclysm_effects() {
            let _ = effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // The second creature should be in the graveyard (but with a new ID)
        // We verify it was sacrificed by checking it's not on battlefield and graveyard has 1 card
        assert!(
            !game.battlefield.contains(&creature2),
            "creature2 should be sacrificed"
        );
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    }

    #[test]
    fn test_cataclysm_empty_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // No permanents on battlefield

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        let mut result = None;
        for effect in cataclysm_effects() {
            result = Some(effect.0.execute(&mut game, &mut ctx).unwrap());
        }

        // Should sacrifice nothing
        assert_eq!(result.unwrap().result, EffectResult::Count(0));
    }

    #[test]
    fn test_cataclysm_one_of_each_type_survives() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Alice has exactly one of each type
        let artifact = create_artifact(&mut game, "Sol Ring", alice);
        let creature = create_creature(&mut game, "Soldier", alice);
        let enchantment = create_enchantment(&mut game, "O-Ring", alice);
        let land = create_land(&mut game, "Plains", alice);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        let mut result = None;
        for effect in cataclysm_effects() {
            result = Some(effect.0.execute(&mut game, &mut ctx).unwrap());
        }

        // Nothing should be sacrificed
        assert_eq!(result.unwrap().result, EffectResult::Count(0));

        // All four should still be on battlefield
        assert!(
            game.battlefield.contains(&artifact),
            "artifact should survive"
        );
        assert!(
            game.battlefield.contains(&creature),
            "creature should survive"
        );
        assert!(
            game.battlefield.contains(&enchantment),
            "enchantment should survive"
        );
        assert!(game.battlefield.contains(&land), "land should survive");
    }

    // ========================================
    // Rules Interaction Tests
    // ========================================

    #[test]
    fn test_cataclysm_is_symmetrical() {
        // The oracle text says "Each player" - this is symmetrical
        let def = cataclysm();
        assert!(def.card.oracle_text.contains("Each player"));
    }

    #[test]
    fn test_cataclysm_planeswalkers_are_sacrificed() {
        // Cataclysm only lets you keep artifact, creature, enchantment, land
        // Planeswalkers and Battles are NOT on that list, so they all get sacrificed
        let def = cataclysm();
        let text = &def.card.oracle_text;

        assert!(text.contains("artifact"));
        assert!(text.contains("creature"));
        assert!(text.contains("enchantment"));
        assert!(text.contains("land"));
        assert!(!text.contains("planeswalker"));
        assert!(!text.contains("battle"));
    }
}
