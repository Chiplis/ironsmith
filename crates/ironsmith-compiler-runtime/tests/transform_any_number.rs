//! UNVALIDATED implementation-first coverage (cf8 p09): "Then transform any
//! number of Human Werewolves you control." chooses any number of matching
//! permanents and transforms each (CR 701.27).
use ironsmith::effects::{ChooseObjectsEffect, TransformEffect};

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn tovolar_transforms_any_number_of_chosen_werewolves() {
    let rows = common::rows(include_str!("../../../fixtures/transform_any_number.json.fixture"));
    for definition in common::definitions(common::row(&rows, "Tovolar, Dire Overlord")) {
        let effects = common::all_effects(&definition);
        assert!(effects.iter().any(|effect| effect.downcast_ref::<ChooseObjectsEffect>().is_some()));
        assert!(effects.iter().any(|effect| effect.downcast_ref::<TransformEffect>().is_some()));
    }
}
