//! Shared helpers for the cf8 p09-other cluster tests. Source-authored,
//! deliberately unrun (implementation-first campaign).
#![allow(dead_code)]
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith_compiled_artifact::CompiledCardArtifact;

/// Frozen rows: `name`, `oracle_id`, `oracle_text`, and `text` (metadata
/// header plus the complete frozen Oracle text).
pub fn rows(fixture_json: &str) -> Vec<serde_json::Value> {
    serde_json::from_str(fixture_json).unwrap()
}

pub fn row<'a>(rows: &'a [serde_json::Value], name: &str) -> &'a serde_json::Value {
    rows.iter().find(|row| row["name"] == name).unwrap_or_else(|| panic!("missing fixture row {name}"))
}

/// Compile the complete frozen text strictly through the direct runtime route
/// and the artifact route (serialized, validated and materialized), asserting
/// neither route reports parse loss or unimplemented content.
pub fn definitions(row: &serde_json::Value) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition), "{name}");
    }
    [direct, decoded]
}

pub fn rendered(definition: &CardDefinition) -> String {
    ironsmith_text::canonical_compiled_lines(definition).join("\n")
}

pub fn static_ids(definition: &CardDefinition) -> Vec<StaticAbilityId> {
    definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Static(static_ability) => Some(static_ability.id()),
        _ => None,
    }).collect()
}

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

/// Every effect (recursively) of the spell body and of every activated or
/// triggered ability.
pub fn all_effects(definition: &CardDefinition) -> Vec<Effect> {
    let mut all = Vec::new();
    if let Some(spell) = definition.spell_effect.as_ref() {
        for effect in spell.all_effects() { collect(effect, &mut all); }
    }
    for ability in &definition.abilities {
        match &ability.kind {
            AbilityKind::Activated(activated) => {
                for effect in activated.effects.all_effects() { collect(effect, &mut all); }
            }
            AbilityKind::Triggered(triggered) => {
                for effect in triggered.effects.all_effects() { collect(effect, &mut all); }
            }
            _ => {}
        }
    }
    all
}
