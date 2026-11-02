//! Rule-equivalent recipient-relative Revolution trigger programs.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::object::CounterType;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
const EXPERIENCE:&str="Whenever this creature attacks, if it has two or fewer +1/+1 counters on it, put a +1/+1 counter on it.";
fn definitions(text:&str)->[CardDefinition;2] {
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Reusable trigger witness",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn creature(game:&mut GameState,owner:PlayerId)->ObjectId {
 let def=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent recipient","Type: Creature — Elf\nPower/Toughness: 2/3\nVigilance",false).unwrap();game.create_object_from_definition(&def,owner,Zone::Battlefield)
}
fn stack(game:&mut GameState)->usize {
 let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();let n=queue.entries.len();ironsmith::game_loop::put_triggers_on_stack(game,&mut queue).unwrap();n
}
fn settle(game:&mut GameState) {stack(game);while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(game,&mut SelectFirstDecisionMaker).unwrap();}}
fn attack(game:&mut GameState,object:ObjectId) {
 game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.turn.active_player=A;
 let mut combat=ironsmith::combat_state::CombatState::default();combat.attackers.push(ironsmith::combat_state::AttackerInfo{creature:object,target:ironsmith::combat_state::AttackTarget::Player(B)});game.combat=Some(combat);
 game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(object,ironsmith::events::AttackEventTarget::Player(B)),Default::default()));
}
fn die(game:&mut GameState,object:ObjectId)->ObjectId {
 let stable=game.object(object).unwrap().stable_id;let mut ctx=ironsmith::effects::EffectContext::new_default(object,A);
 let result=ironsmith::effects::execute_effect(game,&ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(object),Zone::Graveyard,false)),&mut ctx).unwrap();
 for event in result.events{game.queue_trigger_event(Default::default(),event);}game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn experience_checks_threshold_at_trigger_and_resolution_on_the_recipient() {
 for grant in [false,true] {let text=if grant{format!("Type: Enchantment\nCreatures you control have custom keyword \"growwise\", and \"{EXPERIENCE}\"")}else{format!("Type: Creature\nPower/Toughness: 2/2\n{EXPERIENCE}")};
 for definition in definitions(&text){for (initial,at_resolution,expected_triggers,expected) in [(0,0,1,1),(2,2,1,3),(3,3,0,3),(2,3,1,3)]{
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let grantor=game.create_object_from_definition(&definition,A,Zone::Battlefield);let recipient=if grant{creature(&mut game,A)}else{grantor};game.object_mut(recipient).unwrap().counters.insert(CounterType::PlusOnePlusOne,initial);
 attack(&mut game,recipient);assert_eq!(stack(&mut game),expected_triggers,"grant={grant}, initial={initial}, at_resolution={at_resolution}");game.object_mut(recipient).unwrap().counters.insert(CounterType::PlusOnePlusOne,at_resolution);while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}assert_eq!(game.counter_count(recipient,CounterType::PlusOnePlusOne),expected);if grant{assert_eq!(game.counter_count(grantor,CounterType::PlusOnePlusOne),0);}
 }}}
}
#[test]
fn decoy_preserves_noncreature_characteristics_and_exiles_only_next_end_step() {
 let body="Whenever this creature attacks, create a tapped and attacking token that's a copy of this creature, except it's a 1/1 Illusion instead of its other creature types. Exile that token at the beginning of the next end step.";
 for definition in definitions(&format!("Type: Artifact Creature — Elf Equipment\nPower/Toughness: 4/5\nFlying\n{body}")){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);attack(&mut game,source);settle(&mut game);
 let token=*game.battlefield.iter().find(|id|**id!=source).expect("copy token");let characteristics=game.current_characteristics(token).unwrap();assert_eq!(game.calculated_power(token),Some(1));assert_eq!(game.calculated_toughness(token),Some(1));assert!(characteristics.card_types.contains(&ironsmith::types::CardType::Artifact));assert!(characteristics.subtypes.contains(&ironsmith::types::Subtype::Illusion));assert!(!characteristics.subtypes.contains(&ironsmith::types::Subtype::Elf));assert!(characteristics.subtypes.contains(&ironsmith::types::Subtype::Equipment));assert!(game.current_has_static_ability_id(token,ironsmith_core::StaticAbilityId::Flying));assert!(game.is_tapped(token));assert!(ironsmith::combat_state::is_attacking(game.combat.as_ref().unwrap(),token));
 game.combat=None;assert_eq!(game.object(token).unwrap().zone,Zone::Battlefield);game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(A),Default::default()));settle(&mut game);assert!(game.object(token).is_none_or(|object|object.zone!=Zone::Battlefield));assert_eq!(game.object(source).unwrap().zone,Zone::Battlefield);
 }
}
#[test]
fn rapture_restrictions_end_when_the_returned_objects_haze_is_removed() {
 let body="When this creature dies, return it to the battlefield with a haze counter on it. For as long as it has a haze counter on it, it loses all abilities and can't block.";
 for definition in definitions(&format!("Type: Creature\nPower/Toughness: 3/2\nFlying\n{body}")){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;die(&mut game,source);settle(&mut game);let returned=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(returned).unwrap().zone,Zone::Battlefield);let haze=*game.object(returned).unwrap().counters.keys().find(|counter|counter.description().to_lowercase().contains("haze")).expect("haze counter");assert_eq!(game.counter_count(returned,haze),1);assert!(!game.current_has_static_ability_id(returned,ironsmith_core::StaticAbilityId::Flying));let attacker=creature(&mut game,B);assert!(!ironsmith::rules::combat::can_block(game.object(attacker).unwrap(),game.object(returned).unwrap(),&game));game.remove_counters(returned,haze,1,None,None);assert!(game.current_has_static_ability_id(returned,ironsmith_core::StaticAbilityId::Flying));assert!(ironsmith::rules::combat::can_block(game.object(attacker).unwrap(),game.object(returned).unwrap(),&game));
 }
}
#[test]
fn glory_uses_attacking_last_known_information_and_returns_under_trigger_controller() {
 let body="When this creature dies, if it was attacking, return it to the battlefield transformed under your control.";
 for mut front in definitions(&format!("Type: Creature\nPower/Toughness: 4/4\n{body}")){for attacking in [false,true]{
 let mut back=ironsmith_compiler_runtime::compile_to_runtime_definition("Glorious transformed face","Type: Enchantment\nCreatures you control get +1/+1.",false).unwrap();front.card.other_face=Some(back.card.id);front.card.other_face_name=Some(back.card.name.to_string());front.card.linked_face_layout=ironsmith::card::LinkedFaceLayout::TransformLike;back.card.other_face=Some(front.card.id);back.card.other_face_name=Some(front.card.name.to_string());back.card.linked_face_layout=ironsmith::card::LinkedFaceLayout::TransformLike;
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.register_linked_face_definition(&front);game.register_linked_face_definition(&back);let source=game.create_object_from_definition(&front,B,Zone::Battlefield);game.set_current_controller(source,A).unwrap();let stable=game.object(source).unwrap().stable_id;if attacking{let mut combat=ironsmith::combat_state::CombatState::default();combat.attackers.push(ironsmith::combat_state::AttackerInfo{creature:source,target:ironsmith::combat_state::AttackTarget::Player(B)});game.combat=Some(combat);}die(&mut game,source);settle(&mut game);let object=game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap();assert_eq!(object.zone,if attacking{Zone::Battlefield}else{Zone::Graveyard});if attacking{assert_eq!(game.current_controller(object.id),Some(A));assert_eq!(object.name,"Glorious transformed face");}
 }}
}

#[test]
fn explicit_copy_subtype_family_survives_artifact_roundtrip_and_preserves_only_its_family() {
 for (suffix,preserve) in [("instead of its other creature types",true),("instead of its other types",false)] {
  let text=format!("Type: Sorcery\nCreate a token that's a copy of target creature, except it's a 1/1 Illusion {suffix}.");
  for definition in definitions(&text) {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let host_def=ironsmith_compiler_runtime::compile_to_runtime_definition("Different subtype families","Type: Artifact Creature — Elf Equipment\nPower/Toughness: 4/4",false).unwrap();
   let host=game.create_object_from_definition(&host_def,A,Zone::Battlefield);let spell=game.create_object_from_definition(&definition,A,Zone::Stack);
   let mut context=ironsmith::effects::EffectContext::new_default(spell,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(host)]);
   for effect in definition.spell_effect.as_ref().unwrap().all_effects(){ironsmith::effects::execute_effect(&mut game,effect,&mut context).unwrap();}
   let token=*game.battlefield.iter().find(|id|**id!=host).expect("created copy");let characteristics=game.current_characteristics(token).unwrap();
   assert!(characteristics.subtypes.contains(&ironsmith::types::Subtype::Illusion));assert!(!characteristics.subtypes.contains(&ironsmith::types::Subtype::Elf));assert_eq!(characteristics.subtypes.contains(&ironsmith::types::Subtype::Equipment),preserve);
  }
 }
}
