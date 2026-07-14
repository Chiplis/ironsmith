use super::shard_00::{dispatch_matching_priority_action, start_pregame};
use super::*;

#[test]
fn opening_hand_reveal_action_is_public_one_use_and_schedules_its_consequence() {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
        1,
    );
    let alice = PlayerId::from_index(0);
    let definition = compile_to_runtime_definition(
        "Chancellor of the Tangle",
        "Type: Creature — Phyrexian Giant\nYou may reveal this card from your opening hand. If you do, at the beginning of your first main phase of the game, add {G}.",
        false,
    )
    .expect("opening-hand reveal card should compile");
    let chancellor = wasm
        .game
        .create_object_from_definition(&definition, alice, Zone::Hand);

    start_pregame(&mut wasm, 7, MatchFormatInput::Normal);
    for _ in 0..3 {
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::KeepOpeningHand)
        });
    }

    let reveal_action = match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => ctx
            .actions
            .iter()
            .find(|action| {
                matches!(
                    action,
                    LegalAction::UsePregameAction { card_id, .. } if *card_id == chancellor
                )
            })
            .cloned()
            .expect("typed reveal action should be offered"),
        other => panic!("expected Alice opening-actions prompt, got {other:?}"),
    };
    assert_eq!(
        crate::describe_action(&wasm.game, &reveal_action),
        "Reveal Chancellor of the Tangle"
    );

    dispatch_matching_priority_action(&mut wasm, |action| action == &reveal_action);

    assert!(
        wasm.game
            .player(alice)
            .expect("Alice")
            .hand
            .contains(&chancellor),
        "revealing must not move the opening-hand card"
    );
    assert_eq!(wasm.game.effect_store.delayed_triggers.len(), 1);
    let delayed = &wasm.game.effect_store.delayed_triggers[0];
    let main_phase = delayed
        .trigger
        .downcast_ref::<ironsmith::triggers::BeginningOfMainPhaseTrigger>()
        .expect("Tangle should schedule a main-phase trigger");
    assert_eq!(
        main_phase.phase_type,
        ironsmith::triggers::phase_step::MainPhaseType::Precombat
    );

    let active_view = wasm
        .active_viewed_cards
        .as_ref()
        .expect("reveal should produce a transient card view");
    assert!(active_view.public);
    assert_eq!(active_view.subject, alice);
    assert!(active_view.cards.contains(&chancellor));
    assert!(
        wasm.active_audit_viewed_cards.iter().any(|view| {
            view.public && view.subject == alice && view.cards.contains(&chancellor)
        })
    );

    match wasm.pending_decision.as_ref() {
        Some(DecisionContext::Priority(ctx)) => assert!(!ctx.actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::UsePregameAction { card_id, .. } if *card_id == chancellor
            )
        })),
        other => panic!("expected opening-actions prompt after reveal, got {other:?}"),
    }

    dispatch_matching_priority_action(&mut wasm, |action| {
        matches!(action, LegalAction::ContinuePregame)
    });
    assert!(
        wasm.active_viewed_cards.is_none(),
        "public opening-hand reveal should clear when the player advances"
    );
}
