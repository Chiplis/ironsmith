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
 let owners:[(&[&str],PlayerAst);7]=[
  (&["your"],PlayerAst::You),(&["target","player's"],PlayerAst::Target),
  (&["target","opponent's"],PlayerAst::TargetOpponent),(&["their"],PlayerAst::That),
  (&["that","player's"],PlayerAst::That),(&["its","controller's"],PlayerAst::ItsController),
  (&["its","owner's"],PlayerAst::ItsOwner),
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
