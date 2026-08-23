#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

#[test]
fn veteran_ice_climber_keeps_the_optional_player_target_and_source_power() {
    let name = "Veteran Ice Climber";
    let oracle = &oracle_text_by_name()[name];
    let definition = parse_oracle_card_definition(name);
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::ThisAttacksTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Veteran Ice Climber must keep its attack trigger");
    let effects = triggered.effects.flattened_default_effects();
    let [target, mill] = effects else {
        panic!(
            "expected one linked target and mill effect: {:#?}",
            triggered.effects
        );
    };
    let target = target
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .expect("the mill recipient must be a target");
    assert_eq!(
        target.target,
        ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any))
            .with_count(crate::effect::ChoiceCount::up_to(1))
    );
    let mill = mill
        .downcast_ref::<crate::effects::MillEffect>()
        .expect("the selected player must mill");
    assert_eq!(
        mill.player,
        PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any))
    );
    assert!(matches!(
        mill.count.unhinted(),
        Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Source)
    ));

    let compiled = unprocessed_compiled_lines(&definition);
    assert_eq!(
        compiled,
        [
            "Vigilance",
            "This creature can't be blocked.",
            "Whenever this creature attacks, up to one target player mills cards equal to this creature's power.",
        ]
        .map(str::to_string),
        "the standard mill reminder is removed before compilation"
    );
    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        name,
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "{name} must clear the strict semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}
