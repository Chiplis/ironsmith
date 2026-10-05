//! One complete temporary/targeted template instruction, including ability loss.
use crate::lexer::{OwnedLexToken,TokenWordView};
use crate::cards::builders::CardTextError;
use crate::effect::Until;
#[derive(Debug,Clone)]
pub struct AbilityLossTemplate {
    pub subject:Vec<OwnedLexToken>,
    pub template:Vec<OwnedLexToken>,
    pub duration:Until,
}
pub fn parse(tokens:&[OwnedLexToken])->Result<Option<AbilityLossTemplate>,CardTextError> {
    if tokens.iter().any(OwnedLexToken::is_quote) { return Ok(None); }
    let (duration,body)=super::parse_search_restriction_duration_shape_lexed(tokens)?
        .map(|shape|(shape.duration,shape.remainder)).unwrap_or_else(||(Until::Forever,tokens.to_vec()));
    let view=TokenWordView::new(&body);let words=view.word_refs();
    let Some(start)=words.windows(5).position(|w|matches!(w[0],"lose"|"loses") && w[1..4]==["all","abilities","and"] && matches!(w[4],"become"|"becomes")) else { return Ok(None); };
    if start==0 { return Ok(None); }
    if matches!(duration,Until::Forever) && !words[..start].contains(&"target") { return Ok(None); }
    let Some(subject_end)=view.token_index_after_words(start) else {return Ok(None)};
    let Some(template_start)=view.token_index_after_words(start+5) else {return Ok(None)};
    let subject=crate::util::trim_edge_punctuation_tokens(&body[..subject_end]).to_vec();
    let template=crate::util::trim_edge_punctuation_tokens(&body[template_start..]).to_vec();
    let Some(shape)=super::become_shapes::parse_object_template_tokens(&template) else {return Ok(None)};
    if shape.base_power_toughness.is_none() || !shape.ability_tokens.is_empty() {return Ok(None);}
    Ok(Some(AbilityLossTemplate{subject,template,duration}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_loss_and_template_is_one_typed_instruction() {
        let lex=|text|crate::lexer::lex_line(text,0).unwrap();
        assert!(parse(&lex("Until end of turn, target creature loses all abilities and becomes a blue Frog with base power and toughness 1/1.")).unwrap().is_some());
        assert!(parse(&lex("Target creature loses all abilities and becomes a green Elephant with base power and toughness 3/3 until end of turn.")).unwrap().is_some());
        for text in ["Until end of turn, target creature loses all abilities and becomes a blue Frog with base power and toughness 1/1 and draw a card.","Until end of turn, target creature loses all abilities and becomes your choice of a Frog or an Octopus.","Until end of turn, target creature loses some abilities and becomes a blue Frog with base power and toughness 1/1."] {
            assert!(parse(&lex(text)).unwrap().is_none(),"{text}");
        }
    }
}
