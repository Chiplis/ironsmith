use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::game_state::GameState;
use crate::provenance::ProvenanceNodeKind;
use crate::target::ChooseSpec;

/// Resolve a Value to a concrete i32.
pub fn resolve_value(
    game: &GameState,
    value: &Value,
    ctx: &ExecutionContext,
) -> Result<i32, ExecutionError> {
    crate::effects::helpers::resolve_value(game, value, ctx)
}

/// Validate that a resolved target matches a target spec.
pub fn validate_target(
    game: &GameState,
    target: &ResolvedTarget,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> bool {
    let filter_ctx = ctx.filter_context(game);

    match (target, spec) {
        (ResolvedTarget::Object(id), ChooseSpec::Object(filter)) => {
            if let Some(obj) = game.object(*id) {
                filter.matches(obj, &filter_ctx, game)
            } else {
                false
            }
        }
        (ResolvedTarget::Player(id), ChooseSpec::Player(filter)) => {
            game.can_target_player_from_source(*id, ctx.source)
                && filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Player(id), ChooseSpec::PlayerOrPlaneswalker(filter)) => {
            game.can_target_player_from_source(*id, ctx.source)
                && filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Object(id), ChooseSpec::PlayerOrPlaneswalker(_)) => game
            .object(*id)
            .is_some_and(|obj| obj.has_card_type(crate::types::CardType::Planeswalker)),
        (ResolvedTarget::Object(id), ChooseSpec::AnyTarget) => game.object(*id).is_some(),
        (ResolvedTarget::Player(id), ChooseSpec::AnyTarget) => {
            game.player(*id).is_some_and(|p| p.is_in_game())
                && game.can_target_player_from_source(*id, ctx.source)
        }
        (ResolvedTarget::Object(id), ChooseSpec::AnyOtherTarget) => {
            game.object(*id).is_some_and(|obj| obj.id != ctx.source)
        }
        (ResolvedTarget::Player(id), ChooseSpec::AnyOtherTarget) => {
            game.player(*id).is_some_and(|p| p.is_in_game())
                && game.can_target_player_from_source(*id, ctx.source)
        }
        (ResolvedTarget::Object(id), ChooseSpec::SpecificObject(expected)) => id == expected,
        (ResolvedTarget::Player(id), ChooseSpec::SpecificPlayer(expected)) => id == expected,
        _ => false,
    }
}

/// Execute an effect and return the outcome (result + events).
pub fn execute_effect(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    let mut outcome = effect.0.execute(game, ctx)?;

    if !outcome.events.is_empty() {
        let execution_node = game.provenance_graph_mut().alloc_child(
            ctx.provenance,
            ProvenanceNodeKind::EffectExecution {
                source: ctx.source,
                controller: ctx.controller,
            },
        );
        for event in &mut outcome.events {
            let provenance = event.provenance();
            if provenance == crate::provenance::ProvNodeId::default()
                || game.provenance_graph().node(provenance).is_none()
            {
                let node = game.alloc_child_event_provenance(execution_node, event.kind());
                event.set_provenance(node);
            }
        }
        for event in &outcome.events {
            game.stage_turn_history_event(event);
        }
    }

    Ok(outcome)
}
