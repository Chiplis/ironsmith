//! Exact authored activation cause, independently composed with counter events.
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::object::CounterType;
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const TEXT: &str = "Type: Creature — Bear\nPower/Toughness: 2/2\n{0}: This is the enormity activated ability. If this permanent has no +1/+1 counters on it, put three +1/+1 counters on it. Activate only as a sorcery.\n{0}: Put a +1/+1 counter on this creature.\nWhenever one or more +1/+1 counters are put on this creature caused by the enormity activated ability of this permanent, you gain 5 life.";
fn definitions(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_builder_to_artifact(
                ironsmith_compiler::CardDefinitionBuilder::new(
                    ironsmith_compiler::ids::CardId::new(),
                    "Named producer",
                ),
                text,
                false,
            )
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn setup(definition: &CardDefinition) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    (game, source)
}
fn activate(game: &mut GameState, source: ObjectId, index: usize) {
    let selected=compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,ability_index,..} if *id==source && *ability_index==index)).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(selected),
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
fn settle(game: &mut GameState) {
    let mut dm = SelectFirstDecisionMaker;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for _ in 0..16 {
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut dm)
            .unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    panic!("unsettled stack");
}
#[test]
fn labeled_counter_placement_is_exact_and_zero_result_does_not_trigger() {
    for definition in definitions(TEXT) {
        let (mut game, source) = setup(&definition);
        activate(&mut game, source, 0);
        settle(&mut game);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 3);
        assert_eq!(
            game.player(A).unwrap().life,
            25,
            "one trigger for the placement"
        );
        activate(&mut game, source, 0);
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().life,
            25,
            "conditional zero placement has no event"
        );
        activate(&mut game, source, 1);
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().life,
            25,
            "second same-source activation has no matching cause"
        );
        let external = ironsmith::Effect::new(ironsmith::effects::PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            1,
            ironsmith::target::ChooseSpec::SpecificObject(source),
        ));
        ironsmith::effects::execute_effect(
            &mut game,
            &external,
            &mut ironsmith::effects::EffectContext::new_default(source, A),
        )
        .unwrap();
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().life,
            25,
            "same-source effect without exact activation evidence cannot match"
        );
    }
}
#[test]
fn labels_are_not_runtime_keyword_identity_and_ambiguous_or_missing_bindings_reject() {
    for text in [
        TEXT.replace("This is the enormity activated ability. ", ""),
        TEXT.replace(
            "{0}: Put a +1/+1 counter",
            "{0}: This is the enormity activated ability. Put a +1/+1 counter",
        ),
    ] {
        assert!(
            ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Invalid binding",
                &text,
                false
            )
            .is_err()
        );
    }
}

#[test]
fn copied_activation_retains_authored_producer_and_source_departure_does_not_retarget_new_incarnation()
 {
    for definition in definitions(TEXT) {
        let (mut game, source) = setup(&definition);
        activate(&mut game, source, 0);
        let ability = game.stack[0].object_id;
        let stamp = game.stack[0]
            .activation_definition
            .expect("authored producer");
        let copy = ironsmith::Effect::new(
            ironsmith::effects::CopySpellEffect::new(
                ironsmith::target::ChooseSpec::SpecificObject(ability),
                1,
            )
            .with_target_reference_kind(ironsmith::filter::StackObjectKind::Ability),
        );
        ironsmith::effects::execute_effect(
            &mut game,
            &copy,
            &mut ironsmith::effects::EffectContext::new_default(source, A),
        )
        .unwrap();
        assert_eq!(game.stack.len(), 2);
        assert!(
            game.stack
                .iter()
                .all(|entry| entry.activation_definition == Some(stamp))
        );
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().life,
            25,
            "copy places counters once; original condition then does nothing"
        );

        let (mut game, source) = setup(&definition);
        let stable = game.object(source).unwrap().stable_id;
        activate(&mut game, source, 0);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let graveyard = game.find_object_by_stable_id(stable).unwrap();
        game.move_object_by_effect(graveyard, Zone::Battlefield)
            .unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(source, returned);
        settle(&mut game);
        assert_eq!(game.counter_count(returned, CounterType::PlusOnePlusOne), 0);
        assert_eq!(
            game.player(A).unwrap().life,
            20,
            "old activation does not acquire a new source incarnation"
        );
    }
}

#[test]
fn replacement_doubled_placement_triggers_once_and_prevented_placement_never_triggers() {
    for definition in definitions(TEXT) {
        for prevent in [false, true] {
            let (mut game, source) = setup(&definition);
            let text = if prevent {
                "Type: Enchantment\nCounters can't be put on creatures you control."
            } else {
                "Type: Enchantment\nIf one or more +1/+1 counters would be put on a creature you control, twice that many +1/+1 counters are put on it instead."
            };
            let modifier = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Counter modifier",
                text,
                false,
            )
            .unwrap();
            game.create_object_from_definition(&modifier, A, Zone::Battlefield);
            activate(&mut game, source, 0);
            settle(&mut game);
            assert_eq!(
                game.counter_count(source, CounterType::PlusOnePlusOne),
                if prevent { 0 } else { 6 }
            );
            assert_eq!(game.player(A).unwrap().life, if prevent { 20 } else { 25 });
        }
    }
}
