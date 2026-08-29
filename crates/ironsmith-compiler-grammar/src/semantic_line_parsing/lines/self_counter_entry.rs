use super::*;

pub(super) fn parse_self_enters_with_x_counters_static_chunk(
    tokens: &[OwnedLexToken],
) -> Option<LineAst> {
    match semantic_grammar::parse_self_counter_entry_tokens(tokens)? {
        semantic_grammar::SelfCounterEntrySpec::Adamant {
            condition,
            predicate_body,
        } => Some(LineAst::StaticAbilities(vec![
            crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::enters_with_counters_if_condition(
                    crate::object::CounterType::PlusOnePlusOne,
                    crate::effect::Value::Fixed(1),
                    condition,
                    predicate_body,
                ),
            ),
        ])),
        semantic_grammar::SelfCounterEntrySpec::Unconditional { count } => {
            Some(LineAst::StaticAbilities(vec![
                crate::cards::builders::StaticAbilityAst::Static(
                    StaticAbility::enters_with_counters_value(
                        crate::object::CounterType::PlusOnePlusOne,
                        count,
                    ),
                ),
            ]))
        }
    }
}
