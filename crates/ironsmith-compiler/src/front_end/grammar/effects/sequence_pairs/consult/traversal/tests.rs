use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_active_and_passive_consult_traversal_surfaces() {
    let active = parse_consult_traversal_shape(&lex(
        "Exile cards from the top of your library until you exile a nonland card",
    ))
    .unwrap();
    assert_eq!(active.mode, LibraryConsultModeAst::Exile);
    assert_eq!(active.stop.stop_rule, LibraryConsultStopRuleAst::FirstMatch);
    assert!(permission_shapes::exact_tokens(
        &active.stop.filter,
        &["a", "nonland", "card"]
    ));

    let passive = parse_consult_traversal_shape(&lex(
        "Reveal cards from the top of your library until two creature cards are revealed",
    ))
    .unwrap();
    assert_eq!(passive.mode, LibraryConsultModeAst::Reveal);
    assert_eq!(
        passive.stop.stop_rule,
        LibraryConsultStopRuleAst::MatchCount(Value::Fixed(2))
    );
    assert!(permission_shapes::exact_tokens(
        &passive.stop.filter,
        &["creature"]
    ));
}

#[test]
fn repeated_card_filter_union_commas_stay_inside_consult_stop() {
    let parsed = parse_consult_traversal_shape(&lex(
        "Reveal cards from the top of your library until you reveal a Doctor card, a card with doctor's companion, or a Vehicle card",
    ))
    .unwrap();

    assert_eq!(
        TokenWordView::new(&parsed.stop.filter).word_refs(),
        vec![
            "a",
            "doctor",
            "card",
            "a",
            "card",
            "with",
            "doctors",
            "companion",
            "or",
            "a",
            "vehicle",
            "card",
        ]
    );
    assert!(parsed.trailing_effect.is_empty());
}

#[test]
fn captures_a_preceding_effect_before_then() {
    let parsed = parse_consult_traversal_shape(&lex(
        "Target opponent mills a card, then they reveal cards from the top of their library until a land card is revealed",
    ))
    .unwrap();
    assert!(parsed.prefix.is_some());
    assert_eq!(parsed.player, ConsultTraversalPlayerShape::ThatPlayer);
}

#[test]
fn captures_match_count_equal_to_objects_sacrificed_this_way() {
    let parsed = parse_consult_traversal_shape(&lex(
        "Sacrifice X Goblins, then reveal cards from the top of your library until you reveal a number of Goblin creature cards equal to the number of Goblins sacrificed this way",
    ))
    .unwrap();
    let LibraryConsultStopRuleAst::MatchCount(count) = parsed.stop.stop_rule else {
        panic!("expected counted consult stop");
    };
    let Value::PendingPriorEffectMetric(query) = count else {
        panic!("expected typed sacrificed-object metric, got {count:?}");
    };
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Sacrificed)
    );
    assert!(
        query
            .filter
            .as_ref()
            .is_some_and(|filter| { filter.subtypes.contains(&crate::types::Subtype::Goblin) }),
        "{query:#?}"
    );
    assert!(permission_shapes::exact_tokens(
        &parsed.stop.filter,
        &["goblin", "creature", "cards"]
    ));
}

#[test]
fn captures_where_x_value_and_inline_followup() {
    let parsed = parse_consult_traversal_shape(&lex(
        "Reveal cards from the top of your library until you reveal X permanent cards, where X is the number of colors among permanents you control, put any number of those permanent cards onto the battlefield",
    ))
    .unwrap();
    let Some(Value::ColorsAmong(filter)) = parsed.where_x else {
        panic!("expected colors-among binding, got {:?}", parsed.where_x);
    };
    assert_eq!(filter.controller, Some(crate::target::PlayerFilter::You));
    assert_eq!(
        TokenWordView::new(&parsed.trailing_effect).word_refs(),
        vec![
            "put",
            "any",
            "number",
            "of",
            "those",
            "permanent",
            "cards",
            "onto",
            "the",
            "battlefield"
        ]
    );
}

#[test]
fn auspicious_starrix_captures_source_mutation_count() {
    let parsed = parse_consult_traversal_shape(&lex(
        "Exile cards from the top of your library until you exile X permanent cards, where X is the number of times this creature has mutated",
    ))
    .unwrap();
    assert_eq!(parsed.where_x, Some(Value::SourceMutationCount));
    assert_eq!(
        parsed.stop.stop_rule,
        LibraryConsultStopRuleAst::MatchCount(Value::X)
    );
}

#[test]
fn captures_first_match_or_exposed_count_stop() {
    let parsed = parse_consult_traversal_shape(&lex(
        "Target opponent reveals cards from the top of their library until an artifact card or X cards are revealed, whichever comes first",
    ))
    .unwrap();
    assert_eq!(parsed.stop.stop_rule, LibraryConsultStopRuleAst::FirstMatch);
    assert_eq!(parsed.stop.max_exposed, Some(Value::X));
    assert!(permission_shapes::exact_tokens(
        &parsed.stop.filter,
        &["an", "artifact", "card"]
    ));
    assert!(parsed.trailing_effect.is_empty());
}

#[test]
fn does_not_absorb_an_outer_for_each_header_into_the_consult_subject() {
    let tokens = lex(
        "For each creature exiled this way, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then puts the rest on the bottom of their library in a random order.",
    );
    assert!(parse_consult_traversal_shape(&tokens).is_none());

    let each_opponent = lex(
        "Each opponent reveals cards from the top of their library until they reveal X land cards, then puts all cards revealed this way into their graveyard.",
    );
    assert!(parse_consult_traversal_shape(&each_opponent).is_some());
}
