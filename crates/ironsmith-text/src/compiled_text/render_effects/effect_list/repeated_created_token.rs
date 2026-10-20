//! A conditional per-opponent repeat of an identical token can refer back to
//! the first token's complete blueprint. Both create instructions stay live.
use super::*;

pub(super) fn describe(effects: &[Effect]) -> Option<String> {
    let [first_effect, each_effect] = effects else { return None };
    // Some parsers carry a creation tag for later references, while an
    // explicit "another one" repeat can clone the creation directly. Both
    // shapes execute the same token blueprint; tags only affect references.
    let first_tagged = first_effect.downcast_ref::<crate::effects::TaggedEffect>();
    if first_tagged.is_some_and(|tagged| tagged.outcome_only) {
        return None;
    }
    let first = first_tagged
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(first_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    let each = each_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if each.filter != PlayerFilter::Opponent
        || each.starting_with_controller
        || each.sequential
        || each.stop_after_first_happened
    {
        return None;
    }
    let [condition_effect] = each.effects.as_slice() else { return None };
    let condition = condition_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::PlayerHasPoisonCountersOrMore { player, count } = &condition.condition else {
        return None;
    };
    if *player != PlayerFilter::IteratedPlayer
        || *count == 0
        || !condition.if_false.is_empty()
        || condition.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || condition.capture_condition_result
    {
        return None;
    }
    let [repeat_effect] = condition.if_true.as_slice() else { return None };
    let repeat_tagged = repeat_effect.downcast_ref::<crate::effects::TaggedEffect>();
    if repeat_tagged.is_some_and(|tagged| tagged.outcome_only) {
        return None;
    }
    if let (Some(first_tagged), Some(repeat_tagged)) = (first_tagged, repeat_tagged) {
        if first_tagged.tag == repeat_tagged.tag {
            return None;
        }
    }
    let repeat = repeat_tagged
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(repeat_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if first.count != Value::Fixed(1)
        || repeat.count != Value::Fixed(1)
        || first.controller != PlayerFilter::You
        || repeat.controller != PlayerFilter::You
    {
        return None;
    }
    // Authored actor wording can differ between the initial create and the
    // follow-up. Everything that affects the created token must be equal.
    let mut initial = first.clone();
    let mut repeated = repeat.clone();
    initial.actor_surface_explicit = false;
    repeated.actor_surface_explicit = false;
    if initial != repeated {
        return None;
    }
    let count_word = i32::try_from(*count)
        .ok()
        .and_then(number_word)
        .unwrap_or_else(|| count.to_string());
    let first_text = describe_effect(first_effect);
    Some(format!(
        "{}. Then for each opponent with {count_word} or more poison counters, you create another one of those tokens",
        first_text.trim_end_matches('.')
    ))
}
