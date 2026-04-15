use super::*;

pub(crate) fn parse_sentence_each_player_return_with_additional_counter(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_return_with_additional_counter_sentence(tokens)
}

pub(crate) fn parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_lexed_slices_on_comma(tokens);
    if segments.len() != 3 {
        return Ok(None);
    }

    let reveal_tokens = trim_commas(&segments[0]);
    let reveal_words = crate::cards::builders::compiler::token_word_refs(&reveal_tokens);
    let reveal_prefix = [
        "each", "player", "reveals", "a", "number", "of", "cards", "from", "the", "top", "of",
        "their", "library", "equal", "to",
    ];
    if !slice_starts_with(&reveal_words, &reveal_prefix) {
        return Ok(None);
    }
    let Some(count_token_idx) = token_index_for_word_index(&reveal_tokens, reveal_prefix.len())
    else {
        return Ok(None);
    };
    let mut synthetic_where_tokens = vec![
        OwnedLexToken::word("where".to_string(), TextSpan::synthetic()),
        OwnedLexToken::word("x".to_string(), TextSpan::synthetic()),
        OwnedLexToken::word("is".to_string(), TextSpan::synthetic()),
    ];
    synthetic_where_tokens.extend(reveal_tokens[count_token_idx..].iter().cloned());
    let Some(count) = parse_where_x_value_clause(&synthetic_where_tokens) else {
        return Ok(None);
    };

    let put_tokens = trim_commas(&segments[1]);
    if grammar::words_match_prefix(&put_tokens, &["puts", "all", "permanent", "cards"]).is_none()
        || grammar::words_find_phrase(&put_tokens, &["revealed", "this", "way"]).is_none()
        || grammar::words_find_phrase(&put_tokens, &["onto", "the", "battlefield"]).is_none()
    {
        return Ok(None);
    }

    let rest_tokens = trim_commas(&segments[2]);
    let rest_words = crate::cards::builders::compiler::token_word_refs(&rest_tokens);
    let rest_words = if rest_words.first().copied() == Some("and") {
        &rest_words[1..]
    } else {
        rest_words.as_slice()
    };
    if rest_words != ["puts", "the", "rest", "into", "their", "graveyard"] {
        return Ok(None);
    }

    let revealed_tag_key = helper_tag_for_tokens(tokens, "revealed");
    let iterated_target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::LookAtTopCards {
                player: PlayerAst::That,
                count,
                tag: revealed_tag_key.clone(),
            },
            EffectAst::RevealTagged {
                tag: revealed_tag_key.clone(),
            },
            EffectAst::ForEachTagged {
                tag: revealed_tag_key,
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::ItMatches(ObjectFilter::permanent_card()),
                    if_true: vec![EffectAst::MoveToZone {
                        target: iterated_target.clone(),
                        zone: Zone::Battlefield,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::Owner,
                        battlefield_tapped: false,
                        attached_to: None,
                    }],
                    if_false: vec![EffectAst::MoveToZone {
                        target: iterated_target,
                        zone: Zone::Graveyard,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::Preserve,
                        battlefield_tapped: false,
                        attached_to: None,
                    }],
                }],
            },
        ],
    }]))
}

pub(crate) fn parse_return_then_do_same_for_subtypes_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(comma_then_idx) = find_comma_then_idx(tokens) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = crate::cards::builders::compiler::token_word_refs(&tail_tokens);
    if grammar::words_match_prefix(&tail_tokens, &["do", "the", "same", "for"]).is_none() {
        return Ok(None);
    }
    let subtype_words = &tail_words[4..];
    if subtype_words.is_empty() {
        return Ok(None);
    }

    let mut extra_subtypes = Vec::new();
    for word in subtype_words {
        if matches!(*word, "and" | "or") {
            continue;
        }
        let Some(subtype) = parse_pluralized_subtype_word(word) else {
            return Ok(None);
        };
        extra_subtypes.push(subtype);
    }
    if extra_subtypes.is_empty() {
        return Ok(None);
    }

    let mut effects = parse_effect_chain(&head_tokens)?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let base_effect = effects[0].clone();
    for subtype in extra_subtypes {
        let Some(cloned) = clone_return_effect_with_subtype(&base_effect, subtype) else {
            return Ok(None);
        };
        effects.push(cloned);
    }

    Ok(Some(effects))
}

fn split_choose_same_followup_filters(filter: &ObjectFilter) -> Vec<ObjectFilter> {
    match filter.mana_value.clone() {
        Some(crate::filter::Comparison::OneOf(values)) if !values.is_empty() => values
            .into_iter()
            .map(|value| {
                let mut cloned = filter.clone();
                cloned.mana_value = Some(crate::filter::Comparison::Equal(value));
                cloned
            })
            .collect(),
        _ => vec![filter.clone()],
    }
}

pub(crate) fn parse_choose_then_do_same_for_filter_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    if !tokens.first().is_some_and(|token| token.is_word("choose")) {
        return Ok(None);
    }
    let Some((head_slice, tail_slice)) = split_lexed_once_on_comma_then(tokens)
        .or_else(|| grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void()))
    else {
        return Ok(None);
    };

    let head_tokens = trim_commas(head_slice);
    let tail_tokens = trim_commas(tail_slice);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    if grammar::words_match_prefix(&tail_tokens, &["do", "the", "same", "for"]).is_none() {
        return Ok(None);
    }

    let followup_filter_tokens = &tail_tokens[4..];
    if followup_filter_tokens.is_empty() {
        return Ok(None);
    }

    let Some((player, base_filter, count)) = parse_you_choose_objects_clause(&head_tokens)?
        .or_else(|| {
            parse_target_player_choose_objects_clause(&head_tokens)
                .ok()
                .flatten()
        })
    else {
        return Ok(None);
    };
    let tag = TagKey::from(IT_TAG);

    let followup_filter = parse_object_filter(&followup_filter_tokens, false)?;
    if followup_filter.controller.is_some() || followup_filter.owner.is_some() {
        return Ok(None);
    }

    let merged_filter = merge_filters(&base_filter, &followup_filter);
    let followup_filters = split_choose_same_followup_filters(&merged_filter);
    if followup_filters.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::ChooseObjects {
        filter: base_filter.clone(),
        count: count.clone(),
        count_value: None,
        player: player.clone(),
        tag: tag.clone(),
    }];
    for filter in followup_filters {
        effects.push(EffectAst::ChooseObjects {
            filter,
            count: count.clone(),
            count_value: None,
            player: player.clone(),
            tag: tag.clone(),
        });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_return_then_do_same_for_subtypes(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_then_do_same_for_subtypes_sentence(tokens)
}

pub(crate) fn parse_sentence_choose_then_do_same_for_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_choose_then_do_same_for_filter_sentence(tokens)
}

pub(crate) fn parse_sacrifice_any_number_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    let (head_tokens, tail_tokens) = if let Some((head, tail)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())
    {
        if head.is_empty() {
            return Ok(None);
        }
        (trim_commas(head), Some(trim_commas(tail)))
    } else {
        (tokens.to_vec(), None)
    };

    if !head_tokens
        .first()
        .is_some_and(|token| token.is_word("sacrifice"))
    {
        return Ok(None);
    }

    let mut idx = 1usize;
    if !(head_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("any"))
        && head_tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number")))
    {
        return Ok(None);
    }
    idx += 2;
    if head_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("of"))
    {
        idx += 1;
    }
    if idx >= head_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let filter_tokens = trim_commas(&head_tokens[idx..]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let filter = parse_object_filter(&filter_tokens, false)?;
    let tag = TagKey::from(IT_TAG);

    let mut effects = vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::SacrificeAll {
            filter: ObjectFilter::tagged(tag),
            player: PlayerAst::Implicit,
        },
    ];
    if let Some(tail_tokens) = tail_tokens
        && !tail_tokens.is_empty()
    {
        let mut tail_effects = parse_effect_chain(&tail_tokens)?;
        effects.append(&mut tail_effects);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_sacrifice_any_number(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_any_number_sentence(tokens)
}

pub(crate) fn parse_sacrifice_one_or_more_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("sacrifice"))
    {
        return Ok(None);
    }

    let mut idx = 1usize;
    let Some((minimum, used)) = parse_number(&tokens[idx..]) else {
        return Ok(None);
    };
    idx += used;
    if !(tokens.get(idx).is_some_and(|token| token.is_word("or"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("more")))
    {
        return Ok(None);
    }
    idx += 2;
    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }
    if idx >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let filter_tokens = trim_commas(&tokens[idx..]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }
    let filter = parse_object_filter(&filter_tokens, false)?;
    let tag = TagKey::from(IT_TAG);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::at_least(minimum as usize),
            count_value: None,
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::SacrificeAll {
            filter: ObjectFilter::tagged(tag),
            player: PlayerAst::Implicit,
        },
    ]))
}

pub(crate) fn parse_sentence_sacrifice_one_or_more(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_one_or_more_sentence(tokens)
}

pub(crate) fn parse_sentence_keyword_then_chain(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    let Some((head_slice, tail_slice)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())
    else {
        return Ok(None);
    };

    let head_tokens = trim_commas(head_slice);
    let Some(head_effect) = parse_keyword_mechanic_clause(&head_tokens)? else {
        return Ok(None);
    };

    let tail_tokens = trim_commas(tail_slice);
    if tail_tokens.is_empty() {
        return Ok(Some(vec![head_effect]));
    }

    let mut effects = vec![head_effect];
    if let Some(mut counter_effects) = parse_sentence_put_counter_sequence(&tail_tokens)? {
        effects.append(&mut counter_effects);
        return Ok(Some(effects));
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    effects.append(&mut tail_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_chain_then_keyword(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = find_comma_then_idx(tokens)
        .map(|idx| (idx, idx + 2))
        .or_else(|| {
            find_token_word(tokens, "then")
                .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
        });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..head_end]);
    let tail_tokens = trim_commas(&tokens[tail_start..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let Some(keyword_effect) = parse_keyword_mechanic_clause(&tail_tokens)? else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    head_effects.push(keyword_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_return_then_create(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    let split = split_lexed_once_on_comma_then(tokens)
        .or_else(|| grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void()));
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(head_slice);
    let tail_tokens = trim_commas(tail_slice);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    if !head_tokens.first().is_some_and(|t| t.is_word("return"))
        || !tail_tokens.first().is_some_and(|t| t.is_word("create"))
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_exile_then_may_put_from_exile(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    let split = split_lexed_once_on_comma_then(tokens)
        .or_else(|| grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void()));
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(head_slice);
    let tail_tokens = trim_commas(tail_slice);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    if !grammar::strip_lexed_prefix_phrase(
        &tail_tokens,
        &["you", "may", "put", "any", "number", "of"],
    )
    .is_some()
        || !grammar::contains_word(&tail_tokens, "from")
        || !grammar::contains_word(&tail_tokens, "exile")
        || !grammar::contains_word(&tail_tokens, "battlefield")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_exile_then_shuffle_graveyard_into_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    let split = split_lexed_once_on_comma_then(tokens)
        .or_else(|| grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void()));
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(head_slice);
    let tail_tokens = trim_commas(tail_slice);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let head_words = crate::cards::builders::compiler::token_word_refs(&head_tokens);
    if !head_words.first().is_some_and(|word| *word == "exile")
        && !(head_words.first().is_some_and(|word| *word == "you")
            && head_words.get(1).is_some_and(|word| *word == "exile"))
    {
        return Ok(None);
    }

    let tail_words = crate::cards::builders::compiler::token_word_refs(&tail_tokens);
    if !tail_words
        .first()
        .is_some_and(|word| *word == "shuffle" || *word == "shuffles")
    {
        return Ok(None);
    }
    if !tail_words
        .iter()
        .any(|word| *word == "graveyard" || *word == "graveyards")
        || !tail_words
            .iter()
            .any(|word| *word == "library" || *word == "libraries")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if !head_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::Exile { .. }
                | EffectAst::ExileAll { .. }
                | EffectAst::ExileUntilSourceLeaves { .. }
        )
    }) {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if !tail_effects
        .iter()
        .any(|effect| matches!(effect, EffectAst::ShuffleGraveyardIntoLibrary { .. }))
    {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_exile_source_with_counters_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    // "exile <source> with <counter descriptor> on it/them"
    let Some(after_exile) = grammar::strip_lexed_prefix_phrase(tokens, &["exile"]) else {
        return Ok(None);
    };
    let Some((source_name_slice, counter_clause_slice)) =
        grammar::split_lexed_once_on_separator(after_exile, || grammar::kw("with").void())
    else {
        return Ok(None);
    };

    let source_name_tokens = trim_commas(source_name_slice);
    if source_name_tokens.is_empty() {
        return Ok(None);
    }
    let source_name_words = crate::cards::builders::compiler::token_word_refs(&source_name_tokens);
    if !is_likely_named_or_source_reference_words(&source_name_words) {
        return Ok(None);
    }

    let counter_clause_tokens = trim_commas(counter_clause_slice);
    let Some(on_idx) = rfind_token_word(&counter_clause_tokens, "on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words =
        crate::cards::builders::compiler::token_word_refs(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() {
        return Ok(None);
    }
    let (count, counter_type) = parse_counter_descriptor(&descriptor_tokens)?;

    let source_target = TargetAst::Source(span_from_tokens(tokens));
    Ok(Some(vec![
        EffectAst::Exile {
            target: source_target.clone(),
            face_down: false,
        },
        EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target: source_target,
            target_count: None,
            distributed: false,
        },
    ]))
}

pub(crate) fn parse_sentence_exile_source_with_counters(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_source_with_counters_sentence(tokens)
}

pub(crate) fn parse_sentence_comma_then_chain_special(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        words
            .iter()
            .filter_map(|word| match *word {
                "s" | "'" | "’" => None,
                _ => Some(strip_quoted_possessive_suffix(word)),
            })
            .filter(|word: &&str| !word.is_empty())
            .collect()
    }

    let Some(comma_then_idx) = find_comma_then_idx(tokens) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let head_word_storage = crate::cards::builders::compiler::token_word_refs(&head_tokens);
    let tail_word_storage = crate::cards::builders::compiler::token_word_refs(&tail_tokens);
    let head_words = normalize_words(&head_word_storage);
    let tail_words = normalize_words(&tail_word_storage);
    let is_that_player_tail = slice_starts_with(&tail_words, &["that", "player"]);
    let is_return_source_tail = slice_starts_with(&tail_words, &["return", "this"])
        && slice_contains(&tail_words, &"owner")
        && slice_contains(&tail_words, &"hand");
    let is_put_source_on_top_of_library_tail = slice_starts_with(&tail_words, &["put", "this"])
        && slice_contains(&tail_words, &"top")
        && slice_contains(&tail_words, &"owner")
        && tail_words.last().copied() == Some("library");
    let is_choose_card_name_tail =
        (slice_starts_with(&tail_words, &["choose", "any", "card", "name"])
            || slice_starts_with(&tail_words, &["choose", "a", "card", "name"]))
            && head_words.first().copied() == Some("look");
    if !is_that_player_tail
        && !is_return_source_tail
        && !is_put_source_on_top_of_library_tail
        && !is_choose_card_name_tail
    {
        return Ok(None);
    }
    if is_return_source_tail
        && !head_words
            .first()
            .is_some_and(|word| matches!(*word, "tap" | "untap"))
    {
        return Ok(None);
    }
    if is_put_source_on_top_of_library_tail
        && !head_words.first().is_some_and(|word| *word == "draw")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_destroy_then_land_controller_graveyard_count_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(comma_then_idx) = find_comma_then_idx(tokens) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = crate::cards::builders::compiler::token_word_refs(&tail_tokens);
    let suffix = [
        "damage",
        "to",
        "that",
        "lands",
        "controller",
        "equal",
        "to",
        "the",
        "number",
        "of",
        "land",
        "cards",
        "in",
        "that",
        "players",
        "graveyard",
    ];
    let Some(suffix_start) = find_word_sequence_start(&tail_words, &suffix) else {
        return Ok(None);
    };
    if suffix_start == 0 || !matches!(tail_words[suffix_start - 1], "deal" | "deals") {
        return Ok(None);
    }
    if suffix_start + suffix.len() != tail_words.len() {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if !head_effects
        .iter()
        .any(|effect| matches!(effect, EffectAst::Destroy { .. }))
    {
        return Ok(None);
    }

    let mut count_filter = ObjectFilter::default();
    count_filter.zone = Some(Zone::Graveyard);
    let tagged_ref = crate::target::ObjectRef::tagged(IT_TAG);
    count_filter.owner = Some(PlayerFilter::ControllerOf(tagged_ref.clone()));
    count_filter.card_types.push(CardType::Land);
    head_effects.push(EffectAst::DealDamage {
        amount: Value::Count(count_filter),
        target: TargetAst::Player(
            PlayerFilter::ControllerOf(tagged_ref),
            span_from_tokens(&tail_tokens),
        ),
    });
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_destroy_all_attached_to_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    // "destroy all/each <filter> attached to <target>"
    if !tokens.first().is_some_and(|token| token.is_word("destroy")) {
        return Ok(None);
    }
    if !tokens
        .get(1)
        .is_some_and(|token| token.is_word("all") || token.is_word("each"))
    {
        return Ok(None);
    }

    let Some((filter_slice, target_slice)) =
        grammar::split_lexed_once_on_separator(&tokens[2..], || {
            grammar::phrase(&["attached", "to"]).void()
        })
    else {
        return Ok(None);
    };

    let mut filter_tokens = trim_commas(filter_slice).to_vec();
    while filter_tokens
        .last()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| matches!(word, "that" | "were" | "was" | "is" | "are"))
    {
        filter_tokens.pop();
    }
    let target_tokens = trim_commas(target_slice);
    let has_timing_tail = target_tokens.iter().any(|token| {
        token.as_word().is_some_and(|w| {
            matches!(
                w,
                "at" | "beginning" | "end" | "combat" | "turn" | "step" | "until"
            )
        })
    });
    let supported_target = target_tokens.first().is_some_and(|t| t.is_word("target"))
        || grammar::contains_word(&target_tokens, "it") && target_tokens.len() == 1
        || grammar::strip_lexed_prefix_phrase(&target_tokens, &["that", "creature"]).is_some()
        || grammar::strip_lexed_prefix_phrase(&target_tokens, &["that", "permanent"]).is_some()
        || grammar::strip_lexed_prefix_phrase(&target_tokens, &["that", "land"]).is_some()
        || grammar::strip_lexed_prefix_phrase(&target_tokens, &["that", "artifact"]).is_some()
        || grammar::strip_lexed_prefix_phrase(&target_tokens, &["that", "enchantment"]).is_some();
    if filter_tokens.is_empty() || target_tokens.is_empty() || !supported_target || has_timing_tail
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&filter_tokens, false)?;
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(vec![EffectAst::DestroyAllAttachedTo {
        filter,
        target,
    }]))
}

pub(crate) fn parse_sentence_destroy_then_land_controller_graveyard_count_damage(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_then_land_controller_graveyard_count_damage_sentence(tokens)
}

pub(crate) fn find_creature_type_choice_phrase(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    for idx in 0..tokens.len() {
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("the"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("creature"))
            && tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("type"))
            && tokens.get(idx + 4).is_some_and(|token| token.is_word("of"))
            && tokens
                .get(idx + 5)
                .is_some_and(|token| token.is_word("your"))
            && tokens
                .get(idx + 6)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 7));
        }
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("creature"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("type"))
            && tokens.get(idx + 3).is_some_and(|token| token.is_word("of"))
            && tokens
                .get(idx + 4)
                .is_some_and(|token| token.is_word("your"))
            && tokens
                .get(idx + 5)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 6));
        }
    }
    None
}

pub(super) fn find_type_choice_phrase(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    find_creature_type_choice_phrase(tokens).or_else(|| {
        for idx in 0..tokens.len() {
            if tokens[idx].is_word("of")
                && tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("the"))
                && tokens
                    .get(idx + 2)
                    .is_some_and(|token| token.is_word("chosen"))
                && tokens
                    .get(idx + 3)
                    .is_some_and(|token| token.is_word("type"))
            {
                return Some((idx, 4));
            }
            if tokens[idx].is_word("of")
                && tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("chosen"))
                && tokens
                    .get(idx + 2)
                    .is_some_and(|token| token.is_word("type"))
            {
                return Some((idx, 3));
            }
            if tokens[idx].is_word("of")
                && tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("that"))
                && tokens
                    .get(idx + 2)
                    .is_some_and(|token| token.is_word("type"))
            {
                return Some((idx, 3));
            }
            if tokens[idx].is_word("that")
                && tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("type"))
            {
                return Some((idx, 2));
            }
        }
        None
    })
}

pub(crate) fn find_color_choice_phrase(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    for idx in 0..tokens.len() {
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("the"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("color"))
            && tokens.get(idx + 3).is_some_and(|token| token.is_word("of"))
            && (tokens
                .get(idx + 4)
                .is_some_and(|token| token.is_word("your"))
                || tokens
                    .get(idx + 4)
                    .is_some_and(|token| token.is_word("their")))
            && tokens
                .get(idx + 5)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 6));
        }
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("color"))
            && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
            && (tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("your"))
                || tokens
                    .get(idx + 3)
                    .is_some_and(|token| token.is_word("their")))
            && tokens
                .get(idx + 4)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 5));
        }
    }
    None
}
