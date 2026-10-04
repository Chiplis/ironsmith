use crate::cards::builders::{CardTextError,EffectAst,SubjectVerbActionAst,CharacteristicActionAst};
use crate::lexer::OwnedLexToken;
pub(super) fn parse(tokens:&[OwnedLexToken])->Result<Option<EffectAst>,CardTextError> {
    let Some(shape)=crate::grammar::effects::ability_loss_templates::parse(tokens)? else {return Ok(None)};
    let mut effect=super::clause_dispatch::parse_become_clause(&shape.subject,&shape.template)?;
    let EffectAst::SubjectVerb(subject)=&mut effect else {return Err(CardTextError::ParseError("ability-loss template requires one complete characteristic instruction".into()))};
    let SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature{remove_other_abilities,duration,..})=&mut subject.action else {
        return Err(CardTextError::ParseError("unsupported ability-loss template characteristics".into()));
    };
    *remove_other_abilities=true;*duration=shape.duration;
    Ok(Some(effect))
}
