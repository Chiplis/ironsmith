//! Bounded generic source-backed deck choice and optional opening-hand exchange.
use crate::lexer::{OwnedLexToken,TokenKind};
use crate::CardTextError;
use crate::model::CompilerStaticAbilityCore;
use crate::static_abilities::PregameActionKind;
struct Cursor<'a>{tokens:&'a [OwnedLexToken],at:usize}
impl Cursor<'_>{
 fn words(&mut self,words:&[&str])->bool{
  if words.iter().enumerate().all(|(i,word)|self.tokens.get(self.at+i).is_some_and(|token|token.is_word(word))){self.at+=words.len();true}else{false}
 }
 fn kind(&mut self,kind:TokenKind)->bool{if self.tokens.get(self.at).is_some_and(|t|t.kind==kind){self.at+=1;true}else{false}}
 fn quoted(&mut self)->Option<String>{
  if !self.kind(TokenKind::Quote){return None;}
  let start=self.at;
  while self.tokens.get(self.at).is_some_and(|t|!matches!(t.kind,TokenKind::Quote|TokenKind::Period)){self.at+=1;}
  if self.at==start||!self.kind(TokenKind::Quote){return None;}
  Some(crate::lexer::render_token_slice(&self.tokens[start..self.at-1]).trim().to_string())
 }
}
pub(super) fn parse(tokens:&[OwnedLexToken])->Result<Option<CompilerStaticAbilityCore>,CardTextError>{
 let mut c=Cursor{tokens,at:0};
 if !c.words(&["chosen","outside","game","card"]){return Ok(None);}
 let invalid=||CardTextError::ParseError("chosen outside-game card requires a quoted designation, complete printed-text deck criterion, and optional opening-hand exchange".into());
 let designation=c.quoted().ok_or_else(invalid)?;
 if !c.kind(TokenKind::Colon)||!c.words(&["each","nonland","card","in","your","starting","deck","has"]){return Err(invalid());}
 let mut phrases=vec![c.quoted().ok_or_else(invalid)?];
 while c.words(&["or"]){phrases.push(c.quoted().ok_or_else(invalid)?);}
 if !c.words(&["in","its","printed","rules","text"])||!c.kind(TokenKind::Period)
  ||!c.words(&["you","may","exchange","this","card","with","a","card","in","your","opening","hand"]){return Err(invalid());}
 c.kind(TokenKind::Period);
 if c.at!=tokens.len(){return Err(invalid());}
 crate::word_claims::claim_tokens(tokens);
 Ok(Some(CompilerStaticAbilityCore::pregame_action(PregameActionKind::ChosenOutsideGame(ironsmith_core::PregameChosenCardSpec{
  designation,deck_condition:ironsmith_core::PregameDeckCondition::EveryCardPrintedTextContainsAny{exempt_types:vec![crate::types::CardType::Land],phrases},
  action:ironsmith_core::PregameChosenCardAction::ExchangeWithOpeningHand,
 }),crate::lexer::render_token_slice(tokens))))
}
