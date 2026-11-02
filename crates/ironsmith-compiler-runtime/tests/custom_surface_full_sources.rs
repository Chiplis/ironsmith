//! Complete physical Surface sources and ordinary announcement/price/entry behavior.
//! Parent materializes the source-equivalent candidate before running this target.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
#[path="cf8_p08/play.rs"] mod play;
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::mana::ManaSymbol;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct MinimumTargets;
impl ironsmith::decision::DecisionMaker for MinimumTargets {
 fn decide_targets(&mut self,_game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<Target>{ctx.requirements.iter().flat_map(|requirement|requirement.legal_targets.iter().take(requirement.min_targets).copied()).collect()}
 fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{ironsmith::decision::DecisionMaker::decide_options(&mut ironsmith::decision::SelectFirstDecisionMaker,game,ctx)}
 fn decide_boolean(&mut self,_game:&GameState,_ctx:&ironsmith::decisions::context::BooleanContext)->bool{true}
}
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-surface-full-source.json")).unwrap()}
fn card(key:&str)->serde_json::Value {fixture()["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card|card["source_card_key"]==key).unwrap().clone()}
fn fronts(game:&mut GameState,key:&str,route:usize)->CardDefinition {
 let source=card(key);assert_eq!(source["rewrite_status"],"frontend_materialized_unverified","{}",source["name"]);
 let faces=source_routes::linked_definitions(&source).unwrap();
 for pair in &faces {game.register_linked_face_definition(&pair[route]);}
 faces[0][route].clone()
}
fn filler()->CardDefinition {source_routes::definitions(&serde_json::json!({"name":"Other graveyard card","executable_text":"Mana cost: {1}\nType: Artifact"})).unwrap()[0].clone()}
fn fill(game:&mut GameState,count:usize){for _ in 0..count{game.create_object_from_definition(&filler(),A,Zone::Graveyard);}}
fn mana(game:&mut GameState,symbol:ManaSymbol,amount:u32){game.player_mut(A).unwrap().mana_pool.add(symbol,amount);}
fn available(game:&GameState,donor:ObjectId)->Vec<LegalAction>{compute_legal_actions(game,A).unwrap().into_iter().filter(|action|matches!(action,LegalAction::CastSpell{spell_id,from_zone:Zone::Graveyard,..}if *spell_id==donor)).collect()}
fn announce(game:&mut GameState,action:LegalAction,dm:&mut dyn ironsmith::decision::DecisionMaker)->ObjectId {
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 let mut state=PriorityLoopState::new(3);let mut queue=ironsmith::triggers::TriggerQueue::new();
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){return game.stack.iter().find(|entry|!entry.is_ability).expect("announced spell").object_id;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).unwrap();}panic!("announcement incomplete")
}
fn settle(game:&mut GameState,dm:&mut dyn ironsmith::decision::DecisionMaker){let mut queue=ironsmith::triggers::TriggerQueue::new();for _ in 0..64{ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();if game.stack.is_empty(){return}ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();}panic!("stack did not settle")}
fn move_to(game:&mut GameState,id:ObjectId,zone:Zone)->ObjectId{let stable=game.object(id).unwrap().stable_id;play::apply(game,id,ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(id),zone,false)));game.find_object_by_stable_id(stable).unwrap()}

#[test]
fn all_sources_remain_in_inventory_and_thirty_two_in_scope_cards_compile_with_every_face(){
 let input=fixture();let cards=input["mechanics"][0]["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),33);
 let mut keys=std::collections::HashSet::new();let mut faces=0;let mut excluded=0;
 for card in cards{assert!(keys.insert(card["source_card_key"].as_str().unwrap()));
 if card["rewrite_status"]=="excluded_intrinsic_dependent_body"{assert_eq!(card["source_card_key"],"Padpole_CCR");assert!(card["original_text"].as_str().unwrap().contains("create a Berry token"));assert!(card["original_text"].as_str().unwrap().contains("+2/+2"));excluded+=1;continue;}
 assert_eq!(card["rewrite_status"],"frontend_materialized_unverified");assert!(!card["original_text"].as_str().unwrap().is_empty());let result=source_routes::linked_definitions(card).unwrap_or_else(|error|panic!("{}: {error}",card["source_card_key"]));assert_eq!(result.len(),card["source_faces"].as_array().unwrap().len());for pair in result{assert_eq!(pair[0].card.name,pair[1].card.name);faces+=1;}}
 assert_eq!(faces,41);assert_eq!(excluded,1);
}

#[test]
fn padpole_surface_price_is_independent_without_certifying_the_excluded_berry_body(){
 let original=card("Padpole_CCR");assert_eq!(original["rewrite_status"],"excluded_intrinsic_dependent_body");
 for definition in source_routes::definitions(&original["surface_price_candidate"]).unwrap(){
  let mut game=play::game();let donor=game.create_object_from_definition(&definition,A,Zone::Graveyard);fill(&mut game,6);mana(&mut game,ManaSymbol::Green,4);let action=available(&game,donor).remove(0);let copy=announce(&mut game,action,&mut play::Script::default());assert_eq!(game.cast_origin_snapshot(copy).unwrap().object_id,donor);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
 }
}

#[test]
fn threshold_timing_exact_donor_payment_and_normal_cast_control(){
 for route in 0..2{
  let mut game=play::game();let definition=fronts(&mut game,"Starved Piranha_CCR",route);let donor=game.create_object_from_definition(&definition,A,Zone::Graveyard);let stable=game.object(donor).unwrap().stable_id;fill(&mut game,5);mana(&mut game,ManaSymbol::Black,5);assert!(available(&game,donor).is_empty());fill(&mut game,1);let action=available(&game,donor).remove(0);game.turn.active_player=B;assert!(available(&game,donor).is_empty());game.turn.active_player=A;
  let copy=announce(&mut game,action,&mut play::Script::default());let copy_stable=game.object(copy).unwrap().stable_id;assert_ne!(copy_stable,stable);assert_eq!(game.cast_origin_snapshot(copy).unwrap().object_id,donor);assert_eq!(game.cast_origin_snapshot(copy).unwrap().zone,Zone::Graveyard);assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);assert_eq!(game.objects_in_zone(Zone::Graveyard).len(),6);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);settle(&mut game,&mut play::Script::default());let arrived=game.find_object_by_stable_id(copy_stable).unwrap();assert_eq!(game.object(arrived).unwrap().zone,Zone::Battlefield);assert!(matches!(game.object(arrived).unwrap().kind,ironsmith::object::ObjectKind::Token));assert!(game.object_has_ability(arrived,&ironsmith::static_abilities::StaticAbility::menace()));
  mana(&mut game,ManaSymbol::Black,4);let normal=play::cast(&mut game,A,&definition,&mut play::Script::default());assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);assert!(!game.object(normal).unwrap().cast_alternative_method.as_deref().is_some_and(|method|method.battlefield_instructions().cast_copy));
 }
}

#[test]
fn every_transformed_surface_uses_the_back_on_stack_and_keeps_a_transforming_copy(){
 let transformed=["Beloved Poppet//Abandoned Poppet_CCR","Chum Chaser//Ship Shredder_CCR","Claw Fossil//Reborn Scuttler_CCR","Entangled Fossil//Reborn Reef_CCR","Fortune Bearer//Devourer of Fortunes_CCR","Riverbed Snapper//Daunting Snapper_CCR","Root Fossil//Reborn Lily_CCR","Scuttling Cicada//Buzzing Metamorph_CCR","Serrated Fossil//Reborn Fisher_CCR"];
 for key in transformed{for route in 0..2{
  let mut game=play::game();let source=card(key);let definition=fronts(&mut game,key,route);let printed_value=definition.card.mana_cost.as_ref().unwrap().mana_value();let donor=game.create_object_from_definition(&definition,A,Zone::Graveyard);fill(&mut game,6);for symbol in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green]{mana(&mut game,symbol,10);}
  let action=available(&game,donor).into_iter().find(|action|matches!(action,LegalAction::CastSpell{casting_method:CastingMethod::Alternative(_),..})).unwrap();let copy=announce(&mut game,action,&mut MinimumTargets);let stable=game.object(copy).unwrap().stable_id;
  assert_eq!(game.object(copy).unwrap().name.as_ref(),source["source_faces"][1]["name"].as_str().unwrap(),"{key}");assert_eq!(game.object(copy).unwrap().linked_face_mana_value(),Some(printed_value),"{key}");assert!(game.object(copy).unwrap().cast_alternative_method.as_deref().unwrap().casts_transformed());
  settle(&mut game,&mut MinimumTargets);let permanent=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(permanent).unwrap().zone,Zone::Battlefield);play::apply(&mut game,permanent,ironsmith::Effect::new(ironsmith::effects::TransformEffect::new(ironsmith::target::ChooseSpec::SpecificObject(permanent))));assert_eq!(game.object(permanent).unwrap().name.as_ref(),source["source_faces"][0]["name"].as_str().unwrap(),"{key}");
 }}
}

#[test]
fn entry_counter_and_tap_riders_are_owned_by_the_actual_surface_price(){
 for route in 0..2{
  let mut game=play::game();let geode=fronts(&mut game,"Buried Geode_CCR",route);let donor=game.create_object_from_definition(&geode,A,Zone::Graveyard);fill(&mut game,6);mana(&mut game,ManaSymbol::Green,8);let action=available(&game,donor).remove(0);let copy=announce(&mut game,action,&mut play::Script::default());let stable=game.object(copy).unwrap().stable_id;settle(&mut game,&mut play::Script::default());let surface=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(surface).unwrap().counters.get(&ironsmith::CounterType::PlusOnePlusOne).copied(),Some(1));
  let normal=play::cast(&mut game,A,&geode,&mut play::Script::default());let normal_stable=game.object(normal).unwrap().stable_id;settle(&mut game,&mut play::Script::default());assert_eq!(game.object(game.find_object_by_stable_id(normal_stable).unwrap()).unwrap().counters.get(&ironsmith::CounterType::PlusOnePlusOne).copied().unwrap_or(0),0);
  let mut game=play::game();let whale=fronts(&mut game,"Wrecking Whale_CCR",route);let donor=game.create_object_from_definition(&whale,A,Zone::Graveyard);fill(&mut game,6);mana(&mut game,ManaSymbol::Blue,5);let selected=available(&game,donor).remove(0);let copy=announce(&mut game,selected,&mut play::Script::default());let stable=game.object(copy).unwrap().stable_id;settle(&mut game,&mut play::Script::default());let permanent=game.find_object_by_stable_id(stable).unwrap();assert!(game.is_tapped(permanent));assert!(game.object_has_ability(permanent,&ironsmith::static_abilities::StaticAbility::trample()));
 }
}

#[test]
fn a_selected_price_colored_reduction_requires_the_actual_named_creature(){
 for route in 0..2{for matching in [false,true]{
  let mut game=play::game();let bats=fronts(&mut game,"Relentless Bats_CCR",route);let donor=game.create_object_from_definition(&bats,A,Zone::Graveyard);fill(&mut game,6);mana(&mut game,ManaSymbol::Black,1);
  let control=if matching{bats.clone()}else{play::vanilla("Another Bat","{1}{B}","Bat",2,1)};game.create_object_from_definition(&control,A,Zone::Battlefield);
  let actions=available(&game,donor);assert_eq!(!actions.is_empty(),matching);
  if matching{let copy=announce(&mut game,actions.into_iter().next().unwrap(),&mut play::Script::default());assert_eq!(game.cast_origin_snapshot(copy).unwrap().object_id,donor);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);}
 }}
}

#[test]
fn the_death_discount_uses_the_original_donor_not_another_named_card(){
 for route in 0..2{for donor_died in [false,true]{
  let mut game=play::game();let poppet=fronts(&mut game,"Beloved Poppet//Abandoned Poppet_CCR",route);let donor=game.create_object_from_definition(&poppet,A,if donor_died{Zone::Battlefield}else{Zone::Graveyard});
  let donor=if donor_died{move_to(&mut game,donor,Zone::Graveyard)}else{let other=game.create_object_from_definition(&poppet,A,Zone::Battlefield);move_to(&mut game,other,Zone::Graveyard);donor};fill(&mut game,6);mana(&mut game,ManaSymbol::Black,2);let actions=available(&game,donor);assert_eq!(!actions.is_empty(),donor_died);
  if donor_died{let copy=announce(&mut game,actions.into_iter().next().unwrap(),&mut play::Script::default());assert_eq!(game.cast_origin_snapshot(copy).unwrap().object_id,donor);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);assert!(game.object(copy).unwrap().cast_alternative_method.as_deref().unwrap().casts_transformed());}
 }}
}

#[test]
fn an_entangled_fossil_grant_keeps_its_own_mana_price_and_does_not_inherit_transformed_rider(){
 for route in 0..2{
  let mut game=play::game();let fossil=fronts(&mut game,"Entangled Fossil//Reborn Reef_CCR",route);let grantor=game.create_object_from_definition(&fossil,A,Zone::Battlefield);let donor=game.create_object_from_definition(&fossil,A,Zone::Graveyard);fill(&mut game,6);mana(&mut game,ManaSymbol::Green,2);
  let mut dm=play::Script{targets:vec![Target::Object(donor)],..Default::default()};play::activate(&mut game,grantor,play::activated_at(&fossil,0),&mut dm);settle(&mut game,&mut dm);assert_eq!(game.player(A).unwrap().mana_pool.total(),1);
  let selected=available(&game,donor).into_iter().next().expect("granted own-mana Surface is affordable; printed transformed price is not");let copy=announce(&mut game,selected,&mut play::Script::default());assert_eq!(game.object(copy).unwrap().name.as_ref(),"Entangled Fossil");assert!(!game.object(copy).unwrap().cast_alternative_method.as_deref().unwrap().casts_transformed());assert_eq!(game.player(A).unwrap().mana_pool.total(),0);assert_eq!(game.cast_origin_snapshot(copy).unwrap().object_id,donor);
 }
}

#[test]
fn warden_supplies_ending_phase_flash_and_own_mana_surface_only_while_present(){
 for route in 0..2{for active_player in [A,B]{
  let mut game=play::game();game.turn.active_player=active_player;let warden=fronts(&mut game,"Warden of Depths_CCR",route);let provider=game.create_object_from_definition(&warden,A,Zone::Battlefield);let witness=play::vanilla("Graveyard grant recipient","{3}{U}","Human",3,2);let donor=game.create_object_from_definition(&witness,A,Zone::Graveyard);fill(&mut game,6);mana(&mut game,ManaSymbol::Blue,4);assert!(available(&game,donor).is_empty(),"ending-phase grant must not exist in first main");
  game.turn.phase=ironsmith::game_state::Phase::Ending;game.turn.step=Some(ironsmith::game_state::Step::End);let action=available(&game,donor).into_iter().next().expect("ending-phase flash permits a creature card copy outside main phase");let copy=announce(&mut game,action,&mut play::Script::default());assert_eq!(game.player(A).unwrap().mana_pool.total(),if active_player==B{2}else{0},"opponent-turn first-spell reduction applies to original own-mana Surface price exactly once");assert_eq!(game.cast_origin_snapshot(copy).unwrap().object_id,donor);let copied=game.object(copy).unwrap().stable_id;move_to(&mut game,provider,Zone::Hand);settle(&mut game,&mut play::Script::default());assert_eq!(game.object(game.find_object_by_stable_id(copied).unwrap()).unwrap().zone,Zone::Battlefield);
  let second=game.create_object_from_definition(&witness,A,Zone::Graveyard);mana(&mut game,ManaSymbol::Blue,4);assert!(available(&game,second).is_empty(),"departed grantor cannot authorize another copy");
 }}
}
#[test]
fn floe_master_observes_the_surface_entry_once_and_ignores_an_ordinary_cast(){
 for route in 0..2{
  let mut game=play::game();let floe=fronts(&mut game,"Floe Master_CCR",route);let observer=game.create_object_from_definition(&floe,A,Zone::Battlefield);let creature=play::vanilla("Opponent target","{2}","Human",2,2);let target=game.create_object_from_definition(&creature,B,Zone::Battlefield);let piranha=fronts(&mut game,"Starved Piranha_CCR",route);let donor=game.create_object_from_definition(&piranha,A,Zone::Graveyard);fill(&mut game,6);mana(&mut game,ManaSymbol::Black,9);
  let selected=available(&game,donor).remove(0);let mut dm=play::Script{targets:vec![Target::Object(target)],..Default::default()};let surfaced_spell=announce(&mut game,selected,&mut dm);assert_eq!(game.stack.len(),1,"permanent Surface observer does not fire at the cast event");
  play::apply(&mut game,observer,ironsmith::Effect::new(ironsmith::effects::CopySpellEffect::new(ironsmith::target::ChooseSpec::SpecificObject(surfaced_spell),1)));
  assert_eq!(game.stack.len(),2);settle(&mut game,&mut dm);assert!(game.is_tapped(target));assert_eq!(game.object(target).unwrap().counters.get(&ironsmith::CounterType::Stun).copied(),Some(1));
  play::cast(&mut game,A,&piranha,&mut dm);settle(&mut game,&mut dm);assert_eq!(game.object(target).unwrap().counters.get(&ironsmith::CounterType::Stun).copied(),Some(1),"ordinary and uncast copied creature entries are not surfaced");
 }
}

#[test]
fn harbor_mail_reads_another_spells_surface_receipt_from_exile(){
 for route in 0..2{for surfaced in [false,true]{
  let mut game=play::game();let mail=fronts(&mut game,"Harbor Mail_CCR",route);let observer=game.create_object_from_definition(&mail,A,Zone::Exile);let stable=game.object(observer).unwrap().stable_id;game.object_mut(observer).unwrap().counters.insert(ironsmith::CounterType::Named("stamp".into()),1);for _ in 0..4{game.create_object_from_definition(&filler(),A,Zone::Library);}
  let piranha=fronts(&mut game,"Starved Piranha_CCR",route);mana(&mut game,ManaSymbol::Black,2);mana(&mut game,ManaSymbol::Colorless,3);mana(&mut game,ManaSymbol::Blue,1);
  if surfaced{let donor=game.create_object_from_definition(&piranha,A,Zone::Graveyard);fill(&mut game,6);let selected=available(&game,donor).remove(0);announce(&mut game,selected,&mut play::Script::default());}
  else{play::cast(&mut game,A,&piranha,&mut play::Script::default());}
  settle(&mut game,&mut play::Script::default());let current=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(current).unwrap().zone,if surfaced{Zone::Graveyard}else{Zone::Exile});assert_eq!(game.player(A).unwrap().mana_pool.amount(ManaSymbol::Blue),if surfaced{0}else{1});
 }}
}
