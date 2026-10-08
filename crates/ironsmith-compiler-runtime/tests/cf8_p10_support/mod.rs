//! Shared helpers for the cf8 p10 cluster regressions (source-authored, UNRUN).
#![allow(dead_code)]
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

/// Strict, lossless compilation on the direct route and the artifact route.
pub fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    let (artifact, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

/// Every effect of the spell body and of every activated/triggered ability.
pub fn all_effects(definition: &CardDefinition) -> Vec<Effect> {
    let mut all = Vec::new();
    if let Some(spell) = &definition.spell_effect {
        for effect in spell.all_effects() {
            collect(effect, &mut all);
        }
    }
    for ability in &definition.abilities {
        let program = match &ability.kind {
            AbilityKind::Activated(ability) => &ability.effects,
            AbilityKind::Triggered(ability) => &ability.effects,
            _ => continue,
        };
        for effect in program.all_effects() {
            collect(effect, &mut all);
        }
    }
    all
}

pub fn find_all<T: Clone + 'static>(definition: &CardDefinition) -> Vec<T> {
    all_effects(definition)
        .iter()
        .filter_map(|effect| effect.downcast_ref::<T>().cloned())
        .collect()
}
