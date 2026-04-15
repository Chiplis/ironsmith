pub(crate) fn parse_effect_sentence_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let word_view = LexClauseView::from_tokens(tokens);
    let sentence_words = word_view.words.to_word_refs();
    if is_activate_only_restriction_sentence_lexed(tokens) {
        return Ok(Vec::new());
    }
    if is_trigger_only_restriction_sentence_lexed(tokens) {
        return Ok(Vec::new());
    }
    if slice_starts_with(sentence_words.as_slice(), &["round", "up", "each", "time"]) {
        return Ok(Vec::new());
    }

    if let Some(stripped) = split_labeled_effect_prefix_lexed(tokens) {
        return parse_effect_sentence_lexed(stripped);
    }
    if tokens.first().is_some_and(|token| token.is_word("if"))
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            PRE_CONDITIONAL_SENTENCE_PRIMITIVES,
            &PRE_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(effects) =
        parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)?
    {
        return Ok(effects);
    }
    if let Some(effects) = parse_next_spell_grant_sentence_lexed(tokens)? {
        return Ok(effects);
    }
    if tokens.first().is_some_and(|token| token.is_word("exile"))
        && grammar::contains_word(tokens, "then")
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if tokens.first().is_some_and(|token| token.is_word("then")) && tokens.len() > 1 {
        return parse_effect_sentence_lexed(&tokens[1..]);
    }
    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        return Ok(vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: parse_effect_sentence_lexed(prefix.trailing_tokens)?,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: parse_effect_sentence_lexed(prefix.trailing_tokens)?,
            },
        }]);
    }
    if tokens
        .iter()
        .any(|token| token.is_word("search") || token.is_word("searches"))
        && let Some(mut effects) = parse_search_library_sentence_lexed(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if slice_starts_with(
        sentence_words.as_slice(),
        &["exile", "all", "cards", "from"],
    ) && slice_contains_any(sentence_words.as_slice(), &["hand", "hands"])
        && slice_contains_any(sentence_words.as_slice(), &["graveyard", "graveyards"])
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words.first() == Some(&"enchant")
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if slice_contains(sentence_words.as_slice(), &"unless")
        && let Some(mut effects) = super::parse_sentence_unless_pays(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if slice_contains(sentence_words.as_slice(), &"unless")
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words
        .iter()
        .any(|word| matches!(*word, "gain" | "gains" | "lose" | "loses"))
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words
        .iter()
        .any(|word| *word == "vote" || *word == "votes")
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words.first() == Some(&"return")
        && slice_contains(sentence_words.as_slice(), &"rounded")
        && slice_contains(sentence_words.as_slice(), &"up")
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words.first() == Some(&"choose")
        && contains_word_window(sentence_words.as_slice(), &["do", "the", "same", "for"])
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if slice_starts_with_any(
        sentence_words.as_slice(),
        &[
            &["each", "player", "choose"],
            &["each", "player", "chooses"],
        ],
    ) {
        if let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            PRE_CONDITIONAL_SENTENCE_PRIMITIVES,
            &PRE_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
    }
    if let Some(diag) = super::sentence_unsupported::diagnose_sentence_unsupported_lexed(tokens) {
        return Err(diag);
    }
    if super::parse_leading_player_may_lexed(tokens).is_some() {
        return parse_effect_chain_lexed(tokens);
    }
    if super::looks_like_multi_create_chain_lexed(tokens) {
        if let Some(unless_action) = super::parse_or_action_clause_lexed(tokens)? {
            return Ok(vec![unless_action]);
        }
        return super::parse_effect_chain_inner_lexed(tokens);
    }

    let (_, effects) = super::sentence_registry::run_sentence_parse_rules_lexed(tokens)?;
    Ok(effects)
}

pub(crate) fn is_negated_untap_clause(words: &[&str]) -> bool {
    effect_grammar::is_negated_untap_clause_words(words)
}

pub(crate) fn parse_token_copy_modifier_sentence(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = crate::cards::builders::compiler::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let is_gain_haste_until_eot = matches!(
        filtered.as_slice(),
        ["it", "gains", "haste", "until", "end", "of", "turn"]
            | ["they", "gain", "haste", "until", "end", "of", "turn"]
    );
    if is_gain_haste_until_eot {
        return Some(TokenCopyFollowup::GainHasteUntilEndOfTurn);
    }

    let is_has_haste = matches!(
        filtered.as_slice(),
        ["it", "has", "haste"]
            | ["they", "have", "haste"]
            | ["token", "created", "this", "way", "has", "haste"]
            | ["tokens", "created", "this", "way", "have", "haste"]
            | ["token", "created", "this", "way", "gains", "haste"]
            | ["tokens", "created", "this", "way", "gain", "haste"]
    );
    if is_has_haste {
        return Some(TokenCopyFollowup::HasHaste);
    }

    let enters_tapped_and_attacking = matches!(
        filtered.as_slice(),
        ["it", "enters", "tapped", "and", "attacking"]
            | ["they", "enter", "tapped", "and", "attacking"]
            | ["token", "enters", "tapped", "and", "attacking"]
            | ["tokens", "enter", "tapped", "and", "attacking"]
            | [
                "token",
                "created",
                "this",
                "way",
                "enters",
                "tapped",
                "and",
                "attacking"
            ]
            | [
                "tokens",
                "created",
                "this",
                "way",
                "enter",
                "tapped",
                "and",
                "attacking"
            ]
    );
    if enters_tapped_and_attacking {
        return Some(TokenCopyFollowup::EnterTappedAndAttacking);
    }

    if slice_starts_with_any(
        filtered.as_slice(),
        &[
            &["sacrifice", "it"],
            &["sacrifice", "them"],
            &["sacrifice", "that", "token"],
            &["sacrifice", "those", "tokens"],
        ],
    ) {
        let has_next_end_step = contains_word_window(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
        }
    }
    if slice_starts_with_any(filtered.as_slice(), &[&["exile", "it"], &["exile", "them"]]) {
        let has_next_end_step = contains_word_window(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::ExileAtNextEndStep);
        }
    }

    let starts_delayed_end_step_sacrifice = slice_starts_with_any(
        filtered.as_slice(),
        &[
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "sacrifice",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "sacrifice",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "sacrifice",
            ],
        ],
    );
    if starts_delayed_end_step_sacrifice {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }
    let starts_delayed_end_step_exile = slice_starts_with_any(
        filtered.as_slice(),
        &[
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "exile",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "exile",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "exile",
            ],
        ],
    );
    if starts_delayed_end_step_exile {
        return Some(TokenCopyFollowup::ExileAtNextEndStep);
    }

    None
}

pub(crate) fn parse_token_copy_modifier_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = crate::cards::builders::compiler::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let is_gain_haste_until_eot = matches!(
        filtered.as_slice(),
        ["it", "gains", "haste", "until", "end", "of", "turn"]
            | ["they", "gain", "haste", "until", "end", "of", "turn"]
    );
    if is_gain_haste_until_eot {
        return Some(TokenCopyFollowup::GainHasteUntilEndOfTurn);
    }

    let is_has_haste = matches!(
        filtered.as_slice(),
        ["it", "has", "haste"]
            | ["they", "have", "haste"]
            | ["token", "created", "this", "way", "has", "haste"]
            | ["tokens", "created", "this", "way", "have", "haste"]
            | ["token", "created", "this", "way", "gains", "haste"]
            | ["tokens", "created", "this", "way", "gain", "haste"]
    );
    if is_has_haste {
        return Some(TokenCopyFollowup::HasHaste);
    }

    let enters_tapped_and_attacking = matches!(
        filtered.as_slice(),
        ["token", "enters", "tapped", "and", "attacking"]
            | ["tokens", "enter", "tapped", "and", "attacking"]
            | [
                "token",
                "created",
                "this",
                "way",
                "enters",
                "tapped",
                "and",
                "attacking"
            ]
            | [
                "tokens",
                "created",
                "this",
                "way",
                "enter",
                "tapped",
                "and",
                "attacking"
            ]
    );
    if enters_tapped_and_attacking {
        return Some(TokenCopyFollowup::EnterTappedAndAttacking);
    }

    if slice_starts_with_any(
        filtered.as_slice(),
        &[
            &["sacrifice", "it"],
            &["sacrifice", "them"],
            &["sacrifice", "that", "token"],
            &["sacrifice", "those", "tokens"],
        ],
    ) {
        let has_next_end_step = contains_word_window(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
        }
    }
    if slice_starts_with_any(filtered.as_slice(), &[&["exile", "it"], &["exile", "them"]]) {
        let has_next_end_step = contains_word_window(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::ExileAtNextEndStep);
        }
    }

    let starts_delayed_end_step_sacrifice = slice_starts_with_any(
        filtered.as_slice(),
        &[
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "sacrifice",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "sacrifice",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "sacrifice",
            ],
        ],
    );
    if starts_delayed_end_step_sacrifice {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }
    let starts_delayed_end_step_exile = slice_starts_with_any(
        filtered.as_slice(),
        &[
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "exile",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "exile",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "exile",
            ],
        ],
    );
    if starts_delayed_end_step_exile {
        return Some(TokenCopyFollowup::ExileAtNextEndStep);
    }

    None
}
