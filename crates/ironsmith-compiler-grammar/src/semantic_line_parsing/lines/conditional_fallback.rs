// Quotes at the edge may close a child ability owned by a complete menu.
// Only the enclosing clause separators are disposable here.
fn trim_outer_separators(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let edge = |token: &OwnedLexToken| matches!(token.kind, crate::lexer::TokenKind::Comma | crate::lexer::TokenKind::Period | crate::lexer::TokenKind::Semicolon);
    let start=tokens.iter().position(|token| !edge(token)).unwrap_or(tokens.len());
    let end=tokens.iter().rposition(|token| !edge(token)).map_or(start,|index|index+1);
    &tokens[start..end]
}

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
    let second = trim_outer_separators(second);
    if !second.first().is_some_and(|token| token.is_word("otherwise")) { return Ok(None); }
    let false_tokens = trim_outer_separators(&second[1..]);
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

#[cfg(test)]
mod quoted_menu_fallback_tests {
    use super::*;
    #[test]
    fn false_arm_preserves_all_complete_quoted_child_programs() {
        let predicate = PredicateAst::Source(crate::cards::builders::SourcePredicateAst::SourceMatches(crate::target::ObjectFilter::creature()));
        let tokens=crate::lexer::lex_line("Draw a card. Otherwise, choose one instruction — Create an artifact token named \"First\" with \"{0}: Draw a card.\" | Create an artifact token named \"Second\" with \"{0}: Gain 1 life.\".",0).unwrap();
        let effects=parse(Some(&predicate),&tokens).unwrap().unwrap();
        let [EffectAst::Conditionals(ConditionalEffectAst::Conditional{if_false,..})]=effects.as_slice() else {panic!("conditional owner")};
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf{modes,..})]=if_false.as_slice() else {panic!("menu owner")};
        assert_eq!(modes.len(),2);
    }
}
