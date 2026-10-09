//! cf8 p10 library look/put compositions built from existing effects.
//! Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::{LookAtTopCardsEffect, ReorderLibraryTopEffect};
use ironsmith::target::PlayerFilter;

const CORAL_FIGHTERS: &str = "Mana cost: {1}{U}\nType: Creature — Merfolk Soldier\nPower/Toughness: 1/1\nWhenever this creature attacks and isn't blocked, look at the top card of defending player's library. You may put that card on the bottom of that player's library.";
const DIMIR_MACHINATIONS: &str = "Mana cost: {2}{B}\nType: Sorcery\nLook at the top three cards of target player's library. Exile any number of those cards, then put the rest back in any order.\nTransmute {1}{B}{B}";

#[test]
fn coral_fighters_privately_looks_at_the_defending_players_top_card() {
    for definition in support::definitions("Coral Fighters", CORAL_FIGHTERS) {
        let [look] = support::find_all::<LookAtTopCardsEffect>(&definition).try_into().unwrap();
        assert_eq!(look.player, PlayerFilter::Defending, "the defending player's library");
        assert_eq!(look.viewer, PlayerFilter::You, "the trigger's controller looks");
        assert!(!look.reveal);
    }
}

#[test]
fn dimir_machinations_reorders_only_the_cards_left_in_the_library() {
    for definition in support::definitions("Dimir Machinations", DIMIR_MACHINATIONS) {
        let looks = support::find_all::<LookAtTopCardsEffect>(&definition);
        assert_eq!(looks.len(), 1);
        let reorders = support::find_all::<ReorderLibraryTopEffect>(&definition);
        assert_eq!(reorders.len(), 1, "'put the rest back in any order'");
    }
}
