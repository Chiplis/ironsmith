use super::*;
use crate::lexer::{LexedClause, lex_line};

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_flashback_and_prevention_followups() {
    let first = lex("Target card gains flashback until end of turn");
    let second = lex("The flashback cost is equal to its mana cost");
    let shape = parse_flashback_grant_shape(&first, &second).unwrap();
    assert_eq!(
        LexedClause::new(shape.target_tokens).word_refs(),
        vec!["target", "card"]
    );
    assert!(parse_prevention_reflect_followup_shape(&lex(
        "If damage is prevented this way, this creature deals that much damage to any target"
    )));
}

#[test]
fn parses_punctuated_iterative_library_sequence() {
    let first = lex("Exile the top card of your library.");
    let second = lex(
        "You may put that card into your hand unless it has the same name as another card exiled this way.",
    );
    let third = lex(
        "Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.",
    );
    assert!(parse_iterative_library_sequence_shape(
        &first, &second, &third
    ));
}

#[test]
fn parses_punctuated_each_player_pay_life_sequence() {
    let first = lex("Starting with you, each player may pay any amount of life.");
    let second = lex("Repeat this process until no one pays life.");
    let third = lex(
        "Each player creates a 1/1 black Rat creature token for each 1 life they paid this way.",
    );
    assert!(parse_each_player_pay_life_sequence_shape(
        &first, &second, &third
    ));
}

#[test]
fn parses_generic_starting_each_player_optional_repeat() {
    let eureka_first = lex(
        "Starting with you, each player may put a permanent card from their hand onto the battlefield.",
    );
    let eureka_repeat = lex("Repeat this process until no one puts a card onto the battlefield.");
    let eureka = parse_starting_each_player_optional_repeat_shape(&eureka_first, &eureka_repeat)
        .expect("the repeated optional action should be recognized");
    assert_eq!(
        LexedClause::new(eureka.each_player_clause_tokens).word_refs(),
        vec![
            "each",
            "player",
            "may",
            "put",
            "a",
            "permanent",
            "card",
            "from",
            "their",
            "hand",
            "onto",
            "the",
            "battlefield",
        ]
    );

    let pay_first = lex("Starting with you, each player may pay any amount of life.");
    let pay_repeat = lex("Repeat this process until no one pays life.");
    assert!(
        parse_starting_each_player_optional_repeat_shape(&pay_first, &pay_repeat).is_some(),
        "the recognizer should be action-generic and tolerate third-person verb agreement"
    );
}

#[test]
fn rejects_unrelated_repeat_action() {
    let first = lex("Starting with you, each player may discard a card.");
    let second = lex("Repeat this process until no one draws a card.");
    assert!(parse_starting_each_player_optional_repeat_shape(&first, &second).is_none());
}

#[test]
fn parses_delayed_upkeep_payment() {
    let upkeep = lex("At the beginning of your next upkeep, pay {2}{U}");
    let lose = lex("If you don't, you lose the game");
    let shape = parse_delayed_upkeep_payment_shape(&upkeep, &lose).unwrap();
    assert_eq!(shape.mana.to_oracle(), "{2}{U}");
}
