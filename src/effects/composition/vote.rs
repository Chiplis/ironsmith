//! Vote effect implementation for council's dilemma and similar mechanics.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

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
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::vote_runtime::run_vote(self, game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        let groups: Vec<&[Effect]> = self
            .options
            .iter()
            .map(|option| option.effects_per_vote.as_slice())
            .collect();
        super::target_metadata::first_target_spec(&groups)
    }

    fn target_description(&self) -> &'static str {
        let groups: Vec<&[Effect]> = self
            .options
            .iter()
            .map(|option| option.effects_per_vote.as_slice())
            .collect();
        super::target_metadata::first_target_description(&groups, "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        let groups: Vec<&[Effect]> = self
            .options
            .iter()
            .map(|option| option.effects_per_vote.as_slice())
            .collect();
        super::target_metadata::first_target_count(&groups)
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
        crate::tests::test_helpers::setup_two_player_game()
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

    #[test]
    fn vote_effect_forwards_nested_target_metadata() {
        let vote = VoteEffect::basic(vec![
            VoteOption::new("counter", vec![Effect::counter(ChooseSpec::target_spell())]),
            VoteOption::new("life", vec![Effect::gain_life(2)]),
        ]);

        assert!(vote.get_target_spec().is_some());
        assert_eq!(vote.target_description(), "spell to counter");
    }
}
