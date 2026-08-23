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
fn typed_prior_exile_count_renders_for_each_object_kind() {
    let query = ironsmith_core::PriorEffectMetricQuery::new(
        crate::effect::EffectMetricSource::AffectedObjects,
        crate::effect::EffectMetric::Count,
    )
    .with_action(crate::effect::PriorEffectAction::Exiled)
    .with_filter(ObjectFilter::permanent());
    let count = Value::PriorEffectMetric {
        effect_id: crate::effect::EffectId(7),
        query,
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);

    assert_eq!(
        describe_create_for_each_count(&count).as_deref(),
        Some("permanent exiled this way")
    );
}

#[test]
fn typed_prior_sacrifice_count_creates_one_food_per_creature() {
    let query = ironsmith_core::PriorEffectMetricQuery::new(
        crate::effect::EffectMetricSource::AffectedObjects,
        crate::effect::EffectMetric::Count,
    )
    .with_action(crate::effect::PriorEffectAction::Sacrificed)
    .with_filter(ObjectFilter::creature());
    let count = Value::PriorEffectMetric {
        effect_id: crate::effect::EffectId(7),
        query,
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let create = Effect::new(crate::effects::CreateTokenEffect::new(
        crate::cards::tokens::food_token_definition(),
        count,
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect(&create),
        "Create a Food token for each creature sacrificed this way"
    );
}

#[test]
fn scalar_damage_and_life_keep_that_much_surface() {
    for amount in [
        Value::EventValue(EventValueSpec::Amount),
        Value::EffectValue(crate::effect::EffectId(7)),
    ] {
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
}

#[test]
fn additive_for_each_life_amount_keeps_base_and_scaled_terms() {
    let mut spirits = ObjectFilter::default();
    spirits.subtypes.push(crate::types::Subtype::Spirit);
    let addend = Value::Scaled(Box::new(Value::Count(spirits)), 2)
        .with_surface_hint(ValueSurfaceHint::ForEach);
    let amount = Value::Add(Box::new(Value::Fixed(2)), Box::new(addend));

    assert_eq!(
        describe_life_amount_phrase(&amount),
        "2 life plus 2 life for each Spirit"
    );
}

#[test]
fn life_amount_preserves_where_x_history_surface() {
    let amount = Value::TurnHistoryCount(
        ironsmith_core::TurnHistoryCount::PlayersDealtCombatDamageBy {
            players: PlayerFilter::Opponent,
            sources: ObjectFilter::default(),
        },
    )
    .with_surface_hint(ValueSurfaceHint::WhereXIs);

    assert_eq!(
        describe_life_amount_phrase(&amount),
        "X life, where X is the number of opponents who were dealt combat damage this turn"
    );
}
