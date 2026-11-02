//! Named ability counters use ordinary marker occurrence registration and transfers.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone,Effect};
use ironsmith::object::CounterType;
use ironsmith::effects::{execute_effect,EffectContext};
use ironsmith::target::ChooseSpec;
use ironsmith::decision::{SelectFirstDecisionMaker,AttackerDeclaration};
use ironsmith::combat_state::{CombatState,AttackTarget};
use ironsmith::triggers::TriggerQueue;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn bodies()->[CardDefinition;2] {
 // Names are supplied identities; only compiled reminder programs determine behavior.
 let definitions:Vec<_>=[
  ("distorted","Tapped creatures can block this creature as though they were untapped.\nThis creature can't be blocked by untapped creatures."),
  ("respawn","When this creature dies, if this creature attacked this turn, return this card to its owner's hand."),
  ("custom flight lesson","Flying"),
 ].into_iter().map(|(name,reminder)|(name.to_string(),reminder.to_string())).collect();
 let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions("Counter host","Type: Creature\nPower/Toughness: 3/3",&definitions,false).unwrap();
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(restored,artifact);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn has(game:&GameState,id:ObjectId,name:&str)->bool {
 let chars=game.calculated_characteristics_arc(id).unwrap();
 if name=="respawn" {return chars.abilities.iter().any(|a|matches!(a.kind,ironsmith::ability::AbilityKind::Triggered(_)));}
 chars.static_abilities.iter().any(|a|match name {
  "ward {2}"=>a.ward_cost().is_some(),
  "custom flight lesson"=>a.id()==ironsmith::static_abilities::StaticAbilityId::Flying,
  "distorted"=>a.id()==ironsmith::static_abilities::StaticAbilityId::CanBeBlockedAsThoughBlockersWereUntapped,
  _=>false,
 })
}
#[test]
fn named_payloads_follow_real_counter_transfers_removal_and_placement_prevention() {
 for definition in bodies() {for name in ["ward {2}","distorted","respawn","custom flight lesson"] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let ordinary=ironsmith_compiler_runtime::compile_to_runtime_definition("Unrelated recipient","Type: Creature\nPower/Toughness: 3/3",false).unwrap();let target=game.create_object_from_definition(&ordinary,A,Zone::Battlefield);
  let kind=CounterType::Named(name.into());game.add_counters(source,kind,1).unwrap();assert!(has(&game,source,name));assert!(!has(&game,target,name));
  let transfer=Effect::new(ironsmith::effects::MoveCountersEffect::new(kind,1,ChooseSpec::Source,ChooseSpec::SpecificObject(target)));
  execute_effect(&mut game,&transfer,&mut EffectContext::new_default(source,A)).unwrap();
  assert_eq!(game.counter_count(source,kind),0);assert_eq!(game.counter_count(target,kind),1);assert!(!has(&game,source,name));assert!(has(&game,target,name));
  game.remove_counters(target,kind,1,None,None).unwrap();assert!(!has(&game,target,name));
  let shield=ironsmith_compiler_runtime::compile_to_runtime_definition("No counters","Type: Enchantment\nCreatures you control can't have counters put on them.",false).unwrap();let shield_id=game.create_object_from_definition(&shield,A,Zone::Battlefield);
  execute_effect(&mut game,&Effect::put_counters(kind,1,ChooseSpec::SpecificObject(target)),&mut EffectContext::new_default(source,A)).unwrap();assert_eq!(game.counter_count(target,kind),0);assert!(!has(&game,target,name));
  game.remove_object(shield_id);
  execute_effect(&mut game,&Effect::put_counters(kind,1,ChooseSpec::SpecificObject(target)),&mut EffectContext::new_default(source,A)).unwrap();assert_eq!(game.counter_count(target,kind),1);assert!(has(&game,target,name),"a departed prevention provider cannot leave a stale prohibition");
 }}
}
#[test]
fn respawn_counter_returns_only_a_creature_that_really_attacked() {
 for definition in bodies() {for attacked in [false,true] {for removed in [false,true] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;
  let kind=CounterType::Named("respawn".into());game.add_counters(source,kind,1).unwrap();
  if attacked {game.remove_summoning_sickness(source);game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();let mut combat=CombatState::default();let mut queue=TriggerQueue::new();ironsmith::game_loop::apply_attacker_declarations(&mut game,&mut combat,&mut queue,&[AttackerDeclaration{creature:source,target:AttackTarget::Player(B)}]).unwrap();}
  if removed {game.remove_counters(source,kind,1,None,None).unwrap();}
  execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::Source,Zone::Graveyard,false),&mut EffectContext::new_default(source,A)).unwrap();
  let mut queue=TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(queue.entries.len(),usize::from(attacked&&!removed));
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  let current=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(current).unwrap().zone,if attacked&&!removed{Zone::Hand}else{Zone::Graveyard});
 }}}
}
#[test]
fn arbitrary_supplied_names_have_only_their_compiled_program_and_conflicts_are_atomic() {
 let flight=ironsmith_compiler_runtime::compile_to_runtime_definition("Flight program","Type: Creature\nPower/Toughness: 1/1\nFlying",false).unwrap();
 let reach=ironsmith_compiler_runtime::compile_to_runtime_definition("Reach program","Type: Creature\nPower/Toughness: 1/1\nReach",false).unwrap();
 for name in ["any supplied label","respawn","distorted"] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let kind=CounterType::Named(name.into());
  game.register_named_ability_counter(kind,flight.abilities.clone()).unwrap();
  game.register_named_ability_counter(kind,flight.abilities.clone()).unwrap();
  assert!(game.register_named_ability_counter(kind,reach.abilities.clone()).is_err());
  let vanilla=ironsmith_compiler_runtime::compile_to_runtime_definition("Unrelated host","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
  let source=game.create_object_from_definition(&vanilla,A,Zone::Battlefield);game.add_counters(source,kind,1).unwrap();
  let chars=game.calculated_characteristics_arc(source).unwrap();
  assert!(chars.static_abilities.iter().any(|a|a.id()==ironsmith::static_abilities::StaticAbilityId::Flying));
  assert!(!chars.abilities.iter().any(|a|matches!(a.kind,ironsmith::ability::AbilityKind::Triggered(_))));
  assert!(!chars.static_abilities.iter().any(|a|a.id()==ironsmith::static_abilities::StaticAbilityId::Reach || a.id()==ironsmith::static_abilities::StaticAbilityId::CanBeBlockedAsThoughBlockersWereUntapped));
 }
}
#[test]
fn supplied_counter_programs_obey_layer_six_timestamps() {
 for definition in bodies() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let kind=CounterType::Named("custom flight lesson".into());
  game.add_counters(source,kind,1).unwrap();assert!(has(&game,source,"custom flight lesson"));
  let loss=ironsmith_compiler_runtime::compile_to_runtime_definition("Ability loss","Type: Instant\nTarget creature loses all abilities until end of turn.",false).unwrap();
  let effects=loss.spell_effect.as_ref().unwrap().all_effects_owned();
  execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(source)])).unwrap();
  assert!(!has(&game,source,"custom flight lesson"));
  game.remove_counters(source,kind,1,None,None).unwrap();game.add_counters(source,kind,1).unwrap();
  assert!(has(&game,source,"custom flight lesson"),"a new counter occurrence gains a later timestamp than the ability-loss effect");
 }
}
struct LastLegalOption;
impl ironsmith::decision::DecisionMaker for LastLegalOption {
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{ctx.options.iter().rev().find(|o|o.legal).map(|o|vec![o.index]).unwrap_or_default()}
}
#[test]
fn exact_upgrade_respawn_source_uses_supplied_program_and_real_counter_removal() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-growth-actions.json")).unwrap();
 let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]=="msem-705").unwrap();let card=&row["representative_cards"][0];
 let metadata=card["custom_keyword_counter_definitions"].as_array().expect("source counter definitions must be retained by frontend materialization");
 let supplied:Vec<_>=metadata.iter().map(|d|(d["name"].as_str().unwrap().to_string(),d["reminder"].as_str().unwrap().to_string())).collect();assert!(!supplied.is_empty());
 let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap(),&supplied,false).unwrap();
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {for remove in [false,true] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;
  game.remove_summoning_sickness(source);game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();let mut combat=CombatState::default();let mut queue=TriggerQueue::new();
  ironsmith::game_loop::apply_attacker_declarations(&mut game,&mut combat,&mut queue,&[AttackerDeclaration{creature:source,target:AttackTarget::Player(B)}]).unwrap();
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut LastLegalOption).unwrap();while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut LastLegalOption).unwrap();}
  let kind=CounterType::Named("respawn".into());assert_eq!(game.counter_count(source,kind),1);
  if remove {game.remove_counters(source,kind,1,None,None).unwrap();}
  execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::Source,Zone::Graveyard,false),&mut EffectContext::new_default(source,A)).unwrap();
  let mut queue=TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(queue.entries.len(),usize::from(!remove));
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  let current=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(current).unwrap().zone,if remove{Zone::Graveyard}else{Zone::Hand});
 }}
}

#[test]
fn intrinsic_named_counter_meanings_and_identity_only_programs_cannot_be_redefined(){
 let flight=ironsmith_compiler_runtime::compile_to_runtime_definition("Flight program","Type: Creature\nPower/Toughness: 1/1\nFlying",false).unwrap();
 for name in ["prowess","shadow","ward {2}","hexproof from blue","flying","exalted"] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let kind=CounterType::Named(name.into());assert!(game.register_named_ability_counter(kind,flight.abilities.clone()).is_err(),"intrinsic {name}");
 }
 let definitions=vec![("empty custom counter".to_string(),String::new())];
 assert!(ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions("Provider","Type: Creature\nPower/Toughness: 1/1",&definitions,false).is_err());
}
#[test]
fn names_are_canonical_and_failed_batch_admission_cannot_publish_a_prefix() {
 let supplied=vec![(" Custom   Lesson ".to_string(),"Flying".to_string()),("custom lesson".to_string(),"Flying".to_string())];
 let (artifact,definition)=ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions("Canonical host","Type: Creature\nPower/Toughness: 1/1",&supplied,false).unwrap();
 assert_eq!(artifact.payload.definition.ability_counter_definitions.len(),1);
 assert!(definition.ability_counter_definitions.contains_key("custom lesson"));
 let flight=ironsmith_compiler_runtime::compile_to_runtime_definition("Flight","Type: Creature\nPower/Toughness: 1/1\nFlying",false).unwrap();
 let reach=ironsmith_compiler_runtime::compile_to_runtime_definition("Reach","Type: Creature\nPower/Toughness: 1/1\nReach",false).unwrap();
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 let ordinary=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
 let host=game.create_object_from_definition(&ordinary,A,Zone::Battlefield);
 let staged=CounterType::Named("accepted stage".into());let conflict=CounterType::Named("conflict".into());
 game.add_counters(host,staged,1).unwrap();
 game.register_named_ability_counter(conflict,flight.abilities.clone()).unwrap();
 assert!(game.register_named_ability_counters(&[(staged,flight.abilities.clone()),(conflict,reach.abilities.clone())]).is_err());
 assert!(!game.calculated_characteristics_arc(host).unwrap().static_abilities.iter().any(|a|a.id()==ironsmith::static_abilities::StaticAbilityId::Flying));
 game.register_named_ability_counter(staged,reach.abilities.clone()).expect("failed batch must not reserve or bind the earlier name");
 assert!(game.calculated_characteristics_arc(host).unwrap().static_abilities.iter().any(|a|a.id()==ironsmith::static_abilities::StaticAbilityId::Reach));
 assert!(game.register_named_ability_counter(CounterType::Named(" Noncanonical ".into()),flight.abilities.clone()).is_err());
}
#[test]
fn counter_program_outlives_provider_and_does_not_become_a_copiable_ability() {
 for definition in bodies(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let provider=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let ordinary=ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
  let host=game.create_object_from_definition(&ordinary,A,Zone::Battlefield);
  let kind=CounterType::Named("custom flight lesson".into());game.add_counters(host,kind,1).unwrap();assert!(has(&game,host,"custom flight lesson"));
  game.remove_object(provider);
  assert!(has(&game,host,"custom flight lesson"));
  let before=game.battlefield.iter().copied().collect::<std::collections::HashSet<_>>();
  execute_effect(&mut game,&Effect::create_token_copy(ChooseSpec::SpecificObject(host)),&mut EffectContext::new_default(host,A)).unwrap();
  let copied=*game.battlefield.iter().find(|id|!before.contains(id)).unwrap();
  assert_eq!(game.counter_count(copied,kind),0);assert!(!has(&game,copied,"custom flight lesson"),"counter-granted program is not copiable");
  game.add_counters(copied,kind,1).unwrap();assert!(has(&game,copied,"custom flight lesson"),"new objects retain game vocabulary after provider removal");
  let shield=ironsmith_compiler_runtime::compile_to_runtime_definition("Counter prevention","Type: Enchantment\nCreatures you control can't have counters put on them.",false).unwrap();
  game.create_object_from_definition(&shield,A,Zone::Battlefield);
  execute_effect(&mut game,&Effect::new(ironsmith::effects::MoveCountersEffect::new(kind,1,ChooseSpec::SpecificObject(host),ChooseSpec::SpecificObject(copied))),&mut EffectContext::new_default(host,A)).unwrap();
  assert_eq!(game.counter_count(host,kind),1,"a prohibited transfer cannot remove its source counter");
  assert_eq!(game.counter_count(copied,kind),1);assert!(has(&game,host,"custom flight lesson"));
 }
}
#[test]
fn counter_only_subtype_dependencies_survive_artifacts_without_importing_unused_vocabulary() {
 use ironsmith_core::{Subtype,SubtypeFamily};
 use ironsmith_core::custom_subtypes::{CustomSubtype,CustomSubtypeDefinition};
 let vocabulary=vec![
  CustomSubtypeDefinition{singular:"Shrimp".into(),plural:"Shrimps".into(),family:SubtypeFamily::Creature},
  CustomSubtypeDefinition{singular:"Moonkin".into(),plural:"Moonkin".into(),family:SubtypeFamily::Creature},
 ];
 let supplied=vec![("shrimp lesson".into(),"Shrimp creatures you control get +1/+1.".into())];
 let(artifact,direct)=ironsmith_compiler::with_custom_subtypes(&vocabulary,||ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions("Counter-only vocabulary","Type: Creature\nPower/Toughness: 1/1",&supplied,false)).unwrap().unwrap();
 let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 let shrimp=Subtype::Custom(CustomSubtype::new("shrimp",SubtypeFamily::Creature).unwrap());
 let unused=Subtype::Custom(CustomSubtype::new("moonkin",SubtypeFamily::Creature).unwrap());
 for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()] {
  assert!(definition.card.custom_subtypes.contains(&shrimp));
  assert!(!definition.card.custom_subtypes.contains(&unused));
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let object=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let extracted=game.object(object).unwrap().physical_card_definition();
  assert!(extracted.card.custom_subtypes.contains(&shrimp));
  assert!(!extracted.card.custom_subtypes.contains(&unused));
  assert!(extracted.ability_counter_definitions.contains_key("shrimp lesson"));
  assert!(game.subtypes_in_family(SubtypeFamily::Creature).contains(&shrimp));
  assert!(!game.subtypes_in_family(SubtypeFamily::Creature).contains(&unused));
 }
}
#[test]
fn physical_definition_extraction_and_restart_preserve_only_source_owned_counter_programs() {
 let own=vec![("source lesson".to_string(),"Flying".to_string())];
 let foreign=vec![("temporary lesson".to_string(),"Flying".to_string())];
 let(artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions("Persistent source","Type: Creature\nPower/Toughness: 2/2",&own,false).unwrap();
 let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let card=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  game.add_counters(card,CounterType::Named("source lesson".into()),1).unwrap();
  let stable=game.object(card).unwrap().stable_id;
  let(_,temporary)=ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions("Temporary provider","Type: Creature\nPower/Toughness: 1/1",&foreign,false).unwrap();
  game.create_token_object_from_definition(&temporary,A,Zone::Battlefield);
  let plain=ironsmith_compiler_runtime::compile_to_runtime_definition("Plain source","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
  let plain_id=game.create_object_from_definition(&plain,A,Zone::Library);
  assert!(game.object(plain_id).unwrap().to_card_definition().ability_counter_definitions.is_empty(),"game registry vocabulary must not leak into unrelated objects");
  for extracted in [game.object(card).unwrap().to_card_definition(),game.object(card).unwrap().physical_card_definition()]{
   assert_eq!(extracted.ability_counter_definitions.len(),1);
   assert!(extracted.ability_counter_definitions.contains_key("source lesson"));
   assert!(!extracted.ability_counter_definitions.contains_key("temporary lesson"));
   let mut independent=GameState::new(vec!["A".into(),"B".into()],20);
   let rebuilt=independent.create_object_from_definition(&extracted,A,Zone::Battlefield);
   independent.add_counters(rebuilt,CounterType::Named("source lesson".into()),1).unwrap();
   assert!(independent.calculated_characteristics_arc(rebuilt).unwrap().static_abilities.iter().any(|ability|ability.id()==ironsmith::static_abilities::StaticAbilityId::Flying));
  }
  game.restart_game(A,&[]);
  let restarted=game.find_object_by_stable_id(stable).expect("physical source survives restart");
  assert_eq!(game.counter_count(restarted,CounterType::Named("source lesson".into())),0);
  let extracted=game.object(restarted).unwrap().physical_card_definition();
  assert_eq!(extracted.ability_counter_definitions.len(),1);
  assert!(extracted.ability_counter_definitions.contains_key("source lesson"));
  let reach=ironsmith_compiler_runtime::compile_to_runtime_definition("Replacement vocabulary","Type: Creature\nPower/Toughness: 1/1\nReach",false).unwrap();
  game.register_named_ability_counter(CounterType::Named("temporary lesson".into()),reach.abilities).expect("departed token vocabulary is not a physical card dependency in the new game");
  let moved=game.move_object_by_effect(restarted,Zone::Battlefield).unwrap();
  game.add_counters(moved,CounterType::Named("source lesson".into()),1).unwrap();
  assert!(game.calculated_characteristics_arc(moved).unwrap().static_abilities.iter().any(|ability|ability.id()==ironsmith::static_abilities::StaticAbilityId::Flying));
 }
}
#[test]
fn source_counter_metadata_is_preserved_through_hidden_presentation_without_copying_registry_state() {
 let definitions=vec![("source lesson".to_string(),"Flying".to_string())];
 let(_,definition)=ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions("Disguised source","Type: Creature\nPower/Toughness: 2/2",&definitions,false).unwrap();
 let mut object=ironsmith::object::Object::from_card_definition(ObjectId(7000),&definition,A,Zone::Battlefield);
 object.apply_face_down_cast_overlay();
 let restored=object.physical_card_definition();
 assert_eq!(restored.card.name,"Disguised source");
 assert!(restored.ability_counter_definitions.contains_key("source lesson"));
 let token=ironsmith::object::Object::from_token_definition(ObjectId(7001),&definition,A);
 assert!(token.to_card_definition().ability_counter_definitions.contains_key("source lesson"));
}
#[test]
fn independently_compiled_identical_triggered_counter_programs_share_one_registry_meaning() {
 let program="When this creature dies, return this card to its owner's hand.";
 let supplied=vec![("repeatable return".to_string(),program.to_string())];
 let(_,first)=ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions("First provider","Type: Creature\nPower/Toughness: 1/1",&supplied,false).unwrap();
 let(_,second)=ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions("Second provider","Type: Creature\nPower/Toughness: 2/2",&supplied,false).unwrap();
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 game.create_object_from_definition(&first,A,Zone::Battlefield);
 game.create_object_from_definition(&second,A,Zone::Battlefield);
 assert!(game.register_ability_counter_definitions(&second).is_ok());
}

#[test]
fn bare_keyword_lines_are_not_rewritten_as_the_same_spelled_card_name() {
 for (name,id) in [("Reach",ironsmith::static_abilities::StaticAbilityId::Reach),("Flying",ironsmith::static_abilities::StaticAbilityId::Flying),("Trample",ironsmith::static_abilities::StaticAbilityId::Trample)] {
  let text=format!("Type: Creature\nPower/Toughness: 1/1\n{name}");
  let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact(name,text,false).unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
  for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
   assert!(definition.abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(value) if value.id()==id)),"{name} must remain its ordinary keyword ability");
  }
 }
}
