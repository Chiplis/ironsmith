use super::*;

pub(crate) fn contains_triggered_life_gain_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { .. }),
            ..
        }) => true,
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        }) => {
            if_true.iter().any(contains_triggered_life_gain_effect)
                || if_false.iter().any(contains_triggered_life_gain_effect)
        }
        EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. }) => {
            effects.iter().any(contains_triggered_life_gain_effect)
        }
        _ => false,
    }
}
