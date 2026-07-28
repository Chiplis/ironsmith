#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn reusable_comma_then_paths_keep_the_authored_sequence_surface() {
    let mut failures = Vec::new();
    for name in [
        "Alrund, God of the Cosmos // Hakka, Whispering Raven",
        "Shorecrasher Elemental",
        "Doom Foretold",
        "Valakut Exploration",
        "The Neutrinos",
        "Ace, Fearless Rebel",
        "Jadelight Ranger",
    ] {
        let definition = parse_oracle_card_definition(name);
        let rendered = canonical_compiled_lines(&definition).join("\n");
        if !rendered.to_ascii_lowercase().contains(", then ") {
            failures.push(format!("{name} lost its comma-then surface:\n{rendered}"));
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
