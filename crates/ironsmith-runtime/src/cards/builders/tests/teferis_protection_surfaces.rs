#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn teferis_protection_renders_its_duration_protection_and_phasing_exactly() {
    let oracle = "Until your next turn, your life total can't change and you gain protection from everything. All permanents you control phase out.\nExile Teferi's Protection.";
    let definition = parse_oracle_card_definition("Teferi's Protection");
    let compiled = canonical_compiled_lines(&definition).join("\n");
    let debug = format!("{definition:#?}");

    assert_eq!(compiled, oracle, "{debug}");
    assert!(
        debug.contains("ChangeLifeTotal")
            && debug.contains("BeTargetedPlayer")
            && debug.contains("PreventAllDamageToTargetEffect")
            && debug.contains("PhaseOutEffect")
            && debug.contains("ExileEffect")
            && debug.matches("YourNextTurn").count() >= 3,
        "{debug}"
    );
}
