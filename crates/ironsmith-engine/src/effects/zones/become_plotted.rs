//! The plotted designation on an exiled card.
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

pub use ironsmith_core::BecomePlottedEffect;

impl EffectExecutor for BecomePlottedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = match resolve_objects_for_effect(game, ctx, &self.target) {
            Ok(objects) => objects,
            Err(ExecutionError::InvalidTarget) if !self.target.is_target() => {
                return Ok(EffectOutcome::count(0));
            }
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(error) => return Err(error),
        };
        let mut count = 0;
        for id in objects {
            let Some(card) = game.object(id) else {
                continue;
            };
            // A spell copy is not a card and cannot acquire the designation.
            if card.zone != Zone::Exile || card.kind != crate::object::ObjectKind::Card {
                continue;
            }
            let owner = card.owner;
            if game.plotted_by(id).is_none() {
                game.set_plotted(id, owner);
                count += 1;
            }
        }
        Ok(EffectOutcome::count(count))
    }
    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
    fn target_description(&self) -> &'static str {
        "card to become plotted"
    }
}
