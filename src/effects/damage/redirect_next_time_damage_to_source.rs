//! Redirect-the-next-time damage replacement effect.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::events::damage::matchers::DamageSourceConstraint;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_event::DamageTarget;
use crate::game_state::GameState;
use crate::replacement::{RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect};
use crate::target::{ChooseSpec, ObjectFilter};

/// Matches damage events from a constrained source to a specific object target.
#[derive(Debug, Clone)]
struct DamageSourceToSpecificObjectMatcher {
    source: DamageSourceConstraint,
    target: crate::ids::ObjectId,
}

impl DamageSourceToSpecificObjectMatcher {
    fn new(source: DamageSourceConstraint, target: crate::ids::ObjectId) -> Self {
        Self { source, target }
    }
}

impl ReplacementMatcher for DamageSourceToSpecificObjectMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &crate::events::EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }
        let Some(damage) = crate::events::downcast_event::<crate::events::DamageEvent>(event) else {
            return false;
        };
        if damage.target != DamageTarget::Object(self.target) {
            return false;
        }
        match &self.source {
            DamageSourceConstraint::Specific(source) => damage.source == *source,
            DamageSourceConstraint::Filter(filter) => ctx
                .game
                .object(damage.source)
                .is_some_and(|object| filter.matches(object, &ctx.filter_ctx, ctx.game)),
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When the next chosen source would deal damage to that creature".to_string()
    }
}

/// How to constrain which source's damage is redirected.
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectNextTimeDamageSource {
    Choice,
    Filter(ObjectFilter),
}

/// "The next time a source of your choice would deal damage to target creature this turn,
/// that damage is dealt to this creature instead."
#[derive(Debug, Clone, PartialEq)]
pub struct RedirectNextTimeDamageToSourceEffect {
    pub source: RedirectNextTimeDamageSource,
    pub target: ChooseSpec,
}

impl RedirectNextTimeDamageToSourceEffect {
    pub fn new(source: RedirectNextTimeDamageSource, target: ChooseSpec) -> Self {
        Self { source, target }
    }
}

impl EffectExecutor for RedirectNextTimeDamageToSourceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let protected_target = resolve_objects_from_spec(game, &self.target, ctx)?
            .into_iter()
            .next()
            .ok_or(ExecutionError::InvalidTarget)?;

        let source_constraint = match &self.source {
            RedirectNextTimeDamageSource::Filter(filter) => {
                DamageSourceConstraint::Filter(filter.clone())
            }
            RedirectNextTimeDamageSource::Choice => {
                let mut candidates = Vec::new();
                candidates.extend(game.stack.iter().map(|entry| entry.object_id));
                candidates.extend(game.battlefield.iter().copied());
                candidates.sort_by_key(|id| id.0);
                candidates.dedup();

                if candidates.is_empty() {
                    return Ok(EffectOutcome::resolved());
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
                let chosen = ctx
                    .decision_maker
                    .decide_objects(game, &select_ctx)
                    .into_iter()
                    .next()
                    .unwrap_or(candidates[0]);
                DamageSourceConstraint::Specific(chosen)
            }
        };

        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            DamageSourceToSpecificObjectMatcher::new(source_constraint, protected_target),
            ReplacementAction::Redirect {
                target: RedirectTarget::ToSource,
                which: RedirectWhich::First,
            },
        );
        game.replacement_effects.add_one_shot_effect(replacement);
        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target creature to protect and redirect from"
    }
}
