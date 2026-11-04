//! Actual Oath producers, supplied Aura definitions, and retained completion receipts.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::effects::ResolvedTarget;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn fixture()->serde_json::Value{serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-oaths-source-complete.json")).unwrap()}
fn routes(name:&str)->[CardDefinition;2]{let f=fixture();let c=f["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();source_support::definitions(c).unwrap_or_else(|error|panic!("{name}: {error}"))}
fn body()->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Host","Type: Creature — Knight\nPower/Toughness: 2/2",false).unwrap()}
fn game()->GameState{GameState::new(vec!["A".into(),"B".into()],20)}
fn trigger(game:&GameState,source:ObjectId,index:usize)->Vec<Effect>{game.object(source).unwrap().abilities.iter().filter_map(|a|match &a.kind{AbilityKind::Triggered(t)=>Some(t.effects.all_effects_owned()),_=>None}).nth(index).unwrap()}
fn run(game:&mut GameState,source:ObjectId,actor:PlayerId,effects:Vec<Effect>,target:Option<ObjectId>)->Vec<ironsmith::triggers::TriggerEvent>{
 let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(),game);
 let mut dm=SelectFirstDecisionMaker;let mut ctx=EffectContext::new_default(source,actor).with_source_snapshot(snapshot).with_decision_maker(&mut dm);
 if let Some(target)=target{ctx=ctx.with_targets(vec![ResolvedTarget::Object(target)]);}
 execute_effect(game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ctx).unwrap().events
}
fn auras(game:&GameState)->Vec<ObjectId>{game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().has_subtype(ironsmith::Subtype::Aura)).collect()}
fn create_oath(game:&mut GameState,definition:&CardDefinition,host:ObjectId)->ObjectId{
 let source=game.create_object_from_definition(definition,A,Zone::Battlefield);let effects=trigger(game,source,0);let old=auras(game);run(game,source,A,effects,Some(host));*auras(game).iter().find(|id|!old.contains(id)).expect("the real source creates its supplied Aura")
}
#[test]
fn every_actual_operative_source_is_strict_on_independent_native_and_artifact_routes(){let f=fixture();let cards=f["mechanics"][0]["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),21);for card in cards{for d in source_support::definitions(card).unwrap_or_else(|error|panic!("{}: {error}",card["name"])){assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&d));}}}
#[test]
fn opposing_recipient_controls_each_real_aura_and_multiple_oaths_are_not_roles(){for definition in routes("Pactwhisper Demon"){
 let mut game=game();let host=game.create_object_from_definition(&body(),B,Zone::Battlefield);
 for _ in 0..3{let aura=create_oath(&mut game,&definition,host);let object=game.object(aura).unwrap();assert_eq!(game.controller_of(object),B);assert_eq!(object.owner,B);assert_eq!(object.attached_to,Some(ironsmith::object::AttachmentTarget::Object(host)));assert!(!object.has_subtype(ironsmith::Subtype::Role));assert!(object.subtypes.iter().any(|s|s.to_string().eq_ignore_ascii_case("oath")));}
 assert_eq!(auras(&game).len(),3);
}}
#[test]
fn secrets_draw_sacrifice_and_fulfillment_retain_the_knight_after_aura_departure(){for definition in routes("Pactwhisper Demon"){
 let mut game=game();let host=game.create_object_from_definition(&body(),B,Zone::Battlefield);game.create_object_from_definition(&body(),B,Zone::Library);let aura=create_oath(&mut game,&definition,host);let stable=game.object(aura).unwrap().stable_id;
 let effects=trigger(&game,aura,0);let events=run(&mut game,aura,B,effects,None);
 assert_eq!(game.player(B).unwrap().hand.len(),1);assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);
 let completed:Vec<_>=events.iter().filter_map(|e|e.downcast::<ironsmith::events::NamedActionEvent>()).collect();assert_eq!(completed.len(),1);let receipt=completed[0];assert_eq!(receipt.label,"fulfill oath");assert_eq!(receipt.player,B);assert_eq!(receipt.source,aura);assert_eq!(receipt.objects.len(),1);assert_eq!(receipt.objects[0].object_id,host);assert!(receipt.objects[0].subtypes.contains(&ironsmith::Subtype::Knight));
}}
#[test]
fn valor_requires_combat_damage_to_a_player_by_its_exact_enchanted_creature(){for definition in routes("By Blood and Fire"){
 for route in ["combat-player","noncombat-player","combat-creature","other-creature"]{
 let mut game=game();let host=game.create_object_from_definition(&body(),A,Zone::Battlefield);let other=game.create_object_from_definition(&body(),A,Zone::Battlefield);let source=game.create_object_from_definition(&definition,A,Zone::Stack);
 let mut dm=SelectFirstDecisionMaker;let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Player(B),ResolvedTarget::Object(host)]).with_decision_maker(&mut dm);execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(definition.spell_effect.as_ref().unwrap().all_effects_owned())),&mut ctx).unwrap();let aura=*auras(&game).first().unwrap();
 let dealer=if route=="other-creature"{other}else{host};let combat=route!="noncombat-player";let target=if route=="combat-creature"{ironsmith::events::DamageTarget::Object(other)}else{ironsmith::events::DamageTarget::Player(B)};
 let event=ironsmith::triggers::TriggerEvent::new(ironsmith::events::DamageEvent::with_cause(dealer,target,1,combat,if combat{ironsmith::events::EventCause::combat_damage(dealer)}else{ironsmith::events::EventCause::effect()}),Default::default());
 let matches=ironsmith::triggers::check_triggers(&game,&event).into_iter().filter(|e|e.source==aura).count();assert_eq!(matches,usize::from(route=="combat-player"),"{route}");
 }
}}
#[test]
fn secrets_requires_the_enchanted_attacker_and_two_other_declared_attackers(){for definition in routes("Pactwhisper Demon"){
 for (host_attacks,others,expected) in [(true,0,false),(true,1,false),(true,2,true),(false,3,false)]{
 let mut game=game();let host=game.create_object_from_definition(&body(),B,Zone::Battlefield);let aura=create_oath(&mut game,&definition,host);let mut attackers=Vec::new();if host_attacks{attackers.push(host);}for _ in 0..others{attackers.push(game.create_object_from_definition(&body(),B,Zone::Battlefield));}
 let declaration:std::sync::Arc<[ironsmith::combat_state::AttackerInfo]>=attackers.iter().map(|id|ironsmith::combat_state::AttackerInfo{creature:*id,target:ironsmith::combat_state::AttackTarget::Player(A)}).collect::<Vec<_>>().into();
 let mut hits=0;for attacker in &attackers{let event=ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::with_total_attackers(*attacker,ironsmith::triggers::AttackEventTarget::Player(A),attackers.len()).with_declared_attackers(declaration.clone()),Default::default());hits+=ironsmith::triggers::check_triggers(&game,&event).into_iter().filter(|e|e.source==aura).count();}assert_eq!(hits,usize::from(expected));
 }
}}
#[test]
fn actual_hulking_fleshdebter_gives_haste_to_the_zombie_not_the_new_aura(){for definition in routes("Hulking Fleshdebter"){
 let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let effects=trigger(&game,source,0);run(&mut game,source,A,effects,None);
 let zombie=*game.battlefield.iter().find(|id|game.object(**id).unwrap().has_subtype(ironsmith::Subtype::Zombie)).unwrap();let aura=*auras(&game).first().unwrap();assert_eq!(game.object(aura).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(zombie)));
 assert!(game.current_has_static_ability_id(zombie,ironsmith::static_abilities::StaticAbilityId::Haste));assert!(!game.current_has_static_ability_id(aura,ironsmith::static_abilities::StaticAbilityId::Haste));
}}

#[test]
fn real_general_and_knight_observers_consume_one_completed_oath_receipt(){
 let secret=routes("Pactwhisper Demon");let blood=routes("Bloodstained Crusade");let rhysta=routes("Rhysta, Bladebound");
 for route in 0..2{for knight in [false,true]{let mut game=game();let host=game.create_object_from_definition(&body(),B,Zone::Battlefield);if !knight{game.object_mut(host).unwrap().subtypes=vec![ironsmith::Subtype::Goblin].into();}game.create_object_from_definition(&body(),B,Zone::Library);let aura=create_oath(&mut game,&secret[route],host);let blood_source=game.create_object_from_definition(&blood[route],B,Zone::Battlefield);let wrong_player=game.create_object_from_definition(&blood[route],A,Zone::Battlefield);let general_source=game.create_object_from_definition(&rhysta[route],B,Zone::Battlefield);let effects=trigger(&game,aura,0);
 // The queued ability still belongs to B if the Aura subsequently changes controller.
 game.set_current_controller(aura,A).unwrap();let events=run(&mut game,aura,B,effects,None);
 let receipt=events.iter().find(|event|event.downcast::<ironsmith::events::NamedActionEvent>().is_some()).unwrap();let matches=ironsmith::triggers::check_triggers(&game,receipt);assert_eq!(matches.iter().filter(|entry|entry.source==blood_source).count(),usize::from(knight));assert_eq!(matches.iter().filter(|entry|entry.source==wrong_player).count(),0);assert_eq!(matches.iter().filter(|entry|entry.source==general_source).count(),1);
 }}
}

fn move_to(game:&mut GameState,id:ObjectId,zone:Zone){let mut ctx=EffectContext::new_default(id,A);execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(id),zone,false)),&mut ctx).unwrap();}
#[test]
fn attachment_prelude_uses_exact_latest_host_lki_and_never_old_host_or_reentered_card(){
 for definition in routes("Pactwhisper Demon"){for reattach in [false,true]{
 let mut game=game();let old=game.create_object_from_definition(&body(),B,Zone::Battlefield);let aura=create_oath(&mut game,&definition,old);let old_snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(old).unwrap(),&game);
 let host=if reattach{let id=game.create_object_from_definition(&body(),B,Zone::Battlefield);assert!(game.attach_object_to_target(aura,ironsmith::object::AttachmentTarget::Object(id)));id}else{old};
 game.object_mut(host).unwrap().subtypes=vec![ironsmith::Subtype::Goblin].into();
 let aura_snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(aura).unwrap(),&game);let stable=game.object(host).unwrap().stable_id;
 move_to(&mut game,aura,Zone::Graveyard);move_to(&mut game,host,Zone::Graveyard);let grave=game.find_object_by_stable_id(stable).unwrap();move_to(&mut game,grave,Zone::Battlefield);let reentered=game.find_object_by_stable_id(stable).unwrap();assert_ne!(host,reentered);
 let mut ctx=EffectContext::new_default(aura,B).with_source_snapshot(aura_snapshot);ctx.set_tagged_objects("enchanted",vec![old_snapshot]);execute_effect(&mut game,&Effect::tag_attached_to_source("enchanted"),&mut ctx).unwrap();
 let packet=ctx.get_tagged_all("enchanted").unwrap();assert_eq!(packet.len(),1);assert_eq!(packet[0].object_id,host);assert!(packet[0].subtypes.contains(&ironsmith::Subtype::Goblin));assert!(!packet[0].subtypes.contains(&ironsmith::Subtype::Knight));assert_ne!(packet[0].object_id,reentered);
 }}
}
#[test]
fn known_detachment_clears_an_older_host_packet_instead_of_fulfilling_for_it(){for definition in routes("Pactwhisper Demon"){
 let mut game=game();let host=game.create_object_from_definition(&body(),B,Zone::Battlefield);let aura=create_oath(&mut game,&definition,host);let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(host).unwrap(),&game);game.detach_object_from_current_target(aura);
 let mut ctx=EffectContext::new_default(aura,B);ctx.set_tagged_objects("enchanted",vec![snapshot]);execute_effect(&mut game,&Effect::tag_attached_to_source("enchanted"),&mut ctx).unwrap();assert!(ctx.get_tagged_all("enchanted").unwrap().is_empty());
}}

#[test]
fn an_opposing_recipient_chooses_the_mode_and_owns_the_chosen_aura(){
 let f=fixture();let catalog=f["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|c|c["name"]=="Llech Crann Knight").unwrap();
 let input=serde_json::json!({"name":"Recipient choice","executable_text":"Type: Sorcery\nChoose target creature. Its controller chooses one of the following: [Its controller creates a colorless artifact token named Oath of Secrets attached to that creature] or [Its controller creates a colorless artifact token named Oath of Valor attached to that creature].","custom_token_definitions":catalog["custom_token_definitions"],"custom_subtypes":catalog["custom_subtypes"]});
 struct Valor;impl DecisionMaker for Valor{fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{assert_eq!(ctx.player,B);assert_eq!(ctx.options.len(),2);vec![ctx.options[1].index]}}
 for definition in source_support::definitions(&input).unwrap(){let mut game=game();let host=game.create_object_from_definition(&body(),B,Zone::Battlefield);let source=game.create_object_from_definition(&definition,A,Zone::Stack);let mut dm=Valor;let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(host)]).with_decision_maker(&mut dm);execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(definition.spell_effect.as_ref().unwrap().all_effects_owned())),&mut ctx).unwrap();let aura=auras(&game);assert_eq!(aura.len(),1);let aura=game.object(aura[0]).unwrap();assert_eq!(aura.name,"Oath of Valor");assert_eq!(aura.owner,B);assert_eq!(game.controller_of(aura),B);assert_eq!(aura.attached_to,Some(ironsmith::object::AttachmentTarget::Object(host)));}
}

#[test]
fn qualified_named_reference_reads_the_retained_packet_not_later_live_characteristics(){
 let mut game=game();let source=game.create_object_from_definition(&body(),A,Zone::Battlefield);let host=game.create_object_from_definition(&body(),A,Zone::Battlefield);let captured=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(host).unwrap(),&game);game.object_mut(host).unwrap().subtypes=vec![ironsmith::Subtype::Goblin].into();
 let filter=ironsmith::target::ObjectFilter::creature().match_tagged("enchanted",ironsmith::target::TaggedOpbjectRelation::IsTaggedObject);
 let effect=Effect::new(ironsmith_core::PerformNamedActionEffect{label:"qualified completion".into(),performer:None,objects:Some(ironsmith::target::ChooseSpec::Object(filter)),prior_objects:None});let mut ctx=EffectContext::new_default(source,A);ctx.set_tagged_objects("enchanted",vec![captured]);let result=execute_effect(&mut game,&effect,&mut ctx).unwrap();let event=result.events.iter().find_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).unwrap();assert_eq!(event.objects.len(),1);assert!(event.objects[0].subtypes.contains(&ironsmith::Subtype::Knight));assert!(!event.objects[0].subtypes.contains(&ironsmith::Subtype::Goblin));
}

#[test]
fn live_attachment_prelude_replaces_seeded_old_and_duplicate_packets_after_reattachment(){
 for definition in routes("Pactwhisper Demon"){for reattach in [false,true]{
  let mut game=game();let old=game.create_object_from_definition(&body(),B,Zone::Battlefield);let aura=create_oath(&mut game,&definition,old);
  let old_snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(old).unwrap(),&game);
  let trigger_source=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(aura).unwrap(),&game);
  let host=if reattach{let current=game.create_object_from_definition(&body(),B,Zone::Battlefield);assert!(game.attach_object_to_target(aura,ironsmith::object::AttachmentTarget::Object(current)));current}else{old};
  game.object_mut(host).unwrap().subtypes=vec![ironsmith::Subtype::Goblin].into();
  let mut ctx=EffectContext::new_default(aura,B).with_source_snapshot(trigger_source);
  ctx.set_tagged_objects("enchanted",vec![old_snapshot.clone(),old_snapshot]);
  for _ in 0..2 {
   execute_effect(&mut game,&Effect::tag_attached_to_source("enchanted"),&mut ctx).unwrap();
   let packet=ctx.get_tagged_all("enchanted").unwrap();assert_eq!(packet.len(),1,"prelude rebinding is singleton and idempotent");assert_eq!(packet[0].object_id,host);assert!(packet[0].subtypes.contains(&ironsmith::Subtype::Goblin));assert!(!packet[0].subtypes.contains(&ironsmith::Subtype::Knight));
  }
  let filter=ironsmith::target::ObjectFilter::creature().match_tagged("enchanted",ironsmith::target::TaggedOpbjectRelation::IsTaggedObject);
  let completion=Effect::new(ironsmith_core::PerformNamedActionEffect{label:"fulfill oath".into(),performer:None,objects:Some(ironsmith::target::ChooseSpec::Object(filter)),prior_objects:None});
  let outcome=execute_effect(&mut game,&completion,&mut ctx).unwrap();
  let receipt=outcome.events.iter().find_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).unwrap();
  assert_eq!(receipt.player,B);assert_eq!(receipt.objects.len(),1);assert_eq!(receipt.objects[0].object_id,host);assert!(receipt.objects[0].subtypes.contains(&ironsmith::Subtype::Goblin));assert!(!receipt.objects[0].subtypes.contains(&ironsmith::Subtype::Knight));
 }}
}
