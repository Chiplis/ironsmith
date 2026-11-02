//! Generic activation tap-payment parity for equivalent custom reminders.
use ironsmith::{GameState,PlayerId,Zone};
use ironsmith::mana::{ManaCost,ManaSymbol};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
fn strict_artifact(name:&str,text:&str)->(CompiledCardArtifact,ironsmith::CardDefinition) {
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());result.unwrap()
}

#[test]
fn flow_creature_taps_pay_only_generic_activation_mana() {
    let text="Type: Creature\nPower/Toughness: 2/3\n{2}{W}: Draw a card. For each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that mana.";
    let (artifact,direct)=strict_artifact("Scoped helper",text);
    let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
        assert!(format!("{:?}",definition.abilities).contains("creatures_only: true"));
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Tapper","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
        let artifact=ironsmith_compiler_runtime::compile_to_runtime_definition("Artifact","Type: Artifact",false).unwrap();
        game.create_object_from_definition(&artifact,A,Zone::Battlefield);
        game.create_object_from_definition(&creature,PlayerId(1),Zone::Battlefield);
        let request=ironsmith::mana_payment::ManaPaymentRequest::new(A,source,ironsmith::costs::PaymentReason::ActivateAbility,ManaCost::from_symbols(vec![ManaSymbol::Generic(2),ManaSymbol::White]).with_creature_tap_payment());
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White,1);
        assert!(ironsmith::mana_payment::plan_mana_payment(&game,&request).is_err(),"own source is only one eligible creature; artifacts and opposing creatures cannot supply the second tap");
        let tapper=game.create_object_from_definition(&creature,A,Zone::Battlefield);
        assert!(ironsmith::mana_payment::plan_mana_payment(&game,&request).is_ok());
        game.tap(tapper);
        assert!(ironsmith::mana_payment::plan_mana_payment(&game,&request).is_err());
        game.untap(tapper);
        game.player_mut(A).unwrap().mana_pool.empty();
        game.create_object_from_definition(&creature,A,Zone::Battlefield);
        assert!(ironsmith::mana_payment::plan_mana_payment(&game,&request).is_err(),"extra creature cannot pay a white pip");
    }
}
#[test]
fn revered_grants_only_to_other_controlled_counter_bearers() {
    use ironsmith::object::CounterType;
    use ironsmith_core::StaticAbilityId;
    let text="Type: Creature\nPower/Toughness: 3/3\nFlying\nEach other creature you control with counters on it has “Trample”.\nTrample";
    let (artifact,direct)=strict_artifact("Positional grant",text);
    let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
        let recipient=game.create_object_from_definition(&creature,A,Zone::Battlefield);
        let opponent=game.create_object_from_definition(&creature,PlayerId(1),Zone::Battlefield);
        assert!(game.current_has_static_ability_id(source,StaticAbilityId::Flying));
        assert!(!game.current_has_static_ability_id(recipient,StaticAbilityId::Trample));
        for object in [recipient,opponent] {game.add_counters(object,CounterType::Level,1).unwrap();}
        assert!(game.current_has_static_ability_id(recipient,StaticAbilityId::Trample));
        assert!(!game.current_has_static_ability_id(opponent,StaticAbilityId::Trample));
        assert!(!game.current_has_static_ability_id(recipient,StaticAbilityId::Flying),"ability above positional declaration is not shared");
        game.object_mut(recipient).unwrap().counters.clear();
        assert!(!game.current_has_static_ability_id(recipient,StaticAbilityId::Trample));
    }
}
#[test]
fn final_chapter_removes_lore_before_state_based_actions() {
    use ironsmith::object::CounterType;
    use ironsmith::rules::state_based::{check_state_based_actions,StateBasedAction};
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::game_loop::{add_saga_lore_counters,put_triggers_on_stack_with_dm,resolve_stack_entry_with};
    let text="Type: Enchantment — Saga\nAs this Saga's final chapter ability resolves, remove all lore counters from it.\nI — You gain 1 life.\nII — You gain 2 life.";
    let (artifact,direct)=strict_artifact("Repeating saga",text);
    let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {for mode in 0..3 {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        game.add_counters(source,CounterType::Lore,1).unwrap();
        let mut queue=ironsmith::triggers::TriggerQueue::new();
        add_saga_lore_counters(&mut game,&mut queue).unwrap();
        put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.stack.len(),1);
        if mode==1 {
            std::sync::Arc::make_mut(&mut game.object_mut(source).unwrap().abilities).retain(|ability| !matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(ability) if matches!(ability.keyword_marker_mechanic(),Some(ironsmith_core::KeywordMarkerMechanic::FinalChapterResolutionRemovesCounters{..}))));
            game.refresh_continuous_state().unwrap();
        }
        if mode==2 {game.stack.clear();} else {resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
        assert_eq!(game.counter_count(source,CounterType::Lore),if mode==0 {0}else{2},"only current static ability at successful chapter resolution removes counters");
        assert_eq!(check_state_based_actions(&game).iter().any(|action|matches!(action,StateBasedAction::SagaSacrifice(id) if *id==source)),mode!=0);
    }}
}
#[test]
fn artifact_tap_payment_excludes_nonartifact_creatures() {
    let text="Type: Artifact\n{2}{B}: Draw a card. For each mana in this ability's activation cost, you may tap an untapped artifact you control rather than pay that mana. Activate only as a sorcery.";
    let (artifact,direct)=strict_artifact("Artifact cooperation",text);
    let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
        assert!(format!("{:?}",definition.abilities).contains("artifacts_only: true"));
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Nonartifact creature","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
        let artifact=ironsmith_compiler_runtime::compile_to_runtime_definition("Artifact tapper","Type: Artifact",false).unwrap();
        game.create_object_from_definition(&creature,A,Zone::Battlefield);
        game.create_object_from_definition(&artifact,PlayerId(1),Zone::Battlefield);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black,1);
        let request=ironsmith::mana_payment::ManaPaymentRequest::new(A,source,ironsmith::costs::PaymentReason::ActivateAbility,ManaCost::from_symbols(vec![ManaSymbol::Generic(2),ManaSymbol::Black]).with_artifact_tap_payment());
        assert!(ironsmith::mana_payment::plan_mana_payment(&game,&request).is_err());
        let tapper=game.create_object_from_definition(&artifact,A,Zone::Battlefield);
        assert!(ironsmith::mana_payment::plan_mana_payment(&game,&request).is_ok());
        game.tap(tapper);
        assert!(ironsmith::mana_payment::plan_mana_payment(&game,&request).is_err());
        game.untap(tapper);
        game.player_mut(A).unwrap().mana_pool.empty();
        game.create_object_from_definition(&artifact,A,Zone::Battlefield);
        assert!(ironsmith::mana_payment::plan_mana_payment(&game,&request).is_err(),"artifacts do not pay black mana");
    }
}

#[test]
fn zero_width_space_in_source_is_layout_whitespace() {
 let (_,definition)=strict_artifact("Layout witness","Type: Enchantment — Saga\nAs this Saga's final chapter ability resolves, remove all lore counters from it.\nI — You gain 1 life.\nII — You gain 2 life.\n\u{200b}");
 assert!(format!("{:?}",definition.abilities).contains("FinalChapterResolutionRemovesCounters"));
}
