//! A named Equipment token with two adjacent quoted rules whose second rule
//! contains a comma (CR 111.4). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const ICINGDEATH: &str = "Mana cost: {4}{W}{W}\nType: Legendary Creature — Dragon\nPower/Toughness: 4/5\nFlying, vigilance\nWhen Icingdeath, Frost Tyrant dies, create Icingdeath, Frost Tongue, a legendary white Equipment artifact token with \"Equipped creature gets +2/+0,\" \"Whenever equipped creature attacks, tap target creature defending player controls,\" and equip {2}.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition),
            "{name}: unimplemented content"
        );
    }
    [direct, decoded]
}

#[test]
fn icingdeath_token_keeps_both_quoted_rules_and_equip() {
    for definition in definitions("Icingdeath, Frost Tyrant", ICINGDEATH) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("Frost Tongue"), "{debug}");
        assert!(debug.contains("Equip"), "{debug}");
        assert!(debug.contains("Tap"), "attack trigger taps: {debug}");
    }
}
