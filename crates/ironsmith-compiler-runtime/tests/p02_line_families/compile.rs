//! Shared strict compile helper for the p02 line-family cluster tests.
//! Source-authored, deliberately unrun.
#![allow(dead_code)]
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith_compiled_artifact::CompiledCardArtifact;

/// Compile the complete frozen card text strictly through both the direct
/// runtime route and the serialized artifact route, rejecting any parse loss.
pub fn compile_both(name: &str, text: &str) -> [CardDefinition; 2] {
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
    assert_eq!(artifact, restored, "{name}: artifact must round-trip");
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored)
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
    for definition in [&direct, &decoded] {
        assert_eq!(definition.card.name, name);
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition),
            "{name}: compiled definition must not carry unimplemented content"
        );
    }
    [direct, decoded]
}

/// Every static ability of the definition with the given id.
pub fn statics(definition: &CardDefinition, id: StaticAbilityId) -> Vec<StaticAbility> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) if ability.id() == id => Some(ability.clone()),
            _ => None,
        })
        .collect()
}
