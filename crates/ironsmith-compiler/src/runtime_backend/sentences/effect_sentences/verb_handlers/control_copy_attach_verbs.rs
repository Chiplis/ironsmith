const CCA_LIFE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["life"]);
const CCA_THE_GAME_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the", "game"]);
const CCA_UNLESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["unless"]);
const CCA_DURATION_START_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["during"], &["until"]]);
const CCA_OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const CCA_THOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["those"]);
const CCA_IT_OR_THEM_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"]]);
const CCA_CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const CCA_HAND_OR_HANDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["hand"], &["hands"]]);
const CCA_GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["graveyard"], &["graveyards"]]);
const CCA_LIBRARY_OR_LIBRARIES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["library"], &["libraries"]]);
const CCA_REST_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["the", "rest"], &["rest"]]);
const CCA_AND_OR_THEN_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["and", "then"]]);
const CCA_ON_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["on"]);
const CCA_THAT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that"]);
const CCA_ATTACHED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["attached"]);
const CCA_THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const CCA_CHOICE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["choice"]);
const CCA_EITHER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["either"]);
const CCA_TOP_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["top"]);
const CCA_OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const CCA_BOTTOM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["bottom"]);
const CCA_PUT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["put"]);
const CCA_BATTLEFIELD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["battlefield"]);
const CCA_FROM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["from"]);
const CCA_COMMAND_ZONE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["command", "zone"]);
const CCA_DESTINATION_IGNORED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["tapped"], &["attacking"]]);
const CCA_ATTACHED_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["attached", "to"]);
const CCA_UNDER_YOUR_CONTROL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["under", "your", "control"]);
const CCA_OWNER_CONTROL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["under", "its", "owners", "control"],
            &["under", "his", "owners", "control"],
            &["under", "her", "owners", "control"],
            &["under", "their", "owners", "control"],
            &["under", "that", "players", "control"],
        ]
);
const CCA_ALL_OR_EACH_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["all"], &["each"]]);
const CCA_OWNER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["owner"], &["owners"], &["owner's"], &["owners'"]]);
const CCA_PLAYER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["player"],
            &["players"],
            &["player's"],
            &["players'"],
        ]
);
const CCA_FOR_AS_LONG_AS_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["for", "as", "long", "as"]]);
const CCA_YOU_CONTROL_SOURCE_MARKER_PATTERN: ClauseShape<'static> =
    ClauseShape::new()
        .contains_words(&["you", "control"])
        .contains_any_words(&[&["this"], &["thiss"], &["source"], &["creature"], &["permanent"]]);
const CCA_DURING_NEXT_TURN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["during", "next", "turn"]);
const CCA_UNTIL_END_NEXT_TURN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["until", "end", "next", "turn"]);
const CCA_UNTIL_END_TURN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["until", "end", "turn"]);
const CCA_BACK_ANY_ORDER_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["back", "any", "order"]);
const CCA_FROM_AMONG_HAND_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["from", "among", "hand"]);
const CCA_REST_TOP_BOTTOM_LIBRARY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["rest", "top", "bottom", "library"]);
const CCA_YOUR_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["your"]);
const CCA_THEIR_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["their"]);
const CCA_THAT_PLAYER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["that", "player"], &["that", "players"]]);
const CCA_REST_BOTTOM_LIBRARY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["rest", "bottom", "library"]);
const CCA_REST_GRAVEYARD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["rest", "graveyard"]);
const CCA_LIBRARY_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["library"]);
const CCA_POWER_NUMBER_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["power", "number"]);
const CCA_YOU_CONTROL_PHRASE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["you", "control"]]);
const CCA_IT_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["it"]);
const CCA_THEM_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["them"]);
const CCA_HAND_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["hand"]);
const CCA_INTO_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["into"]);
const CCA_ATTACKING_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["attacking"]);
const CCA_TAPPED_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["tapped"]);
const CCA_AMONG_THEM_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["among", "them"]);
const CCA_PERMANENT_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["permanent"]);
const CCA_STICKER_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["sticker"]);

fn parse_put_choice_count_prefix(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(ChoiceCount, usize), CardTextError> {
    parse_choice_count_token_prefix_consumed(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing put count (clause: '{}')",
            clause_words.join(" ")
        ))
    })
}

fn parse_counted_card_target_prefix(
    target_tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let Some((count, used)) = parse_choice_count_token_prefix_consumed(target_tokens) else {
        return Ok(None);
    };
    if !target_tokens
        .get(used)
        .is_some_and(|token| CCA_CARD_OR_CARDS_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }
    let inner = parse_target_phrase(&target_tokens[used..])?;
    Ok(Some(TargetAst::WithCount(Box::new(inner), count)))
}

fn cca_destination_player_from_words(words: &[&str], fallback: PlayerAst) -> PlayerAst {
    if CCA_YOUR_MARKER_PATTERN.matches_words(words) {
        PlayerAst::You
    } else if CCA_THEIR_MARKER_PATTERN.matches_words(words)
        || CCA_THAT_PLAYER_PREFIX_PATTERN.matches_words(words)
    {
        PlayerAst::That
    } else {
        fallback
    }
}

pub(crate) fn parse_lose_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::runtime_backend::token_word_refs(tokens);

    if clause_words.len() == 2
        && CCA_LIFE_WORD_PATTERN.matches_word(clause_words[1])
        && let Some((amount, _)) = parse_number(tokens)
    {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife {
                amount: Value::Fixed(amount as i32),
            },
        ));
    }
    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
            && (grammar::words_find_phrase(tokens, &["its", "power"]).is_some()
                || grammar::words_find_phrase(tokens, &["its", "toughness"]).is_some()
                || grammar::words_find_phrase(tokens, &["its", "mana", "value"]).is_some())
        {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        ));
    }
    if CCA_THE_GAME_PATTERN.matches_words(&clause_words) {
        return Ok(EffectAst::subject_verb_lose_game(player));
    }

    if let Some(amount) = parse_half_life_value(tokens, player) {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        ));
    }

    let (mut amount, used) = parse_life_amount(tokens, "life loss")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(subject_verb_player_resource_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::LoseLife { amount },
            ));
        }
        let base_effect = subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        );
        if let Some(predicate) = parse_trailing_if_predicate_lexed(&trailing) {
            return Ok(EffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            });
        }
        if trailing
            .first()
            .is_some_and(|token| CCA_UNLESS_WORD_PATTERN.matches_token(token))
        {
            let mut unless_as_if_tokens = Vec::with_capacity(trailing.len() + 1);
            unless_as_if_tokens.push(OwnedLexToken::word("if".to_string(), TextSpan::synthetic()));
            unless_as_if_tokens.extend_from_slice(&trailing[1..]);
            if let Some(predicate) = parse_trailing_if_predicate_lexed(&unless_as_if_tokens) {
                return Ok(EffectAst::Conditional {
                    predicate,
                    if_true: Vec::new(),
                    if_false: vec![base_effect],
                });
            }
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-loss clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(subject_verb_player_resource_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::LoseLife { amount },
    ))
}

pub(crate) fn parse_gain_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
            && (grammar::words_find_phrase(tokens, &["its", "power"]).is_some()
                || grammar::words_find_phrase(tokens, &["its", "toughness"]).is_some()
                || grammar::words_find_phrase(tokens, &["its", "mana", "value"]).is_some())
        {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::GainLife { amount },
        ));
    }

    let (mut amount, used) = parse_life_amount(tokens, "life gain")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if grammar::words_find_phrase(
            &trailing,
            &["then", "shuffle", "your", "graveyard", "into", "your"],
        )
        .is_some()
            && CCA_LIBRARY_MARKER_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(
                &trailing,
            ))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing life-gain shuffle-graveyard clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(subject_verb_player_resource_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::GainLife { amount },
            ));
        }
        let base_effect = subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::GainLife { amount },
        );
        if let Some(predicate) = parse_trailing_if_predicate_lexed(&trailing) {
            return Ok(EffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            });
        }
        if trailing
            .first()
            .is_some_and(|token| CCA_UNLESS_WORD_PATTERN.matches_token(token))
        {
            let mut unless_as_if_tokens = Vec::with_capacity(trailing.len() + 1);
            unless_as_if_tokens.push(OwnedLexToken::word("if".to_string(), TextSpan::synthetic()));
            unless_as_if_tokens.extend_from_slice(&trailing[1..]);
            if let Some(predicate) = parse_trailing_if_predicate_lexed(&unless_as_if_tokens) {
                return Ok(EffectAst::Conditional {
                    predicate,
                    if_true: Vec::new(),
                    if_false: vec![base_effect],
                });
            }
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-gain clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(subject_verb_player_resource_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::GainLife { amount },
    ))
}

pub(crate) fn parse_gain_control(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let has_dynamic_power_bound = CCA_POWER_NUMBER_MARKER_PATTERN.matches_words(&clause_words)
        && CCA_YOU_CONTROL_PHRASE_MARKER_PATTERN.matches_words(&clause_words);
    if has_dynamic_power_bound {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic power-bound control clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut idx = 0;
    if token_slice_at_is(tokens, idx, "control") {
        idx += 1;
    } else {
        return Err(CardTextError::ParseError(
            "missing control keyword".to_string(),
        ));
    }

    if token_slice_at_is(tokens, idx, "of") {
        idx += 1;
    }

    let duration_idx = find_index(&tokens[idx..], |token: &OwnedLexToken| {
        CCA_DURATION_START_WORD_PATTERN.matches_token(token)
    })
    .map(|offset| idx + offset)
    .or_else(|| {
        find_window_by(&tokens[idx..], 4, |window: &[OwnedLexToken]| {
            token_slice_starts_with(window, &["for", "as", "long", "as"])
        })
        .map(|offset| idx + offset)
    });

    let target_tokens = if let Some(dur_idx) = duration_idx {
        &tokens[idx..dur_idx]
    } else {
        &tokens[idx..]
    };
    let invalid_conditional_error = || {
        CardTextError::ParseError(format!(
            "unsupported conditional gain-control clause (clause: '{}')",
            clause_words.join(" ")
        ))
    };
    let (target_ast, trailing_predicate, is_unless) =
        if let Some(spec) = split_trailing_if_clause_lexed(target_tokens) {
            (
                parse_target_phrase(spec.leading_tokens)?,
                Some(spec.predicate),
                false,
            )
        } else if crate::runtime_backend::lexer::contains_token_word(target_tokens, "if") {
            return Err(invalid_conditional_error());
        } else if let Some(spec) = split_trailing_unless_clause_lexed(target_tokens) {
            (
                parse_target_phrase(spec.leading_tokens)?,
                Some(spec.predicate),
                true,
            )
        } else if crate::runtime_backend::lexer::contains_token_word(target_tokens, "unless") {
            return Err(invalid_conditional_error());
        } else {
            (parse_target_phrase(target_tokens)?, None, false)
        };
    let duration_tokens = duration_idx
        .map(|dur_idx| &tokens[dur_idx..])
        .unwrap_or(&[]);
    let duration = parse_control_duration(duration_tokens)?;
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let base_effect = match target_ast {
        TargetAst::Player(filter, _) => {
            if matches!(duration, ControlDurationAst::UntilYourNextTurnEnd) {
                return Err(CardTextError::ParseError(
                    "unsupported player-control duration until the end of your next turn"
                        .to_string(),
                ));
            }
            EffectAst::subject_verb_control_player(
                player,
                PlayerFilter::Target(Box::new(filter)),
                duration,
            )
        }
        _ => {
            let until = match duration {
                ControlDurationAst::UntilEndOfTurn => Until::EndOfTurn,
                ControlDurationAst::UntilYourNextTurnEnd => Until::YourNextTurnEnd,
                ControlDurationAst::Forever => Until::Forever,
                ControlDurationAst::AsLongAsYouControlSource => Until::YouStopControllingThis,
                ControlDurationAst::DuringNextTurn => {
                    return Err(CardTextError::ParseError(
                        "unsupported control duration for permanents".to_string(),
                    ));
                }
            };
            EffectAst::subject_verb_gain_control(player, target_ast, until)
        }
    };

    if let Some(predicate) = trailing_predicate {
        return Ok(if is_unless {
            EffectAst::Conditional {
                predicate,
                if_true: Vec::new(),
                if_false: vec![base_effect],
            }
        } else {
            EffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            }
        });
    }

    Ok(base_effect)
}

pub(crate) fn parse_control_duration(
    tokens: &[OwnedLexToken],
) -> Result<ControlDurationAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(ControlDurationAst::Forever);
    }

    let words = crate::runtime_backend::token_word_refs(tokens);
    if CCA_FOR_AS_LONG_AS_MARKER_PATTERN.matches_words(&words)
        && CCA_YOU_CONTROL_SOURCE_MARKER_PATTERN.matches_words(&words)
    {
        return Ok(ControlDurationAst::AsLongAsYouControlSource);
    }

    if CCA_DURING_NEXT_TURN_MARKER_PATTERN.matches_words(&words) {
        return Ok(ControlDurationAst::DuringNextTurn);
    }

    if CCA_UNTIL_END_NEXT_TURN_MARKER_PATTERN.matches_words(&words) {
        return Ok(ControlDurationAst::UntilYourNextTurnEnd);
    }
    if CCA_UNTIL_END_TURN_MARKER_PATTERN.matches_words(&words) {
        return Ok(ControlDurationAst::UntilEndOfTurn);
    }

    Err(CardTextError::ParseError(
        "unsupported control duration".to_string(),
    ))
}

pub(crate) fn parse_put_into_hand(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_put_into_hand_delayed_timing(
        tokens: &[OwnedLexToken],
    ) -> Option<DelayedReturnTimingAst> {
        let hand_idx = rfind_index(tokens, |token: &OwnedLexToken| {
            CCA_HAND_OR_HANDS_WORD_PATTERN.matches_token(token)
        })?;
        let tail_tokens = trim_commas(&tokens[hand_idx + 1..]);
        let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
        parse_delayed_return_timing_words(&tail_words)
    }

    fn force_object_targeting(target: TargetAst, span: TextSpan) -> TargetAst {
        match target {
            TargetAst::Object(filter, explicit_span, fixed_span) => {
                TargetAst::Object(filter, explicit_span.or(Some(span)), fixed_span)
            }
            TargetAst::WithCount(inner, count) => {
                TargetAst::WithCount(Box::new(force_object_targeting(*inner, span)), count)
            }
            other => other,
        }
    }

    fn expand_graveyard_or_hand_disjunction(
        mut target: TargetAst,
        target_tokens: &[OwnedLexToken],
    ) -> TargetAst {
        let target_words = crate::runtime_backend::token_word_refs(target_tokens);
        let has_graveyard = target_words
            .iter()
            .any(|word| CCA_GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN.matches_word(word));
        let has_hand = target_words
            .iter()
            .any(|word| CCA_HAND_OR_HANDS_WORD_PATTERN.matches_word(word));
        if !(has_graveyard && has_hand) {
            return target;
        }

        fn apply(filter: &ObjectFilter) -> ObjectFilter {
            let mut graveyard = filter.clone();
            graveyard.any_of.clear();
            graveyard.zone = Some(Zone::Graveyard);

            let mut hand = filter.clone();
            hand.any_of.clear();
            hand.zone = Some(Zone::Hand);

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![graveyard, hand];
            disjunction
        }

        match &mut target {
            TargetAst::Object(filter, _, _) => {
                *filter = apply(filter);
            }
            TargetAst::WithCount(inner, _) => {
                if let TargetAst::Object(filter, _, _) = inner.as_mut() {
                    *filter = apply(filter);
                }
            }
            _ => {}
        }

        target
    }

    fn apply_source_zone_constraint(target: &mut TargetAst, zone: Zone) {
        match target {
            TargetAst::Source(span) => {
                *target = TargetAst::Object(ObjectFilter::source().in_zone(zone), *span, None);
            }
            TargetAst::Object(filter, _, _) => {
                filter.zone = Some(zone);
            }
            TargetAst::WithCount(inner, _) => apply_source_zone_constraint(inner, zone),
            _ => {}
        }
    }

    fn is_top_or_bottom_choice_destination(tokens: &[OwnedLexToken]) -> bool {
        let words = crate::runtime_backend::token_word_refs(tokens);
        let mut idx = 0usize;

        match words.get(idx).copied() {
            Some("their" | "his" | "her" | "your") => {
                idx += 1;
            }
            Some("its") => {
                idx += 1;
                if words.get(idx).copied().is_some_and(|word| {
                    CCA_OWNER_WORD_PATTERN.matches_word(word)
                }) {
                    idx += 1;
                }
            }
            Some("that") if words.get(idx + 1).copied().is_some_and(|word| {
                CCA_PLAYER_WORD_PATTERN.matches_word(word)
            }) =>
            {
                idx += 2;
            }
            Some(word) if CCA_OWNER_WORD_PATTERN.matches_word(word) => {
                idx += 1;
            }
            _ => {}
        }

        if !CCA_CHOICE_WORD_PATTERN.matches_word_at(&words, idx) {
            return false;
        }
        idx += 1;
        if !CCA_OF_WORD_PATTERN.matches_word_at(&words, idx) {
            return false;
        }
        idx += 1;
        if CCA_EITHER_WORD_PATTERN.matches_word_at(&words, idx) {
            idx += 1;
        }
        if CCA_THE_WORD_PATTERN.matches_word_at(&words, idx) {
            idx += 1;
        }

        let top_or_bottom = CCA_TOP_WORD_PATTERN.matches_word_at(&words, idx)
            && CCA_OR_WORD_PATTERN.matches_word_at(&words, idx + 1)
            && CCA_BOTTOM_WORD_PATTERN.matches_word_at(&words, idx + 2);
        let bottom_or_top = CCA_BOTTOM_WORD_PATTERN.matches_word_at(&words, idx)
            && CCA_OR_WORD_PATTERN.matches_word_at(&words, idx + 1)
            && CCA_TOP_WORD_PATTERN.matches_word_at(&words, idx + 2);
        if !(top_or_bottom || bottom_or_top) {
            return false;
        }
        idx += 3;
        if !CCA_OF_WORD_PATTERN.matches_word_at(&words, idx) {
            return false;
        }
        words[idx + 1..]
            .iter()
            .any(|word| CCA_LIBRARY_OR_LIBRARIES_WORD_PATTERN.matches_word(word))
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::runtime_backend::token_word_refs(tokens);

    fn parse_counted_those_cards_target(tokens: &[OwnedLexToken]) -> Option<u32> {
        let tokens = trim_commas(tokens);
        let words = crate::runtime_backend::token_word_refs(&tokens);
        if !CCA_PUT_WORD_PATTERN.matches_first_word(&words) {
            return None;
        }

        let count_tokens = &tokens[1..];
        let (count, used) = parse_number(count_tokens)?;
        let mut idx = used;
        if count_tokens
            .get(idx)
            .is_some_and(|token: &OwnedLexToken| CCA_OF_WORD_PATTERN.matches_token(token))
        {
            idx += 1;
        }
        if !count_tokens
            .get(idx)
            .is_some_and(|token: &OwnedLexToken| CCA_THOSE_WORD_PATTERN.matches_token(token))
        {
            return None;
        }
        idx += 1;
        if !count_tokens
            .get(idx)
            .is_some_and(|token: &OwnedLexToken| CCA_CARD_OR_CARDS_WORD_PATTERN.matches_token(token))
        {
            return None;
        }
        idx += 1;

        if idx != count_tokens.len() {
            return None;
        }
        Some(count as u32)
    }

    // "Put them/it back in any order." (typically after looking at the top cards of a library).
    if CCA_BACK_ANY_ORDER_MARKER_PATTERN.matches_words(&clause_words)
        && CCA_IT_OR_THEM_WORD_PATTERN.matches_first_word(&clause_words)
    {
        return Ok(EffectAst::subject_verb_reorder_top_of_library(TagKey::from(IT_TAG)));
    }

    if CCA_FROM_AMONG_HAND_MARKER_PATTERN.matches_words(&clause_words) {
        return Ok(EffectAst::subject_verb_put_some_into_hand_rest_into_graveyard(
            player, 1,
        ));
    }
    let has_it = CCA_IT_MARKER_PATTERN.matches_words(&clause_words);
    let has_them = CCA_THEM_MARKER_PATTERN.matches_words(&clause_words);
    let has_hand = CCA_HAND_MARKER_PATTERN.matches_words(&clause_words);
    let has_into = CCA_INTO_MARKER_PATTERN.matches_words(&clause_words);

    // "Put one of those cards on top of your library and the rest on the bottom of your library"
    if CCA_REST_TOP_BOTTOM_LIBRARY_MARKER_PATTERN.matches_words(&clause_words)
        && CCA_AND_OR_THEN_WORD_PATTERN.matches_words(&clause_words)
    {
        let (choice_count, used) = parse_put_choice_count_prefix(tokens, &clause_words)?;

        let mut idx = used;
        if token_slice_at_is(tokens, idx, "of") {
            idx += 1;
        }
        if token_slice_at_is(tokens, idx, "them") {
            idx += 1;
        } else if token_slice_at_is(tokens, idx, "those") {
            idx += 1;
            if token_slice_at_is_any(tokens, idx, &["card", "cards"]) {
                idx += 1;
            }
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported library rearrange put clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        if !token_slice_at_is(tokens, idx, "on") || !token_slice_at_is(tokens, idx + 1, "top")
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported library rearrange put clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let library_owner = cca_destination_player_from_words(&clause_words, player);

        return Ok(EffectAst::subject_verb_rearrange_looked_cards_in_library(
            library_owner,
            TagKey::from(IT_TAG),
            choice_count,
        ));
    }

    if has_hand && has_into && (has_it || has_them) {
        // "Put N of them into your hand and the rest on the bottom of your library in any order."
        if has_them
            && CCA_REST_BOTTOM_LIBRARY_MARKER_PATTERN.matches_words(&clause_words)
            && CCA_AND_OR_THEN_WORD_PATTERN.matches_words(&clause_words)
        {
            let (choice_count, used) = parse_put_choice_count_prefix(tokens, &clause_words)?;
            let mut idx = used;
            if token_slice_at_is(tokens, idx, "of") {
                idx += 1;
            }
            if !token_slice_at_is(tokens, idx, "them") {
                return Err(CardTextError::ParseError(format!(
                    "unsupported multi-destination put clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let dest_player = cca_destination_player_from_words(&clause_words, player);

            return Ok(EffectAst::subject_verb_put_some_into_hand_rest_on_bottom_of_library_with_count(dest_player, choice_count));
        }

        // "Put N of them into your hand and the rest into your graveyard."
        if has_them
            && CCA_REST_GRAVEYARD_MARKER_PATTERN.matches_words(&clause_words)
            && CCA_AND_OR_THEN_WORD_PATTERN.matches_words(&clause_words)
        {
            let (choice_count, used) = parse_put_choice_count_prefix(tokens, &clause_words)?;
            // Accept optional "of" before "them".
            let mut idx = used;
            if token_slice_at_is(tokens, idx, "of") {
                idx += 1;
            }
            if !token_slice_at_is(tokens, idx, "them") {
                return Err(CardTextError::ParseError(format!(
                    "unsupported multi-destination put clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            // The chooser is typically the player whose hand is referenced.
            let dest_player = cca_destination_player_from_words(&clause_words, player);

            return Ok(EffectAst::subject_verb_put_some_into_hand_rest_into_graveyard_with_count(dest_player, choice_count));
        }

        let effect = EffectAst::subject_verb_put_into_hand(player, ObjectRefAst::Tagged(TagKey::from(IT_TAG)));
        return Ok(wrap_return_with_delayed_timing(
            effect,
            parse_put_into_hand_delayed_timing(tokens),
        ));
    }

    // Support destination-first wording:
    // "Put onto the battlefield under your control all creature cards ..."
    if crate::runtime_backend::lexer::token_slice_first_is(tokens, "onto") {
        let mut idx = 1usize;
        while tokens
            .get(idx)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(is_article)
        {
            idx += 1;
        }
        if !tokens
            .get(idx)
            .is_some_and(|token| CCA_BATTLEFIELD_WORD_PATTERN.matches_token(token))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        idx += 1;

        let mut battlefield_tapped = false;
        if token_slice_at_is(tokens, idx, "tapped") {
            battlefield_tapped = true;
            idx += 1;
        }

        let mut battlefield_controller = ReturnControllerAst::Preserve;
        if token_slice_at_is(tokens, idx, "under") {
            let controller_words = crate::runtime_backend::token_word_refs(&tokens[idx..]);
            let consumed = if CCA_UNDER_YOUR_CONTROL_PATTERN.matches_words(&controller_words) {
                battlefield_controller = ReturnControllerAst::You;
                Some(3usize)
            } else if CCA_OWNER_CONTROL_TAIL_PATTERN.matches_words(&controller_words) {
                battlefield_controller = ReturnControllerAst::Owner;
                Some(4usize)
            } else {
                None
            };
            if let Some(consumed) = consumed {
                idx += consumed;
            }
        }

        let target_tokens = trim_commas(&tokens[idx..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        if crate::runtime_backend::lexer::token_slice_first_is(&target_tokens, "attached")
            && crate::runtime_backend::lexer::token_slice_at_is(&target_tokens, 1, "to")
        {
            let after_to = &target_tokens[2..];
            if after_to.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attachment target after 'attached to' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let attachment_target_len = if crate::runtime_backend::lexer::token_slice_first_is(after_to, "it") {
                1usize
            } else if after_to.len() >= 2
                && CCA_THAT_WORD_PATTERN.matches_token(&after_to[0])
                && after_to[1].as_word().is_some_and(|word| {
                    matches!(
                        word,
                        "creature" | "permanent" | "object" | "aura" | "equipment"
                    )
                })
            {
                2usize
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported attachment target after 'attached to' (clause: '{}')",
                    clause_words.join(" ")
                )));
            };

            let attachment_target = parse_target_phrase(&after_to[..attachment_target_len])?;
            let object_tokens = trim_commas(&after_to[attachment_target_len..]);
            if object_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing object after attachment target (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let mut object_target = parse_target_phrase(&object_tokens)?;
            object_target = expand_graveyard_or_hand_disjunction(object_target, &object_tokens);
            object_target = force_object_targeting(object_target, tokens[0].span());

            return Ok(EffectAst::subject_verb_move_to_zone(
                object_target,
                Zone::Battlefield,
                false,
                battlefield_controller,
                battlefield_tapped,
                Some(attachment_target),
            ));
        }

        if !target_tokens
            .first()
            .is_some_and(|token| CCA_ATTACHED_WORD_PATTERN.matches_token(token))
        {
            if target_tokens
                .first()
                .is_some_and(|token| CCA_ALL_OR_EACH_WORD_PATTERN.matches_token(token))
            {
                let filter = parse_object_filter(&target_tokens[1..], false)?;
                return Ok(EffectAst::subject_verb_return_all_to_battlefield(
                    filter,
                    battlefield_tapped,
                    battlefield_controller,
                ));
            }
            let mut rewritten = target_tokens;
            rewritten.push(OwnedLexToken::word("onto".to_string(), tokens[0].span()));
            rewritten.extend_from_slice(&tokens[1..idx]);
            return parse_put_into_hand(&rewritten, subject);
        }
    }

    if let Some(on_idx) = find_index(tokens, |token| CCA_ON_WORD_PATTERN.matches_token(token))
        && is_top_or_bottom_choice_destination(&tokens[on_idx + 1..])
    {
        let target_tokens = trim_commas(&tokens[..on_idx]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before top-or-bottom library choice (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target = if let Some(target) = parse_counted_card_target_prefix(&target_tokens)? {
            target
        } else {
            parse_target_phrase(&target_tokens)?
        };
        return Ok(EffectAst::subject_verb_move_to_library_top_or_bottom_choice(
            target,
        ));
    }

    if let Some((target_slice, after_on_top_of)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::phrase(&["on", "top", "of"]).void()
        })
    {
        let target_tokens = trim_commas(target_slice);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'on top of' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if !super::super::grammar::primitives::contains_word(after_on_top_of, "library") {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'on top of' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target = if let Some(target) = parse_counted_card_target_prefix(&target_tokens)? {
            target
        } else {
            parse_target_phrase(&target_tokens)?
        };
        return Ok(EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Library,
            true,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
    }

    if let Some(on_idx) = find_index(tokens, |token| CCA_ON_WORD_PATTERN.matches_token(token)) {
        let mut bottom_idx = on_idx + 1;
        if tokens
            .get(bottom_idx)
            .is_some_and(|token| CCA_THE_WORD_PATTERN.matches_token(token))
        {
            bottom_idx += 1;
        }
        if tokens
            .get(bottom_idx)
            .is_some_and(|token| CCA_BOTTOM_WORD_PATTERN.matches_token(token))
            && tokens
                .get(bottom_idx + 1)
                .is_some_and(|token| CCA_OF_WORD_PATTERN.matches_token(token))
        {
            let target_tokens = trim_commas(&tokens[..on_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing target before 'on bottom of' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if !CCA_LIBRARY_MARKER_PATTERN
                .matches_words(&crate::runtime_backend::token_word_refs(&tokens[bottom_idx + 2..]))
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported put destination after 'on bottom of' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
            let is_rest_target = CCA_REST_TARGET_PATTERN.matches_words(&target_words);
            if is_rest_target {
                return Ok(EffectAst::subject_verb_put_rest_on_bottom_of_library());
            }

            let target = if let Some(target) = parse_counted_card_target_prefix(&target_tokens)? {
                target
            } else {
                parse_target_phrase(&target_tokens)?
            };

            return Ok(EffectAst::subject_verb_move_to_zone(
                target,
                Zone::Library,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ));
        }
    }

    if let Some((target_slice, destination_tokens)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("into").void()
        })
    {
        let target_tokens = trim_commas(target_slice);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'into' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let zone = if super::super::grammar::primitives::contains_word(destination_tokens, "hand")
            || super::super::grammar::primitives::contains_word(destination_tokens, "hands")
        {
            Some(Zone::Hand)
        } else if super::super::grammar::primitives::contains_word(destination_tokens, "graveyard")
            || super::super::grammar::primitives::contains_word(destination_tokens, "graveyards")
        {
            Some(Zone::Graveyard)
        } else if let Some(position) = parse_library_nth_from_top_destination(destination_tokens) {
            let target = parse_target_phrase(&target_tokens)?;
            return Ok(EffectAst::subject_verb_move_to_library_nth_from_top(
                target, position,
            ));
        } else {
            None
        };

        if let Some(zone) = zone {
            let delayed_hand_timing = if zone == Zone::Hand {
                parse_put_into_hand_delayed_timing(tokens)
            } else {
                None
            };
            let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
            if zone == Zone::Graveyard && CCA_REST_TARGET_PATTERN.matches_words(&target_words) {
                return Ok(EffectAst::subject_verb_move_to_zone(
                    TargetAst::Object(
                        ObjectFilter::tagged(TagKey::from(IT_TAG)),
                        None,
                        None,
                    ),
                    zone,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ));
            }

            if zone == Zone::Hand {
                if let Some(count) = parse_counted_those_cards_target(&target_tokens)
                    && CCA_REST_GRAVEYARD_MARKER_PATTERN
                        .matches_words(&crate::runtime_backend::token_word_refs(destination_tokens))
                    && CCA_AND_OR_THEN_WORD_PATTERN.matches_words(&clause_words)
                {
                    let dest_player = cca_destination_player_from_words(&clause_words, player);

                    return Ok(EffectAst::subject_verb_put_some_into_hand_rest_into_graveyard(
                        dest_player,
                        count,
                    ));
                }

                if matches!(
                    target_words.as_slice(),
                    ["it"] | ["them"] | ["that", "card"] | ["those", "card"] | ["those", "cards"]
                ) {
                    let effect = EffectAst::subject_verb_put_into_hand(player, ObjectRefAst::Tagged(TagKey::from(IT_TAG)));
                    return Ok(wrap_return_with_delayed_timing(effect, delayed_hand_timing));
                }
            }

            let effect = EffectAst::subject_verb_move_to_zone(
                parse_target_phrase(&target_tokens)?,
                zone,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            );
            return Ok(if zone == Zone::Hand {
                wrap_return_with_delayed_timing(effect, delayed_hand_timing)
            } else {
                effect
            });
        }
    }

    if let Some((target_slice, dest_slice)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("onto").void()
        })
    {
        let target_tokens = trim_commas(target_slice);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let destination_tokens: Vec<OwnedLexToken> = dest_slice
            .iter()
            .filter(|token| !token.as_word().is_some_and(is_article))
            .cloned()
            .collect();
        if !destination_tokens
            .first()
            .is_some_and(|token| CCA_BATTLEFIELD_WORD_PATTERN.matches_token(token))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let mut destination_tail: Vec<OwnedLexToken> = destination_tokens[1..].to_vec();
        let destination_tail_words = crate::runtime_backend::token_word_refs(&destination_tail);
        let battlefield_attacking = CCA_ATTACKING_MARKER_PATTERN.matches_words(&destination_tail_words);
        let battlefield_tapped = CCA_TAPPED_MARKER_PATTERN.matches_words(&destination_tail_words);
        if let Some(from_idx) = find_index(&destination_tail, |token| {
            CCA_FROM_WORD_PATTERN.matches_token(token)
        }) && destination_tail.len() >= from_idx + 3
            && CCA_COMMAND_ZONE_TAIL_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(
                &destination_tail[from_idx + 1..],
            ))
        {
            destination_tail.drain(from_idx..from_idx + 3);
        }
        destination_tail
            .retain(|token| !CCA_DESTINATION_IGNORED_WORD_PATTERN.matches_token(token));

        let mut attached_to_target: Option<TargetAst> = None;
        if destination_tail
            .first()
            .is_some_and(|_| {
                CCA_ATTACHED_TO_PREFIX_PATTERN
                    .matches_words(&crate::runtime_backend::token_word_refs(&destination_tail))
            })
        {
            let attachment_target_tokens = trim_commas(&destination_tail[2..]);
            if attachment_target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attachment target after 'attached to' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            attached_to_target = Some(parse_target_phrase(&attachment_target_tokens)?);
            destination_tail.clear();
        }

        let destination_tail_words = crate::runtime_backend::token_word_refs(&destination_tail);
        let supported_control_tail = destination_tail_words.is_empty()
            || CCA_UNDER_YOUR_CONTROL_PATTERN.matches_words(&destination_tail_words)
            || CCA_OWNER_CONTROL_TAIL_PATTERN.matches_words(&destination_tail_words);
        if !supported_control_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let battlefield_controller = if CCA_UNDER_YOUR_CONTROL_PATTERN.matches_words(&destination_tail_words) {
            ReturnControllerAst::You
        } else if CCA_OWNER_CONTROL_TAIL_PATTERN.matches_words(&destination_tail_words) {
            ReturnControllerAst::Owner
        } else {
            ReturnControllerAst::Preserve
        };

        if target_tokens
            .first()
            .is_some_and(|token| CCA_ALL_OR_EACH_WORD_PATTERN.matches_token(token))
        {
            let mut filter = parse_object_filter(&target_tokens[1..], false)?;
            if grammar::words_find_phrase(&target_tokens[1..], &["from", "it"]).is_some() {
                filter.zone = Some(Zone::Hand);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::You);
                }
                filter
                    .tagged_constraints
                    .retain(|constraint| constraint.tag.as_str() != IT_TAG);
            }
            if CCA_AMONG_THEM_MARKER_PATTERN.matches_words(&clause_words) {
                filter.zone = Some(Zone::Exile);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                if CCA_PERMANENT_MARKER_PATTERN.matches_words(&clause_words) {
                    filter.card_types = vec![
                        CardType::Artifact,
                        CardType::Creature,
                        CardType::Enchantment,
                        CardType::Land,
                        CardType::Planeswalker,
                        CardType::Battle,
                    ];
                }
            }
            return Ok(EffectAst::subject_verb_return_all_to_battlefield(
                filter,
                battlefield_tapped,
                battlefield_controller,
            ));
        }

        let mut target = parse_target_phrase(&target_tokens)?;
        if let Some(filter) = crate::runtime_backend::sentences::effect_sentences::zone_counter_helpers::target_object_filter_mut(&mut target)
        {
            crate::runtime_backend::sentences::effect_sentences::zone_counter_helpers::apply_exile_subject_owner_context(filter, subject);
        }
        if super::super::grammar::primitives::contains_phrase(
            dest_slice,
            &["from", "the", "command", "zone"],
        ) || super::super::grammar::primitives::contains_phrase(
            dest_slice,
            &["from", "command", "zone"],
        ) {
            apply_source_zone_constraint(&mut target, Zone::Command);
        }

        return Ok(EffectAst::subject_verb_move_to_zone_with_attacking(
            target,
            Zone::Battlefield,
            false,
            battlefield_controller,
            battlefield_tapped,
            battlefield_attacking,
            attached_to_target,
        ));
    }

    if CCA_STICKER_MARKER_PATTERN.matches_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported sticker clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported put clause (clause: '{}')",
        clause_words.join(" ")
    )))
}
