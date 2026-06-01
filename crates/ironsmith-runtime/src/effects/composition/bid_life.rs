//! Life-bidding effects.

use crate::decision::DecisionMaker as _;
use crate::decisions::context::{BooleanContext, NumberContext};
use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::events::cause::EventCause;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::target::ChooseSpec;

pub type BidLifeEffect = ironsmith_core::BidLifeEffect<Effect>;
pub use ironsmith_core::LifeBidStart;

fn active_players_in_turn_order(game: &GameState, start: PlayerId) -> Vec<PlayerId> {
    let mut players: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|player| player.is_in_game())
        .map(|player| player.id)
        .collect();

    if let Some(start_pos) = players.iter().position(|&player_id| player_id == start) {
        players.rotate_left(start_pos);
    }
    players
}

impl crate::effects::EffectExecutor for BidLifeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        resolve_single_object_for_effect(game, ctx, &self.target)?;

        let (high_bidder, high_bid) = match self.starting_bid {
            LifeBidStart::Fixed(amount) => {
                let original = ctx.controller;
                let mut high_bidder = original;
                let mut high_bid = amount;
                let players = active_players_in_turn_order(game, original);
                if players.len() > 1 {
                    let mut index = players
                        .iter()
                        .position(|&player_id| player_id == high_bidder)
                        .map(|pos| (pos + 1) % players.len())
                        .unwrap_or(0);
                    let mut passes_since_raise = 0usize;
                    while passes_since_raise < players.len().saturating_sub(1) {
                        let player_id = players[index];
                        let chosen_bid = if high_bid < u32::MAX {
                            let top_decision = BooleanContext::new(
                                player_id,
                                Some(ctx.source),
                                format!("Top the high bid of {high_bid} life?"),
                            );
                            if !ctx.decision_maker.decide_boolean(game, &top_decision) {
                                0
                            } else {
                                let min_bid = high_bid.saturating_add(1);
                                let decision = NumberContext::new(
                                    player_id,
                                    Some(ctx.source),
                                    min_bid,
                                    u32::MAX,
                                    format!("Choose a life bid greater than {high_bid}"),
                                );
                                ctx.decision_maker.decide_number(game, &decision)
                            }
                        } else {
                            0
                        };
                        if ctx.decision_maker.awaiting_choice() {
                            break;
                        }
                        if chosen_bid > high_bid {
                            high_bidder = player_id;
                            high_bid = chosen_bid;
                            passes_since_raise = 0;
                        } else {
                            passes_since_raise += 1;
                        }
                        index = (index + 1) % players.len();
                    }
                }
                (high_bidder, high_bid)
            }
        };

        let mut outcomes = Vec::new();
        outcomes.push(execute_effect(
            game,
            &Effect::new(crate::effects::LoseLifeEffect::new(
                high_bid,
                ChooseSpec::SpecificPlayer(high_bidder),
            )),
            ctx,
        )?);

        let saved_controller = ctx.controller;
        let saved_cause = ctx.cause.clone();
        ctx.controller = high_bidder;
        ctx.cause = EventCause::from_effect(ctx.source, high_bidder);
        let winner_result = (|| {
            let mut winner_outcomes = Vec::new();
            for effect in &self.winner_effects {
                winner_outcomes.push(execute_effect(game, effect, ctx)?);
            }
            Ok::<Vec<EffectOutcome>, ExecutionError>(winner_outcomes)
        })();
        ctx.controller = saved_controller;
        ctx.cause = saved_cause;
        outcomes.extend(winner_result?);

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.winner_effects {
            visitor(effect);
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to auction"
    }
}
