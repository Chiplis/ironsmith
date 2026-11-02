//! Independently read one directed source-target relationship. Choices and
//! optional branches remain owned by the ordinary composition grammar.
use crate::cards::builders::{CardTextError,EffectAst,KeywordActionAst,SubjectVerbActionAst,SubjectVerbRoleAst,PlayerAst,OwnedLexToken};
pub(super) fn parse(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
 let tokens=crate::util::trim_edge_punctuation_tokens(tokens);let words=crate::lexer::parser_token_word_refs(tokens);
 if !words.starts_with(&["link","this","permanent","to"]){return Ok(None)}
 let Some(by)=words.iter().position(|word|*word=="by")else{return Ok(None)};
 let Some(relation)=words.iter().position(|word|*word=="relation")else{return Ok(None)};
 if by<=4||relation<=by+1{return Ok(None)}
 let suffix=&words[relation+1..];
 let require_creatures=match suffix {
  ["for","as","long","as","both","remain","permanents","on","the","battlefield","under","your","control"]=>false,
  ["for","as","long","as","both","remain","creatures","on","the","battlefield","under","your","control"]=>true,
  _=>return Ok(None),
 };
 let label=&words[by+1..relation];let label=label.strip_prefix(&["the"]).unwrap_or(label);
 if label.is_empty()||label.iter().any(|word|word.is_empty()||!word.chars().all(|ch|ch.is_ascii_alphabetic()||matches!(ch,'-'|'\''))){return Ok(None)}
 let clause=crate::lexer::LexedClause::new(tokens);let start=clause.token_index_after_words(4).unwrap_or(4);let end=clause.token_index_after_words(by).unwrap_or(by);
 let target=crate::effect_sentences::parse_target_phrase(&tokens[start..end])?;
 crate::word_claims::claim_words(&words[..4]);crate::word_claims::claim_words(&words[by..]);
 Ok(Some(vec![EffectAst::subject_verb(SubjectVerbRoleAst::Chooser,PlayerAst::You,SubjectVerbActionAst::KeywordActions(KeywordActionAst::LinkPermanents{target,relation:label.join(" "),require_creatures}))]))
}
