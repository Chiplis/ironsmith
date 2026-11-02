//! MSEM C702.298/C751: generic preparation and selected characteristic transport.
//! Requires next-wave categories patch and actual frontend materialization.
use ironsmith::{CardDefinition, GameState, ObjectId, Zone};
use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
use ironsmith::special_actions::{SpecialAction,can_perform_check,perform};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
#[path="cf8_p08/support.rs"] mod support;
#[path="cf8_p08/play.rs"] mod play;

fn definitions(name:&str)->[CardDefinition;2] {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-forsake.json")).unwrap();
 let card=fixture["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card|card["name"]==name).unwrap();
 support::definitions(name,card["executable_text"].as_str().expect("materialize actual frontend after scratch patch integration"))
}
fn casts(game:&GameState,player:ironsmith::PlayerId,id:ObjectId)->Vec<LegalAction> {
 compute_legal_actions(game,player).unwrap().into_iter().filter(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==id)).collect()
}
fn preparation_action(game:&GameState,id:ObjectId,index:usize)->SpecialAction {
 SpecialAction::PrepareExile{card_id:id,method_index:index,preparation:game.object(id).unwrap().alternative_casts[index].exile_preparation().unwrap().clone()}
}
fn prepare(game:&mut GameState,definition:&CardDefinition)->ObjectId {
 let id=game.create_object_from_definition(definition,play::A,Zone::Hand);
 play::give_mana(game,play::A,ManaSymbol::Colorless,2);
 let action=preparation_action(game,id,0);
 assert!(can_perform_check(&action,game,play::A).is_ok());
 perform(action,game,play::A,&mut SelectFirstDecisionMaker).unwrap();
 let id=*game.exile.last().unwrap();
 assert!(game.is_face_down(id));
 assert!(!game.is_foretold(id),"described preparation does not acquire the official Foretell identity");
 assert!(game.can_player_look_at_face_down_exiled_card(id,play::A));
 assert!(!game.can_player_look_at_face_down_exiled_card(id,play::B));
 assert!(game.stack.is_empty(),"preparation is a no-stack special action");
 assert!(!game.has_foretold_this_turn(play::A),"custom preparation is not a Foretell action");
 assert_eq!(game.player(play::A).unwrap().mana_pool.total(),0);
 assert!(casts(game,play::A,id).is_empty(),"the same turn never supplies the cast");
 id
}
fn later(game:&mut GameState) {
 game.next_turn();game.turn.active_player=play::A;game.turn.priority_player=Some(play::A);
 game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
}
fn cast_existing(game:&mut GameState,id:ObjectId)->ObjectId {
 let actions=casts(game,play::A,id);
 assert!(!actions.is_empty());
 assert!(actions.iter().all(|action|matches!(action,LegalAction::CastSpell{casting_method:ironsmith::alternative_cast::CastingMethod::Alternative(_),..})),"the prepared permission supplies its selected method, never a normal cast");
 let mut queue=TriggerQueue::new();let mut state=PriorityLoopState::new(3);let mut dm=play::Script::default();
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(actions[0].clone()),&mut dm).unwrap();
 for _ in 0..64 {
  if state.pending_cast.is_none() && state.pending_method_selection.is_none(){break;}
  let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};
  progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
 }
 assert!(state.pending_cast.is_none());game.stack.last().unwrap().object_id
}
#[test]
fn complete_source_variants_compile_on_direct_and_artifact_routes() {
 for name in ["Cling to Old Dreams","Faces of War","Witch's Whisper"] {let _=definitions(name);}
}
#[test]
fn preparation_is_only_at_sorcery_timing_and_owned_hand() {
 for d in definitions("Witch's Whisper") {
  let mut game=play::game();let id=game.create_object_from_definition(&d,play::A,Zone::Hand);
  play::give_mana(&mut game,play::A,ManaSymbol::Colorless,2);
  let action=preparation_action(&game,id,0);assert!(can_perform_check(&action,&game,play::A).is_ok());
  game.turn.phase=ironsmith::game_state::Phase::Combat;assert!(can_perform_check(&action,&game,play::A).is_err());
  game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.active_player=play::B;assert!(can_perform_check(&action,&game,play::A).is_err());
  game.turn.active_player=play::A;let blocker=game.create_object_from_definition(&d,play::A,Zone::Stack);game.stack.push(ironsmith::StackEntry::new(blocker,play::A));
  assert!(can_perform_check(&action,&game,play::A).is_err());assert_eq!(game.object(id).unwrap().zone,Zone::Hand);
  assert_eq!(game.player(play::A).unwrap().mana_pool.total(),2);
 }
}
#[test]
fn prepared_creature_has_copiable_selected_values_and_restores_normal_values_after_exit() {
 for d in definitions("Witch's Whisper") {
  let mut game=play::game();let id=prepare(&mut game,&d);let stable=game.object(id).unwrap().stable_id;later(&mut game);
  play::give_mana(&mut game,play::A,ManaSymbol::Black,1);play::give_mana(&mut game,play::A,ManaSymbol::Colorless,2);
  assert!(casts(&game,play::B,id).is_empty());let spell=cast_existing(&mut game,id);
  assert_eq!(game.object(spell).unwrap().mana_cost_owned().unwrap().to_oracle(),"{2}{B}");
  assert_eq!(game.stack.last().unwrap().optional_costs_paid.cast_was_foretold,Some(false));
  assert_eq!((game.current_characteristics(spell).unwrap().power,game.current_characteristics(spell).unwrap().toughness),(Some(3),Some(2)));
  let copy_effect=ironsmith::Effect::new(ironsmith::effects::CopySpellEffect::new(ironsmith::target::ChooseSpec::SpecificObject(spell),1));
  ironsmith::effects::execute_effect(&mut game,&copy_effect,&mut ironsmith::effects::EffectContext::new_default(spell,play::A)).unwrap();
  let copied_spell=game.stack.last().unwrap().object_id;assert_ne!(copied_spell,spell);
  assert_eq!(game.object(copied_spell).unwrap().mana_cost_owned().unwrap().to_oracle(),"{2}{B}");
  assert_eq!((game.current_characteristics(copied_spell).unwrap().power,game.current_characteristics(copied_spell).unwrap().toughness),(Some(3),Some(2)));
  resolve_stack_entry_with(&mut game,&mut play::Script::default()).unwrap();
  resolve_stack_entry_with(&mut game,&mut play::Script::default()).unwrap();
  let permanent=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(permanent).unwrap().zone,Zone::Battlefield);
  let original=game.object(permanent).unwrap().clone();let copy=game.create_object_from_definition(&d,play::A,Zone::Battlefield);
  game.object_mut(copy).unwrap().copy_copiable_values_from(&original);
  assert_eq!((game.current_characteristics(copy).unwrap().power,game.current_characteristics(copy).unwrap().toughness),(Some(3),Some(2)));
  assert_eq!(game.object(copy).unwrap().mana_cost_owned().unwrap().to_oracle(),"{2}{B}");
  let departed=game.move_object_by_effect(permanent,Zone::Graveyard).unwrap();
  assert_eq!(game.object(departed).unwrap().mana_cost_owned().unwrap().to_oracle(),"{2}{W}");
  assert_eq!((game.object(departed).unwrap().base_power,game.object(departed).unwrap().base_toughness),(Some(ironsmith::card::PtValue::Fixed(2)),Some(ironsmith::card::PtValue::Fixed(3))));
 }
}
#[test]
fn cost_only_source_retains_absent_power_toughness_and_resolves_normally() {
 for d in definitions("Faces of War") {
  let mut game=play::game();let id=prepare(&mut game,&d);let stable=game.object(id).unwrap().stable_id;later(&mut game);
  play::give_mana(&mut game,play::A,ManaSymbol::Red,2);play::give_mana(&mut game,play::A,ManaSymbol::Colorless,1);
  let spell=cast_existing(&mut game,id);let object=game.object(spell).unwrap();
  assert_eq!(object.mana_cost_owned().unwrap().to_oracle(),"{1}{R}{R}");assert_eq!((object.base_power,object.base_toughness),(None,None));
  resolve_stack_entry_with(&mut game,&mut play::Script{modes:vec![0,1],..Default::default()}).unwrap();
  assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);
 }
}
#[test]
fn selected_instant_resolves_its_program_before_entering_as_the_printed_creature_form() {
 for d in definitions("Cling to Old Dreams") {
  let mut game=play::game();let id=prepare(&mut game,&d);let stable=game.object(id).unwrap().stable_id;later(&mut game);
  play::give_mana(&mut game,play::A,ManaSymbol::Black,2);play::give_mana(&mut game,play::A,ManaSymbol::Colorless,4);
  let spell=cast_existing(&mut game,id);assert!(game.object(spell).unwrap().has_card_type(ironsmith::CardType::Instant));
  resolve_stack_entry_with(&mut game,&mut play::Script::default()).unwrap();
  let permanent=game.find_object_by_stable_id(stable).unwrap();let object=game.object(permanent).unwrap();
  assert_eq!(object.zone,Zone::Battlefield);assert!(object.has_card_type(ironsmith::CardType::Creature));
  assert!(!object.has_card_type(ironsmith::CardType::Instant));assert!(object.subtypes.contains(&ironsmith::Subtype::Nightmare));
  assert_eq!((game.current_characteristics(permanent).unwrap().power,game.current_characteristics(permanent).unwrap().toughness),(Some(7),Some(7)));
  let departed=game.move_object_by_effect(permanent,Zone::Graveyard).unwrap();let object=game.object(departed).unwrap();
  assert!(object.has_card_type(ironsmith::CardType::Instant));assert!(!object.has_card_type(ironsmith::CardType::Creature));
  assert_eq!(object.mana_cost_owned().unwrap().to_oracle(),"{1}{G}{W}{U}");
 }
}

#[test]
fn casting_the_source_normally_never_offers_or_uses_its_prepared_characteristics() {
 for d in definitions("Witch's Whisper") {
  let mut game=play::game();play::give_mana(&mut game,play::A,ManaSymbol::White,1);play::give_mana(&mut game,play::A,ManaSymbol::Colorless,2);
  let spell=play::cast(&mut game,play::A,&d,&mut play::Script::default());
  assert_eq!(game.object(spell).unwrap().mana_cost_owned().unwrap().to_oracle(),"{2}{W}");
  assert_eq!((game.current_characteristics(spell).unwrap().power,game.current_characteristics(spell).unwrap().toughness),(Some(2),Some(3)));
  assert!(game.object(spell).unwrap().prototype_cast_state.is_none());
 }
}

#[test]
fn the_source_instant_cast_normally_does_not_enter_as_a_creature() {
 for d in definitions("Cling to Old Dreams") {
  let mut game=play::game();
  for symbol in [ManaSymbol::Green,ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Colorless] {play::give_mana(&mut game,play::A,symbol,1);}
  let spell=play::cast(&mut game,play::A,&d,&mut play::Script::default());let stable=game.object(spell).unwrap().stable_id;
  assert!(game.object(spell).unwrap().prototype_cast_state.is_none());
  resolve_stack_entry_with(&mut game,&mut play::Script::default()).unwrap();
  let object=game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap();
  assert_eq!(object.zone,Zone::Graveyard);assert!(object.has_card_type(ironsmith::CardType::Instant));
  assert!(!object.has_card_type(ironsmith::CardType::Creature));
 }
}

#[test]
fn independent_preparations_keep_exact_prices_methods_and_real_foretell_identity_separate() {
 use ironsmith::alternative_cast::{AlternativeCastingMethod as Method,CastingMethod,ExilePreparation};
 for mut definition in definitions("Witch's Whisper") {
  let mut second=definition.alternative_casts[0].clone();
  let Method::FromZone{name,total_cost,instructions,..}=&mut second else{panic!("source must retain its generic prepared method")};
  *name="Other preparation".into();*total_cost=ironsmith::cost::TotalCost::mana(ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Blue]));
  instructions.exile_preparation=Some(ExilePreparation::FaceDownSorcery{price:ironsmith::mana::ManaCost::new().add_generic(1)});
  definition.alternative_casts.push(second);definition.alternative_casts.push(Method::Foretell{cost:ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Red])});
  for selected in [0usize,1,2] {
   let mut game=play::game();let card=game.create_object_from_definition(&definition,play::A,Zone::Hand);
   play::give_mana(&mut game,play::A,ManaSymbol::Colorless,2);
   let menu=compute_legal_actions(&game,play::A).unwrap();
   assert!(menu.contains(&LegalAction::SpecialAction(preparation_action(&game,card,0))));assert!(menu.contains(&LegalAction::SpecialAction(preparation_action(&game,card,1))));
   assert!(menu.contains(&LegalAction::SpecialAction(SpecialAction::Foretell{card_id:card})));
   let mut forged=preparation_action(&game,card,0);
   if let SpecialAction::PrepareExile{preparation,..}=&mut forged {*preparation=game.object(card).unwrap().alternative_casts[1].exile_preparation().unwrap().clone();}
   assert!(can_perform_check(&forged,&game,play::A).is_err(),"an occurrence cannot borrow another preparation's cheaper price");
   let action=if selected==2{SpecialAction::Foretell{card_id:card}}else{preparation_action(&game,card,selected)};
   perform(action,&mut game,play::A,&mut SelectFirstDecisionMaker).unwrap();let exiled=*game.exile.last().unwrap();
   assert_eq!(game.player(play::A).unwrap().mana_pool.total(),if selected==1{1}else{0});
   assert_eq!(game.is_foretold(exiled),selected==2);assert_eq!(game.has_foretold_this_turn(play::A),selected==2);
   later(&mut game);for color in [ManaSymbol::Black,ManaSymbol::Blue,ManaSymbol::Red,ManaSymbol::Colorless]{play::give_mana(&mut game,play::A,color,10);}
   let options=casts(&game,play::A,exiled);assert!(!options.is_empty());
   assert!(options.iter().all(|action|matches!(action,LegalAction::CastSpell{casting_method:CastingMethod::Alternative(index),..} if *index==selected)),"only the exact preparation occurrence grants a later cast: {options:?}");
   let moved=game.move_object_by_effect(exiled,Zone::Graveyard).unwrap();let orphan=game.move_object_by_effect(moved,Zone::Exile).unwrap();assert!(casts(&game,play::A,orphan).is_empty());
  }
 }
}

#[test]
fn identical_preparation_programs_still_retain_the_selected_occurrence_index() {
 use ironsmith::alternative_cast::CastingMethod;
 for mut definition in definitions("Witch's Whisper") {
  definition.alternative_casts.push(definition.alternative_casts[0].clone());
  let mut game=play::game();let card=game.create_object_from_definition(&definition,play::A,Zone::Hand);play::give_mana(&mut game,play::A,ManaSymbol::Colorless,2);
  let action=preparation_action(&game,card,1);perform(action,&mut game,play::A,&mut SelectFirstDecisionMaker).unwrap();let exiled=*game.exile.last().unwrap();later(&mut game);
  play::give_mana(&mut game,play::A,ManaSymbol::Black,1);play::give_mana(&mut game,play::A,ManaSymbol::Colorless,2);
  let options=casts(&game,play::A,exiled);assert!(!options.is_empty());assert!(options.iter().all(|action|matches!(action,LegalAction::CastSpell{casting_method:CastingMethod::Alternative(1),..})));
 }
}
