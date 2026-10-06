use super::*;

#[test]
pub(super) fn dealer_and_recipient_history_are_different_complete_shapes() {
    let dealer = lex_line("target creature an opponent controls that dealt damage this turn", 0).unwrap();
    let DestroyClauseKind::CombatHistory(DestroyCombatHistoryShape::DealerThisTurn { target_tokens }) =
        parse_destroy_clause_shape(&dealer).kind
    else { panic!("active damage dealer was not retained"); };
    assert_eq!(words(target_tokens), vec!["target", "creature", "an", "opponent", "controls"]);
    let recipient = lex_line("target creature that was dealt damage this turn", 0).unwrap();
    assert!(matches!(parse_destroy_clause_shape(&recipient).kind,
        DestroyClauseKind::CombatHistory(DestroyCombatHistoryShape::DealtDamageThisTurn { .. })));
    let trailing = lex_line("target creature that dealt damage this turn while a puzzle was solved", 0).unwrap();
    assert!(matches!(parse_destroy_clause_shape(&trailing).kind,
        DestroyClauseKind::UnsupportedCombatHistory));
}

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

#[test]
pub(super) fn complete_block_history_filters_reach_the_shared_reader() {
    let all = lex_line("each creature that blocked or was blocked this turn", 0).unwrap();
    assert!(matches!(parse_destroy_clause_shape(&all).kind,
        DestroyClauseKind::All(DestroyAllShape::Plain { .. })));
    let target = lex_line("target creature that blocked or was blocked by a legendary creature this turn", 0).unwrap();
    assert!(matches!(parse_destroy_clause_shape(&target).kind,
        DestroyClauseKind::CombatHistory(DestroyCombatHistoryShape::BlockHistoryFilter { .. })));
    for text in ["each creature that blocked or was blocked this turn while a puzzle was solved",
        "target creature that blocked or was blocked by a legendary creature this turn while a puzzle was solved"] {
        let bad = lex_line(text, 0).unwrap();
        assert!(matches!(parse_destroy_clause_shape(&bad).kind, DestroyClauseKind::UnsupportedCombatHistory));
    }
}
