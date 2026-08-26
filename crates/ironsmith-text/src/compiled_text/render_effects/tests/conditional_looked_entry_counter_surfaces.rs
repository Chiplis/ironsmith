use super::*;

#[test]
fn turntimber_symbiosis_keeps_conditional_entry_counters_and_remainder() {
    let oracle = "Look at the top seven cards of your library. You may put a creature card from among them onto the battlefield. If that card has mana value 3 or less, it enters with three additional +1/+1 counters on it. Put the rest on the bottom of your library in a random order.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Turntimber Symbiosis")
            .card_types(vec![CardType::Sorcery])
            .parse_text(oracle)
            .expect("conditional looked-card entry fixture should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "{debug}"
    );
    assert!(debug.contains("ThatObjectEntersIfCondition"), "{debug}");
}
