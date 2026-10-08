//! Source-authored and deliberately unrun (cf8 p04): triggered abilities that
//! cast a card from a graveyard as they resolve (CR 608.2g). "Cast this card
//! from your graveyard" casts the ability's own card and the ability functions
//! from the graveyard (CR 113.6k); Oskar's "cast it" names the discarded card.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::Zone;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(index: usize) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/graveyard_resolution_casts.json.fixture"
    ))
    .unwrap();
    let row = &rows[index];
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct.unwrap(), materialize_artifact(&restored).unwrap()]
}

fn graveyard_trigger_casts_source(definition: &CardDefinition) {
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    let trigger = definition
        .abilities
        .iter()
        .find(|ability| {
            matches!(ability.kind, AbilityKind::Triggered(_))
                && format!("{ability:?}").contains("CastSourceEffect")
        })
        .unwrap_or_else(|| panic!("{definition:?}"));
    assert_eq!(trigger.functional_zones, vec![Zone::Graveyard], "{trigger:?}");
}

#[test]
fn sproutback_trudge_casts_itself_from_the_graveyard_at_end_step() {
    for definition in definitions(0) {
        graveyard_trigger_casts_source(&definition);
    }
}

#[test]
fn syrix_casts_itself_from_the_graveyard_when_another_phoenix_dies() {
    for definition in definitions(1) {
        graveyard_trigger_casts_source(&definition);
    }
}

#[test]
fn oskar_casts_the_discarded_card_not_itself() {
    for definition in definitions(2) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("CastTagged"), "{debug}");
        assert!(!debug.contains("CastSourceEffect"), "{debug}");
        assert!(debug.contains("triggering"), "{debug}");
    }
}
