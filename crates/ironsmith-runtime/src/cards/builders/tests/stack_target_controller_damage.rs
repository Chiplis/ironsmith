#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn target_spell_controller_damage_keeps_one_shared_stack_target() {
    let expected = "deals damage to target spell's controller equal to that spell's mana value";

    for card_name in ["Parallectric Feedback", "Refuse // Cooperate"] {
        let definition = parse_oracle_card_definition(card_name);
        let compiled = canonical_compiled_lines(&definition).join("\n");
        assert!(
            compiled.to_ascii_lowercase().contains(expected),
            "{card_name}: {compiled}"
        );

        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("TargetOnlyEffect")
                && debug.contains("stack_kind: Some(Spell)")
                && debug.contains("ManaValueOf")
                && debug.contains("ControllerOf"),
            "{card_name} must target the spell once and reuse it for both controller and mana value: {debug}"
        );
    }
}
