//! A finite color loop owns a lexical object-quality binding, not a chosen color.
use ironsmith::{GameState,PlayerId,Zone,CardDefinition};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::decision::DecisionMaker;
const A:PlayerId=PlayerId(0);
struct Decisions(bool);
impl DecisionMaker for Decisions{
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.0}
 fn decide_objects(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ironsmith::ObjectId>{ctx.candidates.iter().filter(|object|object.legal).take(ctx.min).map(|object|object.id).collect()}
}
fn pairs(text:&str)->[CardDefinition;2]{let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Finite color source",text,false)));assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]}
#[test]
fn bounded_optional_object_program_runs_independently_for_each_actual_color(){
 let text="Type: Sorcery\nFor each color, you may do the following: [Exile a nonlegendary card of that color from your graveyard; You gain 1 life].";
 for definition in pairs(text){for accept in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  for cost in ["{W}","{U}","{B}","{R}","{G}"]{let material=pairs(&format!("Mana cost: {cost}\nType: Creature\nPower/Toughness: 2/2"))[0].clone();game.create_object_from_definition(&material,A,Zone::Graveyard);}
  let colorless=pairs("Mana cost: {1}\nType: Creature\nPower/Toughness: 2/2")[0].clone();game.create_object_from_definition(&colorless,A,Zone::Graveyard);
  let legendary=pairs("Mana cost: {R}\nType: Legendary Creature\nPower/Toughness: 2/2")[0].clone();game.create_object_from_definition(&legendary,A,Zone::Graveyard);
  let source=game.create_object_from_definition(&definition,A,Zone::Stack);let mut decision=Decisions(accept);let mut ctx=EffectContext::new_default(source,A).with_decision_maker(&mut decision);
  for effect in definition.spell_effect.as_ref().unwrap().all_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
  assert_eq!(game.player(A).unwrap().graveyard.len(),if accept{2}else{7});
  assert_eq!(game.player(A).unwrap().life,if accept{25}else{20},"later body instructions must remain inside each optional color scope");
  assert!(game.chosen_color(source).is_none());
 }}
}
