use super::*;
use crate::CardType;

fn lex_words(text: &str) -> Vec<OwnedLexToken> {
    let mut tokens = crate::lexer::lex_line(text, 0).expect("test phrase should lex");
    for token in &mut tokens {
        token.lowercase_word();
    }
    tokens
}

#[test]
fn equal_to_parser_returns_typed_word_boundaries() {
    assert_eq!(
        parse_equal_to_start(&["where", "x", "is", "equal", "to", "the"]),
        Some(EqualToStart { start: 3, after: 5 })
    );
    assert_eq!(parse_equal_to_start(&["not", "equal"]), None);
}

#[test]
fn counted_set_keeps_the_same_effect_target_controller_relation() {
    let value = parse_equal_to_number_of_filter_value(&lex_words(
        "equal to the number of nonbasic lands that creature's controller controls",
    ))
    .expect("relative target-controller count should parse");
    let Value::SurfaceHinted { value, .. } = value else {
        panic!("equal-to surface should be retained: {value:?}");
    };
    let Value::Count(filter) = *value else {
        panic!("expected an object count: {value:?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Land]);
    assert!(!filter.card_types.contains(&CardType::Creature));
    assert!(
        filter
            .excluded_supertypes
            .contains(&crate::Supertype::Basic)
    );
    assert_eq!(
        filter.controller,
        Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
    );
}

#[test]
fn mana_symbol_spent_value_preserves_symbol_and_cast_reference() {
    for (text, expected_symbol, expected_reference) in [
        (
            "where X is the amount of {S} spent to cast this spell",
            crate::mana::ManaSymbol::Snow,
            ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell,
        ),
        (
            "the amount of {U} spent to cast it",
            crate::mana::ManaSymbol::Blue,
            ironsmith_core::ManaSpentCastReferenceSurface::It,
        ),
        (
            "amount of {G} spent to cast this creature",
            crate::mana::ManaSymbol::Green,
            ironsmith_core::ManaSpentCastReferenceSurface::ThisCreature,
        ),
    ] {
        assert_eq!(
            parse_mana_symbol_spent_to_cast_value(&lex_words(text)),
            Some(Value::ManaSymbolSpentToCastThisSpell {
                symbol: expected_symbol,
                reference: expected_reference,
            }),
            "{text}",
        );
    }
}

#[test]
fn mana_symbol_spent_value_rejects_non_exact_or_multi_symbol_surfaces() {
    for text in [
        "where X is the amount of {S}{S} spent to cast this spell",
        "where X is the amount of {S} spent to cast this spell, then draw a card",
        "where X is the number of {S} spent to cast this spell",
    ] {
        assert!(
            parse_mana_symbol_spent_to_cast_value(&lex_words(text)).is_none(),
            "{text}",
        );
    }
}

#[test]
fn dynamic_filter_comparisons_preserve_prefix_vs_postfix_surface() {
    let prefix = ["less", "than", "or", "equal", "to", "your", "life", "total"];
    let (prefix_comparison, prefix_used) =
        parse_filter_comparison_tokens("power", &prefix, &prefix)
            .expect("comparison parse should succeed")
            .expect("explicit comparison should parse");
    let crate::filter::Comparison::LessThanOrEqualExpr(prefix_value) = prefix_comparison else {
        panic!("expected dynamic less-than-or-equal comparison");
    };
    assert_eq!(prefix_used, prefix.len());
    assert!(prefix_value.has_surface_hint(ValueSurfaceHint::ExplicitComparison));

    let postfix = ["your", "life", "total", "or", "less"];
    let (postfix_comparison, postfix_used) =
        parse_filter_comparison_tokens("power", &postfix, &postfix)
            .expect("comparison parse should succeed")
            .expect("postfix comparison should parse");
    let crate::filter::Comparison::LessThanOrEqualExpr(postfix_value) = postfix_comparison else {
        panic!("expected dynamic less-than-or-equal comparison");
    };
    assert_eq!(postfix_used, postfix.len());
    assert!(!postfix_value.has_surface_hint(ValueSurfaceHint::ExplicitComparison));

    let greater_prefix = [
        "is", "greater", "than", "or", "equal", "to", "your", "life", "total",
    ];
    let (greater_comparison, _) =
        parse_filter_comparison_tokens("power", &greater_prefix, &greater_prefix)
            .expect("comparison parse should succeed")
            .expect("explicit greater-than comparison should parse");
    let crate::filter::Comparison::GreaterThanOrEqualExpr(greater_value) = greater_comparison
    else {
        panic!("expected dynamic greater-than-or-equal comparison");
    };
    assert!(greater_value.has_surface_hint(ValueSurfaceHint::ExplicitComparison));
}

#[test]
fn parse_aggregate_scope_value_lexed_uses_captured_metric_and_scope() {
    let color_tokens = lex_words("colors among creatures you control");
    let color_value = parse_aggregate_scope_value_lexed(&color_tokens)
        .expect("colors-among aggregate should parse");
    let Value::ColorsAmong(color_filter) = color_value else {
        panic!("expected colors-among value, got {color_value:?}");
    };
    assert_eq!(color_filter.card_types, vec![CardType::Creature]);
    assert_eq!(color_filter.controller, Some(PlayerFilter::You));

    let power_tokens = lex_words("different powers among creatures you control");
    let power_value = parse_aggregate_scope_value_lexed(&power_tokens)
        .expect("distinct-powers aggregate should parse");
    let Value::DistinctPowers(power_filter) = power_value else {
        panic!("expected distinct-powers value, got {power_value:?}");
    };
    assert_eq!(power_filter.card_types, vec![CardType::Creature]);
    assert_eq!(power_filter.controller, Some(PlayerFilter::You));

    let name_tokens = lex_words("differently named lands you control");
    let name_value = parse_aggregate_scope_value_lexed(&name_tokens)
        .expect("distinct-name aggregate should parse");
    let Value::DistinctNames(name_filter) = name_value else {
        panic!("expected distinct-name value, got {name_value:?}");
    };
    assert_eq!(name_filter.card_types, vec![CardType::Land]);
    assert_eq!(name_filter.controller, Some(PlayerFilter::You));
}

#[test]
fn parse_spells_cast_this_turn_matching_count_value_lexed_uses_captured_suffix() {
    let tokens = lex_words("other creature spells an opponent has cast this turn");
    let value = parse_spells_cast_this_turn_matching_count_value_lexed(&tokens)
        .expect("spell-cast count should parse");
    let Value::SpellsCastThisTurnMatching {
        player,
        filter,
        exclude_source,
    } = value
    else {
        panic!("expected spell-cast matching value, got {value:?}");
    };
    assert_eq!(player, PlayerFilter::Opponent);
    assert!(exclude_source);
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
}

#[test]
fn parse_total_mana_value_of_other_spells_cast_this_turn_as_history_aggregate() {
    let tokens = lex_words("equal to the total mana value of other spells you've cast this turn");
    let value = parse_equal_to_aggregate_filter_value(&tokens)
        .expect("spell-cast mana-value aggregate should parse");
    let Value::SurfaceHinted { value, .. } = value else {
        panic!("expected equal-to surface hint");
    };
    let Value::TotalManaValueOfSpellsCastThisTurnMatching {
        player,
        filter,
        exclude_source,
    } = value.as_ref()
    else {
        panic!("expected spell-history mana-value aggregate, got {value:?}");
    };
    assert_eq!(*player, PlayerFilter::You);
    assert!(*exclude_source);
    assert!(!filter.other, "source exclusion is carried by the query");
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
}

#[test]
fn turn_history_counts_keep_event_metric_and_typed_filters() {
    let cases = [
        ("Zubera that died this turn", "Died"),
        (
            "nontoken creatures that died under your control this turn",
            "Died",
        ),
        (
            "nontoken creatures you controlled that died this turn",
            "Died",
        ),
        ("tokens you created this turn", "TokensCreated"),
        (
            "lands that entered the battlefield under your control this turn",
            "EnteredBattlefield",
        ),
        (
            "cards that were put into your graveyard from your hand or library this turn",
            "PutIntoGraveyard",
        ),
        (
            "spells you've cast from anywhere other than your hand this turn",
            "SpellsCast",
        ),
        (
            "instant and sorcery spells you've cast this turn",
            "SpellsCast",
        ),
        (
            "instant and sorcery spells cast before that spell this turn",
            "SpellsCast",
        ),
        (
            "colors among permanents you control and spells you've cast this turn",
            "ColorsAmongPermanentsAndSpellsCast",
        ),
        (
            "+1/+1 counters you've put on creatures under your control this turn",
            "CountersPutOn",
        ),
        (
            "untapped lands they controlled at the beginning of this turn",
            "UntappedLandsAtTurnStart",
        ),
        ("times you descended this turn", "Descended"),
    ];

    for (text, expected) in cases {
        let value = parse_turn_history_count_value(&lex_words(text))
            .unwrap_or_else(|| panic!("history count should parse: {text}"));
        let debug = format!("{value:?}");
        assert!(
            debug.contains("TurnHistoryCount") && debug.contains(expected),
            "{text}: {debug}"
        );
    }
}

#[test]
fn death_history_counts_preserve_authored_controller_order() {
    for (text, expected_surface) in [
        (
            "nontoken creatures that died under your control this turn",
            ironsmith_core::DeathHistoryControllerSurface::DiedUnderControl,
        ),
        (
            "nontoken creatures you controlled that died this turn",
            ironsmith_core::DeathHistoryControllerSurface::ControlledThenDied,
        ),
    ] {
        let value = parse_turn_history_count_value(&lex_words(text))
            .unwrap_or_else(|| panic!("history count should parse: {text}"));
        let Value::TurnHistoryCount(TurnHistoryCount::Died {
            filter,
            controller_surface,
        }) = value
        else {
            panic!("expected a typed death-history count for {text}: {value:?}");
        };
        assert_eq!(filter.controller, Some(PlayerFilter::You), "{text}");
        assert_eq!(controller_surface, expected_surface, "{text}");
    }
}

#[test]
fn spell_cast_history_distinguishes_turn_counts_from_trigger_boundaries() {
    let cases = [
        (
            "instant and sorcery spells you've cast this turn",
            PlayerFilter::You,
            false,
            false,
        ),
        (
            "other spells you've cast this turn",
            PlayerFilter::You,
            true,
            false,
        ),
        (
            "instant and sorcery spells cast before that spell this turn",
            PlayerFilter::Any,
            false,
            true,
        ),
        (
            "other instant and sorcery spells you've cast before it this turn",
            PlayerFilter::You,
            true,
            true,
        ),
    ];

    for (text, expected_player, expected_other, expected_boundary) in cases {
        let value = parse_turn_history_count_value(&lex_words(text))
            .unwrap_or_else(|| panic!("spell history should parse: {text}"));
        let Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
            player,
            filter,
            exclude_source,
            before_triggering_spell,
            ..
        }) = value
        else {
            panic!("expected spell-cast history for {text}: {value:?}");
        };
        assert_eq!(player, expected_player, "{text}");
        assert_eq!(exclude_source, expected_other, "{text}");
        assert_eq!(before_triggering_spell, expected_boundary, "{text}");
        assert!(!filter.other, "other belongs to the history query: {text}");
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell),
            "{text}"
        );
    }
}

#[test]
fn fixed_plus_spell_history_bindings_cover_rionya_and_thunder_surfaces() {
    for (text, expected_fixed, expected_other) in [
        (
            "where X is one plus the number of instant and sorcery spells you've cast this turn",
            1,
            false,
        ),
        (
            "where X is 2 plus the number of other spells you've cast this turn",
            2,
            true,
        ),
    ] {
        let parsed = parse_turn_history_value_binding(&lex_words(text))
            .unwrap_or_else(|| panic!("fixed-plus cast history should parse: {text}"));
        let Value::Add(fixed, history) = parsed else {
            panic!("expected fixed-plus value for {text}: {parsed:?}");
        };
        assert_eq!(*fixed, Value::Fixed(expected_fixed));
        assert!(matches!(
            *history,
            Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
                player: PlayerFilter::You,
                exclude_source,
                before_triggering_spell: false,
                ..
            }) if exclude_source == expected_other
        ));
    }
}

#[test]
fn turn_history_where_bindings_precede_current_zone_counts() {
    let attractions = parse_turn_history_value_binding(&lex_words(
        "where X is the number of Attractions you've visited this turn",
    ))
    .expect("Attraction visit history should parse");
    assert_eq!(
        attractions,
        Value::AttractionsVisitedThisTurn(PlayerFilter::You)
    );

    let graveyard = parse_turn_history_value_binding(&lex_words(
        "where X is the number of cards put into their graveyard from anywhere this turn",
    ))
    .expect("graveyard provenance count should parse");
    let Value::TurnHistoryCount(TurnHistoryCount::PutIntoGraveyard { owner, from }) = graveyard
    else {
        panic!("expected graveyard-history value, got {graveyard:?}");
    };
    assert_eq!(owner, PlayerFilter::IteratedPlayer);
    assert!(from.is_empty());

    let spells = parse_turn_history_value_binding(&lex_words(
            "where X is 1 plus the number of spells you've cast from anywhere other than your hand this turn",
        ))
        .expect("fixed-plus spell provenance count should parse");
    let Value::Add(fixed, history) = spells else {
        panic!("expected fixed-plus history value, got {spells:?}");
    };
    assert_eq!(*fixed, Value::Fixed(1));
    let Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
        player,
        filter,
        from_outside_hand,
        ..
    }) = *history
    else {
        panic!("expected spell-cast history value, got {history:?}");
    };
    assert_eq!(player, PlayerFilter::You);
    assert!(from_outside_hand);
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
    assert!(!filter.has_mana_cost);
}

#[test]
fn dynamic_token_where_bindings_use_typed_turn_history_values() {
    let descended = parse_turn_history_value_binding(&lex_words(
        "where X is the number of times you descended this turn",
    ))
    .expect("descend count should parse");
    assert!(matches!(
        descended,
        Value::TurnHistoryCount(TurnHistoryCount::Descended(PlayerFilter::You))
    ));

    let damage = parse_turn_history_value_binding(&lex_words(
        "where X is the amount of damage dealt to it this turn",
    ))
    .expect("source damage total should parse");
    assert!(matches!(
        damage,
        Value::TurnHistoryCount(TurnHistoryCount::DamageDealtToSource)
    ));
}

#[test]
fn parses_opponents_dealt_combat_damage_without_a_source_qualifier() {
    let value = parse_turn_history_count_value(&lex_words(
        "opponents that were dealt combat damage this turn",
    ))
    .expect("combat-damaged opponent count should parse");
    assert!(matches!(
        value,
        Value::TurnHistoryCount(TurnHistoryCount::PlayersDealtCombatDamageBy {
            players: PlayerFilter::Opponent,
            sources,
        }) if sources == ObjectFilter::default()
    ));
}

#[test]
fn parses_authored_possessive_opponents_who_lost_life_count() {
    let value = parse_turn_history_count_value(&lex_words(
        "for each of your opponents who lost life this turn",
    ))
    .expect("distinct opponents who lost life should parse");
    assert!(matches!(
        value,
        Value::TurnHistoryCount(TurnHistoryCount::PlayersLostLife(PlayerFilter::Opponent))
    ));
}

#[test]
fn turn_history_values_require_complete_supported_provenance_surfaces() {
    assert!(
        parse_turn_history_count_value(&lex_words(
            "Zubera that died this turn among creatures you control"
        ))
        .is_none()
    );
    assert!(
        parse_turn_history_count_value(&lex_words(
            "cards with flying put into your graveyard from your hand or library this turn"
        ))
        .is_none()
    );
    assert!(
            parse_turn_history_value_binding(&lex_words(
                "where X is the number of cards put into their graveyard from anywhere this turn plus one"
            ))
            .is_none()
        );
    assert!(
        parse_turn_history_count_value(&lex_words("Treasure tokens you created this turn"))
            .is_none(),
        "typed created-token counts require a token-filter/creator-aware model"
    );
}

#[test]
fn equal_to_number_of_differently_named_objects_keeps_distinctness() {
    let tokens = lex_words("equal to the number of differently named creature tokens you control");
    let value = parse_equal_to_number_of_filter_value(&tokens)
        .expect("Audience with Trostani count should parse");
    let Value::SurfaceHinted { value, hints } = value else {
        panic!("expected equal-to surface hint");
    };
    assert_eq!(hints, vec![ValueSurfaceHint::EqualTo]);
    let Value::DistinctNames(filter) = *value else {
        panic!("expected a distinct-name count");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(filter.token);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.name, None);
}

#[test]
fn equal_to_hand_count_preserves_authored_that_player_possessive() {
    for (text, expected_hint) in [
        ("equal to the number of cards in that player's hand", true),
        ("equal to the number of cards in their hand", false),
    ] {
        let value = parse_equal_to_number_of_filter_value(&lex_words(text))
            .unwrap_or_else(|| panic!("player-relative hand count should parse: {text}"));
        assert!(
            value.has_surface_hint(ValueSurfaceHint::EqualTo),
            "{value:#?}"
        );
        assert_eq!(
            value.has_surface_hint(ValueSurfaceHint::ThatPlayerPossessive),
            expected_hint,
            "{text}: {value:#?}"
        );
        assert!(matches!(
            value.unhinted(),
            Value::CardsInHand(PlayerFilter::IteratedPlayer)
        ));
    }
}

#[test]
fn equal_to_number_of_players_with_minimum_hand_size_keeps_both_filters() {
    let value = parse_equal_to_number_of_filter_value(&lex_words(
        "equal to the number of your opponents with four or more cards in hand",
    ))
    .expect("qualified opponent count should parse");
    let Value::SurfaceHinted { value, hints } = value else {
        panic!("expected equal-to surface hint");
    };
    assert_eq!(hints, vec![ValueSurfaceHint::EqualTo]);
    assert_eq!(
        *value,
        Value::CountPlayersWithCardsInHandAtLeast(PlayerFilter::Opponent, 4)
    );
}

#[test]
fn minimum_hand_size_player_count_does_not_claim_other_count_domains() {
    for text in [
        "creatures with four or more cards in hand",
        "your opponents with four or fewer cards in hand",
        "your opponents with four or more cards in graveyard",
        "cards in your opponents' hands",
    ] {
        assert!(
            parse_players_with_cards_in_hand_at_least(&lex_words(text)).is_none(),
            "the qualified-player parser must not claim {text:?}"
        );
    }
}

#[test]
fn equal_to_number_of_tapped_this_way_keeps_typed_action() {
    let value = parse_equal_to_number_of_filter_value(&lex_words(
        "equal to the number of creatures tapped this way",
    ))
    .expect("tapped-this-way equal count should parse");
    let Value::SurfaceHinted { value, hints } = value else {
        panic!("expected equal-to surface hint");
    };
    assert_eq!(hints, vec![ValueSurfaceHint::EqualTo]);
    let Value::PendingPriorEffectMetric(query) = *value else {
        panic!("expected typed tapped prior-effect metric");
    };
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Tapped)
    );
    assert_eq!(query.metric, EffectMetric::Count);
}

#[test]
fn equal_to_party_count_plus_fixed_keeps_typed_party_value() {
    let value = parse_equal_to_number_of_filter_plus_or_minus_fixed_value(&lex_words(
        "equal to the number of creatures in your party plus two",
    ))
    .expect("equal-to party offset should parse");
    let Value::SurfaceHinted { value, hints } = value else {
        panic!("expected equal-to surface hint");
    };
    assert_eq!(hints, vec![ValueSurfaceHint::EqualTo]);
    assert_eq!(
        *value,
        Value::Add(
            Box::new(Value::PartySize(PlayerFilter::You)),
            Box::new(Value::Fixed(2)),
        )
    );
}
