//! A linked face is a retained definition, not a name-resolved token recipe.
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::CreateLinkedFaceTokenEffect;
impl EffectExecutor for CreateLinkedFaceTokenEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let empty = || {
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                Vec::new(),
            ))
        };
        let card = game
            .object(ctx.source)
            .and_then(|source| source.card)
            .or_else(|| ctx.source_snapshot.as_ref().and_then(|source| source.card));
        let Some(native) = card
            .and_then(|id| game.retained_card_definition(id))
            .cloned()
        else {
            return Ok(empty());
        };
        let Some(other_id) = native.card.other_face else {
            return Ok(empty());
        };
        let Some(other) = game.retained_card_definition(other_id).cloned() else {
            return Ok(empty());
        };
        if other.card.other_face != Some(native.card.id) {
            return Ok(empty());
        }
        let back = if game.linked_face_is_back(native.card.id, other_id) {
            native
        } else {
            other
        };
        if !back.card.subtypes.contains(&self.required_subtype) {
            return Ok(empty());
        }
        let mut create = super::CreateTokenEffect::one(back);
        create.suppress_aura_attachment_choice = true;
        let previous = ctx
            .pending_entry_attachment
            .replace(self.attached_to.clone());
        let result = create.execute_with_outputs(game, ctx);
        ctx.pending_entry_attachment = previous;
        result
    }
}
