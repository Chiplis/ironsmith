pub(crate) fn parse_lose_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.len() == 2
        && clause_words[1] == "life"
        && let Some((amount, _)) = parse_number(tokens)
    {
        return Ok(EffectAst::LoseLife {
            amount: Value::Fixed(amount as i32),
            player,
        });
    }
    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
            && (grammar::words_find_phrase(tokens, &["its", "power"]).is_some()
                || grammar::words_find_phrase(tokens, &["its", "toughness"]).is_some()
                || grammar::words_find_phrase(tokens, &["its", "mana", "value"]).is_some())
        {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(EffectAst::LoseLife { amount, player });
    }
    if clause_words.as_slice() == ["the", "game"] {
        return Ok(EffectAst::LoseGame { player });
    }

    if let Some(amount) = parse_half_life_value(tokens, player) {
        return Ok(EffectAst::LoseLife { amount, player });
    }

    let (mut amount, used) = parse_life_amount(tokens, "life loss")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(EffectAst::LoseLife { amount, player });
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-loss clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::LoseLife { amount, player })
}

pub(crate) fn parse_gain_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if let Some(amount) = parse_life_equal_to_value(tokens)? {
        return Ok(EffectAst::GainLife { amount, player });
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
            && grammar::contains_word(&trailing, "library")
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing life-gain shuffle-graveyard clause (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            )));
        }
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(EffectAst::GainLife { amount, player });
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-gain clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::GainLife { amount, player })
}

pub(crate) fn parse_gain_control(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let has_dynamic_power_bound = grammar::contains_word(tokens, "power")
        && grammar::contains_word(tokens, "number")
        && grammar::words_find_phrase(tokens, &["you", "control"]).is_some();
    if has_dynamic_power_bound {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic power-bound control clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut idx = 0;
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("control"))
    {
        idx += 1;
    } else {
        return Err(CardTextError::ParseError(
            "missing control keyword".to_string(),
        ));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }

    let duration_idx = find_index(&tokens[idx..], |token: &OwnedLexToken| {
        token.is_word("during") || token.is_word("until")
    })
    .map(|offset| idx + offset)
    .or_else(|| {
        find_window_by(&tokens[idx..], 4, |window: &[OwnedLexToken]| {
            window[0].is_word("for")
                && window[1].is_word("as")
                && window[2].is_word("long")
                && window[3].is_word("as")
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
        } else if target_tokens.iter().any(|token| token.is_word("if")) {
            return Err(invalid_conditional_error());
        } else if let Some(spec) = split_trailing_unless_clause_lexed(target_tokens) {
            (
                parse_target_phrase(spec.leading_tokens)?,
                Some(spec.predicate),
                true,
            )
        } else if target_tokens.iter().any(|token| token.is_word("unless")) {
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
        TargetAst::Player(filter, _) => EffectAst::ControlPlayer {
            player: PlayerFilter::Target(Box::new(filter)),
            duration,
        },
        _ => {
            let until = match duration {
                ControlDurationAst::UntilEndOfTurn => Until::EndOfTurn,
                ControlDurationAst::Forever => Until::Forever,
                ControlDurationAst::AsLongAsYouControlSource => Until::YouStopControllingThis,
                ControlDurationAst::DuringNextTurn => {
                    return Err(CardTextError::ParseError(
                        "unsupported control duration for permanents".to_string(),
                    ));
                }
            };
            EffectAst::GainControl {
                target: target_ast,
                player,
                duration: until,
            }
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

    let has_for_as_long_as =
        grammar::words_find_phrase(tokens, &["for", "as", "long", "as"]).is_some();
    if has_for_as_long_as
        && grammar::contains_word(tokens, "you")
        && grammar::contains_word(tokens, "control")
        && (grammar::contains_word(tokens, "this")
            || grammar::contains_word(tokens, "thiss")
            || grammar::contains_word(tokens, "source")
            || grammar::contains_word(tokens, "creature")
            || grammar::contains_word(tokens, "permanent"))
    {
        return Ok(ControlDurationAst::AsLongAsYouControlSource);
    }

    let has_during = grammar::contains_word(tokens, "during");
    let has_next = grammar::contains_word(tokens, "next");
    let has_turn = grammar::contains_word(tokens, "turn");
    if has_during && has_next && has_turn {
        return Ok(ControlDurationAst::DuringNextTurn);
    }

    let has_until = grammar::contains_word(tokens, "until");
    let has_end = grammar::contains_word(tokens, "end");
    if has_until && has_end && has_turn {
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
            token.is_word("hand") || token.is_word("hands")
        })?;
        let tail_tokens = trim_commas(&tokens[hand_idx + 1..]);
        let tail_words = crate::cards::builders::compiler::token_word_refs(&tail_tokens);
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
        let target_words = crate::cards::builders::compiler::token_word_refs(target_tokens);
        let has_graveyard = target_words
            .iter()
            .any(|word| matches!(*word, "graveyard" | "graveyards"));
        let has_hand = target_words
            .iter()
            .any(|word| matches!(*word, "hand" | "hands"));
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

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);

    // "Put them/it back in any order." (typically after looking at the top cards of a library).
    if grammar::contains_word(tokens, "back")
        && grammar::contains_word(tokens, "any")
        && grammar::contains_word(tokens, "order")
        && matches!(clause_words.first().copied(), Some("it" | "them"))
    {
        return Ok(EffectAst::ReorderTopOfLibrary {
            tag: TagKey::from(IT_TAG),
        });
    }

    if grammar::contains_word(tokens, "from")
        && grammar::contains_word(tokens, "among")
        && grammar::contains_word(tokens, "hand")
    {
        return Ok(EffectAst::PutSomeIntoHandRestIntoGraveyard { player, count: 1 });
    }
    let has_it = grammar::contains_word(tokens, "it");
    let has_them = grammar::contains_word(tokens, "them");
    let has_hand = grammar::contains_word(tokens, "hand");
    let has_into = grammar::contains_word(tokens, "into");

    if has_hand && has_into && (has_it || has_them) {
        // "Put N of them into your hand and the rest on the bottom of your library in any order."
        if has_them
            && grammar::contains_word(tokens, "rest")
            && grammar::contains_word(tokens, "bottom")
            && grammar::contains_word(tokens, "library")
            && clause_words.iter().any(|w| *w == "and" || *w == "then")
        {
            let (count, used) = parse_number(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing put count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            let mut idx = used;
            if tokens.get(idx).is_some_and(|t| t.is_word("of")) {
                idx += 1;
            }
            if !tokens.get(idx).is_some_and(|t| t.is_word("them")) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported multi-destination put clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let dest_player = if grammar::contains_word(tokens, "your") {
                PlayerAst::You
            } else if grammar::contains_word(tokens, "their")
                || grammar::words_match_any_prefix(tokens, THAT_PLAYER_PREFIXES).is_some()
            {
                PlayerAst::That
            } else {
                player
            };

            return Ok(EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: dest_player,
                count: count as u32,
            });
        }

        // "Put N of them into your hand and the rest into your graveyard."
        if has_them
            && grammar::contains_word(tokens, "rest")
            && grammar::contains_word(tokens, "graveyard")
            && clause_words.iter().any(|w| *w == "and" || *w == "then")
        {
            let (count, used) = parse_number(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing put count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            // Accept optional "of" before "them".
            let mut idx = used;
            if tokens.get(idx).is_some_and(|t| t.is_word("of")) {
                idx += 1;
            }
            if !tokens.get(idx).is_some_and(|t| t.is_word("them")) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported multi-destination put clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            // The chooser is typically the player whose hand is referenced.
            let dest_player = if grammar::contains_word(tokens, "your") {
                PlayerAst::You
            } else if grammar::contains_word(tokens, "their")
                || grammar::words_match_any_prefix(tokens, THAT_PLAYER_PREFIXES).is_some()
            {
                PlayerAst::That
            } else {
                player
            };

            return Ok(EffectAst::PutSomeIntoHandRestIntoGraveyard {
                player: dest_player,
                count: count as u32,
            });
        }

        let effect = EffectAst::PutIntoHand {
            player,
            object: ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
        };
        return Ok(wrap_return_with_delayed_timing(
            effect,
            parse_put_into_hand_delayed_timing(tokens),
        ));
    }

    // Support destination-first wording:
    // "Put onto the battlefield under your control all creature cards ..."
    if tokens.first().is_some_and(|token| token.is_word("onto")) {
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
            .is_some_and(|token| token.is_word("battlefield"))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        idx += 1;

        let mut battlefield_tapped = false;
        if tokens.get(idx).is_some_and(|token| token.is_word("tapped")) {
            battlefield_tapped = true;
            idx += 1;
        }

        let mut battlefield_controller = ReturnControllerAst::Preserve;
        if tokens.get(idx).is_some_and(|token| token.is_word("under")) {
            let consumed =
                if grammar::words_match_prefix(&tokens[idx..], &["under", "your", "control"])
                    .is_some()
                {
                    battlefield_controller = ReturnControllerAst::You;
                    Some(3usize)
                } else if grammar::words_match_prefix(
                    &tokens[idx..],
                    &["under", "its", "owners", "control"],
                )
                .is_some()
                    || grammar::words_match_prefix(
                        &tokens[idx..],
                        &["under", "his", "owners", "control"],
                    )
                    .is_some()
                    || grammar::words_match_prefix(
                        &tokens[idx..],
                        &["under", "her", "owners", "control"],
                    )
                    .is_some()
                    || grammar::words_match_prefix(
                        &tokens[idx..],
                        &["under", "their", "owners", "control"],
                    )
                    .is_some()
                    || grammar::words_match_prefix(
                        &tokens[idx..],
                        &["under", "that", "players", "control"],
                    )
                    .is_some()
                {
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

        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("attached"))
            && target_tokens
                .get(1)
                .is_some_and(|token| token.is_word("to"))
        {
            let after_to = &target_tokens[2..];
            if after_to.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attachment target after 'attached to' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let attachment_target_len = if after_to.first().is_some_and(|token| token.is_word("it"))
            {
                1usize
            } else if after_to.len() >= 2
                && after_to[0].is_word("that")
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

            return Ok(EffectAst::MoveToZone {
                target: object_target,
                zone: Zone::Battlefield,
                to_top: false,
                battlefield_controller,
                battlefield_tapped,
                attached_to: Some(attachment_target),
            });
        }

        if !target_tokens
            .first()
            .is_some_and(|token| token.is_word("attached"))
        {
            let mut rewritten = target_tokens;
            rewritten.push(OwnedLexToken::word("onto".to_string(), tokens[0].span()));
            rewritten.extend_from_slice(&tokens[1..idx]);
            return parse_put_into_hand(&rewritten, subject);
        }
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
        let target = if let Some((count, used)) = parse_number(&target_tokens)
            && target_tokens
                .get(used)
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            let inner = parse_target_phrase(&target_tokens[used..])?;
            TargetAst::WithCount(Box::new(inner), ChoiceCount::exactly(count as usize))
        } else {
            parse_target_phrase(&target_tokens)?
        };
        return Ok(EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: true,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        });
    }

    if let Some(on_idx) = find_index(tokens, |token| token.is_word("on")) {
        let mut bottom_idx = on_idx + 1;
        if tokens
            .get(bottom_idx)
            .is_some_and(|token| token.is_word("the"))
        {
            bottom_idx += 1;
        }
        if tokens
            .get(bottom_idx)
            .is_some_and(|token| token.is_word("bottom"))
            && tokens
                .get(bottom_idx + 1)
                .is_some_and(|token| token.is_word("of"))
        {
            let target_tokens = trim_commas(&tokens[..on_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing target before 'on bottom of' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if !grammar::contains_word(&tokens[bottom_idx + 2..], "library") {
                return Err(CardTextError::ParseError(format!(
                    "unsupported put destination after 'on bottom of' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let target_words = crate::cards::builders::compiler::token_word_refs(&target_tokens);
            let is_rest_target =
                target_words.as_slice() == ["the", "rest"] || target_words.as_slice() == ["rest"];
            if is_rest_target {
                return Ok(EffectAst::PutRestOnBottomOfLibrary);
            }

            let target = if let Some((count, used)) = parse_number(&target_tokens)
                && target_tokens
                    .get(used)
                    .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
            {
                let inner = parse_target_phrase(&target_tokens[used..])?;
                TargetAst::WithCount(Box::new(inner), ChoiceCount::exactly(count as usize))
            } else {
                parse_target_phrase(&target_tokens)?
            };

            return Ok(EffectAst::MoveToZone {
                target,
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            });
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
            return Ok(EffectAst::MoveToLibraryNthFromTop { target, position });
        } else {
            None
        };

        if let Some(zone) = zone {
            let delayed_hand_timing = if zone == Zone::Hand {
                parse_put_into_hand_delayed_timing(tokens)
            } else {
                None
            };
            let target_words = crate::cards::builders::compiler::token_word_refs(&target_tokens);
            if zone == Zone::Graveyard
                && matches!(target_words.as_slice(), ["the", "rest"] | ["rest"])
            {
                return Ok(EffectAst::MoveToZone {
                    target: TargetAst::Object(
                        ObjectFilter::tagged(TagKey::from(IT_TAG)),
                        None,
                        None,
                    ),
                    zone,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                });
            }

            if zone == Zone::Hand {
                if matches!(
                    target_words.as_slice(),
                    ["it"] | ["them"] | ["that", "card"] | ["those", "card"] | ["those", "cards"]
                ) {
                    let effect = EffectAst::PutIntoHand {
                        player,
                        object: ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
                    };
                    return Ok(wrap_return_with_delayed_timing(effect, delayed_hand_timing));
                }
            }

            let effect = EffectAst::MoveToZone {
                target: parse_target_phrase(&target_tokens)?,
                zone,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            };
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

        let destination_words: Vec<&str> =
            crate::cards::builders::compiler::token_word_refs(dest_slice)
                .into_iter()
                .filter(|word| !is_article(word))
                .collect();
        if destination_words.first() != Some(&"battlefield") {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut destination_tail: Vec<&str> = destination_words[1..].to_vec();
        let battlefield_attacking = slice_contains(&destination_tail, &"attacking");
        let battlefield_tapped = slice_contains(&destination_tail, &"tapped");
        if let Some(from_idx) =
            find_word_sequence_start(&destination_tail, &["from", "command", "zone"])
        {
            destination_tail.drain(from_idx..from_idx + 3);
        }
        destination_tail.retain(|word| *word != "and");
        destination_tail.retain(|word| *word != "tapped");
        destination_tail.retain(|word| *word != "attacking");
        if battlefield_attacking {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let supported_control_tail = destination_tail.is_empty()
            || destination_tail.as_slice() == ["under", "your", "control"]
            || destination_tail.as_slice() == ["under", "its", "owners", "control"]
            || destination_tail.as_slice() == ["under", "his", "owners", "control"]
            || destination_tail.as_slice() == ["under", "her", "owners", "control"]
            || destination_tail.as_slice() == ["under", "their", "owners", "control"]
            || destination_tail.as_slice() == ["under", "that", "players", "control"];
        if !supported_control_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let battlefield_controller = if destination_tail.as_slice() == ["under", "your", "control"]
        {
            ReturnControllerAst::You
        } else if destination_tail.as_slice() == ["under", "its", "owners", "control"]
            || destination_tail.as_slice() == ["under", "his", "owners", "control"]
            || destination_tail.as_slice() == ["under", "her", "owners", "control"]
            || destination_tail.as_slice() == ["under", "their", "owners", "control"]
            || destination_tail.as_slice() == ["under", "that", "players", "control"]
        {
            ReturnControllerAst::Owner
        } else {
            ReturnControllerAst::Preserve
        };

        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("all") || token.is_word("each"))
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
            if grammar::contains_word(tokens, "among") && grammar::contains_word(tokens, "them") {
                filter.zone = Some(Zone::Exile);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                if grammar::contains_word(tokens, "permanent") {
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
            return Ok(EffectAst::ReturnAllToBattlefield {
                filter,
                tapped: battlefield_tapped,
            });
        }

        let mut target = parse_target_phrase(&target_tokens)?;
        if super::super::grammar::primitives::contains_phrase(
            dest_slice,
            &["from", "the", "command", "zone"],
        ) || super::super::grammar::primitives::contains_phrase(
            dest_slice,
            &["from", "command", "zone"],
        ) {
            apply_source_zone_constraint(&mut target, Zone::Command);
        }

        return Ok(EffectAst::MoveToZone {
            target,
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller,
            battlefield_tapped,
            attached_to: None,
        });
    }

    if grammar::contains_word(tokens, "sticker") {
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
