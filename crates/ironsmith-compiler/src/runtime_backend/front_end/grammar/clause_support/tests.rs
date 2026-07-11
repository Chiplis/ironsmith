use super::super::super::lexer::lex_line;
use super::*;

#[test]
fn protection_chain_returns_typed_targets_and_token_spans() {
    let tokens = lex_line(
        "Protection from red and from permanents that were cast this turn",
        0,
    )
    .unwrap();
    let parsed = parse_protection_chain_tokens(&tokens).unwrap();
    assert_eq!(parsed.targets.len(), 2);
    assert_eq!(parsed.targets[0].value, "red");
    assert_eq!(
        parsed.targets[1].kind,
        ProtectionTargetKind::PermanentCastThisTurn
    );
    assert_eq!(
        tokens[parsed.targets[1].target_token_first].parser_text(),
        "permanents"
    );
}

#[test]
fn protection_chain_classifies_chosen_color_surfaces() {
    let tokens = lex_line("Protection from the last chosen color", 0).unwrap();
    let parsed = parse_protection_chain_tokens(&tokens).unwrap();
    assert_eq!(parsed.targets.len(), 1);
    assert_eq!(parsed.targets[0].kind, ProtectionTargetKind::ChosenColor);
}

#[test]
fn protection_special_shapes_preserve_filter_boundary() {
    let tokens = lex_line(
        "Protection from each mana value among artifacts you control",
        0,
    )
    .unwrap();
    let parsed = parse_protection_chain_tokens(&tokens).unwrap();
    let ProtectionTargetKind::EachManaValueAmong { filter_word_first } = parsed.targets[0].kind
    else {
        panic!("expected each-mana-value protection target");
    };
    assert_eq!(
        TokenWordView::new(&tokens).word_refs()[filter_word_first..],
        ["artifacts", "you", "control"]
    );

    let colored = lex_line("Protection from spells that are one or more colors", 0).unwrap();
    assert!(parse_protection_from_colored_spells_tokens(&colored));
}

#[test]
fn ability_segments_preserve_comma_semicolon_and_conjunction_boundaries() {
    let tokens = lex_line("Flying, first strike; vigilance", 0).unwrap();
    assert_eq!(parse_ability_segments_tokens(&tokens).len(), 3);
    let tokens = lex_line("first strike and vigilance", 0).unwrap();
    assert_eq!(parse_conjoined_segments_tokens(&tokens).len(), 2);
}

#[test]
fn trigger_shapes_return_effect_and_attack_boundaries() {
    let tokens = lex_line(
        "Whenever you cast an instant or sorcery spell or activate an ability, if that spell's mana cost or that ability's activation cost contains {X}, copy that spell or ability.",
        0,
    )
    .unwrap();
    let effect_first = parse_combined_x_cost_trigger_tokens(&tokens).unwrap();
    assert_eq!(tokens[effect_first].parser_text(), "copy");

    let trigger = lex_line("you attack an opponent with one or more creatures", 0).unwrap();
    let shape = parse_attack_with_shape_tokens(&trigger).unwrap();
    assert_eq!(shape.subject_words, 0..1);
    assert_eq!(shape.attacked_words, Some(2..4));
    assert_eq!(trigger[shape.object_token_first].parser_text(), "one");
}

#[test]
fn source_trigger_and_delimiter_facts_are_typed() {
    let tokens = lex_line("this creature becomes blocked, draw a card", 0).unwrap();
    let prefix = parse_source_trigger_prefix_tokens(&tokens).unwrap();
    assert_eq!(prefix.kind, SourceTriggerKind::BecomesBlocked);
    assert_eq!(tokens[prefix.effect_first].kind, TokenKind::Comma);
    let facts = parse_trigger_delimiters_tokens(&tokens);
    assert_eq!(facts.first_comma, Some(prefix.effect_first));
    assert_eq!(
        facts.first_comma_or_then.map(|delimiter| delimiter.kind),
        Some(TriggerDelimiterKind::Comma)
    );
}

#[test]
fn color_only_hexproof_filters_preserve_each_and_disjunction() {
    let each = parse_color_only_hexproof_filter_words(&["each", "color"]).unwrap();
    assert_eq!(each.colors.map(ColorSet::count), Some(5));

    let listed = parse_color_only_hexproof_filter_words(&["white", "and", "from", "blue"]).unwrap();
    assert_eq!(listed.any_of.len(), 2);
    assert_eq!(listed.any_of[0].colors, Some(ColorSet::WHITE));
    assert_eq!(listed.any_of[1].colors, Some(ColorSet::BLUE));
}
