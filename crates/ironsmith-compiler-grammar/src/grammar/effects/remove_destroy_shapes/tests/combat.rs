use super::*;

#[test]
pub(super) fn couldnt_attack_exception_stays_inside_the_destroy_filter_domain() {
    let tokens = lex_line(
        "all untapped creatures that didn't attack this turn except for creatures that couldn't attack",
        0,
    )
    .unwrap();
    let DestroyClauseKind::All(DestroyAllShape::Plain { filter_tokens }) =
        parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("attack eligibility must not become a card-type exclusion");
    };
    assert_eq!(
        words(filter_tokens),
        vec![
            "untapped",
            "creatures",
            "that",
            "didnt",
            "attack",
            "this",
            "turn",
            "except",
            "for",
            "creatures",
            "that",
            "couldnt",
            "attack",
        ]
    );
}

#[test]
pub(super) fn parses_combat_history_and_blocked_targets() {
    let tokens = lex_line("target creature that dealt damage to you this turn", 0).unwrap();
    assert!(matches!(
        parse_destroy_clause_shape(&tokens).kind,
        DestroyClauseKind::CombatHistory(
            DestroyCombatHistoryShape::DealtDamageToPlayerThisTurn { .. }
        )
    ));

    let tokens = lex_line("all creatures that dealt damage to you this turn", 0).unwrap();
    assert!(matches!(
        parse_destroy_clause_shape(&tokens).kind,
        DestroyClauseKind::All(DestroyAllShape::DealtDamageToPlayerThisTurn { .. })
    ));

    let tokens = lex_line("target blocked creature", 0).unwrap();
    let DestroyClauseKind::Blocked { target_tokens } = parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("expected blocked target");
    };
    assert_eq!(words(&target_tokens), vec!["target", "blocked", "creature"]);
}
