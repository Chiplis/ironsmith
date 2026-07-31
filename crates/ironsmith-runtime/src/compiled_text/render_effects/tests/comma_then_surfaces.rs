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

fn looked_card_permission_sequence(permission_tag: TagKey) -> Effect {
    let looked = TagKey::from("__sentence_helper_looked_exiled_l0_s0_e0");
    Effect::new(crate::effects::SequenceEffect::comma_then(vec![
        Effect::new(crate::effects::LookAtTopCardsEffect::new(
            PlayerFilter::DamagedPlayer,
            Value::Fixed(1),
            looked.clone(),
        )),
        Effect::new(
            crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(looked)).with_face_down(true),
        ),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            permission_tag,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled,
            true,
            ironsmith_core::value_model::ManaSpendMode::AnyColor,
        )),
    ]))
}

#[test]
fn comma_then_look_exile_permission_uses_the_shared_tag_as_a_singular_pronoun() {
    let sequence =
        looked_card_permission_sequence(TagKey::from("__sentence_helper_looked_exiled_l0_s0_e0"));

    assert_eq!(
        describe_effect(&sequence),
        "Look at the top card of their library, then exile it face down. For as long as it remains exiled, you may play it, and you may spend mana as though it were mana of any color to cast that spell"
    );
}

#[test]
fn comma_then_look_exile_permission_does_not_fold_an_unrelated_grant() {
    let sequence = looked_card_permission_sequence(TagKey::from("unrelated_permission"));
    let rendered = describe_effect(&sequence);

    assert!(
        !rendered.contains("For as long as it remains exiled, you may play it"),
        "an unrelated grant must not acquire the looked card's singular antecedent: {rendered}"
    );
}

fn graveyard_pile_effects(exile_tag: TagKey) -> Vec<Effect> {
    let chosen = TagKey::from("divvy_chosen");
    let graveyard_creatures = ObjectFilter::creature()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            graveyard_creatures.clone(),
            ChoiceCount::any_number(),
            PlayerFilter::Opponent,
            chosen.clone(),
        )
        .in_zone(Zone::Graveyard),
    );
    let exile = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(exile_tag),
        Zone::Exile,
        true,
    ));
    let return_other = Effect::new(
        crate::effects::ReturnAllToBattlefieldEffect::new(
            graveyard_creatures.not_tagged(chosen),
            false,
        )
        .under_you_control(),
    )
    .tag("returned_0");

    vec![choose, exile, return_other]
}

#[test]
fn graveyard_divvy_clause_routes_the_flat_typed_bundle_before_fallback() {
    let effects = graveyard_pile_effects(TagKey::from("divvy_chosen"));

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "Separate all creature cards in your graveyard into two piles. Exile the pile of an opponent's choice and return the other to the battlefield."
        )
    );
}

#[test]
fn graveyard_divvy_clause_does_not_fold_an_unrelated_exile() {
    let effects = graveyard_pile_effects(TagKey::from("unrelated_pile"));
    let rendered = describe_effect_clause_list(&effects);

    assert_ne!(
        rendered.as_deref(),
        Some(
            "Separate all creature cards in your graveyard into two piles. Exile the pile of an opponent's choice and return the other to the battlefield."
        ),
        "the exile must consume the pile selected by the opponent"
    );
}
