//! cf8 p01 round 3: you separate a graveyard pool into two piles and an
//! opponent picks the pile that is exiled (CR 700.3). Unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn death_or_glory_uses_a_real_pile_split() {
    for definition in support::definitions("Death or Glory") {
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Death or Glory", &text);
        assert!(text.contains("separate all creature cards in your graveyard into two piles"), "{text}");
        assert!(text.contains("exile the pile of an opponent's choice and return the other to the battlefield"), "{text}");
    }
}

/// Abstract Performance: the two exiles make a face-down and a face-up pile;
/// an opponent picks the pile that goes to the graveyard, and from the other
/// you may cast one spell free before the rest go to your hand.
#[test]
fn abstract_performance_face_down_and_face_up_exile_piles() {
    for definition in support::definitions("Abstract Performance") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("CastTaggedEffect"), "{debug}");
        assert!(debug.contains("ChoosePlayerEffect"), "{debug}");
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Abstract Performance", &text);
        assert!(text.contains("in a face-down pile, then exile the top four cards of your library in a face-up pile"), "{text}");
        assert!(text.contains("an opponent chooses one of those piles"), "{text}");
        assert!(text.contains("you may cast a spell from among them without paying its mana cost"), "{text}");
        assert!(text.contains("put the rest into your hand"), "{text}");
    }
}

/// Ecological Appreciation: the searched set, an opponent-chosen subset of
/// two, and its exact complement onto the battlefield; no iterated-membership
/// scaffolding leaks into the text.
#[test]
fn ecological_appreciation_opponent_chooses_two_rest_onto_battlefield() {
    for definition in support::definitions("Ecological Appreciation") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("ShuffleObjectsIntoLibraryEffect"), "{debug}");
        assert!(debug.contains("IsNotTaggedObject"), "{debug}");
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Ecological Appreciation", &text);
        assert!(text.contains("search your library and graveyard for"), "{text}");
        assert!(text.contains("an opponent chooses two of those cards"), "{text}");
        assert!(text.contains("shuffle the chosen cards into your library and put the rest onto the battlefield"), "{text}");
        assert!(text.contains("exile"), "{text}");
    }
}
