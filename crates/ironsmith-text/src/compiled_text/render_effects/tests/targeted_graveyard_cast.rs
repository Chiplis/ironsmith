use super::*;

fn targeted_graveyard_cast(
    owner: Option<PlayerFilter>,
    card_types: &[CardType],
    colors: Option<crate::color::ColorSet>,
    additional_mana_cost: Option<crate::mana::ManaCost>,
    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
) -> Vec<Effect> {
    let target_tag = TagKey::from("targeted_graveyard_card");
    let cast_spell_tag = TagKey::from("cast_spell");
    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    filter.owner = owner;
    filter.card_types = card_types.to_vec();
    filter.colors = colors;

    let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
        ChooseSpec::Object(filter),
    )))
    .tag(target_tag.clone());
    let cast = crate::effects::CastTaggedEffect::new(target_tag, PlayerFilter::You)
        .mana_spend_mode(mana_spend_mode);
    let cast = if let Some(additional_mana_cost) = additional_mana_cost {
        cast.additional_mana_cost(additional_mana_cost)
    } else {
        cast.without_paying_mana_cost()
    };
    let cast = Effect::new(cast).tag(cast_spell_tag.clone());
    let may_cast = Effect::with_id(0, Effect::may(vec![cast]));
    let replacement = Effect::new(crate::effects::RegisterFutureZoneReplacementEffect::new(
        ObjectFilter::tagged(cast_spell_tag).in_zone(Zone::Stack),
        Some(Zone::Stack),
        Some(Zone::Graveyard),
        Zone::Exile,
        crate::effects::ReplacementApplyMode::OneShot,
    ));
    let if_cast = Effect::if_then(
        crate::effect::EffectId(0),
        crate::effect::EffectPredicate::Happened,
        vec![replacement],
    );

    vec![target, may_cast, if_cast]
}

fn duration_scoped_targeted_graveyard_cast(without_paying_mana_cost: bool) -> Vec<Effect> {
    let target_tag = TagKey::from("duration_targeted_graveyard_card");
    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);
    filter.card_types = vec![CardType::Instant, CardType::Sorcery];
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
        ChooseSpec::Object(filter),
    )))
    .tag(target_tag.clone());
    let surface = ironsmith_core::GrantPlayTaggedSurface::default()
        .with_leading_duration(true)
        .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard);
    let grant = Effect::new(
        crate::effects::GrantPlayTaggedEffect::new(
            target_tag.clone(),
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            false,
            false,
        )
        .with_surface(surface),
    );
    let replacement = Effect::new(crate::effects::RegisterFutureZoneReplacementEffect::new(
        ObjectFilter::tagged(target_tag.clone()).in_zone(Zone::Stack),
        Some(Zone::Stack),
        Some(Zone::Graveyard),
        Zone::Exile,
        crate::effects::ReplacementApplyMode::UntilEndOfTurn,
    ));
    let mut effects = vec![target, grant];
    if without_paying_mana_cost {
        effects.push(Effect::new(
            crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(
                target_tag,
                PlayerFilter::You,
            )
            .from_current_zone(),
        ));
    }
    effects.push(replacement);
    effects
}

fn immediate_damaged_player_nonland_permanent_cast() -> Vec<Effect> {
    let target_tag = TagKey::from("targeted_damaged_player_graveyard_card");
    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::DamagedPlayer);
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    filter.excluded_card_types = vec![CardType::Land];
    filter.set_explicit_card_noun(true);
    vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
            ChooseSpec::Object(filter),
        )))
        .tag(target_tag.clone()),
        Effect::may(vec![Effect::new(
            crate::effects::CastTaggedEffect::new(target_tag, PlayerFilter::You)
                .mana_spend_mode(ironsmith_core::value_model::ManaSpendMode::AnyType),
        )]),
    ]
}

#[test]
fn immediate_damaged_player_graveyard_cast_keeps_target_and_mana_mode() {
    let effects = immediate_damaged_player_nonland_permanent_cast();
    assert_eq!(
        describe_effect_list(&effects),
        "You may cast target nonland permanent card from that player's graveyard, and mana of any type can be spent to cast that spell"
    );

    let mut wrong_mode = effects.clone();
    let may = wrong_mode[1]
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("second effect should be optional")
        .clone();
    let cast = may.effects[0]
        .downcast_ref::<crate::effects::CastTaggedEffect>()
        .expect("optional effect should cast the tagged target")
        .clone()
        .mana_spend_mode(ironsmith_core::value_model::ManaSpendMode::Normal);
    wrong_mode[1] = Effect::may(vec![Effect::new(cast)]);
    assert_ne!(
        describe_effect_list(&wrong_mode),
        "You may cast target nonland permanent card from that player's graveyard, and mana of any type can be spent to cast that spell"
    );

    let mut wrong_tag = effects.clone();
    wrong_tag[1] = Effect::may(vec![Effect::new(
        crate::effects::CastTaggedEffect::new("another_target", PlayerFilter::You)
            .mana_spend_mode(ironsmith_core::value_model::ManaSpendMode::AnyType),
    )]);
    assert_ne!(
        describe_effect_list(&wrong_tag),
        "You may cast target nonland permanent card from that player's graveyard, and mana of any type can be spent to cast that spell"
    );
}

#[test]
fn single_type_targeted_cast_from_your_graveyard_keeps_target_surface() {
    assert_eq!(
        describe_effect_list(&targeted_graveyard_cast(
            Some(PlayerFilter::You),
            &[CardType::Instant],
            None,
            None,
            ironsmith_core::value_model::ManaSpendMode::Normal,
        )),
        "You may cast target instant card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead"
    );
}

#[test]
fn targeted_cast_from_any_graveyard_keeps_indefinite_zone_surface() {
    assert_eq!(
        describe_effect_list(&targeted_graveyard_cast(
            None,
            &[CardType::Instant, CardType::Sorcery],
            None,
            None,
            ironsmith_core::value_model::ManaSpendMode::Normal,
        )),
        "You may cast target instant or sorcery card from a graveyard without paying its mana cost. If that spell would be put into a graveyard, exile it instead"
    );
}

#[test]
fn reflexive_target_choice_rejoins_the_cast_and_linked_replacement() {
    let mut effects = targeted_graveyard_cast(
        None,
        &[CardType::Instant, CardType::Sorcery],
        None,
        None,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );
    let target = effects.remove(0);
    let tagged = target
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("target declaration should retain its tag");
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .expect("tagged declaration should contain one target");
    let reflexive = crate::effects::ReflexiveTriggerEffect::new(
        crate::effect::EffectId(17),
        crate::effect::EffectPredicate::Happened,
        effects,
        vec![target_only.target.clone()],
    );

    assert_eq!(
        describe_reflexive_targeted_graveyard_cast_with_replacement(&reflexive),
        Some("You may cast target instant or sorcery card from a graveyard without paying its mana cost. If that spell would be put into a graveyard, exile it instead".to_string())
    );
}

#[test]
fn reflexive_redundant_target_member_rejoins_only_when_it_matches_choice() {
    let effects = targeted_graveyard_cast(
        None,
        &[CardType::Instant, CardType::Sorcery],
        None,
        None,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );
    let target = effects[0]
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("target declaration should retain its tag")
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .expect("tagged declaration should contain one target")
        .target
        .clone();
    let reflexive = crate::effects::ReflexiveTriggerEffect::new(
        crate::effect::EffectId(17),
        crate::effect::EffectPredicate::Happened,
        effects.clone(),
        vec![target],
    );

    assert_eq!(
        describe_reflexive_targeted_graveyard_cast_with_replacement(&reflexive),
        Some("You may cast target instant or sorcery card from a graveyard without paying its mana cost. If that spell would be put into a graveyard, exile it instead".to_string())
    );

    let mut wrong_choice = reflexive.clone();
    wrong_choice.choices = vec![ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature().in_zone(Zone::Graveyard),
    ))];
    assert_eq!(
        describe_reflexive_targeted_graveyard_cast_with_replacement(&wrong_choice),
        None,
        "a retained target declaration must not be folded with an unrelated choice"
    );
}

#[test]
fn targeted_colored_cast_keeps_color_and_replacement_antecedent() {
    let effects = targeted_graveyard_cast(
        Some(PlayerFilter::You),
        &[CardType::Instant, CardType::Sorcery],
        Some(crate::color::ColorSet::RED),
        None,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );

    assert_eq!(
        describe_effect_list(&effects),
        "You may cast target red instant or sorcery card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead"
    );
}

#[test]
fn targeted_cast_from_graveyard_renders_instruction_additional_cost() {
    let effects = targeted_graveyard_cast(
        Some(PlayerFilter::You),
        &[CardType::Instant, CardType::Sorcery],
        None,
        Some(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Red,
            crate::mana::ManaSymbol::Red,
        ])),
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );

    assert_eq!(
        describe_effect_list(&effects),
        "You may cast target instant or sorcery card from your graveyard by paying {R}{R} in addition to its other costs. If that spell would be put into your graveyard, exile it instead"
    );
}

#[test]
fn targeted_cast_from_graveyard_keeps_any_type_mana_permission() {
    let effects = targeted_graveyard_cast(
        None,
        &[CardType::Instant, CardType::Sorcery],
        None,
        None,
        ironsmith_core::value_model::ManaSpendMode::AnyType,
    );

    assert_eq!(
        describe_effect_list(&effects),
        "You may cast target instant or sorcery card from a graveyard, and mana of any type can be spent to cast that spell. If that spell would be put into a graveyard, exile it instead"
    );
}

#[test]
fn duration_scoped_targeted_cast_keeps_the_permission_and_replacement_windows() {
    assert_eq!(
        describe_effect_list(&duration_scoped_targeted_graveyard_cast(true)),
        "Until end of turn, you may cast target instant or sorcery card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead"
    );
    assert_eq!(
        describe_effect_list(&duration_scoped_targeted_graveyard_cast(false)),
        "Until end of turn, you may cast target instant or sorcery card from your graveyard. If that spell would be put into your graveyard, exile it instead"
    );
}

#[test]
fn duration_scoped_targeted_cast_preserves_one_trailing_effect() {
    let mut effects = duration_scoped_targeted_graveyard_cast(true);
    effects.push(Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Source,
        Zone::Exile,
        false,
    )));
    assert_eq!(
        describe_effect_list(&effects),
        "Until end of turn, you may cast target instant or sorcery card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead. Exile this source"
    );
}
