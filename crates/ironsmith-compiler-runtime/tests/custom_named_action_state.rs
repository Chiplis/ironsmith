//! Repeatable noncopiable action state and exact action payment receipts.
//! Requires named-action matcher/producer plus the Advance incremental patch.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
const A: PlayerId = PlayerId(0);
fn definitions(name: &str, text: &str, payment: Option<&str>) -> [CardDefinition; 2] {
    let mut builder =
        ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), name);
    if let Some(label) = payment {
        builder = builder.with_custom_keyword(label);
    }
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler::CompilerFacade::new().compile_definition(
                builder,
                text.to_string(),
                ironsmith_compiler::CompilePolicy {
                    allow_unsupported: false,
                },
            )
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let mut compiled = result.unwrap();
    if let Some(label) = payment {
        assert!(compiled.definition.attribute_custom_keyword_payment(label));
    }
    let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
    let direct =
        ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone())
            .unwrap();
    let artifact = CompiledCardArtifact::new(
        ArtifactCardIdentity {
            local_id: ArtifactCardId(direct.card.id.0),
            name: direct.card.name.clone(),
            face_name: None,
            other_face: None,
            linked_face_layout: None,
        },
        CompiledCardPayload {
            definition: wire,
            canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"),
            ability_labels: ironsmith_text::ability_surface_texts(&direct),
        },
        "named-action-state",
        text.as_bytes(),
    );
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn act(game: &mut GameState, action: LegalAction) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
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
            return;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    panic!("unfinished action");
}
fn settle(game: &mut GameState) {
    let mut dm = SelectFirstDecisionMaker;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for _ in 0..32 {
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut dm)
            .unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    panic!("unsettled");
}
#[test]
fn complete_construct_engine_alternative_entry_and_repeat_activations_each_advance_once() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-advance.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "msem-778")
        .unwrap();
    let card = &row["representative_cards"][0];
    for definition in definitions(
        card["name"].as_str().unwrap(),
        card["executable_text"].as_str().unwrap(),
        Some("advance"),
    ) {
        for alternative in [false, true] {
            let mut game = game();
            for symbol in [ManaSymbol::White, ManaSymbol::Colorless] {
                game.player_mut(A).unwrap().mana_pool.add(symbol, 20);
            }
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = game.object(source).unwrap().stable_id;
            let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==source && matches!(casting_method,CastingMethod::Alternative(_))==alternative)).unwrap();
            act(&mut game, action);
            settle(&mut game);
            let source = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.named_action_count(source, "advance", false),
                u32::from(alternative)
            );
            assert_eq!(game.battlefield.len(), if alternative { 2 } else { 1 });
            for n in 1..=2 {
                let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,..}if *id==source)).unwrap();
                act(&mut game, action);
                settle(&mut game);
                assert_eq!(
                    game.named_action_count(source, "advance", false),
                    n + u32::from(alternative)
                );
            }
            assert_eq!(game.battlefield.len(), if alternative { 4 } else { 3 });
        }
    }
}
#[test]
fn arbitrary_action_history_is_not_counters_and_resets_per_turn_and_incarnation() {
    for definition in definitions(
        "History source",
        "Type: Creature — Bear\nPower/Toughness: 2/2\n{0}: Have this permanent perform the escalation action.\nWhenever this permanent performs the escalation action, you gain 1 life.",
        None,
    ) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        let action = compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source))
            .unwrap();
        act(&mut game, action);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 21);
        assert_eq!(game.named_action_count(source, "escalation", false), 1);
        assert!(game.object(source).unwrap().counters.is_empty());
        game.turn.turn_number += 1;
        assert_eq!(game.named_action_count(source, "escalation", true), 0);
        assert_eq!(game.named_action_count(source, "escalation", false), 1);
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, source).unwrap();
        assert_eq!(
            snapshot.named_action_history.as_ref().unwrap()["escalation"].0,
            1
        );
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let graveyard = game.find_object_by_stable_id(stable).unwrap();
        game.move_object_by_effect(graveyard, Zone::Battlefield)
            .unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(source, returned);
        assert_eq!(game.named_action_count(returned, "escalation", false), 0);
    }
}
#[test]
fn detached_activation_cannot_advance_a_returned_source() {
    for definition in definitions(
        "Departing actor",
        "Type: Creature — Bear\nPower/Toughness: 2/2\n{0}: Have this permanent perform the escalation action.\nWhenever this permanent performs the escalation action, you gain 1 life.",
        None,
    ) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        let action = compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source))
            .unwrap();
        act(&mut game, action);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let graveyard = game.find_object_by_stable_id(stable).unwrap();
        game.move_object_by_effect(graveyard, Zone::Battlefield)
            .unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap();
        settle(&mut game);
        assert_eq!(game.named_action_count(returned, "escalation", false), 0);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}

#[test]
fn scalar_history_updates_continuous_characteristics_and_is_not_copied_or_removed_with_counters() {
    let text = "Type: Creature — Bear\nPower/Toughness: 2/2\nThis creature gets +X/+0, where X is the number of times this permanent has performed the escalation action this turn.\n{0}: Have this permanent perform the escalation action.";
    for definition in definitions("Dynamic actor", text, None) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(game.calculated_power(source), Some(2));
        for expected in [3, 4] {
            let action = compute_legal_actions(&game, A)
                .unwrap()
                .into_iter()
                .find(|a| matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source))
                .unwrap();
            act(&mut game, action);
            settle(&mut game);
            assert_eq!(game.calculated_power(source), Some(expected));
        }
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::PutCountersEffect::new(
                ironsmith::object::CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::SpecificObject(source),
            )),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        assert_eq!(game.calculated_power(source), Some(5));
        game.remove_counters(
            source,
            ironsmith::object::CounterType::PlusOnePlusOne,
            1,
            Some(source),
            Some(A),
        )
        .unwrap();
        assert_eq!(
            game.calculated_power(source),
            Some(4),
            "removing counters does not remove action history"
        );
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::CreateTokenCopyEffect::new(
                ChooseSpec::SpecificObject(source),
                1,
                ironsmith::target::PlayerFilter::You,
            )),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        let token = *game.battlefield.iter().find(|id| **id != source).unwrap();
        assert_eq!(game.named_action_count(token, "escalation", false), 0);
        assert_eq!(game.calculated_power(token), Some(2));
        game.next_turn();
        assert_eq!(
            game.calculated_power(source),
            Some(2),
            "turn-scoped action history expires independently"
        );
    }
}
#[test]
fn action_mana_receipt_does_not_borrow_a_previous_cast_payment() {
    let text = "Mana cost: {R}{W}{B}\nType: Artifact\n{3}: Have this permanent perform the escalation action.\nWhenever this permanent performs the escalation action, if {R}{W}{B} was spent to perform the escalation action, you gain 5 life.";
    for definition in definitions("Payment actor", text, None) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for symbol in [ManaSymbol::Red, ManaSymbol::White, ManaSymbol::Black] {
            game.object_mut(source)
                .unwrap()
                .mana_spent_to_cast
                .add(symbol, 1);
        }
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);
        let action = compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source))
            .unwrap();
        act(&mut game, action);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 20);
        for symbol in [ManaSymbol::Red, ManaSymbol::White, ManaSymbol::Black] {
            game.player_mut(A).unwrap().mana_pool.add(symbol, 1);
        }
        let action = compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source))
            .unwrap();
        act(&mut game, action);
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().life,
            25,
            "only exact action payment colors qualify"
        );
    }
}

#[test]
fn entry_action_retains_selected_cast_payment_instead_of_inventing_activation_mana() {
    let text = "Mana cost: {1}\nType: Artifact\nYou may pay {R}{W}{B} rather than pay this spell's mana cost.\nAs this permanent enters, if its escalation cost was paid, have this permanent perform the escalation action.\nWhenever this permanent performs the escalation action, if {R}{W}{B} was spent to perform the escalation action, you gain 5 life.";
    for definition in definitions("Entry payment actor", text, Some("escalation")) {
        for alternative in [false, true] {
            let mut game = game();
            for symbol in [
                ManaSymbol::Red,
                ManaSymbol::White,
                ManaSymbol::Black,
                ManaSymbol::Colorless,
            ] {
                game.player_mut(A).unwrap().mana_pool.add(symbol, 1);
            }
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = game.object(source).unwrap().stable_id;
            let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==source && matches!(casting_method,CastingMethod::Alternative(_))==alternative)).unwrap();
            act(&mut game, action);
            settle(&mut game);
            let source = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.named_action_count(source, "escalation", false),
                u32::from(alternative)
            );
            assert_eq!(
                game.player(A).unwrap().life,
                if alternative { 25 } else { 20 }
            );
        }
    }
}
