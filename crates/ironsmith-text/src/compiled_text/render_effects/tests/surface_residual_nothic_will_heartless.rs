use super::*;

fn render_card(name: &str, card_type: CardType, text: &str) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![card_type])
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"));
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn nothic_preserves_the_controller_subject_in_every_draw_and_lose_row() {
    assert_eq!(
        render_card(
            "Nothic",
            CardType::Creature,
            "Weird Insight — When this creature dies, roll a d20.\n1—9 | You draw a card and you lose 1 life.\n10—19 | You draw two cards and you lose 2 life.\n20 | You draw seven cards and you lose 7 life.",
        ),
        "Weird Insight — When this creature dies, roll a d20.\n1—9 | You draw a card and you lose 1 life.\n10—19 | You draw two cards and you lose 2 life.\n20 | You draw seven cards and you lose 7 life."
    );
}

#[test]
fn blocking_creature_self_replacement_uses_the_pronoun_condition() {
    assert_eq!(
        render_card(
            "Will of the All-Hunter",
            CardType::Instant,
            "Target creature gets +2/+2 until end of turn. If it's blocking, instead put two +1/+1 counters on it.\nCycling {2}",
        ),
        "Target creature gets +2/+2 until end of turn. If it's blocking, instead put two +1/+1 counters on it.\nCycling {2}"
    );
}

#[test]
fn quantified_life_total_damage_uses_each_players_possessive() {
    assert_eq!(
        render_card(
            "Quantified Life Damage Probe",
            CardType::Creature,
            "{T}: This creature deals damage to each player equal to half that player's life total, rounded down.",
        ),
        "{T}: This creature deals damage to each player equal to half that player's life total, rounded down."
    );
}
