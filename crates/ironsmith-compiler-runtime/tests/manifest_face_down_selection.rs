//! cf8 p05: manifest next to cloak on the put-face-down action.
//! "Manifest one of those cards, then put the other on the top or bottom of
//! your library" selects out of the looked-at group, manifests the selection
//! (CR 701.40a: face down as a 2/2 creature, no ward) and puts each card left
//! in the library on its top or bottom. "exile it and the top card of your
//! library in a face-down pile, shuffle that pile, then manifest those cards"
//! is one pile program whose shuffled pile enters face down one card at a
//! time (CR 701.40e). Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

use ironsmith::effects::{
    ChooseObjectsEffect, ManifestObjectsEffect, MoveToLibraryTopOrBottomChoiceEffect,
};
use ironsmith::target::PlayerFilter;

const CLUSTER: &str = "manifest_face_down_selection";

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 2);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn write_into_being_manifests_one_looked_card_and_places_the_other() {
    for definition in support::definitions(&support::row(CLUSTER, "Write into Being")) {
        let manifests = support::effects_of::<ManifestObjectsEffect>(&definition);
        assert_eq!(manifests.len(), 1, "{manifests:?}");
        assert!(!manifests[0].cloak, "manifest has no ward");
        assert!(!manifests[0].tapped);
        assert!(!manifests[0].shuffle);
        assert_eq!(manifests[0].controller, PlayerFilter::You);
        let choices = support::effects_of::<ChooseObjectsEffect>(&definition);
        assert!(
            choices.iter().any(|choice| choice.count.min == 1 && choice.count.max == Some(1)),
            "exactly one looked card is manifested: {choices:?}"
        );
        assert_eq!(
            support::effects_of::<MoveToLibraryTopOrBottomChoiceEffect>(&definition).len(),
            1,
            "the other card goes on the top or bottom"
        );
    }
}

#[test]
fn jeskai_infiltrator_manifests_its_shuffled_face_down_pile() {
    for definition in support::definitions(&support::row(CLUSTER, "Jeskai Infiltrator")) {
        let manifests = support::effects_of::<ManifestObjectsEffect>(&definition);
        assert_eq!(manifests.len(), 1, "{manifests:?}");
        assert!(!manifests[0].cloak);
        assert!(manifests[0].shuffle, "the pile is shuffled before it enters");
        assert!(!manifests[0].tapped);
        let debug = support::debug(&definition);
        assert!(debug.contains("cloak_pile"), "the pile tag spans both exiles");
    }
}
