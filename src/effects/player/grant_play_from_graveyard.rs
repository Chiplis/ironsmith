//! Grant play-from-graveyard effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant::Grantable;
use crate::grant_registry::GrantSource;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Effect that grants "you may play cards from your graveyard" until end of turn.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantPlayFromGraveyardEffect {
    pub player: PlayerFilter,
}

impl GrantPlayFromGraveyardEffect {
    /// Create a new effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Grant to you.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for GrantPlayFromGraveyardEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let grant_source = GrantSource::Effect {
            source_id: ctx.source,
            expires_end_of_turn: game.turn.turn_number,
        };

        game.grant_registry.grant_to_filter(
            crate::target::ObjectFilter::default(),
            Zone::Graveyard,
            player_id,
            Grantable::PlayFrom,
            grant_source,
        );

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
