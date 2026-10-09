use super::*;

/// A designation change has an object subject, not a player subject. Consume
/// the whole predicate so unknown trailing text cannot disappear.
pub(super) fn parse_clear_suspected_clause(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    if !words.ends_with(&["no", "longer", "suspected"]) { return Ok(None); }
    if words.as_slice() == ["all", "suspected", "creatures", "are", "no", "longer", "suspected"] { return Ok(None); }
    let positions: Vec<_> = tokens.iter().enumerate().filter_map(|(i, t)| t.as_word().map(|_| i)).collect();
    let copula = words.len().saturating_sub(4);
    let (subject, contracted) = match words.get(copula).copied() {
        Some("is" | "are" | "become" | "becomes") => (&tokens[..positions[copula]], false),
        // The contraction is one lexed word ("it's", "they're"); its parser
        // word piece drops the apostrophe.
        _ if words.len() == 4
            && tokens[positions[0]].is_any_word(&["its", "it's", "theyre", "they're"]) =>
        {
            (&tokens[..positions[1]], true)
        }
        _ => return Ok(None),
    };
    let rebuilt;
    let subject = if contracted {
        rebuilt = vec![OwnedLexToken::synthetic_word(if tokens[positions[0]].is_any_word(&["its", "it's"]) { "it" } else { "them" })];
        rebuilt.as_slice()
    } else if subject.first().is_some_and(|token| token.is_word("have")) {
        &subject[1..]
    } else { subject };
    if subject.is_empty() { return Ok(None); }
    let subject_words = crate::lexer::token_word_refs(subject);
    let target = if matches!(subject_words.as_slice(), ["it"] | ["them"] | ["they"]) {
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), span_from_tokens(subject))
    } else { parse_target_phrase(subject)? };
    if matches!(target, TargetAst::Player(..) | TargetAst::PlayerOrPlaneswalker(..)) {
        return Err(CardTextError::ParseError("suspected designation requires an object subject".into()));
    }
    Ok(Some(EffectAst::subject_verb_clear_suspected(Some(target))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::KeywordActionAst;
    #[test]
    fn complete_copular_and_become_forms_keep_the_reference() {
        for text in ["it's no longer suspected", "they're no longer suspected", "it is no longer suspected", "they are no longer suspected", "have it become no longer suspected"] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let Some(EffectAst::SubjectVerb(effect)) = parse_clear_suspected_clause(&tokens).unwrap() else { panic!("{text}"); };
            assert!(matches!(effect.action, SubjectVerbActionAst::KeywordActions(KeywordActionAst::ClearSuspected { target: Some(TargetAst::Tagged(..)) })));
        }
        for text in ["it is no longer suspected until end of turn", "it is no longer suspected of murder", "it is suspected", "it may be no longer suspected"] {
            assert!(parse_clear_suspected_clause(&crate::lexer::lex_line(text, 0).unwrap()).unwrap().is_none(), "{text}");
        }
    }
}
