use super::*;

#[test]
fn haunting_echoes_rejoins_the_exiled_set_search_and_target_shuffle() {
    let oracle = "Exile all cards from target player's graveyard other than basic land cards. For each card exiled this way, search that player's library for all cards with the same name as that card and exile them. Then that player shuffles.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Haunting Echoes")
            .card_types(vec![CardType::Sorcery])
            .parse_text(oracle)
            .expect("graveyard exception and same-name search should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("excluded_card_types: [\n"), "{debug}");
    assert!(debug.contains("excluded_supertypes: [\n"), "{debug}");
    assert!(debug.contains("SameNameAsTagged"), "{debug}");
    assert!(debug.contains("search_mode: AllMatching"), "{debug}");
    assert!(debug.contains("ShuffleLibraryEffect"), "{debug}");
}
