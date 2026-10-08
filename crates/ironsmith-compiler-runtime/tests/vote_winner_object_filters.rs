//! UNVALIDATED implementation-first coverage (cf8 p09): "<noun> with the most
//! votes or tied for most votes" is read at the grammar entrypoint as the
//! vote-winner selection recorded by the preceding object vote (CR 701.38),
//! for both "Return each card ..." (Custodi Squire) and "destroy each
//! creature ..." (Vault 11: Voter's Dilemma).
use ironsmith::effects::{VoteChoice, VoteEffect};
use ironsmith::zone::Zone;

#[path = "p09_common/mod.rs"]
mod common;

const VOTE_WINNERS: &str = "__vote_winners__";

#[test]
fn vote_winner_filters_select_the_recorded_winners() {
    let rows = common::rows(include_str!("../../../fixtures/vote_winner_object_filters.json.fixture"));
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in common::definitions(row) {
            let effects = common::all_effects(&definition);
            let vote = effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<VoteEffect>())
                .unwrap_or_else(|| panic!("{name}: missing vote"));
            let VoteChoice::Objects { filter, .. } = &vote.choice else {
                panic!("{name}: expected an object vote, got {:?}", vote.choice);
            };
            if name == "Custodi Squire" {
                assert_eq!(filter.zone, Some(Zone::Graveyard), "{name}");
                assert!(vote.starting_with_controller, "{name}");
            } else {
                assert!(vote.secret, "{name}");
            }
            // The follow-up removal reads the vote-winner tag.
            assert!(
                effects
                    .iter()
                    .any(|effect| format!("{effect:?}").contains(VOTE_WINNERS)),
                "{name}: no effect reads the vote winners"
            );
            let text = common::rendered(&definition).to_lowercase();
            assert!(text.contains("most votes"), "{name}: {text}");
        }
    }
}
