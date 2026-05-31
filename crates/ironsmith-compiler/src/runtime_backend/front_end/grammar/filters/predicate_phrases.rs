use super::*;

fn parse_outlaw_shorthand_filter(words: &[&str]) -> Option<ObjectFilter> {
    let trimmed = match words {
        ["a" | "an", tail @ ..] => tail,
        _ => words,
    };
    if !matches!(
        trimmed,
        ["outlaw"] | ["outlaws"] | ["outlaw", "creature"] | ["outlaws", "creatures"]
    ) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    push_outlaw_subtypes(&mut filter.subtypes);
    filter.card_types.push(CardType::Creature);
    Some(filter)
}

fn parse_attachment_quantity_prefix(
    tokens: &[OwnedLexToken],
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing quantity in attachment-count predicate".to_string(),
        ));
    }

    if tokens[0].is_word("no") {
        return Ok((crate::effect::Comparison::LessThanOrEqual(0), 1));
    }

    if tokens[0].is_word("exactly") {
        let (value, used) = parse_number(tokens.get(1..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("missing quantity in attachment-count predicate".to_string())
        })?;
        return Ok((crate::effect::Comparison::Equal(value as i32), used + 1));
    }

    if token_slice_first_is_any(tokens, &["fewer", "less"]) && token_slice_at_is(tokens, 1, "than")
    {
        let (value, used) = parse_number(tokens.get(2..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("missing quantity in attachment-count predicate".to_string())
        })?;
        return Ok((crate::effect::Comparison::LessThan(value as i32), used + 2));
    }

    if token_slice_first_is_any(tokens, &["more", "greater"])
        && token_slice_at_is(tokens, 1, "than")
    {
        let (value, used) = parse_number(tokens.get(2..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("missing quantity in attachment-count predicate".to_string())
        })?;
        return Ok((
            crate::effect::Comparison::GreaterThan(value as i32),
            used + 2,
        ));
    }

    if let Some((value, used)) = parse_number(tokens) {
        let value = value as i32;
        if token_slice_at_is(tokens, used, "or")
            && token_slice_at_is_any(tokens, used + 1, &["more", "greater"])
        {
            return Ok((
                crate::effect::Comparison::GreaterThanOrEqual(value),
                used + 2,
            ));
        }
        if token_slice_at_is(tokens, used, "or")
            && token_slice_at_is_any(tokens, used + 1, &["less", "fewer"])
        {
            return Ok((crate::effect::Comparison::LessThanOrEqual(value), used + 2));
        }
        return Ok((crate::effect::Comparison::Equal(value), used));
    }

    Err(CardTextError::ParseError(
        "missing quantity in attachment-count predicate".to_string(),
    ))
}

fn parse_source_exiled_with_counter_predicate(
    raw_words: &[&str],
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let with_idx = if word_slice_starts_with(raw_words, &["this", "card", "is", "exiled", "with"])
        || word_slice_starts_with(raw_words, &["this", "source", "is", "exiled", "with"])
    {
        4
    } else {
        return None;
    };
    let counter_idx = find_index(&raw_words[with_idx + 1..], |word| {
        *word == "counter" || *word == "counters"
    })? + with_idx
        + 1;
    if !matches!(
        raw_words.get(counter_idx + 1..),
        Some(["on", "it"] | ["on", "this"] | ["on", "them"])
    ) {
        return None;
    }

    let counter_type = parse_counter_type_from_tokens(&tokens[with_idx + 1..=counter_idx])?;
    let count = parse_number(&tokens[with_idx + 1..counter_idx])
        .map(|(count, _)| count)
        .unwrap_or(1);
    Some(PredicateAst::And(
        Box::new(PredicateAst::SourceIsInZone(Zone::Exile)),
        Box::new(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        }),
    ))
}

fn parse_stack_object_targets_only_source_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tail =
        if crate::runtime_backend::lexer::word_slice_starts_with(filtered, &["that", "spell"]) {
            &filtered[2..]
        } else if crate::runtime_backend::lexer::word_slice_starts_with(filtered, &["spell"]) {
            &filtered[1..]
        } else if crate::runtime_backend::lexer::word_slice_starts_with(filtered, &["it"]) {
            &filtered[1..]
        } else {
            return None;
        };

    if !crate::runtime_backend::lexer::word_slice_starts_with(tail, &["targets", "only"]) {
        return None;
    }

    let target_words = &tail[2..];
    let mut target_filter = match target_words {
        ["this", "creature"] => ObjectFilter::creature(),
        ["this", "artifact"] => ObjectFilter::artifact(),
        ["this", "enchantment"] => ObjectFilter::enchantment(),
        ["this", "land"] => ObjectFilter::land(),
        ["this", "permanent"] => ObjectFilter::default().in_zone(Zone::Battlefield),
        ["this", "source"] | ["it"] => ObjectFilter::source(),
        _ => return None,
    };
    target_filter.source = true;

    Some(PredicateAst::ItMatches(
        ObjectFilter::spell()
            .targeting_only_object(target_filter)
            .target_count_exact(1),
    ))
}

fn mana_cost_label_from_words(words: &[&str]) -> Option<String> {
    if words.is_empty() {
        return None;
    }

    let mut label = String::new();
    for word in words {
        if word.chars().all(|ch| ch.is_ascii_digit()) {
            label.push('{');
            label.push_str(word);
            label.push('}');
            continue;
        }
        if parse_mana_symbol(word).is_ok() {
            label.push('{');
            label.push_str(&word.to_ascii_uppercase());
            label.push('}');
            continue;
        }
        return None;
    }

    Some(label)
}

fn ordinal_number_word(word: &str) -> Option<u32> {
    ironsmith_core::parse_ordinal_word(word).or_else(|| parse_named_number(word))
}

fn parse_this_ability_resolution_count_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let count = match filtered {
        [
            "this",
            "is",
            count,
            "time",
            "this",
            "ability",
            "has",
            "resolved",
            "this",
            "turn",
        ]
        | [
            "this",
            "is",
            count,
            "time",
            "this",
            "ability",
            "resolved",
            "this",
            "turn",
        ]
        | [
            "this",
            "ability",
            "has",
            "resolved",
            "for",
            count,
            "time",
            "this",
            "turn",
        ]
        | [
            "this",
            "ability",
            "resolved",
            "for",
            count,
            "time",
            "this",
            "turn",
        ] => ordinal_number_word(count)?,
        ["it's", count, "time"] | ["its", count, "time"] | ["it", "s", count, "time"] => {
            ordinal_number_word(count)?
        }
        _ => return None,
    };

    Some(PredicateAst::ThisAbilityResolvedThisTurnExactly(count))
}

fn spell_cast_matching_predicate(
    player: PlayerFilter,
    filter_words: &[&str],
) -> Result<PredicateAst, CardTextError> {
    let filter_tokens = filter_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let filter = parse_object_filter_lexed(&filter_tokens, false)?;
    Ok(PredicateAst::ValueComparison {
        left: Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source: false,
        },
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    })
}

fn parse_both_spell_cast_predicate(
    player: PlayerFilter,
    filter_words: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let stripped = crate::runtime_backend::lexer::word_slice_strip_prefix(filter_words, &["both"])
        .unwrap_or(filter_words);
    let Some(and_idx) = find_index(stripped, |word| *word == "and") else {
        return Ok(None);
    };
    let left_words = &stripped[..and_idx];
    let right_words = &stripped[and_idx + 1..];
    if left_words.is_empty() || right_words.is_empty() {
        return Ok(None);
    }
    if !crate::runtime_backend::lexer::word_slice_starts_with(filter_words, &["both"])
        && !crate::runtime_backend::lexer::word_slice_starts_with_any(
            left_words,
            &[&["a", "spell", "named"], &["spell", "named"]],
        )
    {
        return Ok(None);
    }
    if !crate::runtime_backend::lexer::word_slice_starts_with(filter_words, &["both"])
        && !crate::runtime_backend::lexer::word_slice_starts_with_any(
            right_words,
            &[&["a", "spell", "named"], &["spell", "named"]],
        )
    {
        return Ok(None);
    }
    let left = spell_cast_matching_predicate(player.clone(), left_words)?;
    let right = spell_cast_matching_predicate(player, right_words)?;
    Ok(Some(PredicateAst::And(Box::new(left), Box::new(right))))
}

fn predicate_tokens_from_words(words: &[&str]) -> Vec<OwnedLexToken> {
    words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect()
}

fn parse_color_only_object_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut filter = ObjectFilter::default();
    let mut saw_color = false;
    for word in words {
        if matches!(*word, "and" | "or") {
            continue;
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
            saw_color = true;
            continue;
        }
        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            saw_color = true;
            continue;
        }
        return None;
    }
    saw_color.then_some(filter)
}

fn parse_this_way_object_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let has_card_noun = word_slice_last_is_any(words, &["card", "cards"]);
    let candidates = [
        (words, has_card_noun),
        (
            crate::runtime_backend::lexer::word_slice_strip_suffix(words, &["card"])
                .unwrap_or(words),
            true,
        ),
        (
            crate::runtime_backend::lexer::word_slice_strip_suffix(words, &["cards"])
                .unwrap_or(words),
            true,
        ),
    ];
    for (candidate, stripped_card_noun) in candidates {
        if candidate.is_empty() {
            return Some(ObjectFilter::default());
        }
        let tokens = predicate_tokens_from_words(candidate);
        if let Ok(mut filter) = parse_object_filter(&tokens, false) {
            if stripped_card_noun {
                filter.zone = None;
            }
            return Some(filter);
        }
        if let Some(mut filter) = parse_color_only_object_filter_words(candidate) {
            if stripped_card_noun {
                filter.zone = None;
            }
            return Some(filter);
        }
    }
    None
}

fn parse_passive_this_way_tagged_object_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    if filtered.len() < 5 || !word_slice_ends_with(filtered, &["this", "way"]) {
        return Ok(None);
    }
    let verb_idx = filtered.len() - 3;
    let copula_idx = verb_idx.saturating_sub(1);
    if copula_idx == 0
        || !matches!(filtered[copula_idx], "is" | "are" | "was" | "were")
        || !matches!(
            filtered[verb_idx],
            "countered"
                | "destroyed"
                | "discarded"
                | "exiled"
                | "milled"
                | "returned"
                | "revealed"
                | "sacrificed"
        )
    {
        return Ok(None);
    }

    let filter_words = &filtered[..copula_idx];
    let Some(filter) = parse_this_way_object_filter_words(filter_words) else {
        return Ok(None);
    };
    Ok(Some(PredicateAst::TaggedMatches(
        TagKey::from(IT_TAG),
        filter,
    )))
}

fn parse_repeated_if_or_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(or_idx) =
        crate::runtime_backend::lexer::word_slice_find_phrase_start(filtered, &["or", "if"])
    else {
        return Ok(None);
    };
    if or_idx == 0 || or_idx + 2 >= filtered.len() {
        return Ok(None);
    }

    let left_tokens = predicate_tokens_from_words(&filtered[..or_idx]);
    let right_tokens = predicate_tokens_from_words(&filtered[or_idx + 2..]);
    let left = match parse_predicate(&left_tokens) {
        Ok(predicate) => predicate,
        Err(_) => return Ok(None),
    };
    let right = parse_predicate(&right_tokens)?;
    Ok(Some(PredicateAst::Or(Box::new(left), Box::new(right))))
}

fn predicate_reference_prefix<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    if word_slice_first_is(words, "it") {
        return Some(&words[..1]);
    }
    if words.len() >= 2
        && words[0] == "that"
        && matches!(
            words[1],
            "artifact"
                | "card"
                | "creature"
                | "creatures"
                | "enchantment"
                | "land"
                | "object"
                | "permanent"
                | "source"
                | "spell"
                | "token"
        )
    {
        return Some(&words[..2]);
    }
    None
}

fn predicate_words_start_with_reference(words: &[&str]) -> bool {
    matches!(
        words.first().copied(),
        Some(
            "it" | "its"
                | "this"
                | "that"
                | "you"
                | "your"
                | "opponent"
                | "player"
                | "target"
                | "source"
        )
    )
}

fn parse_single_card_type_card_descriptor(words: &[&str]) -> Option<ObjectFilter> {
    if words.len() == 2
        && matches!(words[1], "card" | "cards")
        && let Some(card_type) = parse_card_type(words[0])
    {
        return Some(ObjectFilter {
            card_types: vec![card_type],
            ..Default::default()
        });
    }
    None
}

fn parse_or_predicate(filtered: &[&str]) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(or_idx) = rfind_index_with(filtered, |idx, word| {
        if *word != "or" || idx == 0 || idx + 1 >= filtered.len() {
            return false;
        }
        if matches!(
            filtered.get(idx + 1).copied(),
            Some("more" | "fewer" | "less" | "greater" | "equal")
        ) {
            return false;
        }
        true
    }) else {
        return Ok(None);
    };

    let left_words = &filtered[..or_idx];
    let right_words = &filtered[or_idx + 1..];
    let left_tokens = predicate_tokens_from_words(left_words);
    let right_tokens = predicate_tokens_from_words(right_words);
    let left = parse_predicate(&left_tokens)?;
    let right = match parse_predicate(&right_tokens) {
        Ok(predicate) => predicate,
        Err(original_err) => {
            let Some(reference_prefix) = predicate_reference_prefix(left_words) else {
                return Err(original_err);
            };
            if predicate_words_start_with_reference(right_words) {
                return Err(original_err);
            }
            let prefixed_words = reference_prefix
                .iter()
                .copied()
                .chain(right_words.iter().copied())
                .collect::<Vec<_>>();
            let prefixed_tokens = predicate_tokens_from_words(&prefixed_words);
            parse_predicate(&prefixed_tokens).map_err(|_| original_err)?
        }
    };
    Ok(Some(PredicateAst::Or(Box::new(left), Box::new(right))))
}

fn player_filter_for_turn_value(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Some(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Some(PlayerFilter::LowestLifeTied),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::NotYou => Some(PlayerFilter::NotYou),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::ThatPlayerOrTargetController => {
            Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
        }
        PlayerAst::ItsController | PlayerAst::ItsOwner => None,
    }
}

fn graveyard_possessive_matches_subject(player: PlayerAst, possessive: &str) -> bool {
    match player {
        PlayerAst::You | PlayerAst::Implicit => possessive == "your",
        _ => possessive == "their",
    }
}

fn permanents_you_control_scope(words: &[&str]) -> Option<ObjectFilter> {
    if matches!(
        words,
        ["permanent" | "permanents", "you", "control" | "controls"]
    ) {
        return Some(ObjectFilter::permanent().you_control());
    }
    None
}

fn cards_in_your_graveyard_scope(words: &[&str]) -> Option<ObjectFilter> {
    if word_slice_eq_any(
        words,
        &[
            &["card", "in", "your", "graveyard"],
            &["cards", "in", "your", "graveyard"],
        ],
    ) {
        return Some(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
    }
    None
}

fn permanents_and_your_graveyard_scope(words: &[&str]) -> Option<ObjectFilter> {
    let graveyard_start = if words.len() == 8 && words[3] == "and/or" {
        4
    } else if words.len() == 9 && words[3] == "and" && words[4] == "or" {
        5
    } else {
        return None;
    };
    let battlefield = permanents_you_control_scope(&words[..3])?;
    let graveyard = cards_in_your_graveyard_scope(&words[graveyard_start..])?;
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![battlefield, graveyard];
    Some(filter)
}

fn parse_colors_among_predicate(words: &[&str]) -> Option<PredicateAst> {
    if words.len() >= 7
        && word_slice_starts_with(words, &["there", "are"])
        && word_slice_at_is_any(words, 3, &["color", "colors"])
        && word_slice_at_is(words, 4, "among")
        && let Some(count) = parse_named_number(words[2])
        && let Some(filter) = permanents_you_control_scope(&words[5..])
    {
        return Some(PredicateAst::ValueComparison {
            left: Value::ColorsAmong(filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }
    None
}

fn parse_card_types_among_predicate(words: &[&str]) -> Option<PredicateAst> {
    if words.len() >= 9
        && word_slice_first_is(words, "there")
        && word_slice_at_is_any(words, 1, &["are", "were"])
        && word_slice_starts_with(&words[3..], &["or", "more", "card"])
        && word_slice_at_is_any(words, 6, &["type", "types"])
        && word_slice_at_is(words, 7, "among")
        && word_slice_at_is_any(words, 8, &["sacrificed", "sacrificed_0"])
        && (word_slice_at_is_any(words, 9, &["permanent", "permanents"]) || words.len() == 9)
        && let Some(count) = parse_named_number(words[2])
    {
        return Some(PredicateAst::ValueComparison {
            left: Value::CardTypesAmong(ObjectFilter::tagged("sacrificed_0")),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }

    if words.len() >= 13
        && word_slice_starts_with(words, &["there", "are"])
        && word_slice_starts_with(&words[3..], &["or", "more", "card"])
        && word_slice_at_is_any(words, 6, &["type", "types"])
        && word_slice_at_is(words, 7, "among")
        && let Some(count) = parse_named_number(words[2])
        && let Some(filter) = permanents_and_your_graveyard_scope(&words[8..])
    {
        return Some(PredicateAst::ValueComparison {
            left: Value::CardTypesAmong(filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }
    None
}

fn parse_life_total_at_least_starting_predicate(words: &[&str]) -> Option<PredicateAst> {
    if matches!(
        words,
        [
            "your", "life", "total", "is", "greater", "than", "or", "equal", "to", "your",
            "starting", "life", "total"
        ]
    ) {
        return Some(PredicateAst::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::StartingLifeTotal(PlayerFilter::You),
        });
    }
    None
}

fn parse_counted_objects_have_counter_predicate(words: &[&str]) -> Option<PredicateAst> {
    if words.len() < 7 {
        return None;
    }
    let count = parse_named_number(words[0])?;
    if words.get(1).copied() != Some("or") || words.get(2).copied() != Some("more") {
        return None;
    }
    let have_idx = find_index(words, |word| matches!(*word, "has" | "have"))?;
    if have_idx <= 3 {
        return None;
    }
    let object_words = &words[3..have_idx];
    let counter_words = &words[have_idx + 1..];
    if object_words.is_empty() || counter_words.is_empty() {
        return None;
    }
    let (counter_constraint, consumed) = parse_filter_counter_constraint_words(counter_words)?;
    if consumed != counter_words.len() {
        return None;
    }

    let object_tokens = object_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let other = object_tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"));
    let mut filter = parse_object_filter(&object_tokens, other).ok()?;
    filter.with_counter = Some(counter_constraint);
    if filter.zone.is_none()
        && filter.card_types.iter().any(|card_type| {
            matches!(
                card_type,
                CardType::Artifact
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
                    | CardType::Battle
            )
        })
    {
        filter.zone = Some(Zone::Battlefield);
    }

    Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    })
}

fn parse_happily_style_conjoined_predicate(words: &[&str]) -> Option<PredicateAst> {
    let cleaned = word_refs_except(words, &[","]);
    let words = cleaned.as_slice();
    let second_there_idx =
        crate::runtime_backend::lexer::word_slice_find_phrase_start(&words[1..], &["there", "are"])
            .map(|idx| idx + 1)?;
    let life_idx = crate::runtime_backend::lexer::word_slice_find_phrase_start(
        &words[second_there_idx + 1..],
        &["and", "your", "life", "total"],
    )
    .map(|idx| idx + second_there_idx + 1)?;

    let first = parse_colors_among_predicate(&words[..second_there_idx])?;
    let second = parse_card_types_among_predicate(&words[second_there_idx..life_idx])?;
    let third = parse_life_total_at_least_starting_predicate(&words[life_idx + 1..])?;

    Some(PredicateAst::And(
        Box::new(PredicateAst::And(Box::new(first), Box::new(second))),
        Box::new(third),
    ))
}

fn parse_revealed_or_controlled_subtype_predicate(words: &[&str]) -> Option<PredicateAst> {
    let suffix_len = usize::from(word_slice_ends_with(
        words,
        &["as", "you", "cast", "this", "spell"],
    )) * 5;
    let core_words = if suffix_len > 0 {
        &words[..words.len().saturating_sub(suffix_len)]
    } else {
        words
    };

    if core_words.len() != 7
        || core_words[0] != "you"
        || core_words[1] != "revealed"
        || parse_subtype_word(core_words[2]).is_none()
        || core_words[3] != "card"
        || core_words[4] != "or"
        || !(core_words[5] == "control" || core_words[5] == "controlled")
        || parse_subtype_word(core_words[6]).is_none()
        || core_words[2] != core_words[6]
    {
        return None;
    }

    Some(PredicateAst::Or(
        Box::new(PredicateAst::ThisSpellPaidLabel("Behold".to_string())),
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: ObjectFilter::default().with_subtype(parse_subtype_word(core_words[2])?),
        }),
    ))
}

fn parse_card_in_your_graveyard_predicate(words: &[&str]) -> Option<PredicateAst> {
    if words.len() < 6 || words[0] != "there" || words[1] != "is" {
        return None;
    }

    let in_idx = crate::runtime_backend::lexer::word_slice_find_word(&words[2..], "in")
        .map(|idx| idx + 2)?;
    if in_idx <= 2 {
        return None;
    }
    if !matches!(
        &words[in_idx..],
        ["in", "your", "graveyard"] | ["in", "graveyard"] | ["in", "the", "graveyard"]
    ) {
        return None;
    }

    let descriptor_tokens = words[2..in_idx]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let mut filter = parse_object_filter(&descriptor_tokens, false).ok()?;
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);

    Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    })
}

pub(crate) fn parse_predicate(tokens: &[OwnedLexToken]) -> Result<PredicateAst, CardTextError> {
    let raw_words_view = GrammarFilterNormalizedWords::new(tokens);
    let raw_words = raw_words_view.to_word_refs();
    let mut filtered = non_article_word_refs(&raw_words);

    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }
    if filtered[0] == "its" || filtered[0] == "it's" {
        filtered[0] = "it";
    }
    if filtered.len() >= 2 && filtered[0] == "it" && filtered[1] == "s" {
        filtered.remove(1);
    }
    if let Some(instead_idx) = word_slice_find_word(&filtered, "instead")
        && instead_idx > 0
    {
        let maybe_predicate = &filtered[..instead_idx];
        let paid_tail = maybe_predicate.len() >= 3
            && word_slice_eq_any(
                &maybe_predicate[maybe_predicate.len() - 3..],
                &[&["cost", "was", "paid"], &["cost", "wasnt", "paid"]],
            );
        let unpaid_tail = maybe_predicate.len() >= 4
            && word_slice_eq(
                &maybe_predicate[maybe_predicate.len() - 4..],
                &["cost", "was", "not", "paid"],
            );
        if paid_tail || unpaid_tail {
            filtered.truncate(instead_idx);
        }
    }

    if let Some(predicate) = parse_repeated_if_or_predicate(&filtered)? {
        return Ok(predicate);
    }
    if let Some(gets_idx) = find_index(&filtered, |word| *word == "gets")
        && gets_idx > 0
        && word_slice_eq(
            &filtered[gets_idx + 1..],
            &["more", "votes", "or", "vote", "is", "tied"],
        )
    {
        return Ok(PredicateAst::VoteOptionGetsMoreVotesOrTied {
            option: filtered[..gets_idx].join(" "),
        });
    }

    if let Some(predicate) = parse_passive_this_way_tagged_object_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_this_ability_resolution_count_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_stack_object_targets_only_source_predicate(&filtered) {
        return Ok(predicate);
    }

    if matches!(
        filtered.as_slice(),
        ["it", "exploited", "that", "creature"] | ["it", "exploited", "that", "object"]
    ) {
        return Ok(PredicateAst::And(
            Box::new(PredicateAst::TaggedMatches(
                TagKey::from(crate::tag::EXPLOITED_TAG),
                ObjectFilter::tagged("triggering"),
            )),
            Box::new(PredicateAst::TaggedMatches(
                TagKey::from(crate::tag::EXPLOITER_TAG),
                ObjectFilter::source(),
            )),
        ));
    }

    for (phrase, zone) in [
        (["this", "is", "in", "your", "hand"].as_slice(), Zone::Hand),
        (
            ["this", "card", "is", "in", "your", "hand"].as_slice(),
            Zone::Hand,
        ),
        (
            ["this", "is", "in", "your", "graveyard"].as_slice(),
            Zone::Graveyard,
        ),
        (
            ["this", "card", "is", "in", "your", "graveyard"].as_slice(),
            Zone::Graveyard,
        ),
        (
            ["this", "creature", "is", "in", "your", "graveyard"].as_slice(),
            Zone::Graveyard,
        ),
        (
            ["this", "permanent", "is", "in", "your", "graveyard"].as_slice(),
            Zone::Graveyard,
        ),
        (
            ["this", "object", "is", "in", "your", "graveyard"].as_slice(),
            Zone::Graveyard,
        ),
        (
            ["this", "is", "in", "your", "library"].as_slice(),
            Zone::Library,
        ),
        (
            ["this", "card", "is", "in", "your", "library"].as_slice(),
            Zone::Library,
        ),
        (["this", "is", "in", "exile"].as_slice(), Zone::Exile),
        (
            ["this", "card", "is", "in", "exile"].as_slice(),
            Zone::Exile,
        ),
        (
            ["this", "is", "in", "the", "command", "zone"].as_slice(),
            Zone::Command,
        ),
        (
            ["this", "card", "is", "in", "the", "command", "zone"].as_slice(),
            Zone::Command,
        ),
    ] {
        if word_slice_eq(&filtered, phrase) {
            return Ok(PredicateAst::SourceIsInZone(zone));
        }
    }

    if let Some(predicate) = parse_source_exiled_with_counter_predicate(&raw_words, tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_happily_style_conjoined_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_revealed_or_controlled_subtype_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_graveyard_threshold_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_in_your_graveyard_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_colors_among_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_types_among_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_life_total_at_least_starting_predicate(&filtered) {
        return Ok(predicate);
    }

    if matches!(
        filtered.as_slice(),
        ["you", "have", "max", "speed"] | ["you", "have", "maximum", "speed"]
    ) {
        return Ok(PredicateAst::ValueComparison {
            left: Value::Speed(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(4),
        });
    }

    if let Some(predicate) = parse_counted_objects_have_counter_predicate(&filtered) {
        return Ok(predicate);
    }

    if filtered.len() == 6
        && filtered[0] == "you"
        && filtered[1] == "have"
        && filtered[3] == "or"
        && filtered[4] == "less"
        && filtered[5] == "life"
        && let Some(amount) = filtered[2]
            .parse::<i32>()
            .ok()
            .or_else(|| parse_named_number(filtered[2]).map(|n| n as i32))
    {
        return Ok(PredicateAst::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(amount),
        });
    }
    if filtered.len() == 7
        && filtered[0] == "your"
        && filtered[1] == "life"
        && filtered[2] == "total"
        && filtered[3] == "is"
        && filtered[5] == "or"
        && filtered[6] == "less"
        && let Some(amount) = filtered[4]
            .parse::<i32>()
            .ok()
            .or_else(|| parse_named_number(filtered[4]).map(|n| n as i32))
    {
        return Ok(PredicateAst::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(amount),
        });
    }

    if let Some(has_idx) = find_index(&filtered, |word| *word == "has" || *word == "have")
        && has_idx > 0
        && has_idx + 1 < filtered.len()
        && word_slice_contains_word(&filtered[..has_idx], "control")
        && let Some((constraint, consumed)) =
            parse_filter_keyword_constraint_words(&filtered[has_idx + 1..])
        && has_idx + 1 + consumed == filtered.len()
    {
        let mut subject_words = filtered[..has_idx].to_vec();
        subject_words.retain(|word| *word != "you" && *word != "control" && *word != "controls");
        let subject_tokens = subject_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut filter = parse_object_filter(&subject_tokens, false)?;
        apply_filter_keyword_constraint(&mut filter, constraint, false);
        filter.controller = Some(PlayerFilter::You);
        return Ok(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        });
    }

    if let Some(has_idx) = find_index(&filtered, |word| *word == "has" || *word == "have")
        && has_idx > 0
        && has_idx + 1 < filtered.len()
        && (word_slice_contains_word(&filtered[..has_idx], "graveyard")
            || word_slice_contains_word(&filtered[..has_idx], "hand")
            || word_slice_contains_word(&filtered[..has_idx], "exile")
            || word_slice_contains_word(&filtered[..has_idx], "library"))
        && let Some((constraint, consumed)) =
            parse_filter_keyword_constraint_words(&filtered[has_idx + 1..])
        && has_idx + 1 + consumed == filtered.len()
    {
        let subject_tokens = filtered[..has_idx]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut filter = parse_object_filter(&subject_tokens, false)?;
        apply_filter_keyword_constraint(&mut filter, constraint, false);
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        return Ok(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        });
    }

    if matches!(
        filtered.as_slice(),
        ["an", "opponent", "controls", "it"]
            | ["an", "opponent", "controls", "that", "creature"]
            | ["an", "opponent", "controls", "that", "permanent"]
            | ["opponent", "controls", "it"]
            | ["opponent", "controls", "that", "creature"]
            | ["opponent", "controls", "that", "permanent"]
    ) {
        let mut filter = ObjectFilter {
            controller: Some(PlayerFilter::Opponent),
            ..Default::default()
        };
        if word_slice_last_is(&filtered, "creature") {
            filter.card_types.push(CardType::Creature);
        }
        return Ok(PredicateAst::ItMatches(filter));
    }

    if filtered.len() >= 3 && filtered[0] == "opponent" && filtered[1] == "controls" {
        let control_tokens = filtered[2..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::Opponent);
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::Opponent,
                filter,
            });
        }
    }

    if raw_words.len() >= 4
        && raw_words[0] == "an"
        && raw_words[1] == "opponent"
        && raw_words[2] == "controls"
    {
        let control_tokens = raw_words[3..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::Opponent);
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::Opponent,
                filter,
            });
        }
    }

    if let Some(gets_idx) = find_index(&filtered, |word| *word == "gets")
        && gets_idx > 0
        && word_slice_eq(&filtered[gets_idx + 1..], &["more", "votes"])
    {
        return Ok(PredicateAst::VoteOptionGetsMoreVotes {
            option: filtered[..gets_idx].join(" "),
        });
    }

    if filtered.len() >= 4
        && filtered[0] == "no"
        && word_slice_eq(&filtered[filtered.len() - 2..], &["got", "votes"])
    {
        let filter_tokens = filtered[1..filtered.len() - 2]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::NoVoteObjectsMatched { filter });
    }

    if let Some(attacking_idx) = crate::runtime_backend::lexer::word_slice_find_phrase_start(
        &filtered,
        &[
            "are",
            "attacking",
            "and",
            "you",
            "both",
            "own",
            "and",
            "control",
            "them",
        ],
    ) && let Some(and_idx) = find_meld_subject_split(&filtered[..attacking_idx])
    {
        let left_words = &filtered[..and_idx];
        let right_words = &filtered[and_idx + 1..attacking_idx];
        if !left_words.is_empty() && !right_words.is_empty() {
            let mut left_filter = parse_meld_subject_filter(left_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate subject (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            left_filter.controller = Some(PlayerFilter::You);
            left_filter.attacking = true;

            let mut right_filter = parse_meld_subject_filter(right_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate tail (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            right_filter.controller = Some(PlayerFilter::You);
            right_filter.attacking = true;

            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if filtered.len() >= 8
        && filtered[0] == "you"
        && filtered[1] == "both"
        && filtered[2] == "own"
        && filtered[3] == "and"
        && (filtered[4] == "control" || filtered[4] == "controls")
        && let Some(and_idx) = find_meld_subject_split(&filtered[5..])
    {
        let and_idx = 5 + and_idx;
        if and_idx > 5 && and_idx + 1 < filtered.len() {
            let mut left_filter =
                parse_meld_subject_filter(&filtered[5..and_idx]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate subject (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            left_filter.controller = Some(PlayerFilter::You);
            let mut right_filter =
                parse_meld_subject_filter(&filtered[and_idx + 1..]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            right_filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if let Some(and_idx) = find_index(&filtered, |word| *word == "and")
        && and_idx > 0
        && and_idx + 1 < filtered.len()
    {
        let right_first = filtered.get(and_idx + 1).copied();
        if matches!(right_first, Some("have") | Some("you")) {
            let left_words = &filtered[..and_idx];
            let mut right_words = filtered[and_idx + 1..].to_vec();
            if word_slice_first_is(&right_words, "have") {
                right_words.insert(0, "you");
            }
            let left_tokens = left_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let right_tokens = right_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let left = parse_predicate(&left_tokens)?;
            let right = parse_predicate(&right_tokens)?;
            return Ok(PredicateAst::And(Box::new(left), Box::new(right)));
        }
    }

    if let Some(while_idx) = find_index(&filtered, |word| *word == "while")
        && while_idx > 0
        && while_idx + 1 < filtered.len()
    {
        let left_tokens = filtered[..while_idx]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let right_tokens = filtered[while_idx + 1..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let left = parse_predicate(&left_tokens)?;
        let right = parse_predicate(&right_tokens)?;
        if matches!(
            left,
            PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
                | PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(_)
        ) {
            return Err(CardTextError::ParseError(format!(
                "unsupported mana-spent predicate tail (predicate: '{}')",
                filtered.join(" ")
            )));
        }
        return Ok(PredicateAst::And(Box::new(left), Box::new(right)));
    }

    if word_slice_eq_any(&filtered, &[&["this", "tapped"], &["thiss", "tapped"]])
        || (word_slice_first_is_any(&filtered, &["this", "thiss"])
            && word_slice_last_is(&filtered, "tapped"))
    {
        return Ok(PredicateAst::SourceIsTapped);
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["this", "untapped"],
            &["thiss", "untapped"],
            &["this", "is", "untapped"],
            &["this", "creature", "is", "untapped"],
            &["this", "permanent", "is", "untapped"],
        ],
    ) || (word_slice_first_is_any(&filtered, &["this", "thiss"])
        && word_slice_last_is(&filtered, "untapped"))
    {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)));
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["this", "creature", "isnt", "saddled"],
            &["this", "permanent", "isnt", "saddled"],
            &["this", "isnt", "saddled"],
            &["it", "isnt", "saddled"],
        ],
    ) {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled)));
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["this", "creature", "is", "saddled"],
            &["this", "permanent", "is", "saddled"],
            &["this", "is", "saddled"],
            &["it", "is", "saddled"],
        ],
    ) {
        return Ok(PredicateAst::SourceIsSaddled);
    }

    if let Some(is_idx) = find_index(&filtered, |word| matches!(*word, "is" | "are")) {
        let subject_words = &filtered[..is_idx];
        let is_source_subject = is_source_reference_words(subject_words)
            || word_slice_eq_any(subject_words, &[&["it"], &["its"]]);
        if is_source_subject
            && word_slice_starts_with(&filtered[is_idx + 1..], &["enchanted", "by"])
        {
            let attachment_tokens = filtered[is_idx + 3..]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let (comparison, used) = parse_attachment_quantity_prefix(&attachment_tokens)?;
            let filter_tokens = &attachment_tokens[used..];
            if !filter_tokens.is_empty() {
                let filter = parse_object_filter(filter_tokens, false).or_else(|_| {
                    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
                    if word_slice_eq_any(&filter_words, &[&["aura"], &["auras"]]) {
                        Ok(ObjectFilter::default().with_subtype(Subtype::Aura))
                    } else {
                        Err(CardTextError::ParseError(format!(
                            "unsupported attachment-count predicate tail (predicate: '{}')",
                            filtered.join(" ")
                        )))
                    }
                })?;
                return Ok(PredicateAst::SourceHasAttachmentsMatching {
                    filter,
                    comparison,
                    display: filtered.join(" "),
                });
            }
        }
    }

    let source_filter_predicate = {
        let predicate_idx = find_index(&filtered, |word| {
            matches!(*word, "is" | "are" | "isnt" | "isn't" | "arent" | "aren't")
        });
        predicate_idx.and_then(|idx| {
            let subject_words = &filtered[..idx];
            let is_source_subject = is_source_reference_words(subject_words);
            if !is_source_subject {
                return None;
            }

            let mut negative = matches!(filtered[idx], "isnt" | "isn't" | "arent" | "aren't");
            let mut tail_start = idx + 1;
            if word_slice_at_is(&filtered, tail_start, "not") {
                negative = true;
                tail_start += 1;
            }
            let descriptor_words = &filtered[tail_start..];
            if descriptor_words.is_empty()
                || descriptor_words
                    .iter()
                    .any(|word| matches!(*word, "attached" | "tapped" | "untapped" | "saddled"))
            {
                return None;
            }

            let descriptor_tokens = descriptor_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let Ok(filter) = parse_object_filter(&descriptor_tokens, false) else {
                return None;
            };
            let has_identity = !filter.card_types.is_empty()
                || !filter.all_card_types.is_empty()
                || !filter.subtypes.is_empty()
                || !filter.supertypes.is_empty()
                || filter.colors.is_some()
                || filter.token
                || filter.nontoken
                || !filter.excluded_card_types.is_empty()
                || !filter.excluded_subtypes.is_empty();
            has_identity.then_some((filter, negative))
        })
    };
    if let Some((filter, negative)) = source_filter_predicate {
        let predicate = PredicateAst::SourceMatches(filter);
        return Ok(if negative {
            PredicateAst::Not(Box::new(predicate))
        } else {
            predicate
        });
    }

    if let Some(has_idx) = find_index(&filtered, |word| *word == "has" || *word == "have")
        && has_idx > 0
        && has_idx + 1 < filtered.len()
    {
        let subject_words = &filtered[..has_idx];
        let is_source_subject = is_source_reference_words(subject_words)
            || word_slice_eq_any(subject_words, &[&["it"], &["its"]]);
        if is_source_subject
            && let Some((constraint, consumed)) =
                parse_filter_keyword_constraint_words(&filtered[has_idx + 1..])
            && has_idx + 1 + consumed == filtered.len()
        {
            let mut filter = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut filter, constraint, false);
            return Ok(PredicateAst::SourceMatches(filter));
        }
    }

    if matches!(
        filtered.as_slice(),
        [
            "this", "creature", "didnt", "attack", "or", "come", "under", "your", "control",
            "this", "turn"
        ] | [
            "this", "creature", "didnt", "attack", "or", "came", "under", "your", "control",
            "this", "turn"
        ]
    ) {
        return Ok(PredicateAst::And(
            Box::new(PredicateAst::Not(Box::new(
                PredicateAst::SourceAttackedThisTurn,
            ))),
            Box::new(PredicateAst::Not(Box::new(
                PredicateAst::SourceCameUnderYourControlThisTurn,
            ))),
        ));
    }

    if word_slice_starts_with(&filtered, &["there", "are", "no"])
        && word_slice_contains_word(&filtered, "counters")
        && word_slice_contains_any_phrase(
            &filtered,
            &[&["on", "this"], &["on", "it"], &["on", "them"]],
        )
        && let Some(counters_idx) =
            find_index(&raw_words, |word| *word == "counter" || *word == "counters")
        && counters_idx >= 4
        && let Some(counter_type) = parse_counter_type_from_tokens(&tokens[..=counters_idx])
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    let source_has_counter_prefix_len = if word_slice_starts_with(&raw_words, &["this", "has"]) {
        Some(2)
    } else if raw_words.len() >= 3
        && raw_words[0] == "this"
        && matches!(
            raw_words[1],
            "creature"
                | "permanent"
                | "artifact"
                | "enchantment"
                | "land"
                | "planeswalker"
                | "battle"
        )
        && raw_words[2] == "has"
    {
        Some(3)
    } else {
        None
    };
    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && raw_words[prefix_len] == "no"
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len + 1])
        && matches!(raw_words[prefix_len + 2], "counter" | "counters")
        && raw_words[prefix_len + 3] == "on"
        && matches!(
            raw_words.get(prefix_len + 4).copied(),
            Some("it" | "him" | "her" | "them" | "this" | "that")
        )
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && !word_slice_starts_with(&raw_words[prefix_len + 1..], &["or", "more"])
        && let Some(counter_idx) = find_index(&raw_words[prefix_len..], |word| {
            *word == "counter" || *word == "counters"
        })
        && counter_idx > 0
        && let Some(counter_type) =
            parse_counter_type_from_tokens(&tokens[prefix_len..=prefix_len + counter_idx])
        && word_slice_at_is(&raw_words, prefix_len + counter_idx + 1, "on")
        && matches!(
            raw_words.get(prefix_len + counter_idx + 2).copied(),
            Some("it" | "him" | "her" | "them" | "this" | "that")
        )
    {
        return Ok(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count: 1,
        });
    }

    let triggering_object_had_no_counter_prefix_len =
        if word_slice_starts_with(&raw_words, &["it", "had", "no"]) {
            Some(3)
        } else if word_slice_starts_with_any(
            &raw_words,
            &[
                &["this", "creature", "had", "no"],
                &["that", "creature", "had", "no"],
                &["this", "permanent", "had", "no"],
                &["that", "permanent", "had", "no"],
            ],
        ) {
            Some(4)
        } else {
            None
        };
    if let Some(prefix_len) = triggering_object_had_no_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len])
        && matches!(raw_words[prefix_len + 1], "counter" | "counters")
        && raw_words[prefix_len + 2] == "on"
        && matches!(
            raw_words[prefix_len + 3],
            "it" | "them" | "this" | "that" | "itself"
        )
    {
        return Ok(PredicateAst::TriggeringObjectHadNoCounter(counter_type));
    }

    let triggering_object_had_counter_prefix_len =
        if word_slice_starts_with(&raw_words, &["it", "had"]) {
            Some(2)
        } else if word_slice_starts_with_any(
            &raw_words,
            &[
                &["this", "creature", "had"],
                &["that", "creature", "had"],
                &["this", "permanent", "had"],
                &["that", "permanent", "had"],
            ],
        ) {
            Some(3)
        } else {
            None
        };
    if let Some(prefix_len) = triggering_object_had_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && let Some(counter_idx) = find_index(&raw_words[prefix_len..], |word| {
            *word == "counter" || *word == "counters"
        })
        && counter_idx > 0
        && let Some(counter_type) =
            parse_counter_type_from_tokens(&tokens[prefix_len..=prefix_len + counter_idx])
        && word_slice_at_is(&raw_words, prefix_len + counter_idx + 1, "on")
        && matches!(
            raw_words
                .get(prefix_len + counter_idx + 2)
                .copied()
                .unwrap_or(""),
            "it" | "them" | "this" | "that" | "itself"
        )
    {
        return Ok(PredicateAst::TriggeringObjectHadCounterAtLeast {
            counter_type,
            count: 1,
        });
    }

    if word_slice_starts_with(&raw_words, &["there", "are"])
        && word_slice_starts_with(&raw_words[3..], &["or", "more"])
        && raw_words
            .iter()
            .any(|w| *w == "counter" || *w == "counters")
    {
        if let Some((count, used)) = parse_number(&tokens[2..]) {
            let rest = &tokens[2 + used..];
            let rest_words = crate::runtime_backend::token_word_refs(rest);
            if let Some(counter_idx) = find_index(rest_words.as_slice(), |word| {
                *word == "counter" || *word == "counters"
            }) && rest_words.len() >= 4
                && word_slice_starts_with(&rest_words, &["or", "more"])
            {
                let consumed_source_tail = matches!(
                    &rest_words[counter_idx + 1..],
                    ["on", "it"]
                        | ["on", "this"]
                        | ["on", "this", "artifact"]
                        | ["on", "this", "creature"]
                        | ["on", "this", "enchantment"]
                        | ["on", "this", "land"]
                        | ["on", "this", "permanent"]
                );
                if counter_idx == 2 && consumed_source_tail {
                    return Ok(PredicateAst::SourceHasCountersAtLeast(count));
                }
                if counter_idx > 2
                    && let Some(counter_type) =
                        parse_counter_type_from_tokens(&rest[2..=counter_idx])
                    && consumed_source_tail
                {
                    return Ok(PredicateAst::SourceHasCounterAtLeast {
                        counter_type,
                        count,
                    });
                }
            }
        }
    }

    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 6
        && let Some(count) = parse_named_number(raw_words[prefix_len])
        && word_slice_starts_with(&raw_words[prefix_len + 1..], &["or", "more"])
        && let Some(counter_idx) = find_index(&raw_words[prefix_len + 3..], |word| {
            *word == "counter" || *word == "counters"
        })
        && counter_idx > 0
        && let Some(counter_type) =
            parse_counter_type_from_tokens(&tokens[prefix_len + 3..=prefix_len + 3 + counter_idx])
        && word_slice_at_is(&raw_words, prefix_len + 4 + counter_idx, "on")
        && matches!(
            raw_words.get(prefix_len + 5 + counter_idx).copied(),
            Some("it" | "him" | "her" | "them" | "this" | "that")
        )
    {
        return Ok(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        });
    }

    if filtered.len() == 7
        && matches!(
            &filtered[..4],
            ["this", "creature", "power", "is"]
                | ["this", "creatures", "power", "is"]
                | ["this", "permanent", "power", "is"]
                | ["this", "permanents", "power", "is"]
        )
        && filtered[5] == "or"
        && filtered[6] == "more"
        && let Some(count_word) = filtered.get(4).copied()
        && let Some(count) = parse_named_number(count_word)
    {
        return Ok(PredicateAst::SourcePowerAtLeast(count));
    }

    if filtered.len() == 6
        && filtered[0] == "this"
        && filtered[1] == "has"
        && filtered[2] == "power"
        && filtered[4] == "or"
        && matches!(filtered[5], "greater" | "more")
        && let Some(count_word) = filtered.get(3).copied()
        && let Some(count) = count_word
            .parse::<u32>()
            .ok()
            .or_else(|| parse_named_number(count_word))
    {
        return Ok(PredicateAst::SourcePowerAtLeast(count));
    }

    if filtered.len() >= 10 && filtered[0] == "there" && filtered[1] == "are" {
        let mut idx = 2usize;
        if let Some(count) = parse_named_number(filtered[idx]) {
            idx += 1;
            if word_slice_starts_with(&filtered[idx..], &["or", "more"]) {
                idx += 2;
            }
            let looks_like_basic_land_type_clause =
                word_slice_starts_with(&filtered[idx..], &["basic", "land"])
                    && word_slice_at_is_any(&filtered, idx + 2, &["type", "types"])
                    && word_slice_at_is(&filtered, idx + 3, "among")
                    && word_slice_at_is_any(&filtered, idx + 4, &["land", "lands"]);
            if looks_like_basic_land_type_clause {
                let tail = &filtered[idx + 5..];
                let player = if word_slice_eq_any(
                    tail,
                    &[
                        &["that", "player", "controls"],
                        &["that", "player", "control"],
                        &["that", "players", "controls"],
                    ],
                ) {
                    PlayerAst::That
                } else if word_slice_eq_any(tail, &[&["you", "control"], &["you", "controls"]]) {
                    PlayerAst::You
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported basic-land-types predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    )));
                };

                return Ok(PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                    player,
                    count,
                });
            }
        }
    }

    if filtered.len() >= 7
        && filtered[0] == "there"
        && filtered[1] == "are"
        && let Some(count) = parse_named_number(filtered[2])
    {
        let mut idx = 3usize;
        if word_slice_starts_with(&filtered[idx..], &["or", "more"]) {
            idx += 2;
        }

        let battlefield_suffix_len =
            if word_slice_ends_with(&filtered[idx..], &["on", "the", "battlefield"]) {
                Some(3usize)
            } else if word_slice_ends_with(&filtered[idx..], &["on", "battlefield"]) {
                Some(2usize)
            } else {
                None
            };
        if let Some(battlefield_suffix_len) = battlefield_suffix_len {
            let raw_filter_words = &filtered[idx..filtered.len() - battlefield_suffix_len];
            let other = raw_filter_words
                .first()
                .is_some_and(|word| matches!(*word, "other" | "another"));
            let filter_words = if other {
                &raw_filter_words[1..]
            } else {
                raw_filter_words
            };
            if !filter_words.is_empty() {
                let filter_tokens = filter_words
                    .iter()
                    .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                    .collect::<Vec<_>>();
                if let Ok(mut filter) = parse_object_filter(&filter_tokens, other) {
                    filter.zone = Some(Zone::Battlefield);

                    return Ok(PredicateAst::ValueComparison {
                        left: Value::Count(filter),
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(count as i32),
                    });
                }
            }
        }
    }

    let parse_graveyard_card_types_subject = |words: &[&str]| -> Option<PlayerAst> {
        match words {
            [first, second] if *first == "your" && *second == "graveyard" => Some(PlayerAst::You),
            [first, second, third]
                if *first == "that"
                    && (*second == "player" || *second == "players")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::That)
            }
            [first, second, third]
                if *first == "target"
                    && (*second == "player" || *second == "players")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::Target)
            }
            [first, second, third]
                if *first == "target"
                    && (*second == "opponent" || *second == "opponents")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::TargetOpponent)
            }
            [first, second]
                if (*first == "opponent" || *first == "opponents") && *second == "graveyard" =>
            {
                Some(PlayerAst::Opponent)
            }
            _ => None,
        }
    };
    if filtered.len() >= 11 {
        let (count_idx, subject_start, constrained_player) =
            if filtered[0] == "there" && filtered[1] == "are" {
                (2usize, 10usize, None)
            } else if filtered[0] == "you" && filtered[1] == "have" {
                (2usize, 10usize, Some(PlayerAst::You))
            } else {
                (usize::MAX, usize::MAX, None)
            };
        if count_idx != usize::MAX
            && word_slice_starts_with(&filtered[count_idx + 1..], &["or", "more", "card"])
            && word_slice_at_is_any(&filtered, count_idx + 4, &["type", "types"])
            && word_slice_at_is(&filtered, count_idx + 5, "among")
            && word_slice_at_is_any(&filtered, count_idx + 6, &["card", "cards"])
            && word_slice_at_is(&filtered, count_idx + 7, "in")
            && subject_start <= filtered.len()
            && let Some(count) = parse_named_number(filtered[count_idx])
            && let Some(player) = parse_graveyard_card_types_subject(&filtered[subject_start..])
            && constrained_player.map_or(true, |expected| expected == player)
        {
            return Ok(PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count });
        }
    }

    let parse_comparison_player_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        match words {
            [first, second, ..] if *first == "that" && *second == "player" => {
                Some((PlayerAst::That, 2))
            }
            [first, second, ..] if *first == "target" && *second == "player" => {
                Some((PlayerAst::Target, 2))
            }
            [first, second, ..] if *first == "target" && *second == "opponent" => {
                Some((PlayerAst::TargetOpponent, 2))
            }
            [first, second, ..] if *first == "each" && *second == "opponent" => {
                Some((PlayerAst::Opponent, 2))
            }
            [first, second, ..] if (*first == "a" || *first == "any") && *second == "player" => {
                Some((PlayerAst::Any, 2))
            }
            [first, second, ..] if *first == "defending" && *second == "player" => {
                Some((PlayerAst::Defending, 2))
            }
            [first, second, ..] if *first == "attacking" && *second == "player" => {
                Some((PlayerAst::Attacking, 2))
            }
            [first, ..] if *first == "you" => Some((PlayerAst::You, 1)),
            [first, ..] if *first == "opponent" || *first == "opponents" => {
                Some((PlayerAst::Opponent, 1))
            }
            [first, second, ..] if *first == "player" && *second == "who" => {
                Some((PlayerAst::That, 1))
            }
            [first, ..] if *first == "player" => Some((PlayerAst::Any, 1)),
            _ => None,
        }
    };
    let parse_life_total_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        match words {
            ["your", "life", "total", ..] => Some((PlayerAst::You, 3)),
            ["their", "life", "total", ..] => Some((PlayerAst::That, 3)),
            ["that", "players", "life", "total", ..] => Some((PlayerAst::That, 4)),
            ["target", "players", "life", "total", ..] => Some((PlayerAst::Target, 4)),
            ["target", "opponents", "life", "total", ..] => Some((PlayerAst::TargetOpponent, 4)),
            ["opponents", "life", "total", ..] | ["opponent", "life", "total", ..] => {
                Some((PlayerAst::Opponent, 3))
            }
            ["defending", "players", "life", "total", ..] => Some((PlayerAst::Defending, 4)),
            ["attacking", "players", "life", "total", ..] => Some((PlayerAst::Attacking, 4)),
            _ => None,
        }
    };
    let half_starting_tail_matches = |tail: &[&str]| {
        matches!(
            tail,
            ["half", "your", "starting", "life", "total"]
                | ["half", "their", "starting", "life", "total"]
                | ["half", "that", "players", "starting", "life", "total"]
                | ["half", "target", "players", "starting", "life", "total"]
                | ["half", "target", "opponents", "starting", "life", "total"]
                | ["half", "opponents", "starting", "life", "total"]
                | ["half", "defending", "players", "starting", "life", "total"]
                | ["half", "attacking", "players", "starting", "life", "total"]
        )
    };
    if let Some((player, subject_len)) = parse_life_total_subject(&filtered)
        && word_slice_at_is(&filtered, subject_len, "is")
    {
        let tail = &filtered[subject_len + 1..];
        if let Some(rest) = slice_strip_prefix(tail, &["less", "than", "or", "equal", "to"])
            && half_starting_tail_matches(rest)
        {
            return Ok(PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player });
        }
        if let Some(rest) = slice_strip_prefix(tail, &["less", "than"])
            && half_starting_tail_matches(rest)
        {
            return Ok(PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player });
        }
    }
    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && word_slice_at_is_any(&filtered, subject_len, &["has", "have"])
        && let Some(count_word) = filtered.get(subject_len + 1).copied()
        && let Some(count) = count_word
            .parse::<i32>()
            .ok()
            .or_else(|| parse_named_number(count_word).map(|n| n as i32))
        && word_slice_at_is(&filtered, subject_len + 2, "or")
        && word_slice_at_is(&filtered, subject_len + 3, "more")
        && word_slice_at_is_any(&filtered, subject_len + 4, &["card", "cards"])
        && word_slice_at_is(&filtered, subject_len + 5, "in")
        && let Some(possessive) = filtered.get(subject_len + 6).copied()
        && graveyard_possessive_matches_subject(player, possessive)
        && word_slice_at_is(&filtered, subject_len + 7, "graveyard")
        && filtered.len() == subject_len + 8
        && let Some(player_filter) = player_filter_for_turn_value(player)
    {
        return Ok(PredicateAst::ValueComparison {
            left: Value::CardsInGraveyard(player_filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count),
        });
    }
    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && word_slice_at_is_any(&filtered, subject_len, &["control", "controls"])
        && word_slice_at_is(&filtered, subject_len + 1, "more")
        && let Some(than_offset) = find_index(&filtered[subject_len + 2..], |word| *word == "than")
    {
        let than_idx = subject_len + 2 + than_offset;
        let tail = &filtered[than_idx..];
        if word_slice_eq_any(tail, &[&["than", "you"], &["than", "you", "do"]]) {
            let filter_tokens = filtered[subject_len + 2..than_idx]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if !filter_tokens.is_empty() {
                let other = filter_tokens
                    .first()
                    .is_some_and(|token| token.is_word("another") || token.is_word("other"));
                if let Ok(filter) = parse_object_filter(&filter_tokens, other)
                    && filter != ObjectFilter::default()
                {
                    return Ok(PredicateAst::PlayerControlsMoreThanYou { player, filter });
                }
            }
        }
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && word_slice_at_is(&filtered, subject_len, "has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "life", "than", "you"] | ["more", "life", "than", "you", "do"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreLifeThanYou { player });
    }

    if word_slice_first_is(&filtered, "you")
        && word_slice_at_is_any(&filtered, 1, &["have", "has"])
        && word_slice_starts_with(&filtered[2..], &["more", "life", "than"])
        && word_slice_at_is_any(&filtered, 5, &["opponent", "opponents"])
    {
        if matches!(
            raw_words.as_slice(),
            [
                "you",
                "have" | "has",
                "more",
                "life",
                "than",
                "each",
                "opponent" | "opponents"
            ]
        ) {
            return Ok(PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
                player: PlayerAst::You,
            });
        }
        return Ok(PredicateAst::PlayerHasLessLifeThanYou {
            player: PlayerAst::Opponent,
        });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && word_slice_at_is_any(&filtered, subject_len, &["has", "have"])
        && filtered.len() == subject_len + 5
        && word_slice_at_is(&filtered, subject_len + 2, "or")
        && word_slice_at_is_any(&filtered, subject_len + 3, &["less", "fewer"])
        && word_slice_at_is(&filtered, subject_len + 4, "life")
        && let Some(amount) = filtered[subject_len + 1]
            .parse::<i32>()
            .ok()
            .or_else(|| parse_named_number(filtered[subject_len + 1]).map(|n| n as i32))
    {
        let player_filter = match player {
            PlayerAst::You => Some(PlayerFilter::You),
            PlayerAst::Opponent => Some(PlayerFilter::Opponent),
            PlayerAst::Any => Some(PlayerFilter::Any),
            PlayerAst::Defending => Some(PlayerFilter::Defending),
            PlayerAst::Attacking => Some(PlayerFilter::Attacking),
            _ => None,
        };
        if let Some(player_filter) = player_filter {
            return Ok(PredicateAst::ValueComparison {
                left: crate::effect::Value::LifeTotal(player_filter),
                operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                right: crate::effect::Value::Fixed(amount),
            });
        }
    }

    if filtered.len() >= 8
        && filtered[0] == "no"
        && matches!(filtered[1], "opponent" | "opponents")
        && filtered[2] == "has"
        && filtered[3] == "more"
        && filtered[4] == "life"
        && filtered[5] == "than"
        && let Some((player, subject_len)) = parse_comparison_player_subject(&filtered[6..])
        && subject_len + 6 == filtered.len()
    {
        return Ok(PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && word_slice_at_is(&filtered, subject_len, "has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "life", "than", "each", "other", "player"]
                | ["more", "life", "than", "each", "other", "players"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && word_slice_at_is(&filtered, subject_len, "has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "card", "in", "hand", "than", "you"]
                | ["more", "cards", "in", "hand", "than", "you"]
                | ["more", "card", "in", "their", "hand", "than", "you"]
                | ["more", "cards", "in", "their", "hand", "than", "you"]
                | ["more", "card", "in", "hand", "than", "you", "do"]
                | ["more", "cards", "in", "hand", "than", "you", "do"]
                | ["more", "card", "in", "their", "hand", "than", "you", "do"]
                | ["more", "cards", "in", "their", "hand", "than", "you", "do"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreCardsInHandThanYou { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && word_slice_at_is(&filtered, subject_len, "has")
        && matches!(
            &filtered[subject_len + 1..],
            [
                "more", "card", "in", "hand", "than", "each", "other", "player"
            ] | [
                "more", "cards", "in", "hand", "than", "each", "other", "player"
            ] | [
                "more", "card", "in", "their", "hand", "than", "each", "other", "player",
            ] | [
                "more", "cards", "in", "their", "hand", "than", "each", "other", "player",
            ]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && word_slice_at_is_any(&filtered, subject_len, &["has", "have"])
        && let Some(count_word) = filtered.get(subject_len + 1).copied()
        && let Some(count) = parse_named_number(count_word)
        && word_slice_at_is(&filtered, subject_len + 2, "or")
        && let Some(comp_word) = filtered.get(subject_len + 3).copied()
        && matches!(comp_word, "more" | "fewer" | "less")
        && word_slice_at_is_any(&filtered, subject_len + 4, &["card", "cards"])
        && word_slice_at_is(&filtered, subject_len + 5, "in")
        && word_slice_at_is(&filtered, subject_len + 6, "hand")
        && filtered.len() == subject_len + 7
    {
        return Ok(if comp_word == "more" {
            PredicateAst::PlayerCardsInHandOrMore { player, count }
        } else {
            PredicateAst::PlayerCardsInHandOrFewer { player, count }
        });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered) {
        let draw_count_idx = if word_slice_at_is(&filtered, subject_len, "drew") {
            Some(subject_len + 1)
        } else if matches!(
            filtered.get(subject_len..subject_len + 2),
            Some(["has", "drawn"] | ["have", "drawn"])
        ) {
            Some(subject_len + 2)
        } else {
            None
        };
        if let Some(count_idx) = draw_count_idx
            && let Some(count_word) = filtered.get(count_idx).copied()
            && let Some(count) = count_word
                .parse::<i32>()
                .ok()
                .or_else(|| parse_named_number(count_word).map(|n| n as i32))
            && word_slice_at_is(&filtered, count_idx + 1, "or")
            && word_slice_at_is(&filtered, count_idx + 2, "more")
            && word_slice_at_is_any(&filtered, count_idx + 3, &["card", "cards"])
            && word_slice_starts_with(&filtered[count_idx + 4..], &["this", "turn"])
            && filtered.len() == count_idx + 6
            && let Some(player_filter) = player_filter_for_turn_value(player)
        {
            return Ok(PredicateAst::ValueComparison {
                left: Value::MaxCardsDrawnThisTurn(player_filter),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(count),
            });
        }
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && word_slice_at_is(&filtered, subject_len, "had")
        && let Some(count_word) = filtered.get(subject_len + 1).copied()
        && let Some(count) = count_word
            .parse::<i32>()
            .ok()
            .or_else(|| parse_named_number(count_word).map(|n| n as i32))
        && word_slice_at_is(&filtered, subject_len + 2, "or")
        && word_slice_at_is(&filtered, subject_len + 3, "more")
        && word_slice_at_is_any(&filtered, subject_len + 4, &["land", "lands"])
        && word_slice_at_is_any(&filtered, subject_len + 5, &["enter", "entered"])
        && word_slice_at_is(&filtered, subject_len + 6, "battlefield")
        && word_slice_at_is(&filtered, subject_len + 7, "under")
        && word_slice_at_is_any(
            &filtered,
            subject_len + 8,
            &["your", "their", "that", "its"],
        )
        && word_slice_at_is(&filtered, subject_len + 9, "control")
        && word_slice_starts_with(&filtered[subject_len + 10..], &["this", "turn"])
        && filtered.len() == subject_len + 12
        && let Some(player_filter) = player_filter_for_turn_value(player)
    {
        return Ok(PredicateAst::ValueComparison {
            left: Value::LandsEnteredBattlefieldThisTurn(player_filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count),
        });
    }

    if word_slice_eq(&filtered, &["you", "have", "no", "cards", "in", "hand"]) {
        return Ok(PredicateAst::YouHaveNoCardsInHand);
    }

    if matches!(
        filtered.as_slice(),
        ["you", "would", "draw", "a", "card"]
            | ["you", "would", "draw", "card"]
            | ["an", "opponent", "would", "draw", "a", "card"]
            | ["an", "opponent", "would", "draw", "card"]
            | ["opponent", "would", "draw", "a", "card"]
            | ["opponent", "would", "draw", "card"]
    ) {
        let player = if filtered[0] == "you" {
            PlayerAst::You
        } else {
            PlayerAst::Opponent
        };
        return Ok(PredicateAst::PlayerWouldDrawCard { player });
    }

    if matches!(
        filtered.as_slice(),
        ["you", "would", "proliferate"]
            | ["an", "opponent", "would", "proliferate"]
            | ["opponent", "would", "proliferate"]
    ) {
        let player = if filtered[0] == "you" {
            PlayerAst::You
        } else {
            PlayerAst::Opponent
        };
        return Ok(PredicateAst::PlayerWouldProliferate { player });
    }

    if matches!(
        filtered.as_slice(),
        ["opponent", "would", "begin", "extra", "turn"]
            | ["an", "opponent", "would", "begin", "an", "extra", "turn"]
            | ["opponents", "would", "begin", "extra", "turn"]
    ) {
        return Ok(PredicateAst::PlayerWouldBeginExtraTurn {
            player: PlayerAst::Opponent,
        });
    }

    if matches!(
        filtered.as_slice(),
        ["it", "your", "turn"] | ["its", "your", "turn"] | ["your", "turn"]
    ) {
        return Ok(PredicateAst::YourTurn);
    }

    if matches!(
        filtered.as_slice(),
        ["it", "not", "your", "turn"]
            | ["its", "not", "your", "turn"]
            | ["it", "is", "not", "your", "turn"]
            | ["its", "is", "not", "your", "turn"]
            | ["not", "your", "turn"]
    ) {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::YourTurn)));
    }

    if matches!(
        filtered.as_slice(),
        ["creature", "died", "this", "turn"] | ["creatures", "died", "this", "turn"]
    ) {
        return Ok(PredicateAst::CreatureDiedThisTurn);
    }

    if matches!(
        filtered.as_slice(),
        ["opponent", "lost", "life", "this", "turn"]
            | [
                "one",
                "or",
                "more",
                "opponents",
                "lost",
                "life",
                "this",
                "turn",
            ]
    ) {
        return Ok(PredicateAst::OpponentLostLifeThisTurn);
    }

    if filtered.len() == 7
        && let Some(count) = parse_named_number(filtered[0])
        && word_slice_eq(
            &filtered[1..],
            &["or", "more", "creatures", "died", "this", "turn"],
        )
    {
        return Ok(PredicateAst::CreatureDiedThisTurnOrMore(count));
    }

    if matches!(
        filtered.as_slice(),
        [
            "a",
            "creature",
            "card",
            "was",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
            "this",
            "turn"
        ] | [
            "creature",
            "card",
            "was",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn);
    }

    if matches!(
        filtered.as_slice(),
        ["no", "permanent", "left", "battlefield", "this", "turn"]
            | ["no", "permanents", "left", "battlefield", "this", "turn"]
    ) {
        return Ok(PredicateAst::Not(Box::new(
            PredicateAst::PermanentLeftBattlefieldThisTurn,
        )));
    }

    if matches!(
        filtered.as_slice(),
        ["a", "permanent", "left", "battlefield", "this", "turn"]
            | ["permanent", "left", "battlefield", "this", "turn"]
            | ["permanents", "left", "battlefield", "this", "turn"]
    ) {
        return Ok(PredicateAst::PermanentLeftBattlefieldThisTurn);
    }

    if matches!(
        filtered.as_slice(),
        [
            "land",
            "you",
            "controlled",
            "was",
            "put",
            "into",
            "graveyard",
            "from",
            "battlefield",
            "this",
            "turn"
        ] | [
            "lands",
            "you",
            "controlled",
            "were",
            "put",
            "into",
            "graveyard",
            "from",
            "battlefield",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(
            ObjectFilter::land().controlled_by(PlayerFilter::You),
        ));
    }

    if matches!(
        filtered.as_slice(),
        [
            "permanent",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "permanents",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "permanent",
            "you",
            "controlled",
            "left",
            "battlefield",
            "this",
            "turn"
        ] | [
            "permanents",
            "you",
            "controlled",
            "left",
            "battlefield",
            "this",
            "turn"
        ] | [
            "creature",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "creatures",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn);
    }

    if matches!(
        filtered.as_slice(),
        [
            "nonland",
            "permanent",
            "left",
            "battlefield",
            "this",
            "turn",
            "or",
            "spell",
            "was",
            "warped",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::PermanentLeftBattlefieldThisTurn);
    }

    if matches!(
        filtered.as_slice(),
        [
            "you",
            "had",
            "another",
            "creature",
            "enter",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "last",
            "turn"
        ] | [
            "you",
            "had",
            "another",
            "creature",
            "entered",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "last",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::ObjectEnteredBattlefieldLastTurn(
            ObjectFilter::creature()
                .controlled_by(PlayerFilter::You)
                .other(),
        ));
    }

    if matches!(
        filtered.as_slice(),
        [
            "artifact",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "artifact",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "artifacts",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "artifacts",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::ObjectEnteredBattlefieldThisTurn(
            ObjectFilter::artifact().controlled_by(PlayerFilter::You),
        ));
    }

    if matches!(
        filtered.as_slice(),
        [
            "you",
            "had",
            "land",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "land",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "lands",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "lands",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
            player: PlayerAst::You,
        });
    }

    if filtered.len() >= 7
        && filtered[0] == "you"
        && filtered[1] == "gained"
        && let Some((count, used)) = parse_number(&tokens[2..])
        && word_slice_eq(
            &filtered[2 + used..],
            &["or", "more", "life", "this", "turn"],
        )
    {
        return Ok(PredicateAst::PlayerGainedLifeThisTurnOrMore {
            player: PlayerAst::You,
            count: count as u32,
        });
    }

    if filtered.len() >= 7
        && filtered[0] == "you"
        && filtered[1] == "lost"
        && let Some((count, used)) = parse_number(&tokens[2..])
        && word_slice_eq(
            &filtered[2 + used..],
            &["or", "more", "life", "this", "turn"],
        )
    {
        return Ok(PredicateAst::ValueComparison {
            left: Value::LifeLostThisTurn(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }

    if word_slice_eq(&filtered, &["you", "gained", "life", "this", "turn"]) {
        return Ok(PredicateAst::PlayerGainedLifeThisTurnOrMore {
            player: PlayerAst::You,
            count: 1,
        });
    }

    if word_slice_eq(&filtered, &["you", "attacked", "this", "turn"]) {
        return Ok(PredicateAst::YouAttackedThisTurn);
    }

    if matches!(
        filtered.as_slice(),
        ["that", "creature", "had", "to", "attack", "this", "combat"]
            | ["it", "had", "to", "attack", "this", "combat"]
            | ["that", "creature", "must", "attack", "this", "combat"]
            | ["it", "must", "attack", "this", "combat"]
    ) {
        return Ok(PredicateAst::TriggeringObjectHadToAttackThisCombat);
    }

    if filtered.len() == 9
        && filtered[0] == "you"
        && filtered[1] == "attacked"
        && filtered[2] == "with"
        && filtered[3] == "exactly"
        && matches!(filtered[5], "other" | "others")
        && matches!(filtered[6], "creature" | "creatures")
        && filtered[7] == "this"
        && filtered[8] == "combat"
        && let Some(count) = parse_named_number(filtered[4])
    {
        return Ok(PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count));
    }

    if matches!(
        filtered.as_slice(),
        [
            "this", "creature", "attacked", "or", "blocked", "this", "turn"
        ] | [
            "this",
            "permanent",
            "attacked",
            "or",
            "blocked",
            "this",
            "turn"
        ] | ["this", "attacked", "or", "blocked", "this", "turn"]
            | ["it", "attacked", "or", "blocked", "this", "turn"]
    ) {
        return Ok(PredicateAst::SourceAttackedOrBlockedThisTurn);
    }

    if word_slice_eq_any(
        &filtered,
        &[&["you", "cast", "it"], &["you", "cast", "this", "spell"]],
    ) {
        return Ok(PredicateAst::SourceWasCast);
    }
    if matches!(
        filtered.as_slice(),
        ["it", "was", "cast"]
            | ["that", "creature", "was", "cast"]
            | ["that", "permanent", "was", "cast"]
            | ["that", "object", "was", "cast"]
    ) {
        return Ok(PredicateAst::TaggedWasCast(TagKey::from(IT_TAG)));
    }

    if filtered.len() >= 6
        && filtered[0] == "this"
        && filtered[1] == "spell"
        && filtered[2] == "was"
        && filtered[3] == "cast"
        && filtered[4] == "from"
    {
        let zone_words = &filtered[5..];
        let zone = if zone_words.len() == 1 {
            parse_zone_word(zone_words[0])
        } else if zone_words.len() == 2 && is_article(zone_words[0]) {
            parse_zone_word(zone_words[1])
        } else if zone_words.len() == 2 && zone_words[0] == "the" {
            parse_zone_word(zone_words[1])
        } else {
            None
        };

        if let Some(zone) = zone {
            return Ok(PredicateAst::ThisSpellWasCastFromZone(zone));
        }
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["no", "spells", "were", "cast", "last", "turn"],
            &["no", "spell", "was", "cast", "last", "turn"],
        ],
    ) {
        return Ok(PredicateAst::NoSpellsWereCastLastTurn);
    }
    if word_slice_eq_any(
        &filtered,
        &[
            &["this", "spell", "was", "kicked"],
            &["this", "creature", "was", "kicked"],
            &["this", "permanent", "was", "kicked"],
        ],
    ) {
        return Ok(PredicateAst::ThisSpellWasKicked);
    }
    if word_slice_eq_any(
        &filtered,
        &[
            &["this", "spell", "was", "bargained"],
            &["it", "was", "bargained"],
        ],
    ) {
        return Ok(PredicateAst::ThisSpellPaidLabel("Bargain".to_string()));
    }
    if filtered.len() == 4
        && matches!(filtered[0], "a" | "an")
        && parse_subtype_word(filtered[1]).is_some()
        && matches!(filtered[2], "was" | "were")
        && filtered[3] == "beheld"
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if filtered.len() == 3
        && parse_subtype_word(filtered[0]).is_some()
        && matches!(filtered[1], "was" | "were")
        && filtered[2] == "beheld"
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if word_slice_eq(&filtered, &["gift", "was", "promised"]) {
        return Ok(PredicateAst::ThisSpellPaidLabel("Gift".to_string()));
    }
    if word_slice_eq_any(
        &filtered,
        &[
            &["gift", "wasnt", "promised"],
            &["gift", "was", "not", "promised"],
        ],
    ) {
        return Ok(PredicateAst::Not(Box::new(
            PredicateAst::ThisSpellPaidLabel("Gift".to_string()),
        )));
    }
    if filtered.len() >= 4
        && word_slice_eq(&filtered[filtered.len() - 3..], &["cost", "was", "paid"])
    {
        let start = usize::from(word_slice_first_is(&filtered, "the"));
        if let Some(label) = mana_cost_label_from_words(&filtered[start..filtered.len() - 3]) {
            return Ok(PredicateAst::ThisSpellPaidLabel(label));
        }
    }
    if filtered.len() >= 4
        && word_slice_eq(&filtered[filtered.len() - 3..], &["cost", "wasnt", "paid"])
    {
        let start = usize::from(word_slice_first_is(&filtered, "the"));
        if let Some(label) = mana_cost_label_from_words(&filtered[start..filtered.len() - 3]) {
            return Ok(PredicateAst::Not(Box::new(
                PredicateAst::ThisSpellPaidLabel(label),
            )));
        }
    }
    if filtered.len() >= 5
        && word_slice_eq(
            &filtered[filtered.len() - 4..],
            &["cost", "was", "not", "paid"],
        )
    {
        let start = usize::from(word_slice_first_is(&filtered, "the"));
        if let Some(label) = mana_cost_label_from_words(&filtered[start..filtered.len() - 4]) {
            return Ok(PredicateAst::Not(Box::new(
                PredicateAst::ThisSpellPaidLabel(label),
            )));
        }
    }
    if filtered.len() == 6
        && filtered[0] == "this"
        && matches!(
            filtered[1],
            "spell's"
                | "spells"
                | "card's"
                | "cards"
                | "creature's"
                | "creatures"
                | "permanent's"
                | "permanents"
        )
        && filtered[3] == "cost"
        && filtered[4] == "was"
        && filtered[5] == "paid"
    {
        let mut chars = filtered[2].chars();
        let Some(first) = chars.next() else {
            return Err(CardTextError::ParseError(
                "missing paid-cost label in predicate".to_string(),
            ));
        };
        let label = format!(
            "{}{}",
            first.to_ascii_uppercase(),
            chars.as_str().to_ascii_lowercase()
        );
        return Ok(PredicateAst::ThisSpellPaidLabel(label));
    }
    if word_slice_eq(&filtered, &["it", "was", "kicked"]) {
        return Ok(PredicateAst::ThisSpellWasKicked);
    }
    if word_slice_eq(&filtered, &["that", "was", "kicked"]) {
        return Ok(PredicateAst::TargetWasKicked);
    }

    if word_slice_eq(&filtered, &["you", "have", "full", "party"]) {
        return Ok(PredicateAst::YouHaveFullParty);
    }
    if word_slice_eq_any(
        &filtered,
        &[
            &["its", "controller", "poisoned"],
            &["that", "spells", "controller", "poisoned"],
        ],
    ) {
        return Ok(PredicateAst::TargetSpellControllerIsPoisoned);
    }
    if word_slice_eq_any(
        &filtered,
        &[
            &["no", "mana", "was", "spent", "to", "cast", "it"],
            &["no", "mana", "were", "spent", "to", "cast", "it"],
            &["no", "mana", "was", "spent", "to", "cast", "that", "spell"],
            &["no", "mana", "were", "spent", "to", "cast", "that", "spell"],
        ],
    ) {
        return Ok(PredicateAst::TargetSpellNoManaSpentToCast);
    }
    if word_slice_eq_any(
        &filtered,
        &[
            &[
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "that",
                "spells",
                "controller",
            ],
            &[
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "its",
                "controller",
            ],
        ],
    ) {
        return Ok(PredicateAst::YouControlMoreCreaturesThanTargetSpellController);
    }
    if filtered.len() == 7
        && matches!(filtered[0], "w" | "u" | "b" | "r" | "g" | "c" | "s")
        && filtered[1] == "was"
        && filtered[2] == "spent"
        && filtered[3] == "to"
        && filtered[4] == "cast"
        && filtered[5] == "this"
        && filtered[6] == "spell"
        && let Ok(symbol) = parse_mana_symbol(filtered[0])
    {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    }
    if filtered.len() >= 8
        && matches!(
            filtered[filtered.len() - 6..],
            ["was" | "were", "spent", "to", "cast", "this", "spell"]
        )
        && filtered[..filtered.len() - 6]
            .iter()
            .all(|word| matches!(*word, "w" | "u" | "b" | "r" | "g" | "c" | "s"))
    {
        let mut predicates = filtered[..filtered.len() - 6]
            .iter()
            .filter_map(|word| parse_mana_symbol(word).ok())
            .map(|symbol| PredicateAst::ManaSpentToCastThisSpellAtLeast {
                amount: 1,
                symbol: Some(symbol),
            });
        if let Some(first) = predicates.next() {
            return Ok(predicates.fold(first, |left, right| {
                PredicateAst::And(Box::new(left), Box::new(right))
            }));
        }
    }

    if let Some(amount) = parse_same_color_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(
            amount,
        ));
    }

    if let Some((amount, symbol)) = parse_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol });
    }

    if filtered.len() >= 5
        && matches!(
            filtered.as_slice(),
            ["this", "permanent", "attached", "to", ..]
                | ["that", "permanent", "attached", "to", ..]
                | ["this", "permanent", "is", "attached", "to", ..]
                | ["that", "permanent", "is", "attached", "to", ..]
        )
    {
        let attached_start = if word_slice_at_is(&filtered, 2, "is") {
            5
        } else {
            4
        };
        let attached_tokens = filtered[attached_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut filter = parse_object_filter(&attached_tokens, false)?;
        if filter.card_types.is_empty() {
            filter.card_types.push(CardType::Creature);
        }
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from("enchanted"),
            filter,
        ));
    }

    if filtered.len() >= 4 && filtered[0] == "sacrificed" && filtered[2] == "was" {
        let sacrificed_head = filtered[1];
        let subject_card_type =
            parse_card_type(sacrificed_head).filter(|card_type| is_permanent_type(*card_type));
        let subject_is_permanent = sacrificed_head == "permanent" || subject_card_type.is_some();

        if subject_is_permanent {
            let descriptor_tokens = filtered[3..]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&descriptor_tokens, false)?;
            if filter.card_types.is_empty() {
                if let Some(card_type) = subject_card_type {
                    filter.card_types.push(card_type);
                }
            }
            if filter.zone.is_none() && sacrificed_head == "permanent" {
                filter.zone = Some(Zone::Battlefield);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
    }

    if matches!(
        filtered.as_slice(),
        ["any", "of", "those", "cards", "remain", "exiled"]
            | ["those", "cards", "remain", "exiled"]
            | ["that", "card", "remains", "exiled"]
            | ["it", "remains", "exiled"]
    ) {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter::default().in_zone(Zone::Exile),
        ));
    }

    if filtered[0] == "its" || filtered[0] == "it's" {
        filtered[0] = "it";
    }
    if filtered.len() >= 2 && filtered[0] == "it" && filtered[1] == "s" {
        filtered.remove(1);
    }

    let demonstrative_reference_len = if word_slice_first_is(&filtered, "it") {
        Some(1usize)
    } else if filtered.len() >= 2
        && filtered[0] == "that"
        && matches!(
            filtered[1],
            "artifact"
                | "card"
                | "creature"
                | "creatures"
                | "enchantment"
                | "land"
                | "object"
                | "permanent"
                | "source"
                | "spell"
                | "token"
        )
    {
        Some(2usize)
    } else {
        None
    };

    let is_it_soulbond_paired = matches!(
        filtered.as_slice(),
        ["it", "paired", "with", "creature"]
            | ["it", "paired", "with", "another", "creature"]
            | ["it", "s", "paired", "with", "creature"]
            | ["it", "s", "paired", "with", "another", "creature"]
    );
    if is_it_soulbond_paired {
        return Ok(PredicateAst::ItIsSoulbondPaired);
    }

    if filtered.len() >= 2 {
        let tag = if word_slice_starts_with(&filtered, &["equipped", "creature"]) {
            Some("equipped")
        } else if word_slice_starts_with(&filtered, &["enchanted", "creature"]) {
            Some("enchanted")
        } else {
            None
        };
        if let Some(tag) = tag {
            let remainder = filtered[2..].to_vec();
            let tokens = remainder
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&tokens, false)?;
            if filter.card_types.is_empty() {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::TaggedMatches(TagKey::from(tag), filter));
        }
    }

    let onto_battlefield_idx = crate::runtime_backend::lexer::word_slice_find_phrase_start(
        &filtered,
        &["onto", "battlefield"],
    )
    .or_else(|| {
        crate::runtime_backend::lexer::word_slice_find_phrase_start(
            &filtered,
            &["onto", "the", "battlefield"],
        )
    });
    if filtered.len() >= 7
        && filtered[0] == "you"
        && filtered[1] == "put"
        && word_slice_ends_with(&filtered, &["this", "way"])
        && let Some(onto_idx) = onto_battlefield_idx
    {
        let filter_words = &filtered[2..onto_idx];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut filter = parse_object_filter(&filter_tokens, false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
    }

    let is_it = demonstrative_reference_len == Some(1);
    let has_card = demonstrative_reference_len
        .map(|reference_len| word_slice_contains_word(&filtered[reference_len..], "card"))
        .unwrap_or(false);

    if is_it {
        if filtered
            .get(1)
            .is_some_and(|word| *word == "has" || *word == "have")
        {
            filtered.remove(1);
        }
        if filtered.len() >= 3 && filtered[1] == "mana" && filtered[2] == "value" {
            let mana_value_tail = if filtered
                .get(3)
                .is_some_and(|word| matches!(*word, "is" | "are" | "was" | "were"))
            {
                &filtered[4..]
            } else {
                &filtered[3..]
            };
            let compares_to_colors_spent = word_slice_eq_any(
                mana_value_tail,
                &[
                    &[
                        "less", "than", "or", "equal", "to", "number", "of", "colors", "of",
                        "mana", "spent", "to", "cast", "this", "spell",
                    ],
                    &[
                        "less", "than", "or", "equal", "to", "number", "of", "color", "of", "mana",
                        "spent", "to", "cast", "this", "spell",
                    ],
                ],
            );
            if compares_to_colors_spent {
                return Ok(PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell);
            }

            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("mana value", mana_value_tail, &filtered)?
            {
                return Ok(PredicateAst::ItMatches(ObjectFilter {
                    mana_value: Some(cmp),
                    ..Default::default()
                }));
            }
        }

        if filtered.len() >= 5
            && filtered[1] == "total"
            && filtered[2] == "power"
            && filtered[3] == "and"
            && filtered[4] == "toughness"
            && let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("power", &filtered[5..], &filtered)?
        {
            return Ok(PredicateAst::ItMatches(ObjectFilter {
                total_power_toughness: Some(cmp),
                ..Default::default()
            }));
        }

        if filtered.len() >= 3 && (filtered[1] == "power" || filtered[1] == "toughness") {
            let axis = filtered[1];
            let value_tail = &filtered[2..];
            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens(axis, value_tail, &filtered)?
            {
                let mut filter = ObjectFilter::default();
                if axis == "power" {
                    filter.power = Some(cmp);
                } else {
                    filter.toughness = Some(cmp);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if demonstrative_reference_len.is_some()
        && word_slice_contains_word(&filtered, "or")
        && crate::runtime_backend::lexer::word_slice_find_phrase_start(
            &filtered,
            &["most", "common", "color", "among", "all", "permanents"],
        )
        .is_none()
        && let Some(predicate) = parse_or_predicate(&filtered)?
    {
        return Ok(predicate);
    }

    if let Some(reference_len) = demonstrative_reference_len {
        let mut descriptor_words = filtered[reference_len..].to_vec();
        if descriptor_words.len() >= 2 && matches!(descriptor_words[0], "power" | "toughness") {
            let axis = descriptor_words[0];
            let value_tail = if matches!(
                descriptor_words.get(1).copied(),
                Some("is" | "are" | "was" | "were")
            ) {
                &descriptor_words[2..]
            } else {
                &descriptor_words[1..]
            };
            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens(axis, value_tail, &filtered)?
            {
                let mut filter = ObjectFilter::default();
                if axis == "power" {
                    filter.power = Some(cmp);
                } else {
                    filter.toughness = Some(cmp);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
        if word_slice_eq_any(&descriptor_words, &[&["has", "toxic"], &["have", "toxic"]]) {
            let mut filter = ObjectFilter::default().with_ability_marker("toxic");
            if word_slice_at_is(&filtered, 1, "creature") {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
        if descriptor_words
            .first()
            .is_some_and(|word| matches!(*word, "is" | "are"))
        {
            descriptor_words.remove(0);
        }
        if matches!(
            descriptor_words.as_slice(),
            ["shares", "a", "card", "type", "with", "that", "spell"]
                | ["shares", "card", "type", "with", "that", "spell"]
        ) {
            return Ok(PredicateAst::ItMatches(
                ObjectFilter::default().shares_card_type_with_tagged("triggering"),
            ));
        }
        if matches!(
            descriptor_words.as_slice(),
            [
                "shares",
                "a",
                "color",
                "with",
                "the",
                "most",
                "common",
                "color",
                "among",
                "all",
                "permanents",
                "or",
                "a",
                "color",
                "tied",
                "for",
                "most",
                "common"
            ] | [
                "shares",
                "color",
                "with",
                "most",
                "common",
                "color",
                "among",
                "all",
                "permanents",
                "or",
                "color",
                "tied",
                "for",
                "most",
                "common"
            ]
        ) {
            return Ok(PredicateAst::ItMatches(
                ObjectFilter::default().shares_most_common_permanent_color(),
            ));
        }
        if word_slice_starts_with(&descriptor_words, &["not", "token"]) {
            descriptor_words.drain(0..2);
            descriptor_words.insert(0, "nontoken");
        }
        if !descriptor_words.is_empty() {
            if let Some(filter) = parse_single_card_type_card_descriptor(&descriptor_words) {
                return Ok(PredicateAst::ItMatches(filter));
            }
            let descriptor_tokens = descriptor_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let Ok(filter) = parse_object_filter_lexed(&descriptor_tokens, false)
                && filter != ObjectFilter::default()
            {
                if has_card
                    && filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    return Ok(PredicateAst::ItIsLandCard);
                }
                if word_slice_starts_with(&filtered, &["that", "enchantment"]) {
                    return Ok(PredicateAst::TaggedMatches(
                        TagKey::from("triggering"),
                        filter,
                    ));
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if filtered.len() >= 3
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && (filtered[2] == "no" || filtered[2] == "neither")
    {
        let control_tokens = filtered[3..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(mut filter) = parse_object_filter(&control_tokens, false) {
            filter.controller = Some(PlayerFilter::You);
            if filtered[2] == "neither" {
                filter = filter
                    .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
            }
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 4
        && filtered[0] == "player"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && filtered[2] == "no"
    {
        let control_tokens = filtered[3..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(mut filter) = parse_object_filter(&control_tokens, false) {
            filter.controller = Some(PlayerFilter::Any);
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::Any,
                filter,
            });
        }
    }

    let you_dont_control_filter_start = if filtered.len() >= 4
        && filtered[0] == "you"
        && matches!(filtered[1], "dont" | "don't")
        && (filtered[2] == "control" || filtered[2] == "controls")
    {
        Some(3usize)
    } else if filtered.len() >= 5
        && filtered[0] == "you"
        && filtered[1] == "do"
        && filtered[2] == "not"
        && (filtered[3] == "control" || filtered[3] == "controls")
    {
        Some(4usize)
    } else {
        None
    };
    if let Some(filter_start) = you_dont_control_filter_start {
        let control_tokens = filtered[filter_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 7
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && let Some(or_idx) = find_index(&filtered, |word| *word == "or")
        && or_idx > 2
    {
        let left_tokens = filtered[2..or_idx]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut right_words = filtered[or_idx + 1..].to_vec();
        if word_slice_first_is(&right_words, "there") {
            right_words = right_words[1..].to_vec();
        }
        if word_slice_contains_word(&right_words, "graveyard")
            && word_slice_contains_word(&right_words, "your")
        {
            let right_tokens = right_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let (Ok(mut control_filter), Ok(mut graveyard_filter)) = (
                parse_object_filter(&left_tokens, false),
                parse_object_filter(&right_tokens, false),
            ) {
                control_filter.controller = Some(PlayerFilter::You);
                if graveyard_filter.zone.is_none() {
                    graveyard_filter.zone = Some(Zone::Graveyard);
                }
                if graveyard_filter.owner.is_none() {
                    graveyard_filter.owner = Some(PlayerFilter::You);
                }
                return Ok(PredicateAst::PlayerControlsOrHasCardInGraveyard {
                    player: PlayerAst::You,
                    control_filter,
                    graveyard_filter,
                });
            }
        }
    }

    if filtered.len() >= 3
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
    {
        if let Some(and_idx) = find_index(&filtered[2..], |word| *word == "and") {
            let and_idx = 2 + and_idx;
            if and_idx > 2 && and_idx + 1 < filtered.len() {
                let left_tokens = filtered[2..and_idx]
                    .iter()
                    .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                    .collect::<Vec<_>>();
                let right_tokens = filtered[and_idx + 1..]
                    .iter()
                    .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                    .collect::<Vec<_>>();
                if let (Ok(mut left_filter), Ok(mut right_filter)) = (
                    parse_object_filter(&left_tokens, false),
                    parse_object_filter(&right_tokens, false),
                ) {
                    left_filter.controller = Some(PlayerFilter::You);
                    right_filter.controller = Some(PlayerFilter::You);
                    return Ok(PredicateAst::And(
                        Box::new(PredicateAst::PlayerControls {
                            player: PlayerAst::You,
                            filter: left_filter,
                        }),
                        Box::new(PredicateAst::PlayerControls {
                            player: PlayerAst::You,
                            filter: right_filter,
                        }),
                    ));
                }
            }
        }

        let mut filter_start = 2usize;
        let mut min_count: Option<u32> = None;
        let mut exact_count: Option<u32> = None;
        if let Some(raw_count) = filtered.get(2)
            && let Some(parsed_count) = parse_named_number(raw_count)
            && word_slice_starts_with(&filtered[3..], &["or", "more"])
        {
            min_count = Some(parsed_count);
            filter_start = 5;
        } else if word_slice_at_is(&filtered, 2, "exactly")
            && let Some(raw_count) = filtered.get(3)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            exact_count = Some(parsed_count);
            filter_start = 4;
        } else if word_slice_starts_with(&filtered[2..], &["at", "least"])
            && let Some(raw_count) = filtered.get(4)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            min_count = Some(parsed_count);
            filter_start = 5;
        }

        let mut control_words = filtered[filter_start..].to_vec();
        let mut requires_different_powers = false;
        if word_slice_ends_with(&control_words, &["with", "different", "powers"])
            || word_slice_ends_with(&control_words, &["with", "different", "power"])
        {
            requires_different_powers = true;
            control_words.truncate(control_words.len().saturating_sub(3));
        }
        let control_tokens = control_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other).or_else(|_| {
            parse_outlaw_shorthand_filter(&control_words)
                .ok_or_else(|| CardTextError::ParseError("unsupported control filter".to_string()))
        }) {
            filter.controller = Some(PlayerFilter::You);
            if let Some(count) = exact_count {
                return Ok(PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::You,
                    filter,
                    count,
                });
            }
            if let Some(count) = min_count
                && count > 1
            {
                if requires_different_powers {
                    return Ok(PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                        player: PlayerAst::You,
                        filter,
                        count,
                    });
                }
                return Ok(PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::You,
                    filter,
                    count,
                });
            }
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 4
        && filtered[0] == "that"
        && (filtered[1] == "player" || filtered[1] == "players")
        && (filtered[2] == "control" || filtered[2] == "controls")
    {
        let mut filter_start = 3usize;
        let mut min_count: Option<u32> = None;
        let mut exact_count: Option<u32> = None;
        if let Some(raw_count) = filtered.get(3)
            && let Some(parsed_count) = parse_named_number(raw_count)
            && word_slice_starts_with(&filtered[4..], &["or", "more"])
        {
            min_count = Some(parsed_count);
            filter_start = 6;
        } else if word_slice_at_is(&filtered, 3, "exactly")
            && let Some(raw_count) = filtered.get(4)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            exact_count = Some(parsed_count);
            filter_start = 5;
        } else if word_slice_starts_with(&filtered[3..], &["at", "least"])
            && let Some(raw_count) = filtered.get(5)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            min_count = Some(parsed_count);
            filter_start = 6;
        }

        let control_tokens = filtered[filter_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(filter) = parse_object_filter(&control_tokens, other) {
            if let Some(count) = exact_count {
                return Ok(PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::That,
                    filter,
                    count,
                });
            }
            if let Some(count) = min_count
                && count > 1
            {
                return Ok(PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::That,
                    filter,
                    count,
                });
            }
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::That,
                filter,
            });
        }
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["you", "controlled", "that", "permanent"],
            &["you", "control", "that", "permanent"],
        ],
    ) {
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter: ObjectFilter::default(),
        });
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["it", "entered", "under", "your", "control"],
            &["that", "card", "entered", "under", "your", "control"],
            &["that", "permanent", "entered", "under", "your", "control"],
        ],
    ) {
        return Ok(PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        });
    }

    if filtered.len() >= 8
        && filtered[0] == "you"
        && filtered[1] == "put"
        && word_slice_ends_with(&filtered, &["onto", "the", "battlefield", "this", "way"])
    {
        let filter_words = &filtered[2..filtered.len() - 5];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
    }

    if filtered.len() >= 7
        && filtered[1] == "is"
        && filtered[2] == "put"
        && word_slice_ends_with(&filtered, &["onto", "battlefield", "this", "way"])
    {
        let filter_words = &filtered[..filtered.len() - 6];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::TaggedMatches(TagKey::from(IT_TAG), filter));
    }

    let didnt_put_into_hand = matches!(
        filtered.as_slice(),
        ["you", "dont", "put", "the", "card", "into", "your", "hand"]
            | ["you", "didnt", "put", "the", "card", "into", "your", "hand"]
            | [
                "you", "did", "not", "put", "the", "card", "into", "your", "hand"
            ]
            | ["you", "dont", "put", "card", "into", "your", "hand"]
            | ["you", "didnt", "put", "card", "into", "your", "hand"]
            | ["you", "did", "not", "put", "card", "into", "your", "hand"]
            | ["you", "dont", "put", "it", "into", "your", "hand"]
            | ["you", "didnt", "put", "it", "into", "your", "hand"]
            | ["you", "did", "not", "put", "it", "into", "your", "hand"]
    );
    if didnt_put_into_hand {
        return Ok(PredicateAst::Not(Box::new(
            PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default().in_zone(Zone::Hand),
            },
        )));
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["it", "wasnt", "blocking"],
            &["it", "was", "not", "blocking"],
            &["that", "creature", "wasnt", "blocking"],
        ],
    ) {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter {
                nonblocking: true,
                ..Default::default()
            },
        ));
    }

    if word_slice_eq(&filtered, &["no", "creatures", "are", "on", "battlefield"]) {
        return Ok(PredicateAst::PlayerControlsNo {
            player: PlayerAst::Any,
            filter: ObjectFilter::creature(),
        });
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["you", "have", "citys", "blessing"],
            &["you", "have", "city", "blessing"],
        ],
    ) || word_slice_starts_with_any(
        &filtered,
        &[
            &["you", "have", "citys", "blessing", "for", "each"],
            &["you", "have", "city", "blessing", "for", "each"],
        ],
    ) {
        return Ok(PredicateAst::PlayerHasCitysBlessing {
            player: PlayerAst::You,
        });
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["youre", "the", "monarch"],
            &["youre", "monarch"],
            &["you", "are", "the", "monarch"],
            &["you", "are", "monarch"],
        ],
    ) {
        return Ok(PredicateAst::PlayerIsMonarch {
            player: PlayerAst::You,
        });
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["you", "have", "the", "initiative"],
            &["you", "have", "initiative"],
        ],
    ) {
        return Ok(PredicateAst::PlayerHasInitiative {
            player: PlayerAst::You,
        });
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &[
                "you",
                "or",
                "player",
                "youre",
                "attacking",
                "has",
                "initiative",
            ],
            &[
                "you",
                "or",
                "a",
                "player",
                "youre",
                "attacking",
                "has",
                "the",
                "initiative",
            ],
        ],
    ) {
        return Ok(PredicateAst::Or(
            Box::new(PredicateAst::PlayerHasInitiative {
                player: PlayerAst::You,
            }),
            Box::new(PredicateAst::PlayerHasInitiative {
                player: PlayerAst::Defending,
            }),
        ));
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["youve", "completed", "a", "dungeon"],
            &["you", "have", "completed", "a", "dungeon"],
        ],
    ) {
        return Ok(PredicateAst::PlayerCompletedDungeon {
            player: PlayerAst::You,
            dungeon_name: None,
        });
    }

    if (word_slice_starts_with(&filtered, &["youve", "completed"]) && filtered.len() > 2)
        || (word_slice_starts_with(&filtered, &["you", "have", "completed"]) && filtered.len() > 3)
    {
        let name_start = if filtered[1] == "have" { 3 } else { 2 };
        let dungeon_name = filtered[name_start..]
            .iter()
            .map(|word| (*word).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(PredicateAst::PlayerCompletedDungeon {
            player: PlayerAst::You,
            dungeon_name: Some(dungeon_name),
        });
    }

    if (word_slice_starts_with(&filtered, &["you", "havent", "completed"]) && filtered.len() > 3)
        || (word_slice_starts_with(&filtered, &["you", "have", "not", "completed"])
            && filtered.len() > 4)
    {
        let name_start = if filtered[1] == "have" { 4 } else { 3 };
        let dungeon_name = filtered[name_start..]
            .iter()
            .map(|word| (*word).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(PredicateAst::Not(Box::new(
            PredicateAst::PlayerCompletedDungeon {
                player: PlayerAst::You,
                dungeon_name: Some(dungeon_name),
            },
        )));
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &["youve", "cast", "another", "spell", "this", "turn"],
            &["you", "have", "cast", "another", "spell", "this", "turn"],
            &["you", "cast", "another", "spell", "this", "turn"],
        ],
    ) {
        return Ok(PredicateAst::PlayerCastSpellsThisTurnOrMore {
            player: PlayerAst::You,
            count: 2,
        });
    }

    let negative_spell_cast_prefix =
        if word_slice_starts_with(&filtered, &["that", "player", "didnt", "cast"]) {
            Some((4usize, PlayerFilter::Active))
        } else if word_slice_starts_with(&filtered, &["that", "player", "did", "not", "cast"]) {
            Some((5usize, PlayerFilter::Active))
        } else if word_slice_starts_with(&filtered, &["you", "didnt", "cast"]) {
            Some((3usize, PlayerFilter::You))
        } else if word_slice_starts_with(&filtered, &["you", "did", "not", "cast"]) {
            Some((4usize, PlayerFilter::You))
        } else {
            None
        };
    if let Some((prefix_len, player)) = negative_spell_cast_prefix
        && filtered.len() > prefix_len + 2
        && word_slice_eq(&filtered[filtered.len() - 2..], &["this", "turn"])
    {
        let filter_words = &filtered[prefix_len..filtered.len() - 2];
        if let Ok(predicate) = spell_cast_matching_predicate(player, filter_words) {
            return Ok(PredicateAst::Not(Box::new(predicate)));
        }
    }

    if word_slice_eq_any(
        &filtered,
        &[&["its", "night"], &["it", "is", "night"], &["it", "night"]],
    ) {
        return Ok(PredicateAst::ItIsNight);
    }

    if word_slice_eq_any(
        &filtered,
        &[
            &[
                "it", "dealt", "combat", "damage", "to", "player", "this", "turn",
            ],
            &[
                "it", "dealt", "combat", "damage", "to", "a", "player", "this", "turn",
            ],
        ],
    ) {
        return Ok(PredicateAst::SourceDealtCombatDamageToPlayerThisTurn);
    }

    if word_slice_eq(
        &filtered,
        &[
            "you", "cast", "this", "spell", "during", "your", "main", "phase",
        ],
    ) {
        return Ok(PredicateAst::ThisSpellPaidLabel(
            "CastDuringYourMainPhase".to_string(),
        ));
    }

    let spell_cast_prefix = if word_slice_starts_with(&filtered, &["opponent", "has", "cast"]) {
        Some((3usize, PlayerFilter::Opponent))
    } else if word_slice_starts_with(&filtered, &["opponents", "have", "cast"]) {
        Some((3usize, PlayerFilter::Opponent))
    } else if word_slice_starts_with(&filtered, &["youve", "cast"]) {
        Some((2usize, PlayerFilter::You))
    } else if word_slice_starts_with(&filtered, &["you", "have", "cast"]) {
        Some((3usize, PlayerFilter::You))
    } else if word_slice_starts_with(&filtered, &["you", "cast"]) {
        Some((2usize, PlayerFilter::You))
    } else {
        None
    };
    if let Some((prefix_len, player)) = spell_cast_prefix
        && filtered.len() > prefix_len + 2
        && word_slice_eq(&filtered[filtered.len() - 2..], &["this", "turn"])
    {
        let filter_words = &filtered[prefix_len..filtered.len() - 2];
        if let Some(predicate) = parse_both_spell_cast_predicate(player.clone(), filter_words)? {
            return Ok(predicate);
        }
        if let Ok(predicate) = spell_cast_matching_predicate(player, filter_words) {
            return Ok(predicate);
        }
    }

    if filtered.len() == 5
        && filtered[0] == "x"
        && filtered[1] == "is"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        if let Some(amount) = filtered[2]
            .parse::<i32>()
            .ok()
            .or_else(|| parse_named_number(filtered[2]).map(|n| n as i32))
        {
            return Ok(PredicateAst::ValueComparison {
                left: Value::X,
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(amount),
            });
        }
    }

    if let Some(predicate) = parse_or_predicate(&filtered)? {
        return Ok(predicate);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported predicate (predicate: '{}')",
        filtered.join(" ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parse_predicate_accepts_unapostrophed_spell_paid_label() -> Result<(), CardTextError> {
        let tokens = lex_line("If this spells surge cost was paid", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ThisSpellPaidLabel("Surge".to_string())
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_accepts_paid_label_with_trailing_instead_effect_tail()
    -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If this creature's spectacle cost was paid instead discard your hand",
            0,
        )?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ThisSpellPaidLabel("Spectacle".to_string())
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_opponent_would_begin_extra_turn() -> Result<(), CardTextError> {
        let tokens = lex_line("If an opponent would begin an extra turn", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::PlayerWouldBeginExtraTurn {
                player: PlayerAst::Opponent,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_or_player_youre_attacking_has_initiative()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If you or a player you're attacking has the initiative", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Or(
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::Defending,
                }),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_its_night() -> Result<(), CardTextError> {
        let tokens = lex_line("If it's night", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, PredicateAst::ItIsNight);
        Ok(())
    }

    #[test]
    fn parse_predicate_inherits_it_for_bare_or_descriptor_tail() -> Result<(), CardTextError> {
        let tokens = lex_line("If it's a creature or planeswalker card", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        match parsed {
            PredicateAst::Or(left, right) => {
                assert!(
                    matches!(*left, PredicateAst::ItMatches(ref filter) if filter.card_types == vec![CardType::Creature]),
                    "expected creature left predicate, got {left:?}"
                );
                assert!(
                    matches!(*right, PredicateAst::ItMatches(ref filter) if filter.card_types == vec![CardType::Planeswalker]),
                    "expected planeswalker right predicate, got {right:?}"
                );
            }
            other => panic!("expected inherited-reference or predicate, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_you_dont_put_card_into_your_hand() -> Result<(), CardTextError> {
        let tokens = lex_line("If you don't put the card into your hand", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default().in_zone(Zone::Hand),
            }))
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_it_dealt_combat_damage_to_player_this_turn()
    -> Result<(), CardTextError> {
        let tokens = lex_line("if it dealt combat damage to a player this turn", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::SourceDealtCombatDamageToPlayerThisTurn
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_cast_this_spell_during_your_main_phase()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If you cast this spell during your main phase", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ThisSpellPaidLabel("CastDuringYourMainPhase".to_string())
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_you_dont_put_it_into_your_hand() -> Result<(), CardTextError> {
        let tokens = lex_line("If you don't put it into your hand", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default().in_zone(Zone::Hand),
            }))
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_equipment_is_put_onto_the_battlefield_this_way()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If an Equipment is put onto the battlefield this way", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;
        let equipment_filter_tokens = lex_line("an Equipment", 0)?;
        let equipment_filter = parse_object_filter(&equipment_filter_tokens, false)?;

        assert_eq!(
            parsed,
            PredicateAst::TaggedMatches(TagKey::from(IT_TAG), equipment_filter)
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_aura_is_put_onto_the_battlefield_this_way()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If an Aura is put onto the battlefield this way", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;
        let aura_filter_tokens = lex_line("an Aura", 0)?;
        let aura_filter = parse_object_filter(&aura_filter_tokens, false)?;

        assert_eq!(
            parsed,
            PredicateAst::TaggedMatches(TagKey::from(IT_TAG), aura_filter)
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_would_draw_card() -> Result<(), CardTextError> {
        let tokens = lex_line("If you would draw a card", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;
        assert_eq!(
            parsed,
            PredicateAst::PlayerWouldDrawCard {
                player: PlayerAst::You
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_would_draw_while_no_cards_in_hand() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If you would draw a card while you have no cards in hand",
            0,
        )?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::And(
                Box::new(PredicateAst::PlayerWouldDrawCard {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::YouHaveNoCardsInHand),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_would_proliferate() -> Result<(), CardTextError> {
        let tokens = lex_line("If you would proliferate", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;
        assert_eq!(
            parsed,
            PredicateAst::PlayerWouldProliferate {
                player: PlayerAst::You
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_have_more_life_than_opponent() -> Result<(), CardTextError> {
        let tokens = lex_line("if you have more life than an opponent", 0)?;

        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::PlayerHasLessLifeThanYou {
                player: PlayerAst::Opponent,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_creature_card_put_into_your_graveyard_this_turn()
    -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If a creature card was put into your graveyard from anywhere this turn",
            0,
        )?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_card_in_your_graveyard_existence() -> Result<(), CardTextError> {
        let tokens = lex_line("If there is an Elf card in your graveyard", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        let mut expected_filter = ObjectFilter::default()
            .with_subtype(parse_subtype_word("elf").expect("elf subtype"))
            .in_zone(Zone::Graveyard);
        expected_filter.owner = Some(PlayerFilter::You);
        assert_eq!(
            parsed,
            PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter: expected_filter,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_behold_or_controlled_subtype_as_cast() -> Result<(), CardTextError>
    {
        let tokens = lex_line(
            "If you revealed a Dragon card or controlled a Dragon as you cast this spell",
            0,
        )?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Or(
                Box::new(PredicateAst::ThisSpellPaidLabel("Behold".to_string())),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: ObjectFilter::default()
                        .with_subtype(parse_subtype_word("dragon").expect("dragon subtype")),
                }),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_this_has_power_or_greater() -> Result<(), CardTextError> {
        let tokens = lex_line("If this has power 7 or greater", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, PredicateAst::SourcePowerAtLeast(7));
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_source_has_keyword() -> Result<(), CardTextError> {
        let tokens = lex_line("If this creature has defender", 0)?;
        let predicate_tokens = tokens
            .iter()
            .filter(|token| !token.is_word("if"))
            .cloned()
            .collect::<Vec<_>>();

        let parsed = parse_predicate(&predicate_tokens)?;

        let mut expected_filter = ObjectFilter::default();
        expected_filter
            .static_abilities
            .push(crate::static_abilities::StaticAbilityId::Defender);
        assert_eq!(parsed, PredicateAst::SourceMatches(expected_filter));
        Ok(())
    }
}
