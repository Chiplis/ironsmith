//! Players finished voting event implementation.

use std::any::Any;
use std::collections::HashMap;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// A single player's vote in a voting event.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerVote {
    /// The player who voted
    pub player: PlayerId,
    /// The index of the option they voted for
    pub option_index: usize,
    /// The name of the option they voted for
    pub option_name: String,
}

/// Event emitted when players finish voting (council's dilemma, etc.).
///
/// This event is triggered after all players have cast their votes and before
/// the vote effects are resolved. It allows cards like Model of Unity to
/// react to the voting results.
#[derive(Debug, Clone)]
pub struct PlayersFinishedVotingEvent {
    /// The source object that initiated the vote (e.g., Tivit)
    pub source: ObjectId,
    /// The controller of the source object
    pub controller: PlayerId,
    /// All votes cast, in order
    pub votes: Vec<PlayerVote>,
    /// Vote counts per option (option_index -> count)
    pub vote_counts: HashMap<usize, usize>,
    /// Option names for reference
    pub option_names: Vec<String>,
    /// Pre-computed player tags for triggered abilities.
    ///
    /// Common tags include:
    /// - "voted_with_you": opponents who voted for at least one choice the controller voted for
    /// - "voted_against_you": opponents who voted only for choices the controller didn't vote for
    /// - "voted_for:{option_name}": players who voted for a specific option
    pub player_tags: HashMap<TagKey, Vec<PlayerId>>,
}

impl PlayersFinishedVotingEvent {
    /// Create a new players finished voting event.
    ///
    /// This constructor automatically computes the standard player tags:
    /// - "voted_with_you": opponents who share at least one vote with the controller
    /// - "voted_against_you": opponents who share no votes with the controller
    pub fn new(
        source: ObjectId,
        controller: PlayerId,
        votes: Vec<PlayerVote>,
        vote_counts: HashMap<usize, usize>,
        option_names: Vec<String>,
    ) -> Self {
        use std::collections::HashSet;

        // Compute which options the controller voted for
        let controller_options: HashSet<usize> = votes
            .iter()
            .filter(|v| v.player == controller)
            .map(|v| v.option_index)
            .collect();

        // Find all unique players who are not the controller
        let other_players: HashSet<PlayerId> = votes
            .iter()
            .filter(|v| v.player != controller)
            .map(|v| v.player)
            .collect();

        // Compute voted_with_you: opponents who share at least one vote
        let mut voted_with_you = Vec::new();
        let mut voted_against_you = Vec::new();

        for player in other_players {
            let player_options: HashSet<usize> = votes
                .iter()
                .filter(|v| v.player == player)
                .map(|v| v.option_index)
                .collect();

            if !controller_options.is_disjoint(&player_options) {
                // They share at least one vote
                voted_with_you.push(player);
            } else {
                // They share no votes
                voted_against_you.push(player);
            }
        }

        // Sort for deterministic ordering
        voted_with_you.sort_by_key(|p| p.0);
        voted_against_you.sort_by_key(|p| p.0);

        let mut player_tags: HashMap<TagKey, Vec<PlayerId>> = HashMap::new();
        if !voted_with_you.is_empty() {
            player_tags.insert(TagKey::from("voted_with_you"), voted_with_you);
        }
        if !voted_against_you.is_empty() {
            player_tags.insert(TagKey::from("voted_against_you"), voted_against_you);
        }

        Self {
            source,
            controller,
            votes,
            vote_counts,
            option_names,
            player_tags,
        }
    }

    /// Create a new event with additional custom player tags.
    ///
    /// This is useful when VoteEffect wants to add per-option tags.
    pub fn with_player_tags(mut self, tags: HashMap<TagKey, Vec<PlayerId>>) -> Self {
        self.player_tags.extend(tags);
        self
    }

    /// Get all players who voted for a specific option.
    pub fn players_who_voted_for(&self, option_index: usize) -> Vec<PlayerId> {
        self.votes
            .iter()
            .filter(|v| v.option_index == option_index)
            .map(|v| v.player)
            .collect()
    }

    /// Get all options that a player voted for.
    pub fn options_voted_by(&self, player: PlayerId) -> Vec<usize> {
        self.votes
            .iter()
            .filter(|v| v.player == player)
            .map(|v| v.option_index)
            .collect()
    }

    /// Check if two players voted for at least one common option.
    pub fn voted_together(&self, player1: PlayerId, player2: PlayerId) -> bool {
        let options1: std::collections::HashSet<_> =
            self.options_voted_by(player1).into_iter().collect();
        let options2: std::collections::HashSet<_> =
            self.options_voted_by(player2).into_iter().collect();
        !options1.is_disjoint(&options2)
    }
}

impl GameEventType for PlayersFinishedVotingEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PlayersFinishedVoting
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        let total_votes: usize = self.vote_counts.values().sum();
        format!("Players finished voting ({} votes cast)", total_votes)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_players_finished_voting_event_creation() {
        let source = ObjectId::from_raw(1);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let votes = vec![
            PlayerVote {
                player: alice,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: bob,
                option_index: 1,
                option_name: "bribery".to_string(),
            },
        ];

        let mut vote_counts = HashMap::new();
        vote_counts.insert(0, 1);
        vote_counts.insert(1, 1);

        let event = PlayersFinishedVotingEvent::new(
            source,
            alice,
            votes,
            vote_counts,
            vec!["evidence".to_string(), "bribery".to_string()],
        );

        assert_eq!(event.source, source);
        assert_eq!(event.controller, alice);
        assert_eq!(event.votes.len(), 2);
    }

    #[test]
    fn test_players_who_voted_for() {
        let source = ObjectId::from_raw(1);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);

        let votes = vec![
            PlayerVote {
                player: alice,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: bob,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: charlie,
                option_index: 1,
                option_name: "bribery".to_string(),
            },
        ];

        let mut vote_counts = HashMap::new();
        vote_counts.insert(0, 2);
        vote_counts.insert(1, 1);

        let event = PlayersFinishedVotingEvent::new(
            source,
            alice,
            votes,
            vote_counts,
            vec!["evidence".to_string(), "bribery".to_string()],
        );

        let evidence_voters = event.players_who_voted_for(0);
        assert_eq!(evidence_voters.len(), 2);
        assert!(evidence_voters.contains(&alice));
        assert!(evidence_voters.contains(&bob));

        let bribery_voters = event.players_who_voted_for(1);
        assert_eq!(bribery_voters.len(), 1);
        assert!(bribery_voters.contains(&charlie));
    }

    #[test]
    fn test_voted_together() {
        let source = ObjectId::from_raw(1);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);

        let votes = vec![
            PlayerVote {
                player: alice,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: bob,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: charlie,
                option_index: 1,
                option_name: "bribery".to_string(),
            },
        ];

        let mut vote_counts = HashMap::new();
        vote_counts.insert(0, 2);
        vote_counts.insert(1, 1);

        let event = PlayersFinishedVotingEvent::new(
            source,
            alice,
            votes,
            vote_counts,
            vec!["evidence".to_string(), "bribery".to_string()],
        );

        // Alice and Bob both voted for evidence
        assert!(event.voted_together(alice, bob));
        // Charlie voted for different option
        assert!(!event.voted_together(alice, charlie));
        assert!(!event.voted_together(bob, charlie));
    }

    #[test]
    fn test_event_kind() {
        let event = PlayersFinishedVotingEvent::new(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            vec![],
            HashMap::new(),
            vec![],
        );
        assert_eq!(event.event_kind(), EventKind::PlayersFinishedVoting);
    }

    #[test]
    fn test_player_tags_voted_with_you() {
        let source = ObjectId::from_raw(1);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);

        // Alice votes evidence, Bob votes evidence, Charlie votes bribery
        let votes = vec![
            PlayerVote {
                player: alice,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: bob,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: charlie,
                option_index: 1,
                option_name: "bribery".to_string(),
            },
        ];

        let mut vote_counts = HashMap::new();
        vote_counts.insert(0, 2);
        vote_counts.insert(1, 1);

        let event = PlayersFinishedVotingEvent::new(
            source,
            alice,
            votes,
            vote_counts,
            vec!["evidence".to_string(), "bribery".to_string()],
        );

        // Bob voted with Alice (evidence), Charlie voted against
        let voted_with = event.player_tags.get("voted_with_you").unwrap();
        assert_eq!(voted_with.len(), 1);
        assert!(voted_with.contains(&bob));

        let voted_against = event.player_tags.get("voted_against_you").unwrap();
        assert_eq!(voted_against.len(), 1);
        assert!(voted_against.contains(&charlie));
    }

    #[test]
    fn test_player_tags_controller_votes_multiple_options() {
        let source = ObjectId::from_raw(1);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);

        // Alice (controller) votes BOTH evidence AND bribery (2 votes from council's dilemma)
        // Bob votes evidence, Charlie votes bribery
        let votes = vec![
            PlayerVote {
                player: alice,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: alice,
                option_index: 1,
                option_name: "bribery".to_string(),
            },
            PlayerVote {
                player: bob,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: charlie,
                option_index: 1,
                option_name: "bribery".to_string(),
            },
        ];

        let mut vote_counts = HashMap::new();
        vote_counts.insert(0, 2);
        vote_counts.insert(1, 2);

        let event = PlayersFinishedVotingEvent::new(
            source,
            alice,
            votes,
            vote_counts,
            vec!["evidence".to_string(), "bribery".to_string()],
        );

        // Both Bob and Charlie voted with Alice since Alice voted for both options
        let voted_with = event.player_tags.get("voted_with_you").unwrap();
        assert_eq!(voted_with.len(), 2);
        assert!(voted_with.contains(&bob));
        assert!(voted_with.contains(&charlie));

        // No one voted against Alice
        assert!(event.player_tags.get("voted_against_you").is_none());
    }
}
