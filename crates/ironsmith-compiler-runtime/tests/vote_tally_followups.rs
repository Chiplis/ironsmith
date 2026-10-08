//! UNVALIDATED implementation-first coverage (cf8 p09): instructions that
//! read a finished vote's tally (CR 701.38).
//! - Council Guardian: "gains protection from each color with the most votes
//!   or tied for most votes" applies once per winning named option, gated by
//!   the "<option> gets more votes or ties" vote predicate.
//! - Círdan the Shipwright: "Each player draws a card for each vote they
//!   received" repeats per received vote; "Each player who received no votes
//!   may put ..." gates on a zero vote total.
use ironsmith::effect::{Condition, Value};
use ironsmith::effects::{ConditionalEffect, VoteChoice, VoteEffect};
use ironsmith::target::PlayerFilter;

#[path = "p09_common/mod.rs"]
mod common;

fn rows() -> Vec<serde_json::Value> {
    common::rows(include_str!("../../../fixtures/vote_tally_followups.json.fixture"))
}

#[test]
fn council_guardian_grants_protection_per_winning_color() {
    let rows = rows();
    for definition in common::definitions(common::row(&rows, "Council Guardian")) {
        let effects = common::all_effects(&definition);
        let vote = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<VoteEffect>())
            .expect("vote");
        let VoteChoice::NamedOptions(options) = &vote.choice else {
            panic!("named vote expected: {:?}", vote.choice);
        };
        let names = options.iter().map(|option| option.name.as_str()).collect::<Vec<_>>();
        assert_eq!(names, vec!["blue", "black", "red", "green"]);
        assert!(vote.starting_with_controller);
        for color in ["blue", "black", "red", "green"] {
            assert!(
                effects.iter().any(|effect| {
                    effect.downcast_ref::<ConditionalEffect>().is_some_and(|conditional| {
                        conditional.condition
                            == Condition::VoteOptionGetsMoreVotesOrTied(color.to_string())
                            && format!("{:?}", conditional.if_true)
                                .to_lowercase()
                                .contains("protection")
                    })
                }),
                "no protection grant gated on {color}"
            );
        }
    }
}

#[test]
fn cirdan_reads_each_players_received_votes() {
    let rows = rows();
    for definition in common::definitions(common::row(&rows, "Círdan the Shipwright")) {
        let effects = common::all_effects(&definition);
        let vote = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<VoteEffect>())
            .expect("vote");
        assert!(vote.secret);
        assert!(matches!(vote.choice, VoteChoice::Players { .. }));
        let per_player_votes = format!(
            "{:?}",
            Value::PlayerVoteCount(PlayerFilter::IteratedPlayer)
        );
        let debug = format!("{effects:?}");
        assert!(debug.contains(&per_player_votes), "{debug}");
        assert!(
            effects.iter().any(|effect| {
                effect.downcast_ref::<ConditionalEffect>().is_some_and(|conditional| {
                    format!("{:?}", conditional.condition).contains(&per_player_votes)
                })
            }),
            "no zero-vote gate"
        );
    }
}
