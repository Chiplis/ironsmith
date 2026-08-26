use super::*;

#[test]
fn oath_of_ghouls_uses_the_migrated_graveyard_minority_program() {
    let oracle = "At the beginning of each player's upkeep, that player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent. The first player may return a creature card from their graveyard to their hand.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Oath of Ghouls")
        .card_types(vec![CardType::Enchantment])
        .parse_text(oracle)
        .expect("graveyard-minority trigger should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("AnOpponentHasFewerThanPlayer"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
    assert!(debug.contains("ReturnFromGraveyardToHandEffect"), "{debug}");
}
