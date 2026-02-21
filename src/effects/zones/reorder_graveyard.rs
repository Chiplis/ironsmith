//! Reorder graveyard effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that lets a player reorder a player's graveyard.
///
/// Some older cards care about graveyard order and instruct a player to
/// "reorder your graveyard as you choose."
#[derive(Debug, Clone, PartialEq)]
pub struct ReorderGraveyardEffect {
    pub player: PlayerFilter,
}

impl ReorderGraveyardEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    #[allow(dead_code)]
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

fn normalize_order_response(
    response: Vec<crate::ids::ObjectId>,
    original: &[crate::ids::ObjectId],
) -> Vec<crate::ids::ObjectId> {
    let mut remaining = original.to_vec();
    let mut out = Vec::with_capacity(original.len());
    for id in response {
        if let Some(pos) = remaining.iter().position(|x| *x == id) {
            out.push(id);
            remaining.remove(pos);
        }
    }
    out.extend(remaining);
    out
}

impl EffectExecutor for ReorderGraveyardEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::decisions::make_decision;
        use crate::decisions::specs::OrderGraveyardSpec;

        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let graveyard = game
            .player(player_id)
            .map(|p| p.graveyard.clone())
            .unwrap_or_default();

        if graveyard.len() <= 1 {
            return Ok(EffectOutcome::resolved());
        }

        let spec = OrderGraveyardSpec::new(ctx.source, graveyard.clone());
        let ordered = make_decision(game, ctx.decision_maker, player_id, Some(ctx.source), spec);
        let ordered = normalize_order_response(ordered, &graveyard);

        if let Some(player) = game.player_mut(player_id) {
            player.graveyard = ordered;
        }

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
