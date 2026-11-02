//! Independent operands and explicit state-transfer options for a generic merge.
use super::*;
pub(super) fn read(tokens:&[OwnedLexToken])->Result<Option<ironsmith_core::MergePermanentsEffect>,CardTextError>{
 let tokens=crate::util::trim_edge_punctuation_tokens(tokens);if !tokens.first().is_some_and(|t|t.is_word("merge")){return Ok(None)}
 let Some(with)=tokens.iter().position(|t|t.is_word("with"))else{return Ok(None)};
 let Some(comma)=tokens.iter().position(|t|t.is_comma())else{return Ok(None)};if with<=1||comma<=with+1{return Ok(None)}
 let incoming=crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(&tokens[1..with],false)?;
 let host=crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(&tokens[with+1..comma],false)?;
 let options=crate::lexer::TokenWordView::new(&tokens[comma+1..]).word_refs();
 let Some(transfer)=options.strip_prefix(&["choosing","which","is","on","top","and","moving","all"])else{return Ok(None)};
 let (transfer_counters,transfer_attachments)=match transfer{
  ["counters","and","attachments","to","the","merged","permanent"] =>(true,true),
  ["counters","to","the","merged","permanent"] =>(true,false),
  ["attachments","to","the","merged","permanent"] =>(false,true),_=>return Ok(None),
 };
 crate::word_claims::claim_tokens(&tokens[..1]);crate::word_claims::claim_tokens(&tokens[with..with+1]);crate::word_claims::claim_tokens(&tokens[comma..]);
 Ok(Some(ironsmith_core::MergePermanentsEffect{incoming,host,choose_top:true,transfer_counters,transfer_attachments}))
}
