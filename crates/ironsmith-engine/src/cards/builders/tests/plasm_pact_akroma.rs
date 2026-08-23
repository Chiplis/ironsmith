#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const PLASM_ORACLE: &str = "Counter target spell. At the beginning of your next first main phase, add X mana in any combination of colors, where X is that spell's mana value.";
const PACT_ORACLE: &str = "As long as this Equipment is attached to a creature, you don't lose the game for having 0 or less life.\nWhenever equipped creature attacks, draw a card and reveal it. The creature gets +X/+X until end of turn and you lose X life, where X is that card's mana value.\nEquip—Discard a card.";
const AKROMA_ORACLE: &str = "Flying, first strike, vigilance, trample\nAt the beginning of each combat, until end of turn, each other creature you control gets +1/+1 if it has flying, +1/+1 if it has first strike, and so on for double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, vigilance, and partner.\nPartner";

#[test]
fn plasm_capture_registers_one_shot_precombat_main_phase_value_schedule() {
    let definition = parse_oracle_card_definition("Plasm Capture");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        PLASM_ORACLE
    );

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Plasm spell effects");
    let debug = format!("{program:#?}");
    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("BeginningOfMainPhaseTrigger"), "{debug}");
    assert!(debug.contains("phase_type: Precombat"), "{debug}");
    assert!(debug.contains("one_shot: true"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("CounterEffect"), "{debug}");
}

#[test]
fn pact_weapon_keeps_drawn_card_and_attacking_creature_as_distinct_references() {
    let definition = parse_oracle_card_definition("Pact Weapon");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        PACT_ORACLE
    );

    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("AttachedToSourceMatches"), "{debug}");
    assert!(debug.contains("__drawn_revealed_card__"), "{debug}");
    assert!(debug.contains("TagTriggeringObjectEffect"), "{debug}");
    assert!(debug.contains("RevealTaggedEffect"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("DiscardEffect"), "{debug}");
}

#[test]
fn akroma_factors_only_the_complete_typed_keyword_pt_ladder() {
    let definition = parse_oracle_card_definition("Akroma, Vision of Ixidor");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        AKROMA_ORACLE
    );

    let debug = format!("{:#?}", definition.abilities);
    assert!(
        debug.matches("ModifyPowerToughness").count() >= 14,
        "{debug}"
    );
    for keyword in ["Flying", "Protection", "Partner"] {
        assert!(debug.contains(keyword), "missing {keyword}: {debug}");
    }
}
