use super::*;

pub fn parse_looked_card_reveal_filter_shape(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let (filter_tokens, same_name) = split_same_name_suffix(tokens);
    let all_words = parser_token_word_refs(filter_tokens);
    let words = non_article_word_refs(&all_words);

    if CHOSEN_CARD_PHRASES
        .iter()
        .any(|expected| permission_shapes::exact_words(&words, expected))
    {
        return Some(apply_same_name(ObjectFilter::default(), true));
    }
    if words.len() == 1 && is_card_word(words[0]) {
        return Some(apply_same_name(ObjectFilter::default(), same_name));
    }
    if words.len() == 4
        && is_card_word(words[0])
        && (permission_shapes::exact_words(&words[1..], &["of", "chosen", "type"])
            || permission_shapes::exact_words(&words[1..], &["of", "that", "type"]))
    {
        let mut filter = ObjectFilter::default();
        filter.chosen_creature_type = true;
        return Some(apply_same_name(filter, same_name));
    }
    if permission_shapes::exact_words(&words, &["permanent", "card"])
        || permission_shapes::exact_words(&words, &["permanent", "cards"])
    {
        return Some(apply_same_name(ObjectFilter::permanent_card(), same_name));
    }
    if permission_shapes::exact_words(&words, &["historic", "card"])
        || permission_shapes::exact_words(&words, &["historic", "cards"])
    {
        let mut filter = ObjectFilter::default();
        filter.historic = true;
        return Some(apply_same_name(filter, same_name));
    }
    if permission_shapes::exact_words(&words, &["nonland", "permanent", "card"])
        || permission_shapes::exact_words(&words, &["nonland", "permanent", "cards"])
    {
        let mut filter = ObjectFilter::permanent_card();
        filter.excluded_card_types.push(CardType::Land);
        return Some(apply_same_name(filter, same_name));
    }
    if let Some(filter) = parse_noncreature_nonland_permanent(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    if let Some(filter) = parse_conjunctive_negated_card_filter(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    if let Some(filter) = parse_land_or_legendary_permanent(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    if let Some(filter) = parse_modified_permanent_cards(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    if let Some(filter) = parse_filter_disjunction(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }

    let filter = parse_generic_disjunction_filter(filter_tokens)
        .or_else(|| parse_object_filter_lexed(filter_tokens, false).ok())?;
    Some(apply_same_name(filter, same_name))
}

pub fn strip_up_to_one_looked_card_choice_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let tokens = trim_lexed_commas(tokens);
    let Some((count, used)) = parse_choice_count_token_prefix_consumed(tokens) else {
        return tokens.to_vec();
    };
    if count == crate::effect::ChoiceCount::up_to(1) {
        trim_lexed_commas(tokens.get(used..).unwrap_or_default()).to_vec()
    } else {
        tokens.to_vec()
    }
}
