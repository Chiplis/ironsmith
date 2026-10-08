//! cf8/p07: "chooses ... in their hand and discards the rest" discards every
//! other matching card in that hand (the complement of the chosen set).
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effects::{ChooseObjectsEffect, DiscardEffect};
use ironsmith::target::TaggedOpbjectRelation;
use ironsmith::Zone;

const FIXTURE: &str = include_str!("../../../fixtures/discard_the_rest.json.fixture");

#[test]
fn chosen_hand_cards_are_kept_and_the_rest_discarded() {
    let rows = support::rows(FIXTURE);
    for (name, oracle_id) in [
        ("Monomania", "29c6935e-ef64-4298-af74-9b18eed93136"),
        ("Breakthrough", "c6b26c1f-121f-4b7c-921e-429b623ca934"),
    ] {
        let row = support::row(&rows, name);
        assert_eq!(row["oracle_id"], oracle_id);
        for definition in support::definitions(row) {
            let all = support::spell_effects(&definition);
            let choices = support::find::<ChooseObjectsEffect>(&all);
            assert_eq!(choices.len(), 1, "{name}");
            assert_eq!(choices[0].zone, Some(Zone::Hand), "{name}");
            let discards = support::find::<DiscardEffect>(&all);
            assert_eq!(discards.len(), 1, "{name}");
            let filter = discards[0].card_filter.as_ref().expect("the rest");
            assert!(
                filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject),
                "{name}: the complement of the chosen cards"
            );
        }
    }
}
