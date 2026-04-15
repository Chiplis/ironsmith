use super::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

pub fn yasharn_implacable_earth() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Yasharn, Implacable Earth")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::White],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental, Subtype::Boar])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "When Yasharn, Implacable Earth enters the battlefield, search your library for a basic Forest card and a basic Plains card, reveal those cards, put them into your hand, then shuffle.\nPlayers can't pay life or sacrifice nonland permanents to cast spells or activate abilities.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::decision::{LegalAction, compute_legal_actions};
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::game_state::Phase;
    use crate::ids::{CardId, PlayerId};
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn yasharn_has_etb_trigger() {
        let def = yasharn_implacable_earth();

        assert!(def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Triggered(triggered)
                if triggered.trigger.display().contains("enters")
        )));
    }

    #[test]
    fn yasharn_etb_searches_forest_and_plains_to_hand() {
        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let def = yasharn_implacable_earth();
        let yasharn_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        game.create_object_from_definition(
            &crate::cards::definitions::basic_forest(),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Filler Card")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Library,
        );
        game.create_object_from_definition(
            &crate::cards::definitions::basic_plains(),
            alice,
            Zone::Library,
        );

        let trigger = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(&triggered.effects[0]),
                _ => None,
            })
            .expect("yasharn should have an ETB trigger");

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(yasharn_id, alice, &mut dm);
        execute_effect(&mut game, trigger, &mut ctx).expect("yasharn trigger should resolve");

        let hand_names: Vec<_> = game
            .player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .filter_map(|&id| game.object(id).map(|object| object.name.clone()))
            .collect();
        assert!(
            hand_names.iter().any(|name| name == "Forest"),
            "Yasharn should put a basic Forest into hand, got {hand_names:?}"
        );
        assert!(
            hand_names.iter().any(|name| name == "Plains"),
            "Yasharn should put a basic Plains into hand, got {hand_names:?}"
        );
    }

    #[test]
    fn yasharn_does_not_stop_shockland_life_payment() {
        let game = run_replay_test(
            vec![
                "1", // Play Godless Shrine
                "1", // Pay 2 life so it enters untapped
                "",  // Pass priority
            ],
            ReplayTestConfig::new()
                .p1_battlefield(vec!["Yasharn, Implacable Earth"])
                .p1_hand(vec!["Godless Shrine"]),
        );

        let alice = PlayerId::from_index(0);
        let shrine_id = game
            .battlefield
            .iter()
            .find(|&&id| {
                game.object(id)
                    .is_some_and(|object| object.name == "Godless Shrine")
            })
            .copied()
            .expect("Godless Shrine should be on the battlefield");

        assert!(
            !game.is_tapped(shrine_id),
            "Godless Shrine should still be allowed to pay life while entering"
        );
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
    }

    #[test]
    fn yasharn_prevents_fetchland_activation() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        game.create_object_from_definition(&yasharn_implacable_earth(), alice, Zone::Battlefield);
        let delta_id = game.create_object_from_definition(
            &crate::cards::definitions::polluted_delta(),
            alice,
            Zone::Battlefield,
        );

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == delta_id
            )),
            "Yasharn should stop Polluted Delta because its activation includes paying life"
        );
    }

    #[test]
    fn yasharn_does_not_stop_resolution_time_sacrifices() {
        let game = run_replay_test(
            vec![
                "1", // Cast Innocent Blood
                "0", // Tap Swamp for mana
                "0", // Sacrifice Yasharn
                "0", // Sacrifice Bob's creature
                "0", // Pass priority after resolution
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Innocent Blood"])
                .p1_battlefield(vec!["Swamp", "Yasharn, Implacable Earth"])
                .p2_battlefield(vec!["Llanowar Elves"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        assert!(
            game.player(alice)
                .expect("alice exists")
                .graveyard
                .iter()
                .any(|&id| {
                    game.object(id)
                        .is_some_and(|object| object.name == "Yasharn, Implacable Earth")
                }),
            "Yasharn should still be sacrificed by Innocent Blood because that sacrifice is on resolution"
        );
        assert!(
            game.player(bob)
                .expect("bob exists")
                .graveyard
                .iter()
                .any(|&id| {
                    game.object(id)
                        .is_some_and(|object| object.name == "Llanowar Elves")
                }),
            "Bob's creature should still be sacrificed as part of Innocent Blood resolving"
        );
    }
}
