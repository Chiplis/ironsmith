//! Excess redirection keeps current recipient thresholds and source/recipient scopes.
#[path="custom_final_next_wave_support.rs"] mod source;
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::effects::{DealDamageEffect,EffectContext,execute_effect};
use ironsmith::target::ChooseSpec;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);const C:PlayerId=PlayerId(2);
const RULE:&str="Excess noncombat damage that would be dealt by sources you control to permanents an opponent controls is dealt to that player instead.";
fn definitions()->[CardDefinition;2]{source::definitions(&serde_json::json!({"name":"Excess redirection control","executable_text":format!("Type: Enchantment\n{RULE}")})).unwrap()}
fn body(name:&str,text:&str)->CardDefinition {ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap()}
fn setup(rule:&CardDefinition)->(GameState,ObjectId,ObjectId){let mut g=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let rule=g.create_object_from_definition(rule,A,Zone::Battlefield);let damage=g.create_object_from_definition(&body("Damage source","Type: Creature\nPower/Toughness: 3/3"),A,Zone::Battlefield);(g,rule,damage)}
fn hit(g:&mut GameState,source:ObjectId,target:ChooseSpec,amount:i32,combat:bool,unpreventable:bool){let controller=g.current_controller(source).unwrap();execute_effect(g,&Effect::new(DealDamageEffect::new(amount,target).with_combat(combat).with_unpreventable(unpreventable)),&mut EffectContext::new_default(source,controller)).unwrap();}
#[test]
fn actual_complete_linked_source_redirects_only_excess_to_the_damaged_permanents_controller(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();
 let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|r|r["representative_cards"].as_array().unwrap()).find(|c|c["source_card_key"]=="Blade of the First Stare//Scourge of the Last Dance_ETF").unwrap();
 assert!(card["source_faces"].to_string().contains("Excess noncombat damage"));
 let routes=source::linked_definitions(card).unwrap();assert_eq!(routes.len(),2);
 for definition in &routes[1]{let(mut g,_,s)=setup(definition);let victim=g.create_object_from_definition(&body("Victim","Type: Creature\nPower/Toughness: 2/2"),B,Zone::Battlefield);hit(&mut g,s,ChooseSpec::SpecificObject(victim),7,false,false);assert_eq!(g.damage_on(victim),2);assert_eq!(g.player(B).unwrap().life,15);assert_eq!(g.player(C).unwrap().life,20);}
}
#[test]
fn source_controller_noncombat_opponent_and_excess_gates_are_independent(){for definition in definitions(){for mode in 0..5{let(mut g,_,s)=setup(&definition);let owner=if mode==2{A}else{B};let victim=g.create_object_from_definition(&body("Victim","Type: Creature\nPower/Toughness: 2/2"),owner,Zone::Battlefield);let damage_source=if mode==1{g.create_object_from_definition(&body("Other source","Type: Creature\nPower/Toughness: 1/1"),C,Zone::Battlefield)}else{s};let amount=if mode==3{1}else{7};let target=if mode==4{ChooseSpec::Player(ironsmith::PlayerFilter::Specific(B))}else{ChooseSpec::SpecificObject(victim)};hit(&mut g,damage_source,target,amount,mode==0,false);assert_eq!(g.player(B).unwrap().life,if mode==4{13}else{20});assert_eq!(g.damage_on(victim),if mode==4{0}else{amount as u32});}}}
#[test]
fn planeswalker_battle_and_marked_damage_use_live_thresholds_not_printed_toughness(){for definition in definitions(){for mode in 0..3{let(mut g,_,s)=setup(&definition);let text=match mode{0=>"Type: Planeswalker\nLoyalty: 3",1=>"Type: Battle — Siege\nDefense: 3",_=>"Type: Creature\nPower/Toughness: 5/5"};let victim=g.create_object_from_definition(&body("Victim",text),B,Zone::Battlefield);if mode==1{assert!(g.set_battle_protector(victim,C));}if mode==2{hit(&mut g,s,ChooseSpec::SpecificObject(victim),2,false,false);}hit(&mut g,s,ChooseSpec::SpecificObject(victim),7,false,false);assert_eq!(g.player(B).unwrap().life,16);assert_eq!(g.player(C).unwrap().life,20);if mode==2{assert_eq!(g.damage_on(victim),5);}}}}
#[test]
fn deathtouch_lifelink_and_unpreventable_damage_survive_the_split(){for definition in definitions(){let(mut g,_,_)=setup(&definition);let s=g.create_object_from_definition(&body("Lethal source","Type: Creature\nPower/Toughness: 2/2\nDeathtouch\nLifelink"),A,Zone::Battlefield);let victim=g.create_object_from_definition(&body("Victim","Type: Creature\nPower/Toughness: 5/5"),B,Zone::Battlefield);hit(&mut g,s,ChooseSpec::SpecificObject(victim),7,false,true);assert_eq!(g.damage_on(victim),1);assert_eq!(g.player(B).unwrap().life,14);assert_eq!(g.player(A).unwrap().life,27);}}
#[test]
fn a_redirected_chunk_can_be_prevented_without_erasing_the_lethal_remainder(){for definition in definitions(){let(mut g,_,s)=setup(&definition);g.create_object_from_definition(&body("Player shield","Type: Enchantment\nPrevent all damage that would be dealt to you."),B,Zone::Battlefield);let victim=g.create_object_from_definition(&body("Victim","Type: Creature\nPower/Toughness: 2/2"),B,Zone::Battlefield);hit(&mut g,s,ChooseSpec::SpecificObject(victim),7,false,false);assert_eq!(g.damage_on(victim),2);assert_eq!(g.player(B).unwrap().life,20);}}

#[test]
fn stack_spell_sources_match_current_controller_instead_of_owner_or_permanent_domain(){
 for definition in definitions(){for (owner,controller,redirects) in [(A,A,true),(C,A,true),(A,C,false)]{
  let(mut g,_,_)=setup(&definition);
  let spell=g.create_object_from_definition(&body("Stack damage source","Type: Instant\nDraw a card."),owner,Zone::Stack);
  g.set_current_controller(spell,controller).unwrap();
  assert_eq!(g.current_controller(spell),Some(controller));
  assert_eq!(g.object(spell).unwrap().owner,owner);
  let victim=g.create_object_from_definition(&body("Victim","Type: Creature\nPower/Toughness: 2/2"),B,Zone::Battlefield);
  hit(&mut g,spell,ChooseSpec::SpecificObject(victim),7,false,false);
  assert_eq!(g.player(B).unwrap().life,if redirects{15}else{20});
  assert_eq!(g.damage_on(victim),if redirects{2}else{7});
 }}
}
