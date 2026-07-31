#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn crag_saurian_binds_control_to_the_damage_source_controller() {
    let definition = parse_oracle_card_definition("Crag Saurian");
    let debug = format!("{definition:#?}");
    let lines = unprocessed_compiled_lines(&definition);

    assert!(
        debug.contains("TagTriggeringSourceEffect"),
        "the damage source must be snapshotted for its controller relation: {debug}"
    );
    assert!(
        debug.contains("ControllerOf") && debug.contains("triggering_source"),
        "the controller change must reference the triggering damage source: {debug}"
    );
    assert!(
        lines.iter().any(|line| {
            line
                == "Whenever a source deals damage to this creature, that source's controller gains control of this creature."
        }),
        "Crag Saurian should retain the exact relational control surface; got {lines:#?}"
    );
}
