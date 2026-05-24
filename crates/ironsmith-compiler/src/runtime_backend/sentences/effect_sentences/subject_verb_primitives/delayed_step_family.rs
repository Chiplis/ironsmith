use super::*;

pub(super) fn wrap_delayed_next_step_unless_pays(
    step: DelayedNextStepKind,
    player: PlayerAst,
    effects: Vec<EffectAst>,
) -> EffectAst {
    match step {
        DelayedNextStepKind::Upkeep => EffectAst::DelayedUntilNextUpkeep { player, effects },
        DelayedNextStepKind::DrawStep => EffectAst::DelayedUntilNextDrawStep { player, effects },
    }
}

pub(crate) fn find_unquoted_token_word(tokens: &[OwnedLexToken], word: &str) -> Option<usize> {
    let mut inside_quotes = false;
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_quote() {
            inside_quotes = !inside_quotes;
            continue;
        }
        if !inside_quotes && token.is_word(word) {
            return Some(idx);
        }
    }
    None
}

fn bind_unless_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::UnlessPays {
            player: unless_player,
            effects,
            ..
        } => {
            if matches!(*unless_player, PlayerAst::Implicit) {
                *unless_player = player;
            }
            for nested in effects {
                bind_unless_player_context(nested, player);
            }
        }
        EffectAst::UnlessAction {
            player: unless_player,
            effects,
            alternative,
        } => {
            if matches!(*unless_player, PlayerAst::Implicit) {
                *unless_player = player;
            }
            for nested in effects {
                bind_unless_player_context(nested, player);
            }
            for nested in alternative {
                bind_unless_player_context(nested, player);
            }
        }
        _ => bind_implicit_player_context(effect, player),
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
    let upkeep_words = crate::runtime_backend::token_word_refs(&upkeep_tokens);
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
        use super::super::super::grammar::primitives as grammar;
        use super::super::super::lexer::LexStream;
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

    let lose_words = crate::runtime_backend::token_word_refs(&lose_tokens);
    let valid_lose_clause = lose_words == ["if", "you", "dont", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "do", "not", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "don't", "you", "lose", "the", "game"];
    if !valid_lose_clause {
        return Ok(None);
    }

    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::subject_verb_lose_game(PlayerAst::You)],
            player: PlayerAst::You,
            cost: crate::cost::TotalCost::mana(crate::mana::ManaCost::from_symbols(mana)),
        }],
    });
    Ok(Some(effects))
}

fn normalize_unless_payment_clause_tokens(
    action_tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let mut tokens = trim_commas(action_tokens);
    let first = tokens.first()?.as_word()?;
    let normalized_first = match first {
        "pay" | "pays" => "pay",
        "sacrifice" | "sacrifices" => "sacrifice",
        _ => return None,
    };

    if tokens[0].as_word() != Some(normalized_first) {
        tokens[0].replace_word(normalized_first);
    }

    if let Some(before_idx) = find_index(&tokens, |token| token.is_word("before")) {
        tokens.truncate(before_idx);
    }

    Some(trim_commas(&tokens))
}

fn parse_unless_payment_clause_as_cost(
    action_tokens: &[OwnedLexToken],
) -> Result<Option<crate::cost::TotalCost>, CardTextError> {
    let Some(payment_tokens) = normalize_unless_payment_clause_tokens(action_tokens) else {
        return Ok(None);
    };
    crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(
        &payment_tokens,
    )
}

/// Try to build an UnlessPays or UnlessAction AST from the tokens after "unless".
/// Returns the unless wrapper containing the given `effects` as the main effects.
pub(crate) fn try_build_unless(
    effects: Vec<EffectAst>,
    tokens: &[OwnedLexToken],
    unless_idx: usize,
) -> Result<Option<EffectAst>, CardTextError> {
    let after_unless = &tokens[unless_idx + 1..];
    let after_word_storage = SubjectVerbPrimitiveNormalizedWords::new(after_unless);
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
    let action_word_storage = SubjectVerbPrimitiveNormalizedWords::new(action_tokens);
    let action_words = action_word_storage.to_word_refs();

    if action_words.first() == Some(&"pay") || action_words.first() == Some(&"pays") {
        if contains_word_window(&action_words, &["mana", "cost"]) {
            return Err(CardTextError::ParseError(format!(
                "unsupported unless-payment mana-cost clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
    } else if matches!(action_words.first(), Some(&"draw" | &"draws")) {
        return Err(CardTextError::ParseError(format!(
            "unsupported non-cost unless action (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    if matches!(
        action_words.first().copied(),
        Some("sacrifice" | "sacrifices")
    ) && let Ok(mut alternative) = super::super::zone_handlers::parse_sacrifice(
        action_tokens,
        Some(SubjectAst::Player(player)),
        None,
    )
    .map(|effect| vec![effect])
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    if let Some(cost) = parse_unless_payment_clause_as_cost(action_tokens)? {
        return Ok(Some(EffectAst::UnlessPays {
            effects,
            player,
            cost,
        }));
    }

    // Prefer the action-only slice for explicit-player clauses like
    // "unless that player discards ... or sacrifices ...". Parsing the full
    // clause first can flatten the trailing "or" branch into the first action.
    if let Ok(mut alternative) = parse_effect_chain(action_tokens) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    // Fall back to the full clause when the action-only parse needs the
    // explicit player prefix to succeed.
    if let Ok(mut alternative) = parse_effect_chain(after_unless) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
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
                bind_unless_player_context(effect, player);
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
                bind_unless_player_context(effect, player);
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
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if matches!(action_words.first().copied(), Some("discard" | "discards"))
        && let Ok(mut alternative) = super::super::zone_handlers::parse_discard(action_tokens, None)
            .map(|effect| vec![effect])
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn try_build_unless_prefers_action_only_parse_for_explicit_player_or_choice() {
        let tokens = lex_line(
            "Target opponent loses 5 life unless that player discards two cards or sacrifices a creature or planeswalker of their choice.",
            0,
        )
        .expect("rewrite lexer should classify explicit-player unless choice");
        let unless_idx = find_token_word(&tokens, "unless").expect("unless token");
        let effects = parse_effect_chain(&tokens[..unless_idx])
            .expect("lead effect should parse before unless clause");

        let unless_effect = try_build_unless(effects, &tokens, unless_idx)
            .expect("unless choice should parse")
            .expect("unless choice should lower");
        let debug = format!("{unless_effect:?}");

        assert!(
            debug.contains("Discard"),
            "expected explicit-player unless choice to keep the discard branch, got {debug}"
        );
        assert!(
            debug.contains("Sacrifice"),
            "expected explicit-player unless choice to keep the sacrifice branch, got {debug}"
        );
        assert!(
            debug.contains("TargetOpponent"),
            "expected explicit-player unless choice to bind the target opponent context, got {debug}"
        );
    }
}

pub(crate) fn parse_sentence_fallback_mechanic_marker(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if parse_cast_or_play_tagged_clause(tokens)?.is_some() {
        return Ok(None);
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.as_slice() == ["venture", "into", "the", "dungeon"] {
        return Ok(Some(vec![EffectAst::subject_verb_venture_into_dungeon(
            crate::cards::builders::PlayerAst::You,
            false,
        )]));
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
            return Ok(Some(vec![EffectAst::subject_verb_remove_card_types(
                target, card_types, duration,
            )]));
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
            EffectAst::subject_verb_set_base_power_toughness(
                power,
                toughness,
                target.clone(),
                duration.clone(),
            ),
            EffectAst::subject_verb_add_subtypes(target, subtypes, duration),
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
        return Ok(Some(vec![EffectAst::subject_verb_add_card_types(
            target, card_types, duration,
        )]));
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

    Ok(Some(vec![EffectAst::subject_verb_add_subtypes(
        target, subtypes, duration,
    )]))
}

pub(crate) fn parse_sentence_gains_or_loses_all_creature_types(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let Some(verb_idx) = words
        .iter()
        .position(|word| matches!(*word, "gain" | "gains" | "lose" | "loses"))
    else {
        return Ok(None);
    };
    let is_gain = matches!(words[verb_idx], "gain" | "gains");
    let tail = &words[verb_idx + 1..];
    if tail != ["all", "creature", "types", "until", "end", "of", "turn"]
        && tail != ["every", "creature", "type", "until", "end", "of", "turn"]
    {
        return Ok(None);
    }

    if !is_gain
        && let Some(get_word_idx) = words[..verb_idx]
            .iter()
            .position(|word| matches!(*word, "get" | "gets"))
    {
        let Some(get_token_idx) = token_index_for_word_index(tokens, get_word_idx) else {
            return Ok(None);
        };
        let Some(modifier_word) = words.get(get_word_idx + 1).copied() else {
            return Ok(None);
        };
        let Ok((power, toughness)) = parse_pt_modifier_values(modifier_word) else {
            return Ok(None);
        };
        let target_tokens = trim_commas(&tokens[..get_token_idx]);
        if target_tokens.is_empty() {
            return Ok(None);
        }
        let target = parse_target_phrase(&target_tokens)?;
        return Ok(Some(vec![
            EffectAst::subject_verb_pump(power, toughness, target.clone(), Until::EndOfTurn, None),
            EffectAst::subject_verb_remove_all_subtypes_of_family(
                target,
                crate::types::SubtypeFamily::Creature,
                Until::EndOfTurn,
            ),
        ]));
    }

    let verb_token_idx = token_index_for_word_index(tokens, verb_idx).unwrap_or(tokens.len());
    let target_tokens = trim_commas(&tokens[..verb_token_idx]);
    let target = if words[..verb_idx] == ["it"] || words[..verb_idx] == ["that", "creature"] {
        TargetAst::Tagged(TagKey::from(IT_TAG), None)
    } else {
        parse_target_phrase(&target_tokens)?
    };
    let effect = if is_gain {
        EffectAst::subject_verb_add_all_subtypes_of_family(
            target,
            crate::types::SubtypeFamily::Creature,
            Until::EndOfTurn,
        )
    } else {
        EffectAst::subject_verb_remove_all_subtypes_of_family(
            target,
            crate::types::SubtypeFamily::Creature,
            Until::EndOfTurn,
        )
    };
    Ok(Some(vec![effect]))
}

fn fixed_count_word(word: &str) -> Option<i32> {
    ironsmith_core::parse_cardinal_word(word).and_then(|value| value.try_into().ok())
}

pub(crate) fn parse_sentence_lose_draw_clash_repeat_process(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let if_idx = words
        .windows(3)
        .position(|window| window == ["if", "you", "win"]);
    let body_words = if let Some(if_idx) = if_idx {
        if words.get(if_idx + 3..) != Some(&["repeat", "this", "process"][..]) {
            return Ok(None);
        }
        &words[..if_idx]
    } else {
        &words[..]
    };
    if body_words.len() != 13
        || words[0] != "you"
        || words[1] != "lose"
        || words[3] != "life"
        || words[4] != "and"
        || words[5] != "draw"
        || !matches!(words[7], "card" | "cards")
        || body_words[8] != "then"
        || body_words[9] != "clash"
        || body_words[10] != "with"
        || body_words[11] != "an"
        || body_words[12] != "opponent"
    {
        return Ok(None);
    }
    let Some(life_count) = fixed_count_word(body_words[2]) else {
        return Ok(None);
    };
    let Some(draw_count) = fixed_count_word(body_words[6]) else {
        return Ok(None);
    };

    let effects = vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LoseLife {
                amount: Value::Fixed(life_count),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: Value::Fixed(draw_count),
            },
        ),
        EffectAst::subject_verb_clash(ClashOpponentAst::Opponent),
    ];
    if if_idx.is_none() {
        return Ok(Some(effects));
    }

    Ok(Some(vec![EffectAst::RepeatProcess {
        effects,
        continue_effect_index: 2,
        continue_predicate: IfResultPredicate::Value(crate::effect::Comparison::GreaterThan(0)),
    }]))
}
