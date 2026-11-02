//! Exact Ada source and ordinary vector consumers; parent owns execution.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
#[path="cf8_p08/play.rs"] mod play;
use ironsmith::{Zone,Target};
#[test]
fn exact_ada_retains_complete_object_packets_for_native_vector_instructions() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();
 let ada=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]=="Ada, the Free_OPH").unwrap();
 let creature=source_routes::definitions(&serde_json::json!({"name":"Packet creature","executable_text":"Type: Creature\nPower/Toughness: 3/3"})).unwrap()[0].clone();
 for provider in source_routes::definitions(ada).unwrap() {for (kind,body) in [
  ("destroy","Destroy target creature."),("exile","Exile target creature."),("return","Return target creature to its owner's hand."),
  ("tap","Tap target creature."),("untap","Untap target creature."),
  ("counter","Put a +1/+1 counter on target creature."),("grant","Target creature gains flying until end of turn."),
 ] {for spell in source_routes::definitions(&serde_json::json!({"name":"Independent vector instruction","executable_text":format!("Mana cost: {{0}}\nType: Instant\n{body}")})).unwrap() {
  let mut game=play::game();game.create_object_from_definition(&provider,play::A,Zone::Battlefield);
  let left=game.create_object_from_definition(&creature,play::B,Zone::Battlefield);
  let right=game.create_object_from_definition(&creature,play::B,Zone::Battlefield);
  let untouched=game.create_object_from_definition(&creature,play::B,Zone::Battlefield);
  let left_stable=game.object(left).unwrap().stable_id;let right_stable=game.object(right).unwrap().stable_id;
  if kind=="untap" {game.tap(left);game.tap(right);game.tap(untouched);}
  let mut dm=play::Script{numbers:vec![2],targets:vec![Target::Object(left),Target::Object(right)],..Default::default()};
  play::cast(&mut game,play::A,&spell,&mut dm);play::resolve_all(&mut game,&mut dm);
  for (id,stable) in [(left,left_stable),(right,right_stable)] {
   match kind {
    "destroy"|"exile"|"return"=>{let actual=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(actual).unwrap().zone,match kind{"destroy"=>Zone::Graveyard,"exile"=>Zone::Exile,_=>Zone::Hand});}
    "tap"=>assert!(game.is_tapped(id)),"untap"=>assert!(!game.is_tapped(id)),
    "counter"=>assert_eq!(game.counter_count(id,ironsmith::object::CounterType::PlusOnePlusOne),1),
    "grant"=>assert!(game.calculated_characteristics(id).unwrap().static_abilities.iter().any(|a|a.id()==ironsmith::static_abilities::StaticAbilityId::Flying)),_=>unreachable!(),
   }
  }
  assert_eq!(game.object(untouched).unwrap().zone,Zone::Battlefield);
  assert_eq!(game.is_tapped(untouched),kind=="untap");
  assert_eq!(game.counter_count(untouched,ironsmith::object::CounterType::PlusOnePlusOne),0);
  assert!(!game.calculated_characteristics(untouched).unwrap().static_abilities.iter().any(|a|a.id()==ironsmith::static_abilities::StaticAbilityId::Flying));
 }}}
}
