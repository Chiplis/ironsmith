use crate::cards::builders::StackActionAst;
use super::*;

pub(super) fn effect_copies_triggering_stack_object(effect: &EffectAst) -> bool {
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                target: TargetAst::Tagged(tag, _),
                ..
            }),
            ..
        }) if tag.as_str() == "triggering"
    ) {
        return true;
    }
    let mut found = false;
    for_each_nested_effects(effect, true, |nested| {
        if !found {
            found = nested.iter().any(effect_copies_triggering_stack_object);
        }
    });
    found
}
