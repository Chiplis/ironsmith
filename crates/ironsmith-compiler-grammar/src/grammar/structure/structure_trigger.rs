use super::*;

pub fn split_triggered_conditional_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    start_idx: usize,
) -> Option<TriggeredConditionalClauseSpec<'a>> {
    // The trigger subject may itself contain a serial type/subtype list. Find
    // the comma whose following clause actually begins with `if`, rather than
    // assuming the first comma ends the trigger header.
    let (leading_tokens, after_if) = primitives::split_lexed_once_on_separator(tokens, || {
        (primitives::comma(), primitives::kw("if")).void()
    })?;
    if leading_tokens.len() <= start_idx {
        return None;
    }

    let trigger_tokens = &leading_tokens[start_idx..];

    let mut comma_indices = Vec::new();
    let mut inside_quotes = false;
    for (comma_idx, token) in after_if.iter().enumerate() {
        if is_sentence_quote(token) {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes || !token.is_comma() {
            continue;
        }
        comma_indices.push(comma_idx);
    }

    // An ordinal triggering-spell condition may itself be a comma-separated
    // union ("the first instant spell, the first sorcery spell, or ...").
    // Its grammar consumes the complete `cast this turn` suffix, so it gives
    // us an exact predicate/effect boundary before the permissive general
    // predicate splitter examines an individually valid first member.
    for &comma_idx in comma_indices.iter().rev() {
        let predicate_tokens = trim_lexed_commas(&after_if[..comma_idx]);
        let effects_tokens = trim_lexed_commas(&after_if[comma_idx + 1..]);
        if predicate_tokens.is_empty() || effects_tokens.is_empty() {
            continue;
        }
        if let Some(predicate) =
            super::super::filters::parse_triggering_spell_ordinal_predicate(predicate_tokens)
        {
            return Some(TriggeredConditionalClauseSpec {
                trigger_tokens,
                predicate,
                effects_tokens,
            });
        }
    }

    // The first comma after `if` is normally the predicate/effect boundary;
    // later commas belong to the coordinated effect body.  Prefer that
    // boundary so an effect such as `copy ..., you may ..., and ...` cannot be
    // absorbed into the predicate merely because its final tail happens to
    // look like a valid split.
    for &comma_idx in &comma_indices {
        let predicate_tokens = trim_lexed_commas(&after_if[..comma_idx]);
        let effects_tokens = trim_lexed_commas(&after_if[comma_idx + 1..]);
        if predicate_tokens.is_empty() || effects_tokens.is_empty() {
            continue;
        }
        if contains_token_kind(predicate_tokens, TokenKind::Period) {
            continue;
        }
        if effects_tokens
            .first()
            .is_some_and(|token| structure_token_is_any(token, &["and", "then"]))
        {
            continue;
        }
        // Search effects commonly contain follow-up commas. When candidates are
        // examined from right to left, a later comma can otherwise absorb the
        // search action into a permissively modeled predicate and leave only a
        // put/reveal/shuffle follow-up as the effect.
        if predicate_candidate_contains_search_action(predicate_tokens) {
            continue;
        }
        if predicate_candidate_contains_damage_action(predicate_tokens) {
            continue;
        }
        // A moved-or-cast origin clause ("it entered from your graveyard or
        // you cast it from your graveyard") scopes the trigger event itself;
        // leave it for the trigger parser instead of modeling it as an
        // intervening-if predicate.
        if crate::activation_and_restrictions::trigger_clause_core::clause_words_are_moved_or_cast_origin_condition(
            &crate::lexer::token_word_refs(predicate_tokens),
        ) {
            continue;
        }
        // A duration following a comma belongs to the effect clause. Reject
        // this later split candidate so the preceding comma can preserve the
        // duration at the head of `effects_tokens`. A predicate that itself
        // ends in "this turn" has no separating comma and remains valid.
        if leaf::parse_leaf_restriction_duration_suffix_tokens(predicate_tokens).is_some_and(
            |shape| {
                shape
                    .rest
                    .last()
                    .is_some_and(|token| token.kind == TokenKind::Comma)
            },
        ) {
            continue;
        }
        if let Some(predicate) = parse_modeled_predicate(predicate_tokens) {
            if let Some(next_comma_position) =
                crate::slice_primitives::select_position(&comma_indices, |next_idx| {
                    *next_idx > comma_idx
                })
            {
                let next_comma_idx = comma_indices[next_comma_position];
                let mut next_fragment = trim_lexed_commas(&after_if[comma_idx + 1..next_comma_idx]);
                if next_fragment
                    .first()
                    .is_some_and(|token| structure_token_is_any(token, &["and", "or"]))
                {
                    next_fragment = trim_lexed_commas(&next_fragment[1..]);
                }
                if !next_fragment.is_empty()
                    && !contains_token_kind(next_fragment, TokenKind::Period)
                    && !predicate_candidate_contains_damage_action(next_fragment)
                    && parse_modeled_predicate(next_fragment).is_some()
                {
                    continue;
                }
            }
            return Some(TriggeredConditionalClauseSpec {
                trigger_tokens,
                predicate,
                effects_tokens,
            });
        }
    }

    None
}

pub fn split_state_triggered_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    start_idx: usize,
    split_idx: usize,
) -> Option<StateTriggeredClauseSpec<'a>> {
    if split_idx <= start_idx || split_idx >= tokens.len() {
        return None;
    }
    if !tokens
        .first()
        .is_some_and(|token| structure_token_is_any(token, &["when", "whenever"]))
    {
        return None;
    }

    let trigger_tokens = &tokens[start_idx..split_idx];
    let effects_tokens = trim_lexed_commas(&tokens[split_idx + 1..]);
    if effects_tokens.is_empty() {
        return None;
    }

    let predicate = if let Some(comma_idx) =
        structure_token_kind_index(trigger_tokens, TokenKind::Comma)
        && trigger_tokens
            .get(comma_idx + 1)
            .is_some_and(|token| structure_token_is(token, "if"))
    {
        let state_predicate =
            parse_modeled_predicate(trim_lexed_commas(&trigger_tokens[..comma_idx]))?;
        let gate_predicate =
            parse_modeled_predicate(trim_lexed_commas(&trigger_tokens[comma_idx + 2..]))?;
        PredicateAst::And(Box::new(state_predicate), Box::new(gate_predicate))
    } else {
        parse_modeled_predicate(trigger_tokens)?
    };

    Some(StateTriggeredClauseSpec {
        trigger_tokens,
        display_tokens: &tokens[..split_idx],
        predicate,
        effects_tokens,
    })
}
