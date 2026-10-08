//! cf8/p07: dynamic card/life quantities.
//! - "gain life and draw cards equal to X": a bare coordinated life change
//!   shares the terminal amount (Lifeblood Hydra, Blim).
//! - "discard cards equal to X" reads the same dynamic count grammar as draw.
//! - "equal to the damage" is the triggering damage event's amount.
//! - "permanents they control but don't own": the elided owner subject is
//!   "they", not "you" (silent miscompile fixed).
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effect::Value;
use ironsmith::effects::{DiscardEffect, DrawCardsEffect, GainLifeEffect, LoseLifeEffect};
use ironsmith::target::PlayerFilter;

const FIXTURE: &str = include_str!("../../../fixtures/dynamic_count_quantities.json.fixture");

fn trigger_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<ironsmith::effect::Effect> {
    support::triggered(definition)
        .into_iter()
        .flat_map(support::triggered_effects)
        .collect()
}

#[test]
fn lifeblood_hydra_gains_life_and_draws_by_its_power() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Lifeblood Hydra");
    assert_eq!(row["oracle_id"], "b14d05c0-fe10-4079-a90e-0aea1a8fd375");
    for definition in support::definitions(row) {
        let all = trigger_effects(&definition);
        let gains = support::find::<GainLifeEffect>(&all);
        let draws = support::find::<DrawCardsEffect>(&all);
        assert_eq!(gains.len(), 1);
        assert_eq!(draws.len(), 1);
        assert!(matches!(gains[0].amount.unhinted(), Value::PowerOf(_)), "{:?}", gains[0].amount);
        assert_eq!(gains[0].amount.unhinted(), draws[0].count.unhinted());
    }
}

#[test]
fn blim_counts_permanents_each_player_controls_but_does_not_own() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Blim, Comedic Genius");
    assert_eq!(row["oracle_id"], "c7cf9574-e2cf-4137-a224-73f76c5d11d7");
    for definition in support::definitions(row) {
        let all = trigger_effects(&definition);
        let losses = support::find::<LoseLifeEffect>(&all);
        let discards = support::find::<DiscardEffect>(&all);
        assert_eq!(losses.len(), 1);
        assert_eq!(discards.len(), 1);
        for amount in [&losses[0].amount, &discards[0].count] {
            let Value::Count(filter) = amount.unhinted() else {
                panic!("count of permanents: {amount:?}");
            };
            assert_eq!(filter.controller, Some(PlayerFilter::IteratedPlayer));
            assert_eq!(
                filter.owner,
                Some(PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::IteratedPlayer)),
                "owned by someone other than that player, not merely not you"
            );
        }
    }
}

#[test]
fn jagged_poppet_discards_by_the_triggering_damage() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Jagged Poppet");
    assert_eq!(row["oracle_id"], "4beec1a6-a9c6-471f-b6fa-c754a42ad227");
    for definition in support::definitions(row) {
        let discards = support::find::<DiscardEffect>(&trigger_effects(&definition));
        assert_eq!(discards.len(), 2);
        for discard in &discards {
            assert!(
                matches!(discard.count.unhinted(), Value::EventValue(_)),
                "both discards use the damage event amount: {:?}",
                discard.count
            );
        }
    }
}
