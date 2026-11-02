//! Typed public-reveal offers enclosing an independently parsed child program.
use super::*;

pub fn parse_reveal_action_offer_line(tokens:&[OwnedLexToken])->Result<Option<StaticAbility>,CardTextError>{
 let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
 let Some(colon)=tokens.iter().position(|token|token.kind==TokenKind::Colon)else{return Ok(None);};
 let header=&tokens[..colon];let words=TokenWordView::new(header);let refs=words.to_word_refs();
 let prefix=["as","you","reveal","this","card","from","a","hidden","zone","you","may","perform","the"];
 if !refs.starts_with(&prefix)||refs.len()<=prefix.len()+5{return Ok(None);}
 if !refs.ends_with(&["as","follows"]){return Ok(None);}
 let ending=refs.len()-5;
 if refs.get(ending..ending+2)!=Some(&["ending","in"]){return Ok(None);}
 let arrival=match refs[ending+2]{"exile"=>Zone::Exile,"hand"=>Zone::Hand,"library"=>Zone::Library,"graveyard"=>Zone::Graveyard,"battlefield"=>Zone::Battlefield,"command"=>Zone::Command,_=>return Ok(None)};
 let (name_end,maximum)=if ending>=4&&refs.get(ending-4..ending)==Some(&["action","once","each","turn"]){(ending-4,Some(1))}
 else if ending>=1&&refs[ending-1]=="action"{(ending-1,None)}else{return Ok(None);};
 if name_end<=prefix.len(){return Ok(None);}
 let Some(name_range)=words.token_span_for_words(prefix.len(),name_end)else{return Ok(None);};
 if header[name_range].iter().any(|token|!matches!(token.kind,TokenKind::Word|TokenKind::Quote)){return Ok(None);}
 let label=refs[prefix.len()..name_end].join(" ");
 let payload=&tokens[colon+1..];
 if payload.first().is_none_or(|token|token.kind!=TokenKind::LBracket)||payload.last().is_none_or(|token|token.kind!=TokenKind::RBracket){return Ok(None);}
 let mut depth=0usize;let mut quoted=false;
 for (index,token) in payload.iter().enumerate(){
  if token.kind==TokenKind::Quote{quoted=!quoted;continue;}if quoted{continue;}
  if token.kind==TokenKind::LBracket{depth+=1;}else if token.kind==TokenKind::RBracket{depth=depth.checked_sub(1).ok_or_else(||CardTextError::ParseError("unbalanced reveal offer program".into()))?;if depth==0&&index+1!=payload.len(){return Ok(None);}}
 }
 if quoted||depth!=0{return Err(CardTextError::ParseError("unbalanced reveal offer program".into()));}
 let effects=crate::clause_support::parse_effect_sentences_lexed(&payload[1..payload.len()-1])?;
 if effects.is_empty(){return Ok(None);}
 // Only this exact bounded declaration owns its framing words.
 crate::word_claims::claim_tokens(header);
 Ok(Some(StaticAbility::reveal_action_offer(label,vec![Zone::Hand,Zone::Library],maximum,arrival,effects,crate::lexer::render_token_slice(tokens))))
}
