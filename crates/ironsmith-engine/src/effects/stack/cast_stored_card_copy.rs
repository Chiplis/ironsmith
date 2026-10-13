//! Create a card copy from captured copiable characteristics and offer an
//! immediate free cast. The original card need not remain in its old zone.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::Object;
use crate::target::PlayerFilter;

#[derive(Debug, Clone)]
pub(crate) struct CastStoredCardCopyEffect {
    card: Object,
}
impl CastStoredCardCopyEffect {
    pub(crate) fn new(card: &Object) -> Self {
        Self { card: card.clone() }
    }
}
impl EffectExecutor for CastStoredCardCopyEffect {
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
        let tag = crate::tag::TagKey::from("__stored_card_copy__");
        let effect = Effect::may(vec![Effect::cast_tagged(
            tag.clone(),
            PlayerFilter::You,
            false,
            false,
            true,
            None,
        )]);
        crate::effects::composition::temporary_card::with_temporary_card_copy(
            game, ctx, &self.card, tag, &effect,
        )
    }
}
