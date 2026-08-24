use super::*;
use crate::lexer::{TokenWordView, lex_line};

#[test]
fn parses_participant_and_who_shapes() {
    let clause = lex_line(
        "for each opponent other than defending player who did this way draw a card",
        0,
    )
    .unwrap();
    let outer = parse_participant_clause_shape(&clause).unwrap();
    assert!(!outer.participant_is_actor);
    assert_eq!(
        outer.scope,
        ForEachParticipantScope::OpponentExceptDefending
    );
    let WhoClauseShape::DidThisWay { effect_tokens, .. } =
        parse_who_clause_shape(outer.inner_tokens).unwrap()
    else {
        panic!("expected this-way shape");
    };
    assert_eq!(
        TokenWordView::new(effect_tokens).to_word_refs(),
        vec!["draw", "a", "card"]
    );
}

#[test]
fn distinguishes_participant_subjects_from_controller_imperatives() {
    let subject = lex_line("Each opponent chooses a creature", 0).unwrap();
    let imperative = lex_line("For each opponent, choose a creature", 0).unwrap();

    assert!(
        parse_participant_clause_shape(&subject)
            .unwrap()
            .participant_is_actor
    );
    assert!(
        !parse_participant_clause_shape(&imperative)
            .unwrap()
            .participant_is_actor
    );
}

#[test]
fn parses_each_other_player_as_filtered_participant_subject() {
    let tokens = lex_line("Each other player draws a card", 0).unwrap();
    let shape = parse_participant_clause_shape(&tokens).unwrap();
    assert!(shape.participant_is_actor);
    assert_eq!(shape.scope, ForEachParticipantScope::PlayerExceptYou);
    assert_eq!(
        TokenWordView::new(shape.inner_tokens).to_word_refs(),
        vec!["draws", "a", "card"]
    );

    let imperative = lex_line("For each other player, draw a card", 0).unwrap();
    assert!(
        parse_participant_clause_shape(&imperative).is_none(),
        "the quantified-subject family must not claim imperative fanout"
    );
}

#[test]
fn parses_source_attacked_player_qualifier_without_absorbing_action() {
    let tokens = lex_line("this creature attacked this turn loses the game", 0).unwrap();
    let shape = parse_source_attacked_player_clause_shape(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(shape.effect_tokens).to_word_refs(),
        vec!["loses", "the", "game"]
    );
}

#[test]
fn parses_relative_control_and_poison_shapes() {
    let control = lex_line("who controls the most creatures draws a card", 0).unwrap();
    let shape = parse_relative_control_clause_shape(&control).unwrap();
    assert!(shape.controls_most);

    let threshold = lex_line(
        "who controls four or fewer lands may search their library",
        0,
    )
    .unwrap();
    let shape = parse_relative_control_clause_shape(&threshold).unwrap();
    assert_eq!(
        shape.count_comparison,
        Some(crate::effect::Comparison::LessThanOrEqual(4))
    );
    assert_eq!(
        TokenWordView::new(shape.filter_tokens).to_word_refs(),
        vec!["lands"]
    );
    assert_eq!(
        TokenWordView::new(shape.effect_tokens).to_word_refs(),
        vec!["may", "search", "their", "library"]
    );

    let counted_choice = lex_line(
        "who controls six or more lands chooses five lands they control and sacrifices the rest",
        0,
    )
    .unwrap();
    let shape = parse_relative_control_clause_shape(&counted_choice).unwrap();
    assert_eq!(
        TokenWordView::new(shape.filter_tokens).to_word_refs(),
        vec!["lands"]
    );
    assert_eq!(
        TokenWordView::new(shape.effect_tokens).to_word_refs(),
        vec![
            "chooses",
            "five",
            "lands",
            "they",
            "control",
            "and",
            "sacrifices",
            "the",
            "rest"
        ]
    );

    let fewer_than_most = lex_line(
            "who controls fewer lands than the player who controls the most lands searches their library",
            0,
        )
        .unwrap();
    let shape = parse_relative_control_clause_shape(&fewer_than_most).unwrap();
    assert_eq!(
        TokenWordView::new(shape.filter_tokens).to_word_refs(),
        vec!["lands"]
    );
    assert_eq!(
        TokenWordView::new(shape.fewer_than_most_filter_tokens.unwrap()).to_word_refs(),
        vec!["lands"]
    );
    assert_eq!(
        TokenWordView::new(shape.effect_tokens).to_word_refs(),
        vec!["searches", "their", "library"]
    );

    let fewer_than_you = lex_line("who controls fewer creatures than you draws a card", 0).unwrap();
    let shape = parse_relative_control_clause_shape(&fewer_than_you).unwrap();
    assert!(shape.fewer_than_you);
    assert_eq!(
        TokenWordView::new(shape.filter_tokens).to_word_refs(),
        vec!["creatures"]
    );
    assert_eq!(
        TokenWordView::new(shape.effect_tokens).to_word_refs(),
        vec!["draws", "a", "card"]
    );

    let poison = lex_line("who has three or more poison counters loses the game", 0).unwrap();
    assert!(matches!(
        parse_opponent_special_shape(&poison).unwrap(),
        Some(OpponentSpecialShape::PoisonCounters { count: 3, .. })
    ));
}
