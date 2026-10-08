//! A dedicated reading owns its complete line over a generic one that would
//! re-read it non-equivalently: "All permanents are colorless" (CR 105.2c),
//! "If you would draw a card, draw two cards instead", and "target creature
//! has base power and toughness 4/4" inside a chain. Source-authored, UNRUN.
#[path = "cf8_p08/support.rs"]
mod support;

const BODIES: &[(&str, &str, &str)] = &[
    ("Thran Lens", "Mana cost: {2}\nType: Artifact\nAll permanents are colorless.", "colorless"),
    ("Mycosynth Lattice", "Mana cost: {6}\nType: Artifact\nAll permanents are artifacts in addition to their other types.\nAll cards that aren't on the battlefield, spells, and permanents are colorless.\nPlayers may spend mana as though it were mana of any color.", "colorless"),
    ("Vnwxt, Verbose Host", "Mana cost: {1}{U}\nType: Legendary Creature — Homunculus\nPower/Toughness: 0/4\nStart your engines!\nYou have no maximum hand size.\nMax speed — If you would draw a card, draw two cards instead.", "draw two cards"),
    ("Wings of Velis Vel", "Mana cost: {1}{U}\nType: Kindred Instant — Shapeshifter\nChangeling\nUntil end of turn, target creature has base power and toughness 4/4, gains all creature types, and gains flying.", "4/4"),
];

#[test]
fn each_line_compiles_through_its_single_owner() {
    for (name, body, needle) in BODIES {
        for definition in support::definitions(name, body) {
            let text = support::rendered(&definition);
            assert!(text.contains(needle), "{name}: {text}");
        }
    }
}

#[test]
fn thran_lens_is_one_colorless_static() {
    for definition in support::definitions("Thran Lens", BODIES[0].1) {
        assert_eq!(definition.abilities.len(), 1);
    }
}
