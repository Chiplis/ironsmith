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
    use crate::effects::EffectExecutor;
    use crate::effects::cards::ImprintFromHandEffect;
    use crate::effects::mana::AddManaOfImprintedColorsEffect;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
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

        // First ability should be a triggered ability (ETB imprint)
        assert!(matches!(def.abilities[0].kind, AbilityKind::Triggered(_)));

        if let AbilityKind::Triggered(triggered) = &def.abilities[0].kind {
            // Now using Trigger struct - check display contains enters
            assert!(
                triggered.trigger.display().contains("enters"),
                "Should trigger on entering battlefield"
            );
        }
    }

    #[test]
    fn test_chrome_mox_has_mana_ability() {
        let def = chrome_mox();

        // Second ability should be a mana ability
        assert!(def.abilities[1].is_mana_ability());

        if let AbilityKind::Mana(mana_ability) = &def.abilities[1].kind {
            // Should have tap cost
            assert!(mana_ability.has_tap_cost());
            // Should have effects (not fixed mana)
            assert!(mana_ability.effects.is_some());
        }
    }

    // =========================================================================
    // Imprint Tests
    // =========================================================================

    #[test]
    fn test_chrome_mox_imprint_tracks_exiled_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Chrome Mox on battlefield
        let def = chrome_mox();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create a red instant in exile (simulating imprint)
        let bolt = create_red_instant(&mut game, alice);
        let exiled_id = game.move_object(bolt, Zone::Exile).unwrap();

        // Imprint the card
        game.imprint_card(mox_id, exiled_id);

        // Verify it's tracked
        assert!(game.has_imprinted_cards(mox_id));
        let imprinted = game.get_imprinted_cards(mox_id);
        assert_eq!(imprinted.len(), 1);
        assert_eq!(imprinted[0], exiled_id);
    }

    #[test]
    fn test_chrome_mox_no_imprint_no_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Chrome Mox on battlefield (no imprinted card)
        let def = chrome_mox();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify no imprinted cards
        assert!(!game.has_imprinted_cards(mox_id));

        // Execute the mana ability effect
        let mut ctx = ExecutionContext::new_default(mox_id, alice);
        let effect = AddManaOfImprintedColorsEffect::new();
        let result = EffectExecutor::execute(&effect, &mut game, &mut ctx).unwrap();

        // Should produce no mana
        assert_eq!(result.result, crate::effect::EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn test_chrome_mox_imprinted_red_produces_red() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Chrome Mox on battlefield
        let def = chrome_mox();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create and imprint a red card
        let bolt = create_red_instant(&mut game, alice);
        let exiled_id = game.move_object(bolt, Zone::Exile).unwrap();
        game.imprint_card(mox_id, exiled_id);

        // Execute the mana ability effect
        let mut ctx = ExecutionContext::new_default(mox_id, alice);
        let effect = AddManaOfImprintedColorsEffect::new();
        let result = EffectExecutor::execute(&effect, &mut game, &mut ctx).unwrap();

        // Should produce 1 red mana
        assert_eq!(result.result, crate::effect::EffectResult::Count(1));
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);
    }

    #[test]
    fn test_chrome_mox_imprinted_blue_produces_blue() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Chrome Mox on battlefield
        let def = chrome_mox();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create and imprint a blue card
        let delver = create_blue_creature(&mut game, alice);
        let exiled_id = game.move_object(delver, Zone::Exile).unwrap();
        game.imprint_card(mox_id, exiled_id);

        // Execute the mana ability effect
        let mut ctx = ExecutionContext::new_default(mox_id, alice);
        let effect = AddManaOfImprintedColorsEffect::new();
        let result = EffectExecutor::execute(&effect, &mut game, &mut ctx).unwrap();

        // Should produce 1 blue mana
        assert_eq!(result.result, crate::effect::EffectResult::Count(1));
        assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
    }

    #[test]
    fn test_chrome_mox_imprinted_colorless_produces_nothing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Chrome Mox on battlefield
        let def = chrome_mox();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create and imprint a colorless card
        let karn = create_colorless_spell(&mut game, alice);
        let exiled_id = game.move_object(karn, Zone::Exile).unwrap();
        game.imprint_card(mox_id, exiled_id);

        // Execute the mana ability effect
        let mut ctx = ExecutionContext::new_default(mox_id, alice);
        let effect = AddManaOfImprintedColorsEffect::new();
        let result = EffectExecutor::execute(&effect, &mut game, &mut ctx).unwrap();

        // Should produce no mana (colorless has no colors)
        assert_eq!(result.result, crate::effect::EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn test_chrome_mox_multicolor_imprint_one_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Chrome Mox on battlefield
        let def = chrome_mox();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create and imprint a blue/red card
        let electrolyze = create_multicolor_card(&mut game, alice);
        let exiled_id = game.move_object(electrolyze, Zone::Exile).unwrap();
        game.imprint_card(mox_id, exiled_id);

        // Execute the mana ability effect
        let mut ctx = ExecutionContext::new_default(mox_id, alice);
        let effect = AddManaOfImprintedColorsEffect::new();
        let result = EffectExecutor::execute(&effect, &mut game, &mut ctx).unwrap();

        // Should produce 1 mana (of either blue or red, player would choose)
        assert_eq!(result.result, crate::effect::EffectResult::Count(1));
        let pool = &game.player(alice).unwrap().mana_pool;
        assert_eq!(pool.blue + pool.red, 1); // One of these should be 1
    }

    // =========================================================================
    // Imprint Filter Tests (what CAN'T be imprinted)
    // =========================================================================

    #[test]
    fn test_imprint_filter_excludes_artifacts() {
        let filter = ImprintFromHandEffect::nonartifact_nonland().filter;
        assert!(filter.excluded_card_types.contains(&CardType::Artifact));
    }

    #[test]
    fn test_imprint_filter_excludes_lands() {
        let filter = ImprintFromHandEffect::nonartifact_nonland().filter;
        assert!(filter.excluded_card_types.contains(&CardType::Land));
    }

    // =========================================================================
    // Mox Leaves Battlefield Tests
    // =========================================================================

    #[test]
    fn test_chrome_mox_leaving_clears_imprint() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Chrome Mox on battlefield
        let def = chrome_mox();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create and imprint a red card
        let bolt = create_red_instant(&mut game, alice);
        let exiled_id = game.move_object(bolt, Zone::Exile).unwrap();
        game.imprint_card(mox_id, exiled_id);

        // Verify imprint exists
        assert!(game.has_imprinted_cards(mox_id));

        // Move mox to graveyard (destroyed)
        let _new_id = game.move_object(mox_id, Zone::Graveyard);

        // Imprint should be cleared (checked in clear_battlefield_state)
        // The old ID no longer has imprinted cards
        assert!(!game.has_imprinted_cards(mox_id));
    }

    #[test]
    fn test_chrome_mox_imprinted_card_leaves_exile_no_mana() {
        // Test that Chrome Mox produces no mana if the imprinted card is moved out of exile
        // (e.g., by Pull from Eternity or similar effects)
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Chrome Mox on battlefield
        let def = chrome_mox();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create and imprint a red card (move to exile, then imprint)
        let bolt = create_red_instant(&mut game, alice);
        let exiled_id = game.move_object(bolt, Zone::Exile).unwrap();
        game.imprint_card(mox_id, exiled_id);

        // Verify mana ability works while card is in exile
        let mut ctx = ExecutionContext::new_default(mox_id, alice);
        let effect = AddManaOfImprintedColorsEffect::new();
        let result = EffectExecutor::execute(&effect, &mut game, &mut ctx).unwrap();
        assert_eq!(result.result, crate::effect::EffectResult::Count(1));
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);

        // Clear mana pool for next test
        game.player_mut(alice).unwrap().mana_pool.red = 0;

        // Now move the imprinted card OUT of exile (simulating Pull from Eternity)
        // This gives it a new ObjectId, making the old imprinted ID stale
        let _new_id = game.move_object(exiled_id, Zone::Graveyard).unwrap();

        // The imprint tracking still points to the old (now invalid) ID
        assert!(game.has_imprinted_cards(mox_id));

        // But executing the mana ability should produce NO mana
        // because the object with that ID no longer exists
        let mut ctx2 = ExecutionContext::new_default(mox_id, alice);
        let result2 = EffectExecutor::execute(&effect, &mut game, &mut ctx2).unwrap();

        // Should produce no mana since the imprinted card is no longer in exile
        assert_eq!(result2.result, crate::effect::EffectResult::Count(0));
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
