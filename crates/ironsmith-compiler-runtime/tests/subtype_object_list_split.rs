//! cf8 p05: a serial subtype object list whose last arm carries "you
//! control" is one object filter, not coordinated actions.
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

const CLUSTER: &str = "subtype_object_list_split";

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 3);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn every_listed_subtype_stays_in_the_filter() {
    support::assert_markers(CLUSTER, "Vaan, Street Thief", &["Scout", "Pirate", "Rogue"]);
    support::assert_markers(CLUSTER, "Oakhollow Village", &["Frog", "Rabbit", "Raccoon", "Squirrel"]);
    support::assert_markers(CLUSTER, "Mirkwood", &["Bear", "Spider", "Wolf"]);
}
