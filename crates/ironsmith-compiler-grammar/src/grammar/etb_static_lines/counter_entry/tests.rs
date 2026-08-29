use super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn parses_counter_entry_heads_and_conditions() {
    let tokens = lex_line("This creature enters tapped with two +1/+1 counters.", 0).unwrap();
    let spec = parse_enters_tapped_with_counters_clause_tokens(&tokens).unwrap();
    assert!(tokens_contain_word(spec.entry_modifier_tokens, "tapped"));

    let tokens = lex_line(
        "This creature enters with a +1/+1 counter if you've attacked this turn.",
        0,
    )
    .unwrap();
    let head = parse_enters_with_counters_clause_tokens(&tokens).unwrap();
    let (condition_index, _, _) =
        primitives::find_prefix(head.counter_clause_tokens, || primitives::kw("if")).unwrap();
    let tail = parse_enters_with_counter_condition_tail_tokens(
        &head.counter_clause_tokens[condition_index..],
    )
    .unwrap();
    assert_eq!(tail.kind, EntersWithCounterConditionTailKind::If);
    assert_eq!(
        parse_enters_with_counter_condition_shape_tokens(tail.condition_tokens),
        Some(EntersWithCounterConditionShape::AttackedThisTurn)
    );
}

#[test]
fn parses_counter_choices_and_dynamic_tails() {
    let tokens = lex_line(
        "your choice of a flying counter or a vigilance counter on it",
        0,
    )
    .unwrap();
    let choice = parse_enters_with_counter_choice_tokens(&tokens).unwrap();
    assert_eq!(
        choice.counter_types,
        vec![CounterType::Flying, CounterType::Vigilance]
    );

    let tokens = lex_line("for each creature that died this turn", 0).unwrap();
    assert_eq!(
        parse_enters_with_counter_known_for_each_tail_tokens(&tokens),
        Some(EntersWithCounterKnownForEachKind::CreaturesDiedThisTurn)
    );

    let tokens = lex_line("for each loyalty counter on planeswalkers you control", 0).unwrap();
    assert_eq!(
        parse_enters_with_counter_known_for_each_tail_tokens(&tokens),
        Some(EntersWithCounterKnownForEachKind::LoyaltyCountersOnPlaneswalkersYouControl)
    );
}

#[test]
fn parses_quoted_counter_entry_ability_tail() {
    let tokens = lex_line(
        "and with \"This creature can attack as though it didn't have defender.\"",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_enters_with_added_abilities_tail_tokens(&tokens),
        Some(EntersWithAddedAbilitiesTail::CanAttackAsThoughNoDefender)
    );
}

#[test]
fn parses_dual_for_each_counter_entry_to_typed_value() {
    let tokens = lex_line(
        "This creature enters with a +1/+1 counter on it for each other red creature you control and a +1/+1 counter on it for each other green creature you control.",
        0,
    )
    .unwrap();
    let shape = parse_enters_with_dual_for_each_counter_tokens(&tokens).unwrap();
    assert_eq!(shape.counter_type, CounterType::PlusOnePlusOne);
    let Value::Add(first, second) = shape.count else {
        panic!("expected additive typed count");
    };
    let (Value::Count(first), Value::Count(second)) = (*first, *second) else {
        panic!("expected two matching-filter counts");
    };
    assert_eq!(first.colors, Some(crate::color::ColorSet::RED));
    assert_eq!(second.colors, Some(crate::color::ColorSet::GREEN));
    assert!(first.other && second.other);
}
