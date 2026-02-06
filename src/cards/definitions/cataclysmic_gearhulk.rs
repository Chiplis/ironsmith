//! Cataclysmic Gearhulk card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Cataclysmic Gearhulk
/// {3}{W}{W}
/// Artifact Creature — Construct
/// 4/5
/// Vigilance
/// When Cataclysmic Gearhulk enters the battlefield, each player chooses an artifact,
/// a creature, an enchantment, and a planeswalker from among the nonland permanents
/// they control, then sacrifices the rest.
pub fn cataclysmic_gearhulk() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Cataclysmic Gearhulk")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Construct])
        .power_toughness(PowerToughness::fixed(4, 5))
        .parse_text(
            "Vigilance\n\
             When Cataclysmic Gearhulk enters the battlefield, each player chooses \
             an artifact, a creature, an enchantment, and a planeswalker from among \
             the nonland permanents they control, then sacrifices the rest.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::effect::{Effect, EffectResult};
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::object::Object;
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

    fn gearhulk_effects() -> Vec<Effect> {
        let def = cataclysmic_gearhulk();
        let ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .expect("Cataclysmic Gearhulk should have a triggered ability");
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            triggered.effects.clone()
        } else {
            Vec::new()
        }
    }

    // ========================================
    // Basic Properties Tests
    // ========================================

    #[test]
    fn test_cataclysmic_gearhulk_basic_properties() {
        let def = cataclysmic_gearhulk();
        assert_eq!(def.name(), "Cataclysmic Gearhulk");
        assert!(def.is_creature());
        assert!(def.card.has_card_type(CardType::Artifact));
        assert!(def.card.has_card_type(CardType::Creature));
        assert_eq!(def.card.mana_value(), 5);
    }

    #[test]
    fn test_cataclysmic_gearhulk_is_white() {
        let def = cataclysmic_gearhulk();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_cataclysmic_gearhulk_is_construct() {
        let def = cataclysmic_gearhulk();
        assert!(def.card.has_subtype(Subtype::Construct));
    }

    #[test]
    fn test_cataclysmic_gearhulk_power_toughness() {
        let def = cataclysmic_gearhulk();
        let pt = def.card.power_toughness.as_ref().unwrap();
        use crate::card::PtValue;
        assert_eq!(pt.power, PtValue::Fixed(4));
        assert_eq!(pt.toughness, PtValue::Fixed(5));
    }

    #[test]
    fn test_cataclysmic_gearhulk_has_vigilance() {
        let def = cataclysmic_gearhulk();
        let has_vigilance = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_vigilance()
            } else {
                false
            }
        });
        assert!(has_vigilance, "Should have vigilance");
    }

    #[test]
    fn test_cataclysmic_gearhulk_has_etb_trigger() {
        let def = cataclysmic_gearhulk();
        // Now using Trigger struct - check display contains enters
        let has_etb = def.abilities.iter().any(|a| {
            matches!(
                &a.kind,
                AbilityKind::Triggered(t) if t.trigger.display().contains("enters")
            )
        });
        assert!(has_etb, "Should have ETB trigger");
    }

    // ========================================
    // Effect Execution Tests - Lands Are Safe
    // ========================================

    #[test]
    fn test_gearhulk_lands_are_not_affected() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Alice has multiple lands
        let land1 = create_land(&mut game, "Plains", alice);
        let land2 = create_land(&mut game, "Forest", alice);
        let land3 = create_land(&mut game, "Mountain", alice);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        for effect in gearhulk_effects() {
            let _ = effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // ALL lands should survive (they're not affected at all)
        assert!(game.battlefield.contains(&land1), "land1 should survive");
        assert!(game.battlefield.contains(&land2), "land2 should survive");
        assert!(game.battlefield.contains(&land3), "land3 should survive");
    }

    #[test]
    fn test_gearhulk_keeps_one_nonland_of_each_type() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Alice has multiple nonland permanents
        let _artifact1 = create_artifact(&mut game, "Sol Ring", alice);
        let artifact2 = create_artifact(&mut game, "Mox", alice);
        let _creature1 = create_creature(&mut game, "Soldier A", alice);
        let creature2 = create_creature(&mut game, "Soldier B", alice);
        let _enchantment1 = create_enchantment(&mut game, "O-Ring", alice);

        // Alice also has lands (should be safe)
        let land1 = create_land(&mut game, "Plains", alice);
        let land2 = create_land(&mut game, "Forest", alice);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        for effect in gearhulk_effects() {
            let _ = effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // Lands should ALL survive
        assert!(game.battlefield.contains(&land1));
        assert!(game.battlefield.contains(&land2));

        // Extra artifact and creature should be sacrificed
        assert!(!game.battlefield.contains(&artifact2));
        assert!(!game.battlefield.contains(&creature2));

        // Alice should have 5 permanents: 2 lands + 1 artifact + 1 creature + 1 enchantment
        let alice_count = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == alice)
            .count();
        assert_eq!(alice_count, 5);
    }

    #[test]
    fn test_gearhulk_affects_both_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Alice has creatures and lands
        let _alice_creature = create_creature(&mut game, "Alice Creature", alice);
        let alice_land = create_land(&mut game, "Alice Land", alice);

        // Bob has multiple creatures and lands
        let _bob_creature1 = create_creature(&mut game, "Bob Creature 1", bob);
        let _bob_creature2 = create_creature(&mut game, "Bob Creature 2", bob);
        let bob_land = create_land(&mut game, "Bob Land", bob);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        for effect in gearhulk_effects() {
            let _ = effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // Both lands should survive
        assert!(game.battlefield.contains(&alice_land));
        assert!(game.battlefield.contains(&bob_land));

        // Alice should have 2 permanents (1 creature + 1 land)
        let alice_count = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == alice)
            .count();
        assert_eq!(alice_count, 2);

        // Bob should have 2 permanents (1 creature + 1 land) - extra creature sacrificed
        let bob_count = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == bob)
            .count();
        assert_eq!(bob_count, 2);
    }

    #[test]
    fn test_gearhulk_with_only_lands() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Alice has only lands
        let land1 = create_land(&mut game, "Plains", alice);
        let land2 = create_land(&mut game, "Forest", alice);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(source, alice);
        let mut result = None;
        for effect in gearhulk_effects() {
            result = Some(effect.0.execute(&mut game, &mut ctx).unwrap());
        }

        // Nothing should be sacrificed (lands are safe)
        assert_eq!(result.unwrap().result, EffectResult::Count(0));
        assert!(game.battlefield.contains(&land1));
        assert!(game.battlefield.contains(&land2));
    }

    #[test]
    fn test_gearhulk_empty_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Execute the effect on empty battlefield
        let mut ctx = ExecutionContext::new_default(source, alice);
        let mut result = None;
        for effect in gearhulk_effects() {
            result = Some(effect.0.execute(&mut game, &mut ctx).unwrap());
        }

        assert_eq!(result.unwrap().result, EffectResult::Count(0));
    }

    // ========================================
    // Trigger Detection Tests
    // ========================================

    #[test]
    fn test_gearhulk_triggers_on_etb() {
        use crate::events::zones::ZoneChangeEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Gearhulk on the battlefield
        let def = cataclysmic_gearhulk();
        let gearhulk_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Simulate ETB event
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            gearhulk_id,
            Zone::Hand,
            Zone::Battlefield,
            None,
        ));

        let triggered = check_triggers(&game, &event);
        assert_eq!(triggered.len(), 1, "Gearhulk should trigger on ETB");
    }

    // ========================================
    // Rules Interaction Tests
    // ========================================

    #[test]
    fn test_gearhulk_oracle_mentions_nonland() {
        let def = cataclysmic_gearhulk();
        assert!(def.card.oracle_text.contains("nonland"));
    }

    #[test]
    fn test_gearhulk_oracle_mentions_planeswalker() {
        // Unlike Cataclysm, Gearhulk includes planeswalker
        let def = cataclysmic_gearhulk();
        assert!(def.card.oracle_text.contains("planeswalker"));
    }

    #[test]
    fn test_gearhulk_is_artifact_creature() {
        let def = cataclysmic_gearhulk();
        // Can be chosen as your artifact OR your creature
        assert!(def.card.has_card_type(CardType::Artifact));
        assert!(def.card.has_card_type(CardType::Creature));
    }
}
