//! "creates an X/X blue Orb creature token with flying, where X is ..." —
//! the where-X binding ends the token's keyword list and sets its
//! power/toughness (CR 111.4, CR 107.3). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const SPHERE: &str = "Mana cost: {1}{U}\nType: Creature — Illusion\nPower/Toughness: 0/1\nFlying\nAt the beginning of your upkeep, put a +1/+1 counter on this creature, then sacrifice this creature unless you pay {1} for each +1/+1 counter on it.\nWhen this creature leaves the battlefield, target opponent creates an X/X blue Orb creature token with flying, where X is the number of +1/+1 counters on this creature.";

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
fn sphere_orb_token_has_flying_and_counter_sized_body() {
    for definition in definitions("Phantasmal Sphere", SPHERE) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("Orb"), "{debug}");
        assert!(debug.contains("Flying"), "{debug}");
        assert!(debug.contains("PlusOnePlusOne"), "counter-count value: {debug}");
        assert!(!debug.contains("where x"), "where clause is not rules text: {debug}");
    }
}
