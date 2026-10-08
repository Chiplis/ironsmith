//! UNVALIDATED implementation-first coverage: a searched-and-revealed group
//! partitioned "one into your hand and the other into your graveyard"
//! (CR 701.23) keeps both destinations.
use ironsmith::effects::{ChooseObjectsEffect, MoveToZoneEffect, ShuffleLibraryEffect};
use ironsmith::Zone;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn searched_pair_goes_to_hand_and_graveyard() {
    let rows = common::rows(include_str!("../../../fixtures/search_partition_destinations.json.fixture"));
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in common::definitions(row) {
            let effects = common::all_effects(&definition);
            let choices: Vec<_> = effects.iter().filter_map(|e| e.downcast_ref::<ChooseObjectsEffect>()).collect();
            let search = choices.iter().find(|choice| choice.is_search).expect("search");
            assert_eq!(search.count.max, Some(2), "{name}");
            assert_eq!(search.count.min, 0, "{name}");
            assert!(choices.iter().any(|choice| !choice.is_search && choice.count.min == 1 && choice.count.max == Some(1)), "{name}: one chosen card");
            let moves: Vec<_> = effects.iter().filter_map(|e| e.downcast_ref::<MoveToZoneEffect>()).collect();
            assert!(moves.iter().any(|m| m.zone == Zone::Hand), "{name}");
            assert!(moves.iter().any(|m| m.zone == Zone::Graveyard), "{name}: the other card's destination is kept");
            assert!(effects.iter().any(|e| e.downcast_ref::<ShuffleLibraryEffect>().is_some()), "{name}");
        }
    }
}
