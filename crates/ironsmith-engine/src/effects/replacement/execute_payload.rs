//! Execute an instead-payload without losing its event or replacement history.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::events::processing::ReplacementEventContext;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

pub(crate) fn execute_replacement_payload(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    effects: &[Effect],
    source: ObjectId,
    controller: PlayerId,
    context: &ReplacementEventContext,
    targets: Option<Vec<crate::effects::ResolvedTarget>>,
) -> Result<EffectOutcome, ExecutionError> {
    let affected_player = context.event.inner().affected_player(game);
    let inherited_replacements = parent.replacement.clone();
    let source_snapshot = game
        .object(source)
        .filter(|_| !game.is_phased_out(source))
        .map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, game,
            )
        })
        .or_else(|| {
            game.turn_store
                .turn_history
                .departed_object_snapshot(source)
                .cloned()
        })
        .or_else(|| {
            parent
                .source_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.object_id == source)
                .cloned()
        });
    // A replacement has its own source/controller and program scope. Inherit
    // the event history, not the interrupted instruction's local outcomes.
    let mut child = ExecutionContext::new(source, controller, &mut *parent.decision_maker);
    child.source_snapshot = source_snapshot;
    child.replacement = inherited_replacements;
    child.iteration.iterated_player = Some(affected_player);
    child.targets =
        targets.unwrap_or_else(|| vec![crate::effects::ResolvedTarget::Player(affected_player)]);
    context.apply_to(&mut child);
    let mut outcomes = Vec::new();
    for effect in effects {
        outcomes.push(execute_effect(game, effect, &mut child)?);
        if child.decision_maker.awaiting_choice() {
            break;
        }
    }
    Ok(EffectOutcome::aggregate(outcomes))
}
