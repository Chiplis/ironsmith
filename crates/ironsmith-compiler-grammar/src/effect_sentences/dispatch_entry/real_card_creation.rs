//! Creation of real cards from an explicitly supplied definition pool.
use crate::object_filters::parse_object_filter_lexed;
use super::*;
use crate::cards::builders::CardCreationSelectionAst;
pub(crate) fn read(tokens:&[OwnedLexToken])->Result<Option<EffectAst>,CardTextError>{
 let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
 let view=crate::lexer::TokenWordView::new(tokens);let words=view.word_refs();
 if words.first()!=Some(&"create"){return Ok(None);}
 let Some(real)=words.iter().position(|word|*word=="real")else{return Ok(None);};
 if real<2||!matches!(words.get(real+1),Some(&"card"|&"cards")){return Ok(None);}
 let Some(count_span)=view.token_span_for_words(1,real)else{return Ok(None);};
 let count_tokens=&tokens[count_span];
 let count=if crate::lexer::parser_token_word_refs(count_tokens)==["a"]{crate::effect::Value::Fixed(1)}else{
  let Some((value,used))=crate::util::parse_value(count_tokens)else{return Ok(None);};
  if used!=count_tokens.len(){return Ok(None);}value
 };
 let mut end=words.len();let mut operation_label=None;
 if words.last()==Some(&"action"){
  if let Some(at)=words.windows(5).rposition(|part|part==["as","part","of","the","named"]){
   if at+5>=end-1{return Ok(None);}operation_label=Some(words[at+5..end-1].join(" "));end=at;
  }
 }
 // Word views spell possessives without the apostrophe ("players").
 let owners:[(&[&str],PlayerAst);12]=[
  (&["your"],PlayerAst::You),(&["target","player's"],PlayerAst::Target),
  (&["target","opponent's"],PlayerAst::TargetOpponent),(&["their"],PlayerAst::That),
  (&["that","player's"],PlayerAst::That),(&["its","controller's"],PlayerAst::ItsController),
  (&["its","owner's"],PlayerAst::ItsOwner),
  (&["target","players"],PlayerAst::Target),(&["target","opponents"],PlayerAst::TargetOpponent),
  (&["that","players"],PlayerAst::That),(&["its","controllers"],PlayerAst::ItsController),
  (&["its","owners"],PlayerAst::ItsOwner),
 ];
 let mut destinations=Vec::new();
 for (owner,player) in owners {
  for (prefix,suffix,zone,position) in [
   (&["in"][..],"hand",crate::zone::Zone::Hand,ironsmith_core::CardCreationLibraryPosition::Top),
   (&["in"][..],"graveyard",crate::zone::Zone::Graveyard,ironsmith_core::CardCreationLibraryPosition::Top),
   (&["on","top","of"][..],"library",crate::zone::Zone::Library,ironsmith_core::CardCreationLibraryPosition::Top),
   (&["on","the","bottom","of"][..],"library",crate::zone::Zone::Library,ironsmith_core::CardCreationLibraryPosition::Bottom),
  ] {
   let mut phrase=prefix.to_vec();phrase.extend(owner);phrase.push(suffix);
   destinations.push((phrase,zone,player,position));
  }
 }
 destinations.push((vec!["on","the","battlefield"],crate::zone::Zone::Battlefield,PlayerAst::You,ironsmith_core::CardCreationLibraryPosition::Top));
 destinations.push((vec!["in","exile"],crate::zone::Zone::Exile,PlayerAst::You,ironsmith_core::CardCreationLibraryPosition::Top));
 let Some((phrase,zone,player,library_position))=destinations.iter().find(|(phrase,..)|words[..end].ends_with(phrase.as_slice()))else{return Ok(None);};
 let subject_end=end-phrase.len();let start=real+2;
 let Some(destination_start)=view.token_start_indices().get(subject_end).copied()else{return Ok(None);};
 if tokens[..destination_start].iter().filter(|token|token.kind==TokenKind::Quote).count()%2!=0{return Ok(None);}
 if subject_end<=start{return Ok(None);}
 let selection=if words.get(start..subject_end)==Some(&["with","the","chosen","name"]){CardCreationSelectionAst::ChosenName}
 else if words.get(start..start+3)==Some(&["for","each","name"]){
  let Some(range)=view.token_span_for_words(start+3,subject_end)else{return Ok(None);};
  let begin=if range.start>0&&tokens[range.start-1].kind==TokenKind::Quote{range.start-1}else{range.start};
  let finish=if tokens.get(range.end).is_some_and(|token|token.kind==TokenKind::Quote){range.end+1}else{range.end};
  let operand=&tokens[begin..finish];let mut quoted=false;let mut current=Vec::new();let mut names=Vec::new();
  for token in operand {
   if token.kind==TokenKind::Quote{if quoted{let name=crate::lexer::render_token_slice(&current);if name.trim().is_empty(){return Ok(None);}names.push(name);current.clear();}quoted=!quoted;}
   else if quoted{current.push(token.clone());}
   else if token.kind!=TokenKind::Comma&&!token.is_word("and"){return Ok(None);}
  }
  if quoted||names.is_empty(){return Ok(None);}CardCreationSelectionAst::AllNamed(names)
 }else if words.get(start)==Some(&"named"){
  let Some(name)=view.token_span_for_words(start+1,subject_end)else{return Ok(None);};
  let name=crate::lexer::render_token_slice(&tokens[name]).trim_matches('"').trim().to_string();
  if name.is_empty(){return Ok(None);}CardCreationSelectionAst::Named(name)
 }else if words.get(start..start+2)==Some(&["matching","randomly"]){
  let independent=words.get(subject_end-1)==Some(&"independently");let stop=subject_end-usize::from(independent);
  let Some(quality)=view.token_span_for_words(start+2,stop)else{return Ok(None);};
  let mut filter=parse_object_filter_lexed(&tokens[quality],false)?;filter.zone=None;
  CardCreationSelectionAst::RandomMatching{filter,independent}
 }else if words.get(start..start+2)==Some(&["duplicating","the"]){
  let Some(target)=view.token_span_for_words(start+2,subject_end)else{return Ok(None);};
  CardCreationSelectionAst::Duplicate(parse_target_phrase(&tokens[target])?)
 }else{return Ok(None);};
 // The finite frame owns creation/count/destination/label; selectors above
 // have their own typed readers. No unrelated source tail is consumed.
 if let Some(prefix)=view.token_span_for_words(0,start){crate::word_claims::claim_tokens(&tokens[prefix]);}
 if let Some(tail)=view.token_span_for_words(subject_end,words.len()){crate::word_claims::claim_tokens(&tokens[tail]);}
 if matches!(selection,CardCreationSelectionAst::Named(_)|CardCreationSelectionAst::AllNamed(_)|CardCreationSelectionAst::ChosenName){
  if let Some(name)=view.token_span_for_words(start,subject_end){crate::word_claims::claim_tokens(&tokens[name]);}
 }
 if !matches!(selection,CardCreationSelectionAst::Named(_)|CardCreationSelectionAst::AllNamed(_)|CardCreationSelectionAst::ChosenName) {
  if let Some(frame)=view.token_span_for_words(start,start+2){crate::word_claims::claim_tokens(&tokens[frame]);}
  if matches!(&selection,CardCreationSelectionAst::RandomMatching{independent:true,..}) {
   if let Some(frame)=view.token_span_for_words(subject_end-1,subject_end){crate::word_claims::claim_tokens(&tokens[frame]);}
  }
 }
 let result_tag=crate::tag::TagRef::of(crate::util::helper_tag_for_tokens(tokens,"created-real-cards"));
 Ok(Some(EffectAst::CreateCards {selection,count,player:*player,zone:*zone,library_position:*library_position,operation_label,result_tag}))
}

/// Arena "conjure": "conjure <count> <Name> card(s) into <zone>". Wizards'
/// Alchemy rules: a conjured card is created from outside the game and put
/// into the named zone; it is a real card (not a token) owned by the
/// conjuring player.
pub(crate) fn read_conjure(tokens:&[OwnedLexToken])->Result<Option<EffectAst>,CardTextError>{
 let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
 let view=crate::lexer::TokenWordView::new(tokens);let words=view.word_refs();
 if words.first()!=Some(&"conjure"){return Ok(None);}
 let owners:[(&[&str],PlayerAst);4]=[
  (&["your"],PlayerAst::You),(&["target","player's"],PlayerAst::Target),
  (&["target","players"],PlayerAst::Target),(&["target","opponents"],PlayerAst::TargetOpponent),
 ];
 let mut destinations:Vec<(Vec<&str>,crate::zone::Zone,PlayerAst,ironsmith_core::CardCreationLibraryPosition)>=Vec::new();
 for (owner,player) in owners {
  for (prefix,suffix,zone,position) in [
   (&["into"][..],"hand",crate::zone::Zone::Hand,ironsmith_core::CardCreationLibraryPosition::Top),
   (&["into"][..],"graveyard",crate::zone::Zone::Graveyard,ironsmith_core::CardCreationLibraryPosition::Top),
   (&["on","top","of"][..],"library",crate::zone::Zone::Library,ironsmith_core::CardCreationLibraryPosition::Top),
   (&["on","the","bottom","of"][..],"library",crate::zone::Zone::Library,ironsmith_core::CardCreationLibraryPosition::Bottom),
  ] {
   let mut phrase=prefix.to_vec();phrase.extend(owner);phrase.push(suffix);
   destinations.push((phrase,zone,player,position));
  }
 }
 destinations.push((vec!["onto","the","battlefield"],crate::zone::Zone::Battlefield,PlayerAst::You,ironsmith_core::CardCreationLibraryPosition::Top));
 destinations.push((vec!["into","exile"],crate::zone::Zone::Exile,PlayerAst::You,ironsmith_core::CardCreationLibraryPosition::Top));
 let Some((phrase,zone,player,library_position))=destinations.iter().find(|(phrase,..)|words.ends_with(phrase.as_slice()))else{return Ok(None);};
 let subject_end=words.len()-phrase.len();
 if subject_end<3||!matches!(words[subject_end-1],"card"|"cards"){return Ok(None);}
 let name_end=subject_end-1;
 // Count: "a"/"an", a number word, or a value phrase of one word ("X").
 let (count,name_start)=match words[1] {
  "a"|"an"=>(crate::effect::Value::Fixed(1),2),
  _=>{
   let Some(span)=view.token_span_for_words(1,2)else{return Ok(None);};
   let Some((value,used))=crate::util::parse_value(&tokens[span.clone()])else{return Ok(None);};
   if used!=tokens[span].len(){return Ok(None);}
   (value,2)
  }
 };
 if name_end<=name_start{return Ok(None);}
 let Some(name_span)=view.token_span_for_words(name_start,name_end)else{return Ok(None);};
 let name=crate::lexer::render_token_slice(&tokens[name_span]).trim_matches('"').trim().to_string();
 if name.is_empty(){return Ok(None);}
 crate::word_claims::claim_tokens(tokens);
 let result_tag=crate::tag::TagRef::of(crate::util::helper_tag_for_tokens(tokens,"conjured-cards"));
 Ok(Some(EffectAst::CreateCards{selection:CardCreationSelectionAst::Catalog(name),count,player:*player,zone:*zone,library_position:*library_position,operation_label:None,result_tag}))
}

/// Arena "seek": "seek <count> <quality> card(s)". Wizards' Alchemy rules:
/// put that many cards matching the quality at random from your library
/// into your hand; if fewer match, seek only those. Read as the equivalent
/// random library-to-hand move.
pub(crate) fn read_seek(tokens:&[OwnedLexToken])->Result<Option<EffectAst>,CardTextError>{
 let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
 let view=crate::lexer::TokenWordView::new(tokens);let words=view.word_refs();
 if words.first()!=Some(&"seek")||words.len()<3{return Ok(None);}
 if !matches!(words.last(),Some(&"card"|&"cards")){return Ok(None);}
 let Some(span)=view.token_span_for_words(1,words.len())else{return Ok(None);};
 // Splice the descriptor's own tokens into the equivalent move phrase
 // instead of rendering them back to text and lexing again.
 let synthetic=|word:&str|OwnedLexToken::word(word,crate::cards::builders::TextSpan::synthetic());
 let mut rewritten=Vec::with_capacity(span.len()+9);
 rewritten.push(synthetic("put"));
 rewritten.extend_from_slice(&tokens[span]);
 rewritten.extend(["at","random","from","your","library","into","your","hand"].into_iter().map(synthetic));
 let effect=crate::effect_sentences::parse_effect_clause_lexed(&rewritten)?;
 crate::word_claims::claim_tokens(tokens);
 Ok(Some(effect))
}
