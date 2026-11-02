//! An unbounded next-event clause uses an ordinary spell filter and one-shot delayed registration.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions()->[CardDefinition;2]{
 let text="Type: Instant\nYou gain 1 life. When you next cast a blue spell, you gain 1 life.";
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Future matching event",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
#[test]
fn next_matching_cast_is_one_shot_without_a_turn_expiry_and_ignores_wrong_player_and_color(){
 for definition in definitions(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Stack);let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();let mut ctx=EffectContext::new_default(source,A);for effect in effects{execute_effect(&mut game,&effect,&mut ctx).unwrap();}
  assert_eq!(game.effect_store.delayed_triggers.len(),1);
  for (color,caster,count) in [("{U}",B,0),("{R}",A,0),("{U}",A,1),("{U}",A,0)]{
   game.turn.turn_number+=2;
   let spell=ironsmith_compiler_runtime::compile_to_runtime_definition("Cast witness",&format!("Mana cost: {color}\nType: Instant\nYou gain 1 life."),false).unwrap();let id=game.create_object_from_definition(&spell,caster,Zone::Stack);
   let event=ironsmith::triggers::TriggerEvent::new(ironsmith::events::SpellCastEvent::new(id,caster,Zone::Hand),Default::default());
   let triggers=ironsmith::triggers::check_delayed_triggers(&mut game,&event);assert_eq!(triggers.len(),count,"caster={caster:?}, color={color}");
  }
  assert!(game.effect_store.delayed_triggers.is_empty());
 }
}
