//! Definition substitution for a typed real-card creation proposal.
use super::*;
pub fn parse_card_creation_replacement_line(tokens:&[OwnedLexToken])->Result<Option<StaticAbility>,CardTextError>{
 let tokens=crate::util::trim_edge_punctuation_tokens(tokens);let view=TokenWordView::new(tokens);let words=view.word_refs();
 let (player,start)=if words.starts_with(&["if","a","player","would"]){(PlayerFilter::Any,4)}else if words.starts_with(&["if","you","would"]){(PlayerFilter::You,3)}else if words.starts_with(&["if","an","opponent","would"]){(PlayerFilter::Opponent,4)}else{return Ok(None);};
 if words.get(start..start+3)!=Some(&["create","real","cards"]){return Ok(None);}
 let Some(use_at)=words.windows(6).position(|part|part==["use","the","supplied","card","definition","named"])else{return Ok(None);};
 let after=start+3;let operation_label=if use_at==after{None}else{
  if words.get(after..after+5)!=Some(&["as","part","of","the","named"])||words.get(use_at-1)!=Some(&"action")||use_at<=after+6{return Ok(None);}
  Some(words[after+5..use_at-1].join(" "))
 };
 if words.last()!=Some(&"instead")||use_at+6>=words.len()-1{return Ok(None);}
 let Some(name_span)=view.token_span_for_words(use_at+6,words.len()-1)else{return Ok(None);};
 let name=crate::lexer::render_token_slice(&tokens[name_span]).trim_matches('"').trim().to_string();if name.is_empty(){return Ok(None);}
 let replacement=EffectAst::CreateCards {
  selection:crate::cards::builders::CardCreationSelectionAst::Named(name),count:Value::Fixed(1),player:PlayerAst::You,zone:Zone::Hand,
  library_position:ironsmith_core::CardCreationLibraryPosition::Top,operation_label:None,
  result_tag:crate::tag::TagRef::of(crate::util::helper_tag_for_tokens(tokens,"creation-replacement-definition")),
 };
 crate::word_claims::claim_tokens(tokens);
 Ok(Some(StaticAbility::card_creation_selection_replacement(operation_label,player,replacement,crate::lexer::render_token_slice(tokens))))
}
