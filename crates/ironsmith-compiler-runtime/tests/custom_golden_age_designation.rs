//! Generic conjunctions of completed turn actions grant independent lasting markers.
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
#[path="custom_final_next_wave_support.rs"]
mod source_support;
const A: PlayerId=PlayerId(0);
const B: PlayerId=PlayerId(1);
const REMINDER:&str="If you cast an instant or sorcery spell this turn, activated a nonmana ability this turn, and attacked this turn, you are in your renaissance for the rest of the game.";
fn definitions(text:&str)->[CardDefinition;2]{
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("History designation",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 let indirect=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
 assert_eq!(ironsmith_text::compiled_text_lines(&direct),ironsmith_text::compiled_text_lines(&indirect));[direct,indirect]
}
fn body()->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Historical actor","Type: Creature\nPower/Toughness: 2/2",false).unwrap()}
fn history(game:&mut GameState,mask:u8,actor:PlayerId,mana:bool,spell_type:&str){
 let source=game.create_object_from_definition(&body(),actor,Zone::Battlefield);
 if mask&1!=0{
  let spell=ironsmith_compiler_runtime::compile_to_runtime_definition("Historical spell",&format!("Type: {spell_type}"),false).unwrap();
  let id=game.create_object_from_definition(&spell,actor,Zone::Stack);
  let event=ironsmith::events::spells::SpellCastEvent::from_completed_cast(id,actor,Zone::Hand,game);
  game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(event,Default::default()));
  game.remove_object(id);
 }
 if mask&2!=0{game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::spells::AbilityActivatedEvent::new(source,actor,mana),Default::default()));}
 if mask&4!=0{game.mark_creature_attacked_this_turn(source);}
 game.remove_object(source);
}
#[test]
fn permanent_checks_all_three_actions_same_actor_and_reapplies_continuous_effects(){
 for definition in definitions(&format!("Type: Creature\nPower/Toughness: 1/1\n{REMINDER}\nThis creature gets +3/+3 as long as you are in a renaissance.")){
  for mask in 0..8{
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
   history(&mut game,mask,A,false,"Instant");history(&mut game,7,B,false,"Sorcery");
   game.refresh_continuous_state().unwrap();
   assert_eq!(game.has_player_designation(A,"renaissance"),mask==7,"mask {mask}");
   assert_eq!(game.current_power(source),Some(if mask==7{4}else{1}));
   assert!(!game.has_player_designation(B,"renaissance"));
   assert!(!game.has_player_designation(A,"endgame"));
   if mask==7{game.remove_object(source);game.refresh_continuous_state().unwrap();assert!(game.has_player_designation(A,"renaissance"));}
  }
 }
}
#[test]
fn mana_activation_creature_spell_and_distributed_actors_do_not_satisfy_conjunction(){
 for definition in definitions(&format!("Type: Artifact\n{REMINDER}")){
  for (mana,kind) in [(true,"Instant"),(false,"Creature")]{
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   game.create_object_from_definition(&definition,A,Zone::Battlefield);history(&mut game,7,A,mana,kind);
   game.refresh_continuous_state().unwrap();assert!(!game.has_player_designation(A,"renaissance"));
  }
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&definition,A,Zone::Battlefield);
  history(&mut game,1,A,false,"Instant");history(&mut game,6,B,false,"Sorcery");game.refresh_continuous_state().unwrap();assert!(!game.has_player_designation(A,"renaissance"));
 }
}
#[test]
fn spell_checks_at_resolution_and_renaissance_condition_reads_its_marker(){
 for definition in definitions(&format!("Type: Sorcery\n{REMINDER}\nIf you are in a renaissance, you gain 3 life.")){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);history(&mut game,7,A,false,"Sorcery");
  let source=game.create_object_from_definition(&definition,A,Zone::Stack);game.refresh_continuous_state().unwrap();assert!(!game.has_player_designation(A,"renaissance"));
  let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A)).unwrap();
  assert!(game.has_player_designation(A,"renaissance"));assert_eq!(game.player(A).unwrap().life,23);
 }
}
#[test]
fn all_forty_three_golden_age_source_cards_round_trip(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-golden-age-designation.json")).unwrap();
 let mut count=0;let mut failures=Vec::new();
 for row in fixture["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){
  let source_key=card["source_card_key"].as_str().or_else(||card["id"].as_str()).unwrap_or("<missing exact source key>");
  let Some(faces)=card["executable_faces"].as_array() else {
   failures.push(format!("{} / {source_key}: missing executable_faces",row["id"]));continue;
  };
  for face in faces {
   let face_name=face["name"].as_str().unwrap_or("<missing face name>");
   let label=format!("{} / {source_key} / {face_name}",row["id"]);
   let checked=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    assert_eq!(card["rewrite_status"],"frontend_materialized_unverified","{}",card["name"]);
    let name=face["name"].as_str().unwrap();
    let [direct,indirect]=source_support::definitions(face).unwrap_or_else(|error|panic!("{name}: {error}"));
    assert_eq!(ironsmith_text::compiled_text_lines(&direct),ironsmith_text::compiled_text_lines(&indirect));
   }));
   match checked {
    Ok(())=>count+=1,
    Err(panic)=>{
     let message=panic.downcast_ref::<String>().cloned().or_else(||panic.downcast_ref::<&str>().map(|message|(*message).to_string())).unwrap_or_else(||"non-text panic".into());
     failures.push(format!("{label}: {message}"));
    }
   }
  }
 }}
 assert!(failures.is_empty(),"{} Golden age source-face failures ({} strictly passed):\n{}",failures.len(),count,failures.join("\n"));
 assert_eq!(count,43);
}

#[test]
fn independent_history_clauses_can_be_reordered_or_used_without_other_clauses(){
 for (condition,mask,mana,kind,expected) in [
  ("attacked this turn",4,false,"Instant",true),
  ("attacked this turn",3,false,"Instant",false),
  ("activated an ability this turn",2,true,"Instant",true),
  ("cast a creature spell this turn",1,false,"Creature",true),
  ("attacked this turn, cast an instant or sorcery spell this turn, and activated a nonmana ability this turn",7,false,"Instant",true),
  ("attacked this turn, cast an instant or sorcery spell this turn, and activated a nonmana ability this turn",7,true,"Instant",false),
 ]{
  for definition in definitions(&format!("Type: Artifact\nIf you {condition}, you are in your finale for the rest of the game.")){
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&definition,A,Zone::Battlefield);history(&mut game,mask,A,mana,kind);game.refresh_continuous_state().unwrap();assert_eq!(game.has_player_designation(A,"finale"),expected,"{condition}");assert!(!game.has_player_designation(A,"renaissance"));
  }
 }
}


#[test]
fn clean_marker_only_designation_rechecks_on_completed_history_without_a_physical_mutation() {
 for definition in definitions("Type: Artifact\nIf you activated a nonmana ability this turn, you are in your finale for the rest of the game.") {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let actor=game.create_object_from_definition(&body(),A,Zone::Battlefield);
  game.refresh_continuous_state().unwrap();
  assert!(!game.has_player_designation(A,"finale"));
  // No tap, zone move or object mutation can incidentally invalidate the
  // warmed state: only publication of the completed activation changes it.
  game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(
   ironsmith::events::spells::AbilityActivatedEvent::new(actor,A,false),Default::default()));
  game.refresh_continuous_state().unwrap();
  assert!(game.has_player_designation(A,"finale"));
  assert!(!game.has_player_designation(B,"finale"));
  assert!(game.stack.is_empty());
 }
}


#[test]
fn supplied_named_token_program_survives_independent_native_and_artifact_compilation() {
 let input=serde_json::json!({"name":"Scoped token creator","executable_text":"Type: Sorcery\nCreate a colorless artifact token named Keepsake.",
  "custom_token_definitions":[{"name":"Keepsake","token":true,"executable_text":"Type: Artifact\n{T}: Add {C}."}]});
 for definition in source_support::definitions(&input).unwrap() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&definition,A,Zone::Stack);
  let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
  execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A)).unwrap();
  assert_eq!(game.battlefield.len(),1);
  let token=game.object(game.battlefield[0]).unwrap();
  assert_eq!(token.name.as_str(),"Keepsake");
  assert_eq!(token.kind,ironsmith::object::ObjectKind::Token);
  assert!(token.card_types.contains(&ironsmith::CardType::Artifact));
  assert!(token.abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::AbilityKind::Activated(_))),"the supplied ability cannot disappear into a nametag placeholder");
 }
 assert!(source_support::definitions(&serde_json::json!({"name":"Invalid token metadata","executable_text":"Type: Sorcery\nDraw a card.","custom_token_definitions":"unread data"})).is_err());
}


#[test]
fn conditional_owner_library_choice_replaces_the_bounce_and_uses_the_owners_library() {
 for definition in definitions("Type: Instant\nReturn target nonland permanent to its owner's hand. If you're in a finale, that permanent's owner puts it on the top or bottom of their library instead.") {
  for designated in [false,true] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let target=game.create_object_from_definition(&body(),B,Zone::Battlefield);
   let stable=game.object(target).unwrap().stable_id;
   if designated {game.grant_player_designation(A,"finale");}
   let source=game.create_object_from_definition(&definition,A,Zone::Stack);
   let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
   let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(target)]);
   let outcome=execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ctx).unwrap();
   let moved=game.find_object_by_stable_id(stable).unwrap();
   assert_eq!(game.object(moved).unwrap().zone,if designated {Zone::Library}else{Zone::Hand});
   assert_eq!(game.object(moved).unwrap().owner,B);
   assert_eq!(game.player(A).unwrap().library.len(),0);
   assert_eq!(game.player(B).unwrap().library.contains(&moved),designated);
   // Native zone commits publish their canonical events through the game
   // queue; effect-local outputs need not repeat those physical events.
   let bounced=outcome.events.iter().chain(game.effect_store.pending_trigger_events.iter()).any(|event|event.downcast::<ironsmith::events::ZoneChangeEvent>().is_some_and(|change|
    change.to==Zone::Hand && change.snapshot.as_ref().is_some_and(|snapshot|snapshot.stable_id==stable)));
   assert_eq!(bounced,!designated,"instead must not perform the original hand move before the library choice");
  }
 }
}

#[test]
fn actual_gilded_sell_sword_uses_only_its_controllers_completed_turn_history(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-golden-age-designation.json")).unwrap();
 let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]=="Gilded Sell-Sword_NJB").unwrap();
 for definition in source_support::definitions(&card["executable_faces"][0]).unwrap(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let base=game.current_power(source).unwrap();
  history(&mut game,7,B,false,"Instant");history(&mut game,3,A,true,"Instant");game.refresh_continuous_state().unwrap();assert!(!game.has_player_designation(A,"renaissance"));assert_eq!(game.current_power(source),Some(base));
  history(&mut game,6,A,false,"Instant");game.refresh_continuous_state().unwrap();assert!(game.has_player_designation(A,"renaissance"));assert_eq!(game.current_power(source),Some(base+2));assert!(!game.has_player_designation(B,"renaissance"));assert!(game.stack.is_empty());
 }
}
