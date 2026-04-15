pub(crate) fn parse_monstrosity_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.first().copied() != Some("monstrosity") {
        return Ok(None);
    }

    let amount_tokens = &tokens[1..];
    let (amount, _) = parse_value(amount_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing monstrosity amount (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Monstrosity { amount }))
}

pub(crate) fn parse_for_each_counter_removed_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if grammar::words_match_prefix(
        tokens,
        &["for", "each", "counter", "removed", "this", "way"],
    )
    .is_none()
    {
        return Ok(None);
    }

    let remainder = if let Some((_before, after)) =
        super::super::grammar::primitives::split_lexed_once_on_delimiter(
            tokens,
            super::super::lexer::TokenKind::Comma,
        ) {
        after
    } else {
        &tokens[6..]
    };

    let remainder_words = crate::cards::builders::compiler::token_word_refs(remainder);
    if remainder_words.is_empty() {
        return Ok(None);
    }

    let gets_idx = find_index(remainder_words.as_slice(), |word| {
        matches!(*word, "gets" | "get")
    });
    let Some(gets_idx) = gets_idx else {
        return Ok(None);
    };

    let subject_tokens = &remainder[..gets_idx];
    let subject = parse_subject(subject_tokens);
    let target = match subject {
        SubjectAst::This => TargetAst::Source(None),
        _ => return Ok(None),
    };

    let after_gets = &remainder[gets_idx + 1..];
    let modifier_token = after_gets
        .first()
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier (clause: '{}')",
                remainder_words.join(" ")
            ))
        })?;
    let (power, toughness) = parse_pt_modifier(modifier_token)?;

    let duration = if grammar::contains_word(remainder, "until")
        && grammar::contains_word(remainder, "end")
        && grammar::contains_word(remainder, "turn")
    {
        Until::EndOfTurn
    } else {
        Until::EndOfTurn
    };

    Ok(Some(EffectAst::PumpByLastEffect {
        power,
        toughness,
        target,
        duration,
    }))
}

pub(crate) fn is_exile_that_token_at_end_of_combat(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.first().copied() != Some("exile") {
        return false;
    }

    let at_idx = if matches!(words.get(1).copied(), Some("that" | "the" | "those"))
        && matches!(words.get(2).copied(), Some("token" | "tokens"))
    {
        3
    } else if words.get(1).copied() == Some("it") {
        2
    } else {
        return false;
    };
    if words.get(at_idx).copied() != Some("at") {
        return false;
    }
    has_end_of_combat_tail(&words, at_idx)
}

pub(crate) fn is_exile_that_token_at_end_of_combat_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.first().copied() != Some("exile") {
        return false;
    }

    let at_idx = if matches!(words.get(1).copied(), Some("that" | "the" | "those"))
        && matches!(words.get(2).copied(), Some("token" | "tokens"))
    {
        3
    } else if words.get(1).copied() == Some("it") {
        2
    } else {
        return false;
    };
    if words.get(at_idx).copied() != Some("at") {
        return false;
    }
    has_end_of_combat_tail(&words, at_idx)
}

pub(crate) fn is_sacrifice_that_token_at_end_of_combat(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.first().copied() != Some("sacrifice") {
        return false;
    }

    let at_idx = if matches!(words.get(1).copied(), Some("that" | "the" | "those"))
        && matches!(words.get(2).copied(), Some("token" | "tokens"))
    {
        3
    } else if words.get(1).copied() == Some("it") {
        2
    } else {
        return false;
    };
    if words.get(at_idx).copied() != Some("at") {
        return false;
    }
    has_end_of_combat_tail(&words, at_idx)
}

pub(crate) fn is_sacrifice_that_token_at_end_of_combat_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.first().copied() != Some("sacrifice") {
        return false;
    }

    let at_idx = if matches!(words.get(1).copied(), Some("that" | "the" | "those"))
        && matches!(words.get(2).copied(), Some("token" | "tokens"))
    {
        3
    } else if words.get(1).copied() == Some("it") {
        2
    } else {
        return false;
    };
    if words.get(at_idx).copied() != Some("at") {
        return false;
    }
    has_end_of_combat_tail(&words, at_idx)
}

fn has_end_of_combat_tail(words: &[&str], at_idx: usize) -> bool {
    matches!(
        words.get(at_idx + 1..),
        Some(["end", "of", "combat"] | ["the", "end", "of", "combat"])
    )
}

pub(crate) fn parse_take_extra_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if matches!(
        words.as_slice(),
        ["take", "an", "extra", "turn", "after", "this", "one"]
    ) {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn {
            player: PlayerAst::You,
            anchor: ExtraTurnAnchorAst::CurrentTurn,
        }));
    }
    if matches!(
        words.as_slice(),
        [
            "the", "chosen", "player", "takes", "an", "extra", "turn", "after", "this", "one"
        ]
    ) {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn {
            player: PlayerAst::Chosen,
            anchor: ExtraTurnAnchorAst::CurrentTurn,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_destroy_or_exile_all_split_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    let verb = if words[0] == "destroy" {
        Some(Verb::Destroy)
    } else if words[0] == "exile" {
        Some(Verb::Exile)
    } else {
        None
    };
    let Some(verb) = verb else {
        return Ok(None);
    };
    if words[1] != "all"
        || !grammar::contains_word(tokens, "and")
        || grammar::contains_word(tokens, "except")
    {
        return Ok(None);
    }
    if grammar::words_match_any_prefix(tokens, EXILE_ALL_CARDS_FROM_PREFIXES).is_some()
        && (grammar::contains_word(tokens, "hand") || grammar::contains_word(tokens, "hands"))
        && (grammar::contains_word(tokens, "graveyard")
            || grammar::contains_word(tokens, "graveyards"))
    {
        return Ok(None);
    }

    let mut raw_segments = Vec::new();
    let mut current = Vec::new();
    for token in &tokens[2..] {
        if token.is_word("and") || token.is_comma() {
            if !current.is_empty() {
                raw_segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        raw_segments.push(current);
    }

    let mut effects = Vec::new();
    for mut segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        if segment.first().is_some_and(|token| token.is_word("all")) {
            segment.remove(0);
        }
        if segment.is_empty() {
            continue;
        }
        let filter = parse_object_filter(&segment, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in split all clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        let effect = match verb {
            Verb::Destroy => EffectAst::DestroyAll { filter },
            Verb::Exile => EffectAst::ExileAll {
                filter,
                face_down: false,
            },
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported split all clause verb".to_string(),
                ));
            }
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        return Ok(Some(effects));
    }
    Ok(None)
}

pub(crate) fn parse_exile_then_return_same_object_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn target_references_it_tag(target: &TargetAst) -> bool {
        match target {
            TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
            TargetAst::Object(filter, _, _) => filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
            }),
            _ => false,
        }
    }

    let mut clause_tokens = tokens;
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("you"))
        && clause_tokens
            .get(1)
            .is_some_and(|token| token.is_word("exile"))
    {
        clause_tokens = &clause_tokens[1..];
    }

    let words_all = crate::cards::builders::compiler::token_word_refs(clause_tokens);
    if words_all.first().copied() != Some("exile")
        || !grammar::contains_word(clause_tokens, "then")
        || !grammar::contains_word(clause_tokens, "return")
    {
        return Ok(None);
    }

    let split_idx = find_window_by(clause_tokens, 3, |window: &[OwnedLexToken]| {
        window[0].is_comma() && window[1].is_word("then") && window[2].is_word("return")
    });
    let Some(split_idx) = split_idx else {
        return Ok(None);
    };

    let first_clause = &clause_tokens[..split_idx];
    let second_clause = &clause_tokens[split_idx + 2..];
    if first_clause.is_empty() || second_clause.is_empty() {
        return Ok(None);
    }

    let (first_clause, delayed_until_end_of_combat) = if let Some(before) =
        grammar::strip_lexed_suffix_phrase(first_clause, &["at", "the", "end", "of", "combat"])
    {
        (before, true)
    } else if let Some(before) =
        grammar::strip_lexed_suffix_phrase(first_clause, &["at", "end", "of", "combat"])
    {
        (before, true)
    } else {
        (first_clause, false)
    };

    let mut first_effects = parse_effect_chain_inner(first_clause)?;
    if !first_effects
        .iter()
        .any(|effect| matches!(effect, EffectAst::Exile { .. }))
    {
        return Ok(None);
    }

    // Preserve return follow-up clauses (for example "with a +1/+1 counter on it")
    // while still rewriting the "it" return target to the tagged exiled object.
    let mut second_effects =
        if let Some(effects) = parse_sentence_return_with_counters_on_it(second_clause)? {
            effects
        } else {
            parse_effect_chain_inner(second_clause)?
        };
    let mut rewrote_return = false;
    for effect in &mut second_effects {
        match effect {
            EffectAst::ReturnToBattlefield {
                target,
                tapped: _,
                transformed: _,
                converted: _,
                controller: _,
            } if target_references_it_tag(target) => {
                *target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
                rewrote_return = true;
            }
            EffectAst::ReturnToHand { target, random: _ } if target_references_it_tag(target) => {
                *target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
                rewrote_return = true;
            }
            _ => {}
        }
    }
    if !rewrote_return {
        return Ok(None);
    }

    if delayed_until_end_of_combat {
        let mut delayed_effects = first_effects;
        delayed_effects.extend(second_effects);
        return Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
            effects: delayed_effects,
        }]));
    }

    first_effects.extend(second_effects);
    Ok(Some(first_effects))
}

pub(crate) fn parse_exile_up_to_one_each_target_type_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.len() < 6 || words[0] != "exile" {
        return Ok(None);
    }
    if grammar::words_match_prefix(tokens, &["exile", "up", "to", "one", "target"]).is_none() {
        return Ok(None);
    }
    // This primitive is for repeated clauses like:
    // "Exile up to one target artifact, up to one target creature, ..."
    // Not for a single disjunctive target like:
    // "Exile up to one target artifact, creature, or enchantment ..."
    let target_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_word("target").then_some(idx))
        .collect();
    if target_positions.len() < 2 {
        return Ok(None);
    }
    for pos in target_positions.iter().skip(1) {
        if *pos < 3
            || !tokens[*pos - 3].is_word("up")
            || !tokens[*pos - 2].is_word("to")
            || !tokens[*pos - 1].is_word("one")
        {
            return Ok(None);
        }
    }

    let mut raw_segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    let mut current: Vec<OwnedLexToken> = Vec::new();
    for token in &tokens[1..] {
        if token.is_comma() || token.is_word("and") || token.is_word("or") {
            if !current.is_empty() {
                raw_segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        raw_segments.push(current);
    }

    let mut filters = Vec::new();
    for segment in raw_segments {
        let mut slice: &[OwnedLexToken] = &segment;
        if slice.len() >= 3
            && slice[0].is_word("up")
            && slice[1].is_word("to")
            && slice[2].is_word("one")
        {
            slice = &slice[3..];
        }
        if slice.first().is_some_and(|token| token.is_word("target")) {
            slice = &slice[1..];
        }
        if slice.is_empty() {
            continue;
        }

        let mut filter = parse_object_filter(slice, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in 'exile up to one each target type' clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        if filter.controller.is_none() {
            // Keep this unrestricted to avoid implicit "you control" defaulting in ChooseObjects compilation.
            filter.controller = Some(PlayerFilter::Any);
        }
        filters.push(filter);
    }

    if filters.len() < 2 {
        return Ok(None);
    }

    let tag = helper_tag_for_tokens(tokens, "exiled");
    let mut effects: Vec<EffectAst> = filters
        .into_iter()
        .map(|filter| EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        })
        .collect();
    effects.push(EffectAst::Exile {
        target: TargetAst::Tagged(tag, None),
        face_down: false,
    });

    Ok(Some(effects))
}

pub(crate) fn parse_look_at_hand_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.as_slice()
        == [
            "look",
            "at",
            "an",
            "opponents",
            "hand",
            "then",
            "choose",
            "any",
            "card",
            "name",
        ]
    {
        return Ok(Some(vec![
            EffectAst::LookAtHand {
                target: TargetAst::Player(PlayerFilter::Opponent, None),
            },
            EffectAst::ChooseCardName {
                player: PlayerAst::You,
                filter: None,
                tag: TagKey::from(IT_TAG),
            },
        ]));
    }
    if words.as_slice() == ["look", "at", "target", "players", "hand"]
        || words.as_slice() == ["look", "at", "target", "player", "hand"]
    {
        let target = TargetAst::Player(PlayerFilter::target_player(), Some(TextSpan::synthetic()));
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    if words.as_slice() == ["look", "at", "target", "opponent", "hand"]
        || words.as_slice() == ["look", "at", "target", "opponents", "hand"]
    {
        let target =
            TargetAst::Player(PlayerFilter::target_opponent(), Some(TextSpan::synthetic()));
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    if words.as_slice() == ["look", "at", "an", "opponents", "hand"]
        || words.as_slice() == ["look", "at", "opponents", "hand"]
    {
        let target = TargetAst::Player(PlayerFilter::Opponent, None);
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    if words.as_slice() == ["look", "at", "that", "players", "hand"] {
        let target = TargetAst::Player(PlayerFilter::IteratedPlayer, None);
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    Ok(None)
}

pub(crate) fn parse_look_at_top_then_exile_one_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let starts_with_look_top = slice_starts_with_any(
        &clause_words,
        &[&["look", "at", "the", "top"], &["look", "at", "top"]],
    );
    if !starts_with_look_top {
        return Ok(None);
    }

    let Some(top_idx) = find_index(tokens, |token| token.is_word("top")) else {
        return Ok(None);
    };
    let Some((count, used_count)) = parse_number(&tokens[top_idx + 1..]) else {
        return Ok(None);
    };
    let mut idx = top_idx + 1 + used_count;
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
    {
        idx += 1;
    }
    if !tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        return Ok(None);
    }
    idx += 1;

    let Some(library_idx) =
        find_index(&tokens[idx..], |token| token.is_word("library")).map(|offset| idx + offset)
    else {
        return Ok(None);
    };
    let owner_tokens = trim_commas(&tokens[idx..library_idx]);
    if owner_tokens.is_empty() {
        return Ok(None);
    }
    let player = match parse_subject(&owner_tokens) {
        SubjectAst::Player(player) => player,
        _ => return Ok(None),
    };

    let mut tail_tokens = trim_commas(&tokens[library_idx + 1..]).to_vec();
    while tail_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        tail_tokens.remove(0);
    }
    let tail_words = crate::cards::builders::compiler::token_word_refs(&tail_tokens);
    let looks_like_exile_one_of_looked = slice_starts_with_any(
        &tail_words,
        &[
            &["exile", "one", "of", "them"],
            &["exile", "one", "of", "those"],
            &["exile", "one", "of", "those", "cards"],
        ],
    );
    if !looks_like_exile_one_of_looked {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(tokens, "looked");
    let chosen_tag = helper_tag_for_tokens(tokens, "chosen");
    let mut looked_filter = ObjectFilter::tagged(looked_tag.clone());
    looked_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::LookAtTopCards {
            player,
            count: Value::Fixed(count as i32),
            tag: looked_tag,
        },
        EffectAst::ChooseObjects {
            filter: looked_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag.clone(),
        },
        EffectAst::Exile {
            target: TargetAst::Tagged(chosen_tag, None),
            face_down: false,
        },
    ]))
}

pub(crate) fn parse_gain_life_equal_to_age_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Legacy fallback previously returned a hardcoded 0-life effect for age-counter clauses.
    // Let generic life parsing handle these so counter-scaled amounts compile correctly.
    let _ = tokens;
    Ok(None)
}

pub(crate) fn parse_you_and_each_opponent_voted_with_you_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    let pattern = [
        "you", "and", "each", "opponent", "who", "voted", "for", "a", "choice", "you", "voted",
        "for", "may", "scry",
    ];

    if words.len() < pattern.len() {
        return Ok(None);
    }

    if grammar::words_match_prefix(tokens, &pattern).is_none() {
        return Ok(None);
    }

    let scry_index = pattern.len() - 1;
    let value_tokens = &tokens[(scry_index + 1)..];
    let Some((count, _)) = parse_value(value_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "missing scry count in vote-with-you clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let you_effect = EffectAst::May {
        effects: vec![EffectAst::Scry {
            count: count.clone(),
            player: PlayerAst::You,
        }],
    };

    let opponent_effect = EffectAst::ForEachTaggedPlayer {
        tag: TagKey::from("voted_with_you"),
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::Scry {
                count,
                player: PlayerAst::Implicit,
            }],
        }],
    };

    Ok(Some(vec![you_effect, opponent_effect]))
}

