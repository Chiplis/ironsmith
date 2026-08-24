use super::*;
use crate::lexer::lex_line;

#[test]
fn prior_or_trigger_amount_drives_that_many_search_counts() {
    for (surface, expected_used, up_to) in [("that many", 2, false), ("up to that many", 4, true)] {
        let tokens = lex_line(surface, 0).expect("count surface should lex");
        let parsed = parse_search_library_count_prefix_lexed(&tokens);

        assert_eq!(parsed.count_used, expected_used);
        assert_eq!(parsed.count.up_to_x, up_to);
        assert_eq!(
            parsed.count_value,
            Some(Value::EventValue(crate::effect::EventValueSpec::Amount))
        );
    }
}

#[test]
fn explicit_exactly_search_count_retains_its_surface() {
    let tokens = lex_line("exactly four legendary creature cards", 0)
        .expect("exact search count should lex");
    let parsed = parse_search_library_count_prefix_lexed(&tokens);

    assert_eq!(
        parsed.count,
        ChoiceCount::exactly(4).with_explicit_exactly()
    );
    assert_eq!(parsed.count_used, 2);
    assert_eq!(parsed.search_mode, SearchSelectionMode::Exact);
}

#[test]
fn refreshed_instead_trailing_random_discard_after_shuffle_has_a_strict_search_boundary() {
    let tokens = lex_line(
            "Search your library for up to three cards, put them into your hand, shuffle, then discard three cards at random.",
            0,
        )
        .expect("search/discard fixture should lex");
    let markers = scan_search_library_clause_markers_lexed(&tokens)
        .expect("search/discard fixture should route");
    let discard =
        find_search_library_discard_after_shuffle_followup_lexed(&tokens, markers.put_idx)
            .expect("trailing random discard should be isolated");
    assert_eq!(
        render_token_slice(discard),
        "discard three cards at random."
    );

    let near_miss = lex_line(
            "Search your library for a card, put it into your hand, shuffle, then draw a card, then discard a card.",
            0,
        )
        .expect("intervening-effect near miss should lex");
    let markers = scan_search_library_clause_markers_lexed(&near_miss)
        .expect("near miss should still route as a search");
    assert!(
        find_search_library_discard_after_shuffle_followup_lexed(&near_miss, markers.put_idx,)
            .is_none(),
        "an intervening effect must not be absorbed into the search tail"
    );
}

#[test]
fn that_players_library_remains_a_contextual_player_reference() {
    let tokens =
        lex_line("search that player's library for a card", 0).expect("search surface should lex");
    let routing = derive_search_library_subject_routing_lexed(&tokens, PlayerAst::Implicit)
        .expect("that-player search should route");

    assert_eq!(routing.player, PlayerAst::That);
    assert_eq!(
        routing.forced_library_owner,
        Some(PlayerFilter::IteratedPlayer)
    );
    assert_eq!(routing.search_player_target, None);
}

#[test]
fn library_and_or_graveyard_search_keeps_both_typed_origins() {
    let tokens = lex_line("search your library and/or graveyard for a card", 0)
        .expect("multi-zone search surface should lex");
    let routing = derive_search_library_subject_routing_lexed(&tokens, PlayerAst::Implicit)
        .expect("multi-zone search should route");

    assert_eq!(routing.forced_library_owner, Some(PlayerFilter::You));
    assert_eq!(
        routing.search_zones_override,
        Some(vec![Zone::Library, Zone::Graveyard])
    );
}

#[test]
fn graveyard_and_or_library_search_keeps_authored_origin_order() {
    let tokens = lex_line("search your graveyard and/or library for a card", 0)
        .expect("multi-zone search surface should lex");
    let routing = derive_search_library_subject_routing_lexed(&tokens, PlayerAst::Implicit)
        .expect("multi-zone search should route");

    assert_eq!(
        routing.search_zones_override,
        Some(vec![Zone::Graveyard, Zone::Library])
    );
}

#[test]
fn search_filter_keeps_shared_characteristic_relation() {
    let tokens = lex_line(
        "a card that shares a color with a legendary creature you control",
        0,
    )
    .expect("search selector should lex");
    let filter = parse_search_library_object_filter_lexed(&tokens, "relation probe")
        .expect("search selector should parse");

    assert_eq!(filter.characteristic_relations.len(), 1);
    assert_eq!(
        filter.description(),
        "card that shares a color with a legendary creature you control"
    );
}

#[test]
fn search_result_reference_keeps_authored_singular_surface() {
    for (line, expected) in [
        (
            "search your library for a card, put it into your hand",
            crate::effect::SearchResultReferenceSurface::It,
        ),
        (
            "search your library for a card, put that card into your hand",
            crate::effect::SearchResultReferenceSurface::ThatCard,
        ),
        (
            "search your library for a card, put the card into your hand",
            crate::effect::SearchResultReferenceSurface::TheCard,
        ),
    ] {
        let tokens = lex_line(line, 0).expect("search surface should lex");
        let markers =
            scan_search_library_clause_markers_lexed(&tokens).expect("search clauses should route");
        let routing = derive_search_library_effect_routing_lexed(&tokens, &tokens, markers, false);

        assert_eq!(routing.result_reference_surface, expected, "{line}");
    }
}

#[test]
fn search_result_reference_keeps_authored_plural_surface() {
    for (line, expected, expected_reveal, expected_order) in [
        (
            "search your library for up to three creature cards, reveal them, then shuffle and put those cards on top in any order",
            crate::effect::SearchResultReferenceSurface::ThoseCards,
            Some(crate::effect::SearchResultReferenceSurface::Them),
            true,
        ),
        (
            "search your library for any number of creature cards, reveal those cards, then shuffle and put them on top",
            crate::effect::SearchResultReferenceSurface::Them,
            Some(crate::effect::SearchResultReferenceSurface::ThoseCards),
            false,
        ),
    ] {
        let tokens = lex_line(line, 0).expect("plural search surface should lex");
        let markers = scan_search_library_clause_markers_lexed(&tokens)
            .expect("plural search clauses should route");
        let routing = derive_search_library_effect_routing_lexed(&tokens, &tokens, markers, false);

        assert_eq!(routing.result_reference_surface, expected, "{line}");
        assert_eq!(routing.reveal_reference_surface, expected_reveal, "{line}");
        assert_eq!(
            routing.search_top_in_any_order_surface, expected_order,
            "{line}"
        );
    }
}

#[test]
fn searched_battlefield_card_keeps_inline_entry_counter() {
    let line = "search their library for a basic land card, put it onto the battlefield tapped with a stun counter on it, then shuffle";
    let tokens = lex_line(line, 0).expect("countered search surface should lex");
    let markers = scan_search_library_clause_markers_lexed(&tokens)
        .expect("countered search clauses should route");
    let routing = derive_search_library_effect_routing_lexed(&tokens, &tokens, markers, false);

    assert_eq!(routing.destination, Zone::Battlefield);
    assert!(routing.has_tapped_modifier);
    let [counter] = routing.battlefield_entry_counters.as_slice() else {
        panic!("expected one typed battlefield-entry counter");
    };
    assert_eq!(counter.counter_type, crate::object::CounterType::Stun);
    assert_eq!(counter.amount, Value::Fixed(1));
    assert_eq!(counter.surface, BattlefieldEntryCounterSurface::Inline);
}

#[test]
fn searched_battlefield_card_keeps_dynamic_additional_entry_counters_once() {
    let line = "search your library and/or graveyard for an artifact creature card with mana value X or less and put it onto the battlefield with X additional +1/+1 counters on it";
    let tokens = lex_line(line, 0).expect("dynamic countered search surface should lex");
    let markers = scan_search_library_clause_markers_lexed(&tokens)
        .expect("dynamic countered search clauses should route");
    let routing = derive_search_library_effect_routing_lexed(&tokens, &tokens, markers, false);

    let [counter] = routing.battlefield_entry_counters.as_slice() else {
        panic!("expected exactly one typed battlefield-entry counter");
    };
    assert_eq!(
        counter.counter_type,
        crate::object::CounterType::PlusOnePlusOne
    );
    assert_eq!(counter.amount.unhinted(), &Value::X);
    assert!(
        counter
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
    );
    assert_eq!(counter.surface, BattlefieldEntryCounterSurface::Inline);
}

#[test]
fn mana_value_or_less_does_not_widen_adjacent_card_types() {
    let tokens = lex_line("an artifact creature card with mana value X or less", 0)
        .expect("comparison-bearing search filter should lex");
    let filter = parse_search_library_object_filter_lexed(&tokens, "test search")
        .expect("comparison-bearing search filter should parse");

    assert!(filter.card_types.is_empty(), "{filter:#?}");
    assert_eq!(
        filter.all_card_types,
        [
            crate::types::CardType::Artifact,
            crate::types::CardType::Creature
        ],
        "adjacent card types are an intersection even when the later comparison uses `or`"
    );
    assert_eq!(
        filter.mana_value,
        Some(crate::filter::Comparison::LessThanOrEqualExpr(Box::new(
            Value::X
        )))
    );
}

#[test]
fn search_filter_keeps_devotion_as_the_dynamic_mana_value_limit() {
    let tokens = lex_line(
        "a card with mana value less than or equal to your devotion to black",
        0,
    )
    .expect("devotion-bounded search filter should lex");
    let filter = parse_search_library_object_filter_lexed(&tokens, "devotion search")
        .expect("devotion-bounded search filter should parse");

    assert!(matches!(
        filter.mana_value.as_ref(),
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value))
            if value.unhinted() == &(Value::Devotion {
                player: PlayerFilter::You,
                color: crate::color::Color::Black,
            })
    ));
}
