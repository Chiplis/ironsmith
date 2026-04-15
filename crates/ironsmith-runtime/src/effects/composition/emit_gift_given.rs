//! Gift-given event emission effect.
//!
//! Gift needs a reusable trigger-visible event when the promised gift actually
//! resolves, distinct from the cast-time promise choice.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::events::GiftGivenEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct EmitGiftGivenEffect {
    pub recipient: PlayerFilter,
}

impl EmitGiftGivenEffect {
    pub fn new(recipient: PlayerFilter) -> Self {
        Self { recipient }
    }
}

impl EffectExecutor for EmitGiftGivenEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let recipient = resolve_player_filter(game, &self.recipient, ctx)?;
        let event = TriggerEvent::new_with_provenance(
            GiftGivenEvent::new(ctx.controller, recipient, ctx.source),
            ctx.provenance,
        );
        Ok(EffectOutcome::resolved().with_event(event))
    }
}
