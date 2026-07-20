use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::special_actions::{ActionError, SpecialAction, can_perform_check, perform};
use ironsmith::{
    Ability, CardBuilder, CardDefinition, CardId, CardType, CompanionDeckCondition, GameState,
    LegalAction, ManaCost, ManaSymbol, Phase, PlayerId, StackEntry, StaticAbility, Zone,
    compute_legal_actions,
};

fn definition(
    name: &str,
    mana: Vec<ManaSymbol>,
    types: Vec<CardType>,
    companion: Option<CompanionDeckCondition>,
) -> CardDefinition {
    let card = CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_symbols(mana))
        .card_types(types)
        .build();
    let abilities = companion
        .map(|condition| {
            Ability::static_ability(StaticAbility::companion(condition, "Companion test"))
        })
        .into_iter()
        .collect();
    CardDefinition::with_abilities(card, abilities)
}

fn setup_even_companion_game() -> (GameState, PlayerId, PlayerId, ironsmith::ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let companion = definition(
        "Even Companion",
        vec![ManaSymbol::Generic(4)],
        vec![CardType::Creature],
        Some(CompanionDeckCondition::OnlyManaValueParity {
            even: true,
            lands_are_exempt: false,
        }),
    );
    let even_card = definition(
        "Even Deck Card",
        vec![ManaSymbol::Generic(2)],
        vec![CardType::Creature],
        None,
    );
    let companion_id = game.create_object_from_definition(&companion, alice, Zone::OutsideGame);
    let deck_id = game.create_object_from_definition(&even_card, alice, Zone::Library);
    game.designate_companion(alice, companion_id, &[deck_id], 1)
        .expect("the even starting deck fulfills the condition");
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    (game, alice, bob, companion_id)
}

#[test]
fn designation_validates_the_complete_deck_and_rejects_without_mutation() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let companion = definition(
        "Even Companion",
        vec![ManaSymbol::Generic(4)],
        vec![CardType::Creature],
        Some(CompanionDeckCondition::OnlyManaValueParity {
            even: true,
            lands_are_exempt: false,
        }),
    );
    let odd_card = definition(
        "Odd Deck Card",
        vec![ManaSymbol::Generic(3)],
        vec![CardType::Creature],
        None,
    );
    let even_card = definition(
        "Even Deck Card",
        vec![ManaSymbol::Generic(2)],
        vec![CardType::Creature],
        None,
    );
    let first = game.create_object_from_definition(&companion, alice, Zone::OutsideGame);
    let second = game.create_object_from_definition(&companion, alice, Zone::OutsideGame);
    let odd = game.create_object_from_definition(&odd_card, alice, Zone::Library);
    let even = game.create_object_from_definition(&even_card, alice, Zone::Library);

    assert!(game.designate_companion(alice, first, &[odd], 1).is_err());
    assert_eq!(game.player(alice).unwrap().companion, None);
    assert_eq!(game.object(first).unwrap().zone, Zone::OutsideGame);

    game.designate_companion(alice, first, &[even], 1)
        .expect("valid designation");
    assert!(game.designate_companion(alice, second, &[even], 1).is_err());
    assert_eq!(game.player(alice).unwrap().companion, Some(first));
    assert_eq!(game.object(second).unwrap().zone, Zone::OutsideGame);
}

#[test]
fn companion_special_action_obeys_every_timing_boundary() {
    let (game, alice, bob, companion_id) = setup_even_companion_game();
    let action = SpecialAction::Companion {
        card_id: companion_id,
    };

    let mut wrong_phase = game.clone();
    wrong_phase.turn.phase = Phase::Combat;
    assert!(matches!(
        can_perform_check(&action, &wrong_phase, alice),
        Err(ActionError::WrongPhase { .. })
    ));

    let mut wrong_turn = game.clone();
    wrong_turn.turn.active_player = bob;
    wrong_turn.turn.priority_player = Some(alice);
    assert_eq!(
        can_perform_check(&action, &wrong_turn, alice),
        Err(ActionError::NotActivePlayer)
    );

    let mut wrong_priority = game.clone();
    wrong_priority.turn.priority_player = Some(bob);
    assert_eq!(
        can_perform_check(&action, &wrong_priority, alice),
        Err(ActionError::NotYourPriority)
    );

    let mut nonempty_stack = game.clone();
    let spell_card = CardBuilder::new(CardId::new(), "Stack Probe")
        .card_types(vec![CardType::Instant])
        .build();
    let spell = nonempty_stack.create_object_from_card(&spell_card, alice, Zone::Stack);
    nonempty_stack.push_to_stack(StackEntry::new(spell, alice));
    assert_eq!(
        can_perform_check(&action, &nonempty_stack, alice),
        Err(ActionError::StackNotEmpty)
    );
}

#[test]
fn payment_is_atomic_moves_directly_to_hand_and_is_once_per_game() {
    let (mut game, alice, _, companion_id) = setup_even_companion_game();
    let action = SpecialAction::Companion {
        card_id: companion_id,
    };

    let before = game.clone();
    let mut dm = SelectFirstDecisionMaker;
    assert_eq!(
        perform(action.clone(), &mut game, alice, &mut dm),
        Err(ActionError::CantPayCost)
    );
    assert_eq!(
        game.player(alice).unwrap().mana_pool,
        before.player(alice).unwrap().mana_pool
    );
    assert_eq!(game.player(alice).unwrap().companion, Some(companion_id));
    assert!(!game.player(alice).unwrap().companion_special_action_used);
    assert!(game.stack_is_empty());

    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 3);
    assert!(
        compute_legal_actions(&game, alice).contains(&LegalAction::SpecialAction(action.clone()))
    );
    perform(action, &mut game, alice, &mut dm).expect("pay {3} for companion");

    let player = game.player(alice).unwrap();
    assert_eq!(player.mana_pool.total(), 0);
    assert_eq!(player.hand.len(), 1);
    assert!(player.companion_special_action_used);
    assert!(
        game.stack_is_empty(),
        "the special action does not use the stack"
    );
    assert_eq!(game.turn.priority_player, Some(alice));
    assert!(!compute_legal_actions(&game, alice).iter().any(|action| {
        matches!(
            action,
            LegalAction::SpecialAction(SpecialAction::Companion { .. })
        )
    }));
}

#[test]
fn commander_is_part_of_the_starting_deck_for_companion_validation() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 40);
    let alice = PlayerId::from_index(0);
    let companion = definition(
        "Small Permanent Companion",
        vec![ManaSymbol::Generic(3)],
        vec![CardType::Creature],
        Some(CompanionDeckCondition::PermanentManaValueAtMost(2)),
    );
    let land = definition("Legal Land", Vec::new(), vec![CardType::Land], None);
    let commander = definition(
        "Too Large Commander",
        vec![ManaSymbol::Generic(3)],
        vec![CardType::Creature],
        None,
    );
    let companion_id = game.create_object_from_definition(&companion, alice, Zone::OutsideGame);
    let land_id = game.create_object_from_definition(&land, alice, Zone::Library);
    let commander_id = game.create_object_from_definition(&commander, alice, Zone::Library);

    assert!(
        game.designate_companion(alice, companion_id, &[land_id, commander_id], 2)
            .is_err(),
        "CR 702.139b validates before the commander leaves the starting deck"
    );
    assert_eq!(game.player(alice).unwrap().companion, None);
}
