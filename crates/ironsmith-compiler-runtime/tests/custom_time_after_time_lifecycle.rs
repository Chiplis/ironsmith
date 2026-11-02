//! Actual granted recipient spell cast, resolution exile, and delayed free recast.
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn grant_definitions() -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-leitmotif-next.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "msem-674")
        .unwrap();
    let card = &row["representative_cards"][0];
    let text = card["executable_text"].as_str().unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_to_artifact(
                card["name"].as_str().unwrap(),
                text,
                false,
            )
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn announce(game: &mut GameState, action: LegalAction) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
}

fn flush(game: &mut GameState) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(
        game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(
        game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
}
#[test]
fn actual_static_grant_appends_recipient_program_and_capture_survives_grantor_departure() {
    for grant in grant_definitions() {
        for granted in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let granter =
                granted.then(|| game.create_object_from_definition(&grant, A, Zone::Battlefield));
            let blue = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Recipient instant",
                "Mana cost: {U}\nType: Instant\nYou gain 1 life.",
                false,
            )
            .unwrap();
            let first = game.create_object_from_definition(&blue, A, Zone::Hand);
            let stable = game.object(first).unwrap().stable_id;
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Blue, 2);
            let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==first)).unwrap();
            announce(&mut game, action);
            assert_eq!(game.stack.len(), 1);
            ironsmith::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            assert_eq!(game.player(A).unwrap().life, 21);
            let finished = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(finished).unwrap().zone,
                if granted {
                    Zone::Exile
                } else {
                    Zone::Graveyard
                }
            );
            assert_eq!(
                game.effect_store.delayed_triggers.len(),
                usize::from(granted)
            );
            if !granted {
                continue;
            }
            // The already-scheduled receipt survives removal of the granting permanent.
            game.remove_object(granter.unwrap());
            game.turn.turn_number += 2;
            game.turn.priority_player = Some(A);
            let second = game.create_object_from_definition(&blue, A, Zone::Hand);
            let second_stable = game.object(second).unwrap().stable_id;
            let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==second)).unwrap();
            announce(&mut game, action);
            flush(&mut game);
            assert_eq!(
                game.stack.len(),
                2,
                "matching actual cast schedules one delayed trigger above the new spell"
            );
            ironsmith::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            assert_eq!(
                game.stack.len(),
                2,
                "freecast replaces resolving delayed trigger with the exact original spell"
            );
            let recast = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(recast).unwrap().zone, Zone::Stack);
            assert_eq!(
                game.player(A).unwrap().mana_pool.total(),
                0,
                "recast did not require mana"
            );
            ironsmith::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            assert_eq!(game.player(A).unwrap().life, 22);
            let finished = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(finished).unwrap().zone, Zone::Graveyard);
            assert!(
                game.effect_store.delayed_triggers.is_empty(),
                "single use capture does not repeat"
            );
            ironsmith::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            assert_eq!(game.player(A).unwrap().life, 23);
            assert_eq!(
                game.object(game.find_object_by_stable_id(second_stable).unwrap())
                    .unwrap()
                    .zone,
                Zone::Graveyard,
                "future spells no longer get program after grantor left"
            );
        }
    }
}
