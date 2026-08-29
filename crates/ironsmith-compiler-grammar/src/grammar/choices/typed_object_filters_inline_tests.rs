use super::super::super::super::lexer::lex_line;
use super::*;
use crate::types::CardType;

#[test]
fn typed_choice_object_clause_returns_filter_and_reference_facts() {
    let tokens = lex_line("You choose a card from it.", 0).unwrap();
    let TypedChoiceObjectClauseKind::Object(parsed) =
        parse_typed_choice_object_clause_tokens(&tokens)
            .unwrap()
            .unwrap()
    else {
        panic!("expected object choice");
    };
    assert!(parsed.references.references_it);
    assert_eq!(parsed.filter.zone, Some(Zone::Hand));
    assert!(
        parsed
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
    );
}

#[test]
fn typed_choice_preserves_greatest_value_domain_and_implicit_actor() {
    let tokens = lex_line(
        "Choose a creature with the greatest mana value among creatures they control.",
        0,
    )
    .unwrap();
    let TypedChoiceObjectClauseKind::Object(parsed) =
        parse_typed_choice_object_clause_tokens(&tokens)
            .unwrap()
            .unwrap()
    else {
        panic!("expected object choice");
    };

    assert_eq!(parsed.actor, ChoiceClauseActor::Implicit);
    assert_eq!(
        parsed.filter.controller,
        Some(crate::target::PlayerFilter::IteratedPlayer)
    );
    assert!(matches!(
        parsed.filter.mana_value,
        Some(Comparison::EqualExpr(value))
            if matches!(value.as_ref(), Value::GreatestManaValue(scope)
                if scope.controller == Some(crate::target::PlayerFilter::IteratedPlayer))
    ));
}

#[test]
fn typed_target_and_sequence_choice_filters_are_owned_by_grammar() {
    let target = lex_line("Target opponent chooses a creature.", 0).unwrap();
    let parsed = parse_typed_target_player_choice_tokens(&target)
        .unwrap()
        .unwrap();
    assert_eq!(parsed.filter.card_types, [CardType::Creature]);

    let block = lex_line("Other creatures can't block this turn.", 0).unwrap();
    let parsed = parse_typed_chosen_cant_block_tokens(&block)
        .unwrap()
        .unwrap();
    assert!(parsed.exclude_tagged_choice);
    assert_eq!(parsed.filter.card_types, [CardType::Creature]);
}

#[test]
fn typed_become_sequence_returns_an_object_filter() {
    let first = lex_line("Choose a creature type.", 0).unwrap();
    let second = lex_line("All creatures become that type.", 0).unwrap();
    let parsed = parse_typed_choice_become_shape(&first, &second)
        .unwrap()
        .unwrap();
    let TypedChoiceBecomeSubject::AllObjects(filter) = parsed.subject else {
        panic!("expected an all-objects subject");
    };
    assert_eq!(filter.card_types, [CardType::Creature]);
}

#[test]
fn typed_choice_filter_rejects_an_effect_clause() {
    let tokens = lex_line("You choose a creature and sacrifice it.", 0).unwrap();
    assert!(
        parse_typed_choice_object_clause_tokens(&tokens)
            .unwrap()
            .is_none()
    );
}

#[test]
fn typed_choice_filter_excludes_the_accumulated_chosen_set() {
    let tokens = lex_line(
        "Choose a nonland permanent they don't control that hasn't been chosen this way.",
        0,
    )
    .unwrap();
    let TypedChoiceObjectClauseKind::Object(parsed) =
        parse_typed_choice_object_clause_tokens(&tokens)
            .unwrap()
            .unwrap()
    else {
        panic!("expected object choice");
    };

    assert!(parsed.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == CHOSEN_OBJECTS_TAG
            && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
    }));
}
