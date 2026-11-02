//! Unregistered generic optional payment tests; parent owns all execution.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,LegalAction,compute_legal_actions};
const A:PlayerId=PlayerId(0);
fn definitions(name:&str,text:&str)->[CardDefinition;2]{
 let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact(name,text,false).unwrap();
 let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[test]
fn referenced_named_payment_reads_captured_exact_spell_and_excludes_other_costs(){
 let mut game=GameState::new(vec!["A".into()],20);let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent spell","Type: Sorcery\nYou gain 1 life.",false).unwrap();
 let spell=game.create_object_from_definition(&definition,A,Zone::Stack);
 let mut label=ironsmith_core::OptionalCostRef::from("Additional");label.custom_keyword=Some("flux".into());
 let mut other=label.clone();other.custom_keyword=Some("other".into());
 game.object_mut(spell).unwrap().optional_costs_paid.costs=vec![(label.clone(),3),(other,7)];
 let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(spell).unwrap(),&game);
 let fresh=game.move_object_by_game_rule(spell,Zone::Hand).unwrap();
 let tag:ironsmith::tag::TagKey="paid_spell".into();let value=ironsmith_core::Value::TimesPaidLabelOf{object:Box::new(ironsmith::target::ChooseSpec::Tagged(tag.clone())),reference:label};
 let mut dm=SelectFirstDecisionMaker;let ctx=ironsmith::effects::EffectContext::new(fresh,A,&mut dm).with_tagged_objects([(tag,vec![snapshot])].into_iter().collect());
 assert_eq!(ironsmith::effects::helpers::resolve_value(&game,&value,&ctx).unwrap(),3);
}
struct Mixed;
impl DecisionMaker for Mixed {
 fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
  if ctx.description.starts_with("Choose optional costs") {assert_eq!(ctx.options.len(),2);vec![ctx.options[0].index,ctx.options[1].index,ctx.options[1].index]}
  else{SelectFirstDecisionMaker.decide_options(game,ctx)}
 }
}
#[test]
fn global_named_optional_alternative_preserves_label_and_allows_mixed_repetitions(){
 let override_text="Type: Creature — Wizard\nPower/Toughness: 1/1\nYou may pay {0} rather than pay the flux cost for spells you cast.";
 let spell_text="Mana cost: {0}\nType: Sorcery\nAs you cast this spell, you may pay the following additional cost any number of times: {2}.\nYou gain 1 life.";
 for (source,mut spell) in definitions("Independent discount",override_text).into_iter().zip(definitions("Independent optional spell",spell_text)){
  assert!(spell.attribute_custom_keyword_payment("flux"));
  let original_ref=spell.optional_costs[0].reference.clone();
  let mut game=GameState::new(vec!["A".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
  game.create_object_from_definition(&source,A,Zone::Battlefield);let id=game.create_object_from_definition(&spell,A,Zone::Hand);
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);
  let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==id)).unwrap();
  use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
  let mut state=PriorityLoopState::new(1);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=Mixed;
  let mut progress=apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
  for _ in 0..64{if !state.has_pending_action(){break}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}
  assert!(!state.has_pending_action());assert_eq!(game.stack.len(),1);let cast=game.object(game.stack.last().unwrap().object_id).unwrap();
  assert_eq!(cast.optional_costs[0].reference,original_ref);assert_eq!(cast.optional_costs_paid.times_paid(0),3);assert!(cast.optional_costs_paid.paid_custom_keyword("flux"));
  assert_eq!(cast.optional_costs_paid.payment_branch_choice(0,0),Some(0));assert_eq!(cast.optional_costs_paid.payment_branch_choice(0,1),Some(1));assert_eq!(cast.optional_costs_paid.payment_branch_choice(0,2),Some(1));
  assert_eq!(game.player(A).unwrap().mana_pool.total(),0,"only the printed payment spends two mana");
 }
}
#[test]
fn exact_null_entity_count_phrase_compiles_through_artifact(){
 let text="Mana cost: {2}{R}\nType: Creature\nPower/Toughness: 2/2\nMenace\nWhenever you cast a spell using flux, this creature gets +2/+0 until end of turn for each time that spell's additional flux cost was paid.";
 for definition in definitions("Independent watcher",text){assert!(format!("{:?}",definition.abilities).contains("TimesPaidLabelOf"));}
}
