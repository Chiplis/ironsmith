use super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn parses_attached_prevention_shapes() {
    let tokens = lex_line(
        "If damage would be dealt to this creature, prevent that damage and remove a shield counter from it.",
        0,
    )
    .unwrap();
    let parsed = primitives::parse_all(
        &tokens,
        parse_remove_counter_prevention_lexed,
        "remove-counter prevention test",
    )
    .unwrap();
    assert_eq!(parsed.amount, RemoveCounterPreventionAmount::Fixed(1));
    assert_eq!(parsed.condition_tokens, None);
    assert_eq!(parsed.follow_up, None);

    let tokens = lex_line(
        "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage and remove that many +1/+1 counters from it.",
        0,
    )
    .unwrap();
    let parsed = primitives::parse_all(
        &tokens,
        parse_remove_counter_prevention_lexed,
        "conditional remove-counter prevention test",
    )
    .unwrap();
    assert_eq!(parsed.amount, RemoveCounterPreventionAmount::DamageAmount);
    assert_eq!(parsed.counter_type, CounterType::PlusOnePlusOne);
    assert_eq!(
        parser_token_word_refs(parsed.condition_tokens.unwrap()),
        vec!["it", "has", "a", "+1/+1", "counter", "on", "it"]
    );

    let tokens = lex_line(
        "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage, remove that many +1/+1 counters from it, then give each player a rad counter for each +1/+1 counter removed this way.",
        0,
    )
    .unwrap();
    let parsed = primitives::parse_all(
        &tokens,
        parse_remove_counter_prevention_lexed,
        "remove-counter prevention follow-up test",
    )
    .unwrap();
    assert_eq!(
        parsed.follow_up,
        Some(RemoveCounterPreventionFollowUp {
            counter_type: CounterType::Rad,
            counters_per_removed: 1,
        })
    );
}
