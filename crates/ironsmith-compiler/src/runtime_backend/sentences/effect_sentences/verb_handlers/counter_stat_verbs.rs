const COUNTER_TARGET_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["target"], &["targets"]]);
const COUNTER_FROM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["from"]);
const COUNTER_AND_OR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["or"]]);
const COUNTER_FOR_EACH_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["for", "each"]);
const COUNTER_YOU_CONTROL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you", "control"], &["you", "controls"]]);
const COUNTER_YOU_DONT_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["you", "dont", "control"],
            &["you", "don't", "control"],
            &["you", "do", "not", "control"],
        ]
);
const COUNTER_OPPONENTS_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["your", "opponents", "control"],
            &["your", "opponents", "controls"],
            &["opponents", "control"],
            &["opponents", "controls"],
            &["an", "opponent", "controls"],
            &["opponent", "controls"],
        ]
);
const COUNTER_ALL_OR_EACH_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["all"], &["each"]]);
const COUNTER_ACTIVATED_OR_TRIGGERED_ABILITY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["activated", "or", "triggered", "ability"]);
const COUNTER_TRIGGERED_OR_ACTIVATED_ABILITY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["triggered", "or", "activated", "ability"]);
const COUNTER_ACTIVATED_ABILITY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["activated", "ability"]);
const COUNTER_TRIGGERED_ABILITY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["triggered", "ability"]);
const COUNTER_ABILITY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["ability"], &["abilities"]]);
const COUNTER_ABILITY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["ability", "abilities"]]);
const COUNTER_ACTIVATED_OR_TRIGGERED_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["activated", "triggered"]]);
const COUNTER_SPELL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["spell"]);
const COUNTER_INSTANT_SPELL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["instant", "spell"]);
const COUNTER_SORCERY_SPELL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["sorcery", "spell"]);
const COUNTER_LEGENDARY_SPELL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["legendary", "spell"]);
const COUNTER_NONCREATURE_SPELL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["noncreature", "spell"]);
const COUNTER_COLORLESS_SPELL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["colorless", "spell"]);
const COUNTER_ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["the"]]);
const COUNTER_SOURCE_OR_SOURCES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["source"], &["sources"]]);
const COUNTER_PLUS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["plus"]);
const COUNTER_ADDITIONAL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["additional"]);
const REVEAL_FULL_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["your", "hand"],
            &["their", "hand"],
            &["his", "or", "her", "hand"],
        ]
);
const PARTY_SIZE_EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "equal",
            "to",
            "the",
            "number",
            "of",
            "creatures",
            "in",
            "your",
            "party",
        ]
);
const EXPLICIT_TOP_CARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["top", "card"], &["the", "top", "card"]]);
const THAT_MANY_TOP_CARDS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "many", "cards", "from", "the", "top", "of"],
            &["that", "many", "cards", "from", "top", "of"],
        ]
);
const TOP_THE_TOP_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["the", "top"], &["top"]]);
const TOP_LIBRARY_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["card", "of", "your", "library"],
            &["cards", "of", "your", "library"],
            &["card", "of", "their", "library"],
            &["cards", "of", "their", "library"],
        ]
);
const WHERE_X_IS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["where", "x", "is"]);
const NUMBER_OF_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["number", "of"]);
const THIS_WAY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["this", "way"]]);
const CHOSEN_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["chosen"]);
const REVEAL_FROM_AMONG_TAGGED_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["from", "among"]; contains_any_words & [&["them"], &["those"]]);
const REVEAL_OUTSIDE_GAME_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["outside", "game"]);
const REVEAL_FIRST_CARD_YOU_DRAW_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "first", "card", "you", "draw"]);
const REVEAL_CARD_THIS_WAY_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_words(&[&["card"], &["cards"]])
    .contains_phrases(&[&["this", "way"]]);
const IF_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["if"]);
const REVEAL_HAND_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["hand"]);
const REVEAL_FROM_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["from"]);
const REVEAL_CARD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["card"], &["cards"]]);
const REVEAL_CARDS_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["cards"]);
const REVEAL_HAND_OWNER_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["their"], &["your"]]);
const THAT_MUCH_LIFE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["that", "much", "life"]);
const EQUAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["equal"]);
const EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["equal", "to"]);
const FOR_EACH_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["for", "each"]);
const LIFE_EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["life", "equal", "to"]);
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const IT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it"]);
const HALF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["half"]);
const LIFE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["life"]);
const LIFE_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["life"]);
const LOST_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["lost"]);
const ROUNDED_DOWN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["rounded", "down"]]);

fn counter_shape_matches_at(
    tokens: &[OwnedLexToken],
    idx: usize,
    shape: &ClauseShape<'static>,
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens.get(idx..).unwrap_or_default());
    shape.matches_words(&words)
}

fn generic_mana_amount_from_group(group: &[ManaSymbol]) -> Option<i32> {
    let [ManaSymbol::Generic(amount)] = group else {
        return None;
    };
    Some(*amount as i32)
}

fn generic_mana_amount_from_symbol(symbol: ManaSymbol) -> Option<i32> {
    match symbol {
        ManaSymbol::Generic(amount) => Some(amount as i32),
        _ => None,
    }
}

pub(crate) fn parse_counter_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if let Some(target) = parse_counter_ability_target_phrase(tokens)? {
        return Ok(target);
    }

    let words = crate::runtime_backend::token_word_refs(tokens);
    if COUNTER_ABILITY_MARKER_PATTERN.matches_words(&words)
        && COUNTER_ACTIVATED_OR_TRIGGERED_MARKER_PATTERN.matches_words(&words)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-ability target clause (clause: '{}')",
            words.join(" ")
        )));
    }

    parse_target_phrase(tokens)
}

fn parse_counter_ability_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let mut clause_tokens = trim_commas(tokens);
    if clause_tokens
        .first()
        .is_some_and(|token| token.as_word() == Some("counter"))
    {
        clause_tokens.drain(..1);
    }
    let is_controller_tail = |idx: usize| {
        counter_shape_matches_at(&clause_tokens, idx, &COUNTER_YOU_CONTROL_PREFIX_PATTERN)
            || counter_shape_matches_at(&clause_tokens, idx, &COUNTER_YOU_DONT_CONTROL_PREFIX_PATTERN)
            || counter_shape_matches_at(
                &clause_tokens,
                idx,
                &COUNTER_OPPONENTS_CONTROL_PREFIX_PATTERN,
            )
    };
    let clause_words = crate::runtime_backend::token_word_refs(&clause_tokens);
    if !COUNTER_ABILITY_MARKER_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }

    let mut idx = 0usize;
    let mut target_count: Option<ChoiceCount> = None;
    if let Some((count, used)) = parse_choice_count_before_target_prefix(&clause_tokens[idx..]) {
        target_count = Some(count);
        idx += used;
    }

    let explicit_target = clause_tokens
        .get(idx)
        .is_some_and(|token| COUNTER_TARGET_WORD_PATTERN.matches_token(token));
    if explicit_target {
        idx += 1;
    } else if clause_tokens
        .get(idx)
        .is_some_and(|token| COUNTER_ALL_OR_EACH_WORD_PATTERN.matches_token(token))
    {
        idx += 1;
    } else {
        return Ok(None);
    }

    #[derive(Clone, Copy)]
    enum CounterTargetTerm {
        Ability,
        Spell,
    }

    let mut term_filters: Vec<(ObjectFilter, CounterTargetTerm)> = Vec::new();
    let mut list_end = clause_tokens.len();
    let mut scan = idx;
    while scan < clause_tokens.len() {
        if clause_tokens
            .get(scan)
            .is_some_and(|token| COUNTER_FROM_WORD_PATTERN.matches_token(token))
        {
            list_end = scan;
            break;
        }
        if is_controller_tail(scan) {
            list_end = scan;
            break;
        }
        scan += 1;
    }

    while idx < list_end {
        let Some(word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word) else {
            idx += 1;
            continue;
        };
        if COUNTER_AND_OR_WORD_PATTERN.matches_word(word) {
            idx += 1;
            continue;
        }

        if counter_shape_matches_at(
            &clause_tokens,
            idx,
            &COUNTER_ACTIVATED_OR_TRIGGERED_ABILITY_PREFIX_PATTERN,
        ) {
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            idx += 4;
            continue;
        }

        if counter_shape_matches_at(
            &clause_tokens,
            idx,
            &COUNTER_TRIGGERED_OR_ACTIVATED_ABILITY_PREFIX_PATTERN,
        ) {
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            idx += 4;
            continue;
        }

        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_ACTIVATED_ABILITY_PREFIX_PATTERN)
        {
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            idx += 2;
            continue;
        }

        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_TRIGGERED_ABILITY_PREFIX_PATTERN)
        {
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            idx += 2;
            continue;
        }

        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_ABILITY_PREFIX_PATTERN) {
            term_filters.push((ObjectFilter::ability(), CounterTargetTerm::Ability));
            idx += 1;
            continue;
        }

        if COUNTER_SPELL_WORD_PATTERN.matches_word(word) {
            term_filters.push((ObjectFilter::spell(), CounterTargetTerm::Spell));
            idx += 1;
            continue;
        }

        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_INSTANT_SPELL_PREFIX_PATTERN) {
            term_filters.push((
                ObjectFilter::spell().with_type(CardType::Instant),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_SORCERY_SPELL_PREFIX_PATTERN) {
            term_filters.push((
                ObjectFilter::spell().with_type(CardType::Sorcery),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_LEGENDARY_SPELL_PREFIX_PATTERN) {
            term_filters.push((
                ObjectFilter::spell().with_supertype(Supertype::Legendary),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_NONCREATURE_SPELL_PREFIX_PATTERN) {
            let mut filter = ObjectFilter::noncreature_spell().in_zone(Zone::Stack);
            filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
            term_filters.push((filter, CounterTargetTerm::Spell));
            idx += 2;
            continue;
        }

        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_COLORLESS_SPELL_PREFIX_PATTERN) {
            term_filters.push((ObjectFilter::spell().colorless(), CounterTargetTerm::Spell));
            idx += 2;
            continue;
        }

        return Ok(None);
    }

    if term_filters.is_empty() {
        return Ok(None);
    }

    let mut source_types: Vec<CardType> = Vec::new();
    let mut controller_filter: Option<PlayerFilter> = None;
    while idx < clause_tokens.len() {
        let Some(word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word) else {
            idx += 1;
            continue;
        };
        if COUNTER_AND_OR_WORD_PATTERN.matches_word(word) {
            idx += 1;
            continue;
        }
        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_YOU_CONTROL_PREFIX_PATTERN) {
            controller_filter = Some(PlayerFilter::You);
            idx += 2;
            continue;
        }
        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_YOU_DONT_CONTROL_PREFIX_PATTERN) {
            controller_filter = Some(PlayerFilter::NotYou);
            idx += if clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.as_word() == Some("do"))
            {
                4
            } else {
                3
            };
            continue;
        }
        if counter_shape_matches_at(&clause_tokens, idx, &COUNTER_OPPONENTS_CONTROL_PREFIX_PATTERN)
        {
            controller_filter = Some(PlayerFilter::Opponent);
            idx += if clause_tokens
                .get(idx)
                .is_some_and(|token| token.as_word() == Some("your"))
            {
                3
            } else if clause_tokens
                .get(idx)
                .is_some_and(|token| token.as_word() == Some("an"))
            {
                3
            } else {
                2
            };
            continue;
        }
        if COUNTER_FROM_WORD_PATTERN.matches_word(word) {
            idx += 1;
            if clause_tokens
                .get(idx)
                .is_some_and(|token| COUNTER_ARTICLE_WORD_PATTERN.matches_token(token))
            {
                idx += 1;
            }

            let mut parsed_type = false;
            while idx < clause_tokens.len() {
                let Some(type_word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word)
                else {
                    idx += 1;
                    continue;
                };
                if COUNTER_SOURCE_OR_SOURCES_WORD_PATTERN.matches_words(&[type_word]) {
                    idx += 1;
                    break;
                }
                if COUNTER_AND_OR_WORD_PATTERN.matches_words(&[type_word]) {
                    idx += 1;
                    continue;
                }
                let parsed = parse_card_type(type_word)
                    .or_else(|| str_strip_suffix(type_word, "s").and_then(parse_card_type));
                let Some(card_type) = parsed else {
                    return Ok(None);
                };
                source_types.push(card_type);
                parsed_type = true;
                idx += 1;
            }
            if !parsed_type {
                return Ok(None);
            }
            continue;
        }

        return Ok(None);
    }

    for (filter, term) in &mut term_filters {
        if let Some(controller) = controller_filter.clone() {
            let mut updated = filter.clone();
            updated.controller = Some(controller);
            *filter = updated;
        }
        if !source_types.is_empty() && matches!(term, CounterTargetTerm::Ability) {
            for card_type in &source_types {
                *filter = filter.clone().with_type(*card_type);
            }
        }
    }

    let target_filter = if term_filters.len() == 1 {
        term_filters
            .pop()
            .map(|(filter, _)| filter)
            .expect("single term filter should be present")
    } else {
        let mut any = ObjectFilter::default();
        any.any_of = term_filters.into_iter().map(|(filter, _)| filter).collect();
        any
    };

    let target = wrap_target_count(
        TargetAst::Object(
            target_filter,
            explicit_target.then(|| span_from_tokens(&clause_tokens)).flatten(),
            None,
        ),
        target_count,
    );
    Ok(Some(target))
}

pub(crate) fn scale_value_multiplier(value: Value, multiplier: i32) -> Value {
    if multiplier <= 0 {
        return Value::Fixed(0);
    }
    if multiplier == 1 {
        return value;
    }
    match value {
        Value::Fixed(amount) => Value::Fixed(amount * multiplier),
        Value::Count(filter) => Value::CountScaled(filter, multiplier),
        Value::CountScaled(filter, factor) => Value::CountScaled(filter, factor * multiplier),
        other => {
            let mut result = Value::Fixed(0);
            for _ in 0..multiplier {
                result = match result {
                    Value::Fixed(0) => other.clone(),
                    _ => Value::Add(Box::new(result), Box::new(other.clone())),
                };
            }
            result
        }
    }
}

pub(crate) fn parse_counter_unless_additional_generic_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if tokens
        .first()
        .is_none_or(|token| !COUNTER_PLUS_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }

    let mut idx = 1usize;
    if token_slice_at_is(tokens, idx, "an") {
        idx += 1;
    }
    if !tokens
        .get(idx)
        .is_some_and(|token| COUNTER_ADDITIONAL_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }
    idx += 1;

    let multiplier = if let Some(token) = tokens.get(idx) {
        if let Some(group) = mana_pips_from_token(token) {
            generic_mana_amount_from_group(&group).ok_or_else(|| {
                CardTextError::ParseError(
                    "unsupported nongeneric additional counter payment".to_string(),
                )
            })?
        } else {
            let symbol_word = token.as_word().ok_or_else(|| {
                CardTextError::ParseError("missing additional mana symbol".to_string())
            })?;
            let symbol = parse_mana_symbol(symbol_word).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported additional payment symbol '{}' in counter clause",
                    symbol_word
                ))
            })?;
            generic_mana_amount_from_symbol(symbol).ok_or_else(|| {
                CardTextError::ParseError(
                    "unsupported nongeneric additional counter payment".to_string(),
                )
            })?
        }
    } else {
        return Err(CardTextError::ParseError(
            "missing additional mana symbol".to_string(),
        ));
    };

    let filter_tokens = trim_commas(&tokens[idx + 1..]);
    if !FOR_EACH_PREFIX_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(
        &filter_tokens,
    )) {
        return Err(CardTextError::ParseError(format!(
            "unsupported additional counter payment tail (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let dynamic = parse_dynamic_cost_modifier_value(&filter_tokens)?.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported additional counter payment filter (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    Ok(Some(scale_value_multiplier(dynamic, multiplier)))
}

pub(crate) fn parse_reveal(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let words = crate::runtime_backend::token_word_refs(tokens);
    // Many effects split "reveal it/that card/those cards" into a standalone clause.
    // The engine does not model hidden information, so this compiles to a semantic no-op
    // that still allows parsing and auditing to proceed.
    if matches!(
        words.as_slice(),
        ["it"]
            | ["them"]
            | ["that"]
            | ["that", "card"]
            | ["those", "cards"]
            | ["those"]
            | ["this", "card"]
            | ["this"]
    ) {
        return Ok(EffectAst::subject_verb_reveal_tagged(TagKey::from(IT_TAG)));
    }
    let reveals_from_among = REVEAL_FROM_AMONG_TAGGED_PATTERN.matches_words(&words);
    if reveals_from_among {
        return Ok(EffectAst::subject_verb_reveal_tagged(TagKey::from(IT_TAG)));
    }
    let reveals_outside_game = REVEAL_OUTSIDE_GAME_PATTERN.matches_words(&words);
    if reveals_outside_game {
        return Ok(EffectAst::subject_verb_reveal_tagged(TagKey::from(IT_TAG)));
    }
    let reveals_first_draw = REVEAL_FIRST_CARD_YOU_DRAW_PREFIX_PATTERN.matches_words(&words);
    if reveals_first_draw {
        return Ok(EffectAst::subject_verb_reveal_tagged(TagKey::from(IT_TAG)));
    }
    let reveals_card_this_way = REVEAL_CARD_THIS_WAY_PATTERN.matches_words(&words);
    if reveals_card_this_way {
        return Ok(EffectAst::subject_verb_reveal_tagged(TagKey::from(IT_TAG)));
    }
    let reveals_conditional_it = words
        .first()
        .is_some_and(|word| IT_WORD_PATTERN.matches_word(word))
        && IF_MARKER_PATTERN.matches_words(&words);
    if reveals_conditional_it {
        return Ok(EffectAst::subject_verb_reveal_tagged(TagKey::from(IT_TAG)));
    }
    if REVEAL_HAND_MARKER_PATTERN.matches_words(&words) {
        let is_full_hand_reveal = REVEAL_FULL_HAND_PATTERN.matches_words(&words);
        if !is_full_hand_reveal {
            if REVEAL_FROM_MARKER_PATTERN.matches_words(&words) {
                if let Some(equal_idx) = EQUAL_WORD_PATTERN.find_word(&words) {
                    let tail = &words[equal_idx..];
                    let count_value = if PARTY_SIZE_EQUAL_TO_PREFIX_PATTERN.matches_words(tail) {
                        Some(Value::PartySize(PlayerFilter::You))
                    } else {
                        parse_dynamic_cost_modifier_value(&tokens[equal_idx..])?
                    };
                    if let Some(count_value) = count_value
                        && REVEAL_CARD_MARKER_PATTERN.matches_words(&words)
                        && REVEAL_HAND_OWNER_MARKER_PATTERN.matches_words(&words)
                    {
                        return Ok(EffectAst::subject_verb_reveal_cards_from_hand(
                            player,
                            ChoiceCount::dynamic_x(),
                            Some(count_value),
                            TagKey::from(IT_TAG),
                        ));
                    }
                }
                if let Some((count, _used)) = parse_number(tokens)
                    && REVEAL_CARDS_MARKER_PATTERN.matches_words(&words)
                    && REVEAL_HAND_OWNER_MARKER_PATTERN.matches_words(&words)
                {
                    return Ok(EffectAst::subject_verb_reveal_cards_from_hand(
                        player,
                        ChoiceCount::exactly(count as usize),
                        None,
                        TagKey::from(IT_TAG),
                    ));
                }
                return Ok(EffectAst::subject_verb_reveal_tagged(TagKey::from(IT_TAG)));
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported reveal-hand clause (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(EffectAst::subject_verb_reveal_hand(player));
    }

    let has_card = words.iter().any(|word| matches!(*word, "card" | "cards"));
    let has_library = words
        .iter()
        .any(|word| matches!(*word, "library" | "libraries"));
    let explicit_top_card = EXPLICIT_TOP_CARD_PATTERN.matches_words(&words)
        || (TOP_THE_TOP_PREFIX_PATTERN.matched_prefix_len(&words).is_some()
            && has_card
            && has_library);
    let top_library_reveal = TOP_THE_TOP_PREFIX_PATTERN
        .matched_prefix_len(&words)
        .is_some_and(|_| has_library);

    if (!has_card && !top_library_reveal)
        || (!has_library && !explicit_top_card && !top_library_reveal)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal clause (clause: '{}')",
            words.join(" ")
        )));
    }

    if THAT_MANY_TOP_CARDS_PREFIX_PATTERN.matches_words(&words) {
        return Ok(EffectAst::subject_verb_reveal_top_cards(
            player,
            Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::Count,
            },
            TagKey::from(IT_TAG),
        ));
    }

    let top_prefix_len = TOP_THE_TOP_PREFIX_PATTERN.matched_prefix_len(&words);
    if let Some(prefix_len) = top_prefix_len
        && let Some(count_token_idx) = token_index_for_word_index(tokens, prefix_len)
        && let Some((mut count, used)) = parse_value(&tokens[count_token_idx..])
    {
        let after_count = &tokens[count_token_idx + used..];
        let after_words = crate::runtime_backend::token_word_refs(after_count);
        let top_library_tail = TOP_LIBRARY_TAIL_PATTERN.matches_words(&after_words);
        if top_library_tail {
            if count == Value::X
                && let Some(where_word_idx) =
                    WHERE_X_IS_PREFIX_PATTERN.find_exact_window(&words, 3)
                && let Some(where_token_idx) = token_index_for_word_index(tokens, where_word_idx)
                && let Some(where_value) =
                    parse_prior_effect_count_binding_clause(&tokens[where_token_idx..])
                        .or_else(|| parse_value_binding_clause(&tokens[where_token_idx..]))
            {
                count = where_value;
            }
            if count != Value::Fixed(1) {
                return Ok(EffectAst::subject_verb_reveal_top_cards(
                    player,
                    count,
                    TagKey::from(IT_TAG),
                ));
            }
        }
    }

    Ok(EffectAst::subject_verb_reveal_top(player))
}

fn parse_prior_effect_count_binding_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_commas(tokens);
    let word_view = TokenWordView::new(&tokens);
    let words = word_view.word_refs();
    if !WHERE_X_IS_PREFIX_PATTERN.matches_words(&words) {
        return None;
    }

    let mut idx = 3usize;
    if THE_WORD_PATTERN.matches_word_at(&words, idx) {
        idx += 1;
    }
    if !NUMBER_OF_PREFIX_PATTERN.matches_words(&words[idx..]) {
        return None;
    }

    let object_words = &words[idx + 2..];
    let references_this_way = THIS_WAY_MARKER_PATTERN.matches_words(object_words);
    let references_memory_action = object_words.iter().any(|word| {
        matches!(
            *word,
            "chosen"
                | "destroyed"
                | "discarded"
                | "exiled"
                | "milled"
                | "revealed"
                | "sacrificed"
                | "searched"
        )
    });
    if !references_this_way && !references_memory_action {
        return None;
    }

    let source = if CHOSEN_MARKER_PATTERN.matches_words(object_words) {
        ironsmith_core::EffectMetricSource::ChosenObjects
    } else {
        ironsmith_core::EffectMetricSource::AffectedObjects
    };
    Some(Value::PendingEffectMetric {
        source,
        metric: ironsmith_core::EffectMetric::Count,
    })
}

pub(crate) fn parse_life_amount(
    tokens: &[OwnedLexToken],
    amount_kind: &str,
) -> Result<(Value, usize), CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if THAT_MUCH_LIFE_PATTERN.matches_words(&clause_words) {
        // "that much life" binds to the triggering event amount.
        return Ok((Value::EventValue(EventValueSpec::Amount), 2));
    }

    parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing {amount_kind} amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })
}

pub(crate) fn parse_life_equal_to_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !LIFE_EQUAL_TO_PREFIX_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }

    let amount_tokens = &tokens[1..];
    let amount_words = crate::runtime_backend::token_word_refs(amount_tokens);

    if let Some(value) = parse_add_mana_equal_amount_value(amount_tokens) {
        return Ok(Some(value));
    }
    if let Some(value) = parse_devotion_value_from_add_clause(amount_tokens)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_equal_to_number_of_filter_value(amount_tokens) {
        return Ok(Some(value));
    }
    if let Some(value) = parse_equal_to_aggregate_filter_value(amount_tokens) {
        return Ok(Some(value));
    }
    if matches!(
        amount_words.as_slice(),
        ["equal", "to", "the", "life", "lost", "this", "way"]
            | ["equal", "to", "life", "lost", "this", "way"]
            | [
                "equal", "to", "the", "amount", "of", "life", "lost", "this", "way"
            ]
            | ["equal", "to", "amount", "of", "life", "lost", "this", "way"]
    ) {
        return Ok(Some(Value::EventValue(EventValueSpec::LifeAmount)));
    }
    if matches!(
        amount_words.as_slice(),
        ["equal", "to", "the", "damage", "prevented", "this", "way"]
            | ["equal", "to", "damage", "prevented", "this", "way"]
            | [
                "equal", "to", "the", "amount", "of", "damage", "prevented", "this", "way"
            ]
            | ["equal", "to", "amount", "of", "damage", "prevented", "this", "way"]
    ) {
        return Ok(Some(Value::EventValue(EventValueSpec::Amount)));
    }
    if matches!(
        amount_words.as_slice(),
        [
            "equal", "to", "the", "total", "life", "lost", "by", "all", "players", "this",
            "turn"
        ] | [
            "equal", "to", "total", "life", "lost", "by", "all", "players", "this",
            "turn"
        ] | [
            "equal", "to", "the", "total", "amount", "of", "life", "lost", "by", "all",
            "players", "this", "turn"
        ] | [
            "equal", "to", "total", "amount", "of", "life", "lost", "by", "all", "players",
            "this", "turn"
        ]
    ) {
        return Ok(Some(Value::LifeLostThisTurn(PlayerFilter::Any)));
    }
    if matches!(
        amount_words.as_slice(),
        [
            "equal", "to", "the", "life", "that", "player", "lost", "this", "turn"
        ] | [
            "equal", "to", "life", "that", "player", "lost", "this", "turn"
        ] | [
            "equal", "to", "the", "amount", "of", "life", "that", "player", "lost", "this", "turn"
        ] | [
            "equal", "to", "amount", "of", "life", "that", "player", "lost", "this", "turn"
        ]
    ) {
        return Ok(Some(Value::LifeLostThisTurn(PlayerFilter::IteratedPlayer)));
    }
    if matches!(
        amount_words.as_slice(),
        [
            "equal", "to", "the", "damage", "already", "dealt", "to", "that", "player",
            "this", "turn"
        ] | [
            "equal", "to", "damage", "already", "dealt", "to", "that", "player", "this",
            "turn"
        ] | [
            "equal", "to", "the", "amount", "of", "damage", "already", "dealt", "to",
            "that", "player", "this", "turn"
        ] | [
            "equal", "to", "amount", "of", "damage", "already", "dealt", "to", "that",
            "player", "this", "turn"
        ]
    ) {
        return Ok(Some(Value::DamageDealtToPlayersThisTurn(
            PlayerFilter::target_player(),
        )));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(amount_tokens)? {
        return Ok(Some(value));
    }
    if EQUAL_TO_PREFIX_PATTERN.matches_words(&amount_words) {
        let value_tokens = &amount_tokens[2..];
        let mut value_words = crate::runtime_backend::token_word_refs(value_tokens);

        let parse_stat_of_target =
            |stat_words: &[&str], constructor: fn(Box<ChooseSpec>) -> Value| {
                if ClauseShape::new().prefix(stat_words).matches_words(&value_words) {
                    let target_tokens = &value_tokens[stat_words.len()..];
                    if let Ok(target) = parse_target_phrase(target_tokens) {
                        let spec = crate::runtime_backend::references::reference_helpers::choose_spec_for_target(&target);
                        return Some(constructor(Box::new(spec)));
                    }
                }
                None
            };
        if let Some(value) = parse_stat_of_target(&["power", "of"], Value::PowerOf) {
            return Ok(Some(value));
        }
        if let Some(value) = parse_stat_of_target(&["the", "power", "of"], Value::PowerOf) {
            return Ok(Some(value));
        }
        if let Some(value) = parse_stat_of_target(&["toughness", "of"], Value::ToughnessOf) {
            return Ok(Some(value));
        }
        if let Some(value) = parse_stat_of_target(&["the", "toughness", "of"], Value::ToughnessOf)
        {
            return Ok(Some(value));
        }
        if let Some(value) =
            parse_stat_of_target(&["mana", "value", "of"], Value::ManaValueOf)
        {
            return Ok(Some(value));
        }
        if let Some(value) =
            parse_stat_of_target(&["the", "mana", "value", "of"], Value::ManaValueOf)
        {
            return Ok(Some(value));
        }
        if let Some(value) = parse_possessive_target_stat_value(value_tokens, &value_words) {
            return Ok(Some(value));
        }
        if let Some(value) = parse_life_total_as_turn_began_value(&value_words) {
            return Ok(Some(value));
        }

        if let Some((value, used)) = parse_value(value_tokens)
            && used == value_tokens.len()
        {
            return Ok(Some(value));
        }
        if value_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| THE_WORD_PATTERN.matches_word(word))
        {
            let stripped_tokens = &value_tokens[1..];
            if let Some((value, used)) = parse_value(stripped_tokens)
                && used == stripped_tokens.len()
            {
                return Ok(Some(value));
            }
            value_words = crate::runtime_backend::token_word_refs(stripped_tokens);
        }
        for (prefix, stat_words) in [
            (&["power", "of"][..], &["power"][..]),
            (&["toughness", "of"][..], &["toughness"][..]),
            (&["mana", "value", "of"][..], &["mana", "value"][..]),
        ] {
            if ClauseShape::new().prefix(prefix).matches_words(&value_words) {
                let mut reordered = value_words[prefix.len()..].to_vec();
                reordered.extend_from_slice(stat_words);
                if let Some((value, used)) =
                    crate::runtime_backend::front_end::shared::util::parse_value_expr_words(
                        &reordered,
                    )
                    && used == reordered.len()
                {
                    return Ok(Some(value));
                }
            }
        }
    }

    Err(CardTextError::ParseError(format!(
        "missing life amount in equal-to clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_possessive_target_stat_value(tokens: &[OwnedLexToken], words: &[&str]) -> Option<Value> {
    let (stat_len, constructor): (usize, fn(Box<ChooseSpec>) -> Value) = match words {
        [.., "power"] => (1, Value::PowerOf),
        [.., "toughness"] => (1, Value::ToughnessOf),
        [.., "mana", "value"] => (2, Value::ManaValueOf),
        _ => return None,
    };
    let target_len = tokens.len().checked_sub(stat_len)?;
    if target_len == 0 {
        return None;
    }

    let mut target_tokens = tokens[..target_len].to_vec();
    let possessive = target_tokens.last_mut()?;
    let word = possessive.as_word()?;
    let base = word
        .strip_suffix("'s")
        .or_else(|| word.strip_suffix("’s"))
        .or_else(|| word.strip_suffix("‘s"))?
        .to_string();
    if base.is_empty() || !possessive.replace_word(base) {
        return None;
    }

    let target = parse_target_phrase(&target_tokens).ok()?;
    let spec =
        crate::runtime_backend::references::reference_helpers::choose_spec_for_target(&target);
    Some(constructor(Box::new(spec)))
}

fn parse_life_total_as_turn_began_value(words: &[&str]) -> Option<Value> {
    let tail = ["life", "total", "as", "the", "turn", "began"];
    if words.len() <= tail.len() || !words.ends_with(&tail) {
        return None;
    }

    let subject = &words[..words.len() - tail.len()];
    let player = match subject {
        ["your"] | ["you"] => PlayerFilter::You,
        ["that", "player"] | ["that", "player's"] => PlayerFilter::IteratedPlayer,
        ["target", "player"] | ["target", "player's"] => PlayerFilter::target_player(),
        ["target", "opponent"] | ["target", "opponent's"] => PlayerFilter::target_opponent(),
        ["opponent"] | ["opponent's"] | ["an", "opponent"] | ["an", "opponent's"] => {
            PlayerFilter::Opponent
        }
        ["each", "opponent"] | ["each", "opponent's"] => PlayerFilter::Opponent,
        _ => return None,
    };
    Some(Value::LifeTotalAsTurnBegan(player))
}

pub(crate) fn parse_life_amount_from_trailing(
    base_amount: &Value,
    trailing: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if trailing.is_empty() {
        return Ok(None);
    }

    if let Some(counter_value) = parse_for_each_counter_on_reference_value(trailing)
        && let Some(multiplier) = match base_amount {
            Value::Fixed(value) => Some(*value),
            Value::X => Some(1),
            _ => None,
        }
    {
        return Ok(Some(scale_value_multiplier(counter_value, multiplier)));
    }

    if let Some(dynamic) = parse_dynamic_cost_modifier_value(trailing)? {
        if let Some(multiplier) = match base_amount {
            Value::Fixed(value) => Some(*value),
            Value::X => Some(1),
            _ => None,
        } {
            return Ok(Some(scale_value_multiplier(dynamic, multiplier)));
        }
    }

    if let Some(where_value) = parse_value_binding_clause(trailing) {
        if value_contains_unbound_x(base_amount) {
            let clause = crate::runtime_backend::token_word_refs(trailing).join(" ");
            return Ok(Some(replace_unbound_x_with_value(
                base_amount.clone(),
                &where_value,
                &clause,
            )?));
        }
        if matches!(base_amount, Value::Fixed(1)) {
            return Ok(Some(where_value));
        }
    }

    Ok(None)
}

fn parse_for_each_counter_on_reference_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !COUNTER_FOR_EACH_PREFIX_PATTERN.matches_words(&words) {
        return None;
    }
    let counter_idx = find_index(words.as_slice(), |word| {
        *word == "counter" || *word == "counters"
    })?;
    if counter_idx <= 2 || !words.get(counter_idx + 1).is_some_and(|word| *word == "on") {
        return None;
    }

    let counter_type = crate::runtime_backend::parse_counter_type_from_tokens(
        &tokens[2..=counter_idx],
    );
    let reference = &words[counter_idx + 2..];
    if word_slice_eq_any(
        reference,
        &[
            &["it"],
            &["this"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "source"],
        ],
    ) {
        return Some(match counter_type {
            Some(counter_type) => Value::CountersOnSource(counter_type),
            None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
        });
    }

    if word_slice_eq_any(
        reference,
        &[
            &["that"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "object"],
            &["those"],
            &["those", "creatures"],
            &["those", "permanents"],
        ],
    ) {
        return Some(Value::CountersOn(
            Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
            counter_type,
        ));
    }

    None
}

pub(crate) fn validate_life_keyword(rest: &[OwnedLexToken]) -> Result<(), CardTextError> {
    if rest
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| !LIFE_WORD_PATTERN.matches_word(word))
    {
        return Err(CardTextError::ParseError(
            "missing life keyword".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn remap_source_stat_value_to_it(value: Value) -> Value {
    match value {
        Value::PowerOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::ToughnessOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::ManaValueOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::Add(left, right) => Value::Add(
            Box::new(remap_source_stat_value_to_it(*left)),
            Box::new(remap_source_stat_value_to_it(*right)),
        ),
        other => other,
    }
}

fn player_filter_for_life_reference(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::NotYou => Some(PlayerFilter::NotYou),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Some(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Some(PlayerFilter::LowestLifeTied),
        PlayerAst::ThatPlayerOrTargetController => None,
        PlayerAst::ItsController | PlayerAst::ItsOwner => None,
    }
}

fn parse_half_life_value(tokens: &[OwnedLexToken], player: PlayerAst) -> Option<Value> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !clause_words
        .first()
        .is_some_and(|word| HALF_WORD_PATTERN.matches_word(word))
        || !LIFE_MARKER_PATTERN.matches_words(&clause_words)
        || LOST_MARKER_PATTERN.matches_words(&clause_words)
    {
        return None;
    }

    let player_filter = player_filter_for_life_reference(player)?;
    let rounded_down = ROUNDED_DOWN_MARKER_PATTERN.matches_words(&clause_words);
    if rounded_down {
        Some(Value::HalfLifeTotalRoundedDown(player_filter))
    } else {
        Some(Value::HalfLifeTotalRoundedUp(player_filter))
    }
}
