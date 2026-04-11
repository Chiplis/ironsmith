pub(crate) fn parse_attach_object_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let object_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let object_span = span_from_tokens(tokens);
    if object_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object to attach".to_string(),
        ));
    }

    let is_source_attachment = is_source_reference_words(&object_words)
        || grammar::words_match_any_prefix(tokens, SOURCE_ATTACHMENT_PREFIXES).is_some();
    if is_source_attachment {
        return Ok(TargetAst::Source(object_span));
    }

    if matches!(object_words.as_slice(), ["it"] | ["them"]) {
        return Ok(TargetAst::Tagged(TagKey::from(IT_TAG), object_span));
    }

    let mut tagged_filter = ObjectFilter::default();
    if matches!(
        object_words.as_slice(),
        ["that", "equipment"] | ["those", "equipment"]
    ) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Artifact);
        tagged_filter.subtypes.push(Subtype::Equipment);
    } else if matches!(
        object_words.as_slice(),
        ["that", "aura"] | ["those", "auras"]
    ) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Enchantment);
        tagged_filter.subtypes.push(Subtype::Aura);
    } else if matches!(
        object_words.as_slice(),
        ["that", "artifact"] | ["those", "artifacts"]
    ) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Artifact);
    } else if object_words.as_slice() == ["that", "enchantment"] {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Enchantment);
    }

    if tagged_filter.zone.is_some() {
        tagged_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(TargetAst::Object(tagged_filter, object_span, None));
    }

    if tokens.first().is_some_and(|token| token.is_word("target"))
        && let Some((head_slice, _after_attached_to)) =
            super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
                use winnow::Parser as _;
                super::super::grammar::primitives::phrase(&["attached", "to"]).void()
            })
    {
        let head_tokens = trim_commas(head_slice);
        if !head_tokens.is_empty() {
            return parse_target_phrase(&head_tokens);
        }
    }

    parse_target_phrase(tokens)
}

pub(crate) fn parse_attach(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "attach clause missing object and destination".to_string(),
        ));
    }

    if tokens.first().is_some_and(|token| token.is_word("to")) {
        let rest = trim_commas(&tokens[1..]);
        let Some(first) = rest.first() else {
            return Err(CardTextError::ParseError(format!(
                "attach clause missing object or destination (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        if first.is_word("it") || first.is_word("them") {
            let target_tokens = vec![first.clone()];
            let object_tokens = trim_commas(&rest[1..]);
            if object_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "attach clause missing object or destination (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens));
            let object = parse_attach_object_phrase(&object_tokens)?;
            return Ok(EffectAst::Attach { object, target });
        }
    }

    let Some(to_idx) = rfind_index(tokens, |token| token.is_word("to")) else {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing destination (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if to_idx == 0 || to_idx + 1 >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing object or destination (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let object_tokens = trim_commas(&tokens[..to_idx]);
    let target_tokens = trim_commas(&tokens[to_idx + 1..]);
    if object_tokens.is_empty() || target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing object or destination (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let object = parse_attach_object_phrase(&object_tokens)?;
    let target_words = crate::cards::builders::compiler::token_word_refs(&target_tokens);
    let target = if matches!(target_words.as_slice(), ["it"] | ["them"]) {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
    } else {
        parse_target_phrase(&target_tokens)?
    };

    Ok(EffectAst::Attach { object, target })
}

pub(crate) fn parse_deal_damage(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let tokens =
        if let Some((_, rest)) = grammar::words_match_any_prefix(tokens, ADDITIONAL_PREFIXES) {
            rest
        } else {
            tokens
        };
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["damage", "to", "each", "opponent", "equal", "to"])
        .is_some()
        && grammar::contains_word(tokens, "number")
        && grammar::contains_word(tokens, "cards")
        && grammar::contains_word(tokens, "hand")
    {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: Value::CardsInHand(PlayerFilter::IteratedPlayer),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }
    let is_divided_as_you_choose_clause = grammar::contains_word(tokens, "divided")
        && grammar::contains_word(tokens, "choose")
        && grammar::contains_word(tokens, "among");
    if is_divided_as_you_choose_clause {
        if let Some((value, used)) = parse_value(tokens) {
            return parse_divided_damage_with_amount(tokens, value, used);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported divided-damage distribution clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_deal_damage_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_deal_damage_to_target_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, EVENT_AMOUNT_PREFIXES) {
        return parse_deal_damage_with_amount(
            tokens,
            Value::EventValue(EventValueSpec::Amount),
            prefix.len(),
        );
    }

    if let Some((value, used)) = parse_value(tokens) {
        return parse_deal_damage_with_amount(tokens, value, used);
    }

    if grammar::words_match_any_prefix(tokens, DAMAGE_TO_EACH_OPPONENT_PREFIXES).is_some()
        && grammar::contains_word(tokens, "number")
        && grammar::contains_word(tokens, "cards")
        && grammar::contains_word(tokens, "hand")
    {
        let value = Value::CardsInHand(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: value,
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }

    Err(CardTextError::ParseError(format!(
        "missing damage amount (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_deal_damage_to_target_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["damage", "to"]).is_none() {
        return Ok(None);
    }

    let Some(equal_word_idx) = grammar::words_find_phrase(tokens, &["equal", "to"]) else {
        return Ok(None);
    };
    let Some(equal_token_idx) = token_index_for_word_index(tokens, equal_word_idx) else {
        return Ok(None);
    };

    let mut target_tokens = trim_commas(&tokens[1..equal_token_idx]);
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens.remove(0);
    }
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let amount = parse_add_mana_equal_amount_value(tokens)
        .or(parse_equal_to_aggregate_filter_value(tokens))
        .or(parse_equal_to_number_of_filter_value(tokens))
        .or(parse_dynamic_cost_modifier_value(tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_words = crate::cards::builders::compiler::token_word_refs(&target_tokens);
    if target_words.as_slice() == ["each", "player"]
        || target_words.as_slice() == ["each", "players"]
    {
        return Ok(Some(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    if target_words.as_slice() == ["each", "opponent"]
        || target_words.as_slice() == ["each", "opponents"]
        || target_words.as_slice() == ["each", "other", "player"]
        || target_words.as_slice() == ["each", "other", "players"]
    {
        return Ok(Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::DealDamage { amount, target }))
}

pub(crate) fn parse_deal_damage_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["damage", "equal", "to"]).is_none() {
        return Ok(None);
    }

    let mut target_to_idx = None;
    for idx in 3..tokens.len() {
        if !tokens[idx].is_word("to") {
            continue;
        }
        let tail_words = crate::cards::builders::compiler::token_word_refs(&tokens[idx + 1..]);
        if tail_words.is_empty() {
            continue;
        }
        let looks_like_target = grammar::contains_word(&tokens[idx + 1..], "target")
            || matches!(
                tail_words.first().copied(),
                Some(
                    "any"
                        | "each"
                        | "all"
                        | "it"
                        | "itself"
                        | "them"
                        | "him"
                        | "her"
                        | "that"
                        | "this"
                        | "you"
                        | "player"
                        | "opponent"
                        | "creature"
                        | "planeswalker"
                )
            );
        if looks_like_target {
            target_to_idx = Some(idx);
            break;
        }
    }

    let Some(target_to_idx) = target_to_idx else {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let amount_tokens = &tokens[..target_to_idx];
    let amount = parse_add_mana_equal_amount_value(amount_tokens)
        .or(parse_equal_to_aggregate_filter_value(amount_tokens))
        .or(parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
            amount_tokens,
        ))
        .or(parse_equal_to_number_of_filter_value(amount_tokens))
        .or(parse_equal_to_number_of_opponents_you_have_value(
            amount_tokens,
        ))
        .or(parse_equal_to_number_of_counters_on_reference_value(
            amount_tokens,
        ))
        .or(parse_dynamic_cost_modifier_value(amount_tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let target_tokens = &tokens[target_to_idx + 1..];
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut normalized_target_tokens = target_tokens;
    if grammar::words_match_any_prefix(target_tokens, EACH_OF_PREFIXES).is_some() {
        let each_of_tokens = &target_tokens[2..];
        if grammar::contains_word(each_of_tokens, "target") {
            normalized_target_tokens = each_of_tokens;
        }
    }
    if grammar::words_match_any_prefix(
        normalized_target_tokens,
        &[&["each", "player"], &["each", "players"]],
    )
    .is_some()
    {
        return Ok(Some(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    if grammar::words_match_any_prefix(
        normalized_target_tokens,
        &[
            &["each", "opponent"],
            &["each", "opponents"],
            &["each", "other", "player"],
            &["each", "other", "players"],
        ],
    )
    .is_some()
    {
        return Ok(Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    let target = parse_target_phrase(normalized_target_tokens)?;
    Ok(Some(EffectAst::DealDamage { amount, target }))
}

fn parse_divided_damage_target(
    target_tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let Some(among_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        token.is_word("among")
    }) else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage targets after 'among' (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(target_tokens).join(" ")
        )));
    };
    let among_tail = trim_commas(&target_tokens[among_idx + 1..]);
    let among_words = crate::cards::builders::compiler::token_word_refs(&among_tail);
    let Some(target_idx) = find_index(&among_words, |word| matches!(*word, "target" | "targets"))
    else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage target phrase (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(target_tokens).join(" ")
        )));
    };

    let max_targets = among_words[..target_idx]
        .iter()
        .filter_map(|word| parse_number_word_u32(word))
        .max()
        .unwrap_or(0);
    if max_targets == 0
        && grammar::words_match_any_prefix(&among_tail, ANY_NUMBER_OF_PREFIXES).is_none()
    {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage target count (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(target_tokens).join(" ")
        )));
    }

    let target_phrase_tokens = &among_tail[target_idx..];
    let base_target =
        if among_words[target_idx..] == ["target"] || among_words[target_idx..] == ["targets"] {
            TargetAst::AnyTarget(span_from_tokens(target_phrase_tokens))
        } else {
            parse_target_phrase(target_phrase_tokens)?
        };
    let count = if grammar::words_match_any_prefix(&among_tail, ANY_NUMBER_OF_PREFIXES).is_some() {
        ChoiceCount::any_number()
    } else {
        ChoiceCount::up_to(max_targets as usize)
    };
    Ok(TargetAst::WithCount(Box::new(base_target), count))
}

fn parse_divided_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let rest = &tokens[used..];
    if !rest.first().is_some_and(|token| token.is_word("damage")) {
        return Err(CardTextError::ParseError(format!(
            "missing damage keyword in divided-damage clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }
    let mut target_tokens = &rest[1..];
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens = &target_tokens[1..];
    }
    let target = parse_divided_damage_target(target_tokens)?;
    Ok(EffectAst::DealDistributedDamage { amount, target })
}

pub(crate) fn parse_deal_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let rest = &tokens[used..];
    let Some(word) = rest.first().and_then(OwnedLexToken::as_word) else {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    };
    if word != "damage" {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    }

    let mut target_tokens = &rest[1..];
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens = &target_tokens[1..];
    }
    if let Some(among_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        token.is_word("among")
    }) {
        let among_tail = &target_tokens[among_idx + 1..];
        if among_tail.iter().any(|token| token.is_word("target"))
            && among_tail.iter().any(|token| {
                token.is_word("player")
                    || token.is_word("players")
                    || token.is_word("creature")
                    || token.is_word("creatures")
            })
        {
            target_tokens = among_tail;
        }
    }

    if target_tokens.iter().any(|token| token.is_word("where")) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing where damage clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    if let Some(instead_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        token.is_word("instead")
    }) && target_tokens
        .get(instead_idx + 1)
        .is_some_and(|token| token.is_word("if"))
    {
        let pre_target_tokens = trim_commas(&target_tokens[..instead_idx]);
        let predicate = if let Some(predicate) =
            parse_instead_if_control_predicate(&trim_commas(&target_tokens[instead_idx + 2..]))?
        {
            predicate
        } else {
            parse_trailing_instead_if_predicate_lexed(&target_tokens[instead_idx..]).ok_or_else(
                || {
                    CardTextError::ParseError(format!(
                        "unsupported trailing instead-if clause in damage effect (clause: '{}')",
                        crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                    ))
                },
            )?
        };
        let target = if pre_target_tokens.is_empty() {
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
        } else {
            parse_target_phrase(&pre_target_tokens)?
        };
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target,
            }],
            if_false: Vec::new(),
        });
    }

    if let Some(spec) = split_trailing_if_clause_lexed(target_tokens) {
        let target = parse_target_phrase(spec.leading_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate: spec.predicate,
            if_true: vec![EffectAst::DealDamage { amount, target }],
            if_false: Vec::new(),
        });
    }

    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
    {
        let predicate = parse_trailing_if_predicate_lexed(target_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported trailing if clause in damage effect (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            ))
        })?;
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::DealDamage {
                amount,
                // Follow-up "deals N damage if ..." clauses can omit the target and rely
                // on parser-level merge with a prior damage sentence.
                target: TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None),
            }],
            if_false: Vec::new(),
        });
    }

    if find_index(&target_tokens, |token| token.is_word("if")).is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing if clause in damage effect (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let target_words = crate::cards::builders::compiler::token_word_refs(target_tokens);
    if target_words.as_slice() == ["instead"] {
        return Ok(EffectAst::DealDamage {
            amount,
            target: TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None),
        });
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_OF_PREFIXES).is_some() {
        let each_of_tokens = &target_tokens[2..];
        let each_of_words = crate::cards::builders::compiler::token_word_refs(each_of_tokens);
        if matches!(
            each_of_words.as_slice(),
            ["up", "to", _, "target"] | ["up", "to", _, "targets"]
        ) && let Some(count) = parse_number_word_u32(each_of_words[2])
        {
            let target = TargetAst::WithCount(
                Box::new(TargetAst::AnyTarget(span_from_tokens(each_of_tokens))),
                ChoiceCount::up_to(count as usize),
            );
            return Ok(EffectAst::DealDamage { amount, target });
        }
        if grammar::contains_word(each_of_tokens, "target") {
            let target = parse_target_phrase(each_of_tokens)?;
            return Ok(EffectAst::DealDamage { amount, target });
        }
    }
    if target_words.as_slice() == ["each", "player"]
        || target_words.as_slice() == ["each", "players"]
    {
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }
    if target_words.as_slice() == ["each", "opponent"]
        || target_words.as_slice() == ["each", "opponents"]
    {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_OPPONENT_WHO_PREFIXES).is_some()
        && grammar::words_find_phrase(target_tokens, &["this", "way"]).is_some()
    {
        let predicate = parse_who_did_this_way_predicate(&target_tokens[2..])?;
        return Ok(EffectAst::ForEachOpponentDid {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
            predicate,
        });
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_PLAYER_WHO_PREFIXES).is_some()
        && grammar::words_find_phrase(target_tokens, &["this", "way"]).is_some()
    {
        let predicate = parse_who_did_this_way_predicate(&target_tokens[2..])?;
        return Ok(EffectAst::ForEachPlayerDid {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
            predicate,
        });
    }

    if matches!(target_words.first(), Some(&"each") | Some(&"all"))
        && let Some(and_each_idx) = find_window_by(&target_words, 3, |window| {
            window == ["and", "each", "player"] || window == ["and", "each", "players"]
        })
        && and_each_idx >= 1
        && and_each_idx + 3 == target_words.len()
    {
        let filter_tokens = &target_tokens[1..and_each_idx];
        let mut filter = parse_object_filter(filter_tokens, false)?;
        if filter.controller.is_none() {
            filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![
                EffectAst::DealDamage {
                    amount: amount.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                },
                EffectAst::DealDamageEach {
                    amount: amount.clone(),
                    filter,
                },
            ],
        });
    }

    if grammar::words_match_any_prefix(target_tokens, EACH_OPPONENT_AND_EACH_PREFIXES).is_some()
        && grammar::contains_word(target_tokens, "creature")
        && grammar::contains_word(target_tokens, "planeswalker")
        && (grammar::words_find_phrase(target_tokens, &["they", "control"]).is_some()
            || grammar::words_find_phrase(target_tokens, &["that", "player", "controls"]).is_some())
    {
        let mut filter = ObjectFilter::default();
        filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::DealDamage {
                    amount: amount.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                },
                EffectAst::DealDamageEach {
                    amount: amount.clone(),
                    filter,
                },
            ],
        });
    }

    if matches!(target_words.first(), Some(&"each") | Some(&"all")) {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter_tokens = &target_tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::DealDamageEach {
            amount: amount.clone(),
            filter,
        });
    }

    if let Some(at_idx) = find_index(&target_tokens, |token| token.is_word("at")) {
        let timing_words =
            crate::cards::builders::compiler::token_word_refs(&target_tokens[at_idx..]);
        let matches_end_of_combat = timing_words.as_slice() == ["at", "end", "of", "combat"]
            || timing_words.as_slice() == ["at", "the", "end", "of", "combat"];
        if matches_end_of_combat && at_idx >= 1 {
            let pre_target_tokens = trim_commas(&target_tokens[..at_idx]);
            if !pre_target_tokens.is_empty() {
                let target = parse_target_phrase(&pre_target_tokens)?;
                return Ok(EffectAst::DelayedUntilEndOfCombat {
                    effects: vec![EffectAst::DealDamage { amount, target }],
                });
            }
        }
    }

    let target = parse_target_phrase(&target_tokens)?;
    Ok(EffectAst::DealDamage { amount, target })
}

pub(crate) fn parse_instead_if_control_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let starts_with_you_control =
        grammar::words_match_any_prefix(tokens, YOU_CONTROL_PREFIXES).is_some();
    if !starts_with_you_control {
        return Ok(None);
    }

    let mut filter_tokens = &tokens[2..];
    let mut min_count: Option<u32> = None;
    if let Some((count, used)) = parse_number(filter_tokens)
        && count > 1
    {
        let tail = &filter_tokens[used..];
        if tail.first().is_some_and(|token| token.is_word("or"))
            && tail.get(1).is_some_and(|token| token.is_word("more"))
        {
            min_count = Some(count);
            filter_tokens = &tail[2..];
        } else if tail.first().is_some_and(|token| token.is_word("or"))
            && tail.get(1).is_some_and(|token| token.is_word("fewer"))
        {
            // Keep unsupported "or fewer" variants as plain control checks for now.
            filter_tokens = &tail[2..];
        }
    }
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("at"))
        && filter_tokens
            .get(1)
            .is_some_and(|token| token.is_word("least"))
        && let Some((count, used)) = parse_number(&filter_tokens[2..])
        && count > 1
    {
        min_count = Some(count);
        filter_tokens = &filter_tokens[2 + used..];
    }
    let cut_markers: &[&[&str]] = &[&["as", "you", "cast", "this", "spell"], &["this", "turn"]];
    for marker in cut_markers {
        let filter_words = crate::cards::builders::compiler::token_word_refs(filter_tokens);
        if let Some(idx) = find_window_by(&filter_words, marker.len(), |window| window == *marker) {
            let cut_idx =
                token_index_for_word_index(filter_tokens, idx).unwrap_or(filter_tokens.len());
            filter_tokens = &filter_tokens[..cut_idx];
            break;
        }
    }
    let mut filter_tokens = trim_commas(filter_tokens);
    let filter_words = crate::cards::builders::compiler::token_word_refs(&filter_tokens);
    let mut requires_different_powers = false;
    if grammar::words_match_suffix(&filter_tokens, &["with", "different", "powers"]).is_some()
        || grammar::words_match_suffix(&filter_tokens, &["with", "different", "power"]).is_some()
    {
        requires_different_powers = true;
        let cut_word_idx = filter_words.len().saturating_sub(3);
        let cut_token_idx =
            token_index_for_word_index(&filter_tokens, cut_word_idx).unwrap_or(filter_tokens.len());
        filter_tokens = trim_commas(&filter_tokens[..cut_token_idx]);
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let other = filter_tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"));
    let filter = parse_object_filter(&filter_tokens, other)?;
    if let Some(count) = min_count {
        if requires_different_powers {
            return Ok(Some(
                PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                    player: PlayerAst::You,
                    filter,
                    count,
                },
            ));
        }
        Ok(Some(PredicateAst::PlayerControlsAtLeast {
            player: PlayerAst::You,
            filter,
            count,
        }))
    } else {
        Ok(Some(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        }))
    }
}

