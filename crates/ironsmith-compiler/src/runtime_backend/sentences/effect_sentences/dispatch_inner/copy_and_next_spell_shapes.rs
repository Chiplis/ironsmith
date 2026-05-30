pub(crate) fn parse_delayed_until_next_end_step_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut idx = 0usize;
    if !token_slice_at_is(tokens, idx, "at") {
        return Ok(None);
    }
    idx += 1;

    if token_slice_at_is(tokens, idx, "the") {
        idx += 1;
    }
    if !token_slice_at_is(tokens, idx, "beginning")
    {
        return Ok(None);
    }
    idx += 1;
    if !token_slice_at_is(tokens, idx, "of") {
        return Ok(None);
    }
    idx += 1;

    if token_slice_at_is(tokens, idx, "the") {
        idx += 1;
    }

    let mut player = if token_slice_at_is(tokens, idx, "your") {
        idx += 1;
        PlayerFilter::You
    } else {
        PlayerFilter::Any
    };
    let mut start_next_turn = false;

    if token_slice_at_is(tokens, idx, "next") {
        if !token_slice_at_is(tokens, idx + 1, "end")
            || !token_slice_at_is(tokens, idx + 2, "step")
        {
            return Ok(None);
        }
        idx += 3;
    } else {
        if !token_slice_at_is(tokens, idx, "end") || !token_slice_at_is(tokens, idx + 1, "step")
        {
            return Ok(None);
        }
        idx += 2;
    }

    if token_slice_at_is(tokens, idx, "of") {
        idx += 1;
        if token_slice_at_is(tokens, idx, "that")
            && (token_slice_at_is(tokens, idx + 1, "player")
                || token_slice_at_is(tokens, idx + 1, "players"))
        {
            player = PlayerFilter::IteratedPlayer;
            idx += 2;
        } else if token_slice_at_is(tokens, idx, "your") {
            player = PlayerFilter::You;
            idx += 1;
        } else if token_slice_at_is(tokens, idx, "target")
            && token_slice_at_is(tokens, idx + 1, "player")
        {
            player = PlayerFilter::Target(Box::new(PlayerFilter::Any));
            idx += 2;
        } else {
            return Ok(None);
        }

        if !token_slice_at_is(tokens, idx, "next")
            || !token_slice_at_is(tokens, idx + 1, "turn")
        {
            return Ok(None);
        }
        idx += 2;
        start_next_turn = true;
    }

    if tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }
    let remainder = trim_commas(&tokens[idx..]);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(
            "missing delayed end-step effect clause".to_string(),
        ));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed end-step effect clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    if start_next_turn {
        let player_ast = match player {
            PlayerFilter::You => PlayerAst::You,
            PlayerFilter::IteratedPlayer => PlayerAst::That,
            PlayerFilter::Target(_) => PlayerAst::Target,
            PlayerFilter::Opponent => PlayerAst::Opponent,
            _ => PlayerAst::Any,
        };
        Ok(Some(vec![EffectAst::DelayedUntilEndStepOfExtraTurn {
            player: player_ast,
            effects: delayed_effects,
        }]))
    } else {
        Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
            player,
            effects: delayed_effects,
        }]))
    }
}

pub(crate) fn parse_sentence_delayed_trigger_this_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if grammar::words_match_prefix(tokens, &["this", "turn"]).is_some() {
        let Some((_duration, delayed_clause)) =
            super::super::grammar::primitives::split_lexed_once_on_delimiter(
                tokens,
                super::super::lexer::TokenKind::Comma,
            )
        else {
            return Ok(None);
        };
        let delayed_clause = trim_commas(delayed_clause);
        if !delayed_clause
            .first()
            .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
        {
            return Ok(None);
        }
        let Some((trigger_part, effect_part)) =
            super::super::grammar::primitives::split_lexed_once_on_delimiter(
                &delayed_clause,
                super::super::lexer::TokenKind::Comma,
            )
        else {
            return Ok(None);
        };

        let mut trigger_tokens = trim_commas(trigger_part);
        if trigger_tokens
            .first()
            .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
        {
            trigger_tokens = trigger_tokens[1..].to_vec();
        }
        if trigger_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing delayed trigger clause after 'this turn' (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }

        let delayed_effects = parse_effect_chain(&trim_commas(effect_part))?;
        if delayed_effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing delayed trigger effect clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }

        let trigger_words = crate::runtime_backend::token_word_refs(&trigger_tokens);
        let attack_unblocked_suffix =
            word_slice_ends_with(trigger_words.as_slice(), &["attacks", "and", "isn't", "blocked"])
                || word_slice_ends_with(
                    trigger_words.as_slice(),
                    &["attacks", "and", "isnt", "blocked"],
                );
        if attack_unblocked_suffix
            && trigger_words
                .first()
                .is_some_and(|word| *word == "target")
        {
            let subject_len = trigger_words.len().saturating_sub(4);
            let subject_tokens = trim_commas(&trigger_tokens[1..subject_len]);
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing target subject for delayed attack trigger (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
            let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported delayed attack target filter (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
            let mut trigger_filter = filter.clone();
            trigger_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            return Ok(Some(vec![
                EffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                },
                EffectAst::DelayedTriggerThisTurn {
                    trigger: TriggerSpec::AttacksAndIsntBlocked(trigger_filter),
                    effects: delayed_effects,
                },
            ]));
        }

        let trigger = parse_trigger_clause_lexed(&trigger_tokens)?;
        return Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
            trigger,
            effects: delayed_effects,
        }]));
    }

    if !tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        return Ok(None);
    }

    let Some((before_comma, after_comma)) =
        super::super::grammar::primitives::split_lexed_once_on_delimiter(
            tokens,
            super::super::lexer::TokenKind::Comma,
        )
    else {
        return Ok(None);
    };

    let mut trigger_tokens = trim_commas(before_comma);
    if trigger_tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        trigger_tokens = trigger_tokens[1..].to_vec();
    }
    if trigger_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger clause before comma (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let trigger_word_storage = DispatchInnerNormalizedWords::new(&trigger_tokens);
    let trigger_words = trigger_word_storage.to_word_refs();
    if trigger_words.len() < 3
        || !word_slice_ends_with(trigger_words.as_slice(), &["this", "turn"])
    {
        return Ok(None);
    }

    let trim_start = trigger_word_storage
        .token_index_for_word_index(trigger_words.len() - 2)
        .unwrap_or(trigger_tokens.len());
    let trigger_core_tokens = trim_commas(&trigger_tokens[..trim_start]);
    if trigger_core_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger clause before 'this turn' (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }
    let trigger_core_words = crate::runtime_backend::token_word_refs(&trigger_core_tokens);
    let trigger = if matches!(
        trigger_core_words.as_slice(),
        ["that", "creature", "is", "dealt", "damage"]
            | ["that", "permanent", "is", "dealt", "damage"]
            | ["that", "creature", "is", "dealt", "combat", "damage"]
            | ["that", "permanent", "is", "dealt", "combat", "damage"]
    ) {
        let mut filter = if trigger_core_words[1] == "creature" {
            ObjectFilter::creature()
        } else {
            ObjectFilter::permanent()
        };
        filter = filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
        if word_slice_contains_word(&trigger_core_words, "combat") {
            TriggerSpec::IsDealtCombatDamage(filter)
        } else {
            TriggerSpec::IsDealtDamage(filter)
        }
    } else {
        parse_trigger_clause_lexed(&trigger_core_tokens)?
    };
    let remainder = trim_commas(after_comma);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger effect clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger effect clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
        trigger,
        effects: delayed_effects,
    }]))
}

pub(crate) fn parse_delayed_when_that_dies_this_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 6 {
        return Ok(None);
    }
    if !matches!(
        clause_words.first().copied(),
        Some("when" | "whenever" | "if")
    ) {
        return Ok(None);
    }
    let mut delayed_filter: Option<ObjectFilter> = None;
    let split_after_word_idx = if word_slice_at_is(&clause_words, 1, "that") {
        let Some(dies_idx) = find_index(clause_words.as_slice(), |word| *word == "dies") else {
            return Ok(None);
        };
        if !word_slice_starts_with_at(&clause_words, dies_idx + 1, &["this", "turn"]) {
            return Ok(None);
        }
        dies_idx + 2
    } else if let Some(dealt_idx) = crate::runtime_backend::lexer::word_slice_find_phrase_start(
        &clause_words,
        &["dealt", "damage", "this", "way", "dies", "this", "turn"],
    ) {
        if dealt_idx <= 1 {
            return Ok(None);
        }
        let clause = LexedClause::new(tokens);
        let Some(subject_clause) = clause.between_word_range(1, dealt_idx) else {
            return Ok(None);
        };
        let mut subject_tokens = trim_edge_punctuation(subject_clause.tokens());
        if subject_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in delayed dies-this-way clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let stripped_subject = strip_leading_articles(&subject_tokens);
        if !stripped_subject.is_empty() {
            subject_tokens = stripped_subject;
        }
        delayed_filter = Some(parse_object_filter(&subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported object filter in delayed dies-this-way clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?);
        dealt_idx + 6
    } else if let Some(dealt_idx) = crate::runtime_backend::lexer::word_slice_find_phrase_start(
        &clause_words,
        &[
            "dealt", "damage", "this", "way", "would", "die", "this", "turn",
        ],
    ) {
        if dealt_idx <= 1 {
            return Ok(None);
        }
        let clause = LexedClause::new(tokens);
        let Some(subject_clause) = clause.between_word_range(1, dealt_idx) else {
            return Ok(None);
        };
        let mut subject_tokens = trim_edge_punctuation(subject_clause.tokens());
        if subject_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in delayed dies-this-way clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let stripped_subject = strip_leading_articles(&subject_tokens);
        if !stripped_subject.is_empty() {
            subject_tokens = stripped_subject;
        }
        delayed_filter = Some(parse_object_filter(&subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported object filter in delayed dies-this-way clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?);
        dealt_idx + 7
    } else {
        return Ok(None);
    };
    let clause = LexedClause::new(tokens);
    let mut remainder = clause
        .after_words(split_after_word_idx + 1)
        .unwrap_or_else(|| clause.from(tokens.len()))
        .tokens();
    if token_slice_first_kind(remainder, TokenKind::Comma) {
        remainder = &remainder[1..];
    }
    let remainder = trim_commas(remainder);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: delayed_filter,
        effects: delayed_effects,
    }]))
}

pub(crate) fn find_from_among(tokens: &[OwnedLexToken]) -> Option<usize> {
    crate::runtime_backend::lexer::find_token_word_sequence(tokens, &["from", "among"])
}

pub(crate) fn find_list_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if is_article(word) {
            if tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .and_then(parse_card_type)
                .is_some()
            {
                return Some(idx);
            }
        } else if parse_card_type(word).is_some() {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn split_choose_list(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    for segment in split_lexed_slices_on_and(tokens) {
        for sub in split_lexed_slices_on_comma(segment) {
            let trimmed = trim_commas(sub);
            if !trimmed.is_empty() {
                segments.push(trimmed);
            }
        }
    }
    segments
}

pub(crate) fn merge_filters(base: &ObjectFilter, specific: &ObjectFilter) -> ObjectFilter {
    let mut merged = base.clone();

    if !specific.card_types.is_empty() {
        merged.card_types = specific.card_types.clone();
    }
    if !specific.all_card_types.is_empty() {
        merged.all_card_types = specific.all_card_types.clone();
    }
    if !specific.subtypes.is_empty() {
        merged.subtypes.extend(specific.subtypes.clone());
    }
    if !specific.excluded_card_types.is_empty() {
        merged
            .excluded_card_types
            .extend(specific.excluded_card_types.clone());
    }
    if !specific.excluded_colors.is_empty() {
        merged.excluded_colors = merged.excluded_colors.union(specific.excluded_colors);
    }
    if let Some(colors) = specific.colors {
        merged.colors = Some(
            merged
                .colors
                .map_or(colors, |existing| existing.union(colors)),
        );
    }
    merged.chosen_color |= specific.chosen_color;
    if merged.zone.is_none() {
        merged.zone = specific.zone;
    }
    if merged.controller.is_none() {
        merged.controller = specific.controller.clone();
    }
    if merged
        .attacking_player_or_planeswalker_controlled_by
        .is_none()
    {
        merged.attacking_player_or_planeswalker_controlled_by = specific
            .attacking_player_or_planeswalker_controlled_by
            .clone();
    }
    if merged.owner.is_none() {
        merged.owner = specific.owner.clone();
    }
    merged.other |= specific.other;
    merged.token |= specific.token;
    merged.nontoken |= specific.nontoken;
    merged.tapped |= specific.tapped;
    merged.untapped |= specific.untapped;
    merged.attacking |= specific.attacking;
    merged.nonattacking |= specific.nonattacking;
    merged.blocking |= specific.blocking;
    merged.nonblocking |= specific.nonblocking;
    merged.blocked |= specific.blocked;
    merged.unblocked |= specific.unblocked;
    merged.is_commander |= specific.is_commander;
    merged.noncommander |= specific.noncommander;
    merged.colorless |= specific.colorless;
    merged.multicolored |= specific.multicolored;
    merged.monocolored |= specific.monocolored;

    if let Some(mv) = &specific.mana_value {
        merged.mana_value = Some(mv.clone());
    }
    if let Some(power) = &specific.power {
        merged.power = Some(power.clone());
        merged.power_reference = specific.power_reference;
    }
    if let Some(toughness) = &specific.toughness {
        merged.toughness = Some(toughness.clone());
        merged.toughness_reference = specific.toughness_reference;
    }
    if specific.has_mana_cost {
        merged.has_mana_cost = true;
    }
    if specific.no_x_in_cost {
        merged.no_x_in_cost = true;
    }
    if merged.with_counter.is_none() {
        merged.with_counter = specific.with_counter;
    }
    if merged.without_counter.is_none() {
        merged.without_counter = specific.without_counter;
    }
    if merged.alternative_cast.is_none() {
        merged.alternative_cast = specific.alternative_cast;
    }
    for ability_id in &specific.static_abilities {
        if !iter_contains(merged.static_abilities.iter(), ability_id) {
            merged.static_abilities.push(*ability_id);
        }
    }
    for ability_id in &specific.excluded_static_abilities {
        if !iter_contains(merged.excluded_static_abilities.iter(), ability_id) {
            merged.excluded_static_abilities.push(*ability_id);
        }
    }
    for marker in &specific.ability_markers {
        if !merged
            .ability_markers
            .iter()
            .any(|value| value.eq_ignore_ascii_case(marker))
        {
            merged.ability_markers.push(marker.clone());
        }
    }
    for marker in &specific.excluded_ability_markers {
        if !merged
            .excluded_ability_markers
            .iter()
            .any(|value| value.eq_ignore_ascii_case(marker))
        {
            merged.excluded_ability_markers.push(marker.clone());
        }
    }

    merged
}
