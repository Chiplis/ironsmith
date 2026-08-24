use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_hand_targets_and_name_followup() {
    let tokens = lex_line("Look at an opponent's hand, then choose any card name.", 0).unwrap();
    assert_eq!(
        parse_look_hand_shape(&tokens),
        Some(LookHandShape {
            player: LookHandPlayerShape::Opponent,
            choose_card_name: true,
        })
    );
}

#[test]
fn parses_look_top_exile_one_shape() {
    let tokens = lex_line(
        "Look at the top three cards of your library, then exile one of those cards.",
        0,
    )
    .unwrap();
    let shape = parse_look_top_exile_one_shape(&tokens).unwrap();
    assert_eq!(shape.count, 3);
    assert_eq!(shape.player, PlayerAst::You);
}
