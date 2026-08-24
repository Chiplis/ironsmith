use super::super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn recognizes_effect_heads_and_preserved_and_boundaries() {
    let tokens = lex_line(
        "Prevent the next 3 damage that would be dealt to any target this turn.",
        0,
    )
    .unwrap();
    assert!(has_extended_effect_head_tokens(&tokens));

    let tokens = lex_line("Create a white and blue creature token.", 0).unwrap();
    let and_idx = tokens
        .iter()
        .position(|token| token.is_word("and"))
        .unwrap();
    assert_eq!(
        preserve_and_reason(&tokens[..and_idx], &tokens[and_idx + 1..], true),
        Some(AndPreservation::ColorPair)
    );

    let tokens = lex_line(
        r#"You get an emblem with "You have no maximum hand size." and "{T}: Draw a card.""#,
        0,
    )
    .unwrap();
    let and_idx = tokens
        .iter()
        .position(|token| token.is_word("and"))
        .unwrap();
    assert_eq!(
        preserve_and_reason(&tokens[..and_idx], &tokens[and_idx + 1..], true),
        Some(AndPreservation::QuotedAbility)
    );

    let tokens = lex_line(
            "Until end of turn, target creature gains trample and \"Whenever this creature attacks, draw a card.\"",
            0,
        )
        .unwrap();
    let and_idx = tokens
        .iter()
        .position(|token| token.is_word("and"))
        .unwrap();
    assert_eq!(
        preserve_and_reason(&tokens[..and_idx], &tokens[and_idx + 1..], true),
        Some(AndPreservation::QuotedAbility)
    );
}

#[test]
fn named_source_damage_equal_followup_is_an_effect_boundary() {
    let before = lex_line("Destroy target land", 0).unwrap();
    let after = lex_line(
            "Roiling Terrain deals damage to that land's controller equal to the number of land cards in that player's graveyard.",
            0,
        )
        .unwrap();

    assert!(then_followup_facts(&before, &after, false).should_split(false));
}

#[test]
fn transform_back_reference_is_an_executable_then_boundary() {
    let before = lex_line("Untap it", 0).unwrap();
    let after = lex_line("transform it", 0).unwrap();

    assert!(then_followup_facts(&before, &after, false).should_split(false));
}

#[test]
fn result_amount_damage_is_an_executable_then_boundary() {
    let before = lex_line("Put them into their owners' graveyards", 0).unwrap();
    let after = lex_line(
        "this enchantment deals that much damage to each opponent",
        0,
    )
    .unwrap();

    assert!(then_followup_facts(&before, &after, false).should_split(false));
}
