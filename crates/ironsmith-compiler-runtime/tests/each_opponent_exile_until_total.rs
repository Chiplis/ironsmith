//! "Each opponent exiles cards from the top of their library until they have
//! exiled cards with total mana value N or greater [this way]": a per-opponent
//! consult (each opponent in turn, CR 101.4) with the cumulative mana-value
//! stop. Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const TASHAS_HIDEOUS_LAUGHTER: &str = "Mana cost: {1}{U}{B}\nType: Sorcery\nEach opponent exiles cards from the top of their library until that player has exiled cards with total mana value 20 or greater.";
const DREAM_HARVEST: &str = "Mana cost: {5}{U}{B}\nType: Sorcery\nEach opponent exiles cards from the top of their library until they have exiled cards with total mana value 5 or greater this way. Until end of turn, you may cast cards exiled this way without paying their mana costs.";

#[test]
fn tashas_hideous_laughter_runs_one_total_mana_value_consult_per_opponent() {
    for definition in support::definitions("Tasha's Hideous Laughter", TASHAS_HIDEOUS_LAUGHTER) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("TotalManaValue(20)"), "{debug}");
        assert!(debug.contains("Exile"), "{debug}");
        assert!(
            debug.contains("ForPlayers") || debug.contains("ForEachOpponent") || debug.contains("Opponent"),
            "one consult per opponent: {debug}"
        );
    }
}

#[test]
fn dream_harvest_consults_each_opponent_then_grants_free_casts_of_the_exiled_cards() {
    for definition in support::definitions("Dream Harvest", DREAM_HARVEST) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("TotalManaValue(5)"), "{debug}");
        assert!(debug.contains("without_paying_mana_cost: true") || debug.contains("WithoutPaying"), "{debug}");
    }
}
