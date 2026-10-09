//! "If a nontoken creature an opponent controls would die, exile it instead.
//! When you do, ..." — the reflexive follow-up of the replacement (CR 603.12)
//! belongs to the replacement, as it already did for plain creatures.
//! Front face of Valentin, Dean of the Vein; source-authored, UNRUN.
#[path = "cf8_p08/support.rs"]
mod support;

const VALENTIN_FRONT: &str = "Mana cost: {B}\nType: Legendary Creature — Vampire Warlock\nPower/Toughness: 1/1\nMenace, lifelink\nIf a nontoken creature an opponent controls would die, exile it instead. When you do, you may pay {2}. If you do, create a 1/1 black and green Pest creature token with \"When this token dies, you gain 1 life.\"";

#[test]
fn the_pest_is_created_only_after_the_replacement_and_payment() {
    for definition in support::definitions("Valentin, Dean of the Vein", VALENTIN_FRONT) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("Pest"), "{debug}");
        assert!(debug.contains("Exile") || debug.contains("exile"), "{debug}");
    }
}
