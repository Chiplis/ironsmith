//! Exact-source Foreshadow, direct and artifact; parent owns execution.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
fn definitions() -> [CardDefinition; 2] {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-foreshadow-next-wave.json")).unwrap();
    let row = &fixture["mechanics"][0];
    let card = &row["representative_cards"][0];
    assert!(card["executable_text"].is_string(), "{card}");
    source_support::definitions(card).unwrap()
}
#[test]
fn foreshadow_selected_price_projects_only_own_death_triggers_without_a_death() {
    use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    for definition in definitions() {
        assert!(definition.canonical_text.contains("as though it died"), "compiled presentation retains the selected-price rider");
        for mode in [
            "normal",
            "selected",
            "selected-leaves",
            "selected-from-exile",
        ] {
            let selected = mode != "normal";
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(
                &definition,
                A,
                if mode == "selected-from-exile" {
                    Zone::Exile
                } else {
                    Zone::Hand
                },
            );
            let stable = game.object(source).unwrap().stable_id;
            if mode == "selected-from-exile" {
                let turn = game.turn.turn_number;
                game.effect_store.grant_registry.grant_to_card(
                    source,
                    Zone::Exile,
                    A,
                    ironsmith::grant::Grantable::PlayFrom,
                    ironsmith::grant_registry::GrantSource::until_end_of_turn(source, turn),
                );
            }
            let observer=ironsmith_compiler_runtime::compile_to_runtime_definition("Death observer","Type: Creature — Human\nPower/Toughness: 2/2\nWhenever a creature dies, you gain 7 life.",false).unwrap();
            game.create_object_from_definition(&observer, A, Zone::Battlefield);
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
                .add(ManaSymbol::Black, 1);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, if selected { 4 } else { 2 });
            let chosen=compute_legal_actions(&game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==source && ((casting_method.is_alternative()||matches!(casting_method.origin_method(),ironsmith::alternative_cast::CastingMethod::PlayFrom{use_alternative:Some(_),..}))==selected))).unwrap();
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
            let permanent = game.find_object_by_stable_id(stable).unwrap();
            if mode == "selected-leaves" {
                let pending = game.take_pending_trigger_events();
                let exile = game.move_object_by_effect(permanent, Zone::Exile).unwrap();
                game.take_pending_trigger_events();
                for pending_event in pending {
                    for trigger in ironsmith::triggers::check_triggers(&game, &pending_event) {
                        q.add(trigger);
                    }
                }
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut q, &mut dm)
                    .unwrap();
                while !game.stack.is_empty() {
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                }
                assert_eq!(
                    game.player(A).unwrap().life,
                    19,
                    "entry-time own trigger survives source departure"
                );
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
                assert_eq!(game.object(exile).unwrap().zone, Zone::Exile);
                continue;
            }
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut q, &mut dm)
                .unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut q, &mut dm)
                .unwrap();
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            }
            assert_eq!(
                game.player(A).unwrap().life,
                if selected { 19 } else { 20 },
                "only entrant's own dies ability triggers"
            );
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(selected));
            assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
            assert!(
                game.player(A).unwrap().graveyard.is_empty(),
                "projection creates no real zone move"
            );
            // A later real death independently triggers both the ordinary observer and source.
            game.move_object_by_effect(permanent, Zone::Graveyard)
                .unwrap();
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut q, &mut dm)
                .unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut q, &mut dm)
                .unwrap();
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            }
            assert_eq!(game.player(A).unwrap().life, if selected { 25 } else { 26 });
        }
    }
}
