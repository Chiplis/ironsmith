#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn blocked_creature_followups_keep_the_singular_target_identity() {
    for (name, expected) in [
        (
            "Ride Down",
            "Destroy target blocking creature. Creatures that were blocked by that creature this combat gain trample until end of turn.",
        ),
        (
            "Glyph of Doom",
            "Choose target Wall creature. At this turn's next end of combat, destroy all creatures that were blocked by that creature this turn.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            canonical_compiled_lines(&definition),
            vec![expected],
            "{definition:#?}"
        );
    }
}
