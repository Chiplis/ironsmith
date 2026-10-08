//! cf8 p05: "time travel" (CR 701.55) read as a clause anywhere, including
//! "time travel three times" and "time travel, then time travel".
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

#[test]
fn time_travel_cards_compile_with_counted_time_travel() {
    let rows = support::rows("time_travel_clause");
    assert_eq!(rows.len(), 3);
    for row in &rows {
        support::definitions(row);
    }
    for name in ["The Parting of the Ways", "The Tenth Doctor"] {
        support::assert_markers("time_travel_clause", name, &["Repeat", "Time"]);
    }
}
