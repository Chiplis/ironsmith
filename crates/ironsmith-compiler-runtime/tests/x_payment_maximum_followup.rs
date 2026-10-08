//! cf8 p10: "X can't be greater than N" bounds the preceding {X} payment.
//! Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::PayManaEffect;

const SHANNA: &str = "Mana cost: {G}{W}{U}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 3/3\nLifelink\nAt the beginning of your end step, you may pay {X}. If you do, draw X cards. X can't be greater than the amount of life you gained this turn.";

#[test]
fn shanna_bounds_the_announced_x_by_life_gained_this_turn() {
    for definition in support::definitions("Shanna, Purifying Blade", SHANNA) {
        let [pay] = support::find_all::<PayManaEffect>(&definition)
            .try_into()
            .expect("one {X} payment");
        assert!(pay.cost.has_x());
        assert_eq!(pay.x_value, None, "the player still chooses X");
        assert!(pay.x_maximum.is_some(), "life gained this turn caps X");
    }
}
