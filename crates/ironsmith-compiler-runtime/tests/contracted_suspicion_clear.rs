//! cf8 p05: "it's/they're no longer suspected" (CR 701.60c) where the
//! contraction is one lexed word. Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

#[test]
fn contracted_no_longer_suspected_clears_the_designation() {
    let rows = support::rows("contracted_suspicion_clear");
    assert_eq!(rows.len(), 2);
    for row in &rows {
        for definition in support::definitions(row) {
            let debug = support::debug(&definition);
            assert!(
                debug.contains("ClearSuspected") || debug.contains("Suspect"),
                "{}: suspicion clear missing",
                row["name"]
            );
        }
    }
}
