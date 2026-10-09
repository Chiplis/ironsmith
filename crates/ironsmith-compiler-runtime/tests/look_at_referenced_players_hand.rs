//! cf8 p10: "look at defending player's hand" / "look at its controller's
//! hand". Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::LookAtHandEffect;
use ironsmith::target::{ChooseSpec, PlayerFilter};

const PORT_INSPECTOR: &str = "Mana cost: {1}{U}\nType: Creature — Human\nPower/Toughness: 1/2\nWhenever this creature becomes blocked, you may look at defending player's hand.";
const LAY_BARE: &str = "Mana cost: {2}{U}{U}\nType: Instant\nCounter target spell. Look at its controller's hand.";

fn hand_owner(spec: &ChooseSpec) -> Option<&PlayerFilter> {
    match spec {
        ChooseSpec::Player(player) => Some(player),
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => hand_owner(inner),
        _ => None,
    }
}

#[test]
fn port_inspector_looks_at_the_defending_players_hand() {
    for definition in support::definitions("Port Inspector", PORT_INSPECTOR) {
        let [look] = support::find_all::<LookAtHandEffect>(&definition).try_into().unwrap();
        assert!(!look.reveal);
        assert!(!look.target.is_target(), "not a targeted player");
        assert_eq!(hand_owner(&look.target), Some(&PlayerFilter::Defending));
    }
}

#[test]
fn lay_bare_looks_at_the_countered_spells_controllers_hand() {
    for definition in support::definitions("Lay Bare", LAY_BARE) {
        let [look] = support::find_all::<LookAtHandEffect>(&definition).try_into().unwrap();
        assert!(matches!(hand_owner(&look.target), Some(PlayerFilter::ControllerOf(_))), "{:?}", look.target);
    }
}
