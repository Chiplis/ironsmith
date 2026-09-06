use super::*;

pub(crate) fn append_to_outer_if_result(
    effect: &mut EffectAst,
    followup: &mut Vec<EffectAst>,
) -> bool {
    let effects = match effect {
        EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. }) | EffectAst::Conditionals(ConditionalEffectAst::ResolvedIfResult { effects, .. }) => {
            effects
        }
        _ => return false,
    };
    effects.append(followup);
    true
}
