#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn waylay_preserves_group_reference_and_the_next_cleanup_timing() {
    let definition = parse_oracle_card_definition("Waylay");
    let lines = unprocessed_compiled_lines(&definition);

    assert_eq!(
        lines,
        [
            "Create three 2/2 white Knight creature tokens. Exile them at the beginning of the next cleanup step."
        ]
    );

    let debug = format!("{:#?}", definition.abilities);
    for expected in [
        "BeginningOfCleanupStepTrigger",
        "next: true",
        "Tagged(TagKey(\"created_0\"))",
        "target_plural_surface: true",
        "one_shot: true",
    ] {
        assert!(
            debug.contains(expected),
            "Waylay should retain {expected:?} in its lowered definition; got {debug}"
        );
    }
}
