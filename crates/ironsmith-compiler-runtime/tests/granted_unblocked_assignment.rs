//! UNVALIDATED implementation-first coverage: granting the "assign combat
//! damage as though it weren't blocked" permission (CR 510.1c) to the
//! creatures you control, as a resolving spell (until end of turn) or a
//! conditioned static ability.
#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn unblocked_assignment_is_granted_to_your_creatures() {
    let rows = common::rows(include_str!("../../../fixtures/granted_unblocked_assignment.json.fixture"));
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in common::definitions(row) {
            let debug = format!("{:?}{:?}", definition.abilities, definition.spell_effect);
            assert!(debug.contains("MayAssignDamageAsUnblocked"), "{name}: {debug}");
            match name {
                "Predatory Focus" => assert!(debug.contains("EndOfTurn"), "{name}: lasts this turn"),
                "Siege Behemoth" => assert!(debug.contains("ttack"), "{name}: conditioned on attacking"),
                other => panic!("unexpected cohort member {other}"),
            }
        }
    }
}
