use super::*;

#[test]
pub(super) fn parses_destroy_unless_target_color_sets_differ() {
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
pub(super) fn parses_target_and_demonstrative_attached_object_set_as_one_destroy_shape() {
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
pub(super) fn does_not_treat_unrelated_coordinated_destroy_subjects_as_attached_to_one_target() {
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
pub(super) fn parses_inline_same_object_no_regeneration_rider() {
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
