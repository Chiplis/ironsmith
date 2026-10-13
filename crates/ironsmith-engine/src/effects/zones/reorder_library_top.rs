//! Reorder top of library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub type ReorderLibraryTopEffect = ironsmith_core::ReorderLibraryTopEffect;

use crate::effects::cards::normalize_order_response;

impl EffectExecutor for ReorderLibraryTopEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::decisions::make_decision;
        use crate::decisions::specs::OrderLibraryTopSpec;

        let Some(snapshots) = ctx.tagged_objects.get(&self.tag) else {
            return Ok(EffectOutcome::resolved());
        };

        let Some((library_owner, current_top_to_bottom)) =
            crate::effects::cards::capture_retained_library_set(game, snapshots)
        else { return Ok(EffectOutcome::resolved()); };
        if current_top_to_bottom.len() <= 1 {
            return Ok(EffectOutcome::resolved());
        }

        let spec = OrderLibraryTopSpec::new(ctx.source, current_top_to_bottom.clone());
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let ordered = make_decision(
            game,
            &mut ctx.decision_maker,
            chooser,
            Some(ctx.source),
            spec,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let ordered = normalize_order_response(ordered, &current_top_to_bottom);

        crate::effects::cards::arrange_library_cards(
            game,
            library_owner,
            &ordered,
            &[],
            "reordered top of library",
        );

        Ok(EffectOutcome::resolved())
    }
}
