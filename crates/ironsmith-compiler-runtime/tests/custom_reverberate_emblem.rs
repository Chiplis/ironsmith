//! Exact emblem source and generic source-local price transport. Parent-run only.
use ironsmith_compiled_artifact::CompiledCardArtifact;
mod custom_final_next_wave_support;

#[test]
fn exact_merkurio_source_preserves_loyalty_siblings_and_emblem_recipient_policy(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-reverberate-emblem-source.json")).unwrap();
 let row=&fixture["mechanics"][0];let card=&row["representative_cards"][0];
 let text=card["executable_text"].as_str().expect("parent materialized complete source");
 for sibling in ["[+1]: Merkurio deals 2 damage", "[-2]: The next instant or sorcery spell", "[-7]: You get an emblem"] { assert!(text.contains(sibling),"source sibling {sibling}"); }
 let [direct,materialized]=custom_final_next_wave_support::definitions(card).unwrap();
 assert!(direct.spell_effect.is_none());assert_eq!(direct.abilities.len(),3);assert_eq!(materialized.abilities.len(),3);
}

#[test]
fn recipient_optional_mana_value_recipe_has_exact_local_name_and_actual_cast_timing(){
 let text="Type: Enchantment\nCreatures you control have custom keyword \"stellar weaving\", and \"As an additional cost to cast this spell, you may pay mana equal to this spell's mana value. When you cast this spell, if you paid this additional cost, copy this spell and you may choose new targets for the copy. This optional additional cost is named stellar weaving.\".";
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Arbitrary recipe grant",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();assert!(direct.spell_effect.is_none());assert!(!direct.abilities.is_empty());
 let serialized=artifact.to_json().unwrap();assert!(serialized.contains("GenericManaValue"));assert!(serialized.contains("CastTrigger"));assert!(serialized.contains("stellar weaving"));
 let restored=CompiledCardArtifact::from_json(&serialized).unwrap();assert_eq!(artifact,restored);
}

mod gameplay {
 use ironsmith::{GameState,PlayerId,Zone,ObjectId,GameProgress};
 use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,LegalAction,compute_legal_actions};
 use ironsmith::decisions::context::SelectOptionsContext;
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
 const A:PlayerId=PlayerId(0);
 struct Choices{pay:bool}
 impl DecisionMaker for Choices {
  fn decide_options(&mut self,game:&GameState,ctx:&SelectOptionsContext)->Vec<usize>{
   if ctx.description.starts_with("Choose optional costs") {return if self.pay {ctx.options.iter().filter(|option|option.legal).map(|option|option.index).collect()}else{vec![]};}
   SelectFirstDecisionMaker.decide_options(game,ctx)
  }
 }
 fn action(game:&mut GameState,action:LegalAction,dm:&mut Choices){
  let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);
  let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
  for _ in 0..96 {
   if !state.has_pending_action(){ironsmith::game_loop::put_triggers_on_stack(game,&mut queue).unwrap();return;}
   let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};
   progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).unwrap();
  }panic!("announcement did not finish");
 }
 fn source_routes()->[ironsmith::CardDefinition;2]{
  let value:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-reverberate-emblem-source.json")).unwrap();
  super::custom_final_next_wave_support::definitions(&value["mechanics"][0]["representative_cards"][0]).unwrap()
 }
 #[test]
 fn actual_ultimate_grants_one_optional_recipient_price_and_cast_trigger(){
  for source in source_routes(){for (paid,available,accepted) in [(false,4,false),(true,4,true),(true,3,false)] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.turn.active_player=A;game.turn.priority_player=Some(A);
   let planeswalker=game.create_object_from_definition(&source,A,Zone::Battlefield);game.object_mut(planeswalker).unwrap().counters.insert(ironsmith::object::CounterType::Loyalty,7);
   let ultimate=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source,ability_index:2} if *source==planeswalker)).expect("actual ultimate is legal");
   let mut dm=Choices{pay:paid};action(&mut game,ultimate,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();
   // The emblem remains after the planeswalker that supplied it has departed.
   if game.object(planeswalker).is_some_and(|object|object.zone==Zone::Battlefield){let _=game.move_object_by_effect(planeswalker,Zone::Graveyard);}
   let spell=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient with distinct mana value","Mana cost: {2}\nType: Sorcery\nYou gain 2 life.",false).unwrap();
   let id=game.create_object_from_definition(&spell,A,Zone::Hand);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,available);
   let cast=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==id)).unwrap();action(&mut game,cast,&mut dm);
   assert_eq!(game.player(A).unwrap().mana_pool.total(),available-if accepted{4}else{2},"recipient value2, independent of planeswalker value3 or emblem value0");
   assert_eq!(game.stack.len(),if accepted{2}else{1},"paid copy is an actual cast trigger, not immediate continuation");
   if accepted {resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.stack.len(),2,"trigger creates exactly one copy");}
   while !game.stack.is_empty(){resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
   assert_eq!(game.player(A).unwrap().life,if accepted{24}else{22});
  }}
 }
}
