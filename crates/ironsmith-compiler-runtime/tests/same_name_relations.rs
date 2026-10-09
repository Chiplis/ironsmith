//! UNVALIDATED implementation-first coverage (cf8 p09): same-name relations.
//! - "all cards with the same name as that spell from their graveyard": the
//!   zone qualifier after the reference still qualifies the head noun.
//! - "costs {1} less ... for each card with the same name as that spell in
//!   your graveyard": one per matching card.
//! - "if another permanent with the same name is on the battlefield" / "if a
//!   card with the same name is in a graveyard": same-name existence.
#[path = "p09_common/mod.rs"]
mod common;

fn rows() -> Vec<serde_json::Value> {
    common::rows(include_str!("../../../fixtures/same_name_relations.json.fixture"))
}

#[test]
fn same_name_references_survive_zone_tails() {
    let rows = rows();
    for name in ["Bloodbond March", "Rat King, Verminister"] {
        for definition in common::definitions(common::row(&rows, name)) {
            let debug = format!("{:?}", common::all_effects(&definition));
            assert!(debug.contains("SameNameAsTagged"), "{name}: {debug}");
            assert!(debug.contains("Graveyard"), "{name}: {debug}");
        }
    }
}

#[test]
fn locket_counts_same_named_cards_in_your_graveyard() {
    let rows = rows();
    for definition in common::definitions(common::row(&rows, "Locket of Yesterdays")) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("count_matching_objects: true"), "{debug}");
        let text = common::rendered(&definition);
        assert!(text.contains("same name as that spell"), "{text}");
    }
}

#[test]
fn same_name_existence_conditions_gate_the_action() {
    let rows = rows();
    for name in ["Winnow", "Bazaar of Wonders"] {
        for definition in common::definitions(common::row(&rows, name)) {
            let debug = format!("{:?}", common::all_effects(&definition));
            assert!(debug.contains("SameNameAsTagged"), "{name}: {debug}");
            if name == "Winnow" {
                assert!(debug.contains("IsNotTaggedObject"), "{name}: {debug}");
            }
        }
    }
}
