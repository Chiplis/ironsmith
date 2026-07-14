use super::*;
use crate::runtime_backend::lexer::{TokenWordView, lex_line};

fn words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    TokenWordView::new(tokens).to_word_refs()
}

#[test]
fn parses_remove_counter_and_combat_shapes() {
    let tokens = lex_line("two charge counters from target artifact", 0).unwrap();
    let shape = parse_remove_clause_shape(&tokens).unwrap();
    let RemoveClauseShape::Counters {
        amount,
        counter_descriptor,
        destination,
        ..
    } = shape
    else {
        panic!("expected counter removal");
    };
    assert_eq!(amount, Value::Fixed(2));
    assert_eq!(words(counter_descriptor), vec!["charge"]);
    let RemoveCounterDestination::Single { target_tokens } = destination else {
        panic!("expected single target");
    };
    assert_eq!(words(target_tokens), vec!["target", "artifact"]);

    let all = lex_line("all counters from that creature", 0).unwrap();
    assert!(matches!(
        parse_remove_clause_shape(&all),
        Ok(RemoveClauseShape::AllCounters {
            counter_descriptor,
            ..
        }) if counter_descriptor.is_empty()
    ));

    let tokens = lex_line("target creature from combat", 0).unwrap();
    assert!(matches!(
        parse_remove_clause_shape(&tokens),
        Ok(RemoveClauseShape::FromCombat { .. })
    ));
}

#[test]
fn parses_destroy_all_and_delayed_shapes() {
    let tokens = lex_line(
        "all creatures except for artifacts at the beginning of the next end step",
        0,
    )
    .unwrap();
    let shape = parse_destroy_clause_shape(&tokens);
    assert_eq!(shape.timing, Some(DelayedDestroyTimingShape::NextEndStep));
    let DestroyClauseKind::All(DestroyAllShape::ExceptFor {
        filter_tokens,
        exception_tokens,
    }) = shape.kind
    else {
        panic!("expected destroy-all exception");
    };
    assert_eq!(words(filter_tokens), vec!["creatures"]);
    assert_eq!(words(exception_tokens), vec!["artifacts"]);
}

#[test]
fn parses_combat_history_and_blocked_targets() {
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
    assert_eq!(words(&target_tokens), vec!["target", "creature"]);
}

#[test]
fn parses_destroy_unless_target_color_sets_differ() {
    let tokens = lex_line(
        "two target nonblack creatures unless either one is a color the other isn't",
        0,
    )
    .unwrap();
    let DestroyClauseKind::UnlessTargetSetPredicate {
        target_tokens,
        predicate,
    } = parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("expected typed target-set destroy condition");
    };
    assert_eq!(
        words(target_tokens),
        vec!["two", "target", "nonblack", "creatures"]
    );
    assert_eq!(
        predicate,
        conditions::TargetSetPredicateAst::DifferentColorSets
    );
}

#[test]
fn parses_not_chosen_this_way_as_the_complement_set() {
    let tokens = lex_line("each creature not chosen this way", 0).unwrap();
    let DestroyClauseKind::All(DestroyAllShape::ChosenThisWay {
        filter_tokens,
        relation,
    }) = parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("expected tagged destroy-all complement");
    };

    assert_eq!(words(filter_tokens), vec!["creature"]);
    assert_eq!(relation, TaggedDestroyRelation::ExceptMatching);
}
