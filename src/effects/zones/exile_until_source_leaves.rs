//! Exile-until effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Duration for "exile ... until ..." effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileUntilDuration {
    /// Return when the source leaves the battlefield.
    SourceLeavesBattlefield,
    /// Return at the beginning of the next end step.
    NextEndStep,
    /// Return at end of combat.
    EndOfCombat,
}

/// Exile objects with an associated duration.
#[derive(Debug, Clone, PartialEq)]
pub struct ExileUntilEffect {
    /// What to exile.
    pub spec: ChooseSpec,
    /// How long the exile lasts.
    pub duration: ExileUntilDuration,
    /// Zone to return cards to when the duration expires.
    pub return_zone: Zone,
}

impl ExileUntilEffect {
    /// Create a new exile-until effect.
    pub fn new(spec: ChooseSpec, duration: ExileUntilDuration) -> Self {
        Self {
            spec,
            duration,
            return_zone: Zone::Battlefield,
        }
    }

    /// Exile until this source leaves the battlefield.
    pub fn source_leaves(spec: ChooseSpec) -> Self {
        Self::new(spec, ExileUntilDuration::SourceLeavesBattlefield)
    }
}

impl EffectExecutor for ExileUntilEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = resolve_objects_from_spec(game, &self.spec, ctx)?;
        let mut exiled_count = 0_i32;
        for object_id in objects {
            if game.move_object(object_id, Zone::Exile).is_some() {
                exiled_count += 1;
            }
        }
        Ok(EffectOutcome::count(exiled_count))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target to exile"
    }
}

