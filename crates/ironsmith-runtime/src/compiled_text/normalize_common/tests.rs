use super::*;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};

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
        "When this creature enters, you may search your library and/or graveyard for a card named Huatli Dinosaur Knight, reveal it, then put it into your hand. If you searched your library this way, shuffle."
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
            "When this creature enters, other Coward creatures target opponent controls have base power and toughness 1/1 until end of turn."
        ),
        "When this creature enters, each creature target opponent controls loses all abilities, becomes a Coward in addition to its other types, and has base power and toughness 1/1."
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
            "Choose target creature. Repeat roll a d6 X times. If you roll 2, 4, or 6, put two -1/-1 counters on it. If you roll 1, 3, or 5, create a 1/2 blue Bird creature token named Storm Crow with flying."
        ),
        "Choose target creature. Roll X six-sided dice. For each even result, put two -1/-1 counters on that creature. For each odd result, create a 1/2 blue Bird creature token with flying named Storm Crow."
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
fn land_animation_prefers_still_land_surface_for_land_targets() {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        target,
        crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    );
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
            "Target land you control becomes an elemental creature with base power and toughness 4/4 until end of turn. It's still a land"
        )
    );
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
    );
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
            "Target Forest becomes an X/X treefolk creature in addition to its other types until end of turn"
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
    });
    let mut effect = crate::effects::ApplyContinuousEffect::with_spec(
        target,
        crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    );
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
            "Up to two target Swamps become treefolk warrior creatures with base power and toughness 3/5 in addition to their other types until end of turn"
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
}
