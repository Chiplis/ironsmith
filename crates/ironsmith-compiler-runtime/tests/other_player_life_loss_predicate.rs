//! cf8/p07: "if a player other than you lost life this turn" is a life-lost
//! history condition over every player except the ability's controller.
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effect::{Condition, Value, ValueComparisonOperator};
use ironsmith::effects::{ConditionalEffect, DrawCardsEffect};
use ironsmith::target::PlayerFilter;

const FIXTURE: &str =
    include_str!("../../../fixtures/other_player_life_loss_predicate.json.fixture");

#[test]
fn ludevic_checks_life_lost_by_players_other_than_you() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Ludevic, Necro-Alchemist");
    assert_eq!(row["oracle_id"], "3b9d587c-39d2-44a2-9802-e4217ed389c0");
    for definition in support::definitions(row) {
        let all: Vec<_> = support::triggered(&definition)
            .into_iter()
            .flat_map(support::triggered_effects)
            .collect();
        let conditionals = support::find::<ConditionalEffect>(&all);
        assert_eq!(conditionals.len(), 1);
        assert_eq!(
            conditionals[0].condition,
            Condition::ValueComparison {
                left: Value::LifeLostThisTurn(PlayerFilter::NotYou),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(1),
            }
        );
        let mut guarded = Vec::new();
        for effect in &conditionals[0].if_true {
            support::collect(effect, &mut guarded);
        }
        assert_eq!(support::find::<DrawCardsEffect>(&guarded).len(), 1);
    }
}
