//! Component-local optional discounts with source-complete Lafterii witnesses.
mod custom_final_next_wave_support;
use custom_final_next_wave_support::definitions;
use ironsmith::{GameState,PlayerId,ObjectId,Zone,GameProgress};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,LegalAction,compute_legal_actions};
use ironsmith::decisions::context::{SelectOptionsContext,SelectObjectsContext};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn routes(name:&str,text:&str,payment:Option<&str>)->[ironsmith::CardDefinition;2]{
 let mut source=serde_json::json!({"name":name,"executable_text":text});if let Some(payment)=payment{source["custom_keyword_payments"]=serde_json::json!([payment]);}definitions(&source).unwrap()
}
fn lafterii()->[ironsmith::CardDefinition;2]{let value:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-reverberate-emblem-source.json")).unwrap();definitions(value["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|s|s["name"]=="Lafterii Ingenuity").unwrap()).unwrap()}
struct Choices{discount:usize,donors:Vec<ObjectId>,cancel:bool,optional_branches:usize}
impl DecisionMaker for Choices{
 fn decide_options(&mut self,game:&GameState,ctx:&SelectOptionsContext)->Vec<usize>{
  if ctx.description.starts_with("Choose optional costs"){self.optional_branches=ctx.options.len();return vec![ctx.options[self.discount.min(ctx.options.len()-1)].index]}
  if ctx.description=="Choose a named casting price"{assert_eq!(ctx.options.len(),2);return if self.cancel{vec![]}else{vec![ctx.options[self.discount].index]}}
  SelectFirstDecisionMaker.decide_options(game,ctx)
 }
 fn decide_objects(&mut self,game:&GameState,ctx:&SelectObjectsContext)->Vec<ObjectId>{if ctx.description.starts_with("Reveal cards to splice"){self.donors.clone()}else{SelectFirstDecisionMaker.decide_objects(game,ctx)}}
}
fn game()->GameState{let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,20);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);game}
fn cast(game:&mut GameState,spell:ObjectId,dm:&mut Choices)->Result<(),String>{
 let action=compute_legal_actions(game,A).map_err(|e|format!("{e:?}"))?.into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell)).ok_or("missing legal cast")?;
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=ironsmith::game_loop::PriorityLoopState::new(2);
 let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(game,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),dm).map_err(|e|format!("{e:?}"))?;
 for _ in 0..96{if !state.has_pending_action(){return Ok(())}let GameProgress::NeedsDecisionCtx(ctx)=progress else{return Err(format!("{progress:?}"))};*game=game.clone();state=state.clone();progress=ironsmith::game_loop::apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).map_err(|e|format!("{e:?}"))?;}Err("cast did not finish".into())
}
#[test]
fn named_optional_prices_keep_base_colored_and_nonmana_costs_and_offer_zero_or_one_discount(){
 let reducer=routes("Scoped price source","Type: Enchantment\nAdditional casting costs named arbitrary spark or other echo you pay cost up to {1} less.",None);
 let spells=routes("Independent priced spell","Mana cost: {3}{U}\nType: Sorcery\nAs an additional cost to cast this spell, you may pay {2} and sacrifice a creature.\nYou gain 1 life.",Some("arbitrary spark"));
 for (source,spell) in reducer.into_iter().zip(spells){for discount in [0,1]{for source_controller in [A,B]{
  let mut game=game();game.create_object_from_definition(&source,source_controller,Zone::Battlefield);let body=routes("Sacrifice witness","Type: Creature\nPower/Toughness: 1/1",None)[0].clone();let victim=game.create_object_from_definition(&body,A,Zone::Battlefield);let stable=game.object(victim).unwrap().stable_id;let spell=game.create_object_from_definition(&spell,A,Zone::Hand);let mut dm=Choices{discount,donors:vec![],cancel:false,optional_branches:0};cast(&mut game,spell,&mut dm).unwrap();
  let reduced=source_controller==A&&discount==1;assert_eq!(dm.optional_branches,if source_controller==A{2}else{1});assert_eq!(game.player(A).unwrap().mana_pool.total(),21-6+u32::from(reduced));assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);
  let object=game.object(game.stack.last().unwrap().object_id).unwrap();assert!(object.optional_costs_paid.paid_custom_keyword("arbitrary spark"));assert!(object.optional_costs_before_payment_alternatives.is_none(),"authored costs restored after announcement");
 }}}
}
mod hybrid_component_prices {
 use super::*;
 struct Decisions{donor:ObjectId,discount:usize,pips:Vec<&'static str>,price_seen:bool,hybrid_seen:usize}
 impl DecisionMaker for Decisions{
  fn decide_objects(&mut self,game:&GameState,ctx:&SelectObjectsContext)->Vec<ObjectId>{
   if ctx.description.starts_with("Reveal cards to splice"){vec![self.donor]}else{SelectFirstDecisionMaker.decide_objects(game,ctx)}
  }
  fn decide_options(&mut self,game:&GameState,ctx:&SelectOptionsContext)->Vec<usize>{
   if ctx.description=="Choose a named casting price"{
    self.price_seen=true;assert_eq!(ctx.options.len(),2,"the generic hybrid half admits a one-mana local discount");return vec![ctx.options[self.discount].index];
   }
   if ctx.description.starts_with("Choose how to pay pip "){
    let wanted=self.pips[self.hybrid_seen];self.hybrid_seen+=1;
    return vec![ctx.options.iter().find(|option|option.description.starts_with(wanted)).expect("chosen hybrid/Phyrexian half is offered").index];
   }
   SelectFirstDecisionMaker.decide_options(game,ctx)
  }
 }
 fn timeshifter()->[ironsmith::CardDefinition;2]{
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-casting-early.json")).unwrap();
  let card=fixture["mechanics"].as_array().unwrap().iter().filter(|row|row["keyword"]=="Reprise").flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]=="Savrios' Timeshifter").unwrap();definitions(card).unwrap()
 }
 fn scenario(route:usize,donor_cost:&str,base_cost:&str,pips:Vec<&'static str>,discount:usize,pool:&[(ironsmith::mana::ManaSymbol,u32)],expected_remaining:u32,expected_life:i32){
  let mut game=super::game();game.create_object_from_definition(&timeshifter()[route],A,Zone::Battlefield);game.create_object_from_definition(&lafterii()[route],A,Zone::Battlefield);
  let donor=routes("Independent hybrid donor",&format!("Mana cost: {donor_cost}\nType: Sorcery\nYou gain 1 life."),None)[route].clone();
  let donor=game.create_object_from_definition(&donor,A,Zone::Graveyard);let donor_stable=game.object(donor).unwrap().stable_id;
  let recipient=routes("Independent receiving spell",&format!("Mana cost: {base_cost}\nType: Sorcery\nYou gain 1 life."),None)[route].clone();let id=game.create_object_from_definition(&recipient,A,Zone::Hand);
  game.player_mut(A).unwrap().mana_pool=Default::default();for(symbol,amount)in pool{game.player_mut(A).unwrap().mana_pool.add(*symbol,*amount);}
  let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==id)).unwrap();
  let mut dm=Decisions{donor,discount,pips,price_seen:false,hybrid_seen:0};
  let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=ironsmith::game_loop::PriorityLoopState::new(2);
  let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
  for _ in 0..96{
   if !state.has_pending_action(){break}
   let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};
   if matches!(&ctx,ironsmith::decisions::context::DecisionContext::HybridChoice(_)){
    let pending=state.pending_cast.as_ref().unwrap();
    if discount>0{assert!(pending.splice_costs.iter().any(|price|price.dynamic_mana_cost().is_some_and(|recipe|recipe.generic_reduction==discount as u32)),"hybrid choice must still own the immutable local discount recipe");}
   }
   game=game.clone();state=state.clone();progress=ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
  }
  assert!(!state.has_pending_action());assert!(dm.price_seen);assert_eq!(dm.hybrid_seen,dm.pips.len());
  assert_eq!(game.player(A).unwrap().mana_pool.total(),expected_remaining);assert_eq!(game.player(A).unwrap().life,expected_life);
  assert_eq!(game.object(game.find_object_by_stable_id(donor_stable).unwrap()).unwrap().zone,Zone::Exile);
  let spell=game.object(game.stack.last().unwrap().object_id).unwrap();let receipts:u32=spell.optional_costs_paid.costs.iter().filter(|(reference,_)|reference.custom_keyword.as_deref()==Some("reprise")).map(|(_,count)|*count).sum();assert_eq!(receipts,1,"price and hybrid continuation clones never duplicate named payment");
 }
 #[test]
 fn actual_timeshifter_and_lafterii_discount_the_announced_generic_half_without_erasing_the_colored_half(){
  use ironsmith::mana::ManaSymbol::*;
  for route in 0..2{
   scenario(route,"{2/U}","{R}",vec!["{2}"],1,&[(Red,1)],0,20);
   scenario(route,"{2/U}","{R}",vec!["{U}"],1,&[(Red,1),(Blue,1)],0,20);
   scenario(route,"{2/U}","{R}",vec!["{2}"],0,&[(Red,1),(Colorless,1)],0,20);
  }
 }
 #[test]
 fn actual_granted_prices_keep_hybrid_indices_after_generic_pips_from_different_components_coalesce(){
  use ironsmith::mana::ManaSymbol::*;
  for route in 0..2{scenario(route,"{1}{2/U}{2/B}","{1}{R}",vec!["{2}","{B}"],1,&[(Red,1),(Black,1),(Colorless,2)],0,20);}
 }
 #[test]
 fn actual_granted_phyrexian_price_keeps_its_life_obligation_after_local_and_global_generic_reductions(){
  use ironsmith::mana::ManaSymbol::*;
  for route in 0..2{scenario(route,"{1}{U/P}","{R}",vec!["2 life"],1,&[(Red,1)],0,18);}
 }
}
