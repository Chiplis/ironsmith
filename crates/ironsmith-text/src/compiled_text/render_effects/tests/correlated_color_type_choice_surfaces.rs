use super::*;

#[test]
fn sarpadian_empires_keeps_correlated_color_type_options() {
    let oracle = "As this artifact enters, choose white Citizen, blue Camarid, black Thrull, red Goblin, or green Saproling.\n{3}, {T}: Create a 1/1 creature token of the chosen color and type.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sarpadian Empires, Vol. VII")
            .card_types(vec![CardType::Artifact])
            .parse_text(oracle)
            .expect("Sarpadian Empires should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    assert!(format!("{definition:#?}").contains("ChooseNamedOptionAsEnters"));
    assert!(!format!("{definition:#?}").contains("ChooseObjectsEffect"));
}
