use super::*;

const STORMFIST_ORACLE: &str =
    "Menace\nAt the beginning of your upkeep, each player draws a card and loses 1 life.";

#[test]
fn bare_menace_and_coordinated_player_actions_keep_their_authored_surface() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Stormfist Crusader")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human, Subtype::Knight])
            .parse_text(STORMFIST_ORACLE)
            .expect("the coordinated each-player trigger should compile");

    let debug = format!("{definition:#?}");
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
    assert!(debug.contains("LoseLifeEffect"), "{debug}");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        STORMFIST_ORACLE
    );
}

#[test]
fn authored_standard_menace_reminder_is_still_preserved() {
    const REMINDER: &str =
        "Menace (This creature can't be blocked except by two or more creatures.)";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Menace Reminder Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(REMINDER)
            .expect("the standard menace reminder should compile");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [REMINDER.to_string()]
    );
}
