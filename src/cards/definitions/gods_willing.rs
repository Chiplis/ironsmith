//! Gods Willing card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Gods Willing - {W}
/// Instant
/// Target creature you control gains protection from the color of your choice until end of turn.
/// Scry 1.
pub fn gods_willing() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Gods Willing")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target creature you control gains protection from the color of your choice until end of turn. Scry 1.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::card::PowerToughness;
    use crate::color::Color;
    use crate::effect::EffectResult;
    use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_gods_willing_basic_properties() {
        let def = gods_willing();
        assert_eq!(def.name(), "Gods Willing");
        assert!(def.is_spell());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_gods_willing_is_instant() {
        let def = gods_willing();
        assert!(def.card.is_instant());
    }

    #[test]
    fn test_gods_willing_is_white() {
        let def = gods_willing();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_gods_willing_has_two_effects() {
        let def = gods_willing();
        // Protection effect + Scry effect
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 2);
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_gods_willing_effect_grants_protection() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature for Alice
        let soldier = create_creature(&mut game, "Soldier", alice);

        // Execute the effect with the soldier as the target
        let source_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.targets = vec![ResolvedTarget::Object(soldier)];

        let def = gods_willing();
        let effect = def
            .spell_effect
            .as_ref()
            .and_then(|effects| effects.first())
            .expect("Should have spell effect");
        let result = execute_effect(&mut game, effect, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // The soldier should now have protection from white (default when no decision maker)
        let chars = game
            .calculated_characteristics(soldier)
            .expect("Should calculate characteristics");

        // Check that the soldier has protection
        let has_protection = chars.static_abilities.iter().any(|a| a.has_protection());
        assert!(has_protection, "Soldier should have protection");
    }

    #[test]
    fn test_gods_willing_effect_fails_without_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Execute without a target
        let source_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.targets = vec![]; // No target

        let def = gods_willing();
        let effect = def
            .spell_effect
            .as_ref()
            .and_then(|effects| effects.first())
            .expect("Should have spell effect");
        let result = execute_effect(&mut game, effect, &mut ctx);

        assert!(result.is_err(), "Should fail without a target");
    }

    #[test]
    fn test_gods_willing_only_affects_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create multiple creatures
        let soldier = create_creature(&mut game, "Soldier", alice);
        let knight = create_creature(&mut game, "Knight", alice);

        // Execute the effect targeting only the soldier
        let source_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.targets = vec![ResolvedTarget::Object(soldier)];

        let def = gods_willing();
        let effect = def
            .spell_effect
            .as_ref()
            .and_then(|effects| effects.first())
            .expect("Should have spell effect");
        let _ = execute_effect(&mut game, effect, &mut ctx).unwrap();

        // The soldier should have protection
        {
            let chars = game
                .calculated_characteristics(soldier)
                .expect("Should calculate characteristics");
            let has_protection = chars.static_abilities.iter().any(|a| a.has_protection());
            assert!(has_protection, "Soldier should have protection");
        }

        // The knight should NOT have protection
        {
            let chars = game
                .calculated_characteristics(knight)
                .expect("Should calculate characteristics");
            let has_protection = chars.static_abilities.iter().any(|a| a.has_protection());
            assert!(!has_protection, "Knight should NOT have protection");
        }
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_gods_willing_oracle_text() {
        let def = gods_willing();
        assert!(def.card.oracle_text.contains("Target creature you control"));
        assert!(def.card.oracle_text.contains("protection"));
        assert!(def.card.oracle_text.contains("color of your choice"));
        assert!(def.card.oracle_text.contains("until end of turn"));
        assert!(def.card.oracle_text.contains("Scry 1"));
    }
}
