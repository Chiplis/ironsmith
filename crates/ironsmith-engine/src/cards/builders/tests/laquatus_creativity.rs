#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn laquatus_creativity_keeps_the_authored_comma_then_connective() {
    let definition = parse_oracle_card_definition("Laquatus's Creativity");
    let sequence = definition
        .spell_effect
        .as_ref()
        .expect("Laquatus's Creativity should have a spell effect")
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::SequenceEffect>())
        .expect("the same-sentence comma-then boundary should survive lowering");

    assert_eq!(sequence.surface, ironsmith_core::SequenceSurface::CommaThen);
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Target player draws cards equal to the number of cards in their hand, then discards that many cards."
    );
}
