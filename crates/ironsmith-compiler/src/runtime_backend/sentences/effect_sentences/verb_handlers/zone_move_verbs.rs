const ZONE_MOVE_CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const ZONE_MOVE_GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["graveyard"], &["graveyards"]]);
const ZONE_MOVE_WHO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["who"]);
const ZONE_MOVE_HALF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["half"]);
const DRAW_TRAILING_IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const DRAW_TRAILING_UNLESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["unless"]);
const COUNTER_MANA_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["mana"]);
const ZONE_MOVE_MINUS_ONE_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["minus", "one"]);
const ZONE_MOVE_PLUS_ONE_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["plus", "one"]);
const ZONE_MOVE_FOR_EACH_PATTERN: ClauseShape<'static> = clause_shape!(contains_phrases & [&["for", "each"]]);
const ZONE_MOVE_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["this", "way"]]);
const ZONE_MOVE_ADDITIONAL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["additional"]);
const ZONE_MOVE_ROUNDED_DOWN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["rounded", "down"]);
const DRAW_TRAILING_INSTEAD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const DRAW_TRAILING_THEN_PUT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["then", "put"]);
const DRAW_AS_MANY_CARDS_AS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any & [&["as", "many", "card", "as"], &["as", "many", "cards", "as"]]
);
const DRAW_EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["equal", "to"]);
const COUNTER_TARGET_SECOND_SPELL_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "counter", "target", "spell", "thats", "second", "spell", "cast", "this", "turn",
            ],
            &[
                "counter", "target", "spell", "thats", "the", "second", "spell", "cast", "this",
                "turn",
            ],
        ]
);
const COUNTER_UNLESS_PAYS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["pays"]);
const COUNTER_DYNAMIC_PAYMENT_TAIL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [
    &["and"],
    &["or"],
    &["where"],
    &["plus"],
    &["additional"],
    &["equal"],
    &["equals"],
]);
const COUNTER_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const COUNTER_LIFE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["life"]);
const COUNTER_SAME_NAME_AS_SPELL_PATTERN: ClauseShape<'static> = clause_shape!(contains_any_phrases & [
    &[
        &["same", "name", "as", "the", "spell"],
        &["same", "name", "as", "that", "spell"],
    ]
]);


pub(crate) fn parse_move(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    use super::super::grammar::primitives as grammar;
    use winnow::Parser as _;

    // "all counters from <source> onto/to <destination>"
    // "a counter from <source> onto/to <destination>"
    let (after_prefix, move_all) = if let Some(rest) =
        grammar::strip_lexed_prefix_phrase(tokens, &["all", "counters", "from"])
    {
        (rest, true)
    } else if let Some(rest) = grammar::strip_lexed_prefix_phrase(tokens, &["a", "counter", "from"])
    {
        (rest, false)
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported move clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };

    let split = grammar::split_lexed_once_on_separator(after_prefix, || grammar::kw("onto").void())
        .or_else(|| {
            grammar::split_lexed_once_on_separator(after_prefix, || grammar::kw("to").void())
        });
    let Some((from_tokens, to_tokens)) = split else {
        return Err(CardTextError::ParseError(format!(
            "missing move destination (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };

    let from = parse_target_phrase(from_tokens)?;
    let to = parse_target_phrase(to_tokens)?;

    Ok(if move_all {
        EffectAst::subject_verb_move_all_counters(from, to)
    } else {
        EffectAst::subject_verb_move_one_counter(from, to)
    })
}

pub(crate) fn parse_draw(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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
                .is_some_and(|token| ZONE_MOVE_CARD_OR_CARDS_WORD_PATTERN.matches_token(token))
            {
                let trailing = trim_commas(&rest[1..]);
                let trailing_words = crate::runtime_backend::token_word_refs(&trailing);
                if ZONE_MOVE_MINUS_ONE_PATTERN.matches_words(&trailing_words) {
                    value = Value::EventValueOffset(EventValueSpec::Amount, -1);
                    parsed_that_many_minus_one = true;
                } else if ZONE_MOVE_PLUS_ONE_PATTERN.matches_words(&trailing_words) {
                    value = Value::EventValueOffset(EventValueSpec::Amount, 1);
                    parsed_that_many_plus_one = true;
                } else if !trailing_words.is_empty()
                    && !ZONE_MOVE_FOR_EACH_PATTERN.matches_words(&trailing_words)
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
        } else if token_slice_first_is(tokens, "another")
            && token_slice_at_is_any(tokens, 1, &["card", "cards"])
        {
            (Value::Fixed(1), 1)
        } else if token_slice_first_is_any(tokens, &["card", "cards"])
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
        } else if token_slice_first_is(tokens, "up")
            && token_slice_at_is(tokens, 1, "to")
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
            .is_some_and(|token| ZONE_MOVE_ADDITIONAL_WORD_PATTERN.matches_token(token))
        {
            card_word_idx = 1;
        }
        let Some(card_word) = rest.get(card_word_idx).and_then(OwnedLexToken::as_word) else {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        };
        if !ZONE_MOVE_CARD_OR_CARDS_WORD_PATTERN.matches_words(&[card_word]) {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        }
        trim_commas(&rest[card_word_idx + 1..])
    };
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let mut effect = subject_verb_player_resource_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::Draw {
            count: count.clone(),
        },
    );

    if !tail.is_empty() {
        let tail_words = crate::runtime_backend::token_word_refs(&tail);
        if !((parsed_that_many_minus_one
            && ZONE_MOVE_MINUS_ONE_PATTERN.matches_words(&tail_words))
            || (parsed_that_many_plus_one
                && ZONE_MOVE_PLUS_ONE_PATTERN.matches_words(&tail_words)))
        {
            if let Some(parsed) = parse_draw_for_each_player_condition(&tail, effect.clone())? {
                effect = parsed;
            } else {
                let has_for_each = find_window_by(&tail, 2, |window: &[OwnedLexToken]| {
                    token_slice_starts_with(window, &["for", "each"])
                })
                .is_some();
                if has_for_each {
                    let dynamic = parse_dynamic_cost_modifier_value(&tail)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported draw for-each clause (clause: '{}')",
                            crate::runtime_backend::token_word_refs(tokens).join(" ")
                        ))
                    })?;
                    match count {
                        Value::Fixed(1) => count = dynamic,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported multiplied draw count (clause: '{}')",
                                crate::runtime_backend::token_word_refs(tokens).join(" ")
                            )));
                        }
                    }
                    effect = subject_verb_player_resource_effect(
                        SubjectVerbRoleAst::AffectedPlayer,
                        player,
                        SubjectVerbActionAst::Draw {
                            count: count.clone(),
                        },
                    );
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
            PredicateAst::Or(left, right) => PredicateAst::Or(
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
            PredicateAst::PlayerHasAtLeast {
                player,
                filter,
                count,
            } if player == PlayerAst::That => PredicateAst::PlayerHasAtLeast {
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

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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
    let inner_words = crate::runtime_backend::token_word_refs(&inner_tokens);
    if !inner_words
        .first()
        .is_some_and(|word| ZONE_MOVE_WHO_WORD_PATTERN.matches_word(word))
    {
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

    let mut draw_effect = draw_effect;
    match &mut draw_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action: SubjectVerbActionAst::Draw { .. },
        }) if *player == PlayerAst::Implicit => {
            *player = PlayerAst::You;
        }
        _ => {}
    }

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
    if !words
        .first()
        .is_some_and(|word| ZONE_MOVE_HALF_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let mut card_idx = None;
    for idx in 1..words.len() {
        if ZONE_MOVE_CARD_OR_CARDS_WORD_PATTERN.matches_word_at(words, idx)
            && ZONE_MOVE_ROUNDED_DOWN_PREFIX_PATTERN.matches_words(&words[idx + 1..])
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
    let tail_words = crate::runtime_backend::token_word_refs(tokens);
    if DRAW_TRAILING_INSTEAD_PATTERN.matches_words(&tail_words) {
        return Ok(Some(draw_effect));
    }

    if let Some(timing) = parse_draw_delayed_timing_words(&tail_words) {
        return Ok(Some(wrap_return_with_delayed_timing(
            draw_effect,
            Some(timing),
        )));
    }

    if DRAW_TRAILING_THEN_PUT_PREFIX_PATTERN.matches_words(&tail_words) {
        let put_tokens = trim_commas(&tokens[2..]);
        let put_effect = parse_put_into_hand(&put_tokens, None)?;
        return Ok(Some(EffectAst::Sequence {
            effects: vec![draw_effect, put_effect],
        }));
    }

    if tail_words
        .first()
        .is_some_and(|word| DRAW_TRAILING_IF_WORD_PATTERN.matches_word(word))
    {
        let predicate = parse_trailing_if_predicate_lexed(tokens).ok_or_else(|| {
            CardTextError::ParseError("missing condition after trailing if clause".to_string())
        })?;
        return Ok(Some(EffectAst::Conditional {
            predicate,
            if_true: vec![draw_effect],
            if_false: Vec::new(),
        }));
    }

    if tail_words
        .first()
        .is_some_and(|word| DRAW_TRAILING_UNLESS_WORD_PATTERN.matches_word(word))
    {
        return try_build_unless(
            vec![draw_effect],
            SubjectVerbPrimitiveClause::new(tokens),
            0,
        );
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
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !DRAW_AS_MANY_CARDS_AS_PREFIX_PATTERN.matches_words(&clause_words) {
        return None;
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let references_previous_event = ZONE_MOVE_THIS_WAY_PATTERN.matches_words(&clause_words);
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
    let token_words = crate::runtime_backend::token_word_refs(tokens);
    if !DRAW_EQUAL_TO_PREFIX_PATTERN.matches_words(&token_words) {
        return Ok(None);
    }

    if grammar::words_match_prefix(
        tokens,
        &[
            "equal",
            "to",
            "the",
            "greatest",
            "number",
            "of",
            "cards",
            "a",
            "player",
            "discarded",
            "this",
            "way",
        ],
    )
    .is_some()
    {
        return Ok(Some(Value::PendingEffectMetric {
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::GreatestPlayerCount,
        }));
    }

    if let Some(value) = parse_devotion_value_from_add_clause(tokens)? {
        return Ok(Some(value));
    }

    if DRAW_EQUAL_TO_PREFIX_PATTERN.matches_words(&token_words) {
        let value_tokens = &tokens[2..];
        let value_words = crate::runtime_backend::token_word_refs(value_tokens);
        let parse_stat_of_target =
            |stat_words: &[&str], constructor: fn(Box<ChooseSpec>) -> Value| {
                if ClauseShape::new().prefix(stat_words).matches_words(&value_words) {
                    let target_start = token_index_for_word_index(value_tokens, stat_words.len())
                        .unwrap_or(value_tokens.len());
                    let target_tokens = &value_tokens[target_start..];
                    if let Ok(target) = parse_target_phrase(target_tokens) {
                        let spec = crate::runtime_backend::references::reference_helpers::choose_spec_for_target(&target);
                        return Some(constructor(Box::new(spec)));
                    }
                }
                None
            };

        if let Some(value) = parse_stat_of_target(&["power", "of"], Value::PowerOf)
            .or_else(|| parse_stat_of_target(&["the", "power", "of"], Value::PowerOf))
            .or_else(|| parse_stat_of_target(&["toughness", "of"], Value::ToughnessOf))
            .or_else(|| parse_stat_of_target(&["the", "toughness", "of"], Value::ToughnessOf))
            .or_else(|| parse_stat_of_target(&["mana", "value", "of"], Value::ManaValueOf))
            .or_else(|| parse_stat_of_target(&["the", "mana", "value", "of"], Value::ManaValueOf))
        {
            return Ok(Some(value));
        }
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
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if ZONE_MOVE_THIS_WAY_PATTERN.matches_words(&clause_words) {
        return Ok(Some(Value::EventValue(EventValueSpec::Amount)));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
        return Ok(Some(value));
    }

    Ok(None)
}

fn counter_unless_payment_total_cost(
    mana: Vec<ManaSymbol>,
    life: Option<Value>,
    additional_generic: Option<Value>,
    x_value: Option<Value>,
    display_hint: ironsmith_core::DynamicManaDisplayHint,
) -> crate::cost::TotalCost {
    let mut components = Vec::new();
    let mana_cost = crate::mana::ManaCost::from_symbols(mana);
    if !mana_cost.is_empty() || additional_generic.is_some() || x_value.is_some() {
        if mana_cost.has_x() || additional_generic.is_some() || x_value.is_some() {
            components.push(crate::costs::Cost::dynamic_mana(
                ironsmith_core::DynamicManaCost::new(
                    mana_cost,
                    x_value,
                    additional_generic,
                    None,
                    display_hint,
                ),
            ));
        } else {
            components.push(crate::costs::Cost::mana(mana_cost));
        }
    }
    if let Some(life) = life {
        components.push(crate::costs::Cost::life(life));
    }
    crate::cost::TotalCost::from_costs(components)
}

pub(crate) fn parse_counter(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if let Some(spec) = split_trailing_if_clause_lexed(tokens) {
        let target = parse_counter_target_phrase(spec.leading_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate: spec.predicate,
            if_true: vec![EffectAst::subject_verb_counter(target)],
            if_false: Vec::new(),
        });
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let target_spell_second_this_turn =
        COUNTER_TARGET_SECOND_SPELL_THIS_TURN_PATTERN.matches_words(&clause_words);
    if target_spell_second_this_turn {
        return Ok(EffectAst::Conditional {
            predicate: crate::cards::builders::PredicateAst::TargetSpellCastOrderThisTurn(2),
            if_true: vec![EffectAst::subject_verb_counter(TargetAst::Spell(
                span_from_tokens(&tokens[1..3]),
            ))],
            if_false: Vec::new(),
        });
    }

    if super::super::grammar::primitives::contains_word(tokens, "if") {
        return Err(CardTextError::ParseError(format!(
            "missing conditional counter target or predicate (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some((target_tokens, unless_tokens)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("unless").void()
        })
    {
        let target = parse_counter_target_phrase(target_tokens)?;
        let pays_idx = find_index(unless_tokens, |token: &OwnedLexToken| {
            COUNTER_UNLESS_PAYS_WORD_PATTERN.matches_token(token)
        })
        .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing pays keyword (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;

        let mut payment_clause_tokens = unless_tokens[pays_idx..].to_vec();
        if let Some(first) = payment_clause_tokens.first_mut()
            && COUNTER_UNLESS_PAYS_WORD_PATTERN.matches_token(first)
        {
            first.replace_word("pay");
        }
        let payment_clause_words = crate::runtime_backend::token_word_refs(&payment_clause_tokens);
        let has_x_mana_payment = payment_clause_tokens.iter().any(|token| {
            mana_pips_from_token(token)
                .is_some_and(|pips| pips.iter().any(|symbol| matches!(symbol, ManaSymbol::X)))
        });
        let has_dynamic_payment_tail =
            payment_clause_words.iter().any(|word| {
                COUNTER_DYNAMIC_PAYMENT_TAIL_WORD_PATTERN.matches_words(&[*word])
            }) || ZONE_MOVE_FOR_EACH_PATTERN.matches_words(&payment_clause_words)
                || has_x_mana_payment;
        match crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(&payment_clause_tokens) {
            Ok(Some(cost)) => {
                let should_keep_subject_verb_dynamic_path = has_dynamic_payment_tail
                    && cost.as_one_of().is_none()
                    && cost.dynamic_mana_cost().is_none();
                if !should_keep_subject_verb_dynamic_path {
                    return Ok(EffectAst::subject_verb_counter_unless_pays(target, cost));
                }
            }
            Ok(None) => {
                if !has_dynamic_payment_tail {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported counter-unless payment cost (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )));
                }
            }
            Err(err) => {
                if !has_dynamic_payment_tail {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported counter-unless payment cost (clause: '{}'): {err}",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )));
                }
            }
        }

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
        let mut x_value = None;
        let mut dynamic_display_hint = ironsmith_core::DynamicManaDisplayHint::Default;
        if mana.is_empty() {
            let payment_tokens = trim_commas(&unless_tokens[pays_idx + 1..]);
            let payment_words = crate::runtime_backend::token_word_refs(&payment_tokens);
            // "unless its controller pays mana equal to ..." uses a dynamic generic payment.
            if payment_words
                .first()
                .is_some_and(|word| COUNTER_MANA_WORD_PATTERN.matches_word(word))
                && let Some(value) = parse_equal_to_aggregate_filter_value(&payment_tokens)
                    .or_else(|| parse_equal_to_number_of_filter_value(&payment_tokens))
            {
                additional_generic = Some(value);
                dynamic_display_hint = ironsmith_core::DynamicManaDisplayHint::ManaEqualTo;
                trailing_start = None;
            } else {
                return Err(CardTextError::ParseError(format!(
                    "missing mana cost (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
        }

        if let Some(trailing_idx) = trailing_start {
            let trailing_tokens = trim_commas(&unless_tokens[trailing_idx..]);
            let trailing_words = crate::runtime_backend::token_word_refs(&trailing_tokens);
            if trailing_tokens
                .first()
                .is_some_and(|token| COUNTER_AND_WORD_PATTERN.matches_token(token))
            {
                let life_tokens = trim_commas(&trailing_tokens[1..]);
                if let Some((amount, used)) = parse_value(&life_tokens)
                    && life_tokens
                        .get(used)
                        .is_some_and(|token| COUNTER_LIFE_WORD_PATTERN.matches_token(token))
                    && trim_commas(&life_tokens[used + 1..]).is_empty()
                {
                    life = Some(amount);
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" "),
                        trailing_words.join(" ")
                    )));
                }
            } else if let Some(value) =
                parse_counter_unless_additional_generic_value(&trailing_tokens)?
            {
                additional_generic = Some(value);
            } else if grammar::words_match_any_prefix(
                &trailing_tokens,
                &[&["where", "x", "is"], &["where", "x", "equals"]],
            )
            .is_some()
                && trailing_words
                    .iter()
                    .any(|word| ZONE_MOVE_GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN.matches_word(word))
                && COUNTER_SAME_NAME_AS_SPELL_PATTERN.matches_words(&trailing_words)
            {
                if mana.as_slice() == [ManaSymbol::X] {
                    x_value = Some(Value::Count(
                        ObjectFilter::default()
                            .in_zone(Zone::Graveyard)
                            .match_tagged(
                                TagKey::from("triggering"),
                                crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
                            ),
                    ));
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" "),
                        trailing_words.join(" ")
                    )));
                }
            } else if let Some(value) = parse_value_binding_clause(&trailing_tokens) {
                if mana.as_slice() == [ManaSymbol::X] {
                    x_value = Some(value);
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" "),
                        trailing_words.join(" ")
                    )));
                }
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
                            crate::runtime_backend::token_word_refs(tokens).join(" "),
                            trailing_words.join(" ")
                        )));
                    }
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" "),
                        trailing_words.join(" ")
                    )));
                }
            } else if !trailing_words.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" "),
                    trailing_words.join(" ")
                )));
            }
        }

        if mana.is_empty() && life.is_none() && additional_generic.is_none() && x_value.is_none() {
            return Err(CardTextError::ParseError(format!(
                "missing mana cost (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }

        if x_value.is_none()
            && mana.as_slice() == [ManaSymbol::X]
            && let Some(where_idx) =
                crate::runtime_backend::lexer::find_token_word(&unless_tokens, "where")
        {
            let where_tokens = trim_commas(&unless_tokens[where_idx..]);
            let where_words = crate::runtime_backend::token_word_refs(&where_tokens);
            x_value = parse_value_binding_clause(&where_tokens).or_else(|| {
                if where_words
                    .iter()
                    .any(|word| ZONE_MOVE_GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN.matches_word(word))
                    && COUNTER_SAME_NAME_AS_SPELL_PATTERN.matches_words(&where_words)
                {
                    Some(Value::Count(
                        ObjectFilter::default()
                            .in_zone(Zone::Graveyard)
                            .match_tagged(
                                TagKey::from("triggering"),
                                crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
                            ),
                    ))
                } else {
                    None
                }
            });
        }

        return Ok(EffectAst::subject_verb_counter_unless_pays(
            target,
            counter_unless_payment_total_cost(
                mana,
                life,
                additional_generic,
                x_value,
                dynamic_display_hint,
            ),
        ));
    }

    let target = parse_counter_target_phrase(tokens)?;
    Ok(EffectAst::subject_verb_counter(target))
}
