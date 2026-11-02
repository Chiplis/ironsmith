//! Source-owned noted names, independent of draft-family/global-name notes.
use crate::lexer::{OwnedLexToken,TokenWordView};
use crate::cards::builders::{EffectAst,PredicateAst};
use crate::filter::ObjectFilter;
use crate::target::ChooseSpec;

fn source_owner(words:&[&str])->Option<ChooseSpec>{
    crate::util::is_source_reference_words(words).then_some(ChooseSpec::Source)
}
fn name_reference(words:&[&str])->Option<bool>{
    match words{
        ["the","triggering","card's","name"]|["the","triggering","object's","name"]=>Some(true),
        ["its","name"]=>Some(false),_=>None,
    }
}
pub(crate) fn split_clause(tokens:&[OwnedLexToken])->Option<(Vec<OwnedLexToken>,ChooseSpec)>{
    if tokens.iter().any(OwnedLexToken::is_quote){return None;}
    let view=TokenWordView::new(tokens);let words=view.word_refs();
    let at=words.windows(6).position(|part|part[..5]==["with","a","name","noted","for"])?;
    let owner=source_owner(&words[at+5..])?;
    let start=*view.token_start_indices().get(at)?;
    crate::word_claims::claim_tokens(&tokens[start..]);
    Some((tokens[..start].to_vec(),owner))
}
pub(crate) fn predicate(tokens:&[OwnedLexToken])->Option<PredicateAst>{
    if tokens.iter().any(OwnedLexToken::is_quote){return None;}
    let original=crate::util::trim_edge_punctuation_tokens(tokens);
    let tokens=if original.first().is_some_and(|token|token.is_word("if")){&original[1..]}else{original};
    if tokens.iter().any(OwnedLexToken::is_quote){return None;}
    let view=TokenWordView::new(tokens);let words=view.word_refs();
    let noted=words.windows(2).position(|part|part==["noted","for"])?;
    let owner=source_owner(&words[noted+2..])?;
    let (subject,negative)=if words.get(noted.wrapping_sub(1))==Some(&"is") {(&words[..noted-1],false)}
        else if words.get(noted.wrapping_sub(1)).is_some_and(|word|matches!(*word,"isnt"|"isn't")){(&words[..noted-1],true)}
        else if noted>=2 && words[noted-2..noted]==["is","not"]{(&words[..noted-2],true)}else{return None};
    let triggering=name_reference(subject)?;
    let filter=ObjectFilter {name_noted_for:Some(Box::new(owner)),..Default::default()};
    let predicate=if triggering{PredicateAst::TaggedMatches(crate::tag::CompilerReferenceTag::Triggering.bind(),filter)}else{PredicateAst::ItMatches(filter)};
    crate::word_claims::claim_tokens(original);
    Some(if negative{PredicateAst::Not(Box::new(predicate))}else{predicate})
}
pub(crate) fn action(tokens:&[OwnedLexToken])->Option<EffectAst>{
    if tokens.iter().any(OwnedLexToken::is_quote){return None;}
    let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
    if tokens.iter().any(OwnedLexToken::is_quote){return None;}
    let view=TokenWordView::new(tokens);let words=view.word_refs();
    if words.first()!=Some(&"note"){return None;}
    let at=words.iter().rposition(|word|*word=="for")?;
    let for_object=source_owner(&words[at+1..])?;
    let triggering=name_reference(&words[1..at])?;
    let tag=if triggering{crate::tag::CompilerReferenceTag::Triggering}else{crate::tag::CompilerReferenceTag::It};
    let object=ChooseSpec::Tagged(tag.bind().into());
    crate::word_claims::claim_tokens(tokens);
    Some(EffectAst::NoteObjectNames(ironsmith_core::NoteObjectNamesEffect{object,for_object,pair:None}))
}
