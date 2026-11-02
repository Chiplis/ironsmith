//! Independent named designations share the existing lasting achievement operation.
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
#[path="custom_final_next_wave_support.rs"] mod source_support;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Designation reader", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    assert_eq!(ironsmith_text::compiled_text_lines(&direct), ironsmith_text::compiled_text_lines(&restored));
    [direct, restored]
}
fn permanent(game: &mut GameState, controller: PlayerId, tapped: bool) -> ironsmith::ObjectId {
    let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Counted field", "Type: Land", false).unwrap();
    let id = game.create_object_from_definition(&body, controller, Zone::Battlefield);
    if tapped { game.tap(id); }
    id
}
const REMINDER: &str = "If you control eight or more tapped permanents, you are in your endgame for the rest of the game.";
#[test]
fn permanent_designation_counts_tapped_controlled_permanents_and_persists() {
    for definition in definitions(&format!("Type: Creature\nPower/Toughness: 1/1\n{REMINDER}\nThis creature gets +3/+3 as long as you are in your endgame.")) {
        let mut game = GameState::new(vec!["A".into(),"B".into()],20);
        let source = game.create_object_from_definition(&definition,A,Zone::Battlefield);
        for _ in 0..7 { permanent(&mut game,A,true); }
        for _ in 0..10 { permanent(&mut game,B,true); }
        let eighth = permanent(&mut game,A,false);
        game.refresh_continuous_state().unwrap();
        assert!(!game.has_player_designation(A,"endgame"));
        assert_eq!(game.current_power(source),Some(1));
        assert!(!game.has_player_designation(B,"endgame"));
        assert!(game.stack.is_empty());
        game.tap(eighth);
        game.refresh_continuous_state().unwrap();
        assert!(game.has_player_designation(A,"endgame"));
        assert_eq!(game.current_power(source),Some(4),"designation reapplies continuous effects immediately");
        assert!(!game.has_player_designation(A,"renaissance"));
        assert!(!game.has_citys_blessing(A));
        game.remove_object(source);
        game.remove_object(eighth);
        game.refresh_continuous_state().unwrap();
        assert!(game.has_player_designation(A,"endgame"));
        assert!(game.stack.is_empty(),"designation is a static check");
    }
}
#[test]
fn spell_designation_checks_only_at_resolution_and_conditions_read_the_same_marker() {
    for definition in definitions(&format!("Type: Sorcery\n{REMINDER}\nIf you are in your endgame, draw a card.")) {
        let mut game = GameState::new(vec!["A".into(),"B".into()],20);
        for _ in 0..8 { permanent(&mut game,A,true); }
        permanent(&mut game,B,true);
        let library = ironsmith_compiler_runtime::compile_to_runtime_definition("Drawn field", "Type: Land", false).unwrap();
        game.create_object_from_definition(&library,A,Zone::Library);
        let source = game.create_object_from_definition(&definition,A,Zone::Stack);
        game.refresh_continuous_state().unwrap();
        assert!(!game.has_player_designation(A,"endgame"),"stack spell cannot grant a static designation");
        let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
        execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A)).unwrap();
        assert!(game.has_player_designation(A,"endgame"));
        assert!(!game.has_player_designation(B,"endgame"));
        assert_eq!(game.player(A).unwrap().hand.len(),1);
    }
}
#[test]
fn arbitrary_designation_names_remain_independent_and_spell_negative_count() {
    for name in ["finale","renaissance"] {for count in [7,8] {
        let text = format!("Type: Sorcery\nIf you control eight or more tapped permanents, you are in your {name} for the rest of the game.");
        for definition in definitions(&text) {
            let mut game = GameState::new(vec!["A".into(),"B".into()],20);
            for _ in 0..count {permanent(&mut game,A,true);}
            let source=game.create_object_from_definition(&definition,A,Zone::Stack);
            let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
            execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A)).unwrap();
            assert_eq!(game.has_player_designation(A,name),count==8);
            assert!(!game.has_player_designation(A,"endgame"));
        }
    }}
}
#[test]
fn complete_source_endgame_variants_preserve_artifact_and_native_programs() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-endgame-designation.json")).unwrap();
 let mut records=0;let mut faces=0;let mut keys=std::collections::BTreeSet::new();let mut failures=Vec::new();
 for row in fixture["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){
  records+=1;let key=card["source_card_key"].as_str().or_else(||card["id"].as_str()).unwrap();keys.insert(key.trim_start_matches("rev:"));let label=format!("{} / {key}",row["id"]);
  if card["rewrite_status"]!="frontend_materialized_unverified"{failures.push(format!("{label}: not materialized"));}
  let Some(inputs)=card["executable_faces"].as_array() else{failures.push(format!("{label}: missing executable faces"));continue};
  if inputs.len()!=card["source_faces"].as_array().unwrap().len(){failures.push(format!("{label}: incomplete original face union"));}
  for input in inputs {faces+=1;match source_support::definitions(input){
   Ok([direct,indirect])=>assert_eq!(ironsmith_text::compiled_text_lines(&direct),ironsmith_text::compiled_text_lines(&indirect),"{label}"),
   Err(error)=>failures.push(format!("{label} / {}: {error}",input["name"]))
  }}
 }}
 assert_eq!(records,28);assert_eq!(keys.len(),28);assert_eq!(faces,28);
 assert!(failures.is_empty(),"{} exact Endgame source failures:\n{}",failures.len(),failures.join("\n"));
}
fn actual_endgame_source(key:&str)->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-endgame-designation.json")).unwrap();
 let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"].as_str().or_else(||card["id"].as_str()).unwrap().trim_start_matches("rev:")==key).unwrap();
 source_support::definitions(&card["executable_faces"][0]).unwrap()
}
#[test]
fn actual_bodyguard_checks_own_tapped_count_continuously_and_retains_the_achievement(){
 for definition in actual_endgame_source("Bodyguard Bruisers_VRD"){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let base=game.current_power(source).unwrap();
  for _ in 0..7{permanent(&mut game,A,true);}for _ in 0..8{permanent(&mut game,B,true);}
  let eighth=permanent(&mut game,A,false);game.refresh_continuous_state().unwrap();
  assert!(!game.has_player_designation(A,"endgame"));assert_eq!(game.current_power(source),Some(base));
  game.tap(eighth);game.refresh_continuous_state().unwrap();assert!(game.has_player_designation(A,"endgame"));assert_eq!(game.current_power(source),Some(base+2));assert!(!game.has_player_designation(B,"endgame"));
  game.remove_object(eighth);game.refresh_continuous_state().unwrap();assert!(game.has_player_designation(A,"endgame"));assert_eq!(game.current_power(source),Some(base+2));assert!(game.stack.is_empty());
 }
}
