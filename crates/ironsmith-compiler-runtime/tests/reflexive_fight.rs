//! "<action>. When you do, it fights up to one target creature you don't
//! control.": the reflexive trigger (CR 603.12) owns the sentence; "when you
//! do" is never part of the first fighter's description. Source-authored and
//! UNRUN.
use ironsmith::effects::{FightEffect, ReflexiveTriggerEffect};

#[path = "cf8_p08/support.rs"]
mod support;

const BODIES: &[(&str, &str)] = &[
    ("Back for More", "Mana cost: {4}{B}{G}\nType: Instant\nReturn target creature card from your graveyard to the battlefield. When you do, it fights up to one target creature you don't control."),
    ("Curse of the Werefox", "Mana cost: {2}{G}\nType: Sorcery\nCreate a Monster Role token attached to target creature you control. When you do, that creature fights up to one target creature you don't control."),
];

#[test]
fn the_fight_lives_inside_the_reflexive_trigger() {
    for (name, body) in BODIES {
        for definition in support::definitions(name, body) {
            let reflexive = support::find_all::<ReflexiveTriggerEffect>(&definition);
            assert_eq!(reflexive.len(), 1, "{name}");
            assert_eq!(support::find_all::<FightEffect>(&definition).len(), 1, "{name}");
            let inner = format!("{:?}", reflexive[0]);
            assert!(inner.contains("Fight"), "{name}: the fight is the reflexive body");
        }
    }
}
