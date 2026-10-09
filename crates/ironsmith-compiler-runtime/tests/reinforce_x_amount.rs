//! "Reinforce X—{X}{C}{C}": CR 702.77a, with X the value paid for the
//! reinforce cost's {X}. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::PutCountersEffect;
use ironsmith::Zone;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/reinforce_x_amount.json.fixture")).unwrap()
}

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
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
fn reinforce_x_puts_the_paid_x_counters_from_hand_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        let colored = if name == "Wren's Run Hydra" { "{X}{G}{G}" } else { "{X}{W}{W}" };
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            let reinforce: Vec<_> = definition
                .abilities
                .iter()
                .filter(|ability| ability.functional_zones.contains(&Zone::Hand))
                .filter_map(|ability| match &ability.kind {
                    AbilityKind::Activated(activated) => Some(activated),
                    _ => None,
                })
                .collect();
            assert_eq!(reinforce.len(), 1, "{name}");
            let mana = reinforce[0].mana_cost.mana_cost().expect("reinforce mana cost");
            assert_eq!(mana.to_oracle(), colored, "{name}");
            let mut all = Vec::new();
            for effect in reinforce[0].effects.all_effects() {
                collect(effect, &mut all);
            }
            let put = all
                .iter()
                .find_map(|effect| effect.downcast_ref::<PutCountersEffect>())
                .expect("reinforce counters");
            assert_eq!(put.amount, Value::X, "{name}: the reinforce cost's X");
            assert_eq!(put.counter_type, ironsmith::object::CounterType::PlusOnePlusOne);
            let target = format!("{:?}", put.target);
            assert!(target.contains("Target"), "{name}: reinforce targets ({target})");
        }
    }
}

#[test]
fn reinforce_x_without_an_x_cost_stays_rejected() {
    let text = "Mana cost: {1}{G}\nType: Creature — Elf\nPower/Toughness: 1/1\nReinforce X—{1}{G}";
    assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Unbound X", text, false).is_err());
    assert!(ironsmith_compiler_runtime::compile_to_artifact("Unbound X", text, false).is_err());
}
