//! "Prevent all damage a source of your choice would deal [to you] this
//! turn." (CR 609.7a, 615). The active-voice chosen-source form now reads
//! as a PreventAllDamageEffect with a resolution-time source choice.
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const PAY_NO_HEED: &str = "Mana cost: {W}\nType: Instant\nPrevent all damage a source of your choice would deal this turn.";
const AURIOK_REPLICA: &str = "Mana cost: {1}\nType: Artifact Creature — Cleric\nPower/Toughness: 1/1\n{W}, Sacrifice this creature: Prevent all damage a source of your choice would deal to you this turn.";

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

#[test]
fn pay_no_heed_shields_every_recipient_from_one_chosen_source() {
    for definition in routes("Pay No Heed", PAY_NO_HEED) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let text = format!("{:?}", definition.spell_effect);
        assert!(text.contains("PreventAllDamageEffect"), "{text}");
        assert!(text.contains("target: All"), "every recipient: {text}");
        assert!(text.contains("source_of_your_choice: true"), "{text}");
        let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.to_ascii_lowercase().contains("source of your choice"), "{rendered}");
    }
}

#[test]
fn auriok_replica_shields_only_you_from_one_chosen_source() {
    for definition in routes("Auriok Replica", AURIOK_REPLICA) {
        let text = format!("{:?}", definition.abilities);
        assert!(text.contains("PreventAllDamageEffect"), "{text}");
        assert!(text.contains("target: You"), "{text}");
        assert!(text.contains("source_of_your_choice: true"), "{text}");
    }
}
