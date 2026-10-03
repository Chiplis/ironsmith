use super::super::super::super::lexer::{lex_line, render_token_slice};
use super::*;

#[test]
fn parses_once_each_turn_graveyard_variants() {
    let tokens = lex_line(
        "Once during each of your turns, you may cast a creature spell from your graveyard.",
        0,
    )
    .unwrap();
    let parsed = parse_once_each_turn_graveyard_cast_tokens(&tokens).unwrap();
    assert_eq!(
        render_token_slice(parsed.subject_tokens),
        "a creature spell"
    );
    assert_eq!(parsed.cost_tokens, None);
    assert!(!parsed.exiles_after_resolution);

    let tokens = lex_line(
            "Once during each of your turns, you may cast a spell from your graveyard by sacrificing a creature in addition to paying its other costs. If a spell cast this way would be put into your graveyard, exile it instead.",
            0,
        )
        .unwrap();
    let parsed = parse_once_each_turn_graveyard_cast_tokens(&tokens).unwrap();
    assert_eq!(
        render_token_slice(parsed.cost_tokens.unwrap()),
        "sacrificing a creature"
    );
    assert!(parsed.exiles_after_resolution);
}

#[test]
fn parses_typed_graveyard_additional_costs() {
    let tokens = lex_line("sacrificing an artifact", 0).unwrap();
    let Some(GraveyardAdditionalCostFact::Sacrifice { filter_tokens }) =
        parse_graveyard_additional_cost_tokens(&tokens)
    else {
        panic!("expected sacrifice cost");
    };
    assert_eq!(render_token_slice(filter_tokens), "an artifact");

    let tokens = lex_line(
        "exiling four instant and/or sorcery cards from your graveyard",
        0,
    )
    .unwrap();
    let Some(GraveyardAdditionalCostFact::ExileCards {
        count,
        card_types,
        other,
    }) = parse_graveyard_additional_cost_tokens(&tokens)
    else {
        panic!("expected exile cost");
    };
    assert_eq!(count, 4);
    assert!(!other);
    assert_eq!(card_types, vec![CardType::Instant, CardType::Sorcery]);
}

#[test]
fn parses_source_zone_cost_and_die_roll_facts() {
    let tokens = lex_line(
            "this card from your graveyard by exiling four instant and/or sorcery cards from your graveyard in addition to paying its other costs",
            0,
        )
        .unwrap();
    let parsed = parse_source_graveyard_additional_cost_tokens(&tokens).unwrap();
    assert_eq!(parsed.source_kind, SourceKindFact::Card);
    assert_eq!(
        render_token_slice(parsed.cost_tokens),
        "exiling four instant and/or sorcery cards from your graveyard"
    );

    let tokens = lex_line("this spell from exile", 0).unwrap();
    assert_eq!(
        parse_source_cast_permission_tokens(&tokens),
        Some(SourceCastPermissionFact {
            source_kind: SourceKindFact::Spell,
            zone: Zone::Exile,
        })
    );

    let tokens = lex_line(
            "this card from your graveyard as long as you've rolled a 6 this turn. If you cast it this way and it would be put into your graveyard, exile it instead.",
            0,
        )
        .unwrap();
    assert_eq!(
        parse_source_graveyard_die_roll_cast_tokens(&tokens),
        Some(SourceGraveyardDieRollCastFact { result: 6 })
    );
}

#[test]
fn parses_source_graveyard_dynamic_surcharge_without_claiming_plain_permissions() {
    let tokens = lex_line(
            "You may cast this creature from your graveyard if you pay {1} more to cast it for each other creature card in your graveyard.",
            0,
        )
        .unwrap();
    let parsed = parse_source_graveyard_dynamic_surcharge_tokens(&tokens).unwrap();
    assert_eq!(render_token_slice(parsed.source_tokens), "this creature");
    assert_eq!(render_token_slice(parsed.cost_tokens), "{1}");
    assert_eq!(
        render_token_slice(parsed.repetition_tokens),
        "for each other creature card in your graveyard"
    );

    let plain = lex_line("You may cast this creature from your graveyard.", 0).unwrap();
    assert!(parse_source_graveyard_dynamic_surcharge_tokens(&plain).is_none());
}

#[test]
fn parses_top_library_shared_type_fact() {
    let tokens = lex_line(
            "Once each turn, you may cast a spell from the top of your library if it shares a card type with a card exiled with this creature.",
            0,
        )
        .unwrap();
    let parsed = parse_once_each_turn_top_library_shared_type_tokens(&tokens).unwrap();
    assert_eq!(render_token_slice(parsed.subject_tokens), "a spell");
    assert_eq!(
        render_token_slice(parsed.source_reference_tokens),
        "a card exiled with this creature"
    );
}

#[test]
fn graveyard_additional_costs_parse_counts_life_and_other_identity() {
    for (text, expected) in [("discarding a card", 1), ("discarding two cards", 2)] {
        assert_eq!(
            parse_graveyard_additional_cost_tokens(&lex_line(text, 0).unwrap()),
            Some(GraveyardAdditionalCostFact::Discard { count: expected })
        );
    }
    let tokens = lex_line("paying 3 life and discarding a card", 0).unwrap();
    let Some(GraveyardAdditionalCostFact::PayLife {
        amount,
        remaining_tokens: Some(rest),
    }) = parse_graveyard_additional_cost_tokens(&tokens)
    else {
        panic!("life plus discard cost")
    };
    assert_eq!(amount, 3);
    assert_eq!(render_token_slice(rest), "discarding a card");
    for (text, count, types) in [
        (
            "exiling another creature card from your graveyard",
            1,
            vec![CardType::Creature],
        ),
        ("exiling three other cards from your graveyard", 3, vec![]),
    ] {
        assert_eq!(
            parse_graveyard_additional_cost_tokens(&lex_line(text, 0).unwrap()),
            Some(GraveyardAdditionalCostFact::ExileCards {
                count,
                card_types: types,
                other: true
            })
        );
    }
}

#[test]
fn graveyard_additional_costs_do_not_drop_unsupported_qualifiers() {
    for text in [
        "discarding two creature cards",
        "discarding a card at random",
        "discarding zero cards",
        "exiling another creature card from an opponent's graveyard",
        "exiling three other cards from your graveyard at random",
        "paying 3 life or discarding a card",
    ] {
        assert!(
            parse_graveyard_additional_cost_tokens(&lex_line(text, 0).unwrap()).is_none(),
            "{text}"
        );
    }
}
