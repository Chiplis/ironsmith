use super::*;

pub(super) fn bind_numeric_result_counter_amounts(effects: &mut [EffectAst]) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutCounters { count, .. },
            ..
        }) = effect
            && matches!(
                count,
                Value::EventValue(crate::effect::EventValueSpec::Amount)
            )
        {
            *count = Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::Count,
            };
        }
        crate::model::visit::for_each_nested_effects_mut(
            effect,
            true,
            bind_numeric_result_counter_amounts,
        );
    }
}
