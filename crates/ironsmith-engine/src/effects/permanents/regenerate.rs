//! Regenerate effect implementation.

use crate::effect::{Effect, EffectOutcome, Until};
use crate::effects::{ApplyReplacementEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::events::permanents::matchers::RegenerationShieldMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::zone::Zone;
pub type RegenerateEffect = ironsmith_core::RegenerateEffect<crate::effect::Effect>;

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
        let targets = crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)
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
            if obj.zone != Zone::Battlefield || !game.current_is_creature(target_id) {
                continue;
            }
            if !game.can_be_regenerated(target_id) {
                continue;
            }
            let controller = ctx.controller;

            let mut replacement_effects = vec![
                Effect::tap(ChooseSpec::SpecificObject(target_id)).tag(TagKey::from("__it__")),
                Effect::clear_damage(ChooseSpec::SpecificObject(target_id)),
                Effect::new(crate::effects::RemoveFromCombatEffect::with_spec(
                    ChooseSpec::SpecificObject(target_id),
                )),
            ];
            replacement_effects.extend(self.follow_up_effects.clone());

            let matcher = RegenerationShieldMatcher::new(target_id);
            let replacement_effect = ReplacementEffect::with_matcher(
                target_id, // source is the creature itself
                controller,
                matcher,
                ReplacementAction::Instead(replacement_effects),
            );

            let apply = ApplyReplacementEffect::one_shot(replacement_effect);
            outcomes.push(execute_effect(game, &Effect::new(apply), ctx)?);
        }

        if outcomes.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }
        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to regenerate"
    }
}
