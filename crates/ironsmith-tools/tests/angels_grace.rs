//! Angel's Grace: "Split second. You can't lose the game this turn and your
//! opponents can't win the game this turn. Until end of turn, damage that
//! would reduce your life total to less than 1 reduces it to 1 instead."
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::target::ChooseSpec;
use ironsmith::{GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Angel's Grace",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

/// Alice (at `life`) has resolved Angel's Grace.
fn resolved(life: i32) -> GameState {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], life);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    let definition = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    game.refresh_continuous_state();
    game
}

fn damage(game: &mut GameState, player: PlayerId, amount: i32) {
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(
        &ironsmith::cards::builders::CardDefinitionBuilder::new(
            ironsmith::ids::CardId::new(),
            "Shock Source",
        )
        .card_types(vec![ironsmith::CardType::Instant])
        .build(),
        bob,
        Zone::Stack,
    );
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut dm);
    ironsmith::effects::execute_effect(
        game,
        &ironsmith::Effect::deal_damage(
            amount,
            ChooseSpec::Player(ironsmith::PlayerFilter::Specific(player)),
        ),
        &mut ctx,
    )
    .unwrap();
}

#[test]
fn you_cannot_lose_and_opponents_cannot_win_this_turn() {
    let game = resolved(20);
    assert!(!game.can_lose_game(PlayerId::from_index(0)));
    assert!(!game.can_win_game(PlayerId::from_index(1)));
    assert!(
        game.can_lose_game(PlayerId::from_index(1)),
        "only you are protected"
    );
    assert!(game.can_win_game(PlayerId::from_index(0)));
}

#[test]
fn lethal_damage_leaves_you_at_one_life() {
    let alice = PlayerId::from_index(0);
    let mut game = resolved(5);
    damage(&mut game, alice, 10);
    assert_eq!(game.player(alice).unwrap().life, 1);
    damage(&mut game, alice, 3);
    assert_eq!(game.player(alice).unwrap().life, 1);
    // Opponents are unaffected.
    let bob = PlayerId::from_index(1);
    damage(&mut game, bob, 3);
    assert_eq!(game.player(bob).unwrap().life, 2);
}

#[test]
fn the_effects_end_at_end_of_turn() {
    let alice = PlayerId::from_index(0);
    let mut game = resolved(5);
    game.cleanup_restrictions_end_of_turn();
    game.refresh_continuous_state();
    game.update_cant_effects();
    assert!(game.can_lose_game(alice));
    damage(&mut game, alice, 10);
    assert_eq!(game.player(alice).unwrap().life, -5);
}
