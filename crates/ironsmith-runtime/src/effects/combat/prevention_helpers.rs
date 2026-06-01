use crate::effect::{Effect, Until};
use crate::effects::helpers::{resolve_objects_from_spec, resolve_players_from_spec};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::prevention::{DamageFilter, PreventionShield, PreventionShieldId, PreventionTarget};
use crate::target::ChooseSpec;

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
) -> PreventionShieldId {
    register_prevention_shield_with_lifetime(
        game,
        ctx,
        protected,
        amount,
        duration,
        damage_filter,
        follow_up_effects,
        false,
    )
}

/// Build and register a prevention shield on the game state with explicit event lifetime.
pub fn register_prevention_shield_with_lifetime(
    game: &mut GameState,
    ctx: &ExecutionContext,
    protected: PreventionTarget,
    amount: Option<u32>,
    duration: Until,
    damage_filter: DamageFilter,
    follow_up_effects: Vec<Effect>,
    expires_after_next_matching_event: bool,
) -> PreventionShieldId {
    let shield = PreventionShield::new(ctx.source, ctx.controller, protected, amount, duration)
        .with_filter(damage_filter)
        .with_follow_up_effects(follow_up_effects);
    let shield = if expires_after_next_matching_event {
        shield.with_expires_after_next_matching_event()
    } else {
        shield
    };
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
        );

        let shields = game.effect_store.prevention_effects.shields();
        assert_eq!(shields.len(), 1);
        assert_eq!(shields[0].id, id);
        assert_eq!(shields[0].amount_remaining, Some(3));
    }
}
