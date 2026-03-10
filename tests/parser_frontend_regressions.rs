use ironsmith::{
    cards::CardDefinitionBuilder,
    ids::CardId,
    types::CardType,
};

#[test]
fn parser_frontend_regression_up_to_x_target_clause_parses() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Return Count Regression")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return up to X target creatures to their owners' hands, where X is one plus the number of cards named Aether Burst in all graveyards as you cast this spell.",
        )
        .expect("up-to-X target clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("dynamic_x: true") && debug.contains("up_to_x: true"),
        "expected optional dynamic target count, got {debug}"
    );
}

#[test]
fn parser_frontend_regression_any_other_target_survives_lowering() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Screaming Nemesis Regression")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Haste\nWhenever this creature is dealt damage, it deals that much damage to any other target. If a player is dealt damage this way, they can't gain life for the rest of the game.",
        )
        .expect("any-other-target followup should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("AnyOtherTarget"),
        "expected lowered ability to preserve any-other-target semantics, got {debug}"
    );
}

#[test]
fn parser_frontend_regression_partner_with_fails_with_targeted_rule() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Partner With Regression")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Partner with Proud Mentor (When this creature enters, target player may put Proud Mentor into their hand from their library, then shuffle.)",
        )
        .expect_err("partner-with should fail loudly until it is implemented");

    let debug = format!("{err:?}");
    assert!(
        debug.contains("unsupported partner-with keyword line")
            && debug.contains("[rule=partner-with-keyword-line]"),
        "expected targeted partner-with diagnostic, got {debug}"
    );
}

#[test]
fn parser_frontend_regression_choose_up_to_x_modal_header_parses() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Modes Regression")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose up to X —\n• Counter target spell.\n• Draw a card.\n• Create a Treasure token.",
        )
        .expect("choose-up-to-X modal header should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("choose_count: X"),
        "expected dynamic modal max count, got {debug}"
    );
    assert!(
        debug.contains("min_choose_count: Some(\n                        Fixed(\n                            0,\n                        ),\n                    )")
            || debug.contains("min_choose_count: Some(Fixed(0))"),
        "expected zero modal minimum for choose-up-to-X, got {debug}"
    );
}
