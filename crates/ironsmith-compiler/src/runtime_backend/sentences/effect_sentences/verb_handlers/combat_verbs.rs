const ATTACH_TAGGED_OBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"]]);
const ATTACH_TAGGED_EQUIPMENT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that", "equipment"], &["those", "equipment"]]);
const ATTACH_TAGGED_AURA_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that", "aura"], &["those", "auras"]]);
const ATTACH_TAGGED_ARTIFACT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that", "artifact"], &["those", "artifacts"]]);
const ATTACH_TAGGED_ENCHANTMENT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["that", "enchantment"]);
const ATTACH_IT_TO_TOKEN_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it"]);
const ATTACH_TOKEN_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the", "token"]);
const DAMAGE_EACH_OPPONENT_HAND_SIZE_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix &["damage", "to", "each", "opponent", "equal", "to"];
    contains_words &["number", "cards", "hand"]
);
const DAMAGE_TO_EACH_OPPONENT_HAND_SIZE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["number", "cards", "hand"]);
const COMBAT_TARGET_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["target"]);
const COMBAT_IT_OR_THEM_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"]]);
const COMBAT_TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const COMBAT_THE_RESULT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the", "result"]);
const COMBAT_EACH_PLAYER_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["each", "player"], &["each", "players"]]);
const COMBAT_EACH_OPPONENT_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["each", "opponent"],
            &["each", "opponents"],
            &["each", "other", "player"],
            &["each", "other", "players"],
        ]
);
const COMBAT_EACH_OR_ALL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["each"], &["all"]]);
const COMBAT_DAMAGE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["damage"]);
const COMBAT_AMONG_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["among"]);
const COMBAT_TARGET_OR_TARGETS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["target"], &["targets"]]);
const COMBAT_PLAYER_OR_PLAYERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["player"], &["players"]]);
const COMBAT_NEGATION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["does"], &["doesnt"], &["doesn"], &["dont"], &["not"]]);
const COMBAT_INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const COMBAT_IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const COMBAT_INSTEAD_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const COMBAT_CREATURE_CONTROLLER_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["the", "creatures", "controller"],
            &["that", "creatures", "controller"],
            &["the", "creature's", "controller"],
            &["that", "creature's", "controller"],
        ]
);
const COMBAT_THE_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the", "player"]);
const COMBAT_MAX_SPEED_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["max", "speed"]]);
const COMBAT_DOES_NOT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["does", "not"]]]);
const COMBAT_END_OF_COMBAT_TIMING_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["at", "end", "of", "combat"],
            &["at", "the", "end", "of", "combat"],
        ]
);
const COMBAT_AS_YOU_CAST_THIS_SPELL_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["as", "you", "cast", "this", "spell"]);
const COMBAT_THIS_TURN_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "turn"]);
const COMBAT_WITH_DIFFERENT_POWER_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["with", "different", "powers"], &["with", "different", "power"]]);
const COMBAT_AT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["at"]);
const COMBAT_OTHER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["another"], &["other"]]);

fn combat_words_start_with_shape(words: &[&str], shape: &ClauseShape<'static>) -> bool {
    words.first().is_some_and(|word| shape.matches_word(word))
}

fn combat_find_exact_window(words: &[&str], width: usize, shape: ClauseShape<'static>) -> Option<usize> {
    find_window_by(words, width, |window| shape.matches_words(window))
}

fn is_divided_damage_clause(words: &[&str]) -> bool {
    let Some(divided_idx) = words.iter().position(|word| *word == "divided") else {
        return false;
    };
    words[divided_idx + 1..].iter().any(|word| *word == "among")
}

pub(crate) fn parse_attach_object_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let object_words = crate::runtime_backend::token_word_refs(tokens);
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

    if ATTACH_TAGGED_OBJECT_PATTERN.matches_words(&object_words) {
        return Ok(TargetAst::Tagged(TagKey::from(IT_TAG), object_span));
    }

    let mut tagged_filter = ObjectFilter::default();
    if ATTACH_TAGGED_EQUIPMENT_PATTERN.matches_words(&object_words) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Artifact);
        tagged_filter.subtypes.push(Subtype::Equipment);
    } else if ATTACH_TAGGED_AURA_PATTERN.matches_words(&object_words) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Enchantment);
        tagged_filter.subtypes.push(Subtype::Aura);
    } else if ATTACH_TAGGED_ARTIFACT_PATTERN.matches_words(&object_words) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Artifact);
    } else if ATTACH_TAGGED_ENCHANTMENT_PATTERN.matches_words(&object_words) {
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

    if crate::runtime_backend::lexer::token_slice_first_is(tokens, "target")
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
    if crate::runtime_backend::lexer::token_slice_first_is(tokens, "target") {
        return parse_target_phrase(tokens);
    }

    if object_words.len() >= 2
        && !COMBAT_TARGET_WORD_PATTERN.matches_words(&object_words)
        && object_words
            .iter()
            .all(|word| word.chars().all(|ch| ch.is_ascii_alphanumeric()))
    {
        return Ok(TargetAst::Source(object_span));
    }

    parse_target_phrase(tokens)
}

pub(crate) fn parse_attach(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "attach clause missing object and destination".to_string(),
        ));
    }

    if crate::runtime_backend::lexer::token_slice_first_is(tokens, "to") {
        let rest = trim_commas(&tokens[1..]);
        let Some(first) = rest.first() else {
            return Err(CardTextError::ParseError(format!(
                "attach clause missing object or destination (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        if COMBAT_IT_OR_THEM_WORD_PATTERN.matches_token(first) {
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
            return Ok(EffectAst::subject_verb_attach(object, target));
        }
    }

    let Some(to_idx) = rfind_index(tokens, |token| {
        COMBAT_TO_WORD_PATTERN.matches_token(token)
    }) else {
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

    let object_words = crate::runtime_backend::token_word_refs(&object_tokens);
    let object = parse_attach_object_phrase(&object_tokens)?;
    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if ATTACH_IT_TO_TOKEN_PATTERN.matches_words(&object_words)
        && ATTACH_TOKEN_TARGET_PATTERN.matches_words(&target_words)
    {
        return Ok(EffectAst::subject_verb_attach(
            TargetAst::Tagged(TagKey::from("triggering"), span_from_tokens(&object_tokens)),
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens)),
        ));
    }
    let target = if ATTACH_TAGGED_OBJECT_PATTERN.matches_words(&target_words) {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
    } else {
        parse_target_phrase(&target_tokens)?
    };

    Ok(EffectAst::subject_verb_attach(object, target))
}

pub(crate) fn parse_deal_damage(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let tokens =
        if let Some((_, rest)) = grammar::words_match_any_prefix(tokens, ADDITIONAL_PREFIXES) {
            rest
        } else {
            tokens
        };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if DAMAGE_EACH_OPPONENT_HAND_SIZE_PATTERN.matches_words(&clause_words) {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                Value::CardsInHand(PlayerFilter::IteratedPlayer),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        });
    }
    if is_divided_damage_clause(&clause_words) {
        if let Some((value, used)) = parse_value(tokens) {
            return parse_divided_damage_with_amount(tokens, value, used);
        }
        if let Some(effect) = parse_divided_damage_equal_to_amount(tokens)? {
            return Ok(effect);
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
        && DAMAGE_TO_EACH_OPPONENT_HAND_SIZE_TAIL_PATTERN.matches_words(&clause_words)
    {
        let value = Value::CardsInHand(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                value,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        });
    }

    Err(CardTextError::ParseError(format!(
        "missing damage amount (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_divided_damage_equal_to_amount(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !matches!(words.as_slice(), ["damage", "equal", "to", ..]) {
        return Ok(None);
    }
    let Some(divided_word_idx) = words.iter().position(|word| *word == "divided") else {
        return Ok(None);
    };
    let Some(divided_token_idx) = token_index_for_word_index(tokens, divided_word_idx) else {
        return Ok(None);
    };
    let amount_tokens = trim_commas(&tokens[3..divided_token_idx]);
    let Some((amount, used)) = parse_value(&amount_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage amount (clause: '{}')",
            words.join(" ")
        )));
    };
    if used != amount_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported divided-damage amount (clause: '{}')",
            words.join(" ")
        )));
    }
    let target = parse_divided_damage_target(&tokens[divided_token_idx..])?;
    Ok(Some(EffectAst::subject_verb_distributed_damage(
        amount, target,
    )))
}

pub(crate) fn parse_deal_damage_to_target_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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
        .is_some_and(|token| COMBAT_TO_WORD_PATTERN.matches_token(token))
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
        .or(parse_devotion_value_from_add_clause(tokens)?)
        .or(parse_equal_to_number_of_filter_value(tokens))
        .or_else(|| {
            let tail_words =
                crate::runtime_backend::token_word_refs(&tokens[equal_token_idx + 2..]);
            COMBAT_THE_RESULT_PATTERN.matches_words(&tail_words)
                .then_some(Value::EventValue(EventValueSpec::Amount))
        })
        .or(parse_dynamic_cost_modifier_value(tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if COMBAT_EACH_PLAYER_TARGET_PATTERN.matches_words(&target_words) {
        return Ok(Some(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        }));
    }
    if COMBAT_EACH_OPPONENT_TARGET_PATTERN.matches_words(&target_words) {
        return Ok(Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        }));
    }
    if combat_words_start_with_shape(&target_words, &COMBAT_EACH_OR_ALL_WORD_PATTERN) {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter = parse_object_filter(&target_tokens[1..], false)?;
        return Ok(Some(EffectAst::subject_verb_damage_each(amount, filter)));
    }
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::subject_verb_damage(amount, target)))
}

pub(crate) fn parse_deal_damage_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["damage", "equal", "to"]).is_none() {
        return Ok(None);
    }

    let mut target_to_idx = None;
    for idx in 3..tokens.len() {
        if !COMBAT_TO_WORD_PATTERN.matches_token(&tokens[idx]) {
            continue;
        }
        let tail_words = crate::runtime_backend::token_word_refs(&tokens[idx + 1..]);
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
            )
            || parse_target_phrase(&tokens[idx + 1..]).is_ok();
        if looks_like_target {
            target_to_idx = Some(idx);
        }
    }

    let Some(target_to_idx) = target_to_idx else {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let amount_tokens = if tokens
        .first()
        .is_some_and(|token| COMBAT_DAMAGE_WORD_PATTERN.matches_token(token))
    {
        &tokens[1..target_to_idx]
    } else {
        &tokens[..target_to_idx]
    };
    let amount = parse_add_mana_equal_amount_value(amount_tokens)
        .or(parse_equal_to_aggregate_filter_value(amount_tokens))
        .or(parse_devotion_value_from_add_clause(amount_tokens)?)
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
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
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
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        }));
    }
    if matches!(
        crate::runtime_backend::token_word_refs(normalized_target_tokens).first(),
        Some(&"each") | Some(&"all")
    ) {
        if normalized_target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter = parse_object_filter(&normalized_target_tokens[1..], false)?;
        return Ok(Some(EffectAst::subject_verb_damage_each(amount, filter)));
    }
    let target = parse_target_phrase(normalized_target_tokens)?;
    Ok(Some(EffectAst::subject_verb_damage(amount, target)))
}

fn parse_divided_damage_target(
    target_tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let Some(among_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        COMBAT_AMONG_WORD_PATTERN.matches_token(token)
    }) else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage targets after 'among' (clause: '{}')",
            crate::runtime_backend::token_word_refs(target_tokens).join(" ")
        )));
    };
    let among_tail = trim_commas(&target_tokens[among_idx + 1..]);
    let among_words = crate::runtime_backend::token_word_refs(&among_tail);
    let Some(target_idx) = find_index(&among_words, |word| {
        COMBAT_TARGET_OR_TARGETS_PATTERN.matches_word(word)
    }) else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage target phrase (clause: '{}')",
            crate::runtime_backend::token_word_refs(target_tokens).join(" ")
        )));
    };

    let count = if let Some((count, used)) = parse_choice_count_before_target_prefix(&among_tail) {
        if used != target_idx {
            return Err(CardTextError::ParseError(format!(
                "unsupported divided-damage target count (clause: '{}')",
                crate::runtime_backend::token_word_refs(target_tokens).join(" ")
            )));
        }
        count
    } else if let Some(max_targets) = among_words[..target_idx]
        .iter()
        .filter_map(|word| parse_number_word_u32(word))
        .max()
    {
        ChoiceCount {
            min: 1,
            max: Some(max_targets as usize),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        }
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage target count (clause: '{}')",
            crate::runtime_backend::token_word_refs(target_tokens).join(" ")
        )));
    };

    let target_phrase_tokens = &among_tail[target_idx..];
    let base_target = if COMBAT_TARGET_OR_TARGETS_PATTERN.matches_words(&among_words[target_idx..])
    {
        TargetAst::AnyTarget(span_from_tokens(target_phrase_tokens))
    } else {
        parse_target_phrase(target_phrase_tokens)?
    };
    Ok(TargetAst::WithCount(Box::new(base_target), count))
}

fn parse_divided_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let rest = &tokens[used..];
    if !crate::runtime_backend::lexer::token_slice_first_is(rest, "damage") {
        return Err(CardTextError::ParseError(format!(
            "missing damage keyword in divided-damage clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }
    let mut target_tokens = &rest[1..];
    if crate::runtime_backend::lexer::token_slice_first_is(target_tokens, "to") {
        target_tokens = &target_tokens[1..];
    }
    if grammar::contains_word(target_tokens, "evenly")
        && let Some(among_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
            COMBAT_AMONG_WORD_PATTERN.matches_token(token)
        })
    {
        let among_tail = trim_commas(&target_tokens[among_idx + 1..]);
        if matches!(
            among_tail.first().and_then(OwnedLexToken::as_word),
            Some("all" | "each" | "every")
        ) && among_tail.len() > 1
        {
            let filter = parse_object_filter(&among_tail[1..], false)?;
            return Ok(EffectAst::subject_verb_damage_each(amount, filter));
        }
    }
    let target = parse_divided_damage_target(target_tokens)?;
    Ok(EffectAst::subject_verb_distributed_damage(amount, target))
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
    if !COMBAT_DAMAGE_WORD_PATTERN.matches_word(word) {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    }

    let mut target_tokens = &rest[1..];
    if target_tokens
        .first()
        .is_some_and(|token| COMBAT_TO_WORD_PATTERN.matches_token(token))
    {
        target_tokens = &target_tokens[1..];
    }
    if let Some(among_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        COMBAT_AMONG_WORD_PATTERN.matches_token(token)
    }) {
        let among_tail = &target_tokens[among_idx + 1..];
        if crate::runtime_backend::lexer::contains_token_word(among_tail, "target")
            && crate::runtime_backend::lexer::contains_token_any_word(
                among_tail,
                &["player", "players", "creature", "creatures"],
            )
        {
            target_tokens = among_tail;
        }
    }

    if crate::runtime_backend::lexer::contains_token_word(target_tokens, "where") {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing where damage clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    if let Some(instead_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        COMBAT_INSTEAD_WORD_PATTERN.matches_token(token)
    }) && target_tokens
        .get(instead_idx + 1)
        .is_some_and(|token| COMBAT_IF_WORD_PATTERN.matches_token(token))
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
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
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
            if_true: vec![EffectAst::subject_verb_damage(amount.clone(), target)],
            if_false: Vec::new(),
        });
    }

    if let Some(spec) = split_trailing_if_clause_lexed(target_tokens) {
        let target = parse_target_phrase(spec.leading_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate: spec.predicate,
            if_true: vec![EffectAst::subject_verb_damage(amount, target)],
            if_false: Vec::new(),
        });
    }

    if target_tokens
        .first()
        .is_some_and(|token| COMBAT_IF_WORD_PATTERN.matches_token(token))
    {
        let predicate = parse_trailing_if_predicate_lexed(target_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported trailing if clause in damage effect (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))
        })?;
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::subject_verb_damage(
                amount,
                // Follow-up "deals N damage if ..." clauses can omit the target and rely
                // on parser-level merge with a prior damage sentence.
                TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None),
            )],
            if_false: Vec::new(),
        });
    }

    if find_index(&target_tokens, |token| COMBAT_IF_WORD_PATTERN.matches_token(token)).is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing if clause in damage effect (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let target_words = crate::runtime_backend::token_word_refs(target_tokens);
    if COMBAT_INSTEAD_TARGET_PATTERN.matches_words(&target_words) {
        return Ok(EffectAst::subject_verb_damage(
            amount,
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None),
        ));
    }
    if COMBAT_CREATURE_CONTROLLER_TARGET_PATTERN.matches_words(&target_words) {
        return Ok(EffectAst::subject_verb_damage(
            amount,
            TargetAst::Player(
                PlayerFilter::ControllerOf(crate::target::ObjectRef::tagged(IT_TAG)),
                span_from_tokens(target_tokens),
            ),
        ));
    }
    if COMBAT_THE_PLAYER_PATTERN.matches_words(&target_words) {
        return Ok(EffectAst::subject_verb_damage(
            amount,
            TargetAst::Player(
                PlayerFilter::IteratedPlayer,
                span_from_tokens(target_tokens),
            ),
        ));
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_OF_PREFIXES).is_some() {
        let each_of_tokens = &target_tokens[2..];
        if let Some((count, used)) = parse_choice_count_before_target_prefix(each_of_tokens)
            && each_of_tokens.len() == used + 1
        {
            let target = TargetAst::WithCount(
                Box::new(TargetAst::AnyTarget(span_from_tokens(each_of_tokens))),
                count,
            );
            return Ok(EffectAst::subject_verb_damage(amount, target));
        }
        if grammar::contains_word(each_of_tokens, "target") {
            let target = parse_target_phrase(each_of_tokens)?;
            return Ok(EffectAst::subject_verb_damage(amount, target));
        }
    }
    if COMBAT_EACH_PLAYER_TARGET_PATTERN.matches_words(&target_words) {
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        });
    }
    let normalized_target_words =
        crate::runtime_backend::lexer::parser_token_word_refs(target_tokens);
    let each_player_max_speed_filter =
        combat_words_start_with_shape(&normalized_target_words, &COMBAT_EACH_OR_ALL_WORD_PATTERN)
        && normalized_target_words
            .iter()
            .any(|word| COMBAT_PLAYER_OR_PLAYERS_WORD_PATTERN.matches_word(word))
        && COMBAT_MAX_SPEED_PATTERN.matches_words(&normalized_target_words);
    if each_player_max_speed_filter {
        let has_max_speed = !(normalized_target_words
            .iter()
            .any(|word| COMBAT_NEGATION_WORD_PATTERN.matches_word(word))
            || COMBAT_DOES_NOT_PATTERN.matches_words(&normalized_target_words));
        let filter = if has_max_speed {
            PlayerFilter::with_max_speed(PlayerFilter::Any)
        } else {
            PlayerFilter::without_max_speed(PlayerFilter::Any)
        };
        return Ok(EffectAst::ForEachPlayersFiltered {
            filter,
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        });
    }
    if COMBAT_EACH_OPPONENT_TARGET_PATTERN.matches_words(&target_words) {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        });
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_OPPONENT_WHO_PREFIXES).is_some()
        && grammar::words_find_phrase(target_tokens, &["this", "way"]).is_some()
    {
        let predicate = parse_who_did_this_way_predicate(&target_tokens[2..])?;
        return Ok(EffectAst::ForEachOpponentDid {
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
            predicate,
        });
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_PLAYER_WHO_PREFIXES).is_some()
        && grammar::words_find_phrase(target_tokens, &["this", "way"]).is_some()
    {
        let predicate = parse_who_did_this_way_predicate(&target_tokens[2..])?;
        return Ok(EffectAst::ForEachPlayerDid {
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
            predicate,
        });
    }

    if combat_words_start_with_shape(&target_words, &COMBAT_EACH_OR_ALL_WORD_PATTERN)
        && let Some(and_each_idx) = find_window_by(&target_words, 3, |window| {
            ClauseShape::new()
                .exact_any(&[&["and", "each", "player"], &["and", "each", "players"]])
                .matches_words(window)
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
                EffectAst::subject_verb_damage(
                    amount.clone(),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                ),
                EffectAst::subject_verb_damage_each(amount.clone(), filter),
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
                EffectAst::subject_verb_damage(
                    amount.clone(),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                ),
                EffectAst::subject_verb_damage_each(amount.clone(), filter),
            ],
        });
    }

    if combat_words_start_with_shape(&target_words, &COMBAT_EACH_OR_ALL_WORD_PATTERN) {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter_tokens = &target_tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::subject_verb_damage_each(amount.clone(), filter));
    }

    if let Some(at_idx) = find_index(&target_tokens, |token| {
        COMBAT_AT_WORD_PATTERN.matches_token(token)
    }) {
        let timing_words = crate::runtime_backend::token_word_refs(&target_tokens[at_idx..]);
        let matches_end_of_combat = COMBAT_END_OF_COMBAT_TIMING_PATTERN.matches_words(&timing_words);
        if matches_end_of_combat && at_idx >= 1 {
            let pre_target_tokens = trim_commas(&target_tokens[..at_idx]);
            if !pre_target_tokens.is_empty() {
                let target = parse_target_phrase(&pre_target_tokens)?;
                return Ok(EffectAst::DelayedUntilEndOfCombat {
                    effects: vec![EffectAst::subject_verb_damage(amount, target)],
                });
            }
        }
    }

    let target = parse_target_phrase(&target_tokens)?;
    Ok(EffectAst::subject_verb_damage(amount, target))
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
    if let Ok((comparison, used)) =
        parse_quantity_comparison_prefix(filter_tokens, false, false, "control predicate")
    {
        if let Some(count) = comparison_to_strict_at_least_threshold(&comparison) {
            min_count = Some(count);
            filter_tokens = &filter_tokens[used..];
        } else if matches!(
            comparison,
            crate::effect::Comparison::LessThan(_)
                | crate::effect::Comparison::LessThanOrEqual(_)
        ) {
            // Keep unsupported upper-bound variants as plain control checks for now.
            filter_tokens = &filter_tokens[used..];
        }
    }
    for (width, marker) in [
        (5usize, COMBAT_AS_YOU_CAST_THIS_SPELL_MARKER_PATTERN),
        (2usize, COMBAT_THIS_TURN_MARKER_PATTERN),
    ] {
        let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
        if let Some(idx) = combat_find_exact_window(&filter_words, width, marker) {
            let cut_idx =
                token_index_for_word_index(filter_tokens, idx).unwrap_or(filter_tokens.len());
            filter_tokens = &filter_tokens[..cut_idx];
            break;
        }
    }
    let mut filter_tokens = trim_commas(filter_tokens);
    let filter_words = crate::runtime_backend::token_word_refs(&filter_tokens);
    let mut requires_different_powers = false;
    if COMBAT_WITH_DIFFERENT_POWER_SUFFIX_PATTERN.matches_words(&filter_words) {
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
        .is_some_and(|token| COMBAT_OTHER_WORD_PATTERN.matches_token(token));
    let filter = parse_object_filter(&filter_tokens, other)?;
    if let Some(count) = min_count {
        if requires_different_powers {
            return Ok(Some(
                PredicateAst::PlayerHasAtLeastWithDifferentPowers {
                    player: PlayerAst::You,
                    filter,
                    count,
                },
            ));
        }
        Ok(Some(PredicateAst::PlayerHasAtLeast {
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
