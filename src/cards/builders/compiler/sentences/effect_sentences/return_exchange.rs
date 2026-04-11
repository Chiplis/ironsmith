use super::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DelayedReturnTimingAst {
    NextEndStep(PlayerFilter),
    NextUpkeep(PlayerAst),
    EndOfCombat,
}

pub(crate) fn parse_delayed_return_timing_words(words: &[&str]) -> Option<DelayedReturnTimingAst> {
    if matches!(
        words,
        ["at", "end", "of", "combat"] | ["at", "the", "end", "of", "combat"]
    ) {
        return Some(DelayedReturnTimingAst::EndOfCombat);
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "end", "step"]
            | ["at", "beginning", "of", "the", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "the", "next", "end", "step"]
    ) {
        return Some(DelayedReturnTimingAst::NextEndStep(PlayerFilter::Any));
    }

    if matches!(
        words,
        ["at", "beginning", "of", "your", "next", "end", "step"]
            | [
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "end",
                "step"
            ]
    ) {
        return Some(DelayedReturnTimingAst::NextEndStep(PlayerFilter::You));
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "upkeep"]
            | ["at", "beginning", "of", "the", "next", "upkeep"]
            | ["at", "the", "beginning", "of", "next", "upkeep"]
            | ["at", "the", "beginning", "of", "the", "next", "upkeep"]
    ) {
        return Some(DelayedReturnTimingAst::NextUpkeep(PlayerAst::Any));
    }

    if matches!(
        words,
        ["at", "beginning", "of", "your", "next", "upkeep"]
            | ["at", "the", "beginning", "of", "your", "next", "upkeep"]
    ) {
        return Some(DelayedReturnTimingAst::NextUpkeep(PlayerAst::You));
    }

    None
}

pub(crate) fn wrap_return_with_delayed_timing(
    effect: EffectAst,
    timing: Option<DelayedReturnTimingAst>,
) -> EffectAst {
    let Some(timing) = timing else {
        return effect;
    };

    match timing {
        DelayedReturnTimingAst::NextEndStep(player) => EffectAst::DelayedUntilNextEndStep {
            player,
            effects: vec![effect],
        },
        DelayedReturnTimingAst::NextUpkeep(player) => EffectAst::DelayedUntilNextUpkeep {
            player,
            effects: vec![effect],
        },
        DelayedReturnTimingAst::EndOfCombat => EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        },
    }
}

pub(crate) fn parse_return(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let rewritten_storage;
    let tokens = if tokens.first().is_some_and(|token| token.is_word("to")) {
        let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
        let hand_or_battlefield_idx = find_index(&clause_words, |word| {
            matches!(*word, "hand" | "hands" | "battlefield")
        });
        if let Some(hand_or_battlefield_idx) = hand_or_battlefield_idx {
            let mut split_word_idx = hand_or_battlefield_idx + 1;

            if clause_words.get(split_word_idx).copied() == Some("under") {
                if let Some(control_rel_idx) =
                    find_index(&clause_words[split_word_idx + 1..], |word| {
                        *word == "control"
                    })
                {
                    split_word_idx = split_word_idx + 1 + control_rel_idx + 1;
                }
            }

            while clause_words
                .get(split_word_idx)
                .is_some_and(|word| *word == "tapped")
            {
                split_word_idx += 1;
            }

            if let Some(split_token_idx) = token_index_for_word_index(tokens, split_word_idx) {
                let target_tokens = trim_commas(&tokens[split_token_idx..]);
                let destination_tokens = trim_commas(&tokens[..split_token_idx]);
                if !target_tokens.is_empty() && !destination_tokens.is_empty() {
                    let mut rewritten = target_tokens.to_vec();
                    rewritten.extend(destination_tokens.to_vec());
                    rewritten_storage = rewritten;
                    &rewritten_storage
                } else {
                    tokens
                }
            } else {
                tokens
            }
        } else {
            tokens
        }
    } else {
        tokens
    };

    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::contains_word(tokens, "unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported return-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut to_idx = None;
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        if !tokens[idx].is_word("to") {
            continue;
        }
        let tail_tokens = &tokens[idx + 1..];
        if grammar::contains_word(tail_tokens, "hand")
            || grammar::contains_word(tail_tokens, "hands")
            || grammar::contains_word(tail_tokens, "battlefield")
            || grammar::contains_word(tail_tokens, "graveyard")
            || grammar::contains_word(tail_tokens, "graveyards")
        {
            to_idx = Some(idx);
            break;
        }
    }
    let to_idx = to_idx.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing return destination (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        ))
    })?;

    let mut target_tokens_vec = tokens[..to_idx].to_vec();
    let mut random = false;
    let mut random_idx = 0usize;
    while random_idx + 1 < target_tokens_vec.len() {
        if target_tokens_vec[random_idx].is_word("at")
            && target_tokens_vec[random_idx + 1].is_word("random")
        {
            random = true;
            target_tokens_vec.drain(random_idx..random_idx + 2);
            break;
        }
        random_idx += 1;
    }
    let target_tokens = target_tokens_vec.as_slice();
    let destination_tokens_full = &tokens[to_idx + 1..];
    let destination_words_full =
        crate::cards::builders::compiler::token_word_refs(destination_tokens_full);
    let mut delayed_timing = None;
    let mut destination_word_cutoff = destination_words_full.len();
    for word_idx in 0..destination_words_full.len() {
        if destination_words_full[word_idx] != "at" {
            continue;
        }
        if let Some(timing) = parse_delayed_return_timing_words(&destination_words_full[word_idx..])
        {
            delayed_timing = Some(timing);
            destination_word_cutoff = word_idx;
            break;
        }
    }

    let destination_tokens = if destination_word_cutoff < destination_words_full.len() {
        let token_cutoff =
            token_index_for_word_index(destination_tokens_full, destination_word_cutoff)
                .unwrap_or(destination_tokens_full.len());
        &destination_tokens_full[..token_cutoff]
    } else {
        destination_tokens_full
    };

    let mut destination_words =
        crate::cards::builders::compiler::token_word_refs(destination_tokens);
    let mut destination_excluded_subtypes: Vec<Subtype> = Vec::new();
    if let Some(except_idx) = find_word_sequence_start(&destination_words, &["except", "for"]) {
        let exception_words = &destination_words[except_idx + 2..];
        if exception_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing return exception qualifiers (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            )));
        }
        for word in exception_words {
            if matches!(*word, "and" | "or") {
                continue;
            }
            let Some(subtype) = parse_subtype_word(word)
                .or_else(|| str_strip_suffix(word, "s").and_then(parse_subtype_word))
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported return exception qualifier '{}' (clause: '{}')",
                    word,
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            };
            if !slice_contains(&destination_excluded_subtypes, &subtype) {
                destination_excluded_subtypes.push(subtype);
            }
        }
        if destination_excluded_subtypes.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing subtype return exception qualifiers (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            )));
        }
        destination_words.truncate(except_idx);
    }
    let is_hand =
        slice_contains(&destination_words, &"hand") || slice_contains(&destination_words, &"hands");
    let is_battlefield = slice_contains(&destination_words, &"battlefield");
    let is_graveyard = slice_contains(&destination_words, &"graveyard")
        || slice_contains(&destination_words, &"graveyards");
    let tapped = slice_contains(&destination_words, &"tapped");
    let transformed = grammar::contains_word(destination_tokens_full, "transformed");
    let converted = grammar::contains_word(destination_tokens_full, "converted");
    let return_controller =
        if contains_word_sequence(&destination_words, &["under", "your", "control"]) {
            ReturnControllerAst::You
        } else if destination_words
            .iter()
            .any(|word| matches!(*word, "owner" | "owners" | "owner's" | "owners'"))
            && slice_contains(&destination_words, &"control")
        {
            ReturnControllerAst::Owner
        } else {
            ReturnControllerAst::Preserve
        };
    let has_delayed_timing_words = grammar::contains_word(destination_tokens_full, "beginning")
        || grammar::contains_word(destination_tokens_full, "upkeep")
        || grammar::words_find_phrase(destination_tokens_full, &["end", "of", "combat"]).is_some()
        || grammar::contains_word(destination_tokens_full, "end")
            && (grammar::contains_word(destination_tokens_full, "next")
                || grammar::contains_word(destination_tokens_full, "step"));
    if delayed_timing.is_none() && has_delayed_timing_words {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed return timing clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }
    if !is_hand && !is_battlefield && !is_graveyard {
        return Err(CardTextError::ParseError(format!(
            "unsupported return destination (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let target_words = crate::cards::builders::compiler::token_word_refs(target_tokens);
    if let Some(and_idx) = find_index(target_tokens, |token| token.is_word("and"))
        && and_idx > 0
    {
        let tail_slice = &target_tokens[and_idx + 1..];
        let starts_multi_target = tail_slice.first().is_some_and(|t| t.is_word("target"))
            || (grammar::words_match_any_prefix(tail_slice, UP_TO_PREFIXES).is_some()
                && grammar::contains_word(tail_slice, "target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target return clause (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            )));
        }
    }
    if !grammar::contains_word(target_tokens, "target")
        && grammar::contains_word(target_tokens, "exiled")
        && grammar::contains_word(target_tokens, "cards")
    {
        let filter = parse_object_filter(target_tokens, false)?;
        let effect = if is_battlefield {
            EffectAst::ReturnAllToBattlefield { filter, tapped }
        } else if is_graveyard {
            EffectAst::MoveToZone {
                target: TargetAst::Object(filter, None, None),
                zone: Zone::Graveyard,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            }
        } else {
            EffectAst::ReturnAllToHand { filter }
        };
        return Ok(wrap_return_with_delayed_timing(effect, delayed_timing));
    }
    if target_words
        .first()
        .is_some_and(|word| *word == "all" || *word == "each")
    {
        let has_unsupported_return_all_qualifier = grammar::contains_word(target_tokens, "dealt")
            || grammar::contains_word(target_tokens, "without")
                && grammar::contains_word(target_tokens, "counter");
        if has_unsupported_return_all_qualifier {
            return Err(CardTextError::ParseError(format!(
                "unsupported qualified return-all filter (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            )));
        }
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing return-all filter".to_string(),
            ));
        }
        let return_filter_tokens = &target_tokens[1..];
        if is_hand
            && let Some((choice_idx, consumed)) = find_color_choice_phrase(return_filter_tokens)
        {
            let base_filter_tokens = trim_commas(&return_filter_tokens[..choice_idx]);
            let trailing = trim_commas(&return_filter_tokens[choice_idx + consumed..]);
            if !trailing.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing color-choice return-all clause (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            }
            if base_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter before color-choice clause (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            }
            let mut filter = parse_object_filter(&base_filter_tokens, false)?;
            for subtype in destination_excluded_subtypes {
                if !slice_contains(&filter.excluded_subtypes, &subtype) {
                    filter.excluded_subtypes.push(subtype);
                }
            }
            return Ok(wrap_return_with_delayed_timing(
                EffectAst::ReturnAllToHandOfChosenColor { filter },
                delayed_timing,
            ));
        }
        let return_filter_words =
            crate::cards::builders::compiler::token_word_refs(return_filter_tokens);
        let chosen_this_way_suffixes: [(&[&str], bool); 7] = [
            (&["not", "chosen", "this", "way"], true),
            (&["that", "weren't", "chosen", "this", "way"], true),
            (&["that", "werent", "chosen", "this", "way"], true),
            (&["that", "were", "not", "chosen", "this", "way"], true),
            (&["chosen", "this", "way"], false),
            (&["that", "were", "chosen", "this", "way"], false),
            (&["that", "was", "chosen", "this", "way"], false),
        ];
        let (return_filter_tokens, chosen_this_way_excluded) = if let Some((suffix, excluded)) =
            chosen_this_way_suffixes.iter().find(|(suffix, _)| {
                return_filter_words.len() >= suffix.len()
                    && &return_filter_words[return_filter_words.len() - suffix.len()..] == *suffix
            }) {
            let cutoff = return_filter_words.len() - suffix.len();
            let token_cutoff = if cutoff == 0 {
                0
            } else {
                token_index_for_word_index(return_filter_tokens, cutoff)
                    .unwrap_or(return_filter_tokens.len())
            };
            (
                trim_commas(&return_filter_tokens[..token_cutoff]).to_vec(),
                Some(*excluded),
            )
        } else {
            (return_filter_tokens.to_vec(), None)
        };
        let return_filter_words =
            crate::cards::builders::compiler::token_word_refs(&return_filter_tokens);
        let chosen_type_suffix_patterns: [(&[&str], bool, bool); 5] = [
            (
                &["that", "arent", "of", "the", "chosen", "type"],
                false,
                true,
            ),
            (
                &["that", "aren't", "of", "the", "chosen", "type"],
                false,
                true,
            ),
            (
                &["that", "are", "not", "of", "the", "chosen", "type"],
                false,
                true,
            ),
            (&["of", "the", "chosen", "type"], true, false),
            (&["that", "are", "of", "the", "chosen", "type"], true, false),
        ];
        let (base_filter_tokens, chosen_creature_type, excluded_chosen_creature_type) =
            if let Some((suffix, chosen_type, excluded_chosen_type)) =
                chosen_type_suffix_patterns.iter().find(|(suffix, _, _)| {
                    return_filter_words.len() >= suffix.len()
                        && &return_filter_words[return_filter_words.len() - suffix.len()..]
                            == *suffix
                })
            {
                let cutoff = return_filter_words.len() - suffix.len();
                let token_cutoff = token_index_for_word_index(&return_filter_tokens, cutoff)
                    .unwrap_or(return_filter_tokens.len());
                (
                    trim_commas(&return_filter_tokens[..token_cutoff]).to_vec(),
                    *chosen_type,
                    *excluded_chosen_type,
                )
            } else {
                (return_filter_tokens, false, false)
            };
        let mut filter = parse_object_filter(&base_filter_tokens, false)?;
        filter.chosen_creature_type |= chosen_creature_type;
        filter.excluded_chosen_creature_type |= excluded_chosen_creature_type;
        for subtype in destination_excluded_subtypes {
            if !slice_contains(&filter.excluded_subtypes, &subtype) {
                filter.excluded_subtypes.push(subtype);
            }
        }
        if let Some(excluded) = chosen_this_way_excluded {
            filter = if excluded {
                filter.not_tagged(TagKey::from(IT_TAG))
            } else {
                filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject)
            };
        }
        let effect = if is_battlefield {
            EffectAst::ReturnAllToBattlefield { filter, tapped }
        } else if is_graveyard {
            EffectAst::MoveToZone {
                target: TargetAst::Object(filter, None, None),
                zone: Zone::Graveyard,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            }
        } else {
            EffectAst::ReturnAllToHand { filter }
        };
        return Ok(wrap_return_with_delayed_timing(effect, delayed_timing));
    }
    if !destination_excluded_subtypes.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported return exception on non-return-all clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let target = if matches!(
        target_words.as_slice(),
        ["it"] | ["them"] | ["that", "card"] | ["those", "cards"]
    ) {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens))
    } else {
        parse_target_phrase(target_tokens)?
    };
    let effect = if is_battlefield {
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            transformed,
            converted,
            controller: return_controller,
        }
    } else if is_graveyard {
        EffectAst::MoveToZone {
            target,
            zone: Zone::Graveyard,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        }
    } else {
        EffectAst::ReturnToHand { target, random }
    };
    Ok(wrap_return_with_delayed_timing(effect, delayed_timing))
}

pub(crate) fn parse_exchange(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn split_shared_type_clause<'a>(
        clause_tokens: &'a [OwnedLexToken],
    ) -> Result<(&'a [OwnedLexToken], Option<SharedTypeConstraintAst>), CardTextError> {
        let tail_words = crate::cards::builders::compiler::token_word_refs(clause_tokens);
        let Some(rel_word_idx) = find_window_by(&tail_words, 2, |window| {
            window[0] == "that" && matches!(window[1], "share" | "shares")
        }) else {
            return Ok((clause_tokens, None));
        };

        let rel_token_idx =
            token_index_for_word_index(clause_tokens, rel_word_idx).unwrap_or(clause_tokens.len());
        let (head, tail) = clause_tokens.split_at(rel_token_idx);
        let share_words = crate::cards::builders::compiler::token_word_refs(tail);
        let share_head =
            if let Some((prefix, _)) = grammar::words_match_any_prefix(tail, SHARE_REL_PREFIXES) {
                &share_words[prefix.len()..]
            } else {
                &share_words[..]
            };
        let share_head = if share_head.first().copied() == Some("a") {
            &share_head[1..]
        } else {
            share_head
        };

        let shared_type = if slice_starts_with(&share_head, &["permanent", "type"])
            || slice_starts_with(&share_head, &["one", "of", "those", "permanent", "types"])
        {
            SharedTypeConstraintAst::PermanentType
        } else if slice_starts_with(&share_head, &["card", "type"])
            || slice_starts_with(&share_head, &["one", "of", "those", "types"])
        {
            SharedTypeConstraintAst::CardType
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported exchange share-type clause (clause: '{}')",
                tail_words.join(" ")
            )));
        };

        Ok((head, Some(shared_type)))
    }

    fn parse_value_operand(operand_tokens: &[OwnedLexToken]) -> Option<ExchangeValueAst> {
        match crate::cards::builders::compiler::token_word_refs(operand_tokens).as_slice() {
            ["your", "life", "total"] => return Some(ExchangeValueAst::LifeTotal(PlayerAst::You)),
            ["target", "player", "life", "total"]
            | ["target", "players", "life", "total"]
            | ["target", "player's", "life", "total"]
            | ["target", "players'", "life", "total"] => {
                return Some(ExchangeValueAst::LifeTotal(PlayerAst::Target));
            }
            ["target", "opponent", "life", "total"]
            | ["target", "opponents", "life", "total"]
            | ["target", "opponent's", "life", "total"]
            | ["target", "opponents'", "life", "total"] => {
                return Some(ExchangeValueAst::LifeTotal(PlayerAst::TargetOpponent));
            }
            ["an", "opponent", "life", "total"]
            | ["opponent", "life", "total"]
            | ["opponents", "life", "total"] => {
                return Some(ExchangeValueAst::LifeTotal(PlayerAst::Opponent));
            }
            ["its", "power"]
            | ["this", "power"]
            | ["thiss", "power"]
            | ["this's", "power"]
            | ["this", "creature", "power"]
            | ["this", "creature's", "power"]
            | ["thiss", "creature", "power"]
            | ["thiss", "creature's", "power"]
            | ["this", "creatures", "power"]
            | ["thiss", "creatures", "power"] => {
                return Some(ExchangeValueAst::Stat {
                    target: TargetAst::Source(span_from_tokens(operand_tokens)),
                    kind: ExchangeValueKindAst::Power,
                });
            }
            ["its", "toughness"]
            | ["this", "toughness"]
            | ["thiss", "toughness"]
            | ["this's", "toughness"]
            | ["this", "creature", "toughness"]
            | ["this", "creature's", "toughness"]
            | ["thiss", "creature", "toughness"]
            | ["thiss", "creature's", "toughness"]
            | ["this", "creatures", "toughness"]
            | ["thiss", "creatures", "toughness"] => {
                return Some(ExchangeValueAst::Stat {
                    target: TargetAst::Source(span_from_tokens(operand_tokens)),
                    kind: ExchangeValueKindAst::Toughness,
                });
            }
            _ => {}
        }

        let power_prefix = if let Some((prefix, _)) =
            grammar::words_match_any_prefix(operand_tokens, POWER_OF_PREFIXES)
        {
            Some((ExchangeValueKindAst::Power, prefix.len()))
        } else if let Some((prefix, _)) =
            grammar::words_match_any_prefix(operand_tokens, TOUGHNESS_OF_PREFIXES)
        {
            Some((ExchangeValueKindAst::Toughness, prefix.len()))
        } else {
            None
        }?;

        let (kind, used) = power_prefix;
        let target = parse_target_phrase(&operand_tokens[used..]).ok()?;
        Some(ExchangeValueAst::Stat { target, kind })
    }

    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::words_match_any_prefix(tokens, LIFE_TOTALS_PREFIXES).is_some() {
        if clause_words.as_slice() == ["life", "totals"] {
            return match subject {
                Some(SubjectAst::Player(PlayerAst::Target)) => Ok(EffectAst::ExchangeLifeTotals {
                    player1: PlayerAst::Target,
                    player2: PlayerAst::Target,
                }),
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported life-total exchange clause (clause: '{}')",
                    clause_words.join(" ")
                ))),
            };
        }

        if grammar::words_match_prefix(tokens, &["life", "totals", "with"]).is_none() {
            return Err(CardTextError::ParseError(format!(
                "unsupported exchange clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let player2 =
            match crate::cards::builders::compiler::token_word_refs(&tokens[3..]).as_slice() {
                ["you"] => Some(PlayerAst::You),
                ["target", "player"] | ["target", "players"] => Some(PlayerAst::Target),
                ["target", "opponent"] | ["target", "opponents"] => Some(PlayerAst::TargetOpponent),
                ["that", "player"] | ["that", "players"] => Some(PlayerAst::That),
                ["opponent"] | ["opponents"] | ["an", "opponent"] => Some(PlayerAst::Opponent),
                _ => None,
            }
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported life-total exchange partner (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let player1 = match subject {
            Some(SubjectAst::Player(player)) => player,
            _ => PlayerAst::You,
        };

        return Ok(EffectAst::ExchangeLifeTotals { player1, player2 });
    }
    if grammar::words_match_any_prefix(tokens, TEXT_BOXES_OF_PREFIXES).is_some() {
        let remainder = if let Some((_, rest)) =
            grammar::words_match_any_prefix(tokens, TEXT_BOXES_OF_PREFIXES)
        {
            rest
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported text-box exchange clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };

        let target = parse_target_phrase(remainder).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported text-box exchange target (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

        return Ok(EffectAst::ExchangeTextBoxes { target });
    }
    let zone_exchange = if slice_starts_with(&clause_words, &["your"]) {
        Some((PlayerAst::You, 1))
    } else if slice_starts_with(&clause_words, &["target", "player"])
        || slice_starts_with(&clause_words, &["target", "players"])
    {
        Some((PlayerAst::Target, 2))
    } else if slice_starts_with(&clause_words, &["target", "opponent"])
        || slice_starts_with(&clause_words, &["target", "opponents"])
    {
        Some((PlayerAst::TargetOpponent, 2))
    } else if slice_starts_with(&clause_words, &["an", "opponent"])
        || slice_starts_with(&clause_words, &["opponent"])
        || slice_starts_with(&clause_words, &["opponents"])
    {
        Some((
            PlayerAst::Opponent,
            if clause_words.first().copied() == Some("an") {
                2
            } else {
                1
            },
        ))
    } else {
        None
    };
    if let Some((player, consumed)) = zone_exchange
        && let Some(zone1) = clause_words
            .get(consumed)
            .and_then(|word| parse_zone_word(*word))
        && clause_words.get(consumed + 1).copied() == Some("and")
        && let Some(zone2) = clause_words
            .get(consumed + 2)
            .and_then(|word| parse_zone_word(*word))
        && consumed + 3 == clause_words.len()
    {
        return Ok(EffectAst::ExchangeZones {
            player,
            zone1,
            zone2,
        });
    }
    if grammar::words_match_prefix(tokens, &["control", "of"]).is_none() {
        if grammar::contains_word(tokens, "life")
            || grammar::contains_word(tokens, "power")
            || grammar::contains_word(tokens, "toughness")
        {
            let (duration, remainder) =
                if let Some((duration, remainder)) = parse_restriction_duration(tokens)? {
                    (duration, remainder)
                } else {
                    (Until::Forever, trim_commas(tokens).to_vec())
                };

            let split_idx = find_index(&remainder, |token: &OwnedLexToken| {
                token.is_word("with") || token.is_word("and")
            })
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported exchange clause (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                ))
            })?;
            let left_tokens = trim_commas(&remainder[..split_idx]);
            let right_tokens = trim_commas(&remainder[split_idx + 1..]);
            let left = parse_value_operand(&left_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported exchange value operand (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(&left_tokens).join(" ")
                ))
            })?;
            let right = parse_value_operand(&right_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported exchange value operand (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(&right_tokens).join(" ")
                ))
            })?;

            return Ok(EffectAst::ExchangeValues {
                left,
                right,
                duration,
            });
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported exchange clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some((before_and, after_and)) =
        crate::cards::builders::compiler::grammar::primitives::split_lexed_once_on_separator(
            tokens,
            || {
                use winnow::Parser as _;
                crate::cards::builders::compiler::grammar::primitives::kw("and").void()
            },
        )
    {
        let left_target = parse_target_phrase(&before_and[2..]).ok();
        let (right_tokens, shared_type) = split_shared_type_clause(after_and)?;
        let right_target = parse_target_phrase(right_tokens).ok();
        if let (Some(permanent1), Some(permanent2)) = (left_target, right_target) {
            return Ok(EffectAst::ExchangeControlHeterogeneous {
                permanent1,
                permanent2,
                shared_type,
            });
        }
    }

    let mut idx = 2usize;
    let mut count = 2u32;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("target")) {
        idx += 1;
    }
    if idx >= tokens.len() {
        return Err(CardTextError::ParseError(
            "missing exchange target filter".to_string(),
        ));
    }

    let (filter_tokens, shared_type) = split_shared_type_clause(&tokens[idx..])?;

    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(EffectAst::ExchangeControl {
        filter,
        count,
        shared_type,
    })
}
