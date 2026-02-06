//! Black Market Connections card definition.

use crate::ability::Ability;
use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::types::{CardType, Subtype};

/// Black Market Connections - {2}{B}
/// Enchantment
/// At the beginning of your precombat main phase, choose one or more —
/// • Sell Contraband — Create a Treasure token. You lose 1 life.
/// • Buy Information — Draw a card. You lose 2 life.
/// • Hire a Mercenary — Create a 3/2 colorless Shapeshifter creature token
///   with changeling. You lose 3 life.
pub fn black_market_connections() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Black Market Connections")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your precombat main phase, choose one or more —\n\
             • Sell Contraband — Create a Treasure token. You lose 1 life.\n\
             • Buy Information — Draw a card. You lose 2 life.\n\
             • Hire a Mercenary — Create a 3/2 colorless Shapeshifter creature token with changeling. You lose 3 life.",
        )
        .expect("Card text should be supported")
}

/// Creates a 3/2 colorless Shapeshifter creature token with changeling.
fn shapeshifter_mercenary_token() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Shapeshifter")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Shapeshifter])
        .power_toughness(PowerToughness::fixed(3, 2))
        .with_ability(Ability::static_ability(StaticAbility::changeling()))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Zone;
    use crate::ability::AbilityKind;
    use crate::cards::tokens::treasure_token_definition;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectOptionsContext;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;

    #[test]
    fn test_black_market_connections_basic_properties() {
        let def = black_market_connections();
        assert_eq!(def.name(), "Black Market Connections");
        assert!(def.card.is_enchantment());
        assert_eq!(def.card.mana_value(), 3); // {2}{B} = 3

        // Check mana cost contains black
        let mana_cost = def.card.mana_cost.as_ref().expect("Should have mana cost");
        // ManaCost stores pips - the {B} pip means it includes black
        assert_eq!(mana_cost.mana_value(), 3);
    }

    #[test]
    fn test_has_triggered_ability() {
        let def = black_market_connections();

        // Should have exactly one ability (the triggered ability)
        assert_eq!(def.abilities.len(), 1);

        // Verify it's a triggered ability
        let ability = &def.abilities[0];
        assert!(matches!(ability.kind, AbilityKind::Triggered(_)));

        // Verify the trigger condition (now using Trigger struct)
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            assert!(
                triggered.trigger.display().contains("main phase"),
                "Should trigger on main phase"
            );
            assert!(triggered.choices.is_empty());
        }
    }

    #[test]
    fn test_triggered_ability_has_modal_effect() {
        let def = black_market_connections();
        let ability = &def.abilities[0];

        if let AbilityKind::Triggered(triggered) = &ability.kind {
            // Should have one effect (the choose_up_to modal effect)
            assert_eq!(triggered.effects.len(), 1);

            // Verify it's a ChooseModeEffect
            let effect_debug = format!("{:?}", triggered.effects[0]);
            assert!(
                effect_debug.contains("ChooseModeEffect"),
                "Effect should be ChooseModeEffect, got: {}",
                effect_debug
            );
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_treasure_token_definition() {
        let token = treasure_token_definition();
        assert_eq!(token.name(), "Treasure");
        assert!(token.card.is_token);
        assert!(token.card.card_types.contains(&CardType::Artifact));
        assert!(token.card.subtypes.contains(&Subtype::Treasure));
        assert!(token.card.power_toughness.is_none());
    }

    #[test]
    fn test_treasure_token_has_mana_ability() {
        use crate::object::Object;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a Treasure token
        let token_id = game.new_object_id();
        let token_def = treasure_token_definition();
        let token_obj = Object::from_token_definition(token_id, &token_def, alice);
        game.add_object(token_obj);

        let token = game.object(token_id).unwrap();

        // Verify the token has exactly one ability (the mana ability)
        let mana_abilities: Vec<_> = token
            .abilities
            .iter()
            .filter(|a| a.is_mana_ability())
            .collect();
        assert_eq!(
            mana_abilities.len(),
            1,
            "Treasure should have exactly one mana ability"
        );

        // Verify the mana ability has tap + sacrifice cost
        let mana_ability = mana_abilities[0];
        if let AbilityKind::Mana(ma) = &mana_ability.kind {
            assert!(
                ma.has_tap_cost(),
                "Treasure mana ability should require tap"
            );
            // Check for SacrificeSelf in the cost_effects
            assert!(
                ma.has_sacrifice_self_cost(),
                "Treasure mana ability should require sacrifice self"
            );
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_treasure_mana_ability_adds_any_color() {
        use crate::object::Object;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a Treasure token
        let token_id = game.new_object_id();
        let token_def = treasure_token_definition();
        let token_obj = Object::from_token_definition(token_id, &token_def, alice);
        game.add_object(token_obj);

        let token = game.object(token_id).unwrap();

        // Get the mana ability
        let mana_ability = token
            .abilities
            .iter()
            .find(|a| a.is_mana_ability())
            .expect("Should have mana ability");

        // Verify the effect is add mana of any color
        if let AbilityKind::Mana(ma) = &mana_ability.kind {
            let effects = ma.effects.as_ref().expect("Should have effects");
            assert_eq!(effects.len(), 1, "Should have 1 effect");

            // Check the effect is AddManaOfAnyColorEffect
            let debug_str = format!("{:?}", &effects[0]);
            assert!(
                debug_str.contains("AddManaOfAnyColorEffect"),
                "Effect should be AddManaOfAnyColorEffect, got: {}",
                debug_str
            );
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_shapeshifter_token_definition() {
        let token = shapeshifter_mercenary_token();
        assert_eq!(token.name(), "Shapeshifter");
        assert!(token.card.is_token);
        assert!(token.card.card_types.contains(&CardType::Creature));
        assert!(token.card.subtypes.contains(&Subtype::Shapeshifter));
        let pt = token
            .card
            .power_toughness
            .as_ref()
            .expect("Should have P/T");
        use crate::card::PtValue;
        assert_eq!(pt.power, PtValue::Fixed(3));
        assert_eq!(pt.toughness, PtValue::Fixed(2));
    }

    /// A decision maker that always chooses specific modes for testing.
    struct TestDecisionMaker {
        modes_to_choose: Vec<usize>,
    }

    impl DecisionMaker for TestDecisionMaker {
        fn decide_options(&mut self, _game: &GameState, _ctx: &SelectOptionsContext) -> Vec<usize> {
            self.modes_to_choose.clone()
        }
    }

    #[test]
    fn test_mode_sell_contraband_creates_treasure_and_loses_life() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let initial_life = game.player(alice).unwrap().life;
        let source = game.new_object_id();

        // Create execution context with decision maker that chooses mode 0 (Sell Contraband)
        let mut dm = TestDecisionMaker {
            modes_to_choose: vec![0],
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        // Get the modal effect from the ability
        let def = black_market_connections();
        if let AbilityKind::Triggered(triggered) = &def.abilities[0].kind {
            let modal_effect = &triggered.effects[0];
            modal_effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // Verify Alice lost 1 life
        assert_eq!(
            game.player(alice).unwrap().life,
            initial_life - 1,
            "Sell Contraband should cost 1 life"
        );

        // Verify a Treasure token was created on the battlefield
        let treasures: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.name == "Treasure" && obj.controller == alice)
            .collect();
        assert_eq!(treasures.len(), 1, "Should have created 1 Treasure token");
        assert!(treasures[0].card_types.contains(&CardType::Artifact));
        assert!(treasures[0].subtypes.contains(&Subtype::Treasure));
    }

    #[test]
    fn test_mode_buy_information_draws_card_and_loses_life() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let initial_life = game.player(alice).unwrap().life;

        // Add some cards to Alice's library so draw works
        for i in 0..5 {
            let card_id = game.new_object_id();
            let card = crate::object::Object::from_card(
                card_id,
                &crate::card::CardBuilder::new(
                    crate::ids::CardId::from_raw(i + 100),
                    format!("Test Card {}", i),
                )
                .card_types(vec![CardType::Instant])
                .build(),
                alice,
                Zone::Library,
            );
            game.add_object(card);
        }

        let initial_library_size = game.player(alice).unwrap().library.len();
        let initial_hand_size = game.player(alice).unwrap().hand.len();

        let source = game.new_object_id();
        let mut dm = TestDecisionMaker {
            modes_to_choose: vec![1], // Buy Information
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let def = black_market_connections();
        if let AbilityKind::Triggered(triggered) = &def.abilities[0].kind {
            let modal_effect = &triggered.effects[0];
            modal_effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // Verify Alice lost 2 life
        assert_eq!(
            game.player(alice).unwrap().life,
            initial_life - 2,
            "Buy Information should cost 2 life"
        );

        // Verify Alice drew 1 card
        assert_eq!(
            game.player(alice).unwrap().library.len(),
            initial_library_size - 1,
            "Should have drawn from library"
        );
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            initial_hand_size + 1,
            "Should have added card to hand"
        );
    }

    #[test]
    fn test_mode_hire_mercenary_creates_shapeshifter_and_loses_life() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let initial_life = game.player(alice).unwrap().life;
        let source = game.new_object_id();

        let mut dm = TestDecisionMaker {
            modes_to_choose: vec![2], // Hire a Mercenary
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let def = black_market_connections();
        if let AbilityKind::Triggered(triggered) = &def.abilities[0].kind {
            let modal_effect = &triggered.effects[0];
            modal_effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // Verify Alice lost 3 life
        assert_eq!(
            game.player(alice).unwrap().life,
            initial_life - 3,
            "Hire a Mercenary should cost 3 life"
        );

        // Verify a Shapeshifter token was created
        let shapeshifters: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.name == "Shapeshifter" && obj.controller == alice)
            .collect();
        assert_eq!(
            shapeshifters.len(),
            1,
            "Should have created 1 Shapeshifter token"
        );
        assert!(shapeshifters[0].card_types.contains(&CardType::Creature));
        assert!(shapeshifters[0].subtypes.contains(&Subtype::Shapeshifter));
        assert_eq!(shapeshifters[0].power(), Some(3));
        assert_eq!(shapeshifters[0].toughness(), Some(2));
        // Verify colorless
        if let Some(colors) = shapeshifters[0].color_override {
            assert!(colors.is_empty(), "Shapeshifter should be colorless");
        }
    }

    #[test]
    fn test_choosing_multiple_modes() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let initial_life = game.player(alice).unwrap().life;

        // Add cards to library for the draw
        for i in 0..5 {
            let card_id = game.new_object_id();
            let card = crate::object::Object::from_card(
                card_id,
                &crate::card::CardBuilder::new(
                    crate::ids::CardId::from_raw(i + 200),
                    format!("Test Card {}", i),
                )
                .card_types(vec![CardType::Instant])
                .build(),
                alice,
                Zone::Library,
            );
            game.add_object(card);
        }

        let source = game.new_object_id();
        // Choose all three modes
        let mut dm = TestDecisionMaker {
            modes_to_choose: vec![0, 1, 2],
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let def = black_market_connections();
        if let AbilityKind::Triggered(triggered) = &def.abilities[0].kind {
            let modal_effect = &triggered.effects[0];
            modal_effect.0.execute(&mut game, &mut ctx).unwrap();
        }

        // Verify Alice lost 1 + 2 + 3 = 6 life total
        assert_eq!(
            game.player(alice).unwrap().life,
            initial_life - 6,
            "All three modes should cost 6 life total"
        );

        // Verify both tokens were created
        let treasures: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.name == "Treasure" && obj.controller == alice)
            .collect();
        assert_eq!(treasures.len(), 1, "Should have 1 Treasure");

        let shapeshifters: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.name == "Shapeshifter" && obj.controller == alice)
            .collect();
        assert_eq!(shapeshifters.len(), 1, "Should have 1 Shapeshifter");
    }

    #[test]
    fn test_ability_functions_only_on_battlefield() {
        let def = black_market_connections();
        let ability = &def.abilities[0];

        assert!(ability.functions_in(&Zone::Battlefield));
        assert!(!ability.functions_in(&Zone::Graveyard));
        assert!(!ability.functions_in(&Zone::Hand));
        assert!(!ability.functions_in(&Zone::Exile));
    }

    #[test]
    fn test_shapeshifter_has_changeling_all_creature_types() {
        // Create a Shapeshifter token and verify it has all creature types due to Changeling
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create the Shapeshifter token directly
        let token_id = game.new_object_id();
        let token_def = shapeshifter_mercenary_token();
        let token_obj = crate::object::Object::from_token_definition(token_id, &token_def, alice);
        game.add_object(token_obj);

        let token = game.object(token_id).unwrap();

        // Verify the token has Changeling ability
        assert!(
            token.has_changeling(),
            "Shapeshifter token should have Changeling"
        );

        // Verify it has all creature types via has_subtype
        assert!(
            token.has_subtype(Subtype::Elf),
            "Shapeshifter with Changeling should have Elf type"
        );
        assert!(
            token.has_subtype(Subtype::Goblin),
            "Shapeshifter with Changeling should have Goblin type"
        );
        assert!(
            token.has_subtype(Subtype::Human),
            "Shapeshifter with Changeling should have Human type"
        );
        assert!(
            token.has_subtype(Subtype::Vampire),
            "Shapeshifter with Changeling should have Vampire type"
        );
        assert!(
            token.has_subtype(Subtype::Zombie),
            "Shapeshifter with Changeling should have Zombie type"
        );
        assert!(
            token.has_subtype(Subtype::Dragon),
            "Shapeshifter with Changeling should have Dragon type"
        );

        // Verify it still has its explicit Shapeshifter type
        assert!(
            token.has_subtype(Subtype::Shapeshifter),
            "Should still have explicit Shapeshifter type"
        );

        // Verify non-creature subtypes are NOT affected by Changeling
        // (Changeling only grants creature types, not land types etc.)
        assert!(
            !token.has_subtype(Subtype::Plains),
            "Changeling should not grant land types"
        );
        assert!(
            !token.has_subtype(Subtype::Aura),
            "Changeling should not grant enchantment subtypes"
        );
    }
}
