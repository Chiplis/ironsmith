//! Shared helpers for the p12-other frozen card bodies. Source-authored, unrun.
#![allow(dead_code)]
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

pub fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../../fixtures/p12_other_card_bodies.json.fixture"
    ))
    .unwrap()
}

pub fn row(name: &str) -> serde_json::Value {
    rows()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("missing frozen row {name}"))
}

/// The complete frozen body compiled strictly on the direct route and on the
/// artifact route (serialized, validated, materialized).
pub fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = row(name);
    let text = row["text"].as_str().unwrap();
    let (direct, loss) =
        parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert_eq!(definition.card.name, name);
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition),
            "{name}: unimplemented content"
        );
    }
    [direct, decoded]
}

pub fn debug(definition: &CardDefinition) -> String {
    format!("{definition:?}")
}

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

/// Every effect of the spell body and of every activated/triggered ability.
pub fn effects(definition: &CardDefinition) -> Vec<Effect> {
    let mut all = Vec::new();
    if let Some(program) = definition.spell_effect.as_ref() {
        for effect in program.all_effects() {
            collect(effect, &mut all);
        }
    }
    for ability in &definition.abilities {
        let program = match &ability.kind {
            AbilityKind::Triggered(ability) => &ability.effects,
            AbilityKind::Activated(ability) => &ability.effects,
            _ => continue,
        };
        for effect in program.all_effects() {
            collect(effect, &mut all);
        }
    }
    all
}

pub fn triggered_count(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .filter(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
        .count()
}
