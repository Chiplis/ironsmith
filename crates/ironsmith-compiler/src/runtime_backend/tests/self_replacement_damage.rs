use super::*;

fn damage_effects(
    definition: &crate::cards::CardDefinition,
) -> (
    &crate::effects::DealDamageEffect,
    &crate::effects::DealDamageEffect,
) {
    let program = definition
        .spell_effect
        .as_ref()
        .expect("expected a spell resolution program");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one spell segment: {program:#?}");
    };
    let [default_effect] = segment.default_effects.as_slice() else {
        panic!("expected one default effect: {segment:#?}");
    };
    let default_damage =
        super::find_nested_effect::<crate::effects::DealDamageEffect>(default_effect)
            .expect("expected default damage");
    let [replacement] = segment.self_replacements.as_slice() else {
        panic!("expected one self-replacement: {segment:#?}");
    };
    let [replacement_effect] = replacement.replacement_effects.as_slice() else {
        panic!("expected one replacement effect: {replacement:#?}");
    };
    let replacement_damage =
        super::find_nested_effect::<crate::effects::DealDamageEffect>(replacement_effect)
            .expect("expected replacement damage");
    assert!(
        super::find_nested_effect::<crate::effects::ExecuteWithSourceEffect>(replacement_effect)
            .is_none(),
        "an amount replacement must not make the old target the damage source: {replacement:#?}"
    );
    (default_damage, replacement_damage)
}

#[test]
fn addendum_damage_replacement_keeps_the_original_source_and_target() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Summary Judgment")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Summary Judgment deals 3 damage to target tapped creature.\nAddendum — If you cast this spell during your main phase, it deals 5 damage instead.",
        )
        .expect("Summary Judgment should compile");
    let (default_damage, replacement_damage) = damage_effects(&definition);

    assert_eq!(default_damage.amount, crate::effect::Value::Fixed(3));
    assert_eq!(replacement_damage.amount, crate::effect::Value::Fixed(5));
    assert_eq!(replacement_damage.target, default_damage.target);
}

#[test]
fn adamant_damage_replacement_keeps_the_original_source_and_target() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Slaying Fire")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Slaying Fire deals 3 damage to any target.\nAdamant — If at least three red mana was spent to cast this spell, it deals 4 damage instead.",
        )
        .expect("Slaying Fire should compile");
    let (default_damage, replacement_damage) = damage_effects(&definition);

    assert_eq!(default_damage.amount, crate::effect::Value::Fixed(3));
    assert_eq!(replacement_damage.amount, crate::effect::Value::Fixed(4));
    assert_eq!(replacement_damage.target, default_damage.target);
}

#[test]
fn leading_labeled_damage_replacements_keep_the_unpreventable_rider() {
    for (name, text, replacement_amount) in [
        (
            "Raid Damage Probe",
            "Raid Damage Probe deals 4 damage to any target.\nRaid — If you attacked this turn, instead Raid Damage Probe deals 5 damage to that permanent or player and the damage can't be prevented.",
            5,
        ),
        (
            "Threshold Damage Probe",
            "Threshold Damage Probe deals 4 damage to any target.\nThreshold — If there are seven or more cards in your graveyard, instead Threshold Damage Probe deals 6 damage to that permanent or player and the damage can't be prevented.",
            6,
        ),
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
        let (default_damage, replacement_damage) = damage_effects(&definition);

        assert_eq!(
            replacement_damage.amount,
            crate::effect::Value::Fixed(replacement_amount),
            "{name}"
        );
        assert_eq!(replacement_damage.target, default_damage.target, "{name}");
        assert!(
            replacement_damage.unpreventable,
            "{name} must preserve the terminal prevention rider: {replacement_damage:#?}"
        );
    }
}
