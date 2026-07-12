use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

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
        parse_player_gets_counters_tokens(&poison),
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
