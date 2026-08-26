use super::*;

#[test]
fn each_player_return_keeps_additional_entry_counter() {
    let oracle = "Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Revival Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("each-player return with an entry counter should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("ReturnAllToBattlefieldEffect"), "{debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("MinusOneMinusOne"), "{debug}");
}
