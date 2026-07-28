use super::*;

fn targeted_graveyard_cast(
    owner: Option<PlayerFilter>,
    card_types: &[CardType],
    colors: Option<crate::color::ColorSet>,
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
    let cast = Effect::new(
        crate::effects::CastTaggedEffect::new(target_tag, PlayerFilter::You)
            .without_paying_mana_cost(),
    )
    .tag(cast_spell_tag.clone());
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

#[test]
fn single_type_targeted_cast_from_your_graveyard_keeps_target_surface() {
    assert_eq!(
        describe_effect_list(&targeted_graveyard_cast(
            Some(PlayerFilter::You),
            &[CardType::Instant],
            None,
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
        )),
        "You may cast target instant or sorcery card from a graveyard without paying its mana cost. If that spell would be put into a graveyard, exile it instead"
    );
}

#[test]
fn targeted_colored_cast_keeps_color_and_replacement_antecedent() {
    let effects = targeted_graveyard_cast(
        Some(PlayerFilter::You),
        &[CardType::Instant, CardType::Sorcery],
        Some(crate::color::ColorSet::RED),
    );

    assert_eq!(
        describe_effect_list(&effects),
        "You may cast target red instant or sorcery card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead"
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
