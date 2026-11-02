//! Explicit resolution-time instruction menus; quoted child abilities are opaque.
use crate::cards::builders::{CardTextError,EffectAst,ObjectChoiceEffectAst,ChooseOneModeAst};
use crate::lexer::{OwnedLexToken,TokenKind};
pub(crate) fn parse(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
 if !crate::lexer::parser_token_word_refs(tokens).starts_with(&["choose","one","instruction"]){return Ok(None)}
 let Some(separator)=tokens.iter().position(|token|matches!(token.kind,TokenKind::EmDash|TokenKind::Colon)) else{return Ok(None)};
 if crate::lexer::parser_token_word_refs(&tokens[..separator])!=["choose","one","instruction"]{return Ok(None)}
 let mut quoted=false;let mut ends=Vec::new();
 for (index,token) in tokens.iter().enumerate().skip(separator+1){
  if token.kind==TokenKind::Quote{quoted=!quoted;}
  else if !quoted && token.kind==TokenKind::Pipe{ends.push(index);}
 }
 if quoted{return Err(CardTextError::ParseError("unterminated quotation in instruction menu".into()))}
 // A speculative sentence prefix can end after the first quoted child.
 // Without an outer separator there is no evidence for this complete owner.
 // Decline before parsing children or claiming the menu header.
 if ends.is_empty(){return Ok(None)}
 ends.push(tokens.len());let mut start=separator+1;let mut modes=Vec::new();
 for end in ends{
  let branch=&tokens[start..end];
  if crate::lexer::token_word_refs(branch).is_empty(){return Err(CardTextError::ParseError("empty instruction choice".into()))}
  modes.push(ChooseOneModeAst{description:crate::lexer::render_token_slice(branch),effects:crate::clause_support::parse_effect_sentences_lexed(branch)?});
  if end<tokens.len(){crate::word_claims::claim_tokens(&tokens[end..end+1]);}
  start=end+1;
 }
 if modes.len()<2{return Err(CardTextError::ParseError("instruction menu requires at least two alternatives".into()))}
 crate::word_claims::claim_tokens(&tokens[..separator+1]);
 Ok(Some(vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf{chooser:crate::target::PlayerFilter::You,modes})]))
}

#[cfg(test)]
mod complete_menu_evidence_tests {
    #[test]
    fn one_branch_and_quoted_pipes_do_not_commit_a_menu_reader() {
        for text in ["Choose one instruction — Draw a card.",
            "Choose one instruction — Create an artifact token named \"One | Two\"."] {
            let tokens=crate::lexer::lex_line(text,0).unwrap();
            assert!(super::parse(&tokens).unwrap().is_none());
        }
        let tokens=crate::lexer::lex_line("Choose one instruction — Draw a card. | Gain 1 life.",0).unwrap();
        assert!(super::parse(&tokens).unwrap().is_some());
    }
}
