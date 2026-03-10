use crate::effect::Until;
use crate::effects::helpers::{resolve_objects_from_spec, resolve_players_from_spec};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::prevention::{DamageFilter, PreventionShield, PreventionShieldId, PreventionTarget};
use crate::target::{ChooseSpec, PlayerFilter};

/// Resolution strategy for prevention target selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreventionTargetResolveMode {
    /// Resolve concrete object/player selections from the choose spec.
    StrictSelection,
    /// Preserve legacy `PreventDamageEffect` behavior.
    LegacyDamageFallback,
}

/// Resolve a [`ChooseSpec`] into a [`PreventionTarget`] for prevention effects.
pub fn resolve_prevention_target_from_spec(
    game: &GameState,
    target_spec: &ChooseSpec,
    ctx: &ExecutionContext,
    mode: PreventionTargetResolveMode,
) -> Result<PreventionTarget, ExecutionError> {
    match mode {
        PreventionTargetResolveMode::StrictSelection => {
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
        PreventionTargetResolveMode::LegacyDamageFallback => {
            resolve_legacy_prevent_damage_target(game, target_spec, ctx)
        }
    }
}

/// Build and register a prevention shield on the game state.
pub fn register_prevention_shield(
    game: &mut GameState,
    ctx: &ExecutionContext,
    protected: PreventionTarget,
    amount: Option<u32>,
    duration: Until,
    damage_filter: DamageFilter,
) -> PreventionShieldId {
    let shield = PreventionShield::new(ctx.source, ctx.controller, protected, amount, duration)
        .with_filter(damage_filter);
    game.prevention_effects.add_shield(shield)
}

fn resolve_legacy_prevent_damage_target(
    game: &GameState,
    target_spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<PreventionTarget, ExecutionError> {
    match target_spec {
        ChooseSpec::Source => Ok(PreventionTarget::Permanent(ctx.source)),

        ChooseSpec::SourceController => Ok(PreventionTarget::Player(ctx.controller)),

        ChooseSpec::Player(filter) => match filter {
            PlayerFilter::You => Ok(PreventionTarget::You),
            PlayerFilter::Opponent | PlayerFilter::Any => first_player_target(ctx)
                .map(PreventionTarget::Player)
                .ok_or(ExecutionError::InvalidTarget),
            PlayerFilter::Specific(id) => Ok(PreventionTarget::Player(*id)),
            _ => first_player_target(ctx)
                .map(PreventionTarget::Player)
                .ok_or(ExecutionError::InvalidTarget),
        },

        ChooseSpec::Object(filter) => {
            if let Some(object_id) = first_object_target(ctx) {
                return Ok(PreventionTarget::Permanent(object_id));
            }
            Ok(PreventionTarget::PermanentsMatching(filter.clone()))
        }

        ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => {
            first_target(ctx).ok_or(ExecutionError::InvalidTarget)
        }

        ChooseSpec::SourceOwner => {
            if let Some(source) = game.object(ctx.source) {
                Ok(PreventionTarget::Player(source.owner))
            } else {
                Err(ExecutionError::ObjectNotFound(ctx.source))
            }
        }

        _ => first_target(ctx).ok_or(ExecutionError::InvalidTarget),
    }
}

fn first_object_target(ctx: &ExecutionContext) -> Option<crate::ids::ObjectId> {
    ctx.targets.iter().find_map(|target| match target {
        ResolvedTarget::Object(object_id) => Some(*object_id),
        _ => None,
    })
}

fn first_player_target(ctx: &ExecutionContext) -> Option<crate::ids::PlayerId> {
    ctx.targets.iter().find_map(|target| match target {
        ResolvedTarget::Player(player_id) => Some(*player_id),
        _ => None,
    })
}

fn first_target(ctx: &ExecutionContext) -> Option<PreventionTarget> {
    ctx.targets.first().map(|target| match target {
        ResolvedTarget::Object(object_id) => PreventionTarget::Permanent(*object_id),
        ResolvedTarget::Player(player_id) => PreventionTarget::Player(*player_id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::target::ObjectFilter;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_resolve_prevention_target_strict_selection_uses_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);

        let target = resolve_prevention_target_from_spec(
            &game,
            &ChooseSpec::SourceController,
            &ctx,
            PreventionTargetResolveMode::StrictSelection,
        )
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

        let target = resolve_prevention_target_from_spec(
            &game,
            &ChooseSpec::AnyTarget,
            &ctx,
            PreventionTargetResolveMode::StrictSelection,
        )
        .unwrap();

        assert_eq!(target, PreventionTarget::Permanent(protected_object));
    }

    #[test]
    fn test_resolve_prevention_target_legacy_object_filter_fallback() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);

        let filter = ObjectFilter::creature();
        let target = resolve_prevention_target_from_spec(
            &game,
            &ChooseSpec::Object(filter.clone()),
            &ctx,
            PreventionTargetResolveMode::LegacyDamageFallback,
        )
        .unwrap();

        assert_eq!(target, PreventionTarget::PermanentsMatching(filter));
    }

    #[test]
    fn test_resolve_prevention_target_legacy_requires_explicit_opponent_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);

        let result = resolve_prevention_target_from_spec(
            &game,
            &ChooseSpec::Player(PlayerFilter::Opponent),
            &ctx,
            PreventionTargetResolveMode::LegacyDamageFallback,
        );

        assert!(matches!(result, Err(ExecutionError::InvalidTarget)));
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
        );

        let shields = game.prevention_effects.shields();
        assert_eq!(shields.len(), 1);
        assert_eq!(shields[0].id, id);
        assert_eq!(shields[0].amount_remaining, Some(3));
    }
}
