//! A card-copy permission retains the ordinary price, condition and entry riders.
use super::*;
use crate::zone::Zone;

pub(super) fn read(tokens: &[OwnedLexToken]) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let Some(first) = sentences.first() else { return Ok(None); };
    let view = TokenWordView::new(first);
    let words = view.word_refs();
    let Some(head) = permission_shapes::find_words(&words, &["you", "may", "cast", "a", "copy", "of", "this", "card"]) else { return Ok(None); };
    if head != 0 && !words.starts_with(&["as", "long", "as"]) { return Ok(None); }
    let transformed = words.get(head + 8) == Some(&"transformed");
    let zone_head = head + 8 + usize::from(transformed);
    if words.get(zone_head..zone_head + 4) != Some(&["from", "your", "graveyard", "by"][..]) { return Ok(None); }
    let Some(rather) = permission_shapes::find_words(&words, &["rather", "than"]) else { return Ok(None); };
    let donor_phrase = ["and", "exiling", "this", "card", "from", "your", "graveyard"];
    let Some(donor) = permission_shapes::find_words(&words, &donor_phrase) else { return Ok(None); };
    if donor + donor_phrase.len() != rather { return Ok(None); }
    let starts = view.token_start_indices();
    let copy_start = starts[head + 3];
    let copy_end = starts[head + 6];
    let donor_start = starts[donor];
    let donor_end = starts[rather];
    let transformed_index = transformed.then(|| starts[head + 8]);
    let ordinary = first.iter().enumerate().filter_map(|(index, token)| {
        (!(copy_start..copy_end).contains(&index)
            && !(donor_start..donor_end).contains(&index)
            && transformed_index != Some(index)).then_some(token.clone())
    }).collect::<Vec<_>>();
    // The full intrinsic reader owns conditional permissions and all ordinary
    // payment forms. The removed copy frame cannot match this reader again.
    let Some(mut method) = super::parse_self_zone_alternative_cost(&ordinary)? else { return Ok(None); };
    if !matches!(method, AlternativeCastingMethod::FromZone { .. }) { return Ok(None); }
    let mut named_cost = false;
    for sentence in &sentences[1..] {
        let label_words = TokenWordView::new(sentence).word_refs();
        if let Some(label) = label_words.strip_prefix(&["this", "alternative", "cost", "is", "named"])
            && !label.is_empty()
        {
            if named_cost || label.len() > 8
                || sentence.iter().any(|token| !matches!(token.kind,
                    crate::lexer::TokenKind::Word | crate::lexer::TokenKind::Period))
            { return Ok(None); }
            let AlternativeCastingMethod::FromZone { name, .. } = &mut method else { unreachable!() };
            *name = label.join(" ").into();
            named_cost = true;
            crate::word_claims::claim_tokens(sentence);
            continue;
        }
        let AlternativeCastingMethod::FromZone { instructions, .. } = &mut method else { unreachable!() };
        if read_conditional_price_reduction(sentence, instructions)? { continue; }
        if super::read_price_entry_instruction(sentence, instructions)
            || super::read_price_reduction(sentence, instructions)? { continue; }
        let rider_words = TokenWordView::new(sentence).word_refs();
        if rider_words.starts_with(&["if", "you", "do", "it", "enters", "with"]) {
            let mut with_counter = ordinary.clone();
            with_counter.extend_from_slice(sentence);
            let Some(counter_method) = super::parse_self_zone_alternative_cost(&with_counter)? else { return Ok(None); };
            if counter_method.entry_counters().is_empty() { return Ok(None); }
            let mut counters = method.entry_counters().to_vec();
            counters.extend_from_slice(counter_method.entry_counters());
            method = method.with_entry_counters(counters);
            continue;
        }
        return Ok(None);
    }
    let AlternativeCastingMethod::FromZone { instructions, .. } = &mut method else { unreachable!() };
    instructions.cast_copy = true;
    instructions.source_destination_as_cost = Some(Zone::Exile);
    instructions.transformed |= transformed;
    crate::word_claims::claim_tokens(&first[copy_start..copy_end]);
    crate::word_claims::claim_tokens(&first[donor_start..donor_end]);
    if let Some(index) = transformed_index { crate::word_claims::claim_tokens(&first[index..=index]); }
    Ok(Some(method))
}

// The reducer owns one complete typed clause. Conditions and colors remain
// data on this permission, never global spell modifiers from another price.
fn read_conditional_price_reduction(tokens: &[OwnedLexToken], instructions: &mut ironsmith_core::ZoneCastInstructions) -> Result<bool, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if !words.starts_with(&["this", "cost", "is", "reduced", "by"]) { return Ok(false); }
    let Some(if_index) = words.iter().enumerate().skip(5).find_map(|(index, word)| (*word == "if").then_some(index)) else { return Ok(false); };
    let starts = view.token_start_indices();
    let amount = crate::grammar::values::parse_mana_cost_tokens(&tokens[starts[4] + 1..starts[if_index]])?;
    let condition_tokens = &tokens[starts[if_index] + 1..];
    let condition_words = TokenWordView::new(condition_tokens).word_refs();
    let condition = if matches!(condition_words.as_slice(),
        ["this", "card", "was", "put", "into", "your", "graveyard", "from", "the", "battlefield", "this", "turn"])
    {
        let filter = ironsmith_core::ObjectFilter {
            source: true,
            zone: Some(Zone::Graveyard),
            entered_graveyard_from_battlefield_this_turn: true,
            ..Default::default()
        };
        crate::word_claims::claim_tokens(condition_tokens);
        Some(ThisSpellCostCondition::ConditionExpr {
            condition: ironsmith_core::Condition::SourceMatches(filter),
            display: render_token_slice(condition_tokens),
        })
    } else { parse_this_spell_cost_condition(condition_tokens) };
    let Some(condition) = condition else { return Ok(false); };
    instructions.conditional_mana_reductions.push((amount, condition));
    crate::word_claims::claim_tokens(&tokens[..starts[if_index] + 1]);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conditional_copy_price_retains_condition_and_independent_donor() {
        for price in ["paying {2}{U}", "paying {1}{G} and sacrificing a nonland permanent"] {
            let text = format!("As long as you have seven or more cards in your graveyard, you may cast a copy of this card from your graveyard by {price} and exiling this card from your graveyard rather than paying its mana cost.");
            let method = read(&lex_line(&text, 0).unwrap()).unwrap().unwrap();
            let AlternativeCastingMethod::FromZone { zone, condition, total_cost, instructions, .. } = method else { panic!() };
            assert_eq!(zone, Zone::Graveyard);
            assert!(condition.is_some());
            assert!(instructions.cast_copy);
            assert_eq!(instructions.source_destination_as_cost, Some(Zone::Exile));
            assert_eq!(total_cost.costs().len(), if price.contains("sacrificing") { 2 } else { 1 });
            assert!(!matches!(read(&lex_line(&(text + " Draw a card."), 0).unwrap()), Ok(Some(_))));
        }
    }
    #[test]
    fn copied_card_can_be_cast_transformed_with_selected_entry_instructions() {
        let text = "As long as you have seven or more cards in your graveyard, you may cast a copy of this card transformed from your graveyard by paying {U} and exiling this card from your graveyard rather than paying its mana cost. If you cast this spell this way, it enters tapped.";
        let method = read(&lex_line(text, 0).unwrap()).unwrap().unwrap();
        assert!(method.casts_transformed());
        assert!(method.battlefield_instructions().enters_tapped);
        assert!(!method.battlefield_instructions().enters_transformed);
        assert!(method.cast_condition().is_some());
    }
    #[test]
    fn granted_copy_permission_has_a_stable_explicit_price_label() {
        let text = "As long as you have seven or more cards in your graveyard, you may cast a copy of this card from your graveyard by paying this card's mana cost and exiling this card from your graveyard rather than paying its mana cost. This alternative cost is named Echohaul.";
        let method = read(&lex_line(text, 0).unwrap()).unwrap().unwrap();
        assert_eq!(method.name(), "echohaul");
        assert!(method.cast_condition().is_some());
        assert!(method.battlefield_instructions().cast_copy);
    }
    #[test]
    fn copy_entry_counters_belong_to_the_selected_permission() {
        let text = "As long as you have seven or more cards in your graveyard, you may cast a copy of this card from your graveyard by paying {3}{G}{G} and exiling this card from your graveyard rather than paying its mana cost. If you do, it enters with a +1/+1 counter on it.";
        let method = read(&lex_line(text, 0).unwrap()).unwrap().unwrap();
        assert_eq!(method.entry_counters(), &[(ironsmith_core::CounterType::PlusOnePlusOne, 1)]);
        assert!(method.cast_condition().is_some());
    }
}
