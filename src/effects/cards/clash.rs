//! Clash effect implementation.
//!
//! Clash (701.30): You and an opponent each reveal the top card of your library,
//! then each may put that card on the bottom of their library. A player wins if
//! their revealed card has greater mana value.

use crate::decisions::{ChoiceSpec, DisplayOption, ScrySpec, make_decision};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClashOpponentMode {
    AnyOpponent,
    TargetOpponent,
    DefendingPlayer,
}

/// Effect that performs a clash with an opponent.
#[derive(Debug, Clone, PartialEq)]
pub struct ClashEffect {
    opponent_mode: ClashOpponentMode,
}

impl ClashEffect {
    pub fn new(opponent_mode: ClashOpponentMode) -> Self {
        Self { opponent_mode }
    }

    pub fn against_any_opponent() -> Self {
        Self::new(ClashOpponentMode::AnyOpponent)
    }

    pub fn against_target_opponent() -> Self {
        Self::new(ClashOpponentMode::TargetOpponent)
    }

    pub fn against_defending_player() -> Self {
        Self::new(ClashOpponentMode::DefendingPlayer)
    }
}

fn in_game_opponents(game: &GameState, controller: PlayerId) -> Vec<PlayerId> {
    game.players
        .iter()
        .filter(|player| player.id != controller && player.is_in_game())
        .map(|player| player.id)
        .collect()
}

fn targeted_opponent(ctx: &ExecutionContext, opponents: &[PlayerId]) -> Option<PlayerId> {
    ctx.targets.iter().find_map(|target| match target {
        ResolvedTarget::Player(player) if opponents.contains(player) => Some(*player),
        _ => None,
    })
}

fn choose_opponent(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    opponents: &[PlayerId],
    mode: ClashOpponentMode,
) -> Option<PlayerId> {
    match mode {
        ClashOpponentMode::TargetOpponent => {
            return targeted_opponent(ctx, opponents);
        }
        ClashOpponentMode::DefendingPlayer => {
            return ctx
                .defending_player
                .filter(|player_id| opponents.contains(player_id));
        }
        ClashOpponentMode::AnyOpponent => {}
    }

    if opponents.is_empty() {
        return None;
    }
    if opponents.len() == 1 {
        return opponents.first().copied();
    }

    let options: Vec<DisplayOption> = opponents
        .iter()
        .enumerate()
        .map(|(index, player_id)| {
            let name = game
                .player(*player_id)
                .map(|player| player.name.clone())
                .unwrap_or_else(|| format!("Player {}", player_id.0));
            DisplayOption::new(index, name)
        })
        .collect();

    let spec = ChoiceSpec::single(ctx.source, options);
    let chosen = make_decision(
        game,
        &mut ctx.decision_maker,
        ctx.controller,
        Some(ctx.source),
        spec,
    );
    let index = chosen.first().copied().unwrap_or(0);
    opponents
        .get(index)
        .copied()
        .or_else(|| opponents.first().copied())
}

fn top_card(game: &GameState, player: PlayerId) -> Option<ObjectId> {
    game.player(player)
        .and_then(|entry| entry.library.last().copied())
}

fn card_mana_value(game: &GameState, card: Option<ObjectId>) -> Option<u32> {
    card.and_then(|card_id| {
        game.object(card_id).map(|object| {
            object
                .mana_cost
                .as_ref()
                .map_or(0, |cost| cost.mana_value())
        })
    })
}

fn maybe_put_revealed_card_on_bottom(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    card: ObjectId,
) {
    let spec = ScrySpec::new(ctx.source, vec![card]);
    let to_bottom: Vec<ObjectId> = make_decision(
        game,
        &mut ctx.decision_maker,
        player,
        Some(ctx.source),
        spec,
    );

    if !to_bottom.contains(&card) {
        return;
    }

    if let Some(player_state) = game.player_mut(player)
        && let Some(pos) = player_state.library.iter().position(|id| *id == card)
    {
        player_state.library.remove(pos);
        player_state.library.insert(0, card);
    }
}

fn controller_wins_clash(controller_mv: Option<u32>, opponent_mv: Option<u32>) -> bool {
    match (controller_mv, opponent_mv) {
        (Some(left), Some(right)) => left > right,
        (Some(_), None) => true,
        _ => false,
    }
}

impl EffectExecutor for ClashEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let opponents = in_game_opponents(game, ctx.controller);
        let Some(opponent) = choose_opponent(game, ctx, &opponents, self.opponent_mode) else {
            return Ok(EffectOutcome::count(0));
        };

        let controller_card = top_card(game, ctx.controller);
        let opponent_card = top_card(game, opponent);

        let did_win = controller_wins_clash(
            card_mana_value(game, controller_card),
            card_mana_value(game, opponent_card),
        );

        if let Some(card) = controller_card {
            maybe_put_revealed_card_on_bottom(game, ctx, ctx.controller, card);
        }
        if let Some(card) = opponent_card {
            maybe_put_revealed_card_on_bottom(game, ctx, opponent, card);
        }

        Ok(EffectOutcome::count(if did_win { 1 } else { 0 }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
