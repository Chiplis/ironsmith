use ironsmith_compiled_artifact::CompiledCardArtifact;
#[test]
fn prefix_frequency_lowers_to_the_existing_trigger_limit_with_artifact_parity() {
    for (word, cap) in [("Once", 1), ("Twice", 2)] {
        let text = format!("Type: Creature — Fox\nPower/Toughness: 2/2\n{word} each turn, when another creature you control enters, draw a card.");
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true,true,|| ironsmith_compiler_runtime::compile_to_artifact("Guide", &text, false)));
        assert!(!loss.is_lossy(),"{}",loss.reasons_text());
        let (artifact, direct) = result.unwrap();
        let artifact = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap();
        for definition in [direct, materialized] {
            let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger), _ => None}).unwrap();
            assert_eq!(trigger.intervening_if, Some(ironsmith::effect::Condition::MaxTimesEachTurn(cap)));
        }
    }
    assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Guide", "Type: Creature — Fox\nPower/Toughness: 2/2\nOnce each turn, draw a card.", false).is_err());
}
