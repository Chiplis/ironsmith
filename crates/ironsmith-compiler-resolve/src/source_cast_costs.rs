//! Bind an Emerge-cost condition's sacrificed-object reference to its cast
//! payment receipt, independently of the triggering permanent/object memory.
use crate::cards::builders::{EffectAst, ConditionalEffectAst, PredicateAst};
use crate::model::control_flow::{ControlFlowNodeAst, ControlPredicateAst};
use ironsmith_core::tag::TagKeyWalk;

fn is_emerge(predicate: &PredicateAst) -> bool {
    matches!(predicate, PredicateAst::ThisSpellPaidLabel(cost)
        if *cost == ironsmith_core::OptionalCostRef::from_label("Emerge"))
}

fn bind_program(effects: &mut [EffectAst]) {
    let from = crate::tag::CompilerReferenceTag::AdditionalCostObject.key();
    let to = crate::tag::CompilerReferenceTag::SourceEmergeSacrifice.key();
    for effect in effects {
        effect.map_tag_keys(&mut |tag| {
            if *tag == from { *tag = to.clone(); }
        });
    }
}

pub fn bind_source_cast_cost_references(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::SelfReplacement { predicate, if_true, .. }
            | EffectAst::Conditionals(ConditionalEffectAst::Conditional { predicate, if_true, .. })
                if is_emerge(predicate) => bind_program(if_true),
            EffectAst::ControlFlow(control) => {
                let guarded = match &control.node {
                    ControlFlowNodeAst::Condition { condition, consequence_program, .. }
                        if matches!(&condition.predicate, ControlPredicateAst::State(p) if is_emerge(p)) =>
                        Some(*consequence_program),
                    ControlFlowNodeAst::Replacement(replacement)
                        if replacement.condition.as_ref().is_some_and(|condition|
                            matches!(&condition.predicate, ControlPredicateAst::State(p) if is_emerge(p))) =>
                        Some(replacement.replacement_program),
                    _ => None,
                };
                if let Some(program) = guarded { bind_program(&mut control.programs[program].effects); }
            }
            _ => {}
        }
        crate::model::visit::for_each_nested_effects_mut(effect, true,
            bind_source_cast_cost_references);
    }
}
