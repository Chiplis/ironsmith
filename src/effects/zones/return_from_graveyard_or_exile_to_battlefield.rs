//! Return from graveyard or exile to battlefield effect implementation.

use crate::effect::{Effect, EffectOutcome, EffectResult};
use crate::effects::{EffectExecutor, PutOntoBattlefieldEffect};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::target::{ChooseSpec, ObjectRef, PlayerFilter};
use crate::zone::Zone;

/// Effect that returns a card from graveyard or exile to the battlefield.
///
/// This uses the triggering event's snapshot stable_id to find the card,
/// so it works correctly with zone changes.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardOrExileToBattlefieldEffect {
    /// Whether the permanent enters tapped.
    pub tapped: bool,
}

impl ReturnFromGraveyardOrExileToBattlefieldEffect {
    pub fn new(tapped: bool) -> Self {
        Self { tapped }
    }

    /// Find a card in the graveyard or exile by stable_id.
    fn find_by_stable_id(
        game: &GameState,
        owner: PlayerId,
        stable_id: StableId,
    ) -> Option<ObjectId> {
        let in_graveyard = game.player(owner).and_then(|p| {
            p.graveyard
                .iter()
                .find(|&&id| {
                    game.object(id)
                        .is_some_and(|obj| obj.stable_id == stable_id)
                })
                .copied()
        });

        if in_graveyard.is_some() {
            return in_graveyard;
        }

        game.exile.iter().find_map(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.stable_id == stable_id && obj.owner == owner)
                .then_some(id)
        })
    }
}

impl EffectExecutor for ReturnFromGraveyardOrExileToBattlefieldEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(ref triggering_event) = ctx.triggering_event else {
            return Err(ExecutionError::Impossible(
                "ReturnFromGraveyardOrExileToBattlefieldEffect requires a triggering event"
                    .to_string(),
            ));
        };

        let Some(snapshot) = triggering_event.snapshot() else {
            return Err(ExecutionError::Impossible(
                "Triggering event has no snapshot".to_string(),
            ));
        };

        let stable_id = snapshot.stable_id;
        let owner = snapshot.owner;

        let Some(target_id) = Self::find_by_stable_id(game, owner, stable_id) else {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        };

        let original_targets = ctx.targets.clone();
        ctx.targets = vec![ResolvedTarget::Object(target_id)];

        let zone = game
            .object(target_id)
            .map(|obj| obj.zone)
            .unwrap_or(Zone::Graveyard);
        let target_spec = ChooseSpec::card_in_zone(zone);

        let put_effect = PutOntoBattlefieldEffect::new(
            target_spec,
            self.tapped,
            PlayerFilter::OwnerOf(ObjectRef::Target),
        );
        let outcome = execute_effect(game, &Effect::new(put_effect), ctx)?;

        ctx.targets = original_targets;

        let EffectOutcome { result, events } = outcome;

        match result {
            // Preserve prior behavior: ETB prevented is treated as TargetInvalid.
            EffectResult::Impossible => Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
            other => Ok(EffectOutcome::from_result(other).with_events(events)),
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
