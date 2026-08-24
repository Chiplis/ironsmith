use super::*;

#[test]
fn put_into_graveyard_history_uses_the_structured_owner_for_each_zone() {
    let your_history =
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PutIntoGraveyard {
            owner: PlayerFilter::You,
            from: vec![Zone::Hand, Zone::Library],
        });
    assert_eq!(
        describe_value(&your_history),
        "the number of cards put into your graveyard from your hand or library this turn"
    );

    let their_history =
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PutIntoGraveyard {
            owner: PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)),
            from: vec![Zone::Library, Zone::Hand],
        });
    assert_eq!(
        describe_value(&their_history),
        "the number of cards put into their graveyard from their hand or library this turn"
    );
}

#[test]
fn draw_then_gain_shared_history_value_renders_one_where_x_basis() {
    let shared = Value::TurnHistoryCount(
        ironsmith_core::TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(PlayerFilter::You),
    )
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let effects = vec![
        Effect::new(crate::effects::DrawCardsEffect::new(
            shared.clone(),
            PlayerFilter::You,
        )),
        Effect::new(crate::effects::GainLifeEffect::you(shared)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "You draw X cards and gain X life, where X is the number of colors among permanents you control and spells you've cast this turn"
    );
}

#[test]
fn gain_then_put_counters_shared_value_renders_one_where_x_basis() {
    let shared = Value::PowerOf(Box::new(
        ChooseSpec::Tagged(TagKey::from("sacrificed_0")).with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SacrificedObject(
                ironsmith_core::SacrificedObjectKind::Creature,
            ),
        ),
    ))
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature()
            .you_control()
            .in_zone(Zone::Battlefield),
    ));
    let effects = vec![
        Effect::new(crate::effects::GainLifeEffect::you(shared.clone())),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            shared,
            target,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "You gain X life and put X +1/+1 counters on target creature you control, where X is the sacrificed creature's power"
    );

    let coordinated = Effect::new(crate::effects::SequenceEffect::coordinated(effects));
    assert_eq!(
        describe_effect(&coordinated),
        "You gain X life and put X +1/+1 counters on target creature you control, where X is the sacrificed creature's power"
    );
}

#[test]
fn any_number_from_hand_then_linked_draw_keeps_order_and_offset_surface() {
    let hand = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::You),
        ..Default::default()
    };
    let move_to_library = crate::effects::MoveToZoneEffect::to_bottom_of_library(
        ChooseSpec::Object(hand).with_count(ChoiceCount::any_number()),
    )
    .with_destination_player_surface(PlayerFilter::You);
    let effects = vec![
        Effect::with_id(7, Effect::new(move_to_library).tag(TagKey::from("moved_7"))),
        Effect::new(crate::effects::DrawCardsEffect::new(
            Value::EffectValueOffset(crate::effect::EffectId(7), 1),
            PlayerFilter::You,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Put any number of cards from your hand on the bottom of your library, then draw that many cards plus one"
    );
}

#[test]
fn whole_hand_then_linked_draw_keeps_order_and_any_order_surface() {
    let hand = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::You),
        ..Default::default()
    };
    let move_to_library =
        crate::effects::MoveToZoneEffect::to_bottom_of_library(ChooseSpec::All(hand))
            .with_library_order(ironsmith_core::LibraryPlacementOrder::ChosenBy(
                PlayerFilter::You,
            ))
            .with_destination_player_surface(PlayerFilter::You);
    let effects = vec![
        Effect::with_id(9, Effect::new(move_to_library)),
        Effect::new(crate::effects::DrawCardsEffect::new(
            Value::EffectValue(crate::effect::EffectId(9))
                .with_surface_hint(ValueSurfaceHint::ThatManyCards),
            PlayerFilter::You,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Put the cards in your hand on the bottom of your library in any order, then draw that many cards"
    );
}

#[test]
fn must_block_set_then_assignment_control_keeps_anaphoric_surface() {
    let must_block = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Filter(
            ObjectFilter::creature()
                .controlled_by(PlayerFilter::Opponent)
                .in_zone(Zone::Battlefield),
        ),
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::must_block(),
        ),
        Until::EndOfTurn,
    );
    let effects = vec![
        Effect::new(must_block).tag("granted_0"),
        Effect::control_combat_choices_this_turn(false, true),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Creatures your opponents control block this turn if able, and you choose how those creatures block"
    );
}

#[test]
fn draw_then_gain_different_history_values_do_not_share_a_where_x_basis() {
    let draw = Value::TurnHistoryCount(
        ironsmith_core::TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(PlayerFilter::You),
    )
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let gain = Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::TokensCreated(
        PlayerFilter::You,
    ))
    .with_surface_hint(ValueSurfaceHint::WhereXIs);

    assert!(
        describe_same_actor_draw_then_gain(
            &Effect::new(crate::effects::DrawCardsEffect::new(
                draw,
                PlayerFilter::You,
            )),
            &Effect::new(crate::effects::GainLifeEffect::you(gain)),
        )
        .is_none()
    );
}

#[test]
fn permanent_gain_life_restriction_uses_rest_of_game_surface() {
    let effect = Effect::cant_until(
        crate::effect::Restriction::GainLife(PlayerFilter::DamagedPlayer),
        Until::Forever,
    );

    assert_eq!(
        describe_effect(&effect),
        "that player can't gain life for the rest of the game"
    );
}

#[test]
fn for_each_object_uses_iteration_pronouns_only_inside_the_loop() {
    let effect = Effect::for_each(
        ObjectFilter::creature(),
        vec![Effect::destroy(ChooseSpec::Iterated)],
    );

    assert_eq!(describe_effect(&effect), "For each creature, destroy it");
    assert_eq!(
        describe_effect(&Effect::destroy(ChooseSpec::Iterated)),
        "Destroy that object"
    );
}

#[test]
fn skip_draw_step_uses_controller_grammar_and_possessive() {
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::SkipDrawStepEffect::you())),
        "you skip your next draw step"
    );
}

#[test]
fn tapped_target_group_freeze_keeps_plural_reference() {
    let tapped = TagKey::from("tapped_group");
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
        .with_count(ChoiceCount::up_to(2));
    let tap = Effect::tap(target).tag(tapped.clone());
    let mut filter = ObjectFilter::creature();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: tapped,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let cant = Effect::cant_until(
        crate::effect::Restriction::Untap(filter),
        Until::ControllersNextUntapStep,
    );

    assert_eq!(
        describe_effect_list(&[tap, cant]),
        "Tap up to two target creatures. Those creatures don't untap during their controllers' next untap steps"
    );
}

#[test]
fn consult_first_match_to_hand_keeps_oracle_sentence_boundary() {
    let revealed = TagKey::from("revealed_until_match");
    let matched = TagKey::from("matched_until_match");
    let effects = vec![
        Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::You,
            crate::effects::consult_helpers::LibraryConsultMode::Reveal,
            ObjectFilter::creature(),
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
            revealed.clone(),
            matched.clone(),
        )),
        Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(matched.clone()),
            Zone::Hand,
            false,
        )),
        Effect::new(
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                revealed,
                Some(matched),
                crate::effects::consult_helpers::LibraryBottomOrder::Random,
                PlayerFilter::You,
            ),
        ),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal cards from the top of your library until you reveal a creature card. Put that card into your hand and the rest on the bottom of your library in a random order"
    );
}

fn reveal_put_all_matching_rest_bottom_effects(
    count: Value,
    mut filter: ObjectFilter,
    enters_tapped: bool,
) -> Vec<Effect> {
    let revealed = TagKey::from("revealed");
    let matching = TagKey::from("matching");
    filter.zone = None;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: revealed.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let look = Effect::new(crate::effects::LookAtTopCardsEffect::new(
        PlayerFilter::You,
        count,
        revealed.clone(),
    ));
    let reveal = Effect::new(crate::effects::RevealTaggedEffect::new(revealed.clone()));
    let tag_matching = Effect::new(
        crate::effects::TagMatchingObjectsEffect::new(filter, matching.clone())
            .in_zone(Zone::Library),
    );
    let mut move_to_battlefield =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Battlefield, false);
    move_to_battlefield.enters_tapped = enters_tapped;
    let move_matching = Effect::new(crate::effects::ForEachTaggedEffect::new(
        matching.clone(),
        vec![Effect::new(move_to_battlefield)],
    ));
    let remainder = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(matching),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    );

    vec![look, reveal, tag_matching, move_matching, remainder]
}

#[test]
fn put_all_matching_lands_from_revealed_set_preserves_tapped_destination_and_random_order() {
    let effects = reveal_put_all_matching_rest_bottom_effects(Value::X, ObjectFilter::land(), true);

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top X cards of your library. Put all land cards from among them onto the battlefield tapped and the rest on the bottom of your library in a random order"
    );
}

#[test]
fn put_all_matching_goblin_creatures_from_revealed_set_preserves_filter() {
    let filter = ObjectFilter::creature()
        .with_subtype(Subtype::Goblin)
        .with_mana_value(crate::filter::Comparison::LessThanOrEqual(5));
    let effects = reveal_put_all_matching_rest_bottom_effects(Value::Fixed(6), filter, false);

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top six cards of your library. Put all Goblin creature cards with mana value 5 or less from among them onto the battlefield and the rest on the bottom of your library in a random order"
    );
}

fn reveal_choose_lands_to_battlefield_prefix(
    count: Value,
    choice_count: ChoiceCount,
    enters_tapped: bool,
) -> (Vec<Effect>, TagKey, TagKey) {
    let revealed = TagKey::from("revealed_counted");
    let chosen = TagKey::from("chosen_counted");
    // The lowered real-card shape carries the exact looked-card membership in
    // the filter while retaining broad legacy zone metadata on the choice.
    // Compaction must follow the tag, not treat the first legacy zone as the
    // semantic source of the candidates.
    let mut filter = ObjectFilter::land();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: revealed.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let choose = crate::effects::ChooseObjectsEffect::new(
        filter,
        choice_count,
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zones(vec![
        Zone::Battlefield,
        Zone::Hand,
        Zone::Graveyard,
        Zone::Library,
        Zone::Exile,
    ])
    .reveal();
    let mut move_to_battlefield =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Battlefield, false);
    move_to_battlefield.enters_tapped = enters_tapped;

    (
        vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                count,
                revealed.clone(),
            )),
            Effect::new(crate::effects::RevealTaggedEffect::new(revealed.clone())),
            Effect::new(choose),
            Effect::new(crate::effects::ForEachTaggedEffect::new(
                chosen.clone(),
                vec![Effect::new(move_to_battlefield)],
            )),
        ],
        revealed,
        chosen,
    )
}

#[test]
fn counted_revealed_subset_to_battlefield_compacts_typed_bottom_remainder() {
    let (mut effects, revealed, chosen) =
        reveal_choose_lands_to_battlefield_prefix(Value::Fixed(6), ChoiceCount::up_to(1), true);
    effects.push(Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(chosen),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    ));

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top six cards of your library. Put up to one land card from among them onto the battlefield tapped. Put the rest on the bottom of your library in a random order"
    );
}

#[test]
fn counted_revealed_subset_to_battlefield_compacts_complement_to_graveyard() {
    let (mut effects, revealed, chosen) =
        reveal_choose_lands_to_battlefield_prefix(Value::Fixed(5), ChoiceCount::up_to(1), false);
    let mut iterated_is_chosen = ObjectFilter::default();
    iterated_is_chosen
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from("__it__"),
            relation: TaggedOpbjectRelation::SameStableId,
        });
    effects.push(Effect::new(crate::effects::ForEachTaggedEffect::new(
        revealed,
        vec![Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(chosen, iterated_is_chosen),
            Vec::new(),
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Graveyard,
                false,
            ))],
        ))],
    )));

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top five cards of your library. Put up to one land card from among them onto the battlefield. Put the rest into your graveyard"
    );
}

#[test]
fn counted_revealed_subset_compacts_dynamic_total_mana_value_constraint() {
    let (mut effects, revealed, chosen) =
        reveal_choose_lands_to_battlefield_prefix(Value::Fixed(8), ChoiceCount::up_to(2), false);
    let choose = effects[2]
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("real-shape choice");
    let mut choose = choose.clone();
    choose.filter = ObjectFilter::artifact();
    choose.filter.excluded_card_types.push(CardType::Creature);
    choose
        .filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: revealed.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    choose.aggregate_constraint = Some(
        crate::effect::ChoiceAggregateConstraint::total_mana_value_at_most(Value::ManaValueOf(
            Box::new(
                ChooseSpec::Tagged(TagKey::from("sacrifice_cost_0")).with_surface_hint(
                    crate::target::ChooseSpecSurfaceHint::SourceReference(
                        crate::target::SourceReferenceSurface::ThisPermanentType(
                            "the sacrificed artifact".to_string(),
                        ),
                    ),
                ),
            ),
        )),
    );
    effects[2] = Effect::new(choose);
    effects.push(Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(chosen),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    ));

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top eight cards of your library. Put up to two noncreature artifact cards with total mana value less than or equal to the sacrificed artifact's mana value from among them onto the battlefield. Put the rest on the bottom of your library in a random order"
    );
}

#[test]
fn counter_and_draw_followups_preserve_conjoined_clause_composition() {
    let draw_then_counter = vec![
        Effect::draw(Value::Fixed(1)),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::Charge,
            1,
            ChooseSpec::Source,
        )),
    ];
    let counter_then_draw = vec![
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::Stun,
            3,
            ChooseSpec::Source,
        )),
        Effect::draw(Value::Fixed(3)),
    ];

    assert_eq!(
        describe_effect_list(&draw_then_counter),
        "Draw a card and put a charge counter on this source"
    );
    assert_eq!(
        describe_effect_clause_list(&draw_then_counter).as_deref(),
        Some("draw a card and put a charge counter on this source")
    );
    assert_eq!(
        describe_effect_list(&counter_then_draw),
        "Put three stun counters on this source and draw three cards"
    );

    let draw_lose_then_counter = vec![
        Effect::draw(Value::Fixed(1)),
        Effect::new(crate::effects::LoseLifeEffect::you(1)),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::Named("plan".into()),
            1,
            ChooseSpec::Source,
        )),
    ];
    assert_eq!(
        describe_effect_clause_list(&draw_lose_then_counter).as_deref(),
        Some("draw a card, lose 1 life, and put a plan counter on this source")
    );

    let scry_then_counter = vec![
        Effect::scry(Value::Fixed(1)),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::Oil,
            1,
            ChooseSpec::Source,
        )),
    ];
    assert_eq!(
        describe_effect_list(&scry_then_counter),
        "Scry 1 and put an oil counter on this source"
    );
}

#[test]
fn discard_result_draw_sequence_preserves_up_to_and_that_many_surface() {
    let effect_id = crate::effect::EffectId(7);
    let discard = Effect::with_id(
        effect_id.0,
        Effect::new(
            crate::effects::DiscardEffect::new(2, PlayerFilter::You, false).with_any_number(true),
        ),
    );
    let draw = Effect::new(crate::effects::DrawCardsEffect::new(
        Value::EffectMetric {
            effect_id,
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::Count,
        },
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect_list(&[discard, draw]),
        "Discard up to two cards, then draw that many cards"
    );
}

#[test]
fn targeted_damage_does_not_rebind_an_explicit_you_discard_draw_sequence() {
    let discard_id = crate::effect::EffectId(41);
    let hand_count =
        Value::CardsInHand(PlayerFilter::You).with_surface_hint(ValueSurfaceHint::AllCardsInHand);
    let damage = Effect::deal_damage(
        Value::Count(
            ObjectFilter::default()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::You),
        ),
        ChooseSpec::AnyTarget,
    )
    .tag("damaged_0");
    let discard = Effect::with_id(
        discard_id.0,
        Effect::new(crate::effects::DiscardEffect::new(
            hand_count,
            PlayerFilter::You,
            false,
        )),
    );
    let draw = Effect::new(crate::effects::DrawCardsEffect::new(
        Value::EffectMetric {
            effect_id: discard_id,
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::Count,
        },
        PlayerFilter::You,
    ));

    let rendered = describe_pre_clause_structural_effect_list(&[damage, discard, draw]).unwrap();
    assert!(
        rendered.contains(". Discard all the cards in your hand, then draw that many cards"),
        "{rendered}"
    );
    assert!(!rendered.contains("that player discards"), "{rendered}");
}

#[test]
fn selection_then_draw_keeps_later_actions_in_separate_sentences() {
    let scry_draw_lose = vec![
        Effect::scry(Value::Fixed(2)),
        Effect::draw(Value::Fixed(2)),
        Effect::new(crate::effects::LoseLifeEffect::you(2)),
    ];
    assert_eq!(
        describe_effect_clause_list(&scry_draw_lose).as_deref(),
        Some("scry 2, then draw two cards. You lose 2 life")
    );

    let surveil_draw_damage = vec![
        Effect::surveil(Value::Fixed(2)),
        Effect::draw(Value::Fixed(2)),
        Effect::new(crate::effects::DealDamageEffect::new(
            Value::Fixed(2),
            ChooseSpec::Player(PlayerFilter::You),
        )),
    ];
    assert_eq!(
        describe_effect_clause_list(&surveil_draw_damage).as_deref(),
        Some("surveil 2, then draw two cards. Deal 2 damage to you")
    );

    // A bare draw/life-loss pair is intentionally outside this structural
    // rule; many Oracle cards conjoin exactly those two instructions.
    let conjoined_draw_lose = vec![
        Effect::draw(Value::Fixed(2)),
        Effect::new(crate::effects::LoseLifeEffect::you(2)),
    ];
    assert_eq!(
        describe_effect_clause_list(&conjoined_draw_lose).as_deref(),
        Some("draw two cards and lose 2 life")
    );
}

#[test]
fn discard_hand_then_fixed_draw_stays_a_then_sequence() {
    let effects = vec![
        Effect::new(crate::effects::DiscardHandEffect::you()),
        Effect::new(crate::effects::DrawCardsEffect::you(8)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Discard your hand, then draw eight cards"
    );
}

#[test]
fn damaged_player_discard_draw_uses_the_contextual_that_player_subject() {
    let effect_id = crate::effect::EffectId(3);
    let discard = Effect::with_id(
        effect_id.0,
        Effect::new(crate::effects::DiscardEffect::new(
            Value::CardsInHand(PlayerFilter::DamagedPlayer),
            PlayerFilter::DamagedPlayer,
            false,
        )),
    );
    let draw = Effect::new(crate::effects::DrawCardsEffect::new(
        Value::EffectValue(effect_id),
        PlayerFilter::DamagedPlayer,
    ));

    assert_eq!(
        describe_effect_list(&[discard, draw]),
        "that player discards all the cards in their hand, then draws that many cards"
    );
}

#[test]
fn explicit_all_cards_in_hand_surface_survives_context_rebinding() {
    let effect_id = crate::effect::EffectId(31);
    let discard = Effect::with_id(
        effect_id.0,
        Effect::new(crate::effects::DiscardEffect::new(
            Value::CardsInHand(PlayerFilter::IteratedPlayer)
                .with_surface_hint(ValueSurfaceHint::AllCardsInHand),
            PlayerFilter::DamagedPlayer,
            false,
        )),
    );
    let draw = Effect::new(crate::effects::DrawCardsEffect::new(
        Value::EffectValue(effect_id),
        PlayerFilter::DamagedPlayer,
    ));

    assert_eq!(
        describe_effect_list(&[discard, draw]),
        "that player discards all the cards in their hand, then draws that many cards"
    );
}

#[test]
fn move_to_zone_preserves_an_explicit_contextual_destination_surface() {
    let tagged = ChooseSpec::Tagged(crate::TagKey::from("revealed_0"));
    let contextual = Effect::new(
        crate::effects::MoveToZoneEffect::new(tagged.clone(), Zone::Graveyard, false)
            .with_destination_player_surface(PlayerFilter::You),
    );
    let canonical = Effect::new(crate::effects::MoveToZoneEffect::new(
        tagged,
        Zone::Graveyard,
        false,
    ));

    assert_eq!(describe_effect(&contextual), "Put it into your graveyard");
    assert_eq!(
        describe_effect(&canonical),
        "Put it into its owner's graveyard"
    );
}

fn target_player_look_then_may_move(
    reference_surface: ironsmith_core::DestinationPlayerReferenceSurface,
) -> Vec<Effect> {
    let looked = crate::TagKey::from("looked");
    let target_player = PlayerFilter::target_player();
    vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_player(),
        )),
        Effect::new(crate::effects::LookAtTopCardsEffect::new(
            target_player.clone(),
            Value::Fixed(1),
            looked.clone(),
        )),
        Effect::may(vec![Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(looked),
                Zone::Graveyard,
                false,
            )
            .with_destination_player_surface(target_player)
            .with_destination_player_reference_surface(reference_surface),
        )]),
    ]
}

#[test]
fn target_player_look_bundle_preserves_destination_reference_surface() {
    assert_eq!(
        describe_effect_list(&target_player_look_then_may_move(
            ironsmith_core::DestinationPlayerReferenceSurface::Pronoun,
        )),
        "Look at the top card of target player's library. You may put that card into their graveyard"
    );
    assert_eq!(
        describe_effect_list(&target_player_look_then_may_move(
            ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer,
        )),
        "Look at the top card of target player's library. You may put that card into that player's graveyard"
    );
}

#[test]
fn return_to_hand_preserves_an_explicit_contextual_destination_surface() {
    let tagged = ChooseSpec::Tagged(crate::TagKey::from("returned_0"));
    let contextual = Effect::new(
        crate::effects::ReturnToHandEffect::with_spec(tagged.clone())
            .with_destination_player_surface(PlayerFilter::You),
    );
    let canonical = Effect::new(crate::effects::ReturnToHandEffect::with_spec(
        tagged.clone(),
    ));
    let contextual_from_graveyard = Effect::new(
        crate::effects::ReturnFromGraveyardToHandEffect::new(tagged, false)
            .with_destination_player_surface(PlayerFilter::DamagedPlayer),
    );

    assert_eq!(describe_effect(&contextual), "Return it to your hand");
    assert_eq!(describe_effect(&canonical), "Return it to its owner's hand");
    assert_eq!(
        describe_effect(&contextual_from_graveyard),
        "Return it from a graveyard to their hand"
    );
}

#[test]
fn destination_first_targeted_return_preserves_tagged_lesser_mana_surface() {
    let mut filter = ObjectFilter::artifact()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    filter.set_explicit_card_noun(true);
    filter.set_return_destination_first_surface(true);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("triggering"),
        relation: TaggedOpbjectRelation::ManaValueLtTagged,
    });
    let effect = Effect::new(
        crate::effects::ReturnFromGraveyardToHandEffect::new(
            ChooseSpec::target(ChooseSpec::Object(filter)),
            false,
        )
        .with_graveyard_player_surface(PlayerFilter::You)
        .with_destination_player_surface(PlayerFilter::You),
    );

    assert_eq!(
        describe_effect(&effect),
        "Return to your hand target artifact card in your graveyard with lesser mana value"
    );
}

#[test]
fn aliased_target_actions_compact_under_the_original_target_subject() {
    let alias = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent));
    let effects = vec![
        Effect::new(crate::effects::DiscardEffect::new(
            Value::Fixed(2),
            PlayerFilter::target_opponent(),
            false,
        )),
        Effect::new(crate::effects::MillEffect::new(
            Value::Fixed(2),
            alias.clone(),
        )),
        Effect::new(crate::effects::LoseLifeEffect::with_filter(
            Value::Fixed(2),
            alias,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target opponent discards two cards, mills two cards, and loses 2 life"
    );
}

#[test]
fn discard_then_aliased_graveyard_exile_keeps_the_then_surface() {
    let alias = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
    let discard = Effect::new(crate::effects::DiscardEffect::new(
        Value::Fixed(2),
        PlayerFilter::target_player(),
        false,
    ));
    let exile = Effect::new(crate::effects::ExileEffect::all(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(alias),
    ));

    assert_eq!(
        describe_effect_list(&[discard, exile]),
        "Target player discards two cards. Then exile that player's graveyard"
    );
}

#[test]
fn target_hand_count_uses_the_same_players_pronoun() {
    let hand = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)));
    let draw = crate::effects::DrawCardsEffect::new(
        Value::Count(hand).with_surface_hint(ValueSurfaceHint::EqualTo),
        PlayerFilter::target_player(),
    );

    assert_eq!(
        describe_draw_for_each(&draw).as_deref(),
        Some("target player draws cards equal to the number of cards in their hand")
    );
}

#[test]
fn reveal_hand_then_count_that_players_hand_keeps_both_surfaces() {
    let hand = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)));
    let effects = vec![
        Effect::new(crate::effects::LookAtHandEffect::reveal(
            ChooseSpec::target_player(),
        )),
        Effect::new(crate::effects::GainLifeEffect::you(Value::Count(hand))),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target player reveals their hand. You gain life equal to the number of cards in that player's hand"
    );
}

#[test]
fn followup_owner_and_damage_references_do_not_retarget() {
    let opponent_alias = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent));
    let owned_nonland = ObjectFilter::default()
        .without_type(CardType::Land)
        .owned_by(opponent_alias);
    assert!(owned_nonland.description().contains("that player owns"));
    assert!(!owned_nonland.description().contains("target opponent owns"));

    let player_alias = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
    let damage = Effect::new(crate::effects::DealDamageEffect::new(
        Value::Fixed(4),
        ChooseSpec::Player(player_alias),
    ));
    assert_eq!(describe_effect(&damage), "Deal 4 damage to that player");
}

#[test]
fn unless_damage_keeps_the_decider_causative() {
    let player_alias = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
    let effect = Effect::new(crate::effects::UnlessActionEffect::new(
        vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target_player(),
            )),
            Effect::new(crate::effects::DiscardEffect::new(
                Value::Fixed(2),
                PlayerFilter::target_player(),
                true,
            )),
        ],
        vec![Effect::new(crate::effects::DealDamageEffect::new(
            Value::Fixed(4),
            ChooseSpec::Player(PlayerFilter::target_player()),
        ))],
        player_alias,
    ));

    assert_eq!(
        describe_effect(&effect),
        "Target player discards two cards at random unless that player has this source deal 4 damage to them"
    );
}

#[test]
fn reveal_choose_exile_then_cast_uses_the_selected_opponents_owner_alias() {
    let chosen = TagKey::from("__it__");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .without_type(CardType::Land)
            .owned_by(PlayerFilter::target_opponent()),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zones(vec![Zone::Graveyard, Zone::Hand]);
    let effects = vec![
        Effect::new(crate::effects::LookAtHandEffect::reveal(
            ChooseSpec::target_opponent(),
        )),
        Effect::new(choose),
        Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(chosen),
            Zone::Exile,
            true,
        )),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            "__source_exiled__",
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled,
            false,
            true,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or hand and exile it. You may cast that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell"
    );
}

#[test]
fn target_only_plus_revealing_look_introduces_the_target() {
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_opponent(),
        )),
        Effect::new(crate::effects::LookAtTopCardsEffect::revealing(
            PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent)),
            Value::Fixed(2),
            crate::TagKey::from("revealed"),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target opponent reveals the top two cards of their library"
    );
}

#[test]
fn draw_then_conditional_discard_renders_positive_unless_condition() {
    let effects = vec![
        Effect::draw(Value::Fixed(2)),
        Effect::new(crate::effects::ConditionalEffect::new(
            Condition::Not(Box::new(Condition::AttackedThisTurn)),
            vec![Effect::new(crate::effects::DiscardEffect::new(
                Value::Fixed(1),
                PlayerFilter::You,
                false,
            ))],
            Vec::new(),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Draw two cards, then discard a card unless you attacked this turn"
    );
}

#[test]
fn source_sentence_leading_then_surface_renders_the_explicit_connective() {
    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::draw(Value::Fixed(1))]),
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(
            crate::effects::SequenceEffect::sentence_leading_then(vec![Effect::gain_life(1)]),
        )]),
    ]);

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "you draw a card. Then you gain 1 life"
    );
}

#[test]
fn ordinary_sequential_sentence_does_not_gain_a_then_connective() {
    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::draw(Value::Fixed(1))]),
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(
            crate::effects::SequenceEffect::new(vec![Effect::gain_life(1)]),
        )]),
    ]);

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "you draw a card. you gain 1 life"
    );
}

#[test]
fn modal_modes_with_shared_spells_cast_value_render_one_x_preamble() {
    let spells_cast = Value::SpellsCastThisTurn(PlayerFilter::You);
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new(
            "Scry X.",
            vec![Effect::new(crate::effects::ScryEffect::you(
                spells_cast.clone(),
            ))],
        ),
        ironsmith_core::EffectMode::new(
            "This creature deals X damage to target creature.",
            vec![
                Effect::new(crate::effects::DealDamageEffect::new(
                    spells_cast.clone(),
                    ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
                ))
                .tag(TagKey::from("damaged_0")),
            ],
        ),
        ironsmith_core::EffectMode::new(
            "You gain X life.",
            vec![Effect::new(crate::effects::GainLifeEffect::you(
                spells_cast,
            ))],
        ),
    ]);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Choose one. X is the number of spells you've cast this turn —\n\
         • Scry X.\n\
         • This creature deals X damage to target creature.\n\
         • You gain X life."
    );
}

#[test]
fn modal_modes_with_different_spells_cast_values_do_not_invent_shared_x() {
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new(
            "Scry X.",
            vec![Effect::new(crate::effects::ScryEffect::you(
                Value::SpellsCastThisTurn(PlayerFilter::You),
            ))],
        ),
        ironsmith_core::EffectMode::new(
            "You gain X life.",
            vec![Effect::new(crate::effects::GainLifeEffect::you(
                Value::SpellsCastThisTurn(PlayerFilter::Opponent),
            ))],
        ),
    ]);

    let rendered = describe_effect(&Effect::new(choose));
    assert!(rendered.starts_with("Choose one —"), "{rendered}");
    assert!(!rendered.contains(". X is "), "{rendered}");
}

#[test]
fn modal_common_return_suffix_renders_once_before_target_modes() {
    let return_mode = |card_type, source_text| {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        filter.card_types = vec![card_type];
        filter.set_explicit_card_noun(true);
        let returned = crate::effects::ReturnFromGraveyardToHandEffect::new(
            ChooseSpec::target(ChooseSpec::Object(filter)),
            false,
        )
        .with_graveyard_player_surface(PlayerFilter::You)
        .with_destination_player_surface(PlayerFilter::You);
        ironsmith_core::EffectMode::new(source_text, vec![Effect::new(returned)])
    };
    let choose = ironsmith_core::ChooseModeEffect::choose_up_to(
        2,
        0,
        vec![
            return_mode(CardType::Artifact, "Target artifact card."),
            return_mode(CardType::Creature, "Target creature card."),
            return_mode(CardType::Enchantment, "Target enchantment card."),
            return_mode(CardType::Land, "Target land card."),
        ],
    )
    .with_common_suffix_effect_count(1);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Choose up to two. Return those cards from your graveyard to your hand.\n\
• Target artifact card.\n\
• Target creature card.\n\
• Target enchantment card.\n\
• Target land card."
    );
}

#[test]
fn repeated_modal_choices_keep_each_mode_on_a_bullet_line() {
    let choose = ironsmith_core::ChooseModeEffect::choose_exactly(
        Value::Fixed(4),
        vec![
            ironsmith_core::EffectMode::new("Draw a card.", vec![Effect::draw(Value::Fixed(1))]),
            ironsmith_core::EffectMode::new(
                "You gain 1 life.",
                vec![Effect::gain_life(Value::Fixed(1))],
            ),
        ],
    )
    .with_repeated_modes();

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Choose four. You may choose the same mode more than once.\n\
         • Draw a card.\n\
         • You gain 1 life."
    );
}

#[test]
fn modal_common_prefix_renders_once_in_the_header() {
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new("Draw a card.", vec![Effect::draw(Value::Fixed(1))]),
        ironsmith_core::EffectMode::new(
            "You gain 1 life.",
            vec![Effect::gain_life(Value::Fixed(1))],
        ),
    ])
    .with_common_prefix_effects(vec![Effect::gain_life(Value::Fixed(3))]);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Choose one and you gain 3 life.\n• Draw a card.\n• You gain 1 life."
    );

    let ordinary = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new("Draw a card.", vec![Effect::draw(Value::Fixed(1))]),
        ironsmith_core::EffectMode::new(
            "You gain 1 life.",
            vec![Effect::gain_life(Value::Fixed(1))],
        ),
    ]);
    assert_eq!(
        describe_effect(&Effect::new(ordinary)),
        "Choose one —\n• Draw a card.\n• You gain 1 life."
    );
}

#[test]
fn inline_named_token_creation_choice_renders_as_authored_or_instruction() {
    let named_artifact_token = |name, subtype| {
        crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
            .token()
            .card_types(vec![crate::types::CardType::Artifact])
            .subtypes(vec![subtype])
            .build()
    };
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(crate::effects::CreateTokenEffect::one(
                named_artifact_token("Food", crate::types::Subtype::Food),
            ))],
        ),
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(crate::effects::CreateTokenEffect::one(
                named_artifact_token("Treasure", crate::types::Subtype::Treasure),
            ))],
        ),
    ])
    .with_chooser(PlayerFilter::You);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Create a Food token or a Treasure token"
    );
}

#[test]
fn inline_destroy_all_choice_renders_as_authored_or_instruction() {
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(crate::effects::DestroyEffect::all(
                ObjectFilter::default().with_type(crate::types::CardType::Land),
            ))],
        ),
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(crate::effects::DestroyEffect::all(
                ObjectFilter::creature(),
            ))],
        ),
    ])
    .with_chooser(PlayerFilter::You);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Destroy all lands or all creatures"
    );
}

#[test]
fn inline_destroy_all_choice_keeps_shared_no_regeneration_followup() {
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(
                crate::effects::DestroyNoRegenerationEffect::all(
                    ObjectFilter::default().with_type(crate::types::CardType::Land),
                ),
            )],
        ),
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(
                crate::effects::DestroyNoRegenerationEffect::all(ObjectFilter::creature()),
            )],
        ),
    ])
    .with_chooser(PlayerFilter::You);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Destroy all lands or all creatures. They can't be regenerated"
    );
}

fn source_exiled_filter() -> ObjectFilter {
    ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile)
}

#[test]
fn source_exiled_move_preserves_all_source_type_and_plural_owners() {
    let surface = ironsmith_core::ExiledWithSourceMoveSurface {
        verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Put,
        subject: ironsmith_core::ExiledWithSourceSubjectSurface::AllCards,
        source: ironsmith_core::ExiledWithSourceReferenceSurface::Source(
            crate::target::SourceReferenceSurface::ThisPermanentType("this creature".to_string()),
        ),
        destination: ironsmith_core::ExiledWithSourceDestinationSurface::TheirOwners,
    };
    let effect = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::All(source_exiled_filter()),
        Zone::Hand,
        false,
    )
    .with_exiled_with_source_surface(surface);

    assert_eq!(
        describe_effect(&Effect::new(effect)),
        "Put all cards exiled with this creature into their owners' hands"
    );
}

#[test]
fn source_exiled_return_preserves_one_card_and_contextual_hand() {
    let surface = ironsmith_core::ExiledWithSourceMoveSurface {
        verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Put,
        subject: ironsmith_core::ExiledWithSourceSubjectSurface::OneCard,
        source: ironsmith_core::ExiledWithSourceReferenceSurface::Source(
            crate::target::SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string(),
            ),
        ),
        destination: ironsmith_core::ExiledWithSourceDestinationSurface::ContextualPlayer,
    };
    let effect = crate::effects::ReturnToHandEffect::all(source_exiled_filter())
        .with_destination_player_surface(PlayerFilter::You)
        .with_exiled_with_source_surface(surface);

    assert_eq!(
        describe_effect(&Effect::new(effect)),
        "Put a card exiled with this enchantment into your hand"
    );
}

#[test]
fn source_exiled_move_preserves_each_it_and_singular_owner() {
    let surface = ironsmith_core::ExiledWithSourceMoveSurface {
        verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Put,
        subject: ironsmith_core::ExiledWithSourceSubjectSurface::EachCard,
        source: ironsmith_core::ExiledWithSourceReferenceSurface::It,
        destination: ironsmith_core::ExiledWithSourceDestinationSurface::ItsOwner,
    };
    let effect = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::All(source_exiled_filter()),
        Zone::Graveyard,
        false,
    )
    .with_exiled_with_source_surface(surface);

    assert_eq!(
        describe_effect(&Effect::new(effect)),
        "Put each card exiled with it into its owner's graveyard"
    );
}

#[test]
fn source_exiled_battlefield_move_preserves_typed_subject_and_controller() {
    let surface = ironsmith_core::ExiledWithSourceMoveSurface {
        verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Put,
        subject: ironsmith_core::ExiledWithSourceSubjectSurface::Custom(
            "each creature card".to_string(),
        ),
        source: ironsmith_core::ExiledWithSourceReferenceSurface::Source(
            crate::target::SourceReferenceSurface::ThisPermanentType("this artifact".to_string()),
        ),
        destination: ironsmith_core::ExiledWithSourceDestinationSurface::ContextualPlayer,
    };
    let effect = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::All(source_exiled_filter()),
        Zone::Battlefield,
        false,
    )
    .tapped()
    .under_you_control()
    .with_exiled_with_source_surface(surface);

    assert_eq!(
        describe_effect(&Effect::new(effect)),
        "Put each creature card exiled with this artifact onto the battlefield tapped under your control"
    );
}

#[test]
fn source_exiled_return_preserves_definite_singular_without_source_suffix() {
    let surface = ironsmith_core::ExiledWithSourceMoveSurface {
        verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Return,
        subject: ironsmith_core::ExiledWithSourceSubjectSurface::TheExiledCard,
        source: ironsmith_core::ExiledWithSourceReferenceSurface::Omitted,
        destination: ironsmith_core::ExiledWithSourceDestinationSurface::ItsOwner,
    };
    let effect = crate::effects::ReturnToHandEffect::all(source_exiled_filter())
        .with_exiled_with_source_surface(surface);

    assert_eq!(
        describe_effect(&Effect::new(effect)),
        "Return the exiled card to its owner's hand"
    );
}

#[test]
fn behold_surfaces_preserve_subtype_optional_verb_and_condition_provenance() {
    let behold = crate::effects::BeholdEffect::you(Subtype::Dragon, 1);
    assert_eq!(
        describe_effect(&Effect::new(behold.clone())),
        "Behold a Dragon"
    );

    let optional_cost = crate::cost::OptionalCost::custom(
        "As an additional cost to cast this spell, you may behold a Dragon.",
        crate::cost::TotalCost::from_cost(crate::costs::Cost::effect(behold)),
    );
    assert_eq!(
        describe_optional_cost_line(&optional_cost),
        "As an additional cost to cast this spell, you may behold a Dragon"
    );

    let typed_behold = crate::cost::OptionalCostRef::with_discriminator(
        crate::cost::OptionalCostKind::Behold,
        "Dragon",
    );
    assert_eq!(
        describe_condition(&Condition::ThisSpellPaidLabel(typed_behold.clone())),
        "a Dragon was beheld"
    );
    assert_eq!(
        describe_condition(&Condition::Or(
            Box::new(Condition::ThisSpellPaidLabel(typed_behold)),
            Box::new(Condition::PlayerControls {
                player: PlayerFilter::You,
                filter: ObjectFilter::default().with_subtype(Subtype::Dragon),
            }),
        )),
        "you revealed a Dragon card or controlled a Dragon as you cast this spell"
    );
    assert_eq!(
        describe_condition(&Condition::ThisSpellPaidLabel(
            crate::cost::OptionalCostKind::Behold.into(),
        )),
        "this spell's behold cost was paid"
    );
}
#[test]
fn demonstrative_for_each_damage_keeps_each_of_those_surface() {
    let tag = TagKey::from("chosen_targets");
    let mut filter = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .match_tagged(tag, crate::filter::TaggedOpbjectRelation::IsTaggedObject);
    filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
    let damage = Effect::new(crate::effects::ForEachObject::new(
        filter,
        vec![Effect::new(crate::effects::DealDamageEffect::new(
            Value::X,
            ChooseSpec::Iterated,
        ))],
    ));

    assert_eq!(
        describe_effect(&damage),
        "Deal X damage to each of those creatures"
    );
}
