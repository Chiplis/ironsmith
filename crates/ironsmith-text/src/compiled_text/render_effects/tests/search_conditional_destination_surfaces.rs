use super::*;

#[test]
fn loyal_inventor_rejoins_search_and_correlated_destinations() {
    let oracle = "Vigilance\nWhen this creature enters, you may search your library for an artifact card, reveal it, then shuffle. Put that card into your hand if you control an Assassin. Otherwise, put that card on top of your library.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Loyal Inventor")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("conditional searched-card destination should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("RevealTaggedEffect"), "{debug}");
    assert!(debug.contains("SearchedLibrary"), "{debug}");
    assert!(debug.contains("PlayerControls"), "{debug}");
    assert!(debug.contains("DidNotHappen"), "{debug}");
}
