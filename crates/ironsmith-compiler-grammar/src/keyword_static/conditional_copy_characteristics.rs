//! Independently owned copy exceptions with an optional entering-object filter.
use super::*;
pub(super) fn read(tokens:&[OwnedLexToken])->Result<Option<ironsmith_core::EnterAsCopyFollowup>,CardTextError> {
    let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
    let view=crate::lexer::TokenWordView::new(tokens);let words=view.word_refs();
    let Some(body)=words.strip_prefix(&["it","has"]) else{return Ok(None)};
    let (remove_mana_cost,offset)=if body.starts_with(&["no","mana","cost"]) {(true,5)} else {(false,2)};
    let conditional=words.iter().position(|word|*word=="if");
    let end=conditional.unwrap_or(words.len());
    let mut cursor=offset;
    if remove_mana_cost && words.get(cursor)==Some(&"and") {cursor+=1;}
    if words.get(cursor)==Some(&"it") {cursor+=1;}
    if words.get(cursor)==Some(&"has") {cursor+=1;}
    let pt=if cursor<end && words.get(cursor..cursor+4)==Some(&["base","power","and","toughness"]) {
        let start=view.token_index_after_words(cursor+4).ok_or_else(||CardTextError::ParseError("copy base power/toughness is missing".into()))?;
        let finish=conditional.map(|index|view.token_start_indices()[index]).unwrap_or(tokens.len());
        let text=render_token_slice(&tokens[start..finish]).replace(' ',"");
        let Some((power,toughness))=text.split_once('/') else{return Ok(None)};
        let (Ok(power),Ok(toughness))=(power.parse::<i32>(),toughness.parse::<i32>()) else{return Ok(None)};
        Some((power,toughness))
    } else if cursor==end {None} else {return Ok(None)};
    if !remove_mana_cost && pt.is_none() {return Ok(None)};
    if remove_mana_cost && end > offset && pt.is_none() {return Ok(None)};
    let mut descriptor_start=None;
    let entering_filter=if let Some(index)=conditional {
        let tail=&words[index..];
        let prefix=if tail.starts_with(&["if","its"]) {2} else if tail.starts_with(&["if","it","is"]) {3} else {return Ok(None)};
        let Some(start)=view.token_index_after_words(index+prefix) else{return Ok(None)};
        descriptor_start=Some(start);
        // This filter is the complete condition of the exception, not a
        // best-effort prefix that can silently swallow a sibling action.
        let (parsed, claims) = crate::word_claims::scoped(|| parse_object_filter(&tokens[start..],false));
        let parsed = parsed?;
        if claims.leaves_unread(&tokens[start..]) { return Ok(None); }
        crate::word_claims::adopt(&claims);
        parsed
    } else {ObjectFilter::default()};
    crate::word_claims::claim_tokens(&tokens[..descriptor_start.unwrap_or(tokens.len())]);
    Ok(Some(ironsmith_core::EnterAsCopyFollowup::ConditionalCharacteristics {entering_filter,remove_mana_cost,set_base_power_toughness:pt}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_mana_cost_exception_does_not_erase_dangling_conjunctions() {
        for text in ["it has no mana cost and if it's a token", "it has no mana cost it has if it's a token", "it has no mana cost and"] {
            let tokens=crate::lexer::lex_line(text,0).unwrap();
            assert!(read(&tokens).unwrap().is_none(),"{text}");
        }
        let tokens=crate::lexer::lex_line("it has no mana cost if it's a token",0).unwrap();
        assert!(read(&tokens).unwrap().is_some());
    }
    #[test]
    fn complete_exception_keeps_the_entering_token_condition_and_both_changes() {
        let tokens=crate::lexer::lex_line("it has no mana cost and base power and toughness 1/1 if it's a token",0).unwrap();
        let Some(ironsmith_core::EnterAsCopyFollowup::ConditionalCharacteristics{entering_filter,remove_mana_cost,set_base_power_toughness})=read(&tokens).unwrap() else{panic!("typed exception")};
        assert!(entering_filter.token);assert!(remove_mana_cost);assert_eq!(set_base_power_toughness,Some((1,1)));
        let tokens=crate::lexer::lex_line("it has no mana cost and base power and toughness 1/1 if it's a token and you draw a card",0).unwrap();
        assert!(!matches!(read(&tokens),Ok(Some(_))),"unread sibling cannot disappear into the exception");
    }
}
