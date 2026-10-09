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

const HARMS_WAY: &str = "Mana cost: {W}\nType: Instant\nThe next 2 damage that a source of your choice would deal to you and/or permanents you control this turn is dealt to any target instead.";
const SHINING_SHOAL: &str = "Mana cost: {X}{W}{W}\nType: Instant — Arcane\nYou may exile a white card with mana value X from your hand rather than pay this spell's mana cost.\nThe next X damage that a source of your choice would deal to you and/or creatures you control this turn is dealt to any target instead.";

#[test]
fn chosen_source_bounded_redirection_protects_you_and_your_permanents() {
    for (name, text, amount, card_types) in [
        ("Harm's Way", HARMS_WAY, "Fixed(2)", Vec::new()),
        ("Shining Shoal", SHINING_SHOAL, "X", vec![ironsmith::CardType::Creature]),
    ] {
        for definition in routes(name, text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let text = format!("{:?}", definition.spell_effect);
            assert!(text.contains("RedirectNextDamageToTargetEffect"), "{name}: {text}");
            assert!(text.contains("source_of_your_choice: true"), "{name}: {text}");
            assert!(text.contains("protect_you_and_permanents: Some("), "{name}: {text}");
            assert!(text.contains("protected_target: None"), "{name}: the protected set is not a target");
            assert!(text.contains(amount), "{name}: {text}");
            fn recipients(effect: &ironsmith::effect::Effect) -> Option<ironsmith::target::ObjectFilter> {
                if let Some(redirect) = effect.downcast_ref::<ironsmith::effects::RedirectNextDamageToTargetEffect>() {
                    return redirect.protect_you_and_permanents.clone();
                }
                let mut found = None;
                effect.visit_child_effects(&mut |child| { if found.is_none() { found = recipients(child); } });
                found
            }
            let filter = definition.spell_effect.as_ref().unwrap().all_effects().iter()
                .find_map(|effect| recipients(effect)).expect("protected permanent set");
            assert_eq!(filter.card_types, card_types);
            assert_eq!(filter.zone, Some(ironsmith::Zone::Battlefield));
            assert_eq!(filter.controller, Some(ironsmith::target::PlayerFilter::You));
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            assert!(rendered.contains("source of your choice"), "{rendered}");
        }
    }
}

#[test]
fn unqualified_bounded_redirection_keeps_no_source_choice() {
    let text = "Mana cost: {W}\nType: Instant\nThe next 2 damage that would be dealt to target creature this turn is dealt to you instead.";
    for definition in routes("Bounded control", text) {
        let text = format!("{:?}", definition.spell_effect);
        assert!(text.contains("source_of_your_choice: false"), "{text}");
    }
}
