use ironsmith::events::other::DieRolledEvent;
use ironsmith::triggers::Trigger;
use ironsmith::{
    Ability, CardBuilder, CardDefinition, CardId, CardType, CastingMethod, Effect, GameState,
    LegalAction, PlayerFilter, PlayerId, StaticAbility, Subtype, TotalCost, TriggerEvent, Zone,
    check_triggers,
};

fn vanguard(name: &str, hand_modifier: i32, life_modifier: i32) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Vanguard])
            .vanguard_modifiers(hand_modifier, life_modifier)
            .build(),
    )
}

#[test]
fn u062_vanguards_modify_life_hands_and_function_from_command() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 99);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut alice_vanguard = vanguard("Alice Avatar", 3, -3);
    alice_vanguard.abilities.extend([
        Ability::static_ability(StaticAbility::increase_maximum_hand_size(
            PlayerFilter::You,
            2,
        )),
        Ability::triggered(
            Trigger::player_rolls_die(PlayerFilter::You),
            vec![Effect::gain_life(1)],
        ),
        Ability::activated(TotalCost::free(), vec![Effect::gain_life(1)]),
    ]);

    game.enable_vanguard(vec![
        (alice, alice_vanguard),
        (bob, vanguard("Bob Avatar", -1, 5)),
    ])
    .expect("one valid Vanguard card per player");

    let alice_card = game.vanguard_card(alice).expect("Alice vanguard");
    assert_eq!(game.object(alice_card).unwrap().hand_modifier, 3);
    assert_eq!(game.object(alice_card).unwrap().life_modifier, -3);
    assert_eq!(game.player(alice).unwrap().starting_life, 17);
    assert_eq!(game.player(alice).unwrap().life, 17);
    assert_eq!(game.player(bob).unwrap().life, 25);
    assert_eq!(game.vanguard_starting_hand_size(alice), 10);
    assert_eq!(game.vanguard_starting_hand_size(bob), 6);
    assert_eq!(game.object(alice_card).unwrap().zone, Zone::Command);
    assert_eq!(game.controller_of_id(alice_card), Some(alice));
    assert!(
        game.object(alice_card)
            .unwrap()
            .abilities
            .iter()
            .all(|ability| ability.functional_zones == [Zone::Command])
    );

    game.update_cant_effects();
    assert_eq!(game.player(alice).unwrap().max_hand_size, 12);
    assert_eq!(game.player(bob).unwrap().max_hand_size, 6);
    assert!(
        ironsmith::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == alice_card))
    );
    assert!(!ironsmith::decision::can_cast_spell(
        &game,
        alice,
        game.object(alice_card).unwrap(),
        &CastingMethod::Normal,
    ));
    assert_eq!(
        game.move_object_by_effect(alice_card, Zone::Exile),
        Some(alice_card)
    );
    assert_eq!(game.object(alice_card).unwrap().zone, Zone::Command);

    game.set_current_controller(alice_card, bob);
    assert_eq!(game.controller_of_id(alice_card), Some(alice));
    let event = TriggerEvent::new_with_provenance(
        DieRolledEvent::new(alice, alice_card, 4, 6),
        ironsmith::provenance::ProvNodeId::default(),
    );
    let triggers = check_triggers(&game, &event);
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].source, alice_card);
    assert_eq!(triggers[0].controller, alice);
}

#[test]
fn u062_vanguard_setup_is_transactional_and_owned_cards_leave_with_their_player() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut invalid = vanguard("Subtype Avatar", 0, 0);
    invalid.card.subtypes.push(Subtype::Goblin);

    assert!(
        game.enable_vanguard(vec![(alice, invalid), (bob, vanguard("Bob Avatar", 0, 0)),])
            .is_err()
    );
    assert!(game.vanguard.is_none());
    assert!(game.command_zone.is_empty());

    game.enable_vanguard(vec![
        (alice, vanguard("Alice Avatar", 0, 0)),
        (bob, vanguard("Bob Avatar", 0, 0)),
    ])
    .expect("valid Vanguard setup");
    let bob_card = game.vanguard_card(bob).expect("Bob vanguard");
    assert!(game.leave_game(bob));
    assert!(game.vanguard_card(bob).is_none());
    assert!(game.object(bob_card).is_none());
    assert!(game.vanguard_card(alice).is_some());
}

#[test]
fn u062_restart_keeps_vanguards_in_command_and_reapplies_starting_values() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.enable_vanguard(vec![
        (alice, vanguard("Alice Avatar", 3, -3)),
        (bob, vanguard("Bob Avatar", -1, 5)),
    ])
    .expect("valid Vanguard setup");

    for (player, prefix) in [(alice, "A"), (bob, "B")] {
        for index in 0..12 {
            let definition = CardDefinition::new(
                CardBuilder::new(CardId::new(), format!("{prefix} Card {index}"))
                    .card_types(vec![CardType::Artifact])
                    .build(),
            );
            game.create_object_from_definition(&definition, player, Zone::Library);
        }
    }

    game.restart_game(alice, &[]);

    assert_eq!(game.player(alice).unwrap().starting_life, 17);
    assert_eq!(game.player(alice).unwrap().life, 17);
    assert_eq!(game.player(bob).unwrap().starting_life, 25);
    assert_eq!(game.player(bob).unwrap().life, 25);
    assert_eq!(game.player(alice).unwrap().hand.len(), 10);
    assert_eq!(game.player(bob).unwrap().hand.len(), 6);
    assert_eq!(game.player(alice).unwrap().max_hand_size, 10);
    assert_eq!(game.player(bob).unwrap().max_hand_size, 6);
    for player in [alice, bob] {
        let vanguard = game.vanguard_card(player).expect("restarted vanguard");
        assert_eq!(game.object(vanguard).unwrap().zone, Zone::Command);
        assert!(game.command_zone.contains(&vanguard));
    }
}
