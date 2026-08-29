use super::super::super::lexer::lex_line;
use super::*;

#[test]
fn choice_object_shape_preserves_reference_and_dynamic_count_facts() {
    let tokens = lex_line(
        "You choose cards from it for each card discarded this way.",
        0,
    )
    .unwrap();
    let tokens = &tokens[..tokens.len() - 1];

    let ChoiceObjectClauseKind::Object(parsed) =
        parse_choice_object_clause_tokens(tokens).unwrap().unwrap()
    else {
        panic!("expected object choice");
    };
    assert_eq!(parsed.actor, ChoiceClauseActor::You);
    assert_eq!(parsed.filter_words, ["cards"]);
    assert_eq!(parsed.count, ChoiceCount::dynamic_x());
    assert_eq!(
        parsed.count_source,
        Some(ChoiceObjectCountSource::CardsDiscardedThisWay)
    );
    assert!(parsed.references.references_it);
    assert!(parsed.references.references_container_it);
    assert!(parsed.references.explicit_container_reference);
}

#[test]
fn singular_opponent_choice_keeps_the_authored_chooser() {
    let tokens = lex_line(
        "An opponent chooses a permanent you control other than this creature.",
        0,
    )
    .unwrap();
    let ChoiceObjectClauseKind::Object(parsed) =
        parse_choice_object_clause_tokens(&tokens[..tokens.len() - 1])
            .unwrap()
            .unwrap()
    else {
        panic!("expected object choice");
    };
    assert_eq!(parsed.actor, ChoiceClauseActor::Opponent);
    assert_eq!(
        parsed.filter_words,
        [
            "permanent",
            "you",
            "control",
            "other",
            "than",
            "this",
            "creature"
        ]
    );
}

#[test]
fn choice_object_shape_preserves_up_to_prior_amount_count() {
    let tokens = lex_line("Choose up to that many target creatures you control.", 0).unwrap();
    let tokens = &tokens[..tokens.len() - 1];

    let ChoiceObjectClauseKind::Object(parsed) =
        parse_choice_object_clause_tokens(tokens).unwrap().unwrap()
    else {
        panic!("expected object choice");
    };
    assert!(parsed.count.is_up_to_dynamic_x());
    assert_eq!(parsed.count_source, Some(ChoiceObjectCountSource::ThatMany));
    assert_eq!(
        parsed.filter_words,
        ["target", "creatures", "you", "control"]
    );
}

#[test]
fn choice_object_shape_preserves_for_each_count_basis() {
    let tokens = lex_line("Choose a permanent for each card in their graveyard.", 0).unwrap();
    let tokens = &tokens[..tokens.len() - 1];

    let ChoiceObjectClauseKind::Object(parsed) =
        parse_choice_object_clause_tokens(tokens).unwrap().unwrap()
    else {
        panic!("expected object choice");
    };
    assert_eq!(parsed.filter_words, ["permanent"]);
    assert_eq!(parsed.count, ChoiceCount::dynamic_x());
    assert_eq!(
        parsed.count_source,
        Some(ChoiceObjectCountSource::ForEach(vec![
            "for".to_string(),
            "each".to_string(),
            "card".to_string(),
            "in".to_string(),
            "their".to_string(),
            "graveyard".to_string(),
        ]))
    );
}

#[test]
fn choice_object_shape_preserves_chosen_set_exclusion() {
    let tokens = lex_line(
        "Choose a nonland permanent they don't control that hasn't been chosen this way.",
        0,
    )
    .unwrap();
    let tokens = &tokens[..tokens.len() - 1];

    let ChoiceObjectClauseKind::Object(parsed) =
        parse_choice_object_clause_tokens(tokens).unwrap().unwrap()
    else {
        panic!("expected object choice");
    };
    assert_eq!(
        parsed.filter_words,
        ["nonland", "permanent", "they", "dont", "control"]
    );
    assert!(parsed.references.excludes_chosen_this_way);
}

#[test]
fn choice_player_shape_is_typed() {
    let tokens = lex_line(
        "Choose another player who cast one or more sorcery spells this turn",
        0,
    )
    .unwrap();
    let parsed = parse_choice_player_clause_tokens(&tokens).unwrap().unwrap();

    assert_eq!(
        parsed.filter,
        PlayerFilter::CastCardTypeThisTurn(CardType::Sorcery)
    );
    assert_eq!(parsed.exclude_previous_choices, 1);
    assert!(!parsed.random);
}

#[test]
fn card_type_reveal_pair_returns_count() {
    let first = [
        "choose", "a", "card", "type", "then", "reveal", "the", "top", "four", "cards", "of",
        "your", "library",
    ];
    let second = [
        "put", "all", "cards", "of", "the", "chosen", "type", "revealed", "this", "way", "into",
        "your", "hand", "and", "the", "rest", "on", "the", "bottom", "of", "your", "library",
    ];

    assert_eq!(
        parse_choice_card_type_reveal_shape_words(&first, &second),
        Some(ChoiceCardTypeRevealShape { count: 4 })
    );
}

#[test]
fn choice_separator_returns_typed_token_span() {
    let tokens = lex_line("those creatures become that type", 0).unwrap();
    let parsed =
        parse_choice_clause_separator_tokens(&tokens, ChoiceClauseSeparator::Become).unwrap();

    assert_eq!(parsed.first, 2);
    assert_eq!(parsed.end, 3);
}
