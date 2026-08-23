#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "When this creature enters, create a Food token.\nWhenever you gain life for the first time during each of your turns, create a 1/1 white Cat creature token.";

#[test]
fn cat_collector_keeps_the_turn_scoped_first_life_gain_gate() {
    let definition = parse_oracle_card_definition("Cat Collector");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let triggered = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .find(|triggered| {
            triggered
                .trigger
                .downcast_ref::<crate::triggers::YouGainLifeTrigger>()
                .is_some()
        })
        .expect("Cat Collector should retain its life-gain trigger");
    let gain_life = triggered
        .trigger
        .downcast_ref::<crate::triggers::YouGainLifeTrigger>()
        .expect("the trigger should be typed");
    assert_eq!(gain_life.during_turn, Some(PlayerFilter::You));
    assert_eq!(
        triggered.intervening_if,
        Some(crate::ConditionExpr::FirstTimeThisTurn)
    );
}
