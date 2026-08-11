#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn find_exchange_control(effect: &Effect) -> Option<crate::effects::ExchangeControlEffect> {
    if let Some(exchange) = effect.downcast_ref::<crate::effects::ExchangeControlEffect>() {
        return Some(exchange.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_exchange_control(child);
        }
    });
    found
}

#[test]
fn modify_memory_keeps_the_different_controller_target_set_and_neither_reference() {
    let definition = parse_oracle_card_definition("Modify Memory");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Exchange control of two target creatures controlled by different players. If you control neither creature, draw three cards."
        ]
    );

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Modify Memory should have a resolution program");
    let exchange = program
        .flattened_default_effects()
        .iter()
        .find_map(|effect| find_exchange_control(effect))
        .expect("the first sentence should remain an exchange-control effect");
    assert_eq!(exchange.permanent1, exchange.permanent2);
    assert_eq!(exchange.permanent1.count(), crate::ChoiceCount::exactly(2));
    let ChooseSpec::Object(filter) = exchange.permanent1.base() else {
        panic!("the exchange should target one typed creature set: {exchange:#?}");
    };
    assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
    assert!(filter.target_set_different_controllers, "{filter:#?}");
}
