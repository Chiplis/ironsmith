//! Repeated grouped characteristic actions retain exact completion packets.
use ironsmith::{GameState,PlayerId,Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
fn definitions(name:&str,text:&str)->[ironsmith::cards::CardDefinition;2] {
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();
 let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[test]
fn grouped_waste_completion_is_one_exact_packet_and_repeats_without_status_transition() {
 let text="Type: Sorcery\nChoose two lands you control. Those lands become a basic land named Wastes with \"{T}: Add {C}\" and loses all other supertypes, card types, subtypes, and abilities. Perform the waste action with permanents affected this way.";
 for spell in definitions("Overwrite batch",text) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let land=definitions("Grove","Type: Land — Forest\n{T}: Add {G}.").into_iter().next().unwrap();
  let first=game.create_object_from_definition(&land,A,Zone::Battlefield);
  let second=game.create_object_from_definition(&land,A,Zone::Battlefield);
  let observer=definitions("Realm observer","Type: Artifact\nWhenever you perform the waste action with lands, you gain that much life.").into_iter().next().unwrap();
  game.create_object_from_definition(&observer,A,Zone::Battlefield);
  for iteration in 0..2 {
   let source=game.create_object_from_definition(&spell,A,Zone::Stack);
   let mut dm=SelectFirstDecisionMaker;let mut ctx=EffectContext::new(source,A,&mut dm);
   let mut events=Vec::new();
   for effect in spell.spell_effect.as_ref().unwrap().all_effects(){events.extend(execute_effect(&mut game,effect,&mut ctx).unwrap().events);}
   let packets=events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).collect::<Vec<_>>();
   assert_eq!(packets.len(),1,"one batch must not become two individual actions");
   assert_eq!(packets[0].amount,2);
   assert_eq!(packets[0].objects.len(),2);
   assert!(packets[0].objects.iter().any(|object|object.object_id==first));
   assert!(packets[0].objects.iter().any(|object|object.object_id==second));
   for event in events {game.queue_trigger_event(Default::default(),event);}
   let mut queue=ironsmith::triggers::TriggerQueue::new();
   ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();
   assert_eq!(queue.entries.len(),1);
   ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
   ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
   assert_eq!(game.player(A).unwrap().life,22+iteration*2);
  }
 }
}
#[test]
fn named_payment_receipt_is_attached_after_the_physical_conversion() {
 let text="Type: Artifact\nHave two lands you control not named Wastes become a basic land named Wastes with \"{T}: Add {C}\" and loses all other supertypes, card types, subtypes, and abilities and perform the waste action: You gain 1 life.";
 for definition in definitions("Receipt cost",text){
  let ability=definition.abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Activated(ability)=>Some(ability),_=>None}).unwrap();
  for resources in [0,1,2] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
   let land=definitions("Reserve","Type: Land\n{T}: Add {G}.").into_iter().next().unwrap();
   let own=(0..resources).map(|_|game.create_object_from_definition(&land,A,Zone::Battlefield)).collect::<Vec<_>>();
   let watcher=definitions("Payment observer","Type: Artifact\nWhenever you perform the waste action with lands, you gain that much life.").into_iter().next().unwrap();
   game.create_object_from_definition(&watcher,A,Zone::Battlefield);
   let mut dm=SelectFirstDecisionMaker;
   let costs=ability.mana_cost.costs();assert_eq!(costs.len(),1);
   let mut payment=ironsmith::costs::CostContext::new(source,A,&mut dm);
   assert_eq!(costs[0].can_pay(&game,&payment).is_ok(),resources==2);
   if resources<2 {
    assert!(own.iter().all(|id|game.current_characteristics(*id).unwrap().name.as_ref()=="Reserve"));
    continue;
   }
   costs[0].pay(&mut game,&mut payment).unwrap();
   assert!(own.iter().all(|id|game.current_characteristics(*id).unwrap().name.as_ref()=="Wastes"));
   assert_eq!(game.player(A).unwrap().life,20,"the activated ability itself has not resolved");
   let mut queue=ironsmith::triggers::TriggerQueue::new();
   ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();
   assert_eq!(queue.entries.len(),1,"physical group payment emits exactly one completion");
   ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
   ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
   assert_eq!(game.player(A).unwrap().life,22);
  }
 }
}
struct SelectConversion { count:usize }
impl ironsmith::decision::DecisionMaker for SelectConversion {
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{true}
 fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ironsmith::ObjectId>{
  if ctx.candidates.iter().any(|candidate|game.current_has_card_type(candidate.id,ironsmith::CardType::Land)) {
   return ctx.candidates.iter().filter(|candidate|candidate.legal).take(self.count).map(|candidate|candidate.id).collect();
  }
  ironsmith::decision::DecisionMaker::decide_objects(&mut SelectFirstDecisionMaker,game,ctx)
 }
 fn decide_number(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::NumberContext)->u32{(self.count as u32).clamp(ctx.min,ctx.max)}
}
#[test]
fn additional_conversion_discount_counts_completed_lands_not_one_action_invocation() {
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 let text="Mana cost: {6}{B}\nType: Creature\nPower/Toughness: 1/1\nYou may have any number of lands you control not named Wastes become a basic land named Wastes with \"{T}: Add {C}\" and loses all other supertypes, card types, subtypes, and abilities and perform the waste action as an additional cost to cast this spell.\nThis spell costs {1} less to cast for each land affected this way.";
 for definition in definitions("Conversion discount",text){for count in [0usize,3]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.active_player=A;game.turn.priority_player=Some(A);
  let land=definitions("Barren reserve","Type: Land").into_iter().next().unwrap();
  let own=(0..3).map(|_|game.create_object_from_definition(&land,A,Zone::Battlefield)).collect::<Vec<_>>();
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black,1);
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,6-count as u32);
  let hand=game.create_object_from_definition(&definition,A,Zone::Hand);
  let stable=game.object(hand).unwrap().stable_id;
  let action=ironsmith::decision::LegalAction::CastSpell{spell_id:hand,from_zone:Zone::Hand,casting_method:ironsmith::alternative_cast::CastingMethod::Normal};
  assert!(ironsmith::decision::compute_legal_actions(&game,A).unwrap().contains(&action));
  let mut dm=SelectConversion{count};let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);
  let mut progress=apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
  for _ in 0..100{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}
  assert!(!state.has_pending_action());
  let stack=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(stack).unwrap().zone,Zone::Stack);
  assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
  assert_eq!(own.iter().filter(|id|game.current_characteristics(**id).unwrap().name.as_ref()=="Wastes").count(),count);
  assert!(own.iter().all(|id|!game.is_tapped(*id)),"the discounted price was fully funded from the starting pool");
 }}
}
