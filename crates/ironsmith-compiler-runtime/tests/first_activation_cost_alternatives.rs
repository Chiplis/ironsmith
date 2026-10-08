//! cf8 p01 round 4: "You may pay {0} rather than pay the <keyword> cost of the
//! first <keyword> ability you activate each turn" is a real activation-cost
//! replacement gated to the controller's first activation of that keyword
//! ability kind each turn (CR 118.9, 602.2b), not a display-only static.
//! Bruenor / Forge Anew / Kíli were silent miscompiles (no engine effect).
//! Unrun.
#[path = "p01_support/mod.rs"]
mod support;

use ironsmith::cards::CardDefinition;

const BRUENOR: &str = "Mana cost: {2}{R}{W}\nType: Legendary Creature — Dwarf Warrior\nPower/Toughness: 5/3\nEach creature you control gets +2/+0 for each Equipment attached to it.\nYou may pay {0} rather than pay the equip cost of the first equip ability you activate each turn.";
const FORGE_ANEW: &str = "Mana cost: {2}{W}\nType: Enchantment\nWhen this enchantment enters, return target Equipment card from your graveyard to the battlefield.\nDuring your turn, you may activate equip abilities any time you could cast an instant.\nYou may pay {0} rather than pay the equip cost of the first equip ability you activate during each of your turns.";
const KILI: &str = "Mana cost: {1}{W}\nType: Legendary Creature — Dwarf Scout\nPower/Toughness: 1/2\nStoried (If you control three or more artifacts, legendaries, and/or Sagas, you have an enduring story for the rest of the game.)\nAs long as you have an enduring story, you may pay {0} rather than pay the equip cost of the first equip ability you activate each turn.\nWhenever another Dwarf or Equipment you control enters, draw a card. This ability triggers only once each turn.";

fn assert_first_keyword_alternative(
    definition: &CardDefinition,
    keyword: &str,
    during_your_turn: bool,
) {
    let debug = format!("{definition:?}");
    assert!(debug.contains("ActivatedAbilityCostReduction"), "{debug}");
    assert!(debug.contains("replacement_mana_cost: Some"), "{debug}");
    assert!(debug.contains("FirstKeywordAbilityThisTurn"), "{debug}");
    assert!(
        debug.contains(&format!("keyword: {keyword}, during_your_turn: {during_your_turn}")),
        "{debug}"
    );
    assert!(debug.contains(&format!("Keyword({keyword})")), "{debug}");
    assert!(debug.contains("Activator(You)"), "{debug}");
    assert!(!debug.contains("FirstEquipCostAlternative"), "{debug}");
}

#[test]
fn bruenor_first_equip_each_turn_costs_zero() {
    for definition in support::definitions_for_text("Bruenor Battlehammer", BRUENOR) {
        assert_first_keyword_alternative(&definition, "Equip", false);
        let text = support::rendered(&definition);
        assert!(
            text.contains("you may pay {0} rather than pay the equip cost of the first equip ability you activate each turn"),
            "{text}"
        );
    }
}

#[test]
fn forge_anew_first_equip_only_during_your_turns() {
    for definition in support::definitions_for_text("Forge Anew", FORGE_ANEW) {
        assert_first_keyword_alternative(&definition, "Equip", true);
        let text = support::rendered(&definition);
        assert!(text.contains("during each of your turns"), "{text}");
    }
}

#[test]
fn kili_first_equip_alternative_keeps_its_enduring_story_gate() {
    for definition in support::definitions_for_text("Kíli the Resourceful", KILI) {
        assert_first_keyword_alternative(&definition, "Equip", false);
        let text = support::rendered(&definition);
        assert!(text.contains("as long as you have an enduring story"), "{text}");
    }
}

#[test]
fn gavi_first_cycled_card_each_turn_costs_zero() {
    for definition in support::definitions("Gavi, Nest Warden") {
        assert_first_keyword_alternative(&definition, "Cycling", false);
        let text = support::rendered(&definition);
        assert!(
            text.contains("rather than pay the cycling cost of the first card you cycle each turn"),
            "{text}"
        );
        support::assert_no_internal_markers("Gavi, Nest Warden", &text);
    }
}

#[test]
fn advancing_the_spirit_first_power_up_during_your_turns() {
    for definition in support::definitions("Advancing the Spirit") {
        assert_first_keyword_alternative(&definition, "PowerUp", true);
        let text = support::rendered(&definition);
        assert!(
            text.contains("the first power-up ability you activate during each of your turns"),
            "{text}"
        );
    }
}

/// Echo-cost alternative: a static whose compiled model the echo payment
/// reads when the echo trigger resolves (CR 118.9, 702.30a).
#[test]
fn thick_skinned_goblin_offers_a_zero_echo_cost() {
    for definition in support::definitions("Thick-Skinned Goblin") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("EchoCostAlternative"), "{debug}");
        assert!(debug.contains("replacement_mana_cost"), "{debug}");
        let text = support::rendered(&definition);
        assert!(
            text.contains("you may pay {0} rather than pay the echo cost for permanents you control"),
            "{text}"
        );
    }
}
