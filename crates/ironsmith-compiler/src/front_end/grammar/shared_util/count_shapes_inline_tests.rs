use super::*;
use crate::target::ObjectFilter;
use ironsmith_core::ValueSurfaceHint;

#[test]
fn parses_for_each_draw_and_kick_counts() {
    assert_eq!(
        parse_for_each_count_value_words(&[
            "for", "each", "card", "youve", "drawn", "this", "turn"
        ]),
        Some((Value::MaxCardsDrawnThisTurn(PlayerFilter::You), 7))
    );
    assert_eq!(
        parse_for_each_count_value_words(&[
            "for", "each", "time", "this", "spell", "was", "kicked"
        ]),
        Some((Value::KickCount, 7))
    );
    assert_eq!(
        parse_for_each_count_value_words(&[
            "for", "each", "time", "this", "creature", "was", "kicked"
        ]),
        Some((Value::KickCount, 7))
    );

    let (drawn_this_way, used) =
        parse_for_each_count_value_words(&["for", "each", "card", "drawn", "this", "way"])
            .expect("drawn-this-way count");
    assert_eq!(used, 6);
    assert!(drawn_this_way.has_surface_hint(ValueSurfaceHint::CardsDrawnThisWay));
    let Value::PendingPriorEffectMetric(query) = drawn_this_way.unhinted() else {
        panic!("expected an exact drawn-card action count, got {drawn_this_way:#?}");
    };
    assert_eq!(query.action, Some(ironsmith_core::PriorEffectAction::Drawn));
    assert_eq!(
        query.source,
        ironsmith_core::EffectMetricSource::AffectedObjects
    );
    assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
}

#[test]
fn died_this_way_count_preempts_current_battlefield_filter() {
    let words = ["for", "each", "creature", "that", "died", "this", "way"];
    let (value, used) =
        parse_for_each_count_value_words(&words).expect("died-this-way count should parse");
    assert_eq!(used, words.len());
    assert!(value.has_surface_hint(ValueSurfaceHint::DiedThisWay));
    let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
        panic!("expected a prior destroy-result metric, got {value:#?}");
    };
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Destroyed)
    );
}

#[test]
fn for_each_shared_terminal_subtype_color_card_keeps_union_semantics() {
    let words = ["for", "each", "forest", "and", "green", "card"];
    let (value, used) =
        parse_for_each_count_value_words(&words).expect("shared-terminal union count");
    assert_eq!(used, words.len());
    let Value::Count(filter) = value.unhinted() else {
        panic!("expected a typed object count, got {value:#?}");
    };
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert_eq!(filter.any_of[0].subtypes, [crate::Subtype::Forest]);
    assert_eq!(filter.any_of[1].colors, Some(crate::ColorSet::GREEN));
    assert!(filter.has_conjunctive_set_surface());
}

#[test]
fn typed_counter_removed_count_uses_action_count_and_preserves_counter_kind() {
    let words = ["for", "each", "lore", "counter", "removed", "this", "way"];
    let (value, used) =
        parse_for_each_count_value_words(&words).expect("typed removed-counter count");
    assert_eq!(used, words.len());
    assert!(value.has_surface_hint(ValueSurfaceHint::CountersRemovedThisWay));
    let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
        panic!("expected an exact prior-action count, got {value:#?}");
    };
    assert_eq!(query.source, ironsmith_core::EffectMetricSource::Outcome);
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Removed)
    );
    assert_eq!(query.counter_type, Some(crate::object::CounterType::Lore));
    assert!(query.filter.is_none());
}

#[test]
fn parses_generic_mana_source_spent_to_cast_counts() {
    let cave_words = [
        "for", "each", "mana", "from", "a", "cave", "spent", "to", "cast", "it",
    ];
    let (cave_value, used) =
        parse_for_each_count_value_words(&cave_words).expect("Cave-spent count should parse");
    assert_eq!(used, cave_words.len());
    let Value::ManaFromSourceSpentToCastThisSpell {
        source_filter,
        include_source_noun,
        reference,
    } = cave_value
    else {
        panic!("expected a typed mana-source count");
    };
    assert!(!include_source_noun);
    assert_eq!(reference, ironsmith_core::ManaSpentCastReferenceSurface::It);
    assert_eq!(source_filter.subtypes, [crate::types::Subtype::Cave]);

    let artifact_words = [
        "for", "each", "mana", "from", "an", "artifact", "source", "that", "was", "spent", "to",
        "cast", "this", "spell",
    ];
    let (artifact_value, used) = parse_for_each_count_value_words(&artifact_words)
        .expect("artifact-source-spent count should parse");
    assert_eq!(used, artifact_words.len());
    let Value::ManaFromSourceSpentToCastThisSpell {
        source_filter,
        include_source_noun,
        reference,
    } = artifact_value
    else {
        panic!("expected a typed mana-source count");
    };
    assert!(include_source_noun);
    assert_eq!(
        reference,
        ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell
    );
    assert_eq!(source_filter.card_types, [crate::types::CardType::Artifact]);
}

#[test]
fn commander_cast_history_precedes_generic_commander_object_counts() {
    let words = [
        "for",
        "each",
        "time",
        "you",
        "ve",
        "cast",
        "your",
        "commander",
        "from",
        "the",
        "command",
        "zone",
        "this",
        "game",
    ];
    let (value, used) = parse_for_each_count_value_words(&words)
        .expect("commander cast history count should parse");
    assert_eq!(used, words.len());
    assert_eq!(value, Value::CommanderCastCount(PlayerFilter::You));

    let current_set = ["for", "each", "commander", "you", "control"];
    let (value, used) = parse_for_each_count_value_words(&current_set)
        .expect("ordinary commander object count should remain supported");
    assert_eq!(used, current_set.len());
    assert!(matches!(value.unhinted(), Value::Count(filter) if filter.is_commander));
}

#[test]
fn parses_for_each_creature_in_your_party_as_party_size() {
    assert_eq!(
        parse_for_each_count_value_words(&["for", "each", "creature", "in", "your", "party"]),
        Some((Value::PartySize(PlayerFilter::You), 6))
    );
}

#[test]
fn leading_other_remains_local_to_the_first_scoped_count_arm() {
    let words = [
        "for",
        "each",
        "other",
        "assassin",
        "you",
        "control",
        "and",
        "each",
        "assassin",
        "card",
        "in",
        "your",
        "graveyard",
    ];
    let (value, used) =
        parse_for_each_count_value_words(&words).expect("compound count should parse");
    assert_eq!(used, words.len());
    let Value::Count(filter) = value else {
        panic!("expected object count");
    };
    assert_eq!(filter.any_of.len(), 2);
    assert!(filter.any_of[0].other);
    assert!(!filter.any_of[1].other);
}

#[test]
fn repeated_each_keeps_suspended_cards_and_permanents_as_distinct_count_arms() {
    let words = [
        "for",
        "each",
        "suspended",
        "card",
        "you",
        "own",
        "and",
        "each",
        "other",
        "permanent",
        "you",
        "control",
        "with",
        "a",
        "time",
        "counter",
        "on",
        "it",
    ];
    let (value, used) =
        parse_for_each_count_value_words(&words).expect("compound suspended count should parse");
    assert_eq!(used, words.len());
    let Value::Count(filter) = value else {
        panic!("expected a typed object count, got {value:#?}");
    };
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(
        filter.any_of.iter().any(|arm| {
            arm.zone == Some(crate::zone::Zone::Exile)
                && arm.owner == Some(PlayerFilter::You)
                && arm.alternative_cast == Some(crate::filter::AlternativeCastKind::Suspend)
        }),
        "{filter:#?}"
    );
    assert!(
        filter.any_of.iter().any(|arm| {
            arm.zone == Some(crate::zone::Zone::Battlefield)
                && arm.controller == Some(PlayerFilter::You)
                && arm.other
                && arm.with_counter
                    == Some(crate::filter::CounterConstraint::Typed(
                        crate::object::CounterType::Time,
                    ))
        }),
        "{filter:#?}"
    );
}

#[test]
fn parses_for_each_colored_mana_symbol_across_a_filtered_scope() {
    let words = [
        "for",
        "each",
        "white",
        "mana",
        "symbol",
        "in",
        "the",
        "mana",
        "costs",
        "of",
        "permanents",
        "you",
        "control",
    ];
    let (value, used) =
        parse_for_each_count_value_words(&words).expect("mana-symbol token count should parse");
    assert_eq!(used, words.len());
    let Value::ManaSymbolsInManaCostOf { spec, color } = value else {
        panic!("expected structured mana-symbol value");
    };
    assert_eq!(color, crate::color::Color::White);
    let ChooseSpec::All(filter) = spec.unhinted() else {
        panic!("expected aggregate object scope");
    };
    assert_eq!(filter.zone, Some(crate::zone::Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
}

#[test]
fn parses_for_each_opponent_you_have_as_a_player_count() {
    for noun in ["opponent", "opponents"] {
        let words = ["for", "each", noun, "you", "have"];
        let (value, used) =
            parse_for_each_count_value_words(&words).expect("opponent count should parse");
        assert_eq!(used, words.len());
        assert_eq!(value, Value::CountPlayers(PlayerFilter::Opponent));
    }
}

#[test]
fn preserves_explicit_card_noun_in_this_way_counts() {
    let (value, used) = parse_for_each_count_value_words(&[
        "for",
        "each",
        "nonland",
        "card",
        "discarded",
        "this",
        "way",
    ])
    .expect("nonland cards discarded this way count");
    assert_eq!(used, 7);
    assert!(value.has_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay));
    let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
        panic!("expected typed discarded-object count");
    };
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Discarded)
    );
    assert!(
        query
            .filter
            .as_ref()
            .is_some_and(ObjectFilter::has_explicit_card_noun)
    );
}

#[test]
fn returned_to_your_hand_count_filters_the_exact_result_by_owner() {
    let words = [
        "for", "each", "card", "returned", "to", "your", "hand", "this", "way",
    ];
    let (value, used) =
        parse_for_each_count_value_words(&words).expect("returned-to-your-hand count");
    assert_eq!(used, words.len());
    let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
        panic!("expected a filtered prior-effect metric, got {value:?}");
    };
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Returned)
    );
    assert_eq!(
        query
            .filter
            .as_ref()
            .and_then(|filter| filter.owner.clone()),
        Some(PlayerFilter::You)
    );
}

#[test]
fn types_plain_tapped_counts_but_leaves_player_partitioned_counts_unbound() {
    let (value, used) =
        parse_for_each_count_value_words(&["for", "each", "creature", "tapped", "this", "way"])
            .expect("creatures tapped this way count");
    assert_eq!(used, 6);
    let Value::PendingPriorEffectMetric(query) = value else {
        panic!("expected typed prior-effect metric");
    };
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Tapped)
    );
    assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
    assert_eq!(
        query.filter.expect("creature filter").card_types,
        [crate::types::CardType::Creature],
    );

    let (partitioned, used) = parse_for_each_count_value_words(&[
        "for",
        "each",
        "creature",
        "they",
        "controlled",
        "that",
        "was",
        "tapped",
        "this",
        "way",
    ])
    .expect("player-partitioned tapped count retains legacy form");
    assert_eq!(used, 10);
    let Value::PendingPriorEffectMetric(query) = partitioned else {
        panic!("expected typed player-partitioned prior-effect metric");
    };
    assert_eq!(query.player, None);
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Tapped)
    );
}

#[test]
fn counts_typed_counters_among_a_filtered_object_set() {
    let words = [
        "for",
        "each",
        "+1/+1",
        "counter",
        "among",
        "other",
        "creatures",
        "you",
        "control",
    ];
    let (value, used) = parse_for_each_count_value_words(&words).expect("typed counter aggregate");
    assert_eq!(used, words.len());
    assert!(value.has_surface_hint(ValueSurfaceHint::CountersAmong));
    let Value::CountersOn(spec, Some(crate::object::CounterType::PlusOnePlusOne)) =
        value.unhinted()
    else {
        panic!("expected a typed +1/+1 counter aggregate, got {value:#?}");
    };
    let ChooseSpec::All(filter) = spec.unhinted() else {
        panic!("expected an aggregate object filter, got {spec:#?}");
    };
    assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.other);
}

#[test]
fn coordinated_countered_stack_objects_keep_spells_and_abilities() {
    let words = [
        "for",
        "each",
        "spell",
        "and",
        "ability",
        "countered",
        "this",
        "way",
    ];
    let (value, used) = parse_for_each_count_value_words(&words).expect("coordinated stack count");
    assert_eq!(used, words.len());
    let Value::PendingPriorEffectMetric(query) = value else {
        panic!("expected typed prior-effect count");
    };
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Countered)
    );
    let filter = query.filter.expect("stack filter");
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::SpellOrAbility)
    );
    assert!(filter.has_conjunctive_set_surface());
}
