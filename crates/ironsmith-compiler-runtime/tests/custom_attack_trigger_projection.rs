//! An explicit event projection matches retained abilities, never creates attackers.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone,Effect};
use ironsmith::combat_state::{CombatState,AttackTarget};
use ironsmith::decision::{AttackerDeclaration,SelectFirstDecisionMaker};
use ironsmith::game_loop::{apply_attacker_declarations,put_triggers_on_stack_with_dm,resolve_stack_entry_with};
use ironsmith::effects::{execute_effect,EffectContext};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions()->[CardDefinition;2]{let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();let row=fixture["mechanics"].as_array().unwrap().iter().find(|m|m["id"]=="msem-939").unwrap();let card=&row["representative_cards"][0];let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap(),false)));assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(decoded,artifact);[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]}
#[test]
fn exact_source_projects_owned_graveyard_creature_abilities_into_original_trigger_batch(){
 for definition in definitions(){for matching_attacker in [false,true]{let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();let provider=game.create_object_from_definition(&definition,A,Zone::Battlefield);
 let learner=ironsmith_compiler_runtime::compile_to_runtime_definition("Retained attack program","Type: Creature\nPower/Toughness: 1/1\nWhenever this creature attacks, you gain 2 life.",false).unwrap();let card=game.create_object_from_definition(&learner,A,Zone::Graveyard);game.create_object_from_definition(&learner,B,Zone::Graveyard);game.create_object_from_definition(&learner,A,Zone::Exile);let other=game.create_object_from_definition(&learner,A,Zone::Battlefield);
 let attacker=if matching_attacker{provider}else{other};game.remove_summoning_sickness(attacker);let mut combat=CombatState::default();let mut queue=TriggerQueue::new();apply_attacker_declarations(&mut game,&mut combat,&mut queue,&[AttackerDeclaration{creature:attacker,target:AttackTarget::Player(B)}]).unwrap();
 assert_eq!(queue.entries.iter().filter(|entry|entry.source==card).count(),usize::from(matching_attacker));assert_eq!(game.turn_store.turn_history.creatures_attacked_this_turn.len(),1);assert!(!game.creature_attacked_this_turn(card));assert_eq!(combat.attackers.len(),1);assert_eq!(game.object(card).unwrap().zone,Zone::Graveyard);
 if matching_attacker{let entry=queue.entries.iter().find(|entry|entry.source==card).unwrap();assert_eq!(entry.source_snapshot.as_ref().unwrap().zone,Zone::Graveyard);execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(card),Zone::Exile,false),&mut EffectContext::new_default(provider,A)).unwrap();}
 put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();while !game.stack.is_empty(){resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}assert_eq!(game.player(A).unwrap().life,22);
 }}
}
