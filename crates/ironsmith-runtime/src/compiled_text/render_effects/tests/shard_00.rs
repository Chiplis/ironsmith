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
    assert_eq!(describe_effect(&scaled), "Add {R} for each creature");
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
        "Deal 4 damage to any target unless that object's controller pays {2}. If that doesn't happen, deal 2 damage to any target"
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
        "Exile target card in a graveyard. When you do, choose one —\n• Draw a card.\n• Gain 2 life."
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
        "you may cast an instant or sorcery spell from your hand with mana value less than or equal to the other result without paying its mana cost"
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
        "Roll two d8 and choose one result. Draw cards equal to that result. Then you may cast an instant or sorcery spell from your hand with mana value less than or equal to the other result without paying its mana cost."
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
        describe_trigger_surface_with_frequency(&triggered, None),
        "Whenever a creature you control becomes blocked"
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
        describe_trigger_surface_with_frequency(&triggered, None),
        "Whenever enchanted creature becomes blocked"
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
        "you discard the number of cards in your hand"
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
        "that player discards the number of cards in that player's hand"
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
        Effect::new(crate::effects::TargetOnlyEffect::new(target)),
        Effect::new(exchange).tag("exchanged_0"),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Choose target permanent that shares a card type with it. Exchange control of those permanents"
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
