//! Recheck eligibility after replacement-aware creation, then choose one object.
//!
//! The caller owns the enclosing instruction transaction and completion event.
//! No state-based actions run between creation and the caller's modification.
use crate::decisions::{make_decision, specs::ChooseObjectsSpec};
use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

pub(crate) fn ensure_chosen_object(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    description: &str,
    candidates: impl Fn(&GameState, PlayerId) -> Vec<ObjectId>,
    create: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError>,
) -> Result<(Option<ObjectId>, Vec<CompletedEffectOutputs>), ExecutionError> {
    let mut objects = candidates(game, ctx.controller);
    let mut children = Vec::new();
    if objects.is_empty() {
        children.push(create(game, ctx)?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok((None, children));
        }
        objects = candidates(game, ctx.controller);
    }
    let chosen = match objects.as_slice() {
        [] => None,
        [object] => Some(*object),
        _ => {
            let spec = ChooseObjectsSpec::new(ctx.source, description, objects.clone(), 1, Some(1));
            let selected = make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok((None, children));
            }
            Some(
                *crate::effects::helpers::normalize_object_selection(selected, &objects, 1)
                    .first()
                    .ok_or(ExecutionError::InvalidTarget)?,
            )
        }
    };
    Ok((chosen, children))
}

pub(crate) fn aggregate_children(children: Vec<CompletedEffectOutputs>) -> CompletedEffectOutputs {
    let summary = EffectOutcome::aggregate(children.iter().map(|child| child.outcome.clone()));
    CompletedEffectOutputs::with_primary_result(summary, children)
}
