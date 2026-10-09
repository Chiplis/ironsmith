//! "You may cast this card from your graveyard, but not from anywhere else."
//! (Haakon): a graveyard cast permission plus a restriction checked where the
//! card is when proposed (CR 601.3e). Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const HAAKON: &str = "Mana cost: {3}{B}{B}\nType: Legendary Creature — Zombie Knight\nPower/Toughness: 3/3\nYou may cast this card from your graveyard, but not from anywhere else.\nAs long as Haakon is on the battlefield, you may cast Knight spells from your graveyard.\nWhen Haakon dies, you lose 2 life.";

#[test]
fn haakon_is_castable_from_the_graveyard_only() {
    for definition in support::definitions("Haakon, Stromgald Scourge", HAAKON) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("SourceIsInZone(Graveyard)"), "{debug}");
        assert!(debug.contains("Graveyard"), "{debug}");
    }
}
