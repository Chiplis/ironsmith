//! cf8/p07: resolving damage multipliers scoped by a leading duration or to
//! the next damage event, with a referenced source or recipient player
//! (CR 614.1a, 611.2c). Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effects::{RegisterDamageMultiplierEffect, ReplacementApplyMode};
use ironsmith::target::PlayerFilter;

const FIXTURE: &str = include_str!("../../../fixtures/scoped_damage_multipliers.json.fixture");

fn multipliers(definition: &ironsmith::cards::CardDefinition) -> Vec<RegisterDamageMultiplierEffect> {
    let mut effects = Vec::new();
    for ability in support::activated(definition) {
        effects.extend(support::activated_effects(ability));
    }
    for ability in support::triggered(definition) {
        effects.extend(support::triggered_effects(ability));
    }
    support::find::<RegisterDamageMultiplierEffect>(&effects)
}

#[test]
fn jeska_triples_that_creatures_combat_damage_to_opponents_until_your_next_turn() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Jeska, Thrice Reborn");
    assert_eq!(row["oracle_id"], "b1fbfcf3-6921-4417-a58e-0f5e5d34a105");
    for definition in support::definitions(row) {
        let found = multipliers(&definition);
        assert_eq!(found.len(), 1);
        let spec = &found[0];
        assert_eq!(spec.factor, 3);
        assert!(spec.combat_only);
        assert_eq!(spec.mode, ReplacementApplyMode::UntilYourNextTurn);
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::Opponent));
        assert!(spec.target_object_filter.is_none());
        // The source is the chosen target creature, not every creature.
        assert!(!spec.source_filter.tagged_constraints.is_empty(), "{spec:?}");
    }
}

#[test]
fn lightning_doubles_damage_to_the_damaged_player_and_their_permanents() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Lightning, Army of One");
    assert_eq!(row["oracle_id"], "585eb5bc-5a3d-44d8-b593-1ff0d67f96a7");
    for definition in support::definitions(row) {
        let found = multipliers(&definition);
        assert_eq!(found.len(), 1);
        let spec = &found[0];
        assert_eq!(spec.factor, 2);
        assert!(!spec.combat_only);
        assert_eq!(spec.mode, ReplacementApplyMode::UntilYourNextTurn);
        let player = spec.target_player_filter.clone().expect("player recipient");
        assert!(!matches!(player, PlayerFilter::Any | PlayerFilter::Opponent), "{player:?}");
        assert!(!player.mentions_iterated_player(), "bound to the damaged player: {player:?}");
        let object = spec.target_object_filter.clone().expect("permanent recipient");
        assert_eq!(object.controller.as_ref(), Some(&player));
    }
}

#[test]
fn impulsive_maneuvers_doubles_the_next_combat_damage_or_prevents_it() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Impulsive Maneuvers");
    assert_eq!(row["oracle_id"], "39a9323d-dddc-42ac-929d-3f4fa7c87567");
    for definition in support::definitions(row) {
        let found = multipliers(&definition);
        assert_eq!(found.len(), 1);
        let spec = &found[0];
        assert_eq!(spec.factor, 2);
        assert!(spec.combat_only);
        assert_eq!(spec.mode, ReplacementApplyMode::OneShot);
        assert!(!spec.source_filter.tagged_constraints.is_empty(), "{spec:?}");
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("PreventNextTimeDamageEffect"), "{debug}");
        assert!(debug.contains("Coin") || debug.contains("coin"), "{debug}");
    }
}
