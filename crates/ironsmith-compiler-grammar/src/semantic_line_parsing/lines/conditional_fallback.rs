use super::*;

/// An explicit `otherwise` supplies the false arm of the condition. The
/// condition then selects a resolution-time branch; it cannot gate whether
/// the surrounding trigger exists. Each action arm uses ordinary readers.
pub(super) fn parse(
    predicate: Option<&PredicateAst>,
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(predicate) = predicate else { return Ok(None); };
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let [first, second] = sentences.as_slice() else { return Ok(None); };
    let second = crate::util::trim_edge_punctuation_tokens(second);
    if !second.first().is_some_and(|token| token.is_word("otherwise")) { return Ok(None); }
    let false_tokens = crate::util::trim_edge_punctuation_tokens(&second[1..]);
    if false_tokens.is_empty() { return Ok(None); }
    let if_true = crate::effect_sentences::parse_effect_sentences_lexed(first)?;
    let if_false = crate::effect_sentences::parse_effect_sentences_lexed(false_tokens)?;
    Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: predicate.clone(), if_true, if_false,
    })]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_explicit_false_arm_is_control_flow_not_an_action_failure() {
        let predicate = PredicateAst::Source(crate::cards::builders::SourcePredicateAst::SourceMatches(
            crate::target::ObjectFilter::creature(),
        ));
        let tokens = crate::lexer::lex_line("Draw a card. Otherwise, gain 3 life.", 0).unwrap();
        let effects = parse(Some(&predicate), &tokens).unwrap().unwrap();
        let [EffectAst::Conditionals(ConditionalEffectAst::Conditional { predicate: actual, if_true, if_false })] = effects.as_slice() else { panic!("{effects:?}"); };
        assert_eq!(actual, &predicate);
        assert!(!if_true.is_empty());
        assert!(!if_false.is_empty());
        assert!(parse(None, &tokens).unwrap().is_none());
        let unrelated = crate::lexer::lex_line("Draw a card. Gain 3 life.", 0).unwrap();
        assert!(parse(Some(&predicate), &unrelated).unwrap().is_none());
    }
}
