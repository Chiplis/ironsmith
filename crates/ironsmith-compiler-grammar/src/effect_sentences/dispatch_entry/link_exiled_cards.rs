//! A named relationship operation, separate from choosing/exiling objects.
use super::*;
pub(crate) fn read(tokens:&[OwnedLexToken])->Result<Option<EffectAst>,CardTextError>{
    let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
    if !tokens.first().is_some_and(|token|token.is_word("link")){return Ok(None);}
    let Some(to)=tokens.iter().position(|token|token.is_word("to"))else{return Ok(None)};
    let Some(under)=tokens.windows(2).position(|pair|pair[0].is_word("under")&&pair[1].is_word("the"))else{return Ok(None)};
    if to<=1||under<=to+1||under+3>=tokens.len()||!tokens.last().is_some_and(|token|token.is_word("relationship")){return Ok(None);}
    let name_tokens=&tokens[under+2..tokens.len()-1];
    if name_tokens.iter().any(|token|token.kind!=TokenKind::Word){return Ok(None);}
    let relationship=crate::lexer::parser_token_word_refs(name_tokens).join(" ");
    if relationship.is_empty()||relationship.len()>64{return Ok(None);}
    let card_words=crate::lexer::parser_token_word_refs(&tokens[1..to]);
    let cards=if matches!(card_words.as_slice(),["the","iterated","card"|"object"]){
        crate::word_claims::claim_tokens(&tokens[1..to]);None
    }else{Some(parse_target_phrase(&tokens[1..to])?)};
    let host=parse_target_phrase(&tokens[to+1..under])?;
    crate::word_claims::claim_tokens(&tokens[..1]);
    crate::word_claims::claim_tokens(&tokens[to..to+1]);
    crate::word_claims::claim_tokens(&tokens[under..]);
    Ok(Some(EffectAst::LinkExiledCards{cards,host,relationship}))
}
