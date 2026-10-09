//! cf8 p05: "repeat this process" forms that name what changes or who acts,
//! next to the condition-driven RepeatProcessEffect loop.
//! "Then repeat this process for instant cards with mana values 2 and 1"
//! performs the previous instruction again once per listed value, in order,
//! with that value in place of the one it named (CR 608.2c). "Repeat the
//! following process for each opponent in turn order" performs the rest of
//! the ability once per opponent, with "that player" bound to that opponent.
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

use ironsmith::effects::ChooseObjectsEffect;
use ironsmith::Zone;

const CLUSTER: &str = "repeat_process_variants";

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 3);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn firemind_searches_for_mana_values_three_then_two_then_one() {
    for definition in support::definitions(&support::row(CLUSTER, "Firemind's Foresight")) {
        let searches = support::effects_of::<ChooseObjectsEffect>(&definition)
            .into_iter()
            .filter(|choice| choice.is_search && choice.zone == Some(Zone::Library))
            .collect::<Vec<_>>();
        assert_eq!(searches.len(), 3, "one search per mana value: {searches:#?}");
        // Each repetition keeps the instant restriction with its own value.
        let values = searches
            .iter()
            .map(|search| format!("{:?}", search.filter.mana_value))
            .collect::<Vec<_>>();
        assert_eq!(values.len(), 3);
        assert!(values[0].contains('3') && values[1].contains('2') && values[2].contains('1'), "{values:?}");
        assert_eq!(
            support::effects_of::<ironsmith::effects::ShuffleLibraryEffect>(&definition).len(),
            1
        );
    }
}

#[test]
fn protection_racket_runs_the_following_process_once_per_opponent() {
    for definition in support::definitions(&support::row(CLUSTER, "Protection Racket")) {
        let debug = support::debug(&definition);
        assert!(debug.contains("Opponent"), "{debug}");
        // The opponent of the current iteration decides whether to pay.
        assert!(debug.contains("IteratedPlayer"), "{debug}");
        assert!(debug.contains("PayLife") || debug.contains("LoseLife"), "{debug}");
    }
}

#[test]
fn timesifter_repeats_the_round_among_tied_players_then_the_winner_takes_a_turn() {
    for definition in support::definitions(&support::row(CLUSTER, "Timesifter")) {
        assert_eq!(
            support::effects_of::<ironsmith::effects::TagPlayersEffect>(&definition).len(),
            1,
            "every player starts as a contender"
        );
        assert_eq!(
            support::effects_of::<ironsmith::effects::KeepGreatestManaValuePlayersEffect>(&definition)
                .len(),
            1
        );
        let debug = support::debug(&definition);
        assert!(debug.contains("RepeatProcessEffect"), "{debug}");
        assert!(debug.contains("GreaterThan(1)"), "repeat while a tie remains: {debug}");
        assert!(debug.contains("ExtraTurn"), "{debug}");
    }
}
