use super::*;

#[test]
fn smeagol_keeps_the_selected_land_and_revealed_remainder_partition() {
    let oracle = "At the beginning of your end step, if a creature died under your control this turn, the Ring tempts you.\nWhenever the Ring tempts you, target opponent reveals cards from the top of their library until they reveal a land card. Put that card onto the battlefield tapped under your control and the rest into their graveyard.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sméagol, Helpful Guide")
            .card_types(vec![CardType::Creature])
            .parse_text(oracle)
            .expect("selected-land consult partition should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ConsultTopOfLibraryEffect"), "{debug}");
    assert!(debug.contains("battlefield_controller: You"), "{debug}");
    assert!(debug.contains("enters_tapped: true"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}
