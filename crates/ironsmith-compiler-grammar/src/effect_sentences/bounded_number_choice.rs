use crate::cards::builders::{CardTextError, ChoiceActionAst, EffectAst, OwnedLexToken, PlayerAst, SubjectVerbActionAst, SubjectVerbRoleAst};
/// Resolution-local choices preserve whether the upper bound was authored.
/// Persistent as-enters choices are owned by a different instruction.
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>,CardTextError> {
    let complete = if tokens.last().is_some_and(|token| token.kind == crate::lexer::TokenKind::Period) {
        &tokens[..tokens.len() - 1]
    } else { tokens };
    if complete.len() == 3 && complete.iter().zip(["choose", "a", "number"]).all(|(token, word)| token.is_word(word)) {
        return Ok(Some(EffectAst::subject_verb(SubjectVerbRoleAst::Chooser, PlayerAst::You,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNumber { min: 0, max: None, source_owned: false }))));
    }
    if complete.len() == 6 && complete.iter().zip(["choose", "a", "number", "greater", "than", "0"])
        .all(|(token, word)| token.is_word(word)) {
        return Ok(Some(EffectAst::subject_verb(SubjectVerbRoleAst::Chooser, PlayerAst::You,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNumber { min: 1, max: None, source_owned: false }))));
    }
    let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
    let Some((_,rest))=crate::grammar::primitives::parse_prefix(tokens,crate::grammar::primitives::phrase(&["choose","a","number","between"])) else{return Ok(None)};
    let Some((min,used))=crate::util::parse_number(rest) else{return Ok(None)};
    let rest=&rest[used..];if !rest.first().is_some_and(|token|token.is_word("and")){return Ok(None)}
    let Some((max,used))=crate::util::parse_number(&rest[1..]) else{return Ok(None)};
    if used+1!=rest.len(){return Ok(None)}
    if min>max{return Err(CardTextError::ParseError("numeric choice minimum exceeds maximum".into()))}
    Ok(Some(EffectAst::subject_verb(SubjectVerbRoleAst::Chooser,PlayerAst::You,SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNumber{min,max:Some(max),source_owned:false}))))
}
/// A coordinated choice sentence exports two independent result producers.
pub(super) fn parse_sentence(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let complete = if tokens.last().is_some_and(|token| token.kind == crate::lexer::TokenKind::Period) {
        &tokens[..tokens.len() - 1]
    } else { tokens };
    if complete.len() >= 3 && complete[complete.len() - 3..].iter().zip(["and", "a", "color"])
        .all(|(token, word)| token.is_word(word)) {
        if let Some(number) = parse(&complete[..complete.len() - 3])? {
            return Ok(Some(vec![number, EffectAst::subject_verb(SubjectVerbRoleAst::Chooser,
                PlayerAst::You, SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseColor))]));
        }
        return Ok(None);
    }
    parse(tokens).map(|effect| effect.map(|effect| vec![effect]))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_bounds_zero_and_tails_stay_typed() {
        for text in ["Choose a number between 0 and 13.","Choose a number between one and ten."] {
            assert!(parse(&crate::lexer::lex_line(text,0).unwrap()).unwrap().is_some());
        }
        assert!(parse(&crate::lexer::lex_line("Choose a number between 13 and 0.",0).unwrap()).is_err());
        assert!(parse(&crate::lexer::lex_line("Choose a number.",0).unwrap()).unwrap().is_some());
        for text in ["Choose a number!", "Choose a number except on Tuesdays.","Choose a number between 0 and 13 except on Tuesdays.","Choose a number between 0 and 13. Draw a card."] {
            assert!(parse(&crate::lexer::lex_line(text,0).unwrap()).unwrap().is_none());
        }
    }
}

#[cfg(test)]
mod positive_number_tests {
    use super::*;
    #[test]
    fn positive_number_and_color_are_two_complete_typed_producers() {
        let parsed=parse_sentence(&crate::lexer::lex_line("Choose a number greater than 0 and a color.",0).unwrap()).unwrap().unwrap();
        assert_eq!(parsed.len(),2);
        assert!(matches!(&parsed[0],EffectAst::SubjectVerb(subject) if matches!(subject.action,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNumber{min:1,max:None,source_owned:false}))));
        assert!(matches!(&parsed[1],EffectAst::SubjectVerb(subject) if matches!(subject.action,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseColor))));
        assert!(parse_sentence(&crate::lexer::lex_line("Choose a number greater than 0 and a color except blue.",0).unwrap()).unwrap().is_none());
    }
}
