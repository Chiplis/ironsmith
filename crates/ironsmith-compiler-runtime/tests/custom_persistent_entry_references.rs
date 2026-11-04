//! Actual persistent entry-card consumers; every case runs independent source routes.
#[path="custom_final_next_wave_support.rs"] mod source;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
const A:PlayerId=PlayerId(0);
const KEYS:[&str;4]=["Deadland Scavenger_AFM","Deadland Summoner_AFM","Chillwhisper Witch_AFM","Subturgian Proliferator_AFM"];
fn definitions(key:&str)->[CardDefinition;2]{
 let data:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-alpha-engorge-fabled-complete-inventory.json")).unwrap();
 let card=data["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]==key).unwrap();
 let mut faces=source::linked_definitions(card).unwrap_or_else(|error|panic!("{key}: {error}"));assert_eq!(faces.len(),1);faces.remove(0)
}
fn ordinary(name:&str,text:&str)->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap()}
fn game()->GameState{let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::Phase::FirstMain;g}
struct Answer { yes:bool, mode:Option<usize> }
impl DecisionMaker for Answer {
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.yes}
 fn decide_options(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
  if c.max==1 && c.options.len()==2 {return self.mode.into_iter().collect();}
  SelectFirstDecisionMaker.decide_options(g,c)
 }
}
fn settle(g:&mut GameState,dm:&mut dyn DecisionMaker){let mut q=ironsmith::triggers::TriggerQueue::new();for _ in 0..64 {
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(g,&mut q,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(g,&mut q,dm).unwrap();
 if g.stack.is_empty(){return}ironsmith::game_loop::resolve_stack_entry_with(g,dm).unwrap();}panic!("source scenario did not settle")}
fn enter(g:&mut GameState,d:&CardDefinition,witness:&CardDefinition)->(ObjectId,ObjectId){
 let old=g.create_object_from_definition(witness,A,Zone::Library);let stable=g.object(old).unwrap().stable_id;
 let id=g.create_object_from_definition(d,A,Zone::Hand);let receipt=g.move_object_with_etb_processing_with_dm(id,Zone::Battlefield,&mut SelectFirstDecisionMaker).unwrap();
 assert!(!receipt.pending);let source=receipt.original.into_result().unwrap().new_id;
 let exiled=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(exiled).unwrap().zone,Zone::Exile);(source,exiled)
}
fn leave_and_return(g:&mut GameState,id:ObjectId)->ObjectId{let stable=g.object(id).unwrap().stable_id;g.move_object_by_game_rule(id,Zone::Hand).unwrap();let hand=g.find_object_by_stable_id(stable).unwrap();g.move_object_by_game_rule(hand,Zone::Exile).unwrap();let current=g.find_object_by_stable_id(stable).unwrap();assert_ne!(current,id);current}
#[test]fn all_four_actual_complete_sources_preserve_full_independent_transport(){for key in KEYS {
 let [native,artifact]=definitions(key);
 let encode=|definition|serde_json::to_value(ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition).unwrap()).unwrap();
 assert_eq!(encode(native),encode(artifact),"{key}: full typed transport must agree");
}}
#[test]fn scavenger_returns_only_its_exact_still_exiled_creature_or_land_after_source_dies(){
 for d in definitions(KEYS[0]){for (kind,reentered,accept) in [("Creature",false,true),("Land",false,true),("Artifact",false,true),("Creature",true,true),("Creature",false,false)]{
 let mut g=game();let witness=ordinary("Entry reference witness",&format!("Type: {kind}\n{}",if kind=="Creature"{"Power/Toughness: 3/5"}else{""}));
 let unrelated=g.create_object_from_definition(&ordinary("Unrelated exile","Type: Creature\nPower/Toughness: 9/9"),A,Zone::Exile);
 let(s,mut linked)=enter(&mut g,&d,&witness);if reentered{linked=leave_and_return(&mut g,linked)}
 let stable=g.object(linked).unwrap().stable_id;
 g.move_object_by_game_rule(s,Zone::Graveyard).unwrap();settle(&mut g,&mut Answer{yes:accept,mode:None});
 let current=g.find_object_by_stable_id(stable).unwrap();
 assert_eq!(g.object(current).map(|o|o.zone),Some(if accept && kind!="Artifact" && !reentered{Zone::Hand}else{Zone::Exile}));
 assert_eq!(g.object(unrelated).unwrap().zone,Zone::Exile);
 }}}
#[test]fn scavenger_checks_the_current_referenced_cards_type_not_only_its_entry_snapshot() {
 for d in definitions(KEYS[0]) {
  let mut g=game();let witness=ordinary("Entry creature before change","Type: Creature\nPower/Toughness: 2/2");
  let(s,linked)=enter(&mut g,&d,&witness);
  // Arrange the current comparison operand independently of the historical receipt.
  g.object_mut(linked).unwrap().card_types=vec![ironsmith::CardType::Artifact].into();
  g.move_object_by_game_rule(s,Zone::Graveyard).unwrap();settle(&mut g,&mut Answer{yes:true,mode:None});
  assert_eq!(g.object(linked).unwrap().zone,Zone::Exile);
 }
}
#[test]fn summoner_return_and_free_cast_are_mutually_exclusive_and_optional(){
 for d in definitions(KEYS[1]){for mode in [None,Some(0),Some(1)]{for reentered in [false,true]{
 let mut g=game();let witness=ordinary("Independent instant","Mana cost: {6}{U}\nType: Instant\nYou gain 4 life.");let(s,mut linked)=enter(&mut g,&d,&witness);
 if reentered{linked=leave_and_return(&mut g,linked)}let stable=g.object(linked).unwrap().stable_id;
 g.move_object_by_game_rule(s,Zone::Graveyard).unwrap();settle(&mut g,&mut Answer{yes:mode.is_some(),mode});
 let current=g.find_object_by_stable_id(stable).unwrap();
 let expected=if reentered||mode.is_none(){Zone::Exile}else if mode==Some(0){Zone::Hand}else{Zone::Graveyard};
 assert_eq!(g.object(current).unwrap().zone,expected,"reentered={reentered}, mode={mode:?}");
 assert_eq!(g.player(A).unwrap().life,if !reentered && mode==Some(1){24}else{20});assert_eq!(g.player(A).unwrap().mana_pool.total(),0);
 }}}}
#[test]fn summoners_cast_alternative_does_not_cast_an_unqualified_creature() {
 for d in definitions(KEYS[1]) {
  let mut g=game();let witness=ordinary("Unqualified entry creature","Type: Creature\nPower/Toughness: 2/2");
  let(s,linked)=enter(&mut g,&d,&witness);g.move_object_by_game_rule(s,Zone::Graveyard).unwrap();
  settle(&mut g,&mut Answer{yes:true,mode:Some(1)});
  assert_eq!(g.object(linked).unwrap().zone,Zone::Exile);assert!(g.player(A).unwrap().hand.is_empty());
 }
}
#[test]fn witch_compares_spell_card_types_only_with_its_exact_entry_card(){
 for d in definitions(KEYS[2]){for reentered in [false,true]{
 let mut g=game();let instant=ordinary("Entry instant","Mana cost: {2}\nType: Instant\nYou gain 1 life.");let(_,linked)=enter(&mut g,&d,&instant);
 if reentered{leave_and_return(&mut g,linked);}
 g.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless,1);
 let qualifies=g.create_object_from_definition(&instant,A,Zone::Hand);let other=g.create_object_from_definition(&ordinary("Unrelated creature spell","Mana cost: {2}\nType: Creature\nPower/Toughness: 1/1"),A,Zone::Hand);
 g.refresh_continuous_state().unwrap();let actions=ironsmith::decision::compute_legal_actions(&g,A).unwrap();
 let castable=|id|actions.iter().any(|action|matches!(action,ironsmith::decision::LegalAction::CastSpell{spell_id,..}if *spell_id==id));
 assert_eq!(castable(qualifies),!reentered);assert!(!castable(other));
 }}}
#[test]fn proliferator_uses_the_linked_creatures_power_and_toughness_and_pays_only_if_accepted(){
 for d in definitions(KEYS[3]){for (creature,accept,funded) in [(true,true,true),(true,false,true),(false,true,true),(true,true,false)]{
 let mut g=game();let witness=ordinary("Entry quantity witness",if creature{"Type: Creature\nPower/Toughness: 3/5"}else{"Type: Artifact"});
 g.create_object_from_definition(&ordinary("Unrelated large exile","Type: Creature\nPower/Toughness: 9/9"),A,Zone::Exile);
 let(s,_)=enter(&mut g,&d,&witness);g.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless,3);if funded{g.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Black,1);}
 g.turn.phase=ironsmith::Phase::Beginning;g.turn.step=Some(ironsmith::Step::Upkeep);
 g.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()));settle(&mut g,&mut Answer{yes:accept,mode:None});
 let tokens=g.battlefield.iter().copied().filter(|id|*id!=s&&g.object(*id).is_some_and(|o|o.kind==ironsmith::object::ObjectKind::Token)).collect::<Vec<_>>();
 assert_eq!(tokens.len(),usize::from(creature&&accept&&funded));if let Some(token)=tokens.first(){assert_eq!(g.calculated_power(*token),Some(3));assert_eq!(g.calculated_toughness(*token),Some(5));}
 assert_eq!(g.player(A).unwrap().mana_pool.total(),if creature&&accept&&funded{0}else if funded{4}else{3});
 }}}

#[test]fn summoners_alternative_choice_is_made_during_resolution_not_trigger_announcement() {
 struct Timing{resolving:bool,choices:usize}
 impl DecisionMaker for Timing {
  fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool {assert!(self.resolving);true}
  fn decide_options(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
   assert!(self.resolving,"the authored instead alternative must remain a resolution decision");
   if c.options.len()==2 && c.max==1 {self.choices+=1;vec![1]}else{SelectFirstDecisionMaker.decide_options(g,c)}
  }
 }
 for d in definitions(KEYS[1]) {
  let mut g=game();let witness=ordinary("Resolution timing witness","Mana cost: {7}{U}\nType: Instant\nYou gain 4 life.");
  let(s,_)=enter(&mut g,&d,&witness);g.move_object_by_game_rule(s,Zone::Graveyard).unwrap();
  let mut dm=Timing{resolving:false,choices:0};let mut queue=ironsmith::triggers::TriggerQueue::new();
  ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut g,&mut queue,&mut dm).unwrap();
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g,&mut queue,&mut dm).unwrap();
  assert_eq!(dm.choices,0);assert_eq!(g.stack.len(),1);dm.resolving=true;
  settle(&mut g,&mut dm);assert_eq!(dm.choices,1);assert_eq!(g.player(A).unwrap().life,24);
 }
}

#[test]fn root_entry_reference_capture_failure_is_explicit_and_rolls_back_without_an_existing_meter() {
 for d in definitions(KEYS[2]) {
  let mut g=game();let witness=ordinary("Entry capture witness","Type: Creature\nPower/Toughness: 3/5");let(s,linked)=enter(&mut g,&d,&witness);
  let anthem=ordinary("Checked capture pressure","Type: Enchantment\nCreatures you control get +1/+1.");g.create_object_from_definition(&anthem,A,Zone::Battlefield);
  g.set_static_effect_discovery_limits(ironsmith::static_ability_processor::StaticEffectDiscoveryLimits{max_rounds:128,max_generated_effects:0});
  let mut context=ironsmith::effects::EffectContext::new_default(s,A);
  let effect=ironsmith::Effect::new(ironsmith::effects::GainLifeEffect::you(1));
  let result=ironsmith::effects::execute_effect(&mut g,&effect,&mut context);
  assert!(matches!(result,Err(ironsmith::effects::ExecutionError::ContinuousDiscovery(_))),"{result:?}");
  assert_eq!(g.player(A).unwrap().life,20);assert_eq!(g.object(linked).unwrap().zone,Zone::Exile);
  assert!(context.get_tagged_all("__entry_reference_exiled__").is_none(),"failed derived references cannot leak from the restored instruction checkpoint");
  assert!(ironsmith::decision::compute_legal_actions(&g,A).is_err(),"public legality queries retain their checked failure boundary");
 }
}

fn independently_exiling_host(mut definition:CardDefinition,kind:&str)->CardDefinition {
 let article=if kind=="instant"{"an"}else{"a"};
 let extra=ordinary("Independent entry producer",&format!("Type: Creature\nPower/Toughness: 1/1\nAs this creature enters, exile {article} {kind} card from your graveyard."));
 definition.abilities.extend(extra.abilities);definition
}
fn enter_with_independent_producer(g:&mut GameState,d:&CardDefinition,own_kind:&str,other_kind:&str)->(ObjectId,ObjectId,ObjectId) {
 struct EntryChoice;impl DecisionMaker for EntryChoice {
  fn decide_objects(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{
   if let Some(candidate)=c.candidates.iter().find(|candidate|candidate.legal&&g.object(candidate.id).is_some_and(|object|object.zone==Zone::Graveyard&&object.name=="Named entry witness")){return vec![candidate.id];}
   SelectFirstDecisionMaker.decide_objects(g,c)
  }
 }
 let own=ordinary("Named entry witness",&format!("Type: {own_kind}\n{}",if own_kind=="Creature"{"Power/Toughness: 3/5"}else{""}));
 for _ in 0..3{g.create_object_from_definition(&own,A,Zone::Library);}
 let other=ordinary("Independent entry witness",&format!("Type: {other_kind}\n{}",if other_kind=="Creature"{"Power/Toughness: 9/9"}else{""}));
 let id=g.create_object_from_definition(&other,A,Zone::Graveyard);let other_stable=g.object(id).unwrap().stable_id;
 let source=g.create_object_from_definition(d,A,Zone::Hand);
 let receipt=g.move_object_with_etb_processing_with_dm(source,Zone::Battlefield,&mut EntryChoice).unwrap();assert!(!receipt.pending);let source=receipt.original.into_result().unwrap().new_id;
 let named=&g.object(source).unwrap().cast_tagged_objects[&ironsmith::tag::TagKey::from("__entry_action_exiled_by__:engorge")];
 assert_eq!(named.len(),1);assert_eq!(named[0].name,"Named entry witness");let own=named[0].object_id;
 let other=g.find_object_by_stable_id(other_stable).unwrap();assert_eq!(g.object(other).unwrap().zone,Zone::Exile);
 let aggregate=&g.object(source).unwrap().cast_tagged_objects[&ironsmith::tag::TagKey::from("__entry_action_exiled__")];assert_eq!(aggregate.len(),2);
 (source,own,other)
}
#[test]fn actual_entry_program_designations_survive_both_source_routes(){for key in KEYS {for d in definitions(key){
 let mut found=false;for ability in &d.abilities {
  if let ironsmith::ability::AbilityKind::Static(static_ability)=&ability.kind {
   if let Some(model)=static_ability.compiled_model(){if let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram{program,..}=&model.payload {
    found=true;assert!(ability.designations.iter().any(|label|label=="engorge"));assert!(!program.segments.is_empty());
    assert!(program.segments.iter().all(|segment|segment.ability_designations.iter().any(|label|label=="engorge")));
   }}
  }
 }assert!(found,"{key}: complete entry program missing");
}}}
#[test]fn witch_does_not_borrow_types_from_an_independent_entry_exile_program(){for d in definitions(KEYS[2]){
 let d=independently_exiling_host(d,"instant");let mut g=game();let(_,_,other)=enter_with_independent_producer(&mut g,&d,"Creature","Instant");
 g.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless,1);
 let creature=g.create_object_from_definition(&ordinary("Qualified creature spell","Mana cost: {2}\nType: Creature\nPower/Toughness: 1/1"),A,Zone::Hand);
 let instant=g.create_object_from_definition(&ordinary("Unqualified instant spell","Mana cost: {2}\nType: Instant\nYou gain 1 life."),A,Zone::Hand);
 let actions=ironsmith::decision::compute_legal_actions(&g,A).unwrap();let castable=|id|actions.iter().any(|action|matches!(action,ironsmith::decision::LegalAction::CastSpell{spell_id,..}if *spell_id==id));
 assert!(castable(creature));assert!(!castable(instant));assert_eq!(g.object(other).unwrap().zone,Zone::Exile);
}}
#[test]fn scavenger_returns_its_producers_card_after_death_and_leaves_an_independent_result_exiled(){for d in definitions(KEYS[0]){
 let d=independently_exiling_host(d,"instant");let mut g=game();let(s,own,other)=enter_with_independent_producer(&mut g,&d,"Creature","Instant");let stable=g.object(own).unwrap().stable_id;
 g.move_object_by_game_rule(s,Zone::Graveyard).unwrap();settle(&mut g,&mut Answer{yes:true,mode:None});
 let own=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(own).unwrap().zone,Zone::Hand);assert_eq!(g.object(other).unwrap().zone,Zone::Exile);
}}
#[test]fn proliferators_historical_condition_rejects_a_creature_from_an_independent_entry_program(){for d in definitions(KEYS[3]){
 let d=independently_exiling_host(d,"creature");let mut g=game();let(s,_,other)=enter_with_independent_producer(&mut g,&d,"Artifact","Creature");
 g.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless,3);g.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Black,1);
 g.turn.phase=ironsmith::Phase::Beginning;g.turn.step=Some(ironsmith::Step::Upkeep);g.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()));settle(&mut g,&mut Answer{yes:true,mode:None});
 assert_eq!(g.player(A).unwrap().mana_pool.total(),4);assert_eq!(g.object(other).unwrap().zone,Zone::Exile);assert_eq!(g.battlefield.iter().copied().filter(|id|*id!=s&&g.object(*id).is_some_and(|o|o.kind==ironsmith::object::ObjectKind::Token)).count(),0);
}}
#[test]fn anonymous_entry_tags_and_another_sources_snapshot_cannot_authorize_named_entry_references(){for d in definitions(KEYS[2]){
 let mut g=game();let witness=ordinary("Owned reference witness","Type: Creature\nPower/Toughness: 3/5");let(owner,card)=enter(&mut g,&d,&witness);
 let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(g.object(owner).unwrap(),&g);let receipt=snapshot.entry_action_objects[&ironsmith::tag::TagKey::from("__entry_action_exiled_by__:engorge")].clone();
 let stranger=g.create_object_from_definition(&d,A,Zone::Battlefield);let mut ctx=ironsmith::effects::EffectContext::new_default(stranger,A);
 ctx.source_snapshot=Some(snapshot);ctx.set_tagged_objects("__entry_action_exiled_by__:engorge",receipt.clone());ctx.set_tagged_objects("__entry_reference_exiled_by__:engorge",receipt);
 let count=ironsmith::effect::Value::TaggedSnapshotCount{tag:"__entry_action_exiled_by__:engorge".into(),filter:Default::default()};
 assert_eq!(ironsmith::effects::helpers::resolve_value(&g,&count,&ctx).unwrap(),0);
 let effect=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::ChooseSpec::Tagged("__entry_reference_exiled_by__:engorge".into()),Zone::Hand,false));
 let _rejected=ironsmith::effects::execute_effect(&mut g,&effect,&mut ctx);assert_eq!(g.object(card).unwrap().zone,Zone::Exile);assert!(g.player(A).unwrap().hand.is_empty());
}}

#[test]fn summoner_cannot_cast_an_instant_donated_by_an_independent_entry_program(){for d in definitions(KEYS[1]){
 let d=independently_exiling_host(d,"instant");let mut g=game();let(s,own,other)=enter_with_independent_producer(&mut g,&d,"Creature","Instant");
 g.move_object_by_game_rule(s,Zone::Graveyard).unwrap();settle(&mut g,&mut Answer{yes:true,mode:Some(1)});
 assert_eq!(g.object(own).unwrap().zone,Zone::Exile);assert_eq!(g.object(other).unwrap().zone,Zone::Exile);assert_eq!(g.player(A).unwrap().life,20);assert!(g.player(A).unwrap().hand.is_empty());
}}
