use super::super::super::lexer::lex_line;
use super::*;

#[test]
fn parses_partner_name_without_reminder_text() {
    let tokens = lex_line(
        "Partner with Toothy, Imaginary Friend (When this creature enters...)",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_partner_with_name_tokens(&tokens).as_deref(),
        Some("Toothy, Imaginary Friend")
    );

    let label_tokens = lex_line(
        "Partner - Friends forever (You can have two commanders.)",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_partner_visible_label_tokens(&label_tokens).as_deref(),
        Some("Partner - Friends forever")
    );
    let compact_label = lex_line("Partner—Character select (Reminder.)", 0).unwrap();
    assert_eq!(
        parse_partner_visible_label_tokens(&compact_label).as_deref(),
        Some("Partner—Character select")
    );
}

#[test]
fn parses_optional_cost_and_cast_trigger_payloads() {
    let tokens = lex_line(
        "As an additional cost to cast this spell, you may sacrifice a creature. When you do, draw two cards.",
        0,
    )
    .unwrap();
    let parsed = parse_optional_cost_with_cast_trigger_tokens(&tokens).unwrap();
    assert_eq!(
        render_token_slice(parsed.label_tokens),
        "you may sacrifice a creature"
    );
    assert_eq!(
        render_token_slice(parsed.optional_cost_effect_tokens),
        "sacrifice a creature"
    );
    assert_eq!(
        render_token_slice(parsed.followup_effect_tokens),
        "draw two cards"
    );
}

#[test]
fn parses_optional_behold_and_blight_costs() {
    for (text, expected) in [
        (
            "As an additional cost to cast this spell, you may behold a Goblin.",
            OptionalKeywordCostKind::Behold,
        ),
        (
            "As an additional cost to cast this spell, you may blight 2.",
            OptionalKeywordCostKind::Blight,
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let parsed = parse_optional_keyword_additional_cost_tokens(&tokens).unwrap();
        assert_eq!(parsed.kind, expected);
    }
}
