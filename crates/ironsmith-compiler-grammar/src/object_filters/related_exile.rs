//! Named exile relationships qualify ordinary card and permanent filters.
use super::*;
use ironsmith_core::filter_model::{RelatedExileAnchor,RelatedExileMembers};
use crate::target::ChooseSpec;
fn host(tokens:&[OwnedLexToken])->Result<ChooseSpec,CardTextError>{
    let words=crate::lexer::parser_token_word_refs(tokens);
    if crate::util::is_source_reference_words(&words)||matches!(words.as_slice(),["it"]|["them"]){
        crate::word_claims::claim_tokens(tokens);return Ok(ChooseSpec::Source);
    }
    if matches!(words.as_slice(),["that","permanent"]|["that","creature"]|["the","chosen","permanent"]){
        crate::word_claims::claim_tokens(tokens);
        return Ok(ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.key()));
    }
    let mut filter=parse_object_filter_lexed(tokens,false)?;filter.zone.get_or_insert(Zone::Battlefield);
    Ok(ChooseSpec::All(filter))
}
pub(super) fn read(tokens:&[OwnedLexToken],other:bool)->Result<Option<ObjectFilter>,CardTextError>{
    let clean=crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(linked)=clean.iter().position(|token|token.is_word("linked"))else{return Ok(None)};
    let Some(under)=clean.windows(2).rposition(|pair|pair[0].is_word("under")&&pair[1].is_word("the"))else{return Ok(None)};
    if linked==0||under<=linked||under+3>=clean.len()||!clean.last().is_some_and(|token|token.is_word("relationship")){return Ok(None);}
    let name=&clean[under+2..clean.len()-1];
    if name.iter().any(|token|token.kind!=crate::lexer::TokenKind::Word){return Ok(None);}
    let relationship=crate::lexer::parser_token_word_refs(name).join(" ");
    if relationship.is_empty()||relationship.len()>64{return Ok(None);}
    if clean.get(linked+1).is_some_and(|token|token.is_word("to")){
        if under<=linked+2{return Ok(None);}
        let anchor=host(&clean[linked+2..under])?;
        let mut filter=parse_object_filter_lexed(&clean[..linked],other)?;
        filter.zone=Some(Zone::Exile);
        filter.related_exile_anchor=Some(RelatedExileAnchor{relationship,host:Box::new(anchor)});
        crate::word_claims::claim_tokens(&clean[linked..linked+2]);crate::word_claims::claim_tokens(&clean[under..]);
        return Ok(Some(filter));
    }
    // The collection belongs to each candidate host, not to this ability's source.
    if under!=linked+1{return Ok(None);}
    let Some(with)=clean[..linked].iter().rposition(|token|token.is_word("with"))else{return Ok(None)};
    if with==0||with+1>=linked{return Ok(None);}
    let mut member_start=with+1;
    let words=crate::lexer::parser_token_word_refs(&clean[member_start..linked]);
    let (minimum,qualifier_words)=if let [number,"or","more",..]=words.as_slice(){
        let Some(number)=crate::util::parse_number_word_u32(number)else{return Ok(None)};(number,3)
    }else if let ["at","least",number,..]=words.as_slice(){
        let Some(number)=crate::util::parse_number_word_u32(number)else{return Ok(None)};(number,3)
    }else{(1,0)};
    if qualifier_words>0{
        let view=crate::lexer::TokenWordView::new(&clean[member_start..linked]);
        let Some(range)=view.token_span_for_words(0,qualifier_words)else{return Ok(None)};
        crate::word_claims::claim_tokens(&clean[member_start..member_start+range.end]);member_start+=range.end;
    }
    let member=parse_object_filter_lexed(&clean[member_start..linked],false)?;
    let mut filter=parse_object_filter_lexed(&clean[..with],other)?;
    filter.related_exile_members=Some(RelatedExileMembers{relationship,filter:Box::new(member),count:ironsmith_core::filter_model::Comparison::GreaterThanOrEqual(minimum.min(i32::MAX as u32)as i32)});
    crate::word_claims::claim_tokens(&clean[with..with+1]);crate::word_claims::claim_tokens(&clean[linked..]);
    Ok(Some(filter))
}
