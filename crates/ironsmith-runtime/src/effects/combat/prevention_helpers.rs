use crate::effect::{Effect, Until};
use crate::effects::helpers::{resolve_objects_from_spec, resolve_players_from_spec};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::{GameState, TargetAssignment};
use crate::ids::ObjectId;
use crate::prevention::{DamageFilter, PreventionShield, PreventionShieldId, PreventionTarget};
use crate::target::ChooseSpec;

pub enum SourceChoiceSelection {
    Chosen(ObjectId),
    NoAvailableSource,
    NoChoiceMade,
}

/// Choose a damage source as an effect resolves for "a source of your choice" shields.
pub fn choose_source_of_your_choice(
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> SourceChoiceSelection {
    let mut candidates = Vec::new();
    candidates.extend(game.stack.iter().map(|entry| entry.object_id));
    candidates.extend(game.battlefield.iter().copied());
    candidates.sort_by_key(|id| id.0);
    candidates.dedup();

    if candidates.is_empty() {
        return SourceChoiceSelection::NoAvailableSource;
    }

    let selectable = candidates
        .iter()
        .copied()
        .map(|id| {
            let name = game
                .object(id)
                .map(|object| object.name.clone())
                .unwrap_or_else(|| format!("object {}", id.0));
            crate::decisions::context::SelectableObject::new(id, name)
        })
        .collect::<Vec<_>>();
    let select_ctx = crate::decisions::context::SelectObjectsContext::new(
        ctx.controller,
        Some(ctx.source),
        "Choose a source",
        selectable,
        1,
        Some(1),
    );
    let chosen_source = ctx
        .decision_maker
        .decide_objects(game, &select_ctx)
        .into_iter()
        .find(|id| candidates.contains(id));
    if ctx.decision_maker.awaiting_choice() {
        return SourceChoiceSelection::NoChoiceMade;
    }
    chosen_source
        .map(SourceChoiceSelection::Chosen)
        .unwrap_or(SourceChoiceSelection::NoChoiceMade)
}

/// Resolve a [`ChooseSpec`] into a [`PreventionTarget`] for prevention effects.
pub fn resolve_prevention_target_from_spec(
    game: &GameState,
    target_spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<PreventionTarget, ExecutionError> {
    if matches!(
        target_spec,
        ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget | ChooseSpec::PlayerOrPlaneswalker(_)
    ) && let Some(target) = ctx.targets.first()
    {
        return Ok(match target {
            ResolvedTarget::Object(object_id) => PreventionTarget::Permanent(*object_id),
            ResolvedTarget::Player(player_id) => PreventionTarget::Player(*player_id),
        });
    }

    if let Ok(objects) = resolve_objects_from_spec(game, target_spec, ctx)
        && let Some(object_id) = objects.first()
    {
        return Ok(PreventionTarget::Permanent(*object_id));
    }
    if let Ok(players) = resolve_players_from_spec(game, target_spec, ctx)
        && let Some(player_id) = players.first()
    {
        return Ok(PreventionTarget::Player(*player_id));
    }
    Err(ExecutionError::InvalidTarget)
}

/// Build and register a prevention shield on the game state.
pub fn register_prevention_shield(
    game: &mut GameState,
    ctx: &ExecutionContext,
    protected: PreventionTarget,
    amount: Option<u32>,
    duration: Until,
    damage_filter: DamageFilter,
    follow_up_effects: Vec<Effect>,
    follow_up_targets: Vec<ResolvedTarget>,
    follow_up_target_assignments: Vec<TargetAssignment>,
) -> PreventionShieldId {
    let shield = PreventionShield::new(ctx.source, ctx.controller, protected, amount, duration)
        .with_filter(damage_filter)
        .with_follow_up_effects(follow_up_effects)
        .with_follow_up_targets(follow_up_targets)
        .with_follow_up_target_assignments(follow_up_target_assignments);
    game.effect_store.prevention_effects.add_shield(shield)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_resolve_prevention_target_strict_selection_uses_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);

        let target =
            resolve_prevention_target_from_spec(&game, &ChooseSpec::SourceController, &ctx)
                .unwrap();

        assert_eq!(target, PreventionTarget::Player(alice));
    }

    #[test]
    fn test_resolve_prevention_target_strict_selection_uses_context_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let protected_object = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(protected_object)]);

        let target =
            resolve_prevention_target_from_spec(&game, &ChooseSpec::AnyTarget, &ctx).unwrap();

        assert_eq!(target, PreventionTarget::Permanent(protected_object));
    }

    #[test]
    fn test_register_prevention_shield_adds_shield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);

        let id = register_prevention_shield(
            &mut game,
            &ctx,
            PreventionTarget::Player(alice),
            Some(3),
            Until::EndOfTurn,
            DamageFilter::all(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );

        let shields = game.effect_store.prevention_effects.shields();
        assert_eq!(shields.len(), 1);
        assert_eq!(shields[0].id, id);
        assert_eq!(shields[0].amount_remaining, Some(3));
    }
}
