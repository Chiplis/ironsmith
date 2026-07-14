use super::*;

fn backref_values() -> [Value; 3] {
    [
        Value::EffectValue(crate::effect::EffectId(7)),
        Value::EventValue(EventValueSpec::Amount),
        Value::EventValue(EventValueSpec::Amount).with_surface_hint(ValueSurfaceHint::EqualTo),
    ]
}

#[test]
fn countable_effects_render_amount_backrefs_as_that_many() {
    for count in backref_values() {
        let create = Effect::new(crate::effects::CreateTokenEffect::new(
            crate::cards::tokens::treasure_token_definition(),
            count.clone(),
            PlayerFilter::You,
        ));
        let expected_create = if count.has_surface_hint(ValueSurfaceHint::EqualTo) {
            "Create a number of Treasure tokens equal to that much"
        } else {
            "Create that many Treasure tokens"
        };
        assert_eq!(describe_effect(&create), expected_create);

        let counters = Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::Charge,
            count.clone(),
            ChooseSpec::Source,
        ));
        assert_eq!(
            describe_effect(&counters),
            "Put that many charge counters on this source"
        );

        let investigate = Effect::new(crate::effects::InvestigateEffect::you(count.clone()));
        assert_eq!(describe_effect(&investigate), "Investigate that many times");

        let energy = Effect::new(crate::effects::EnergyCountersEffect::you(count.clone()));
        assert_eq!(describe_effect(&energy), "you get that many {E}");

        let exile = Effect::new(crate::effects::ExileTopOfLibraryEffect::new(
            count,
            PlayerFilter::You,
        ));
        assert_eq!(
            describe_effect(&exile),
            "Exile that many cards from the top of your library"
        );
    }
}

#[test]
fn scalar_damage_and_life_keep_that_much_surface() {
    let amount = Value::EventValue(EventValueSpec::Amount);
    assert_eq!(
        describe_effect(&Effect::deal_damage(
            amount.clone(),
            ChooseSpec::target_player(),
        )),
        "Deal that much damage to target player"
    );
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::GainLifeEffect::you(amount))),
        "you gain that much life"
    );
}
