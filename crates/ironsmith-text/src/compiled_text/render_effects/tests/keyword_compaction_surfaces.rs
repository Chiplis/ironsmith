use super::*;

#[test]
fn transfigure_compacts_the_canonical_library_sequence() {
    let oracle = "Transfigure {1}{B}{B}";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Transfigure Surface Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(oracle)
            .expect("Transfigure should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

#[test]
fn mutual_fight_keeps_the_authored_collection_surface() {
    let oracle = "Choose two target creatures. Those creatures fight each other.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Mutual Fight Surface Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(oracle)
            .expect("mutual fight should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}
