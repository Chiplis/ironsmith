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
fn parses_counter_distribution_from_among_all_permanents() {
    let tokens = lex_line("up to three stun counters from among all permanents", 0).unwrap();
    let RemoveClauseShape::Counters {
        amount,
        up_to,
        counter_descriptor,
        destination,
    } = parse_remove_clause_shape(&tokens).unwrap()
    else {
        panic!("expected counter removal");
    };
    assert_eq!(amount, Value::Fixed(3));
    assert!(up_to);
    assert_eq!(words(counter_descriptor), vec!["stun"]);
    let RemoveCounterDestination::Among { filter_tokens } = destination else {
        panic!("expected distributed among destination");
    };
    assert_eq!(words(filter_tokens), vec!["permanents"]);
}

#[test]
fn parses_number_of_counters_equal_to_referenced_card_mana_value() {
    let tokens = lex_line(
        "a number of loyalty counters equal to that card's mana value from Jace",
        0,
    )
    .unwrap();
    let RemoveClauseShape::Counters {
        amount,
        counter_descriptor,
        destination,
        ..
    } = parse_remove_clause_shape(&tokens).unwrap()
    else {
        panic!("expected dynamic counter removal");
    };
    assert!(
        matches!(amount.unhinted(), Value::ManaValueOf(_)),
        "{amount:?}"
    );
    assert!(amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo));
    assert_eq!(words(counter_descriptor), vec!["loyalty"]);
    let RemoveCounterDestination::Single { target_tokens } = destination else {
        panic!("expected source-named single destination");
    };
    assert_eq!(words(target_tokens), vec!["jace"]);
}

#[test]
fn parses_each_of_any_number_as_an_optional_unbounded_subset() {
    let tokens = lex_line(
        "a loyalty counter from each of any number of permanents you control",
        0,
    )
    .unwrap();
    let RemoveClauseShape::Counters { destination, .. } =
        parse_remove_clause_shape(&tokens).unwrap()
    else {
        panic!("expected counter removal");
    };
    let RemoveCounterDestination::EachOfAnyNumber { filter_tokens } = destination else {
        panic!("expected an any-number subset, not an all-permanents destination");
    };
    assert_eq!(words(filter_tokens), vec!["permanents", "you", "control"]);
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
fn couldnt_attack_exception_stays_inside_the_destroy_filter_domain() {
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
            "didn't",
            "attack",
            "this",
            "turn",
            "except",
            "for",
            "creatures",
            "that",
            "couldn't",
            "attack",
        ]
    );
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
    assert_eq!(words(&target_tokens), vec!["target", "blocked", "creature"]);
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

#[test]
fn parses_not_chosen_by_any_player_as_the_complement_set() {
    let tokens = lex_line("all Plains that weren't chosen this way by any player", 0).unwrap();
    let DestroyClauseKind::All(DestroyAllShape::ChosenThisWay {
        filter_tokens,
        relation,
    }) = parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("expected tagged destroy-all complement");
    };

    assert_eq!(words(filter_tokens), vec!["plains"]);
    assert_eq!(relation, TaggedDestroyRelation::ExceptMatching);
}

#[test]
fn chosen_this_way_type_qualifier_remains_an_object_filter() {
    let tokens = lex_line("all creatures that aren't of a type chosen this way", 0).unwrap();
    let DestroyClauseKind::All(DestroyAllShape::Plain { filter_tokens }) =
        parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("chosen creature-type qualifier must not become an object-result tag");
    };

    assert_eq!(
        words(filter_tokens),
        vec![
            "creatures",
            "that",
            "arent",
            "of",
            "a",
            "type",
            "chosen",
            "this",
            "way"
        ]
    );
}

#[test]
fn parses_target_and_demonstrative_attached_object_set_as_one_destroy_shape() {
    let tokens = lex_line(
        "target creature with flying and all Equipment attached to that creature",
        0,
    )
    .unwrap();
    let DestroyClauseKind::TargetAndAttached(shape) = parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("expected a target plus its attached-object set");
    };

    assert_eq!(
        words(shape.target_tokens),
        vec!["target", "creature", "with", "flying"]
    );
    assert_eq!(words(shape.attachment_filter_tokens), vec!["equipment"]);
    assert_eq!(
        shape.demonstrative_antecedent,
        Some(ironsmith_core::DemonstrativeAntecedentSurface::Creature)
    );
}

#[test]
fn does_not_treat_unrelated_coordinated_destroy_subjects_as_attached_to_one_target() {
    let tokens = lex_line(
        "target creature and all Equipment attached to another creature",
        0,
    )
    .unwrap();

    assert!(!matches!(
        parse_destroy_clause_shape(&tokens).kind,
        DestroyClauseKind::TargetAndAttached(_)
    ));
}

#[test]
fn parses_inline_same_object_no_regeneration_rider() {
    let tokens = lex_line("target Knight and it can't be regenerated", 0).unwrap();
    let DestroyClauseKind::InlineNoRegeneration { target_tokens } =
        parse_destroy_clause_shape(&tokens).kind
    else {
        panic!("expected inline no-regeneration destroy shape");
    };
    assert_eq!(words(target_tokens), vec!["target", "knight"]);

    let near_miss = lex_line("target Knight and draw a card", 0).unwrap();
    assert!(!matches!(
        parse_destroy_clause_shape(&near_miss).kind,
        DestroyClauseKind::InlineNoRegeneration { .. }
    ));
}
