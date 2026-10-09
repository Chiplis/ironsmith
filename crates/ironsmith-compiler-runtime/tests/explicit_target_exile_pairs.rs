//! cf8 p10: one exile verb governing two independently chosen targets.
//! Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::ExileEffect;
use ironsmith::target::ChooseSpec;
use ironsmith::CardType;

const GRIP_OF_DESOLATION: &str = "Mana cost: {4}{B}{B}\nType: Instant\nDevoid (This card has no color.)\nExile target creature and target land.";

fn targeted_type(spec: &ChooseSpec) -> Option<CardType> {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => targeted_type(inner),
        ChooseSpec::Object(filter) => filter.card_types.first().copied(),
        _ => None,
    }
}

#[test]
fn grip_of_desolation_exiles_a_target_creature_and_a_target_land() {
    for definition in support::definitions("Grip of Desolation", GRIP_OF_DESOLATION) {
        let exiles = support::find_all::<ExileEffect>(&definition);
        assert_eq!(exiles.len(), 2, "two independent exiles, not a creature-or-land union");
        let mut types: Vec<_> = exiles.iter().filter_map(|exile| targeted_type(&exile.spec)).collect();
        types.sort_by_key(|card_type| format!("{card_type:?}"));
        assert_eq!(types, vec![CardType::Creature, CardType::Land]);
        assert!(exiles.iter().all(|exile| exile.spec.is_target()), "both are targets (CR 115.1)");
    }
}
