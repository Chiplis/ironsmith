use std::collections::HashMap;

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, LegalAction};
use ironsmith::special_actions::{SpecialAction, perform};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{
    CardBuilder, CardDefinition, CardId, CardType, CastingMethod, ConspiracyDraftState,
    ConspiracySetupCard, DraftSelection, GameState, PlayerId, Supertype, Zone,
};

struct TestDecisionMaker;
impl DecisionMaker for TestDecisionMaker {}

fn card(name: &str) -> CardDefinition {
    CardDefinition::new(CardBuilder::new(CardId::new(), name).build())
}

fn conspiracy(name: &str, rules: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Conspiracy])
        .parse_text(rules)
        .unwrap_or_else(|error| panic!("failed to compile {name}: {error}"))
}

fn packs(prefix: &str, cards_per_pack: usize) -> Vec<Vec<CardDefinition>> {
    (1..=3)
        .map(|round| {
            (0..cards_per_pack)
                .map(|index| card(&format!("{prefix} R{round} C{index}")))
                .collect()
        })
        .collect()
}

fn first_pick(draft: &ConspiracyDraftState, player: PlayerId) -> DraftSelection {
    DraftSelection {
        player,
        card_ids: vec![draft.current_pack_view(player, player)[0].id],
        exchange_face_up: None,
        public_notes: HashMap::new(),
    }
}

#[test]
fn u064_conspiracy_type_and_agenda_keywords_compile_structurally() {
    let hidden = conspiracy("Hidden Test", "Hidden agenda");
    assert_eq!(hidden.card.card_types, vec![CardType::Conspiracy]);
    assert!(hidden.card.subtypes.is_empty());
    assert!(hidden.abilities.iter().any(|ability| {
        matches!(&ability.kind, ironsmith::AbilityKind::Static(ability) if ability.id() == StaticAbilityId::HiddenAgenda)
    }));

    let double = conspiracy("Double Test", "Double agenda");
    assert!(double.abilities.iter().any(|ability| {
        matches!(&ability.kind, ironsmith::AbilityKind::Static(ability) if ability.id() == StaticAbilityId::DoubleAgenda)
    }));
}

#[test]
fn u064_three_round_draft_passes_left_right_left_and_finalizes_card_pools() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    let players = vec![alice, bob, cara];
    let mut draft = ConspiracyDraftState::new(
        players.clone(),
        vec![
            (alice, packs("Alice", 2)),
            (bob, packs("Bob", 2)),
            (cara, packs("Cara", 2)),
        ],
    )
    .unwrap();

    draft
        .draft_step(
            players
                .iter()
                .map(|player| first_pick(&draft, *player))
                .collect(),
        )
        .unwrap();
    assert_eq!(
        draft.current_pack_view(bob, bob)[0].name.as_deref(),
        Some("Alice R1 C1")
    );
    assert_eq!(
        draft.current_pack_view(cara, cara)[0].name.as_deref(),
        Some("Bob R1 C1")
    );
    assert_eq!(
        draft.current_pack_view(alice, alice)[0].name.as_deref(),
        Some("Cara R1 C1")
    );
    draft
        .draft_step(
            players
                .iter()
                .map(|player| first_pick(&draft, *player))
                .collect(),
        )
        .unwrap();
    assert_eq!(draft.round(), 2);

    draft
        .draft_step(
            players
                .iter()
                .map(|player| first_pick(&draft, *player))
                .collect(),
        )
        .unwrap();
    assert_eq!(
        draft.current_pack_view(cara, cara)[0].name.as_deref(),
        Some("Alice R2 C1")
    );
    assert_eq!(
        draft.current_pack_view(alice, alice)[0].name.as_deref(),
        Some("Bob R2 C1")
    );
    assert_eq!(
        draft.current_pack_view(bob, bob)[0].name.as_deref(),
        Some("Cara R2 C1")
    );
    draft
        .draft_step(
            players
                .iter()
                .map(|player| first_pick(&draft, *player))
                .collect(),
        )
        .unwrap();

    for _ in 0..2 {
        draft
            .draft_step(
                players
                    .iter()
                    .map(|player| first_pick(&draft, *player))
                    .collect(),
            )
            .unwrap();
    }
    assert!(draft.is_complete());
    assert_eq!(draft.card_pool(alice).unwrap().len(), 6);
    assert_eq!(draft.card_pool(bob).unwrap().len(), 6);
    assert_eq!(draft.card_pool(cara).unwrap().len(), 6);
}

#[test]
fn u064_completed_pool_limits_nonbasic_deck_cards_but_allows_unlimited_basics() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let players = vec![alice, bob];
    let mut draft = ConspiracyDraftState::new(
        players.clone(),
        vec![(alice, packs("Alice", 14)), (bob, packs("Bob", 14))],
    )
    .unwrap();
    while !draft.is_complete() {
        draft
            .draft_step(
                players
                    .iter()
                    .map(|player| first_pick(&draft, *player))
                    .collect(),
            )
            .unwrap();
    }

    let pool = draft.card_pool(alice).unwrap();
    let basic = CardDefinition::new(
        CardBuilder::new(CardId::new(), "Wastes")
            .supertypes(vec![Supertype::Basic])
            .card_types(vec![CardType::Land])
            .build(),
    );
    let mut legal = pool.iter().take(39).cloned().collect::<Vec<_>>();
    legal.extend(std::iter::repeat_n(basic, 20));
    draft.validate_deck(alice, &legal).unwrap();

    let mut foreign = legal;
    foreign[0] = card("Undrafted Card");
    assert!(
        draft
            .validate_deck(alice, &foreign)
            .unwrap_err()
            .contains("not in")
    );

    let mut with_conspiracy = pool.iter().take(40).cloned().collect::<Vec<_>>();
    with_conspiracy[0] = conspiracy("Illegal Deck Conspiracy", "Hidden agenda");
    assert!(
        draft
            .validate_deck(alice, &with_conspiracy)
            .unwrap_err()
            .contains("cannot be included")
    );
}

#[test]
fn u064_real_draft_rules_enforce_private_picks_public_notes_and_exchange_pick() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cogwork = CardDefinitionBuilder::new(CardId::new(), "Cogwork Librarian")
        .parse_text(
            "Draft this card face up.\nAs you draft a card, you may draft an additional card from that booster pack. If you do, put this card into that booster pack.",
        )
        .expect("real Cogwork Librarian draft text");
    let pyretic = CardDefinitionBuilder::new(CardId::new(), "Pyretic Hunter")
        .parse_text(
            "Reveal this card as you draft it and note how many cards you've drafted this draft round, including this card.",
        )
        .expect("real Pyretic Hunter draft text");
    let mut alice_packs = packs("Alice", 3);
    alice_packs[0][0] = cogwork;
    let mut bob_packs = packs("Bob", 3);
    bob_packs[0][0] = pyretic;
    let mut draft = ConspiracyDraftState::new(
        vec![alice, bob],
        vec![(alice, alice_packs), (bob, bob_packs)],
    )
    .unwrap();

    let cogwork_id = draft.current_pack_view(alice, alice)[0].id;
    let bob_other_id = draft.current_pack_view(bob, bob)[1].id;
    draft
        .draft_step(vec![
            DraftSelection {
                player: alice,
                card_ids: vec![cogwork_id],
                ..first_pick(&draft, alice)
            },
            DraftSelection {
                player: bob,
                card_ids: vec![bob_other_id],
                ..first_pick(&draft, bob)
            },
        ])
        .unwrap();
    assert_eq!(
        draft.drafted_view(bob, alice)[0].name.as_deref(),
        Some("Cogwork Librarian"),
        "face-up picks are public"
    );

    let alice_pack = draft.current_pack_view(alice, alice);
    let pyretic_id = alice_pack
        .iter()
        .find(|card| card.name.as_deref() == Some("Pyretic Hunter"))
        .unwrap()
        .id;
    let other_id = alice_pack
        .iter()
        .find(|card| card.id != pyretic_id)
        .unwrap()
        .id;
    draft
        .draft_step(vec![
            DraftSelection {
                player: alice,
                card_ids: vec![pyretic_id, other_id],
                exchange_face_up: Some(cogwork_id),
                public_notes: HashMap::from([(pyretic_id, "2".to_string())]),
            },
            first_pick(&draft, bob),
        ])
        .unwrap();
    let opponent_view = draft.drafted_view(bob, alice);
    let pyretic_view = opponent_view
        .iter()
        .find(|card| card.id == pyretic_id)
        .unwrap();
    assert_eq!(
        pyretic_view.name, None,
        "revealed-and-noted cards turn face down"
    );
    assert_eq!(pyretic_view.public_note.as_deref(), Some("2"));
    assert!(
        draft
            .drafted_view(bob, alice)
            .iter()
            .any(|card| card.name.is_none())
    );
}

#[test]
fn u064_hidden_agenda_is_secret_owner_controlled_and_revealed_by_priority_action() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let hidden = conspiracy("Secret Summoning", "Hidden agenda");
    let double = conspiracy("Two Secrets", "Double agenda");
    let public = conspiracy("Public Conspiracy", "Creatures you control get +1/+1.");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 99);
    game.enable_conspiracy(vec![(
        alice,
        vec![
            ConspiracySetupCard {
                definition: hidden,
                agenda_names: vec!["Grizzly Bears".into()],
            },
            ConspiracySetupCard {
                definition: double,
                agenda_names: vec!["Runeclaw Bear".into(), "Balduvian Bears".into()],
            },
            ConspiracySetupCard {
                definition: public,
                agenda_names: Vec::new(),
            },
        ],
    )])
    .unwrap();
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(bob).unwrap().life, 20);

    let hidden_id = game
        .conspiracy_cards()
        .into_iter()
        .find(|id| {
            game.object(*id)
                .is_some_and(|card| card.name == "Secret Summoning")
        })
        .unwrap();
    assert!(game.is_face_down_conspiracy(hidden_id));
    assert_eq!(game.current_name(hidden_id).as_deref(), Some(""));
    assert!(game.current_card_types(hidden_id).unwrap().is_empty());
    assert_eq!(
        game.agenda_names_for(alice, hidden_id).unwrap(),
        ["Grizzly Bears"]
    );
    assert!(game.agenda_names_for(bob, hidden_id).is_none());
    assert!(
        game.object(hidden_id)
            .unwrap()
            .abilities
            .iter()
            .all(|ability| ability.functional_zones.is_empty())
    );

    game.turn.priority_player = Some(alice);
    let action = SpecialAction::TurnConspiracyFaceUp {
        conspiracy_id: hidden_id,
    };
    assert!(ironsmith::special_actions::can_perform_check(&action, &game, alice).is_ok());
    assert!(ironsmith::decision::compute_legal_actions(&game, alice).iter().any(
        |candidate| matches!(candidate, LegalAction::SpecialAction(found) if *found == action)
    ));
    perform(action, &mut game, alice, &mut TestDecisionMaker).unwrap();
    assert!(!game.is_face_down_conspiracy(hidden_id));
    assert_eq!(
        game.current_name(hidden_id).as_deref(),
        Some("Secret Summoning")
    );
    assert_eq!(
        game.agenda_names_for(bob, hidden_id).unwrap(),
        ["Grizzly Bears"]
    );
    assert!(
        game.object(hidden_id)
            .unwrap()
            .abilities
            .iter()
            .all(|ability| ability.functional_zones == [Zone::Command])
    );
    assert_eq!(
        game.move_object_by_effect(hidden_id, Zone::Graveyard),
        Some(hidden_id)
    );
    game.set_current_controller(hidden_id, bob);
    assert_eq!(game.controller_of_id(hidden_id), Some(alice));
    assert!(!ironsmith::decision::can_cast_spell(
        &game,
        alice,
        game.object(hidden_id).unwrap(),
        &CastingMethod::Normal,
    ));
}

#[test]
fn u064_restart_returns_agendas_face_down_and_departure_removes_owned_conspiracies() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let hidden = conspiracy("Restart Secret", "Hidden agenda");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.enable_conspiracy(vec![(
        alice,
        vec![ConspiracySetupCard {
            definition: hidden,
            agenda_names: vec!["Serra Angel".into()],
        }],
    )])
    .unwrap();
    let original = game.conspiracy_cards()[0];
    game.turn.priority_player = Some(alice);
    game.turn_conspiracy_face_up(alice, original).unwrap();
    game.restart_game(bob, &[]);
    let restarted = game.conspiracy_cards()[0];
    assert_ne!(restarted, original);
    assert!(game.is_face_down_conspiracy(restarted));
    assert_eq!(
        game.agenda_names_for(alice, restarted).unwrap(),
        ["Serra Angel"]
    );
    assert!(game.leave_game(alice));
    assert!(game.conspiracy_cards().is_empty());
    assert!(game.object(restarted).is_none());
}
