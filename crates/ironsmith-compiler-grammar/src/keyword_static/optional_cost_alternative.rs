//! A full payment alternative names a cost receipt, not a keyword executor.
use super::*;
pub(super) fn parse(tokens:&[OwnedLexToken])->Result<Option<StaticAbility>,CardTextError>{
 let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
 let view=crate::lexer::TokenWordView::new(tokens);let words=view.word_refs();
 if !words.starts_with(&["you","may","pay"]){return Ok(None)}
 let Some(rather)=words.windows(4).position(|part|part==["rather","than","pay","the"]) else{return Ok(None)};
 let Some(cost)=words.windows(2).position(|part|part==["cost","for"]) else{return Ok(None)};
 if cost<=rather+4 || cost+2>=words.len(){return Ok(None)}
 let named=&words[rather+4..cost];
 // Official upkeep and equip alternatives remain under their existing owners.
 if named==["echo"] || named==["equip"] {return Ok(None)}
 let named=named.strip_prefix(&["optional","additional"]).unwrap_or(named);
 if named.is_empty(){return Ok(None)}
 let starts=view.token_start_indices();
 let replacement=crate::activation_and_restrictions::activated_line_core::parse_compiler_activation_cost(&tokens[starts[2]+1..starts[rather]])?;
 let filter=parse_object_filter(&tokens[starts[cost+2]..],false)?;
 let name=named.join(" ");let mut reference=ironsmith_core::OptionalCostRef::from(name.as_str());
 if matches!(reference.kind,ironsmith_core::OptionalCostKind::CustomUnsupported(_)){
  reference=ironsmith_core::OptionalCostRef::from("Additional");reference.custom_keyword=Some(name);
 }
 crate::word_claims::claim_tokens(tokens);
 Ok(Some(StaticAbility {id:Some(ironsmith_core::StaticAbilityId::OptionalCostPaymentAlternative),label:crate::lexer::render_token_slice(tokens),payload:ironsmith_core::StaticAbilityPayload::OptionalCostPaymentAlternative {filter,reference,replacement}}))
}
