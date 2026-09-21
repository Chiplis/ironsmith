use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};
fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Grab the Prize",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
fn probe(types: Vec<CardType>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Card probe")
        .card_types(types)
        .build()
}
fn cast(game: &mut GameState, player: PlayerId, spell: ObjectId) {
    let action = compute_legal_actions(game, player).into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).expect("spell cast must be legal");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, &mut dm,
        )
        .unwrap();
    }
    assert_eq!(game.stack.len(), 1);
}
#[test]
fn strict_snapshot_and_full_quality_gate() {
    let s = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        s.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{:?}",
        s.parse_error
    );
    assert!(!s.parse_lossy && !s.has_unimplemented && s.parse_error.is_none());
    assert!(
        s.similarity_score >= 0.99,
        "{}: {:?}",
        s.similarity_score,
        s.compiled_text
    );
}
#[test]
fn paid_discard_controls_damage_after_draw_and_after_discarded_card_moves() {
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for discarded_types in [
        vec![CardType::Land],
        vec![CardType::Sorcery],
        vec![CardType::Artifact, CardType::Land],
    ] {
        for drawn_type in [CardType::Land, CardType::Creature] {
            for move_discarded in [false, true] {
                let mut game =
                    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                game.turn.active_player = alice;
                game.turn.priority_player = Some(alice);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                let spell = game.create_object_from_definition(&def, alice, Zone::Hand);
                let discarded = game.create_object_from_definition(
                    &probe(discarded_types.clone()),
                    alice,
                    Zone::Hand,
                );
                let identity = game.object(discarded).unwrap().stable_id;
                for _ in 0..2 {
                    game.create_object_from_definition(
                        &probe(vec![drawn_type]),
                        alice,
                        Zone::Library,
                    );
                }
                game.player_mut(alice)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::mana::ManaSymbol::Red, 1);
                game.player_mut(alice)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::mana::ManaSymbol::Colorless, 1);
                cast(&mut game, alice, spell);
                assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
                assert!(
                    game.player(alice).unwrap().hand.is_empty(),
                    "discard is paid before drawing"
                );
                assert_eq!(game.player(alice).unwrap().library.len(), 2);
                let discarded = game.find_object_by_stable_id(identity).unwrap();
                assert_eq!(game.object(discarded).unwrap().zone, Zone::Graveyard);
                if move_discarded {
                    game.move_object_by_effect(discarded, Zone::Exile);
                }
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert!(game.stack.is_empty());
                assert_eq!(game.player(alice).unwrap().hand.len(), 2);
                assert_eq!(game.player(alice).unwrap().life, 20);
                let expected = if discarded_types.contains(&CardType::Land) {
                    20
                } else {
                    18
                };
                for opponent in [bob, carol] {
                    assert_eq!(
                        game.player(opponent).unwrap().life,
                        expected,
                        "discard={discarded_types:?}, drawn={drawn_type:?}, moved={move_discarded}"
                    );
                }
            }
        }
    }
}

#[test]
fn casting_requires_another_card_and_sorcery_timing() {
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for other_card in [false, true] {
        for active in [alice, bob] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = active;
            game.turn.priority_player = Some(alice);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            let spell = game.create_object_from_definition(&def, alice, Zone::Hand);
            if other_card {
                game.create_object_from_definition(&probe(vec![CardType::Land]), alice, Zone::Hand);
            }
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Red, 1);
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Colorless, 1);
            let offered = compute_legal_actions(&game, alice).iter().any(
                |a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell),
            );
            assert_eq!(offered, other_card && active == alice);
        }
    }
}

#[test]
fn countering_does_not_refund_discard_or_apply_resolution_effects() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let spell = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let discarded =
        game.create_object_from_definition(&probe(vec![CardType::Sorcery]), alice, Zone::Hand);
    let identity = game.object(discarded).unwrap().stable_id;
    for _ in 0..2 {
        game.create_object_from_definition(&probe(vec![CardType::Land]), alice, Zone::Library);
    }
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Red, 1);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Colorless, 1);
    cast(&mut game, alice, spell);
    let stack_spell = game.stack.last().unwrap().object_id;
    let mut context = ironsmith::effects::EffectContext::new_default(stack_spell, bob);
    ironsmith::effects::execute_effect(
        &mut game,
        &ironsmith::effect::Effect::counter(ironsmith::target::ChooseSpec::SpecificObject(
            stack_spell,
        )),
        &mut context,
    )
    .unwrap();
    assert!(game.stack.is_empty());
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert_eq!(game.player(alice).unwrap().library.len(), 2);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    assert_eq!(game.player(bob).unwrap().life, 20);
    let discarded = game.find_object_by_stable_id(identity).unwrap();
    assert_eq!(game.object(discarded).unwrap().zone, Zone::Graveyard);
}
