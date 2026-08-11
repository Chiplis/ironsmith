#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn cait_keeps_the_participant_loot_result_partition_and_tied_maximum_gate() {
    let definition = parse_oracle_card_definition("Cait, Cage Brawler");
    let debug = format!("{definition:#?}");
    let rendered = canonical_compiled_lines(&definition);

    assert_eq!(
        rendered.get(1).map(String::as_str),
        Some(
            "Whenever Cait attacks, you and defending player each draw a card, then discard a card. Put two +1/+1 counters on Cait if you discarded the card with the greatest mana value among those cards or tied for greatest."
        ),
        "the shared discard and its exact conditional result must round-trip: {debug}"
    );
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(
        debug.contains("PlayerAffectedObjectHasGreatestManaValue"),
        "{debug}"
    );
    assert!(debug.contains("Defending"), "{debug}");
    assert!(debug.contains("WithIdEffect"), "{debug}");
    assert!(debug.contains("IfEffect"), "{debug}");
}
