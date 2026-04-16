//! Shelter card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Shelter - {1}{W}
/// Instant
/// Target creature you control gains protection from the color of your choice until end of turn.
/// Draw a card.
pub fn shelter() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Shelter")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target creature you control gains protection from the color of your choice until end of turn. Draw a card.",
        )
        .expect("Card text should be supported")
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::card::PowerToughness;
    use crate::color::Color;
    use crate::decision::AutoPassDecisionMaker;
    use crate::effects::ExecutionContext;
    use crate::effects::ResolvedTarget;
    use crate::effects::execute_effect;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_shelter_basic_properties() {
        let def = shelter();
        assert_eq!(def.name(), "Shelter");
        assert!(def.is_spell());
        assert_eq!(def.card.mana_value(), 2);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_shelter_is_instant() {
        let def = shelter();
        assert!(def.card.is_instant());
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_shelter_is_white() {
        let def = shelter();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_shelter_has_three_effects() {
        let def = shelter();
        // TargetOnlyEffect + Protection effect + Draw effect
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 3);
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_shelter_effect_grants_protection() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature for Alice
        let soldier = create_creature(&mut game, "Soldier", alice);

        let def = shelter();
        let effects = def
            .spell_effect
            .as_ref()
            .expect("Shelter should have spell effects");

        let source_id = game.new_object_id();
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        ctx.targets = vec![ResolvedTarget::Object(soldier)];

        // Execute all effects (TargetOnly + ChooseMode/GrantProtection + Draw)
        for effect in effects {
            let _ = execute_effect(&mut game, effect, &mut ctx).unwrap();
        }

        // The soldier should now have protection from white (default when no decision maker)
        let chars = game
            .calculated_characteristics(soldier)
            .expect("Should calculate characteristics");

        // Check that the soldier has protection
        let has_protection = chars.static_abilities.iter().any(|a| a.has_protection());
        assert!(has_protection, "Soldier should have protection");
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_shelter_effect_fails_without_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = shelter();
        let effect = def
            .spell_effect
            .as_ref()
            .expect("Shelter should have spell effects")
            .first()
            .expect("Shelter should have a protection effect");

        let source_id = game.new_object_id();
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        ctx.targets = vec![]; // No target

        let result = execute_effect(&mut game, effect, &mut ctx);

        assert!(result.is_err(), "Should fail without a target");
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_shelter_only_affects_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create multiple creatures
        let soldier = create_creature(&mut game, "Soldier", alice);
        let knight = create_creature(&mut game, "Knight", alice);

        let def = shelter();
        let effects = def
            .spell_effect
            .as_ref()
            .expect("Shelter should have spell effects");

        // Execute all effects targeting only the soldier
        let source_id = game.new_object_id();
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        ctx.targets = vec![ResolvedTarget::Object(soldier)];

        for effect in effects {
            let _ = execute_effect(&mut game, effect, &mut ctx).unwrap();
        }

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

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_shelter_oracle_text() {
        let def = shelter();
        assert!(def.card.oracle_text.contains("Target creature you control"));
        assert!(def.card.oracle_text.contains("protection"));
        assert!(def.card.oracle_text.contains("color of your choice"));
        assert!(def.card.oracle_text.contains("until end of turn"));
        assert!(def.card.oracle_text.contains("Draw a card"));
    }
}
