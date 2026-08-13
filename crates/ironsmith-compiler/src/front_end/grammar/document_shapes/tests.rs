use super::*;

#[test]
fn parses_document_routing_surfaces() {
    let header = lex_line("As this enters, choose red or blue", 0).expect("lex");
    assert!(parse_named_option_choice_header(&header).is_some());

    let replacement = lex_line("If a creature would die, exile it instead", 0).expect("lex");
    assert!(parse_conditional_replacement_surface(&replacement).is_some());

    let trigger = lex_line("Whenever you next cast a spell this turn", 0).expect("lex");
    assert!(parse_next_cast_trigger_surface(&trigger).is_some());

    let loyalty = lex_line("+1", 0).expect("lex");
    assert!(matches!(
        parse_activation_cost_head(&loyalty),
        Some(ActivationCostHeadSurface::Signed)
    ));
}

#[test]
fn parses_source_alias_and_sentence_split_surfaces() {
    assert!(parse_source_alias_effect_verb_surface("Mill", "three cards").is_some());
    assert_eq!(
        parse_named_source_prefix("brago attacks", "brago")
            .expect("prefix")
            .tail,
        "attacks"
    );
    assert_eq!(
        parse_first_comma("when this enters, draw a card")
            .expect("split")
            .body,
        "draw a card"
    );

    let delayed = lex_line("When that dies this turn, draw a card", 0).expect("lex");
    assert!(parse_delayed_prior_object_dies_surface(&delayed).is_some());

    assert_eq!(
        parse_named_source_enters_surface("Brago enters tapped")
            .expect("enters tail")
            .tail,
        "tapped"
    );
    assert!(parse_named_source_enters_surface("Brago enters, draw a card").is_none());

    let reflexive = lex_line(
        "Whenever you discard one or more cards this way, draw a card.",
        0,
    )
    .expect("lex");
    assert!(parse_when_one_or_more_this_way_followup_surface(&reflexive).is_some());
}
