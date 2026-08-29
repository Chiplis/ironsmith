use super::*;

fn render_card(name: &str, card_type: CardType, text: &str) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![card_type])
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"));
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn identity_crisis_rejoins_the_shared_target_players_zones() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Identity Crisis")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Exile all cards from target player's hand and graveyard.")
            .expect("Identity Crisis should compile");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert_eq!(
        rendered, "Exile all cards from target player's hand and graveyard.",
        "{:#?}",
        definition.spell_effect
    );
}

#[test]
fn coin_of_fate_returns_the_exact_other_exiled_card() {
    assert_eq!(
        render_card(
            "Coin of Fate",
            CardType::Artifact,
            "When this artifact enters, surveil 1.\n{3}{W}, {T}, Exile two creature cards from your graveyard, Sacrifice this artifact: An opponent chooses one of the exiled cards. You put that card on the bottom of your library and return the other to the battlefield tapped. You become the monarch.",
        ),
        "When this artifact enters, surveil 1.\n{3}{W}, {T}, Exile two creature cards from your graveyard, Sacrifice this artifact: An opponent chooses one of the exiled cards. You put that card on the bottom of your library and return the other to the battlefield tapped. You become the monarch."
    );
}

#[test]
fn deliver_unto_evil_keeps_the_remainder_move_inside_the_false_branch() {
    let rendered = render_card(
        "Deliver Unto Evil",
        CardType::Sorcery,
        "Choose up to four target cards in your graveyard. If you control a Bolas planeswalker, return those cards to your hand. Otherwise, an opponent chooses two of them. Leave the chosen cards in your graveyard and put the rest into your hand.\nExile Deliver Unto Evil.",
    );
    assert_eq!(
        rendered,
        "Choose up to four target cards in your graveyard. If you control a Bolas planeswalker, return those cards to your hand. Otherwise, an opponent chooses two of them. Leave the chosen cards in your graveyard and put the rest into your hand.\nExile Deliver Unto Evil."
    );
}

#[test]
fn wake_to_slaughter_preserves_the_chosen_and_other_card_identity() {
    assert_eq!(
        render_card(
            "Wake to Slaughter",
            CardType::Sorcery,
            "Choose up to two target creature cards in your graveyard. An opponent chooses one of them. Return that card to your hand. Return the other to the battlefield under your control. It gains haste. Exile it at the beginning of the next end step.\nFlashback {4}{B}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
        ),
        "Choose up to two target creature cards in your graveyard. An opponent chooses one of them. Return that card to your hand. Return the other to the battlefield under your control. It gains haste. Exile it at the beginning of the next end step.\nFlashback—{4}{B}{R}."
    );
}

#[test]
fn karn_reveal_partition_preserves_the_chosen_and_other_card_identity() {
    let rendered = render_card(
        "Karn, Scion of Urza",
        CardType::Planeswalker,
        "+1: Reveal the top two cards of your library. An opponent chooses one of them. Put that card into your hand and exile the other with a silver counter on it.\n−1: Put a card you own with a silver counter on it from exile into your hand.\n−2: Create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\"",
    );
    assert_eq!(
        rendered.lines().next().expect("Karn +1 line"),
        "+1: Reveal the top two cards of your library. An opponent chooses one of them. Put that card into your hand and exile the other with a silver counter on it."
    );
}
