//! All immutable Revolution Integrate sources, admitted through actual frontend metadata.
#[path = "custom_final_next_wave_support.rs"] mod support;
#[test]
fn all_twenty_five_actual_sources_compile_without_loss_direct_and_serialized() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-integrate-complete.json")).unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for row in fixture["mechanics"].as_array().unwrap() {
        for card in row["representative_cards"].as_array().unwrap() {
            assert!(card["executable_text"].as_str().is_some(), "parent must materialize actual frontend first: {}", card["id"]);
            support::definitions(card).unwrap_or_else(|error| panic!("{}: {error}", card["id"]));
            keys.insert(card["source_card_key"].as_str().unwrap());
        }
    }
    assert_eq!(keys.len(), 25);
}

#[path = "cf8_p08/support.rs"] mod compiled;
use ironsmith::{Effect, Zone};
use ironsmith::effects::{EffectContext,execute_effect,LinkExiledCardsEffect};
use ironsmith::target::ChooseSpec;
const A: ironsmith::PlayerId = ironsmith::PlayerId::from_index(0);
const B: ironsmith::PlayerId = ironsmith::PlayerId::from_index(1);
fn link(game:&mut ironsmith::GameState,host:ironsmith::ObjectId,card:ironsmith::ObjectId){
 let effect=Effect::new(LinkExiledCardsEffect{cards:ChooseSpec::SpecificObject(card),host:ChooseSpec::SpecificObject(host),relationship:"archive".into()});
 execute_effect(game,&effect,&mut EffectContext::new_default(host,A)).unwrap();
}
#[test]
fn relationships_use_exact_zone_incarnations_and_survive_control_change(){
 for definition in compiled::definitions("Ordinary relationship host","Type: Creature\nPower/Toughness: 1/1\nThis creature gets +1/+1 for each card linked to this permanent under the archive relationship."){
  let mut game=ironsmith::GameState::new(vec!["A".into(),"B".into()],20);
  let host=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let card=game.create_object_from_definition(&definition,B,Zone::Exile);
  link(&mut game,host,card);link(&mut game,host,card);
  assert_eq!(game.calculated_power(host),Some(2),"same exact association cannot duplicate");
  game.set_current_controller(host,B).unwrap();
  assert_eq!(game.calculated_power(host),Some(2));
  let returned=game.move_object_by_effect(card,Zone::Graveyard).unwrap();
  let reexiled=game.move_object_by_effect(returned,Zone::Exile).unwrap();
  assert_ne!(reexiled,card);assert_eq!(game.calculated_power(host),Some(1));
  link(&mut game,host,reexiled);assert_eq!(game.calculated_power(host),Some(2));
  let absent=game.move_object_by_effect(host,Zone::Exile).unwrap();
  let new_host=game.move_object_by_effect(absent,Zone::Battlefield).unwrap();
  assert_ne!(new_host,host);assert_eq!(game.calculated_power(new_host),Some(1));
 }
}
#[test]
fn a_card_outside_exile_cannot_be_associated(){
 for definition in compiled::definitions("Admission host","Type: Creature\nPower/Toughness: 1/1\nThis creature gets +1/+1 for each card linked to this permanent under the archive relationship."){
  let mut game=ironsmith::GameState::new(vec!["A".into(),"B".into()],20);
  let host=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let card=game.create_object_from_definition(&definition,A,Zone::Graveyard);
  link(&mut game,host,card);assert_eq!(game.calculated_power(host),Some(1));
 }
}

#[path = "cf8_p08/play.rs"] mod play;
#[test]
fn sacrificed_source_reads_its_last_known_relationship_collection(){
 for definition in compiled::definitions("Sacrificed archive host","Type: Creature\nPower/Toughness: 1/1\n{0}, Sacrifice this creature: You gain 1 life for each card linked to this permanent under the archive relationship."){
  let mut game=play::game();
  let host=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let member=game.create_object_from_definition(&definition,B,Zone::Exile);
  link(&mut game,host,member);
  let mut dm=play::Script::default();
  play::activate(&mut game,host,play::activated_at(&definition,0),&mut dm);
  assert!(!game.battlefield.contains(&host));
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.player(A).unwrap().life,21);
 }
}

const CAPTURE_PROGRAM:&str="Type: Sorcery\nPerform the archive action on the top two cards of your library as follows: [Exile the proposed objects. For each card exiled this way, do the following: [You may do the following: [Choose a nonland permanent; Link the iterated card to the chosen permanent under the archive relationship.]]].";
#[test]
fn pure_boundary_capture_does_not_reveal_or_move_a_prevented_proposal(){
 for definition in compiled::definitions("Preventable collection",CAPTURE_PROGRAM){
  let mut game=play::game();
  let source=game.create_object_from_definition(&definition,A,Zone::Stack);
  let member=game.create_object_from_definition(&definition,A,Zone::Library);
  game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
   source,A,ironsmith::events::other::WouldNamedActionMatcher{
    label:"archive".into(),source_filter:ironsmith::ObjectFilter::default(),performer_filter:Some(ironsmith::PlayerFilter::You),
    object_requirement:Some(ironsmith_core::filter_model::NamedActionObjectRequirement{minimum:1,filter:Box::new(ironsmith::ObjectFilter::default())}),
   },ironsmith::replacement::ReplacementAction::Prevent));
  let mut ctx=EffectContext::new_default(source,A);
  for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects(){
   let outcome=execute_effect(&mut game,effect,&mut ctx).unwrap();
   assert!(!outcome.events.iter().any(|event|event.kind()==ironsmith::events::traits::EventKind::CardRevealed));
  }
  assert_eq!(game.player(A).unwrap().library,vec![member]);
  assert_eq!(game.object(member).unwrap().zone,Zone::Library);
 }
}
#[test]
fn a_short_library_is_taken_completely_and_declining_hosts_does_not_cancel_exile(){
 for definition in compiled::definitions("Short collection",CAPTURE_PROGRAM){
  let mut game=play::game();
  let source=game.create_object_from_definition(&definition,A,Zone::Stack);
  game.create_object_from_definition(&definition,A,Zone::Library);
  let mut dm=play::Script{decline:true,..Default::default()};
  let mut ctx=EffectContext::new(source,A,&mut dm);
  for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
  assert!(game.player(A).unwrap().library.is_empty());
  assert_eq!(game.zone_ids(Zone::Exile).count(),1);
 }
}

fn actual(name:&str)->[ironsmith::CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-integrate-complete.json")).unwrap();
 let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap();
 support::definitions(card).unwrap()
}
#[test]
fn actual_data_weaver_replaces_one_bulk_proposal_and_loses_life_once_per_opponent(){
 let program=CAPTURE_PROGRAM.replace("archive action","integrate action");
 for (weaver,program) in actual("Data Weaver").into_iter().zip(compiled::definitions("Bulk integrate source",&program)){
  let mut game=play::game();
  game.create_object_from_definition(&weaver,A,Zone::Battlefield);
  let source=game.create_object_from_definition(&program,A,Zone::Stack);
  for owner in [A,A,B,play::C]{game.create_object_from_definition(&program,owner,Zone::Library);}
  let mut dm=play::Script{decline:true,..Default::default()};
  let mut ctx=EffectContext::new(source,A,&mut dm);
  for effect in program.spell_effect.as_ref().unwrap().flattened_default_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
  assert_eq!(game.zone_ids(Zone::Exile).count(),4);
  assert_eq!(game.player(B).unwrap().life,19);
  assert_eq!(game.player(play::C).unwrap().life,19);
  assert!(game.player(A).unwrap().library.is_empty());
  assert!(game.player(B).unwrap().library.is_empty());
  assert!(game.player(play::C).unwrap().library.is_empty());
 }
}

#[test]
fn actual_offer_receives_a_creature_relationship_event_once_and_creates_its_token(){
 for definition in actual("Offerings to Sky Gods"){
  let mut game=play::game();
  let host=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let creature=play::vanilla("Related creature","{1}","Elf",2,2);
  let member=game.create_object_from_definition(&creature,B,Zone::Exile);
  for expected in [1,0]{
   let effect=Effect::new(LinkExiledCardsEffect{cards:ChooseSpec::SpecificObject(member),host:ChooseSpec::SpecificObject(host),relationship:"integrated".into()});
   let result=execute_effect(&mut game,&effect,&mut EffectContext::new_default(host,A)).unwrap();
   for event in result.events {game.queue_trigger_event(Default::default(),event);}
   let mut queue=ironsmith::triggers::TriggerQueue::new();
   ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
   assert_eq!(queue.entries.len(),expected);
   ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
   play::resolve_all(&mut game,&mut play::Script::default());
  }
  assert_eq!(game.battlefield.len(),2);
  let token=*game.battlefield.iter().find(|id|**id!=host).unwrap();
  let characteristics=game.current_characteristics(token).unwrap();
  assert!(characteristics.card_types.contains(&ironsmith::types::CardType::Creature));
  assert!(characteristics.card_types.contains(&ironsmith::types::CardType::Artifact));
  assert_eq!(game.calculated_power(token),Some(1));
 }
}

#[test]
fn actual_corpsepyre_inherits_each_supported_keyword_from_related_creature_cards(){
 for definition in actual("Corpsepyre Fiend"){
  let mut game=play::game();
  let host=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let flying=compiled::definitions("Flying donor","Type: Creature\nPower/Toughness: 2/2\nFlying")[0].clone();
  let reach=compiled::definitions("Reach donor","Type: Creature\nPower/Toughness: 2/2\nReach")[0].clone();
  let mut members=Vec::new();
  for donor in [&flying,&reach]{
   let member=game.create_object_from_definition(donor,B,Zone::Exile);members.push(member);
   let effect=Effect::new(LinkExiledCardsEffect{cards:ChooseSpec::SpecificObject(member),host:ChooseSpec::SpecificObject(host),relationship:"integrated".into()});
   execute_effect(&mut game,&effect,&mut EffectContext::new_default(host,A)).unwrap();
  }
  assert!(game.current_has_static_ability_id(host,ironsmith_core::StaticAbilityId::Flying));
  assert!(game.current_has_static_ability_id(host,ironsmith_core::StaticAbilityId::Reach));
  game.set_current_controller(host,B).unwrap();
  assert!(game.current_has_static_ability_id(host,ironsmith_core::StaticAbilityId::Flying));
  game.move_object_by_effect(members[0],Zone::Graveyard).unwrap();
  assert!(!game.current_has_static_ability_id(host,ironsmith_core::StaticAbilityId::Flying));
  assert!(game.current_has_static_ability_id(host,ironsmith_core::StaticAbilityId::Reach));
 }
}

#[test]
fn actual_mirrormeld_copies_all_ability_categories_from_lands_related_to_your_permanents(){
 for definition in actual("Mirrormeld Spire"){
  let mut game=play::game();
  let mirror=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let host=game.create_object_from_definition(&play::vanilla("Nonland host","{1}","Elf",2,2),A,Zone::Battlefield);
  let donor=compiled::definitions("Land ability donor","Type: Land\nCreatures you control get +1/+1.\n{T}: Add {R}.\nWhenever this land becomes tapped, you gain 1 life.")[0].clone();
  let member=game.create_object_from_definition(&donor,B,Zone::Exile);
  let effect=Effect::new(LinkExiledCardsEffect{cards:ChooseSpec::SpecificObject(member),host:ChooseSpec::SpecificObject(host),relationship:"integrated".into()});
  execute_effect(&mut game,&effect,&mut EffectContext::new_default(host,A)).unwrap();
  assert_eq!(game.calculated_power(host),Some(3),"copied static anthem is functional on the recipient");
  let abilities=game.current_characteristics(mirror).unwrap().abilities;
  assert!(abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Triggered(_))));
  assert!(abilities.iter().filter(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Activated(_))).count()>=3);
  game.move_object_by_effect(member,Zone::Graveyard).unwrap();
  assert_eq!(game.calculated_power(host),Some(2));
  assert!(!game.current_characteristics(mirror).unwrap().abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Triggered(_))));
 }
}
