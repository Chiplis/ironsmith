#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::oracle_text_by_name;
use super::*;

#[test]
fn undercity_plague_public_payload_keeps_choice_chain_and_cipher_source_line() {
    let name = "Undercity Plague";
    let oracle = oracle_text_by_name()
        .get(name)
        .expect("Undercity Plague should be present in cards.json");
    let parse_input = format!(
        "Mana cost: {{4}}{{B}}{{B}}\nType: Sorcery\nFirst printed set: Gatecrash\n{oracle}"
    );
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .parse_text(parse_input)
        .expect("the authoritative metadata-backed payload should parse");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Undercity Plague should have a spell program");
    let [actions, cipher] = program.segments.as_slice() else {
        panic!("the action line and Cipher must remain distinct: {program:#?}");
    };
    assert!(!actions.starts_new_source_line);
    assert!(cipher.starts_new_source_line);
    let [sequence] = actions.default_effects.as_slice() else {
        panic!("the action line should remain one typed sequence: {actions:#?}");
    };
    let sequence = sequence
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the action line should remain a sequence");
    assert_eq!(sequence.surface, ironsmith_core::SequenceSurface::CommaThen);
    assert_eq!(sequence.effects.len(), 5);
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle.as_str()
    );
}
