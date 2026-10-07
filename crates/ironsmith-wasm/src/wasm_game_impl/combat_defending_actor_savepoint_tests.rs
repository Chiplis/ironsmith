// RECOVERED / UNRUN. Native branches retain GameState, event envelopes,
// stack copies, runner state and pending decision continuations together.
fn defender_native_definition(name:&str)->ironsmith::cards::CardDefinition {
    let rows:Vec<serde_json::Value>=serde_json::from_str(include_str!("../../../../fixtures/combat_defending_actor_references.json.fixture")).unwrap();
    let row=rows.iter().find(|row|row["name"]==name).unwrap();let mut text=format!("Mana cost: {}\nType: {}\n",row["mana_cost"].as_str().unwrap(),row["type_line"].as_str().unwrap());
    if let (Some(p),Some(t))=(row["power"].as_str(),row["toughness"].as_str()){text.push_str(&format!("Power/Toughness: {p}/{t}\n"));}text.push_str(row["oracle_text"].as_str().unwrap());
    ironsmith_registry_test::compile_to_runtime_definition(name,text,false).unwrap()
}
fn defender_native_fixture()->WasmGame {
    let mut wasm=WasmGame::new();wasm.initialize_empty_match(vec!["Alice".into(),"Bob".into(),"Charlie".into(),"Dana".into()],20,1);
    wasm.game.turn.active_player=PlayerId(0);wasm.game.turn.priority_player=Some(PlayerId(0));wasm.game.turn.turn_number=1;wasm.game.turn.phase=Phase::Combat;wasm.game.turn.step=Some(Step::DeclareAttackers);
    wasm.priority_state=PriorityLoopState::new(4);wasm.priority_state.seed_priority_tracker_for_test(0,4);wasm
}
fn defender_native_attack(wasm:&mut WasmGame,source:ObjectId,defender:PlayerId){
    wasm.game.remove_summoning_sickness(source);let mut combat=ironsmith::combat_state::CombatState::default();
    ironsmith::game_loop::apply_attacker_declarations(&mut wasm.game,&mut combat,&mut wasm.trigger_queue,&[ironsmith::decision::AttackerDeclaration{creature:source,target:ironsmith::combat_state::AttackTarget::Player(defender)}]).unwrap();wasm.game.combat=Some(combat);
}
#[test]
fn defender_native_savepoint_restores_current_and_last_roles_without_cross_branch_aliasing(){
    let _guard=crate::test_id_counter_guard();let mut wasm=defender_native_fixture();let source=wasm.game.create_object_from_definition(&defender_native_definition("Falkenrath Perforator"),PlayerId(0),Zone::Battlefield);
    defender_native_attack(&mut wasm,source,PlayerId(1));ironsmith::game_loop::put_triggers_on_stack(&mut wasm.game,&mut wasm.trigger_queue).unwrap();let original=RuntimeSavepoint::capture(&wasm);
    let mut runner=ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::EndCombatPriority);*runner.combat_mut()=wasm.game.combat.clone().unwrap();
    wasm.game.combat.as_mut().unwrap().attackers[0].target=ironsmith::combat_state::AttackTarget::Player(PlayerId(2));runner.advance(&mut wasm.game,&mut wasm.trigger_queue).unwrap();wasm.runner=Some(runner);let ended=RuntimeSavepoint::capture(&wasm);
    ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();assert_eq!(wasm.game.player(PlayerId(2)).unwrap().life,19);original.clone().restore(&mut wasm);
    ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();assert_eq!(wasm.game.player(PlayerId(1)).unwrap().life,19);assert_eq!(wasm.game.player(PlayerId(2)).unwrap().life,20);
    ended.restore(&mut wasm);assert!(wasm.game.combat.as_ref().unwrap().attackers.is_empty());ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();assert_eq!(wasm.game.player(PlayerId(2)).unwrap().life,19);
    original.restore(&mut wasm);let graveyard=wasm.game.move_object_by_effect(source,Zone::Graveyard).unwrap();let returned=wasm.game.move_object_by_effect(graveyard,Zone::Battlefield).unwrap();assert_ne!(returned,source);
    wasm.game.combat.as_mut().unwrap().attackers.push(ironsmith::combat_state::AttackerInfo{creature:returned,target:ironsmith::combat_state::AttackTarget::Player(PlayerId(3))});let reincarnated=RuntimeSavepoint::capture(&wasm);reincarnated.restore(&mut wasm);
    ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();assert_eq!(wasm.game.player(PlayerId(1)).unwrap().life,19);assert_eq!(wasm.game.player(PlayerId(3)).unwrap().life,20);
}
#[test]
fn defender_native_pending_blocking_memory_choice_round_trips_and_rejects_invalid_answers(){
    use ironsmith::decision::{AttackerDeclaration,BlockerDeclaration,DecisionMaker};
    use ironsmith::target::ObjectFilter;
    use ironsmith::replacement::{ReplacementAction,ReplacementEffect,RedirectTarget,RedirectWhich};
    struct InitialTarget;
    impl DecisionMaker for InitialTarget {
        fn decide_targets(&mut self,_:&GameState,context:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{let target=ironsmith::Target::Player(PlayerId(2));assert!(context.requirements.iter().any(|requirement|requirement.legal_targets.contains(&target)));vec![target]}
    }
    let _guard=crate::test_id_counter_guard();let mut wasm=defender_native_fixture();let source=wasm.game.create_object_from_definition(&defender_native_definition("Memory Vampire"),PlayerId(1),Zone::Battlefield);
    let plain=ironsmith_registry_test::compile_to_runtime_definition("Native attacker","Type: Creature — Human\nPower/Toughness: 1/8",false).unwrap();let attacker=wasm.game.create_object_from_definition(&plain,PlayerId(0),Zone::Battlefield);
    let evidence=ironsmith_registry_test::compile_to_runtime_definition("Native evidence","Mana cost: {3}\nType: Artifact",false).unwrap();let paid:Vec<_>=(0..3).map(|_|wasm.game.create_object_from_definition(&evidence,PlayerId(1),Zone::Graveyard)).collect();
    let spell=ironsmith_registry_test::compile_to_runtime_definition("Native spell","Mana cost: {6}\nType: Instant\nYou gain 3 life.",false).unwrap();let spells:Vec<_>=(1..4).map(|player|wasm.game.create_object_from_definition(&spell,PlayerId(player),Zone::Graveyard)).collect();
    for _ in 0..7{wasm.game.create_object_from_definition(&plain,PlayerId(2),Zone::Library);}wasm.game.remove_summoning_sickness(attacker);let mut combat=ironsmith::combat_state::CombatState::default();
    ironsmith::game_loop::apply_attacker_declarations(&mut wasm.game,&mut combat,&mut wasm.trigger_queue,&[AttackerDeclaration{creature:attacker,target:ironsmith::combat_state::AttackTarget::Player(PlayerId(1))}]).unwrap();
    wasm.game.combat=Some(combat.clone());wasm.game.turn.step=Some(Step::DeclareBlockers);
    ironsmith::game_loop::apply_multiplayer_blocker_declarations(&mut wasm.game,&mut combat,&mut wasm.trigger_queue,&[BlockerDeclaration{blocker:source,blocking:attacker}]).unwrap();
    wasm.game.combat=Some(combat.clone());wasm.game.turn.step=Some(Step::CombatDamage);
    wasm.game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source,PlayerId(1),ironsmith::events::damage::matchers::DamageToObjectMatcher::new(ObjectFilter::specific(attacker)),ReplacementAction::Redirect{target:RedirectTarget::ToPlayer(PlayerId(2)),which:RedirectWhich::First}));
    let damage=ironsmith::game_loop::execute_combat_damage_step(&mut wasm.game,&combat,false);ironsmith::game_loop::queue_combat_damage_triggers(&mut wasm.game,&damage,&mut wasm.trigger_queue);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut wasm.game,&mut wasm.trigger_queue,&mut InitialTarget).unwrap();wasm.runner=Some(ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::CombatDamageRegularPriority));
    *wasm.runner.as_mut().unwrap().combat_mut()=combat;wasm.runner_awaiting_priority=true;payment_disclosure_prepare_priority(&mut wasm);
    for _ in 0..24{match wasm.pending_decision.clone(){
        Some(DecisionContext::Priority(_))=>dispatch_pass_priority(&mut wasm),
        Some(DecisionContext::Boolean(_))=>dispatch_manual_payment_command(&mut wasm,UiCommand::SelectOptions{option_indices:vec![1]}),
        Some(DecisionContext::SelectObjects(_))=>dispatch_manual_payment_command(&mut wasm,UiCommand::SelectObjects{object_ids:paid.iter().map(|id|id.0).collect(),object_stable_ids:Vec::new(),object_hidden_refs:Vec::new()}),
        Some(DecisionContext::SelectOptions(context))if context.options.iter().any(|option|option.description=="Dana")=>break,
        other=>panic!("unexpected native continuation: {other:?}"),
    }}
    let Some(DecisionContext::SelectOptions(context))=wasm.pending_decision.as_ref()else{panic!("defending-player choice");};assert_eq!(context.player,PlayerId(1));assert_eq!(context.options.iter().map(|option|option.description.as_str()).collect::<Vec<_>>(),vec!["Bob","Charlie","Dana"]);
    let pending=RuntimeSavepoint::capture(&wasm);let before=wasm.game.next_object_id_counter();assert!(wasm.dispatch_typed_command(UiCommand::SelectOptions{option_indices:vec![99]},0.0).is_err());assert_eq!(wasm.game.next_object_id_counter(),before);
    pending.clone().restore(&mut wasm);dispatch_manual_payment_command(&mut wasm,UiCommand::SelectOptions{option_indices:vec![2]});
    let Some(DecisionContext::Targets(targets))=wasm.pending_decision.as_ref()else{panic!("reflexive targets");};
    assert!(targets.requirements.iter().any(|requirement|requirement.legal_targets.contains(&ironsmith::Target::Object(spells[2]))));assert!(!targets.requirements.iter().any(|requirement|requirement.legal_targets.contains(&ironsmith::Target::Object(spells[1]))));
    pending.restore(&mut wasm);dispatch_manual_payment_command(&mut wasm,UiCommand::SelectOptions{option_indices:vec![0]});let Some(DecisionContext::Targets(targets))=wasm.pending_decision.as_ref()else{panic!("restored reflexive targets");};
    assert!(targets.requirements.iter().any(|requirement|requirement.legal_targets.contains(&ironsmith::Target::Object(spells[0]))));assert!(!targets.requirements.iter().any(|requirement|requirement.legal_targets.contains(&ironsmith::Target::Object(spells[2]))));
}
#[test]
fn defender_native_missing_evidence_restores_the_stack_and_retries_the_same_failure(){
    let _guard=crate::test_id_counter_guard();let mut wasm=defender_native_fixture();let source=wasm.game.create_object_from_definition(&defender_native_definition("Falkenrath Perforator"),PlayerId(0),Zone::Battlefield);
    defender_native_attack(&mut wasm,source,PlayerId(1));ironsmith::game_loop::put_triggers_on_stack(&mut wasm.game,&mut wasm.trigger_queue).unwrap();wasm.game.stack.last_mut().unwrap().defending_player_reference=Some(ironsmith::combat_state::DefendingPlayerReference::Missing);let saved=RuntimeSavepoint::capture(&wasm);
    for attempt in 0..2{let before=wasm.game.next_object_id_counter();let error=ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap_err();assert!(error.to_string().contains("retained evidence"));assert_eq!(wasm.game.stack.len(),1);assert_eq!(wasm.game.next_object_id_counter(),before);assert_eq!(wasm.game.player(PlayerId(1)).unwrap().life,20);if attempt==0{saved.clone().restore(&mut wasm);}}
}
#[test]
fn defender_native_copied_cane_trigger_revalidates_its_attacker_after_reselection_and_removal(){
    use ironsmith::effects::{EffectExecutor, EffectContext as ExecutionContext};use ironsmith::target::ChooseSpec;
    let _guard=crate::test_id_counter_guard();for remove in [false,true]{
        let mut wasm=defender_native_fixture();let equipment=wasm.game.create_object_from_definition(&defender_native_definition("Blue Mage's Cane"),PlayerId(0),Zone::Battlefield);
        let host_def=ironsmith_registry_test::compile_to_runtime_definition("Copied trigger attacker","Type: Creature — Human\nPower/Toughness: 2/4",false).unwrap();let host=wasm.game.create_object_from_definition(&host_def,PlayerId(0),Zone::Battlefield);wasm.game.attach_object_to_target(equipment,ironsmith::object::AttachmentTarget::Object(host));
        let spell=ironsmith_registry_test::compile_to_runtime_definition("Cane target","Mana cost: {7}\nType: Instant\nYou gain 2 life.",false).unwrap();let target=wasm.game.create_object_from_definition(&spell,PlayerId(1),Zone::Graveyard);defender_native_attack(&mut wasm,host,PlayerId(1));
        struct OriginalTarget(ObjectId);impl ironsmith::decision::DecisionMaker for OriginalTarget{
            fn decide_targets(&mut self,_:&GameState,context:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{let target=ironsmith::Target::Object(self.0);assert!(context.requirements.iter().any(|requirement|requirement.legal_targets.contains(&target)));vec![target]}}
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut wasm.game,&mut wasm.trigger_queue,&mut OriginalTarget(target)).unwrap();let original=wasm.game.stack.last().unwrap().clone();assert_eq!(original.targets,vec![ironsmith::Target::Object(target)]);let ability=original.ability_id.unwrap();let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
        ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(ability)).execute(&mut wasm.game,&mut ExecutionContext::new(equipment,PlayerId(0),&mut dm)).unwrap();assert_eq!(wasm.game.stack.len(),2);assert_eq!(wasm.game.stack.last().unwrap().defending_player_reference,original.defending_player_reference);
        let saved=RuntimeSavepoint::capture(&wasm);wasm.game.combat.as_mut().unwrap().attackers[0].target=ironsmith::combat_state::AttackTarget::Player(PlayerId(2));if remove{wasm.game.move_object_by_effect(host,Zone::Graveyard).unwrap();}wasm.game.player_mut(PlayerId(0)).unwrap().mana_pool.colorless=6;
        ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();assert!(wasm.game.stack_is_empty());assert_eq!(wasm.game.object(target).unwrap().zone,Zone::Graveyard);assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool.total(),6);
        saved.restore(&mut wasm);assert_eq!(wasm.game.stack.last().unwrap().defending_player_reference,original.defending_player_reference);assert_eq!(wasm.game.defending_player_candidates(original.defending_player_reference.unwrap()).unwrap(),vec![PlayerId(1)]);
    }
}
