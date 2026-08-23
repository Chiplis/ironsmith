use super::shard_16::parse_oracle_card_definition;
use super::*;

fn compiled_card_text(name: &str) -> String {
    let definition = parse_oracle_card_definition(name);
    compiled_text_lines(&definition).join("\n")
}

#[test]
fn unearthly_child_keeps_all_three_typed_consult_filter_arms() {
    let definition = parse_oracle_card_definition("An Unearthly Child");
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("Doctor")
            && debug.contains("Vehicle")
            && debug.contains("doctor's companion"),
        "the consult filter must retain the Doctor, doctor's companion, and Vehicle arms: {debug}"
    );
    assert_eq!(
        compiled_text_lines(&definition).join("\n"),
        "I, II, III — Reveal cards from the top of your library until you reveal a Doctor card, a card with doctor's companion, or a Vehicle card. Put that card into your hand and the rest on the bottom of your library in a random order."
    );
}

#[test]
fn descendants_fury_links_consult_filter_to_the_optional_sacrifice() {
    let definition = parse_oracle_card_definition("Descendants' Fury");
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("SharesSubtypeWithTagged") && debug.contains("sacrificed_"),
        "the consult filter must share a creature type with the creature sacrificed during resolution: {debug}"
    );
    assert_eq!(
        compiled_card_text("Descendants' Fury"),
        "Whenever one or more creatures you control deal combat damage to a player, you may sacrifice one of them. If you do, reveal cards from the top of your library until you reveal a creature card that shares a creature type with the sacrificed creature. Put that card onto the battlefield and the rest on the bottom of your library in a random order."
    );
}
