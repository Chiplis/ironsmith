//! UNVALIDATED implementation-first coverage: a combat-history destroy target
//! keeps its authored count wrapper ("up to one target creature that was
//! dealt damage this turn").
use ironsmith::ability::AbilityKind;
use ironsmith::effects::DestroyEffect;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn case_of_the_gorgons_kiss_targets_up_to_one_damaged_creature() {
    let rows = common::rows(include_str!("../../../fixtures/combat_history_target_counts.json.fixture"));
    let row = common::row(&rows, "Case of the Gorgon's Kiss");
    for definition in common::definitions(row) {
        let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        }).expect("ETB trigger");
        let debug = format!("{trigger:?}");
        assert!(debug.contains("was_dealt_damage_this_turn: true"), "{debug}");
        let effects = common::all_effects(&definition);
        assert!(effects.iter().any(|effect| effect.downcast_ref::<DestroyEffect>().is_some()));
        let lines = common::rendered(&definition);
        assert!(lines.contains("up to one target creature"), "{lines}");
        assert!(lines.contains("dealt damage this turn"), "{lines}");
    }
}
