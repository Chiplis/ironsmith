//! Accepted zero operations complete; observer bounds belong to their matchers.
use ironsmith::effect::EffectPredicateRuntimeExt;
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith::decision::DecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::{ChooseSpec, PlayerFilter};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
struct Decision(bool);
impl DecisionMaker for Decision {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.0 }
}
fn observer_pairs() -> [CardDefinition; 2] {
    let text = "Type: Enchantment\nWhenever you perform the fund action, you gain 1 life.\nWhenever you perform the fund action with an amount of 1 or more, you gain 10 life.\nWhenever you perform the fund action with an amount of 2 or more, you gain 100 life.";
    // Freeze the actual compiler model before either conversion route so a
    // parser/lowering loss cannot be mistaken for event matching or serde loss.
    let (compiled,loss)=ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true,true,||
            ironsmith_compiler::CompilerFacade::new().compile_definition(
                ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Amount observers"),
                text.to_string(),ironsmith_compiler::CompilePolicy{allow_unsupported:false})));
    assert!(!loss.is_lossy(),"compiler model: {}",loss.reasons_text());
    let compiled=compiled.unwrap();
    let core_bounds=compiled.definition.abilities.iter().filter_map(|ability|match &ability.kind {
        ironsmith_compiler::ability::AbilityKind::Triggered(triggered)=>match &triggered.trigger.kind {
            ironsmith_core::trigger_model::TriggerKind::NamedAction{label,minimum_amount,object_filter,..}=>Some((label.clone(),*minimum_amount,object_filter.is_some())),_=>None,
        },_=>None,
    }).collect::<Vec<_>>();
    assert_eq!(core_bounds.iter().map(|(_,minimum,_)|*minimum).collect::<Vec<_>>(),vec![0,1,2],
        "raw compiler named-action model bounds (label,minimum,objectfilter): {core_bounds:?}");
    let direct=ironsmith_compiler_runtime::into_runtime_definition(compiled.definition).unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
        ironsmith_compiler_runtime::compile_to_artifact("Amount observers", text, false)));
    assert!(!loss.is_lossy(), "artifact model: {}", loss.reasons_text());
    let (artifact, _artifact_materialized) = result.unwrap();
    let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    let pairs=[direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    for definition in &pairs {
        let bounds=definition.abilities.iter().filter_map(|ability|match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(triggered)=>match &triggered.trigger.compiled_model().unwrap().kind {
                ironsmith_core::trigger_model::TriggerKind::NamedAction{minimum_amount,..}=>Some(*minimum_amount),_=>None,
            },_=>None,
        }).collect::<Vec<_>>();
        assert_eq!(bounds,vec![0,1,2],"each observer must retain its own declared amount bound on each route");
    }
    pairs
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
    let mut matched=0;
    for _ in 0..16 {
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut Decision(true)).unwrap();
        matched+=queue.entries.len();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut Decision(true)).unwrap();
        if game.stack_is_empty() { return matched; }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut Decision(true)).unwrap();
    }
    panic!("unexpected recurring observer");
}
#[test]
fn accepted_zero_retains_completion_history_primary_acceptance_and_unbounded_observers() {
    for observer in observer_pairs() { for count in [0, 1, 2] { for accepted in [false, true] { for actor in [A, B] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&observer, A, Zone::Battlefield);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Performer", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let source = game.create_object_from_definition(&body, actor, Zone::Battlefield);
        let objects: Vec<_> = (0..count).map(|_| {
            let object = game.create_object_from_definition(&body, actor, Zone::Battlefield);
            ironsmith::snapshot::ObjectSnapshot::from_object(game.object(object).unwrap(), &game)
        }).collect();
        let effect = Effect::new(ironsmith::effects::NamedActionProgramEffect {
            label: "fund".into(), player: PlayerFilter::You,
            completion: ironsmith_core::effect::NamedActionProgramCompletion::AcceptedPayment,
            objects: Some(ChooseSpec::Tagged("chosen".into())),
            effects: vec![Effect::new(ironsmith::effects::MayEffect::new(vec![Effect::pay_life(0)]).with_pay_as_cost(true))],
        });
        let mut decision = Decision(accepted);
        let mut ctx = EffectContext::new_default(source, actor).with_decision_maker(&mut decision);
        ctx.tagged_objects.insert("chosen".into(), objects);
        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(ironsmith::effect::EffectPredicate::Happened.evaluate_outcome(&result), accepted);
        assert_eq!(result.instruction_result().value.as_count().unwrap_or(0), 0, "completion amount must not replace accepted payment quantity");
        let receipts: Vec<_> = result.events.iter().filter_map(|event| event.downcast::<ironsmith::events::NamedActionEvent>()).collect();
        assert_eq!(receipts.len(), usize::from(accepted));
        if accepted { assert_eq!(receipts[0].amount, count); assert_eq!(receipts[0].objects.len(), count as usize); }
        assert_eq!(game.named_action_count(source, "fund", false), u32::from(accepted), "zero is one completed operation in source history");
        drop(ctx);
        publish_named_completions(&mut game,&result);
        let matched=settle(&mut game);
        let expected_observers=if accepted && actor==A {1+usize::from(count>=1)+usize::from(count>=2)}else{0};
        assert_eq!(matched,expected_observers,"amount {count}, accepted {accepted}, actor {actor:?}: actual event observer count");
        let gained = if accepted && actor == A { 1 + if count >= 1 {10} else {0} + if count >= 2 {100} else {0} } else {0};
        assert_eq!(game.player(A).unwrap().life, 20 + gained, "only explicit minimum bounds exclude amount zero");
    }}}}
}
#[test]
fn missing_collection_is_not_an_authoritative_empty_completion() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Performer", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
    let source = game.create_object_from_definition(&body, A, Zone::Battlefield);
    let effect = Effect::new(ironsmith::effects::NamedActionProgramEffect {
        label: "fund".into(), player: PlayerFilter::You,
        completion: ironsmith_core::effect::NamedActionProgramCompletion::AcceptedPayment,
        objects: Some(ChooseSpec::Tagged("missing".into())),
        effects: vec![Effect::new(ironsmith::effects::MayEffect::new(vec![Effect::pay_life(0)]).with_pay_as_cost(true))],
    });
    let mut decision = Decision(true);
    let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut decision);
    assert!(execute_effect(&mut game, &effect, &mut ctx).is_err());
    assert_eq!(game.named_action_count(source, "fund", false), 0);
}
#[test]
fn successful_zero_predecessor_cost_marker_records_completion_but_failed_or_missing_receipts_do_not() {
    for (receipt, expected) in [
        (Some(ironsmith::effect::EffectOutcome::count(0)), Some(true)),
        (Some(ironsmith::effect::EffectOutcome::with_objects(vec![])), Some(true)),
        (Some(ironsmith::effect::EffectOutcome::replaced()), Some(false)),
        (Some(ironsmith::effect::EffectOutcome::prevented()), Some(false)),
        (Some(ironsmith::effect::EffectOutcome::declined()), Some(false)),
        (Some(ironsmith::effect::EffectOutcome::count(1)), None),
        (None, None),
    ] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Performer", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let source = game.create_object_from_definition(&body, A, Zone::Battlefield);
        let mut ctx = EffectContext::new_default(source, A);
        if let Some(receipt) = receipt { ctx.store_outcome(ironsmith::effect::EffectId(101), receipt); }
        let effect = Effect::new(ironsmith::effects::PerformNamedActionEffect {
            label: "fund".into(), performer: None, objects: None,
            prior_objects: Some((ironsmith::effect::EffectId(101), ironsmith_core::PriorEffectAction::Exiled)),
        });
        let result = execute_effect(&mut game, &effect, &mut ctx);
        match expected {
            None => assert!(result.is_err(), "missing producer evidence cannot fabricate amount zero"),
            Some(completed) => {
                let result = result.unwrap();
                assert_eq!(result.events.iter().filter(|event| event.downcast::<ironsmith::events::NamedActionEvent>().is_some()).count(), usize::from(completed));
                assert_eq!(game.named_action_count(source, "fund", false), u32::from(completed));
            }
        }
    }
}
