//! "If a source you control would deal damage this turn, it deals double that
//! damage instead." (Insult) and "... would deal damage this turn to an
//! opponent or a permanent an opponent controls, it deals triple that damage
//! instead." (Isengard Unleashed): one-shot this-turn damage multipliers
//! (CR 614.1a), with the duration before or without a recipient.
//! Source-authored, deliberately unrun.

#[path = "p02_line_families/compile.rs"]
mod compile;

const INSULT: &str = "Mana cost: {2}{R}\nType: Sorcery\nDamage can't be prevented this turn. If a source you control would deal damage this turn, it deals double that damage instead.";
const INJURY: &str = "Mana cost: {2}{R}\nType: Sorcery\nAftermath (Cast this spell only from your graveyard. Then exile it.)\nInjury deals 2 damage to target creature and 2 damage to target player or planeswalker.";
const ISENGARD: &str = "Mana cost: {2}{R}{R}{R}\nType: Sorcery\nDamage can't be prevented this turn. If a source you control would deal damage this turn to an opponent or a permanent an opponent controls, it deals triple that damage instead.\nFlashback {4}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)";

#[test]
fn insult_doubles_all_damage_from_your_sources_this_turn() {
    for definition in compile::compile_both("Insult", INSULT) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("factor: 2"), "{debug}");
        assert!(debug.contains("UntilEndOfTurn"), "{debug}");
    }
    compile::compile_both("Injury", INJURY);
}

#[test]
fn isengard_triples_damage_to_opponents_and_their_permanents() {
    for definition in compile::compile_both("Isengard Unleashed", ISENGARD) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("factor: 3"), "{debug}");
        assert!(debug.contains("target_player_filter: Some(Opponent)"), "{debug}");
    }
}
