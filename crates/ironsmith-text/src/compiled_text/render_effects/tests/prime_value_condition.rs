use super::*;

#[test]
fn prime_controlled_land_count_keeps_its_condition_and_created_token_referent() {
    let text = "At the beginning of your end step, if a land entered the battlefield under your control this turn and you control a prime number of lands, create Primo, the Indivisible, a legendary 0/0 green and blue Fractal creature token, then put that many +1/+1 counters on it.";
    let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Zimone, All-Questioning",
    )
    .card_types(vec![CardType::Creature])
    .parse_text(text)
    .expect("the grammar-owned prime-count program should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}
