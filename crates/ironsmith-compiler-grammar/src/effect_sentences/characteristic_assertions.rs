use crate::cards::builders::{CardTextError,EffectAst};
use crate::grammar::effects::characteristic_assertions::{self as shape,Property};
use crate::lexer::OwnedLexToken;
use crate::effect::Until;
pub(super) fn parse(tokens:&[OwnedLexToken])->Result<Option<EffectAst>,CardTextError> {
    let (duration,tokens)=super::parse_restriction_duration(tokens)?.unwrap_or_else(||(Until::Forever,tokens.to_vec()));
    let Some(shape)=shape::parse(&tokens) else{return Ok(None)};
    if !shape.remove {return Ok(None);}
    let Ok(target)=crate::util::parse_target_phrase(shape.subject) else { return Ok(None); };
    Ok(Some(match shape.property {
        Property::Supertype(kind)=>EffectAst::subject_verb_remove_supertypes(target,vec![kind],duration),
        Property::CardType(kind)=>EffectAst::subject_verb_remove_card_types(target,vec![kind],duration),
    }))
}
