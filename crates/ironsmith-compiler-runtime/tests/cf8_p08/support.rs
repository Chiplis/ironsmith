//! Shared helpers for the cf8 p08 cluster scenarios. Source-authored, UNRUN.
#![allow(dead_code)]
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

/// Compile the complete frozen body strictly on the direct route and through
/// the artifact round trip; both must be lossless and fully implemented.
pub fn definitions(name: &str, body: &str) -> [CardDefinition; 2] {
    let (direct, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, body, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, body, false));
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(artifact, decoded);
    let restored =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    for definition in [&direct, &restored] {
        assert_eq!(definition.card.name, name);
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition),
            "{name}: unimplemented content"
        );
    }
    [direct, restored]
}

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

/// Every effect (recursively) of the spell body and of every activated or
/// triggered ability.
pub fn all_effects(definition: &CardDefinition) -> Vec<Effect> {
    let mut all = Vec::new();
    if let Some(program) = &definition.spell_effect {
        for effect in program.all_effects() {
            collect(effect, &mut all);
        }
    }
    for ability in &definition.abilities {
        let program = match &ability.kind {
            AbilityKind::Activated(ability) => &ability.effects,
            AbilityKind::Triggered(ability) => &ability.effects,
            AbilityKind::Static(_) => continue,
        };
        for effect in program.all_effects() {
            collect(effect, &mut all);
        }
    }
    all
}

/// Effects of type `T` anywhere in the definition.
pub fn find_all<T: 'static + Clone>(definition: &CardDefinition) -> Vec<T> {
    all_effects(definition)
        .iter()
        .filter_map(|effect| effect.downcast_ref::<T>().cloned())
        .collect()
}

pub fn rendered(definition: &CardDefinition) -> String {
    ironsmith_text::canonical_compiled_lines(definition).join("\n")
}
