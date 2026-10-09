//! "turned face up this turn" is a turn-history predicate (CR 708.8), never
//! the permanent's current face-up state. Kaust is the only printed card with
//! the wording; a synthetic target body proves the object-filter route.
//! Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const KAUST: &str = "Mana cost: {R/W}{G}\nType: Legendary Creature — Dryad Detective\nPower/Toughness: 2/2\nWhenever a creature you control that was turned face up this turn deals combat damage to a player, draw a card.\n{T}: Turn target face-down attacking creature you control face up.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

#[test]
fn kaust_trigger_reads_the_turn_history() {
    for definition in definitions("Kaust, Eyes of the Glade", KAUST) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("turned_face_up_this_turn: true"), "{debug}");
    }
}

#[test]
fn target_filter_never_degrades_to_face_up() {
    for definition in definitions(
        "Face-up history probe",
        "Mana cost: {B}\nType: Instant\nDestroy target creature that was turned face up this turn.",
    ) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("turned_face_up_this_turn: true"), "{debug}");
        assert!(!debug.contains("face_down: Some(false)"), "no current-state substitute");
    }
}
