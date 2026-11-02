//! Generic complete-line readers exposed by the full Endgame source cohort.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
fn definitions(text:&str)->[CardDefinition;2]{
 let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Source witness",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);let indirect=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();assert_eq!(ironsmith_text::compiled_text_lines(&direct),ironsmith_text::compiled_text_lines(&indirect));[direct,indirect]
}
fn game()->GameState{let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;game}
fn actions(game:&GameState,source:ObjectId)->Vec<LegalAction>{compute_legal_actions(game,A).unwrap().into_iter().filter(|action|matches!(action,LegalAction::ActivateAbility{source:id,..}if *id==source)||matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).collect()}
fn announce(game:&mut GameState,action:LegalAction){
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}assert!(!state.has_pending_action());
}
#[test]
fn complete_named_count_definition_owns_its_words_with_any_comma_spacing(){
 for separator in [", ",","," , "]{for definition in definitions(&format!("Type: Artifact\nIf you control 0 or more tapped permanents{separator}you are in your finale for the rest of the game.")){
  let mut game=game();game.create_object_from_definition(&definition,A,Zone::Battlefield);game.refresh_continuous_state().unwrap();assert!(game.has_player_designation(A,"finale"));assert!(!game.has_player_designation(A,"endgame"));
 }}
}
#[test]
fn conditional_graveyard_cast_names_only_its_source_and_uses_the_named_marker(){
 for source in ["this card","this creature","Source witness"]{for definition in definitions(&format!("Mana cost: {{1}}\nType: Creature\nPower/Toughness: 2/2\nYou may cast {source} from your graveyard as long as you're in your finale.")){
  let mut game=game();let id=game.create_object_from_definition(&definition,A,Zone::Graveyard);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);assert!(actions(&game,id).is_empty());game.grant_player_designation(A,"endgame");assert!(actions(&game,id).is_empty());game.grant_player_designation(A,"finale");assert!(!actions(&game,id).is_empty());
  let other=ironsmith_compiler_runtime::compile_to_runtime_definition("Unrelated card","Mana cost: {1}\nType: Creature\nPower/Toughness: 1/1",false).unwrap();let other=game.create_object_from_definition(&other,A,Zone::Graveyard);assert!(actions(&game,other).is_empty());
 }}
}
#[test]
fn trailing_as_long_as_reduction_prices_only_the_authored_equip_ability(){
 for definition in definitions("Type: Artifact — Equipment\nEquip {4}. This ability costs {3} less to activate as long as you're in your finale."){
  let mut game=game();let equipment=game.create_object_from_definition(&definition,A,Zone::Battlefield);let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Equip target","Type: Creature\nPower/Toughness: 1/1",false).unwrap();game.create_object_from_definition(&body,A,Zone::Battlefield);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);assert!(actions(&game,equipment).is_empty());game.grant_player_designation(A,"endgame");assert!(actions(&game,equipment).is_empty());game.grant_player_designation(A,"finale");assert!(!actions(&game,equipment).is_empty());
 }
}
#[test]
fn active_only_lexical_variant_keeps_the_actual_activation_condition(){
 for verb in ["Activate","Active"]{for definition in definitions(&format!("Type: Artifact\n{{0}}: You gain 1 life. {verb} only if you're in your finale.")){
  let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);assert!(actions(&game,source).is_empty());game.grant_player_designation(A,"finale");let action=actions(&game,source).into_iter().next().unwrap();announce(&mut game,action);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().life,21);
 }}
}
#[test]
fn hard_wrapped_duration_keeps_its_activation_and_conditioned_followup(){
 for definition in definitions("Type: Creature\nPower/Toughness: 1/1\n{0}: This creature gets +1/+0 until\nend of turn. If you're in your finale, it also gains double strike until end of turn."){
  for marked in [false,true]{let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);if marked{game.grant_player_designation(A,"finale");}let action=actions(&game,source).into_iter().next().unwrap();announce(&mut game,action);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.current_power(source),Some(2));assert_eq!(game.current_has_static_ability_id(source,ironsmith_core::StaticAbilityId::DoubleStrike),marked);}
 }
}
