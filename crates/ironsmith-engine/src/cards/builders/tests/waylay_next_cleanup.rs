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

    // Waylay is an instant, so resolving the spell schedules the delayed
    // trigger. It is not a printed ability stored in `definition.abilities`.
    let debug = format!("{:#?}", definition.spell_effect);
    let compact_debug = debug.split_whitespace().collect::<String>();
    for expected in [
        "BeginningOfCleanupStepTrigger",
        "next: true",
        "TaggedEffect",
        "TagKey(\"created_0\"",
        "target_plural_surface: true",
        "one_shot: true",
    ] {
        let compact_expected = expected.split_whitespace().collect::<String>();
        assert!(
            compact_debug.contains(&compact_expected),
            "Waylay should retain {expected:?} in its lowered definition; got {debug}"
        );
    }
}
