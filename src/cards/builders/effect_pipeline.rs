use super::*;

#[derive(Debug, Clone)]
pub(crate) struct PreparedEffectsForLowering {
    pub(crate) effects: Vec<EffectAst>,
    pub(crate) bindings: ReferenceBindings,
}

pub(crate) fn prepare_effects_for_lowering(
    effects: &[EffectAst],
    seed_last_object_tag: Option<&str>,
) -> PreparedEffectsForLowering {
    let normalized = normalize_effects_ast(effects);
    let bound = bind_unresolved_it_references_with_bindings(&normalized, seed_last_object_tag);
    PreparedEffectsForLowering {
        effects: bound.effects,
        bindings: bound.bindings,
    }
}
