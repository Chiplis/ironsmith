use crate::cards::builders::{CardTextError, ChoiceActionAst, EffectAst, OwnedLexToken, PlayerAst, SubjectVerbActionAst, SubjectVerbRoleAst};
/// Only the complete authored bounded form. Unbounded/entry-persisted choices
/// have a different storage and host-representation contract and remain unclaimed.
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>,CardTextError> {
    let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
    let Some((_,rest))=crate::grammar::primitives::parse_prefix(tokens,crate::grammar::primitives::phrase(&["choose","a","number","between"])) else{return Ok(None)};
    let Some((min,used))=crate::util::parse_number(rest) else{return Ok(None)};
    let rest=&rest[used..];if !rest.first().is_some_and(|token|token.is_word("and")){return Ok(None)}
    let Some((max,used))=crate::util::parse_number(&rest[1..]) else{return Ok(None)};
    if used+1!=rest.len(){return Ok(None)}
    if min>max{return Err(CardTextError::ParseError("numeric choice minimum exceeds maximum".into()))}
    Ok(Some(EffectAst::subject_verb(SubjectVerbRoleAst::Chooser,PlayerAst::You,SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNumber{min,max}))))
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
        for text in ["Choose a number.","Choose a number between 0 and 13 except on Tuesdays.","Choose a number between 0 and 13. Draw a card."] {
            assert!(parse(&crate::lexer::lex_line(text,0).unwrap()).unwrap().is_none());
        }
    }
}
