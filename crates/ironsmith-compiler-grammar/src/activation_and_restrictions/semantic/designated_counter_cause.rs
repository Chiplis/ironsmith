//! Counter placement by an immutable authored ability designation.
use super::*;
pub(super) fn read(tokens: &[OwnedLexToken]) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    if let Some(at) = words.windows(5).position(|part| part == ["caused", "by", "an", "ability", "designated"]) {
        let label_start = view.token_start_indices().get(at + 5).copied().unwrap_or(tokens.len());
        let quote = label_start.checked_sub(1).filter(|index| tokens[*index].kind == crate::lexer::TokenKind::Quote);
        if let Some(quote) = quote
            && tokens.last().is_some_and(|token| token.kind == crate::lexer::TokenKind::Quote)
            && label_start < tokens.len() - 1
            && tokens[label_start..tokens.len()-1].iter().all(|token| token.kind == crate::lexer::TokenKind::Word)
        {
            let label = crate::lexer::render_token_slice(&tokens[label_start..tokens.len()-1]).to_ascii_lowercase();
            let mut trigger = parse_trigger_clause_lexed_unstacked(&tokens[..view.token_start_indices()[at]])?;
            if let TriggerSpec::CounterPutOn { cause_designation, .. } = &mut trigger {
                *cause_designation = Some(label);
                crate::word_claims::claim_tokens(&tokens[view.token_start_indices()[at]..]);
                let _ = quote;
                return Ok(Some(trigger));
            }
            return Err(CardTextError::ParseError("ability designation cause requires a counter-placement event".into()));
        }
        return Ok(None);
    }
    if !matches!(words.first(), Some(&"a" | &"an")) { return Ok(None); }
    let Some(ability) = words.iter().position(|word| *word == "ability") else { return Ok(None); };
    if ability < 2 || words.get(ability + 1) != Some(&"puts") { return Ok(None); }
    let Some(on) = words.iter().enumerate().skip(ability + 2).find_map(|(index, word)| (*word == "on").then_some(index)) else { return Ok(None); };
    if on <= ability + 2 || on + 1 >= words.len() { return Ok(None); }
    let label_start = view.token_start_indices()[1];
    let label_end = view.token_start_indices()[ability];
    if tokens[label_start..label_end].iter().any(|token| token.kind != crate::lexer::TokenKind::Word) { return Ok(None); }
    let label = words[1..ability].join(" ").to_ascii_lowercase();
    let counter_start = view.token_start_indices()[ability + 2];
    let on_start = view.token_start_indices()[on];
    let mut passive = tokens[counter_start..on_start].to_vec();
    passive.extend(crate::lexer::synthetic_word_tokens([if words[on - 1] == "counters" { "are" } else { "is" }, "put"]));
    passive.extend_from_slice(&tokens[on_start..]);
    let mut trigger = parse_trigger_clause_lexed_unstacked(&passive)?;
    if let TriggerSpec::CounterPutOn { cause_designation, .. } = &mut trigger {
        *cause_designation = Some(label);
        crate::word_claims::claim_tokens(tokens);
        return Ok(Some(trigger));
    }
    Err(CardTextError::ParseError("a named ability must put an explicit counter on the recipient".into()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_multiword_ability_class_retains_counter_and_recipient() {
        let trigger = read(&crate::lexer::lex_line("an echoing source ability puts a +1/+1 counter on a creature you control", 0).unwrap()).unwrap().unwrap();
        let TriggerSpec::CounterPutOn { cause_designation, counter_type, one_or_more, .. } = trigger else { panic!("counter placement"); };
        assert_eq!(cause_designation.as_deref(), Some("echoing source"));
        assert_eq!(counter_type, Some(CounterType::PlusOnePlusOne)); assert!(!one_or_more);
    }
    #[test]
    fn passive_designation_is_structurally_the_same_event() {
        let trigger = read(&crate::lexer::lex_line("a +1/+1 counter is put on a creature you control caused by an ability designated \"echoing source\"", 0).unwrap()).unwrap().unwrap();
        assert!(matches!(trigger, TriggerSpec::CounterPutOn { cause_designation: Some(label), .. } if label == "echoing source"));
    }
    #[test]
    fn an_ordinary_counter_event_is_not_named_by_its_permanent() {
        assert!(read(&crate::lexer::lex_line("a +1/+1 counter is put on a creature with echoing source", 0).unwrap()).unwrap().is_none());
    }
}
