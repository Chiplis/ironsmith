//! "Prevent all combat damage that would be dealt this turn. If this spell's
//! additional cost was paid, this effect doesn't affect combat damage that
//! would be dealt by red creatures." (Undergrowth; CR 615.1).
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const UNDERGROWTH: &str = "Mana cost: {G}\nType: Instant\nAs an additional cost to cast this spell, you may pay {2}{R}.\nPrevent all combat damage that would be dealt this turn. If this spell's additional cost was paid, this effect doesn't affect combat damage that would be dealt by red creatures.";

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
fn undergrowth_branches_on_its_optional_cost() {
    for definition in routes("Undergrowth", UNDERGROWTH) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert_eq!(definition.optional_costs.len(), 1, "the {{2}}{{R}} optional additional cost");
        let text = format!("{:?}", definition.spell_effect);
        assert!(text.contains("Conditional"), "{text}");
        assert!(text.contains("PreventAllCombatDamage"), "unkicked Fog: {text}");
        assert!(text.contains("excluded_colors"), "kicked branch spares red creatures: {text}");
    }
}

const INSPIRE_AWE: &str = "Mana cost: {3}{G}\nType: Instant\nPrevent all combat damage that would be dealt this turn except combat damage that would be dealt by enchanted creatures and enchantment creatures. Scry 2.";

#[test]
fn inspire_awe_spares_enchanted_and_enchantment_creatures() {
    for definition in routes("Inspire Awe", INSPIRE_AWE) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let text = format!("{:?}", definition.spell_effect);
        assert!(text.contains("without_attached_object: Some("), "enchanted creatures excepted: {text}");
        assert!(text.contains("excluded_card_types: [Enchantment]"), "enchantment creatures excepted: {text}");
        assert!(text.contains("Aura"), "{text}");
        assert!(text.contains("Scry"), "{text}");
    }
}
