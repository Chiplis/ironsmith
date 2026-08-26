use super::*;

fn compile_search(text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Search Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("search sentence should compile")
}

#[test]
fn multi_zone_search_preserves_independent_filter_slots() {
    let text = "Search your library and graveyard for a basic land card and a card named Jiang Yanggu, reveal them, put them into your hand, then shuffle.";
    let definition = compile_search(text);
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
    assert!(debug.contains("SearchLibrarySlotsEffect"), "{debug}");
    assert_eq!(debug.matches("SearchLibrarySlot {").count(), 2, "{debug}");
}

#[test]
fn ordinary_multi_zone_search_keeps_a_single_combined_filter() {
    let definition = compile_search(
        "Search your library and graveyard for a creature card, reveal it, put it into your hand, then shuffle.",
    );
    let debug = format!("{definition:#?}");

    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(!debug.contains("SearchLibrarySlotsEffect"), "{debug}");
}
