use super::*;

#[test]
fn dynamic_draw_counts_keep_the_counted_unit() {
    for oracle in [
        "Draw a card for each card you've discarded this turn.",
        "Draw a card for each card type among spells you've cast this turn.",
        "Draw a card for each player being attacked.",
    ] {
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(), "Dynamic Draw Probe",
        )
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("dynamic draw count must parse");
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), oracle);
    }
}

#[test]
fn dynamic_draw_counts_do_not_replace_explicit_equal_to_or_where_x() {
    for oracle in [
        "Draw cards equal to the number of cards you've discarded this turn.",
        "Draw X cards, where X is the number of cards you've discarded this turn.",
    ] {
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(), "Dynamic Draw Surface Probe",
        )
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("explicit dynamic draw count must parse");
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), oracle);
    }
}
