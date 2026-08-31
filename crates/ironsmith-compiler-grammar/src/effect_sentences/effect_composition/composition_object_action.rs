use super::*;

pub(super) fn parse_regenerate_then_gain_control_if_regenerates_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_regenerate_control_shape(first, second)?;
    let regenerate_target =
        crate::grammar::primitives::probe_shape(parse_target_phrase(shape.regenerate_target))?;
    let control_target =
        crate::grammar::primitives::probe_shape(parse_target_phrase(shape.control_target))?;
    let follow_up = EffectAst::subject_verb_gain_control(
        PlayerAst::Implicit,
        control_target,
        crate::effect::Until::Forever,
    );

    Some(vec![
        EffectAst::subject_verb_regenerate_with_follow_up_effects(
            regenerate_target,
            vec![follow_up],
        ),
    ])
}
