//! Reuse an exact token-size value for damage dealt by that created token.
use super::*;

pub(super) fn describe(effects: &[Effect]) -> Option<String> {
    let [create_effect, damage_effect] = effects else { return None };
    let tagged = create_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let create = tagged.effect.downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if tagged.outcome_only
        || create.count != Value::Fixed(1)
        || create.controller != PlayerFilter::You
    {
        return None;
    }
    let (power, toughness) = create.dynamic_power_toughness.as_ref()?;
    if power != toughness
        || !power.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
        || !matches!(power.unhinted(), Value::HalfLifeTotalRoundedUp(PlayerFilter::You))
    {
        return None;
    }
    let damage_source = damage_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(damage_source.source.base(), ChooseSpec::Tagged(tag) if tag == &tagged.tag) {
        return None;
    }
    let damage = damage_source.effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    let Value::HalfRoundedDown(inner) = damage.amount.unhinted() else { return None };
    let Value::Add(life, offset) = inner.unhinted() else { return None };
    if !matches!(life.unhinted(), Value::LifeTotal(PlayerFilter::You))
        || !matches!(offset.unhinted(), Value::Fixed(1))
        || !damage.amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
        || damage.target != ChooseSpec::SourceController
        || damage.source_is_combat
        || damage.unpreventable
        || damage.excess_to_controller.is_some()
    {
        return None;
    }
    let token = describe_effect(create_effect);
    let token = token.trim_end_matches('.');
    if !token.contains("where X is half your life total") {
        return None;
    }
    Some(format!("{token}, rounded up. It deals X damage to you"))
}
