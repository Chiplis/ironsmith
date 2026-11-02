//! Real source linked admission and exact designation-scoped actual-color exception.
#[path="custom_final_next_wave_support.rs"]mod routes;
#[path="cf8_p08/play.rs"]mod play;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{LegalAction,compute_legal_actions};
use ironsmith::game_loop::resolve_stack_entry_with;
use ironsmith::mana::ManaSymbol;
use play::{A,B};
fn actual(name:&str)->Vec<[CardDefinition;2]>{
 let f:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();
 let c=f["mechanics"].as_array().unwrap().iter().flat_map(|r|r["representative_cards"].as_array().unwrap()).find(|c|c["name"]==name).unwrap();routes::linked_definitions(c).unwrap()
}
fn index(d:&CardDefinition,designation:&str)->usize{d.abilities.iter().position(|a|matches!(&a.kind,AbilityKind::Activated(_))&&a.has_designation(designation)).expect("explicit original activation designation")}
fn legal(g:&GameState,p:PlayerId,s:ObjectId,i:usize)->bool{compute_legal_actions(g,p).unwrap().contains(&LegalAction::ActivateAbility{source:s,ability_index:i})}
#[test]fn full_linked_alluring_and_janneke_sources_admit_with_complete_typed_parity(){
 for name in ["Alluring Rider","Janneke, Surface Scholar"]{let faces=actual(name);assert_eq!(faces.len(),2);for pair in &faces{
  assert_eq!(serde_json::to_value(ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(&pair[0]).unwrap()).unwrap(),serde_json::to_value(ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(&pair[1]).unwrap()).unwrap());
 }for route in 0..2{assert_eq!(faces[0][route].card.other_face,Some(faces[1][route].card.id));assert_eq!(faces[1][route].card.other_face,Some(faces[0][route].card.id));index(&faces[0][route],"attain");}}
}
#[test]fn actual_janneke_waives_only_controlled_attain_color_minimum_and_actual_activation_transforms(){
 let riders=actual("Alluring Rider");let scholars=actual("Janneke, Surface Scholar");
 for route in 0..2{
  let mut g=play::game();for face in riders.iter().chain(scholars.iter()){g.register_linked_face_definition(&face[route]);}
  play::give_mana(&mut g,A,ManaSymbol::Green,10);play::give_mana(&mut g,B,ManaSymbol::Green,10);
  let rider=g.create_object_from_definition(&riders[0][route],A,Zone::Battlefield);g.refresh_continuous_state().unwrap();assert!(g.must_be_blocked(rider),"the intrinsic requirement survives disjoint reader ownership");let other=g.create_object_from_definition(&riders[0][route],B,Zone::Battlefield);let i=index(&riders[0][route],"attain");
  assert!(!legal(&g,A,rider,i),"one actual color cannot satisfy the original three-color rider");
  let provider=g.create_object_from_definition(&scholars[1][route],A,Zone::Battlefield);g.refresh_continuous_state().unwrap();
  assert!(legal(&g,A,rider,i),"actual back-face permission admits mono-green payment after the printed cost reduction");
  g.turn.active_player=B;g.turn.priority_player=Some(B);assert!(!legal(&g,B,other,i),"another controller's attain is outside the permission");g.turn.active_player=A;g.turn.priority_player=Some(A);
  let before=g.player(A).unwrap().mana_pool.total();play::activate(&mut g,rider,i,&mut play::Script::default());
  assert_eq!(before-g.player(A).unwrap().mana_pool.total(),2,"Janneke reduces five mana by three; the printed green pip still costs actual green mana");
  resolve_stack_entry_with(&mut g,&mut play::Script::default()).unwrap();assert_eq!(g.object(rider).unwrap().name,"Orphaned Nuckalavee");
  let second=g.create_object_from_definition(&riders[0][route],A,Zone::Battlefield);g.move_object_by_effect(provider,Zone::Graveyard).unwrap();g.refresh_continuous_state().unwrap();assert!(!legal(&g,A,second,i),"permission ends when its real provider leaves");
 }
}
#[test]fn generic_designated_permission_preserves_other_groups_colored_pips_and_wire_payload(){
 let bodies=routes::definitions(&serde_json::json!({"name":"Declared resource","executable_text":"Type: Artifact\nKeyword ability \"rite\" — {3}: Put a +1/+1 counter on this permanent. Spend mana of at least three different colors to activate this ability.\nKeyword ability \"other\" — {3}: Put a +1/+1 counter on this permanent. Spend mana of at least three different colors to activate this ability.\nKeyword ability \"rite\" — {U}{U}{U}: Put a +1/+1 counter on this permanent. Spend mana of at least three different colors to activate this ability."})).unwrap();
 let permissions=routes::definitions(&serde_json::json!({"name":"Declared spending exception","executable_text":"Type: Enchantment\nYou may spend any number of colors of mana to activate rite abilities of permanents you control."})).unwrap();
 for route in 0..2{
  let mut g=play::game();play::give_mana(&mut g,A,ManaSymbol::Green,3);let body=g.create_object_from_definition(&bodies[route],A,Zone::Battlefield);
  let indices:Vec<_>=bodies[route].abilities.iter().enumerate().filter_map(|(i,a)|matches!(a.kind,AbilityKind::Activated(_)).then_some(i)).collect();assert_eq!(indices.len(),3);
  assert!(!legal(&g,A,body,indices[0]));g.create_object_from_definition(&permissions[route],A,Zone::Battlefield);g.refresh_continuous_state().unwrap();
  assert!(legal(&g,A,body,indices[0]));assert!(!legal(&g,A,body,indices[1]),"same-source unselected designation keeps its minimum");assert!(!legal(&g,A,body,indices[2]),"waiving a color-count rule never converts green mana into blue");
  play::activate(&mut g,body,indices[0],&mut play::Script::default());resolve_stack_entry_with(&mut g,&mut play::Script::default()).unwrap();assert_eq!(g.counter_count(body,ironsmith::CounterType::PlusOnePlusOne),1);
 }
}

#[test]fn permission_wire_retains_exact_exception_and_legacy_payload_has_none(){
 let pair=routes::definitions(&serde_json::json!({"name":"Wire exception","executable_text":"Type: Enchantment\nYou may spend any number of colors of mana to activate rite abilities of permanents you control."})).unwrap();
 let values:Vec<_>=pair.iter().map(|d|serde_json::to_value(ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(d).unwrap()).unwrap()).collect();assert_eq!(values[0],values[1]);
 let permission=ironsmith::effect::ManaSpendPermission::any_color(ironsmith::target::PlayerFilter::You);
 let mut wire=serde_json::to_value(&permission).unwrap();wire.as_object_mut().unwrap().remove("ignore_minimum_colors_for_designation");
 let legacy:ironsmith::effect::ManaSpendPermission=serde_json::from_value(wire).unwrap();assert!(legacy.ignore_minimum_colors_for_designation.is_none());
}
