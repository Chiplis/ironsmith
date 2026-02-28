//! Chrome Mox card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::ManaCost;
use crate::types::CardType;

/// Creates the Chrome Mox card definition.
///
/// Chrome Mox {0}
/// Artifact
/// Imprint — When Chrome Mox enters the battlefield, you may exile a nonartifact,
/// nonland card from your hand.
/// {T}: Add one mana of any of the exiled card's colors.
pub fn chrome_mox() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Chrome Mox")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Artifact])
        .parse_text("Imprint — When Chrome Mox enters the battlefield, you may exile a nonartifact, nonland card from your hand.\n{T}: Add one mana of any of the exiled card's colors.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::CardBuilder;
    use crate::effect::EffectResult;
    use crate::effects::cards::ImprintFromHandEffect;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_chrome_mox(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let def = chrome_mox();
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    fn create_red_instant(game: &mut GameState, owner: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Lightning Bolt")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    fn create_blue_creature(game: &mut GameState, owner: PlayerId) -> crate::ids::ObjectId {
        use crate::card::PowerToughness;
        let card = CardBuilder::new(CardId::new(), "Delver of Secrets")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    fn create_multicolor_card(game: &mut GameState, owner: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Electrolyze")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Red],
                vec![ManaSymbol::Generic(1)],
            ]))
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    fn create_colorless_spell(game: &mut GameState, owner: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Karn Liberated")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
            .card_types(vec![CardType::Planeswalker])
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    fn create_artifact_card(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Sol Ring")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    fn create_land_card(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Mountain")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    fn execute_chrome_mox_imprint_trigger(game: &mut GameState, controller: PlayerId, source: ObjectId) {
        let def = chrome_mox();
        let triggered = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .expect("Chrome Mox should have an imprint trigger");
        let AbilityKind::Triggered(triggered) = &triggered.kind else {
            unreachable!("Expected triggered ability");
        };

        let mut ctx = ExecutionContext::new_default(source, controller);
        for effect in &triggered.effects {
            effect.0.execute(game, &mut ctx).unwrap();
        }
    }

    fn execute_chrome_mox_mana_ability(
        game: &mut GameState,
        controller: PlayerId,
        source: ObjectId,
    ) -> EffectResult {
        let def = chrome_mox();
        let mana_ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Activated(_)))
            .expect("Chrome Mox should have a mana ability");
        let AbilityKind::Activated(activated) = &mana_ability.kind else {
            unreachable!("Expected activated ability");
        };

        let mut ctx = ExecutionContext::new_default(source, controller);
        let mut last_result = EffectResult::Resolved;
        for effect in &activated.effects {
            last_result = effect.0.execute(game, &mut ctx).unwrap().result;
        }
        last_result
    }

    // =========================================================================
    // Basic Properties Tests
    // =========================================================================

    #[test]
    fn test_chrome_mox_basic_properties() {
        let def = chrome_mox();

        // Check name
        assert_eq!(def.name(), "Chrome Mox");

        // Check it's an artifact
        assert!(def.card.is_artifact());
        assert!(def.card.card_types.contains(&CardType::Artifact));

        // Check mana cost is {0}
        assert_eq!(def.card.mana_value(), 0);

        // Check it's colorless
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_chrome_mox_has_two_abilities() {
        let def = chrome_mox();

        // Should have 2 abilities: triggered (imprint) and mana
        assert_eq!(def.abilities.len(), 2);
    }

    #[test]
    fn test_chrome_mox_has_etb_trigger() {
        let def = chrome_mox();

        let triggered = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .expect("Expected a triggered ability");
        let AbilityKind::Triggered(triggered) = &triggered.kind else {
            unreachable!("Expected triggered ability");
        };

        assert!(
            triggered.trigger.display().contains("enters"),
            "Should trigger on entering battlefield"
        );
        assert_eq!(triggered.effects.len(), 1);
        assert!(
            triggered.effects[0]
                .downcast_ref::<ImprintFromHandEffect>()
                .is_some(),
            "ETB trigger should compile to ImprintFromHandEffect"
        );
    }

    #[test]
    fn test_chrome_mox_has_mana_ability() {
        let def = chrome_mox();

        // Second ability should be a mana ability
        assert!(def.abilities[1].is_mana_ability());

        if let AbilityKind::Activated(mana_ability) = &def.abilities[1].kind {
            assert!(mana_ability.is_mana_ability());
            // Should have tap cost
            assert!(mana_ability.has_tap_cost());
            // Should have effects (not fixed mana)
            assert!(!mana_ability.effects.is_empty());
        }
    }

    // =========================================================================
    // Imprint Tests
    // =========================================================================

    #[test]
    fn test_chrome_mox_imprint_tracks_exiled_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mox_id = create_chrome_mox(&mut game, alice);
        let _bolt = create_red_instant(&mut game, alice);

        execute_chrome_mox_imprint_trigger(&mut game, alice, mox_id);

        assert!(game.has_imprinted_cards(mox_id));
        let imprinted = game.get_imprinted_cards(mox_id);
        assert_eq!(imprinted.len(), 1);
        let imprinted_object = game.object(imprinted[0]).expect("Imprinted card should exist");
        assert_eq!(imprinted_object.zone, Zone::Exile);
    }

    #[test]
    fn test_chrome_mox_no_imprint_no_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mox_id = create_chrome_mox(&mut game, alice);
        execute_chrome_mox_imprint_trigger(&mut game, alice, mox_id);

        assert!(!game.has_imprinted_cards(mox_id));

        let result = execute_chrome_mox_mana_ability(&mut game, alice, mox_id);
        assert_eq!(result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn test_chrome_mox_imprinted_red_produces_red() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mox_id = create_chrome_mox(&mut game, alice);
        let _bolt = create_red_instant(&mut game, alice);
        execute_chrome_mox_imprint_trigger(&mut game, alice, mox_id);

        let result = execute_chrome_mox_mana_ability(&mut game, alice, mox_id);
        assert_eq!(result, EffectResult::Count(1));
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);
    }

    #[test]
    fn test_chrome_mox_imprinted_blue_produces_blue() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mox_id = create_chrome_mox(&mut game, alice);
        let _delver = create_blue_creature(&mut game, alice);
        execute_chrome_mox_imprint_trigger(&mut game, alice, mox_id);

        let result = execute_chrome_mox_mana_ability(&mut game, alice, mox_id);
        assert_eq!(result, EffectResult::Count(1));
        assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
    }

    #[test]
    fn test_chrome_mox_imprinted_colorless_produces_nothing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mox_id = create_chrome_mox(&mut game, alice);
        let _karn = create_colorless_spell(&mut game, alice);
        execute_chrome_mox_imprint_trigger(&mut game, alice, mox_id);

        let result = execute_chrome_mox_mana_ability(&mut game, alice, mox_id);
        assert_eq!(result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn test_chrome_mox_multicolor_imprint_one_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mox_id = create_chrome_mox(&mut game, alice);
        let _electrolyze = create_multicolor_card(&mut game, alice);
        execute_chrome_mox_imprint_trigger(&mut game, alice, mox_id);

        let result = execute_chrome_mox_mana_ability(&mut game, alice, mox_id);
        assert_eq!(result, EffectResult::Count(1));
        let pool = &game.player(alice).unwrap().mana_pool;
        assert_eq!(pool.blue + pool.red, 1);
    }

    #[test]
    fn test_chrome_mox_imprint_filter_excludes_artifacts() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mox_id = create_chrome_mox(&mut game, alice);
        let artifact_id = create_artifact_card(&mut game, alice);
        execute_chrome_mox_imprint_trigger(&mut game, alice, mox_id);

        assert!(!game.has_imprinted_cards(mox_id));
        assert!(game.player(alice).unwrap().hand.contains(&artifact_id));
    }

    #[test]
    fn test_chrome_mox_imprint_filter_excludes_lands() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mox_id = create_chrome_mox(&mut game, alice);
        let land_id = create_land_card(&mut game, alice);
        execute_chrome_mox_imprint_trigger(&mut game, alice, mox_id);

        assert!(!game.has_imprinted_cards(mox_id));
        assert!(game.player(alice).unwrap().hand.contains(&land_id));
    }

    // =========================================================================
    // Mox Leaves Battlefield Tests
    // =========================================================================

    #[test]
    fn test_chrome_mox_leaving_clears_imprint() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mox_id = create_chrome_mox(&mut game, alice);
        let _bolt = create_red_instant(&mut game, alice);
        execute_chrome_mox_imprint_trigger(&mut game, alice, mox_id);
        assert!(game.has_imprinted_cards(mox_id));

        let _new_id = game.move_object(mox_id, Zone::Graveyard);
        assert!(!game.has_imprinted_cards(mox_id));
    }

    #[test]
    fn test_chrome_mox_imprinted_card_leaves_exile_no_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mox_id = create_chrome_mox(&mut game, alice);
        let _bolt = create_red_instant(&mut game, alice);
        execute_chrome_mox_imprint_trigger(&mut game, alice, mox_id);

        let imprinted_id = game.get_imprinted_cards(mox_id)[0];

        let result = execute_chrome_mox_mana_ability(&mut game, alice, mox_id);
        assert_eq!(result, EffectResult::Count(1));
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);

        game.player_mut(alice).unwrap().mana_pool.red = 0;

        let _new_id = game.move_object(imprinted_id, Zone::Graveyard).unwrap();
        assert!(game.has_imprinted_cards(mox_id));

        let result2 = execute_chrome_mox_mana_ability(&mut game, alice, mox_id);
        assert_eq!(result2, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }

    // =========================================================================
    // Oracle Text Tests
    // =========================================================================

    #[test]
    fn test_chrome_mox_oracle_text() {
        let def = chrome_mox();

        assert!(def.card.oracle_text.contains("Imprint"));
        assert!(def.card.oracle_text.contains("nonartifact"));
        assert!(def.card.oracle_text.contains("nonland"));
        assert!(def.card.oracle_text.contains("exile"));
        assert!(def.card.oracle_text.contains("colors"));
    }
}
