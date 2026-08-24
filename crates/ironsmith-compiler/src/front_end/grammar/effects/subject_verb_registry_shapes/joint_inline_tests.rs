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
