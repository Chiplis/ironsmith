//! "If that spell would be put into a graveyard, put it on the bottom of its
//! owner's library instead." (Quintorius, Loremaster): the replacement
//! program of a zone replacement that followed a card onto the stack. The
//! object to move is the one in the replaced zone-change event (CR 614.1a),
//! which is a different object from the card the replacement was created
//! for (CR 400.7), so it is read from the replaced event, never captured.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct MoveReplacedObjectToLibraryEffect {
    pub placement: ironsmith_core::ZoneReplacementLibraryPlacement,
}

impl MoveReplacedObjectToLibraryEffect {
    pub fn new(placement: ironsmith_core::ZoneReplacementLibraryPlacement) -> Self {
        Self { placement }
    }
}

impl EffectExecutor for MoveReplacedObjectToLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(event) = ctx.replacement.original_zone_event.clone() else {
            return Ok(EffectOutcome::count(0));
        };
        let objects = event
            .objects
            .iter()
            .copied()
            .filter(|id| game.object(*id).is_some_and(|object| object.zone == event.from))
            .collect::<Vec<_>>();
        let mut moved = Vec::new();
        for object in objects {
            let target = ChooseSpec::SpecificObject(object);
            let effect = match self.placement {
                ironsmith_core::ZoneReplacementLibraryPlacement::Top => {
                    Effect::move_to_zone(target, Zone::Library, true)
                }
                ironsmith_core::ZoneReplacementLibraryPlacement::Bottom => {
                    Effect::move_to_zone(target, Zone::Library, false)
                }
                ironsmith_core::ZoneReplacementLibraryPlacement::TopOrBottom => Effect::new(
                    crate::effects::MoveToLibraryTopOrBottomChoiceEffect::new(target)
                        .with_chooser(PlayerFilter::You),
                ),
            };
            let outcome = crate::effects::execute_effect(game, &effect, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if !outcome.status.is_failure() {
                moved.push(object);
            }
        }
        Ok(EffectOutcome::with_objects(moved))
    }
}
