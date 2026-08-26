use super::*;

const ORACLE: &str = "You may pay {2}{U} rather than pay this spell's mana cost.\nIf the {2}{U} cost was paid, you draw three cards, then an opponent creates two Treasure tokens and they scry 2. If that cost wasn't paid, you draw X cards.";

#[test]
fn public_route_keeps_a_singular_opponent_actor_for_they_followup() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Anaphoric Opponent Actor Probe",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text(ORACLE)
    .expect("alternative-cost opponent actor program should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        ORACLE
    );
}
