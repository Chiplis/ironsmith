//! Full source permissions are exercised through current typed rules and the
//! combat-damage assignment owner, independently of their storage shape.
#[path="p09_common/mod.rs"] mod common;
use ironsmith::{GameState,ObjectId,PlayerId,Zone,Target};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn permitted(game:&GameState,id:ObjectId)->bool{
 game.object_has_static_ability_id(id,ironsmith::static_abilities::StaticAbilityId::MayAssignDamageAsUnblocked)
  ||game.effect_store.cant_effects.object_has_rule(id,&ironsmith_core::ObjectRule::MayAssignCombatDamageAsThoughUnblocked)
}
#[test]
fn unblocked_assignment_is_granted_to_your_creatures(){
 let rows=common::rows(include_str!("../../../fixtures/granted_unblocked_assignment.json.fixture"));assert_eq!(rows.len(),2);
 for row in &rows{let name=row["name"].as_str().unwrap();for definition in common::definitions(row){for enabled in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;
  let source=game.create_object_from_definition(&definition,A,if name=="Predatory Focus"{Zone::Stack}else{Zone::Battlefield});
  let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Own attacker","Type: Creature\nPower/Toughness: 2/3",false).unwrap();let attacker=game.create_object_from_definition(&body,A,Zone::Battlefield);
  let block=ironsmith_compiler_runtime::compile_to_runtime_definition("Opposing blocker","Type: Creature\nPower/Toughness: 0/10",false).unwrap();let blocker=game.create_object_from_definition(&block,B,Zone::Battlefield);
  let mut combat=ironsmith::combat_state::CombatState::default();combat.attackers.push(ironsmith::combat_state::AttackerInfo{creature:attacker,target:ironsmith::combat_state::AttackTarget::Player(B)});combat.blockers.insert(attacker,vec![blocker]);
  let source_damage=if name=="Siege Behemoth"&&enabled{combat.attackers.push(ironsmith::combat_state::AttackerInfo{creature:source,target:ironsmith::combat_state::AttackTarget::Player(B)});game.current_power(source).unwrap()}else{0};
  game.combat=Some(combat.clone());
  if name=="Predatory Focus"&&enabled{
   let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();
  }
  game.refresh_continuous_state().unwrap();assert_eq!(permitted(&game,attacker),enabled,"{name}: actual actor-scoped permission");assert!(!permitted(&game,blocker),"{name}: opposing creatures cannot receive your permission");
  let prompt=ironsmith::game_loop::next_combat_damage_assignment_prompt(&game,&combat,false,None);
  if enabled{
   let prompt=prompt.expect("a blocked attacker may choose the defending-player alternative");assert_eq!(prompt.source,attacker);assert_eq!(prompt.player,A);assert!(prompt.division.targets().contains(&Target::Player(B)));
   let choice=[(Target::Player(B),2)];prompt.validate(&game,&choice).unwrap();prompt.record(&mut game,&choice);
  }else{assert!(prompt.is_none(),"one blocker and no permission has no player-damage alternative");}
  ironsmith::game_loop::try_execute_combat_damage_step(&mut game,&combat,false).unwrap();
  assert_eq!(game.player(B).unwrap().life,20-source_damage-if enabled{2}else{0},"{name}: chosen combat damage reaches the defender only when permitted");assert_eq!(game.damage_on(blocker),if enabled{0}else{2});
  if enabled{
   if name=="Predatory Focus"{game.next_turn();}else{game.combat.as_mut().unwrap().attackers.retain(|entry|entry.creature!=source);}
   game.refresh_continuous_state().unwrap();assert!(!permitted(&game,attacker),"{name}: permission expires with its printed turn/attacking condition");
  }
 }}}
}
