//! Experience is an ordinary intervening-if attack ability, including its exact producer class.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::object::CounterType;
const A:PlayerId=PlayerId(0); const B:PlayerId=PlayerId(1);
const EXPERIENCE:&str="experience — Whenever this creature attacks, if it has fewer than three +1/+1 counters on it, put a +1/+1 counter on it.";
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/experience-source-complete.json")).unwrap()}
fn source(name:&str)->serde_json::Value {
 fixture()["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap_or_else(||panic!("missing actual source {name}")).clone()
}
fn definitions(text:&str)->[CardDefinition;2] {
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Independent learning witness",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let wire=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&wire).unwrap()]
}
fn drain(game:&mut GameState)->usize {
 let mut q=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut q,&mut SelectFirstDecisionMaker).unwrap();let n=q.entries.len();ironsmith::game_loop::put_triggers_on_stack(game,&mut q).unwrap();n
}
fn resolve(game:&mut GameState){ironsmith::game_loop::resolve_stack_entry_with(game,&mut SelectFirstDecisionMaker).unwrap();}
fn attack(game:&mut GameState,id:ObjectId){
 game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);
 let mut combat=ironsmith::combat_state::CombatState::default();combat.attackers.push(ironsmith::combat_state::AttackerInfo{creature:id,target:ironsmith::combat_state::AttackTarget::Player(B)});game.combat=Some(combat);
 game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(id,ironsmith::events::AttackEventTarget::Player(B)),Default::default()));
}
fn ordinary_counter(game:&mut GameState,source:ObjectId,recipient:ObjectId){
 let outcome=ironsmith::effects::execute_effect(game,&ironsmith::Effect::new(ironsmith::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne,1,ironsmith::target::ChooseSpec::SpecificObject(recipient))),&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();for event in outcome.events{game.queue_trigger_event(Default::default(),event);}
}
#[test]
fn all_sixty_one_physical_sources_and_eighty_faces_are_strict_on_both_routes(){
 let data=fixture();let mut physical=0;let mut faces=0;let mut failures=Vec::new();
 for row in data["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){physical+=1;faces+=card["source_faces"].as_array().unwrap().len();
  match source_support::linked_definitions(card){Ok(pair)=>assert_eq!(pair.len(),card["source_faces"].as_array().unwrap().len()),Err(e)=>failures.push(format!("{} {}: {e}",row["scope"],card["id"]))}
 }}assert_eq!((physical,faces),(61,80));assert!(failures.is_empty(),"{}",failures.join("\n"));
}
#[test]
fn experience_threshold_is_rechecked_and_is_not_a_counter_cap_or_implicit_transform(){
 for noun in ["+1/+1","level"] {let counter=if noun=="level"{CounterType::Level}else{CounterType::PlusOnePlusOne};
 let body=format!("Type: Creature\nPower/Toughness: 2/2\nindependent learning — Whenever this creature attacks, if it has fewer than three {noun} counters on it, put a {noun} counter on it.");
 for def in definitions(&body){for (initial,before_resolution,expected) in [(0,0,1),(2,2,3),(3,3,3),(2,3,3)]{
 let mut g=GameState::new(vec!["A".into(),"B".into()],20);let id=g.create_object_from_definition(&def,A,Zone::Battlefield);g.object_mut(id).unwrap().counters.insert(counter,initial);attack(&mut g,id);assert_eq!(drain(&mut g),usize::from(initial<3));g.object_mut(id).unwrap().counters.insert(counter,before_resolution);if !g.stack.is_empty(){resolve(&mut g);}assert_eq!(g.counter_count(id,counter),expected);assert_eq!(g.object(id).unwrap().name,def.card.name);
 }}}
}
#[test]
fn actual_maternal_monster_observes_only_the_retained_experience_producer_and_does_not_recurse(){
 let observers=source_support::definitions(&source("Maternal Monster")).unwrap();
 for (route,learner) in definitions(&format!("Type: Creature\nPower/Toughness: 2/2\n{EXPERIENCE}")).into_iter().enumerate(){
 let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.create_object_from_definition(&observers[route],A,Zone::Battlefield);let id=g.create_object_from_definition(&learner,A,Zone::Battlefield);
 ordinary_counter(&mut g,id,id);assert_eq!(drain(&mut g),0,"a current source marker cannot classify an ordinary instruction");g.remove_counters(id,CounterType::PlusOnePlusOne,1,None,None);
 g.object_mut(id).unwrap().counters.insert(CounterType::PlusOnePlusOne,2);attack(&mut g,id);assert_eq!(drain(&mut g),1);resolve(&mut g);assert_eq!(g.counter_count(id,CounterType::PlusOnePlusOne),3);assert_eq!(drain(&mut g),1,"exact experience producer is retained through native/wire program execution");resolve(&mut g);assert_eq!(g.counter_count(id,CounterType::PlusOnePlusOne),4,"ordinary bonus can exceed three");assert_eq!(drain(&mut g),0,"bonus is not itself an Experience ability");
 }}
#[test]
fn granting_a_designated_program_keeps_recipient_ownership_and_classification(){
 let observers=source_support::definitions(&source("Maternal Monster")).unwrap();
 let text=format!("Type: Enchantment\nCreatures you control have custom keyword \"experience\", and \"{EXPERIENCE}\"");
 for (route,grant) in definitions(&text).into_iter().enumerate(){let mut g=GameState::new(vec!["A".into(),"B".into()],20);let aura=g.create_object_from_definition(&grant,A,Zone::Battlefield);g.create_object_from_definition(&observers[route],A,Zone::Battlefield);let body=definitions("Type: Creature\nPower/Toughness: 2/2");let recipient=g.create_object_from_definition(&body[route],A,Zone::Battlefield);
 attack(&mut g,recipient);assert_eq!(drain(&mut g),1);resolve(&mut g);assert_eq!(drain(&mut g),1);resolve(&mut g);assert_eq!(g.counter_count(recipient,CounterType::PlusOnePlusOne),2);assert_eq!(g.counter_count(aura,CounterType::PlusOnePlusOne),0);assert_eq!(drain(&mut g),0);
 }}

fn settle_bounded(g:&mut GameState){
 for _ in 0..8{drain(g);if g.stack.is_empty(){return}resolve(g);}panic!("ordinary source triggers failed to settle");
}
#[test]
fn actual_grinding_retains_queued_recipient_experience_after_the_aura_leaves(){
 let gifts=source_support::definitions(&source("Grinding")).unwrap();let observers=source_support::definitions(&source("Maternal Monster")).unwrap();let bodies=definitions("Type: Creature\nPower/Toughness: 2/2");
 for route in 0..2{let mut g=GameState::new(vec!["A".into(),"B".into()],20);let host=g.create_object_from_definition(&bodies[route],A,Zone::Battlefield);g.create_object_from_definition(&observers[route],A,Zone::Battlefield);let gift=g.create_object_from_definition(&gifts[route],A,Zone::Battlefield);
 assert!(g.attach_object_to_target(gift,ironsmith::object::AttachmentTarget::Object(host)));attack(&mut g,host);assert_eq!(drain(&mut g),1);
 let outcome=ironsmith::effects::execute_effect(&mut g,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(gift),Zone::Graveyard,false),&mut ironsmith::effects::EffectContext::new_default(gift,A)).unwrap();for event in outcome.events{g.queue_trigger_event(Default::default(),event);}
 resolve(&mut g);settle_bounded(&mut g);assert_eq!(g.counter_count(host,CounterType::PlusOnePlusOne),2,"queued recipient ability retains its producer class after grant removal");
 attack(&mut g,host);assert_eq!(drain(&mut g),0,"future attacks no longer have the granted program");
 }}
#[test]
fn actual_copperhide_removes_levels_only_when_its_printed_state_trigger_transforms_it(){
 let faces=source_support::linked_definitions(&source("Copperhide Calf")).unwrap();assert_eq!(faces.len(),2);
 for route in 0..2{let mut g=GameState::new(vec!["A".into(),"B".into()],20);for pair in &faces{g.register_linked_face_definition(&pair[route]);}let id=g.create_object_from_definition(&faces[0][route],A,Zone::Battlefield);
 g.add_counters(id,CounterType::Level,2).unwrap();assert_eq!(g.calculated_power(id),Some(3));attack(&mut g,id);settle_bounded(&mut g);
 assert_eq!(g.current_name(id).as_deref(),Some("Verdigris Behemoth"));assert_eq!(g.counter_count(id,CounterType::Level),0);assert_eq!(g.counter_count(id,CounterType::PlusOnePlusOne),0);assert_eq!(g.calculated_power(id),Some(5));
 for expected in 1..=3{attack(&mut g,id);settle_bounded(&mut g);assert_eq!(g.counter_count(id,CounterType::Level),expected);assert_eq!(g.current_name(id).as_deref(),Some("Verdigris Behemoth"));}
 assert_eq!(g.calculated_power(id),Some(7));attack(&mut g,id);assert_eq!(drain(&mut g),0,"back keeps the same ordinary attack threshold");
 }}
#[test]
fn actual_forgotten_temple_shares_the_whole_program_only_for_its_own_linked_creature_card(){
 let temples=source_support::definitions(&source("Forgotten Temple")).unwrap();
 let learners=source_support::definitions(&serde_json::json!({"name":"Independent archived learner","custom_keyword_identities":["experience"],"executable_text":format!("Type: Creature\nPower/Toughness: 2/2\n{EXPERIENCE}")})).unwrap();
 let artifacts=source_support::definitions(&serde_json::json!({"name":"Independent false recipient","custom_keyword_identities":["experience"],"executable_text":"Type: Artifact"})).unwrap();
 for route in 0..2{let mut g=GameState::new(vec!["A".into(),"B".into()],20);let temple=g.create_object_from_definition(&temples[route],A,Zone::Battlefield);let other=g.create_object_from_definition(&temples[route],A,Zone::Battlefield);
 let has_program=|game:&GameState,id|game.current_abilities(id).unwrap().iter().any(|a|a.has_designation("experience")&&matches!(a.kind,ironsmith::ability::AbilityKind::Triggered(_)));
 assert!(!has_program(&g,temple));let noncreature=g.create_object_from_definition(&artifacts[route],A,Zone::Exile);g.add_exiled_with_source_link(temple,noncreature);assert!(!has_program(&g,temple),"custom membership on a noncreature card is insufficient");
 let card=g.create_object_from_definition(&learners[route],A,Zone::Exile);g.add_exiled_with_source_link(other,card);assert!(!has_program(&g,temple));assert!(has_program(&g,other));g.add_exiled_with_source_link(temple,card);assert!(has_program(&g,temple));assert!(!g.current_is_creature(temple),"the conditional grant does not animate the land");
 let outcome=ironsmith::effects::execute_effect(&mut g,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(card),Zone::Hand,false),&mut ironsmith::effects::EffectContext::new_default(temple,A)).unwrap();for event in outcome.events{g.queue_trigger_event(Default::default(),event);}assert!(!has_program(&g,temple));assert!(!has_program(&g,other));
 }}
