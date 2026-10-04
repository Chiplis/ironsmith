// UNVALIDATED: exact full-card group costs use the same typed payment boundary.
#[test]
fn payment_disclosure_group_costs_reject_invalid_groups_before_commit_then_pay_exact_valid_groups()
{
    let _guard = crate::test_id_counter_guard();
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/grouped_hand_costs.json.fixture"
    ))
    .unwrap();
    assert_eq!(cards.len(), 3);
    for card in cards {
        let name = card["name"].as_str().unwrap();
        let definition = ironsmith_registry_test::compile_to_runtime_definition(
            name,
            card["text"].as_str().unwrap(),
            false,
        )
        .unwrap();
        let (mut wasm, _) = manual_payment_fixture();
        let source =
            wasm.game
                .create_object_from_definition(&definition, PlayerId(1), Zone::Battlefield);
        wasm.game.set_current_controller(source, PlayerId(0));
        wasm.game.remove_summoning_sickness(source);
        let names = if name == "Sphinx of the Chimes" {
            ["Pair", "Pair", "Other", "Pair"]
        } else {
            ["Alpha", "Beta", "Gamma", "Alpha"]
        };
        let mut hand = Vec::new();
        for (index, card_name) in names.into_iter().enumerate() {
            let chosen = wasm.game.create_object_from_card(
                &ironsmith::card::CardBuilder::new(CardId::new(), card_name)
                    .card_types(vec![CardType::Artifact])
                    .color_indicator(if index == 2 {
                        ironsmith::color::ColorSet::BLUE
                    } else {
                        ironsmith::color::ColorSet::RED
                    })
                    .build(),
                PlayerId(0),
                Zone::Hand,
            );
            payment_disclosure_track_hand(&mut wasm, chosen, index as u16);
            hand.push(chosen);
        }
        let foreign = wasm.game.create_object_from_card(
            &ironsmith::card::CardBuilder::new(CardId::new(), "Pair")
                .card_types(vec![CardType::Artifact])
                .color_indicator(ironsmith::color::ColorSet::RED)
                .build(),
            PlayerId(1),
            Zone::Hand,
        );
        if name == "Illuminated Folio" {
            wasm.game
                .player_mut(PlayerId(0))
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 1);
        }
        if name == "Ormos, Archive Keeper" {
            wasm.game
                .player_mut(PlayerId(0))
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 1);
            wasm.game
                .player_mut(PlayerId(0))
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Blue, 2);
        }
        disclosure_activate(&mut wasm, source);
        for _ in 0..10 {
            match wasm.pending_decision.as_ref().unwrap() {
                DecisionContext::SelectObjects(_) => break,
                DecisionContext::ManaPayment(_) => disclosure_confirm_mana(&mut wasm),
                other => panic!("unexpected group-cost setup for {name}: {other:?}"),
            }
        }
        let Some(DecisionContext::SelectObjects(objects)) = wasm.pending_decision.as_ref() else {
            panic!("group choice");
        };
        assert_eq!(objects.reveal_policy, SelectionRevealPolicy::Public);
        assert!(objects.relation_filter.is_some());
        assert!(
            !objects
                .candidates
                .iter()
                .any(|candidate| candidate.id == foreign)
        );
        assert!(wasm.is_cancelable());
        let before_pool = wasm.game.player(PlayerId(0)).unwrap().mana_pool.clone();
        let before_pending = format!("{:?}", wasm.priority_state.pending_activation);
        let invalid = if name == "Ormos, Archive Keeper" {
            vec![hand[0], hand[1], hand[3]]
        } else {
            vec![hand[0], hand[2]]
        };
        let selected = |ids: Vec<ObjectId>| UiCommand::SelectObjects {
            object_ids: ids.into_iter().map(|id| id.0).collect(),
            object_stable_ids: Vec::new(),
            object_hidden_refs: Vec::new(),
        };
        assert!(
            disclosure_command(&mut wasm, selected(invalid)).is_err(),
            "{name}"
        );
        assert!(
            wasm.payment_disclosure.is_none(),
            "invalid groups cannot acquire an irreversible retry pin"
        );
        assert!(wasm.is_cancelable());
        assert_eq!(wasm.game.player(PlayerId(0)).unwrap().hand, hand);
        assert_eq!(
            wasm.game.player(PlayerId(0)).unwrap().mana_pool,
            before_pool
        );
        assert_eq!(
            format!("{:?}", wasm.priority_state.pending_activation),
            before_pending
        );
        let count = if name == "Ormos, Archive Keeper" {
            3
        } else {
            2
        };
        disclosure_command(&mut wasm, selected(hand[..count].to_vec())).unwrap();
        assert_eq!(
            wasm.game.stack.len(),
            1,
            "{name} completes actual cost payment"
        );
        assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool.total(), 0);
        assert_eq!(wasm.game.player(PlayerId(1)).unwrap().hand, vec![foreign]);
        assert_eq!(
            wasm.game.player(PlayerId(0)).unwrap().hand.len(),
            if name == "Illuminated Folio" {
                4
            } else {
                4 - count
            }
        );
        assert_eq!(wasm.game.is_tapped(source), name == "Illuminated Folio");
        assert!(wasm.payment_disclosure.is_none());
        assert!(
            !wasm.is_cancelable(),
            "{name} cannot undo a valid published group"
        );
    }
}
