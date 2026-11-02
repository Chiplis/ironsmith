//! Named-operation participant roles retain their own pre-action observer
//! packet and exact source/object incarnations, without reviving actor roles.
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
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
fn add(game: &mut GameState, name: &str, text: &str) -> ObjectId {
    let card =
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&card, A, Zone::Battlefield)
}

fn publish_named_completions(game:&mut GameState,outcome:&ironsmith::effect::EffectOutcome) {
    // Raw execution returns receipt events. Route the completed named action
    // exactly once; physical actions already publish their own events.
    for event in &outcome.events {
        if event.downcast::<ironsmith::events::NamedActionEvent>().is_some() {
            game.queue_trigger_event(event.provenance(),event.clone());
        }
    }
}
fn settle(game: &mut GameState) -> usize {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut dm).unwrap();
    let count = queue.entries.len();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    count
}
#[test]
fn participant_ltb_sources_and_old_observers_do_not_revive_actor_or_late_participant_abilities() {
    let victim_text = "Type: Creature\nPower/Toughness: 2/2\nWhenever this creature participates in the enspirit action, you gain 2 life.\nWhenever you perform the enspirit action, you gain 7 life.\nWhen this creature leaves the battlefield, you gain 3 life.";
    for victim_def in routes("Participant", victim_text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let actor = add(&mut game, "Actor", "Type: Artifact");
        let victim = game.create_object_from_definition(&victim_def, A, Zone::Battlefield);
        add(
            &mut game,
            "Existing observer",
            "Type: Artifact\nWhenever a creature participates in the enspirit action, you gain 1 life.",
        );
        let mut ctx = EffectContext::new_default(actor, A);
        execute_effect(
            &mut game,
            &Effect::with_id(
                41,
                Effect::new(ironsmith::effects::ExileEffect::with_spec(
                    ChooseSpec::SpecificObject(victim),
                )),
            ),
            &mut ctx,
        )
        .unwrap();
        add(
            &mut game,
            "Late participant observer",
            "Type: Artifact\nWhenever a creature participates in the enspirit action, you gain 9 life.",
        );
        add(
            &mut game,
            "Late actor observer",
            "Type: Artifact\nWhenever you perform the enspirit action, you gain 4 life.",
        );
        let completion=execute_effect(
            &mut game,
            &Effect::new(ironsmith_core::PerformNamedActionEffect {
                performer: None,
                label: "enspirit".into(),
                objects: None,
                prior_objects: Some((
                    ironsmith::effect::EffectId(41),
                    ironsmith_core::PriorEffectAction::Exiled,
                )),
            }),
            &mut ctx,
        )
        .unwrap();
        let action=completion.events.iter().find_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).unwrap();
        let observers=action.participant_source_lookback.as_ref().expect("the physical predecessor retains its original observer packet even when its events were queued separately");
        assert!(observers.iter().any(|source|source.name=="Existing observer"));
        assert!(!observers.iter().any(|source|source.name=="Late participant observer"));
        publish_named_completions(&mut game,&completion);
        assert_eq!(
            settle(&mut game),
            4,
            "LTB ordinary, LTB participant, old spectator and current actor spectator"
        );
        assert_eq!(game.player(A).unwrap().life, 30);
    }
}
#[test]
fn singular_participants_fan_out_while_one_or_more_and_actor_roles_remain_single() {
    for restored in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let actor = add(&mut game, "Actor", "Type: Artifact");
        let singular = routes(
            "Singular",
            "Type: Artifact\nWhenever a creature participates in the trade action, you gain 1 life.",
        );
        let aggregate = routes(
            "Aggregate",
            "Type: Artifact\nWhenever one or more creatures participate in the trade action, you gain 5 life.",
        );
        game.create_object_from_definition(&singular[usize::from(restored)], A, Zone::Battlefield);
        game.create_object_from_definition(&aggregate[usize::from(restored)], A, Zone::Battlefield);
        add(
            &mut game,
            "Actor observer",
            "Type: Artifact\nWhenever you perform the trade action, you gain 3 life.",
        );
        let first = add(&mut game, "First", "Type: Creature\nPower/Toughness: 1/1");
        let second = add(&mut game, "Second", "Type: Creature\nPower/Toughness: 2/2");
        let mut ctx = EffectContext::new_default(actor, A);
        execute_effect(
            &mut game,
            &Effect::with_id(
                42,
                Effect::new(ironsmith::effects::ExileEffect::with_spec(ChooseSpec::All(
                    ObjectFilter::creature().controlled_by(PlayerFilter::You),
                ))),
            ),
            &mut ctx,
        )
        .unwrap();
        let completion=execute_effect(
            &mut game,
            &Effect::new(ironsmith_core::PerformNamedActionEffect {
                performer: None,
                label: "trade".into(),
                objects: None,
                prior_objects: Some((
                    ironsmith::effect::EffectId(42),
                    ironsmith_core::PriorEffectAction::Exiled,
                )),
            }),
            &mut ctx,
        )
        .unwrap();
        publish_named_completions(&mut game,&completion);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(
            &mut game,
            &mut queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(queue.entries.len(), 4);
        let mut singular_ids = Vec::new();
        for entry in &queue.entries {
            let model = entry.ability.trigger.compiled_model().unwrap();
            if matches!(
                model.kind,
                ironsmith_core::trigger_model::TriggerKind::NamedActionParticipant {
                    one_or_more: false,
                    ..
                }
            ) {
                let event = entry
                    .triggering_event
                    .downcast::<ironsmith::events::NamedActionEvent>()
                    .unwrap();
                assert_eq!(event.objects.len(), 1);
                assert_eq!(event.amount, 1);
                singular_ids.push(event.objects[0].object_id);
            }
        }
        singular_ids.sort();
        let mut expected = vec![first, second];
        expected.sort();
        assert_eq!(singular_ids, expected);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut game,
            &mut queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
        }
        assert_eq!(game.player(A).unwrap().life, 30);
    }
}
#[test]
fn only_an_authoritative_empty_action_receipt_completes_without_participant_triggers() {
    let watcher = routes(
        "Observer",
        "Type: Artifact\nWhenever a creature participates in the enspirit action, you gain 9 life.",
    );
    for definition in watcher {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut ctx = EffectContext::new_default(source, A);
        execute_effect(
            &mut game,
            &Effect::with_id(43, Effect::gain_life(1)),
            &mut ctx,
        )
        .unwrap();
        let malformed=execute_effect(
            &mut game,
            &Effect::new(ironsmith_core::PerformNamedActionEffect {
                performer: None,
                label: "enspirit".into(),
                objects: None,
                prior_objects: Some((
                    ironsmith::effect::EffectId(43),
                    ironsmith_core::PriorEffectAction::Exiled,
                )),
            }),
            &mut ctx,
        )
        ;
        assert!(matches!(malformed,Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))),"a life gain is not an authoritative empty exile receipt");
        assert_eq!(game.named_action_count(source,"enspirit",false),0);
        // A real empty original exile packet is different from missing object
        // evidence: completion exists, but no participant can match it.
        execute_effect(&mut game,&Effect::with_id(44,Effect::new(ironsmith::effects::ExileEffect::with_spec(ChooseSpec::All(ObjectFilter::creature())))),&mut ctx).unwrap();
        let empty=execute_effect(&mut game,&Effect::new(ironsmith_core::PerformNamedActionEffect {
            performer:None,label:"enspirit".into(),objects:None,
            prior_objects:Some((ironsmith::effect::EffectId(44),ironsmith_core::PriorEffectAction::Exiled)),
        }),&mut ctx).unwrap();
        let action=empty.events.iter().find_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).expect("an authoritative empty set still completes the action");
        assert_eq!(action.amount,0);assert!(action.objects.is_empty());
        assert_eq!(game.named_action_count(source,"enspirit",false),1);
        publish_named_completions(&mut game,&empty);
        assert_eq!(settle(&mut game), 0);
        assert_eq!(game.player(A).unwrap().life, 21);
    }
}
