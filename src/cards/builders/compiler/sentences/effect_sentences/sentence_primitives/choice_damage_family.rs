use super::*;

pub(crate) fn parse_sentence_each_opponent_loses_x_and_you_gain_x(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    if grammar::strip_lexed_prefix_phrase(tokens, &["each", "opponent"]).is_none()
        && grammar::strip_lexed_prefix_phrase(tokens, &["each", "opponents"]).is_none()
    {
        return Ok(None);
    }

    let sentence_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let has_lose_x = find_window_by(&sentence_words, 3, |window| {
        (window[0] == "lose" || window[0] == "loses") && window[1] == "x" && window[2] == "life"
    })
    .is_some();
    let has_gain_x = grammar::contains_phrase(tokens, &["you", "gain", "x", "life"]);
    let Some(where_token_idx) = grammar::find_phrase_start(tokens, &["where", "x", "is"]) else {
        return Ok(None);
    };
    if !has_lose_x || !has_gain_x {
        return Ok(None);
    }

    let where_tokens = &tokens[where_token_idx..];
    let where_value = parse_where_x_value_clause(where_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported where-x value in opponent life-drain clause (clause: '{}')",
            sentence_words.join(" ")
        ))
    })?;

    Ok(Some(vec![
        EffectAst::ForEachOpponent {
            effects: vec![EffectAst::LoseLife {
                amount: where_value.clone(),
                player: PlayerAst::Implicit,
            }],
        },
        EffectAst::GainLife {
            amount: where_value,
            player: PlayerAst::You,
        },
    ]))
}

pub(crate) fn parse_sentence_vote_start(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_vote_start_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_vote_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_vote_clause(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_vote_extra(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_vote_extra_sentence(tokens).map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_after_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_after_turn_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_same_name_target_fanout(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_same_name_target_fanout_sentence(tokens)
}

pub(crate) fn parse_sentence_shared_color_target_fanout(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shared_color_target_fanout_sentence(tokens)
}

pub(crate) fn parse_sentence_same_name_gets_fanout(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_same_name_gets_fanout_sentence(tokens)
}

pub(crate) fn parse_sentence_delayed_until_next_end_step(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_delayed_until_next_end_step_sentence(tokens)
}

pub(crate) fn parse_sentence_destroy_or_exile_all_split(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_or_exile_all_split_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_up_to_one_each_target_type(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_up_to_one_each_target_type_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_multi_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    if !tokens.first().is_some_and(|t| t.is_word("exile"))
        || grammar::contains_word(tokens, "unless")
    {
        return Ok(None);
    }

    let mut split_idx = None;
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_word("and") || idx == 0 || idx + 1 >= tokens.len() {
            continue;
        }
        let tail = &tokens[idx + 1..];
        let starts_second_target = tail.first().is_some_and(|t| t.is_word("target"))
            || (grammar::strip_lexed_prefix_phrase(tail, &["up", "to"]).is_some()
                && grammar::contains_word(tail, "target"));
        if starts_second_target {
            split_idx = Some(idx);
            break;
        }
    }

    let Some(and_idx) = split_idx else {
        return Ok(None);
    };

    let first_tokens = trim_commas(&tokens[1..and_idx]);
    let second_tokens = trim_commas(&tokens[and_idx + 1..]);
    if first_tokens.is_empty() || second_tokens.is_empty() {
        return Ok(None);
    }

    let first_words = crate::cards::builders::compiler::token_word_refs(&first_tokens);
    let first_is_explicit_target = first_tokens.first().is_some_and(|t| t.is_word("target"))
        || (grammar::strip_lexed_prefix_phrase(&first_tokens, &["up", "to"]).is_some()
            && grammar::contains_word(&first_tokens, "target"));
    let second_is_explicit_target = second_tokens.first().is_some_and(|t| t.is_word("target"))
        || (grammar::strip_lexed_prefix_phrase(&second_tokens, &["up", "to"]).is_some()
            && grammar::contains_word(&second_tokens, "target"));

    let mut first_target = match parse_target_phrase(&first_tokens) {
        Ok(target) => target,
        Err(_)
            if !first_is_explicit_target
                && is_likely_named_or_source_reference_words(&first_words) =>
        {
            TargetAst::Source(span_from_tokens(&first_tokens))
        }
        Err(err) => return Err(err),
    };
    let mut second_target = parse_target_phrase(&second_tokens)?;

    if first_is_explicit_target
        && second_is_explicit_target
        && let (Some((mut first_filter, first_count)), Some((mut second_filter, second_count))) = (
            object_target_with_count(&first_target),
            object_target_with_count(&second_target),
        )
        && first_filter.zone == Some(Zone::Graveyard)
        && second_filter.zone == Some(Zone::Graveyard)
    {
        if first_filter.controller.is_none() {
            first_filter.controller = Some(PlayerFilter::Any);
        }
        if second_filter.controller.is_none() {
            second_filter.controller = Some(PlayerFilter::Any);
        }
        let tag = helper_tag_for_tokens(tokens, "exiled");
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: first_filter,
                count: first_count,
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::ChooseObjects {
                filter: second_filter,
                count: second_count,
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::Exile {
                target: TargetAst::Tagged(tag, None),
                face_down: false,
            },
        ]));
    }

    apply_exile_subject_hand_owner_context(&mut first_target, None);
    apply_exile_subject_hand_owner_context(&mut second_target, None);
    Ok(Some(vec![
        EffectAst::Exile {
            target: first_target,
            face_down: false,
        },
        EffectAst::Exile {
            target: second_target,
            face_down: false,
        },
    ]))
}

pub(crate) fn split_destroy_target_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut raw_segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    for and_segment in split_lexed_slices_on_and(tokens) {
        for comma_segment in split_lexed_slices_on_comma(and_segment) {
            let trimmed = trim_commas(&comma_segment);
            if !trimmed.is_empty() {
                raw_segments.push(trimmed.to_vec());
            }
        }
    }

    let mut segments = Vec::new();
    for segment in raw_segments {
        let split_starts = segment
            .iter()
            .enumerate()
            .filter_map(|(idx, token)| {
                if idx >= 3
                    && token.is_word("target")
                    && segment[idx - 3].is_word("up")
                    && segment[idx - 2].is_word("to")
                    && segment[idx - 1].is_word("one")
                {
                    Some(idx - 3)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if split_starts.len() <= 1 {
            segments.push(segment);
            continue;
        }

        for (idx, start) in split_starts.iter().enumerate() {
            let end = split_starts.get(idx + 1).copied().unwrap_or(segment.len());
            let trimmed = trim_commas(&segment[*start..end]);
            if !trimmed.is_empty() {
                segments.push(trimmed.to_vec());
            }
        }
    }

    segments
}

pub(crate) fn parse_sentence_destroy_multi_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    if !tokens.first().is_some_and(|t| t.is_word("destroy")) {
        return Ok(None);
    }
    if tokens
        .get(1)
        .is_some_and(|t| t.is_word("all") || t.is_word("each"))
    {
        return Ok(None);
    }
    if grammar::contains_word(tokens, "unless") || grammar::contains_word(tokens, "if") {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..]);
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let has_separator = target_tokens
        .iter()
        .any(|token| token.is_word("and") || token.is_comma());
    let mut repeated_up_to_one_targets = 0usize;
    let mut start = 0usize;
    while start + 4 <= target_tokens.len() {
        let window = &target_tokens[start..start + 4];
        if window[0].is_word("up")
            && window[1].is_word("to")
            && window[2].is_word("one")
            && window[3].is_word("target")
        {
            repeated_up_to_one_targets += 1;
        }
        start += 1;
    }
    let has_repeated_up_to_one_targets = repeated_up_to_one_targets >= 2;
    if !has_separator && !has_repeated_up_to_one_targets {
        return Ok(None);
    }

    let segments = split_destroy_target_segments(&target_tokens);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for segment in segments {
        let segment_words = crate::cards::builders::compiler::token_word_refs(&segment);
        if segment_words.iter().any(|word| {
            matches!(
                *word,
                "then" | "if" | "unless" | "where" | "when" | "whenever"
            )
        }) {
            return Ok(None);
        }
        let is_explicit_target = segment_words.first() == Some(&"target")
            || (grammar::words_match_any_prefix(&segment, UP_TO_PREFIXES).is_some()
                && grammar::contains_word(&segment, "target"));
        if !is_explicit_target && !is_likely_named_or_source_reference_words(&segment_words) {
            return Ok(None);
        }
        let target = match parse_target_phrase(&segment) {
            Ok(target) => target,
            Err(_)
                if !is_explicit_target
                    && is_likely_named_or_source_reference_words(&segment_words) =>
            {
                TargetAst::Source(span_from_tokens(&segment))
            }
            Err(err) => return Err(err),
        };
        effects.push(EffectAst::Destroy { target });
    }

    if effects.len() < 2 {
        return Ok(None);
    }
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_reveal_selected_cards_in_your_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.first() != Some(&"reveal") {
        return Ok(None);
    }
    if clause_words.iter().any(|word| {
        matches!(
            *word,
            "then" | "if" | "unless" | "where" | "when" | "whenever"
        )
    }) {
        return Ok(None);
    }

    use super::super::super::grammar::primitives as grammar;

    let in_your_hand = grammar::strip_lexed_suffix_phrase(tokens, &["in", "your", "hand"])
        .or_else(|| grammar::strip_lexed_suffix_phrase(tokens, &["in", "your", "hands"]))
        .or_else(|| grammar::strip_lexed_suffix_phrase(tokens, &["from", "your", "hand"]))
        .or_else(|| grammar::strip_lexed_suffix_phrase(tokens, &["from", "your", "hands"]));
    let Some(before_in) = in_your_hand else {
        return Ok(None);
    };
    if before_in.is_empty() {
        return Ok(None);
    }

    let mut descriptor_tokens = trim_commas(&before_in[1..]);
    if descriptor_tokens.is_empty() {
        return Ok(None);
    }

    let mut count = ChoiceCount::exactly(1);
    let descriptor_words = crate::cards::builders::compiler::token_word_refs(&descriptor_tokens);
    if grammar::words_match_any_prefix(&descriptor_tokens, ANY_NUMBER_OF_PREFIXES).is_some() {
        count = ChoiceCount::any_number();
        descriptor_tokens = trim_commas(&descriptor_tokens[3..]);
    } else if grammar::words_match_any_prefix(&descriptor_tokens, UP_TO_PREFIXES).is_some() {
        if let Some((value, used)) = parse_number(&descriptor_tokens[2..]) {
            count = ChoiceCount::up_to(value as usize);
            descriptor_tokens = trim_commas(&descriptor_tokens[2 + used..]);
            if descriptor_tokens
                .first()
                .is_some_and(|token| token.is_word("of"))
            {
                descriptor_tokens = trim_commas(&descriptor_tokens[1..]);
            }
        } else {
            return Ok(None);
        }
    } else if descriptor_words.first() == Some(&"x") {
        count = ChoiceCount::any_number();
        descriptor_tokens = trim_commas(&descriptor_tokens[1..]);
    } else if descriptor_words
        .first()
        .is_some_and(|word| matches!(*word, "a" | "an" | "one"))
    {
        descriptor_tokens = trim_commas(&descriptor_tokens[1..]);
    } else if descriptor_words
        .first()
        .is_some_and(|word| matches!(*word, "all" | "each"))
    {
        return Ok(None);
    }

    if descriptor_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = match parse_object_filter(&descriptor_tokens, false) {
        Ok(filter) => filter,
        Err(_) => {
            let descriptor_words =
                crate::cards::builders::compiler::token_word_refs(&descriptor_tokens);
            let mut filter = ObjectFilter::default();
            let mut idx = 0usize;
            if let Some(color) = descriptor_words.get(idx).and_then(|word| parse_color(word)) {
                filter.colors = Some(color.into());
                idx += 1;
            }
            if !descriptor_words
                .get(idx)
                .is_some_and(|word| matches!(*word, "card" | "cards"))
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported reveal-hand clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            filter
        }
    };
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(PlayerFilter::You);

    let tag = helper_tag_for_tokens(tokens, "revealed");
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count,
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        },
        EffectAst::RevealTagged { tag },
    ]))
}

pub(crate) fn parse_sentence_target_player_reveals_random_card_from_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(reveal_idx) = find_index(tokens, |token| {
        token.is_word("reveal") || token.is_word("reveals")
    }) else {
        return Ok(None);
    };
    if reveal_idx == 0 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..reveal_idx]);
    let SubjectAst::Player(player) = parse_subject(&subject_tokens) else {
        return Ok(None);
    };
    if !matches!(
        player,
        PlayerAst::You
            | PlayerAst::Target
            | PlayerAst::TargetOpponent
            | PlayerAst::Opponent
            | PlayerAst::That
    ) {
        return Ok(None);
    }

    let reveal_tokens = trim_commas(&tokens[reveal_idx + 1..]);
    let reveal_words = crate::cards::builders::compiler::token_word_refs(&reveal_tokens);
    if reveal_words.is_empty()
        || !reveal_words
            .first()
            .is_some_and(|word| matches!(*word, "a" | "an" | "one"))
    {
        return Ok(None);
    }

    let descriptor_words = &reveal_words[1..];
    if descriptor_words.is_empty() || !descriptor_words.contains(&"card") {
        return Ok(None);
    }

    let Some(from_idx) = find_word_sequence_start(descriptor_words, &["from"]) else {
        return Ok(None);
    };
    if !find_word_sequence_start(descriptor_words, &["at", "random"])
        .is_some_and(|idx| idx < from_idx)
    {
        return Ok(None);
    }

    let hand_words = &descriptor_words[from_idx + 1..];
    if !matches!(
        hand_words,
        ["their", "hand"]
            | ["their", "hands"]
            | ["your", "hand"]
            | ["your", "hands"]
            | ["that", "player", "hand"]
            | ["that", "player", "hands"]
            | ["target", "player", "hand"]
            | ["target", "player", "hands"]
    ) {
        return Ok(None);
    }

    let filter = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(match player {
            PlayerAst::You => PlayerFilter::You,
            PlayerAst::Target => PlayerFilter::target_player(),
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::Opponent => PlayerFilter::Opponent,
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            _ => return Ok(None),
        }),
        ..ObjectFilter::default()
    };
    let tag = helper_tag_for_tokens(tokens, "revealed");

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1).at_random(),
            count_value: None,
            player,
            tag: tag.clone(),
        },
        EffectAst::RevealTagged { tag },
    ]))
}

pub(crate) fn object_target_with_count(target: &TargetAst) -> Option<(ObjectFilter, ChoiceCount)> {
    match target {
        TargetAst::Object(filter, _, _) => Some((filter.clone(), ChoiceCount::exactly(1))),
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, _, _) => Some((filter.clone(), count.clone())),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn is_likely_named_or_source_reference_words(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }
    if is_source_reference_words(words) {
        return true;
    }
    if words.iter().any(|word| {
        matches!(
            *word,
            "then"
                | "if"
                | "unless"
                | "where"
                | "when"
                | "whenever"
                | "for"
                | "each"
                | "search"
                | "destroy"
                | "exile"
                | "draw"
                | "gain"
                | "lose"
                | "counter"
                | "put"
                | "return"
                | "create"
                | "sacrifice"
                | "deal"
                | "populate"
        )
    }) {
        return false;
    }
    !words.iter().any(|word| {
        matches!(
            *word,
            "a" | "an"
                | "the"
                | "this"
                | "that"
                | "those"
                | "it"
                | "them"
                | "target"
                | "all"
                | "any"
                | "each"
                | "another"
                | "other"
                | "up"
                | "to"
                | "card"
                | "cards"
                | "creature"
                | "creatures"
                | "permanent"
                | "permanents"
                | "artifact"
                | "artifacts"
                | "enchantment"
                | "enchantments"
                | "land"
                | "lands"
                | "planeswalker"
                | "planeswalkers"
                | "spell"
                | "spells"
        )
    })
}

pub(crate) fn parse_sentence_damage_unless_controller_has_source_deal_damage(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    let Some((before_slice, after_unless_slice)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("unless").void())
    else {
        return Ok(None);
    };

    let before_tokens = trim_commas(before_slice);
    if before_tokens.is_empty() {
        return Ok(None);
    }
    let effects = parse_effect_chain(&before_tokens)?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let Some(main_damage) = effects.first() else {
        return Ok(None);
    };
    let EffectAst::DealDamage {
        amount: main_amount,
        target: main_target,
    } = main_damage
    else {
        return Ok(None);
    };
    if !matches!(
        main_target,
        TargetAst::Object(_, _, _) | TargetAst::WithCount(_, _)
    ) {
        return Ok(None);
    }

    let after_unless = trim_commas(after_unless_slice);
    let has_controller_clause = grammar::words_match_any_prefix(&after_unless, THAT_PREFIXES)
        .is_some()
        && (grammar::contains_word(&after_unless, "controller")
            || grammar::contains_word(&after_unless, "controllers"));
    if !has_controller_clause {
        return Ok(None);
    }
    let Some(has_idx) = find_index(&after_unless, |token| {
        token.is_word("has") || token.is_word("have")
    }) else {
        return Ok(None);
    };
    if has_idx + 1 >= after_unless.len() {
        return Ok(None);
    }

    let alt_tokens = &after_unless[has_idx + 1..];
    let Some(deal_idx) = find_index(&alt_tokens, |token| {
        token.is_word("deal") || token.is_word("deals")
    }) else {
        return Ok(None);
    };
    let deal_tail = &alt_tokens[deal_idx..];
    let Some((alt_amount, used)) = parse_value(&deal_tail[1..]) else {
        return Ok(None);
    };
    if !deal_tail
        .get(1 + used)
        .is_some_and(|token| token.is_word("damage"))
    {
        return Ok(None);
    }

    let mut alt_target_tokens = &deal_tail[2 + used..];
    if alt_target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        alt_target_tokens = &alt_target_tokens[1..];
    }
    let alt_target_words = crate::cards::builders::compiler::token_word_refs(alt_target_tokens);
    if !matches!(alt_target_words.as_slice(), ["them"] | ["that", "player"]) {
        return Ok(None);
    }

    let alternative = EffectAst::DealDamage {
        amount: alt_amount,
        target: TargetAst::Player(
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            None,
        ),
    };
    let unless = EffectAst::UnlessAction {
        effects: vec![EffectAst::DealDamage {
            amount: main_amount.clone(),
            target: main_target.clone(),
        }],
        alternative: vec![alternative],
        player: PlayerAst::ItsController,
    };
    Ok(Some(vec![unless]))
}

pub(crate) fn parse_sentence_damage_to_that_player_unless_enchanted_attacked(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    let Some((before_slice, after_slice)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("unless").void())
    else {
        return Ok(None);
    };

    let before_tokens = trim_commas(before_slice);
    let after_tokens = trim_commas(after_slice);
    if before_tokens.is_empty() || after_tokens.is_empty() {
        return Ok(None);
    }

    if !matches!(
        crate::cards::builders::compiler::token_word_refs(&after_tokens).as_slice(),
        ["that", "creature", "attacked", "this", "turn"]
            | ["enchanted", "creature", "attacked", "this", "turn"]
    ) {
        return Ok(None);
    }

    let deal_split =
        grammar::split_lexed_once_on_separator(&before_tokens, || grammar::kw("deal").void())
            .or_else(|| {
                grammar::split_lexed_once_on_separator(&before_tokens, || {
                    grammar::kw("deals").void()
                })
            });
    let Some((subject_slice, damage_tokens)) = deal_split else {
        return Ok(None);
    };

    if !matches!(
        crate::cards::builders::compiler::token_word_refs(subject_slice).as_slice(),
        ["this", "aura"] | ["this", "permanent"] | ["this", "enchantment"]
    ) {
        return Ok(None);
    }
    let Some((amount, used)) = parse_value(damage_tokens) else {
        return Ok(None);
    };
    if !damage_tokens
        .get(used)
        .is_some_and(|token| token.is_word("damage"))
    {
        return Ok(None);
    }

    let mut target_tokens = trim_commas(&damage_tokens[used + 1..]);
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens.remove(0);
    }
    if crate::cards::builders::compiler::token_word_refs(&target_tokens).as_slice()
        != ["that", "player"]
    {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::Conditional {
        predicate: PredicateAst::Not(Box::new(PredicateAst::EnchantedPermanentAttackedThisTurn)),
        if_true: vec![EffectAst::DealDamage {
            amount,
            target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
        }],
        if_false: Vec::new(),
    }]))
}

pub(crate) fn parse_sentence_unless_pays(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let unless_idx = match find_index(tokens, |t| t.is_word("unless")) {
        Some(idx) => idx,
        None => return Ok(None),
    };

    if unless_idx == 0 {
        let comma_idx = match find_index(tokens, |token| token.is_comma()) {
            Some(idx) => idx,
            None => return Ok(None),
        };
        if comma_idx + 1 >= tokens.len() {
            return Ok(None);
        }

        let effects = parse_effect_chain(&tokens[comma_idx + 1..])?;
        if effects.is_empty() {
            return Ok(None);
        }

        let unless_clause = &tokens[..comma_idx];
        if let Some(unless_effect) = try_build_unless(effects, unless_clause, 0)? {
            return Ok(Some(vec![unless_effect]));
        }
        return Ok(None);
    }

    let before_words: Vec<&str> = tokens[..unless_idx]
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect();

    if before_words.first() == Some(&"counter") {
        return Ok(None);
    }
    if before_words.first() == Some(&"create")
        && grammar::contains_word(&tokens[..unless_idx], "token")
        && grammar::contains_word(&tokens[..unless_idx], "sacrifice")
        && grammar::contains_word(&tokens[..unless_idx], "counter")
    {
        return Ok(None);
    }

    let each_prefix =
        if grammar::words_match_any_prefix(&tokens[..unless_idx], EACH_OPPONENT_PREFIXES).is_some()
        {
            Some("opponent")
        } else if grammar::words_match_any_prefix(&tokens[..unless_idx], EACH_PLAYER_PREFIXES)
            .is_some()
        {
            Some("player")
        } else {
            None
        };
    if let Some(prefix_kind) = each_prefix {
        let inner_token_start = tokens
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.as_word().map(|_| i))
            .nth(2)
            .unwrap_or(2);
        let inner_tokens = &tokens[inner_token_start..unless_idx];
        if let Ok(inner_effects) = parse_effect_chain(inner_tokens) {
            if !inner_effects.is_empty() {
                if let Some(unless_effect) = try_build_unless(inner_effects, tokens, unless_idx)? {
                    let wrapper = match prefix_kind {
                        "opponent" => EffectAst::ForEachOpponent {
                            effects: vec![unless_effect],
                        },
                        _ => EffectAst::ForEachPlayer {
                            effects: vec![unless_effect],
                        },
                    };
                    return Ok(Some(vec![wrapper]));
                }
            }
        }
        return Ok(None);
    }

    let effect_tokens = &tokens[..unless_idx];
    if let Some((timing_start_word, _timing_end_word, step, player)) =
        delayed_next_step_marker(effect_tokens)
    {
        let Some(timing_token_idx) = token_index_for_word_index(effect_tokens, timing_start_word)
        else {
            return Ok(None);
        };
        let delayed_effect_tokens = trim_commas(&effect_tokens[..timing_token_idx]);
        if delayed_effect_tokens.is_empty() {
            return Ok(None);
        }
        let delayed_effects = parse_effect_chain(&delayed_effect_tokens)?;
        if delayed_effects.is_empty() {
            return Ok(None);
        }
        if let Some(unless_effect) = try_build_unless(delayed_effects, tokens, unless_idx)? {
            return Ok(Some(vec![wrap_delayed_next_step_unless_pays(
                step,
                player,
                vec![unless_effect],
            )]));
        }
    }

    let effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Ok(None);
    }

    if let Some(unless_effect) = try_build_unless(effects, tokens, unless_idx)? {
        return Ok(Some(vec![unless_effect]));
    }
    Ok(None)
}
