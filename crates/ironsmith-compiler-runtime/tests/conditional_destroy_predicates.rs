//! UNVALIDATED implementation-first coverage: conditional destroy gates
//! "if it didn't attack this turn" and "if no other creature has greater
//! power" (CR 608.2c: the condition is checked as the instruction resolves).
use ironsmith::effects::DestroyEffect;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn conditional_destroy_gates_compile() {
    let rows = common::rows(include_str!("../../../fixtures/conditional_destroy_predicates.json.fixture"));
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in common::definitions(row) {
            let effects = common::all_effects(&definition);
            assert!(effects.iter().any(|e| e.downcast_ref::<DestroyEffect>().is_some()), "{name}");
            let debug = format!("{:?}{:?}", definition.abilities, definition.spell_effect);
            match name {
                "Aggression" => assert!(debug.contains("attacked_this_turn: true"), "{name}"),
                "Getaway Glamer" => assert!(debug.contains("TargetHasGreatestPowerAmongCreatures"), "{name}"),
                other => panic!("unexpected cohort member {other}"),
            }
        }
    }
}
