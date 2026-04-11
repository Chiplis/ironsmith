pub(crate) fn parse_move(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    use super::super::grammar::primitives as grammar;
    use winnow::Parser as _;

    // "all counters from <source> onto/to <destination>"
    let Some(after_prefix) =
        grammar::strip_lexed_prefix_phrase(tokens, &["all", "counters", "from"])
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported move clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    };

    let split = grammar::split_lexed_once_on_separator(after_prefix, || grammar::kw("onto").void())
        .or_else(|| {
            grammar::split_lexed_once_on_separator(after_prefix, || grammar::kw("to").void())
        });
    let Some((from_tokens, to_tokens)) = split else {
        return Err(CardTextError::ParseError(format!(
            "missing move destination (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    };

    let from = parse_target_phrase(from_tokens)?;
    let to = parse_target_phrase(to_tokens)?;

    Ok(EffectAst::MoveAllCounters { from, to })
}

pub(crate) fn parse_draw(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let mut parsed_that_many_minus_one = false;
    let mut parsed_that_many_plus_one = false;
    let mut consumed_embedded_card_keyword = false;
    let (mut count, used) =
        if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, EVENT_AMOUNT_PREFIXES) {
            let mut value = Value::EventValue(EventValueSpec::Amount);
            let consumed = prefix.len();
            let rest = &tokens[consumed..];
            if rest
                .first()
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
            {
                let trailing = trim_commas(&rest[1..]);
                let trailing_words = crate::cards::builders::compiler::token_word_refs(&trailing);
                if trailing_words.as_slice() == ["minus", "one"] {
                    value = Value::EventValueOffset(EventValueSpec::Amount, -1);
                    parsed_that_many_minus_one = true;
                } else if trailing_words.as_slice() == ["plus", "one"] {
                    value = Value::EventValueOffset(EventValueSpec::Amount, 1);
                    parsed_that_many_plus_one = true;
                } else if !trailing_words.is_empty()
                    && find_window_by(&trailing_words, 2, |window| {
                        window[0] == "for" && window[1] == "each"
                    })
                    .is_none()
                {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing draw clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
            }
            (value, consumed)
        } else if let Some((value, used_words)) =
            parse_half_rounded_down_draw_count_words(&clause_words)
        {
            consumed_embedded_card_keyword = true;
            (
                value,
                token_index_for_word_index(tokens, used_words).unwrap_or(tokens.len()),
            )
        } else if let Some(value) = parse_draw_as_many_cards_value(tokens) {
            consumed_embedded_card_keyword = true;
            (value, tokens.len())
        } else if tokens.first().is_some_and(|token| token.is_word("another"))
            && tokens
                .get(1)
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            (Value::Fixed(1), 1)
        } else if tokens
            .first()
            .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            let tail = trim_commas(&tokens[1..]);
            let value = parse_draw_card_prefixed_count_value(&tail)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing draw count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            consumed_embedded_card_keyword = true;
            (value, tokens.len())
        } else if tokens.first().is_some_and(|token| token.is_word("up"))
            && tokens.get(1).is_some_and(|token| token.is_word("to"))
        {
            let Some((amount, used_amount)) = parse_number(&tokens[2..]) else {
                return Err(CardTextError::ParseError(format!(
                    "missing draw count (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            (Value::Fixed(amount as i32), 2 + used_amount)
        } else {
            parse_value(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing draw count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
        };

    let rest = &tokens[used..];
    let tail = if consumed_embedded_card_keyword {
        trim_commas(rest)
    } else {
        let mut card_word_idx = 0usize;
        if rest
            .first()
            .is_some_and(|token| token.is_word("additional"))
        {
            card_word_idx = 1;
        }
        let Some(card_word) = rest.get(card_word_idx).and_then(OwnedLexToken::as_word) else {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        };
        if card_word != "card" && card_word != "cards" {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        }
        trim_commas(&rest[card_word_idx + 1..])
    };
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let mut effect = EffectAst::Draw {
        count: count.clone(),
        player,
    };

    if !tail.is_empty() {
        let tail_words = crate::cards::builders::compiler::token_word_refs(&tail);
        if !((parsed_that_many_minus_one && tail_words.as_slice() == ["minus", "one"])
            || (parsed_that_many_plus_one && tail_words.as_slice() == ["plus", "one"]))
        {
            if let Some(parsed) = parse_draw_for_each_player_condition(&tail, effect.clone())? {
                effect = parsed;
            } else {
                let has_for_each = find_window_by(&tail, 2, |window: &[OwnedLexToken]| {
                    window[0].is_word("for") && window[1].is_word("each")
                })
                .is_some();
                if has_for_each {
                    let dynamic = parse_dynamic_cost_modifier_value(&tail)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported draw for-each clause (clause: '{}')",
                            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                        ))
                    })?;
                    match count {
                        Value::Fixed(1) => count = dynamic,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported multiplied draw count (clause: '{}')",
                                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                            )));
                        }
                    }
                    effect = EffectAst::Draw {
                        count: count.clone(),
                        player,
                    };
                } else if let Some(parsed) = parse_draw_trailing_clause(&tail, effect.clone())? {
                    effect = parsed;
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing draw clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
            }
        }
    }
    Ok(effect)
}

fn parse_draw_for_each_player_condition(
    tokens: &[OwnedLexToken],
    draw_effect: EffectAst,
) -> Result<Option<EffectAst>, CardTextError> {
    fn bind_loop_player_predicate(predicate: PredicateAst) -> PredicateAst {
        match predicate {
            PredicateAst::And(left, right) => PredicateAst::And(
                Box::new(bind_loop_player_predicate(*left)),
                Box::new(bind_loop_player_predicate(*right)),
            ),
            PredicateAst::Not(inner) => {
                PredicateAst::Not(Box::new(bind_loop_player_predicate(*inner)))
            }
            PredicateAst::PlayerControls { player, filter } if player == PlayerAst::That => {
                PredicateAst::PlayerControls {
                    player: PlayerAst::Implicit,
                    filter,
                }
            }
            PredicateAst::PlayerControlsAtLeast {
                player,
                filter,
                count,
            } if player == PlayerAst::That => PredicateAst::PlayerControlsAtLeast {
                player: PlayerAst::Implicit,
                filter,
                count,
            },
            PredicateAst::PlayerControlsExactly {
                player,
                filter,
                count,
            } if player == PlayerAst::That => PredicateAst::PlayerControlsExactly {
                player: PlayerAst::Implicit,
                filter,
                count,
            },
            PredicateAst::PlayerControlsMost { player, filter } if player == PlayerAst::That => {
                PredicateAst::PlayerControlsMost {
                    player: PlayerAst::Implicit,
                    filter,
                }
            }
            PredicateAst::PlayerControlsMoreThanYou { player, filter }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerControlsMoreThanYou {
                    player: PlayerAst::Implicit,
                    filter,
                }
            }
            PredicateAst::PlayerHasLessLifeThanYou { player } if player == PlayerAst::That => {
                PredicateAst::PlayerHasLessLifeThanYou {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasMoreLifeThanYou { player } if player == PlayerAst::That => {
                PredicateAst::PlayerHasMoreLifeThanYou {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHasNoOpponentWithMoreLifeThan {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasMoreCardsInHandThanYou { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHasMoreCardsInHandThanYou {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerTappedLandForManaThisTurn { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerTappedLandForManaThisTurn {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
                    player: PlayerAst::Implicit,
                }
            }
            other => other,
        }
    }

    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let (start, opponents_only) = if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, FOR_EACH_OPPONENT_WHO_PREFIXES)
    {
        (prefix.len() - 1, true)
    } else if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, FOR_EACH_PLAYER_WHO_PREFIXES)
    {
        (prefix.len() - 1, false)
    } else if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, EACH_OPPONENT_WHO_PREFIXES)
    {
        (prefix.len() - 1, true)
    } else if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, EACH_PLAYER_WHO_PREFIXES)
    {
        (prefix.len() - 1, false)
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&tokens[start..]);
    let inner_words = crate::cards::builders::compiler::token_word_refs(&inner_tokens);
    if inner_words.first().copied() != Some("who") {
        return Ok(None);
    }

    let predicate_tail = trim_commas(&inner_tokens[1..]);
    if predicate_tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing predicate in draw for-each clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let predicate = bind_loop_player_predicate(
        parse_who_player_predicate_lexed(&inner_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing predicate in draw for-each clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?,
    );

    let effects = vec![EffectAst::Conditional {
        predicate,
        if_true: vec![draw_effect],
        if_false: Vec::new(),
    }];
    Ok(Some(if opponents_only {
        EffectAst::ForEachOpponent { effects }
    } else {
        EffectAst::ForEachPlayer { effects }
    }))
}

pub(crate) fn parse_half_rounded_down_draw_count_words(words: &[&str]) -> Option<(Value, usize)> {
    if words.first().copied() != Some("half") {
        return None;
    }

    let mut card_idx = None;
    for idx in 1..words.len() {
        if matches!(words.get(idx).copied(), Some("card" | "cards"))
            && words.get(idx + 1..idx + 3) == Some(&["rounded", "down"][..])
        {
            card_idx = Some(idx);
            break;
        }
    }
    let card_idx = card_idx?;

    let inner_words = &words[1..card_idx];
    let (inner, used_inner) = parse_value_expr_words(inner_words)?;
    if used_inner != inner_words.len() {
        return None;
    }

    Some((Value::HalfRoundedDown(Box::new(inner)), card_idx + 3))
}

pub(crate) fn parse_draw_trailing_clause(
    tokens: &[OwnedLexToken],
    draw_effect: EffectAst,
) -> Result<Option<EffectAst>, CardTextError> {
    let tail_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if tail_words.as_slice() == ["instead"] {
        return Ok(Some(draw_effect));
    }

    if let Some(timing) = parse_draw_delayed_timing_words(&tail_words) {
        return Ok(Some(wrap_return_with_delayed_timing(
            draw_effect,
            Some(timing),
        )));
    }

    if tail_words.first().copied() == Some("if") {
        let predicate = parse_trailing_if_predicate_lexed(tokens).ok_or_else(|| {
            CardTextError::ParseError("missing condition after trailing if clause".to_string())
        })?;
        return Ok(Some(EffectAst::Conditional {
            predicate,
            if_true: vec![draw_effect],
            if_false: Vec::new(),
        }));
    }

    if tail_words.first().copied() == Some("unless") {
        return try_build_unless(vec![draw_effect], tokens, 0);
    }

    Ok(None)
}

pub(crate) fn parse_draw_delayed_timing_words(words: &[&str]) -> Option<DelayedReturnTimingAst> {
    if let Some(timing) = parse_delayed_return_timing_words(words) {
        return Some(timing);
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "turns", "upkeep"]
            | ["at", "beginning", "of", "next", "turn's", "upkeep"]
            | ["at", "beginning", "of", "next", "turn’s", "upkeep"]
            | ["at", "beginning", "of", "the", "next", "turns", "upkeep"]
            | ["at", "beginning", "of", "the", "next", "turn's", "upkeep"]
            | ["at", "beginning", "of", "the", "next", "turn’s", "upkeep"]
            | ["at", "the", "beginning", "of", "next", "turns", "upkeep"]
            | ["at", "the", "beginning", "of", "next", "turn's", "upkeep"]
            | ["at", "the", "beginning", "of", "next", "turn’s", "upkeep"]
            | [
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "turns",
                "upkeep"
            ]
            | [
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "turn's",
                "upkeep"
            ]
            | [
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "turn’s",
                "upkeep"
            ]
    ) {
        return Some(DelayedReturnTimingAst::NextUpkeep(PlayerAst::Any));
    }

    None
}

pub(crate) fn parse_draw_as_many_cards_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let starts_as_many = clause_words.len() >= 4
        && clause_words[0] == "as"
        && clause_words[1] == "many"
        && matches!(clause_words[2], "card" | "cards")
        && clause_words[3] == "as";
    if !starts_as_many {
        return None;
    }

    let references_previous_event = grammar::words_find_phrase(tokens, &["this", "way"]).is_some();
    if references_previous_event {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }

    None
}

pub(crate) fn parse_draw_card_prefixed_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    if let Some(value) = parse_draw_equal_to_value(tokens)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
        return Ok(Some(value));
    }

    Ok(None)
}

pub(crate) fn parse_draw_equal_to_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if grammar::words_match_prefix(tokens, &["equal", "to"]).is_none() {
        return Ok(None);
    }

    if let Some(value) = parse_devotion_value_from_add_clause(tokens)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_add_mana_equal_amount_value(tokens)
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(tokens))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(tokens))
        .or_else(|| parse_equal_to_aggregate_filter_value(tokens))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(tokens))
        .or_else(|| parse_equal_to_number_of_filter_value(tokens))
    {
        return Ok(Some(value));
    }
    if grammar::words_find_phrase(tokens, &["this", "way"]).is_some() {
        return Ok(Some(Value::EventValue(EventValueSpec::Amount)));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
        return Ok(Some(value));
    }

    Ok(None)
}

pub(crate) fn parse_counter(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if let Some(spec) = split_trailing_if_clause_lexed(tokens) {
        let target = parse_counter_target_phrase(spec.leading_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate: spec.predicate,
            if_true: vec![EffectAst::Counter { target }],
            if_false: Vec::new(),
        });
    }

    if super::super::grammar::primitives::contains_word(tokens, "if") {
        return Err(CardTextError::ParseError(format!(
            "missing conditional counter target or predicate (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    if let Some((target_tokens, unless_tokens)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("unless").void()
        })
    {
        let target = parse_counter_target_phrase(target_tokens)?;
        let pays_idx = find_index(unless_tokens, |token: &OwnedLexToken| token.is_word("pays"))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing pays keyword (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                ))
            })?;

        // Parse the contiguous mana payment immediately following "pays".
        // Stop at the first non-mana word so trailing dynamic qualifiers
        // ("for each ...", "where X is ...", "plus an additional ...") do not
        // accidentally duplicate symbols.
        let mut mana = Vec::new();
        let mut trailing_start: Option<usize> = None;
        for (offset, token) in unless_tokens[pays_idx + 1..].iter().enumerate() {
            if let Some(group) = mana_pips_from_token(token) {
                mana.extend(group);
                continue;
            }
            if token.is_comma() || token.is_period() {
                continue;
            }
            let Some(word) = token.as_word() else {
                if !mana.is_empty() {
                    trailing_start = Some(pays_idx + 1 + offset);
                    break;
                }
                continue;
            };
            match parse_mana_symbol(word) {
                Ok(symbol) => mana.push(symbol),
                Err(_) => {
                    trailing_start = Some(pays_idx + 1 + offset);
                    break;
                }
            }
        }

        let mut life = None;
        let mut additional_generic = None;
        if mana.is_empty() {
            let payment_tokens = trim_commas(&unless_tokens[pays_idx + 1..]);
            let payment_words = crate::cards::builders::compiler::token_word_refs(&payment_tokens);
            // "unless its controller pays mana equal to ..." uses a dynamic generic payment.
            if payment_words.first().copied() == Some("mana")
                && let Some(value) = parse_equal_to_aggregate_filter_value(&payment_tokens)
                    .or_else(|| parse_equal_to_number_of_filter_value(&payment_tokens))
            {
                additional_generic = Some(value);
                trailing_start = None;
            } else {
                return Err(CardTextError::ParseError(format!(
                    "missing mana cost (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            }
        }

        if let Some(trailing_idx) = trailing_start {
            let trailing_tokens = trim_commas(&unless_tokens[trailing_idx..]);
            let trailing_words = crate::cards::builders::compiler::token_word_refs(&trailing_tokens);
            if trailing_tokens
                .first()
                .is_some_and(|token| token.is_word("and"))
            {
                let life_tokens = trim_commas(&trailing_tokens[1..]);
                if let Some((amount, used)) = parse_value(&life_tokens)
                    && life_tokens
                        .get(used)
                        .is_some_and(|token| token.is_word("life"))
                    && trim_commas(&life_tokens[used + 1..]).is_empty()
                {
                    life = Some(amount);
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        crate::cards::builders::compiler::token_word_refs(tokens).join(" "),
                        trailing_words.join(" ")
                    )));
                }
            } else if let Some(value) =
                parse_counter_unless_additional_generic_value(&trailing_tokens)?
            {
                additional_generic = Some(value);
            } else if grammar::words_match_any_prefix(&trailing_tokens, FOR_EACH_PREFIXES).is_some()
            {
                if let Some(dynamic) = parse_dynamic_cost_modifier_value(&trailing_tokens)? {
                    if let [ManaSymbol::Generic(multiplier)] = mana.as_slice() {
                        additional_generic =
                            Some(scale_value_multiplier(dynamic, *multiplier as i32));
                        mana.clear();
                    } else {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                            crate::cards::builders::compiler::token_word_refs(tokens).join(" "),
                            trailing_words.join(" ")
                        )));
                    }
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        crate::cards::builders::compiler::token_word_refs(tokens).join(" "),
                        trailing_words.join(" ")
                    )));
                }
            } else if !trailing_words.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" "),
                    trailing_words.join(" ")
                )));
            }
        }

        if mana.is_empty() && life.is_none() && additional_generic.is_none() {
            return Err(CardTextError::ParseError(format!(
                "missing mana cost (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            )));
        }

        return Ok(EffectAst::CounterUnlessPays {
            target,
            mana,
            life,
            additional_generic,
        });
    }

    let target = parse_counter_target_phrase(tokens)?;
    Ok(EffectAst::Counter { target })
}

