//! Repeated coin flips for doubling life stakes (Game of Chaos).

use crate::decision::DecisionMaker as _;
use crate::decisions::context::BooleanContext;
use crate::effect::{Effect, EffectOutcome, ExecutionFact};
use crate::effects::helpers::resolve_player_from_spec;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};

pub use ironsmith_core::CoinFlipLifeStakesEffect;

/// The winner of the single called flip an instruction just completed.
fn flip_winner(outcome: &EffectOutcome) -> Option<crate::ids::PlayerId> {
    outcome
        .instruction_result()
        .execution_facts
        .iter()
        .find_map(|fact| match fact {
            ExecutionFact::CoinFlip { winner, .. } => Some(*winner),
            ExecutionFact::CoinFlips(flips) => flips.first().map(|flip| flip.winner),
            _ => None,
        })
        .flatten()
}

impl EffectExecutor for CoinFlipLifeStakesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller = ctx.controller;
        let opponent = resolve_player_from_spec(game, &self.opponent, ctx)?;
        let mut stakes = i32::try_from(self.initial_stakes).unwrap_or(i32::MAX);
        let mut outcomes = Vec::new();
        loop {
            // The controller flips each time (CR 705); the stakes move
            // between the two players according to who won.
            let flip = Effect::new(crate::effects::FlipCoinEffect::new(PlayerFilter::Specific(
                controller,
            )));
            let flipped = crate::effects::execute_effect(game, &flip, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let won = flip_winner(&flipped) == Some(controller);
            outcomes.push(flipped);
            let (gainer, loser) = if won {
                (controller, opponent)
            } else {
                (opponent, controller)
            };
            let gain = Effect::new(crate::effects::GainLifeEffect::new(
                stakes,
                ChooseSpec::Player(PlayerFilter::Specific(gainer)),
            ));
            outcomes.push(crate::effects::execute_effect(game, &gain, ctx)?);
            let lose = Effect::new(crate::effects::LoseLifeEffect::new(
                crate::effect::Value::Fixed(stakes),
                PlayerFilter::Specific(loser),
            ));
            outcomes.push(crate::effects::execute_effect(game, &lose, ctx)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            // The flip's winner decides whether to flip again.
            let decider = if won { controller } else { opponent };
            if !game.player(decider).is_some_and(|player| player.is_in_game()) {
                break;
            }
            let next_stakes = stakes.saturating_mul(2);
            let decision = BooleanContext::new(
                decider,
                Some(ctx.source),
                format!("Flip again for {next_stakes} life?"),
            );
            let again = ctx.decision_maker.decide_boolean(game, &decision);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if !again {
                break;
            }
            // Double the life stakes with each flip.
            stakes = next_stakes;
        }
        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.opponent.is_target().then_some(&self.opponent)
    }

    fn target_description(&self) -> &'static str {
        "opponent"
    }
}
