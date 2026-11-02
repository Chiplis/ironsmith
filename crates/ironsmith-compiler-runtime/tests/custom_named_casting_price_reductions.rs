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
struct Choices{discount:usize,donors:Vec<ObjectId>,cancel:bool,optional_branches:usize,price_seen:bool}
impl DecisionMaker for Choices{
 fn decide_options(&mut self,game:&GameState,ctx:&SelectOptionsContext)->Vec<usize>{
  if ctx.description.starts_with("Choose optional costs"){self.optional_branches=ctx.options.len();return vec![ctx.options[self.discount.min(ctx.options.len()-1)].index]}
  if ctx.description=="Choose a named casting price"{self.price_seen=true;assert_eq!(ctx.options.len(),2);return if self.cancel{vec![]}else{vec![ctx.options[self.discount].index]}}
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
  let mut game=game();game.create_object_from_definition(&source,source_controller,Zone::Battlefield);let body=routes("Sacrifice witness","Type: Creature\nPower/Toughness: 1/1",None)[0].clone();let victim=game.create_object_from_definition(&body,A,Zone::Battlefield);let stable=game.object(victim).unwrap().stable_id;let spell=game.create_object_from_definition(&spell,A,Zone::Hand);let mut dm=Choices{discount,donors:vec![],cancel:false,optional_branches:0,price_seen:false};cast(&mut game,spell,&mut dm).unwrap();
  let reduced=source_controller==A&&discount==1;assert_eq!(dm.optional_branches,if source_controller==A{2}else{1});assert_eq!(game.player(A).unwrap().mana_pool.total(),21-6+u32::from(reduced));assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);
  let object=game.object(game.stack.last().unwrap().object_id).unwrap();assert!(object.optional_costs_paid.paid_custom_keyword("arbitrary spark"));assert!(object.optional_costs_before_payment_alternatives.is_none(),"authored costs restored after announcement");
 }}}
}
#[test]
fn actual_lafterii_reduces_each_selected_reprise_price_and_cancellation_restores_announcement(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-casting-early.json")).unwrap();let donor=fixture["mechanics"].as_array().unwrap().iter().filter(|r|r["keyword"]=="Reprise").flat_map(|r|r["representative_cards"].as_array().unwrap()).find(|c|c["name"]=="Echoing Knowledge").unwrap();let donors=definitions(donor).unwrap();
 let spells=routes("Independent receiving spell","Mana cost: {3}\nType: Sorcery\nYou gain 1 life.",None);
 for ((source,donor),spell) in lafterii().into_iter().zip(donors).zip(spells){for discount in [0,1]{for cancel in [false,true]{
  let mut game=game();game.create_object_from_definition(&source,A,Zone::Battlefield);let donor=game.create_object_from_definition(&donor,A,Zone::Graveyard);let stable=game.object(donor).unwrap().stable_id;let spell=game.create_object_from_definition(&spell,A,Zone::Hand);let mut dm=Choices{discount,donors:vec![donor],cancel,optional_branches:0,price_seen:false};let result=cast(&mut game,spell,&mut dm);assert!(dm.price_seen);
  assert_eq!(game.player(A).unwrap().mana_pool.total(),if cancel{21}else{21-4+discount as u32},"spell base3 receives ordinary Lafterii1 reduction; only donor2 receives optional0or1");
  let donor=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(donor).unwrap().zone,if cancel{Zone::Graveyard}else{Zone::Exile});
  if cancel{assert!(result.is_err());assert!(game.stack.is_empty());}else{result.unwrap();let object=game.object(game.stack.last().unwrap().object_id).unwrap();let count:u32=object.optional_costs_paid.costs.iter().filter(|(r,_)|r.custom_keyword.as_deref()==Some("reprise")).map(|(_,n)|*n).sum();assert_eq!(count,1,"resuming a price subchoice never pays the named receipt twice");}
 }}}
}
#[test]
fn actual_lafterii_reverberate_discount_preserves_the_colored_base_and_named_receipt(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-reverberate-emblem-source.json")).unwrap();let card=fixture["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|c|c["name"]=="Quiet Meditation").unwrap();
 for (provider,spell) in lafterii().into_iter().zip(definitions(card).unwrap()){for discount in [0,1]{
  let mut game=game();game.create_object_from_definition(&provider,A,Zone::Battlefield);let spell=game.create_object_from_definition(&spell,A,Zone::Hand);let mut dm=Choices{discount,donors:vec![],cancel:false,optional_branches:0,price_seen:false};cast(&mut game,spell,&mut dm).unwrap();assert_eq!(dm.optional_branches,2);assert_eq!(game.player(A).unwrap().mana_pool.total(),19+discount as u32,"ordinary spell discount1 applies to total additional generic mana; named component discount0or1 is separate and the blue base remains payable");
  let spell=game.object(game.stack.last().unwrap().object_id).unwrap();assert!(spell.optional_costs_paid.paid_custom_keyword("reverberate"));
 }}
}

mod chosen_x {
 use super::*;
 use ironsmith::decisions::context::NumberContext;
 struct Decisions { discount:usize,x:u32,options:usize,raw_recipe_seen:bool }
 impl DecisionMaker for Decisions {
  fn decide_options(&mut self,game:&GameState,ctx:&SelectOptionsContext)->Vec<usize>{
   if ctx.description.starts_with("Choose optional costs"){
    self.options=ctx.options.len();assert_eq!(self.options,3,"two actual providers freeze discounts 0, 1, and 2 before X");
    let option=&ctx.options[self.discount];assert!(option.legal,"the retained component price has an affordable X=0 preview");return vec![option.index];
   }
   SelectFirstDecisionMaker.decide_options(game,ctx)
  }
  fn decide_number(&mut self,game:&GameState,ctx:&NumberContext)->u32{
   if let Some(spell)=ctx.source.and_then(|id|game.object(id)) {
    if self.discount>0 {
     self.raw_recipe_seen=spell.optional_costs.iter().any(|optional|optional.cost.as_one_of().is_some_and(|prices|prices.iter().any(|price|price.dynamic_mana_cost().is_some_and(|recipe|recipe.base.has_x()&&recipe.generic_reduction==self.discount as u32))));
     assert!(self.raw_recipe_seen,"X and the component-local discount remain retained until this decision");
    }
   }
   self.x
  }
 }
 fn drive(game:&mut GameState,action:LegalAction,dm:&mut Decisions,mut depart:Option<ObjectId>)->Result<(),String>{
  let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=ironsmith::game_loop::PriorityLoopState::new(2);
  let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(game,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),dm).map_err(|e|format!("{e:?}"))?;
  for _ in 0..96 {
   if !state.has_pending_action(){return Ok(())}
   let GameProgress::NeedsDecisionCtx(ctx)=progress else{return Err(format!("{progress:?}"))};
   if matches!(&ctx,ironsmith::decisions::context::DecisionContext::Number(_)) {
    if let Some(id)=depart.take(){game.move_object_by_effect(id,Zone::Graveyard).unwrap();}
   }
   *game=game.clone();state=state.clone();
   progress=ironsmith::game_loop::apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).map_err(|e|format!("{e:?}"))?;
  }Err("announcement did not complete".into())
 }
 fn prepare(route:usize)->(GameState,ObjectId){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-reverberate-emblem-source.json")).unwrap();
  let card=fixture["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card|card["name"]=="Merkurio Zellurys").unwrap();
  let mut game=super::game();let source=game.create_object_from_definition(&definitions(card).unwrap()[route],A,Zone::Battlefield);
  game.object_mut(source).unwrap().counters.insert(ironsmith::object::CounterType::Loyalty,7);
  let ultimate=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index:2}if *id==source)).unwrap();
  let mut dm=Decisions{discount:0,x:0,options:0,raw_recipe_seen:false};drive(&mut game,ultimate,&mut dm,None).unwrap();
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  let provider=lafterii()[route].clone();game.create_object_from_definition(&provider,A,Zone::Battlefield);game.create_object_from_definition(&provider,A,Zone::Battlefield);
  let spell=routes("Recipient with announced X","Mana cost: {X}{R}\nType: Sorcery\nYou gain X life.",None)[route].clone();
  let id=game.create_object_from_definition(&spell,A,Zone::Hand);
  game.player_mut(A).unwrap().mana_pool=Default::default();
  (game,id)
 }
 fn cast_recipient(game:&mut GameState,id:ObjectId,dm:&mut Decisions,depart:Option<ObjectId>)->Result<(),String>{
  let action=compute_legal_actions(game,A).map_err(|error|format!("{error:?}"))?.into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==id)).ok_or("missing recipient cast")?;
  drive(game,action,dm,depart)
 }
 #[test]
 fn actual_emblem_and_two_actual_lafterii_providers_price_each_announced_x_after_the_component_discount(){
  for route in 0..2{for x in [0,3,5]{for discount in 0..3 {
   let(mut game,id)=prepare(route);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,20);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red,1);
   let mut dm=Decisions{discount,x,options:0,raw_recipe_seen:false};cast_recipient(&mut game,id,&mut dm,None).unwrap();assert_eq!(dm.options,3);
   let component=(x+1).saturating_sub(discount as u32);let generic=(x+component).saturating_sub(2);
   assert_eq!(game.player(A).unwrap().mana_pool.total(),21-(generic+1),"X={x}, named discount={discount}; ordinary reductions apply after independent prices combine");
   let spell=game.object(game.stack.last().unwrap().object_id).unwrap();assert_eq!(spell.x_value,Some(x));assert!(spell.optional_costs_paid.paid_custom_keyword("reverberate"));
  }}}
 }
 #[test]
 fn actual_x_price_is_not_free_and_failure_restores_the_cast(){
  for route in 0..2{for (generic,accepted) in [(2,false),(3,true)] {
   let(mut game,id)=prepare(route);let stable=game.object(id).unwrap().stable_id;
   game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,generic);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red,1);
   let mut dm=Decisions{discount:2,x:3,options:0,raw_recipe_seen:false};let result=cast_recipient(&mut game,id,&mut dm,None);
   if accepted{result.unwrap();assert_eq!(game.player(A).unwrap().mana_pool.total(),0)}else{assert!(result.is_err());assert_eq!(game.player(A).unwrap().mana_pool.total(),generic+1);assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Hand);}
  }}
 }
 #[test]
 fn chosen_named_discount_survives_provider_departure_before_x_is_announced(){
  for route in 0..2 {
   let(mut game,id)=prepare(route);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,20);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red,1);
   let provider=game.battlefield.iter().copied().find(|id|game.object(*id).is_some_and(|object|object.name.as_str()=="Lafterii Ingenuity")).unwrap();
   let mut dm=Decisions{discount:2,x:3,options:0,raw_recipe_seen:false};cast_recipient(&mut game,id,&mut dm,Some(provider)).unwrap();
   assert_eq!(game.player(A).unwrap().mana_pool.total(),16,"named discount stays two; the independent ordinary spell discount now has only one provider");
  }
 }
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
