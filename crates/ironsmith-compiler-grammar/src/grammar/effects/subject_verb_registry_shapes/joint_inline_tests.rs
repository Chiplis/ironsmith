use super::*;
use crate::lexer::{TokenWordView, lex_line};

#[test]
fn parses_joint_player_actions() {
    let draw = lex_line("You and that player each draw two cards.", 0).unwrap();
    let shape = parse_joint_draw_shape(&draw).unwrap();
    assert_eq!(shape.other_player, PlayerAst::That);
    assert_eq!(
        TokenWordView::new(shape.amount_tokens).to_word_refs(),
        vec!["two", "cards"]
    );

    let draw_without_period = lex_line("You and target opponent each draw three cards", 0).unwrap();
    let shape = parse_joint_draw_shape(&draw_without_period)
        .expect("sentence dispatch strips terminal punctuation before registry shapes run");
    assert_eq!(shape.other_player, PlayerAst::TargetOpponent);
    assert_eq!(
        TokenWordView::new(shape.amount_tokens).to_word_refs(),
        vec!["three", "cards"]
    );

    let create = lex_line("You and target opponent each create a Treasure token.", 0).unwrap();
    assert_eq!(
        TokenWordView::new(parse_joint_create_shape(&create).unwrap().effect_tokens).to_word_refs(),
        vec!["create", "a", "treasure", "token"]
    );

    let sacrifice = lex_line("You and that player each sacrifice a creature.", 0).unwrap();
    let sacrifice = parse_joint_sacrifice_shape(&sacrifice).unwrap();
    assert_eq!(sacrifice.other_player, PlayerAst::That);
    assert_eq!(
        TokenWordView::new(sacrifice.object_tokens).to_word_refs(),
        vec!["a", "creature"]
    );
}

#[test]
fn parses_only_matching_joint_object_subjects() {
    let tokens = lex_line(
        "This creature and that creature each get +2/+0 and gain haste until end of turn.",
        0,
    )
    .unwrap();
    let shape = parse_joint_object_each_actions_shape(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(shape.action_tokens).to_word_refs(),
        vec![
            "get", "+2/+0", "and", "gain", "haste", "until", "end", "of", "turn"
        ]
    );

    let changed_kind = lex_line("This creature and that artifact each get +2/+0.", 0).unwrap();
    assert!(parse_joint_object_each_actions_shape(&changed_kind).is_none());
    let changed_referent =
        lex_line("This creature and another creature each get +2/+0.", 0).unwrap();
    assert!(parse_joint_object_each_actions_shape(&changed_referent).is_none());
    let missing_each = lex_line("This creature and that creature get +2/+0.", 0).unwrap();
    assert!(parse_joint_object_each_actions_shape(&missing_each).is_none());
}
