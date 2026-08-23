use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn rendered_spell(definition: &CardDefinition) -> String {
    canonical_compiled_lines(definition).join("\n")
}

fn dynamic_power_spell(name: &str, target: ChooseSpec, power: Value) -> CardDefinition {
    let effect = crate::effects::ApplyContinuousEffect::with_spec_runtime(
        target,
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
            power,
            toughness: Value::Fixed(0),
        },
        Until::EndOfTurn,
    );
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::new(effect)])
        .build()
}

fn target_object_value(characteristic: fn(Box<ChooseSpec>) -> Value) -> Value {
    characteristic(Box::new(ChooseSpec::Target(Box::new(ChooseSpec::Object(
        ObjectFilter::default(),
    )))))
}

#[test]
pub(super) fn onward_and_rush_of_blood_render_same_target_power_as_its_power() {
    for name in ["Onward // Victory", "Rush of Blood"] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let rendered = rendered_spell(&definition);
        assert!(
            rendered
                .contains("Target creature gets +X/+0 until end of turn, where X is its power."),
            "{name} should render its sole target's power anaphorically:\n{rendered}"
        );
        assert!(
            !rendered.contains("where X is target permanent's power"),
            "{name} should not expose the generic target-value selector:\n{rendered}"
        );
    }
}

#[test]
pub(super) fn rethink_renders_same_target_mana_value_as_its_mana_value() {
    assert_oracle_card_parses_strict("Rethink");
    let definition = parse_oracle_card_definition("Rethink");
    let rendered = rendered_spell(&definition);
    assert!(
        rendered.contains(
            "Counter target spell unless its controller pays {X}, where X is its mana value."
        ),
        "Rethink should render its sole target's mana value anaphorically:\n{rendered}"
    );
    assert!(
        !rendered.contains("where X is target permanent's mana value"),
        "Rethink should not expose the generic target-value selector:\n{rendered}"
    );
}

#[test]
pub(super) fn power_anaphora_preserves_multiple_and_constrained_value_targets() {
    let two_creatures = ChooseSpec::WithCount(
        Box::new(ChooseSpec::Target(Box::new(ChooseSpec::Object(
            ObjectFilter::creature().in_zone(Zone::Battlefield),
        )))),
        ChoiceCount::exactly(2),
    );
    let multiple = dynamic_power_spell(
        "Multiple Target Pump",
        two_creatures,
        target_object_value(Value::PowerOf),
    );
    let multiple_rendered = rendered_spell(&multiple);
    assert!(
        multiple_rendered.contains("where X is target permanent's power"),
        "a multiple-target action must keep the value target explicit:\n{multiple_rendered}"
    );

    let constrained_value_target = Value::PowerOf(Box::new(ChooseSpec::Target(Box::new(
        ChooseSpec::Object(ObjectFilter::artifact().in_zone(Zone::Battlefield)),
    ))));
    let unrelated = dynamic_power_spell(
        "Unrelated Target Pump",
        ChooseSpec::Target(Box::new(ChooseSpec::Object(
            ObjectFilter::creature().in_zone(Zone::Battlefield),
        ))),
        constrained_value_target,
    );
    let unrelated_rendered = rendered_spell(&unrelated);
    assert!(
        unrelated_rendered.contains("where X is target artifact's power"),
        "a constrained value target must remain explicit:\n{unrelated_rendered}"
    );
}

#[test]
pub(super) fn counterspell_anaphora_preserves_multiple_value_targets() {
    let mut spell_filter = ObjectFilter::default();
    spell_filter.zone = Some(Zone::Stack);
    spell_filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
    let target = ChooseSpec::WithCount(
        Box::new(ChooseSpec::Target(Box::new(ChooseSpec::Object(
            spell_filter,
        )))),
        ChoiceCount::exactly(2),
    );
    let dynamic = ironsmith_core::DynamicManaCost::new(
        ManaCost::from_symbols(vec![ManaSymbol::X]),
        Some(target_object_value(Value::ManaValueOf)),
        None,
        None,
        ironsmith_core::DynamicManaDisplayHint::Default,
    );
    let effect = Effect::counter_unless_pays_total_cost(
        target,
        crate::cost::TotalCost::from_cost(crate::costs::Cost::dynamic_mana(dynamic)),
    );
    let definition = CardDefinitionBuilder::new(CardId::new(), "Multiple Target Rethink")
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![effect])
        .build();
    let rendered = rendered_spell(&definition);
    assert!(
        rendered.contains("where X is target permanent's mana value"),
        "a multiple-target counterspell must keep the value target explicit:\n{rendered}"
    );
}
