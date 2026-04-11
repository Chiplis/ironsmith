fn wrap_delayed_next_step_unless_pays(
    step: DelayedNextStepKind,
    player: PlayerAst,
    effects: Vec<EffectAst>,
) -> EffectAst {
    match step {
        DelayedNextStepKind::Upkeep => EffectAst::DelayedUntilNextUpkeep { player, effects },
        DelayedNextStepKind::DrawStep => EffectAst::DelayedUntilNextDrawStep { player, effects },
    }
}

pub(crate) fn parse_sentence_delayed_next_step_unless_pays(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_lexed_slices_on_period(tokens);
    if segments.is_empty() {
        return Ok(None);
    }

    let (leading_segments, final_segment) = segments.split_at(segments.len() - 1);
    let final_segment = trim_commas(&final_segment[0]);
    let Some((timing_start_word, _timing_end_word, step, player)) =
        delayed_next_step_marker(&final_segment)
    else {
        return Ok(None);
    };

    let Some(timing_token_idx) = token_index_for_word_index(&final_segment, timing_start_word)
    else {
        return Ok(None);
    };
    let delayed_effect_tokens = trim_commas(&final_segment[..timing_token_idx]);
    if delayed_effect_tokens.is_empty() {
        return Ok(None);
    }

    let delayed_effects = parse_effect_chain(&delayed_effect_tokens)?;
    if delayed_effects.is_empty() {
        return Ok(None);
    }

    let timing_tokens = trim_commas(&final_segment[timing_token_idx..]);
    let Some(unless_idx) = find_token_word(&timing_tokens, "unless") else {
        return Ok(None);
    };
    let Some(unless_effect) = try_build_unless(delayed_effects, &timing_tokens, unless_idx)? else {
        return Ok(None);
    };

    let mut effects = Vec::new();
    for segment in leading_segments {
        let parsed = parse_effect_chain(segment)?;
        if parsed.is_empty() {
            return Ok(None);
        }
        effects.extend(parsed);
    }
    effects.push(wrap_delayed_next_step_unless_pays(
        step,
        player,
        vec![unless_effect],
    ));
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_delayed_next_upkeep_unless_pays_lose_game(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_lexed_slices_on_period(tokens);
    if segments.len() != 2 && segments.len() != 3 {
        return Ok(None);
    }

    let (mut effects, upkeep_tokens, lose_tokens) = if segments.len() == 3 {
        let first_effects = parse_effect_chain(&segments[0])?;
        if first_effects.is_empty() {
            return Ok(None);
        }
        (
            first_effects,
            trim_commas(&segments[1]),
            trim_commas(&segments[2]),
        )
    } else {
        (
            Vec::new(),
            trim_commas(&segments[0]),
            trim_commas(&segments[1]),
        )
    };
    let upkeep_words = crate::cards::builders::compiler::token_word_refs(&upkeep_tokens);
    let pay_idx = if grammar::words_match_prefix(
        &upkeep_tokens,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "your",
            "next",
            "upkeep",
            "pay",
        ],
    )
    .is_some()
    {
        7usize
    } else if grammar::words_match_prefix(
        &upkeep_tokens,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "the",
            "next",
            "upkeep",
            "pay",
        ],
    )
    .is_some()
    {
        8usize
    } else {
        return Ok(None);
    };

    let Some(pay_token_idx) = token_index_for_word_index(&upkeep_tokens, pay_idx) else {
        return Ok(None);
    };
    let mana_tokens = &upkeep_tokens[pay_token_idx + 1..];
    if mana_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing mana payment in delayed next-upkeep clause (clause: '{}')",
            upkeep_words.join(" ")
        )));
    }

    let mana = {
        use super::super::grammar::primitives as grammar;
        use super::super::lexer::LexStream;
        use winnow::prelude::*;

        let mut stream = LexStream::new(mana_tokens);
        grammar::collect_mana_symbols
            .parse_next(&mut stream)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "missing mana payment in delayed next-upkeep clause (clause: '{}')",
                    upkeep_words.join(" ")
                ))
            })?
    };

    let lose_words = crate::cards::builders::compiler::token_word_refs(&lose_tokens);
    let valid_lose_clause = lose_words == ["if", "you", "dont", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "do", "not", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "don't", "you", "lose", "the", "game"];
    if !valid_lose_clause {
        return Ok(None);
    }

    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::LoseGame {
                player: PlayerAst::You,
            }],
            player: PlayerAst::You,
            mana,
        }],
    });
    Ok(Some(effects))
}

/// Try to build an UnlessPays or UnlessAction AST from the tokens after "unless".
/// Returns the unless wrapper containing the given `effects` as the main effects.
pub(crate) fn try_build_unless(
    effects: Vec<EffectAst>,
    tokens: &[OwnedLexToken],
    unless_idx: usize,
) -> Result<Option<EffectAst>, CardTextError> {
    let after_unless = &tokens[unless_idx + 1..];
    let after_word_storage = SentencePrimitiveNormalizedWords::new(after_unless);
    let after_words = after_word_storage.to_word_refs();
    let pay_word_idx = find_index(&after_words, |word| matches!(*word, "pay" | "pays"));
    let pay_token_idx = find_index(after_unless, |token| {
        token.is_word("pay") || token.is_word("pays")
    });

    let match_player_prefix = |prefix: &[&str]| -> Option<(PlayerAst, usize)> {
        if prefix == ["you"] {
            Some((PlayerAst::You, 1))
        } else if prefix == ["target", "opponent"] {
            Some((PlayerAst::TargetOpponent, 2))
        } else if prefix == ["target", "player"] {
            Some((PlayerAst::Target, 2))
        } else if prefix == ["any", "player"] {
            Some((PlayerAst::Any, 2))
        } else if prefix == ["they"] {
            Some((PlayerAst::That, 1))
        } else if prefix == ["defending", "player"] {
            Some((PlayerAst::Defending, 2))
        } else if prefix == ["that", "player"] {
            Some((PlayerAst::That, 2))
        } else if prefix == ["its", "controller"] || prefix == ["their", "controller"] {
            Some((PlayerAst::ItsController, 2))
        } else if prefix == ["its", "owner"] || prefix == ["their", "owner"] {
            Some((PlayerAst::ItsOwner, 2))
        } else if prefix.len() >= 6
            && prefix[0] == "that"
            && prefix[1] == "player"
            && prefix[2] == "or"
            && prefix[3] == "that"
            && matches!(
                prefix[4],
                "ability"
                    | "abilitys"
                    | "card"
                    | "cards"
                    | "creature"
                    | "creatures"
                    | "object"
                    | "objects"
                    | "permanent"
                    | "permanents"
                    | "planeswalker"
                    | "planeswalkers"
                    | "source"
                    | "sources"
                    | "spell"
                    | "spells"
            )
            && matches!(prefix[5], "controller" | "controllers")
        {
            Some((PlayerAst::ThatPlayerOrTargetController, 6))
        } else if prefix.len() >= 3
            && prefix[0] == "that"
            && matches!(
                prefix[1],
                "ability"
                    | "abilitys"
                    | "card"
                    | "cards"
                    | "creature"
                    | "creatures"
                    | "object"
                    | "objects"
                    | "permanent"
                    | "permanents"
                    | "planeswalker"
                    | "planeswalkers"
                    | "source"
                    | "sources"
                    | "spell"
                    | "spells"
            )
            && matches!(prefix[2], "controller" | "controllers")
        {
            Some((PlayerAst::ItsController, 3))
        } else if prefix.len() >= 3
            && prefix[0] == "that"
            && matches!(
                prefix[1],
                "ability"
                    | "abilitys"
                    | "card"
                    | "cards"
                    | "creature"
                    | "creatures"
                    | "object"
                    | "objects"
                    | "permanent"
                    | "permanents"
                    | "planeswalker"
                    | "planeswalkers"
                    | "source"
                    | "sources"
                    | "spell"
                    | "spells"
            )
            && matches!(prefix[2], "owner" | "owners")
        {
            Some((PlayerAst::ItsOwner, 3))
        } else if prefix.len() >= 6
            && prefix[0] == "that"
            && matches!(
                prefix[1],
                "card"
                    | "cards"
                    | "creature"
                    | "creatures"
                    | "object"
                    | "objects"
                    | "permanent"
                    | "permanents"
                    | "planeswalker"
                    | "planeswalkers"
                    | "source"
                    | "sources"
                    | "spell"
                    | "spells"
            )
            && matches!(prefix[2], "controller" | "controllers")
            && prefix[3] == "or"
            && prefix[4] == "that"
            && prefix[5] == "player"
        {
            Some((PlayerAst::ThatPlayerOrTargetController, 6))
        } else {
            None
        }
    };

    let match_player_clause_prefix = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        let max_prefix_len = words.len().min(6);
        for prefix_len in 1..=max_prefix_len {
            if let Some((player, consumed)) = match_player_prefix(&words[..prefix_len]) {
                return Some((player, consumed));
            }
        }
        None
    };

    // Determine the player from the "unless" clause
    let Some((player, action_word_start)) = (if let Some(pay_idx) = pay_word_idx {
        match_player_prefix(&after_words[..pay_idx]).map(|(player, _)| (player, pay_idx))
    } else {
        match_player_clause_prefix(&after_words)
    }) else {
        return Ok(None);
    };

    let action_token_idx = if let Some(pay_idx) = pay_token_idx {
        pay_idx
    } else {
        after_word_storage
            .token_index_after_words(action_word_start)
            .unwrap_or(0)
    };

    let action_tokens = &after_unless[action_token_idx..];
    let action_word_storage = SentencePrimitiveNormalizedWords::new(action_tokens);
    let action_words = action_word_storage.to_word_refs();

    // "unless [player] pays N life" should compile as an unless-action branch
    // where the deciding player loses life.
    if action_words.first() == Some(&"pay") || action_words.first() == Some(&"pays") {
        let life_tokens = &action_tokens[1..];
        if let Some((amount, used)) = parse_value(life_tokens)
            && life_tokens
                .get(used)
                .is_some_and(|token| token.is_word("life"))
            && life_tokens
                .get(used + 1)
                .map_or(true, |token| token.is_period())
        {
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative: vec![EffectAst::LoseLife { amount, player }],
                player,
            }));
        }
    }

    // Try mana payment first: "pay(s) {mana} [optional trailing condition]"
    // Uses greedy mana parsing — collects mana symbols until first non-mana word,
    // then categorizes remaining tokens to decide whether to accept.
    if action_words.first() == Some(&"pay") || action_words.first() == Some(&"pays") {
        if contains_word_window(&action_words, &["mana", "cost"]) {
            return Err(CardTextError::ParseError(format!(
                "unsupported unless-payment mana-cost clause (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            )));
        }

        // Skip any non-word tokens between "pay" and mana
        let mana_start = find_index(&action_tokens[1..], |token| {
            token.as_word().is_some() || mana_pips_from_token(token).is_some()
        })
        .map(|idx| idx + 1)
        .unwrap_or(1);
        let mana_tokens = &action_tokens[mana_start..];
        let mut mana = Vec::new();
        let mut remaining_idx = mana_tokens.len();
        for (i, token) in mana_tokens.iter().enumerate() {
            if let Some(group) = mana_pips_from_token(token) {
                mana.extend(group);
                continue;
            }
            if let Some(word) = token.as_word() {
                match parse_mana_symbol(word) {
                    Ok(symbol) => mana.push(symbol),
                    Err(_) => {
                        remaining_idx = i;
                        break;
                    }
                }
            }
        }

        if !mana.is_empty() {
            // Check what follows the mana symbols
            let remaining_word_storage =
                SentencePrimitiveNormalizedWords::new(&mana_tokens[remaining_idx..]);
            let remaining_words = remaining_word_storage.to_word_refs();

            let accept = if remaining_words.is_empty() {
                // Pure mana payment (e.g., "pays {2}")
                true
            } else if remaining_words.first() == Some(&"life") {
                // "pay N life" — not a mana payment, it's a life cost
                false
            } else if remaining_words.first() == Some(&"before") {
                // Timing condition like "before that step" — accept, drop condition
                true
            } else {
                // Unknown trailing tokens (for each, where X is, etc.) — skip for now
                false
            };

            if accept {
                return Ok(Some(EffectAst::UnlessPays {
                    effects,
                    player,
                    mana,
                }));
            }

            if !remaining_words.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing unless-payment clause (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            }
        }
    }

    // Try full-clause parsing first to preserve existing behavior for explicit
    // player phrasing such as "unless that player ...".
    if let Ok(mut alternative) = parse_effect_chain(after_unless) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_sentence_lexed(after_unless) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_chain(action_tokens) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_sentence_lexed(action_tokens) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_clause(action_tokens).map(|effect| vec![effect]) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if matches!(action_words.first().copied(), Some("discard" | "discards"))
        && let Ok(mut alternative) =
            super::zone_handlers::parse_discard(action_tokens, None).map(|effect| vec![effect])
    {
        for effect in &mut alternative {
            bind_implicit_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    Ok(None)
}

pub(crate) fn parse_sentence_fallback_mechanic_marker(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.as_slice() == ["venture", "into", "the", "dungeon"] {
        return Ok(Some(vec![EffectAst::VentureIntoDungeon {
            player: crate::cards::builders::PlayerAst::You,
            undercity_if_no_active: false,
        }]));
    }

    let is_match = clause_words.as_slice() == ["its", "still", "a", "land"]
        || clause_words.as_slice() == ["it", "still", "a", "land"]
        || grammar::words_match_any_prefix(tokens, &MECHANIC_MARKER_PREFIXES[..3]).is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "chooses",
                "any",
                "number",
                "of",
                "creatures",
                "they",
                "control",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "each",
                "player",
                "chooses",
                "any",
                "number",
                "of",
                "creatures",
                "they",
                "control",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["an", "opponent", "chooses", "one", "of", "those", "piles"],
        )
        .is_some()
        || grammar::words_match_prefix(tokens, &["put", "that", "pile", "into", "your", "hand"])
            .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["cast", "that", "card", "for", "as", "long", "as"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "until", "end", "of", "turn", "this", "creature", "loses", "prevent", "all",
                "damage",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "until",
                "end",
                "of",
                "turn",
                "target",
                "creature",
                "loses",
                "all",
                "abilities",
                "and",
                "has",
                "base",
                "power",
                "and",
                "toughness",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["for", "each", "1", "damage", "prevented", "this", "way"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "for", "each", "card", "less", "than", "two", "a", "player", "draws", "this", "way",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["this", "deals", "4", "damage", "if", "there", "are"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "this", "deals", "4", "damage", "instead", "if", "there", "are",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "that", "spell", "deals", "damage", "to", "each", "opponent", "equal", "to",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "the", "next", "spell", "you", "cast", "this", "turn", "costs",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "there",
                "is",
                "an",
                "additional",
                "combat",
                "phase",
                "after",
                "this",
                "phase",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "that",
                "creature",
                "attacks",
                "during",
                "its",
                "controllers",
                "next",
                "combat",
                "phase",
                "if",
                "able",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to", "target",
                "creature", "you", "control", "by", "a", "source", "of", "your", "choice", "is",
                "dealt", "to", "another", "target", "creature", "instead",
            ],
        )
        .is_some()
        || (grammar::words_match_any_prefix(tokens, &MECHANIC_MARKER_PREFIXES[3..]).is_some()
            && grammar::contains_word(tokens, "remains")
            && grammar::contains_word(tokens, "tapped"));
    if !is_match {
        return Ok(None);
    }
    Err(CardTextError::ParseError(format!(
        "unsupported mechanic marker clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_sentence_implicit_become_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = TokenWordView::new(tokens).to_word_refs();
    let (target, rest_word_idx) = match clause_words.as_slice() {
        ["this", "permanent", ..] | ["this", "creature", ..] | ["this", "land", ..] => {
            (TargetAst::Source(None), 2)
        }
        ["this", ..] => (TargetAst::Source(None), 1),
        ["each", "of", "them", ..] => (TargetAst::Tagged(TagKey::from(IT_TAG), None), 3),
        ["they", ..] => (TargetAst::Tagged(TagKey::from(IT_TAG), None), 1),
        ["its", ..] | ["it", ..] => (TargetAst::Tagged(TagKey::from(IT_TAG), None), 1),
        _ => return Ok(None),
    };

    let rest_token_idx = token_index_for_word_index(tokens, rest_word_idx).unwrap_or(tokens.len());
    let rest_tokens = trim_commas(&tokens[rest_token_idx..]);
    let (mut duration, duration_remainder) =
        if let Some((duration, remainder)) = parse_restriction_duration(&rest_tokens)? {
            (duration, remainder)
        } else {
            (Until::Forever, rest_tokens.to_vec())
        };
    let rest_tokens = trim_commas(&duration_remainder);
    let mut rest_words = TokenWordView::new(&rest_tokens).to_word_refs();
    if rest_words.first().copied() == Some("still") {
        rest_words.remove(0);
    }
    if rest_words.is_empty() {
        return Ok(None);
    }

    let negated = if slice_starts_with(&rest_words, &["is", "not"])
        || slice_starts_with(&rest_words, &["are", "not"])
    {
        rest_words.drain(..2);
        true
    } else if matches!(
        rest_words.first().copied(),
        Some("isnt" | "isn't" | "arent" | "aren't")
    ) {
        rest_words.remove(0);
        true
    } else {
        if matches!(rest_words.first().copied(), Some("is" | "are" | "s" | "’s")) {
            rest_words.remove(0);
        }
        false
    };
    if slice_ends_with(&rest_words, &["until", "end", "of", "turn"]) {
        duration = Until::EndOfTurn;
        let new_len = rest_words.len().saturating_sub(4);
        rest_words.truncate(new_len);
    }
    if rest_words.is_empty() {
        return Ok(None);
    }

    let negative_type_words = if negated {
        if rest_words
            .first()
            .copied()
            .is_some_and(|word| matches!(word, "a" | "an" | "the"))
        {
            Some(&rest_words[1..])
        } else {
            Some(&rest_words[..])
        }
    } else if slice_starts_with(&rest_words, &["not", "a"]) && rest_words.len() > 2 {
        Some(&rest_words[2..])
    } else if slice_starts_with(&rest_words, &["not", "an"]) && rest_words.len() > 2 {
        Some(&rest_words[2..])
    } else if slice_starts_with(&rest_words, &["not"]) && rest_words.len() > 1 {
        Some(&rest_words[1..])
    } else {
        None
    };
    if let Some(type_words) = negative_type_words {
        let mut card_types = Vec::new();
        let mut all_card_types = true;
        for word in type_words {
            if let Some(card_type) = parse_card_type(word) {
                if !iter_contains(card_types.iter(), &card_type) {
                    card_types.push(card_type);
                }
            } else {
                all_card_types = false;
                break;
            }
        }
        if all_card_types && !card_types.is_empty() {
            return Ok(Some(vec![EffectAst::RemoveCardTypes {
                target,
                card_types,
                duration,
            }]));
        }
    }

    let addition_tail_len = if slice_ends_with(
        &rest_words,
        &["in", "addition", "to", "its", "other", "types"],
    ) || slice_ends_with(
        &rest_words,
        &["in", "addition", "to", "their", "other", "types"],
    ) || slice_ends_with(
        &rest_words,
        &["in", "addition", "to", "its", "other", "type"],
    ) || slice_ends_with(
        &rest_words,
        &["in", "addition", "to", "their", "other", "type"],
    ) {
        Some(6usize)
    } else {
        None
    };

    let body_words = if rest_words
        .first()
        .is_some_and(|word| matches!(*word, "a" | "an" | "the"))
    {
        &rest_words[1..]
    } else {
        &rest_words[..]
    };
    if body_words.is_empty() {
        return Ok(None);
    }

    if let Ok((power, toughness)) = parse_pt_modifier_values(body_words[0])
        && let Some(tail_len) = addition_tail_len
        && body_words.len() > 1 + tail_len
    {
        let subtype_words = &body_words[1..body_words.len().saturating_sub(tail_len)];
        let mut subtypes = Vec::new();
        for word in subtype_words {
            let Some(subtype) = parse_pluralized_subtype_word(word) else {
                return Ok(None);
            };
            if !iter_contains(subtypes.iter(), &subtype) {
                subtypes.push(subtype);
            }
        }
        if subtypes.is_empty() {
            return Ok(None);
        }
        return Ok(Some(vec![
            EffectAst::SetBasePowerToughness {
                power,
                toughness,
                target: target.clone(),
                duration: duration.clone(),
            },
            EffectAst::AddSubtypes {
                target,
                subtypes,
                duration,
            },
        ]));
    }

    let type_words = if let Some(tail_len) = addition_tail_len {
        &body_words[..body_words.len().saturating_sub(tail_len)]
    } else {
        body_words
    };
    if type_words.is_empty() {
        return Ok(None);
    }

    let mut card_types = Vec::new();
    let mut all_card_types = true;
    for word in type_words {
        if let Some(card_type) = parse_card_type(word) {
            if !iter_contains(card_types.iter(), &card_type) {
                card_types.push(card_type);
            }
        } else {
            all_card_types = false;
            break;
        }
    }
    if all_card_types && !card_types.is_empty() {
        return Ok(Some(vec![EffectAst::AddCardTypes {
            target,
            card_types,
            duration,
        }]));
    }

    let mut subtypes = Vec::new();
    for word in type_words {
        let Some(subtype) = parse_pluralized_subtype_word(word) else {
            return Ok(None);
        };
        if !iter_contains(subtypes.iter(), &subtype) {
            subtypes.push(subtype);
        }
    }
    if subtypes.is_empty() {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::AddSubtypes {
        target,
        subtypes,
        duration,
    }]))
}

