use super::*;

const ORACLE: &str = "At the beginning of your upkeep, the player with the lowest life total gains control of this creature. If two or more players are tied for lowest life total, you choose one of them, and that player gains control of this creature.";

#[test]
fn lowest_life_control_handoff_keeps_the_typed_tie_choice() {
    let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Lowest Life Handoff Probe",
    )
    .card_types(vec![CardType::Creature])
    .parse_text(ORACLE)
    .expect("the complete lowest-life handoff should compile");

    let debug = format!("{definition:#?}");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [ORACLE]
    );
    assert!(debug.contains("LowestLifeTied"), "{debug}");
    assert!(debug.contains("ChoosePlayerEffect"), "{debug}");
}

#[test]
fn lowest_life_control_handoff_does_not_invent_a_tie_choice() {
    const SINGLE_HANDOFF: &str = "At the beginning of your upkeep, the player with the lowest life total gains control of this creature.";
    let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Lowest Life Single Handoff Probe",
    )
    .card_types(vec![CardType::Creature])
    .parse_text(SINGLE_HANDOFF)
    .expect("a single lowest-life handoff should compile");

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join(" ");
    assert!(
        !rendered.contains("If two or more players are tied"),
        "{rendered}"
    );
    assert!(!format!("{definition:#?}").contains("ChoosePlayerEffect"));
}
