use super::*;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn value_comparison_accepts_authored_is_exactly_surface() {
    let tokens = lex("is exactly 20");
    let (operator, remaining) = parse_value_comparison_tokens(&tokens)
        .expect("is exactly should be an equality comparison");
    assert_eq!(operator, ValueComparisonOperator::Equal);
    assert_eq!(parser_token_word_refs(remaining), vec!["20"]);
}

#[test]
fn parses_players_who_control_more_filter_shape() {
    let parsed = parse_players_who_control_more_than_you_value_lexed(&lex(
        "the number of players who control more lands than you",
    ))
    .unwrap();
    let Value::PlayersWhoControlMoreThanYou { players, filter } = parsed else {
        panic!("expected players-who-control-more value");
    };
    assert_eq!(players, PlayerFilter::Any);
    assert_eq!(filter.card_types, vec![CardType::Land]);
}

#[test]
fn parses_opponents_who_control_more_and_at_least_shapes() {
    let parsed = parse_players_who_control_more_than_you_value_lexed(&lex(
        "the number of opponents who control more creatures than you",
    ))
    .unwrap();
    let Value::PlayersWhoControlMoreThanYou { players, filter } = parsed else {
        panic!("expected opponents-who-control-more value");
    };
    assert_eq!(players, PlayerFilter::Opponent);
    assert_eq!(filter.card_types, vec![CardType::Creature]);

    let parsed = parse_players_who_control_more_than_you_value_lexed(&lex(
        "the number of opponents who control at least two more lands than you",
    ))
    .unwrap();
    let Value::PlayersWhoControlAtLeastMoreThanYou {
        players,
        filter,
        minimum_difference,
    } = parsed
    else {
        panic!("expected opponents-who-control-at-least-more value");
    };
    assert_eq!(players, PlayerFilter::Opponent);
    assert_eq!(filter.card_types, vec![CardType::Land]);
    assert_eq!(minimum_difference, 2);
}

#[test]
fn max_cards_in_hand_accepts_sentence_punctuation() {
    assert_eq!(
        parse_max_cards_in_hand_value_lexed(&lex(
            "cards in the hand of the opponent with the most cards in hand."
        )),
        Some(Value::MaxCardsInHand(PlayerFilter::Opponent))
    );
}

#[test]
fn type_line_keeps_time_lord_distinct_from_doctor() {
    let parsed = parse_type_line_rewrite("Legendary Creature — Time Lord Doctor")
        .expect("Time Lord type line should parse");
    assert_eq!(parsed.subtypes, vec![Subtype::TimeLord, Subtype::Doctor]);
}

#[test]
fn parses_stat_and_mana_value_segment_shapes() {
    let stat_tokens = lex("that creature power");
    assert_eq!(
        parse_value_stat_segment_shape(LexedClause::new(&stat_tokens)),
        Some(ValueStatSegmentShape {
            subject: ValueStatSubjectShape::Tagged,
            axis: ValueStatAxisShape::Power,
        })
    );

    let mana_tokens = lex("that spell mana value");
    assert_eq!(
        parse_value_mana_value_segment_shape(LexedClause::new(&mana_tokens)),
        Some(ValueManaValueSegmentShape {
            subject: ValueManaValueSubjectShape::Tagged,
        })
    );

    let possessive_tokens = lex("its mana value");
    let possessive_shape =
        parse_value_mana_value_segment_shape(LexedClause::new(&possessive_tokens))
            .expect("possessive mana value should parse");
    assert_eq!(
        possessive_shape,
        ValueManaValueSegmentShape {
            subject: ValueManaValueSubjectShape::TaggedPossessivePronoun,
        }
    );
    let Value::ManaValueOf(spec) = value_from_mana_value_segment_shape(possessive_shape) else {
        panic!("possessive shape should lower to a mana-value reference");
    };
    assert_eq!(
        spec.source_reference_surface(),
        Some(&crate::target::SourceReferenceSurface::ThisPermanentType(
            "it".to_string()
        ))
    );
}

#[test]
fn lady_loki_parses_absolute_mana_value_difference() {
    let value = parse_add_mana_equal_amount_value_lexed(&lex(
            "equal to the difference between that spell's mana value and that nonland card's mana value",
        ))
        .expect("Lady Loki mana-value difference should parse");
    let debug = format!("{value:#?}");

    assert!(
        value.has_surface_hint(ValueSurfaceHint::Difference),
        "{debug}"
    );
    assert!(debug.contains("Min"), "{debug}");
    assert!(debug.contains("triggering"), "{debug}");
    assert!(debug.contains("__it__"), "{debug}");
}

#[test]
fn parses_add_mana_equal_amount_tail_with_typed_shape() {
    let parsed =
        parse_add_mana_equal_amount_value_lexed(&lex("add mana equal to that creature power"))
            .unwrap();
    assert_eq!(
        parsed,
        Value::PowerOf(Box::new(ChooseSpec::Tagged(
            crate::tag::CompilerReferenceTag::It.key()
        )))
    );
}
