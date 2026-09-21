use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardType, GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Transmute Artifact",
    )
    .unwrap()
    .remove(0)
}

fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}

fn artifact(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .build()
}

fn resolve(game: &mut GameState, alice: PlayerId) {
    let spell = game.create_object_from_definition(&definition(), alice, Zone::Stack);
    game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice));
    ironsmith::game_loop::resolve_stack_entry_with(
        game,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
}

#[test]
fn strict_snapshot_and_structural_behavior_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented);
    assert!(
        snapshot.similarity_score >= 0.99,
        "similarity {}: {}",
        snapshot.similarity_score,
        snapshot.compiled_text.as_deref().unwrap_or("<missing>")
    );
}

#[test]
fn failing_to_sacrifice_does_not_search_or_move_the_library_card() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let found = game.create_object_from_definition(
        &artifact("Unsearched artifact", 2),
        alice,
        Zone::Library,
    );
    resolve(&mut game, alice);
    assert_eq!(game.object(found).unwrap().zone, Zone::Library);
    assert!(game.battlefield.is_empty());
}

#[test]
fn lower_or_equal_mana_value_enters_without_payment() {
    let alice = PlayerId::from_index(0);
    for found_value in [2, 3] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let sacrificed = game.create_object_from_definition(
            &artifact("Sacrificed artifact", 3),
            alice,
            Zone::Battlefield,
        );
        let sacrifice_identity = game.object(sacrificed).unwrap().stable_id;
        let found = game.create_object_from_definition(
            &artifact("Searched artifact", found_value),
            alice,
            Zone::Library,
        );
        let found_identity = game.object(found).unwrap().stable_id;
        resolve(&mut game, alice);
        let sacrifice_now = game.find_object_by_stable_id(sacrifice_identity).unwrap();
        let found_now = game.find_object_by_stable_id(found_identity).unwrap();
        assert_eq!(game.object(sacrifice_now).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(found_now).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn higher_mana_value_requires_exact_difference_or_goes_to_graveyard() {
    let alice = PlayerId::from_index(0);
    for available in [2, 3] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(
            &artifact("Sacrificed artifact", 3),
            alice,
            Zone::Battlefield,
        );
        let found = game.create_object_from_definition(
            &artifact("More expensive artifact", 6),
            alice,
            Zone::Library,
        );
        let identity = game.object(found).unwrap().stable_id;
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, available);
        resolve(&mut game, alice);
        let current = game.find_object_by_stable_id(identity).unwrap();
        assert_eq!(
            game.object(current).unwrap().zone,
            if available == 3 {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
        assert_eq!(
            game.player(alice).unwrap().mana_pool.total(),
            if available == 3 { 0 } else { 2 }
        );
    }
}

#[test]
fn payment_can_be_declined_even_with_enough_mana() {
    struct DeclinePayment;
    impl ironsmith::decision::DecisionMaker for DeclinePayment {
        fn decide_objects(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::SelectObjectsContext,
        ) -> Vec<ironsmith::ObjectId> {
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .take(1)
                .map(|c| c.id)
                .collect()
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&artifact("Sacrifice", 2), alice, Zone::Battlefield);
    let found = game.create_object_from_definition(&artifact("Find", 5), alice, Zone::Library);
    let identity = game.object(found).unwrap().stable_id;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);
    let spell = game.create_object_from_definition(&definition(), alice, Zone::Stack);
    game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice));
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut DeclinePayment).unwrap();
    let current = game.find_object_by_stable_id(identity).unwrap();
    assert_eq!(game.object(current).unwrap().zone, Zone::Graveyard);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 3);
}

#[test]
fn search_with_no_matching_card_finishes_without_payment() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let sacrifice =
        game.create_object_from_definition(&artifact("Sacrifice", 3), alice, Zone::Battlefield);
    let identity = game.object(sacrifice).unwrap().stable_id;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 4);
    resolve(&mut game, alice);
    let current = game.find_object_by_stable_id(identity).unwrap();
    assert_eq!(game.object(current).unwrap().zone, Zone::Graveyard);
    assert!(game.battlefield.is_empty());
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 4);
}

#[test]
fn library_search_may_fail_to_find_an_available_artifact() {
    struct FailToFind;
    impl ironsmith::decision::DecisionMaker for FailToFind {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &ironsmith::decisions::context::SelectObjectsContext,
        ) -> Vec<ironsmith::ObjectId> {
            ctx.candidates
                .iter()
                .filter(|c| {
                    c.legal
                        && game
                            .object(c.id)
                            .is_some_and(|o| o.zone == Zone::Battlefield)
                })
                .take(1)
                .map(|c| c.id)
                .collect()
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&artifact("Sacrifice", 3), alice, Zone::Battlefield);
    let available =
        game.create_object_from_definition(&artifact("Available", 2), alice, Zone::Library);
    let spell = game.create_object_from_definition(&definition(), alice, Zone::Stack);
    game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice));
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut FailToFind).unwrap();
    assert_eq!(game.object(available).unwrap().zone, Zone::Library);
    assert!(game.battlefield.is_empty());
}
