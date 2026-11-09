//! Capture a declared action operand once, without moving or exposing it.
//! The ordinary tag/target/choice effects own execution and announcement.
use super::*;
use crate::cards::builders::{EffectAst,ObjectChoiceEffectAst,PlayerAst,TargetAst};
use crate::lexer::{OwnedLexToken,TokenKind};
use crate::model::ast::{SubjectVerbActionAst,SubjectVerbRoleAst};
use crate::target::{ObjectFilter,PlayerFilter};
use crate::zone::Zone;

pub(super) fn capture(tokens:&[OwnedLexToken],tag:crate::tag::TagRef)->Result<Vec<EffectAst>,crate::CardTextError>{
    // A set union retains both exact input packets. It does not re-query an
    // earlier selected set after a later selector changes the current `it`.
    for (index,token) in tokens.iter().enumerate(){
        if !token.is_word("and")||index==0||index+1>=tokens.len(){continue;}
        let tail=crate::lexer::parser_token_word_refs(&tokens[index+1..]);
        if !tail.starts_with(&["the","top"])&&!tail.starts_with(&["the","bottom"]){continue;}
        let left=crate::util::helper_tag_for_tokens(&tokens[..index],"action_inputs_left");
        let right=crate::util::helper_tag_for_tokens(&tokens[index+1..],"action_inputs_right");
        let mut effects=capture(&tokens[..index],left.clone())?;
        effects.extend(capture(&tokens[index+1..],right.clone())?);
        effects.push(EffectAst::subject_verb_tagged_object_union(ObjectFilter::default(),Vec::new(),tag,vec![left,right]));
        crate::word_claims::claim_tokens(&tokens[index..index+1]);
        return Ok(effects);
    }
    let shape=crate::grammar::effects::parse_exile_top_library_shape(tokens,PlayerAst::Implicit)
        .map(|shape|(true,shape)).or_else(||crate::grammar::effects::parse_exile_bottom_library_shape(tokens,PlayerAst::Implicit).map(|shape|(false,shape)));
    if let Some((from_top,shape))=shape {
        let mut filter=ObjectFilter::default().in_zone(Zone::Library);
        filter.library_boundary=Some(ironsmith_core::filter_model::LibraryBoundary{from_top,count:Box::new(shape.count)});
        let player=match shape.player {
            // The library's owner is the filter's; a player-subject wrapper
            // would scope the capture's result tag away from its readers.
            crate::grammar::effects::ExileLibraryPlayerShape::Player(player)=>{filter.owner=Some(PlayerFilter::IteratedPlayer);if matches!(player,PlayerAst::That){PlayerAst::Implicit}else{player}},
            crate::grammar::effects::ExileLibraryPlayerShape::EachPlayer=>{filter.owner=Some(PlayerFilter::Any);PlayerAst::Implicit},
            crate::grammar::effects::ExileLibraryPlayerShape::EachOpponent=>{filter.owner=Some(PlayerFilter::Opponent);PlayerAst::Implicit},
            crate::grammar::effects::ExileLibraryPlayerShape::EachOfThosePlayers=>return Err(crate::CardTextError::UnsupportedLine("proposed library set needs its retained player collection".into())),
        };
        crate::word_claims::claim_tokens(tokens);
        return Ok(vec![EffectAst::subject_verb(SubjectVerbRoleAst::Actor,player,SubjectVerbActionAst::TagMatchingObjects{
            filter,zones:vec![Zone::Library],tag,source_tags:Vec::new(),filter_source_snapshots:false,
        })]);
    }
    if tokens.first().is_some_and(|token|token.is_word("each")||token.is_word("all")){
        let mut filter=crate::object_filters::parse_object_filter_lexed(&tokens[1..],false)?;
        let zone=filter.zone.unwrap_or(Zone::Battlefield);filter.zone=Some(zone);
        crate::word_claims::claim_tokens(&tokens[..1]);
        return Ok(vec![EffectAst::subject_verb_tag_matching_objects(filter,vec![zone],tag)]);
    }
    let words=crate::lexer::parser_token_word_refs(tokens);
    let has_target=words.iter().any(|word|*word=="target");
    if !has_target && tokens.first().is_some_and(|token|token.is_word("up")) {
        if let Some((count,used))=crate::util::parse_choice_count_token_prefix_consumed(tokens){
            let filter=crate::object_filters::parse_object_filter_lexed(&tokens[used..],false)?;
            crate::word_claims::claim_tokens(&tokens[..used]);
            return Ok(vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects{filter,count,count_value:None,player:PlayerAst::You,tag})]);
        }
    }
    let target=crate::util::parse_target_phrase(tokens)?;
    Ok(vec![EffectAst::TagAffected{tag,effect:Box::new(EffectAst::subject_verb_explicit_target_only(target))}])
}
