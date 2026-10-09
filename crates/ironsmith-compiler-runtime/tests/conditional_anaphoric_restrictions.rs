//! "<ability word> — If <condition>, those creatures don't untap during their
//! controllers' next untap steps." (Icy Blast, Send to Sleep): a leading state
//! condition gates a restriction on the objects the previous instruction
//! tapped. Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith::effects::ConditionalEffect;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/conditional_anaphoric_restrictions.json.fixture"
    ))
    .unwrap();
    let row = rows.into_iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
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
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

#[test]
fn ability_word_condition_gates_the_dont_untap_restriction_on_the_tapped_targets() {
    for name in ["Icy Blast", "Send to Sleep"] {
        for definition in definitions(name) {
            let mut all = Vec::new();
            for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects() {
                collect(effect, &mut all);
            }
            let conditional = all
                .iter()
                .find_map(|effect| effect.downcast_ref::<ConditionalEffect>())
                .unwrap_or_else(|| panic!("{name}: the restriction is gated"));
            let inner = format!("{:?}", conditional.if_true);
            assert!(inner.contains("CantEffect"), "{name}: {inner}");
            assert!(inner.contains("Untap"), "{name}: {inner}");
            assert!(inner.contains("ControllersNextUntapStep"), "{name}: {inner}");
            assert!(conditional.if_false.is_empty());
        }
    }
}
