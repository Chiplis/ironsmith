//! Chosen-source finite prevention in the active voice (Refraction Trap,
//! CR 615.7 with a "prevented this way" rider, CR 615.5) and next-time
//! redirection to the source's controller (Reflect Damage, CR 614.9).
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const REFRACTION_TRAP: &str = "Mana cost: {3}{W}\nType: Instant — Trap\nIf an opponent cast a red instant or sorcery spell this turn, you may pay {W} rather than pay this spell's mana cost.\nPrevent the next 3 damage that a source of your choice would deal to you and/or permanents you control this turn. If damage is prevented this way, Refraction Trap deals that much damage to any target.";
const REFLECT_DAMAGE: &str = "Mana cost: {3}{R}{W}\nType: Instant\nThe next time a source of your choice would deal damage this turn, that damage is dealt to that source's controller instead.";

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
fn refraction_trap_is_a_chosen_source_shield_with_a_reflect_rider() {
    for definition in routes("Refraction Trap", REFRACTION_TRAP) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let text = format!("{:?}", definition.spell_effect);
        assert!(text.contains("PreventDamageEffect"), "{text}");
        assert!(text.contains("source_of_your_choice: true"), "{text}");
        assert!(text.contains("protect_you_and_permanents_you_control: true"), "{text}");
        assert!(text.contains("Fixed(3)"), "{text}");
        assert!(text.contains("DealDamage"), "reflect rider: {text}");
        assert!(!definition.alternative_casts.is_empty(), "trap price retained");
    }
}

#[test]
fn reflect_damage_redirects_the_chosen_sources_next_damage_to_its_controller() {
    for definition in routes("Reflect Damage", REFLECT_DAMAGE) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let text = format!("{:?}", definition.spell_effect);
        assert!(text.contains("RedirectNextTimeDamageToSourceEffect"), "{text}");
        assert!(text.contains("Choice"), "{text}");
        assert!(text.contains("SourceController"), "{text}");
        assert!(text.contains("target: None"), "any recipient: {text}");
    }
}
