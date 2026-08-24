use super::*;

pub(super) fn normalize_mana_replacement_effects(effects: Vec<EffectAst>) -> Vec<EffectAst> {
    effects
        .into_iter()
        .map(|effect| match effect {
            EffectAst::SelfReplacement { .. } => effect,
            other => rewrite_self_replacements_as_conditionals(other),
        })
        .collect()
}
