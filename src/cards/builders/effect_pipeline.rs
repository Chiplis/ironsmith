use super::*;

pub(crate) fn prepare_effects_for_lowering(
    effects: &[EffectAst],
    seed_last_object_tag: Option<&str>,
) -> Vec<EffectAst> {
    let normalized = normalize_effects_ast(effects);
    bind_unresolved_it_references(&normalized, seed_last_object_tag)
}
