use super::*;

#[test]
fn sacrifice_all_never_gains_an_implied_choice_suffix() {
    let wave = "Each player sacrifices all artifacts, enchantments, and nonbasic lands they control. For each land sacrificed this way, its controller may search their library. Then each player who searched their library this way shuffles.";
    assert_eq!(normalize_sacrifice_implied_choice(wave), None);
    assert_eq!(
        normalize_sacrifice_implied_choice("Each player sacrifices a creature."),
        Some("each player sacrifices a creature of their choice.".to_string())
    );
}

#[test]
fn plural_object_filters_pluralize_bare_ability_nouns() {
    assert_eq!(
        pluralize_noun_phrase("land with mana ability"),
        "lands with mana abilities"
    );
    assert_eq!(
        pluralize_noun_phrase("card with a cycling ability"),
        "cards with a cycling ability"
    );
}

#[test]
fn unscoped_where_x_count_preserves_battlefield_zone() {
    let global_clerics = ObjectFilter::default()
        .with_subtype(Subtype::Cleric)
        .in_zone(Zone::Battlefield);
    assert_eq!(
        describe_where_x_basis(
            &Value::Count(global_clerics).with_surface_hint(ValueSurfaceHint::WhereXIs)
        )
        .as_deref(),
        Some("the number of Clerics on the battlefield")
    );

    let controlled_clerics = ObjectFilter::default()
        .with_subtype(Subtype::Cleric)
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    assert_eq!(
        describe_where_x_basis(
            &Value::Count(controlled_clerics).with_surface_hint(ValueSurfaceHint::WhereXIs)
        )
        .as_deref(),
        Some("the number of Clerics you control")
    );
}

#[test]
fn where_x_count_can_preserve_an_explicit_ability_noun() {
    let cycling_cards = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
        .with_ability_marker("cycling");
    let compact = Value::Count(cycling_cards.clone()).with_surface_hint(ValueSurfaceHint::WhereXIs);
    let explicit = compact
        .clone()
        .with_surface_hint(ValueSurfaceHint::ExplicitAbilityNoun);

    assert_eq!(
        describe_where_x_basis(&compact).as_deref(),
        Some("the number of cards with cycling in your graveyard")
    );
    assert_eq!(
        describe_where_x_basis(&explicit).as_deref(),
        Some("the number of cards with a cycling ability in your graveyard")
    );
}

#[test]
fn counted_shared_characteristic_prior_result_renders_exact_gate() {
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

    assert_eq!(
        describe_effect_predicate(&predicate),
        "two nonland cards that share a color were milled this way"
    );
}

#[test]
fn generic_search_filter_detection_keeps_nested_characteristic_constraints() {
    let generic = ObjectFilter::default().owned_by(PlayerFilter::You);
    assert!(is_generic_owned_card_search_filter(&generic));

    let qualified = generic.sharing_characteristics_with(
        vec![crate::ObjectCharacteristic::Color],
        ObjectFilter::creature().you_control(),
    );
    assert!(
        !is_generic_owned_card_search_filter(&qualified),
        "a nested characteristic comparison must not collapse to 'a card'"
    );
}

#[test]
fn historical_spell_choice_and_half_damage_render_as_one_oracle_action() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "You choose a player who cast one or more sorcery spells this turn. Choose one of sorceries cast this turn by that player, then Backdraft deals half the damage dealt this turn by the chosen spell, rounded down damage to that player."
        ),
        "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down."
    );
}

#[test]
fn attack_group_total_power_uses_the_group_as_the_trigger_subject() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever you attack with one or more Phyrexians, return target artifact card from your graveyard to the battlefield if its mana value is less than or equal to their total power."
        ),
        "Whenever one or more Phyrexians you control attack, return target artifact card from your graveyard to the battlefield if its mana value is less than or equal to their total power."
    );
}

#[test]
fn declared_target_mana_value_condition_uses_its_pronoun() {
    let target = ChooseSpec::Target(Box::new(ChooseSpec::Object(
        ObjectFilter::artifact().in_zone(Zone::Graveyard),
    )));
    let condition = Condition::ValueComparison {
        left: Value::ManaValueOf(Box::new(target)),
        operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
        right: Value::Fixed(3),
    };

    assert_eq!(
        describe_condition(&condition),
        "its mana value is 3 or less"
    );
}

#[test]
fn existential_countered_land_condition_keeps_plural_collection_surface() {
    let mut filter = ObjectFilter::land()
        .controlled_by(PlayerFilter::DamagedPlayer)
        .with_counter_type(crate::object::CounterType::Named("contested"));
    filter.union_surface = filter
        .union_surface
        .with_counter_requirement_surface(false, true, true);
    let condition = Condition::PlayerControls {
        player: PlayerFilter::DamagedPlayer,
        filter,
    };

    assert_eq!(
        describe_condition(&condition),
        "that player controls one or more lands with contested counters on them"
    );
}

#[test]
fn shared_owned_and_controlled_conditions_factor_the_common_player_scope() {
    let mut source = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You)
        .owned_by(PlayerFilter::You);
    source.source = true;
    source.source_surface = Some(crate::target::SourceReferenceSurface::ThisPermanentType(
        "this creature".to_string(),
    ));
    let named = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You)
        .owned_by(PlayerFilter::You)
        .named("bruna");
    let condition = Condition::And(
        Box::new(Condition::PlayerControls {
            player: PlayerFilter::You,
            filter: source,
        }),
        Box::new(Condition::PlayerControls {
            player: PlayerFilter::You,
            filter: named,
        }),
    );

    assert_eq!(
        describe_condition(&condition),
        "you both own and control this creature and a creature named bruna"
    );
}

#[test]
fn face_down_conditional_return_keeps_its_turn_followup_after_the_condition() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Return it to the battlefield face down under its owner's control, then turn it face up if that object is a permanent card."
        ),
        "Return it to the battlefield face down under its owner's control if it's a permanent card, then turn it face up."
    );
}

#[test]
fn triggered_face_down_return_moves_the_resolution_condition_before_the_followup() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever a face-down creature you control dies, return it to the battlefield face down under its owner's control, then turn it face up if that object is a permanent."
        ),
        "Whenever a face-down creature you control dies, return it to the battlefield face down under its owner's control if it's a permanent card, then turn it face up."
    );
}

#[test]
fn all_filter_renders_relation_exception_before_additional_sets() {
    let comparison = ObjectFilter::creature()
        .match_tagged("convoked_this_spell", TaggedOpbjectRelation::IsTaggedObject);
    let excepted_creatures = ObjectFilter::creature().sharing_no_characteristics_with(
        vec![crate::ObjectCharacteristic::Subtype(
            crate::types::SubtypeFamily::Creature,
        )],
        comparison,
    );
    let mut union = ObjectFilter::default();
    union.any_of = vec![
        excepted_creatures,
        ObjectFilter::artifact(),
        ObjectFilter::enchantment(),
    ];

    assert_eq!(
        describe_choose_spec(&ChooseSpec::All(union)),
        "all creatures except those that share a creature type with a creature that convoked this spell, all artifacts, and all enchantments"
    );
}

#[test]
fn relative_object_pluralization_keeps_creation_provenance_postpositive() {
    assert_eq!(
        pluralize_relative_object_phrase("token created with this enchantment"),
        "tokens created with this enchantment"
    );
}

#[test]
fn relative_object_pluralization_keeps_attachment_anchor_singular() {
    assert_eq!(
        pluralize_relative_object_phrase("Aura attached to that creature"),
        "Auras attached to that creature"
    );
}

#[test]
fn relative_object_pluralization_changes_another_to_other() {
    assert_eq!(
        pluralize_relative_object_phrase("another target planeswalker you control"),
        "other target planeswalkers you control"
    );
}

use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};

#[test]
fn cast_spell_filter_renders_instant_or_sorcery_restriction() {
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![
        ObjectFilter::default().with_type(CardType::Instant),
        ObjectFilter::default().with_type(CardType::Sorcery),
    ];

    assert_eq!(
        describe_cast_ban_spell_filter(&filter),
        "instant or sorcery spells"
    );
}

#[test]
fn cast_spell_filter_suppresses_permission_context_without_mutating_filter() {
    let mut filter = ObjectFilter::nonland().owned_by(PlayerFilter::You);
    filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(4));
    let original = filter.clone();

    assert_eq!(
        describe_cast_spell_filter(&filter, CastSpellFilterContext::EnclosingPermission),
        "spell with mana value 4 or less"
    );
    assert_eq!(
        filter, original,
        "rendering must not alter runtime legality"
    );
}

#[test]
fn cast_spell_filter_places_subtype_before_spell_noun() {
    let mut filter = ObjectFilter::nonland().owned_by(PlayerFilter::You);
    filter.subtypes.push(Subtype::Hero);

    assert_eq!(
        describe_cast_spell_filter(&filter, CastSpellFilterContext::EnclosingPermission),
        "Hero spell"
    );
}

#[test]
fn cast_spell_filter_keeps_flashback_and_graveyard_origin() {
    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    filter.alternative_cast = Some(crate::filter::AlternativeCastKind::Flashback);

    let described = describe_cast_limit_spell_filter(&filter);
    assert_eq!(described, "spell with flashback from a graveyard");
    assert_eq!(
        pluralize_cast_spell_description(&described),
        "spells with flashback from a graveyard"
    );
    assert_eq!(
        pluralize_cast_spell_description(
            "spell with mana value less than or equal to that spell's mana value"
        ),
        "spells with mana value less than or equal to that spell's mana value"
    );
    assert_eq!(
        pluralize_cast_spell_description("spell with flying or spell with haste"),
        "spells with flying or spells with haste"
    );
    assert!(!described.contains("spell matching"));
}

#[test]
fn your_turn_condition_uses_oracle_contraction() {
    assert_eq!(describe_condition(&Condition::YourTurn), "it's your turn");
}

#[test]
fn negated_source_tapped_condition_renders_as_untapped() {
    let condition = Condition::Not(Box::new(Condition::SourceIsTapped));

    assert_eq!(describe_condition(&condition), "this source is untapped");
}

#[test]
fn negated_player_cast_spell_condition_uses_havent_cast_surface() {
    let condition = Condition::Not(Box::new(Condition::PlayerCastSpellsThisTurnOrMore {
        player: PlayerFilter::You,
        count: 1,
    }));

    assert_eq!(
        describe_condition(&condition),
        "you haven't cast a spell this turn"
    );
}

#[test]
fn dice_roll_threshold_condition_uses_rolled_dice_surface() {
    let condition = Condition::ValueComparison {
        left: Value::MaxDiceRolledThisTurn(PlayerFilter::You),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(3),
    };

    assert_eq!(
        describe_condition(&condition),
        "you've rolled three or more dice this turn"
    );
}

#[test]
fn negated_source_creature_condition_uses_a_readable_permanent_subject() {
    let condition = Condition::Not(Box::new(Condition::SourceMatches(ObjectFilter::creature())));

    assert_eq!(
        describe_condition(&condition),
        "this permanent isn't a creature"
    );
}

#[test]
fn aliased_object_relative_player_uses_contextual_possessive_pronoun() {
    for player in [
        PlayerFilter::AliasedControllerOf(crate::target::ObjectRef::Target),
        PlayerFilter::AliasedOwnerOf(crate::target::ObjectRef::Target),
    ] {
        assert_eq!(describe_player_filter(&player), "that player");
        assert_eq!(describe_possessive_player_filter(&player), "their");
        assert_eq!(describe_possessive_graveyard_owner_filter(&player), "their");
    }
}

#[test]
fn describe_single_basic_land_subtype_choice_uses_singular_articles() {
    let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    filter.subtypes = vec![Subtype::Island, Subtype::Swamp];
    let condition = Condition::PlayerHasAtLeast {
        player: PlayerFilter::You,
        filter,
        count: 1,
    };

    assert_eq!(
        describe_condition(&condition),
        "you control an Island or a Swamp"
    );
}

#[test]
fn attached_and_related_creatures_render_as_a_plural_conjunction() {
    let mut attached = ObjectFilter::creature();
    attached.zone = Some(Zone::Battlefield);
    attached.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("enchanted"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let mut related = ObjectFilter::creature();
    related.zone = Some(Zone::Battlefield);
    related.tagged_constraints.extend([
        TaggedObjectConstraint {
            tag: TagKey::from("enchanted"),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        },
        TaggedObjectConstraint {
            tag: TagKey::from("enchanted"),
            relation: TaggedOpbjectRelation::SharesSubtypeWithTagged,
        },
    ]);

    let mut union = ObjectFilter::default();
    union.zone = Some(Zone::Battlefield);
    union.any_of = vec![attached, related];

    assert_eq!(
        describe_attached_and_related_creatures_filter(&union).as_deref(),
        Some("enchanted creature and other creatures that share a creature type with it")
    );

    let effect = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Filter(union),
        crate::continuous::Modification::ModifyPowerToughness {
            power: 1,
            toughness: 0,
        },
        Until::EndOfTurn,
    );
    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some(
            "enchanted creature and other creatures that share a creature type with it get +1/+0 until end of turn"
        )
    );
}

#[test]
fn negative_toughness_modifier_preserves_the_authored_negative_zero_power() {
    let effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
        crate::continuous::Modification::ModifyPowerToughness {
            power: 0,
            toughness: -3,
        },
        Until::EndOfTurn,
    );

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some("target creature gets -0/-3 until end of turn")
    );

    let mut runtime_effect = crate::effects::ApplyContinuousEffect::new_runtime(
        crate::continuous::EffectTarget::Source,
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
            power: Value::Fixed(0),
            toughness: Value::Fixed(-3),
        },
        Until::EndOfTurn,
    );
    runtime_effect.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature(),
    )));

    assert_eq!(
        describe_apply_continuous_effect(&runtime_effect).as_deref(),
        Some("target creature gets -0/-3 until end of turn")
    );
}

#[test]
fn describe_total_power_of_sacrificed_objects_keeps_the_sacrifice_link() {
    let mut filter = ObjectFilter::creature();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("sacrificed_0"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    assert_eq!(
        describe_count_filter_value_subject(&filter),
        "the sacrificed creatures"
    );
    assert_eq!(
        describe_value(&Value::TotalPower(filter)),
        "the total power of the sacrificed creatures"
    );
}

#[test]
fn greatest_mana_value_spell_history_keeps_aggregate_and_cast_surface() {
    let mut filter = ObjectFilter {
        zone: Some(Zone::Stack),
        cast_by: Some(PlayerFilter::You),
        cast_this_turn: true,
        any_of: vec![
            ObjectFilter::default().with_type(CardType::Instant),
            ObjectFilter::default().with_type(CardType::Sorcery),
        ],
        ..ObjectFilter::default()
    };
    filter.set_conjunctive_set_surface(true);

    assert_eq!(
        describe_value(&Value::GreatestManaValue(filter)),
        "the greatest mana value among instant and sorcery spells you've cast this turn"
    );
}

#[test]
fn permanent_aggregate_subject_uses_card_noun_only_outside_the_battlefield() {
    let mut battlefield = ObjectFilter::permanent_card()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    battlefield.set_explicit_card_noun(true);
    assert_eq!(
        describe_count_filter_value_subject(&battlefield),
        "permanents you control"
    );
    assert_eq!(
        describe_value(&Value::GreatestManaValue(battlefield)),
        "the greatest mana value among permanents you control"
    );

    let mut graveyard = ObjectFilter::permanent_card()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    graveyard.set_explicit_card_noun(true);
    assert_eq!(
        describe_count_filter_value_subject(&graveyard),
        "permanent cards in your graveyard"
    );
}

#[test]
fn count_subject_preserves_postpositive_controller_qualifier_order() {
    let mut numeric = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .with_power(crate::filter::Comparison::GreaterThanOrEqual(4));
    numeric.set_controller_after_qualifiers_surface(true);
    assert_eq!(
        describe_count_filter_value_subject(&numeric),
        "creatures with power 4 or greater you control"
    );

    let mut chosen = ObjectFilter::permanent().controlled_by(PlayerFilter::You);
    chosen.chosen_creature_type = true;
    chosen.set_controller_after_qualifiers_surface(true);
    assert_eq!(
        describe_count_filter_value_subject(&chosen),
        "permanents of the chosen type you control"
    );

    let mut relative = ObjectFilter::creature().controlled_by(PlayerFilter::You);
    relative.subtypes = vec![Subtype::Wolf, Subtype::Werewolf];
    relative.set_relative_characteristic_list_surface(true);
    relative.set_explicit_union_branch_articles(true);
    assert_eq!(
        describe_for_each_count_filter(&relative),
        "creature you control that's a Wolf or a Werewolf"
    );
}

#[test]
fn count_subject_uses_attacked_player_instead_of_possessive_plural() {
    let mut attacking_you = ObjectFilter::creature();
    attacking_you.attacking = true;
    attacking_you.attacking_player_only = true;
    attacking_you.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::You);

    assert_eq!(
        describe_for_each_count_filter(&attacking_you),
        "creature attacking you"
    );
}

#[test]
fn cast_time_modified_creature_snapshot_has_a_semantic_count_surface() {
    let filter = ObjectFilter::tagged(TagKey::from(ironsmith_core::CAST_MODIFIED_CREATURES_TAG));

    assert_eq!(
        describe_value(&Value::Count(filter.clone())),
        "the number of modified creatures you controlled as you cast this spell"
    );
    assert_eq!(
        describe_for_each_count_filter(&filter),
        "modified creature you controlled as you cast this spell"
    );

    let count = Value::Count(filter).with_surface_hints([
        ValueSurfaceHint::ForEach,
        ValueSurfaceHint::AdditionalPowerToughnessModifier,
        ValueSurfaceHint::DurationBeforeForEach,
    ]);
    let pump = Effect::new(crate::effects::ModifyPowerToughnessForEachEffect::new(
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
        -1,
        -1,
        count,
        Until::EndOfTurn,
    ));
    assert_eq!(
        describe_effect(&pump),
        "target creature gets an additional -1/-1 until end of turn for each modified creature you controlled as you cast this spell"
    );
}

#[test]
fn typed_sacrificed_object_references_render_characteristics_and_copy_sources() {
    let tagged = ChooseSpec::Tagged(TagKey::from("sacrificed_0"));
    let mana_value = Value::ManaValueOf(Box::new(tagged.clone())).with_surface_hints([
        ValueSurfaceHint::EqualTo,
        ValueSurfaceHint::SacrificedObject(ironsmith_core::SacrificedObjectKind::Permanent),
    ]);
    assert_eq!(
        describe_card_count(&mana_value),
        "cards equal to the sacrificed permanent's mana value"
    );

    let copy_source =
        tagged.with_surface_hint(crate::target::ChooseSpecSurfaceHint::SacrificedObject(
            ironsmith_core::SacrificedObjectKind::Creature,
        ));
    assert_eq!(
        describe_choose_spec(&copy_source),
        "the sacrificed creature"
    );
}

#[test]
fn explicit_revealed_card_reference_renders_without_losing_tag_identity() {
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(
        crate::effects::PUBLIC_REVEALED_TAG,
    ))))
    .with_surface_hints([
        ValueSurfaceHint::WhereXIs,
        ValueSurfaceHint::RevealedCardReference,
    ]);

    assert_eq!(describe_value(&value), "the revealed card's mana value");
}

#[test]
fn consult_match_mana_value_renders_as_that_card() {
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(
        "__sentence_helper_consult_match_l0_s0_e0",
    ))))
    .with_surface_hint(ValueSurfaceHint::WhereXIs);

    assert_eq!(describe_value(&value), "that card's mana value");
}

#[test]
fn tagged_characteristic_can_preserve_the_typed_this_way_object_surface() {
    let value = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from("tap_cost_0"))))
        .with_surface_hint(ValueSurfaceHint::CharacteristicOfObjectThisWay {
            card_type: crate::types::CardType::Creature,
            action: crate::effect::PriorEffectAction::Tapped,
        });

    assert_eq!(
        describe_value(&value),
        "the power of the creature tapped this way"
    );
}

#[test]
fn destroyed_result_count_can_preserve_died_this_way_surface() {
    let value = Value::PriorEffectMetric {
        effect_id: crate::effect::EffectId(7),
        query: crate::effect::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::AffectedObjects,
            crate::effect::EffectMetric::Count,
        )
        .with_filter(ObjectFilter::creature())
        .with_action(crate::effect::PriorEffectAction::Destroyed),
    }
    .with_surface_hint(ValueSurfaceHint::DiedThisWay);

    assert_eq!(
        describe_value(&value),
        "the number of creatures that died this way"
    );
}

#[test]
fn scry_event_amount_can_preserve_cards_looked_at_this_way_surface() {
    let value = Value::EventValue(crate::effect::EventValueSpec::Amount)
        .with_surface_hint(ValueSurfaceHint::CardsLookedAtWhileScryingThisWay);

    assert_eq!(
        describe_value(&value),
        "the number of cards looked at while scrying this way"
    );
}

#[test]
fn lady_loki_absolute_difference_names_both_mana_values() {
    let triggering_spell =
        Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from("triggering"))));
    let nonland_card = Value::ManaValueOf(Box::new(
        ChooseSpec::Tagged(TagKey::from("consult_match_0")).with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "that nonland card".to_string(),
                ),
            ),
        ),
    ));
    let difference = Value::absolute_difference(triggering_spell, nonland_card)
        .with_surface_hint(ValueSurfaceHint::Difference);

    assert_eq!(
        describe_value(&difference),
        "the difference between that spell's mana value and that nonland card's mana value"
    );
}

#[test]
fn chosen_creature_power_difference_names_the_selected_set() {
    let filter = ObjectFilter::creature().match_tagged(
        TagKey::from("__chosen_objects__"),
        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    );
    let difference = Value::absolute_difference(
        Value::GreatestPower(filter.clone()),
        Value::LeastPower(filter),
    )
    .with_surface_hint(ValueSurfaceHint::Difference);

    assert_eq!(
        describe_value(&difference),
        "the difference between the chosen creatures' powers"
    );
}

#[test]
fn dynamic_subtraction_renders_as_minus_instead_of_plus_negative() {
    let value = Value::Add(
        Box::new(Value::Count(ObjectFilter::creature().you_control())),
        Box::new(Value::Scaled(
            Box::new(Value::Count(
                ObjectFilter::creature().controlled_by(PlayerFilter::target_opponent()),
            )),
            -1,
        )),
    );

    let rendered = describe_value(&value);
    assert!(rendered.contains(" minus "), "{rendered}");
    assert!(!rendered.contains(" plus -"), "{rendered}");
}

#[test]
fn authored_that_player_hand_possessive_is_distinct_from_their() {
    let hand_count = Value::Count(ObjectFilter::default().in_zone(Zone::Hand).owned_by(
        PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent)),
    ));

    assert_eq!(
        describe_value(&hand_count),
        "the number of cards in their hand"
    );
    assert_eq!(
        describe_value(&hand_count.with_surface_hint(ValueSurfaceHint::ThatPlayerPossessive)),
        "the number of cards in that player's hand"
    );
}

#[test]
fn matching_filtered_hand_count_renders_as_discard_all_matching_cards() {
    let owner = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
    let mut nonland_hand = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(owner.clone());
    nonland_hand.excluded_card_types.push(CardType::Land);

    assert_eq!(
        describe_discard_count(&Value::Count(nonland_hand.clone()), Some(&nonland_hand)),
        "all nonland cards"
    );

    let whole_hand = ObjectFilter::default().in_zone(Zone::Hand).owned_by(owner);
    assert_eq!(
        describe_discard_count(&Value::Count(whole_hand), Some(&nonland_hand)),
        "the number of cards in their hand"
    );
}

#[test]
fn prior_draw_count_keeps_discard_for_each_surface() {
    let count = Value::PendingPriorEffectMetric(
        crate::effect::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::AffectedObjects,
            crate::effect::EffectMetric::Count,
        )
        .with_filter(ObjectFilter::default())
        .with_action(crate::effect::PriorEffectAction::Drawn),
    )
    .with_surface_hints([
        ValueSurfaceHint::CardsDrawnThisWay,
        ValueSurfaceHint::ForEach,
    ]);

    assert_eq!(
        describe_discard_count(&count, None),
        "a card for each card drawn this way"
    );
}

#[test]
fn authored_in_excess_of_subtraction_preserves_its_connective() {
    let value = Value::Add(
        Box::new(Value::Count(ObjectFilter::creature().you_control())),
        Box::new(Value::Scaled(
            Box::new(Value::Count(
                ObjectFilter::creature().controlled_by(PlayerFilter::target_opponent()),
            )),
            -1,
        )),
    )
    .with_surface_hint(ValueSurfaceHint::InExcessOf);

    assert_eq!(
        describe_value(&value),
        "the number of creatures you control in excess of the number of creatures target opponent controls"
    );
}

#[test]
fn counter_set_values_distinguish_among_from_explicit_on_surface() {
    let filter = ObjectFilter::creature().you_control();
    let explicit_on = Value::CountersOn(Box::new(ChooseSpec::All(filter.clone())), None);
    let aggregate_among = explicit_on
        .clone()
        .with_surface_hint(ValueSurfaceHint::CountersAmong);

    assert!(describe_value(&explicit_on).contains("counters on"));
    assert!(describe_value(&aggregate_among).contains("counters among"));
}

#[test]
fn square_bracket_cleanup_preserves_loyalty_activation_costs() {
    assert_eq!(
        strip_square_bracketed_segments(
            "Planeswalkers you control have \"[0]: Proliferate\" and \"[−12]: Take an extra turn after this one.\" [reminder]"
        ),
        "Planeswalkers you control have \"[0]: Proliferate\" and \"[−12]: Take an extra turn after this one.\""
    );
}

#[test]
fn describe_condition_uses_attached_object_for_equipped_color_checks() {
    let condition = Condition::TaggedObjectMatches(
        TagKey::from("equipped"),
        ObjectFilter::default().with_colors(crate::color::ColorSet::GREEN),
    );

    assert_eq!(describe_condition(&condition), "equipped creature is green");
}

#[test]
fn describe_condition_uses_attached_object_for_equipped_subtype_checks() {
    let condition = Condition::TaggedObjectMatches(
        TagKey::from("equipped"),
        ObjectFilter::default().with_subtype(crate::types::Subtype::Human),
    );

    assert_eq!(
        describe_condition(&condition),
        "equipped creature is a human"
    );
}

#[test]
fn describe_condition_preserves_strict_controls_more_than_each_other_player() {
    let condition = Condition::PlayerControlsMoreThanEachOtherPlayer {
        player: PlayerFilter::Active,
        filter: ObjectFilter::land(),
    };

    assert_eq!(
        describe_condition(&condition),
        "that player controls more lands than each other player"
    );
}

#[test]
fn describe_countered_permanent_condition_uses_countered_this_way_surface() {
    let condition =
        Condition::TaggedObjectMatches(TagKey::from("countered_0"), ObjectFilter::permanent());

    assert_eq!(
        describe_condition(&condition),
        "a permanent's ability is countered this way"
    );
}

#[test]
fn describe_activate_abilities_of_tagged_permanent_uses_that_permanent() {
    let mut filter = ObjectFilter::permanent();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("countered_0"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    assert_eq!(
        describe_restriction(&crate::effect::Restriction::ActivateAbilitiesOf(filter)),
        "activated abilities of that permanent can't be activated"
    );
}

#[test]
fn describe_count_condition_with_counter_uses_have_counter_clause() {
    let mut filter = ObjectFilter::permanent().controlled_by(PlayerFilter::NotYou);
    filter.zone = Some(Zone::Battlefield);
    filter.with_counter = Some(crate::filter::CounterConstraint::Typed(
        crate::object::CounterType::Named("aim"),
    ));

    assert_eq!(
        describe_condition(&Condition::ValueComparison {
            left: Value::Count(filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(2),
        }),
        "two or more permanents you don't control have an aim counter on them"
    );
}

#[test]
fn iterated_player_land_thresholds_keep_controls_surface() {
    let mut lands = ObjectFilter::land().controlled_by(PlayerFilter::IteratedPlayer);
    lands.zone = None;
    let at_least_six = Condition::ValueComparison {
        left: Value::Count(lands.clone()),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(6),
    };
    let at_most_four = Condition::ValueComparison {
        left: Value::Count(lands),
        operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
        right: Value::Fixed(4),
    };
    assert_eq!(
        describe_condition(&at_least_six),
        "that player controls six or more lands"
    );
    assert_eq!(
        describe_player_relative_condition(&at_least_six).as_deref(),
        Some("controls six or more lands")
    );
    assert_eq!(
        describe_condition(&at_most_four),
        "that player controls four or fewer lands"
    );
    assert_eq!(
        describe_player_relative_condition(&at_most_four).as_deref(),
        Some("controls four or fewer lands")
    );

    let battlefield_lands = ObjectFilter::land()
        .controlled_by(PlayerFilter::IteratedPlayer)
        .in_zone(Zone::Battlefield);
    assert_eq!(
        describe_player_relative_condition(&Condition::ValueComparison {
            left: Value::Count(battlefield_lands),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(6),
        })
        .as_deref(),
        Some("controls six or more lands")
    );
}

#[test]
fn iterated_player_count_below_global_max_keeps_relative_controls_surface() {
    assert_eq!(
        describe_condition(&Condition::ValueComparison {
            left: Value::Count(ObjectFilter::land().controlled_by(PlayerFilter::IteratedPlayer),),
            operator: crate::effect::ValueComparisonOperator::LessThan,
            right: Value::GreatestCount(ObjectFilter::land().controlled_by(PlayerFilter::Any),),
        }),
        "that player controls fewer lands than the player who controls the most lands"
    );
}

#[test]
fn iterated_player_count_below_yours_keeps_relative_controls_surface() {
    let condition = Condition::ValueComparison {
        left: Value::Count(ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer)),
        operator: crate::effect::ValueComparisonOperator::LessThan,
        right: Value::Count(ObjectFilter::creature().controlled_by(PlayerFilter::You)),
    };
    assert_eq!(
        describe_condition(&condition),
        "that player controls fewer creatures than you"
    );
    assert_eq!(
        describe_player_relative_condition(&condition).as_deref(),
        Some("controls fewer creatures than you")
    );
}

#[test]
fn describe_life_total_at_most_condition_uses_or_less_life_surface() {
    assert_eq!(
        describe_condition(&Condition::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(5),
        }),
        "you have 5 or less life"
    );
}

#[test]
fn describe_opponent_life_total_at_most_condition_uses_or_less_life_surface() {
    assert_eq!(
        describe_condition(&Condition::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::Opponent),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(0),
        }),
        "an opponent has 0 or less life"
    );
}

#[test]
fn describe_empty_library_condition_uses_no_cards_surface() {
    assert_eq!(
        describe_condition(&Condition::ValueComparison {
            left: Value::CardsInLibrary(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::Equal,
            right: Value::Fixed(0),
        }),
        "your library has no cards in it"
    );
}

#[test]
fn describe_empty_graveyard_condition_uses_no_cards_surface() {
    assert_eq!(
        describe_condition(&Condition::ValueComparison {
            left: Value::CardsInGraveyard(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(0),
        }),
        "there are no cards in your graveyard"
    );
}

#[test]
fn describe_opponent_poison_threshold_uses_player_predicate_surface() {
    assert_eq!(
        describe_condition(&Condition::PlayerHasPoisonCountersOrMore {
            player: PlayerFilter::Opponent,
            count: 3,
        }),
        "an opponent has three or more poison counters"
    );
}

#[test]
fn describe_player_counter_value_poison_threshold_uses_player_predicate_surface() {
    assert_eq!(
        describe_condition(&Condition::ValueComparison {
            left: Value::PlayerCounters(PlayerFilter::Opponent, CounterType::Poison),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(3),
        }),
        "an opponent has three or more poison counters"
    );
}

#[test]
fn describe_crime_condition_uses_authored_present_perfect_surface() {
    assert_eq!(
        describe_condition(&Condition::PlayerCommittedCrimeThisTurn {
            player: PlayerFilter::You,
        }),
        "you've committed a crime this turn"
    );
    assert_eq!(
        describe_condition(&Condition::PlayerCommittedCrimeThisTurn {
            player: PlayerFilter::Opponent,
        }),
        "an opponent has committed a crime this turn"
    );
}

#[test]
fn temporary_generic_granted_trigger_renders_quoted_oracle_surface() {
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
    let effect = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Filter(
            ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent),
        ),
        crate::continuous::Modification::AddAbilityGeneric(ability),
        Until::EndOfTurn,
    );

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some(
            "Until end of turn, permanents an opponent controls gain \"When this permanent deals damage to the player who cast this spell, sacrifice this permanent. You lose 2 life.\""
        )
    );
}

#[test]
fn temporary_granted_rule_restriction_renders_as_a_quoted_ability() {
    let mut affected = ObjectFilter::creature();
    affected.dealt_damage_by_source_this_turn = Some(ironsmith_core::DamagedBySource::ThisCreature);
    let ability = crate::static_abilities::StaticAbility::restriction(
        crate::effect::Restriction::be_regenerated(affected),
        "Creatures dealt damage by this creature this turn can't be regenerated this turn"
            .to_string(),
    );
    let effect = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(ability),
        Until::EndOfTurn,
    )
    .with_source_reference_surface(crate::target::SourceReferenceSurface::ThisPermanentType(
        "this creature".to_string(),
    ));

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some(
            "Until end of turn, this creature gains \"Creatures dealt damage by this creature this turn can't be regenerated this turn.\""
        )
    );
}

#[test]
fn executable_annihilator_grant_renders_as_its_keyword() {
    let ability = Ability::triggered(
        crate::triggers::Trigger::this_attacks(),
        vec![Effect::sacrifice_player(
            ObjectFilter::permanent(),
            Value::Fixed(2),
            PlayerFilter::Defending,
        )],
    );
    let effect = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbilityGeneric(ability),
        Until::EndOfTurn,
    )
    .with_source_reference_surface(crate::target::SourceReferenceSurface::ThisPermanentType(
        "this creature".to_string(),
    ));

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some("this creature gains annihilator 2 until end of turn")
    );
}

#[test]
fn temporary_additional_blocker_grant_uses_action_surface() {
    let effect = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::can_block_additional_creature_each_combat(1),
        ),
        Until::EndOfTurn,
    )
    .with_source_reference_surface(crate::target::SourceReferenceSurface::ThisPermanentType(
        "this creature".to_string(),
    ));

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some("this creature can block an additional creature this turn")
    );
}

#[test]
fn turn_long_global_x_block_cost_renders_as_a_per_blocker_tax() {
    let dynamic_x = crate::costs::Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(
        crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::X]]),
        None,
        None,
        None,
        ironsmith_core::DynamicManaDisplayHint::Default,
    ));
    let block_cost = crate::static_abilities::StaticAbility::block_cost(
        ObjectFilter::source(),
        ObjectFilter::creature(),
        crate::cost::TotalCost::from_cost(dynamic_x),
        "display text is not a rendering input",
    );
    let effect = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Filter(ObjectFilter::creature()),
        crate::continuous::Modification::AddAbility(block_cost),
        Until::EndOfTurn,
    );

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some(
            "This turn, creatures can't block unless their controller pays {X} for each blocking creature they control"
        )
    );
}

#[test]
fn executable_sunburst_grant_compacts_only_the_complete_counter_bundle() {
    fn compiled_static(
        model: crate::static_abilities::CompiledStaticAbility,
    ) -> crate::static_abilities::StaticAbility {
        crate::static_abilities::StaticAbility::from_model(model)
    }

    let creature_filter = ObjectFilter::creature();
    let creature_counters: crate::static_abilities::CompiledStaticAbility =
        ironsmith_core::StaticAbility::enters_with_counters_value(
            CounterType::PlusOnePlusOne,
            Value::ColorsOfManaSpentToCastThisSpell,
        )
        .with_condition(Condition::SourceMatches(creature_filter.clone()));
    let charge_counters: crate::static_abilities::CompiledStaticAbility =
        ironsmith_core::StaticAbility::enters_with_counters_value(
            CounterType::Charge,
            Value::ColorsOfManaSpentToCastThisSpell,
        )
        .with_condition(Condition::Not(Box::new(Condition::SourceMatches(
            creature_filter,
        ))));
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Tagged(TagKey::from("triggering")),
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::keyword_marker("sunburst"),
        ),
        Until::Forever,
    );
    effect.additional_modifications.extend([
        crate::continuous::Modification::AddAbility(compiled_static(creature_counters)),
        crate::continuous::Modification::AddAbility(compiled_static(charge_counters)),
    ]);

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some("it gains sunburst")
    );
}

#[test]
fn executable_vanishing_pair_compacts_only_the_complete_keyword_bundle() {
    let upkeep = Ability::triggered(
        crate::triggers::Trigger::beginning_of_upkeep(PlayerFilter::You),
        vec![Effect::remove_counters(
            CounterType::Time,
            1,
            ChooseSpec::Source,
        )],
    );
    let last_counter = Ability::triggered(
        crate::triggers::Trigger::custom(
            "vanishing-last-time-counter-removed",
            "when the last time counter is removed".to_string(),
        ),
        vec![Effect::sacrifice_source()],
    );
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Tagged(TagKey::from("triggering")),
        crate::continuous::Modification::AddAbilityGeneric(upkeep),
        Until::Forever,
    );
    effect
        .additional_modifications
        .push(crate::continuous::Modification::AddAbilityGeneric(
            last_counter,
        ));

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some("it gains vanishing")
    );
}

#[test]
fn source_generic_granted_trigger_uses_source_surface_for_target() {
    let surface =
        crate::target::SourceReferenceSurface::ThisPermanentType("this creature".to_string());
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::new(
            crate::triggers::zone_changes::ZoneChangeTrigger::this_leaves_battlefield()
                .this_surface(surface.clone()),
        ),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
            crate::effects::DrawCardsEffect::new(Value::Fixed(1), PlayerFilter::Opponent),
        )]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };
    let ability = Ability {
        kind: AbilityKind::Triggered(triggered),
        functional_zones: vec![Zone::Battlefield],
    };
    let effect = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbilityGeneric(ability),
        Until::Forever,
    )
    .with_source_reference_surface(surface);

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some(
            "this creature gains \"When this creature leaves the battlefield, an opponent draws a card.\""
        )
    );
}

#[test]
fn implicit_source_zone_change_can_use_a_contextual_pronoun() {
    let ability = Ability {
        kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::this_leaves_battlefield(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::DrawCardsEffect::new(Value::Fixed(1), PlayerFilter::Opponent),
            )]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    assert_eq!(
        describe_inline_ability_with_self_subject(&ability, "it"),
        "When it leaves the battlefield, an opponent draws a card"
    );
}

#[test]
fn source_generic_static_ability_removal_renders_typed_ability_and_duration() {
    let effect = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::RemoveAbilityGeneric {
            ability: Ability::static_ability(crate::static_abilities::StaticAbility::trample()),
            mode: ironsmith_core::AbilityLossMode::Lose,
        },
        Until::EndOfTurn,
    )
    .with_source_reference_surface(crate::target::SourceReferenceSurface::ThisPermanentType(
        "this creature".to_string(),
    ));

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some("this creature loses trample until end of turn")
    );
}

#[test]
fn typed_apply_continuous_target_takes_precedence_over_legacy_source_fallback() {
    let effect = crate::effects::ApplyContinuousEffect::with_spec_runtime(
        ChooseSpec::Target(Box::new(ChooseSpec::Object(ObjectFilter::creature()))),
        crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
        Until::SourceUntaps,
    )
    .with_condition(Condition::SourceIsTapped)
    .with_source_reference_surface(crate::target::SourceReferenceSurface::ThisPermanentType(
        "this creature".to_string(),
    ));

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some(
            "Gain control of target creature for as long as you control this creature and this creature remains tapped"
        )
    );
}

#[test]
fn normalize_for_each_opponent_clause_inside_trigger_text() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "At the beginning of your end step, for each opponent, that player loses 3 life."
        ),
        "At the beginning of your end step, each opponent loses 3 life."
    );
}

#[test]
fn normalize_card_ins_typo_does_not_corrupt_instead() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "If a card would be put into your graveyard from anywhere this turn, exile that card instead"
        ),
        "If a card would be put into your graveyard from anywhere this turn, exile that card instead"
    );
}

#[test]
fn normalize_then_if_saga_counter_followup_removes_sequence_scaffolding() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "It gains haste. Then if it's a Saga, put up to three lore counters on it."
        ),
        "It gains haste. If it's a Saga, put up to three lore counters on it."
    );
}

#[test]
fn normalize_token_death_quote_uses_it_for_token_damage() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Create three 1/1 red Devil creature tokens with \"When this token dies, this token deals 1 damage to any target\""
        ),
        "Create three 1/1 red Devil creature tokens with \"When this token dies, it deals 1 damage to any target\""
    );
}

#[test]
fn normalize_another_creatures_plural_typo_without_touching_singular() {
    assert_eq!(
        normalize_common_semantic_phrasing("another creatures you control get +1/+1."),
        "other creatures you control get +1/+1."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever another creature you control enters, you gain 1 life."
        ),
        "Whenever another creature you control enters, you gain 1 life."
    );
}

#[test]
fn normalize_composed_quantifiers_articles_and_plural_references() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever a red source you control deals damage to one or more one or more permanents and/or players, put a +1/+1 counter on this creature."
        ),
        "Whenever a red source you control deals damage to one or more permanents and/or players, put a +1/+1 counter on this creature."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "They reveal cards from the top of their library until they reveal creature card."
        ),
        "They reveal cards from the top of their library until they reveal a creature card."
    );
    assert_eq!(
        normalize_common_semantic_phrasing("Return those creatures to its owner's hand."),
        "Return those creatures to their owners' hands."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Other creatures you control get +2/+2 and gains trample until end of turn."
        ),
        "Other creatures you control get +2/+2 and gain trample until end of turn."
    );
}

#[test]
fn normalize_plural_counter_qualifier_before_controller() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Other creatures with counters on them you control have flying and haste."
        ),
        "Other creatures you control with counters on them have flying and haste."
    );
}

#[test]
fn describe_tagged_object_matches_simple_subtype_uses_is_clause() {
    let mut filter = ObjectFilter::default();
    filter.subtypes = vec![Subtype::Spider];
    let condition = Condition::TaggedObjectMatches(TagKey::from("triggering"), filter);

    assert_eq!(describe_condition(&condition), "that object is a Spider");
}

#[test]
fn describe_triggering_graveyard_origin_uses_event_object_pronoun() {
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![
        ObjectFilter::permanent().owned_by(PlayerFilter::You),
        ObjectFilter::permanent()
            .owned_by(PlayerFilter::You)
            .cast_by(PlayerFilter::You),
    ];
    let condition = Condition::TaggedObjectMatches(TagKey::from("triggering"), filter);

    assert_eq!(
        describe_condition(&condition),
        "that object entered from your graveyard or you cast it from your graveyard"
    );
}

#[test]
fn describe_triggering_historic_check_uses_event_object_pronoun() {
    let condition = Condition::TaggedObjectMatches(
        TagKey::from("triggering"),
        ObjectFilter::permanent().historic(),
    );

    assert_eq!(describe_condition(&condition), "that object was historic");
}

#[test]
fn describe_triggering_power_check_uses_event_object_pronoun() {
    let mut filter = ObjectFilter::permanent();
    filter.power = Some(ironsmith_core::FilterComparison::GreaterThanOrEqual(3));
    let condition = Condition::TaggedObjectMatches(TagKey::from("triggering"), filter);

    assert_eq!(
        describe_condition(&condition),
        "that object's power is 3 or greater"
    );
}

#[test]
fn describe_last_known_tagged_conditions_preserves_past_tense_and_negation() {
    let triggering = TagKey::from("triggering");

    assert_eq!(
        describe_condition(&Condition::TaggedObjectMatchedLastKnown(
            triggering.clone(),
            ObjectFilter::creature(),
        )),
        "it was a creature"
    );

    let horror = ObjectFilter::default().with_subtype(Subtype::Horror);
    assert_eq!(
        describe_condition(&Condition::TaggedObjectMatchedLastKnown(
            triggering.clone(),
            horror,
        )),
        "it was a Horror"
    );

    let mut power = ObjectFilter::default();
    power.power = Some(ironsmith_core::FilterComparison::GreaterThanOrEqual(3));
    assert_eq!(
        describe_condition(&Condition::TaggedObjectMatchedLastKnown(
            triggering.clone(),
            power,
        )),
        "its power was 3 or greater"
    );

    let demon = ObjectFilter::default().with_subtype(Subtype::Demon);
    assert_eq!(
        describe_condition(&Condition::Not(Box::new(
            Condition::TaggedObjectMatchedLastKnown(triggering, demon),
        ))),
        "it wasn't a Demon"
    );

    let mut legendary_spell = ObjectFilter::spell();
    legendary_spell.supertypes = vec![crate::types::Supertype::Legendary];
    assert_eq!(
        describe_condition(&Condition::TaggedObjectMatchedLastKnown(
            TagKey::from("countered_0"),
            legendary_spell,
        )),
        "it was a legendary spell"
    );

    let jace_spell = ObjectFilter::spell()
        .with_type(CardType::Planeswalker)
        .with_subtype(Subtype::Jace);
    assert_eq!(
        describe_condition(&Condition::TaggedObjectMatchedLastKnown(
            TagKey::from("countered_0"),
            jace_spell,
        )),
        "it was a Jace planeswalker spell"
    );
}

#[test]
fn describe_past_controller_tagged_condition_uses_authored_control_surface() {
    let mut filter = ObjectFilter::default();
    filter.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Permanent,
    ));
    let condition = Condition::PlayerTaggedObjectMatches {
        player: PlayerFilter::You,
        tag: TagKey::from("returned_0"),
        filter,
        mode: crate::effect::TaggedObjectMatchMode::LastKnown,
    };

    assert_eq!(
        describe_condition(&condition),
        "you controlled that permanent"
    );
}

#[test]
fn describe_tagged_object_matches_all_card_types_uses_is_clause() {
    let mut filter = ObjectFilter::default();
    filter.all_card_types = vec![CardType::Artifact, CardType::Creature];
    let condition = Condition::TaggedObjectMatches(TagKey::from("pumped_0"), filter);

    assert_eq!(describe_condition(&condition), "it's an artifact creature");
}

#[test]
fn describe_revealed_tagged_object_matches_all_card_types_uses_card_clause() {
    let mut filter = ObjectFilter::default();
    filter.all_card_types = vec![CardType::Artifact, CardType::Creature];
    let condition =
        Condition::TaggedObjectMatches(TagKey::from("__sentence_helper_revealed_l0_s0_e0"), filter);

    assert_eq!(
        describe_condition(&condition),
        "it's an artifact creature card"
    );
}

#[test]
fn describe_revealed_tagged_object_action_keeps_card_type_qualifiers() {
    let mut filter = ObjectFilter::creature();
    filter.mana_value = Some(ironsmith_core::FilterComparison::LessThanOrEqual(3));
    let condition =
        Condition::TaggedObjectMatches(TagKey::from("__sentence_helper_revealed_l0_s0_e0"), filter);

    assert_eq!(
        describe_condition(&condition),
        "a creature card with mana value 3 or less was revealed this way"
    );
}

#[test]
fn graveyard_scoped_discard_tag_preserves_put_this_way_surface() {
    let any_graveyard = ObjectFilter::land()
        .in_zone(Zone::Graveyard)
        .match_tagged("discarded_0", TaggedOpbjectRelation::IsTaggedObject);
    assert_eq!(
        describe_for_each_count_filter(&any_graveyard),
        "land card put into a graveyard this way"
    );

    let your_graveyard = any_graveyard.owned_by(PlayerFilter::You);
    assert_eq!(
        describe_for_each_count_filter(&your_graveyard),
        "land card put into your graveyard this way"
    );
}

#[test]
fn count_filter_places_controller_before_restrictive_qualifiers() {
    let keyword_filter = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .with_static_ability(crate::static_abilities::StaticAbilityId::Deathtouch);
    assert_eq!(
        describe_for_each_count_filter(&keyword_filter),
        "creature you control with deathtouch"
    );

    let mana_value_filter = ObjectFilter::spell()
        .controlled_by(PlayerFilter::You)
        .with_mana_value(ironsmith_core::FilterComparison::LessThanOrEqual(2));
    assert_eq!(
        describe_for_each_count_filter(&mana_value_filter),
        "spell you control with mana value 2 or less"
    );
}

#[test]
fn count_filter_combines_controller_and_owner_scope() {
    let permanent = ObjectFilter::permanent()
        .controlled_by(PlayerFilter::You)
        .owned_by(PlayerFilter::NotYou);

    assert_eq!(
        describe_for_each_count_filter(&permanent),
        "permanent you control but don't own"
    );
}

#[test]
fn exile_count_keeps_all_opponents_as_the_owner_scope() {
    let exiled_cards = ObjectFilter::default()
        .in_zone(Zone::Exile)
        .owned_by(PlayerFilter::Opponent);

    assert_eq!(
        describe_for_each_count_filter(&exiled_cards),
        "card your opponents own in exile"
    );
}

#[test]
fn goad_target_preserves_relative_enchanted_by_attachment_filter() {
    let mut host = ObjectFilter::creature().controlled_by(PlayerFilter::Opponent);
    host.with_attached_object = Some(Box::new(
        ObjectFilter::default()
            .with_subtype(Subtype::Aura)
            .controlled_by(PlayerFilter::You),
    ));
    host.set_relative_attachment_state_surface(true);

    assert_eq!(
        describe_goad_target(&ChooseSpec::All(host)),
        "each creature an opponent controls that's enchanted by an Aura you control"
    );
}

#[test]
fn describe_revealed_tagged_object_keeps_shared_creature_type_relation() {
    let mut filter = ObjectFilter::creature();
    filter.shares_creature_type_with_source = true;
    let condition =
        Condition::TaggedObjectMatches(TagKey::from("__sentence_helper_revealed_l0_s0_e0"), filter);

    assert_eq!(
        describe_condition(&condition),
        "it shares a creature type with this creature"
    );
}

#[test]
fn describe_revealed_set_shared_card_type_uses_card_set_and_triggering_spell() {
    let mut filter = ObjectFilter::default();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("triggering"),
        relation: TaggedOpbjectRelation::SharesCardType,
    });
    let condition =
        Condition::TaggedObjectMatches(TagKey::from("__sentence_helper_revealed_l0_s0_e0"), filter);

    assert_eq!(
        describe_condition(&condition),
        "any of those cards shares a card type with that spell"
    );
}

#[test]
fn describe_tagged_object_same_name_comparison_preserves_the_comparison_set() {
    let mut graveyard = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    graveyard.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("__it__"),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    let condition = Condition::TaggedObjectMatches(
        TagKey::from("__sentence_helper_revealed_l0_s0_e0"),
        graveyard,
    );

    assert_eq!(
        describe_condition(&condition),
        "it has the same name as a card in your graveyard"
    );

    let mut permanents = ObjectFilter::permanent();
    permanents.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("__it__"),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    let condition = Condition::TaggedObjectMatches(
        TagKey::from("__sentence_helper_revealed_l0_s0_e0"),
        permanents,
    );
    assert_eq!(
        describe_condition(&condition),
        "it has the same name as a permanent"
    );
}

#[test]
fn describe_blocking_tag_uses_that_creature() {
    assert_eq!(
        describe_choose_spec(&ChooseSpec::Tagged(TagKey::from("blocking"))),
        "that creature"
    );
}

#[test]
fn describe_target_face_up_exiled_card_uses_exiled_surface() {
    let spec = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default().in_zone(Zone::Exile).face_up(),
    ));

    assert_eq!(describe_choose_spec(&spec), "target face-up exiled card");
}

#[test]
fn describe_object_or_player_union_only_adds_target_for_target_wrapper() {
    let union = ChooseSpec::ObjectOrPlayer(
        ObjectFilter::default().with_type(CardType::Battle).other(),
        PlayerFilter::Opponent,
    );

    assert_eq!(describe_choose_spec(&union), "another battle or opponent");
    assert_eq!(
        describe_choose_spec(&ChooseSpec::target(union)),
        "another target battle or opponent"
    );
}

#[test]
fn counted_other_targets_use_plural_other_but_keep_singular_another() {
    let other_creature = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().other()));

    assert_eq!(
        describe_choose_spec(&other_creature.clone().with_count(ChoiceCount::up_to(3))),
        "up to three other target creatures"
    );
    assert_eq!(
        describe_choose_spec(&other_creature.with_count(ChoiceCount::up_to(1))),
        "up to one other target creature"
    );
    assert_eq!(
        describe_choose_spec(
            &ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
                .with_count(ChoiceCount::up_to(3)),
        ),
        "up to three target creatures"
    );
}

#[test]
fn describe_generic_object_or_player_union() {
    let union = ChooseSpec::target(ChooseSpec::ObjectOrPlayer(
        ObjectFilter::creature(),
        PlayerFilter::Any,
    ));

    assert_eq!(describe_choose_spec(&union), "target creature or player");
}

#[test]
fn describe_target_fixed_effective_pt_uses_creature_shorthand() {
    let mut filter = ObjectFilter::creature().you_control();
    filter.power = Some(ironsmith_core::FilterComparison::Equal(1));
    filter.toughness = Some(ironsmith_core::FilterComparison::Equal(1));
    let spec = ChooseSpec::target(ChooseSpec::Object(filter));

    assert_eq!(
        describe_choose_spec(&spec),
        "target 1/1 creature you control"
    );
}

#[test]
fn describe_target_fixed_pt_preserves_comparisons_and_base_reference() {
    let mut power_limited = ObjectFilter::creature();
    power_limited.power = Some(ironsmith_core::FilterComparison::LessThanOrEqual(2));
    let power_limited = ChooseSpec::target(ChooseSpec::Object(power_limited));
    assert_eq!(
        describe_choose_spec(&power_limited),
        "target creature with power 2 or less"
    );

    let mut base_pt = ObjectFilter::creature();
    base_pt.power = Some(ironsmith_core::FilterComparison::Equal(1));
    base_pt.toughness = Some(ironsmith_core::FilterComparison::Equal(1));
    base_pt.power_reference = ironsmith_core::PtReference::Base;
    base_pt.toughness_reference = ironsmith_core::PtReference::Base;
    let base_pt = ChooseSpec::target(ChooseSpec::Object(base_pt));
    assert_eq!(
        describe_choose_spec(&base_pt),
        "target creature with base power and toughness 1/1"
    );
}

#[test]
fn describe_colors_among_sacrificed_creature_uses_was_surface() {
    let filter = ObjectFilter::creature()
        .match_tagged("sacrificed_0", TaggedOpbjectRelation::IsTaggedObject);

    assert_eq!(describe_colors_among(&filter), "colors that creature was");
}

#[test]
fn normalize_demotes_sentence_leading_target_that_references() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Choose target permanent. Target that permanent doesn't untap during its controller's next untap step."
        ),
        "Choose target permanent. That permanent doesn't untap during its controller's next untap step."
    );
}

#[test]
fn normalize_keeps_it_reference_for_tap_freeze_text() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "If you roll 10-20, tap it. That permanent doesn't untap during its controller's next untap step."
        ),
        "If you roll 10-20, tap it. It doesn't untap during its controller's next untap step."
    );
}

#[test]
fn normalize_target_that_player_creature_possessive() {
    assert_eq!(
        normalize_common_semantic_phrasing("Strax fights another target that player's creature."),
        "Strax fights another target creature that player controls."
    );
}

#[test]
fn normalize_repeated_process_once_surface_compacts_structural_duplicate_halves() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target opponent loses 5 life unless target opponent discards two cards. Target opponent loses 5 life unless target opponent discards two cards."
        ),
        "Target opponent loses 5 life unless that player discards two cards. Repeat this process once."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target opponent loses 5 life unless target opponent discards two cards or sacrifices a creature or planeswalker of their choice, then target opponent loses 5 life unless target opponent discards two cards or sacrifices a creature or planeswalker of their choice."
        ),
        "Target opponent loses 5 life unless that player discards two cards or sacrifices a creature or planeswalker of their choice. Repeat this process once."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "You choose a nonlegendary creature. Create a token that's a copy of that object. You choose a nonlegendary creature. Create a token that's a copy of that object."
        ),
        "You choose a nonlegendary creature. Create a token that's a copy of that object. Repeat this process once."
    );
    assert_eq!(
        normalize_common_semantic_phrasing("Draw a card. Draw a card."),
        "Draw a card. Draw a card."
    );
}

#[test]
fn normalize_repeated_named_counter_recipient_to_damage_pronoun() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever you cast a noncreature spell, put a +1/+1 counter on Vivi Ornitier and Vivi Ornitier deals 1 damage to each opponent."
        ),
        "Whenever you cast a noncreature spell, put a +1/+1 counter on Vivi Ornitier and it deals 1 damage to each opponent."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Put a +1/+1 counter on each creature and each creature deals 1 damage."
        ),
        "Put a +1/+1 counter on each creature and each creature deals 1 damage."
    );
}

#[test]
fn normalize_multi_zone_named_search_to_battlefield_compacts_followup() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{T}, Sacrifice three Clerics: You search your graveyard, hand, and library for a permanent named scion of darkness, for each card searched for this way, put them onto the battlefield, then shuffle your library."
        ),
        "{T}, Sacrifice three Clerics: Search your graveyard, hand, and/or library for a card named Scion of Darkness and put it onto the battlefield. If you search your library this way, shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{T}, Sacrifice three Clerics: Search your graveyard, hand, and library for a permanent named scion of darkness, for each card searched for this way, put them onto the battlefield, then shuffle your library."
        ),
        "{T}, Sacrifice three Clerics: Search your graveyard, hand, and/or library for a card named Scion of Darkness and put it onto the battlefield. If you search your library this way, shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When Kassandra enters, you search your graveyard, hand, and library for a permanent named the spear of leonidas, for each card searched for this way, put them onto the battlefield, then shuffle your library."
        ),
        "When Kassandra enters, search your graveyard, hand, and library for a card named The Spear of Leonidas, put it onto the battlefield, then shuffle."
    );
}

#[test]
fn normalize_named_library_graveyard_search_to_hand_surfaces() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When this creature enters, you may search your library and/or graveyard for a card named huatli dinosaur knight, reveal it, and put it into your hand. If you do, shuffle your library."
        ),
        "When this creature enters, you may search your library and/or graveyard for a card named Huatli Dinosaur Knight, reveal it, and put it into your hand. If you search your library this way, shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Search your library for a basic land card, put it onto the battlefield tapped, you search your library and/or graveyard for a card named nissa natures artisan, reveal it, put it into your hand. If you search your library this way, shuffle your library."
        ),
        "Search your library for a basic land card and put it onto the battlefield tapped. Search your library and graveyard for a card named Nissa Natures Artisan, reveal it, put it into your hand, then shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Search your library for a basic land card, put it onto the battlefield tapped, search your library and/or graveyard for a card named nissa natures artisan, reveal it, put it into your hand. If you search your library this way, shuffle your library."
        ),
        "Search your library for a basic land card and put it onto the battlefield tapped. Search your library and graveyard for a card named Nissa Natures Artisan, reveal it, put it into your hand, then shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Search your library for a basic land card, put it onto the battlefield tapped. You search your library and/or graveyard for a card named nissa natures artisan. Reveal it. Put it into your hand. Then if you search your library this way, shuffle."
        ),
        "Search your library for a basic land card and put it onto the battlefield tapped. Search your library and graveyard for a card named Nissa Natures Artisan, reveal it, put it into your hand, then shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Spell effects: Search your library for a basic land card, put it onto the battlefield tapped. You search your library and/or graveyard for a card named nissa natures artisan. Reveal it. Put it into your hand. Then if you search your library this way, shuffle."
        ),
        "Spell effects: Search your library for a basic land card and put it onto the battlefield tapped. Search your library and graveyard for a card named Nissa Natures Artisan, reveal it, put it into your hand, then shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Search your library for a basic land card, put it onto the battlefield tapped. You search your library and/or graveyard for a card named nissa natures artisan. Reveal it. Put it into your hand. Then if you search your library this way, shuffle your library."
        ),
        "Search your library for a basic land card and put it onto the battlefield tapped. Search your library and graveyard for a card named Nissa Natures Artisan, reveal it, put it into your hand, then shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Search your library for a basic land card, put it onto the battlefield tapped. You search your library and/or graveyard for a card named nissa natures artisan. Reveal it. For each card searched for this way, put it into its owner's hand. Then if you search your library this way, shuffle."
        ),
        "Search your library for a basic land card and put it onto the battlefield tapped. Search your library and graveyard for a card named Nissa Natures Artisan, reveal it, put it into your hand, then shuffle."
    );
}

#[test]
fn normalize_split_multi_zone_search_reveal_hand_shuffle_surface() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When this creature enters, you search your library and/or graveyard for an artifact with mana value 2 or less you own. Reveal it. Put it into your hand. Then if you search your library this way, shuffle your library."
        ),
        "When this creature enters, search your library and/or graveyard for an artifact card with mana value 2 or less you own, reveal it, and put it into your hand. If you search your library this way, shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When this siege enters, you search your library, graveyard, and/or outside the game for an instant or sorcery you own. Reveal it. Put it into your hand. Then if you search your library this way, shuffle your library."
        ),
        "When this Siege enters, search your library, graveyard, and/or outside the game for an instant or sorcery card you own, reveal it, and put it into your hand. If you search your library this way, shuffle."
    );
}

#[test]
fn normalize_optional_multi_zone_search_shuffle_linkage() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When this creature enters, you may search your graveyard, hand, and/or library for an Aura card and put it onto the battlefield attached to this creature. If you do, shuffle your library."
        ),
        "When this creature enters, you may search your graveyard, hand, and/or library for an Aura card and put it onto the battlefield attached to this creature. If you search your library this way, shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When this creature enters, you may search your library and/or graveyard for a Rune card, reveal it, and put it into your hand. If you do, shuffle your library."
        ),
        "When this creature enters, you may search your library and/or graveyard for a Rune card, reveal it, and put it into your hand. If you search your library this way, shuffle."
    );
}

#[test]
fn normalize_bionic_blow_omits_tautological_x_tail() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target creature you control gets +X/+0 until end of turn, where X is X, then that creature deals damage equal to its power to up to one other target creature."
        ),
        "Target creature you control gets +X/+0 until end of turn, then that creature deals damage equal to its power to up to one other target creature."
    );
}

#[test]
fn normalize_bottom_bucket_looked_card_and_delirium_surfaces() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Look at the top three cards of your library, choose a card, choose an other card, choose an other other card, return it to its owner's hand, put it on the bottom of its owner's library, exile it, then you may play those cards this turn."
        ),
        "Look at the top three cards of your library. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may play the exiled card this turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Counter target sorcery spell. If there are four or more card types among cards in your graveyard, you search its controller's graveyard, hand, and library for any number permanents with the same name as that object that object's controller owns. For each card searched for this way, exile them. If you searched your library this way, shuffle its controller's library. Shuffle their library."
        ),
        "Counter target sorcery spell. Delirium — If there are four or more card types among cards in your graveyard, search the graveyard, hand, and library of that spell's controller for any number of cards with the same name as that spell, exile those cards, then that player shuffles."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{3}{U}, {T}: Look at the top X cards of your library, where X is the greatest mana value among artifacts you control. You may choose an artifact card. For each card chosen this way, put that object onto the battlefield. For each card chosen this way, Unless it's a permanent, put that object on the bottom of its owner's library."
        ),
        "{3}{U}, {T}: Look at the top X cards of your library, where X is the greatest mana value among artifacts you control. You may put an artifact card from among them onto the battlefield. Put the rest on the bottom of your library in any order."
    );
}

#[test]
fn normalize_lithobraking_if_you_do_surface() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Create a Lander token. You may sacrifice an artifact. If you do, Lithobraking deals 2 damage to each creature."
        ),
        "Create a Lander token. You may sacrifice an artifact. If you do, Lithobraking deals 2 damage to each creature."
    );
}

#[test]
fn normalize_bottom_batch_look_choose_and_hand_choice_surfaces() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever a creature enters, lose 1 life, then add {B}."
        ),
        "Whenever a creature enters, you lose 1 life and add {B}."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "This creature gets +X/+0, where X is the number of artifact cards in your graveyard."
        ),
        "This creature gets +1/+0 for each artifact card in your graveyard."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Exile Time Spiral, each player shuffles their hand and graveyard into their library, each player draws seven cards, then untap up to six lands."
        ),
        "Exile Time Spiral. Each player shuffles their hand and graveyard into their library, then draws seven cards. You untap up to six lands."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target opponent reveals their hand. You choose a nonland card from it and exile that card. If there are four or more card types among cards in your graveyard, target opponent chooses a card exiled with this source. You search target opponent's graveyard, hand, and library for any number permanents with the same name as that object target opponent owns. For each card searched for this way, exile them. If you searched your library this way, shuffle target opponent's library. Shuffle target opponent's library."
        ),
        "Target opponent reveals their hand. You choose a nonland card from it and exile that card. Delirium — If there are four or more card types among cards in your graveyard, search that player's graveyard, hand, and library for any number of cards with the same name as the exiled card, exile those cards, then that player shuffles."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When this creature enters, a player may choose two creatures on the battlefield. Sacrifice all permanents. If a player does, sacrifice this creature."
        ),
        "When this creature enters, any player may sacrifice two creatures of their choice. If a player does, sacrifice this creature."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever you attack, you may sacrifice another creature. If you do, choose two —\n• Create two 1/1 red and white Soldier creature tokens with haste that are tapped and attacking.\n• Draw a card and you lose 1 life.\n• Caesar deals damage equal to the number of creature tokens you control to target opponent."
        ),
        "Whenever you attack, you may sacrifice another creature. When you do, choose two —\n• Create two 1/1 red and white Soldier creature tokens with haste that are tapped and attacking.\n• You draw a card and you lose 1 life.\n• Caesar deals damage equal to the number of creature tokens you control to target opponent."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{1}{W}: Search your library for an enchantment card, reveal it, put it into your hand, discard a card at random, then shuffle your library."
        ),
        "{1}{W}: Search your library for an enchantment card and reveal that card. Put it into your hand, then discard a card at random. Then shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Domain — When Bortuk Bonerattle enters, if you cast it, choose target creature card in your graveyard. Then if its mana value is a dynamic value or less, return it from graveyard to the battlefield. Otherwise, return it to its owner's hand."
        ),
        "Domain — When Bortuk Bonerattle enters, if you cast it, choose target creature card in your graveyard. Return that card to the battlefield if its mana value is less than or equal to the number of basic land types among lands you control. Otherwise, put it into your hand."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever a land an opponent controls enters, if the number of lands that entered the battlefield under that player's control this turn is greater than or equal to 2, this creature deals 3 damage to that object's controller."
        ),
        "Whenever a land enters under an opponent's control, if that player had another land enter the battlefield under their control this turn, this creature deals 3 damage to that player."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{T}: Choose target creature card in an opponent's graveyard. Then if its mana value is the number of a ally you control or less, put target creature card in an opponent's graveyard onto the battlefield under your control."
        ),
        "{T}: Put target creature card from an opponent's graveyard onto the battlefield under your control if its mana value is less than or equal to the number of Allies you control."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Equipped creature has {2}: This creature gets +1/+0 until end of turn.\nWhenever equipped creature deals damage to blocking creature, this Equipment deals that much damage to each other defending player's creature.\nEquip {3}"
        ),
        "Equipped creature has \"{2}: This creature gets +1/+0 until end of turn.\"\nWhenever equipped creature deals damage to a blocking creature, this Equipment deals that much damage to each other creature defending player controls.\nEquip {3}"
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Valiant — Whenever this creature becomes the target of a spell or ability you controls for the first time each turn, look at the top five cards of your library. You may reveal it. Then if it is your turn, you may put it onto the battlefield. Then if not, put it into its owner's hand. For each card revealed this way, Unless it's a permanent, put that object on the bottom of its owner's library."
        ),
        "Valiant — Whenever this creature becomes the target of a spell or ability you control for the first time each turn, look at the top five cards of your library. You may reveal a creature card with mana value 3 or less from among them. You may put it onto the battlefield if it's your turn. If you don't put it onto the battlefield, put it into your hand. Put the rest on the bottom of your library in a random order."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Search target opponent's library for a creature card, put it onto the battlefield under target opponent's control, then shuffle target opponent's library."
        ),
        "Search target opponent's library for a creature card and put that card onto the battlefield under your control. Then that player shuffles."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Search target opponent's library for an artifact card, put it onto the battlefield under target opponent's control, then shuffle target opponent's library."
        ),
        "Search target opponent's library for an artifact card and put that card onto the battlefield under your control. Then that player shuffles."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever a spell you've cast is countered, draw a card.",
        ),
        "Whenever a spell you've cast is countered, draw a card."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Each opponent sacrifices a creature or planeswalker with mana value equal to a dynamic value of their choice."
        ),
        "Each opponent sacrifices a creature or planeswalker with the greatest mana value among creatures and planeswalkers they control."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Return to your hand all cards in your graveyard that you cycled or discarded this turn."
        ),
        "Return to your hand all cards in your graveyard that you cycled or discarded this turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever an opponent casts a spell, you may reveal the top card of your library. Then if it's a permanent that shares a card type with that object, counter it, then that object's controller may cast that card without paying its mana cost."
        ),
        "Whenever an opponent casts a spell from their hand, you may reveal the top card of your library. If it shares a card type with that spell, counter it and that opponent may cast the revealed card without paying its mana cost."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever an opponent casts a spell from their hand, you may reveal the top card of your library. If a permanent that shares a card type with it was revealed this way, counter it and that player may cast that card without paying its mana cost."
        ),
        "Whenever an opponent casts a spell from their hand, you may reveal the top card of your library. If it shares a card type with that spell, counter it and that opponent may cast the revealed card without paying its mana cost."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever an opponent casts a spell from their hand, you may reveal the top card of your library. If a permanent that shares a card type with it was revealed this way, counter it and that player may cast that card without paying its mana cost"
        ),
        "Whenever an opponent casts a spell from their hand, you may reveal the top card of your library. If it shares a card type with that spell, counter it and that opponent may cast the revealed card without paying its mana cost"
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. Then if a permanent that shares a card type with it was revealed this way, copy that spell, you may choose new targets for the copy, then each opponent draws a card. Otherwise, draw a card."
        ),
        "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. If any of those cards shares a card type with that spell, copy that spell, you may choose new targets for the copy, and each opponent draws a card. Otherwise, you draw a card."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. Then if a permanent that shares a card type with it was revealed this way, copy that spell, you may choose new targets for the copy, then each opponent draws a card. Otherwise, draw a card"
        ),
        "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. If any of those cards shares a card type with that spell, copy that spell, you may choose new targets for the copy, and each opponent draws a card. Otherwise, you draw a card"
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever creature attacks, this creature gets +2/+0 until end of turn."
        ),
        "Whenever a creature attacks one of your opponents or a planeswalker an opponent controls, that creature gets +2/+0 until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "You may discard a Forest card rather than pay this spell's mana cost.\nPrevent all combat damage that would be dealt this turn by unblocked creature."
        ),
        "You may discard a Forest card rather than pay this spell's mana cost.\nPrevent all combat damage that would be dealt by unblocked creatures this turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Prevent all combat damage that would be dealt this turn by unblocked creature."
        ),
        "Prevent all combat damage that would be dealt by unblocked creatures this turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Counter target spell and Ionize deals 2 damage to that object's controller."
        ),
        "Counter target spell. Ionize deals 2 damage to that spell's controller."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever a player taps a Forest for mana, that object's controller adds {G}."
        ),
        "Whenever a Forest is tapped for mana, its controller adds an additional {G}."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{T}: Each player exiles the top card of their library face down.\n{7}, {T}, Sacrifice this artifact: For each player, Return all permanent card in that player's exile to the battlefield under their owners' control."
        ),
        "{T}: Each player exiles the top card of their library face down.\n{7}, {T}, Sacrifice this artifact: Each player turns face up all cards they own exiled with this artifact, then puts all permanent cards among them onto the battlefield."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{7}, {T}, Sacrifice this artifact: For each player, Return all permanent card in that player's exile to the battlefield under their owners' control."
        ),
        "{7}, {T}, Sacrifice this artifact: Each player turns face up all cards they own exiled with this artifact, then puts all permanent cards among them onto the battlefield."
    );
    assert_eq!(
        normalize_common_semantic_phrasing("Destroy target colored creature."),
        "Destroy target creature that's one or more colors."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Each player sacrifices all colored permanents they control."
        ),
        "Each player sacrifices all permanents they control that are one or more colors."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "As an additional cost to cast this spell, you may choose a Dragon card. Reveal it."
        ),
        "As an additional cost to cast this spell, you may reveal a Dragon card from your hand."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Counter target spell unless its controller pays {1}. If this spell's behold cost was paid or you control a Dragon, instead counter that spell."
        ),
        "Counter target spell unless its controller pays {1}. If you revealed a Dragon card or controlled a Dragon as you cast this spell, counter that spell instead."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target player sacrifices a creature of their choice. Then if this spell's behold cost was paid or you control a Dragon, you gain 4 life."
        ),
        "Target player sacrifices a creature of their choice. If you revealed a Dragon card or controlled a Dragon as you cast this spell, you gain 4 life."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target opponent reveals cards from the top of target opponent's library until they reveal a creature card. For each card revealed this way, Unless it's a permanent, put that object into its owner's graveyard. Put it onto the battlefield."
        ),
        "Target opponent reveals cards from the top of their library until they reveal a creature card. That player puts all noncreature cards revealed this way into their graveyard, then you put the creature card onto the battlefield under your control."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "At the beginning of your end step, draw a card, each player may put a land card from their hand onto the battlefield, then for each opponent, if effect #0 that doesn't happen, that player draws a card."
        ),
        "At the beginning of your end step, draw a card. Each player may put a land card from their hand onto the battlefield, then each opponent who didn't draws a card."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Choose target instant or sorcery spell. Each opponent may copy it. Each opponent may choose new targets for the copy. Copy it that many players plus 1 time. You may choose new targets for the copy."
        ),
        "Tempting offer — Choose target instant or sorcery spell. Each opponent may copy that spell and may choose new targets for the copy they control. You copy that spell once plus an additional time for each opponent who copied the spell this way. You may choose new targets for the copies you control."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "At the beginning of your upkeep, if your life total is greater than or equal to 50, you win the game."
        ),
        "At the beginning of your upkeep, if you have 50 or more life, you win the game."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Gain control of each other creature until end of turn, untap that creature, then it gains haste until end of turn."
        ),
        "You and target opponent each gain control of all creatures the other controls until end of turn. Untap those creatures. Those creatures gain haste until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{2}, {T}, Sacrifice this artifact: Search target player's library for exactly 3 cards, exile them. Target player shuffles."
        ),
        "{2}, {T}, Sacrifice this artifact: Search target player's library for three cards and exile them. Then that player shuffles."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Search target player's library for a card, exile it, then shuffle target player's library."
        ),
        "Search target player's library for a card and exile it. Then that player shuffles."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Choose any number white cards, reveal it, then gain 2 life for each permanent."
        ),
        "Reveal any number of white cards in your hand. You gain 2 life for each card revealed this way."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target creature gets +2/+2 until end of turn, clash with an opponent, then creatures gain trample until end of turn."
        ),
        "Target creature gets +2/+2 until end of turn. Clash with an opponent. If you win, that creature gets an additional +2/+2 and gains trample until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "each player sacrifices all colored permanents each player controls of their choice."
        ),
        "Each player sacrifices all permanents they control that are one or more colors."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever this creature deals combat damage to a player, that player reveals the top two cards of their library. You choose a card. Put it into its owner's graveyard."
        ),
        "Whenever this creature deals combat damage to a player, that player reveals the top two cards of their library. You choose one of those cards and put it into their graveyard."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Creatures gain hexproof and indestructible until end of turn, Players have hexproof this turn, Players can't lose life this turn, Players can't win the game this turn, then Players can't lose the game this turn."
        ),
        "All creatures gain hexproof and indestructible until end of turn. Players gain hexproof until end of turn. Players can't lose life this turn and players can't lose the game or win the game this turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Each player mills ten cards, for each player, you choose a creature or planeswalker card, put that card onto the battlefield under your control, then for each creature you control, each creature you control becomes a phyrexian in addition to its other types."
        ),
        "Each player mills ten cards. For each player, choose a creature or planeswalker card in that player's graveyard. Put those cards onto the battlefield under your control. Then each creature you control becomes a Phyrexian in addition to its other types."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Scry 3, reveal the top card of your library, then draw its mana value cards."
        ),
        "Scry 3, then reveal the top card of your library. Draw cards equal to that card's mana value."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{U}, {T}: Target creature gains flying, then it becomes blue until end of turn."
        ),
        "{U}, {T}: Target creature gains flying and becomes blue until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target opponent loses 3 life. Put a card from their hand on top of target opponent's library."
        ),
        "Target opponent loses 3 life and puts a card from their hand on top of their library."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Draw a card for each instant or sorcery card in your graveyard."
        ),
        "Draw cards equal to the number of instant and sorcery cards in your graveyard."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Create a 5/5 red and green Elemental creature token for each colors among permanent you control, then gain 1 life for each creature you control."
        ),
        "Vivid — Create a number of 5/5 red and green Elemental creature tokens equal to the number of colors among permanents you control. Then you gain life equal to the number of creatures you control."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Choose a nonland permanent card name, target player reveals their hand, then target player discards the number of cards named {chosen Name}."
        ),
        "Choose a nonland card name. Target player reveals their hand and discards all cards with that name."
    );
    assert_eq!(
        normalize_common_semantic_phrasing("Flashback—{0}, Sacrifice a creature."),
        "Flashback—Sacrifice a creature."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When this creature enters, target opponent loses 1 life for each Elf you control."
        ),
        "When this creature enters, target opponent loses life equal to the number of Elves you control."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target creature gets +X/+0 until end of turn, where X is target creature's power."
        ),
        "Double the power of target creature until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Discard the number of cards in your hand, draw that many plus 1 cards, then gain 1 life for each card in your hand."
        ),
        "Discard all the cards in your hand, then draw that many cards plus one. You gain life equal to the number of cards in your hand."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Gain control of target creature until end of turn, untap it, it gets +X/+0 until end of turn, where X is X, then it gains haste until end of turn."
        ),
        "Gain control of target creature until end of turn. Untap that creature. It gets +X/+0 and gains haste until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Untap target creature, it gets +2/+2 until end of turn, then it gains lifelink until end of turn."
        ),
        "Untap target creature. It gets +2/+2 and gains lifelink until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Return target permanent spell to its owner's hand, Jeskai Revelation deals 4 damage to any target, create two 1/1 white Monk creature tokens with prowess, draw two cards, then gain 4 life."
        ),
        "Return target spell or permanent to its owner's hand. Jeskai Revelation deals 4 damage to any target. Create two 1/1 white Monk creature tokens with prowess. Draw two cards. You gain 4 life."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When this siege enters, search your library and/or graveyard for a non-Human creature with mana value X or less you own, for each card searched for this way, put them onto the battlefield, then shuffle your library."
        ),
        "When this Siege enters, search your library and/or graveyard for a non-Human creature card with mana value X or less and put it onto the battlefield. If you search your library this way, shuffle."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Untap all creatures. It changes controller to this effect's controller and gains haste until end of turn."
        ),
        "Untap all creatures and gain control of them until end of turn. They gain haste until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{1}, {T}: This creature loses this ability, this creature becomes an enchantment in addition to its other types, isn't an artifact, battle, creature, kindred, land, or planeswalker, becomes an aura in addition to its other types, and has enchant restriction, attach it to target creature, then you may pay {1}."
        ),
        "{1}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {1} to end this effect."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{G}, {T}: This creature loses this ability, this creature becomes an enchantment in addition to its other types, isn't an artifact, battle, creature, kindred, land, or planeswalker, becomes an aura in addition to its other types, and has enchant restriction, attach it to target creature, then you may pay {G}."
        ),
        "{G}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {G} to end this effect."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Enchanted creature is artifact in addition to its other types."
        ),
        "Enchanted creature gets +1/+1 and is an artifact in addition to its other types."
    );
    assert_eq!(
        normalize_common_semantic_phrasing("{G}: Regenerate an enchanted creature."),
        "{G}: Regenerate enchanted creature."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target opponent reveals their hand, choose a nonland card, target opponent discards that card, then put a +1/+1 counter on a creature you control."
        ),
        "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card. Put a +1/+1 counter on a creature you control."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "You gain 1 life for each creature card in your graveyard."
        ),
        "You gain life equal to the number of creature cards in your graveyard."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Destroy all artifacts, destroy all enchantments, then gain life equal to twice that many."
        ),
        "Destroy all artifacts and enchantments. You gain 2 life for each permanent destroyed this way."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{T}: For each player, that player exiles the top card of that player's library face down."
        ),
        "{T}: Each player exiles the top card of their library face down."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{T}, Sacrifice this creature: For each player, exile all cards in that player's hand face down. Each player draws seven cards. At the beginning of the next end step, each player discards their hand. Return those cards in exile to their owners' hands."
        ),
        "{T}, Sacrifice this creature: Each player exiles all cards from their hand face down and draws seven cards. At the beginning of the next end step, each player discards their hand and returns to their hand each card they exiled this way."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{T}, Discard 2 cards: Draw three cards, put a fuse counter on this artifact, this artifact deals damage to target opponent equal to the number of fuse counters on this artifact, then target opponent gains control of this artifact."
        ),
        "{T}, Discard two cards: Draw three cards, then put a fuse counter on this artifact. It deals damage equal to the number of fuse counters on it to target opponent. They gain control of this artifact."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Sacrifice this enchantment: Creatures your opponents control get -1/-1 and gain attacks each combat if able until end of turn."
        ),
        "Sacrifice this enchantment: Creatures your opponents control get -1/-1 until end of turn. Those creatures attack this turn if able."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Exile the top card of your library. If it's an artifact, creature, enchantment, land, planeswalker, or battle card, you may return it to the battlefield. If it happened,. Repeat this process."
        ),
        "Exile the top card of your library. If it's a permanent card, you may put it onto the battlefield. If you do, repeat this process."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever this creature attacks, choose target land. Destroy all Aura attached to that object."
        ),
        "Whenever this creature attacks, destroy all Auras attached to target land."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Exile all cards from their hand. Exile target player's graveyard."
        ),
        "Exile all cards from target player's hand and graveyard."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Choose a creature at random on the battlefield, gain control of it until end of turn, untap it, it gains haste until end of turn, then destroy all other creatures."
        ),
        "Choose a creature at random. You gain control of that creature until end of turn. Untap it. It gains haste until end of turn. Then destroy all other creatures."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "For each creature you control, that object gets +X/+0 until end of turn, where X is that object's power."
        ),
        "Double the power of each creature you control until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{T}: Each player loses the number of Zombies on the battlefield life."
        ),
        "{T}: Each player loses 1 life for each Zombie on the battlefield."
    );
    assert_eq!(
        normalize_common_semantic_phrasing("Exile all nonland nonlegendary permanents."),
        "Exile all nonland permanents that aren't legendary."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target player reveals their hand, choose a nonland card, target player discards that card, then lose 2 life."
        ),
        "Target player reveals their hand. You choose a nonland card from it. That player discards that card. You lose 2 life."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Choose target creature. Target permanent must block creature if able this turn."
        ),
        "Target creature blocks target creature this turn if able."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever this creature attacks, choose target defending player's creature. Target permanent must block target permanent if able this turn."
        ),
        "Whenever this creature attacks, target creature defending player controls blocks it this combat if able."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Look at the top four cards of your library, choose two cards, for each card chosen this way, return that object to its owner's hand, for each card revealed this way, Unless it's a permanent, put that object into its owner's graveyard, then lose 2 life."
        ),
        "Look at the top four cards of your library, choose two cards, for each card chosen this way, return that object to its owner's hand, for each card revealed this way, Unless it's a permanent, put that object into its owner's graveyard, then lose 2 life."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Look at the top 2*X cards of your library, choose X cards, for each card chosen this way, return that object to its owner's hand, for each card revealed this way, Unless it's a permanent, put that object into its owner's graveyard, then lose X life."
        ),
        "Look at the top 2*X cards of your library, choose X cards, for each card chosen this way, return that object to its owner's hand, for each card revealed this way, Unless it's a permanent, put that object into its owner's graveyard, then lose X life."
    );
}

#[test]
fn equipment_token_compactor_requires_pump_clause() {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
    let equip = crate::ability::Ability {
        kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: crate::cost::TotalCost::free(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                crate::effect::Effect::attach_to(target.clone()),
            ]),
            choices: vec![target],
            timing: crate::ability::ActivationTiming::SorcerySpeed,
            is_loyalty_ability: false,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
    };
    let token = crate::cards::CardDefinitionBuilder::new(
        crate::ids::CardId::from_raw(1),
        "Stoneforged Blade",
    )
    .token()
    .card_types(vec![CardType::Artifact])
    .subtypes(vec![Subtype::Equipment])
    .with_ability(crate::ability::Ability::static_ability(
        crate::static_abilities::StaticAbility::make_colorless(ObjectFilter::source()),
    ))
    .with_ability(crate::ability::Ability::static_ability(
        crate::static_abilities::StaticAbility::attached_ability_grant(
            crate::ability::indestructible(),
            "Equipped creature has Indestructible".to_string(),
        ),
    ))
    .with_ability(equip)
    .build();

    assert_eq!(compact_equipment_token_ability_payload(&token), None);
}

#[test]
fn normalize_until_next_turn_token_copy_haste_compacts_trigger_surface() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Until your next turn, at the beginning of combat on your turn, exile target white or black or red creature card from your graveyard, create a token that's a copy of that card, with base power and toughness 1/1, then it gains haste."
        ),
        "At the beginning of combat on your turn, exile target red, white, or black creature card from your graveyard. Create a token that's a copy of that card, except it's 1/1. It gains haste until your next turn."
    );
}

#[test]
fn normalize_attached_creature_with_base_pt_combines_sentences() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Enchanted artifact is creature. Enchanted artifact has base power and toughness 5/5."
        ),
        "Enchanted artifact is a creature with base power and toughness 5/5 in addition to its other types."
    );
}

#[test]
fn normalize_ability_loss_transform_surface_repairs_base_pt_clause() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Enchanted creature loses all abilities and is is treefolk, has base power, and toughness 0/4 creature."
        ),
        "Enchanted creature loses all abilities and is a Treefolk creature with base power and toughness 0/4."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Enchanted creature loses all abilities and is is frog, is blue, has base power, and toughness 1/1 creature."
        ),
        "Enchanted creature loses all abilities and is a blue Frog creature with base power and toughness 1/1."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Enchanted creature loses all abilities and is green is citizen, is white, is named legitimate businessperson, has base power, and toughness 1/1 creature."
        ),
        "Enchanted creature loses all abilities and is a green and white Citizen creature with base power and toughness 1/1 named Legitimate Businessperson."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Enchanted creature loses all abilities and is green is a citizen, is white, is named legitimate businessperson, has base power, and toughness 1/1 creature."
        ),
        "Enchanted creature loses all abilities and is a green and white Citizen creature with base power and toughness 1/1 named Legitimate Businessperson."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Enchanted creature loses all abilities and is white and green citizen is creature named legitimate businessperson and, has base power, and toughness 1/1 creature."
        ),
        "Enchanted creature loses all abilities and is a white and green Citizen creature with base power and toughness 1/1 named Legitimate Businessperson."
    );
}

#[test]
fn normalize_defender_attack_permission_surface() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{0}: This creature gains can attack as though it didn't have defender until end of turn. At the beginning of the next end step, exile it."
        ),
        "{0}: This creature can attack this turn as though it didn't have defender. At the beginning of the next end step, exile it."
    );
}

#[test]
fn tagged_set_with_each_surface_renders_each_of_them() {
    let effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Tagged(TagKey::from("untapped_0")),
        crate::continuous::Modification::ModifyPowerToughness {
            power: 1,
            toughness: 1,
        },
        Until::EndOfTurn,
    )
    .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each));

    assert_eq!(
        describe_apply_continuous_target(&effect),
        ("each of them".to_string(), false)
    );
}

#[test]
fn tagged_plural_pronoun_surface_does_not_pluralize_an_unmarked_singular_reference() {
    let tagged = || {
        crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(TagKey::from("created_0")),
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::haste(),
            ),
            Until::EndOfTurn,
        )
    };

    assert_eq!(
        describe_apply_continuous_target(
            &tagged()
                .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::They,)),
        ),
        ("They".to_string(), true)
    );
    assert_eq!(
        describe_apply_continuous_target(&tagged()),
        ("it".to_string(), false)
    );
}

#[test]
fn coordinated_return_result_union_keeps_the_plural_copula() {
    let mut returned = ObjectFilter::default();
    returned.any_of = vec![
        ObjectFilter::tagged(TagKey::from("returned_0")),
        ObjectFilter::tagged(TagKey::from("returned_1")),
        ObjectFilter::tagged(TagKey::from("returned_2")),
    ];
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Object(returned),
        crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        Until::Forever,
    )
    .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::They))
    .with_type_retention_surface(Some(
        ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes,
    ))
    .with_animation_pt_surface(Some(
        ironsmith_core::AnimationPtSurface::LeadingPowerToughness,
    ));
    effect.additional_modifications.extend([
        crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(5),
            toughness: Value::Fixed(5),
            sublayer: crate::continuous::PtSublayer::Setting,
        },
        crate::continuous::Modification::AddSubtypes(vec![Subtype::Elemental]),
    ]);

    assert_eq!(
        describe_apply_continuous_effect(&effect).as_deref(),
        Some("They are 5/5 Elemental creatures in addition to their other types")
    );
    assert_eq!(
        describe_returned_object_animation_effect(&effect, false).as_deref(),
        Some("They are 5/5 Elemental creatures in addition to their other types")
    );
    let wrapped = Effect::new(effect).tag(TagKey::from("animated_creature_0"));
    assert_eq!(
        describe_effect(&wrapped),
        "They are 5/5 Elemental creatures in addition to their other types"
    );
    assert_eq!(
        describe_effect_list(&[wrapped]),
        "They are 5/5 Elemental creatures in addition to their other types"
    );
}

#[test]
fn plural_move_result_animation_uses_copula_without_changing_singular_followups() {
    let effect = || {
        let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(TagKey::from("moved_0")),
            crate::continuous::Modification::AddCardTypes(vec![
                CardType::Artifact,
                CardType::Creature,
            ]),
            Until::Forever,
        )
        .with_animation_pt_surface(Some(
            ironsmith_core::AnimationPtSurface::LeadingPowerToughness,
        ));
        effect
            .additional_modifications
            .push(crate::continuous::Modification::SetPowerToughness {
                power: Value::Fixed(5),
                toughness: Value::Fixed(5),
                sublayer: crate::continuous::PtSublayer::Setting,
            });
        effect
    };

    assert_eq!(
        describe_apply_continuous_effect(
            &effect().with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::They)),
        )
        .as_deref(),
        Some("They're 5/5 artifact creatures")
    );
    let singular = describe_apply_continuous_effect(&effect()).expect("animation renders");
    assert!(singular.contains(" becomes "), "{singular}");
    assert!(!singular.starts_with("They're "), "{singular}");
}

#[test]
fn demonstrative_set_and_condition_keep_their_antecedent_surfaces() {
    let effect = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Filter(ObjectFilter::creature().you_control().other()),
        crate::continuous::Modification::ModifyPowerToughness {
            power: 2,
            toughness: 0,
        },
        Until::EndOfTurn,
    )
    .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
    assert_eq!(
        describe_apply_continuous_target(&effect),
        ("Those creatures".to_string(), true)
    );

    let mut swamp = ObjectFilter::default().with_subtype(Subtype::Swamp);
    swamp.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Land,
    ));
    assert_eq!(
        describe_condition(&Condition::TaggedObjectMatches(
            TagKey::from("triggering"),
            swamp,
        )),
        "that land is a Swamp"
    );

    let mut other_hero = ObjectFilter::default().with_subtype(Subtype::Hero);
    other_hero.other = true;
    other_hero.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Creature,
    ));
    assert_eq!(
        describe_condition(&Condition::TaggedObjectMatches(
            TagKey::from("triggering"),
            other_hero,
        )),
        "that creature is another Hero"
    );

    let mut toxic = ObjectFilter::default().with_ability_marker("toxic");
    toxic.card_types.push(CardType::Creature);
    toxic.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Creature,
    ));
    assert_eq!(
        describe_condition(&Condition::TaggedObjectMatches(
            TagKey::from("pumped_0"),
            toxic,
        )),
        "that creature has toxic"
    );

    let mut power = ObjectFilter::default();
    power.power = Some(ironsmith_core::FilterComparison::LessThanOrEqual(2));
    power.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Creature,
    ));
    assert_eq!(
        describe_condition(&Condition::TaggedObjectMatchedLastKnown(
            TagKey::from("exiled_0"),
            power,
        )),
        "that creature had power 2 or less"
    );
}

#[test]
fn copy_exception_type_modifications_render_only_inside_exception_tail() {
    let copy_source = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    let mut additive = crate::effects::ApplyContinuousEffect::with_spec_runtime(
        ChooseSpec::Source,
        crate::effects::continuous::RuntimeModification::CopyOf {
            source: copy_source.clone(),
            preserve_source_abilities: false,
            name_override: None,
            name_override_surface: None,
            add_supertypes: Vec::new(),
            copy_exception_surface: Some(
                "it's a Vehicle artifact in addition to its other types".to_string(),
            ),
        },
        Until::EndOfTurn,
    );
    additive.additional_modifications.extend([
        crate::continuous::Modification::AddCardTypes(vec![CardType::Artifact]),
        crate::continuous::Modification::AddSubtypes(vec![Subtype::Vehicle]),
    ]);
    let additive_text = describe_apply_continuous_effect(&additive).expect("additive copy text");
    assert!(
        additive_text.contains("except it's a Vehicle artifact in addition to its other types"),
        "{additive_text}"
    );
    assert_eq!(
        additive_text.matches("artifact").count(),
        1,
        "{additive_text}"
    );
    assert_eq!(
        additive_text.matches("Vehicle").count(),
        1,
        "{additive_text}"
    );

    let mut setting = crate::effects::ApplyContinuousEffect::with_spec_runtime(
        ChooseSpec::Source,
        crate::effects::continuous::RuntimeModification::CopyOf {
            source: copy_source,
            preserve_source_abilities: false,
            name_override: Some("Taskmaster, Mercenary Mimic".to_string()),
            name_override_surface: None,
            add_supertypes: vec![crate::types::Supertype::Legendary],
            copy_exception_surface: Some(
                "his name is Taskmaster, Mercenary Mimic and he's a legendary Human Mercenary Villain creature"
                    .to_string(),
            ),
        },
        Until::YourNextTurn,
    );
    setting.additional_modifications.extend([
        crate::continuous::Modification::SetCardTypes(vec![CardType::Creature]),
        crate::continuous::Modification::RemoveAllSubtypesOfFamily(
            crate::types::SubtypeFamily::Creature,
        ),
        crate::continuous::Modification::AddSubtypes(vec![
            Subtype::Human,
            Subtype::Mercenary,
            Subtype::Villain,
        ]),
    ]);
    let setting_text = describe_apply_continuous_effect(&setting).expect("setting copy text");
    assert!(
        setting_text.contains(
            "except his name is Taskmaster, Mercenary Mimic and he's a legendary Human Mercenary Villain creature"
        ),
        "{setting_text}"
    );
    assert_eq!(
        setting_text.matches("Taskmaster").count(),
        1,
        "{setting_text}"
    );
    assert_eq!(setting_text.matches("Villain").count(), 1, "{setting_text}");
}

#[test]
fn land_animation_prefers_still_land_surface_for_land_targets() {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        target,
        crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    )
    .with_type_retention_surface(Some(ironsmith_core::TypeRetentionSurface::StillALand));
    effect
        .additional_modifications
        .push(crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(4),
            toughness: Value::Fixed(4),
            sublayer: crate::continuous::PtSublayer::Setting,
        });
    effect
        .additional_modifications
        .push(crate::continuous::Modification::AddSubtypes(vec![
            Subtype::Elemental,
        ]));

    let (target_text, plural_target) = describe_apply_continuous_target(&effect);
    assert_eq!(
        describe_apply_continuous_animation_effect(&effect, &target_text, plural_target).as_deref(),
        Some(
            "Target land you control becomes an Elemental creature with base power and toughness 4/4 until end of turn. It's still a land"
        )
    );
}

#[test]
fn typed_animation_pt_surface_preserves_leading_and_authored_base_pt_forms() {
    let build = |surface, power| {
        let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact())),
            crate::continuous::Modification::AddCardTypes(vec![
                CardType::Artifact,
                CardType::Creature,
            ]),
            Until::EndOfTurn,
        )
        .with_animation_pt_surface(Some(surface));
        effect.additional_modifications.extend([
            crate::continuous::Modification::SetPowerToughness {
                power: Value::Fixed(power),
                toughness: Value::Fixed(power),
                sublayer: crate::continuous::PtSublayer::Setting,
            },
            crate::continuous::Modification::AddSubtypes(vec![Subtype::Angel]),
        ]);
        effect
    };

    let leading = build(ironsmith_core::AnimationPtSurface::LeadingPowerToughness, 4);
    let (target, plural) = describe_apply_continuous_target(&leading);
    assert_eq!(
        describe_apply_continuous_animation_effect(&leading, &target, plural).as_deref(),
        Some("Target artifact becomes a 4/4 Angel artifact creature until end of turn")
    );

    let explicit = build(
        ironsmith_core::AnimationPtSurface::ExplicitBasePowerToughness,
        4,
    );
    let (target, plural) = describe_apply_continuous_target(&explicit);
    assert_eq!(
        describe_apply_continuous_animation_effect(&explicit, &target, plural).as_deref(),
        Some(
            "Target artifact becomes an Angel artifact creature with base power and toughness 4/4 until end of turn"
        )
    );

    for (power, article) in [(1, "a"), (8, "an")] {
        let leading = build(
            ironsmith_core::AnimationPtSurface::LeadingPowerToughness,
            power,
        );
        let (target, plural) = describe_apply_continuous_target(&leading);
        let rendered = describe_apply_continuous_animation_effect(&leading, &target, plural)
            .expect("fixed leading P/T animation should render");
        assert!(
            rendered.contains(&format!("becomes {article} {power}/{power}")),
            "{rendered}"
        );
    }
}

#[test]
fn typed_animation_duration_surface_preserves_authored_leading_placement() {
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control())),
        crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    )
    .with_type_retention_surface(Some(ironsmith_core::TypeRetentionSurface::StillALand))
    .with_animation_pt_surface(Some(
        ironsmith_core::AnimationPtSurface::LeadingPowerToughness,
    ))
    .with_animation_duration_surface(Some(ironsmith_core::AnimationDurationSurface::Leading));
    effect.additional_modifications.extend([
        crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(4),
            toughness: Value::Fixed(4),
            sublayer: crate::continuous::PtSublayer::Setting,
        },
        crate::continuous::Modification::AddSubtypes(vec![Subtype::Dinosaur]),
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::reach(),
        ),
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::haste(),
        ),
    ]);

    let (target, plural) = describe_apply_continuous_target(&effect);
    assert_eq!(
        describe_apply_continuous_animation_effect(&effect, &target, plural).as_deref(),
        Some(
            "Until end of turn, target land you control becomes a 4/4 Dinosaur creature with reach and haste. It's still a land"
        )
    );
}

#[test]
fn explicit_type_retention_is_not_suppressed_by_redundant_artifact_type() {
    let mut target_filter = ObjectFilter::artifact().you_control();
    target_filter.excluded_card_types.push(CardType::Creature);
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::target(ChooseSpec::Object(target_filter)),
        crate::continuous::Modification::AddCardTypes(vec![CardType::Artifact, CardType::Creature]),
        Until::Forever,
    )
    .with_type_retention_surface(Some(
        ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes,
    ))
    .with_animation_pt_surface(Some(
        ironsmith_core::AnimationPtSurface::LeadingPowerToughness,
    ));
    effect.additional_modifications.extend([
        crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(8),
            toughness: Value::Fixed(8),
            sublayer: crate::continuous::PtSublayer::Setting,
        },
        crate::continuous::Modification::AddSubtypes(vec![Subtype::Robot, Subtype::Villain]),
    ]);

    let (target, plural) = describe_apply_continuous_target(&effect);
    assert_eq!(
        describe_apply_continuous_animation_effect(&effect, &target, plural).as_deref(),
        Some(
            "Target noncreature artifact you control becomes an 8/8 Robot Villain artifact creature in addition to its other types"
        )
    );
}

#[test]
fn dynamic_equal_animation_renders_explicit_value_without_lossy_x_rewrite() {
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
        crate::continuous::Modification::SetCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    );
    let value = Value::Add(Box::new(Value::X), Box::new(Value::Fixed(1)));
    effect.additional_modifications.extend([
        crate::continuous::Modification::SetPowerToughness {
            power: value.clone(),
            toughness: value,
            sublayer: crate::continuous::PtSublayer::Setting,
        },
        crate::continuous::Modification::SetColors(
            crate::color::ColorSet::GREEN.union(crate::color::ColorSet::BLUE),
        ),
        crate::continuous::Modification::AddSubtypes(vec![Subtype::Fractal]),
    ]);

    let (target_text, plural_target) = describe_apply_continuous_target(&effect);
    let rendered = describe_apply_continuous_animation_effect(&effect, &target_text, plural_target)
        .expect("dynamic animation should render structurally");
    assert!(
        rendered.contains("with base power and toughness each equal to X plus 1"),
        "{rendered}"
    );
    assert!(!rendered.contains("until end of turn plus 1"), "{rendered}");
}

#[test]
fn attached_land_animation_is_singular_and_quotes_its_granted_ability() {
    let mut filter = ObjectFilter::default().with_subtype(Subtype::Swamp);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("enchanted"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Object(filter),
        crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    )
    .with_type_retention_surface(Some(ironsmith_core::TypeRetentionSurface::StillALand));
    effect.additional_modifications.extend([
        crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(2),
            toughness: Value::Fixed(2),
            sublayer: crate::continuous::PtSublayer::Setting,
        },
        crate::continuous::Modification::AddSubtypes(vec![Subtype::Spirit]),
        crate::continuous::Modification::AddAbilityGeneric(Ability::static_ability(
            crate::static_abilities::StaticAbility::trample(),
        )),
    ]);

    let (target_text, plural_target) = describe_apply_continuous_target(&effect);
    let rendered = describe_apply_continuous_animation_effect(&effect, &target_text, plural_target)
        .expect("attached land animation should render structurally");

    assert!(
        rendered.starts_with("Until end of turn, enchanted Swamp becomes"),
        "{rendered}"
    );
    assert!(rendered.contains("\"Trample.\""), "{rendered}");
    assert!(rendered.ends_with("It's still a land"), "{rendered}");
    assert!(!rendered.contains("All enchanted"), "{rendered}");
}

#[test]
fn animation_quotes_a_nested_conditional_source_ability() {
    let conditional_first_strike =
        crate::static_abilities::StaticAbility::grant_object_ability_for_filter(
            ObjectFilter::source(),
            Ability::static_ability(crate::static_abilities::StaticAbility::first_strike()),
            "First strike".to_string(),
        )
        .with_condition(Condition::ActivationTiming(
            crate::ability::ActivationTiming::DuringYourTurn,
        ))
        .expect("source grant supports a runtime condition");
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Source,
        crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    )
    .with_source_reference_surface(crate::target::SourceReferenceSurface::ThisPermanentType(
        "this land".to_string(),
    ))
    .with_type_retention_surface(Some(ironsmith_core::TypeRetentionSurface::StillALand));
    effect.additional_modifications.extend([
        crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(2),
            toughness: Value::Fixed(1),
            sublayer: crate::continuous::PtSublayer::Setting,
        },
        crate::continuous::Modification::SetColors(
            crate::color::ColorSet::BLUE.union(crate::color::ColorSet::RED),
        ),
        crate::continuous::Modification::AddSubtypes(vec![Subtype::Elemental]),
        crate::continuous::Modification::AddAbility(conditional_first_strike),
    ]);

    let (target_text, plural_target) = describe_apply_continuous_target(&effect);
    let rendered = describe_apply_continuous_animation_effect(&effect, &target_text, plural_target)
        .expect("nested source ability should remain a structural animation");

    assert!(
        rendered.contains("\"During your turn, this creature has first strike.\""),
        "{rendered}"
    );
    assert!(rendered.ends_with("It's still a land"), "{rendered}");
}

#[test]
fn plain_type_setting_animation_omits_addition_surface() {
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Source,
        crate::continuous::Modification::SetCardTypes(vec![CardType::Creature]),
        Until::Forever,
    );
    effect
        .additional_modifications
        .push(crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(5),
            toughness: Value::Fixed(3),
            sublayer: crate::continuous::PtSublayer::Setting,
        });
    effect.additional_modifications.push(
        crate::continuous::Modification::RemoveAllSubtypesOfFamily(
            crate::types::SubtypeFamily::Creature,
        ),
    );
    effect
        .additional_modifications
        .push(crate::continuous::Modification::AddSubtypes(vec![
            Subtype::Soldier,
        ]));
    effect
        .additional_modifications
        .push(crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::trample(),
        ));

    let (target_text, plural_target) = describe_apply_continuous_target(&effect);
    let rendered = describe_apply_continuous_animation_effect(&effect, &target_text, plural_target)
        .expect("type-setting animation should render structurally");
    assert!(rendered.contains("soldier creature"), "{rendered}");
    assert!(
        rendered.contains("base power and toughness 5/3"),
        "{rendered}"
    );
    assert!(!rendered.contains("in addition to"), "{rendered}");

    let reset = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Source,
        crate::continuous::Modification::SetCardTypes(vec![CardType::Enchantment]),
        Until::Forever,
    );
    assert_eq!(
        describe_apply_continuous_effect(&reset).as_deref(),
        Some("this source becomes an enchantment")
    );

    let mut land_reset = crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land())),
        crate::continuous::Modification::SetCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    );
    land_reset
        .additional_modifications
        .push(crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(3),
            toughness: Value::Fixed(3),
            sublayer: crate::continuous::PtSublayer::Setting,
        });
    let (target_text, plural_target) = describe_apply_continuous_target(&land_reset);
    let rendered =
        describe_apply_continuous_animation_effect(&land_reset, &target_text, plural_target)
            .expect("land type-setting animation should render structurally");
    assert!(!rendered.contains("still a land"), "{rendered}");
}

#[test]
fn subtype_dynamic_land_animation_keeps_addition_surface() {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default().with_subtype(Subtype::Forest),
    ));
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        target,
        crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    )
    .with_type_retention_surface(Some(
        ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes,
    ));
    effect
        .additional_modifications
        .push(crate::continuous::Modification::SetPowerToughness {
            power: Value::X,
            toughness: Value::X,
            sublayer: crate::continuous::PtSublayer::Setting,
        });
    effect
        .additional_modifications
        .push(crate::continuous::Modification::AddSubtypes(vec![
            Subtype::Treefolk,
        ]));

    let (target_text, plural_target) = describe_apply_continuous_target(&effect);
    assert_eq!(
        describe_apply_continuous_animation_effect(&effect, &target_text, plural_target).as_deref(),
        Some(
            "Target Forest becomes an X/X Treefolk creature in addition to its other types until end of turn"
        )
    );
}

#[test]
fn plural_subtype_land_animation_keeps_addition_surface() {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default().with_subtype(Subtype::Swamp),
    ))
    .with_count(ChoiceCount {
        min: 0,
        max: Some(2),
        dynamic_x: false,
        up_to_x: false,
        random: false,
        explicit_exactly: false,
    });
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        target,
        crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    )
    .with_type_retention_surface(Some(
        ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes,
    ));
    effect
        .additional_modifications
        .push(crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(3),
            toughness: Value::Fixed(5),
            sublayer: crate::continuous::PtSublayer::Setting,
        });
    effect
        .additional_modifications
        .push(crate::continuous::Modification::AddSubtypes(vec![
            Subtype::Treefolk,
            Subtype::Warrior,
        ]));

    let (target_text, plural_target) = describe_apply_continuous_target(&effect);
    assert_eq!(
        describe_apply_continuous_animation_effect(&effect, &target_text, plural_target).as_deref(),
        Some(
            "Up to two target Swamps become Treefolk Warrior creatures with base power and toughness 3/5 in addition to their other types until end of turn"
        )
    );
}

#[test]
fn quoted_token_abilities_use_token_self_reference_for_activation_costs() {
    assert_eq!(
        quote_token_granted_ability_text("Sacrifice this creature, add {c}"),
        "\"Sacrifice this token: Add {C}.\""
    );
    assert_eq!(
        quote_token_granted_ability_text("{t}, Sacrifice this artifact, add {r} or {g}"),
        "\"{T}, Sacrifice this token: Add {R} or {G}.\""
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this creature, add {C}\""
        ),
        "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\""
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target creature gains \"When this creature dies, create a Wicked Role token attached to it.\" until end of turn."
        ),
        "Target creature gains \"When this creature dies, create a Wicked Role token attached to it.\" until end of turn."
    );
    assert_eq!(
        quote_token_granted_ability_text(
            "When this token dies, create a 2/2 red Dragon creature token with flying and \"{R}: This token gets +1/+0 until end of turn.\""
        ),
        "\"When this token dies, create a 2/2 red Dragon creature token with flying and '{R}: This token gets +1/+0 until end of turn.'\""
    );
}

#[test]
fn cant_block_token_blueprint_uses_with_clause() {
    let token = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::from_raw(1),
        "Fungus",
    )
    .token()
    .card_types(vec![CardType::Creature])
    .subtypes(vec![Subtype::Fungus])
    .color_indicator(crate::ColorSet::BLACK)
    .power_toughness(crate::PowerToughness::fixed(1, 1))
    .with_ability(Ability::static_ability(
        crate::static_abilities::StaticAbility::cant_block(),
    ))
    .build();

    assert_eq!(
        describe_token_blueprint(&token),
        "1/1 black Fungus creature token with \"This token can't block.\""
    );
}

#[test]
fn chosen_characteristic_token_blueprint_uses_dynamic_source_qualifiers() {
    let token = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::from_raw(1),
        "Creature",
    )
    .token()
    .card_types(vec![CardType::Creature])
    .power_toughness(crate::PowerToughness::fixed(2, 2))
    .build();
    let create = crate::effects::CreateTokenEffect::one(token)
        .with_source_chosen_color()
        .with_source_chosen_creature_type();

    assert_eq!(
        describe_create_token_blueprint(&create),
        "2/2 creature token of the chosen color and type"
    );
}

#[test]
fn named_token_keyword_order_follows_typed_creature_surface() {
    let storm_crow = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::from_raw(1),
        "Storm Crow",
    )
    .token()
    .card_types(vec![CardType::Creature])
    .subtypes(vec![Subtype::Bird])
    .color_indicator(crate::ColorSet::BLUE)
    .power_toughness(crate::PowerToughness::fixed(1, 2))
    .with_ability(Ability::static_ability(
        crate::static_abilities::StaticAbility::flying(),
    ))
    .build();
    assert_eq!(
        describe_token_blueprint(&storm_crow),
        "1/2 blue Bird creature token with flying named Storm Crow"
    );

    let zeppelin = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::from_raw(2),
        "Zeppelin",
    )
    .token()
    .card_types(vec![CardType::Artifact])
    .subtypes(vec![Subtype::Vehicle])
    .color_indicator(crate::ColorSet::COLORLESS)
    .power_toughness(crate::PowerToughness::fixed(5, 5))
    .with_abilities(vec![
        Ability::static_ability(crate::static_abilities::StaticAbility::make_colorless(
            ObjectFilter::source(),
        )),
        Ability::static_ability(crate::static_abilities::StaticAbility::flying()),
    ])
    .build();
    assert_eq!(
        describe_token_blueprint(&zeppelin),
        "5/5 colorless Vehicle artifact token named Zeppelin with flying"
    );
}

#[test]
fn inline_token_rule_surfaces_prefer_with_and_trim_nonfinal_periods() {
    assert!(token_extra_abilities_prefer_with_clause(&[
        "\"This token gets +2/+2 as long as an artifact entered this turn.\"".to_string(),
    ]));
    assert!(token_extra_abilities_prefer_with_clause(&[
        "\"Spells you cast cost {2} less to cast.\"".to_string(),
        "\"{T}: Draw a card.\"".to_string(),
    ]));

    let mut abilities = vec![
        "\"Spells you cast cost {2} less to cast.\"".to_string(),
        "\"{T}: Draw a card.\"".to_string(),
    ];
    strip_nonfinal_quoted_ability_periods(&mut abilities);
    assert_eq!(abilities[0], "\"Spells you cast cost {2} less to cast\"");
    assert_eq!(abilities[1], "\"{T}: Draw a card.\"");
}

#[test]
fn nonfinal_quoted_granted_abilities_move_the_list_comma_inside() {
    assert_eq!(
        join_granted_ability_list(&[
            "haste".to_string(),
            "\"When this permanent leaves the battlefield, draw a card.\"".to_string(),
            "\"At the beginning of your end step, sacrifice this permanent.\"".to_string(),
        ]),
        "haste, \"When this permanent leaves the battlefield, draw a card,\" and \"At the beginning of your end step, sacrifice this permanent.\""
    );
}

#[test]
fn quoted_token_characteristic_definition_has_inner_terminal_period() {
    assert_eq!(
        quote_token_granted_ability_text(
            "This token's power and toughness are each equal to the number of artifacts and/or creatures you control",
        ),
        "\"This token's power and toughness are each equal to the number of artifacts and/or creatures you control.\""
    );
}

#[test]
fn characteristic_defining_token_blueprint_omits_placeholder_zero_zero() {
    let count = Value::Count(ObjectFilter::land().you_control());
    let token =
        crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::from_raw(1), "Ox")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Ox])
            .color_indicator(crate::ColorSet::BLUE)
            .power_toughness(crate::PowerToughness::fixed(0, 0))
            .with_ability(Ability::static_ability(
                crate::static_abilities::StaticAbility::characteristic_defining_pt(
                    count.clone(),
                    count,
                ),
            ))
            .build();

    let blueprint = describe_token_blueprint(&token);
    assert!(!blueprint.contains("0/0"), "{blueprint}");
    assert!(
        blueprint.starts_with("blue Ox creature token with "),
        "{blueprint}"
    );
}

#[test]
fn legendary_named_artifact_token_blueprint_uses_leading_name_surface() {
    let token = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::from_raw(1),
        "Tamiyo's Notebook",
    )
    .token()
    .supertypes(vec![crate::types::Supertype::Legendary])
    .card_types(vec![CardType::Artifact])
    .subtypes(vec![Subtype::Book])
    .with_ability(Ability::static_ability(
        crate::static_abilities::StaticAbility::make_colorless(ObjectFilter::source()),
    ))
    .build();

    assert_eq!(
        describe_token_blueprint(&token),
        "Tamiyo's Notebook, a legendary colorless Book artifact token"
    );
}

#[test]
fn toxic_token_blueprint_keeps_toxic_as_keyword() {
    let toxic = Ability {
        kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::this_deals_combat_damage_to_player(
                PlayerFilter::Any,
            ),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::PoisonCountersEffect::new(1, PlayerFilter::DamagedPlayer),
            )]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: Some(crate::ability::PresentationLabel::Keyword(
                crate::ability::PresentationKeyword::Toxic(1),
            )),
        }),
        functional_zones: vec![Zone::Battlefield],
    };
    let token = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::from_raw(1),
        "Phyrexian",
    )
    .token()
    .card_types(vec![CardType::Artifact, CardType::Creature])
    .subtypes(vec![Subtype::Phyrexian, Subtype::Mite])
    .power_toughness(crate::PowerToughness::fixed(1, 1))
    .with_abilities(vec![
        toxic,
        Ability::static_ability(crate::static_abilities::StaticAbility::cant_block()),
    ])
    .build();

    assert_eq!(
        describe_token_blueprint(&token),
        "1/1 colorless Phyrexian Mite artifact creature token with toxic 1 and \"This token can't block.\""
    );
}

#[test]
fn token_with_non_toxic_poison_trigger_does_not_promote_toxic_surface() {
    let toxic = Ability {
        kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::this_deals_combat_damage_to_player(
                PlayerFilter::Any,
            ),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::PoisonCountersEffect::new(1, PlayerFilter::DamagedPlayer),
            )]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    };
    let poison_trigger = Ability {
        kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::this_deals_damage_to_player(PlayerFilter::Any, None),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::PoisonCountersEffect::new(1, PlayerFilter::You),
            )]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    };
    let token = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::from_raw(1),
        "Snake",
    )
    .token()
    .card_types(vec![CardType::Artifact, CardType::Creature])
    .subtypes(vec![Subtype::Snake])
    .power_toughness(crate::PowerToughness::fixed(1, 1))
    .with_abilities(vec![toxic, poison_trigger])
    .build();

    assert_eq!(
        describe_token_blueprint(&token),
        "1/1 colorless Snake artifact creature token. It has \"Whenever this token deals combat damage to a player, that player gets a poison counter\" and \"Whenever this token deals damage to a player, you get a poison counter\""
    );
}

#[test]
fn normalize_recent_regression_surfaces() {
    assert_eq!(
        normalize_common_semantic_phrasing(
            "This enchantment enters with X hope counters on it, where X is the number of a creature you control. Then if this enchantment has no hope counters on it, sacrifice this enchantment, then gain 4 life."
        ),
        "This enchantment enters with a hope counter on it for each creature you control. Then if this enchantment has no hope counters on it, sacrifice it and you gain 4 life."
    );
    assert_eq!(
        normalize_common_semantic_phrasing("Then if that object is a Villain, draw a card."),
        "If that creature was a Villain, draw a card."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "This creature has trample as long as it has two or fewer oil counters on it otherwise it has hexproof. Then if it has no oil counters on it, sacrifice this creature."
        ),
        "This creature has trample as long as it has two or fewer oil counters on it. Otherwise, it has hexproof. Then if it has no oil counters on it, sacrifice it."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "{7}: This creature gets +2/+0 until end of turn, this creature gains trample until end of turn and can attack this turn as though it didn't have defender."
        ),
        "{7}: This creature gets +2/+0 and gains trample until end of turn. It can attack this turn as though it didn't have defender."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever you cast a black spell, if that object is a tapped permanent, you may destroy target creature."
        ),
        "Whenever you cast a black spell, you may destroy target creature if it's tapped."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever you cast your second spell each turn, choose target creature an opponent controls. Then if it's a tapped permanent, put a stun counter on it. Otherwise, tap it."
        ),
        "Whenever you cast your second spell each turn, choose target creature an opponent controls. If it's tapped, put a stun counter on it. Otherwise, tap it."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "For each opponent, gain control of up to one target creature that player controls until end of turn, untap that creature, then it gains haste until end of turn."
        ),
        "For each opponent, gain control of up to one target creature that player controls until end of turn. Untap those creatures. They gain haste until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When this creature enters, for each opponent, gain control of up to one target creature that player controls until end of turn. Untap that creature. It gains haste until end of turn."
        ),
        "When this creature enters, for each opponent, gain control of up to one target creature that player controls until end of turn. Untap those creatures. They gain haste until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "IV — For each opponent, Gain control of up to one target creature that player controls until end of turn. Untap that creature. It gains haste until end of turn. The Ring tempts you."
        ),
        "IV — For each opponent, gain control of up to one target creature that player controls until end of turn. Untap those creatures. They gain haste until end of turn. The Ring tempts you."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever this creature blocks or becomes blocked by a creature, this creature deals 3 damage to it. This creature deals 3 damage to that object's controller."
        ),
        "Whenever this creature blocks or becomes blocked by a creature, this creature deals 3 damage to that creature and 3 damage to that creature's controller."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Breath of Darigaaz deals 1 damage to each creature without flying. Breath of Darigaaz deals 1 damage to each player. If this spell was kicked, Breath of Darigaaz deals 4 damage to each creature without flying. Breath of Darigaaz deals 4 damage to each player instead."
        ),
        "Breath of Darigaaz deals 1 damage to each creature without flying and each player. If this spell was kicked, Breath of Darigaaz deals 4 damage to each creature without flying and each player instead."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "When this creature enters, this creature deals 1 damage to target creature, then this creature deals 1 damage to you."
        ),
        "When this creature enters, this creature deals 1 damage to target creature and 1 damage to you."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Landfall — Whenever a land you control enters, this creature deals 1 damage to any target. If it's a Mountain, this creature deals 2 damage instead."
        ),
        "Landfall — Whenever a land you control enters, this creature deals 1 damage to any target. If that land is a Mountain, this creature deals 2 damage instead."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Slaying Fire deals 3 damage to any target. If at least three {R} mana was spent to cast this spell, Slaying Fire deals 4 damage instead."
        ),
        "Slaying Fire deals 3 damage to any target. Adamant — If at least three red mana was spent to cast this spell, it deals 4 damage instead."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Goblin Barrage deals 4 damage to target creature. Then if this spell was kicked, Goblin Barrage deals 4 damage to target player or planeswalker."
        ),
        "Goblin Barrage deals 4 damage to target creature. If this spell was kicked, it also deals 4 damage to target player or planeswalker."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Quenchable Fire deals 3 damage to target player or planeswalker. At the beginning of your next upkeep, Quenchable Fire deals 3 damage to a planeswalker unless that player or that object's controller pays {U}."
        ),
        "Quenchable Fire deals 3 damage to target player or planeswalker. It deals an additional 3 damage to that player or planeswalker at the beginning of your next upkeep step unless that player or that planeswalker's controller pays {U} before that step."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Vibro-shock gauntlets — When Shocker enters, Shocker deals 2 damage to target creature and 2 damage to that creature's controller."
        ),
        "Vibro-Shock Gauntlets — When Shocker enters, Shocker deals 2 damage to target creature and 2 damage to that creature's controller."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Serpentine Spike deals 2 damage to target creature. Serpentine Spike deals 3 damage to another target creature. Serpentine Spike deals 4 damage to another target creature. Then if a creature dealt damage this way would die this turn, exile it instead."
        ),
        "Serpentine Spike deals 2 damage to target creature, 3 damage to another target creature, and 4 damage to a third target creature. If a creature dealt damage this way would die this turn, exile it instead."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Whenever this creature attacks, choose target creature defending player controls. Target permanent must block target permanent if able this turn."
        ),
        "Whenever this creature attacks, target creature defending player controls blocks it this combat if able."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target creature gets +2/+2 until end of turn. If it's a Human, it gets +3/+3 and gains indestructible until end of turn instead."
        ),
        "Target creature gets +2/+2 until end of turn. If it's a Human, instead it gets +3/+3 and gains indestructible until end of turn."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Target creature gets +2/+2 until end of turn. If you had a land enter under your control this turn, it gets +4/+4 until end of turn instead."
        ),
        "Target creature gets +2/+2 until end of turn. If you had a land enter under your control this turn, it gets +4/+4 until end of turn instead."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "Magma Burst deals 3 damage to any target. Then if this spell was kicked, Magma Burst deals 3 damage to any other target."
        ),
        "Magma Burst deals 3 damage to any target. If this spell was kicked, it deals 3 damage to another target."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "You may You put those cards on the bottom of your library in any order."
        ),
        "You may put those cards on the bottom of your library in any order."
    );
    assert_eq!(
        normalize_common_semantic_phrasing(
            "You may You may repeat this process any number of times."
        ),
        "You may repeat this process any number of times."
    );
}
