pub(crate) fn parse_counter_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if let Some(target) = parse_counter_ability_target_phrase(tokens)? {
        return Ok(target);
    }

    if grammar::contains_word(tokens, "ability")
        && (grammar::contains_word(tokens, "activated")
            || grammar::contains_word(tokens, "triggered"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-ability target clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    parse_target_phrase(tokens)
}

fn parse_counter_ability_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let clause_tokens = trim_commas(tokens);
    let is_you_control_tail = |idx: usize| {
        clause_tokens
            .get(idx)
            .is_some_and(|token| token.is_word("you"))
            && ((clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("control") || token.is_word("controls")))
                || (clause_tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("dont") || token.is_word("don't"))
                    && clause_tokens
                        .get(idx + 2)
                        .is_some_and(|token| token.is_word("control")))
                || (clause_tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("do"))
                    && clause_tokens
                        .get(idx + 2)
                        .is_some_and(|token| token.is_word("not"))
                    && clause_tokens
                        .get(idx + 3)
                        .is_some_and(|token| token.is_word("control"))))
    };
    if !grammar::contains_word(&clause_tokens, "ability")
        || (!grammar::contains_word(&clause_tokens, "activated")
            && !grammar::contains_word(&clause_tokens, "triggered"))
    {
        return Ok(None);
    }

    let mut idx = 0usize;
    let mut target_count: Option<ChoiceCount> = None;
    if clause_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("up"))
        && clause_tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("to"))
        && let Some((count, used)) = parse_number(&clause_tokens[idx + 2..])
    {
        target_count = Some(ChoiceCount::up_to(count as usize));
        idx += 2 + used;
    } else if let Some((count, used)) = parse_number(&clause_tokens[idx..])
        && clause_tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"))
    {
        target_count = Some(ChoiceCount::exactly(count as usize));
        idx += used;
    } else if let Some((count, used)) = parse_target_count_range_prefix(&clause_tokens[idx..])
        && clause_tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"))
    {
        target_count = Some(count);
        idx += used;
    }

    if !clause_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("target"))
    {
        return Ok(None);
    }
    idx += 1;

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
            .is_some_and(|token| token.is_word("from"))
        {
            list_end = scan;
            break;
        }
        if is_you_control_tail(scan) {
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
        if matches!(word, "or" | "and") {
            idx += 1;
            continue;
        }

        if word == "activated"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("or"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("triggered"))
            && clause_tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("ability"))
        {
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

        if word == "triggered"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("or"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("activated"))
            && clause_tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("ability"))
        {
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

        if word == "activated"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("ability"))
        {
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            idx += 2;
            continue;
        }

        if word == "triggered"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("ability"))
        {
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            idx += 2;
            continue;
        }

        if word == "spell" {
            term_filters.push((ObjectFilter::spell(), CounterTargetTerm::Spell));
            idx += 1;
            continue;
        }

        if word == "instant"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            term_filters.push((
                ObjectFilter::spell().with_type(CardType::Instant),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if word == "sorcery"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            term_filters.push((
                ObjectFilter::spell().with_type(CardType::Sorcery),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if word == "legendary"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            term_filters.push((
                ObjectFilter::spell().with_supertype(Supertype::Legendary),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if word == "noncreature"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            let mut filter = ObjectFilter::noncreature_spell().in_zone(Zone::Stack);
            filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
            term_filters.push((filter, CounterTargetTerm::Spell));
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
        if matches!(word, "and" | "or") {
            idx += 1;
            continue;
        }
        if word == "you"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("control") || token.is_word("controls"))
        {
            controller_filter = Some(PlayerFilter::You);
            idx += 2;
            continue;
        }
        if word == "you"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("dont") || token.is_word("don't"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("control"))
        {
            controller_filter = Some(PlayerFilter::NotYou);
            idx += 3;
            continue;
        }
        if word == "you"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("do"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("not"))
            && clause_tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("control"))
        {
            controller_filter = Some(PlayerFilter::NotYou);
            idx += 4;
            continue;
        }
        if word == "from" {
            idx += 1;
            if clause_tokens
                .get(idx)
                .is_some_and(|token| matches!(token.as_word(), Some("a" | "an" | "the")))
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
                if matches!(type_word, "source" | "sources") {
                    idx += 1;
                    break;
                }
                if matches!(type_word, "and" | "or") {
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
        TargetAst::Object(target_filter, span_from_tokens(&clause_tokens), None),
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
    if tokens.is_empty() || !tokens[0].is_word("plus") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if tokens.get(idx).is_some_and(|token| token.is_word("an")) {
        idx += 1;
    }
    if !tokens
        .get(idx)
        .is_some_and(|token| token.is_word("additional"))
    {
        return Ok(None);
    }
    idx += 1;

    let multiplier = if let Some(token) = tokens.get(idx) {
        if let Some(group) = mana_pips_from_token(token) {
            match group.as_slice() {
                [ManaSymbol::Generic(amount)] => *amount as i32,
                _ => {
                    return Err(CardTextError::ParseError(
                        "unsupported nongeneric additional counter payment".to_string(),
                    ));
                }
            }
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
            match symbol {
                ManaSymbol::Generic(amount) => amount as i32,
                _ => {
                    return Err(CardTextError::ParseError(
                        "unsupported nongeneric additional counter payment".to_string(),
                    ));
                }
            }
        }
    } else {
        return Err(CardTextError::ParseError(
            "missing additional mana symbol".to_string(),
        ));
    };

    let filter_tokens = trim_commas(&tokens[idx + 1..]);
    if grammar::words_match_prefix(&filter_tokens, &["for", "each"]).is_none() {
        return Err(CardTextError::ParseError(format!(
            "unsupported additional counter payment tail (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let dynamic = parse_dynamic_cost_modifier_value(&filter_tokens)?.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported additional counter payment filter (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        ))
    })?;
    Ok(Some(scale_value_multiplier(dynamic, multiplier)))
}

pub(crate) fn parse_reveal(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let words = crate::cards::builders::compiler::token_word_refs(tokens);
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
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_from_among = grammar::contains_word(tokens, "from")
        && grammar::contains_word(tokens, "among")
        && (grammar::contains_word(tokens, "them") || grammar::contains_word(tokens, "those"));
    if reveals_from_among {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_outside_game =
        grammar::contains_word(tokens, "outside") && grammar::contains_word(tokens, "game");
    if reveals_outside_game {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_first_draw =
        grammar::words_match_any_prefix(tokens, FIRST_CARD_YOU_DRAW_PREFIXES).is_some();
    if reveals_first_draw {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_card_this_way = (grammar::contains_word(tokens, "card")
        || grammar::contains_word(tokens, "cards"))
        && grammar::words_match_suffix(tokens, &["this", "way"]).is_some();
    if reveals_card_this_way {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_conditional_it =
        words.first() == Some(&"it") && grammar::contains_word(tokens, "if");
    if reveals_conditional_it {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    if grammar::contains_word(tokens, "hand") {
        let is_full_hand_reveal = matches!(words.as_slice(), ["your", "hand"] | ["their", "hand"])
            || words.as_slice() == ["his", "or", "her", "hand"];
        if !is_full_hand_reveal {
            if grammar::contains_word(tokens, "from") {
                return Ok(EffectAst::RevealTagged {
                    tag: TagKey::from(IT_TAG),
                });
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported reveal-hand clause (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(EffectAst::RevealHand { player });
    }

    let has_card =
        grammar::contains_word(tokens, "card") || grammar::contains_word(tokens, "cards");
    let has_library =
        grammar::contains_word(tokens, "library") || grammar::contains_word(tokens, "libraries");
    let explicit_top_card =
        words.as_slice() == ["top", "card"] || words.as_slice() == ["the", "top", "card"];

    if !has_card || (!has_library && !explicit_top_card) {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(EffectAst::RevealTop { player })
}

pub(crate) fn parse_life_amount(
    tokens: &[OwnedLexToken],
    amount_kind: &str,
) -> Result<(Value, usize), CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words == ["that", "much", "life"] {
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
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["life", "equal", "to"]).is_none() {
        return Ok(None);
    }

    let amount_tokens = &tokens[1..];
    let amount_words = crate::cards::builders::compiler::token_word_refs(amount_tokens);

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
    if let Some(value) = parse_dynamic_cost_modifier_value(amount_tokens)? {
        return Ok(Some(value));
    }

    Err(CardTextError::ParseError(format!(
        "missing life amount in equal-to clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_life_amount_from_trailing(
    base_amount: &Value,
    trailing: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if trailing.is_empty() {
        return Ok(None);
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

    if let Some(where_value) = parse_where_x_value_clause(trailing) {
        if value_contains_unbound_x(base_amount) {
            let clause = crate::cards::builders::compiler::token_word_refs(trailing).join(" ");
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

pub(crate) fn validate_life_keyword(rest: &[OwnedLexToken]) -> Result<(), CardTextError> {
    if rest
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word != "life")
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
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Some(PlayerFilter::MostLifeTied),
        PlayerAst::ThatPlayerOrTargetController => None,
        PlayerAst::ItsController | PlayerAst::ItsOwner => None,
    }
}

fn parse_half_life_value(tokens: &[OwnedLexToken], player: PlayerAst) -> Option<Value> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.first().copied() != Some("half")
        || !grammar::contains_word(tokens, "life")
        || grammar::contains_word(tokens, "lost")
    {
        return None;
    }

    let player_filter = player_filter_for_life_reference(player)?;
    let rounded_down = grammar::words_find_phrase(tokens, &["rounded", "down"]).is_some();
    if rounded_down {
        Some(Value::HalfLifeTotalRoundedDown(player_filter))
    } else {
        Some(Value::HalfLifeTotalRoundedUp(player_filter))
    }
}

