use super::*;
use crate::lexer::lex_line;

#[test]
fn classifies_duration_and_dynamic_tails() {
    let timed = lex_line("+2/+2 until your next turn", 0).unwrap();
    assert_eq!(
        parse_modifier_tail_shape(&timed).duration,
        Until::YourNextTurn
    );

    let dynamic = lex_line("+1/+1 for each creature you control", 0).unwrap();
    assert!(matches!(
        parse_modifier_tail_shape(&dynamic).action,
        ModifierTailAction::DynamicForEach(_)
    ));
}

#[test]
fn splits_two_signed_pt_modifiers_and_their_shared_tail() {
    let tokens = lex_line("+1/-1 or -1/+1 until end of turn", 0).unwrap();
    let shape = parse_fixed_pt_alternative_shape(&tokens)
        .expect("inline P/T alternative should have two modifier branches");

    assert_eq!(shape.first_modifier.parser_text(), "+1/-1");
    assert_eq!(shape.second_modifier.parser_text(), "-1/+1");
    assert!(is_eot_tail(shape.trailing_tokens));
}
