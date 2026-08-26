use super::*;

#[test]
fn different_power_search_keeps_opponent_choice_and_complementary_destinations() {
    let oracle = "Search your library for up to four creature cards with different powers and reveal them. An opponent chooses two of those cards. Shuffle the chosen cards into your library and put the rest into your hand.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Threat Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("delegated searched-set partition should compile");
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        oracle,
        "{debug}"
    );
    assert!(debug.contains("ChoosePlayerEffect"), "{debug}");
    assert!(debug.contains("ShuffleObjectsIntoLibraryEffect"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(
        !debug.contains("controller: Some(\n                                        Opponent"),
        "{debug}"
    );
}
