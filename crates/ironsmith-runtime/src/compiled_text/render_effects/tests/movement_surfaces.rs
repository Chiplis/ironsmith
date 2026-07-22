use super::*;

#[test]
fn typed_move_surface_preserves_put_return_and_actor_agreement() {
    let put = crate::effects::MoveToZoneEffect::new(ChooseSpec::Source, Zone::Hand, false)
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
        .with_actor_surface(PlayerFilter::Opponent);
    let put_text = describe_effect(&Effect::new(put));
    assert!(put_text.starts_with("An opponent puts "), "{put_text}");
    assert!(put_text.contains(" into "), "{put_text}");

    let returned = crate::effects::MoveToZoneEffect::new(ChooseSpec::Source, Zone::Hand, false)
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
        .with_actor_surface(PlayerFilter::Opponent);
    let return_text = describe_effect(&Effect::new(returned));
    assert!(
        return_text.starts_with("An opponent returns "),
        "{return_text}"
    );
    assert!(return_text.contains(" to "), "{return_text}");

    let you_put = crate::effects::MoveToZoneEffect::new(ChooseSpec::Source, Zone::Exile, false)
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
        .with_actor_surface(PlayerFilter::You);
    let you_put_text = describe_effect(&Effect::new(you_put));
    assert!(you_put_text.starts_with("You put "), "{you_put_text}");
    assert!(you_put_text.ends_with(" into exile"), "{you_put_text}");
}

#[test]
fn typed_move_surface_preserves_plural_tagged_sets_and_contextual_actor() {
    let tag = TagKey::from("exiled_set");
    let move_set =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag), Zone::Battlefield, false)
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
            .with_target_plural_surface()
            .with_actor_surface(PlayerFilter::Active)
            .with_destination_player_surface(PlayerFilter::Active)
            .with_destination_player_reference_surface(
                ironsmith_core::DestinationPlayerReferenceSurface::Pronoun,
            )
            .under_owner_control();

    let text = describe_effect(&Effect::new(move_set));
    assert!(text.starts_with("That player puts "), "{text}");
    assert!(text.contains("them onto the battlefield"), "{text}");
    assert!(text.contains("their owners' control"), "{text}");
}

#[test]
fn bulk_battlefield_move_retains_printed_put_surface() {
    let put_all = crate::effects::ReturnAllToBattlefieldEffect::new(
        ObjectFilter::creature().in_zone(Zone::Graveyard),
        false,
    )
    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
    .under_you_control();

    let text = describe_effect_list(&[Effect::new(put_all)]);
    assert!(text.starts_with("Put all creature cards"), "{text}");
    assert!(text.contains("onto the battlefield"), "{text}");
    assert!(text.ends_with("under your control"), "{text}");
}

#[test]
fn bulk_battlefield_move_pluralizes_cards_from_hand() {
    let put_all = crate::effects::ReturnAllToBattlefieldEffect::new(
        ObjectFilter::land()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You),
        false,
    )
    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put);

    assert_eq!(
        describe_effect_list(&[Effect::new(put_all)]),
        "Put all land cards in your hand onto the battlefield under their owners' control"
    );
}

#[test]
fn choose_then_move_retains_explicit_battlefield_controller_surface() {
    let tag = TagKey::from("chosen");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .owned_by(PlayerFilter::Opponent)
            .in_zone(Zone::Graveyard),
        1usize,
        PlayerFilter::You,
        tag.clone(),
    );
    let move_chosen =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag), Zone::Battlefield, false)
            .under_you_control();

    let text = describe_effect_list(&[Effect::new(choose), Effect::new(move_chosen)]);
    assert!(text.ends_with("under your control"), "{text}");
}

#[test]
fn search_then_move_retains_explicit_battlefield_controller_surface() {
    let tag = TagKey::from("searched");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .owned_by(PlayerFilter::Opponent)
            .in_zone(Zone::Library),
        1usize,
        PlayerFilter::You,
        tag.clone(),
    )
    .as_search();
    let move_chosen =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag), Zone::Battlefield, false)
            .under_you_control();

    let text = describe_effect_list(&[Effect::new(choose), Effect::new(move_chosen)]);
    assert!(text.contains("under your control"), "{text}");
}

#[test]
fn triggering_return_then_counter_uses_exact_returned_object_link() {
    let triggering = TagKey::from("triggering");
    let returned = TagKey::from("returned");
    let move_back = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(triggering),
        Zone::Battlefield,
        false,
    )
    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
    .under_you_control();
    let counter = Effect::put_counters(
        crate::object::CounterType::Named("death"),
        Value::Fixed(1),
        ChooseSpec::Tagged(returned.clone()),
    );

    let text = describe_effect_list(&[Effect::new(move_back.clone()).tag(returned), counter]);
    assert_eq!(
        text,
        "Return it to the battlefield under your control and put a death counter on it"
    );

    let put_surface = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(TagKey::from("triggering")),
        Zone::Battlefield,
        false,
    )
    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
    .under_you_control();
    let put_surface_text = describe_effect_list(&[
        Effect::new(put_surface).tag(TagKey::from("put_result")),
        Effect::put_counters(
            crate::object::CounterType::Named("death"),
            Value::Fixed(1),
            ChooseSpec::Tagged(TagKey::from("put_result")),
        ),
    ]);
    assert!(
        !put_surface_text.contains("and put a death counter on it"),
        "{put_surface_text}"
    );

    let nonmatching = describe_effect_list(&[
        Effect::new(move_back).tag(TagKey::from("other_return")),
        Effect::put_counters(
            crate::object::CounterType::Named("death"),
            Value::Fixed(1),
            ChooseSpec::Tagged(TagKey::from("different_return")),
        ),
    ]);
    assert!(
        !nonmatching.contains("and put a death counter on it"),
        "{nonmatching}"
    );
}

#[test]
fn chosen_exile_then_counter_accepts_only_exact_single_source_exiled_result() {
    let chosen = TagKey::from("chosen_exile");
    let mut filter = ObjectFilter::default().owned_by(PlayerFilter::You);
    filter.card_types = vec![CardType::Artifact, CardType::Creature];
    let choose = crate::effects::ChooseObjectsEffect::new(
        filter,
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zones(vec![Zone::Hand, Zone::Graveyard]);
    let exile = Effect::exile(ChooseSpec::Tagged(chosen));
    let counter = Effect::put_counters(
        crate::object::CounterType::Named("cage"),
        Value::Fixed(1),
        ChooseSpec::Tagged(TagKey::from(crate::tag::SOURCE_EXILED_TAG)),
    );

    let text = describe_effect_list(&[Effect::new(choose.clone()), exile.clone(), counter]);
    assert!(
        text.contains("from your hand or graveyard and put a cage counter on it"),
        "{text}"
    );

    let many = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You),
        ChoiceCount::any_number(),
        PlayerFilter::You,
        TagKey::from("many_exiled"),
    )
    .in_zone(Zone::Graveyard);
    let many_text = describe_effect_list(&[
        Effect::new(many),
        Effect::exile(ChooseSpec::Tagged(TagKey::from("many_exiled"))),
        Effect::put_counters(
            crate::object::CounterType::Named("cage"),
            Value::Fixed(1),
            ChooseSpec::Tagged(TagKey::from(crate::tag::SOURCE_EXILED_TAG)),
        ),
    ]);
    assert!(
        !many_text.contains("and put a cage counter on it"),
        "{many_text}"
    );

    let nonmatching = describe_effect_list(&[
        Effect::new(choose),
        exile,
        Effect::put_counters(
            crate::object::CounterType::Named("cage"),
            Value::Fixed(1),
            ChooseSpec::Tagged(TagKey::from("unrelated_exile")),
        ),
    ]);
    assert!(
        !nonmatching.contains("and put a cage counter on it"),
        "{nonmatching}"
    );
}

#[test]
fn chosen_graveyard_return_then_counter_compacts_only_the_return_result() {
    let chosen = TagKey::from("chosen_return");
    let returned = TagKey::from("returned");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zone(Zone::Graveyard);
    let move_back = Effect::return_from_graveyard_to_battlefield(ChooseSpec::Tagged(chosen), false)
        .tag(returned.clone());
    let counter = Effect::put_counters(
        crate::object::CounterType::Finality,
        Value::Fixed(1),
        ChooseSpec::Tagged(returned),
    );

    let text = describe_effect_list(&[Effect::new(choose.clone()), move_back.clone(), counter]);
    assert!(
        text.contains("to the battlefield with a finality counter on it"),
        "{text}"
    );

    let nonmatching = describe_effect_list(&[
        Effect::new(choose),
        move_back,
        Effect::put_counters(
            crate::object::CounterType::Finality,
            Value::Fixed(1),
            ChooseSpec::Tagged(TagKey::from("different_return")),
        ),
    ]);
    assert!(
        !nonmatching.contains("to the battlefield with a finality counter on it"),
        "{nonmatching}"
    );
}

#[test]
fn counter_then_source_attachment_joins_only_on_the_same_object() {
    let created = TagKey::from("created");
    let token = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Fractal")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Fractal])
        .color_indicator(crate::color::ColorSet::GREEN.union(crate::color::ColorSet::BLUE))
        .power_toughness(crate::card::PowerToughness::fixed(0, 0))
        .build();
    let create = Effect::new(crate::effects::CreateTokenEffect::new(
        token,
        Value::Fixed(1),
        PlayerFilter::You,
    ))
    .tag(created.clone());
    let counter = Effect::put_counters(
        crate::object::CounterType::PlusOnePlusOne,
        Value::X.with_surface_hint(ValueSurfaceHint::CounterFollowupSeparateSentence),
        ChooseSpec::Tagged(created.clone()),
    )
    .tag(TagKey::from("counters"));
    let attach = Effect::new(crate::effects::AttachObjectsEffect::new(
        ChooseSpec::Source,
        ChooseSpec::Tagged(created),
    ));

    let text = describe_effect_list(&[create.clone(), counter.clone(), attach]);
    assert!(
        text.contains(". Put X +1/+1 counters on it and attach "),
        "{text}"
    );
    assert!(!text.contains(". Attach "), "{text}");

    let nonmatching = describe_effect_list(&[
        create,
        counter,
        Effect::new(crate::effects::AttachObjectsEffect::new(
            ChooseSpec::Source,
            ChooseSpec::Tagged(TagKey::from("different_created")),
        )),
    ]);
    assert!(nonmatching.contains(". Attach "), "{nonmatching}");
}
