//! A card name that begins with a rules connective ("And They Shall Know No
//! Fear") never aliases that connective to the source. Source-authored, unrun.
#[path = "p12_other/support.rs"]
mod support;

#[test]
fn and_they_shall_know_no_fear_keeps_its_coordinated_grant() {
    for definition in support::definitions("And They Shall Know No Fear") {
        let debug = support::debug(&definition);
        assert!(debug.contains("chosen_creature_type: true"), "chosen-type subject");
        assert!(debug.contains("Indestructible") || debug.contains("indestructible"));
        assert!(!debug.contains("source: true"), "'and' must not become a source reference");
    }
}
