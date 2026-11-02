//! Existing face-state actions with ordinary target/all/count selectors.
use crate::cards::builders::{CardTextError,EffectAst,ObjectChoiceEffectAst,PermanentStateActionAst,PlayerAst,SubjectVerbActionAst,SubjectVerbRoleAst,TargetAst};
use crate::lexer::OwnedLexToken;
pub(crate) fn parse(tokens:&[OwnedLexToken])->Result<Option<Vec<EffectAst>>,CardTextError>{
    let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
    if tokens.len()<4 || !tokens[0].is_word("turn") || !tokens[tokens.len()-2].is_word("face") {return Ok(None);}
    let up=if tokens[tokens.len()-1].is_word("up"){true}else if tokens[tokens.len()-1].is_word("down"){false}else{return Ok(None);};
    let operand=&tokens[1..tokens.len()-2];
    let reference=matches!(crate::lexer::parser_token_word_refs(operand).as_slice(),["it"]|["them"])
        || operand.first().is_some_and(|token|token.is_any_word(&["this","that","those","all","each"]));
    let targeted=operand.iter().any(|token|token.is_word("target"));
    let counted=operand.first().and_then(OwnedLexToken::as_word).is_some_and(|word|
        matches!(word,"a"|"an"|"any"|"up") || crate::util::parse_number_word_i32(word).is_some() || word.parse::<usize>().is_ok());
    if !reference && !targeted && !counted{return Ok(None);}
    let mut effects=Vec::new();
    let target=if reference||targeted{crate::util::parse_target_phrase(operand)?}else{
        let mut choose=vec![OwnedLexToken::synthetic_word("choose")];choose.extend_from_slice(operand);
        effects=super::parse_effect_sentence_lexed(&choose)?;
        let [EffectAst::ObjectChoices(choice)]=effects.as_mut_slice()else{return Ok(None);};
        let filter=match choice{
            ObjectChoiceEffectAst::ChooseObjects{filter,..}|ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint{filter,..}=>filter,
            _=>return Ok(None),
        };
        if filter.zone.is_none() && filter.any_of.is_empty(){filter.zone=Some(crate::zone::Zone::Battlefield);}
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::ChosenObjects.bind(),crate::util::span_from_tokens(operand))
    };
    crate::word_claims::claim_tokens(&tokens[..1]);crate::word_claims::claim_tokens(&tokens[tokens.len()-2..]);
    let action=if up{PermanentStateActionAst::TurnFaceUp{target}}else{PermanentStateActionAst::TurnFaceDown{target}};
    effects.push(EffectAst::subject_verb(SubjectVerbRoleAst::Actor,PlayerAst::You,SubjectVerbActionAst::PermanentState(action)));
    Ok(Some(effects))
}
