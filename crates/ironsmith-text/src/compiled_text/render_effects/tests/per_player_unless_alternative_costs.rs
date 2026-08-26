use super::*;

const PERFORATING_ARTIST: &str = "Deathtouch\nRaid — At the beginning of your end step, if you attacked this turn, each opponent loses 3 life unless that player sacrifices a nonland permanent of their choice or discards a card.";

fn compile(name: &str, oracle: &str) -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"))
}

#[test]
fn per_player_sacrifice_or_discard_remains_one_alternative_payment() {
    let definition = compile("Perforating Artist", PERFORATING_ARTIST);
    let debug = format!("{definition:#?}");

    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("UnlessPaysEffect"), "{debug}");
    assert!(debug.contains("kind: OneOf"), "{debug}");
    assert!(debug.contains("SacrificeEffect"), "{debug}");
    assert!(debug.contains("DiscardEffect"), "{debug}");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        PERFORATING_ARTIST
    );
}

#[test]
fn conjunctive_sacrifice_and_discard_does_not_gain_an_alternative_surface() {
    let oracle = "You lose 3 life unless you sacrifice a nonland permanent and discard a card.";
    let definition = compile("Conjunctive Payment Probe", oracle);
    let debug = format!("{definition:#?}");

    assert!(debug.contains("UnlessPaysEffect"), "{debug}");
    assert!(!debug.contains("kind: OneOf"), "{debug}");
}
