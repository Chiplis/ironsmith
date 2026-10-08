//! UNVALIDATED implementation-first coverage: unlimited land plays and the
//! triggering land's ordinal among lands played this turn (CR 305.2).
use ironsmith::ability::AbilityKind;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn fastbond_unlimited_plays_and_not_first_land_gate() {
    let rows = common::rows(include_str!("../../../fixtures/land_play_ordinals.json.fixture"));
    let row = common::row(&rows, "Fastbond");
    for definition in common::definitions(row) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains(&format!("AdditionalLandPlays({})", u32::MAX))
            || debug.contains("any number of lands"), "unlimited land plays: {debug}");
        let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        }).expect("land-play trigger");
        let gate = format!("{:?}", trigger.intervening_if);
        assert!(gate.contains("LandsPlayed(You)"), "{gate}");
        assert!(gate.contains("NotEqual"), "{gate}");
        assert!(gate.contains("Fixed(1)"), "{gate}");
        let lines = common::rendered(&definition);
        assert!(!lines.contains("if it wasn't a land"), "old lossy reading: {lines}");
    }
}

#[test]
fn ordinal_land_gate_reads_the_ordinal() {
    let text = "Mana cost: {G}\nType: Enchantment\nWhenever you play a land, if it's the second land you played this turn, draw a card.";
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition("Second land probe", text, false).unwrap();
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Triggered(triggered) => Some(triggered),
        _ => None,
    }).unwrap();
    let gate = format!("{:?}", trigger.intervening_if);
    assert!(gate.contains("LandsPlayed(You)") && gate.contains("Equal") && gate.contains("Fixed(2)"), "{gate}");
}
