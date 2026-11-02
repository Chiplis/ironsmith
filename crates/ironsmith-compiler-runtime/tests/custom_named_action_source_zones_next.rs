//! A source-zone-qualified ordinary named event functions from that exact zone.
use ironsmith::{Effect,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::decision::SelectFirstDecisionMaker;
const A:PlayerId=PlayerId(0);
fn pairs(text:&str)->[ironsmith::CardDefinition;2]{
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Witness",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[test]
fn source_while_graveyard_clause_preserves_functional_zone_and_positive_receipt_gate(){
 let text="Type: Creature — Zombie Soldier\nPower/Toughness: 2/2\nWhenever you perform the support action with an amount of 1 or more while this card is in your graveyard, you gain 1 life.";
 for definition in pairs(text){
  let ability=definition.abilities.iter().find(|ability|matches!(ability.kind,ironsmith::ability::AbilityKind::Triggered(_))).unwrap();assert_eq!(ability.functional_zones,vec![Zone::Graveyard]);
  for zone in [Zone::Battlefield,Zone::Graveyard,Zone::Hand,Zone::Exile]{for count in [0,1]{
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&definition,A,zone);let producer=game.create_object_from_definition(&pairs("Type: Artifact")[0],A,Zone::Battlefield);
   let tag:ironsmith::tag::TagKey="selected".into();let mut ctx=EffectContext::new_default(producer,A);let objects=if count==0{vec![]}else{vec![ironsmith::snapshot::ObjectSnapshot::from_object(game.object(producer).unwrap(),&game)]};ctx.tagged_objects.insert(tag.clone(),objects);
   execute_effect(&mut game,&Effect::new(ironsmith::effects::PerformNamedActionEffect{label:"support".into(),performer:None,objects:Some(ironsmith::target::ChooseSpec::Tagged(tag)),prior_objects:None}),&mut ctx).unwrap();
   let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(queue.entries.len(),usize::from(zone==Zone::Graveyard&&count==1));
  }}
 }
}
