use super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn parses_both_anthem_keyword_orders() {
    let tokens = lex_line(
            "Equipped creature has first strike and gets +1/+0 for each instant and sorcery card in your graveyard.",
            0,
        )
        .unwrap();
    let head = parse_anthem_keyword_head(&tokens).unwrap();
    assert_eq!(head.order, AnthemKeywordOrder::KeywordBeforeAnthem);
    let shape = parse_keyword_before_anthem_shape(&tokens, head).unwrap();
    assert!(!shape.subject_tokens.is_empty());
    assert_eq!(
        crate::lexer::parser_token_word_refs(shape.keyword_tokens),
        ["first", "strike"]
    );
    assert_eq!(shape.anthem_tail_tokens[0].parser_text(), "gets");

    let tokens = lex_line("Creatures you control get +1/+1 and have flying.", 0).unwrap();
    let head = parse_anthem_keyword_head(&tokens).unwrap();
    assert_eq!(head.order, AnthemKeywordOrder::AnthemBeforeKeyword);
}

#[test]
fn ignores_anthem_verbs_inside_quoted_granted_abilities() {
    let tokens = lex_line(
            "As long as enchanted permanent is an Equipment, it has \"Equipped creature gets +1/+1 and has trample.\"",
            0,
        )
        .unwrap();
    assert!(parse_anthem_keyword_head(&tokens).is_none());
}

#[test]
fn parses_color_and_compound_segments() {
    let tokens = lex_line("This creature gets +1/+1, is red, and has {T}: Add {R}.", 0).unwrap();
    let head = parse_anthem_keyword_head(&tokens).unwrap();
    assert_eq!(
        parse_anthem_keyword_color_segment(&tokens, head)
            .unwrap()
            .color,
        ColorSet::RED
    );

    let tokens = lex_line(
        "Creatures you control get +1/+1 and are red and have flying.",
        0,
    )
    .unwrap();
    let head = parse_anthem_keyword_head(&tokens).unwrap();
    assert!(parse_anthem_keyword_color_segment(&tokens, head).is_none());

    let tokens = lex_line(
            "Creatures you control get +1/+1 and are red, and creatures you control get +0/+1 and have flying.",
            0,
        )
        .unwrap();
    let head = parse_anthem_keyword_head(&tokens).unwrap();
    assert!(parse_anthem_keyword_compound_split(&tokens, head).is_some());

    let attached_count = lex_line(
        "Equipped creature gets +1/+1 for each Aura and Equipment attached to it and has ward {2}.",
        0,
    )
    .unwrap();
    let head = parse_anthem_keyword_head(&attached_count).unwrap();
    assert!(parse_anthem_keyword_compound_split(&attached_count, head).is_none());
}

#[test]
fn splits_conditions_additions_and_activated_tails() {
    let tokens = lex_line("flying as long as you control an artifact", 0).unwrap();
    let split = split_anthem_keyword_trailing_condition(&tokens)
        .unwrap()
        .unwrap();
    assert_eq!(split.ability_tokens.len(), 1);
    assert!(!split.condition_tokens.is_empty());
    assert!(!split.trailing_if_surface);

    let tokens = lex_line(
        "split second if mana from an artifact was spent to cast it",
        0,
    )
    .unwrap();
    let split = split_anthem_keyword_trailing_condition(&tokens)
        .unwrap()
        .unwrap();
    assert_eq!(split.ability_tokens.len(), 2);
    assert!(split.trailing_if_surface);
    assert_eq!(
        crate::lexer::parser_token_word_refs(split.condition_tokens),
        [
            "mana", "from", "an", "artifact", "was", "spent", "to", "cast", "it"
        ]
    );

    let tokens = lex_line("if mana from an artifact was spent", 0).unwrap();
    assert_eq!(
        split_anthem_keyword_trailing_condition(&tokens),
        Err(AnthemKeywordTrailingConditionError::MissingAbility)
    );

    let tokens = lex_line("flying and is red", 0).unwrap();
    assert!(split_anthem_keyword_and_is(&tokens).is_some());

    let tokens = lex_line("flying and has {T}: Add {G}.", 0).unwrap();
    let split = split_anthem_keyword_and_have(&tokens).unwrap();
    assert!(parse_colon_tail_split(split.tail_tokens).is_some());
}
