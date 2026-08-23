#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const COMPILED: &str = "Menace\nWhen this creature enters, create a Lander token. At the beginning of your next end step, sacrifice it.";

#[test]
fn kav_landseeker_keeps_created_token_provenance_and_next_turn_timing() {
    let definition = parse_oracle_card_definition("Kav Landseeker");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), COMPILED);

    let triggered = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        triggered.len(),
        1,
        "the delayed sentence is not a separate ability"
    );
    let debug = format!("{:#?}", triggered[0].effects);
    assert!(debug.contains("CreateTokenEffect"), "{debug}");
    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("start_next_turn: true"), "{debug}");
    assert!(debug.contains("SacrificeTargetEffect"), "{debug}");
    assert!(debug.matches("created_0").count() >= 2, "{debug}");
}
