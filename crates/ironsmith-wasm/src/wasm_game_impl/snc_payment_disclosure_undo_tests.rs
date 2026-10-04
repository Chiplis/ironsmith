// UNVALIDATED: included beside the ordinary live-action rollback fixtures.
#[test]
fn payment_disclosure_exact_snc_self_exile_costs_disable_completed_action_undo() {
    let _guard = crate::test_id_counter_guard();
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/snc_exiled_land_mana_grants.json.fixture"
    ))
    .unwrap();
    assert_eq!(cards.len(), 5);
    for card in cards {
        let name = card["name"].as_str().unwrap();
        let definition = ironsmith_registry_test::compile_to_runtime_definition(
            name,
            card["text"].as_str().unwrap(),
            false,
        )
        .unwrap();
        let (mut wasm, land) = manual_payment_fixture();
        let alice = PlayerId(0);
        let source = wasm
            .game
            .create_object_from_definition(&definition, alice, Zone::Hand);
        payment_disclosure_track_hand(&mut wasm, source, 0);
        let stable = wasm.game.object(source).unwrap().stable_id;
        let library = wasm.game.player(alice).unwrap().library.clone();
        wasm.game
            .player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        payment_disclosure_prepare_priority(&mut wasm);
        assert!(
            !wasm
                .public_hand_disclosure_identities()
                .contains(&(alice, source))
        );
        disclosure_priority_matching(
            &mut wasm,
            |action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source),
        );
        for _ in 0..20 {
            let command = match wasm.pending_decision.as_ref().unwrap() {
                DecisionContext::Priority(_) => break,
                DecisionContext::Targets(_) => UiCommand::SelectTargets {
                    targets: vec![TargetInput::Object { object: land.0 }],
                },
                DecisionContext::ManaPayment(_) => {
                    disclosure_confirm_mana(&mut wasm);
                    continue;
                }
                DecisionContext::SelectOptions(options) => UiCommand::SelectOptions {
                    option_indices: vec![
                        options
                            .options
                            .iter()
                            .find(|option| option.legal)
                            .unwrap()
                            .index,
                    ],
                },
                other => panic!("unexpected self-exile payment prompt for {name}: {other:?}"),
            };
            disclosure_command(&mut wasm, command).unwrap();
        }
        assert!(matches!(
            wasm.pending_decision,
            Some(DecisionContext::Priority(_))
        ));
        let exiled = wasm.game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(wasm.game.object(exiled).unwrap().zone, Zone::Exile);
        assert_eq!(wasm.game.stack.len(), 1, "the ability has not resolved");
        assert_eq!(wasm.game.player(alice).unwrap().library, library);
        assert!(
            wasm.public_hand_disclosure_identities()
                .contains(&(alice, source)),
            "the original hand identity must be observed through its cost zone-change event"
        );
        assert!(
            !wasm.is_cancelable(),
            "completed self-exile payment must not be returned to a private hand: {name}"
        );
        #[cfg(target_arch = "wasm32")]
        assert!(wasm.cancel_decision().is_err());
    }
}
