//! Grouping reads subjects, optional owner and combat requirement separately.
use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::OwnedLexToken;
pub fn parse(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
 let clean=crate::util::trim_edge_punctuation_tokens(tokens);
 if !clean.first().is_some_and(|token|token.is_word("group")){return Ok(None)}
 let so=clean.iter().position(|token|token.is_word("so")).unwrap_or(clean.len());
 let rule=crate::lexer::token_word_refs(&clean[so..]);
 let owned=rule==["so","all","members","attack","and","block","if","any","member","or","the","owner","does","if","able"];
 let ownerless=rule==["so","all","members","attack","and","block","if","any","member","does","if","able"];
 if so<clean.len()&&!owned&&!ownerless{return Ok(None)}
 let body=&clean[1..so];let with=body.iter().position(|token|token.is_word("under"));
 if so<clean.len()&&owned!=with.is_some(){return Ok(None)}
 let subject_tokens=if let Some(with)=with{&body[..with]}else{body};
 // "Group each token created this way": every member of the created set,
 // the same subject as "the tokens created this way".
 let distributive=crate::lexer::token_word_refs(subject_tokens)==["each","token","created","this","way"];
 let subjects=if distributive {
  let plural=crate::lexer::synthetic_word_tokens(["the","tokens","created","this","way"]);
  let subjects=crate::util::parse_target_phrase(&plural)?;crate::word_claims::claim_tokens(subject_tokens);subjects
 } else {crate::util::parse_target_phrase(subject_tokens)?};
 let owner=if let Some(with)=with{Some(crate::model::ast::choose_spec_for_target(&crate::util::parse_target_phrase(&body[with+1..])?))}else{None};
 crate::word_claims::claim_tokens(&clean[..1]);crate::word_claims::claim_tokens(&clean[so..]);if let Some(with)=with{crate::word_claims::claim_tokens(&body[with..with+1]);}
 Ok(Some(vec![EffectAst::LinkObjectsToGroup(ironsmith_core::LinkObjectsToGroupEffect{groups:vec![ironsmith_core::ObjectGroupAssignment{subjects:crate::model::ast::choose_spec_for_target(&subjects),owner,combat_together:so<clean.len()}]})]))
}
