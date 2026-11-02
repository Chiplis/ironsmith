fn private_transport_copy_owner_payment(owner:&WasmGame,peer:&mut WasmGame) {
    fn fingerprint(replay:&PendingReplayAction)->(u64,u64){match &replay.nested_answers[0]{
        ReplayDecisionAnswer::ManaPayment(ironsmith::mana_payment::ManaPaymentResponse::Confirm{plan_id,request_hash})=>(*plan_id,*request_hash),_=>panic!("recorded native confirmation")}}
    let owner=owner.pending_replay_action.as_ref().unwrap();
    let peer=peer.pending_replay_action.as_mut().unwrap();
    assert_eq!(fingerprint(owner),fingerprint(peer),"owner and placeholder peer must propose the same authenticated payment question");
    // The network carries the OWNER'S signed response, never a separately
    // generated peer answer that could conceal prompt divergence.
    peer.nested_answers=owner.nested_answers.clone();
}
fn private_transport_payment_answer(wasm:&WasmGame,reservation:&PrivateCostReservation)->ReplayDecisionAnswer {
    let mut dm=WasmReplayDecisionMaker::new(&[]);
    assert!(matches!(stage_private_action(&wasm.game,reservation,&mut dm).unwrap(),PrivateActionStage::AwaitingDecision));
    let Some(DecisionContext::ManaPayment(payment))=dm.pending_context else{panic!("native full price must ask the recorded mana question")};
    ReplayDecisionAnswer::ManaPayment(ironsmith::mana_payment::ManaPaymentResponse::Confirm{plan_id:payment.plan.id,request_hash:payment.plan.request_hash})
}
fn private_transport_definitions()->Vec<CardDefinition> {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../../fixtures/custom_mechanics/msem-premonition-full-source.json")).unwrap();
    let card=&fixture["mechanics"][0]["representative_cards"][0];
    assert_eq!(card["source_card_key"],"Cultish Sorcerer_MSY");
    let text=card["executable_text"].as_str().unwrap();
    let (direct,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_registry_test::compile_to_runtime_definition("Cultish Sorcerer",text,false));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let direct=direct.unwrap();
    let (artifact,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler_runtime_build::compile_to_artifact("Cultish Sorcerer",text,false));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,_)=artifact.unwrap();
    let artifact=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    artifact.validate().unwrap();
    vec![direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap()]
}
fn private_transport_source(wasm:&mut WasmGame,definition:&CardDefinition)->(ObjectId,usize) {
    let source=wasm.game.create_object_from_definition(definition,PlayerId(0),Zone::Hand);
    payment_disclosure_track_hand(wasm,source,21);
    let index=definition.abilities.iter().position(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(rule)
        if matches!(rule.compiled_model().map(|model|&model.payload),Some(ironsmith_core::StaticAbilityPayload::IntrinsicImmediateAction{..})))).unwrap();
    (source,index)
}
#[test]
fn private_cost_wire_roundtrip_does_not_publish_pending_payment_or_open_source() {
    let _ids=crate::test_id_counter_guard();
    for definition in private_transport_definitions() {
        let (mut wasm,_)=manual_payment_fixture();
        let (source,index)=private_transport_source(&mut wasm,&definition);
        wasm.game.create_object_from_definition(&ironsmith_registry_test::cards::definitions::basic_mountain(),PlayerId(0),Zone::Battlefield);
        let action=LegalAction::SpecialAction(ironsmith::special_actions::SpecialAction::PerformIntrinsicAction{source_id:source,ability_index:index});
        let reference=priority_action_ref_for_game(&wasm.game,&action);
        let serialized=serde_json::to_string(&reference).unwrap();
        let reference:PriorityActionRef=serde_json::from_str(&serialized).unwrap();
        assert!(serialized.contains("cost_reservation"));
        let context=DecisionContext::Priority(PriorityContext::new(&wasm.game,PlayerId(0),vec![action]).unwrap());
        let command=UiCommand::PriorityAction{action_index:None,action_ref:Some(reference)};
        let reservation=wasm.private_cost_reservation_for_command(&context,&command).unwrap().unwrap();
        let checkpoint=wasm.capture_replay_checkpoint();
        let root=ReplayRoot::PrivateHandSpecialAction(reservation);
        let before=wasm.capture_crypto_audit_state();
        wasm.previewing_crypto_requirements=true;
        assert!(matches!(wasm.execute_with_replay(&checkpoint,&root,&[]).unwrap(),ReplayOutcome::NeedsDecision(_)));
        wasm.update_crypto_requirements_from(before);
        assert!(wasm.last_crypto_requirements.iter().all(|requirement|requirement.object_id!=Some(source.0)),"unfinished costs never request source proof");
        assert_eq!(wasm.game.object(source).unwrap().zone,Zone::Hand);
        assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool.total(),0);
        assert!(wasm.pending_decision_game.is_none(),"isolated claimed payment world cannot authorize crypto");
    }
}
#[test]
fn verified_final_opening_hydrates_only_retained_root_then_commits_native_action_once() {
    let _ids=crate::test_id_counter_guard();
    for definition in private_transport_definitions() {
        let (mut wasm,_)=manual_payment_fixture();
        let (source,index)=private_transport_source(&mut wasm,&definition);
        wasm.game.player_mut(PlayerId(0)).unwrap().mana_pool.add(ManaSymbol::Colorless,1);
        wasm.game.player_mut(PlayerId(0)).unwrap().mana_pool.add(ManaSymbol::Red,1);
        let occurrence=PrivateActionOccurrence::Intrinsic{ability_index:index};
        let cost=private_action_cost(&wasm.game,PlayerId(0),source,&occurrence).unwrap();
        let reservation=reserve_private_action_with_incarnation(&wasm.game,PlayerId(0),source,Some(0),occurrence,cost).unwrap();
        let checkpoint=wasm.capture_replay_checkpoint();
        wasm.pending_replay_action=Some(PendingReplayAction{checkpoint:checkpoint.clone(),root:ReplayRoot::PrivateHandSpecialAction(reservation.clone()),nested_answers:vec![]});
        let info=wasm.game.hidden_card_info(source).unwrap().clone();
        wasm.record_verified_public_hidden_opening_input(VerifiedPublicPrivateActionOpening{object_id:source.0,owner:0,slot:info.slot,commitment:info.commitment}).unwrap();
        let root=wasm.pending_replay_action.as_ref().unwrap();
        assert!(root.checkpoint.game.is_publicly_revealed_hidden_card(source));
        assert_eq!(root.checkpoint.game.object(source).unwrap().zone,Zone::Hand);
        assert_eq!(root.checkpoint.game.player(PlayerId(0)).unwrap().mana_pool.total(),2,"proof imports identity, never a paid preview");
        let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
        assert!(commit_private_action(&mut wasm.game,&reservation,&mut dm).unwrap());
        assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool.total(),0);
        let stable=checkpoint.game.object(source).unwrap().stable_id;
        let exiled=wasm.game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(wasm.game.object(exiled).unwrap().zone,Zone::Exile);
        assert_eq!(wasm.game.battlefield.iter().filter(|id|wasm.game.object(**id).is_some_and(|object|object.name=="Cultish Sorcerer")).count(),1);
        assert!(commit_private_action(&mut wasm.game,&reservation,&mut dm).is_err(),"same exact source occurrence cannot be paid twice");
    }
}
#[test]
fn private_cost_wire_rejects_wrong_actor_generation_and_crossed_opening_anchors() {
    let _ids=crate::test_id_counter_guard();
    let definition=private_transport_definitions().remove(0);
    let (mut wasm,_)=manual_payment_fixture();
    let (source,index)=private_transport_source(&mut wasm,&definition);
    let action=LegalAction::SpecialAction(ironsmith::special_actions::SpecialAction::PerformIntrinsicAction{source_id:source,ability_index:index});
    let context=DecisionContext::Priority(PriorityContext::new(&wasm.game,PlayerId(0),vec![action.clone()]).unwrap());
    let original=priority_action_ref_for_game(&wasm.game,&action);
    for change in [0,1] {
        let mut reference=original.clone();
        let PriorityActionRef::SpecialAction{action}= &mut reference else {panic!("special action")};
        let claim=private_claim_mut(action).unwrap().as_mut().unwrap();
        if change==0{claim.actor=1;}else{claim.hidden_incarnation=Some(44);}
        assert!(wasm.private_cost_reservation_for_command(&context,&UiCommand::PriorityAction{action_index:None,action_ref:Some(reference)}).is_err());
    }
    let mut info=wasm.game.hidden_card_info(source).unwrap().clone();
    info.public_slot=Some(34);info.public_commitment=Some("public-anchor".into());
    wasm.game.set_hidden_card_info(source,info.clone());
    assert!(wasm.record_verified_public_hidden_opening_input(VerifiedPublicPrivateActionOpening{object_id:source.0,owner:0,slot:34,commitment:info.commitment}).is_err(),"slot from one anchor cannot authenticate another anchor's ciphertext");
    assert!(!wasm.game.is_publicly_revealed_hidden_card(source));
}
#[test]
fn private_source_owner_and_placeholder_peer_replay_same_final_opening_before_public_payment() {
    let _ids=crate::test_id_counter_guard();
    for definition in private_transport_definitions() {
        let (mut owner,_)=manual_payment_fixture();
        let source=owner.game.create_hidden_card_placeholder(PlayerId(0),Zone::Hand,27,"private-action-root-ciphertext".into());
        payment_disclosure_track_hand(&mut owner,source,27);
        owner.game.player_mut(PlayerId(0)).unwrap().mana_pool.add(ManaSymbol::Colorless,1);
        owner.game.player_mut(PlayerId(0)).unwrap().mana_pool.add(ManaSymbol::Red,1);
        let peer_game=owner.game.clone();
        owner.game.reveal_hidden_card_with_definition(source,&definition).unwrap();
        let (mut peer,_)=manual_payment_fixture();peer.game=peer_game;
        assert!(peer.game.is_hidden_card_placeholder(source));
        let index=definition.abilities.iter().position(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(rule)
            if matches!(rule.compiled_model().map(|model|&model.payload),Some(ironsmith_core::StaticAbilityPayload::IntrinsicImmediateAction{..})))).unwrap();
        let action=LegalAction::SpecialAction(ironsmith::special_actions::SpecialAction::PerformIntrinsicAction{source_id:source,ability_index:index});
        let reference=priority_action_ref_for_game(&owner.game,&action);
        let wire=serde_json::to_string(&reference).unwrap();
        for wasm in [&mut owner,&mut peer] {
            let context=DecisionContext::Priority(PriorityContext::new(&wasm.game,PlayerId(0),vec![action.clone()]).unwrap());
            let reference=serde_json::from_str(&wire).unwrap();
            let reservation=wasm.private_cost_reservation_for_command(&context,&UiCommand::PriorityAction{action_index:None,action_ref:Some(reference)}).unwrap().unwrap();
            let payment=private_transport_payment_answer(wasm,&reservation);
            wasm.pending_replay_action=Some(PendingReplayAction{checkpoint:wasm.capture_replay_checkpoint(),root:ReplayRoot::PrivateHandSpecialAction(reservation),nested_answers:vec![payment]});
        }
        private_transport_copy_owner_payment(&owner,&mut peer);
        // Manifest/position proof verification is the existing JS owner's job.
        // Hydrate that verified face into both retained roots before replay.
        peer.game.reveal_hidden_card_with_definition(source,&definition).unwrap();
        for wasm in [&mut owner,&mut peer] {
            let info=wasm.game.hidden_card_info(source).unwrap().clone();
            wasm.record_verified_public_hidden_opening_input(VerifiedPublicPrivateActionOpening{object_id:source.0,owner:0,slot:info.slot,commitment:info.commitment}).unwrap();
            let replay=wasm.pending_replay_action.as_ref().unwrap().clone();
            assert_eq!(replay.checkpoint.game.object(source).unwrap().zone,Zone::Hand);
            wasm.execute_with_replay(&replay.checkpoint,&replay.root,&replay.nested_answers).unwrap();
            assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool.total(),0);
            let stable=replay.checkpoint.game.object(source).unwrap().stable_id;
            let successor=wasm.game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(wasm.game.object(successor).unwrap().zone,Zone::Exile);
            assert_eq!(wasm.game.battlefield.iter().filter(|id|wasm.game.object(**id).is_some_and(|object|object.name=="Cultish Sorcerer")).count(),1);
        }
        assert_eq!(owner.game.player(PlayerId(0)).unwrap().life,peer.game.player(PlayerId(0)).unwrap().life);
        assert_eq!(owner.game.stack.len(),peer.game.stack.len());
    }
}
fn private_transport_orchestrate_definitions(name:&str)->Vec<CardDefinition>{
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../../fixtures/custom_mechanics/msem-orchestrate-full-19.json")).unwrap();
    let row=fixture["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|row|row["name"]==name).unwrap();
    let text=row["executable_text"].as_str().unwrap();
    let (direct,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler_runtime_build::compile_to_runtime_definition(name,text,false));assert!(!loss.is_lossy(),"{}",loss.reasons_text());
    let (artifact,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler_runtime_build::compile_to_artifact(name,text,false));assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,_)=artifact.unwrap();
    let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();restored.validate().unwrap();
    vec![direct.unwrap(),ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
#[test]
fn exact_reytali_private_grant_transports_materialized_card_price_and_peer_authenticates_same_occurrence(){
    let _ids=crate::test_id_counter_guard();
    let providers=private_transport_orchestrate_definitions("Reytali, Song of the Sand");
    let recipients=private_transport_orchestrate_definitions("Return to Form");
    for (provider,recipient) in providers.into_iter().zip(recipients){
        let (mut owner,_)=manual_payment_fixture();
        let donor=owner.game.create_object_from_definition(&provider,PlayerId(0),Zone::Battlefield);
        let source=owner.game.create_hidden_card_placeholder(PlayerId(0),Zone::Hand,28,"reytali-private-recipient".into());
        payment_disclosure_track_hand(&mut owner,source,28);
        // Printed {3}{W}{W}, reduced by {2}: exactly {1}{W}{W}.
        owner.game.player_mut(PlayerId(0)).unwrap().mana_pool.add(ManaSymbol::Colorless,1);
        owner.game.player_mut(PlayerId(0)).unwrap().mana_pool.add(ManaSymbol::White,2);
        let peer_game=owner.game.clone();
        owner.game.reveal_hidden_card_with_definition(source,&recipient).unwrap();
        let action=compute_legal_actions(&owner.game,PlayerId(0)).unwrap().into_iter().find(|action|matches!(action,
            LegalAction::SpecialAction(ironsmith::special_actions::SpecialAction::PrepareGrantedExile{card_id,grant,..})if *card_id==source && grant.source==donor)).unwrap();
        let reference=priority_action_ref_for_game(&owner.game,&action);
        let wire=serde_json::to_string(&reference).unwrap();assert!(wire.contains("grant_identity"));
        let (mut peer,_)=manual_payment_fixture();peer.game=peer_game;
        for wasm in [&mut owner,&mut peer]{
            let context=DecisionContext::Priority(PriorityContext::new(&wasm.game,PlayerId(0),vec![action.clone()]).unwrap());
            let command=UiCommand::PriorityAction{action_index:None,action_ref:Some(serde_json::from_str(&wire).unwrap())};
            let reservation=wasm.private_cost_reservation_for_command(&context,&command).unwrap().unwrap();
            assert_eq!(reservation.cost().mana_cost(),Some(&ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1),ManaSymbol::White,ManaSymbol::White])),"wire carries resolved card-derived price, not a placeholder-dependent dynamic lookup");
            if wasm.game.is_hidden_card_placeholder(source){
                let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
                assert!(matches!(stage_private_action(&wasm.game,&reservation,&mut dm).unwrap(),PrivateActionStage::AwaitingAuthenticatedOpening),"placeholder must pay the same concrete three-mana price, not a symbolic zero-price source lookup");
                assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool.total(),3);
            }
            let payment=private_transport_payment_answer(wasm,&reservation);
            wasm.pending_replay_action=Some(PendingReplayAction{checkpoint:wasm.capture_replay_checkpoint(),root:ReplayRoot::PrivateHandSpecialAction(reservation),nested_answers:vec![payment]});
        }
        private_transport_copy_owner_payment(&owner,&mut peer);
        peer.game.reveal_hidden_card_with_definition(source,&recipient).unwrap();
        for wasm in [&mut owner,&mut peer]{
            let info=wasm.game.hidden_card_info(source).unwrap().clone();
            wasm.record_verified_public_hidden_opening_input(VerifiedPublicPrivateActionOpening{object_id:source.0,owner:0,slot:info.slot,commitment:info.commitment}).unwrap();
            let replay=wasm.pending_replay_action.as_ref().unwrap().clone();
            wasm.execute_with_replay(&replay.checkpoint,&replay.root,&replay.nested_answers).unwrap();
            assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool.total(),0);
            assert_eq!(wasm.game.effect_store.delayed_triggers.len(),1);
            let stable=replay.checkpoint.game.object(source).unwrap().stable_id;
            assert_eq!(wasm.game.object(wasm.game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);
        }
    }
}
#[test]
fn authenticated_private_action_body_pause_keeps_exact_pending_world_without_publishing_price_twice(){
    let _ids=crate::test_id_counter_guard();
    let definition=ironsmith_compiler_runtime_build::compile_to_runtime_definition("Private action body pause","Type: Sorcery\nAny time you could cast a sorcery, you may pay {1}. If you do, take a special action. If you do, exile this card. You may gain 1 life. Take this action only while this card is in your hand.",false).unwrap();
    let (mut wasm,_)=manual_payment_fixture();
    let (source,index)=private_transport_source(&mut wasm,&definition);
    wasm.game.player_mut(PlayerId(0)).unwrap().mana_pool.add(ManaSymbol::Colorless,1);
    let occurrence=PrivateActionOccurrence::Intrinsic{ability_index:index};
    let cost=private_action_cost(&wasm.game,PlayerId(0),source,&occurrence).unwrap();
    let reservation=reserve_private_action_with_incarnation(&wasm.game,PlayerId(0),source,Some(0),occurrence,cost).unwrap();
    let payment=private_transport_payment_answer(&wasm,&reservation);
    wasm.pending_replay_action=Some(PendingReplayAction{checkpoint:wasm.capture_replay_checkpoint(),root:ReplayRoot::PrivateHandSpecialAction(reservation),nested_answers:vec![payment]});
    let info=wasm.game.hidden_card_info(source).unwrap().clone();
    wasm.record_verified_public_hidden_opening_input(VerifiedPublicPrivateActionOpening{object_id:source.0,owner:0,slot:info.slot,commitment:info.commitment}).unwrap();
    let replay=wasm.pending_replay_action.as_ref().unwrap().clone();
    let stable=replay.checkpoint.game.object(source).unwrap().stable_id;
    assert!(matches!(wasm.execute_with_replay(&replay.checkpoint,&replay.root,&replay.nested_answers).unwrap(),ReplayOutcome::NeedsDecision(DecisionContext::Boolean(_))));
    assert_eq!(wasm.game.object(source).unwrap().zone,Zone::Hand,"incomplete body never publishes isolated price/program");
    assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool.total(),1);
    let exact=wasm.pending_decision_game.as_deref().expect("authenticated body keeps exact candidate world");
    assert_eq!(exact.object(exact.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);
    let mut answers=replay.nested_answers;answers.push(ReplayDecisionAnswer::Boolean(true));
    wasm.execute_with_replay(&replay.checkpoint,&replay.root,&answers).unwrap();
    assert_eq!(wasm.game.player(PlayerId(0)).unwrap().mana_pool.total(),0);
    assert_eq!(wasm.game.player(PlayerId(0)).unwrap().life,21);
    assert_eq!(wasm.game.object(wasm.game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);
}
