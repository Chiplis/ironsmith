use super::*;

#[test]
fn each_player_reveal_land_partition_keeps_one_shared_revealed_set() {
    let text = "Each player reveals the top five cards of their library, puts all land cards revealed this way onto the battlefield tapped, and exiles the rest.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Reveal Partition Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .expect("each-player reveal partition should compile");
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text,
        "{debug}"
    );
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("LookAtTopCardsEffect"), "{debug}");
    assert!(debug.contains("ForEachTaggedEffect"), "{debug}");
    assert!(debug.contains("enters_tapped: true"), "{debug}");
    assert!(debug.contains("zone: Exile"), "{debug}");
    assert!(
        !debug
            .contains("TagKey(\n                                                        \"rest\""),
        "{debug}"
    );
}
