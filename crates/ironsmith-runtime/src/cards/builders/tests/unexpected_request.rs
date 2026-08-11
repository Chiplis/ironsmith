#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn. You may attach an Equipment you control to that creature. If you do, unattach it at the beginning of the next end step.";

#[test]
fn unexpected_request_keeps_the_result_gated_delayed_unattach() {
    let definition = parse_oracle_card_definition("Unexpected Request");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Unexpected Request should have a spell program");
    let debug = format!("{program:#?}");
    assert!(debug.contains("MayEffect"), "{debug}");
    assert!(debug.contains("IfEffect"), "{debug}");
    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("UnattachObjectsEffect"), "{debug}");
}
