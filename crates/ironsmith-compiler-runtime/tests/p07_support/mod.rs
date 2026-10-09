//! Shared full-card helpers for the cf8/p07 cluster regressions.
//! Source-authored, deliberately unrun.
#![allow(dead_code)]

use ironsmith::ability::{AbilityKind, TriggeredAbility, ActivatedAbility};
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

pub fn rows(fixture: &str) -> Vec<serde_json::Value> {
    serde_json::from_str(fixture).unwrap()
}

pub fn card_text(row: &serde_json::Value) -> String {
    let mut text = String::new();
    if let Some(cost) = row["mana_cost"].as_str().filter(|cost| !cost.is_empty()) {
        text += &format!("Mana cost: {cost}\n");
    }
    text += &format!("Type: {}\n", row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text += &format!("Power/Toughness: {p}/{t}\n");
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        text += &format!("Loyalty: {loyalty}\n");
    }
    text += row["oracle_text"].as_str().unwrap();
    text
}

/// Compile the complete frozen body strictly on the direct route and through
/// the serialized artifact, failing on any recorded parse loss.
pub fn definitions(row: &serde_json::Value) -> [CardDefinition; 2] {
    // A transforming card's frozen body is its front face.
    let name = row["name"].as_str().unwrap().split(" // ").next().unwrap();
    let text = card_text(row);
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, &text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&direct),
        "{name}: unimplemented content"
    );
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

pub fn row<'a>(rows: &'a [serde_json::Value], name: &str) -> &'a serde_json::Value {
    rows.iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("missing fixture row {name}"))
}

pub fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

pub fn flatten<'a>(effects: impl IntoIterator<Item = &'a Effect>) -> Vec<Effect> {
    let mut all = Vec::new();
    for effect in effects {
        collect(effect, &mut all);
    }
    all
}

pub fn spell_effects(definition: &CardDefinition) -> Vec<Effect> {
    flatten(definition.spell_effect.as_ref().unwrap().all_effects())
}

pub fn triggered(definition: &CardDefinition) -> Vec<&TriggeredAbility> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect()
}

pub fn activated(definition: &CardDefinition) -> Vec<&ActivatedAbility> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .collect()
}

pub fn triggered_effects(triggered: &TriggeredAbility) -> Vec<Effect> {
    flatten(triggered.effects.all_effects())
}

pub fn activated_effects(activated: &ActivatedAbility) -> Vec<Effect> {
    flatten(activated.effects.all_effects())
}

pub fn find<T: 'static>(effects: &[Effect]) -> Vec<T>
where
    T: Clone,
{
    effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<T>().cloned())
        .collect()
}
