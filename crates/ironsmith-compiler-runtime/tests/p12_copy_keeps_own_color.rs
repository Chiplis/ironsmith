//! "except it doesn't copy that creature's color and it has \"...this
//! ability.\"" (CR 707.9b): the copy keeps its own colors, both as it enters
//! and when its granted upkeep ability makes it a copy again.
//! Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const VESUVAN: &str = "Mana cost: {3}{U}{U}\nType: Creature — Shapeshifter\nPower/Toughness: 0/0\nYou may have this creature enter as a copy of any creature on the battlefield, except it doesn't copy that creature's color and it has \"At the beginning of your upkeep, you may have this creature become a copy of target creature, except it doesn't copy that creature's color and it has this ability.\"";

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
fn vesuvan_keeps_its_color_on_entry_and_on_each_upkeep_copy() {
    for definition in definitions("Vesuvan Doppelganger", VESUVAN) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("RetainOwnColors"), "entry keeps its color: {debug}");
        assert!(debug.contains("RetainSourceColors"), "upkeep copy keeps its color: {debug}");
        assert!(debug.contains("preserve_source_abilities: true"), "keeps this ability: {debug}");
    }
}
