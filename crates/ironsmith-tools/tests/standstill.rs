//! Standstill: "When a player casts a spell, sacrifice this enchantment. If
//! you do, each of that player's opponents draws three cards."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardType, GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Standstill",
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

/// Three players; Alice controls Standstill and each player has a ten-card
/// library. `caster` casts a sorcery. Returns hand sizes after the trigger
/// resolves and Standstill's zone.
fn cast_by(caster: PlayerId) -> (Vec<usize>, Zone) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = caster;
    game.turn.priority_player = Some(caster);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    for player in 0..3 {
        for i in 0..10 {
            game.create_object_from_definition(
                &CardDefinitionBuilder::new(CardId::new(), format!("Card {i}"))
                    .card_types(vec![CardType::Sorcery])
                    .build(),
                PlayerId::from_index(player),
                Zone::Library,
            );
        }
    }
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let standstill = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let stable = game.object(standstill).unwrap().stable_id;
    let spell = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .build(),
        caster,
        Zone::Hand,
    );
    game.player_mut(caster)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 1);
    let action = compute_legal_actions(&game, caster)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 2, "the spell and Standstill's trigger");
    // Resolve only the trigger (top of the stack).
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let hands = (0..3)
        .map(|p| game.player(PlayerId::from_index(p)).unwrap().hand.len())
        .collect();
    let zone = game
        .object(game.find_object_by_stable_id(stable).unwrap())
        .unwrap()
        .zone;
    (hands, zone)
}

#[test]
fn the_casters_opponents_each_draw_three() {
    let (hands, zone) = cast_by(PlayerId::from_index(1));
    assert_eq!(zone, Zone::Graveyard);
    assert_eq!(hands, vec![3, 0, 3], "Alice and Carol are Bob's opponents");
}

#[test]
fn its_controller_casting_gives_the_cards_to_everyone_else() {
    let (hands, _) = cast_by(PlayerId::from_index(0));
    assert_eq!(hands, vec![0, 3, 3]);
}
