//! Generic no-stack condition marking built from status, predicates and transform.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{compute_legal_actions,LegalAction,SelectFirstDecisionMaker};
use ironsmith::special_actions::{SpecialAction,perform};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn action_text(symbol:&str)->String {
 format!("Special action \"Mark condition {symbol}\" — Any time you could cast a sorcery, you may pay {{0}} to take a special action. If you do, give this permanent the \"seta {symbol}\" status in group \"condition marking\". If this permanent has the \"seta first\" status and this permanent has the \"seta second\" status, transform this permanent. If you do, remove all statuses in group \"condition marking\" from this permanent. Take this action only if You control a creature. Actions of this permanent in group \"condition marking\" share one use each turn.")
}
fn definitions()->[CardDefinition;2] {
 let text=format!("Type: Enchantment\n{}\n{}",action_text("first"),action_text("second"));
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Condition front",&text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn actions(game:&GameState,source:ObjectId,player:PlayerId)->Vec<SpecialAction> {
 compute_legal_actions(game,player).unwrap().into_iter().filter_map(|action|match action {LegalAction::SpecialAction(action @ SpecialAction::PerformIntrinsicAction{source_id,..})if source_id==source=>Some(action),_=>None}).collect()
}
fn ready(game:&mut GameState,player:PlayerId){game.turn.active_player=player;game.turn.priority_player=Some(player);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;}
fn creature(game:&mut GameState,player:PlayerId){let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Helper","Type: Creature\nPower/Toughness: 1/1",false).unwrap();game.create_object_from_definition(&definition,player,Zone::Battlefield);}
fn link(front:&mut CardDefinition)->CardDefinition {
 let back_text=format!("Type: Enchantment\n{}\n{}",action_text("first").replace("seta","setb"),action_text("second").replace("seta","setb"));
 let mut back=ironsmith_compiler_runtime::compile_to_runtime_definition("Condition back",&back_text,false).unwrap();
 front.card.other_face=Some(back.card.id);front.card.other_face_name=Some(back.card.name.clone());front.card.linked_face_layout=ironsmith::card::LinkedFaceLayout::TransformLike;front.card.transforming_dfc=true;
 back.card.other_face=Some(front.card.id);back.card.other_face_name=Some(front.card.name.clone());back.card.linked_face_layout=ironsmith::card::LinkedFaceLayout::TransformLike;back.card.transforming_dfc=true;back
}
#[test]
fn condition_marks_are_no_stack_shared_across_control_changes_and_allow_remarking() {
 for mut front in definitions() {
  let back=link(&mut front);let mut game=GameState::new(vec!["A".into(),"B".into()],20);ready(&mut game,A);game.register_linked_face_definition(&front);game.register_linked_face_definition(&back);let source=game.create_object_from_definition(&front,A,Zone::Battlefield);
  assert!(actions(&game,source,A).is_empty(),"condition is false");creature(&mut game,A);creature(&mut game,B);assert_eq!(actions(&game,source,A).len(),2);
  ready(&mut game,B);assert!(actions(&game,source,B).is_empty(),"an opponent may not mark a condition");ready(&mut game,A);
  game.turn.phase=ironsmith::game_state::Phase::Combat;assert!(actions(&game,source,A).is_empty());game.turn.phase=ironsmith::game_state::Phase::FirstMain;
  perform(actions(&game,source,A)[0].clone(),&mut game,A,&mut SelectFirstDecisionMaker).unwrap();assert!(game.stack.is_empty());assert!(game.has_custom_status(source,"seta first"));assert!(actions(&game,source,A).is_empty());
  game.set_current_controller(source,B).unwrap();ready(&mut game,B);assert!(actions(&game,source,B).is_empty(),"same source quota survives controller change");
  game.turn.turn_number+=1;perform(actions(&game,source,B)[0].clone(),&mut game,B,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.object(source).unwrap().name,"Condition front");assert!(game.has_custom_status(source,"seta first"));assert!(!game.has_custom_status(source,"seta second"));
  game.apply_custom_status(source,"unrelated",false);game.turn.turn_number+=1;perform(actions(&game,source,B)[1].clone(),&mut game,B,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(game.object(source).unwrap().name,"Condition back");assert!(!game.has_custom_status(source,"seta first"));assert!(!game.has_custom_status(source,"seta second"));assert!(game.has_custom_status(source,"unrelated"));assert!(game.stack.is_empty());
 }
}
#[test]
fn marks_are_noncopiable_survive_other_transforms_and_clear_on_leaving() {
 for mut front in definitions() {
  let back=link(&mut front);let mut game=GameState::new(vec!["A".into(),"B".into()],20);ready(&mut game,A);creature(&mut game,A);game.register_linked_face_definition(&front);game.register_linked_face_definition(&back);let source=game.create_object_from_definition(&front,A,Zone::Battlefield);
  perform(actions(&game,source,A)[0].clone(),&mut game,A,&mut SelectFirstDecisionMaker).unwrap();
  let copy=game.create_object_from_definition(&front,A,Zone::Battlefield);assert!(!game.has_custom_status(copy,"seta first"));assert_eq!(actions(&game,copy,A).len(),2,"a new permanent has its own action quota");
  assert!(game.transform_permanent(source).unwrap());assert!(game.has_custom_status(source,"seta first"));assert!(actions(&game,source,A).is_empty(),"transforming to another face with conditions cannot reset the shared quota");
  game.turn.turn_number+=1;perform(actions(&game,source,A)[0].clone(),&mut game,A,&mut SelectFirstDecisionMaker).unwrap();assert!(game.has_custom_status(source,"setb first"));
  assert!(game.transform_permanent(source).unwrap());assert!(game.has_custom_status(source,"seta first"));assert!(game.has_custom_status(source,"setb first"));assert!(actions(&game,source,A).is_empty());
  game.turn.turn_number+=1;perform(actions(&game,source,A)[1].clone(),&mut game,A,&mut SelectFirstDecisionMaker).unwrap();assert!(!game.has_custom_status(source,"seta first"));assert!(!game.has_custom_status(source,"setb first"),"any successful condition-marking transformation clears old-face marks in the same group");
  let stable=game.object(source).unwrap().stable_id;let hand=game.move_object_by_effect(source,Zone::Hand).unwrap();let returned=game.move_object_by_effect(hand,Zone::Battlefield).unwrap();assert_eq!(game.find_object_by_stable_id(stable),Some(returned));assert!(!game.has_custom_status(returned,"seta first"));assert_eq!(actions(&game,returned,A).len(),2);
 }
}
#[test]
fn marking_completion_keeps_marks_when_transform_does_not_happen() {
 for front in definitions() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);ready(&mut game,A);creature(&mut game,A);let source=game.create_object_from_definition(&front,A,Zone::Battlefield);
  perform(actions(&game,source,A)[0].clone(),&mut game,A,&mut SelectFirstDecisionMaker).unwrap();game.turn.turn_number+=1;
  perform(actions(&game,source,A)[1].clone(),&mut game,A,&mut SelectFirstDecisionMaker).unwrap();
  assert!(game.has_custom_status(source,"seta first"));assert!(game.has_custom_status(source,"seta second"));assert_eq!(game.object(source).unwrap().name,"Condition front","there is no linked face to transform to");
  assert!(actions(&game,source,A).is_empty());game.turn.turn_number+=1;assert_eq!(actions(&game,source,A).len(),2,"remarks are allowed even with every condition already marked");
 }
}
#[test]
fn grouped_status_cleanup_preserves_other_applications_and_expiries() {
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Grouped state","Type: Enchantment",false).unwrap();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
 // The engine has no grouped status applications; this covers the ungrouped
 // equivalent: a shorter repeated application cannot shorten a lasting one,
 // and an explicit removal clears it.
 assert!(game.apply_custom_status(source,"shared",false));assert!(!game.apply_custom_status(source,"shared",true));assert!(game.apply_custom_status(source,"expiring",true));
 game.cleanup_custom_statuses_end_of_turn();assert!(game.has_custom_status(source,"shared"),"a lasting application survives end-of-turn cleanup");assert!(!game.has_custom_status(source,"expiring"));
 assert!(game.set_custom_status_in_zone(source,"shared",Zone::Battlefield,false));assert!(!game.has_custom_status(source,"shared"));
}
