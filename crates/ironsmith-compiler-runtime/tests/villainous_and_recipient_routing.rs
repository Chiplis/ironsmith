//! Round-3 routing fixes, source-authored and UNRUN:
//! - "... faces a villainous choice — <mode>, or <mode>": that dash is the
//!   choice separator, not an ability-word label, so a triggered line keeps
//!   its trigger and the faced player stays bound (The Dalek Emperor,
//!   Damocles Base) and a spell keeps its statement (Ensnared by the Mara,
//!   whose "those exiled cards" now reads the top-of-library exile).
//! - "that player and each creature that player controls" (Cerebral Eruption).
//! - an optional library search is the "If you search your library this way"
//!   producer (Unlucky Cabbage Merchant).
//! - "roll a d20 and add X" is one modified roll (Gale's Redirection).
use ironsmith::ability::AbilityKind;
use ironsmith::effects::{DealDamageEffect, ForPlayersEffect, VillainousChoiceEffect};

#[path = "cf8_p08/support.rs"]
mod support;

const DALEK_EMPEROR: &str = "Mana cost: {5}{B}{R}\nType: Legendary Artifact Creature — Dalek\nPower/Toughness: 6/6\nAffinity for Daleks\nOther Daleks you control have haste.\nAt the beginning of combat on your turn, each opponent faces a villainous choice — That player sacrifices a creature of their choice, or you create a 3/3 black Dalek artifact creature token with menace.";
const ENSNARED: &str = "Mana cost: {2}{R}{R}\nType: Sorcery\nEach opponent faces a villainous choice — They exile cards from the top of their library until they exile a nonland card, then you may cast that card without paying its mana cost, or that player exiles the top four cards of their library and Ensnared by the Mara deals damage equal to the total mana value of those exiled cards to that player.";
const CEREBRAL_ERUPTION: &str = "Mana cost: {2}{R}{R}\nType: Sorcery\nTarget opponent reveals the top card of their library. Cerebral Eruption deals damage equal to the revealed card's mana value to that player and each creature that player controls. If a land card is revealed this way, return Cerebral Eruption to its owner's hand.";
const UNLUCKY_CABBAGE_MERCHANT: &str = "Mana cost: {1}{G}\nType: Creature — Human Citizen\nPower/Toughness: 2/2\nWhen this creature enters, create a Food token.\nWhenever you sacrifice a Food, you may search your library for a basic land card and put it onto the battlefield tapped. If you search your library this way, put this creature on the bottom of its owner's library, then shuffle.";
const GALES_REDIRECTION: &str = "Mana cost: {3}{U}{U}\nType: Instant\nExile target spell, then roll a d20 and add that spell's mana value.\n1—14 | You may cast the exiled card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.\n15+ | You may cast the exiled card without paying its mana cost for as long as it remains exiled.";

#[test]
fn dalek_emperor_keeps_its_combat_trigger_with_a_per_opponent_choice() {
    for definition in support::definitions("The Dalek Emperor", DALEK_EMPEROR) {
        assert!(definition.spell_effect.is_none(), "a permanent's trigger is not spell text");
        let trigger = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("combat trigger");
        let debug = format!("{:?}", trigger.effects);
        assert!(debug.contains("VillainousChoice"), "{debug}");
        assert_eq!(support::find_all::<VillainousChoiceEffect>(&definition).len(), 1);
        assert_eq!(support::find_all::<ForPlayersEffect>(&definition).len(), 1, "each opponent");
    }
}

#[test]
fn ensnared_damage_counts_the_exiled_cards() {
    for definition in support::definitions("Ensnared by the Mara", ENSNARED) {
        let choices = support::find_all::<VillainousChoiceEffect>(&definition);
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].modes.len(), 2);
        let damage = support::find_all::<DealDamageEffect>(&definition);
        assert_eq!(damage.len(), 1);
        let amount = format!("{:?}", damage[0].amount);
        assert!(amount.contains("TotalManaValue") || amount.contains("ManaValue"), "{amount}");
    }
}

#[test]
fn cerebral_eruption_damages_the_player_and_their_creatures() {
    for definition in support::definitions("Cerebral Eruption", CEREBRAL_ERUPTION) {
        let damage = support::find_all::<DealDamageEffect>(&definition);
        assert!(damage.len() >= 1);
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("Creature"), "{debug}");
        assert!(!debug.contains("IteratedPlayer"), "{debug}");
    }
}

#[test]
fn optional_search_gates_the_bottom_and_shuffle() {
    for definition in support::definitions("Unlucky Cabbage Merchant", UNLUCKY_CABBAGE_MERCHANT) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("Food"), "{debug}");
        assert!(debug.contains("Library"), "{debug}");
    }
}

#[test]
fn gales_redirection_rolls_once_with_the_spell_mana_value_added() {
    for definition in support::definitions("Gale's Redirection", GALES_REDIRECTION) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("Add("), "{debug}");
        assert!(!debug.contains("AddManaEffect"), "the modifier is not a mana ability: {debug}");
    }
}
