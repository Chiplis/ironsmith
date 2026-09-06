use crate::cards::builders::ConditionalEffectAst;
use super::*;

pub(super) fn rewrite_self_replacements_as_conditionals(effect: EffectAst) -> EffectAst {
    match effect {
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }) => EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true: if_true
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
            if_false: if_false
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
        }),
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            ..
        } => EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true: if_true
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
            if_false: if_false
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
        }),
        other => other,
    }
}
