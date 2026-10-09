//! cf8 p05: "turn a face-down creature you control face up" / "turn a
//! permanent you control face up" read as a chosen object for the existing
//! TurnFaceUpEffect (CR 708.8). Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

const CLUSTER: &str = "face_up_effect_targets";

#[test]
fn chosen_permanent_is_turned_face_up() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 2);
    for row in &rows {
        support::assert_markers(CLUSTER, row["name"].as_str().unwrap(), &["TurnFaceUpEffect"]);
    }
}
