#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn tidewalker_keeps_dynamic_entry_count_bare_vanishing_and_self_pronoun() {
    let definition = parse_oracle_card_definition("Tidewalker");

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "This creature enters with a time counter on it for each Island you control.\n\
         Vanishing\n\
         Tidewalker's power and toughness are each equal to the number of time counters on it."
    );

    let enter_with_counter_abilities = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::EnterWithCounters =>
            {
                Some(static_ability)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        enter_with_counter_abilities.len(),
        1,
        "bare Vanishing must not add a second fixed entry-counter ability"
    );
    let model = enter_with_counter_abilities[0]
        .compiled_model()
        .expect("dynamic entry counter ability should retain its compiled model");
    let ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { counter, count } =
        &model.payload
    else {
        panic!("expected typed variable entry-counter payload, got {model:#?}");
    };
    assert_eq!(*counter, crate::CounterType::Time);
    assert!(matches!(
        count.unhinted(),
        Value::Count(filter)
            if filter.subtypes == vec![Subtype::Island]
                && filter.controller == Some(PlayerFilter::You)
    ));
}
