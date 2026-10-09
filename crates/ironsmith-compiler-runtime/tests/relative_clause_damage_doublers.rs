//! "Double all damage that creatures you control with counters on them would
//! deal." (Raphael, the Muscle): the imperative damage multiplier (CR 614.1a)
//! whose relative clause names the dealing objects without a "source" noun.
//! Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const RAPHAEL: &str = "Mana cost: {4}{R}\nType: Legendary Creature — Mutant Ninja Turtle\nPower/Toughness: 4/4\nDouble all damage that creatures you control with counters on them would deal.\nWhen Raphael enters, create a Mutagen token.\nPartner—Character select (You can have two commanders if both have this ability.)";

#[test]
fn raphael_doubles_damage_from_countered_creatures_you_control() {
    for definition in compile::compile_both("Raphael, the Muscle", RAPHAEL) {
        let doubler = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(format!("{ability:?}")),
                _ => None,
            })
            .find(|debug| debug.contains("factor: 2"))
            .expect("damage doubling replacement");
        assert!(doubler.contains("Creature"), "{doubler}");
        assert!(doubler.contains("with_counter"), "{doubler}");
        assert!(doubler.contains("controller: Some(You)"), "{doubler}");
    }
}
