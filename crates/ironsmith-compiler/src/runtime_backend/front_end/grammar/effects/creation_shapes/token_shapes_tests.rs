use super::super::super::super::super::lexer::{lex_line, parser_token_word_refs, token_word_refs};
use super::*;

#[test]
fn parses_create_head_and_delays() {
    let tokens = lex_line("Create that many tapped Goblin tokens.", 0).unwrap();
    let head = parse_create_head_tokens(&tokens).unwrap();
    assert_eq!(head.count, CreateCountHead::EventAmount);
    assert_eq!(head.name_words, vec!["tapped", "Goblin"]);

    let words = ["sacrifice", "those", "tokens", "at", "end", "of", "combat"];
    assert_eq!(
        parse_delayed_combat_token_action_words(&words),
        Some(DelayedCombatTokenAction::Sacrifice)
    );
}

#[test]
fn named_token_clause_ignores_names_inside_quoted_rules() {
    let named = lex_line("named Twin with flying", 0).unwrap();
    let shape = parse_named_token_clause_tokens(&named).unwrap();
    assert_eq!(token_word_refs(&named[shape.name]), vec!["Twin"]);

    let quoted = lex_line(
        "with \"When this token leaves the battlefield, return target card named Ozox from your graveyard to your hand.\"",
        0,
    )
    .unwrap();
    assert!(parse_named_token_clause_tokens(&quoted).is_none());

    let multiple_quoted = lex_line(
        "with \"Jumblebones can't block\" and \"When Jumblebones leaves the battlefield, return target card named Ozox, the Clattering King from your graveyard to your hand.\"",
        0,
    )
    .unwrap();
    assert!(parse_named_token_clause_tokens(&multiple_quoted).is_none());

    let mut missing_final_quote = multiple_quoted;
    assert!(
        missing_final_quote
            .last()
            .is_some_and(OwnedLexToken::is_quote)
    );
    missing_final_quote.pop();
    assert!(parse_named_token_clause_tokens(&missing_final_quote).is_none());
}

#[test]
fn parses_copy_combat_modifiers() {
    let tokens = lex_line(
        "target creature that's tapped and attacking that player or a planeswalker they control",
        0,
    )
    .unwrap();
    let parsed = parse_inline_combat_tokens(&tokens);
    assert!(parsed.enters_tapped);
    assert!(parsed.enters_attacking);
    assert!(parsed.attacks_that_player_or_planeswalker);
    assert_eq!(
        token_word_refs(&parsed.source_tokens),
        vec!["target", "creature"]
    );
}

#[test]
fn parses_pt_words_with_winnow() {
    assert_eq!(
        parse_pt_word("-1/*"),
        Some(PtSurface {
            power: PtComponent::Fixed(-1),
            toughness: PtComponent::Star,
        })
    );
    assert_eq!(
        parse_pt_word("x/1"),
        Some(PtSurface {
            power: PtComponent::X,
            toughness: PtComponent::Fixed(1),
        })
    );
    assert_eq!(parse_unsigned_pt_word("2/3"), Some((2, 3)));
    assert_eq!(parse_unsigned_pt_word("+2/3"), None);
}

#[test]
fn creation_count_ignores_for_each_inside_quoted_token_rules() {
    let quoted_only = lex_line(
        "a Book Equipment artifact token with \"Equipped creature gets +1/+1 for each quest counter among permanents you control\" and equip {1}",
        0,
    )
    .unwrap();
    assert!(parse_for_each_clause_tokens(&quoted_only).is_none());

    let unquoted = lex_line(
        "a 1/1 Soldier creature token for each creature you control with \"This creature gets +1/+1 for each counter on it\"",
        0,
    )
    .unwrap();
    let clause = parse_for_each_clause_tokens(&unquoted).unwrap();
    assert_eq!(
        parser_token_word_refs(clause.filter_tokens),
        vec![
            "creature", "you", "control", "with", "this", "creature", "gets", "+1/+1", "for",
            "each", "counter", "on", "it"
        ]
    );
}
