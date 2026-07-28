use super::shard_01::*;
use super::*;

#[test]
pub(super) fn ward_keyword_renderer_distinguishes_pure_mana_and_composite_costs() {
    let pure_mana = Ability::static_ability(crate::static_abilities::StaticAbility::ward(
        crate::cost::TotalCost::mana(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Generic(8),
        ])),
    ));
    assert_eq!(
        describe_keyword_ability(&pure_mana).as_deref(),
        Some("Ward {8}")
    );

    let composite = Ability::static_ability(crate::static_abilities::StaticAbility::ward(
        crate::cost::TotalCost::from_costs(vec![
            crate::costs::Cost::mana(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Generic(2),
            ])),
            crate::costs::Cost::life(3),
        ]),
    ));
    assert_eq!(
        describe_keyword_ability(&composite).as_deref(),
        Some("Ward—{2}, Pay 3 life")
    );
}

#[test]
pub(super) fn mana_rendering_uses_current_oracle_surface_for_your_pool() {
    assert_eq!(
        describe_effect(&Effect::add_mana_of_any_one_color(Value::Fixed(3))),
        "Add three mana of any one color"
    );

    let scaled = Effect::new(crate::effects::AddScaledManaEffect::new(
        vec![ManaSymbol::Red],
        Value::Count(ObjectFilter::creature()),
        PlayerFilter::You,
    ));
    assert_eq!(
        describe_effect(&scaled),
        "Add {R} for each creature on the battlefield"
    );
}

#[test]
pub(super) fn scaled_mana_preserves_sacrificed_object_characteristic_surface() {
    for (kind, noun) in [
        (ironsmith_core::SacrificedObjectKind::Creature, "creature"),
        (ironsmith_core::SacrificedObjectKind::Artifact, "artifact"),
    ] {
        let amount = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(
            "sacrifice_cost_0",
        ))))
        .with_surface_hint(ValueSurfaceHint::SacrificedObject(kind));
        let scaled = Effect::new(crate::effects::AddScaledManaEffect::new(
            vec![ManaSymbol::Black],
            amount,
            PlayerFilter::You,
        ));
        assert_eq!(
            describe_effect(&scaled),
            format!("Add an amount of {{B}} equal to the sacrificed {noun}'s mana value")
        );
    }
}

#[test]
pub(super) fn consult_any_number_to_battlefield_hides_internal_tags() {
    let revealed = TagKey::from("revealed_cards");
    let matched = TagKey::from("matched_permanents");
    let chosen = TagKey::from("chosen_permanents");
    let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
        PlayerFilter::You,
        crate::effects::consult_helpers::LibraryConsultMode::Reveal,
        ObjectFilter::permanent(),
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::ColorsAmong(
            ObjectFilter::permanent()
                .in_zone(Zone::Battlefield)
                .controlled_by(PlayerFilter::You),
        )),
        revealed.clone(),
        matched.clone(),
    ));
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::tagged(matched).in_zone(Zone::Library),
            ChoiceCount::any_number(),
            PlayerFilter::You,
            chosen.clone(),
        )
        .in_zone(Zone::Library),
    );
    let move_each = Effect::new(crate::effects::ForEachTaggedEffect::new(
        chosen.clone(),
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Battlefield,
            false,
        ))],
    ));
    let bottom = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(chosen),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    );

    let triggering = Effect::new(crate::effects::TagTriggeringObjectEffect::new("triggering"));
    let rendered = describe_effect_list(&[triggering, consult, choose, move_each, bottom]);
    assert_eq!(
        rendered,
        "Reveal cards from the top of your library until you reveal X permanent cards, where X is the number of colors among permanents you control. Put any number of those permanent cards onto the battlefield, then put the rest of the revealed cards on the bottom of your library in a random order"
    );
    assert!(!rendered.contains("tagged"));
}

#[test]
pub(super) fn controller_owned_consult_uses_imperative_surface() {
    let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
        PlayerFilter::You,
        crate::effects::consult_helpers::LibraryConsultMode::Reveal,
        ObjectFilter::default().with_subtype(Subtype::Equipment),
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)),
        TagKey::from("revealed"),
        TagKey::from("matched"),
    ));

    assert_eq!(
        describe_effect(&consult),
        "Reveal cards from the top of your library until you reveal an Equipment card"
    );
}

#[test]
pub(super) fn revealed_collection_bottom_move_keeps_authored_action_surface() {
    let bottom = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            TagKey::from("__sentence_helper_revealed_test"),
            None,
            crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses,
            PlayerFilter::You,
        )
        .with_surface(ironsmith_core::LibraryRemainderSurface::CardsYouRevealedThisWay),
    );

    assert_eq!(
        describe_effect(&bottom),
        "Put the cards you revealed this way on the bottom of your library in any order"
    );
}

#[test]
pub(super) fn triggering_spell_relative_consult_keeps_terse_oracle_surface() {
    let exiled = TagKey::from("__sentence_helper_exiled_test");
    let matched = TagKey::from("__sentence_helper_consult_match_test");
    let mut filter = ObjectFilter::default()
        .without_type(CardType::Land)
        .with_supertype(Supertype::Legendary);
    filter.set_explicit_card_noun(true);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("triggering"),
        relation: TaggedOpbjectRelation::ManaValueLtTagged,
    });
    let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
        PlayerFilter::You,
        crate::effects::consult_helpers::LibraryConsultMode::Exile,
        filter,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)),
        exiled.clone(),
        matched.clone(),
    ));
    let may_cast = Effect::new(crate::effects::MayEffect::new_for_player(
        vec![Effect::new(
            crate::effects::CastTaggedEffect::new(matched.clone(), PlayerFilter::You)
                .without_paying_mana_cost(),
        )],
        PlayerFilter::You,
    ));
    let remainder = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            exiled,
            Some(matched),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    );

    assert_eq!(
        describe_effect_list(&[consult, may_cast, remainder]),
        "you exile cards from the top of your library until you exile a legendary nonland card with lesser mana value. you may cast that card without paying its mana cost. Put the rest on the bottom of your library in a random order"
    );
}

#[test]
pub(super) fn consult_result_battlefield_attachment_compacts_with_remainder() {
    let revealed = TagKey::from("__sentence_helper_revealed_test");
    let matched = TagKey::from("__sentence_helper_consult_match_test");
    let moved = TagKey::from("moved_0");
    let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
        PlayerFilter::You,
        crate::effects::consult_helpers::LibraryConsultMode::Reveal,
        ObjectFilter::default().with_subtype(Subtype::Equipment),
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)),
        revealed.clone(),
        matched.clone(),
    ));
    let move_match = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(matched.clone()),
        Zone::Battlefield,
        false,
    ))
    .tag(moved.clone());
    let attachment_target = ChooseSpec::Tagged(TagKey::from("triggering")).with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("that creature".to_string()),
        ),
    );
    let attach = Effect::new(crate::effects::AttachObjectsEffect::new(
        ChooseSpec::All(ObjectFilter::tagged(moved)),
        attachment_target,
    ));
    let remainder = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(matched),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    );

    assert_eq!(
        describe_cross_segment_consult_bundle(&[consult, move_match, attach, remainder]).as_deref(),
        Some(
            "Reveal cards from the top of your library until you reveal an Equipment card. Put that card onto the battlefield attached to that creature, then put the rest on the bottom of your library in a random order"
        )
    );
}

#[test]
pub(super) fn cross_segment_variable_consult_keeps_typed_matched_collection() {
    let revealed = TagKey::from("__sentence_helper_exiled_test");
    let matched = TagKey::from("__sentence_helper_consult_match_test");
    let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
        PlayerFilter::You,
        crate::effects::consult_helpers::LibraryConsultMode::Exile,
        ObjectFilter::permanent_card(),
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(
            Value::SourceMutationCount.with_surface_hint(ValueSurfaceHint::WhereXIs),
        ),
        revealed,
        matched.clone(),
    ));
    let move_matches = Effect::new(
        crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(matched.clone()),
            Zone::Battlefield,
            false,
        )
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
        .with_target_plural_surface(),
    );

    let effects = [consult, move_matches];
    let move_to_zone = effects[1]
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .expect("second effect should be a move");
    assert!(choose_spec_references_tagged_object(
        &move_to_zone.target,
        &matched
    ));
    assert_eq!(
        describe_structural_multisentence_effect_list(&effects).as_deref(),
        Some(
            "Exile cards from the top of your library until you exile X permanent cards, where X is the number of times this creature has mutated. Put those permanent cards onto the battlefield"
        )
    );
    assert_eq!(
        describe_cross_segment_consult_bundle(&effects).as_deref(),
        Some(
            "Exile cards from the top of your library until you exile X permanent cards, where X is the number of times this creature has mutated. Put those permanent cards onto the battlefield"
        )
    );
}

#[test]
pub(super) fn reweave_consult_match_move_shuffle_hides_internal_match_tag() {
    let sacrificed = TagKey::from("sacrificed_0");
    let all_revealed = TagKey::from("__sentence_helper_revealed_l0_s0_e1");
    let matched = TagKey::from("__sentence_helper_consult_match_l0_s0_e1");
    let player =
        PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(sacrificed.clone()));
    let mut filter = ObjectFilter::permanent_card();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: sacrificed,
        relation: TaggedOpbjectRelation::SharesCardType,
    });
    let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
        player.clone(),
        crate::effects::consult_helpers::LibraryConsultMode::Reveal,
        filter,
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
        all_revealed,
        matched.clone(),
    ));
    let put_match = Effect::new(crate::effects::ForEachTaggedEffect::new(
        matched,
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Battlefield,
            false,
        ))],
    ));
    let shuffle = Effect::new(crate::effects::ShuffleLibraryEffect::new(player));

    let rendered = describe_effect_list(&[consult, put_match, shuffle]);
    assert!(
        rendered.starts_with("that player reveals cards from the top of their library"),
        "{rendered}"
    );
    assert!(
        rendered.contains("shares a card type with the sacrificed permanent"),
        "{rendered}"
    );
    assert!(
        rendered.ends_with(", puts that card onto the battlefield, then shuffles"),
        "{rendered}"
    );
    assert!(!rendered.contains("tagged"), "{rendered}");
    assert!(!rendered.contains("consult_match"), "{rendered}");
}

#[test]
pub(super) fn targeted_sacrifice_result_names_the_acting_controller_in_consult_followup() {
    let sacrificed = TagKey::from("sacrificed_0");
    let matched = TagKey::from("consult_match_0");
    let player = PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Target);
    let sacrifice = Effect::with_id(
        0,
        Effect::new(crate::effects::SacrificeTargetEffect::new(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::permanent())),
        ))
        .tag(sacrificed.clone()),
    );
    let mut filter = ObjectFilter::permanent_card();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: sacrificed,
        relation: TaggedOpbjectRelation::SharesCardType,
    });
    let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
        player.clone(),
        crate::effects::consult_helpers::LibraryConsultMode::Reveal,
        filter,
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
        "consult_revealed_0",
        matched.clone(),
    ));
    let put_match = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(matched),
        Zone::Battlefield,
        false,
    ));
    let shuffle = Effect::new(crate::effects::ShuffleLibraryEffect::new(player));
    let conditional = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![consult, put_match, shuffle],
    );

    assert_eq!(
        describe_effect_list(&[sacrifice, conditional]),
        "Target permanent's controller sacrifices it. If the player does, they reveal cards from the top of their library until they reveal a permanent card that shares a card type with the sacrificed permanent, put that card onto the battlefield, then shuffle"
    );
}

#[test]
pub(super) fn repeat_process_renders_counted_shared_characteristic_result_gate() {
    let mill = Effect::with_id(
        0,
        Effect::mill_player(Value::Fixed(2), PlayerFilter::Opponent),
    );
    let mut nonland = ObjectFilter::default();
    nonland.excluded_card_types.push(CardType::Land);
    nonland.set_explicit_card_noun(true);
    let predicate = EffectPredicate::PriorEffectResult(
        crate::effect::PriorEffectResultSurface::new(
            crate::effect::PriorEffectAction::Milled,
            nonland,
            crate::effect::PriorEffectResultActor::Passive,
            crate::effect::PriorEffectResultQuantifier::One,
        )
        .with_count_sharing(2, crate::ObjectCharacteristic::Color),
    );
    let repeat = Effect::repeat_process(vec![mill], crate::effect::EffectId(0), predicate);

    assert_eq!(
        describe_effect(&repeat),
        "an opponent mills two cards. If two nonland cards that share a color were milled this way, repeat this process"
    );
}

#[test]
pub(super) fn repeated_revealed_permanent_groups_hide_internal_tags() {
    let revealed = TagKey::from("revealed_cards");
    let revealed_collection = TagKey::from("revealed_collection");
    let moved = TagKey::from("moved_permanents");
    let shuffled = ObjectFilter::permanent()
        .in_zone(Zone::Battlefield)
        .owned_by(PlayerFilter::You);
    let shuffle = Effect::with_id(
        0,
        Effect::new(crate::effects::ShuffleObjectsIntoLibraryEffect::new(
            ChooseSpec::Object(shuffled),
            PlayerFilter::You,
        )),
    );
    let look = Effect::new(crate::effects::LookAtTopCardsEffect::revealing(
        PlayerFilter::You,
        Value::EffectMetric {
            effect_id: crate::effect::EffectId(0),
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::Count,
        },
        revealed.clone(),
    ));
    let tagged_revealed = |mut filter: ObjectFilter| {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: revealed_collection.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        filter
    };
    let mut non_aura = tagged_revealed(ObjectFilter::permanent());
    non_aura.excluded_subtypes.push(crate::types::Subtype::Aura);
    let aura = tagged_revealed(ObjectFilter::default().with_subtype(crate::types::Subtype::Aura));
    let mut union = ObjectFilter::default();
    union.any_of = vec![non_aura, aura];
    let tag = Effect::new(
        crate::effects::TagMatchingObjectsEffect::new(union, moved.clone()).in_zone(Zone::Library),
    );
    let move_each = Effect::new(crate::effects::ForEachTaggedEffect::new(
        moved.clone(),
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Battlefield,
            false,
        ))],
    ));
    let bottom = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(moved),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    );

    let rendered = describe_effect_list(&[shuffle, look, tag, move_each, bottom]);
    assert_eq!(
        rendered,
        "Shuffle all permanents you own into your library, then reveal that many cards from the top of your library. Put all non-Aura permanent cards revealed this way onto the battlefield, then do the same for Aura cards, then put the rest on the bottom of your library in a random order"
    );
    assert!(!rendered.contains("tagged"));
}

#[test]
pub(super) fn return_to_hand_dynamic_count_value_renders_where_clause_and_plural_owner() {
    let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
        .with_count_value(ChoiceCount::up_to_dynamic_x(), Value::Fixed(2));
    let effect = Effect::new(crate::effects::ReturnToHandEffect::with_spec(spec));

    assert_eq!(
        describe_effect(&effect),
        "Return up to X target creatures to their owners' hands, where X is 2"
    );
}

#[test]
pub(super) fn return_to_hand_preserves_plural_tag_reference_surface() {
    let effect = Effect::new(
        crate::effects::ReturnToHandEffect::with_spec(ChooseSpec::Tagged(TagKey::from(
            "returned_0",
        )))
        .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each))
        .with_set_reference_surface(Some("those creatures".to_string())),
    );

    assert_eq!(
        describe_effect(&effect),
        "Return those creatures to their owners' hands"
    );
}

#[test]
pub(super) fn damage_to_one_or_two_any_targets_preserves_each_surface() {
    let target = ChooseSpec::AnyTarget.with_count(ChoiceCount {
        min: 1,
        max: Some(2),
        dynamic_x: false,
        up_to_x: false,
        random: false,
        explicit_exactly: false,
    });

    assert_eq!(
        describe_effect(&Effect::deal_damage(Value::Fixed(2), target)),
        "Deal 2 damage to each of one or two targets"
    );
}

#[test]
pub(super) fn return_from_your_graveyard_dynamic_count_value_keeps_from_to_surface() {
    let mut target = ObjectFilter::default().in_zone(Zone::Graveyard);
    target.owner = Some(PlayerFilter::You);
    let count_value = Value::ColorsAmong(
        ObjectFilter::creature()
            .match_tagged("sacrificed_0", TaggedOpbjectRelation::IsTaggedObject),
    );
    let spec =
        ChooseSpec::Object(target).with_count_value(ChoiceCount::up_to_dynamic_x(), count_value);
    let effect = Effect::return_from_graveyard_to_hand(spec);

    assert_eq!(
        describe_effect(&effect),
        "Return up to X cards from your graveyard to your hand, where X is the number of colors that creature was"
    );
}

#[test]
pub(super) fn rewrite_damage_phrases_keeps_scanning_after_initial_deal_clause() {
    assert_eq!(
        rewrite_damage_phrases_for_permanent_abilities(
            "Deal 2 damage to target creature or planeswalker. If that permanent is green, deal 6 damage instead",
            "Burning Hands",
            false,
        ),
        "Burning Hands deals 2 damage to target creature or planeswalker. If that permanent is green, Burning Hands deals 6 damage instead"
    );
    assert_eq!(
        rewrite_damage_phrases_for_permanent_abilities(
            "Deal 1 damage to target creature. If that creature is white or blue, deal 4 damage to it instead",
            "Lightning Dart",
            false,
        ),
        "Lightning Dart deals 1 damage to target creature. If that creature is white or blue, Lightning Dart deals 4 damage to it instead"
    );
    assert_eq!(
        rewrite_damage_phrases_for_permanent_abilities(
            "Deal 4 damage to any target unless that object's controller pays {2}. If that doesn't happen, deal 2 damage to any target",
            "Rhystic Lightning Variant",
            false,
        ),
        "Rhystic Lightning Variant deals 4 damage to any target unless that object's controller pays {2}. If that doesn't happen, deal 2 damage to any target"
    );
}

#[test]
pub(super) fn describe_effect_list_keeps_sacrifice_return_exile_as_sentences() {
    let sacrificed = TagKey::from("sacrificed_0");
    let choose_sacrificed = Effect::new(crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature().you_control(),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        sacrificed.clone(),
    ));
    let sacrifice = Effect::sacrifice_player(
        ObjectFilter::tagged(sacrificed.clone()),
        1,
        PlayerFilter::You,
    );

    let mut target = ObjectFilter::default().in_zone(Zone::Graveyard);
    target.owner = Some(PlayerFilter::You);
    let count_value = Value::ColorsAmong(
        ObjectFilter::creature().match_tagged(sacrificed, TaggedOpbjectRelation::IsTaggedObject),
    );
    let return_spec =
        ChooseSpec::Object(target).with_count_value(ChoiceCount::up_to_dynamic_x(), count_value);
    let return_to_hand = Effect::return_from_graveyard_to_hand(return_spec).tag("returned_1");

    let source = ChooseSpec::Source.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("this card".to_string()),
        ),
    );
    let exile_source = Effect::new(crate::effects::MoveToZoneEffect::to_exile(source));

    assert_eq!(
        describe_effect_list(&[choose_sacrificed, sacrifice, return_to_hand, exile_source,]),
        "Sacrifice a creature. Return up to X cards from your graveyard to your hand, where X is the number of colors that creature was. Exile this card"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_source_and_target_exile() {
    let source = ChooseSpec::Source.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("this creature".to_string()),
        ),
    );
    let source_exile = Effect::new(crate::effects::MoveToZoneEffect::to_exile(source));
    let target_exile = Effect::new(crate::effects::MoveToZoneEffect::to_exile(
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::permanent())),
    ))
    .tag("__sentence_helper_exiled_l0_s0_e0");
    let effects = vec![source_exile, target_exile];

    assert_eq!(
        describe_effect_list(&effects),
        "Exile this creature and target permanent"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("Exile this creature and target permanent")
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_secret_named_vote_followups() {
    let mut vote = crate::effects::VoteEffect::named(
        vec![
            crate::effects::VoteOption::new("truth", Vec::new()),
            crate::effects::VoteOption::new("consequences", Vec::new()),
        ],
        0,
        0,
    );
    vote.secret = true;
    let vote = Effect::new(vote);
    let truth = Effect::repeat_effects(
        Value::VoteCount("truth".to_string()),
        vec![Effect::new(crate::effects::DrawCardsEffect::you(
            Value::Fixed(1),
        ))],
    );
    let choose_opponent = Effect::new(
        crate::effects::ChoosePlayerEffect::new(
            PlayerFilter::You,
            PlayerFilter::Opponent,
            "chosen_player_0",
        )
        .at_random(),
    );
    let consequences = Effect::repeat_effects(
        Value::VoteCount("consequences".to_string()),
        vec![Effect::new(crate::effects::DealDamageEffect::new(
            Value::Fixed(3),
            ChooseSpec::Player(PlayerFilter::TaggedPlayer(TagKey::from("__it__"))),
        ))],
    );

    assert_eq!(
        describe_effect_list(&[vote, truth, choose_opponent, consequences]),
        "Secret council — Each player secretly votes for truth or consequences, then those votes are revealed. You draw cards equal to the number of truth votes. Then choose an opponent at random. This deals 3 damage to that player for each consequences vote"
    );
}

#[test]
pub(super) fn move_target_face_up_exiled_card_to_graveyard_uses_exiled_surface() {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default().in_zone(Zone::Exile).face_up(),
    ));
    let effect = Effect::new(crate::effects::MoveToZoneEffect::new(
        target,
        Zone::Graveyard,
        false,
    ));

    assert_eq!(
        describe_effect(&effect),
        "Put target face-up exiled card into its owner's graveyard"
    );
}

#[test]
pub(super) fn move_source_card_from_exile_to_battlefield_keeps_from_exile_surface() {
    let target = ChooseSpec::Object(ObjectFilter::source().in_zone(Zone::Exile));
    let effect = Effect::new(
        crate::effects::MoveToZoneEffect::new(target, Zone::Battlefield, false).tapped(),
    );

    assert_eq!(
        describe_effect(&effect),
        "Put this card from exile onto the battlefield tapped"
    );
}

#[test]
pub(super) fn put_source_card_from_exile_onto_battlefield_keeps_from_exile_surface() {
    let target = ChooseSpec::Object(ObjectFilter::source().in_zone(Zone::Exile));
    let effect = Effect::new(crate::effects::PutOntoBattlefieldEffect::new(
        target,
        true,
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect(&effect),
        "Put this card from exile onto the battlefield tapped"
    );
}

#[test]
pub(super) fn source_exile_then_source_exiled_return_compacts_without_graveyard() {
    let exile_source = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Source,
        Zone::Exile,
        true,
    ));
    let return_source = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(TagKey::from(crate::tag::SOURCE_EXILED_TAG)),
        Zone::Battlefield,
        false,
    ));

    assert_eq!(
        describe_effect_list(&[exile_source, return_source]),
        "Exile this, then return it to the battlefield"
    );
}

#[test]
pub(super) fn return_source_card_from_exile_under_owner_control_keeps_from_exile_surface() {
    let target = ChooseSpec::Object(ObjectFilter::source().in_zone(Zone::Exile));
    let effect = Effect::new(
        crate::effects::MoveToZoneEffect::new(target, Zone::Battlefield, false)
            .under_owner_control(),
    );

    assert_eq!(
        describe_effect(&effect),
        "Return this card from exile to the battlefield under its owner's control"
    );
}

#[test]
pub(super) fn describe_false_only_conditional_prefers_unless_surface() {
    assert_eq!(
        describe_false_only_conditional(
            &crate::effect::Condition::AttackedThisTurn,
            "you lose 4 life"
        ),
        "Unless you attacked this turn, you lose 4 life"
    );
}

#[test]
pub(super) fn scaled_mana_fallback_renders_as_amount_equal_to_value() {
    let scaled = Effect::new(crate::effects::AddScaledManaEffect::new(
        vec![ManaSymbol::Red],
        Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from("sacrificed_0")))),
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect(&scaled),
        "Add an amount of {R} equal to its mana value"
    );
}

#[test]
pub(super) fn execute_with_source_damage_preserves_amount_surface_hints() {
    let amount = Value::Count(ObjectFilter::artifact().controlled_by(PlayerFilter::Active))
        .with_surface_hint(ValueSurfaceHint::EqualTo);
    let effect = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
        ChooseSpec::Source,
        Effect::deal_damage(amount, ChooseSpec::Player(PlayerFilter::Active)),
    ));

    assert_eq!(
        describe_effect(&effect),
        "this creature deals damage equal to the number of artifacts they control to that player"
    );
}

#[test]
pub(super) fn power_damage_to_tagged_controllers_keeps_spell_source_surface() {
    let tagged = TagKey::from("destroyed_0");
    let effect = Effect::deal_damage(
        Value::PowerOf(Box::new(ChooseSpec::Tagged(tagged.clone()))),
        ChooseSpec::Player(PlayerFilter::ControllerOf(
            crate::target::ObjectRef::Tagged(tagged),
        )),
    );

    assert_eq!(
        describe_effect(&effect),
        "Deal damage equal to that creature's power to that object's controller"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_choose_two_one_other_counter_sequence() {
    let tag = TagKey::from("targeted_0");
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
        .with_count(ChoiceCount::exactly(2));
    let target_effect = Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag(tag.clone());
    let mut exiled_filter = ObjectFilter::creature();
    exiled_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let exile_one = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Object(exiled_filter).with_count(ChoiceCount::exactly(1)),
        Zone::Exile,
        true,
    ));
    let counters = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        2,
        ChooseSpec::AnyOtherTarget,
    ));
    let effects = vec![target_effect, exile_one, counters];
    let expected = "Choose two target creatures. Exile one of those creatures and put two +1/+1 counters on the other";

    assert_eq!(describe_effect_list(&effects), expected);
}

#[test]
pub(super) fn describe_effect_list_compacts_draw_reveal_discard_nonland_sequence() {
    let revealed = TagKey::from("__sentence_helper_revealed_l0_s0_e0");
    let mut revealed_hand_card = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::You);
    revealed_hand_card
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: revealed.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let effects = vec![
        Effect::draw(Value::Fixed(1)),
        Effect::new(crate::effects::RevealTaggedEffect::new(revealed.clone())),
        Effect::new(crate::effects::ConditionalEffect::new(
            Condition::Not(Box::new(Condition::TaggedObjectMatches(
                revealed.clone(),
                ObjectFilter {
                    card_types: vec![CardType::Land],
                    ..Default::default()
                },
            ))),
            vec![Effect::new(crate::effects::DiscardEffect::new_with_filter(
                Value::Fixed(1),
                PlayerFilter::You,
                false,
                Some(revealed_hand_card),
            ))],
            Vec::new(),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Draw a card and reveal it. If it isn't a land card, discard it"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_exile_all_creatures_each_player_fractal_power_counters()
{
    let exiled = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let created = TagKey::from("created_1");
    let fractal = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Fractal")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Fractal])
        .color_indicator(crate::color::ColorSet::GREEN.union(crate::color::ColorSet::BLUE))
        .power_toughness(crate::card::PowerToughness::fixed(0, 0))
        .build();

    let effects = vec![
        Effect::with_id(
            0,
            Effect::new(crate::effects::ExileEffect::all(
                ObjectFilter::creature().in_zone(Zone::Battlefield),
            ))
            .tag(exiled),
        ),
        Effect::for_players(
            PlayerFilter::Any,
            vec![
                Effect::create_tokens_player(
                    fractal,
                    Value::Fixed(1),
                    PlayerFilter::IteratedPlayer,
                )
                .tag(created.clone()),
            ],
        ),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            Value::EffectMetric {
                effect_id: crate::effect::EffectId(0),
                source: crate::effect::EffectMetricSource::AffectedObjects,
                metric: crate::effect::EffectMetric::TotalPower,
            },
            ChooseSpec::Tagged(created),
        ))
        .tag(TagKey::from("counters_2")),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Exile all creatures. Each player creates a 0/0 green and blue Fractal creature token and puts a number of +1/+1 counters on it equal to the total power of creatures they controlled that were exiled this way"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_choose_two_sacrifice_one_return_other() {
    let tag = TagKey::from("targeted_0");
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
        .with_count(ChoiceCount::exactly(2));
    let target_effect = Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag(tag.clone());
    let sacrifice = Effect::new(crate::effects::SacrificeTargetEffect::new(
        ChooseSpec::Tagged(tag),
    ));
    let return_to_hand = Effect::new(crate::effects::ReturnToHandEffect::with_spec(
        ChooseSpec::AnyOtherTarget,
    ));
    let effects = vec![target_effect, sacrifice, return_to_hand];
    let expected = "Choose two target creatures. Their controller sacrifices one of them. Return the other to its owner's hand";

    assert_eq!(describe_effect_list(&effects), expected);
}

#[test]
pub(super) fn describe_effect_list_compacts_same_controller_choice_sacrifice_return_other() {
    let target_tag = TagKey::from("targeted_0");
    let chosen_tag = TagKey::from("chosen_0");
    let mut target_filter = ObjectFilter::creature();
    target_filter.target_set_same_controller = true;
    let target =
        ChooseSpec::target(ChooseSpec::Object(target_filter)).with_count(ChoiceCount::exactly(2));
    let target_effect =
        Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag(target_tag.clone());
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::tagged(target_tag.clone()),
            ChoiceCount::exactly(1),
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(target_tag.clone())),
            chosen_tag.clone(),
        )
        .in_zone(Zone::Battlefield),
    );
    let sacrifice = Effect::new(crate::effects::SacrificeTargetEffect::new(
        ChooseSpec::Tagged(chosen_tag.clone()),
    ));
    let mut other_filter = ObjectFilter::tagged(target_tag);
    other_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_tag,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
    let return_to_hand = Effect::new(crate::effects::ReturnToHandEffect::with_spec(
        ChooseSpec::Object(other_filter),
    ))
    .tag(TagKey::from("returned_0"));
    let effects = vec![target_effect, choose, sacrifice, return_to_hand];
    let expected = "Choose two target creatures controlled by the same player. Their controller chooses and sacrifices one of them. Return the other to its owner's hand";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_choose_exiled_cards_exile_library_put_chosen_on_top() {
    let chosen_tag = TagKey::from("chosen_0");
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::default()
                .in_zone(Zone::Exile)
                .owned_by(PlayerFilter::You)
                .face_up(),
            ChoiceCount::up_to(7),
            PlayerFilter::You,
            chosen_tag.clone(),
        )
        .in_zone(Zone::Exile),
    );
    let exile_library = Effect::new(crate::effects::ExileEffect::all(
        ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::You),
    ));
    let put_chosen_on_top = Effect::new(crate::effects::MoveToZoneEffect::to_top_of_library(
        ChooseSpec::Tagged(chosen_tag),
    ));
    let effects = vec![choose, exile_library, put_chosen_on_top];
    let expected = "Choose up to seven face-up exiled cards you own. Exile all the cards from your library, then put the chosen cards on top of your library";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );

    let comma_then = Effect::new(crate::effects::SequenceEffect::comma_then(
        effects[1..].to_vec(),
    ));
    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![effects[0].clone()]),
        crate::resolution::ResolutionSegment::from_effects(vec![comma_then]),
    ]);
    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        expected
    );

    let coordinated = Effect::new(crate::effects::SequenceEffect::coordinated(
        effects[1..].to_vec(),
    ));
    let wrong_surface = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![effects[0].clone()]),
        crate::resolution::ResolutionSegment::from_effects(vec![coordinated]),
    ]);
    assert_ne!(
        super::super::ast_render::describe_resolution_program(&wrong_surface),
        expected,
        "an unrelated conjunction must not be rewritten as an authored comma-then disposition"
    );

    let wrong_tag_move = Effect::new(crate::effects::MoveToZoneEffect::to_top_of_library(
        ChooseSpec::Tagged("different_choice".into()),
    ));
    let wrong_tag_sequence = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
        effects[1].clone(),
        wrong_tag_move,
    ]));
    let wrong_tag = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![effects[0].clone()]),
        crate::resolution::ResolutionSegment::from_effects(vec![wrong_tag_sequence]),
    ]);
    assert_ne!(
        super::super::ast_render::describe_resolution_program(&wrong_tag),
        expected,
        "the consumer must refer to the exact chosen-set tag"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_sacrifice_power_damage_sequence() {
    let tag = TagKey::from("sacrificed_0");
    let choose = Effect::new(crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature().you_control(),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    ));

    let mut sacrificed = ObjectFilter::permanent();
    sacrificed.tagged_constraints.push(TaggedObjectConstraint {
        tag: tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let sacrifice = Effect::sacrifice_player(sacrificed, 1, PlayerFilter::You);

    let amount = Value::PowerOf(Box::new(ChooseSpec::Tagged(tag.clone())));
    let mut creatures_without_flying = ObjectFilter::default();
    creatures_without_flying.card_types.push(CardType::Creature);
    creatures_without_flying
        .excluded_static_abilities
        .push(crate::static_abilities::StaticAbilityId::Flying);
    let damage_creatures = Effect::for_each(
        creatures_without_flying,
        vec![
            Effect::deal_damage(amount.clone(), ChooseSpec::Iterated)
                .tag(TagKey::from("damaged_1")),
        ],
    );
    let damage_players = Effect::for_players(
        PlayerFilter::Any,
        vec![Effect::deal_damage(
            amount,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer),
        )],
    );
    let effects = vec![choose, sacrifice, damage_creatures, damage_players];
    let expected = "Sacrifice a creature. This deals damage equal to that creature's power to each creature without flying and each player";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

fn iterated_damage_pair(
    creature_filter: ObjectFilter,
    second_object_filter: Option<ObjectFilter>,
) -> (Effect, Effect) {
    let amount = Value::X;
    let creature_damage = Effect::for_each(
        creature_filter,
        vec![
            Effect::deal_damage(amount.clone(), ChooseSpec::Iterated)
                .tag(TagKey::from("damaged_0")),
        ],
    );
    let second_damage = if let Some(second_filter) = second_object_filter {
        Effect::for_each(
            second_filter,
            vec![Effect::deal_damage(amount, ChooseSpec::Iterated).tag(TagKey::from("damaged_1"))],
        )
    } else {
        Effect::for_players(
            PlayerFilter::Any,
            vec![Effect::deal_damage(
                amount,
                ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            )],
        )
    };
    (creature_damage, second_damage)
}

#[test]
pub(super) fn joint_damage_preserves_without_flying_for_creatures_and_players() {
    let without_flying = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .without_static_ability(crate::static_abilities::StaticAbilityId::Flying);
    let (creatures, players) = iterated_damage_pair(without_flying, None);
    assert_eq!(
        describe_joint_subject_pair(&creatures, &players).as_deref(),
        Some("this deals X damage to each creature without flying and each player")
    );

    let with_flying = ObjectFilter::creature()
        .with_static_ability(crate::static_abilities::StaticAbilityId::Flying);
    let (creatures, players) = iterated_damage_pair(with_flying, None);
    assert_eq!(
        describe_joint_subject_pair(&creatures, &players).as_deref(),
        Some("this deals X damage to each creature with flying and each player")
    );

    let (creatures, players) = iterated_damage_pair(ObjectFilter::creature(), None);
    assert_eq!(
        describe_joint_subject_pair(&creatures, &players).as_deref(),
        Some("this deals X damage to each creature and each player")
    );
}

#[test]
pub(super) fn joint_damage_preserves_without_flying_for_creatures_and_planeswalkers() {
    let without_flying = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .without_static_ability(crate::static_abilities::StaticAbilityId::Flying);
    let (creatures, planeswalkers) =
        iterated_damage_pair(without_flying, Some(ObjectFilter::planeswalker()));
    assert_eq!(
        describe_joint_subject_pair(&creatures, &planeswalkers).as_deref(),
        Some("this deals X damage to each creature without flying and each planeswalker")
    );
}

#[test]
pub(super) fn coordinated_joint_damage_keeps_excluded_planeswalker_subtype() {
    let (creatures, planeswalkers) = iterated_damage_pair(
        ObjectFilter::creature(),
        Some(ObjectFilter::planeswalker().without_subtype(Subtype::Bolas)),
    );
    let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
        creatures,
        planeswalkers,
    ]));
    let rendered = describe_effect(&sequence);
    assert!(
        rendered.contains("each creature and each non-Bolas planeswalker"),
        "{rendered}"
    );
    assert!(!rendered.contains(". This deals"), "{rendered}");
}

#[test]
pub(super) fn describe_effect_list_compacts_destroy_search_graveyard_shuffle_sequence() {
    let destroyed = Effect::destroy_all(ObjectFilter::creature());
    let tag = TagKey::from("searched_0");
    let mut search_filter = ObjectFilter::creature();
    search_filter.zone = Some(Zone::Library);
    search_filter.owner = Some(PlayerFilter::target_opponent());
    let search = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            search_filter,
            ChoiceCount {
                min: 0,
                max: Some(3),
                dynamic_x: false,
                up_to_x: false,
                random: false,
                explicit_exactly: false,
            },
            PlayerFilter::target_opponent(),
            tag.clone(),
        )
        .in_zone(Zone::Library)
        .as_optional_search(),
    );
    let move_each = Effect::new(crate::effects::ForEachTaggedEffect::new(
        tag,
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Graveyard,
            false,
        ))],
    ));
    let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![search, move_each]));
    let shuffle = Effect::shuffle_library_player(PlayerFilter::target_opponent());
    let effects = vec![destroyed, sequence, shuffle];
    let expected = "Destroy all creatures, then search target opponent's library for up to three creature cards and put them into their graveyard. Then that player shuffles";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_tagged_mill_payment_then_milled_card_to_hand() {
    let milled_tag = TagKey::from("milled_0");
    let chosen_tag = TagKey::from("chosen_0");
    let mill = Effect::mill(Value::Fixed(3)).tag(milled_tag.clone());
    let payment = Effect::with_id(
        0,
        Effect::may(vec![
            Effect::new(crate::effects::PayManaEffect::new(
                crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]),
                ChooseSpec::Player(PlayerFilter::You),
            )),
            Effect::lose_life(Value::Fixed(3)),
        ]),
    );
    let mut choice_filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: milled_tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            choice_filter,
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            chosen_tag.clone(),
        )
        .in_zone(Zone::Graveyard),
    );
    let move_to_hand = Effect::new(crate::effects::ForEachTaggedEffect::new(
        chosen_tag,
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Hand,
            false,
        ))],
    ));
    let if_paid = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![choose, move_to_hand],
    );
    let effects = vec![mill, payment, if_paid];
    let expected = "Mill three cards. Then you may pay {1} and 3 life. If you do, put a card from among those cards into your hand";

    assert_eq!(describe_effect_list(&effects), expected);
}

#[test]
pub(super) fn nested_mill_collection_tags_compact_at_the_production_list_boundary() {
    let outer_milled_tag = TagKey::from("__sentence_helper_milled");
    let inner_milled_tag = TagKey::from("milled_0");
    let chosen_tag = TagKey::from("__sentence_helper_chosen_milled");
    let mill = Effect::mill(Value::Fixed(3))
        .tag(inner_milled_tag)
        .tag(outer_milled_tag.clone());

    let mut choice_filter = ObjectFilter::default().historic().in_zone(Zone::Graveyard);
    choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: outer_milled_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            choice_filter,
            ChoiceCount::up_to(1),
            PlayerFilter::You,
            chosen_tag.clone(),
        )
        .in_zone(Zone::Graveyard),
    );
    let move_to_hand = Effect::new(crate::effects::ForEachTaggedEffect::new(
        chosen_tag,
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Hand,
            false,
        ))],
    ));
    let effects = vec![mill, choose, move_to_hand];

    let (collection_tag, typed_mill) = mill_with_collection_tag(&effects[0]).unwrap();
    assert_eq!(collection_tag, &outer_milled_tag);
    assert_eq!(typed_mill.count, Value::Fixed(3));

    let expected = "Mill three cards. You may put a historic card from among them into your hand";
    assert_eq!(
        describe_pre_clause_structural_effect_list(&effects).as_deref(),
        Some(expected)
    );
    assert_eq!(describe_effect_list(&effects), expected);

    let inline = Effect::new(crate::effects::SequenceEffect::coordinated(effects));
    assert_eq!(
        describe_effect(&inline),
        "Mill three cards, then you may put a historic card from among them into your hand"
    );
}

#[test]
pub(super) fn nested_mill_collection_prefix_compacts_before_independent_rider() {
    let milled_tag = TagKey::from("__sentence_helper_milled");
    let chosen_tag = TagKey::from("__sentence_helper_chosen_milled");
    let mill = Effect::mill(Value::Fixed(2))
        .tag(TagKey::from("milled_0"))
        .tag(milled_tag.clone());

    let mut choice_filter = ObjectFilter::permanent_card().in_zone(Zone::Graveyard);
    choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: milled_tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            choice_filter,
            ChoiceCount::up_to(1),
            PlayerFilter::You,
            chosen_tag.clone(),
        )
        .in_zone(Zone::Graveyard),
    );
    let move_to_hand = Effect::for_each_tagged(
        chosen_tag,
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Hand,
            false,
        ))],
    );
    let effects = vec![
        mill,
        choose,
        move_to_hand,
        Effect::gain_life(Value::Fixed(2)),
    ];

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "Mill two cards. You may put a permanent card from among them into your hand. Gain 2 life"
        )
    );
}

#[test]
pub(super) fn describe_with_id_if_happened_compacts_mana_and_life_payment() {
    let payment = Effect::with_id(
        0,
        Effect::may(vec![
            Effect::new(crate::effects::PayManaEffect::new(
                crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]),
                ChooseSpec::Player(PlayerFilter::You),
            )),
            Effect::lose_life(Value::Fixed(1)),
        ]),
    );
    let if_paid = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::draw(Value::Fixed(1))],
    );

    assert_eq!(
        describe_effect_list(&[payment, if_paid]),
        "You may pay {1} and 1 life. If you do, you draw a card"
    );
}

#[test]
pub(super) fn describe_with_id_unless_damage_paid_branch_uses_imperative_damage() {
    let first_tag = TagKey::from("damaged_0");
    let second_tag = TagKey::from("damaged_1");
    let setup = Effect::with_id(
        0,
        Effect::unless_pays(
            vec![
                Effect::deal_damage(Value::Fixed(4), ChooseSpec::AnyTarget).tag(first_tag.clone()),
            ],
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(first_tag)),
            vec![ManaSymbol::Generic(2)],
        ),
    );
    let followup = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::DidNotHappen,
        vec![Effect::deal_damage(Value::Fixed(2), ChooseSpec::AnyTarget).tag(second_tag)],
    );

    assert_eq!(
        describe_effect_list(&[setup, followup]),
        "Deal 4 damage to any target unless its controller pays {2}. If they do, deal 2 damage to any target"
    );
}

#[test]
pub(super) fn describe_with_id_reflexive_compacts_mana_and_life_payment() {
    let payment = Effect::with_id(
        0,
        Effect::may(vec![
            Effect::new(crate::effects::PayManaEffect::new(
                crate::mana::ManaCost::from_symbols(vec![ManaSymbol::White, ManaSymbol::Black]),
                ChooseSpec::Player(PlayerFilter::You),
            )),
            Effect::lose_life(Value::Fixed(2)),
        ]),
    );
    let reflexive = Effect::reflexive_trigger(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::draw(Value::Fixed(1))],
        vec![],
    );

    assert_eq!(
        describe_effect_list(&[payment, reflexive]),
        "You may pay {W}{B} and 2 life. When you do, you draw a card"
    );
}

#[test]
pub(super) fn standalone_reflexive_fallback_never_leaks_internal_effect_ids() {
    let mut creature_card = ObjectFilter::creature();
    creature_card.zone = None;
    creature_card.set_explicit_card_noun(true);
    let reveal_predicate =
        EffectPredicate::PriorEffectResult(crate::effect::PriorEffectResultSurface::new(
            crate::effect::PriorEffectAction::Revealed,
            creature_card,
            crate::effect::PriorEffectResultActor::You,
            crate::effect::PriorEffectResultQuantifier::One,
        ));
    let revealed = Effect::reflexive_trigger(
        crate::effect::EffectId(37),
        reveal_predicate,
        vec![Effect::draw(Value::Fixed(1))],
        vec![],
    );
    let happened = Effect::reflexive_trigger(
        crate::effect::EffectId(41),
        EffectPredicate::Happened,
        vec![Effect::draw(Value::Fixed(1))],
        vec![],
    );

    assert_eq!(
        describe_effect(&revealed),
        "When you reveal a creature card this way, you draw a card"
    );
    assert_eq!(describe_effect(&happened), "When you do, you draw a card");
    assert!(!describe_effect(&revealed).contains("effect #"));
    assert!(!describe_effect(&happened).contains("effect #"));
}

#[test]
pub(super) fn damage_to_player_result_condition_keeps_typed_surface() {
    let damage = Effect::with_id(
        0,
        Effect::deal_damage(Value::Fixed(2), ChooseSpec::AnyTarget),
    );
    let discard = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::DealtDamageToPlayer,
        vec![Effect::new(crate::effects::DiscardEffect::new(
            1,
            PlayerFilter::DamagedPlayer,
            false,
        ))],
    );

    assert_eq!(
        describe_effect_list(&[damage, discard]),
        "Deal 2 damage to any target. If a player is dealt damage this way, that player discards a card"
    );
}

#[test]
pub(super) fn damaged_player_result_condition_keeps_player_actor() {
    let discard = Effect::with_id(
        0,
        Effect::new(crate::effects::DiscardEffect::new(
            1,
            PlayerFilter::DamagedPlayer,
            true,
        )),
    );
    let draw = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::new(crate::effects::DrawCardsEffect::new(
            1,
            PlayerFilter::DamagedPlayer,
        ))],
    );

    assert_eq!(
        describe_effect_list(&[discard, draw]),
        "that player discards a card at random. If they do, that player draws a card"
    );
}

#[test]
pub(super) fn describe_with_id_reflexive_names_counted_may_sacrifice() {
    let tag = TagKey::from("sacrificed_0");
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::default()
                .with_subtype(Subtype::Zombie)
                .controlled_by(PlayerFilter::You)
                .in_zone(Zone::Battlefield),
            ChoiceCount::up_to(3),
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Battlefield),
    );
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(tag.clone()),
        Value::Count(ObjectFilter::tagged(tag)),
        PlayerFilter::You,
    ));
    let payment = Effect::with_id(0, Effect::may(vec![choose, sacrifice]));
    let reflexive = Effect::reflexive_trigger(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::draw(Value::Fixed(1))],
        vec![],
    );

    assert_eq!(
        describe_effect_list(&[payment, reflexive]),
        "You may sacrifice up to three Zombies. When you sacrifice one or more Zombies this way, you draw a card"
    );
}

#[test]
pub(super) fn describe_choose_sacrifice_reflexive_compacts_when_you_do() {
    let tag = TagKey::from("sacrificed_0");
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::creature()
                .controlled_by(PlayerFilter::You)
                .in_zone(Zone::Battlefield),
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Battlefield),
    );
    let sacrifice = Effect::with_id(
        0,
        Effect::sacrifice_player(ObjectFilter::tagged(tag), 1, PlayerFilter::You),
    );
    let reflexive = Effect::reflexive_trigger(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::draw(Value::Fixed(1))],
        vec![],
    );

    assert_eq!(
        describe_effect_list(&[choose, sacrifice, reflexive]),
        "Sacrifice a creature. When you do, you draw a card"
    );
}

#[test]
pub(super) fn describe_counted_sacrifice_reflexive_names_sacrificed_objects() {
    let tag = TagKey::from("sacrificed_0");
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::default()
                .with_subtype(Subtype::Zombie)
                .controlled_by(PlayerFilter::You)
                .in_zone(Zone::Battlefield),
            ChoiceCount::up_to(3),
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Battlefield),
    );
    let sacrifice = Effect::with_id(
        0,
        Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
            ObjectFilter::tagged(tag.clone()),
            Value::Count(ObjectFilter::tagged(tag)),
            PlayerFilter::You,
        )),
    );
    let reflexive = Effect::reflexive_trigger(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::draw(Value::Fixed(1))],
        vec![],
    );

    assert_eq!(
        describe_effect_list(&[choose, sacrifice, reflexive]),
        "Sacrifice up to three Zombies. When you sacrifice one or more Zombies this way, you draw a card"
    );
}

#[test]
pub(super) fn describe_if_happened_choose_one_followup_as_reflexive_modal_choice() {
    let followup = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::choose_one(vec![
            ironsmith_core::EffectMode::new("Draw a card", vec![Effect::draw(Value::Fixed(1))]),
            ironsmith_core::EffectMode::new("Gain 2 life", vec![Effect::gain_life(2)]),
        ])],
    );

    assert_eq!(
        describe_effect(&followup),
        "When you do, choose one —\n• Draw a card.\n• Gain 2 life."
    );
}

#[test]
pub(super) fn describe_with_id_exile_choose_one_followup_as_reflexive_modal_choice() {
    let exiled_tag = TagKey::from("exiled_0");
    let setup = Effect::with_id(
        0,
        Effect::exile(ChooseSpec::Object(
            ObjectFilter::default().in_zone(Zone::Graveyard),
        ))
        .tag(exiled_tag),
    );
    let followup = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::choose_one(vec![
            ironsmith_core::EffectMode::new("Draw a card", vec![Effect::draw(Value::Fixed(1))]),
            ironsmith_core::EffectMode::new("Gain 2 life", vec![Effect::gain_life(2)]),
        ])],
    );

    assert_eq!(
        describe_effect_list(&[setup, followup]),
        "Exile target card from a graveyard. When you do, choose one —\n• Draw a card.\n• Gain 2 life."
    );
}

#[test]
pub(super) fn tagged_exile_for_each_surface_keeps_producer_card_noun() {
    let exiled_tag = TagKey::from("exiled_0");
    let setup = Effect::exile(ChooseSpec::Object(
        ObjectFilter::creature().in_zone(Zone::Graveyard),
    ))
    .tag(exiled_tag.clone());
    let followup = Effect::for_each_tagged(exiled_tag, vec![Effect::lose_life(1)]);
    let rendered = describe_effect_list(&[setup, followup]);
    assert!(
        rendered.contains("For each creature card exiled this way"),
        "{rendered}"
    );
}

#[test]
pub(super) fn describe_with_id_move_to_exile_choose_one_followup_as_reflexive_modal_choice() {
    let setup = Effect::with_id(
        0,
        Effect::move_to_zone(
            ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Graveyard)),
            Zone::Exile,
            true,
        ),
    );
    let followup = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::choose_one(vec![
            ironsmith_core::EffectMode::new("Draw a card", vec![Effect::draw(Value::Fixed(1))]),
            ironsmith_core::EffectMode::new("Gain 2 life", vec![Effect::gain_life(2)]),
        ])],
    );

    assert_eq!(
        describe_effect_list(&[setup, followup]),
        "Exile a card from a graveyard. When you do, choose one —\n• Draw a card.\n• Gain 2 life."
    );
}

#[test]
pub(super) fn describe_grant_flashback_from_card_mana_cost_keeps_cost_sentence() {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::instant_or_sorcery()
            .owned_by(PlayerFilter::You)
            .in_zone(Zone::Graveyard),
    ));
    let grant = Effect::grant(
        crate::grant::Grantable::flashback_from_cards_mana_cost(),
        target,
        crate::grant::GrantDuration::UntilEndOfTurn,
    );

    assert_eq!(
        describe_effect(&grant),
        "target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost"
    );
}

#[test]
pub(super) fn may_cast_matching_spell_renders_other_result_mana_value_limit() {
    let mut filter = ObjectFilter::instant_or_sorcery().owned_by(PlayerFilter::You);
    filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqualExpr(Box::new(
        Value::EffectMetric {
            effect_id: crate::effect::EffectId(0),
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::OtherNumber,
        },
    )));
    let effect = Effect::new(
        crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect::new(
            PlayerFilter::You,
            filter,
            Zone::Hand,
        ),
    );

    assert_eq!(
        describe_effect(&effect),
        "you may cast an instant or sorcery spell with mana value less than or equal to the other result from your hand without paying its mana cost"
    );
}

#[test]
pub(super) fn may_cast_matching_spell_renders_generic_mana_value_filter_once() {
    let mut filter = ObjectFilter::nonland().owned_by(PlayerFilter::You);
    filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(4));
    let effect = Effect::new(
        crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect::new(
            PlayerFilter::You,
            filter,
            Zone::Hand,
        ),
    );

    assert_eq!(
        describe_effect(&effect),
        "you may cast a spell with mana value 4 or less from your hand without paying its mana cost"
    );
}

#[test]
pub(super) fn may_cast_matching_spell_renders_subtype_before_spell_noun() {
    let mut filter = ObjectFilter::nonland().owned_by(PlayerFilter::You);
    filter.subtypes.push(Subtype::Hero);
    let effect = Effect::new(
        crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect::new(
            PlayerFilter::You,
            filter,
            Zone::Hand,
        ),
    );

    assert_eq!(
        describe_effect(&effect),
        "you may cast a Hero spell from your hand without paying its mana cost"
    );
}

#[test]
pub(super) fn roll_choose_draw_then_may_cast_renders_arcane_endeavor_surface() {
    let roll = Effect::with_id(
        0,
        Effect::roll_dice_choose_result_with_die_text(
            2,
            8,
            PlayerFilter::You,
            Some("d8".to_string()),
        ),
    );
    let draw = Effect::draw(Value::EffectValue(crate::effect::EffectId(0)));
    let mut filter = ObjectFilter::instant_or_sorcery().owned_by(PlayerFilter::You);
    filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqualExpr(Box::new(
        Value::EffectMetric {
            effect_id: crate::effect::EffectId(0),
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::OtherNumber,
        },
    )));
    let may_cast = Effect::new(crate::effects::MayEffect::new(vec![Effect::new(
        crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect::new(
            PlayerFilter::You,
            filter,
            Zone::Hand,
        ),
    )]));

    assert_eq!(
        describe_effect_list(&[roll, draw, may_cast]),
        "Roll two d8 and choose one result. Draw cards equal to that result. Then you may cast an instant or sorcery spell with mana value less than or equal to the other result from your hand without paying its mana cost."
    );
}

#[test]
pub(super) fn sacrificed_reflexive_value_reference_names_sacrificed_object() {
    assert_eq!(
        rewrite_sacrificed_reflexive_value_references(
            "creatures get -X/-X until end of turn, where X is its toughness"
        ),
        "creatures get -X/-X until end of turn, where X is the sacrificed creature's toughness"
    );
}

#[test]
pub(super) fn describe_may_have_target_creature_block_source_compacts_choice() {
    let tag = TagKey::from("targeted_0");
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
        ChooseSpec::Object(ObjectFilter::creature()),
    )))
    .tag(tag.clone());
    let must_block = Effect::new(crate::effects::CantEffect::new(
        crate::effect::Restriction::MustBlockSpecificAttacker {
            blockers: ObjectFilter::tagged(tag),
            attacker: ObjectFilter::source(),
        },
        Until::EndOfTurn,
    ));
    let may = Effect::may(vec![target, must_block]);

    assert_eq!(
        describe_effect(&may),
        "You may have target creature block this creature this turn if able"
    );
}

#[test]
pub(super) fn look_reorder_then_optional_target_player_shuffle_uses_that_player() {
    let tag = TagKey::from("looked");
    let effects = vec![
        Effect::new(crate::effects::LookAtTopCardsEffect::new(
            PlayerFilter::target_player(),
            Value::Fixed(3),
            tag.clone(),
        )),
        Effect::new(crate::effects::ReorderLibraryTopEffect::new(tag)),
        Effect::new(crate::effects::MayEffect::new_for_player(
            vec![
                Effect::new(crate::effects::TargetOnlyEffect::new(
                    ChooseSpec::target_player(),
                )),
                Effect::new(crate::effects::ShuffleLibraryEffect::new(
                    PlayerFilter::target_player(),
                )),
            ],
            PlayerFilter::You,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Look at the top three cards of target player's library, then put them back in any order. You may have that player shuffle"
    );
}

#[test]
pub(super) fn each_player_may_draw_then_each_who_drew_gains_life_compacts() {
    let effect = Effect::for_players(
        PlayerFilter::Any,
        vec![
            Effect::with_id(
                0,
                Effect::may(vec![Effect::target_draws(
                    Value::Fixed(1),
                    PlayerFilter::IteratedPlayer,
                )]),
            ),
            Effect::if_then(
                crate::effect::EffectId(0),
                EffectPredicate::Happened,
                vec![Effect::new(crate::effects::GainLifeEffect::with_filter(
                    Value::Fixed(1),
                    PlayerFilter::IteratedPlayer,
                ))],
            ),
        ],
    );

    assert_eq!(
        describe_effect(&effect),
        "Each player may draw a card, then each player who drew a card this way gains 1 life"
    );
}

#[test]
pub(super) fn chosen_color_protection_radiance_fanout_compacts() {
    let protection = || {
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::protection(
                crate::ability::ProtectionFrom::ChosenColor,
            ),
        )
    };
    let target_tag = TagKey::from("targeted_0");
    let target_filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
    let mut target_grant = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        protection(),
        Until::EndOfTurn,
    );
    target_grant.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(target_filter)));

    let mut shared_filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
    shared_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: target_tag.clone(),
            relation: TaggedOpbjectRelation::SharesColorWithTagged,
        });
    shared_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: target_tag,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
    let shared_grant = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Filter(shared_filter),
        protection(),
        Until::EndOfTurn,
    );
    let effects = vec![
        Effect::choose_color(PlayerFilter::You),
        Effect::new(target_grant).tag(TagKey::from("granted_0")),
        Effect::new(shared_grant),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Radiance — Choose a color. Target creature and each other creature that shares a color with it gain protection from the chosen color until end of turn"
    );
}

#[test]
pub(super) fn equal_to_draw_count_renders_as_cards_equal_to_dynamic_value() {
    let amount = Value::GreatestManaValue(ObjectFilter::creature().you_control())
        .with_surface_hint(ValueSurfaceHint::EqualTo);
    let effect = Effect::new(crate::effects::DrawCardsEffect::new(
        amount,
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect(&effect),
        "you draw cards equal to the greatest mana value among creatures you control"
    );
}

#[test]
pub(super) fn equal_to_power_draw_count_uses_that_creature_reference() {
    let amount = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from("sacrificed_0"))))
        .with_surface_hint(ValueSurfaceHint::EqualTo);
    let effect = Effect::new(crate::effects::DrawCardsEffect::new(
        amount,
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect(&effect),
        "you draw cards equal to that creature's power"
    );
}

#[test]
pub(super) fn equal_to_power_mill_count_uses_direct_card_action_surface() {
    let amount = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from("__it__"))))
        .with_surface_hint(ValueSurfaceHint::EqualTo);
    let effect = Effect::new(crate::effects::MillEffect::new(
        amount,
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(TagKey::from(
            "destroyed_0",
        ))),
    ));

    assert_eq!(
        describe_effect(&effect),
        "its controller mills cards equal to that creature's power"
    );
}

#[test]
pub(super) fn equal_to_count_draw_does_not_render_as_for_each() {
    let count = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You),
    );
    let equal_to = Effect::new(crate::effects::DrawCardsEffect::new(
        count.clone().with_surface_hint(ValueSurfaceHint::EqualTo),
        PlayerFilter::You,
    ));
    let for_each = Effect::new(crate::effects::DrawCardsEffect::new(
        count.with_surface_hint(ValueSurfaceHint::ForEach),
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect(&equal_to),
        "you draw cards equal to the number of cards in your hand"
    );
    assert_eq!(
        describe_effect(&for_each),
        "you draw a card for each card in your hand"
    );
}

#[test]
pub(super) fn repeat_effects_uses_typed_for_each_count_surfaces() {
    let hand_cards = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You),
    )
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let draw = Effect::new(crate::effects::DrawCardsEffect::you(Value::Fixed(1)));

    assert_eq!(
        describe_effect(&Effect::repeat_effects(hand_cards, vec![draw.clone()])),
        "you draw a card for each card in your hand"
    );

    let mut revealed_cards = ObjectFilter::tagged(TagKey::from("__it__"));
    revealed_cards.set_explicit_card_noun(true);
    let blue_symbols = Value::ManaSymbolsInManaCostOf {
        spec: Box::new(ChooseSpec::All(revealed_cards)),
        color: crate::color::Color::Blue,
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);

    assert_eq!(
        describe_effect(&Effect::repeat_effects(blue_symbols, vec![draw])),
        "you draw a card for each blue mana symbol in the mana costs of those cards"
    );
}

#[test]
pub(super) fn fixed_draw_then_equal_to_named_graveyard_count_keeps_sequence_and_scope() {
    let your_graveyard_count = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .named("Take Inventory"),
    )
    .with_surface_hint(ValueSurfaceHint::EqualTo);
    let all_graveyards_count = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .named("Accumulated Knowledge"),
    )
    .with_surface_hint(ValueSurfaceHint::EqualTo);

    let render = |count| {
        describe_effect_list(&[
            Effect::new(crate::effects::DrawCardsEffect::you(Value::Fixed(1))),
            Effect::new(crate::effects::DrawCardsEffect::you(count)),
        ])
    };

    assert_eq!(
        render(your_graveyard_count),
        "you draw a card, then draw cards equal to the number of cards named Take Inventory in your graveyard"
    );
    assert_eq!(
        render(all_graveyards_count),
        "you draw a card, then draw cards equal to the number of cards named Accumulated Knowledge in all graveyards"
    );
}

#[test]
pub(super) fn equal_to_life_count_renders_as_life_equal_to_dynamic_value() {
    let amount = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You),
    )
    .with_surface_hint(ValueSurfaceHint::EqualTo);

    assert_eq!(
        describe_effect(&Effect::new(crate::effects::LoseLifeEffect::new(
            amount.clone(),
            ChooseSpec::Player(PlayerFilter::You),
        ))),
        "you lose life equal to the number of cards in your hand"
    );
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::GainLifeEffect::you(amount))),
        "you gain life equal to the number of cards in your hand"
    );
}

#[test]
pub(super) fn sentence_helper_exiled_grant_play_renders_singular_card_reference() {
    let effect = Effect::new(crate::effects::GrantPlayTaggedEffect::new(
        TagKey::from("__sentence_helper_exiled_l0_s0_e0"),
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        false,
        false,
    ));

    assert_eq!(describe_effect(&effect), "you may cast that card this turn");
}

#[test]
pub(super) fn target_graveyard_card_then_cast_grant_renders_single_oracle_clause() {
    let tag = TagKey::from("targeted_0");
    let mut filter = ObjectFilter::creature()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    filter.subtypes.push(Subtype::Zombie);
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
        ChooseSpec::Object(filter),
    )))
    .tag(tag.clone());
    let grant = Effect::new(crate::effects::GrantPlayTaggedEffect::new(
        tag,
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        false,
        false,
    ));

    assert_eq!(
        describe_effect_list(&[target, grant]),
        "You may cast target Zombie creature card from your graveyard this turn"
    );
}

#[test]
pub(super) fn sentence_helper_consult_match_grants_render_as_a_singular_card() {
    let tag = TagKey::from("__sentence_helper_consult_match_l0_s0_e0");
    let grant_play = Effect::new(crate::effects::GrantPlayTaggedEffect::new(
        tag.clone(),
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        false,
        false,
    ));
    let grant_free = Effect::new(
        crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(tag, PlayerFilter::You),
    );

    assert_eq!(
        describe_effect(&grant_play),
        "you may cast that card this turn"
    );
    assert_eq!(
        describe_effect(&grant_free),
        "you may cast that card from exile this turn without paying its mana cost"
    );
}

#[test]
pub(super) fn chosen_exiled_countered_card_free_play_renders_as_one_permission() {
    let tag = TagKey::from("__chosen_objects__");
    let mut filter = ObjectFilter::default()
        .in_zone(Zone::Exile)
        .owned_by(PlayerFilter::Opponent)
        .with_counter_type(CounterType::Void);
    filter.set_explicit_card_noun(true);
    let choose = Effect::new(crate::effects::ChooseObjectsEffect::new(
        filter,
        1,
        PlayerFilter::You,
        tag.clone(),
    ));
    let grant_play = Effect::new(crate::effects::GrantPlayTaggedEffect::new(
        tag.clone(),
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        true,
        false,
    ));
    let grant_free = Effect::new(
        crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(tag, PlayerFilter::You),
    );
    let effects = [choose, grant_play, grant_free];

    assert_eq!(
        describe_effect_list(&effects),
        "Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost"
        )
    );

    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![effects[0].clone()]),
        crate::resolution::ResolutionSegment::from_effects(effects[1..].to_vec()),
    ]);
    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost"
    );
}

#[test]
pub(super) fn typed_temporary_permission_surface_joins_cost_and_collection_provenance() {
    let tag = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let surface = ironsmith_core::GrantPlayTaggedSurface::default()
        .with_leading_duration(true)
        .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::ThoseCards);
    let grant_play = Effect::new(
        crate::effects::GrantPlayTaggedEffect::new(
            tag.clone(),
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            true,
            false,
        )
        .with_surface(surface),
    );
    let grant_free = Effect::new(
        crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(tag, PlayerFilter::You),
    );

    assert_eq!(
        describe_effect_clause_list(&[grant_play, grant_free]).as_deref(),
        Some("until end of turn, you may play those cards without paying their mana costs")
    );
}

#[test]
pub(super) fn typed_source_exiled_permission_and_legacy_default_stay_distinct() {
    let source_surface = ironsmith_core::GrantPlayTaggedSurface::default()
        .with_leading_duration(true)
        .with_object(
            ironsmith_core::GrantPlayTaggedObjectSurface::SpellFromAmongCardsExiledWithSource {
                creature_spell: true,
                source: ironsmith_core::SourceReferenceSurface::ThisPermanentType(
                    "this artifact".to_string(),
                ),
            },
        );
    let source_grant = Effect::new(
        crate::effects::GrantPlayTaggedEffect::new(
            TagKey::from("__source_exiled__"),
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            false,
            false,
        )
        .with_surface(source_surface),
    );
    assert_eq!(
        describe_effect(&source_grant),
        "Until end of turn, you may cast a creature spell from among cards exiled with this artifact"
    );

    let legacy = Effect::new(crate::effects::GrantPlayTaggedEffect::new(
        TagKey::from("__sentence_helper_exiled_l0_s0_e0"),
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        true,
        false,
    ));
    assert_eq!(describe_effect(&legacy), "you may play that card this turn");
}

#[test]
pub(super) fn linked_exile_top_consumes_only_matching_synthetic_player_target() {
    let exiled = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let target_opponent = PlayerFilter::target_opponent();
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
        ChooseSpec::Player(PlayerFilter::Opponent),
    )));
    let exile = Effect::new(
        crate::effects::ExileTopOfLibraryEffect::new(Value::Fixed(1), target_opponent.clone())
            .with_surface(ironsmith_core::ExileTopLibrarySurface::LibraryOwnerAsActor)
            .tag_moved(exiled.clone()),
    );
    let permission_surface = ironsmith_core::GrantPlayTaggedSurface::default()
        .with_leading_duration(true)
        .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard);
    let grant = Effect::new(
        crate::effects::GrantPlayTaggedEffect::new(
            exiled.clone(),
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            true,
            false,
        )
        .with_surface(permission_surface.clone()),
    );

    assert_eq!(
        describe_effect_list(&[target.clone(), exile, grant.clone()]),
        "Target opponent exiles the top card of their library. Until end of turn, you may play that card"
    );

    let imperative = Effect::new(
        crate::effects::ExileTopOfLibraryEffect::new(Value::Fixed(1), target_opponent)
            .tag_moved(exiled.clone()),
    );
    assert_eq!(
        describe_effect_list(&[target.clone(), imperative, grant.clone()]),
        "Exile the top card of target opponent's library. Until end of turn, you may play that card"
    );

    let unrelated = Effect::new(
        crate::effects::ExileTopOfLibraryEffect::new(Value::Fixed(1), PlayerFilter::You)
            .tag_moved(exiled),
    );
    assert_eq!(
        describe_effect_list(&[target, unrelated, grant]),
        "Choose target opponent. Exile the top card of your library. Until end of turn, you may play that card"
    );
}

#[test]
pub(super) fn unknown_grant_play_tags_keep_the_diagnostic_fallback() {
    let effect = Effect::new(crate::effects::GrantPlayTaggedEffect::new(
        TagKey::from("opaque_collection"),
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        false,
        false,
    ));

    assert_eq!(
        describe_effect(&effect),
        "you may cast tagged 'opaque_collection' cards this turn"
    );
}

#[test]
pub(super) fn sentence_helper_exiled_control_source_permission_respects_pool_plurality() {
    let singular = Effect::new(crate::effects::GrantPlayTaggedEffect::new(
        TagKey::from("__sentence_helper_exiled_l0_s0_e0"),
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::ForAsLongAsYouControlSource,
        true,
        false,
    ));
    let plural = Effect::new(
        crate::effects::GrantPlayTaggedEffect::new(
            TagKey::from("__sentence_helper_exiled_l0_s0_e0"),
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsYouControlSource,
            true,
            false,
        )
        .cast_pool_is_plural(true),
    );

    assert_eq!(
        describe_effect(&singular),
        "you may play that card for as long as you control this source"
    );
    assert_eq!(
        describe_effect(&plural),
        "you may play those cards for as long as you control this source"
    );
}

#[test]
pub(super) fn typed_exile_top_conditional_free_cast_compacts_structurally() {
    let exiled = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let player =
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(TagKey::from("triggering")));
    let exile = crate::effects::ExileTopOfLibraryEffect::new(Value::Fixed(1), player)
        .tag_moved(exiled.clone());
    let mut nonland = ObjectFilter::default();
    nonland.excluded_card_types.push(CardType::Land);
    let cast = crate::effects::CastTaggedEffect::new(exiled.clone(), PlayerFilter::You)
        .without_paying_mana_cost();
    let conditional = crate::effects::ConditionalEffect::new(
        Condition::TaggedObjectMatches(exiled, nonland),
        vec![Effect::new(crate::effects::MayEffect::new(vec![
            Effect::new(cast),
        ]))],
        Vec::new(),
    );

    assert_eq!(
        describe_effect_list(&[Effect::new(exile), Effect::new(conditional)]),
        "that player exiles the top card of their library. If it's a nonland card, you may cast it without paying its mana cost"
    );
}

#[test]
pub(super) fn each_player_may_discard_draw_commander_value_compaction_preserves_equal_to_hint() {
    let mut commanders = ObjectFilter::default();
    commanders.any_of = vec![
        ObjectFilter::default()
            .in_zone(Zone::Battlefield)
            .owned_by(PlayerFilter::IteratedPlayer)
            .commander(),
        ObjectFilter::default()
            .in_zone(Zone::Command)
            .owned_by(PlayerFilter::IteratedPlayer)
            .commander(),
    ];
    let amount = Value::GreatestManaValue(commanders).with_surface_hint(ValueSurfaceHint::EqualTo);
    let effects = vec![Effect::new(crate::effects::ForPlayersEffect {
        filter: PlayerFilter::Any,
        effects: vec![Effect::new(crate::effects::MayEffect::new(vec![
            Effect::new(crate::effects::DiscardHandEffect::new(
                PlayerFilter::IteratedPlayer,
            )),
            Effect::new(crate::effects::DrawCardsEffect::new(
                amount,
                PlayerFilter::IteratedPlayer,
            )),
        ]))],
        starting_with_controller: false,
        stop_after_first_happened: false,
    })];

    assert_eq!(
        describe_effect_list(&effects),
        "Each player may discard their hand and draw cards equal to the greatest mana value of a commander they own on the battlefield or in the command zone"
    );
}

#[test]
pub(super) fn structural_quantified_optional_hand_wheel_keeps_coordinated_surface() {
    let effect = Effect::for_players(
        PlayerFilter::Any,
        vec![Effect::new(crate::effects::MayEffect::new_for_player(
            vec![
                Effect::new(crate::effects::DiscardHandEffect::new(
                    PlayerFilter::IteratedPlayer,
                )),
                Effect::new(crate::effects::DrawCardsEffect::new(
                    Value::Fixed(7),
                    PlayerFilter::IteratedPlayer,
                )),
            ],
            PlayerFilter::IteratedPlayer,
        ))],
    );

    assert_eq!(
        describe_structural_multisentence_effect_list(&[effect]).as_deref(),
        Some("Each player may discard their hand and draw seven cards")
    );
}

#[test]
pub(super) fn each_player_on_your_team_may_discard_card_then_draw_compacts() {
    let team_filter = PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::Opponent);
    let effects = vec![
        Effect::with_id(
            0,
            Effect::for_players(
                team_filter,
                vec![Effect::may(vec![Effect::discard_player(
                    Value::Fixed(1),
                    PlayerFilter::IteratedPlayer,
                    false,
                )])],
            ),
        ),
        Effect::if_then(
            crate::effect::EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::target_draws(
                Value::Fixed(1),
                PlayerFilter::IteratedPlayer,
            )],
        ),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Each player on your team may discard a card, then each player who discarded a card this way draws a card"
    );
}

#[test]
pub(super) fn nested_each_player_on_your_team_may_discard_card_then_draw_compacts() {
    let team_filter = PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::Opponent);
    let effects = vec![Effect::for_players(
        team_filter,
        vec![
            Effect::with_id(
                0,
                Effect::may(vec![Effect::discard_player(
                    Value::Fixed(1),
                    PlayerFilter::IteratedPlayer,
                    false,
                )]),
            ),
            Effect::if_then(
                crate::effect::EffectId(0),
                EffectPredicate::Happened,
                vec![Effect::target_draws(
                    Value::Fixed(1),
                    PlayerFilter::IteratedPlayer,
                )],
            ),
        ],
    )];

    assert_eq!(
        describe_effect_list(&effects),
        "Each player on your team may discard a card, then each player who discarded a card this way draws a card"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_direct_each_other_player_discard_draw_for_discarded() {
    let discard = Effect::with_id(
        7,
        Effect::new(crate::effects::DiscardEffect::new_with_filter(
            1,
            PlayerFilter::NotYou,
            false,
            None,
        )),
    );
    let draw = Effect::new(crate::effects::DrawCardsEffect::new(
        Value::EffectMetric {
            effect_id: crate::effect::EffectId(7),
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::Count,
        },
        PlayerFilter::You,
    ));
    let effects = vec![discard, draw];
    let expected =
        "Each other player discards a card. You draw a card for each card discarded this way";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_each_other_player_discard_draw_for_discarded() {
    let discard = Effect::with_id(
        7,
        Effect::for_players(
            PlayerFilter::NotYou,
            vec![Effect::discard_player(
                1,
                PlayerFilter::IteratedPlayer,
                false,
            )],
        ),
    );
    let draw = Effect::new(crate::effects::DrawCardsEffect::new(
        Value::EffectMetric {
            effect_id: crate::effect::EffectId(7),
            source: crate::effect::EffectMetricSource::AffectedObjects,
            metric: crate::effect::EffectMetric::AffectedCount,
        },
        PlayerFilter::You,
    ));
    let effects = vec![discard, draw];
    let expected =
        "Each other player discards a card. You draw a card for each card discarded this way";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_each_player_may_search_then_shuffle() {
    let tag = TagKey::from("searched_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::IteratedPlayer),
        ChoiceCount::exactly(1),
        PlayerFilter::IteratedPlayer,
        tag.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let move_each = crate::effects::ForEachTaggedEffect::new(
        tag,
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Hand,
            false,
        ))],
    );
    let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![
        Effect::new(choose),
        Effect::new(move_each),
    ]));
    let search = Effect::with_id(
        9,
        Effect::may_player(PlayerFilter::IteratedPlayer, vec![sequence]),
    );
    let shuffle = Effect::if_then(
        crate::effect::EffectId(9),
        EffectPredicate::Happened,
        vec![Effect::new(crate::effects::ShuffleLibraryEffect::new(
            PlayerFilter::IteratedPlayer,
        ))],
    );
    let effects = vec![Effect::for_players(
        PlayerFilter::Any,
        vec![search, shuffle],
    )];
    let expected = "Each player may search their library for a card and put that card into their hand. Then each player who searched their library this way shuffles";

    assert_eq!(describe_effect_list(&effects), expected);
}

#[test]
pub(super) fn describe_effect_list_compacts_conditional_player_search_then_shuffle() {
    let tag = TagKey::from("searched_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::IteratedPlayer)
            .with_type(CardType::Land)
            .with_supertype(Supertype::Basic),
        ChoiceCount::exactly(1),
        PlayerFilter::IteratedPlayer,
        tag.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let move_each = crate::effects::ForEachTaggedEffect::new(
        tag,
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Hand,
            false,
        ))],
    );
    let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![
        Effect::new(choose),
        Effect::new(move_each),
    ]));
    let controlled_lands = ObjectFilter::land()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::IteratedPlayer);
    let conditional = Effect::new(crate::effects::ConditionalEffect::new(
        Condition::ValueComparison {
            left: Value::Count(controlled_lands),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(4),
        },
        vec![Effect::may_player(
            PlayerFilter::IteratedPlayer,
            vec![sequence],
        )],
        Vec::new(),
    ));
    let search = Effect::with_id(9, conditional);
    let shuffle = Effect::if_then(
        crate::effect::EffectId(9),
        EffectPredicate::Happened,
        vec![Effect::new(crate::effects::ShuffleLibraryEffect::new(
            PlayerFilter::IteratedPlayer,
        ))],
    );
    let effects = vec![Effect::for_players(
        PlayerFilter::Any,
        vec![search, shuffle],
    )];
    let expected = "Each player who controls four or fewer lands may search their library for a basic land card and put that card into their hand. Then each player who searched their library this way shuffles";

    assert_eq!(describe_effect_list(&effects), expected);
}

#[test]
pub(super) fn conditional_dynamic_player_search_places_where_x_after_the_destination() {
    let tag = TagKey::from("searched_0");
    let controlled_lands = ObjectFilter::land()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::IteratedPlayer);
    let count_value = Value::Add(
        Box::new(Value::Fixed(5)),
        Box::new(Value::Scaled(
            Box::new(Value::Count(controlled_lands.clone())),
            -1,
        )),
    )
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::IteratedPlayer)
            .with_type(CardType::Land)
            .with_supertype(Supertype::Basic),
        ChoiceCount::up_to_dynamic_x(),
        PlayerFilter::IteratedPlayer,
        tag.clone(),
    )
    .with_count_value(count_value)
    .in_zone(Zone::Library)
    .as_optional_search();
    let move_each = crate::effects::ForEachTaggedEffect::new(
        tag,
        vec![Effect::put_onto_battlefield(
            ChooseSpec::Iterated,
            false,
            PlayerFilter::IteratedPlayer,
        )],
    );
    let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![
        Effect::new(choose),
        Effect::new(move_each),
    ]));
    let conditional = Effect::new(crate::effects::ConditionalEffect::new(
        Condition::ValueComparison {
            left: Value::Count(controlled_lands),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(4),
        },
        vec![Effect::may_player(
            PlayerFilter::IteratedPlayer,
            vec![sequence],
        )],
        Vec::new(),
    ));
    let search = Effect::with_id(9, conditional);
    let shuffle = Effect::if_then(
        crate::effect::EffectId(9),
        EffectPredicate::Happened,
        vec![Effect::new(crate::effects::ShuffleLibraryEffect::new(
            PlayerFilter::IteratedPlayer,
        ))],
    );
    let effects = vec![Effect::for_players(
        PlayerFilter::Any,
        vec![search, shuffle],
    )];

    assert_eq!(
        describe_effect_list(&effects),
        "Each player who controls four or fewer lands may search their library for up to X basic land cards and put them onto the battlefield, where X is five minus the number of lands they control. Then each player who searched their library this way shuffles"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_inline_each_opponent_may_search_then_shuffle() {
    let tag = TagKey::from("searched_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::IteratedPlayer)
            .with_type(CardType::Land)
            .with_supertype(Supertype::Basic),
        ChoiceCount::exactly(1),
        PlayerFilter::IteratedPlayer,
        tag.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let move_each = crate::effects::ForEachTaggedEffect::new(
        tag,
        vec![Effect::put_onto_battlefield(
            ChooseSpec::Iterated,
            true,
            PlayerFilter::IteratedPlayer,
        )],
    );
    let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![
        Effect::new(choose),
        Effect::new(move_each),
        Effect::shuffle_library_player(PlayerFilter::IteratedPlayer),
    ]));
    let effects = vec![Effect::for_players(
        PlayerFilter::Opponent,
        vec![Effect::may_player(
            PlayerFilter::IteratedPlayer,
            vec![sequence],
        )],
    )];

    assert_eq!(
        describe_effect_list(&effects),
        "Each opponent may search their library for a basic land card, put it onto the battlefield tapped, then shuffle"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_inline_each_player_search_then_shuffle() {
    let tag = TagKey::from("searched_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::IteratedPlayer)
            .with_type(CardType::Land)
            .with_supertype(Supertype::Basic),
        ChoiceCount::up_to(2),
        PlayerFilter::IteratedPlayer,
        tag.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let move_each = crate::effects::ForEachTaggedEffect::new(
        tag,
        vec![Effect::put_onto_battlefield(
            ChooseSpec::Iterated,
            false,
            PlayerFilter::IteratedPlayer,
        )],
    );
    let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![
        Effect::new(choose),
        Effect::new(move_each),
        Effect::shuffle_library_player(PlayerFilter::IteratedPlayer),
    ]));
    let effects = vec![Effect::for_players(PlayerFilter::Any, vec![sequence])];

    assert_eq!(
        describe_effect_list(&effects),
        "Each player searches their library for up to 2 basic land cards, puts them onto the battlefield, then shuffles"
    );
}

#[test]
pub(super) fn qualified_dynamic_player_search_preserves_the_difference_bound() {
    let tag = TagKey::from("searched_0");
    let iterated_lands = ObjectFilter::land()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::IteratedPlayer);
    let most_lands = ObjectFilter::land()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::Any);
    let difference = Value::Add(
        Box::new(Value::GreatestCount(most_lands.clone())),
        Box::new(Value::Scaled(
            Box::new(Value::Count(iterated_lands.clone())),
            -1,
        )),
    )
    .with_surface_hint(ValueSurfaceHint::Difference);
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::IteratedPlayer)
            .with_type(CardType::Land)
            .with_supertype(Supertype::Basic),
        ChoiceCount::up_to_dynamic_x(),
        PlayerFilter::IteratedPlayer,
        tag.clone(),
    )
    .with_count_value(difference)
    .in_zone(Zone::Library)
    .as_search();
    let move_each = crate::effects::ForEachTaggedEffect::new(
        tag,
        vec![Effect::put_onto_battlefield(
            ChooseSpec::Iterated,
            true,
            PlayerFilter::IteratedPlayer,
        )],
    );
    let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![
        Effect::new(choose),
        Effect::new(move_each),
        Effect::shuffle_library_player(PlayerFilter::IteratedPlayer),
    ]));
    let conditional = Effect::new(crate::effects::ConditionalEffect::new(
        Condition::ValueComparison {
            left: Value::Count(iterated_lands),
            operator: crate::effect::ValueComparisonOperator::LessThan,
            right: Value::GreatestCount(most_lands),
        },
        vec![sequence],
        Vec::new(),
    ));
    let effects = vec![Effect::for_players(PlayerFilter::Any, vec![conditional])];

    assert_eq!(
        describe_effect_list(&effects),
        "Each player who controls fewer lands than the player who controls the most lands searches their library for a number of basic land cards less than or equal to the difference, puts those cards onto the battlefield tapped, then shuffles"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_sacrifice_that_many_graveyard_return() {
    let sacrifice_tag = TagKey::from("sacrificed_0");
    let mut sacrifice_filter = ObjectFilter::default().controlled_by(PlayerFilter::You);
    sacrifice_filter.any_of = vec![
        ObjectFilter::artifact().in_zone(Zone::Battlefield),
        ObjectFilter::enchantment().in_zone(Zone::Battlefield),
        ObjectFilter::default().token().in_zone(Zone::Battlefield),
    ];
    let choose_sacrificed = crate::effects::ChooseObjectsEffect::new(
        sacrifice_filter.in_zone(Zone::Battlefield),
        ChoiceCount::any_number(),
        PlayerFilter::You,
        sacrifice_tag.clone(),
    );
    let sacrifice = Effect::with_id(
        11,
        Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
            ObjectFilter::tagged(sacrifice_tag.clone()),
            Value::Count(ObjectFilter::tagged(sacrifice_tag)),
            PlayerFilter::You,
        )),
    );

    let return_tag = TagKey::from("chosen_return_0");
    let choose_returned = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You),
        ChoiceCount::dynamic_x(),
        PlayerFilter::You,
        return_tag.clone(),
    )
    .with_count_value(Value::EffectValue(crate::effect::EffectId(11)));
    let return_chosen =
        Effect::return_from_graveyard_to_battlefield(ChooseSpec::Tagged(return_tag), false)
            .tag("returned_0");
    let effects = vec![
        Effect::new(choose_sacrificed),
        sacrifice,
        Effect::new(choose_returned),
        return_chosen,
    ];
    let expected = "Sacrifice any number of artifacts, enchantments, and/or tokens. Return that many creature cards from your graveyard to the battlefield";
    let refs = effects.iter().collect::<Vec<_>>();
    let choose = refs[0]
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("choose sacrificed");
    let with_id = refs[1]
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("with-id sacrifice");
    let sacrifice_view = sacrifice_view(&with_id.effect).expect("sacrifice view");
    let return_choose = refs[2]
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("choose returned");
    let return_to_battlefield = unwrap_basic_tag_wrappers(refs[3])
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        .expect("return effect");

    assert_eq!(choose_primary_zone(choose), Some(Zone::Battlefield));
    assert!(filter_is_exactly_tagged(sacrifice_view.filter, &choose.tag));
    assert!(
        matches!(sacrifice_view.count, Value::Count(count_filter) if filter_is_exactly_tagged(count_filter, &choose.tag))
    );
    assert_eq!(choose_primary_zone(return_choose), Some(Zone::Graveyard));
    assert!(return_choose.count.is_dynamic_x());
    assert!(
        return_choose
            .count_value
            .as_ref()
            .is_some_and(|value| is_effect_count_reference(value, Some(with_id.id)))
    );
    assert!(
        is_creature_card_filter_from_your_graveyard(&return_choose.filter),
        "return filter: {:?}",
        return_choose.filter
    );
    assert!(choose_spec_is_tagged_object(
        &return_to_battlefield.target,
        &return_choose.tag
    ));

    assert_eq!(
        describe_choose_sacrifice_then_return_from_graveyard(&refs).as_deref(),
        Some(expected)
    );
    assert_eq!(describe_effect_list(&effects), expected);
}

#[test]
pub(super) fn prevent_all_damage_follow_up_counters_uses_prevention_surface() {
    let prevent = crate::effects::PreventAllDamageToTargetEffect::new(
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
        Until::EndOfTurn,
    )
    .with_follow_up_effects(vec![Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        Value::EventValue(EventValueSpec::Amount),
        ChooseSpec::AnyTarget,
    ))]);
    let effect = Effect::new(prevent);

    assert_eq!(
        describe_effect(&effect),
        "Prevent all damage that would be dealt to target creature this turn. For each 1 damage prevented this way, put a +1/+1 counter on that creature"
    );
}

#[test]
pub(super) fn prevent_all_damage_follow_up_counters_keeps_replacement_surface_for_tagged_target() {
    let protected = TagKey::from("protected_0");
    let prevent = crate::effects::PreventAllDamageToTargetEffect::new(
        ChooseSpec::Tagged(protected),
        Until::EndOfTurn,
    )
    .with_follow_up_effects(vec![Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        Value::EventValue(EventValueSpec::Amount),
        ChooseSpec::AnyTarget,
    ))]);
    let effect = Effect::new(prevent);

    assert_eq!(
        describe_effect(&effect),
        "If damage would be dealt to it this turn, prevent that damage and put that many +1/+1 counters on it"
    );
}

#[test]
pub(super) fn prevent_all_damage_from_opponents_creatures_uses_by_clause_surface() {
    let mut source_filter = ObjectFilter::creature();
    source_filter.zone = Some(Zone::Battlefield);
    source_filter.controller = Some(PlayerFilter::Opponent);
    let mut damage_filter = crate::prevention::DamageFilter::all();
    damage_filter.from_source = Some(source_filter);

    let effect = Effect::new(crate::effects::PreventAllDamageEffect::all_with_filter(
        damage_filter,
        Until::EndOfTurn,
    ));

    assert_eq!(
        describe_effect(&effect),
        "Prevent all damage that would be dealt this turn by creatures your opponents control"
    );
}

#[test]
pub(super) fn prevent_all_combat_damage_from_opponents_creatures_uses_by_clause_surface() {
    let mut source_filter = ObjectFilter::creature();
    source_filter.zone = Some(Zone::Battlefield);
    source_filter.controller = Some(PlayerFilter::Opponent);
    let mut damage_filter = crate::prevention::DamageFilter::combat();
    damage_filter.from_source = Some(source_filter);

    let effect = Effect::new(crate::effects::PreventAllDamageEffect::all_with_filter(
        damage_filter,
        Until::EndOfTurn,
    ));

    assert_eq!(
        describe_effect(&effect),
        "Prevent all combat damage that would be dealt this turn by creatures your opponents control"
    );
}

#[test]
pub(super) fn sacrifice_source_then_extra_turn_renders_as_single_clause() {
    let effects = vec![
        Effect::new(crate::effects::SacrificeTargetEffect::new(
            ChooseSpec::Source,
        )),
        Effect::new(crate::effects::ExtraTurnEffect::you()),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Sacrifice this source and take an extra turn after this one"
    );
}

#[test]
pub(super) fn targeted_permanent_sacrifice_names_its_controller_as_actor() {
    let sacrifice = Effect::new(crate::effects::SacrificeTargetEffect::new(
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::permanent())),
    ));

    assert_eq!(
        describe_effect(&sacrifice),
        "Target permanent's controller sacrifices it"
    );
}

#[test]
pub(super) fn backup_etb_trigger_renders_as_keyword_surface() {
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_enters_battlefield(),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::backup(
            1,
            Vec::new(),
        )]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        describe_backup_keyword(&triggered),
        Some("Backup 1".to_string())
    );
    assert_eq!(
        describe_triggered_inline_ability(&triggered, "this creature"),
        "Backup 1"
    );

    let ability = Ability {
        kind: AbilityKind::Triggered(triggered),
        functional_zones: vec![Zone::Battlefield],
    };
    assert_eq!(
        describe_ability(0, &ability, "this creature", true),
        vec!["Triggered ability 0: Backup 1".to_string()]
    );
}

fn hideaway_trigger(count: i32, face_down: bool) -> crate::ability::TriggeredAbility {
    let looked = TagKey::from("hideaway_looked");
    let chosen = TagKey::from("hideaway_exiled");
    let mut choose_filter = ObjectFilter::tagged(looked.clone());
    choose_filter.zone = Some(Zone::Library);
    let effects = vec![
        Effect::new(crate::effects::LookAtTopCardsEffect::new(
            PlayerFilter::You,
            Value::Fixed(count),
            looked.clone(),
        )),
        Effect::new(crate::effects::ChooseObjectsEffect::new(
            choose_filter,
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            chosen.clone(),
        )),
        Effect::new(
            crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(chosen.clone()))
                .with_face_down(face_down),
        ),
        Effect::new(
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                looked,
                Some(chosen),
                crate::effects::consult_helpers::LibraryBottomOrder::Random,
                PlayerFilter::You,
            ),
        ),
    ];
    crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_enters_battlefield(),
        effects: crate::resolution::ResolutionProgram::from_effects(effects),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    }
}

#[test]
pub(super) fn hideaway_etb_trigger_renders_as_numbered_keyword_surface() {
    for count in [4, 5] {
        let triggered = hideaway_trigger(count, true);
        assert_eq!(
            describe_structural_hideaway_keyword(&triggered),
            Some(format!("Hideaway {count}"))
        );
        assert_eq!(
            describe_triggered_inline_ability(&triggered, "this permanent"),
            format!("Hideaway {count}")
        );
        let ability = Ability {
            kind: AbilityKind::Triggered(triggered),
            functional_zones: vec![Zone::Battlefield],
        };
        assert_eq!(
            describe_ability(0, &ability, "this permanent", true),
            vec![format!("Keyword ability 0: Hideaway {count}")]
        );
    }

    let face_up_exile = hideaway_trigger(4, false);
    assert_eq!(
        describe_structural_hideaway_keyword(&face_up_exile),
        None,
        "non-Hideaway look/exile sequences must not receive the keyword surface"
    );
}

#[test]
pub(super) fn tap_for_mana_trigger_tap_matching_lands_renders_mana_web_surface() {
    let trigger_filter = ObjectFilter::land()
        .controlled_by(PlayerFilter::Opponent)
        .in_zone(Zone::Battlefield);
    let tap_filter = ObjectFilter::land()
        .controlled_by(PlayerFilter::IteratedPlayer)
        .in_zone(Zone::Battlefield);
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::player_taps_for_mana(PlayerFilter::Any, trigger_filter),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::tap(
            ChooseSpec::All(tap_filter),
        )]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        describe_triggered_inline_ability(&triggered, "this artifact"),
        "Whenever a land an opponent controls is tapped for mana, tap all lands that player controls that could produce any type of mana that land could produce"
    );

    let ability = Ability {
        kind: AbilityKind::Triggered(triggered),
        functional_zones: vec![Zone::Battlefield],
    };
    assert_eq!(
        describe_ability(0, &ability, "this artifact", true),
        vec![
            "Triggered ability 0: Whenever a land an opponent controls is tapped for mana, tap all lands that player controls that could produce any type of mana that land could produce"
                .to_string()
        ]
    );
}

#[test]
pub(super) fn tap_for_mana_additional_mana_triggers_render_oracle_surface() {
    fn triggered(
        filter: ObjectFilter,
        add_mana: impl FnOnce(PlayerFilter) -> Effect,
    ) -> crate::ability::TriggeredAbility {
        let triggering_tag = TagKey::from("triggering");
        let controller =
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(triggering_tag.clone()));
        crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::player_taps_for_mana(PlayerFilter::Any, filter),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::new(crate::effects::TagTriggeringObjectEffect::new(
                    triggering_tag,
                )),
                add_mana(controller),
            ]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        }
    }

    let enchanted_land =
        ObjectFilter::land().match_tagged("enchanted", TaggedOpbjectRelation::IsTaggedObject);
    let any_colors = triggered(enchanted_land.clone(), |controller| {
        Effect::new(crate::effects::AddManaOfAnyColorEffect::restricted(
            Value::Fixed(2),
            controller,
            crate::color::Color::ALL.to_vec(),
        ))
    });

    assert_eq!(
        describe_tap_for_mana_additional_mana_trigger(&any_colors).as_deref(),
        Some(
            "Whenever enchanted land is tapped for mana, its controller adds an additional two mana in any combination of colors"
        )
    );
    assert_eq!(
        describe_triggered_inline_ability(&any_colors, "this enchantment"),
        "Whenever enchanted land is tapped for mana, its controller adds an additional two mana in any combination of colors"
    );

    let forest = ObjectFilter::land().with_subtype(crate::types::Subtype::Forest);
    let fixed = triggered(forest, |controller| {
        Effect::new(crate::effects::AddManaEffect::new(
            vec![crate::mana::ManaSymbol::Green],
            controller,
        ))
    });
    assert_eq!(
        describe_triggered_inline_ability(&fixed, "this enchantment"),
        "Whenever a Forest is tapped for mana, its controller adds an additional {G}"
    );

    let scaled = triggered(enchanted_land, |controller| {
        Effect::new(crate::effects::AddScaledManaEffect::new(
            vec![crate::mana::ManaSymbol::Green],
            Value::Count(
                ObjectFilter::creature()
                    .with_subtype(crate::types::Subtype::Elf)
                    .in_zone(Zone::Battlefield),
            ),
            controller,
        ))
    });
    assert_eq!(
        describe_triggered_inline_ability(&scaled, "this enchantment"),
        "Whenever enchanted land is tapped for mana, its controller adds an additional {G} for each Elf on the battlefield"
    );

    let enchanted_forest = ObjectFilter::land()
        .with_subtype(crate::types::Subtype::Forest)
        .match_tagged("enchanted", TaggedOpbjectRelation::IsTaggedObject);
    let chosen = triggered(enchanted_forest, |controller| {
        Effect::new(crate::effects::mana::AddManaOfChosenColorEffect::new(
            Value::Fixed(1),
            controller,
        ))
    });
    assert_eq!(
        describe_triggered_inline_ability(&chosen, "this enchantment"),
        "Whenever enchanted Forest is tapped for mana, its controller adds an additional one mana of the chosen color"
    );
}

#[test]
pub(super) fn generic_becomes_blocked_trigger_surface_keeps_indefinite_article() {
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::becomes_blocked(
            crate::target::ObjectFilter::creature().controlled_by(crate::target::PlayerFilter::You),
        ),
        effects: crate::resolution::ResolutionProgram::from_effects(Vec::new()),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        describe_trigger_surface_with_frequency(&triggered, None, "this creature"),
        "Whenever a creature you control becomes blocked"
    );
}

#[test]
pub(super) fn each_player_upkeep_contextualizes_active_player_antecedents_only() {
    let triggered = |player| crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::beginning_of_upkeep(player),
        effects: crate::resolution::ResolutionProgram::from_effects(Vec::new()),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };
    let each_upkeep = triggered(PlayerFilter::Any);

    assert_eq!(
        rewrite_each_upkeep_active_player_reference(
            &each_upkeep,
            "The active player mills X cards, where X is the number of cards in the active player's hand"
                .to_string(),
        ),
        "That player mills X cards, where X is the number of cards in their hand"
    );
    assert_eq!(
        rewrite_each_upkeep_active_player_reference(
            &each_upkeep,
            "Return the active player's permanent to its owner's hand unless they pay 2 life"
                .to_string(),
        ),
        "That player returns a permanent they control to its owner's hand unless they pay 2 life"
    );
    assert_eq!(
        rewrite_each_upkeep_active_player_reference(
            &each_upkeep,
            "Untap the active player's land".to_string(),
        ),
        "That player untaps a land they control"
    );
    assert_eq!(
        rewrite_each_upkeep_active_player_reference(
            &each_upkeep,
            "The active player may pay 2 life. If that player doesn't, that player returns a permanent they control"
                .to_string(),
        ),
        "That player may pay 2 life. If they don't, they return a permanent they control"
    );

    let your_upkeep = triggered(PlayerFilter::You);
    let standalone =
        "Untap the active player's land. If that player doesn't, draw a card".to_string();
    assert_eq!(
        rewrite_each_upkeep_active_player_reference(&your_upkeep, standalone.clone()),
        standalone,
        "active-player terminology outside an each-player upkeep must remain literal"
    );
}

#[test]
pub(super) fn generic_becomes_blocked_attached_subject_omits_indefinite_article() {
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::becomes_blocked(
            crate::target::ObjectFilter::creature().match_tagged(
                "enchanted",
                crate::target::TaggedOpbjectRelation::IsTaggedObject,
            ),
        ),
        effects: crate::resolution::ResolutionProgram::from_effects(Vec::new()),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        describe_trigger_surface_with_frequency(&triggered, None, "this creature"),
        "Whenever enchanted creature becomes blocked"
    );
}

#[test]
pub(super) fn generic_attack_attached_subject_omits_indefinite_article() {
    for tag in ["enchanted", "equipped"] {
        let triggered = crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::attacks(
                crate::target::ObjectFilter::creature()
                    .match_tagged(tag, crate::target::TaggedOpbjectRelation::IsTaggedObject),
            ),
            effects: crate::resolution::ResolutionProgram::from_effects(Vec::new()),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        };

        assert_eq!(
            describe_trigger_surface_with_frequency(&triggered, None, "this creature"),
            format!("Whenever {tag} creature attacks")
        );
    }
}

#[test]
pub(super) fn source_bound_trigger_heads_use_typed_self_subjects() {
    let render = |trigger, self_subject| {
        let triggered = crate::ability::TriggeredAbility {
            trigger,
            effects: crate::resolution::ResolutionProgram::from_effects(Vec::new()),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        };
        describe_trigger_surface_with_frequency(&triggered, None, self_subject)
    };

    assert_eq!(
        render(crate::triggers::Trigger::this_attacks(), "this Vehicle"),
        "Whenever this Vehicle attacks"
    );
    assert_eq!(
        render(
            crate::triggers::Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),
            "this Vehicle",
        ),
        "Whenever this Vehicle deals combat damage to a player"
    );
    assert_eq!(
        render(crate::triggers::Trigger::this_attacks(), "this land"),
        "Whenever this land attacks"
    );
    assert_eq!(
        render(crate::triggers::Trigger::this_dies(), "this permanent"),
        "When this permanent dies"
    );
}

#[test]
pub(super) fn typed_self_subjects_do_not_rewrite_creature_or_filtered_trigger_heads() {
    let render = |trigger, self_subject| {
        let triggered = crate::ability::TriggeredAbility {
            trigger,
            effects: crate::resolution::ResolutionProgram::from_effects(Vec::new()),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        };
        describe_trigger_surface_with_frequency(&triggered, None, self_subject)
    };

    assert_eq!(
        render(crate::triggers::Trigger::this_attacks(), "this creature"),
        "Whenever this creature attacks"
    );
    assert_eq!(
        render(
            crate::triggers::Trigger::attacks(ObjectFilter::creature()),
            "this Vehicle",
        ),
        "Whenever a creature attacks"
    );
}

#[test]
pub(super) fn unowned_count_damage_keeps_where_x_surface() {
    let mut filter = ObjectFilter::land();
    filter.tapped = false;
    let effect = Effect::deal_damage(
        Value::Count(filter).with_surface_hint(ValueSurfaceHint::WhereXIs),
        ChooseSpec::Player(PlayerFilter::Active),
    );

    assert_eq!(
        describe_effect(&effect),
        "Deal X damage to that player, where X is the number of lands"
    );
}

#[test]
pub(super) fn angels_trumpet_tapped_metric_damage_preserves_authored_scalar_surface() {
    let amount = Value::PriorEffectMetric {
        effect_id: crate::effect::EffectId(0),
        query: crate::effect::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::AffectedObjects,
            crate::effect::EffectMetric::Count,
        )
        .with_filter(ObjectFilter::creature())
        .with_action(crate::effect::PriorEffectAction::Tapped),
    };
    let target = ChooseSpec::Player(PlayerFilter::Any);

    let equal_to = Effect::deal_damage(
        amount.clone().with_surface_hint(ValueSurfaceHint::EqualTo),
        target.clone(),
    );
    assert_eq!(
        describe_effect(&equal_to),
        "Deal damage to a player equal to the number of creatures tapped this way"
    );

    let for_each = Effect::deal_damage(amount.with_surface_hint(ValueSurfaceHint::ForEach), target);
    assert_eq!(
        describe_effect(&for_each),
        "Deal 1 damage to a player for each creature tapped this way"
    );
}

#[test]
pub(super) fn destroyed_metric_for_each_surfaces_render_counters_and_life_multipliers() {
    let destroyed = Value::PriorEffectMetric {
        effect_id: crate::effect::EffectId(0),
        query: crate::effect::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::AffectedObjects,
            crate::effect::EffectMetric::Count,
        )
        .with_filter(ObjectFilter::creature())
        .with_action(crate::effect::PriorEffectAction::Destroyed),
    };

    let counters = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        destroyed
            .clone()
            .with_surface_hint(ValueSurfaceHint::ForEach),
        ChooseSpec::Source,
    ));
    assert_eq!(
        describe_effect(&counters),
        "Put a +1/+1 counter on this source for each creature destroyed this way"
    );

    let gain_one = Effect::gain_life(
        destroyed
            .clone()
            .with_surface_hint(ValueSurfaceHint::ForEach),
    );
    assert_eq!(
        describe_effect(&gain_one),
        "you gain 1 life for each creature destroyed this way"
    );

    let gain_two = Effect::gain_life(
        Value::Add(Box::new(destroyed.clone()), Box::new(destroyed.clone()))
            .with_surface_hint(ValueSurfaceHint::ForEach),
    );
    assert_eq!(
        describe_effect(&gain_two),
        "you gain 2 life for each creature destroyed this way"
    );

    let lose_two = Effect::lose_life(
        Value::Scaled(Box::new(destroyed), 2).with_surface_hint(ValueSurfaceHint::ForEach),
    );
    assert_eq!(
        describe_effect(&lose_two),
        "you lose 2 life for each creature destroyed this way"
    );
}

#[test]
pub(super) fn scaled_count_life_loss_uses_per_object_surface() {
    let mut destroyed = ObjectFilter::creature();
    destroyed.zone = None;
    destroyed.set_prior_effect_action_surface(Some(crate::effect::PriorEffectAction::Destroyed));
    let count = Value::Count(destroyed);
    for amount in [
        Value::Scaled(Box::new(count.clone()), 2),
        Value::Add(Box::new(count.clone()), Box::new(count)),
    ] {
        let lose = Effect::lose_life(amount);
        assert_eq!(
            describe_effect(&lose),
            "you lose 2 life for each creature destroyed this way"
        );
    }
}

#[test]
pub(super) fn dynamic_any_one_color_mana_renders_as_x_with_where_clause() {
    let effect = Effect::add_mana_of_any_one_color(Value::PowerOf(Box::new(ChooseSpec::Source)));

    assert_eq!(
        describe_effect(&effect),
        "Add X mana of any one color, where X is this source's power"
    );
}

#[test]
pub(super) fn chosen_and_land_produced_mana_omit_your_pool_suffix() {
    let chosen = Effect::new(crate::effects::mana::AddManaOfChosenColorEffect::new(
        Value::Fixed(1),
        PlayerFilter::You,
    ));
    assert_eq!(describe_effect(&chosen), "Add one mana of the chosen color");

    let fixed_choice = Effect::new(
        crate::effects::mana::AddManaOfChosenColorEffect::with_fixed_option(
            Value::Fixed(2),
            PlayerFilter::You,
            crate::color::Color::White,
        ),
    );
    assert_eq!(
        describe_effect(&fixed_choice),
        "Add {W} or two mana of the chosen color"
    );

    let land_produced = Effect::new(crate::effects::AddManaOfLandProducedTypesEffect::new(
        Value::Fixed(1),
        PlayerFilter::You,
        ObjectFilter::land().controlled_by(PlayerFilter::You),
        true,
        false,
    ));
    assert_eq!(
        describe_effect(&land_produced),
        "Add one mana of any type that a land you control could produce"
    );
}

#[test]
pub(super) fn actual_land_produced_mana_uses_trigger_player_subject_and_event_surface() {
    let actual = Effect::new(
        crate::effects::AddManaOfLandProducedTypesEffect::from_triggering_event(
            Value::Fixed(1),
            PlayerFilter::IteratedPlayer,
            ObjectFilter::land(),
            true,
            false,
        ),
    );
    assert_eq!(
        describe_effect(&actual),
        "That player adds one mana of any type that land produced"
    );

    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::player_taps_for_mana(
            PlayerFilter::Any,
            ObjectFilter::land(),
        ),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![actual]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };
    assert_eq!(
        describe_triggered_inline_ability(&triggered, "this enchantment"),
        "Whenever a player taps a land for mana, that player adds one mana of any type that land produced"
    );
}

#[test]
pub(super) fn all_color_combination_mana_uses_colors_surface() {
    let effect = Effect::new(crate::effects::AddManaOfAnyColorEffect::restricted(
        Value::Fixed(2),
        PlayerFilter::You,
        crate::color::Color::ALL.to_vec(),
    ));

    assert_eq!(
        describe_effect(&effect),
        "Add two mana in any combination of colors"
    );
}

#[test]
pub(super) fn riot_structural_keyword_accepts_permanent_haste_choice() {
    let haste_effect = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Source,
        crate::continuous::Modification::AddAbilityGeneric(Ability::static_ability(
            crate::static_abilities::StaticAbility::haste(),
        )),
        Until::Forever,
    ));
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_enters_battlefield(),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::choose_one(
            vec![
                ironsmith_core::EffectMode::new(
                    "This creature enters with a +1/+1 counter on it",
                    vec![Effect::put_counters_on_source(
                        CounterType::PlusOnePlusOne,
                        1,
                    )],
                ),
                ironsmith_core::EffectMode::new("This creature gains haste", vec![haste_effect]),
            ],
        )]),
        choices: vec![],
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        describe_structural_riot_keyword(&triggered),
        Some("Riot".to_string())
    );
}

#[test]
pub(super) fn inline_mana_ability_normalizes_this_source_cost_to_subject() {
    let ability = Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: crate::cost::TotalCost::from_cost(
                crate::costs::Cost::try_effect(Effect::sacrifice_source())
                    .expect("sacrifice source is a cost-executable effect"),
            ),
            effects: crate::resolution::ResolutionProgram::default(),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            is_loyalty_ability: false,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(vec![ManaSymbol::Black]),
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    assert_eq!(
        describe_inline_ability_with_self_subject(&ability, "this land"),
        "Sacrifice this land: Add {B}"
    );
}

#[test]
pub(super) fn granted_attack_trigger_target_blocks_it_compacts_tagged_requirement() {
    let triggering_tag = TagKey::from("triggering");
    let target_tag = TagKey::from("targeted_0");
    let target_choice = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_attacks(),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::tag_triggering_object(triggering_tag.clone()),
            Effect::new(crate::effects::TargetOnlyEffect::new(target_choice.clone()))
                .tag(target_tag.clone()),
            Effect::cant_until(
                crate::effect::Restriction::MustBlockSpecificAttacker {
                    blockers: ObjectFilter::tagged(target_tag),
                    attacker: ObjectFilter::tagged(triggering_tag),
                },
                Until::EndOfTurn,
            ),
        ]),
        choices: vec![target_choice],
        intervening_if: None,
        presentation_label: None,
    };
    let ability = Ability {
        kind: AbilityKind::Triggered(triggered),
        functional_zones: vec![Zone::Battlefield],
    };

    assert_eq!(
        describe_inline_ability_with_self_subject(&ability, "this creature"),
        "Whenever this creature attacks, target creature blocks it this turn if able"
    );
}

#[test]
pub(super) fn granted_damage_trigger_to_effect_controller_uses_spell_caster_surface() {
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_deals_damage_to_player(
            PlayerFilter::EffectController,
            None,
        ),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::new(crate::effects::SacrificeTargetEffect::new(
                ChooseSpec::Source,
            )),
            Effect::new(crate::effects::LoseLifeEffect::you(Value::Fixed(2))),
        ]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };
    let ability = Ability {
        kind: AbilityKind::Triggered(triggered),
        functional_zones: vec![Zone::Battlefield],
    };

    assert_eq!(
        describe_granted_ability_phrase(&ability, "this permanent"),
        "Whenever this permanent deals damage to the player who cast this spell, sacrifice this permanent. You lose 2 life."
    );
}

#[test]
pub(super) fn zero_cost_loyalty_mana_ability_renders_zero_prefix() {
    let ability = Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: crate::cost::TotalCost::free(),
            effects: crate::resolution::ResolutionProgram::default(),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            is_loyalty_ability: true,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(vec![
                ManaSymbol::Colorless,
                ManaSymbol::Colorless,
                ManaSymbol::Colorless,
            ]),
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    assert_eq!(
        describe_ability(4, &ability, "This planeswalker", false),
        vec!["0: Add {C}{C}{C}".to_string()]
    );
}

#[test]
pub(super) fn loyalty_mana_ability_keeps_usage_restriction_clause() {
    let ability = Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: crate::cost::TotalCost::from_cost(crate::costs::Cost::add_counters(
                CounterType::Loyalty,
                1,
            )),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::add_mana_of_any_color_restricted(
                    Value::Fixed(2),
                    crate::color::Color::ALL.to_vec(),
                ),
            ]),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            is_loyalty_ability: true,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![
                crate::ability::ManaUsageRestriction::CastSpellMatching {
                    filter: ObjectFilter::default().with_subtype(Subtype::Dragon),
                    restrict_to_matching_spell: true,
                    grant_uncounterable: false,
                    enters_with_counters: vec![],
                    granted_abilities: vec![],
                },
            ],
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    let rendered = describe_ability(1, &ability, "This planeswalker", false);
    assert_eq!(rendered.len(), 1);
    assert!(rendered[0].starts_with("+1: Add two mana in any combination of colors."));
    assert!(rendered[0].contains("Spend this mana only to cast"));
}

#[test]
pub(super) fn display_x_counter_removal_cost_preserves_x_damage_surface() {
    let ability = Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: crate::cost::TotalCost::from_cost(
                crate::costs::Cost::remove_any_counters_from_source(
                    Some(CounterType::Loyalty),
                    true,
                ),
            ),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::deal_damage(
                Value::X,
                ChooseSpec::target_creature(),
            )]),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            is_loyalty_ability: true,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    assert_eq!(
        describe_ability(0, &ability, "Chandra Nalaar", false),
        vec!["−X: Chandra Nalaar deals X damage to target creature".to_string()]
    );
}

#[test]
pub(super) fn non_display_x_counter_removal_cost_still_expands_x_damage_surface() {
    let effects = "This creature deals X damage to any target".to_string();
    let costs = vec![crate::costs::Cost::remove_all_counters_from_source(Some(
        CounterType::PlusOnePlusOne,
    ))];

    assert_eq!(
        rewrite_cost_bound_x_phrases(effects, &costs),
        "This creature deals damage equal to the number of +1/+1 counters removed this way to any target"
    );
}

#[test]
pub(super) fn commander_identity_mana_uses_oracle_color_identity_surface() {
    let effect =
        Effect::new(crate::effects::AddManaFromCommanderColorIdentityEffect::you(Value::Fixed(1)));
    assert_eq!(
        describe_effect(&effect),
        "Add one mana of any color in your commander's color identity"
    );
}

#[test]
pub(super) fn mana_usage_restriction_special_spell_filters_render_oracle_surfaces() {
    assert_eq!(
        describe_mana_usage_spell_filter_target_with_options(
            &ObjectFilter::default()
                .commander()
                .owned_by(PlayerFilter::You),
            false,
        ),
        Some("your commander".to_string())
    );
    assert_eq!(
        describe_mana_usage_spell_filter_target_with_options(
            &ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
            false,
        ),
        Some("a spell from your graveyard".to_string())
    );
    assert_eq!(
        describe_mana_usage_spell_filter_target_with_options(
            &ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
            true,
        ),
        Some("spells from your graveyard".to_string())
    );
    assert_eq!(
        describe_mana_usage_spell_filter_target_with_options(
            &ObjectFilter::default().in_zone(Zone::Exile),
            false,
        ),
        Some("spells from exile".to_string())
    );
    assert_eq!(
        describe_mana_usage_spell_filter_target_with_options(
            &ObjectFilter::default()
                .with_static_ability(crate::static_abilities::StaticAbilityId::MakeColorless),
            false,
        ),
        Some("a spell with devoid".to_string())
    );

    let mut no_abilities = ObjectFilter::default().with_type(crate::types::CardType::Creature);
    no_abilities.no_abilities = true;
    assert_eq!(
        describe_mana_usage_spell_filter_target_with_options(&no_abilities, false),
        Some("creature spells with no abilities".to_string())
    );
    assert_eq!(
        describe_mana_usage_spell_filter_target_with_options(
            &ObjectFilter::default().owned_by(PlayerFilter::NotYou),
            false,
        ),
        Some("spells you don't own".to_string())
    );

    let mut flashback = ObjectFilter::default().in_zone(Zone::Graveyard);
    flashback.alternative_cast = Some(crate::filter::AlternativeCastKind::Flashback);
    assert_eq!(
        describe_mana_usage_spell_filter_target_with_options(&flashback, true),
        Some("spells with flashback from a graveyard".to_string())
    );
}

#[test]
pub(super) fn effect_metric_values_render_oracle_reference_surfaces() {
    let count_metric = Value::EffectMetric {
        effect_id: crate::effect::EffectId(0),
        source: crate::effect::EffectMetricSource::AffectedObjects,
        metric: crate::effect::EffectMetric::Count,
    };
    assert_eq!(
        describe_effect(&Effect::draw(count_metric.clone())),
        "you draw that many cards"
    );
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            count_metric,
            ChooseSpec::Source,
        ))),
        "Put that many +1/+1 counters on this source"
    );

    let life_lost_metric = Value::EffectMetric {
        effect_id: crate::effect::EffectId(1),
        source: crate::effect::EffectMetricSource::Outcome,
        metric: crate::effect::EffectMetric::LifeLost,
    };
    assert_eq!(
        describe_effect(&Effect::gain_life(life_lost_metric)),
        "you gain life equal to the life lost this way"
    );

    let total_mana_value_metric = Value::EffectMetric {
        effect_id: crate::effect::EffectId(2),
        source: crate::effect::EffectMetricSource::AffectedObjects,
        metric: crate::effect::EffectMetric::TotalManaValue,
    };
    assert!(
        describe_effect(&Effect::deal_damage(
            total_mana_value_metric,
            ChooseSpec::target_player(),
        ))
        .contains("the total mana value of those cards"),
        "aggregate metrics should render as oracle-style aggregate references"
    );

    let greatest_player_count = Value::EffectMetric {
        effect_id: crate::effect::EffectId(3),
        source: crate::effect::EffectMetricSource::Outcome,
        metric: crate::effect::EffectMetric::GreatestPlayerCount,
    };
    assert_eq!(
        describe_effect(&Effect::draw(greatest_player_count)),
        "you draw the greatest number of cards a player discarded this way"
    );

    let hand_count = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You),
    );
    assert_eq!(
        describe_effect(&Effect::discard_player_filtered(
            hand_count,
            PlayerFilter::You,
            false,
            Some(
                ObjectFilter::default()
                    .in_zone(Zone::Hand)
                    .owned_by(PlayerFilter::You)
            ),
        )),
        "you discard all the cards in your hand"
    );

    let iterated_hand_count = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::IteratedPlayer),
    );
    assert_eq!(
        describe_effect(&Effect::discard_player_filtered(
            iterated_hand_count,
            PlayerFilter::IteratedPlayer,
            false,
            Some(
                ObjectFilter::default()
                    .in_zone(Zone::Hand)
                    .owned_by(PlayerFilter::IteratedPlayer)
            ),
        )),
        "that player discards all the cards in their hand"
    );
}

#[test]
pub(super) fn target_player_draw_life_loss_shared_count_uses_x_surface() {
    let artifact_count = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::You)
            .with_type(CardType::Artifact),
    );
    let effects = vec![
        Effect::new(crate::effects::DrawCardsEffect::new(
            artifact_count.clone(),
            PlayerFilter::target_player(),
        )),
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_player(),
        )),
        Effect::new(crate::effects::LoseLifeEffect::with_filter(
            artifact_count,
            PlayerFilter::target_player(),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target player draws X cards and loses X life, where X is the number of artifacts you control"
    );
}

#[test]
pub(super) fn choose_creature_type_target_player_draw_life_loss_shared_count_compacts() {
    let chosen_type_count = Value::Count(
        ObjectFilter::creature()
            .controlled_by(PlayerFilter::target_player())
            .of_chosen_creature_type()
            .in_zone(Zone::Battlefield),
    )
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let effects = vec![
        Effect::new(crate::effects::ChooseCreatureTypeEffect::new(
            PlayerFilter::You,
            Vec::new(),
        )),
        Effect::new(crate::effects::DrawCardsEffect::new(
            chosen_type_count.clone(),
            PlayerFilter::target_player(),
        )),
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_player(),
        )),
        Effect::new(crate::effects::LoseLifeEffect::with_filter(
            chosen_type_count,
            PlayerFilter::target_player(),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Choose a creature type. Target player draws X cards and loses X life, where X is the number of creatures they control of the chosen type"
    );
}

#[test]
pub(super) fn id_backed_destroy_count_consumers_render_this_way_surfaces() {
    let id = crate::effect::EffectId(17);
    let count = Value::EffectMetric {
        effect_id: id,
        source: crate::effect::EffectMetricSource::AffectedObjects,
        metric: crate::effect::EffectMetric::Count,
    };
    let mut tapped_creatures = ObjectFilter::creature().in_zone(Zone::Battlefield);
    tapped_creatures.tapped = true;
    let effects = vec![
        Effect::with_id(
            id.0,
            Effect::new(crate::effects::DestroyEffect::all(tapped_creatures)),
        ),
        Effect::new(crate::effects::GainLifeEffect::you(Value::Add(
            Box::new(count.clone()),
            Box::new(count),
        ))),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Destroy all tapped creatures. You gain 2 life for each creature destroyed this way"
    );
}

#[test]
pub(super) fn id_backed_target_discard_draw_renders_discarded_this_way() {
    let id = crate::effect::EffectId(23);
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_player(),
        )),
        Effect::with_id(
            id.0,
            Effect::new(crate::effects::DiscardEffect::new(
                2,
                PlayerFilter::target_player(),
                false,
            )),
        ),
        Effect::new(crate::effects::DrawCardsEffect::new(
            Value::EffectValue(id),
            PlayerFilter::target_player(),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target player discards two cards. Target player draws a card for each card discarded this way"
    );
}

#[test]
pub(super) fn id_backed_return_draw_renders_returned_to_hand_this_way() {
    let id = crate::effect::EffectId(24);
    let count = Value::PriorEffectMetric {
        effect_id: id,
        query: ironsmith_core::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::AffectedObjects,
            crate::effect::EffectMetric::Count,
        )
        .with_filter(ObjectFilter::default().owned_by(PlayerFilter::You))
        .with_action(ironsmith_core::PriorEffectAction::Returned),
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let effects = vec![
        Effect::with_id(
            id.0,
            Effect::new(crate::effects::ReturnToHandEffect::all(
                ObjectFilter::permanent().in_zone(Zone::Battlefield),
            )),
        ),
        Effect::new(crate::effects::DrawCardsEffect::you(count)),
    ];

    assert_eq!(
        describe_effect(&effects[1]),
        "you draw a card for each card returned to your hand this way"
    );
    assert_eq!(
        describe_effect_list(&effects),
        "Return all permanents to their owners' hands. You draw a card for each card returned to your hand this way"
    );
}

#[test]
pub(super) fn id_backed_graveyard_move_token_count_renders_put_this_way() {
    let id = crate::effect::EffectId(25);
    let count = Value::EffectMetric {
        effect_id: id,
        source: crate::effect::EffectMetricSource::Outcome,
        metric: crate::effect::EffectMetric::Count,
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let effects = vec![
        Effect::with_id(
            id.0,
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::all(ObjectFilter::default().in_zone(Zone::Exile)),
                Zone::Graveyard,
                false,
            )),
        ),
        Effect::new(crate::effects::CreateTokenEffect::new(
            crate::cards::tokens::treasure_token_definition(),
            count,
            PlayerFilter::You,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Put all cards in exile into its owner's graveyard. Create a Treasure token for each card put into a graveyard this way"
    );
}

#[test]
pub(super) fn id_backed_destroy_restricted_mana_renders_one_choice_per_result() {
    let id = crate::effect::EffectId(28);
    let count = Value::EffectMetric {
        effect_id: id,
        source: crate::effect::EffectMetricSource::AffectedObjects,
        metric: crate::effect::EffectMetric::Count,
    };
    let effects = vec![
        Effect::with_id(
            id.0,
            Effect::new(crate::effects::DestroyEffect::all(
                ObjectFilter::permanent().in_zone(Zone::Battlefield),
            )),
        ),
        Effect::new(crate::effects::AddManaOfAnyColorEffect::restricted(
            count,
            PlayerFilter::You,
            vec![crate::color::Color::Black, crate::color::Color::Green],
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Destroy all permanents. Add {B} or {G} for each permanent destroyed this way"
    );
}

#[test]
pub(super) fn restricted_mana_choice_renders_typed_prior_result_without_its_producer() {
    let id = crate::effect::EffectId(29);
    let amount = Value::PriorEffectMetric {
        effect_id: id,
        query: ironsmith_core::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::AffectedObjects,
            crate::effect::EffectMetric::Count,
        )
        .with_filter(ObjectFilter::permanent())
        .with_action(ironsmith_core::PriorEffectAction::Destroyed),
    };
    let add = Effect::new(crate::effects::AddManaOfAnyColorEffect::restricted(
        amount,
        PlayerFilter::You,
        vec![crate::color::Color::Black, crate::color::Color::Green],
    ));

    assert_eq!(
        describe_effect(&add),
        "Add {B} or {G} for each permanent destroyed this way"
    );
}

#[test]
pub(super) fn id_backed_discard_repeat_return_renders_one_return_per_discard() {
    let id = crate::effect::EffectId(31);
    let count = Value::EffectMetric {
        effect_id: id,
        source: crate::effect::EffectMetricSource::Outcome,
        metric: crate::effect::EffectMetric::Count,
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let effects = vec![
        Effect::with_id(
            id.0,
            Effect::new(crate::effects::DiscardEffect::new(
                Value::X,
                PlayerFilter::You,
                false,
            )),
        ),
        Effect::new(crate::effects::RepeatEffectsEffect::new(
            count,
            vec![Effect::new(crate::effects::ReturnToHandEffect::with_spec(
                ChooseSpec::Object(
                    ObjectFilter::default()
                        .in_zone(Zone::Graveyard)
                        .owned_by(PlayerFilter::You),
                )
                .with_count(ChoiceCount::exactly(1)),
            ))],
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "You discard X cards. Return a card from your graveyard to your hand for each card discarded this way"
    );
}

#[test]
pub(super) fn typed_repeat_once_hint_preserves_the_process_surface() {
    let repeated = Effect::new(crate::effects::RepeatEffectsEffect::new(
        Value::Fixed(2).with_surface_hint(ValueSurfaceHint::RepeatThisProcessOnce),
        vec![Effect::new(crate::effects::DrawCardsEffect::new(
            1,
            PlayerFilter::You,
        ))],
    ));

    assert_eq!(
        describe_effect(&repeated),
        "Draw a card. Repeat this process once"
    );
}

#[test]
pub(super) fn battlefield_choice_zone_is_implicit_when_stored_on_the_choice_effect() {
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::creature(),
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Battlefield),
    );

    assert_eq!(describe_effect(&choose), "You choose a creature");
}

#[test]
pub(super) fn id_backed_aggregate_damage_gain_renders_damage_dealt_this_way() {
    let id = crate::effect::EffectId(26);
    let producer = Effect::with_id(
        id.0,
        Effect::new(crate::effects::ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![Effect::new(crate::effects::DealDamageEffect::new(
                2,
                ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            ))],
        )),
    );
    let amount = Value::EffectValue(id)
        .with_surface_hint(ValueSurfaceHint::DamageDealt)
        .with_surface_hint(ValueSurfaceHint::EqualTo);
    let gain = Effect::new(crate::effects::GainLifeEffect::you(amount));

    assert_eq!(
        describe_effect_list(&[producer, gain]),
        "Deal 2 damage to each opponent and you gain life equal to the damage dealt this way"
    );
}

#[test]
pub(super) fn aggregate_life_loss_event_gain_renders_life_lost_this_way() {
    let producer = Effect::with_id(
        27,
        Effect::new(crate::effects::ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![Effect::new(crate::effects::LoseLifeEffect::with_filter(
                1,
                PlayerFilter::IteratedPlayer,
            ))],
        )),
    );
    let gain = Effect::new(crate::effects::GainLifeEffect::you(Value::EventValue(
        crate::effect::EventValueSpec::LifeAmount,
    )));

    assert_eq!(
        describe_effect_list(&[producer, gain]),
        "Each opponent loses 1 life. You gain life equal to the life lost this way"
    );
}

#[test]
pub(super) fn typed_life_and_damage_backrefs_render_without_their_producers() {
    let life_loss = Effect::new(crate::effects::GainLifeEffect::you(Value::EventValue(
        crate::effect::EventValueSpec::LifeAmount,
    )));
    assert_eq!(
        describe_effect(&life_loss),
        "you gain life equal to the life lost this way"
    );

    let damage = Effect::new(crate::effects::GainLifeEffect::you(
        Value::EffectValue(crate::effect::EffectId(30))
            .with_surface_hint(ValueSurfaceHint::DamageDealt)
            .with_surface_hint(ValueSurfaceHint::EqualTo),
    ));
    assert_eq!(
        describe_effect(&damage),
        "you gain life equal to the damage dealt this way"
    );
}

#[test]
pub(super) fn tagged_animation_counter_followup_renders_became_a_creature_this_way() {
    let tag = TagKey::from("animated_creature_0");
    let mut selected = ObjectFilter::artifact()
        .controlled_by(PlayerFilter::You)
        .in_zone(Zone::Battlefield);
    selected.excluded_card_types.push(CardType::Creature);
    let mut animation = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddCardTypes(vec![CardType::Artifact, CardType::Creature]),
        Until::Forever,
    );
    animation.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(selected)));
    let producer = Effect::new(animation).tag(tag.clone());

    let mut animated = ObjectFilter::artifact().in_zone(Zone::Battlefield);
    animated.card_types.push(CardType::Creature);
    animated.all_card_types = vec![CardType::Artifact, CardType::Creature];
    animated.tagged_constraints.push(TaggedObjectConstraint {
        tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let consumer = Effect::new(crate::effects::ForEachObject::new(
        animated,
        vec![Effect::new(crate::effects::PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            4,
            ChooseSpec::Iterated,
        ))],
    ));

    assert_eq!(
        describe_effect(&consumer),
        "Put four +1/+1 counters on each artifact that became a creature this way"
    );
    assert_eq!(
        describe_effect_list(&[producer, consumer]),
        "Target noncreature artifact you control becomes an artifact creature. Put four +1/+1 counters on each artifact that became a creature this way"
    );
}

#[test]
pub(super) fn tagged_counter_goad_followup_renders_exact_countered_result() {
    let tag = TagKey::from("counters_0");
    let amount = Value::Fixed(1).with_surface_hint(ValueSurfaceHint::CounterFollowupThen);
    let producer = Effect::new(crate::effects::PutCountersEffect::new(
        CounterType::DoubleStrike,
        amount,
        ChooseSpec::target_creature(),
    ))
    .tag(tag.clone());

    let mut countered = ObjectFilter::creature().in_zone(Zone::Battlefield);
    countered.tagged_constraints.push(TaggedObjectConstraint {
        tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let consumer = Effect::new(crate::effects::GoadEffect::new(ChooseSpec::all(countered)));

    assert_eq!(
        describe_effect(&consumer),
        "Goad each creature that had counters put on it this way"
    );
    assert_eq!(
        describe_effect_list(&[producer, consumer]),
        "Put a double strike counter on target creature, then goad each creature that had a double strike counter put on it this way"
    );
}

#[test]
pub(super) fn each_player_exile_sacrifice_return_uses_exact_exiled_result() {
    let tag = TagKey::from("exiled_this_way_0");
    let exiled_filter = ObjectFilter::artifact()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::IteratedPlayer);
    let exile = Effect::new(crate::effects::ExileEffect::all(exiled_filter)).tag(tag.clone());

    let sacrificed_filter = ObjectFilter::artifact()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::IteratedPlayer);
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        sacrificed_filter.clone(),
        Value::Count(sacrificed_filter),
        PlayerFilter::IteratedPlayer,
    ));

    let returned = Effect::new(crate::effects::PutOntoBattlefieldEffect::new(
        ChooseSpec::Tagged(tag),
        false,
        PlayerFilter::IteratedPlayer,
    ))
    .tag(TagKey::from("moved_1"));
    let per_player = Effect::for_players(PlayerFilter::Any, vec![exile, sacrifice, returned]);

    assert_eq!(
        describe_effect_list(&[per_player]),
        "Each player exiles all artifact cards from their graveyard, then sacrifices all artifacts they control, then puts all cards they exiled this way onto the battlefield"
    );
}

#[test]
pub(super) fn id_backed_consumer_rejects_ambiguous_or_filtered_shapes() {
    let first_id = crate::effect::EffectId(29);
    let second_id = crate::effect::EffectId(30);
    let destroy = |id| {
        Effect::with_id(
            id,
            Effect::new(crate::effects::DestroyEffect::all(
                ObjectFilter::creature().in_zone(Zone::Battlefield),
            )),
        )
    };
    let draw = Effect::new(crate::effects::DrawCardsEffect::you(Value::EffectMetric {
        effect_id: second_id,
        source: crate::effect::EffectMetricSource::AffectedObjects,
        metric: crate::effect::EffectMetric::Count,
    }));
    assert!(
        describe_id_backed_prior_action_count_consumer(&[
            destroy(first_id.0),
            destroy(second_id.0),
            draw,
        ])
        .is_none()
    );

    let discard = Effect::with_id(
        first_id.0,
        Effect::new(crate::effects::DiscardEffect::new(
            2,
            PlayerFilter::target_player(),
            false,
        )),
    );
    let gain = Effect::new(crate::effects::GainLifeEffect::you(Value::EffectMetric {
        effect_id: first_id,
        source: crate::effect::EffectMetricSource::Outcome,
        metric: crate::effect::EffectMetric::Count,
    }));
    assert!(describe_id_backed_prior_action_count_consumer(&[discard, gain]).is_none());
}

#[test]
pub(super) fn target_player_fixed_draw_life_loss_pairs_render_as_single_clause() {
    let target_effects = vec![
        Effect::new(crate::effects::DrawCardsEffect::new(
            Value::Fixed(1),
            PlayerFilter::target_player(),
        )),
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_player(),
        )),
        Effect::new(crate::effects::LoseLifeEffect::with_filter(
            Value::Fixed(1),
            PlayerFilter::target_player(),
        )),
    ];

    assert_eq!(
        describe_effect_list(&target_effects),
        "Target player draws a card and loses 1 life"
    );
}

#[test]
pub(super) fn destroy_then_draw_lose_shared_counter_count_compacts() {
    let counter_count =
        Value::CountersOn(Box::new(ChooseSpec::Tagged(TagKey::from("__it__"))), None)
            .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let effects = vec![
        Effect::destroy(ChooseSpec::target_creature()).tag("destroyed_0"),
        Effect::new(crate::effects::DrawCardsEffect::new(
            counter_count.clone(),
            PlayerFilter::You,
        )),
        Effect::new(crate::effects::LoseLifeEffect::with_filter(
            counter_count,
            PlayerFilter::You,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Destroy target creature. You draw X cards and you lose X life, where X is the number of counters on it"
    );
}

#[test]
pub(super) fn target_only_exchange_control_renders_selected_permanents() {
    let mut target_filter = ObjectFilter::permanent();
    let chooser =
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(TagKey::from("triggering")));
    target_filter.controller = Some(PlayerFilter::excluding(PlayerFilter::Any, chooser.clone()));
    target_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from("triggering"),
            relation: TaggedOpbjectRelation::SharesCardType,
        });
    let target = ChooseSpec::target(ChooseSpec::Object(target_filter));

    let mut selected_filter = ObjectFilter::permanent();
    selected_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from("__it__"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let selected =
        ChooseSpec::target(ChooseSpec::Object(selected_filter)).with_count(ChoiceCount::exactly(2));
    let exchange = crate::effects::ExchangeControlEffect::new(selected.clone(), selected);

    let effects = vec![
        Effect::new(crate::effects::TagTriggeringObjectEffect::new("triggering")),
        Effect::new(crate::effects::TargetOnlyEffect::explicit(target).with_chooser(chooser)),
        Effect::new(exchange).tag("exchanged_0"),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Its controller chooses target permanent another player controls that shares a card type with it. Exchange control of those permanents"
    );

    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(effects[..2].to_vec()),
        crate::resolution::ResolutionSegment::from_effects(effects[2..].to_vec()),
    ]);
    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "Its controller chooses target permanent another player controls that shares a card type with it. Exchange control of those permanents"
    );
}

#[test]
pub(super) fn target_most_common_color_conditional_return_renders_as_if_clause() {
    let target_tag = TagKey::from("targeted_0");
    let target_spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::permanent()));
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()))
        .tag(target_tag.clone());

    let mut condition_filter = ObjectFilter::permanent();
    condition_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from("most_common_permanent_color"),
            relation: TaggedOpbjectRelation::SharesMostCommonPermanentColor,
        });
    let return_effect =
        Effect::new(crate::effects::ReturnToHandEffect::with_spec(target_spec)).tag("returned_0");
    let conditional = Effect::conditional_only(
        Condition::TaggedObjectMatches(target_tag, condition_filter),
        vec![return_effect],
    );

    assert_eq!(
        describe_effect_list(&[target, conditional]),
        "Return target permanent to its owner's hand if that permanent shares a color with the most common color among all permanents or a color tied for most common"
    );
}

#[test]
pub(super) fn target_then_cant_untap_renders_single_target_sentence() {
    let target_tag = TagKey::from("targeted_0");
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
        ChooseSpec::Object(ObjectFilter::permanent()),
    )))
    .tag(target_tag.clone());

    let mut filter = ObjectFilter::permanent();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: target_tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let cant = Effect::cant_until(
        crate::effect::Restriction::Untap(filter),
        Until::ControllersNextUntapStep,
    );

    assert_eq!(
        describe_effect_list(&[target, cant]),
        "Target permanent doesn't untap during its controller's next untap step"
    );
}

#[test]
pub(super) fn target_then_must_be_blocked_renders_single_target_sentence() {
    let target_tag = TagKey::from("targeted_0");
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
        ChooseSpec::Object(ObjectFilter::creature()),
    )))
    .tag(target_tag.clone());

    let mut filter = ObjectFilter::creature();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: target_tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let cant = Effect::cant_until(
        crate::effect::Restriction::must_be_blocked(filter),
        Until::EndOfTurn,
    );

    assert_eq!(
        describe_effect_list(&[target, cant]),
        "Target creature must be blocked this turn if able"
    );
}

#[test]
pub(super) fn coordinated_target_pump_then_must_be_blocked_keeps_and_surface() {
    let mut pump = crate::effects::ApplyContinuousEffect::new_runtime(
        crate::continuous::EffectTarget::Source,
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
            power: Value::Fixed(3),
            toughness: Value::Fixed(3),
        },
        Until::EndOfTurn,
    );
    pump.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature().in_zone(Zone::Battlefield),
    )));
    pump.require_creature_target = true;
    let cant = Effect::cant_until(
        crate::effect::Restriction::must_be_blocked(ObjectFilter::tagged(TagKey::from("__it__"))),
        Until::EndOfTurn,
    );
    let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
        Effect::new(pump).tag(TagKey::from("__it__")),
        cant,
    ]));

    assert_eq!(
        describe_effect(&sequence),
        "Target creature gets +3/+3 until end of turn and must be blocked this turn if able"
    );
}

#[test]
pub(super) fn tap_it_then_cant_untap_keeps_it_reference() {
    let tag = TagKey::from("__it__");
    let tap = Effect::tap(ChooseSpec::Tagged(tag.clone()));
    let mut filter = ObjectFilter::permanent();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let cant = Effect::cant_until(
        crate::effect::Restriction::Untap(filter),
        Until::ControllersNextUntapStep,
    );

    assert_eq!(
        describe_effect_list(&[tap, cant]),
        "Tap it. It doesn't untap during its controller's next untap step"
    );
}

#[test]
pub(super) fn tagged_blocked_set_tap_then_cant_untap_keeps_distributive_reference() {
    let selected = TagKey::from("selected_creatures");
    let tapped = TagKey::from("tapped_creatures");
    let mut blocked = ObjectFilter::creature();
    blocked.blocked = true;
    blocked.blocked_by = Some(crate::filter::ObjectRef::Tagged(selected));

    let tag = Effect::new(crate::effects::TagMatchingObjectsEffect::new(
        blocked.clone(),
        tapped.clone(),
    ));
    let tap = Effect::tap(ChooseSpec::All(blocked));
    let cant = Effect::cant_until(
        crate::effect::Restriction::Untap(ObjectFilter::tagged(tapped)),
        Until::ControllersNextUntapStep,
    );

    assert_eq!(
        describe_effect_list(&[tag, tap, cant]),
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step"
    );
}

#[test]
pub(super) fn target_player_life_loss_you_gain_shared_greatest_power_uses_x_surface() {
    let greatest_power =
        Value::GreatestPower(ObjectFilter::creature().controlled_by(PlayerFilter::You));
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_player(),
        )),
        Effect::new(crate::effects::LoseLifeEffect::with_filter(
            greatest_power.clone(),
            PlayerFilter::target_player(),
        )),
        Effect::new(crate::effects::GainLifeEffect::you(greatest_power)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target player loses X life and you gain X life, where X is the greatest power among creatures you control"
    );
}

#[test]
pub(super) fn tagged_damage_then_gain_life_shared_count_uses_x_surface() {
    let swamp_count = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::You)
            .with_subtype(Subtype::Swamp),
    );
    let effects = vec![
        Effect::new(crate::effects::DealDamageEffect::new(
            swamp_count.clone(),
            ChooseSpec::target_creature(),
        ))
        .tag(TagKey::from("damaged_0")),
        Effect::new(crate::effects::GainLifeEffect::you(swamp_count)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Deal X damage to target creature and you gain X life, where X is the number of Swamps you control"
    );
}

#[test]
pub(super) fn player_or_planeswalker_damage_then_controlled_creature_damage_uses_planeswalker_surface()
 {
    let mut controlled_creatures = ObjectFilter::creature();
    controlled_creatures.controller = Some(PlayerFilter::TargetPlayerOrControllerOfTarget);
    let effects = vec![
        Effect::deal_damage(
            Value::Fixed(10),
            ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any),
        ),
        Effect::new(crate::effects::ForEachObject::new(
            controlled_creatures,
            vec![
                Effect::deal_damage(Value::Fixed(10), ChooseSpec::Iterated)
                    .tag(TagKey::from("damaged_0")),
            ],
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Deal 10 damage to target player or planeswalker and each creature that player or that planeswalker's controller controls"
    );
}

#[test]
pub(super) fn coordinated_player_or_planeswalker_damage_preserves_distinct_fanout_amount() {
    let mut controlled_creatures = ObjectFilter::creature();
    controlled_creatures.controller = Some(PlayerFilter::TargetPlayerOrControllerOfTarget);
    let coordinated = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
        Effect::deal_damage(
            Value::Fixed(4),
            ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any),
        ),
        Effect::new(crate::effects::ForEachObject::new(
            controlled_creatures,
            vec![
                Effect::deal_damage(Value::Fixed(1), ChooseSpec::Iterated)
                    .tag(TagKey::from("damaged_0")),
            ],
        )),
    ]));

    assert_eq!(
        describe_effect_list(&[coordinated]),
        "Deal 4 damage to target player or planeswalker and 1 damage to each creature that player or that planeswalker's controller controls"
    );
}

#[test]
pub(super) fn player_chosen_attachment_renders_exact_actor_and_choice_provenance() {
    let tag = TagKey::from("attachment_target_0");
    let chooser = PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(
        TagKey::from("destroyed_0"),
    ));
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::land().in_zone(Zone::Battlefield),
            ChoiceCount::exactly(1),
            chooser,
            tag.clone(),
        )
        .in_zone(Zone::Battlefield),
    );
    let attach = Effect::attach_objects(ChooseSpec::Source, ChooseSpec::Tagged(tag));

    assert_eq!(
        describe_effect_list(&[choose, attach]),
        "That player attaches this source to a land of their choice"
    );
}

#[test]
pub(super) fn tapped_permanent_trigger_uses_typed_noun_for_triggering_objects_controller() {
    let triggering = TagKey::from("triggering");
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::permanent_becomes_tapped(ObjectFilter::land()),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::deal_damage(
            Value::Fixed(1),
            ChooseSpec::Player(PlayerFilter::ControllerOf(
                crate::target::ObjectRef::Tagged(triggering),
            )),
        )]),
        choices: vec![],
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        rewrite_typed_triggering_object_player_reference(
            &triggered,
            "this Aura deals 1 damage to that creature's controller".to_string(),
        ),
        "this Aura deals 1 damage to that land's controller"
    );
}

#[test]
pub(super) fn tagged_damage_then_gain_life_additive_count_uses_x_surface() {
    let named_count = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .named("Feast of Flesh"),
    );
    let amount = Value::Add(Box::new(Value::Fixed(1)), Box::new(named_count));
    let effects = vec![
        Effect::new(crate::effects::DealDamageEffect::new(
            amount.clone(),
            ChooseSpec::target_creature(),
        ))
        .tag(TagKey::from("damaged_0")),
        Effect::new(crate::effects::GainLifeEffect::you(amount)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Deal X damage to target creature and you gain X life, where X is 1 plus the number of cards named Feast of Flesh in all graveyards"
    );
}

#[test]
pub(super) fn for_each_opponent_life_loss_you_gain_shared_count_uses_x_surface() {
    let party = Value::PartySize(PlayerFilter::You);
    let effects = vec![
        Effect::new(crate::effects::ForPlayersEffect {
            filter: PlayerFilter::Opponent,
            effects: vec![Effect::new(crate::effects::LoseLifeEffect::with_filter(
                party.clone(),
                PlayerFilter::IteratedPlayer,
            ))],
            starting_with_controller: false,
            stop_after_first_happened: false,
        }),
        Effect::new(crate::effects::GainLifeEffect::you(party)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Each opponent loses X life and you gain X life, where X is the number of creatures in your party"
    );
}

#[test]
pub(super) fn dynamic_mill_count_uses_x_where_surface() {
    let land_count = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::You)
            .with_type(CardType::Land),
    )
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let effect = Effect::new(crate::effects::MillEffect::new(
        land_count,
        PlayerFilter::target_player(),
    ));

    assert_eq!(
        describe_effect(&effect),
        "target player mills X cards, where X is the number of lands you control"
    );
}

#[test]
pub(super) fn damaged_player_reveal_choose_graveyard_compacts_revealed_card_choice() {
    let revealed = TagKey::from("revealed_cards");
    let chosen = TagKey::from("__it__");
    let mut choose_filter = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::DamagedPlayer);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: revealed.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let choose = crate::effects::ChooseObjectsEffect::new(
        choose_filter,
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        chosen,
    )
    .in_zones(vec![
        Zone::Battlefield,
        Zone::Hand,
        Zone::Graveyard,
        Zone::Library,
        Zone::Exile,
    ]);
    let effects = vec![
        Effect::new(crate::effects::TagTriggeringObjectEffect::new("triggering")),
        Effect::new(crate::effects::LookAtTopCardsEffect::revealing(
            PlayerFilter::DamagedPlayer,
            Value::Fixed(2),
            revealed,
        )),
        Effect::new(choose),
        Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Graveyard,
            false,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "The damaged player reveals the top two cards of the damaged player's library. You choose one of those cards and put it into the damaged player's graveyard"
    );
}

#[test]
pub(super) fn block_specific_attacker_renders_cant_be_blocked_by_filter() {
    let targeted = TagKey::from("targeted_0");
    let tagged = Effect::new(crate::effects::TargetOnlyEffect::new(
        ChooseSpec::target_creature(),
    ))
    .tag(targeted.clone());
    let mut attacker = ObjectFilter::creature();
    attacker.tagged_constraints.push(TaggedObjectConstraint {
        tag: targeted,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let cant = Effect::new(crate::effects::CantEffect::until_end_of_turn(
        crate::effect::Restriction::BlockSpecificAttacker {
            blockers: ObjectFilter::default()
                .in_zone(Zone::Battlefield)
                .with_subtype(Subtype::Wall),
            attacker,
        },
    ));

    assert_eq!(
        describe_effect_list(&[tagged, cant]),
        "Target creature can't be blocked by Walls this turn"
    );
}

#[test]
pub(super) fn random_choose_then_destroy_all_others_renders_rest_surface() {
    let tag = TagKey::from("__it__");
    let choose = Effect::new(crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature(),
        ChoiceCount::exactly(1).at_random(),
        PlayerFilter::You,
        tag.clone(),
    ));
    let destroy_filter = ObjectFilter::creature().not_tagged(tag);
    let destroy = Effect::new(crate::effects::DestroyEffect::with_spec(ChooseSpec::All(
        destroy_filter,
    )));

    assert_eq!(
        describe_effect_list(&[choose, destroy]),
        "Choose a creature at random, then destroy the rest"
    );
}

#[test]
pub(super) fn up_to_one_choose_then_destroy_all_others_renders_rest_surface() {
    let tag = TagKey::from("__it__");
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::creature().in_zone(Zone::Battlefield),
            ChoiceCount::up_to(1),
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Battlefield),
    );
    let destroy_filter = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .not_tagged(tag);
    let destroy = Effect::new(crate::effects::DestroyEffect::with_spec(ChooseSpec::All(
        destroy_filter,
    )));

    assert_eq!(
        describe_effect_list(&[choose, destroy]),
        "Choose up to one creature. Destroy the rest"
    );
}

#[test]
pub(super) fn destroy_all_plain_type_union_uses_inclusive_and_surface() {
    let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    filter.card_types = vec![CardType::Artifact, CardType::Enchantment];
    let destroy = Effect::new(crate::effects::DestroyEffect::all(filter));

    assert_eq!(
        describe_effect(&destroy),
        "Destroy all artifacts and enchantments"
    );

    let mut union = ObjectFilter::default();
    union.any_of = vec![
        ObjectFilter::artifact().in_zone(Zone::Battlefield),
        ObjectFilter::enchantment().in_zone(Zone::Battlefield),
    ];
    let destroy = Effect::new(crate::effects::DestroyEffect::all(union));
    assert_eq!(
        describe_effect(&destroy),
        "Destroy all artifacts and enchantments"
    );
}

#[test]
pub(super) fn cant_block_power_toughness_relation_uses_each_subject() {
    let filter = ObjectFilter::creature().with_power_toughness_relation(
        crate::filter::PowerToughnessRelation::ToughnessGreaterThanPower,
    );
    let cant = Effect::new(crate::effects::CantEffect::until_end_of_turn(
        crate::effect::Restriction::Block(filter),
    ));

    assert_eq!(
        describe_effect_list(&[cant]),
        "Each creature with toughness greater than its power can't block this turn"
    );
}

#[test]
pub(super) fn pluralize_power_toughness_relation_uses_each_have_surface() {
    assert_eq!(
        pluralize_noun_phrase("creature with toughness greater than its power"),
        "creatures that each have toughness greater than their power"
    );
    assert_eq!(
        pluralize_noun_phrase("creature with power greater than its toughness"),
        "creatures that each have power greater than their toughness"
    );
}

#[test]
pub(super) fn pluralize_creation_provenance_qualifies_the_noun() {
    assert_eq!(
        pluralize_noun_phrase("token created with this enchantment"),
        "tokens created with this enchantment"
    );
}

#[test]
pub(super) fn pluralize_battlefield_location_qualifies_the_noun() {
    assert_eq!(
        pluralize_noun_phrase("green creature on the battlefield"),
        "green creatures on the battlefield"
    );
}

#[test]
pub(super) fn pluralize_and_or_unions_pluralizes_each_independent_noun() {
    assert_eq!(
        pluralize_noun_phrase("artifact and/or creature"),
        "artifacts and/or creatures"
    );
    assert_eq!(
        pluralize_noun_phrase("target creature and/or planeswalker an opponent controls"),
        "target creatures and/or planeswalkers an opponent controls"
    );
    assert_eq!(
        pluralize_noun_phrase("card in your hand and/or card in your graveyard"),
        "cards in your hand and/or cards in your graveyard"
    );
    assert_eq!(
        pluralize_noun_phrase("Assassin, Pirate, and/or Vehicle"),
        "Assassins, Pirates, and/or Vehicles"
    );
}

#[test]
pub(super) fn pluralize_and_or_unions_preserves_a_shared_terminal_noun() {
    assert_eq!(
        pluralize_noun_phrase("instant and/or sorcery card"),
        "instant and/or sorcery cards"
    );
    assert_eq!(
        pluralize_noun_phrase("artifact and/or creature card in your graveyard"),
        "artifact and/or creature cards in your graveyard"
    );
    assert_eq!(
        pluralize_noun_phrase("target creature and/or planeswalker card in a graveyard"),
        "target creature and/or planeswalker cards in a graveyard"
    );
    assert_eq!(
        pluralize_noun_phrase("instant, sorcery, and/or enchantment card"),
        "instant, sorcery, and/or enchantment cards"
    );
}

#[test]
pub(super) fn pluralize_relative_union_conjugates_and_pluralizes_members() {
    assert_eq!(
        pluralize_noun_phrase("a creature you control that's a Fungus and/or Saproling"),
        "creatures you control that are Fungi and/or Saprolings"
    );
    assert_eq!(
        pluralize_noun_phrase("a creature you control that's a Zombie and/or token"),
        "creatures you control that are Zombies and/or tokens"
    );
}

#[test]
pub(super) fn pluralize_conjunctive_subtype_set_pluralizes_each_member() {
    assert_eq!(
        pluralize_noun_phrase("a Plant and Treefolk you control"),
        "Plants and Treefolk you control"
    );
}

#[test]
pub(super) fn pluralize_conjunctive_modifiers_preserves_the_shared_terminal_noun() {
    assert_eq!(
        pluralize_noun_phrase("red instant and sorcery spell you control"),
        "red instant and sorcery spells you control"
    );
}

#[test]
pub(super) fn blocks_or_becomes_blocked_preserves_one_or_more_colored_creature_surface() {
    let mut blocker = ObjectFilter::creature()
        .with_colors(crate::color::ColorSet::BLUE.union(crate::color::ColorSet::BLACK));
    blocker.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
    blocker.set_union_one_or_more(true);
    let trigger = crate::triggers::Trigger::either(
        crate::triggers::Trigger::this_blocks_object(blocker.clone()),
        crate::triggers::Trigger::this_becomes_blocked_by_object(blocker),
    );

    assert_eq!(
        describe_this_blocks_or_becomes_blocked_by_trigger(&trigger).as_deref(),
        Some(
            "Whenever this creature blocks or becomes blocked by one or more blue and/or black creatures"
        )
    );
}

#[test]
pub(super) fn choose_color_as_enters_uses_oracle_static_surface() {
    let choose_any = crate::static_abilities::StaticAbility::choose_color_as_enters(
        None,
        "Choose color as enters.".to_string(),
    );
    assert_eq!(
        describe_static_ability_with_subject(&choose_any, "this artifact"),
        "As this artifact enters, choose a color"
    );

    let choose_except = crate::static_abilities::StaticAbility::choose_color_as_enters(
        Some(crate::color::Color::Blue),
        "Choose color as enters.".to_string(),
    );
    assert_eq!(
        describe_static_ability_with_subject(&choose_except, "this land"),
        "As this land enters, choose a color other than blue"
    );
}

#[test]
pub(super) fn attached_keyword_grant_keeps_the_attachment_subject() {
    let double_strike = crate::static_abilities::StaticAbility::attached_ability_grant(
        Ability::static_ability(crate::static_abilities::StaticAbility::double_strike()),
        "Double strike".to_string(),
    );
    assert_eq!(
        describe_static_ability_with_subject(&double_strike, "this Equipment"),
        "Equipped creature has double strike"
    );

    let unblockable = crate::static_abilities::StaticAbility::attached_ability_grant(
        Ability::static_ability(crate::static_abilities::StaticAbility::unblockable()),
        "This can't be blocked".to_string(),
    );
    assert_eq!(
        describe_static_ability_with_subject(&unblockable, "this Equipment"),
        "Equipped creature can't be blocked"
    );
}

#[test]
pub(super) fn describe_choose_then_sacrifice_compacts_any_number_costs() {
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature(),
        ChoiceCount::any_number(),
        PlayerFilter::You,
        TagKey::from("sacrificed_0"),
    )
    .in_zone(Zone::Battlefield);

    let mut sacrifice_filter = ObjectFilter::creature();
    sacrifice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from("sacrificed_0"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let sacrifice = Effect::new(crate::effects::SacrificeEffect::player(
        sacrifice_filter.clone(),
        Value::Count(sacrifice_filter),
        PlayerFilter::You,
    ));

    let compact = describe_choose_then_sacrifice(
        &choose,
        sacrifice_view(&sacrifice).expect("sacrifice view"),
    )
    .expect("any-number sacrifice should compact");
    assert_eq!(
        normalize_cost_phrase(&compact),
        "Sacrifice any number of creatures"
    );
}

#[test]
pub(super) fn resolution_program_compacts_tracked_any_number_sacrifice() {
    // Production lowering assigns IDs to both effects: the sacrifice chooses
    // from `__it__`, then the following segment reads the sacrifice outcome.
    let tag = TagKey::from("__it__");
    let choose = Effect::with_id(
        0,
        Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default()
                    .with_subtype(Subtype::Mountain)
                    .you_control(),
                ChoiceCount::any_number(),
                PlayerFilter::You,
                tag.clone(),
            )
            .in_zone(Zone::Battlefield),
        ),
    );
    let sacrifice_filter = ObjectFilter::tagged(tag);
    let sacrifice = Effect::with_id(
        1,
        Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
            sacrifice_filter.clone(),
            Value::Count(sacrifice_filter),
            PlayerFilter::You,
        )),
    );
    let damage = Effect::deal_damage(
        Value::EffectValue(crate::effect::EffectId(1)),
        ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any),
    );
    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![choose, sacrifice]),
        crate::resolution::ResolutionSegment::from_effects(vec![damage]),
    ]);

    let rendered = super::super::ast_render::describe_resolution_program(&program);
    assert!(
        rendered.starts_with("Sacrifice any number of Mountains"),
        "{rendered}"
    );
    assert!(!rendered.contains("choose any number"), "{rendered}");
    assert!(!rendered.contains("sacrifice all permanents"), "{rendered}");
    assert!(rendered.contains("that much damage"), "{rendered}");
}

#[test]
pub(super) fn cost_object_value_and_battlefield_exile_use_oracle_surfaces() {
    let tag = TagKey::from("exile_cost_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature().you_control(),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let exile = crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(tag));
    assert_eq!(
        describe_choose_then_exile(&choose, &exile).as_deref(),
        Some("you exile a creature you control")
    );

    let red_symbols = Value::ManaSymbolsInManaCostOf {
        spec: Box::new(ChooseSpec::Tagged(TagKey::from("sacrifice_cost_0"))),
        color: crate::color::Color::Red,
    }
    .with_surface_hint(ValueSurfaceHint::SacrificedObject(
        crate::target::SacrificedObjectKind::Creature,
    ));
    assert_eq!(
        describe_value(&red_symbols),
        "the number of red mana symbols in the sacrificed creature's mana cost"
    );
}

#[test]
pub(super) fn one_or_more_graveyard_exile_preserves_choice_and_target_surfaces() {
    let graveyard_creature = ObjectFilter::creature()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    let choice =
        ChooseSpec::Object(graveyard_creature.clone()).with_count(ChoiceCount::at_least(1));
    assert!(!choice.is_target());
    assert_eq!(choice.count(), ChoiceCount::at_least(1));
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::ExileEffect::with_spec(choice))),
        "Exile one or more creature cards from your graveyard"
    );

    let targets = ChooseSpec::target(ChooseSpec::Object(graveyard_creature))
        .with_count(ChoiceCount::at_least(1));
    assert!(targets.is_target());
    assert_eq!(targets.count(), ChoiceCount::at_least(1));
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::ExileEffect::with_spec(
            targets
        ))),
        "Exile one or more target creature cards from your graveyard"
    );
}

#[test]
pub(super) fn graveyard_exile_renders_one_card_per_card_type_selection() {
    let mut filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::Defending);
    filter.one_per_card_type = true;
    let choice = ChooseSpec::Object(filter).with_count(ChoiceCount::any_number());

    assert_eq!(
        describe_effect(&Effect::new(crate::effects::ExileEffect::with_spec(choice))),
        "Exile up to one card of each card type from defending player's graveyard"
    );
}

#[test]
pub(super) fn tagged_graveyard_exile_preserves_random_choice_surface() {
    let graveyard_cards = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    let choice =
        ChooseSpec::Object(graveyard_cards).with_count(ChoiceCount::exactly(3).at_random());
    let exile = Effect::new(crate::effects::TaggedEffect::new(
        TagKey::from("exiled_cards"),
        Effect::new(crate::effects::ExileEffect::with_spec(choice)),
    ));

    assert_eq!(
        describe_effect(&exile),
        "Exile three cards at random from your graveyard"
    );
}

#[test]
pub(super) fn graveyard_exile_surfaces_distinguish_single_plural_and_all_scopes() {
    let single_graveyard = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .single_graveyard();
    let up_to_two_single =
        ChooseSpec::target(ChooseSpec::Object(single_graveyard)).with_count(ChoiceCount::up_to(2));
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::ExileEffect::with_spec(
            up_to_two_single
        ))),
        "Exile up to two target cards from a single graveyard"
    );

    let any_graveyards = ObjectFilter::default().in_zone(Zone::Graveyard);
    let up_to_two_any =
        ChooseSpec::target(ChooseSpec::Object(any_graveyards)).with_count(ChoiceCount::up_to(2));
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::ExileEffect::with_spec(
            up_to_two_any
        ))),
        "Exile up to two target cards from graveyards"
    );

    let mut all_graveyards = ObjectFilter::creature().in_zone(Zone::Graveyard);
    all_graveyards.entered_graveyard_from_battlefield_this_turn = true;
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::ExileEffect::with_spec(
            ChooseSpec::All(all_graveyards)
        ))),
        "Exile all creature cards in all graveyards that were put there from the battlefield this turn"
    );
}

#[test]
pub(super) fn describe_choose_then_sacrifice_compacts_sacrifice_player_effect() {
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        ChoiceCount::exactly(1),
        PlayerFilter::Opponent,
        TagKey::from("sacrificed_0"),
    )
    .in_zone(Zone::Battlefield);

    let mut sacrifice_filter = ObjectFilter::creature();
    sacrifice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from("sacrificed_0"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        sacrifice_filter,
        Value::Fixed(1),
        PlayerFilter::Opponent,
    ));

    let compact = describe_choose_then_sacrifice(
        &choose,
        sacrifice_view(&sacrifice).expect("sacrifice-player view"),
    )
    .expect("sacrifice-player effect should compact");
    assert_eq!(compact, "an opponent sacrifices a creature of their choice");
}

#[test]
pub(super) fn describe_choose_then_sacrifice_elides_your_battlefield_controller() {
    let tag = TagKey::from("sacrificed_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent()
            .controlled_by(PlayerFilter::You)
            .in_zone(Zone::Battlefield),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);

    let sacrifice = Effect::sacrifice_player(ObjectFilter::tagged(tag), 1, PlayerFilter::You);
    let compact = describe_choose_then_sacrifice(
        &choose,
        sacrifice_view(&sacrifice).expect("sacrifice effect should compact"),
    )
    .expect("your sacrifice effect should compact");

    assert_eq!(compact, "you sacrifice a permanent");
}

#[test]
pub(super) fn describe_choose_then_sacrifice_compacts_up_to_counted_tagged_set() {
    let tag = TagKey::from("sacrificed_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .with_subtype(Subtype::Zombie)
            .controlled_by(PlayerFilter::You)
            .in_zone(Zone::Battlefield),
        ChoiceCount::up_to(3),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);

    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(tag.clone()),
        Value::Count(ObjectFilter::tagged(tag)),
        PlayerFilter::You,
    ));

    let compact = describe_choose_then_sacrifice(
        &choose,
        sacrifice_view(&sacrifice).expect("sacrifice-player view"),
    )
    .expect("up-to tagged sacrifice should compact");

    assert_eq!(compact, "you sacrifice up to three Zombies");
}

#[test]
pub(super) fn describe_choose_then_sacrifice_matches_controller_followup_alias() {
    let tag = TagKey::from("sacrificed_amount");
    let chooser = PlayerFilter::ControllerOf(crate::target::ObjectRef::Target);
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent()
            .controlled_by(chooser.clone())
            .in_zone(Zone::Battlefield),
        ChoiceCount::dynamic_x(),
        chooser,
        tag.clone(),
    )
    .with_count_value(Value::EffectValue(crate::effect::EffectId(7)))
    .in_zone(Zone::Battlefield);
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(tag.clone()),
        Value::Count(ObjectFilter::tagged(tag)),
        PlayerFilter::AliasedControllerOf(crate::target::ObjectRef::Target),
    ));

    let compact = describe_choose_then_sacrifice(
        &choose,
        sacrifice_view(&sacrifice).expect("sacrifice-player view"),
    )
    .expect("controller follow-up alias should preserve the chosen set");

    assert_eq!(
        compact,
        "its controller sacrifices that many permanents of their choice"
    );
}

#[test]
pub(super) fn describe_choose_then_sacrifice_compacts_exact_sentence_helper_set() {
    let tag = TagKey::from("__sentence_helper_sacrificed_l1_s4_e35");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent().in_zone(Zone::Battlefield),
        ChoiceCount::exactly(7),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(tag.clone()),
        Value::Count(ObjectFilter::tagged(tag)),
        PlayerFilter::TargetPlayerOrControllerOfTarget,
    ));

    let compact = describe_choose_then_sacrifice(
        &choose,
        sacrifice_view(&sacrifice).expect("sacrifice-player view"),
    )
    .expect("exact sentence-helper chosen set should compact");

    assert_eq!(
        compact,
        "that player or that object's controller sacrifices seven permanents of their choice"
    );
}

#[test]
pub(super) fn describe_choose_then_sacrifice_preserves_aliased_target_actor() {
    let tag = TagKey::from("__sentence_helper_sacrificed_l1_s4_e36");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent().in_zone(Zone::Battlefield),
        ChoiceCount::exactly(7),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(tag.clone()),
        Value::Count(ObjectFilter::tagged(tag)),
        PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)),
    ));

    let compact = describe_choose_then_sacrifice(
        &choose,
        sacrifice_view(&sacrifice).expect("sacrifice-player view"),
    )
    .expect("aliased target sentence-helper chosen set should compact");

    assert_eq!(
        compact,
        "that player sacrifices seven permanents of their choice"
    );
}

#[test]
pub(super) fn describe_choose_then_sacrifice_keeps_implicit_you_for_generated_cost_set() {
    let tag = TagKey::from("__sentence_helper_sacrificed_l2_s0_e42");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .with_subtype(Subtype::Forest)
            .untapped()
            .in_zone(Zone::Battlefield),
        ChoiceCount::any_number(),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(tag.clone()),
        Value::Count(ObjectFilter::tagged(tag)),
        PlayerFilter::IteratedPlayer,
    ));

    let compact = describe_choose_then_sacrifice(
        &choose,
        sacrifice_view(&sacrifice).expect("sacrifice-player view"),
    )
    .expect("generated cost chosen set should compact");

    assert_eq!(compact, "you sacrifice any number of untapped Forests");
}

#[test]
pub(super) fn describe_choose_then_sacrifice_does_not_bridge_real_actor_mismatch() {
    let ordinary_tag = TagKey::from("chosen_permanents");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent().in_zone(Zone::Battlefield),
        ChoiceCount::any_number(),
        PlayerFilter::You,
        ordinary_tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let ordinary_sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(ordinary_tag.clone()),
        Value::Count(ObjectFilter::tagged(ordinary_tag)),
        PlayerFilter::Opponent,
    ));
    assert!(
        describe_choose_then_sacrifice(
            &choose,
            sacrifice_view(&ordinary_sacrifice).expect("sacrifice-player view"),
        )
        .is_none()
    );

    let helper_tag = TagKey::from("__sentence_helper_sacrificed_l3_s0_e18");
    let helper_choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent().in_zone(Zone::Battlefield),
        ChoiceCount::any_number(),
        PlayerFilter::You,
        helper_tag,
    )
    .in_zone(Zone::Battlefield);
    let real_all_sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::permanent(),
        Value::Count(ObjectFilter::permanent()),
        PlayerFilter::Opponent,
    ));
    assert!(
        describe_choose_then_sacrifice(
            &helper_choose,
            sacrifice_view(&real_all_sacrifice).expect("sacrifice-player view"),
        )
        .is_none()
    );
}

#[test]
pub(super) fn describe_for_players_choose_then_sacrifice_compacts_multi_count_tagged_set() {
    let tag = TagKey::from("sacrificed_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature().in_zone(Zone::Battlefield),
        ChoiceCount::exactly(2),
        PlayerFilter::IteratedPlayer,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);

    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(tag.clone()),
        Value::Count(ObjectFilter::tagged(tag)),
        PlayerFilter::IteratedPlayer,
    ));
    let for_players = crate::effects::ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![Effect::new(choose), sacrifice],
    );

    let compact = describe_for_players_choose_then_sacrifice(&for_players)
        .expect("multi-count per-player sacrifice should compact");

    assert_eq!(
        compact,
        "Each player sacrifices two creatures of their choice"
    );
}

#[test]
pub(super) fn iterated_shared_card_type_choice_keeps_controller_before_relation() {
    let chosen_tag = TagKey::from("__it__");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent()
            .controlled_by(PlayerFilter::IteratedPlayer)
            .in_zone(Zone::Battlefield)
            .match_tagged(
                TagKey::from("sacrificed_0"),
                TaggedOpbjectRelation::SharesCardType,
            ),
        ChoiceCount::exactly(1),
        PlayerFilter::IteratedPlayer,
        chosen_tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let sacrifice = Effect::new(crate::effects::SacrificeTargetEffect::new(
        ChooseSpec::Tagged(chosen_tag),
    ));
    let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
        Effect::new(choose),
        sacrifice,
    ]));
    let for_players = crate::effects::ForPlayersEffect::new(PlayerFilter::Opponent, vec![sequence]);
    let expected = "Each opponent sacrifices a permanent of their choice that shares a card type with the sacrificed permanent";

    assert_eq!(
        describe_for_players_choose_then_sacrifice(&for_players).as_deref(),
        Some(expected)
    );
    let rendered_effect = Effect::new(for_players);
    assert_eq!(describe_effect(&rendered_effect), expected);
    assert_eq!(
        describe_structural_multisentence_effect_list(&[rendered_effect]).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn iterated_shared_card_type_player_sacrifice_compacts_to_direct_choice() {
    let chosen_tag = TagKey::from("__sentence_helper_sacrificed_l0_s0_e0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent()
            .in_zone(Zone::Battlefield)
            .match_tagged(
                TagKey::from("triggering"),
                TaggedOpbjectRelation::SharesCardType,
            ),
        ChoiceCount::exactly(1),
        PlayerFilter::IteratedPlayer,
        chosen_tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(chosen_tag),
        Value::Fixed(1),
        PlayerFilter::IteratedPlayer,
    ));
    let for_players = crate::effects::ForPlayersEffect::new(
        PlayerFilter::Opponent,
        vec![Effect::new(choose), sacrifice],
    );

    assert_eq!(
        describe_for_players_choose_then_sacrifice(&for_players).as_deref(),
        Some(
            "Each opponent sacrifices a permanent of their choice that shares a card type with it"
        )
    );
}

#[test]
pub(super) fn party_slot_choices_render_as_choose_a_party_then_sacrifice_rest() {
    let tag = TagKey::from("keep");
    let mut effects = [
        Subtype::Cleric,
        Subtype::Rogue,
        Subtype::Warrior,
        Subtype::Wizard,
    ]
    .into_iter()
    .map(|role| {
        Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::creature()
                    .controlled_by(PlayerFilter::IteratedPlayer)
                    .in_zone(Zone::Battlefield)
                    .with_subtype(role)
                    .not_tagged(tag.clone()),
                ChoiceCount::up_to(1),
                PlayerFilter::IteratedPlayer,
                tag.clone(),
            )
            .in_zone(Zone::Battlefield),
        )
    })
    .collect::<Vec<_>>();
    let complement = ObjectFilter::creature()
        .controlled_by(PlayerFilter::IteratedPlayer)
        .in_zone(Zone::Battlefield)
        .not_tagged(tag);
    effects.push(Effect::new(
        crate::effects::zones::SacrificePlayerEffect::new(
            complement.clone(),
            Value::Count(complement),
            PlayerFilter::IteratedPlayer,
        ),
    ));
    let for_players = crate::effects::ForPlayersEffect::new(PlayerFilter::Any, effects);

    assert_eq!(
        describe_for_players_choose_types_then_sacrifice_rest(&for_players).as_deref(),
        Some(
            "Each player chooses a party from among creatures they control, then sacrifices the rest"
        )
    );
}

#[test]
pub(super) fn counted_choice_complement_hides_the_internal_keep_exclusion() {
    let tag = TagKey::from("keep");
    let complement = ObjectFilter::permanent()
        .controlled_by(PlayerFilter::IteratedPlayer)
        .not_tagged(tag.clone());
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            complement.clone(),
            ChoiceCount::exactly(3),
            PlayerFilter::IteratedPlayer,
            tag,
        )
        .in_zone(Zone::Battlefield),
    );
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        complement.clone(),
        Value::Count(complement),
        PlayerFilter::IteratedPlayer,
    ));
    let for_players =
        crate::effects::ForPlayersEffect::new(PlayerFilter::Any, vec![choose, sacrifice]);

    assert_eq!(
        describe_for_players_choose_types_then_sacrifice_rest(&for_players).as_deref(),
        Some("Each player chooses three permanents they control, then sacrifices the rest")
    );
}

#[test]
pub(super) fn qualified_counted_choice_complement_keeps_one_player_relative_clause() {
    let tag = TagKey::from("keep");
    let complement = ObjectFilter::land()
        .controlled_by(PlayerFilter::IteratedPlayer)
        .in_zone(Zone::Battlefield)
        .not_tagged(tag.clone());
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            complement.clone(),
            ChoiceCount::exactly(5),
            PlayerFilter::IteratedPlayer,
            tag,
        )
        .in_zone(Zone::Battlefield),
    );
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        complement.clone(),
        Value::Count(complement),
        PlayerFilter::IteratedPlayer,
    ));
    let controlled_lands = ObjectFilter::land()
        .controlled_by(PlayerFilter::IteratedPlayer)
        .in_zone(Zone::Battlefield);
    let conditional = Effect::new(crate::effects::ConditionalEffect::new(
        Condition::ValueComparison {
            left: Value::Count(controlled_lands),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(6),
        },
        vec![choose, sacrifice],
        Vec::new(),
    ));
    let effects = vec![Effect::for_players(PlayerFilter::Any, vec![conditional])];

    assert_eq!(
        describe_effect_list(&effects),
        "Each player who controls six or more lands chooses five lands they control and sacrifices the rest"
    );
}

#[test]
pub(super) fn describe_may_choose_then_sacrifice_compacts_optional_player_choice() {
    let tag = TagKey::from("sacrificed_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature().in_zone(Zone::Battlefield),
        ChoiceCount::exactly(2),
        PlayerFilter::Any,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(tag.clone()),
        Value::Count(ObjectFilter::tagged(tag)),
        PlayerFilter::Any,
    ));
    let may = crate::effects::MayEffect::new_for_player(
        vec![Effect::new(choose), sacrifice],
        PlayerFilter::Any,
    );

    assert_eq!(
        describe_effect(&Effect::new(may)),
        "any player may sacrifice two creatures of their choice"
    );
}

#[test]
pub(super) fn describe_may_copy_then_choose_new_target_keeps_one_optional_sentence() {
    let copy_id = crate::effect::EffectId(0);
    let copy = Effect::with_id(0, Effect::copy_spell(ChooseSpec::Source))
        .tag(TagKey::from("__copied_stack_object__"));
    let retarget = Effect::new(
        crate::effects::ChooseNewTargetsEffect::may(copy_id).with_single_target_surface(),
    );

    assert_eq!(
        describe_effect(&Effect::may(vec![copy, retarget])),
        "You may copy this spell and may choose a new target for the copy"
    );
}

#[test]
pub(super) fn describe_may_choose_tagged_subset_then_phase_out_keeps_pronoun_surface() {
    let available_tag = TagKey::from("connived_0");
    let chosen_tag = TagKey::from("phase_out_selection");
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::tagged(available_tag).in_zone(Zone::Battlefield),
            ChoiceCount::any_number(),
            PlayerFilter::You,
            chosen_tag.clone(),
        )
        .in_zone(Zone::Battlefield),
    );
    let phase_out = Effect::new(crate::effects::PhaseOutEffect::all(
        ObjectFilter::tagged(chosen_tag).in_zone(Zone::Battlefield),
    ));

    assert_eq!(
        describe_effect(&Effect::may(vec![choose, phase_out])),
        "You may have any number of them phase out"
    );
}

#[test]
pub(super) fn target_player_permanent_piles_keep_split_and_pile_choice_surface() {
    let target_player = PlayerFilter::target_player();
    let chosen_tag = TagKey::from("divvy_chosen");
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::permanent().controlled_by(target_player.clone()),
            ChoiceCount::any_number(),
            target_player.clone(),
            chosen_tag.clone(),
        )
        .in_zone(Zone::Battlefield),
    );
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(
        ChooseSpec::target_player(),
    ));
    let chosen = ObjectFilter::tagged(chosen_tag);
    let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        chosen.clone(),
        Value::Count(chosen),
        target_player,
    ));

    assert_eq!(
        describe_structural_multisentence_effect_list(&[choose, target, sacrifice]).as_deref(),
        Some(
            "Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice"
        )
    );
}

#[test]
pub(super) fn describe_for_players_choose_then_sacrifice_compacts_x_and_that_many_counts() {
    let x_tag = TagKey::from("sacrificed_x");
    let choose_x = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::land().in_zone(Zone::Battlefield),
        ChoiceCount::dynamic_x(),
        PlayerFilter::IteratedPlayer,
        x_tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let sacrifice_x = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(x_tag.clone()),
        Value::Count(ObjectFilter::tagged(x_tag)),
        PlayerFilter::IteratedPlayer,
    ));
    let for_players_x = crate::effects::ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![Effect::new(choose_x), sacrifice_x],
    );

    assert_eq!(
        describe_for_players_choose_then_sacrifice(&for_players_x).as_deref(),
        Some("Each player sacrifices X lands of their choice")
    );

    let amount_tag = TagKey::from("sacrificed_amount");
    let choose_amount = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature().in_zone(Zone::Battlefield),
        ChoiceCount::dynamic_x(),
        PlayerFilter::IteratedPlayer,
        amount_tag.clone(),
    )
    .with_count_value(Value::EffectValue(crate::effect::EffectId(7)))
    .in_zone(Zone::Battlefield);
    let sacrifice_amount = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
        ObjectFilter::tagged(amount_tag.clone()),
        Value::Count(ObjectFilter::tagged(amount_tag)),
        PlayerFilter::IteratedPlayer,
    ));
    let for_players_amount = crate::effects::ForPlayersEffect::new(
        PlayerFilter::Opponent,
        vec![Effect::new(choose_amount), sacrifice_amount],
    );

    assert_eq!(
        describe_for_players_choose_then_sacrifice(&for_players_amount).as_deref(),
        Some("Each opponent sacrifices that many creatures of their choice")
    );
}

#[test]
pub(super) fn party_size_renders_as_for_each_party_basis() {
    let party = Value::PartySize(PlayerFilter::You).with_surface_hint(ValueSurfaceHint::ForEach);
    assert_eq!(
        describe_create_for_each_count(&party).as_deref(),
        Some("creature in your party")
    );

    let pump = Effect::new(
        crate::effects::ModifyPowerToughnessForEachEffect::symmetric(
            ChooseSpec::Source,
            1,
            party,
            Until::EndOfTurn,
        ),
    );
    let rendered = describe_effect(&pump);
    assert!(
        rendered.contains("+1/+1 for each creature in your party"),
        "expected typed party for-each surface, got {rendered}"
    );
}

#[test]
pub(super) fn dynamic_single_axis_stat_scaling_uses_oracle_double_surface() {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    for (power, toughness, expected) in [
        (
            Value::PowerOf(Box::new(target.clone())),
            Value::Fixed(0),
            "Double target creature's power until end of turn",
        ),
        (
            Value::Fixed(0),
            Value::ToughnessOf(Box::new(target.clone())),
            "Double target creature's toughness until end of turn",
        ),
    ] {
        let effect = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec_runtime(
                target.clone(),
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power,
                    toughness,
                },
                Until::EndOfTurn,
            )
            .require_creature_target(),
        );
        assert_eq!(describe_effect(&effect), expected);
    }
}

#[test]
pub(super) fn targeted_double_power_and_tagged_keyword_grant_rejoin_one_duration_scope() {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    let target_tag = TagKey::from("pumped_0");
    let pump = Effect::new(
        crate::effects::ApplyContinuousEffect::with_spec_runtime(
            target.clone(),
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power: Value::PowerOf(Box::new(target)),
                toughness: Value::Fixed(0),
            },
            Until::EndOfTurn,
        )
        .require_creature_target(),
    )
    .tag(target_tag.clone());
    let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Tagged(target_tag),
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::first_strike(),
        ),
        Until::EndOfTurn,
    ))
    .tag(TagKey::from("granted_0"));

    assert_eq!(
        describe_effect_list(&[pump, grant]),
        "Until end of turn, double target creature's power and it gains first strike"
    );
}

#[test]
pub(super) fn dynamic_single_axis_where_x_hint_preserves_authored_pump_surface() {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    let effect = Effect::new(
        crate::effects::ApplyContinuousEffect::with_spec_runtime(
            target.clone(),
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power: Value::PowerOf(Box::new(target.clone()))
                    .with_surface_hint(ValueSurfaceHint::WhereXIs),
                toughness: Value::Fixed(0),
            },
            Until::EndOfTurn,
        )
        .require_creature_target(),
    );

    assert_eq!(
        describe_effect(&effect),
        "target creature gets +X/+0 until end of turn, where X is target creature's power"
    );
}

#[test]
pub(super) fn divided_evenly_damage_precedes_generic_for_each_damage_rendering() {
    let creatures = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::target_opponent());
    let effect = Effect::for_each(
        creatures,
        vec![Effect::deal_damage(Value::X, ChooseSpec::Iterated).tag(TagKey::from("damaged_0"))],
    );

    assert_eq!(
        describe_effect(&effect),
        "Deal X damage divided evenly, rounded down, among all creatures target opponent controls"
    );
}

#[test]
pub(super) fn hinted_count_damage_to_each_opponent_and_life_gain_share_one_x_clause() {
    let amount = Value::Count(
        ObjectFilter::creature()
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::You),
    )
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let damage = Effect::for_players(
        PlayerFilter::Opponent,
        vec![Effect::deal_damage(
            amount.clone(),
            ChooseSpec::Player(PlayerFilter::IteratedPlayer),
        )],
    );
    let gain = Effect::gain_life(amount);

    assert_eq!(
        describe_effect_list(&[damage, gain]),
        "it deals X damage to each opponent and you gain X life, where X is the number of creatures you control"
    );
}

#[test]
pub(super) fn dynamic_both_axis_triple_surface_remains_structural() {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    let effect = Effect::new(
        crate::effects::ApplyContinuousEffect::with_spec_runtime(
            target.clone(),
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power: Value::Scaled(Box::new(Value::PowerOf(Box::new(target.clone()))), 2),
                toughness: Value::Scaled(Box::new(Value::ToughnessOf(Box::new(target))), 2),
            },
            Until::EndOfTurn,
        )
        .require_creature_target(),
    );

    assert_eq!(
        describe_effect(&effect),
        "Triple target creature's power and toughness until end of turn"
    );
}

#[test]
pub(super) fn named_source_stat_scaling_preserves_oracle_possessive() {
    let source = ChooseSpec::Source.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ShortName("Casey Jones".to_string()),
        ),
    );
    let effect = Effect::new(
        crate::effects::ApplyContinuousEffect::with_spec_runtime(
            source.clone(),
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power: Value::PowerOf(Box::new(source)),
                toughness: Value::Fixed(0),
            },
            Until::EndOfTurn,
        )
        .require_creature_target(),
    );

    assert_eq!(
        describe_effect(&effect),
        "Double Casey Jones's power until end of turn"
    );
}

#[test]
pub(super) fn for_each_dynamic_single_axis_scaling_uses_oracle_double_surface() {
    let creatures_you_control = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    for (power, toughness, expected) in [
        (
            Value::PowerOf(Box::new(ChooseSpec::Iterated)),
            Value::Fixed(0),
            "Double the power of each creature you control until end of turn",
        ),
        (
            Value::Fixed(0),
            Value::ToughnessOf(Box::new(ChooseSpec::Iterated)),
            "Double the toughness of each creature you control until end of turn",
        ),
    ] {
        let apply = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec_runtime(
                ChooseSpec::Iterated,
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power,
                    toughness,
                },
                Until::EndOfTurn,
            )
            .require_creature_target(),
        );
        let effect = Effect::for_each(creatures_you_control.clone(), vec![apply]);
        assert_eq!(describe_effect(&effect), expected);
    }
}

#[test]
pub(super) fn removed_counter_metric_draw_keeps_this_way_surface() {
    let id = crate::effect::EffectId(71);
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::permanent()));
    let producer = Effect::with_id(
        id.0,
        Effect::new(crate::effects::RemoveUpToAnyCountersEffect::exact(
            Value::CountersOn(Box::new(target.clone()), None),
            target,
        )),
    );
    let removed = Value::PriorEffectMetric {
        effect_id: id,
        query: crate::effect::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::Outcome,
            crate::effect::EffectMetric::Count,
        )
        .with_action(crate::effect::PriorEffectAction::Removed),
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let draw = Effect::new(crate::effects::DrawCardsEffect::you(removed));

    assert_eq!(
        describe_effect_list(&[producer, draw]),
        "Remove all counters from target permanent. You draw a card for each counter removed this way"
    );
}

#[test]
pub(super) fn nested_removed_counter_metric_scaled_mana_keeps_counter_kind() {
    let id = crate::effect::EffectId(72);
    let remove = Effect::with_id(
        id.0,
        Effect::new(crate::effects::RemoveCountersEffect::new(
            crate::object::CounterType::Charge,
            Value::CountersOnSource(crate::object::CounterType::Charge),
            ChooseSpec::Source,
        )),
    )
    .tag("removed_charge_counters");
    let producer = Effect::with_id(
        id.0,
        Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::tap(ChooseSpec::Source).tag("tapped_source"),
            remove,
        ])),
    );
    let amount = Value::PriorEffectMetric {
        effect_id: id,
        query: crate::effect::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::Outcome,
            crate::effect::EffectMetric::Count,
        )
        .with_action(crate::effect::PriorEffectAction::Removed),
    }
    .with_surface_hint(ValueSurfaceHint::CountersRemovedThisWay);
    let add = Effect::new(crate::effects::AddScaledManaEffect::new(
        vec![ManaSymbol::Colorless],
        amount,
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect_list(&[producer, add]),
        "Tap it and remove all charge counters from it. Add {C} for each charge counter removed this way"
    );
}

#[test]
pub(super) fn typed_removed_counter_metric_draw_keeps_counter_kind() {
    let count = Value::PriorEffectMetric {
        effect_id: crate::effect::EffectId(73),
        query: crate::effect::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::Outcome,
            crate::effect::EffectMetric::Count,
        )
        .with_action(crate::effect::PriorEffectAction::Removed)
        .with_counter_type(Some(crate::object::CounterType::Stun)),
    }
    .with_surface_hint(ValueSurfaceHint::CountersRemovedThisWay)
    .with_surface_hint(ValueSurfaceHint::EqualTo);
    let draw = Effect::new(crate::effects::DrawCardsEffect::you(count));

    assert_eq!(
        describe_effect(&draw),
        "you draw cards equal to the number of stun counters removed this way"
    );
}
