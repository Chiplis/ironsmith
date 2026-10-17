//! The plotted designation on an exiled card.
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

pub use ironsmith_core::BecomePlottedEffect;
pub use ironsmith_core::BecomeForetoldEffect;

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

impl EffectExecutor for BecomeForetoldEffect {
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
            // CR 702.143: only a card in exile can be foretold.
            if card.zone != Zone::Exile || card.kind != crate::object::ObjectKind::Card {
                continue;
            }
            if !game.is_foretold(id) {
                game.set_foretold(id);
                count += 1;
            }
            // "Its foretell cost is its mana cost reduced by {N}." The card
            // keeps this foretell cost while it stays in exile; a face-down
            // card's own mana cost is the one saved by its face-down state.
            if let Some(reduction) = self.foretell_cost_reduction
                && let Some(obj) = game.object_mut(id)
            {
                let mana_cost = obj
                    .face_down_cast_state
                    .as_ref()
                    .and_then(|state| state.mana_cost.as_ref())
                    .map(|cost| crate::object::SharedValue::to_owned_value(cost))
                    .or_else(|| obj.mana_cost_owned());
                if let Some(mana_cost) = mana_cost {
                    obj.alternative_casts.retain(|method| {
                        !matches!(
                            method,
                            crate::alternative_cast::AlternativeCastingMethod::Foretell { .. }
                        )
                    });
                    obj.alternative_casts.push(
                        crate::alternative_cast::AlternativeCastingMethod::Foretell {
                            cost: mana_cost.reduce_generic(reduction),
                        },
                    );
                }
            }
        }
        Ok(EffectOutcome::count(count))
    }
    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
    fn target_description(&self) -> &'static str {
        "card to become foretold"
    }
}
