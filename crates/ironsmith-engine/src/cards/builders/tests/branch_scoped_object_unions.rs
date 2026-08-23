#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn assert_compiled_line(card_name: &str, expected: &str) {
    let definition = parse_oracle_card_definition(card_name);
    let lines = canonical_compiled_lines(&definition);
    assert!(
        lines.iter().any(|line| line == expected),
        "{card_name} should retain the exact branch-scoped union line; got {lines:#?}"
    );
}

#[test]
fn kickoff_celebrations_keeps_creatures_and_vehicles_as_a_union() {
    assert_compiled_line(
        "Kickoff Celebrations",
        "Max speed — Sacrifice this enchantment: Creatures and Vehicles you control gain haste until end of turn.",
    );
}

#[test]
fn hemlock_vial_keeps_equipped_creatures_separate_from_equipment() {
    assert_compiled_line(
        "Hemlock Vial",
        "{B}, {T}, Sacrifice this artifact: Each equipped creature and Equipment you control gains deathtouch until end of turn.",
    );
}

#[test]
fn death_tyrant_keeps_combat_state_and_controller_on_each_or_arm() {
    assert_compiled_line(
        "Death Tyrant",
        "Negative Energy Cone — Whenever an attacking creature you control or a blocking creature an opponent controls dies, create a 2/2 black Zombie creature token.",
    );
}

#[test]
fn remove_enchantments_keeps_all_branch_scopes_and_authored_connectives() {
    let definition = parse_oracle_card_definition("Remove Enchantments");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Return to your hand all enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control. Then destroy all other enchantments you control, all other Auras attached to permanents you control, and all other Auras attached to attacking creatures your opponents control.".to_string(),
        ]
    );
}
