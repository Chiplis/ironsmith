//! cf8 p05: "manifest a number of cards from the top of your library equal
//! to <amount>" repeats the single-card manifest, since multiple cards are
//! manifested one at a time (CR 701.40c). Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

#[test]
fn omarthis_manifests_one_card_per_counter() {
    support::assert_markers(
        "manifest_top_count",
        "Omarthis, Ghostfire Initiate",
        &["ManifestTopCardOfLibraryEffect", "Repeat", "CountersOn"],
    );
}
