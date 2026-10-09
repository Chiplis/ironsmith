//! "When you next activate an exhaust ability that isn't a mana ability this
//! turn, copy it" (CR 702.177a, CR 707.10). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const PIT_AUTOMATON: &str = "Mana cost: {2}\nType: Artifact Creature — Construct\nPower/Toughness: 0/4\nDefender\n{T}: Add {C}{C}. Spend this mana only to activate abilities.\n{2}, {T}: When you next activate an exhaust ability that isn't a mana ability this turn, copy it. You may choose new targets for the copy.";
const DYNAHEIR: &str = "Mana cost: {2}{U}{R}\nType: Legendary Creature — Human Wizard\nPower/Toughness: 3/3\nHaste\n{T}: When you next activate an ability that isn't a mana ability this turn by spending four or more mana to activate it, copy it. You may choose new targets for the copy.";

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
fn pit_automaton_schedules_a_one_shot_exhaust_only_copy() {
    for definition in definitions("Pit Automaton", PIT_AUTOMATON) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("\"exhaust\""), "exhaust marker: {debug}");
        assert!(debug.contains("non_mana_only: true"), "{debug}");
        assert!(debug.contains("one_shot: true"), "{debug}");
        assert!(debug.contains("CopySpell"), "copy of the triggering ability: {debug}");
        assert!(
            !debug.contains("ManaSpentToActivateAtLeast"),
            "no mana requirement on the exhaust form: {debug}"
        );
    }
}

#[test]
fn dynaheir_keeps_its_mana_spent_requirement_without_a_marker() {
    for definition in definitions("Dynaheir, Invoker Adept", DYNAHEIR) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("TriggeringAbilityManaSpentToActivateAtLeast(4)"), "{debug}");
        assert!(!debug.contains("\"exhaust\""), "{debug}");
    }
}
