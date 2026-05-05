use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::events::ReplacementPriority;
use crate::game_state::GameState;
use crate::replacement::ReplacementEffect;
use crate::target::ObjectFilter;

/// Execute an effect while temporary replacement effects are scoped to that execution.
///
/// This models self-replacement patterns like "Counter target spell. If that spell is
/// countered this way, exile it instead..." where the replacement applies only to the
/// event caused by the antecedent effect.
pub type LocalRewriteEffect = ironsmith_core::LocalRewriteEffect<Effect>;

fn resolve_zone_replacements(
    replacement: &ironsmith_core::RegisterZoneReplacementEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<Vec<ReplacementEffect>, ExecutionError> {
    let object_ids = resolve_objects_for_effect(game, ctx, &replacement.target)?;
    if object_ids.is_empty() {
        return Err(ExecutionError::InvalidTarget);
    }

    Ok(object_ids
        .into_iter()
        .map(|object_id| {
            ReplacementEffect::with_matcher(
                ctx.source,
                ctx.controller,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(object_id),
                    replacement.from_zone,
                    replacement.to_zone,
                ),
                crate::effects::replacement::zone_replacement_action(
                    replacement.to_zone,
                    replacement.replacement_zone,
                    replacement.optional,
                    replacement.choice_description.clone(),
                ),
            )
        })
        .collect())
}

impl EffectExecutor for LocalRewriteEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&self.effect);
    }

    fn transparent_child_effect(&self) -> Option<&Effect> {
        Some(&self.effect)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut replacements = Vec::new();
        let fallback_target = self.effect.0.get_target_spec().cloned();
        for replacement in &self.zone_replacements {
            match resolve_zone_replacements(replacement, game, ctx) {
                Ok(resolved) => replacements.extend(resolved.into_iter().map(|effect| {
                    effect.with_priority_override(ReplacementPriority::SelfReplacement)
                })),
                Err(ExecutionError::InvalidTarget) => {
                    let Some(target_spec) = &fallback_target else {
                        continue;
                    };
                    let mut rebound = replacement.clone();
                    rebound.target = target_spec.clone();
                    if let Ok(resolved) = resolve_zone_replacements(&rebound, game, ctx) {
                        replacements.extend(resolved.into_iter().map(|effect| {
                            effect.with_priority_override(ReplacementPriority::SelfReplacement)
                        }));
                    }
                }
                Err(err) => return Err(err),
            }
        }

        ctx.with_temp_additional_replacement_effects(replacements, |ctx| {
            execute_effect(game, &self.effect, ctx)
        })
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        self.effect.0.get_target_spec()
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        self.effect.0.decision_related_object_specs()
    }

    fn target_description(&self) -> &'static str {
        self.effect.0.target_description()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        self.effect.0.get_target_count()
    }
}
