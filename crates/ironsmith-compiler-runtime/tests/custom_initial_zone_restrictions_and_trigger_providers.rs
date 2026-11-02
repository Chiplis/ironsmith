//! Actual source clauses plus independent initial-state and trigger-provider negatives.
#[path="custom_final_next_wave_support.rs"] mod support;
use ironsmith::{GameState,CardDefinition,PlayerId,ObjectId,Zone,Effect};
use ironsmith::effects::{execute_effect,EffectContext};
use ironsmith::effect::{Restriction,Until};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
const FIND:&str="This turn, players can't search libraries, bring cards from outside the game into game zones, or move cards that began the game face down in the command zone out of the command zone. (An unlocked puzzlebox cannot be unlocked again.)\nDraw a card.";
fn routes(name:&str,text:&str)->[CardDefinition;2]{support::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap()}
fn blank()->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary witness","Type: Creature — Fox\nPower/Toughness: 2/2",false).unwrap()}
fn prohibit(game:&mut GameState,source:ObjectId){execute_effect(game,&Effect::cant_until(Restriction::MoveBetweenZones{from:Zone::Command,to:None,initial_state:Some((Zone::Command,true)),cards_only:true},Until::EndOfTurn),&mut EffectContext::new_default(source,A)).unwrap();}
#[test]
fn initial_facts_are_not_current_face_state_or_a_later_command_arrival(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let d=blank();
 let started_down=game.create_object_from_definition(&d,A,Zone::Command);assert!(game.set_face_down(started_down));
 let started_up=game.create_object_from_definition(&d,B,Zone::Command);
 let late=game.create_object_from_definition(&d,B,Zone::Hand);
 game.capture_game_start_object_states();
 game.set_face_up(started_down).unwrap();game.set_face_down(started_up);
 let late=game.move_object_by_effect(late,Zone::Command).unwrap();game.set_face_down(late);
 // Repeating the boundary cannot turn a late arrival into a game-start fact.
 game.capture_game_start_object_states();prohibit(&mut game,started_down);
 assert!(game.move_object_by_effect(started_down,Zone::Hand).is_none());
 assert_eq!(game.object(started_down).unwrap().zone,Zone::Command);
 assert!(game.move_object_by_effect(started_up,Zone::Hand).is_some());
 assert!(game.move_object_by_effect(late,Zone::Hand).is_some());
 game.turn.turn_number+=1;game.update_cant_effects();
 assert!(game.move_object_by_effect(started_down,Zone::Hand).is_some());
}
#[test]
fn immutable_initial_fact_survives_incarnations_and_missing_proof_fails_visibly(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let d=blank();let id=game.create_object_from_definition(&d,A,Zone::Command);game.set_face_down(id);
 let restriction=Effect::cant_until(Restriction::MoveBetweenZones{from:Zone::Command,to:None,initial_state:Some((Zone::Command,true)),cards_only:true},Until::EndOfTurn);
 assert!(matches!(execute_effect(&mut game,&restriction,&mut EffectContext::new_default(id,A)),Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
 game.capture_game_start_object_states();let hand=game.move_object_by_effect(id,Zone::Hand).unwrap();let returned=game.move_object_by_effect(hand,Zone::Command).unwrap();assert_ne!(id,returned);
 prohibit(&mut game,returned);assert!(game.move_object_by_effect(returned,Zone::Graveyard).is_none());
}
#[test]
fn actual_find_nothing_composes_search_outside_and_initial_command_rules_then_draws(){
 for d in routes("Find Nothing",&format!("Mana cost: {{1}}{{U}}\nType: Instant\n{FIND}")){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let card=blank();let source=game.create_object_from_definition(&d,A,Zone::Hand);
  let locked=game.create_object_from_definition(&card,B,Zone::Command);game.set_face_down(locked);
  let outside=game.create_object_from_definition(&card,B,Zone::OutsideGame);let draw=game.create_object_from_definition(&card,A,Zone::Library);let stable=game.object(draw).unwrap().stable_id;
  game.capture_game_start_object_states();let source=game.move_object_by_effect(source,Zone::Stack).unwrap();
  let program=d.spell_effect.as_ref().expect("spell program");let mut ctx=EffectContext::new_default(source,A);
  for effect in program.all_effects_owned(){execute_effect(&mut game,&effect,&mut ctx).unwrap();}
  assert!(game.effect_store.cant_effects.cant_search.contains(&A));assert!(game.effect_store.cant_effects.cant_search.contains(&B));
  assert!(game.move_object_by_effect(outside,Zone::Hand).is_none());assert!(game.move_object_by_effect(locked,Zone::Hand).is_none());
  let drawn=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(drawn).unwrap().zone,Zone::Hand);
 }
}
#[test]
fn actual_hisano_filters_the_entering_provider_separately_from_ability_controller(){
 const ORIGINAL:&str="If a permanent with spirited entering the battlefield causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.";
 let canonical=ORIGINAL.replace("with spirited","with custom keyword \"Spirited\"");
 for hisano in routes("Hisano the White Lion",&format!("Mana cost: {{R}}{{G}}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 3/2\n{canonical}")){
  for marked in [false,true]{
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&hisano,A,Zone::Battlefield);
   let observer=routes("Observer","Type: Enchantment\nWhenever a permanent enters, you gain 1 life.")[0].clone();
   let ours=game.create_object_from_definition(&observer,A,Zone::Battlefield);let theirs=game.create_object_from_definition(&observer,B,Zone::Battlefield);
   let mut donor=blank();if marked{donor.abilities.push(ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::from_model(ironsmith::static_abilities::CompiledStaticAbility::custom_keyword_identity("Spirited"))));}
   let entrant=game.create_object_from_definition(&donor,B,Zone::Hand);
   execute_effect(&mut game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(entrant),Zone::Battlefield,false)),&mut EffectContext::new_default(entrant,B)).unwrap();
   let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
   assert_eq!(queue.entries.iter().filter(|e|e.source==ours).count(),if marked{2}else{1});
   assert_eq!(queue.entries.iter().filter(|e|e.source==theirs).count(),1);
  }
 }
}

#[test]
fn card_scoped_rule_does_not_forbid_virtual_copy_or_token_but_object_scoped_rule_can() {
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let d=blank();
 let card=game.create_object_from_definition(&d,A,Zone::Graveyard);
 let copy=game.create_object_from_definition(&d,A,Zone::Graveyard);
 game.object_mut(copy).unwrap().kind=ironsmith::object::ObjectKind::SpellCopy;
 let token=game.create_object_from_definition(&d,A,Zone::Graveyard);
 game.object_mut(token).unwrap().kind=ironsmith::object::ObjectKind::Token;
 let effect=Effect::cant_until(Restriction::MoveBetweenZones{from:Zone::Graveyard,to:None,initial_state:None,cards_only:true},Until::EndOfTurn);
 execute_effect(&mut game,&effect,&mut EffectContext::new_default(card,A)).unwrap();
 assert!(game.zone_move_is_prohibited(card,Zone::Hand));
 assert!(!game.zone_move_is_prohibited(copy,Zone::Hand));
 assert!(!game.zone_move_is_prohibited(token,Zone::Hand));
 let effect=Effect::cant_until(Restriction::MoveBetweenZones{from:Zone::Graveyard,to:None,initial_state:None,cards_only:false},Until::EndOfTurn);
 execute_effect(&mut game,&effect,&mut EffectContext::new_default(card,A)).unwrap();
 assert!(game.zone_move_is_prohibited(copy,Zone::Hand));
 assert!(game.zone_move_is_prohibited(token,Zone::Hand));
}
