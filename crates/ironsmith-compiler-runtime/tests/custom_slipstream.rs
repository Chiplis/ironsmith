//! Universal identity predicates compose with ordinary conditional evasion.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::combat_state::{AttackTarget,CombatState};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::game_loop::apply_attacker_declarations;
use ironsmith::triggers::TriggerQueue;
use ironsmith_compiled_artifact::{ArtifactCardId,ArtifactCardIdentity,CompiledCardArtifact,CompiledCardPayload,wire_definition_from_serializable};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
const RULE:&str="This creature can't be blocked as long as each attacking creature has custom keyword slipstream.";
fn definitions(name:&str,text:&str,identity:bool)->[CardDefinition;2] {
 let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),name);
 let builder=if identity{builder.with_custom_keyword("slipstream")}else{builder};
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler::CompilerFacade::new().compile_definition(builder,text,ironsmith_compiler::CompilePolicy{allow_unsupported:false})));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let compiled=result.unwrap();
 let wire=wire_definition_from_serializable(&compiled.definition).unwrap();let direct=ironsmith_compiler_runtime::into_runtime_definition(compiled.definition.clone()).unwrap();
 let artifact=CompiledCardArtifact::new(ArtifactCardIdentity{local_id:ArtifactCardId(direct.card.id.0),name:direct.card.name.clone(),face_name:None,other_face:None,linked_face_layout:None},CompiledCardPayload{definition:wire,canonical_text:ironsmith_text::compiled_text_lines(&direct).join("\n"),ability_labels:ironsmith_text::ability_surface_texts(&direct)},"universal-keyword-condition",text.as_bytes());
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn body()->CardDefinition {ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary body","Type: Creature — Merfolk\nPower/Toughness: 2/2",false).unwrap()}
fn combat(game:&mut GameState,attackers:&[ironsmith::ObjectId])->CombatState {
 for id in attackers {game.remove_summoning_sickness(*id);}
 game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();
 let mut combat=CombatState::default();let mut queue=TriggerQueue::new();let declarations=attackers.iter().map(|id|AttackerDeclaration{creature:*id,target:AttackTarget::Player(B)}).collect::<Vec<_>>();apply_attacker_declarations(game,&mut combat,&mut queue,&declarations).unwrap();combat
}
#[test]
fn all_current_attackers_must_have_identity_and_nonattackers_do_not_count() {
 for definition in definitions("Universal evasion",&format!("Type: Creature — Merfolk\nPower/Toughness: 2/2\n{RULE}"),true) {for mixed in [false,true] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let ordinary=body();
  let attacker=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let other=game.create_object_from_definition(if mixed{&ordinary}else{&definition},A,Zone::Battlefield);
  game.create_object_from_definition(&ordinary,A,Zone::Battlefield);game.create_object_from_definition(&ordinary,B,Zone::Battlefield);
  let blocker=game.create_object_from_definition(&ordinary,B,Zone::Battlefield);let combat=combat(&mut game,&[attacker,other]);
  assert_eq!(ironsmith::rules::combat::can_block(game.object(attacker).unwrap(),game.object(blocker).unwrap(),&game),mixed);
  let legal=ironsmith::decision::compute_legal_blockers(&game,&combat,B);let attacker_options=legal.iter().find(|options|options.attacker==attacker).unwrap();
  assert_eq!(attacker_options.valid_blockers.contains(&blocker),mixed);
  if !mixed {
   std::sync::Arc::make_mut(&mut game.object_mut(other).unwrap().abilities).clear();
   assert!(ironsmith::rules::combat::can_block(game.object(attacker).unwrap(),game.object(blocker).unwrap(),&game),"loss of another attacker's identity immediately restores blocking");
  }
 }}
}
#[test]
fn granted_identity_is_recipient_relative_and_expires_with_the_granted_program() {
 use ironsmith::ability::AbilityKind;use ironsmith::effects::{EffectContext,ResolvedTarget,execute_effect};
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-slipstream-next.json")).unwrap();
 let card=&fixture["mechanics"][2]["representative_cards"][0];
 for source in definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap(),false) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let ordinary=body();let grantor=game.create_object_from_definition(&source,A,Zone::Battlefield);let attacker=game.create_object_from_definition(&ordinary,A,Zone::Battlefield);let blocker=game.create_object_from_definition(&ordinary,B,Zone::Battlefield);
  let effects=source.abilities.iter().find_map(|a|match &a.kind{AbilityKind::Activated(a)=>Some(a.effects.all_effects_owned()),_=>None}).unwrap();
  let mut context=EffectContext::new_default(grantor,A).with_targets(vec![ResolvedTarget::Object(attacker)]);for effect in &effects{execute_effect(&mut game,effect,&mut context).unwrap();}
  combat(&mut game,&[attacker]);assert!(!ironsmith::rules::combat::can_block(game.object(attacker).unwrap(),game.object(blocker).unwrap(),&game));
  let grantor_abilities=game.calculated_characteristics(grantor).unwrap().abilities.clone();assert!(!grantor_abilities.iter().any(|a|matches!(&a.kind,AbilityKind::Static(s) if s.custom_keyword_name()==Some("slipstream"))));
  game.effect_store.continuous_effects.cleanup_end_of_turn();assert!(ironsmith::rules::combat::can_block(game.object(attacker).unwrap(),game.object(blocker).unwrap(),&game));
 }
}
#[test]
fn source_token_program_keeps_identity_and_universal_evasion() {
 use ironsmith::ability::AbilityKind;use ironsmith::effects::{EffectContext,execute_effect};
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-slipstream-next.json")).unwrap();let card=&fixture["mechanics"][1]["representative_cards"][0];
 for source in definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap(),false) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let ordinary=body();let creator=game.create_object_from_definition(&source,A,Zone::Battlefield);let blocker=game.create_object_from_definition(&ordinary,B,Zone::Battlefield);
  let effects=source.abilities.iter().find_map(|a|match &a.kind{AbilityKind::Triggered(a)=>Some(a.effects.all_effects_owned()),_=>None}).unwrap();let mut ctx=EffectContext::new_default(creator,A);for effect in &effects{execute_effect(&mut game,effect,&mut ctx).unwrap();}
  let token=*game.battlefield.iter().find(|id|game.object(**id).unwrap().has_subtype(ironsmith::types::Subtype::Jellyfish)).unwrap();combat(&mut game,&[token]);
  assert!(!ironsmith::rules::combat::can_block(game.object(token).unwrap(),game.object(blocker).unwrap(),&game));
 }
}
