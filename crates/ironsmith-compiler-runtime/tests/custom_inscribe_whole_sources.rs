//! Whole operative Inscribe corpus: frozen26 plus additive13, never a representative-only proof.
mod custom_final_next_wave_support;
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::effects::{EffectContext,execute_effect};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn inputs()->Vec<serde_json::Value>{[("IRONSMITH_INSCRIBE_SOURCE_FIXTURE","msem-inscribe-all-sources-candidate.json"),("IRONSMITH_INSCRIBE_ADDITIONAL_FIXTURE","msem-inscribe-additional-sources.json"),("IRONSMITH_INSCRIBE_OBSERVER_FIXTURE","msem-inscribe-observer-sources.json")].into_iter().map(|(env,name)|{let path=std::env::var(env).unwrap_or_else(|_|format!("{}/../../fixtures/custom_mechanics/{name}",env!("CARGO_MANIFEST_DIR")));serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()}).collect()}
fn definitions(name:&str)->[CardDefinition;2]{let fixtures=inputs();let card=fixtures.iter().flat_map(|f|f["mechanics"].as_array().unwrap()).flat_map(|r|r["representative_cards"].as_array().unwrap()).find(|c|c["name"]==name).unwrap();let mut faces=custom_final_next_wave_support::linked_definitions(card).unwrap();assert_eq!(faces.len(),1);faces.remove(0)}
fn ordinary(name:&str,text:&str)->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap()}
fn game()->GameState{let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;g}
struct HostChoice{host:ObjectId}
impl DecisionMaker for HostChoice{
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{false}
 fn decide_objects(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{if c.candidates.iter().any(|candidate|candidate.id==self.host&&candidate.legal){vec![self.host]}else{SelectFirstDecisionMaker.decide_objects(g,c)}}
}
fn announce(g:&mut GameState,source:ObjectId,dm:&mut HostChoice){use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};let action=compute_legal_actions(g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source)).expect("actual source's inscribe activation");let mut q=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut progress=apply_priority_response_with_dm(g,&mut q,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(context)=progress else{panic!("{progress:?}");};progress=apply_decision_context_with_dm(g,&mut q,&mut state,&context,dm).unwrap();}assert!(!state.has_pending_action());}
fn activate(g:&mut GameState,source:ObjectId,dm:&mut HostChoice){announce(g,source,dm);ironsmith::game_loop::resolve_stack_entry_with(g,dm).unwrap();}
fn move_to(g:&mut GameState,id:ObjectId,zone:Zone)->ObjectId{let stable=g.object(id).unwrap().stable_id;execute_effect(g,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(id),zone,false)),&mut EffectContext::new_default(id,A)).unwrap();g.find_object_by_stable_id(stable).unwrap()}
fn has(g:&GameState,source:ObjectId,id:ironsmith::static_abilities::StaticAbilityId)->bool{g.current_abilities(source).unwrap_or_default().iter().any(|a|matches!(&a.kind,AbilityKind::Static(a)if a.id()==id))}
#[test]
fn all_fifty_complete_source_bodies_strictly_compile_native_and_artifact(){let fixtures=inputs();assert_eq!(fixtures[0]["mechanics"].as_array().unwrap().iter().map(|r|r["representative_cards"].as_array().unwrap().len()).sum::<usize>(),26);assert_eq!(fixtures[1]["mechanics"].as_array().unwrap().iter().map(|r|r["representative_cards"].as_array().unwrap().len()).sum::<usize>(),13);let mut keys=std::collections::BTreeSet::new();let mut failures=Vec::new();let mut passed=0;for f in &fixtures{for row in f["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){let key=card["source_card_key"].as_str().unwrap();assert!(keys.insert(key.to_owned()),"duplicate physical source {key}");let label=format!("{} / {key} / {}",row["id"],card["name"]);let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||custom_final_next_wave_support::linked_definitions(card)));match result{Ok(Ok(faces))=>{assert_eq!(faces.len(),card["source_faces"].as_array().unwrap().len());passed+=faces.len();for pair in faces{for definition in pair{assert_eq!(definition.card.name,card["name"].as_str().unwrap());}}},Ok(Err(e))=>failures.push(format!("{label}: {e}")),Err(_)=>failures.push(format!("{label}: compiler panic (see exact diagnostic above)"))}}}}assert_eq!(keys.len(),50);assert!(failures.is_empty(),"{} whole-source failures ({passed} strictly passed):\n{}",failures.len(),failures.join("\n"));assert_eq!(passed,50);}
#[test]
fn additional_passive_sources_use_exact_relations_and_end_at_host_or_source_incarnation_change(){use ironsmith::static_abilities::StaticAbilityId;for (name,bonus,ability) in [("Voice of Sheraku",1,StaticAbilityId::Vigilance),("Fleeting Thoughts",0,StaticAbilityId::Flying)]{for definition in definitions(name){for depart_source in [false,true]{let mut g=game();let creature=ordinary("Relation host","Type: Creature\nPower/Toughness: 2/2");let host=g.create_object_from_definition(&creature,A,Zone::Battlefield);let unrelated=g.create_object_from_definition(&creature,A,Zone::Battlefield);let source=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(source).unwrap().stable_id;for mana in [ironsmith::mana::ManaSymbol::White,ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Colorless]{g.player_mut(A).unwrap().mana_pool.add(mana,10);}activate(&mut g,source,&mut HostChoice{host});let exiled=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(exiled).unwrap().zone,Zone::Exile);assert_eq!(g.source_collection_objects(exiled,"inscribed host"),vec![host]);assert_eq!(g.current_power(host),Some(2+bonus));assert!(has(&g,host,ability));assert_eq!(g.current_power(unrelated),Some(2));assert!(!has(&g,unrelated,ability));let old=g.object(exiled).unwrap().abilities.clone();g.object_mut(exiled).unwrap().abilities_mut().clear();assert_eq!(g.current_power(host),Some(2));assert!(!has(&g,host,ability));assert_eq!(g.source_collection_objects(exiled,"inscribed host"),vec![host]);g.object_mut(exiled).unwrap().abilities=old;assert!(has(&g,host,ability));if depart_source{let hand=move_to(&mut g,exiled,Zone::Hand);let returned=move_to(&mut g,hand,Zone::Exile);assert_ne!(returned,exiled);assert!(g.source_collection_objects(returned,"inscribed host").is_empty());assert_eq!(g.current_power(host),Some(2));assert!(!has(&g,host,ability));continue;}let hand=move_to(&mut g,host,Zone::Hand);let returned=move_to(&mut g,hand,Zone::Battlefield);assert_ne!(returned,host);assert!(!has(&g,returned,ability));assert!(g.source_collection_objects(exiled,"inscribed host").is_empty());}}}}
#[test]
fn prayer_actual_main_spell_keeps_buff_and_later_death_token_separate_from_inscription_activation(){for definition in definitions("Prayer of Drifting Lights"){let mut g=game();let creature=ordinary("Prayer recipient","Type: Creature\nPower/Toughness: 2/2");let host=g.create_object_from_definition(&creature,A,Zone::Battlefield);let source=g.create_object_from_definition(&definition,A,Zone::Stack);let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(host)]);for effect in definition.spell_effect.as_ref().unwrap().all_effects(){execute_effect(&mut g,effect,&mut ctx).unwrap();}drop(ctx);assert_eq!(g.current_power(host),Some(3));assert!(g.source_collection_objects(source,"inscribed host").is_empty());move_to(&mut g,host,Zone::Graveyard);let mut q=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut g,&mut q,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack(&mut g,&mut q).unwrap();assert_eq!(g.stack.len(),1);ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();let spirits:Vec<_>=g.battlefield.iter().filter(|id|g.object(**id).unwrap().subtypes.contains(&ironsmith::types::Subtype::Spirit)).collect();assert_eq!(spirits.len(),1);assert_eq!(g.current_power(*spirits[0]),Some(2));}}

#[test]
fn kamito_actual_permission_uses_only_designated_occurrence_and_literal_later_hand_exile(){
 for grant in definitions("Kamito Eternalist") {for source_definition in definitions("Voice of Sheraku") {
  let mut g=game();let host=g.create_object_from_definition(&ordinary("Kamito host","Type: Creature\nPower/Toughness: 2/2"),A,Zone::Battlefield);
  let kamito=g.create_object_from_definition(&grant,A,Zone::Battlefield);
  let mut source_definition=source_definition.clone();
  let mut unrelated=source_definition.abilities.iter().find(|ability|matches!(ability.kind,AbilityKind::Activated(_))).unwrap().clone();
  assert!(unrelated.has_designation("inscribe"));unrelated.designations.clear();
  let unrelated_index=source_definition.abilities.len();source_definition.abilities.push(unrelated);
  let source=g.create_object_from_definition(&source_definition,A,Zone::Graveyard);
  for mana in [ironsmith::mana::ManaSymbol::White,ironsmith::mana::ManaSymbol::Colorless]{g.player_mut(A).unwrap().mana_pool.add(mana,10);}
  let actions=compute_legal_actions(&g,A).unwrap();
  assert!(actions.iter().any(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index}if *id==source&&*ability_index!=unrelated_index)));
  assert!(!actions.iter().any(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index}if *id==source&&*ability_index==unrelated_index)),"card-wide keyword identity must not authorize an unrelated occurrence");
  let index=actions.iter().find_map(|action|match action{LegalAction::ActivateAbility{source:id,ability_index}if *id==source&&*ability_index!=unrelated_index=>Some(*ability_index),_=>None}).unwrap();
  let original=g.current_ability(source,index).unwrap();let AbilityKind::Activated(original)=&original.kind else{unreachable!()};
  let captured=ironsmith::activation_zone_permissions::prepare(&g,source,index,original).unwrap();
  assert_eq!(captured.effects,original.effects,"permission must not rewrite the independent resolving program");
  let reveal_zone=|ability:&ironsmith::ability::ActivatedAbility|ability.mana_cost.as_all().unwrap().iter().find_map(|cost|cost.effect_ref().and_then(|effect|effect.downcast_ref::<ironsmith::effects::RevealSourceFromHandEffect>()).map(|reveal|reveal.activation_source_zone));
  assert_eq!(reveal_zone(original),Some(None));assert_eq!(reveal_zone(&captured),Some(Some(Zone::Graveyard)));
  // Permission affects announcement/reveal, not the real zone or the later
  // independent instruction to exile this card FROM HAND.
  announce(&mut g,source,&mut HostChoice{host});assert_eq!(g.stack.len(),1);
  assert_eq!(g.object(source).unwrap().zone,Zone::Graveyard);
  move_to(&mut g,kamito,Zone::Graveyard);
  ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut HostChoice{host}).unwrap();
  assert_eq!(g.object(source).unwrap().zone,Zone::Graveyard);
  assert!(g.source_collection_objects(source,"inscribed host").is_empty());
  assert_eq!(g.current_power(host),Some(2));
 }}
}
#[test]
fn kamito_permission_cannot_be_borrowed_from_opponents_or_without_current_grant(){
 for grant in definitions("Kamito Eternalist") {for definition in definitions("Fleeting Thoughts") {
  let mut g=game();let source=g.create_object_from_definition(&definition,A,Zone::Graveyard);
  for mana in [ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Colorless]{g.player_mut(A).unwrap().mana_pool.add(mana,10);}
  let usable=|g:&GameState|compute_legal_actions(g,A).unwrap().iter().any(|action|matches!(action,LegalAction::ActivateAbility{source:id,..}if *id==source));
  assert!(!usable(&g));g.create_object_from_definition(&grant,B,Zone::Battlefield);assert!(!usable(&g));
  let own=g.create_object_from_definition(&grant,A,Zone::Battlefield);assert!(usable(&g));
  g.object_mut(own).unwrap().abilities_mut().clear();assert!(!usable(&g));
 }}
}
#[test]
fn activation_local_zone_permission_is_captured_before_sacrificing_its_granter_as_a_cost(){
 for grant in definitions("Kamito Eternalist") {
  let mut g=game();let kamito=g.create_object_from_definition(&grant,A,Zone::Battlefield);
  // Generic activation exercises the same permission without changing its
  // resolving effects. The first actual cost removes the permission source.
  let definition=ordinary("Captured activation zone","Type: Sorcery\nSacrifice a creature, Reveal this card from your hand: Exile this card from your hand. This activated ability has designation inscribe. Activate only if this card is in your hand. Activate only as a sorcery.");
  let source=g.create_object_from_definition(&definition,A,Zone::Graveyard);
  announce(&mut g,source,&mut HostChoice{host:kamito});
  assert_eq!(g.stack.len(),1);assert!(g.object(kamito).is_none());
  assert_eq!(g.object(source).unwrap().zone,Zone::Graveyard);
  ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(g.object(source).unwrap().zone,Zone::Graveyard);
 }
}

#[test]
fn sanctified_presence_chooses_one_literal_protection_at_resolution_for_the_same_target() {
    struct ProtectionChoice { index:usize, calls:usize }
    impl DecisionMaker for ProtectionChoice {
        fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {
            self.calls+=1; assert!(ctx.options.len()>=5+ironsmith_core::ChooseCardTypeEffect::all_card_types().len()); vec![self.index]
        }
    }
    for definition in definitions("Sanctified Presence") {
        for (index,expected) in [(0,ironsmith::ability::ProtectionFrom::Color(ironsmith::color::ColorSet::WHITE)),
            (5,ironsmith::ability::ProtectionFrom::CardType(ironsmith::CardType::Artifact))] {
            let mut g=game();let source=g.create_object_from_definition(&definition,A,Zone::Stack);
            let witness=ordinary("Protection recipient","Type: Creature\nPower/Toughness: 2/2");
            let target=g.create_object_from_definition(&witness,A,Zone::Battlefield);
            let other=g.create_object_from_definition(&witness,A,Zone::Battlefield);
            let mut chooser=ProtectionChoice{index,calls:0};
            let mut ctx=EffectContext::new(source,A,&mut chooser).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(target)]);
            for effect in definition.spell_effect.as_ref().unwrap().all_effects() {execute_effect(&mut g,effect,&mut ctx).unwrap();}
            drop(ctx);assert_eq!(chooser.calls,1,"one domain-union decision at resolution");
            let matches=|id|g.current_abilities(id).unwrap_or_default().iter().filter(|ability|matches!(&ability.kind,AbilityKind::Static(rule) if rule.protection_from()==Some(&expected))).count();
            assert_eq!(matches(target),1);assert_eq!(matches(other),0,"the choice retains the original target");
        }
    }
}

/// Parent-run diagnostic: compare the identical granted-duration activation
/// under distinct preceding source instructions, preserving all source metadata.
#[test]
fn inscribe_compound_duration_public_route_diagnostic_matrix() {
    let fixtures=inputs();
    let original=fixtures.iter().flat_map(|f|f["mechanics"].as_array().unwrap()).flat_map(|row|row["representative_cards"].as_array().unwrap())
        .find(|card|card["name"]=="Only Memories Remain").unwrap();
    let text=original["executable_text"].as_str().unwrap();
    assert!(text.contains("Target player mills seven cards."));
    let variants=[("complete",text.to_string()),
        ("no_prior_spell_instruction",text.replace("Target player mills seven cards.\n","")),
        ("source_draw_first",text.replace("Target player mills seven cards.","Draw a card.")),
        ("object_target_first",text.replace("Target player mills seven cards.","Target creature gets +0/+1 until end of turn."))];
    let mut failures=Vec::new();
    for (label,text) in variants {
        let mut card=original.clone();card["executable_text"]=text.into();
        match custom_final_next_wave_support::definitions(&card) {
            Ok(_)=>eprintln!("INSCRIBE_DURATION_DIAGNOSTIC {label}: strict independent native/artifact OK"),
            Err(error)=>{eprintln!("INSCRIBE_DURATION_DIAGNOSTIC {label}: {error}");failures.push(format!("{label}: {error}"));}
        }
    }
    assert!(failures.is_empty(),"Exact duration diagnostic failures:\n{}",failures.join("\n"));
}
