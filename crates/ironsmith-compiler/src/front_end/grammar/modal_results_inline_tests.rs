use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_modal_result_predicates() {
    for (raw, expected) in [
        ("you do", IfResultPredicate::Did),
        ("you don't discard it this way", IfResultPredicate::DidNot),
        (
            "that creature dies this way",
            IfResultPredicate::DiesThisWay,
        ),
        (
            "result is 3",
            IfResultPredicate::Value(crate::effect::Comparison::Equal(3)),
        ),
        ("that player does", IfResultPredicate::Did),
        ("first player does", IfResultPredicate::Did),
        (
            "they searched their library this way",
            IfResultPredicate::SearchedLibrary,
        ),
        (
            "it connives this way",
            IfResultPredicate::PriorEffectResult(PriorEffectResultSurface::new(
                PriorEffectAction::Connived,
                crate::target::ObjectFilter::default(),
                PriorEffectResultActor::It,
                PriorEffectResultQuantifier::ActionOnly,
            )),
        ),
        (
            "one or more cards are exiled this way",
            IfResultPredicate::Did,
        ),
        (
            "a player is dealt damage this way",
            IfResultPredicate::DealtDamageToPlayer,
        ),
        (
            "excess damage is dealt to the creature an opponent controls this way",
            IfResultPredicate::ExcessDamageDealt,
        ),
        ("you lost the flip", IfResultPredicate::DidNot),
        ("that player doesn't", IfResultPredicate::DidNot),
        ("its power becomes 3 this way", IfResultPredicate::Did),
        (
            "you milled a card this way",
            IfResultPredicate::PriorEffectResult({
                let mut filter = crate::target::ObjectFilter::default();
                filter.set_explicit_card_noun(true);
                PriorEffectResultSurface::new(
                    PriorEffectAction::Milled,
                    filter,
                    PriorEffectResultActor::You,
                    PriorEffectResultQuantifier::One,
                )
            }),
        ),
        (
            "you reveal a nonland card this way",
            IfResultPredicate::PriorEffectResult({
                let mut filter = crate::target::ObjectFilter::default();
                filter.excluded_card_types = vec![crate::types::CardType::Land];
                filter.set_explicit_card_noun(true);
                PriorEffectResultSurface::new(
                    PriorEffectAction::Revealed,
                    filter,
                    PriorEffectResultActor::You,
                    PriorEffectResultQuantifier::One,
                )
            }),
        ),
        (
            "no counters were removed this way",
            IfResultPredicate::DidNot,
        ),
    ] {
        let tokens = lex_line(raw, 0).unwrap();
        let actual = parse_if_result_predicate_lexed_tokens(&tokens);
        assert_eq!(actual, Some(expected));
    }
}

#[test]
fn positive_counter_removal_result_remains_typed_and_positive() {
    let tokens = lex_line("one or more counters were removed this way", 0).unwrap();
    let Some(IfResultPredicate::PriorEffectResult(surface)) =
        parse_if_result_predicate_lexed_tokens(&tokens)
    else {
        panic!("expected a positive typed counter-removal result");
    };
    assert_eq!(surface.action, PriorEffectAction::Removed);
    assert_eq!(surface.actor, PriorEffectResultActor::Passive);
    assert_eq!(surface.quantifier, PriorEffectResultQuantifier::ActionOnly);
}

#[test]
fn typed_prior_result_preserves_chosen_name_comparison() {
    let tokens = lex_line("a card with the chosen name was milled this way", 0).unwrap();
    let Some(IfResultPredicate::PriorEffectResult(surface)) =
        parse_if_result_predicate_lexed_tokens(&tokens)
    else {
        panic!("expected typed prior-effect result");
    };

    assert_eq!(surface.action, PriorEffectAction::Milled);
    assert!(surface.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "__chosen_name__"
            && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    }));
}

#[test]
fn typed_prior_result_preserves_active_counted_sacrifice_surface() {
    let tokens = lex_line("you sacrifice one or more artifacts this way", 0).unwrap();
    let Some(IfResultPredicate::PriorEffectResult(surface)) =
        parse_if_result_predicate_lexed_tokens(&tokens)
    else {
        panic!("expected typed prior-effect result");
    };

    assert_eq!(surface.action, PriorEffectAction::Sacrificed);
    assert_eq!(surface.actor, PriorEffectResultActor::You);
    assert_eq!(surface.quantifier, PriorEffectResultQuantifier::OneOrMore);
    assert_eq!(
        surface.filter.card_types,
        vec![crate::types::CardType::Artifact]
    );
}

#[test]
fn destination_qualified_return_is_a_typed_prior_result() {
    let tokens = lex_line("that card is returned to its owner's hand this way", 0).unwrap();
    let Some(IfResultPredicate::PriorEffectResult(surface)) =
        parse_if_result_predicate_lexed_tokens(&tokens)
    else {
        panic!("expected a typed return-to-hand result");
    };

    assert_eq!(surface.action, PriorEffectAction::Returned);
    assert_eq!(surface.actor, PriorEffectResultActor::Passive);
    assert_eq!(surface.quantifier, PriorEffectResultQuantifier::One);
    assert_eq!(
        surface.filter.demonstrative_antecedent_surface(),
        Some(ironsmith_core::DemonstrativeAntecedentSurface::Card)
    );
}

#[test]
fn present_zone_state_is_not_a_prior_return_result() {
    let tokens = lex_line("that card is in its owner's hand", 0).unwrap();
    assert!(!matches!(
        parse_if_result_predicate_lexed_tokens(&tokens),
        Some(IfResultPredicate::PriorEffectResult(surface))
            if surface.action == PriorEffectAction::Returned
    ));
}

#[test]
fn typed_prior_result_preserves_independently_qualified_or_arms() {
    let tokens = lex_line(
        "a permanent you controlled or a token was destroyed this way",
        0,
    )
    .unwrap();
    let Some(IfResultPredicate::PriorEffectResult(surface)) =
        parse_if_result_predicate_lexed_tokens(&tokens)
    else {
        panic!("expected typed prior-effect result");
    };

    assert_eq!(surface.action, PriorEffectAction::Destroyed);
    assert_eq!(surface.filter.any_of.len(), 2);
    assert_eq!(
        surface.filter.any_of[0].controller,
        Some(crate::target::PlayerFilter::You)
    );
    assert!(surface.filter.any_of[1].token);
    assert!(surface.filter.has_explicit_union_branch_articles());
}

#[test]
fn typed_prior_result_preserves_counted_shared_color_set_condition() {
    let tokens = lex_line(
        "two nonland cards that share a color were milled this way",
        0,
    )
    .unwrap();
    let Some(IfResultPredicate::PriorEffectResult(surface)) =
        parse_if_result_predicate_lexed_tokens(&tokens)
    else {
        panic!("expected typed counted prior-effect result");
    };

    assert_eq!(surface.action, PriorEffectAction::Milled);
    assert_eq!(surface.required_count, Some(2));
    assert_eq!(
        surface.shared_characteristic,
        Some(ObjectCharacteristic::Color)
    );
    assert!(
        surface
            .filter
            .excluded_card_types
            .contains(&crate::types::CardType::Land)
    );
}
