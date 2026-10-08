//! Antecedents bound by the clause or the trigger that names them:
//! "it deals damage to target player equal to ... that player controls"
//! (the explicit target), "If a card named X was revealed this way" after a
//! revealing look (the reveal is the producer), and "that player faces a
//! villainous choice" in a combat-damage trigger. Source-authored, UNRUN.
#[path = "cf8_p08/support.rs"]
mod support;

const ANATHEMANCER: &str = "Mana cost: {1}{B}{R}\nType: Creature — Zombie Wizard\nPower/Toughness: 2/2\nWhen this creature enters, it deals damage to target player equal to the number of nonbasic lands that player controls.\nUnearth {5}{B}{R}";
const STOMPING_SLABS: &str = "Mana cost: {2}{R}\nType: Sorcery\nReveal the top seven cards of your library, then put those cards on the bottom of your library in any order. If a card named Stomping Slabs was revealed this way, Stomping Slabs deals 7 damage to any target.";
const DAMOCLES_BASE: &str = "Mana cost: {4}{B}\nType: Legendary Artifact — Vehicle\nPower/Toughness: 5/5\nFlying, deathtouch\nWhenever Damocles Base deals combat damage to a player, that player faces a villainous choice — They sacrifice a nontoken creature of their choice, or they lose 2 life and you draw two cards.\nCrew 3";

#[test]
fn that_player_in_the_amount_is_the_targeted_player() {
    for definition in support::definitions("Anathemancer", ANATHEMANCER) {
        let debug = format!("{:?}", definition.abilities);
        assert!(!debug.contains("IteratedPlayer"), "{debug}");
        assert!(debug.contains("Target("), "{debug}");
    }
}

#[test]
fn revealed_this_way_reads_the_revealing_look() {
    for definition in support::definitions("Stomping Slabs", STOMPING_SLABS) {
        let damage = support::find_all::<ironsmith::effects::DealDamageEffect>(&definition);
        assert_eq!(damage.len(), 1);
        assert_eq!(damage[0].amount, ironsmith::effect::Value::Fixed(7));
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("Stomping Slabs"), "the name condition is retained: {debug}");
    }
}

#[test]
fn the_damaged_player_faces_one_villainous_choice() {
    for definition in support::definitions("Damocles Base, Sword of Kang", DAMOCLES_BASE) {
        let choices = support::find_all::<ironsmith::effects::VillainousChoiceEffect>(&definition);
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].modes.len(), 2);
        assert!(support::find_all::<ironsmith::effects::ForPlayersEffect>(&definition).is_empty(), "no opponent loop");
    }
}
