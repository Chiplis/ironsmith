use super::*;
use crate::lexer::lex_line;
use crate::object::CounterType;
use crate::target::SourceReferenceSurface;

#[test]
fn parses_rounded_and_tagged_value_expressions() {
    assert_eq!(
        parse_value_expr_words(&["half", "x", "rounded", "up"]),
        Some((
            Value::HalfRoundedDown(Box::new(Value::Add(
                Box::new(Value::X),
                Box::new(Value::Fixed(1)),
            ))),
            4,
        ))
    );
    assert_eq!(
        parse_value_expr_words(&["the", "exploited", "creature", "power"]),
        Some((
            Value::PowerOf(Box::new(ChooseSpec::Tagged(
                crate::tag::CompilerReferenceTag::Exploited.key()
            ))),
            4,
        ))
    );
}

#[test]
fn counted_cards_in_target_creature_controller_hand_keep_player_scope() {
    let tokens = lex_line(
        "the number of cards in that creature's controller's hand",
        0,
    )
    .expect("target-controller hand count should lex");
    let (value, used) =
        parse_value_expr_tokens(&tokens).expect("target-controller hand count should parse");
    assert_eq!(used, tokens.len());
    let Value::Count(filter) = value else {
        panic!("expected typed object count: {value:?}");
    };
    assert_eq!(filter.zone, Some(crate::zone::Zone::Hand));
    assert!(filter.card_types.is_empty());
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
    );

    let ordinary = lex_line("the number of creature cards in all players' hands", 0)
        .expect("ordinary hand count should lex");
    let (ordinary, _) = parse_value_expr_tokens(&ordinary).expect("ordinary count");
    let Value::Count(ordinary) = ordinary else {
        panic!("expected ordinary object count: {ordinary:?}");
    };
    assert_eq!(ordinary.card_types, vec![crate::CardType::Creature]);
    assert_ne!(ordinary.owner, filter.owner);
}

#[test]
fn counted_curses_attached_to_them_keep_player_attachment_scope() {
    let tokens = lex_line("the number of Curses attached to them", 0)
        .expect("attached Curse count should lex");
    let (value, used) =
        parse_value_expr_tokens(&tokens).expect("attached Curse count should parse");
    assert_eq!(used, tokens.len());
    let Value::Count(filter) = value else {
        panic!("expected attached-Curse object count: {value:?}");
    };
    assert!(filter.attached_to_object.is_none());
    assert_eq!(
        filter.attached_to_player,
        Some(PlayerFilter::AliasedTarget(Box::new(
            PlayerFilter::IteratedPlayer
        )))
    );

    let object_attachments = lex_line("the number of Auras attached to them", 0)
        .expect("object attachment near miss should lex");
    let (object_attachments, _) = parse_value_expr_tokens(&object_attachments)
        .expect("object attachment near miss should still parse");
    let Value::Count(object_attachments) = object_attachments else {
        panic!("expected ordinary object attachment count");
    };
    assert!(object_attachments.attached_to_player.is_none());
}

#[test]
fn parses_character_count_in_source_name_stickers() {
    let tokens = lex_line("the number of o's in name stickers on this enchantment", 0)
        .expect("name-sticker character-count fixture should lex");
    let (value, used) =
        parse_value_expr_tokens(&tokens).expect("name-sticker character count should parse");
    assert_eq!(used, tokens.len());
    assert_eq!(
        value,
        Value::NameStickerCharacterCountOnSource {
            character: 'o',
            surface: Some(SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string()
            )),
        }
    );
}

#[test]
fn possessive_it_characteristics_keep_the_object_antecedent() {
    assert_eq!(
        parse_value_expr_words(&["its", "power"]),
        Some((
            Value::PowerOf(Box::new(
                ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.key()).with_surface_hint(
                    ChooseSpecSurfaceHint::SourceReference(
                        SourceReferenceSurface::ThisPermanentType("it".to_string()),
                    ),
                ),
            )),
            2,
        ))
    );
    assert_eq!(
        parse_value_expr_words(&["its", "toughness"]),
        Some((
            Value::ToughnessOf(Box::new(
                ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.key()).with_surface_hint(
                    ChooseSpecSurfaceHint::SourceReference(
                        SourceReferenceSurface::ThisPermanentType("it".to_string()),
                    ),
                ),
            )),
            2,
        ))
    );
    assert_eq!(
        parse_value_expr_words(&["this", "creatures", "toughness"]),
        Some((
            Value::ToughnessOf(Box::new(source_choose_spec_for_surface(
                SourceReferenceSurface::ThisPermanentType("this creature".to_string())
            ))),
            3
        ))
    );
}

#[test]
fn parses_maximum_hand_size_as_a_bound_player_aggregate() {
    assert_eq!(
        parse_value_expr_words(&[
            "the", "number", "of", "cards", "in", "the", "hand", "of", "the", "opponent", "with",
            "the", "most", "cards", "in", "hand",
        ]),
        Some((Value::MaxCardsInHand(PlayerFilter::Opponent), 16)),
    );
}

#[test]
fn preserves_token_boundary_for_value_prefixes() {
    let tokens = lex_line("x plus two cards", 0).expect("lex fixture");
    assert_eq!(
        parse_value_expr_tokens(&tokens),
        Some((Value::Add(Box::new(Value::X), Box::new(Value::Fixed(2))), 3,))
    );
}

#[test]
fn parses_dynamic_subtraction_and_in_excess_of_as_composable_values() {
    for (operator, in_excess_of) in [
        (["minus"].as_slice(), false),
        (["in", "excess", "of"].as_slice(), true),
    ] {
        let mut words = vec!["number", "of", "creatures", "you", "control"];
        words.extend_from_slice(operator);
        words.extend([
            "number",
            "of",
            "creatures",
            "target",
            "opponent",
            "controls",
        ]);

        let (value, used) =
            parse_value_expr_words(&words).expect("dynamic difference should parse");
        assert_eq!(used, words.len());
        assert_eq!(
            value.has_surface_hint(ValueSurfaceHint::InExcessOf),
            in_excess_of,
        );
        let Value::Add(left, right) = value.unhinted() else {
            panic!("difference should be represented as composable addition");
        };
        assert!(matches!(left.as_ref(), Value::Count(_)));
        assert!(
            matches!(right.as_ref(), Value::Scaled(inner, -1) if matches!(inner.as_ref(), Value::Count(_)))
        );
    }
}

#[test]
fn hand_count_preserves_authored_that_player_possessive() {
    for (owner_words, expected_hint) in [
        (["that", "players"].as_slice(), true),
        (["their"].as_slice(), false),
    ] {
        let mut words = vec!["the", "number", "of", "cards", "in"];
        words.extend_from_slice(owner_words);
        words.push("hand");

        let (value, used) =
            parse_value_expr_words(&words).expect("player-relative hand count should parse");
        assert_eq!(used, words.len());
        assert_eq!(
            value.has_surface_hint(ValueSurfaceHint::ThatPlayerPossessive),
            expected_hint,
            "{words:?}: {value:#?}"
        );
        assert!(matches!(
            value.unhinted(),
            Value::Count(filter)
                if filter.zone == Some(crate::zone::Zone::Hand)
                    && filter.owner == Some(PlayerFilter::IteratedPlayer)
        ));
    }
}

#[test]
fn parses_triggering_cast_mana_and_excess_damage_values() {
    assert_eq!(
        parse_value_expr_words(&["the", "excess"]),
        Some((
            Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::ExcessDamage,
            },
            2
        ))
    );
    assert_eq!(
        parse_value_expr_words(&[
            "the", "amount", "of", "mana", "spent", "to", "cast", "that", "spell",
        ]),
        Some((
            Value::ManaSpentToCast(Box::new(crate::target::ChooseSpec::Tagged(
                (crate::tag::CompilerReferenceTag::It.bind()).into(),
            ))),
            9
        ))
    );
    assert_eq!(
        parse_value_expr_words(&[
            "the", "excess", "damage", "dealt", "to", "that", "creature", "this", "way",
        ]),
        Some((
            Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::Outcome,
                    ironsmith_core::EffectMetric::ExcessDamage,
                )
                .with_action(ironsmith_core::PriorEffectAction::DealtDamage)
            ),
            9,
        ))
    );
}

#[test]
fn parses_life_total_player_counter_and_source_controller_graveyard_values() {
    assert_eq!(
        parse_value_expr_words(&["your", "life", "total"]),
        Some((Value::LifeTotal(PlayerFilter::You), 3))
    );
    assert_eq!(
        parse_value_expr_words(&[
            "the", "amount", "of", "life", "you", "gained", "this", "turn",
        ]),
        Some((Value::LifeGainedThisTurn(PlayerFilter::You), 8))
    );
    assert_eq!(
        parse_value_expr_words(&[
            "the",
            "number",
            "of",
            "experience",
            "counters",
            "you",
            "have",
        ]),
        Some((
            Value::PlayerCounters(PlayerFilter::You, CounterType::Experience),
            7,
        ))
    );

    let (value, used) = parse_value_expr_words(&[
        "the",
        "number",
        "of",
        "creature",
        "cards",
        "in",
        "its",
        "controller",
        "graveyard",
    ])
    .expect("source-controller graveyard count");
    assert_eq!(used, 9);
    let Value::Count(filter) = value else {
        panic!("expected object count");
    };
    assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
    assert_eq!(filter.zone, Some(crate::zone::Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
}

#[test]
fn count_value_preserves_player_or_planeswalker_controller_reference() {
    let words = [
        "the",
        "number",
        "of",
        "creatures",
        "that",
        "opponent",
        "or",
        "that",
        "planeswalkers",
        "controller",
        "controls",
    ];
    let (value, used) =
        parse_value_expr_words(&words).expect("controller-relative count should parse");

    assert_eq!(used, words.len());
    let Value::Count(filter) = value else {
        panic!("expected an object count");
    };
    assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
    assert_eq!(
        filter.controller,
        Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
    );
    assert!(
        !filter
            .card_types
            .contains(&crate::types::CardType::Planeswalker)
    );
}

#[test]
fn generic_number_of_value_preserves_tapped_this_way_link() {
    let (value, used) =
        parse_value_expr_words(&["the", "number", "of", "creatures", "tapped", "this", "way"])
            .expect("tapped-this-way count");

    assert_eq!(used, 7);
    let Value::PendingPriorEffectMetric(query) = value else {
        panic!("expected typed prior-effect metric");
    };
    assert_eq!(
        query.source,
        ironsmith_core::EffectMetricSource::AffectedObjects
    );
    assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Tapped)
    );
    assert_eq!(
        query.filter.expect("creature filter").card_types,
        vec![crate::types::CardType::Creature]
    );
}

#[test]
fn discarded_card_type_count_binds_to_the_discard_result() {
    let words = [
        "the",
        "number",
        "of",
        "card",
        "types",
        "the",
        "discarded",
        "card",
        "has",
    ];
    let (value, used) =
        parse_value_expr_words(&words).expect("discarded-card type count should parse");

    assert_eq!(used, words.len());
    let Value::PendingPriorEffectMetric(query) = value else {
        panic!("expected a typed prior-effect metric, got {value:?}");
    };
    assert_eq!(
        query.source,
        ironsmith_core::EffectMetricSource::AffectedObjects
    );
    assert_eq!(query.metric, ironsmith_core::EffectMetric::CardTypesAmong);
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Discarded)
    );
}

#[test]
fn iterated_players_exiled_creature_power_keeps_partitioned_provenance() {
    let words = ["the", "power", "of", "the", "creature", "they", "exiled"];
    let (value, used) =
        parse_value_expr_words(&words).expect("per-player exiled creature power should parse");

    assert_eq!(used, words.len());
    let Value::PendingPriorEffectMetric(query) = value else {
        panic!("expected a typed prior-effect metric")
    };
    assert_eq!(query.metric, ironsmith_core::EffectMetric::FirstPower);
    assert_eq!(query.player, Some(PlayerFilter::IteratedPlayer));
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Exiled)
    );
    assert_eq!(
        query.filter.expect("creature filter").card_types,
        vec![crate::types::CardType::Creature]
    );
}

#[test]
fn generic_number_of_value_keeps_hand_threshold_on_qualified_players() {
    let words = [
        "the",
        "number",
        "of",
        "your",
        "opponents",
        "with",
        "four",
        "or",
        "more",
        "cards",
        "in",
        "hand",
    ];
    let (value, used) =
        parse_value_expr_words(&words).expect("qualified player count should parse");

    assert_eq!(used, words.len());
    assert_eq!(
        value,
        Value::CountPlayersWithCardsInHandAtLeast(PlayerFilter::Opponent, 4)
    );
}

#[test]
fn sacrificed_characteristic_values_keep_identity_and_typed_surface() {
    let sacrificed_creature = Value::ToughnessOf(Box::new(ChooseSpec::Tagged(
        crate::tag::CompilerReferenceTag::AdditionalCostObject.key(),
    )))
    .with_surface_hint(ValueSurfaceHint::SacrificedObject(
        SacrificedObjectKind::Creature,
    ));
    assert_eq!(
        parse_value_expr_words(&["the", "sacrificed", "creature", "toughness"]),
        Some((sacrificed_creature, 4))
    );

    let sacrificed_permanent = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
        crate::tag::CompilerReferenceTag::AdditionalCostObject.key(),
    )))
    .with_surface_hint(ValueSurfaceHint::SacrificedObject(
        SacrificedObjectKind::Permanent,
    ));
    assert_eq!(
        parse_value_expr_words(&[
            "the",
            "mana",
            "value",
            "of",
            "the",
            "sacrificed",
            "permanent",
        ]),
        Some((sacrificed_permanent, 7))
    );

    let red_symbols = Value::ManaSymbolsInManaCostOf {
        spec: Box::new(ChooseSpec::Tagged(
            crate::tag::CompilerReferenceTag::AdditionalCostObject.key(),
        )),
        color: Color::Red,
    }
    .with_surface_hint(ValueSurfaceHint::SacrificedObject(
        SacrificedObjectKind::Creature,
    ));
    assert_eq!(
        parse_value_expr_words(&[
            "the",
            "number",
            "of",
            "red",
            "mana",
            "symbols",
            "in",
            "the",
            "sacrificed",
            "creatures",
            "mana",
            "cost",
        ]),
        Some((red_symbols, 12))
    );
}

#[test]
fn parses_colored_mana_symbols_across_filtered_scopes() {
    let battlefield_words = [
        "the",
        "number",
        "of",
        "green",
        "mana",
        "symbols",
        "in",
        "the",
        "mana",
        "costs",
        "of",
        "permanents",
        "you",
        "control",
    ];
    let (value, used) = parse_value_expr_words(&battlefield_words)
        .expect("battlefield mana-symbol aggregate should parse");
    assert_eq!(used, battlefield_words.len());
    let Value::ManaSymbolsInManaCostOf { spec, color } = value else {
        panic!("expected structured mana-symbol value");
    };
    assert_eq!(color, Color::Green);
    let ChooseSpec::All(filter) = spec.unhinted() else {
        panic!("expected aggregate object scope");
    };
    assert_eq!(filter.zone, Some(crate::zone::Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));

    let graveyard_words = [
        "the",
        "number",
        "of",
        "black",
        "mana",
        "symbols",
        "in",
        "the",
        "mana",
        "costs",
        "of",
        "cards",
        "in",
        "your",
        "graveyard",
    ];
    let (value, used) = parse_value_expr_words(&graveyard_words)
        .expect("graveyard mana-symbol aggregate should parse");
    assert_eq!(used, graveyard_words.len());
    let Value::ManaSymbolsInManaCostOf { spec, color } = value else {
        panic!("expected structured mana-symbol value");
    };
    assert_eq!(color, Color::Black);
    let ChooseSpec::All(filter) = spec.unhinted() else {
        panic!("expected aggregate object scope");
    };
    assert_eq!(filter.zone, Some(crate::zone::Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
}

#[test]
fn explicit_revealed_card_mana_value_keeps_reference_surface() {
    let (value, used) = parse_value_expr_words(&["the", "revealed", "card", "mana", "value"])
        .expect("revealed-card mana value");

    assert_eq!(used, 5);
    assert!(
        matches!(value.unhinted(), Value::PendingPriorEffectMetric(query)
        if query.metric == ironsmith_core::EffectMetric::FirstManaValue
            && query.action == Some(ironsmith_core::PriorEffectAction::Revealed))
    );
}

#[test]
fn its_characteristic_keeps_the_pronoun_on_the_object_reference() {
    let (value, used) =
        parse_value_expr_words(&["its", "mana", "value"]).expect("possessive mana value");
    assert_eq!(used, 3);
    let Value::ManaValueOf(spec) = value else {
        panic!("expected a typed mana-value reference");
    };
    assert!(matches!(
        spec.base(),
        ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
    ));
    assert_eq!(
        spec.source_reference_surface(),
        Some(&SourceReferenceSurface::ThisPermanentType("it".to_string()))
    );
}

#[test]
fn devotion_is_a_typed_value_expression_only_with_a_proven_owner_and_color() {
    assert_eq!(
        parse_value_expr_words(&["your", "devotion", "to", "black"]),
        Some((
            Value::Devotion {
                player: PlayerFilter::You,
                color: Color::Black,
            },
            4,
        ))
    );
    assert_eq!(
        parse_value_expr_words(&["their", "devotion", "to", "blue"]),
        Some((
            Value::Devotion {
                player: PlayerFilter::IteratedPlayer,
                color: Color::Blue,
            },
            4,
        ))
    );
    assert_eq!(
        parse_value_expr_words(&["your", "devotion", "to", "that", "color"]),
        Some((Value::DevotionToChosenColor(PlayerFilter::You), 5))
    );
    assert_eq!(
        parse_value_expr_words(&["your", "devotion", "for", "black"]),
        None,
        "near-miss prepositions must not become a devotion value"
    );
}

#[test]
fn whichever_is_greater_builds_an_executable_maximum() {
    let words = [
        "the",
        "number",
        "of",
        "zombies",
        "you",
        "control",
        "or",
        "the",
        "number",
        "of",
        "zombie",
        "cards",
        "in",
        "your",
        "graveyard",
        "whichever",
        "is",
        "greater",
    ];
    let (value, used) = parse_value_expr_words(&words).expect("maximum value should parse");
    assert_eq!(used, words.len());
    assert!(value.has_surface_hint(ValueSurfaceHint::WhicheverIsGreater));
    assert!(matches!(
        value.unhinted(),
        Value::Add(total, negative_minimum)
            if matches!(total.as_ref(), Value::Add(_, _))
                && matches!(
                    negative_minimum.as_ref(),
                    Value::Scaled(minimum, -1)
                        if matches!(minimum.as_ref(), Value::Min(_, _))
                )
    ));
}

#[test]
fn excess_damage_phrases_preserve_explicit_producer_binding() {
    for text in [
        "the amount of excess damage dealt to that creature this way",
        "the amount of excess damage dealt this way",
        "excess damage dealt to that permanent this way",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
        assert_eq!(used, tokens.len(), "{text}");
        assert!(
            matches!(value.unhinted(), Value::PendingPriorEffectMetric(query)
            if query.action == Some(ironsmith_core::PriorEffectAction::DealtDamage)
                && query.metric == ironsmith_core::EffectMetric::ExcessDamage)
        );
    }
    for text in [
        "that excess damage",
        "that amount of excess damage",
        "that much excess damage",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
        assert_eq!(used, tokens.len());
        assert!(matches!(
            value.unhinted(),
            Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::ExcessDamage,
            }
        ));
    }
}

#[test]
fn greatest_power_is_a_composable_maximum_operand_without_inner_equal_to_hint() {
    let tokens = lex_line(
        "2 or the greatest power among Dinosaurs you control, whichever is greater",
        0,
    )
    .unwrap();
    let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
    assert_eq!(used, tokens.len());
    assert!(value.has_surface_hint(ValueSurfaceHint::WhicheverIsGreater));
    let Value::Add(sum, subtract) = value.unhinted() else {
        panic!("{value:?}");
    };
    let Value::Add(left, right) = sum.as_ref() else {
        panic!("{sum:?}");
    };
    assert_eq!(left.as_ref(), &Value::Fixed(2));
    assert!(!right.has_surface_hint(ValueSurfaceHint::EqualTo));
    let Value::GreatestPower(filter) = right.unhinted() else {
        panic!("{right:?}");
    };
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.subtypes, vec![crate::types::Subtype::Dinosaur]);
    assert!(matches!(subtract.as_ref(), Value::Scaled(minimum, -1)
        if matches!(minimum.as_ref(), Value::Min(a, b) if a == left && b == right)));
}

#[test]
fn total_number_prefix_is_cardinality_and_preserves_owned_zone_union() {
    let tokens = lex_line(
        "equal to the total number of instant and sorcery cards you own in exile and in your graveyard",
        0,
    ).unwrap();
    let value =
        crate::grammar::shared_util::value_semantics::parse_equal_to_number_of_filter_value(
            &tokens,
        )
        .unwrap();
    let Value::Count(filter) = value.unhinted() else {
        panic!("{value:?}");
    };
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(
        filter.card_types,
        vec![
            crate::types::CardType::Instant,
            crate::types::CardType::Sorcery
        ]
    );
    assert!(filter.all_card_types.is_empty());
    assert_eq!(filter.any_of.len(), 2);
    let zones: Vec<_> = filter
        .any_of
        .iter()
        .filter_map(|branch| branch.zone)
        .collect();
    assert!(
        zones.contains(&crate::zone::Zone::Exile) && zones.contains(&crate::zone::Zone::Graveyard)
    );
}

#[test]
fn plural_subtype_union_and_negative_card_types_remain_complete_count_operands() {
    for (text, wolf) in [
        ("the number of Wolves and Werewolves you control", true),
        (
            "the number of noncreature, nonland cards in your graveyard",
            false,
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let value =
            crate::grammar::shared_util::value_semantics::parse_equal_to_number_of_filter_value(
                &tokens,
            )
            .unwrap();
        let Value::Count(filter) = value.unhinted() else {
            panic!("{value:?}");
        };
        if wolf {
            assert_eq!(filter.controller, Some(PlayerFilter::You));
            assert_eq!(
                filter.subtypes,
                vec![crate::types::Subtype::Wolf, crate::types::Subtype::Werewolf]
            );
            assert!(filter.all_subtypes.is_empty());
        } else {
            assert_eq!(filter.zone, Some(crate::zone::Zone::Graveyard));
            assert!(
                filter
                    .excluded_card_types
                    .contains(&crate::types::CardType::Creature)
            );
            assert!(
                filter
                    .excluded_card_types
                    .contains(&crate::types::CardType::Land)
            );
        }
    }
}

#[test]
fn coordinated_characteristic_reference_keeps_distinct_axes_of_one_object() {
    for prefix in ["this creature's", "that creature's"] {
        let tokens = lex_line(&format!("{prefix} power and toughness"), 0).unwrap();
        let view = TokenWordView::new(&tokens);
        let (power, toughness) = parse_power_toughness_value_pair_words(&view.word_refs()).unwrap();
        let Value::PowerOf(power) = power.unhinted() else {
            panic!("power");
        };
        let Value::ToughnessOf(toughness) = toughness.unhinted() else {
            panic!("toughness");
        };
        assert_eq!(power, toughness);
    }
    assert!(
        parse_power_toughness_value_pair_words(&["its", "power", "and", "toughness", "plus", "1"])
            .is_none()
    );
}

#[test]
fn named_vote_counts_and_the_highest_life_scalar_remain_composable_values() {
    for (text, expected) in [
        (
            "twice the number of profit votes",
            Value::Scaled(Box::new(Value::VoteCount("profit".into())), 2),
        ),
        (
            "the number of security votes",
            Value::VoteCount("security".into()),
        ),
        (
            "the highest life total among all players",
            Value::MaximumLifeTotal(PlayerFilter::Any),
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
        assert_eq!(used, tokens.len());
        assert_eq!(value, expected);
    }
    // A different maximum scope is not silently widened to every player.
    assert!(
        parse_value_expr_words(&["the", "highest", "life", "total", "among", "opponents"])
            .is_none()
    );
}

#[test]
fn life_extrema_and_fractional_player_counts_keep_their_scopes() {
    for (text, value) in [
        (
            "the highest life total among your opponents",
            Value::MaximumLifeTotal(PlayerFilter::Opponent),
        ),
        (
            "the highest life total among players",
            Value::MaximumLifeTotal(PlayerFilter::Any),
        ),
        (
            "the number of opponents whose life total is less than half their starting life total",
            Value::CountPlayersBelowHalfStartingLifeTotal(PlayerFilter::Opponent),
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let (parsed, used) = parse_value_expr_tokens(&tokens).unwrap();
        assert_eq!(used, tokens.len());
        assert_eq!(parsed, value);
    }
    let words = "half the highest life total among your opponents rounded up"
        .split_whitespace()
        .collect::<Vec<_>>();
    let (value, used) = parse_value_expr_words(&words).unwrap();
    assert_eq!(used, words.len());
    assert_eq!(
        value,
        Value::HalfRoundedDown(Box::new(Value::Add(
            Box::new(Value::MaximumLifeTotal(PlayerFilter::Opponent)),
            Box::new(Value::Fixed(1))
        )))
    );
}
