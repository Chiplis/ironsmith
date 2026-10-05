use crate::cards::builders::{CardTextError, ChoiceActionAst, EffectAst, OwnedLexToken, PlayerAst, SubjectVerbActionAst, SubjectVerbRoleAst};
/// Resolution-local choices preserve whether the upper bound was authored.
/// Persistent as-enters choices are owned by a different instruction.
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>,CardTextError> {
    let complete = if tokens.last().is_some_and(|token| token.kind == crate::lexer::TokenKind::Period) {
        &tokens[..tokens.len() - 1]
    } else { tokens };
    if complete.len() == 3 && complete.iter().zip(["choose", "a", "number"]).all(|(token, word)| token.is_word(word)) {
        return Ok(Some(EffectAst::subject_verb(SubjectVerbRoleAst::Chooser, PlayerAst::You,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNumber { min: 0, max: None }))));
    }
    let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
    let Some((_,rest))=crate::grammar::primitives::parse_prefix(tokens,crate::grammar::primitives::phrase(&["choose","a","number","between"])) else{return Ok(None)};
    let Some((min,used))=crate::util::parse_number(rest) else{return Ok(None)};
    let rest=&rest[used..];if !rest.first().is_some_and(|token|token.is_word("and")){return Ok(None)}
    let Some((max,used))=crate::util::parse_number(&rest[1..]) else{return Ok(None)};
    if used+1!=rest.len(){return Ok(None)}
    if min>max{return Err(CardTextError::ParseError("numeric choice minimum exceeds maximum".into()))}
    Ok(Some(EffectAst::subject_verb(SubjectVerbRoleAst::Chooser,PlayerAst::You,SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNumber{min,max:Some(max)}))))
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
