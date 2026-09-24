use super::*;

fn compile(name: &str, type_line: &str, text: &str) -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .parse_text(format!("Type: {type_line}\n{text}"))
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"))
}

#[test]
fn creature_you_control_connives_trigger_keeps_its_subject_filter() {
    let definition = compile(
        "Iron Monger, Sadistic Tycoon",
        "Legendary Artifact Creature — Construct Villain",
        "Whenever a creature you control connives, put a +1/+1 counter on each Villain you control.",
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        ["Whenever a creature you control connives, put a +1/+1 counter on each Villain you control."]
    );
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("Connive"), "{debug}");
    assert!(
        debug.contains("source_filter: Some("),
        "the trigger must be scoped to creatures you control: {debug}"
    );
}

#[test]
fn leader_connive_replacement_compiles_to_a_keyword_action_replacement() {
    let definition = compile(
        "Leader, Super-Genius",
        "Legendary Creature — Gamma Scientist Villain",
        "If a creature you control would connive, instead you draw a card, then that creature connives.",
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        ["If a creature you control would connive, instead draw a card, then that creature connives."]
    );
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("KeywordActionReplacement"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
    assert!(debug.contains("ConniveEffect"), "{debug}");
}

#[test]
fn optional_connive_keeps_the_causative_have() {
    let definition = compile(
        "Baron Strucker, HYDRA Overlord",
        "Legendary Creature — Human Villain",
        "Whenever another Villain you control enters, you may have it connive. Do this only once each turn.",
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        ["Whenever another Villain you control enters, you may have it connive. Do this only once each turn."]
    );
}
