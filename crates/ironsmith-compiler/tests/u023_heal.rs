use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::cards::{CardDefinition, CardDefinitionBuilder};
use ironsmith_compiler::effect::{Effect, Value};
use ironsmith_compiler::effects::{HealDamageEffect, TaggedEffect};
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::types::CardType;

fn compile_instant(name: &str, text: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"))
}

fn compiled_heal(definition: &CardDefinition) -> &HealDamageEffect {
    definition
        .spell_effect
        .as_ref()
        .expect("instant should have a resolution program")
        .flattened_default_effects()
        .iter()
        .find_map(|effect: &Effect| {
            effect.downcast_ref::<HealDamageEffect>().or_else(|| {
                effect
                    .downcast_ref::<TaggedEffect>()
                    .and_then(|tagged| tagged.effect.downcast_ref::<HealDamageEffect>())
            })
        })
        .expect("resolution program should contain HealDamageEffect")
}

#[test]
fn heal_exact_amount_lowers_to_typed_damage_removal() {
    let definition = compile_instant(
        "Exact Heal Probe",
        "Heal 2 damage already dealt to target creature.",
    );
    let heal = compiled_heal(&definition);

    assert_eq!(heal.amount, Some(Value::Fixed(2)));
    assert!(heal.target.is_target());
}

#[test]
fn passive_is_healed_surface_lowers_to_all_marked_damage() {
    let definition = compile_instant(
        "All Damage Heal Probe",
        "All damage already dealt to target creature is healed.",
    );
    let heal = compiled_heal(&definition);

    assert_eq!(heal.amount, None);
    assert!(heal.target.is_target());
}
