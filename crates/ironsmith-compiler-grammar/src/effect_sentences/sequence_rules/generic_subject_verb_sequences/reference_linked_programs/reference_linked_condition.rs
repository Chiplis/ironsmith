use super::*;

pub(super) fn append_to_outer_if_result(
    effect: &mut EffectAst,
    followup: &mut Vec<EffectAst>,
) -> bool {
    let effects = match effect {
        EffectAst::IfResult { effects, .. } | EffectAst::ResolvedIfResult { effects, .. } => {
            effects
        }
        _ => return false,
    };
    effects.append(followup);
    true
}
