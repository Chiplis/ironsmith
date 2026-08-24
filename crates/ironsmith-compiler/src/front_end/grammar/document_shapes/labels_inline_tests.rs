use super::*;
use crate::lexer::lex_line;

#[test]
fn classifies_keyword_and_council_labels() {
    let kicker = lex_line("Kicker", 0).unwrap();
    assert_eq!(
        parse_label_prefix_kind_tokens(&kicker),
        Some(LabelPrefixKind::PreservedKeyword(
            PreservedKeywordLabelKind::CostOrCasting
        ))
    );
    let prototype = lex_line("Prototype {2}{R}", 0).unwrap();
    assert_eq!(
        parse_label_prefix_kind_tokens(&prototype),
        Some(LabelPrefixKind::PreservedKeyword(
            PreservedKeywordLabelKind::CostOrCasting
        ))
    );
    let council = lex_line("Council's dilemma", 0).unwrap();
    assert_eq!(
        parse_label_prefix_kind_tokens(&council),
        Some(LabelPrefixKind::CouncilChoice)
    );

    let council_with_dash = lex_line("Council's dilemma —", 0).unwrap();
    assert_eq!(
        parse_label_prefix_kind_tokens(&council_with_dash),
        Some(LabelPrefixKind::CouncilChoice)
    );
}

#[test]
fn statement_label_parser_preserves_numeric_result_tables_and_keyword_labels() {
    let numeric = lex_line("1—4 | Create a token", 0).unwrap();
    assert!(parse_numeric_result_prefix_tokens(&numeric).is_some());
    assert!(parse_statement_label_split_tokens(&numeric).is_none());

    let compact_ascii_numeric = lex_line("1-9 | Draw a card", 0).unwrap();
    assert!(parse_numeric_result_prefix_tokens(&compact_ascii_numeric).is_some());
    assert!(parse_statement_label_split_tokens(&compact_ascii_numeric).is_none());

    let exact_numeric_with_label = lex_line("1 | Trapped! — You lose 3 life", 0).unwrap();
    assert!(parse_numeric_result_prefix_tokens(&exact_numeric_with_label).is_some());
    assert!(parse_statement_label_split_tokens(&exact_numeric_with_label).is_none());

    let labeled = lex_line("Landfall — Draw a card", 0).unwrap();
    let stripped = parse_statement_label_strip_tokens(&labeled);
    assert_eq!(stripped.stripped_labels, 1);
    assert_eq!(stripped.body_tokens[0].parser_text(), "draw");

    let keyword = lex_line("Kicker — Draw a card", 0).unwrap();
    let preserved = parse_statement_label_strip_tokens(&keyword);
    assert_eq!(preserved.stripped_labels, 0);
    assert_eq!(preserved.body_tokens, keyword.as_slice());
}
