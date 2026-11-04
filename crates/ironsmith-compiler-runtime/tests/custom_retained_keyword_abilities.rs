//! Complete retained keyword payloads, not reconstructed marker abilities.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,Effect,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith::static_abilities::StaticAbilityId;
const A:PlayerId=PlayerId(0);
fn definition(name:&str,text:&str)->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap()}
fn snapshot(game:&GameState,id:ironsmith::ObjectId)->ironsmith::snapshot::ObjectSnapshot{
    ironsmith::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(game.object(id).unwrap(),game).unwrap()
}
fn static_payloads(game:&mut GameState,id:ironsmith::ObjectId,kind:StaticAbilityId)->Vec<ironsmith::static_abilities::StaticAbility>{
    game.refresh_continuous_state().unwrap();game.current_abilities(id).unwrap_or_default().into_iter().filter_map(|ability|match ability.kind{ironsmith::ability::AbilityKind::Static(ability)if ability.id()==kind=>Some(ability),_=>None}).collect()
}
fn card()->serde_json::Value{
    let f:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-accomplice-source-complete.json")).unwrap();
    f["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]=="The Masked Benefactor").unwrap().clone()
}
#[test]
fn actual_masked_benefactor_keeps_complete_old_protections_after_donor_reentry() {
    for definition in source_support::definitions(&card()).unwrap(){for donor_returns in [false,true]{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let donor_definition=crate::definition("Donor","Type: Creature\nPower/Toughness: 2/2\nFirst strike\nProtection from red\nProtection from blue");
        let donor=game.create_object_from_definition(&donor_definition,A,Zone::Battlefield);
        let donor_snapshot=snapshot(&game,donor);
        let expected:Vec<_>=donor_snapshot.abilities.iter().filter_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Static(ability)if ability.id()==StaticAbilityId::Protection=>Some(ability.clone()),_=>None}).collect();assert_eq!(expected.len(),2);
        let event=ironsmith::events::NamedActionEvent::new("accomplice association",A,source,snapshot(&game,source),vec![donor_snapshot],1,ironsmith::events::cause::EventCause::effect());
        game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new_with_provenance(event,Default::default()));
        let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
        assert_eq!(queue.entries.len(),1);ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
        let new_donor=if donor_returns{let hand=game.move_object(donor,Zone::Hand,ironsmith::events::cause::EventCause::effect()).unwrap();game.move_object(hand,Zone::Battlefield,ironsmith::events::cause::EventCause::effect()).unwrap()}else{donor};
        *game.object_mut(new_donor).unwrap().abilities_mut()=vec![ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::trample())];game.invalidate_continuous_state_public();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
        assert_eq!(static_payloads(&mut game,source,StaticAbilityId::Protection),expected);
        assert_eq!(static_payloads(&mut game,source,StaticAbilityId::FirstStrike).len(),1);
        assert!(static_payloads(&mut game,source,StaticAbilityId::Trample).is_empty(),"new donor characteristics cannot replace the completion snapshot");
        assert_eq!(game.object(source).unwrap().counters.get(&ironsmith::CounterType::PlusOnePlusOne),Some(&1));
    }}
}
#[test]
fn historical_copy_requires_native_ability_evidence_but_accepts_authoritative_empty_packet() {
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    let source=game.create_object_from_definition(&definition("Recipient","Type: Creature\nPower/Toughness: 2/2"),A,Zone::Battlefield);
    let donor=game.create_object_from_definition(&definition("Donor","Type: Creature\nPower/Toughness: 2/2\nProtection from red"),A,Zone::Battlefield);
    let mut grant=ironsmith::effects::GrantAbilitiesTargetEffect::new(ChooseSpec::Source,Vec::new(),ironsmith::effect::Until::EndOfTurn);
    grant.keyword_from=Some(ironsmith_core::KeywordAbilitySource{source:ChooseSpec::Tagged("donor".into()),keywords:vec![StaticAbilityId::Protection],last_known:true});
    let effect=Effect::new(grant);let mut ctx=EffectContext::new_default(source,A);
    assert!(matches!(execute_effect(&mut game,&effect,&mut ctx),Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
    ctx.set_tagged_objects("donor",vec![]);execute_effect(&mut game,&effect,&mut ctx).unwrap();assert!(static_payloads(&mut game,source,StaticAbilityId::Protection).is_empty());
    let mut public_claim=snapshot(&game,donor);public_claim.ability_origins=None;ctx.set_tagged_objects("donor",vec![public_claim]);
    assert!(matches!(execute_effect(&mut game,&effect,&mut ctx),Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
    assert!(static_payloads(&mut game,source,StaticAbilityId::Protection).is_empty());
    let mut filter=ironsmith::target::ObjectFilter::creature().match_tagged("donor",ironsmith::target::TaggedOpbjectRelation::IsTaggedObject);
    filter.static_abilities.push(StaticAbilityId::Flying);
    let mut qualified=ironsmith::effects::GrantAbilitiesTargetEffect::new(ChooseSpec::Source,Vec::new(),ironsmith::effect::Until::EndOfTurn);
    qualified.keyword_from=Some(ironsmith_core::KeywordAbilitySource{source:ChooseSpec::Object(filter),keywords:vec![StaticAbilityId::Protection],last_known:true});
    let qualified=Effect::new(qualified);
    assert!(matches!(execute_effect(&mut game,&qualified,&mut ctx),Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))),"missing executable evidence cannot prove a false ability qualifier and silently erase its donor");
    ctx.set_tagged_objects("donor",vec![]);execute_effect(&mut game,&qualified,&mut ctx).unwrap();
}
#[test]
fn typed_historical_grant_retains_distinct_protection_instances_and_only_requested_keywords() {
    let text="Type: Instant\nTarget creature gains protection until end of turn if that creature had protection.";
    for definition in source_support::definitions(&serde_json::json!({"name":"Payload copy","executable_text":text})).unwrap(){
        let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
        fn keywords(effect:&Effect)->Option<Vec<StaticAbilityId>> {
            if let Some(grant)=effect.downcast_ref::<ironsmith::effects::GrantAbilitiesTargetEffect>(){return grant.keyword_from.as_ref().map(|source|source.keywords.clone());}
            let mut found=None;effect.visit_child_effects(&mut |child|{if found.is_none(){found=keywords(child);}});found
        }
        let found=effects.iter().find_map(keywords).expect("typed existing grant family");assert_eq!(found,vec![StaticAbilityId::Protection]);
    }
}
