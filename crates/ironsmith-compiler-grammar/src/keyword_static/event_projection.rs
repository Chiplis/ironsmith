//! Causative event projection reads source/action, subject set and projected action independently.
use super::*;
pub fn parse_trigger_event_projection_line(tokens:&[OwnedLexToken])->Result<Option<StaticAbility>,CardTextError>{
 let clean=trim_edge_punctuation_tokens(tokens);
 let Some(action)=clean.iter().position(|t|t.is_word("attacking")) else{return Ok(None)};
 if !is_source_reference_words(&parser_token_word_refs(&clean[..action])){return Ok(None)}
 let Some((_,subjects))=crate::grammar::primitives::parse_prefix(&clean[action+1..],crate::grammar::primitives::phrase(&["causes","abilities","of"])) else{return Ok(None)};
 let Some(split)=subjects.iter().position(|t|t.is_word("to")) else{return Ok(None)};
 let tail=&subjects[split..];let words=parser_token_word_refs(tail);
 if words!=["to","trigger","as","though","they","attacked"] && words!=["to","trigger","as","though","it","attacked"]{return Ok(None)}
 let filter=parse_object_filter_lexed(&subjects[..split],false)?;
 crate::word_claims::claim_tokens(&clean[..action+1]);crate::word_claims::claim_tokens(tail);
 use ironsmith_core::static_ability_model::TriggerProjectionEvent;
 let mut ability=StaticAbility::trigger_event_projection(TriggerProjectionEvent::Attacks,ObjectFilter::source(),filter,TriggerProjectionEvent::Attacks);ability.label=render_token_slice(clean);
 Ok(Some(ability))
}


pub fn parse_phase_boundary_player_action_line(tokens: &[OwnedLexToken]) -> Result<Option<StaticAbilityAst>, CardTextError> {
    use ironsmith_core::static_ability_model::{PlayerActionBoundary, StaticAbilityPayload};
    let clean = trim_edge_punctuation_tokens(tokens);
    let words = parser_token_word_refs(clean);
    let (boundary, prefix_len) = if words.starts_with(&["as", "your", "end", "step", "begins"]) {
        (PlayerActionBoundary::EndStep, 5)
    } else if words.starts_with(&["as", "your", "combat", "begins"]) {
        (PlayerActionBoundary::BeginCombat, 4)
    } else { return Ok(None); };
    let suffix = ["action", "once", "rather", "than", "once", "for", "each", "permanent", "with", "this", "ability"];
    if !words.ends_with(&suffix) { return Ok(None); }
    let commas: Vec<_> = clean.iter().enumerate().filter_map(|(i,t)| matches!(t.kind, TokenKind::Comma).then_some(i)).collect();
    if commas.len() != 2 { return Ok(None); }
    let first = commas[0]; let second = commas[1];
    if parser_token_word_refs(&clean[..first]).len() != prefix_len { return Ok(None); }
    let condition_tokens = &clean[first+1..second];
    let Some((_, condition_tokens)) = crate::grammar::primitives::parse_prefix(condition_tokens, crate::grammar::primitives::phrase(&["if"])) else { return Ok(None); };
    let condition = crate::grammar::filters::parse_condition_predicate_lexed(condition_tokens)?;
    let body = &clean[second+1..];
    let body_words = parser_token_word_refs(body);
    if !body_words.starts_with(&["you", "perform", "the"]) || body_words.len() <= 3 + suffix.len() { return Ok(None); }
    let label = body_words[3..body_words.len()-suffix.len()].join(" ");
    crate::word_claims::claim_tokens(&clean[..first+1]);
    crate::word_claims::claim_tokens(&clean[second..]);
    crate::word_claims::claim_tokens(&clean[first+1..first+2]);
    Ok(Some(StaticAbilityAst::Static(crate::model::CompilerStaticAbilityCore { id: None, label: render_token_slice(clean), payload: StaticAbilityPayload::PhaseBoundaryPlayerAction { boundary, label, condition } })))
}
