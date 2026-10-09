//! "Enchanted creature gets +0/+2 and can block creatures with landwalk
//! abilities as though they didn't have those abilities." (Street Savvy):
//! a blocker-side permission; only the enchanted creature ignores landwalk
//! evasion when blocking (CR 702.14). Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const STREET_SAVVY: &str = "Mana cost: {G}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +0/+2 and can block creatures with landwalk abilities as though they didn't have those abilities.";

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
fn street_savvy_grants_the_blocker_side_landwalk_permission() {
    for definition in routes("Street Savvy", STREET_SAVVY) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let text = format!("{:?}", definition.abilities);
        assert!(text.contains("CanBlockAsThoughNoLandwalk"), "{text}");
        assert!(!text.contains("BlockingAsThoughNoLandwalk"), "not the global attacker override: {text}");
    }
}
