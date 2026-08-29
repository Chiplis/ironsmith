use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_split_all_and_preserves_exclusions() {
    let split = lex_line("Destroy all artifacts and enchantments.", 0).unwrap();
    let split = parse_split_all_shape(&split).unwrap();
    assert_eq!(split.connective, SplitAllConnectiveShape::And);
    assert_eq!(split.filter_tokens.len(), 2);
    let alternative = lex_line("Destroy all lands or all creatures.", 0).unwrap();
    let alternative = parse_split_all_shape(&alternative).unwrap();
    assert_eq!(alternative.connective, SplitAllConnectiveShape::Or);
    assert_eq!(alternative.filter_tokens.len(), 2);
    let union = lex_line("Destroy all creatures or planeswalkers.", 0).unwrap();
    assert!(parse_split_all_shape(&union).is_none());
    let temporary = lex_line(
        "Exile all creatures and planeswalkers until this enchantment leaves the battlefield.",
        0,
    )
    .unwrap();
    assert!(parse_split_all_shape(&temporary).is_none());
}

#[test]
fn parses_exile_return_and_repeated_target_shapes() {
    let returned = lex_line(
            "You may exile target artifact, then return it to the battlefield with a +1/+1 counter on it.",
            0,
        )
        .unwrap();
    assert!(
        parse_exile_return_same_shape(&returned)
            .unwrap()
            .counter_tokens
            .is_some()
    );
    let delayed = lex_line(
        "Exile target creature at end of combat, then return it to the battlefield.",
        0,
    )
    .unwrap();
    assert!(
        parse_exile_return_same_shape(&delayed)
            .unwrap()
            .delayed_until_end_of_combat
    );
    let repeated = lex_line(
            "Exile up to one target artifact, up to one target creature, and up to one target enchantment.",
            0,
        )
        .unwrap();
    assert_eq!(
        parse_exile_each_target_type_shape(&repeated)
            .unwrap()
            .filter_tokens
            .len(),
        3
    );

    let and_or = lex_line(
            "Exile up to one target artifact, up to one target creature, up to one target enchantment, up to one target planeswalker, and/or up to one target land.",
            0,
        )
        .unwrap();
    assert_eq!(
        parse_exile_each_target_type_shape(&and_or)
            .unwrap()
            .filter_tokens
            .len(),
        5
    );
}
