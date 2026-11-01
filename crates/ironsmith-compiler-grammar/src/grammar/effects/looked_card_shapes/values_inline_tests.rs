use super::*;
use crate::lexer::lex_line;

#[test]
fn revealed_group_count_preserves_sacrificed_power() {
    let tokens = lex_line(
        "Reveal a number of cards from the top of your library equal to the sacrificed creature's power",
        0,
    ).unwrap();
    let shape = parse_top_cards_view_shape(&tokens).unwrap();
    let value_tokens = lex_line("the sacrificed creature's power", 0).unwrap();
    let (expected, used) = parse_value_prefix_lexed(&value_tokens).unwrap();
    assert_eq!(used, value_tokens.len());
    assert!(shape.revealed);
    assert_eq!(shape.count, expected);
    let incomplete = lex_line(
        "Reveal a number of cards from the top of your library equal to the sacrificed creature's power plus nonsense",
        0,
    ).unwrap();
    assert!(parse_top_cards_view_shape(&incomplete).is_none());
}

#[test]
fn parses_typed_top_card_view_counts() {
    let tokens = lex_line("Look at the top three cards of your library", 0).unwrap();
    let shape = parse_top_cards_view_shape(&tokens).unwrap();
    assert!(!shape.revealed);
    assert_eq!(shape.count, Value::Fixed(3));

    let tokens = lex_line(
        "Reveal the top X cards of your library, where X is the number of cards milled this way",
        0,
    )
    .unwrap();
    let shape = parse_top_cards_view_shape(&tokens).unwrap();
    assert!(shape.revealed);
    assert!(shape.count.has_surface_hint(ValueSurfaceHint::WhereXIs));

    let tokens = lex_line(
            "Reveal the top X cards of your library, where X is the number of lands sacrificed this way",
            0,
        )
        .unwrap();
    let shape = parse_top_cards_view_shape(&tokens).unwrap();
    let Value::PendingPriorEffectMetric(query) = shape.count.unhinted() else {
        panic!("expected typed prior-effect count, got {:?}", shape.count);
    };
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Sacrificed)
    );
    assert_eq!(
        query
            .filter
            .as_ref()
            .map(|filter| filter.card_types.as_slice()),
        Some(&[crate::types::CardType::Land][..])
    );

    let tokens = lex_line("Look at that many cards from the top of your library", 0).unwrap();
    let shape = parse_top_cards_view_shape(&tokens).unwrap();
    assert!(!shape.revealed);
    assert_eq!(shape.count, Value::EventValue(EventValueSpec::Amount));

    let tokens = lex_line("Reveal the top X plus one cards of your library", 0).unwrap();
    let shape = parse_top_cards_view_shape(&tokens).unwrap();
    assert!(shape.revealed);
    assert_eq!(
        shape.count,
        Value::Add(Box::new(Value::X), Box::new(Value::Fixed(1)))
    );
}

#[test]
fn looked_card_where_x_uses_typed_fixed_plus_party_value() {
    let tokens = lex_line(
            "Look at the top X cards of your library, where X is three plus the number of creatures in your party",
            0,
        )
        .unwrap();
    let shape = parse_top_cards_view_shape(&tokens).unwrap();

    assert!(shape.count.has_surface_hint(ValueSurfaceHint::WhereXIs));
    assert_eq!(
        shape.count.unhinted(),
        &Value::Add(
            Box::new(Value::Fixed(3)),
            Box::new(Value::PartySize(crate::target::PlayerFilter::You)),
        )
    );
}

#[test]
fn looked_card_where_x_preserves_battlefield_and_graveyard_sum_terms() {
    let tokens = lex_line(
            "Look at the top X cards of your library, where X is the number of Caves you control plus the number of Cave cards in your graveyard",
            0,
        )
        .unwrap();
    let shape = parse_top_cards_view_shape(&tokens).unwrap();
    let Value::Add(controlled, graveyard) = shape.count.unhinted() else {
        panic!(
            "expected two typed Cave-count terms, got {:#?}",
            shape.count
        );
    };
    let Value::Count(controlled) = controlled.as_ref() else {
        panic!("expected controlled-Cave count, got {controlled:#?}");
    };
    let Value::Count(graveyard) = graveyard.as_ref() else {
        panic!("expected graveyard-Cave count, got {graveyard:#?}");
    };

    assert_eq!(controlled.zone, Some(crate::Zone::Battlefield));
    assert_eq!(
        controlled.controller,
        Some(crate::target::PlayerFilter::You)
    );
    assert!(controlled.subtypes.contains(&crate::Subtype::Cave));
    assert_eq!(graveyard.zone, Some(crate::Zone::Graveyard));
    assert_eq!(graveyard.owner, Some(crate::target::PlayerFilter::You));
    assert!(graveyard.subtypes.contains(&crate::Subtype::Cave));
}

#[test]
fn singular_top_card_has_exact_implicit_one_with_complete_library_tail() {
    for verb in ["Look at", "Reveal"] {
        let singular = lex_line(&format!("{verb} the top card of your library"), 0).unwrap();
        let explicit = lex_line(&format!("{verb} the top one card of your library"), 0).unwrap();
        let shape = parse_top_cards_view_shape(&singular).unwrap();
        assert_eq!(shape, parse_top_cards_view_shape(&explicit).unwrap());
        assert_eq!(shape.count, Value::Fixed(1));
        assert_eq!(shape.revealed, verb == "Reveal");
        for tail in ["cards of your library", "card of your library plus nonsense", "card of your library {2}"] {
            let tokens = lex_line(&format!("{verb} the top {tail}"), 0).unwrap();
            assert!(parse_top_cards_view_shape(&tokens).is_none(), "unread count or tail accepted: {tail}");
        }
    }
}
