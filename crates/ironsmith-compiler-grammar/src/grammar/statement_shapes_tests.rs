use super::player_counters::{PlayerCounterKind, PlayerCounterSubject, PlayerGetsCountersShape};
use super::*;
use crate::lexer::lex_line;

#[test]
fn recognizes_statement_surfaces() {
    let die = lex_line(
        "After you roll a die, you may pay {1}. If you do, increase or decrease the result by 1. Do this only once each turn.",
        0,
    )
    .unwrap();
    assert!(parse_die_roll_adjustment_tokens(&die).is_some());

    let day = lex_line(
        "If it is neither day nor night as this creature enters, it becomes day.",
        0,
    )
    .unwrap();
    assert!(parse_day_night_enters_tokens(&day).is_some());

    let poison = lex_line("Each opponent gets a poison counter.", 0).unwrap();
    assert_eq!(
        parse_player_gets_counters_surface_tokens(&poison),
        Some(PlayerGetsCountersShape {
            subject: PlayerCounterSubject::EachOpponent,
            count: 1,
            kind: PlayerCounterKind::Poison,
        })
    );

    let compound = lex_line(
        "You draw two cards and you lose 2 life. Each opponent gets a poison counter.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_player_gets_counters_surface_tokens(&compound),
        Some(PlayerGetsCountersShape {
            subject: PlayerCounterSubject::EachOpponent,
            count: 1,
            kind: PlayerCounterKind::Poison,
        })
    );

    let conjoined = lex_line(
        "Each opponent sacrifices a creature or planeswalker of their choice and gets a poison counter.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_player_gets_counters_surface_tokens(&conjoined),
        Some(PlayerGetsCountersShape {
            subject: PlayerCounterSubject::EachOpponent,
            count: 1,
            kind: PlayerCounterKind::Poison,
        })
    );
}

#[test]
fn extra_die_replacement_owns_the_complete_lowest_ignore_instruction() {
    let text = "If you would roll one or more dice, instead roll that many dice plus one and ignore the lowest roll.";
    assert!(is_extra_die_ignore_lowest(&lex_line(text, 0).unwrap()));
    for text in [
        "If you would roll one or more dice, instead roll that many dice plus one.",
        "If you would roll one or more dice, instead roll that many dice plus one and ignore the highest roll.",
        "If you would roll one or more dice, instead roll that many dice plus one and ignore the lowest roll and draw a card.",
        "If an opponent would roll one or more dice, instead roll that many dice plus one and ignore the lowest roll.",
    ] {
        assert!(!is_extra_die_ignore_lowest(&lex_line(text, 0).unwrap()));
    }
}
