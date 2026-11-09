//! One atomic activation instruction, shared by sentence and nested clause readers.
use crate::cards::builders::{EffectAst,KeywordActionAst,PlayerAst,SubjectVerbActionAst,SubjectVerbRoleAst,TargetAst};
use crate::lexer::{OwnedLexToken,TokenWordView};
use crate::CardTextError;
pub(crate) fn parse(tokens:&[OwnedLexToken])->Result<Option<EffectAst>,CardTextError>{
 if let Some(effect)=parse_cast_or_activation(tokens)? {return Ok(Some(effect));}
 let end=tokens.iter().rposition(|token|!token.is_period()).map_or(0,|index|index+1);
 let tokens=&tokens[..end];let view=TokenWordView::new(tokens);let words=view.word_refs();
 if words.first()!=Some(&"activate"){return Ok(None);}
 let Some(without)=words.windows(2).position(|pair|pair==["without","paying"])else{return Ok(None);};
 let (price,invocation_action)=if let Some(during)=words[without+2..].iter().position(|word|*word=="during"){
  let during=during+without+2;
  if words.last()!=Some(&"actions")||during+2>=words.len(){return Ok(None);}
  // "during making-a-pledge actions": the action's name, hyphenated as the
  // activation exception "unless you're making a pledge" names it.
  (&words[without+2..during],Some(words[during+1..words.len()-1].join("-")))
 }else{(&words[without+2..],None)};
 if !matches!(price,["its","cost"|"costs"]|["its","activation","cost"|"costs"]|["that","ability's"|"abilitys","cost"|"costs"]){return Ok(None);}
 let (target,designation)=match &words[..without]{
  ["activate","one","of","its","activated","abilities"] =>(TargetAst::Source(None),None),
  ["activate","one","of","this","permanent's"|"permanents"|"creature's"|"creatures","activated","abilities"] =>(TargetAst::Source(None),None),
  prefix if prefix.starts_with(&["activate","an","activated","ability","with","designation"]) && prefix.get(7)==Some(&"of")=>{
   let start=view.token_start_indices()[8];let end=view.token_start_indices()[without];
   (crate::util::parse_target_phrase(&tokens[start..end])?,Some(words[6].to_string()))
  },
  prefix if prefix.starts_with(&["activate","an","activated","ability","of"])=>{
   let start=view.token_start_indices()[5];let end=view.token_start_indices()[without];
   (crate::util::parse_target_phrase(&tokens[start..end])?,None)
  },
  _=>return Ok(None),
 };
 crate::word_claims::claim_tokens(tokens);
 Ok(Some(EffectAst::subject_verb(SubjectVerbRoleAst::Actor,PlayerAst::Implicit,
  SubjectVerbActionAst::KeywordActions(KeywordActionAst::ActivateAbilityWithoutCosts{target,designation,invocation_action}))))
}

/// Resolution-time alternatives preserve the native cast and activation owners.
/// An explicit optional actor scopes both arms; this is not an announced mode.
fn parse_cast_or_activation(tokens:&[OwnedLexToken])->Result<Option<EffectAst>,CardTextError>{
 let words=crate::lexer::parser_token_word_refs(tokens);
 if !words.starts_with(&["you","may","cast"]){return Ok(None);}
 let mut quoted=false;
 let mut separators=Vec::new();
 for (index,token) in tokens.iter().enumerate(){
  if token.kind==crate::lexer::TokenKind::Quote{quoted=!quoted;}
  else if !quoted && token.is_word("or") && tokens.get(index+1).is_some_and(|next|next.is_word("activate")){separators.push(index);}
 }
 if quoted || separators.len()!=1{return Ok(None);}
 let separator=separators[0];
 let Some(activation)=parse(&tokens[separator+1..])? else{return Ok(None);};
 let Some(cast)=crate::permission_helpers::parse_cast_or_play_tagged_clause_keeping_may(&tokens[..separator])? else{return Ok(None);};
 crate::word_claims::claim_tokens(&tokens[separator..separator+1]);
 Ok(Some(EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseOneOf{
  chooser:crate::target::PlayerFilter::You,
  modes:vec![
   crate::cards::builders::ChooseOneModeAst{description:crate::lexer::render_token_slice(&tokens[..separator]),effects:vec![cast]},
   crate::cards::builders::ChooseOneModeAst{description:crate::lexer::render_token_slice(&tokens[separator+1..]),effects:vec![EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::May{effects:vec![activation]})]},
  ],
 })))
}
#[cfg(test)]
mod alternative_action_tests{
 #[test]
 fn optional_cast_or_designated_activation_keeps_two_native_instruction_owners(){
  let text="You may cast an enchantment spell from your hand without paying its mana cost or activate an activated ability with designation sample of a permanent you control without paying its costs during sample actions.";
  let tokens=crate::lexer::lex_line(text,0).unwrap();
  let effect=super::parse(&tokens).unwrap().expect("complete optional alternatives");
  let crate::cards::builders::EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseOneOf{modes,..})=effect else{panic!("resolution-time choice");};
  assert_eq!(modes.len(),2);
  assert!(matches!(modes[0].effects.as_slice(),[crate::cards::builders::EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost{zone:crate::zone::Zone::Hand,..})]));
  let crate::cards::builders::EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::May{effects})=&modes[1].effects[0] else{panic!("activation remains optional");};
  assert!(format!("{effects:?}").contains("ActivateAbilityWithoutCosts"));
  assert!(format!("{effects:?}").contains("sample"));
 }
 #[test]
 fn unrelated_or_in_a_cast_filter_is_not_an_activation_alternative(){
  let tokens=crate::lexer::lex_line("You may cast an enchantment or instant spell from your hand without paying its mana cost.",0).unwrap();
  assert!(super::parse(&tokens).unwrap().is_none());
 }
}
