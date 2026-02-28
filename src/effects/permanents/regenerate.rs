//! Regenerate effect implementation.

use crate::effect::{Effect, EffectOutcome, EffectResult, Until};
use crate::effects::{ApplyReplacementEffect, EffectExecutor};
use crate::events::permanents::matchers::RegenerationShieldMatcher;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that regenerates a target creature.
///
/// Creates a "regeneration shield" as a one-shot replacement effect that lasts
/// for the specified duration. When the creature would be destroyed, instead:
/// - Tap it
/// - Remove all damage from it
/// - Remove it from combat (if applicable)
/// - The replacement effect is consumed
///
/// The regeneration shield is implemented as a proper replacement effect rather
/// than a counter, which aligns with the MTG rules and allows it to interact
/// correctly with other replacement effects.
///
/// # Fields
///
/// * `target` - The creature to regenerate
///
/// # Example
///
/// ```ignore
/// // Regenerate target creature
/// let effect = RegenerateEffect::new(ChooseSpec::creature(), Until::EndOfTurn);
///
/// // Regenerate this creature (source)
/// let effect = RegenerateEffect::source(Until::EndOfTurn);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RegenerateEffect {
    /// The targeting specification.
    pub target: ChooseSpec,
    /// Duration for the regeneration shield.
    pub duration: Until,
}

impl RegenerateEffect {
    /// Create a new regenerate effect with explicit duration.
    pub fn new(target: ChooseSpec, duration: Until) -> Self {
        Self { target, duration }
    }

    /// Create an effect that regenerates the source creature with explicit duration.
    pub fn source(duration: Until) -> Self {
        Self::new(ChooseSpec::Source, duration)
    }

    /// Create an effect that regenerates target creature with explicit duration.
    pub fn target_creature(duration: Until) -> Self {
        Self::new(ChooseSpec::creature(), duration)
    }
}

impl EffectExecutor for RegenerateEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.duration != Until::EndOfTurn {
            return Err(ExecutionError::Impossible(
                "RegenerateEffect currently supports only Until::EndOfTurn".to_string(),
            ));
        }

        // Resolve all matching targets. This supports both traditional
        // "target creature" regeneration and "regenerate each/all ..." forms.
        let targets = crate::effects::helpers::resolve_objects_from_spec(game, &self.target, ctx)
            .map_err(|_| ExecutionError::InvalidTarget)?;
        if targets.is_empty() {
            return Err(ExecutionError::InvalidTarget);
        }

        let mut outcomes = Vec::new();
        for target_id in targets {
            // Regeneration only applies to creatures currently on the battlefield.
            let Some(obj) = game.object(target_id) else {
                continue;
            };
            if obj.zone != Zone::Battlefield || !obj.is_creature() {
                continue;
            }
            if !game.can_be_regenerated(target_id) {
                continue;
            }
            let controller = obj.controller;

            let replacement_effects = vec![
                Effect::tap(ChooseSpec::SpecificObject(target_id)),
                Effect::clear_damage(ChooseSpec::SpecificObject(target_id)),
                // Removing from combat is intentionally omitted until combat-state
                // tracking for regenerated permanents is wired in.
            ];

            let matcher = RegenerationShieldMatcher::new(target_id);
            let replacement_effect = ReplacementEffect::with_matcher(
                target_id, // source is the creature itself
                controller,
                matcher,
                ReplacementAction::Instead(replacement_effects),
            )
            .self_replacing();

            let apply = ApplyReplacementEffect::one_shot(replacement_effect);
            outcomes.push(execute_effect(game, &Effect::new(apply), ctx)?);
        }

        if outcomes.is_empty() {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }
        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to regenerate"
    }
}
