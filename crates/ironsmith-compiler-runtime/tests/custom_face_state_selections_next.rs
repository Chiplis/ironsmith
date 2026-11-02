//! Existing face-state effects retain counted selection and targeting semantics.
use ironsmith::{Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::DecisionMaker;
use ironsmith::effects::{EffectContext,ResolvedTarget,execute_effect};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct Select{count:usize,calls:usize}
impl DecisionMaker for Select{
 fn decide_objects(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{self.calls+=1;ctx.candidates.iter().filter(|candidate|candidate.legal).take(self.count).map(|candidate|candidate.id).collect()}
}
fn body()->ironsmith::CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Body","Type: Creature\nPower/Toughness: 3/3",false).unwrap()}
fn pairs(text:&str)->[ironsmith::CardDefinition;2]{
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Face selection",text,false)));assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[test]
fn counted_nontarget_face_up_chooses_exact_friendly_facedown_subset_including_zero(){
 for definition in pairs("Type: Sorcery\nTurn any number of face down creatures you control face up."){for count in [0,1,2]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Stack);let own:Vec<_>=(0..2).map(|_|{let id=game.create_object_from_definition(&body(),A,Zone::Battlefield);game.set_face_down(id);id}).collect();let enemy=game.create_object_from_definition(&body(),B,Zone::Battlefield);game.set_face_down(enemy);let up=game.create_object_from_definition(&body(),A,Zone::Battlefield);let mut dm=Select{count,calls:0};let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
  assert_eq!(own.iter().filter(|id|!game.is_face_down(**id)).count(),count);assert!(game.is_face_down(enemy));assert!(!game.is_face_down(up));
 }}
}
#[test]
fn targeted_face_up_preserves_announced_object_without_a_second_resolution_choice(){
 for definition in pairs("Type: Sorcery\nTurn target face down creature face up."){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Stack);let own=game.create_object_from_definition(&body(),A,Zone::Battlefield);game.set_face_down(own);let enemy=game.create_object_from_definition(&body(),B,Zone::Battlefield);game.set_face_down(enemy);let mut dm=Select{count:0,calls:0};let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(enemy)]).with_decision_maker(&mut dm)).unwrap();assert_eq!(dm.calls,0);assert!(!game.is_face_down(enemy));assert!(game.is_face_down(own));
 }
}
