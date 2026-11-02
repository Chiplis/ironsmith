//! Exact-source Crumble, direct and artifact; parent owns execution.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
fn definitions() -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-final-abilities.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "msem-915")
        .unwrap();
    let card = &row["representative_cards"][0];
    assert!(card["executable_text"].is_string(), "{card}");
    source_support::definitions(card).unwrap()
}
fn settle(game: &mut GameState) {
    let mut q = ironsmith::triggers::TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    for _ in 0..64 {
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut q, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut q, &mut dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    panic!("trigger processing did not settle");
}
#[test]
fn crumble_selected_price_adds_entry_counters_then_upkeeps_copy_and_last_counter_sacrifices_and_draws()
 {
    use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    use ironsmith::object::CounterType;
    for definition in definitions() {
        for selected in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = game.object(source).unwrap().stable_id;
            let land = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Library filler",
                "Type: Basic Land — Island",
                false,
            )
            .unwrap();
            for _ in 0..4 {
                game.create_object_from_definition(&land, A, Zone::Library);
            }
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Blue, 1);
            if selected {
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 2);
            }
            let chosen=compute_legal_actions(&game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==source && (matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Normal)!=selected))).unwrap();
            use ironsmith::game_loop::{
                PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
                apply_priority_response_with_dm,
            };
            let mut q = ironsmith::triggers::TriggerQueue::new();
            let mut state = PriorityLoopState::new(2);
            let mut dm = SelectFirstDecisionMaker;
            let mut progress = apply_priority_response_with_dm(
                &mut game,
                &mut q,
                &mut state,
                &PriorityResponse::PriorityAction(chosen),
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
                    apply_decision_context_with_dm(&mut game, &mut q, &mut state, &ctx, &mut dm)
                        .unwrap();
            }
            assert!(!state.has_pending_action());
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            settle(&mut game);
            let permanent = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.counter_count(permanent, CounterType::Time),
                if selected { 3 } else { 0 }
            );
            // Opponent upkeep cannot remove our counters.
            game.queue_trigger_event(
                Default::default(),
                ironsmith::triggers::TriggerEvent::new_with_provenance(
                    ironsmith::events::BeginningOfUpkeepEvent::new(PlayerId(1)),
                    Default::default(),
                ),
            );
            settle(&mut game);
            assert_eq!(
                game.counter_count(permanent, CounterType::Time),
                if selected { 3 } else { 0 }
            );
            for turn in 1..=3 {
                game.turn.turn_number += 1;
                game.queue_trigger_event(
                    Default::default(),
                    ironsmith::triggers::TriggerEvent::new_with_provenance(
                        ironsmith::events::BeginningOfUpkeepEvent::new(A),
                        Default::default(),
                    ),
                );
                settle(&mut game);
                let tokens = game
                    .battlefield
                    .iter()
                    .filter(|id| {
                        game.object(**id)
                            .is_some_and(|o| o.kind == ironsmith::object::ObjectKind::Token)
                    })
                    .count();
                assert_eq!(tokens, if selected { turn } else { 0 });
                for id in game.battlefield.iter() {
                    if game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token {
                        assert_eq!(
                            game.counter_count(*id, CounterType::Time),
                            0,
                            "copied permanent did not pay the selected price"
                        );
                    }
                }
            }
            let final_source = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(final_source).unwrap().zone,
                if selected {
                    Zone::Graveyard
                } else {
                    Zone::Battlefield
                }
            );
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(selected));
        }
    }
}
