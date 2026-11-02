//! Aggregate tap cost, designation event, and sacrifice compose independently.
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm, resolve_stack_entry_with};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const RULES: &str = "Type: Enchantment\nWhen this enchantment gains the traversed status, you gain 3 life.\nTap any number of untapped creatures you control with total power 4 or greater: Give this permanent the traversed status. Sacrifice this permanent. Activate only as a sorcery.";
fn definitions() -> [CardDefinition; 2] {
    let direct = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_runtime_definition("Aggregate threshold", RULES, false)).unwrap();
    let (artifact, _) = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Aggregate threshold", RULES, false)).unwrap();
    let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn body(power: i32) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition("Power contributor", &format!("Type: Creature\nPower/Toughness: {power}/3"), false).unwrap()
}
fn offered(game: &GameState, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility {source: id, ..} if *id == source))
}
fn setup(definition: &CardDefinition) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain; game.turn.step = None;
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    (game, source)
}
#[test]
fn aggregate_payment_requires_own_untapped_power_and_sorcery_timing() {
    for definition in definitions() {
        let (mut game, source) = setup(&definition);
        let first = game.create_object_from_definition(&body(2), A, Zone::Battlefield);
        game.create_object_from_definition(&body(9), B, Zone::Battlefield);
        assert!(offered(&game, source).is_none(), "opponent power cannot pay");
        let second = game.create_object_from_definition(&body(2), A, Zone::Battlefield);
        game.tap(second);
        assert!(offered(&game, source).is_none(), "already tapped creature cannot pay");
        game.untap(second);
        assert!(offered(&game, source).is_some(), "sum of two2-power creatures reaches4");
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        assert!(offered(&game, source).is_none());
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let action = offered(&game, source).unwrap();
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() {break;}
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
        }
        assert!(!state.has_pending_action());
        assert!(game.is_tapped(first));
        assert!(game.is_tapped(second));
        let stable = game.object(source).unwrap().stable_id;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let departed = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(departed).unwrap().zone, Zone::Graveyard);
        assert!(!game.has_custom_status(departed, "traversed"), "designation resets after sacrifice");
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1, "gained designation is observed before source sacrifices itself");
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().life, 23);
    }
}


fn named_payment_definitions(label: &str) -> [CardDefinition; 2] {
    let text = format!("Type: Enchantment\nTap any number of untapped creatures you control with total power 4 or greater as payment named \"{label}\": Give this permanent the traversed status. Sacrifice this permanent. Activate only as a sorcery.");
    let direct = ironsmith_compiler::word_claims::with_enforcement_options(true,true, || ironsmith_compiler_runtime::compile_to_runtime_definition("Named aggregate payment",&text,false)).unwrap();
    let (artifact,_) = ironsmith_compiler::word_claims::with_enforcement_options(true,true, || ironsmith_compiler_runtime::compile_to_artifact("Named aggregate payment",&text,false)).unwrap();
    let artifact = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap()]
}

#[test]
fn named_tap_payment_excludes_prohibited_contributors_and_records_one_exact_paid_batch() {
    for definition in named_payment_definitions("traverse") {
        let (mut game,source)=setup(&definition);
        let prohibited=ironsmith_compiler_runtime::compile_to_runtime_definition("Restricted contributor","Type: Creature\nPower/Toughness: 9/9\nThis creature can't be tapped to pay the generic tap payment named traverse.",false).unwrap();
        let prohibited=game.create_object_from_definition(&prohibited,A,Zone::Battlefield);
        let first=game.create_object_from_definition(&body(2),A,Zone::Battlefield);
        assert!(offered(&game,source).is_none(),"prohibited power does not make the aggregate price payable");
        let second=game.create_object_from_definition(&body(2),A,Zone::Battlefield);
        let action=offered(&game,source).unwrap();
        let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
        let mut progress=apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
        for _ in 0..64 {if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else {panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}
        assert!(!state.has_pending_action());assert!(game.is_tapped(first)&&game.is_tapped(second));assert!(!game.is_tapped(prohibited));
        assert_eq!(game.named_action_count(source,"traverse",true),1,"one payment, independently of its contributor count");
        assert!(game.named_action_affected_this_turn(source,"traverse",first));assert!(game.named_action_affected_this_turn(source,"traverse",second));
        assert!(!game.named_action_affected_this_turn(source,"traverse",prohibited));assert!(!game.has_custom_status(source,"traversed"),"payment precedes the resolving designation");
        use ironsmith::effects::EffectExecutor;
        let ability=game.stack.last().unwrap().target_id();
        ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(ability)).execute(&mut game,&mut ironsmith::effects::EffectContext::new(source,B,&mut dm)).unwrap();
        assert!(game.object(source).is_some());assert!(!game.has_custom_status(source,"traversed"));assert_eq!(game.named_action_count(source,"traverse",true),1,"countering resolution cannot undo completed payment");
    }
}

#[test]
fn named_tap_policy_is_exact_and_actual_payment_rechecks_a_stale_chosen_set() {
    for definition in named_payment_definitions("crossing") {
        let (mut game,source)=setup(&definition);
        let restricted=ironsmith_compiler_runtime::compile_to_runtime_definition("Restricted contributor","Type: Creature\nPower/Toughness: 4/4\nThis creature can't be tapped to pay the generic tap payment named traverse.",false).unwrap();
        let contributor=game.create_object_from_definition(&restricted,A,Zone::Battlefield);
        assert!(offered(&game,source).is_some(),"a different opaque payment name remains legal");
        let mut dm=SelectFirstDecisionMaker;
        let mut ctx=ironsmith::costs::CostContext::new(source,A,&mut dm).with_reason(ironsmith::costs::PaymentReason::ActivateAbility);
        let tap=ironsmith::costs::Cost::validated_effect(ironsmith::Effect::new(ironsmith::effects::TapEffect::with_spec(ironsmith::target::ChooseSpec::SpecificObject(contributor)).with_named_payment(Some("traverse".into()))));
        assert!(tap.pay(&mut game,&mut ctx).is_err(),"actual cost payment cannot bypass eligibility using a saved object selection");
        assert!(!game.is_tapped(contributor));assert_eq!(game.named_action_count(source,"traverse",true),0);
        drop(ctx);
        // Check the producer receipt before source departure, then move the
        // exact Rift. Its recorded incarnation must not rebind to the new one.
        let activation=definition.abilities.iter().find_map(|ability|if let ironsmith::ability::AbilityKind::Activated(activation)=&ability.kind{Some(activation)}else{None}).unwrap();
        let mut ctx=ironsmith::costs::CostContext::new(source,A,&mut dm).with_reason(ironsmith::costs::PaymentReason::ActivateAbility);
        let mut packet=None;
        for part in activation.mana_cost.costs(){let receipt=part.pay_with_outputs(&mut game,&mut ctx).unwrap();if let Some(outputs)=receipt.outputs{for event in outputs.outcome.events{if let Some(action)=event.downcast::<ironsmith::events::NamedActionEvent>(){packet=Some(action.clone());}}}}
        let packet=packet.expect("actual named payment completion");assert_eq!(packet.source,source);assert_eq!(packet.player,A);assert_eq!(packet.amount,1);assert_eq!(packet.objects[0].object_id,contributor);assert!(packet.objects[0].tapped);
        drop(ctx);let stable=game.object(source).unwrap().stable_id;game.move_object_by_effect(source,Zone::Graveyard).unwrap();let departed=game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(departed,packet.source);assert_eq!(packet.snapshot.object_id,source);assert_eq!(packet.snapshot.zone,Zone::Battlefield);
    }
}
