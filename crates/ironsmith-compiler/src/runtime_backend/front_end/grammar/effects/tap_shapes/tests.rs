use super::*;
use crate::runtime_backend::front_end::lexer::{lex_line, render_token_slice};

#[test]
fn splits_tap_then_return() {
    let tokens = lex_line("target creature, then return it to its owner's hand", 0).unwrap();
    let shape = parse_tap_then_return_tokens(&tokens).unwrap();
    assert_eq!(render_token_slice(shape.tap_tokens), "target creature,");
    assert_eq!(
        render_token_slice(shape.return_tokens),
        "return it to its owner's hand"
    );
}

#[test]
fn finds_control_relations() {
    let target = lex_line("creatures target player controls", 0).unwrap();
    assert_eq!(
        parse_tap_control_relation_tokens(&target),
        Some(TapControlRelation::TargetPlayer)
    );
    let that = lex_line("creatures that player controls", 0).unwrap();
    assert_eq!(
        parse_tap_control_relation_tokens(&that),
        Some(TapControlRelation::ThatPlayer)
    );

    let all = lex_line(
        "all creatures target player controls or untap all creatures that player controls",
        0,
    )
    .unwrap();
    assert!(parse_tap_or_untap_all_shape_tokens(&all).is_some());

    let choice = lex_line("or untap target permanent", 0).unwrap();
    assert!(parse_tap_or_untap_target_tokens(&choice).is_some());
}

#[test]
fn captures_type_choice_qualifier() {
    let tokens = lex_line("creatures of the chosen type you control", 0).unwrap();
    let shape = parse_tap_type_choice_tokens(&tokens).unwrap();
    assert_eq!(render_token_slice(shape.before_tokens), "creatures");
    assert_eq!(render_token_slice(shape.after_tokens), "you control");
}

#[test]
fn captures_coordinated_tap_operands_before_then_followup() {
    let tokens = lex_line(
        "Tap this creature and all creatures named Kobolds of Kher Keep, then an opponent gains control of them.",
        0,
    )
    .unwrap();
    let shape = parse_tap_object_union_then_tokens(&tokens).unwrap();
    assert_eq!(
        render_token_slice(shape.first_target_tokens),
        "this creature"
    );
    assert_eq!(
        render_token_slice(shape.all_filter_tokens),
        "creatures named Kobolds of Kher Keep"
    );
    assert_eq!(
        render_token_slice(shape.followup_tokens),
        "an opponent gains control of them."
    );
}
