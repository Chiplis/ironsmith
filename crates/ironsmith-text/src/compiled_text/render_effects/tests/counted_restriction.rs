use super::*;

#[test]
fn counted_counter_threshold_keeps_its_resolved_quantity_clause() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter Restriction Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, count the number of +1/+1 counters on creatures you control. Creatures your opponents control with power less than or equal to that number can't block this turn.")
        .expect("counter threshold compiles");
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(text.contains("count the number of +1/+1 counters on creatures you control. Creatures your opponents control with power less than or equal to that number can't block this turn"), "{text}");
}
