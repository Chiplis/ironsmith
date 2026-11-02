//! Public announcement/payment/target/copy transactions; deliberately unrun by worker.
use ironsmith::{GameState,GameProgress,PlayerId,ObjectId,Target,Zone};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::decisions::context::{SelectOptionsContext,TargetsContext,ManaPaymentContext};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::effects::{CopySpellEffect,EffectContext,EffectExecutor};
use ironsmith::target::ChooseSpec;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0); const B:PlayerId=PlayerId(1);
fn definitions()->[CardDefinition;2]{
 let text="Mana cost: {1}\nType: Instant\nAs you cast this spell, you may pay the following additional cost any number of times: {1}.\nFor each time you pay this spell's optional additional cost, remove one bracketed text fragment from it.\nDestroy target [black] creature [with flying].";
 let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact("Subset transaction",text,false).unwrap();
 let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()].map(|mut definition|{assert!(definition.attribute_custom_keyword_payment("glitch"));definition})
}
fn game()->GameState{
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=Phase::FirstMain;game.turn.step=None;
 game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,30);game
}
fn targets(game:&mut GameState)->[ObjectId;4]{
 [("{B}",true),("{W}",true),("{B}",false),("{W}",false)].map(|(mana,flying)|{
  let text=format!("Mana cost: {mana}\nType: Creature — Bird\nPower/Toughness: 2/2{}",if flying{"\nFlying"}else{""});
  let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Victim",&text,false).unwrap();
  game.create_object_from_definition(&definition,B,Zone::Battlefield)
 })
}
struct Choices{payments:usize,fragments:Vec<usize>,legal:Vec<Target>,optional_seen:bool,fragment_seen:bool,target_seen:bool,cancel_mana:bool,cancel_seen:bool,mixed_branches:bool}
impl DecisionMaker for Choices{
 fn decide_options(&mut self,game:&GameState,ctx:&SelectOptionsContext)->Vec<usize>{
  if ctx.description.starts_with("Choose optional costs"){
   self.optional_seen=true;
   if self.mixed_branches{assert_eq!(ctx.options.len(),2);assert!(ctx.options.iter().all(|option|option.legal&&option.repeatable));return vec![ctx.options[0].index,ctx.options[1].index]}
   let option=ctx.options.iter().find(|option|option.legal).unwrap();assert!(option.repeatable);
   return vec![option.index;self.payments];
  }
  if ctx.description=="Choose bracketed text fragments to remove"{self.fragment_seen=true;assert_eq!(ctx.min,self.fragments.len());assert_eq!(ctx.max,self.fragments.len());return self.fragments.clone()}
  SelectFirstDecisionMaker.decide_options(game,ctx)
 }
 fn decide_targets(&mut self,game:&GameState,ctx:&TargetsContext)->Vec<Target>{
  if self.mixed_branches{
   let source=game.object(ctx.source).unwrap();assert!(source.optional_costs_before_payment_alternatives.is_some());assert!(source.optional_costs[0].cost.as_one_of().is_some());
   let captured=ironsmith::snapshot::ObjectSnapshot::from_object(source,game);
   assert!(captured.copiable_values.optional_costs[0].cost.as_one_of().is_none(),"pre-payment targeting snapshots retain authored copiable costs");
   assert!(source.to_card_definition().optional_costs[0].cost.as_one_of().is_none());
  }
  self.target_seen=true;assert_eq!(ctx.requirements.len(),1);
  let actual=&ctx.requirements[0].legal_targets;assert_eq!(actual.len(),self.legal.len());for target in &self.legal{assert!(actual.contains(target),"exact selected subset must govern target declaration")}
  vec![self.legal[0]]
 }
 fn decide_mana_payment(&mut self,game:&GameState,ctx:&ManaPaymentContext)->ManaPaymentResponse{
  if self.cancel_mana{self.cancel_seen=true;return ManaPaymentResponse::Cancel}SelectFirstDecisionMaker.decide_mana_payment(game,ctx)
 }
}
fn announce(game:&mut GameState,spell:ObjectId,choices:&mut Choices){
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==spell)).expect("some funded optional subset makes this cast legal");
 let mut queue=TriggerQueue::new();let mut state=PriorityLoopState::new(2);
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),choices).unwrap();
 for _ in 0..64{
  if !state.has_pending_action(){return}
  let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("unfinished transaction: {progress:?}")};
  // Resuming from cloned public state must retain pending subset announcements.
  *game=game.clone();state=state.clone();
  progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,choices).unwrap();
 }panic!("transaction did not complete")
}
fn choices(payments:usize,fragments:Vec<usize>,legal:Vec<Target>)->Choices{Choices{payments,fragments,legal,optional_seen:false,fragment_seen:false,target_seen:false,cancel_mana:false,cancel_seen:false,mixed_branches:false}}
#[test]
fn real_cast_choices_pay_repetitions_and_select_exact_target_variants(){
 for definition in definitions(){for (count,mask,subset,indices) in [(0,0,vec![],vec![0]),(1,1,vec![0],vec![0,1]),(1,2,vec![1],vec![0,2]),(2,3,vec![],vec![0,1,2,3]),(7,3,vec![],vec![0,1,2,3])]{
  let mut game=game();let victims=targets(&mut game);let spell=game.create_object_from_definition(&definition,A,Zone::Hand);
  let mut dm=choices(count,subset,indices.into_iter().map(|index|Target::Object(victims[index])).collect());announce(&mut game,spell,&mut dm);
  assert!(dm.optional_seen);assert!(dm.target_seen);assert_eq!(dm.fragment_seen,count==1);
  assert_eq!(game.player(A).unwrap().mana_pool.total(),29-count as u32);
  let entry=game.stack.last().unwrap();assert_eq!(entry.optional_costs_paid.times_paid(0),count as u32);assert_eq!(entry.optional_costs_paid.text_fragment_choice(0),Some(mask));
  let object=game.object(entry.object_id).unwrap();assert_eq!(object.optional_costs_paid.text_fragment_choice(0),Some(mask));
 }}
}
#[test]
fn real_spell_copy_retains_selected_subset_and_payment_count(){
 for definition in definitions(){for (subset,mask) in [(vec![0],1),(vec![1],2)]{
  let mut game=game();let victims=targets(&mut game);let spell=game.create_object_from_definition(&definition,A,Zone::Hand);
  let legal=if mask==1{vec![Target::Object(victims[0]),Target::Object(victims[1])]}else{vec![Target::Object(victims[0]),Target::Object(victims[2])]};
  let mut dm=choices(1,subset,legal);announce(&mut game,spell,&mut dm);let original=game.stack.last().unwrap().object_id;
  CopySpellEffect::single(ChooseSpec::SpecificObject(original)).execute(&mut game,&mut EffectContext::new(original,A,&mut SelectFirstDecisionMaker)).unwrap();
  assert_eq!(game.stack.len(),2);let copy=game.stack.last().unwrap();assert_ne!(copy.object_id,original);
  assert_eq!(copy.optional_costs_paid.times_paid(0),1);assert_eq!(copy.optional_costs_paid.text_fragment_choice(0),Some(mask));
  assert_eq!(game.object(copy.object_id).unwrap().optional_costs_paid.text_fragment_choice(0),Some(mask));
  assert_eq!(game.current_spell_program(original).unwrap()[0].0.get_target_spec(),game.current_spell_program(copy.object_id).unwrap()[0].0.get_target_spec());
 }}
}
#[test]
fn cancelling_actual_mana_payment_rolls_back_subset_receipts_and_cast(){
 for definition in definitions(){
  let mut game=game();let victims=targets(&mut game);let spell=game.create_object_from_definition(&definition,A,Zone::Hand);
  let mut dm=choices(1,vec![1],vec![Target::Object(victims[0]),Target::Object(victims[2])]);dm.cancel_mana=true;announce(&mut game,spell,&mut dm);
  assert!(dm.optional_seen&&dm.fragment_seen&&dm.target_seen&&dm.cancel_seen);
  assert!(game.stack_is_empty());assert_eq!(game.player(A).unwrap().mana_pool.total(),30);
  let object=game.object(spell).unwrap();assert_eq!(object.zone,Zone::Hand);assert_eq!(object.optional_costs_paid.times_paid(0),0);assert_eq!(object.optional_costs_paid.text_fragment_choice(0),None);
 }
}

#[test]
fn repeated_named_cost_can_mix_printed_and_global_free_price_in_one_cast(){
 for definition in definitions(){
  let mut game=game();let victims=targets(&mut game);
  let replacement=ironsmith_compiler_runtime::compile_to_runtime_definition("Price source","Type: Enchantment\nYou may pay {0} rather than pay the glitch cost for spells you cast.",false).unwrap();
  let price_source=game.create_object_from_definition(&replacement,A,Zone::Battlefield);
  let spell=game.create_object_from_definition(&definition,A,Zone::Hand);
  let mut dm=choices(2,vec![],victims.into_iter().map(Target::Object).collect());dm.mixed_branches=true;announce(&mut game,spell,&mut dm);
  assert_eq!(game.player(A).unwrap().mana_pool.total(),28,"one base mana plus one printed fee; the other fee uses the global free alternative");
  let entry=game.stack.last().unwrap();assert_eq!(entry.optional_costs_paid.times_paid(0),2);assert_eq!(entry.optional_costs_paid.text_fragment_choice(0),Some(3));
  assert_eq!(entry.optional_costs_paid.payment_branch_choice(0,0),Some(0));assert_eq!(entry.optional_costs_paid.payment_branch_choice(0,1),Some(1));
  assert_eq!(game.object(entry.object_id).unwrap().optional_costs_paid,entry.optional_costs_paid);
  let committed=entry.object_id;
  assert!(game.object(committed).unwrap().optional_costs[0].cost.as_one_of().is_none(),"expanded prices are announcement-only");
  assert!(game.object(committed).unwrap().optional_costs_before_payment_alternatives.is_none());
  CopySpellEffect::single(ChooseSpec::SpecificObject(committed)).execute(&mut game,&mut EffectContext::new(committed,A,&mut SelectFirstDecisionMaker)).unwrap();
  let copied=game.stack.last().unwrap().object_id;assert_ne!(copied,committed);
  assert!(game.object(copied).unwrap().optional_costs[0].cost.as_one_of().is_none(),"a copy receives authored costs, not a vanished global price");
  assert_eq!(game.object(copied).unwrap().optional_costs_paid.payment_branch_choice(0,1),Some(1));
  game.stack.pop();game.move_object_by_effect(copied,Zone::Graveyard).unwrap();game.stack.pop();
  let returned=game.move_object_by_effect(committed,Zone::Hand).unwrap();game.move_object_by_effect(price_source,Zone::Graveyard).unwrap();game.turn.priority_player=Some(A);
  let mut again=choices(2,vec![],victims.into_iter().map(Target::Object).collect());announce(&mut game,returned,&mut again);
  assert_eq!(game.player(A).unwrap().mana_pool.total(),25,"after the source leaves, recast pays both printed fees plus its base cost");
  let recast=game.stack.last().unwrap();assert_eq!(recast.optional_costs_paid.times_paid(0),2);
  assert_eq!(recast.optional_costs_paid.text_fragment_choice(0),Some(3));
  assert!(game.object(recast.object_id).unwrap().optional_costs[0].cost.as_one_of().is_none(),"the departed provider cannot leave a one-of price behind");
  // A plain authored cost has no branch choice. None distinguishes an
  // unbranched payment from choosing branch zero of an offered alternative.
  assert_eq!(recast.optional_costs_paid.payment_branch_choice(0,0),None);
  assert_eq!(recast.optional_costs_paid.payment_branch_choice(0,1),None);
 }
}
