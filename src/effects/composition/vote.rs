//! Vote effect implementation for council's dilemma and similar mechanics.

use std::collections::{BTreeMap, HashMap};

use crate::decision::FallbackStrategy;
use crate::decisions::spec::DisplayOption;
use crate::decisions::specs::ChoiceSpec;
use crate::decisions::{make_boolean_decision, make_decision};
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, InvestigateEffect};
use crate::events::{
    EventCause, EventKind, KeywordActionEvent, KeywordActionKind, PlayerVote,
    PlayersFinishedVotingEvent, ZoneChangeEvent,
};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::ObjectKind;
use crate::tag::TagKey;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

fn option_vote_tag(option_name: &str) -> TagKey {
    let slug = option_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    TagKey::new(format!("voted_for:{}", slug))
}

/// A vote option with a name and effects to execute per vote.
#[derive(Debug, Clone, PartialEq)]
pub struct VoteOption {
    /// Name of this vote option (e.g., "evidence", "bribery").
    pub name: String,
    /// Effects to execute once per vote for this option.
    /// For example, "investigate" for evidence, or "create a Treasure token" for bribery.
    pub effects_per_vote: Vec<Effect>,
}

impl VoteOption {
    /// Create a new vote option.
    pub fn new(name: impl Into<String>, effects: Vec<Effect>) -> Self {
        Self {
            name: name.into(),
            effects_per_vote: effects,
        }
    }
}

/// Effect that implements council's dilemma and similar voting mechanics.
///
/// Each player votes for one of the options. The controller can get extra votes.
/// After all votes are cast, effects are executed based on vote counts.
///
/// # Example
///
/// ```ignore
/// // Tivit's council's dilemma
/// let vote = VoteEffect::new(
///     vec![
///         VoteOption::new("evidence", vec![Effect::investigate()]),
///         VoteOption::new("bribery", vec![Effect::create_tokens(treasure_token(), 1)]),
///     ],
///     1, // Controller gets 1 extra vote
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct VoteEffect {
    /// Available vote options.
    pub options: Vec<VoteOption>,
    /// Mandatory extra votes for the controller.
    pub controller_extra_votes: u32,
    /// Optional extra votes for the controller (e.g., "you may vote an additional time" = 1).
    pub controller_optional_extra_votes: u32,
}

impl VoteEffect {
    /// Create a new vote effect.
    pub fn new(options: Vec<VoteOption>, controller_extra_votes: u32) -> Self {
        Self {
            options,
            controller_extra_votes,
            controller_optional_extra_votes: 0,
        }
    }

    /// Create a vote effect with optional extra votes for the controller.
    pub fn with_optional_extra(
        options: Vec<VoteOption>,
        controller_extra_votes: u32,
        controller_optional_extra_votes: u32,
    ) -> Self {
        Self {
            options,
            controller_extra_votes,
            controller_optional_extra_votes,
        }
    }

    /// Create a vote effect with no extra votes for the controller.
    pub fn basic(options: Vec<VoteOption>) -> Self {
        Self::new(options, 0)
    }

    /// Create a council's dilemma vote effect (controller may vote an additional time).
    pub fn councils_dilemma(options: Vec<VoteOption>) -> Self {
        Self::with_optional_extra(options, 0, 1)
    }
}

impl EffectExecutor for VoteEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.options.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let controller = ctx.controller;

        // Get all players in turn order, starting with controller
        let players: Vec<PlayerId> = {
            let mut all_players: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.is_in_game())
                .map(|p| p.id)
                .collect();

            // Reorder to start with controller
            if let Some(controller_pos) = all_players.iter().position(|&p| p == controller) {
                all_players.rotate_left(controller_pos);
            }

            all_players
        };

        // Build display options for the vote
        let display_options: Vec<DisplayOption> = self
            .options
            .iter()
            .enumerate()
            .map(|(i, opt)| DisplayOption::new(i, &opt.name))
            .collect();

        // Track individual votes and vote counts
        let mut votes: Vec<PlayerVote> = Vec::new();
        let mut vote_counts: Vec<usize> = vec![0; self.options.len()];

        // Each player votes
        for player_id in &players {
            // Determine how many votes this player gets
            let mut num_votes = 1;
            if *player_id == controller {
                num_votes += self.controller_extra_votes as usize;
                for _ in 0..self.controller_optional_extra_votes {
                    let wants_extra = make_boolean_decision(
                        game,
                        &mut ctx.decision_maker,
                        *player_id,
                        ctx.source,
                        "vote an additional time",
                        FallbackStrategy::Decline,
                    );
                    if wants_extra {
                        num_votes += 1;
                    }
                }
            }

            // Each vote is a separate decision
            for _vote_num in 0..num_votes {
                // Create a spec for this vote
                // Note: We're using ChoiceSpec but the player making the decision
                // is the voting player, not necessarily the controller
                let spec = ChoiceSpec::single(ctx.source, display_options.clone());

                // We need to make this decision as the voting player
                let chosen = make_decision(
                    game,
                    &mut ctx.decision_maker,
                    *player_id,
                    Some(ctx.source),
                    spec,
                );

                // Record the vote
                if let Some(&vote_index) = chosen.first()
                    && vote_index < vote_counts.len()
                {
                    vote_counts[vote_index] += 1;
                    votes.push(PlayerVote {
                        player: *player_id,
                        option_index: vote_index,
                        option_name: self.options[vote_index].name.clone(),
                    });
                }
            }
        }

        // Build vote counts as HashMap for the event
        let vote_counts_map: HashMap<usize, usize> = vote_counts
            .iter()
            .enumerate()
            .filter(|(_, count)| **count > 0)
            .map(|(idx, count)| (idx, *count))
            .collect();

        // Emit the PlayersFinishedVotingEvent
        let option_names: Vec<String> = self.options.iter().map(|o| o.name.clone()).collect();
        let mut extra_tags: HashMap<TagKey, Vec<PlayerId>> = HashMap::new();
        for (option_index, option) in self.options.iter().enumerate() {
            let mut voters = votes
                .iter()
                .filter(|vote| vote.option_index == option_index)
                .map(|vote| vote.player)
                .collect::<Vec<_>>();
            if voters.is_empty() {
                continue;
            }
            voters.sort_by_key(|p| p.0);
            voters.dedup();
            extra_tags.insert(option_vote_tag(&option.name), voters);
        }

        let voting_event = PlayersFinishedVotingEvent::new(
            ctx.source,
            controller,
            votes.clone(),
            vote_counts_map,
            option_names,
        )
        .with_player_tags(extra_tags);

        // Emit a keyword-action event for action-based trigger composition.
        let vote_action_event = KeywordActionEvent::new(
            KeywordActionKind::Vote,
            controller,
            ctx.source,
            votes.len() as u32,
        )
        .with_votes(votes.clone())
        .with_player_tags(
            voting_event
                .player_tags
                .iter()
                .filter_map(|(tag, players)| {
                    if tag.as_str() == "voted_with_you" || tag.as_str() == "voted_against_you" {
                        None
                    } else {
                        Some((tag.clone(), players.clone()))
                    }
                })
                .collect(),
        );
        game.queue_trigger_event(TriggerEvent::new(vote_action_event));

        // Queue the voting event for trigger checking by the game loop.
        // This allows triggers like Model of Unity's "whenever players finish voting"
        // to fire and be processed by the normal trigger queuing flow.
        let trigger_event = TriggerEvent::new(voting_event);
        game.queue_trigger_event(trigger_event);

        // Execute effects for each vote, setting iterated_player to the voter.
        // This enables effects like Expropriate's "choose a permanent owned by the voter".
        let mut outcomes = Vec::new();
        let original_iterated_player = ctx.iterated_player;
        let mut token_batches: Vec<BTreeMap<PlayerId, Vec<ObjectId>>> =
            vec![BTreeMap::new(); self.options.len()];

        for vote in &votes {
            if let Some(option) = self.options.get(vote.option_index) {
                // Set iterated_player to the voter for this vote
                ctx.iterated_player = Some(vote.player);

                for effect in &option.effects_per_vote {
                    let is_investigate = effect.downcast_ref::<InvestigateEffect>().is_some();
                    let mut outcome = execute_effect(game, effect, ctx)?;

                    if !is_investigate && !outcome.events.is_empty() {
                        let mut filtered_events = Vec::with_capacity(outcome.events.len());
                        let batches = token_batches
                            .get_mut(vote.option_index)
                            .expect("vote option index should be valid");

                        for event in outcome.events {
                            if event.kind() == EventKind::ZoneChange
                                && let Some(zc) = event.downcast::<ZoneChangeEvent>()
                                && zc.to == Zone::Battlefield
                                && zc.objects.iter().all(|&id| {
                                    game.object(id)
                                        .map(|obj| matches!(obj.kind, ObjectKind::Token))
                                        .unwrap_or(false)
                                })
                            {
                                for &id in &zc.objects {
                                    if let Some(obj) = game.object(id) {
                                        batches.entry(obj.controller).or_default().push(id);
                                    }
                                }
                                continue;
                            }

                            filtered_events.push(event);
                        }

                        outcome.events = filtered_events;
                    }

                    outcomes.push(outcome);
                }
            }
        }

        // Restore the original iterated_player
        ctx.iterated_player = original_iterated_player;

        let mut outcome = EffectOutcome::aggregate(outcomes);
        let cause = EventCause::from_effect(ctx.source, ctx.controller);

        for by_controller in token_batches {
            for (_controller, mut ids) in by_controller {
                if ids.is_empty() {
                    continue;
                }
                ids.sort();
                ids.dedup();
                outcome
                    .events
                    .push(TriggerEvent::new(ZoneChangeEvent::batch(
                        ids,
                        Zone::Stack,
                        Zone::Battlefield,
                        cause.clone(),
                    )));
            }
        }

        Ok(outcome)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::AutoPassDecisionMaker;
    use crate::decision::DecisionMaker;
    use crate::effect::EffectResult;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn setup_multiplayer_game() -> GameState {
        GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        )
    }

    struct AcceptExtraVoteDecisionMaker;
    impl DecisionMaker for AcceptExtraVoteDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    #[test]
    fn test_vote_effect_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Simple vote: gain life options
        let vote = VoteEffect::basic(vec![
            VoteOption::new("option_a", vec![Effect::gain_life(1)]),
            VoteOption::new("option_b", vec![Effect::gain_life(2)]),
        ]);

        let initial_life = game.player(alice).unwrap().life;

        // With AutoPassDecisionMaker, players vote for first option
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        vote.execute(&mut game, &mut ctx).unwrap();

        // Both players voted for option_a (gain 1 life), so 2 votes = 2 life gained
        assert_eq!(game.player(alice).unwrap().life, initial_life + 2);
    }

    #[test]
    fn test_vote_effect_with_extra_votes() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Council's dilemma: controller gets 1 extra vote
        let vote = VoteEffect::councils_dilemma(vec![
            VoteOption::new("option_a", vec![Effect::gain_life(1)]),
            VoteOption::new("option_b", vec![Effect::gain_life(2)]),
        ]);

        let initial_life = game.player(alice).unwrap().life;

        let mut dm = AcceptExtraVoteDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        vote.execute(&mut game, &mut ctx).unwrap();

        // Alice votes twice (1 + 1 optional extra), Bob votes once
        // All vote for first option = 3 votes for option_a
        // 3 votes * 1 life = 3 life gained
        assert_eq!(game.player(alice).unwrap().life, initial_life + 3);
    }

    #[test]
    fn test_vote_effect_multiplayer() {
        let mut game = setup_multiplayer_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Basic vote with 3 players
        let vote = VoteEffect::basic(vec![
            VoteOption::new("option_a", vec![Effect::gain_life(1)]),
            VoteOption::new("option_b", vec![Effect::gain_life(2)]),
        ]);

        let initial_life = game.player(alice).unwrap().life;

        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        vote.execute(&mut game, &mut ctx).unwrap();

        // 3 players, each votes once for option_a = 3 life gained
        assert_eq!(game.player(alice).unwrap().life, initial_life + 3);
    }

    #[test]
    fn test_vote_effect_empty_options() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let vote = VoteEffect::basic(vec![]);

        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let result = vote.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_vote_effect_clone_box() {
        let vote = VoteEffect::councils_dilemma(vec![VoteOption::new(
            "option_a",
            vec![Effect::gain_life(1)],
        )]);
        let cloned = vote.clone_box();
        assert!(format!("{:?}", cloned).contains("VoteEffect"));
    }
}
