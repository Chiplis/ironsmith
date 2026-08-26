use super::*;

#[test]
fn delayed_return_keeps_sibling_conditional_entry_counters() {
    let oracle = "Exile any number of target creatures and/or planeswalkers you control. At the beginning of the next end step, return each of them to the battlefield under its owner's control. Each of them enters with an additional +1/+1 counter on it if it's a creature and an additional loyalty counter on it if it's a planeswalker.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Semester Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("conditional entry counters on a delayed returned set should compile");
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        oracle,
        "{debug}"
    );
    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    assert!(debug.contains("Loyalty"), "{debug}");
    assert_eq!(
        debug.matches("BattlefieldEntryCounterSpec").count(),
        2,
        "{debug}"
    );
}
