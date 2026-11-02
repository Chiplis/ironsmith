//! A named payment selector changes only that casting-price component.
use super::*;
pub(super) fn parse(tokens:&[OwnedLexToken])->Result<Option<StaticAbility>,CardTextError>{
 let tokens=crate::util::trim_edge_punctuation_tokens(tokens);let view=crate::lexer::TokenWordView::new(tokens);let words=view.word_refs();
 if !words.starts_with(&["additional","casting","costs","named"]){return Ok(None)}
 let Some(end)=words.windows(3).position(|p|p==["you","pay","cost"])else{return Ok(None)};
 if end<=4 || words.last()!=Some(&"less"){return Ok(None)}
 let tail=&words[end+3..];let optional=tail.starts_with(&["up","to"]);
 let keywords=words[4..end].split(|w|matches!(*w,"or"|"and")).map(|part|part.join(" ")).collect::<Vec<_>>();
 if keywords.iter().any(|s|s.is_empty()||!s.chars().all(|c|c.is_alphabetic()||matches!(c,' '|'-'|'\''))){return Ok(None)}
 let starts=view.token_start_indices();let price_start=starts[end+2]+1+if optional{2}else{0};let less=starts[words.len()-1];
 if price_start>=less{return Ok(None)}
 let cost=crate::activation_and_restrictions::activated_line_core::parse_compiler_activation_cost(&tokens[price_start..less])?;
 if cost.has_non_mana_costs(){return Ok(None)}let Some(mana)=cost.mana_cost()else{return Ok(None)};
 let maximum=mana.generic_mana_total();if maximum==0 || mana.pips().iter().any(|pip|!matches!(pip.as_slice(),[crate::mana::ManaSymbol::Generic(_)])){return Ok(None)}
 crate::word_claims::claim_tokens(tokens);
 Ok(Some(StaticAbility{id:Some(ironsmith_core::StaticAbilityId::NamedCastingPriceReduction),label:crate::lexer::render_token_slice(tokens),payload:ironsmith_core::StaticAbilityPayload::NamedCastingPriceReduction{filter:ObjectFilter::default().cast_by(PlayerFilter::You),keywords,maximum,optional}}))
}
