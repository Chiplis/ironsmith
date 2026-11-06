//! Explicit custom identity operands. Names are quoted data, not keywords.
use crate::lexer::{OwnedLexToken,TokenKind};
pub(crate) fn filter_clause(tokens:&[OwnedLexToken])->Option<(Vec<OwnedLexToken>,String,bool)>{
 let mut quoted=false;
 for start in 0..tokens.len(){
  if tokens[start].kind==TokenKind::Quote{quoted=!quoted;continue;}
  let offset=if tokens.get(start+1).is_some_and(|token|token.is_word("the")){1}else{0};
  let payment=tokens[start].is_word("using");
  if quoted || !(payment || tokens[start].is_any_word(&["with","without"]))
    || !tokens.get(start+1+offset).is_some_and(|token|token.is_word("custom"))
    || !tokens.get(start+2+offset).is_some_and(|token|token.is_word(if payment {"cost"} else {"keyword"}))
    || !tokens.get(start+3+offset).is_some_and(|token|token.kind==TokenKind::Quote){continue;}
  // A clause boundary trims the closing quote of a final operand ("if this
  // spell is a copy with custom keyword \"X\", ..."); the end of the clause
  // then closes the name.
  let close=(start+4+offset..tokens.len()).find(|index|tokens[*index].kind==TokenKind::Quote)
   .unwrap_or(tokens.len());
  if close==start+4+offset{return None;}
  let name=crate::lexer::render_literal_token_slice(&tokens[start+4+offset..close]).split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase();
  if name.is_empty(){return None;}
  let cast_start=if payment && start>=3 && tokens[start-3].is_word("that") && tokens[start-2].is_word("was") && tokens[start-1].is_word("cast") {Some(start-3)}
   else if payment && start>=1 && tokens[start-1].is_word("cast") {Some(start-1)}else{None};
  let marker_start=cast_start.unwrap_or(start);
  let mut base=tokens[..marker_start].to_vec();base.extend_from_slice(tokens.get(close+1..).unwrap_or_default());
  crate::word_claims::claim_tokens(&tokens[marker_start..close.min(tokens.len()-1)+1]);
  let marker=if cast_start.is_some(){format!("cast using {name}")}else if payment{format!("using {name}")}else{name};
  return Some((base,marker,tokens[start].is_word("without")));
 }
 None
}
pub(crate) fn has_unquoted_union(tokens:&[OwnedLexToken])->bool{
 let mut quoted=false;for token in tokens{if token.kind==TokenKind::Quote{quoted=!quoted;}else if !quoted&&token.is_any_word(&["and","or","and/or"]){return true;}}false
}
#[cfg(test)]mod tests{
 #[test]fn an_explicit_named_price_operand_is_distinct_from_keyword_identity(){
  use crate::object_filters::parse_object_filter;
  let tokens=crate::lexer::lex_line("another creature you control using custom cost \"Aerial Passage\"",0).unwrap();
  let filter=parse_object_filter(&tokens,false).unwrap();
  assert_eq!(filter.ability_markers,vec!["using aerial passage"]);
  assert!(filter.other);
  assert_eq!(filter.controller,Some(crate::target::PlayerFilter::You));
  let tokens=crate::lexer::lex_line("a creature with custom keyword \"Aerial Passage\"",0).unwrap();
  let identity=parse_object_filter(&tokens,false).unwrap();
  assert_eq!(identity.ability_markers,vec!["aerial passage"]);
  let tokens=crate::lexer::lex_line("another creature you control that was cast using custom cost \"Aerial Passage\"",0).unwrap();
  let actual=parse_object_filter(&tokens,false).unwrap();
  assert_eq!(actual.ability_markers,vec!["cast using aerial passage"]);
 }
 #[test]fn identity_operand_names_and_union_branches_are_independent(){
  use crate::object_filters::parse_object_filter;
  for name in ["breach","seven seas","doctor's spark"]{
   let tokens=crate::lexer::lex_line(&format!("creatures without custom keyword \"{name}\" you control"),0).unwrap();let filter=parse_object_filter(&tokens,false).unwrap();assert_eq!(filter.excluded_ability_markers,vec![name]);
  }
  let tokens=crate::lexer::lex_line("Curses and creatures with custom keyword \"breach\" you control",0).unwrap();let filter=parse_object_filter(&tokens,false).unwrap();assert_eq!(filter.any_of.len(),2);assert!(filter.any_of[0].ability_markers.is_empty());assert_eq!(filter.any_of[1].ability_markers,vec!["breach"]);
 }
}

#[cfg(test)]
mod recursive_branch_entry_tests {
    use super::*;
    #[test]
    fn grammar_union_arms_preserve_custom_identity_operands() {
        for (name, without) in [("breach", false), ("moon compose", true)] {
            let relation = if without { "without" } else { "with" };
            let text = format!("other Curses and creatures {relation} custom keyword \"{name}\" you control");
            let tokens = crate::lexer::lex_line(&text, 0).unwrap();
            let filter = crate::object_filters::parse_object_filter_lexed(&tokens, false).unwrap();
            assert_eq!(filter.any_of.len(), 2, "{filter:?}");
            let creature = &filter.any_of[1];
            assert!(filter.any_of[0].ability_markers.is_empty());
            assert!(filter.any_of[0].excluded_ability_markers.is_empty());
            if without { assert_eq!(creature.excluded_ability_markers, [name]); }
            else { assert_eq!(creature.ability_markers, [name]); }
            assert!(creature.other || filter.other, "shared other constraint survives");
            assert!(creature.controller.is_some() || filter.controller.is_some(), "shared controller survives");
        }
    }
    #[test]
    fn grammar_direct_identity_operand_keeps_exact_name() {
        let tokens = crate::lexer::lex_line("creatures with custom keyword \"moon compose\"", 0).unwrap();
        let filter = crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.ability_markers, ["moon compose"]);
    }
}
