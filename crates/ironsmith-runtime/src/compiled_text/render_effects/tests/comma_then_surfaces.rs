use super::*;

#[test]
fn typed_comma_then_draw_discard_keeps_the_shared_player_subject() {
    let player = PlayerFilter::target_player();
    let hand = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)));
    let draw_id = crate::effect::EffectId(7);
    let draw = Effect::with_id(
        draw_id.0,
        Effect::new(crate::effects::DrawCardsEffect::new(
            Value::Count(hand),
            player.clone(),
        )),
    );
    let discard = Effect::new(crate::effects::DiscardEffect::new(
        Value::EffectValue(draw_id),
        PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)),
        false,
    ));
    let sequence = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
        draw, discard,
    ]));

    assert_eq!(
        describe_effect(&sequence),
        "Target player draws cards equal to the number of cards in their hand, then discards that many cards"
    );
}

#[test]
fn typed_comma_then_reveal_choose_discard_ignores_implicit_target_scaffolding() {
    let target = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any));
    let player = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
    let chosen = TagKey::from("chosen_hand_card");
    let look = Effect::new(crate::effects::LookAtHandEffect::reveal(target.clone()));
    let mut choice_filter = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(player.clone());
    choice_filter.set_explicit_card_noun(true);
    choice_filter.excluded_card_types.push(CardType::Land);
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            choice_filter,
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            chosen.clone(),
        )
        .in_zone(Zone::Hand),
    );
    let discard = Effect::new(crate::effects::DiscardEffect::new_with_filter(
        Value::Fixed(1),
        player,
        false,
        Some(ObjectFilter::tagged(chosen).in_zone(Zone::Hand)),
    ));
    let sequence = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target)),
        look,
        choose,
        discard,
    ]));

    assert_eq!(
        describe_effect(&sequence),
        "Target player reveals their hand, you choose a nonland card from it, then that player discards that card"
    );
}

#[test]
fn typed_comma_then_keeps_dynamic_token_pt_before_delayed_cleanup() {
    let token = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Horror")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Horror])
        .color_indicator(crate::color::ColorSet::RED)
        .power_toughness(crate::card::PowerToughness::fixed(0, 0))
        .build();
    let created = TagKey::from("created_0");
    let create =
        Effect::new(crate::effects::CreateTokenEffect::you(token, 1).sacrifice_at_next_end_step())
            .tag(created.clone());
    let power = Value::CountersOn(Box::new(ChooseSpec::Source), Some(CounterType::Oil))
        .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let set_pt =
        Effect::set_base_power_toughness(power, 1, ChooseSpec::Tagged(created), Until::Forever);
    let sequence = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
        Effect::put_counters_on_source(CounterType::Oil, 1),
        create,
        set_pt,
    ]));

    assert_eq!(
        describe_effect(&sequence),
        "Put an oil counter on this source, then create an X/1 red Horror creature token, where X is the number of oil counters on this source. Sacrifice it at the beginning of the next end step"
    );
}

#[test]
fn typed_comma_then_compacts_prior_destroy_count_into_dynamic_token_pt() {
    let token =
        crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Phyrexian Horror")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Phyrexian, Subtype::Horror])
            .power_toughness(crate::card::PowerToughness::fixed(0, 0))
            .build();
    let created = TagKey::from("created_1");
    let create = Effect::new(crate::effects::CreateTokenEffect::you(token, 1)).tag(created.clone());
    let destroyed = Value::PriorEffectMetric {
        effect_id: crate::effect::EffectId(0),
        query: crate::effect::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::AffectedObjects,
            crate::effect::EffectMetric::Count,
        )
        .with_filter(ObjectFilter::creature())
        .with_action(crate::effect::PriorEffectAction::Destroyed),
    }
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let set_pt = Effect::set_base_power_toughness(
        destroyed.clone(),
        destroyed,
        ChooseSpec::Tagged(created),
        Until::Forever,
    );
    let destroy = Effect::new(crate::effects::DestroyEffect::with_spec(ChooseSpec::all(
        ObjectFilter::creature(),
    )))
    .tag("destroyed_0");
    let sequence = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
        destroy, create, set_pt,
    ]));

    assert_eq!(
        describe_effect(&sequence),
        "Destroy all creatures, then create an X/X colorless Phyrexian Horror artifact creature token, where X is the number of creatures destroyed this way"
    );
}
