//! A bounded copy-source replacement with an explicit supplied card name.
use super::*;
pub(super) fn read(raw:&[OwnedLexToken])->Result<Option<StaticAbilityAst>,CardTextError>{
 let tokens=crate::util::trim_edge_punctuation_tokens(raw);
 let view=crate::lexer::TokenWordView::new(tokens);let words=view.word_refs();
 let prefix=["if","a","player","would","copy","a","spell","or","cast","a","copy","they","copy","the","card","named"];
 let suffix=["instead","of","anything","else"];
 if !words.starts_with(&prefix)||!words.ends_with(&suffix)||words.len()<=prefix.len()+suffix.len(){return Ok(None);}
 let start=view.token_index_after_words(prefix.len()).unwrap_or(view.token_start_indices()[prefix.len()]);
 let end=view.token_start_indices()[words.len()-suffix.len()];
 let mut name_tokens=&tokens[start..end];
 // Quotes are an optional name delimiter, never part of the supplied identity.
 if name_tokens.first().is_some_and(|t|t.kind==crate::lexer::TokenKind::Quote){
  if !name_tokens.last().is_some_and(|t|t.kind==crate::lexer::TokenKind::Quote){return Ok(None);}
  name_tokens=&name_tokens[1..name_tokens.len()-1];
 }
 let definition_name=crate::lexer::render_token_slice(name_tokens).trim().to_string();
 if definition_name.is_empty()||name_tokens.iter().any(|t|matches!(t.kind,crate::lexer::TokenKind::Quote|crate::lexer::TokenKind::Period)){return Ok(None);}
 crate::word_claims::claim_tokens(tokens);
 Ok(Some(StaticAbilityAst::CopySourceSelectionReplacement{player:PlayerFilter::Any,definition_name,display:crate::lexer::render_token_slice(raw)}))
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn copy_definition_replacement_binds_explicit_full_names_and_rejects_other_operations(){
  for name in ["Other-Card", "\"A Different Identity\""] {
   let text=format!("If a player would copy a spell or cast a copy, they copy the card named {name} instead of anything else.");
   let parsed=read(&crate::lexer::lex_line(&text,0).unwrap()).unwrap().expect("bounded copy source replacement");
   let StaticAbilityAst::CopySourceSelectionReplacement {definition_name,..}=parsed else{panic!("typed replacement")};assert_eq!(definition_name,name.trim_matches('"'));
  }
  for text in ["If a player would copy an ability or cast a copy, they copy the card named Other-Card instead of anything else.","If a player would copy a spell or cast a copy, they copy the card named Other-Card."] {
   assert!(read(&crate::lexer::lex_line(text,0).unwrap()).unwrap().is_none());
  }
 }
}
