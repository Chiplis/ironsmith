//! UNVALIDATED implementation-first coverage (cf8 p09): "You choose how each
//! player votes this turn." (Illusion of Choice). For the rest of the turn the
//! effect's controller makes every vote choice; each player still casts their
//! own votes (CR 701.38).
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::effect::Effect;
use ironsmith::effects::{
    ControlVotesThisTurnEffect, EffectContext, VoteEffect, VoteOption, execute_effect,
};
use ironsmith::{GameState, PlayerId};

#[path = "p09_common/mod.rs"]
mod common;

const A: PlayerId = PlayerId::from_index(0);

#[test]
fn illusion_of_choice_compiles_to_vote_control() {
    let rows = common::rows(include_str!("../../../fixtures/vote_control.json.fixture"));
    for definition in common::definitions(common::row(&rows, "Illusion of Choice")) {
        let effects = common::all_effects(&definition);
        assert!(
            effects
                .iter()
                .any(|effect| effect.downcast_ref::<ControlVotesThisTurnEffect>().is_some())
        );
        let text = common::rendered(&definition);
        assert!(text.contains("You choose how each player votes this turn"), "{text}");
    }
}

#[derive(Default)]
struct RecordChoosers(Vec<PlayerId>);

impl DecisionMaker for RecordChoosers {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.0.push(ctx.player);
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
}

#[test]
fn the_vote_controller_chooses_every_vote_this_turn() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let source = game.new_object_id();
    let mut dm = RecordChoosers::default();
    {
        let mut ctx = EffectContext::new(source, A, &mut dm);
        execute_effect(
            &mut game,
            &Effect::new(ControlVotesThisTurnEffect::new()),
            &mut ctx,
        )
        .unwrap();
    }
    assert_eq!(game.vote_controller_this_turn(), Some(A));
    let vote = VoteEffect::basic(vec![
        VoteOption::new("grace", vec![Effect::gain_life(1)]),
        VoteOption::new("condemnation", vec![Effect::gain_life(2)]),
    ]);
    {
        let mut ctx = EffectContext::new(source, PlayerId::from_index(1), &mut dm);
        execute_effect(&mut game, &Effect::new(vote), &mut ctx).unwrap();
    }
    // Three voters, every vote chosen by Alice.
    assert_eq!(dm.0, vec![A, A, A]);
    game.cleanup_vote_control_end_of_turn();
    assert_eq!(game.vote_controller_this_turn(), None);
}
