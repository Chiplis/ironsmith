//! Return destinations beyond hand/battlefield/graveyard: the command zone
//! (CR 408), card-type exceptions in a return-all ("except for Giants,
//! Wizards, and lands"), and "return that card under your control" with no
//! zone, which can only mean the battlefield (only permanents have
//! controllers, CR 108.4). Frozen complete bodies; source-authored, UNRUN.
use ironsmith::effects::MoveToZoneEffect;
use ironsmith::Zone;

#[path = "cf8_p08/support.rs"]
mod support;

const LEADERSHIP_VACUUM: &str = "Mana cost: {2}{U}\nType: Instant\nTarget player returns each commander they control from the battlefield to the command zone.\nDraw a card.";
const HELLKITE_COURSER: &str = "Mana cost: {4}{R}{R}\nType: Creature — Dragon\nPower/Toughness: 6/5\nFlying\nWhen this creature enters, you may put a commander you own from the command zone onto the battlefield. It gains haste. Return it to the command zone at the beginning of the next end step.";
const CYCLONE_SUMMONER: &str = "Mana cost: {5}{U}{U}\nType: Creature — Giant Wizard\nPower/Toughness: 7/7\nWhen this creature enters, if you cast it from your hand, return all permanents to their owners' hands except for Giants, Wizards, and lands.";
const MEATHOOK_MASSACRE_II: &str = "Mana cost: {X}{X}{B}{B}{B}{B}\nType: Legendary Enchantment\nWhen Meathook Massacre II enters, each player sacrifices X creatures of their choice.\nWhenever a creature you control dies, you may pay 3 life. If you do, return that card under your control with a finality counter on it.\nWhenever a creature an opponent controls dies, they may pay 3 life. If they don't, return that card under your control with a finality counter on it.";

#[test]
fn command_zone_returns_move_to_the_command_zone() {
    for (name, body) in [("Leadership Vacuum", LEADERSHIP_VACUUM), ("Hellkite Courser", HELLKITE_COURSER)] {
        for definition in support::definitions(name, body) {
            let moves = support::find_all::<MoveToZoneEffect>(&definition);
            assert!(moves.iter().any(|movement| movement.zone == Zone::Command), "{name}: {moves:?}");
        }
    }
    for definition in support::definitions("Leadership Vacuum", LEADERSHIP_VACUUM) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("is_commander: true"), "{debug}");
    }
}

#[test]
fn return_all_exceptions_exclude_subtypes_and_card_types() {
    for definition in support::definitions("Cyclone Summoner", CYCLONE_SUMMONER) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("excluded_card_types: [Land]"), "{debug}");
        assert!(debug.contains("Giant") && debug.contains("Wizard"), "{debug}");
    }
}

#[test]
fn return_under_your_control_without_a_zone_enters_the_battlefield() {
    for definition in support::definitions("Meathook Massacre II", MEATHOOK_MASSACRE_II) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("Finality"), "{debug}");
        assert!(debug.contains("Battlefield"), "{debug}");
    }
    // A return that names its zone keeps it.
    let hand = "Mana cost: {B}\nType: Sorcery\nReturn target creature card from your graveyard to your hand.";
    assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Hand return", hand, false).is_ok());
}
