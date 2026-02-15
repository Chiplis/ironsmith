//! Prevent all damage to a specific target effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_from_spec, resolve_players_from_spec};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::prevention::{DamageFilter, PreventionShield, PreventionTarget};
use crate::target::ChooseSpec;

/// Effect that prevents all damage to a chosen target for a duration.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllDamageToTargetEffect {
    /// What to protect.
    pub target: ChooseSpec,
    /// Duration for the prevention shield.
    pub duration: Until,
    /// Filter for what damage this shield applies to.
    pub damage_filter: DamageFilter,
}

impl PreventAllDamageToTargetEffect {
    /// Create a new "prevent all damage to target" effect.
    pub fn new(target: ChooseSpec, duration: Until) -> Self {
        Self {
            target,
            duration,
            damage_filter: DamageFilter::all(),
        }
    }

    /// Set a damage filter for this prevention effect.
    pub fn with_filter(mut self, filter: DamageFilter) -> Self {
        self.damage_filter = filter;
        self
    }
}

impl EffectExecutor for PreventAllDamageToTargetEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if !game.can_prevent_damage() {
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }

        let protected = resolve_prevention_target(game, &self.target, ctx)?;
        let shield = PreventionShield::new(ctx.source, ctx.controller, protected, None, self.duration.clone())
            .with_filter(self.damage_filter.clone());
        game.prevention_effects.add_shield(shield);

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to protect from all damage"
    }
}

fn resolve_prevention_target(
    game: &GameState,
    target_spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<PreventionTarget, ExecutionError> {
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
