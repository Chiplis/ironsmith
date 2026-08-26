use super::*;

#[test]
fn trailing_instead_if_keeps_the_nested_threshold_on_the_announced_target() {
    let oracle = "Destroy target creature if it has mana value 2 or less.\nRevolt — Destroy that creature if it has mana value 4 or less instead if a permanent left the battlefield under your control this turn.";
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Target Threshold Replacement Probe",
    )
    .card_types(vec![CardType::Instant])
    .parse_text(oracle)
    .expect("target threshold self-replacement should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}
