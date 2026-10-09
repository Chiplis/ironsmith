//! "put ... onto the battlefield blocking that creature" (Aetherplasm): the
//! entering creature blocks the named attacker without being declared as a
//! blocker (CR 509.4). Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::MoveToZoneEffect;
use ironsmith::Zone;

const AETHERPLASM: &str = "Mana cost: {2}{U}{U}\nType: Creature — Illusion\nPower/Toughness: 1/1\nWhenever this creature blocks a creature, you may return this creature to its owner's hand. If you do, you may put a creature card from your hand onto the battlefield blocking that creature.";

#[test]
fn aetherplasm_puts_the_creature_onto_the_battlefield_blocking_the_attacker() {
    for definition in support::definitions("Aetherplasm", AETHERPLASM) {
        let moves = support::find_all::<MoveToZoneEffect>(&definition);
        let entry = moves
            .iter()
            .find(|effect| effect.zone == Zone::Battlefield)
            .unwrap_or_else(|| panic!("{moves:#?}"));
        assert!(entry.enters_blocking.is_some(), "{entry:#?}");
        assert!(!entry.enters_attacking);
        assert!(
            moves.iter().any(|effect| effect.zone == Zone::Hand),
            "this creature returns to hand first: {moves:#?}"
        );
    }
}
