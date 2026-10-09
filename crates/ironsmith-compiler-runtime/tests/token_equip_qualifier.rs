//! "Equip creature token {1}" (CR 702.6a with a qualified target).
//! Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const TEAM_PENNANT: &str = "Mana cost: {1}\nType: Artifact — Equipment\nEquipped creature gets +1/+1 and has vigilance and trample.\nEquip creature token {1}\nEquip {3}";

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
fn team_pennant_has_a_token_only_equip_and_a_general_equip() {
    for definition in routes("Team Pennant", TEAM_PENNANT) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let equips: Vec<String> = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(format!("{activated:?}")),
                _ => None,
            })
            .collect();
        assert_eq!(equips.len(), 2, "{equips:#?}");
        assert_eq!(
            equips.iter().filter(|equip| equip.contains("token: true")).count(),
            1,
            "exactly one equip is restricted to creature tokens: {equips:#?}"
        );
    }
}

#[test]
fn bare_token_word_is_not_an_equip_qualifier() {
    let source = "Mana cost: {1}\nType: Artifact — Equipment\nEquip token {1}";
    assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Bad equip", source, false).is_err());
}
