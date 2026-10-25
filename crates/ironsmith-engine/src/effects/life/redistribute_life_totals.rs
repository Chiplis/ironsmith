//! "Redistribute any number of players' life totals" (Reverse the Sands).

use crate::decision::DecisionMaker as _;
use crate::decisions::context::BooleanContext;
use crate::effect::EffectOutcome;
use crate::effects::composition::scoped_choice::{SingleChoicePolicy, select_typed_option};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;

pub use ironsmith_core::RedistributeLifeTotalsEffect;

fn redistribution_target_legal(game: &GameState, player: PlayerId, life: i32) -> bool {
    let current = game.player(player).map(|state| state.life).unwrap_or(0);
    life == current
        || (life > current && game.can_gain_life(player))
        || (life < current && game.can_lose_life(player))
}

fn redistribution_feasible(game: &GameState, players: &[PlayerId], pool: &[i32]) -> bool {
    let Some((first, rest_players)) = players.split_first() else {
        return true;
    };
    pool.iter().enumerate().any(|(slot, life)| {
        redistribution_target_legal(game, *first, *life) && {
            let mut rest = pool.to_vec();
            rest.remove(slot);
            redistribution_feasible(game, rest_players, &rest)
        }
    })
}

impl EffectExecutor for RedistributeLifeTotalsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;

        // Choose any number of players, one yes/no decision per player in
        // seat order.
        let candidates: Vec<PlayerId> = game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect();
        let mut chosen = Vec::new();
        for player in candidates {
            let name = game
                .player(player)
                .map(|state| state.name.clone())
                .unwrap_or_default();
            let decision = BooleanContext::new(
                chooser,
                Some(ctx.source),
                format!("Include {name}'s life total in the redistribution?"),
            );
            let include = ctx.decision_maker.decide_boolean(game, &decision);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if include {
                chosen.push(player);
            }
        }
        if chosen.len() < 2 {
            return Ok(EffectOutcome::resolved());
        }

        // Each chosen player gets one of the chosen players' life totals.
        let mut pool: Vec<i32> = chosen
            .iter()
            .map(|player| game.player(*player).map(|state| state.life).unwrap_or(0))
            .collect();
        let mut assignments = Vec::with_capacity(chosen.len());
        for (index, player) in chosen.iter().enumerate() {
            let new_life = if index + 1 == chosen.len() {
                pool.remove(0)
            } else {
                let name = game
                    .player(*player)
                    .map(|state| state.name.clone())
                    .unwrap_or_default();
                // CR 119.7/119.8: offer only totals that leave a legal
                // assignment for every remaining chosen player.
                let menu = pool
                    .iter()
                    .enumerate()
                    .filter(|(slot, life)| {
                        if !redistribution_target_legal(game, *player, **life) {
                            return false;
                        }
                        let mut rest = pool.clone();
                        rest.remove(*slot);
                        redistribution_feasible(game, &chosen[index + 1..], &rest)
                    })
                    .map(|(slot, life)| (slot, format!("{life} life")))
                    .collect::<Vec<_>>();
                let Some(slot) = select_typed_option(
                    game,
                    ctx,
                    chooser,
                    &format!("Choose the life total {name} gets"),
                    &menu,
                    SingleChoicePolicy::First,
                )?
                else {
                    return Ok(EffectOutcome::count(0));
                };
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                pool.remove(slot)
            };
            assignments.push((*player, new_life));
        }

        // CR 119.5: setting a life total is a gain or loss of the difference.
        let mut proposals = Vec::new();
        for (player, new_life) in assignments {
            let current = game.player(player).map(|state| state.life).unwrap_or(0);
            if new_life > current {
                if !game.can_gain_life(player) {
                    continue;
                }
                proposals.push(crate::events::Event::new_with_provenance(
                    crate::events::LifeGainEvent::new(player, new_life.abs_diff(current))
                        .with_source(ctx.source),
                    ctx.provenance,
                ));
            } else if new_life < current {
                if !game.can_lose_life(player) {
                    continue;
                }
                proposals.push(crate::events::Event::new_with_provenance(
                    crate::events::LifeLossEvent::from_effect(player, current.abs_diff(new_life)),
                    ctx.provenance,
                ));
            }
        }
        if proposals.is_empty() {
            return Ok(EffectOutcome::resolved());
        }
        let mut outcome = super::life_change::execute_life_changes(game, ctx, proposals)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(outcome);
        }
        outcome.set_value(crate::effect::OutcomeValue::None);
        Ok(outcome)
    }
}
