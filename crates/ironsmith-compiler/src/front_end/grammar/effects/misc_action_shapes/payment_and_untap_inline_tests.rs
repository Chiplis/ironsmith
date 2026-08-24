use crate::lexer::lex_line;

use super::*;

#[test]
fn parses_untap_targets_and_repeated_tagged_mana() {
    let all = lex_line("each artifact", 0).unwrap();
    assert!(matches!(
        parse_untap_action_tokens(&all),
        UntapActionShape::All { .. }
    ));
    let conjoined = lex_line(
        "all nonland permanents you control and all nonland permanents that player controls",
        0,
    )
    .unwrap();
    let conjoined = parse_conjoined_untap_all_tokens(&conjoined)
        .expect("two quantified untap sets should parse");
    assert_eq!(
        crate::lexer::token_word_refs(conjoined.left_filter_tokens),
        ["nonland", "permanents", "you", "control"]
    );
    assert_eq!(
        crate::lexer::token_word_refs(conjoined.right_filter_tokens),
        ["nonland", "permanents", "that", "player", "controls"]
    );
    let tagged = lex_line("them", 0).unwrap();
    assert_eq!(
        parse_untap_action_tokens(&tagged),
        UntapActionShape::Tagged {
            filter_tokens: None
        }
    );

    let chosen = lex_line("the chosen permanents you control", 0).unwrap();
    assert_eq!(
        parse_chosen_object_set_filter_tokens(&chosen).map(crate::lexer::token_word_refs),
        Some(vec!["permanents", "you", "control"])
    );

    let those_creatures = lex_line("those creatures", 0).unwrap();
    let UntapActionShape::Tagged {
        filter_tokens: Some(filter_tokens),
    } = parse_untap_action_tokens(&those_creatures)
    else {
        panic!("expected a typed tagged-set untap subject");
    };
    assert_eq!(
        filter_tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>(),
        ["creatures"]
    );

    let payment = lex_line("{w} for each of those chosen this way", 0).unwrap();
    assert_eq!(
        parse_repeated_tagged_mana_payment_tokens(&payment)
            .unwrap()
            .pip_groups
            .len(),
        1
    );
}

#[test]
fn parses_x_payment_bounded_by_triggering_life_gain() {
    let payment = lex_line(
        "pay {X}, where X is less than or equal to the amount of life you gained",
        0,
    )
    .unwrap();
    let parsed = parse_bounded_x_payment_tokens(&payment).expect("bounded X payment should parse");
    assert_eq!(parsed.cost.to_oracle(), "{X}");
    assert_eq!(parsed.maximum, BoundedXMaximumShape::TriggeringLifeGained);
}
