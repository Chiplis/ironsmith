use super::*;

pub(crate) type SentencePrimitiveParser =
    fn(&[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError>;

pub(crate) struct SentencePrimitive {
    pub(crate) name: &'static str,
    pub(crate) parser: SentencePrimitiveParser,
}

pub(crate) fn run_sentence_primitives(
    tokens: &[Token],
    primitives: &[SentencePrimitive],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    for primitive in primitives {
        match (primitive.parser)(tokens) {
            Ok(Some(effects)) => {
                let stage = format!("parse_effect_sentence:primitive-hit:{}", primitive.name);
                parser_trace(&stage, tokens);
                if effects.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "primitive '{}' produced empty effects (clause: '{}')",
                        primitive.name,
                        words(tokens).join(" ")
                    )));
                }
                return Ok(Some(effects));
            }
            Ok(None) => {}
            Err(err) => {
                if parser_trace_enabled() {
                    eprintln!(
                        "[parser-flow] stage=parse_effect_sentence:primitive-error primitive={} clause='{}' error={err:?}",
                        primitive.name,
                        words(tokens).join(" ")
                    );
                }
                return Err(err);
            }
        }
    }
    Ok(None)
}

pub(crate) fn parse_you_and_target_player_each_draw_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 6 {
        return Ok(None);
    }
    if !clause_words.starts_with(&["you", "and", "target"]) {
        return Ok(None);
    }

    let target_player = match clause_words.get(3).copied() {
        Some("opponent" | "opponents") => PlayerAst::TargetOpponent,
        Some("player" | "players") => PlayerAst::Target,
        _ => return Ok(None),
    };

    let mut idx = 4usize;

    if clause_words.get(idx) == Some(&"each") {
        idx += 1;
    }
    if !matches!(clause_words.get(idx).copied(), Some("draw" | "draws")) {
        return Ok(None);
    }
    idx += 1;

    let remainder_words = &clause_words[idx..];
    if remainder_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing draw count in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let synthetic_tokens = remainder_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_value(&synthetic_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing draw count in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if synthetic_tokens
        .get(used)
        .and_then(Token::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let trailing_words = words(&synthetic_tokens[used + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing shared draw clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::Draw {
            count: count.clone(),
            player: PlayerAst::You,
        },
        EffectAst::Draw {
            count,
            player: target_player,
        },
    ]))
}

pub(crate) fn parse_sentence_you_and_target_player_each_draw(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_target_player_each_draw_sentence(tokens)
}

pub(crate) fn parse_sentence_you_and_attacking_player_each_draw_and_lose(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 11 || !clause_words.starts_with(&["you", "and"]) {
        return Ok(None);
    }

    let mut idx = 2usize;
    if clause_words.get(idx) == Some(&"the") {
        idx += 1;
    }
    if clause_words.get(idx) != Some(&"attacking") || clause_words.get(idx + 1) != Some(&"player") {
        return Ok(None);
    }
    idx += 2;

    if clause_words.get(idx) == Some(&"each") {
        idx += 1;
    }
    if !matches!(clause_words.get(idx).copied(), Some("draw" | "draws")) {
        return Ok(None);
    }
    idx += 1;

    let draw_tokens = clause_words[idx..]
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (draw_count, draw_used) = parse_value(&draw_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing shared draw count (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if draw_tokens
        .get(draw_used)
        .and_then(Token::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in shared draw/lose sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let after_draw_words = words(&draw_tokens[draw_used + 1..]);
    if after_draw_words.first() != Some(&"and")
        || !matches!(after_draw_words.get(1).copied(), Some("lose" | "loses"))
    {
        return Ok(None);
    }

    let lose_tokens = after_draw_words[2..]
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (lose_amount, lose_used) = parse_value(&lose_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing shared life-loss amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if lose_tokens
        .get(lose_used)
        .and_then(Token::as_word)
        .is_none_or(|word| word != "life")
    {
        return Err(CardTextError::ParseError(format!(
            "missing life keyword in shared draw/lose sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let trailing_words = words(&lose_tokens[lose_used + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing shared draw/lose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::Draw {
            count: draw_count.clone(),
            player: PlayerAst::You,
        },
        EffectAst::Draw {
            count: draw_count,
            player: PlayerAst::Attacking,
        },
        EffectAst::LoseLife {
            amount: lose_amount.clone(),
            player: PlayerAst::You,
        },
        EffectAst::LoseLife {
            amount: lose_amount,
            player: PlayerAst::Attacking,
        },
    ]))
}

pub(crate) fn parse_sentence_sacrifice_it_next_end_step(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("sacrifice"))
    {
        return Ok(None);
    }

    let Some(at_idx) = tokens.iter().position(|token| token.is_word("at")) else {
        return Ok(None);
    };
    if at_idx <= 1 {
        return Ok(None);
    }

    let timing_words = words(&tokens[at_idx..]);
    let matches_sacrifice_delay = timing_words.as_slice()
        == ["at", "the", "beginning", "of", "the", "next", "end", "step"]
        || timing_words.as_slice() == ["at", "the", "beginning", "of", "next", "end", "step"];
    if !matches_sacrifice_delay {
        return Ok(None);
    }

    let object_tokens = trim_commas(&tokens[1..at_idx]);
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in delayed next-end-step clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let object_words = words(&object_tokens);
    let filter = if matches!(
        object_words.as_slice(),
        ["it"]
            | ["them"]
            | ["the", "creature"]
            | ["that", "creature"]
            | ["the", "permanent"]
            | ["that", "permanent"]
            | ["the", "token"]
            | ["that", "token"]
    ) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(&object_tokens, false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
        player: PlayerFilter::Any,
        effects: vec![EffectAst::Sacrifice {
            filter,
            player: PlayerAst::Implicit,
            count: 1,
        }],
    }]))
}

pub(crate) fn parse_sentence_sacrifice_at_end_of_combat(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("sacrifice"))
    {
        return Ok(None);
    }
    let Some(at_idx) = tokens.iter().position(|token| token.is_word("at")) else {
        return Ok(None);
    };
    if at_idx <= 1 {
        return Ok(None);
    }

    let timing_words = words(&tokens[at_idx..]);
    let matches_end_of_combat = timing_words.as_slice() == ["at", "end", "of", "combat"]
        || timing_words.as_slice() == ["at", "the", "end", "of", "combat"];
    if !matches_end_of_combat {
        return Ok(None);
    }

    let object_tokens = trim_commas(&tokens[1..at_idx]);
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in end-of-combat clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let object_words = words(&object_tokens);
    let filter = if matches!(
        object_words.as_slice(),
        ["it"]
            | ["them"]
            | ["that", "token"]
            | ["this", "token"]
            | ["that", "permanent"]
            | ["this", "permanent"]
    ) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(&object_tokens, false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
        effects: vec![EffectAst::Sacrifice {
            filter,
            player: PlayerAst::Implicit,
            count: 1,
        }],
    }]))
}

pub(crate) fn parse_sentence_each_player_choose_and_sacrifice_rest(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_each_player_choose_and_sacrifice_rest(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_exile_instead_of_graveyard(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_exile_instead_of_graveyard_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_monstrosity(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_monstrosity_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_counter_removed(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_counter_removed_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_counter_kind_put_or_remove(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["for", "each", "kind", "of", "counter", "on"]) {
        return Ok(None);
    }
    let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };
    if comma_idx <= 6 || comma_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[6..comma_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(&target_tokens)?;

    let tail_tokens = trim_commas(&tokens[comma_idx + 1..]);
    let tail_words = words(&tail_tokens);
    if !tail_words.starts_with(&[
        "put", "another", "counter", "of", "that", "kind", "on", "it", "or", "remove",
    ]) {
        return Ok(None);
    }
    if tail_words.len() < 12 || tail_words[10] != "one" || tail_words[11] != "from" {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::ForEachCounterKindPutOrRemove {
        target,
    }]))
}

pub(crate) fn parse_put_counter_ladder_segments(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_on_comma(tokens);
    if segments.len() != 3 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let mut clause = trim_commas(segment).to_vec();
        if idx == 0 {
            if clause.is_empty() || !clause[0].is_word("put") {
                return Ok(None);
            }
            clause.remove(0);
        } else if clause.first().is_some_and(|token| token.is_word("and")) {
            clause.remove(0);
        }
        if clause.is_empty() {
            return Ok(None);
        }

        let Some(on_idx) = clause.iter().position(|token| token.is_word("on")) else {
            return Ok(None);
        };
        let descriptor = trim_commas(&clause[..on_idx]);
        let target_tokens = trim_commas(&clause[on_idx + 1..]);
        if descriptor.is_empty() || target_tokens.is_empty() {
            return Ok(None);
        }

        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        let target = parse_target_phrase(&target_tokens)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target,
            target_count: None,
            distributed: false,
        });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_put_counter_sequence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("put")) {
        return Ok(None);
    }
    if !tokens
        .iter()
        .any(|token| token.is_word("counter") || token.is_word("counters"))
    {
        return Ok(None);
    }

    if let Some(effects) = parse_put_counter_ladder_segments(tokens)? {
        return Ok(Some(effects));
    }

    if let Some(on_idx) = tokens.iter().position(|token| token.is_word("on")) {
        let descriptor_tokens = trim_commas(&tokens[1..on_idx]);
        let target_tokens = trim_commas(&tokens[on_idx + 1..]);
        if !descriptor_tokens.is_empty() && !target_tokens.is_empty() {
            let mut descriptors: Vec<Vec<Token>> = Vec::new();
            let comma_segments = split_on_comma(&descriptor_tokens);
            if comma_segments.len() >= 2 {
                for segment in comma_segments {
                    let mut clause = trim_commas(&segment);
                    if clause.first().is_some_and(|token| token.is_word("and")) {
                        clause.remove(0);
                    }
                    if clause.is_empty() {
                        descriptors.clear();
                        break;
                    }
                    descriptors.push(clause);
                }
            } else if let Some(and_idx) = descriptor_tokens
                .iter()
                .position(|token| token.is_word("and"))
            {
                let first = trim_commas(&descriptor_tokens[..and_idx]);
                let second = trim_commas(&descriptor_tokens[and_idx + 1..]);
                if !first.is_empty() && !second.is_empty() {
                    descriptors.push(first);
                    descriptors.push(second);
                }
            }

            if descriptors.len() >= 2 {
                let target = parse_target_phrase(&target_tokens)?;
                let mut effects = Vec::new();
                for descriptor in descriptors {
                    let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
                    effects.push(EffectAst::PutCounters {
                        counter_type,
                        count: Value::Fixed(count as i32),
                        target: target.clone(),
                        target_count: None,
                        distributed: false,
                    });
                }
                return Ok(Some(effects));
            }
        }
    }

    // Handle "put ... counter on X and it gains ... until end of turn."
    if let Some(and_idx) = tokens
        .windows(2)
        .position(|window| window[0].is_word("and") && window[1].is_word("it"))
    {
        let first_clause = trim_commas(&tokens[1..and_idx]);
        let second_clause = trim_commas(&tokens[and_idx + 1..]);
        if !first_clause.is_empty()
            && !second_clause.is_empty()
            && second_clause.iter().any(|token| {
                token.is_word("gain")
                    || token.is_word("gains")
                    || token.is_word("has")
                    || token.is_word("have")
            })
            && let Ok(first) = parse_put_counters(&first_clause)
            && let Some(mut gain_effects) = parse_gain_ability_sentence(&second_clause)?
        {
            let source_target = match &first {
                EffectAst::PutCounters { target, .. } => Some(target.clone()),
                EffectAst::Conditional { if_true, .. }
                    if if_true.len() == 1
                        && matches!(if_true.first(), Some(EffectAst::PutCounters { .. })) =>
                {
                    if let Some(EffectAst::PutCounters { target, .. }) = if_true.first() {
                        Some(target.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(source_target) = source_target {
                for effect in &mut gain_effects {
                    match effect {
                        EffectAst::Pump { target, .. }
                        | EffectAst::GrantAbilitiesToTarget { target, .. }
                        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. } => {
                            if let TargetAst::Tagged(tag, _) = target
                                && tag.as_str() == IT_TAG
                            {
                                *target = source_target.clone();
                            }
                        }
                        _ => {}
                    }
                }

                let mut effects = vec![first];
                effects.append(&mut gain_effects);
                return Ok(Some(effects));
            }
        }
    }

    // Handle "put ... and ... counter on ..." without comma separation.
    if let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) {
        let first_clause = trim_commas(&tokens[1..and_idx]);
        let second_clause = trim_commas(&tokens[and_idx + 1..]);
        if !first_clause.is_empty() && !second_clause.is_empty() {
            if let (Ok(first), Ok(second)) = (
                parse_put_counters(&first_clause),
                parse_put_counters(&second_clause),
            ) {
                return Ok(Some(vec![first, second]));
            }
        }
    }

    let segments = split_on_comma(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let mut clause = segment.clone();
        if idx == 0 {
            if clause.is_empty() || !clause[0].is_word("put") {
                return Ok(None);
            }
            clause.remove(0);
        } else if clause.first().is_some_and(|token| token.is_word("and")) {
            clause.remove(0);
        }

        if clause.is_empty() {
            return Ok(None);
        }

        let clause_words = words(&clause);
        if !clause_words.contains(&"counter") && !clause_words.contains(&"counters") {
            return Ok(None);
        }

        let Ok(effect) = parse_put_counters(&clause) else {
            return Ok(None);
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        Ok(Some(effects))
    } else {
        Ok(None)
    }
}

pub(crate) fn is_pump_like_effect(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::Pump { .. }
            | EffectAst::PumpByLastEffect { .. }
            | EffectAst::SetBasePowerToughness { .. }
            | EffectAst::SetBasePower { .. }
    )
}

pub(crate) fn parse_gets_then_fights_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut body_tokens = tokens;
    if body_tokens
        .first()
        .is_some_and(|token| token.is_word("then"))
    {
        body_tokens = &body_tokens[1..];
    }
    if body_tokens.is_empty() {
        return Ok(None);
    }

    let Some(fight_idx) = body_tokens
        .iter()
        .position(|token| token.is_word("fight") || token.is_word("fights"))
    else {
        return Ok(None);
    };
    if fight_idx == 0 || fight_idx + 1 >= body_tokens.len() {
        return Ok(None);
    }

    let mut left_tokens = trim_commas(&body_tokens[..fight_idx]).to_vec();
    while left_tokens.last().is_some_and(|token| token.is_word("and")) {
        left_tokens.pop();
    }
    let left_tokens = trim_commas(&left_tokens);
    let right_tokens = trim_commas(&body_tokens[fight_idx + 1..]);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return Ok(None);
    }

    let Some(get_idx) = left_tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    if get_idx == 0 {
        return Ok(None);
    }

    let pump_effect = parse_effect_clause(&left_tokens)?;
    if !is_pump_like_effect(&pump_effect) {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&left_tokens[..get_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let creature1 = parse_target_phrase(&subject_tokens)?;
    let creature2 = parse_target_phrase(&right_tokens)?;
    if matches!(
        creature1,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) || matches!(
        creature2,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "fight target must be a creature (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(Some(vec![
        pump_effect,
        EffectAst::Fight {
            creature1,
            creature2,
        },
    ]))
}

pub(crate) fn parse_sentence_gets_then_fights(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gets_then_fights_sentence(tokens)
}

pub(crate) fn parse_return_with_counters_on_it_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }

    let Some(to_idx) = tokens.iter().rposition(|token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..to_idx]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before destination (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let destination_tokens = trim_commas(&tokens[to_idx + 1..]);
    if destination_tokens.is_empty() {
        return Ok(None);
    }
    if !words(&destination_tokens).contains(&"battlefield") {
        return Ok(None);
    }

    let Some(with_idx) = destination_tokens
        .iter()
        .position(|token| token.is_word("with"))
    else {
        return Ok(None);
    };
    if with_idx + 1 >= destination_tokens.len() {
        return Ok(None);
    }

    let base_destination_words: Vec<&str> = words(&destination_tokens[..with_idx])
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let Some(battlefield_idx) = base_destination_words
        .iter()
        .position(|word| *word == "battlefield")
    else {
        return Ok(None);
    };
    let tapped = base_destination_words.contains(&"tapped");
    let destination_tail: Vec<&str> = base_destination_words[battlefield_idx + 1..]
        .iter()
        .copied()
        .filter(|word| *word != "tapped")
        .collect();
    let battlefield_controller = if destination_tail.is_empty()
        || destination_tail == ["under", "its", "control"]
        || destination_tail == ["under", "their", "control"]
    {
        ReturnControllerAst::Preserve
    } else if destination_tail == ["under", "your", "control"] {
        ReturnControllerAst::You
    } else if destination_tail == ["under", "its", "owners", "control"]
        || destination_tail == ["under", "their", "owners", "control"]
        || destination_tail == ["under", "that", "players", "control"]
    {
        ReturnControllerAst::Owner
    } else {
        return Ok(None);
    };

    let counter_clause_tokens = trim_commas(&destination_tokens[with_idx + 1..]);
    let Some(on_idx) = counter_clause_tokens
        .iter()
        .rposition(|token| token.is_word("on"))
    else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words = words(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut descriptors = Vec::new();
    for descriptor in split_on_and(&descriptor_tokens) {
        let descriptor = trim_commas(&descriptor);
        if descriptor.is_empty() {
            continue;
        }
        descriptors.push(descriptor);
    }
    if descriptors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut effects = vec![EffectAst::ReturnToBattlefield {
        target: parse_target_phrase(&target_tokens)?,
        tapped,
        controller: battlefield_controller,
    }];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target: tagged_target.clone(),
            target_count: None,
            distributed: false,
        });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_put_onto_battlefield_with_counters_on_it_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("put") || token.is_word("puts"))
    {
        return Ok(None);
    }

    let Some(onto_idx) = tokens.iter().position(|token| token.is_word("onto")) else {
        return Ok(None);
    };
    if onto_idx <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..onto_idx]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing put target before destination (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let destination_tokens = trim_commas(&tokens[onto_idx + 1..]);
    if destination_tokens.is_empty() {
        return Ok(None);
    }
    if !words(&destination_tokens).contains(&"battlefield") {
        return Ok(None);
    }

    let Some(with_idx) = destination_tokens
        .iter()
        .position(|token| token.is_word("with"))
    else {
        return Ok(None);
    };
    if with_idx + 1 >= destination_tokens.len() {
        return Ok(None);
    }

    let base_destination_words: Vec<&str> = words(&destination_tokens[..with_idx])
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if base_destination_words.first() != Some(&"battlefield") {
        return Ok(None);
    }

    let destination_tail = &base_destination_words[1..];
    let supported_control_tail = destination_tail.is_empty()
        || destination_tail == ["under", "your", "control"]
        || destination_tail == ["under", "its", "owners", "control"]
        || destination_tail == ["under", "their", "owners", "control"]
        || destination_tail == ["under", "that", "players", "control"];
    if !supported_control_tail {
        return Ok(None);
    }
    let battlefield_controller = if destination_tail == ["under", "your", "control"] {
        ReturnControllerAst::You
    } else if destination_tail == ["under", "its", "owners", "control"]
        || destination_tail == ["under", "their", "owners", "control"]
        || destination_tail == ["under", "that", "players", "control"]
    {
        ReturnControllerAst::Owner
    } else {
        ReturnControllerAst::Preserve
    };

    let counter_clause_tokens = trim_commas(&destination_tokens[with_idx + 1..]);
    let Some(on_idx) = counter_clause_tokens
        .iter()
        .rposition(|token| token.is_word("on"))
    else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words = words(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    let descriptor_words = words(&descriptor_tokens);
    if descriptor_tokens.is_empty() || !descriptor_words.contains(&"counter") {
        return Ok(None);
    }

    let mut descriptors = Vec::new();
    for descriptor in split_on_and(&descriptor_tokens) {
        let descriptor = trim_commas(&descriptor);
        if descriptor.is_empty() {
            continue;
        }
        descriptors.push(descriptor);
    }
    if descriptors.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::MoveToZone {
        target: parse_target_phrase(&target_tokens)?,
        zone: Zone::Battlefield,
        to_top: false,
        battlefield_controller,
        battlefield_tapped: false,
        attached_to: None,
    }];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target: tagged_target.clone(),
            target_count: None,
            distributed: false,
        });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_return_with_counters_on_it(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_with_counters_on_it_sentence(tokens)
}

pub(crate) fn parse_sentence_put_onto_battlefield_with_counters_on_it(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_put_onto_battlefield_with_counters_on_it_sentence(tokens)
}

pub(crate) fn replace_target_subtype(target: &mut TargetAst, subtype: Subtype) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.subtypes = vec![subtype];
            true
        }
        TargetAst::WithCount(inner, _) => replace_target_subtype(inner, subtype),
        _ => false,
    }
}

pub(crate) fn clone_return_effect_with_subtype(
    base: &EffectAst,
    subtype: Subtype,
) -> Option<EffectAst> {
    match base {
        EffectAst::ReturnToHand { target, random } => {
            let mut cloned_target = target.clone();
            replace_target_subtype(&mut cloned_target, subtype).then_some(EffectAst::ReturnToHand {
                target: cloned_target,
                random: *random,
            })
        }
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            controller,
        } => {
            let mut cloned_target = target.clone();
            replace_target_subtype(&mut cloned_target, subtype).then_some(
                EffectAst::ReturnToBattlefield {
                    target: cloned_target,
                    tapped: *tapped,
                    controller: *controller,
                },
            )
        }
        EffectAst::ReturnAllToHand { filter } => {
            let mut cloned_filter = filter.clone();
            cloned_filter.subtypes = vec![subtype];
            Some(EffectAst::ReturnAllToHand {
                filter: cloned_filter,
            })
        }
        EffectAst::ReturnAllToBattlefield { filter, tapped } => {
            let mut cloned_filter = filter.clone();
            cloned_filter.subtypes = vec![subtype];
            Some(EffectAst::ReturnAllToBattlefield {
                filter: cloned_filter,
                tapped: *tapped,
            })
        }
        _ => None,
    }
}

pub(crate) fn parse_draw_then_connive_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(comma_then_idx) = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
    else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    if !tail_tokens
        .iter()
        .any(|token| token.is_word("connive") || token.is_word("connives"))
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let Some(connive_effect) = parse_connive_clause(&tail_tokens)? else {
        return Ok(None);
    };
    head_effects.push(connive_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_draw_then_connive(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_then_connive_sentence(tokens)
}

pub(crate) fn parse_if_enters_with_additional_counter_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("if")) {
        return Ok(None);
    }

    let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };
    if comma_idx <= 1 || comma_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let predicate_tokens = trim_commas(&tokens[1..comma_idx]);
    let predicate_words: Vec<&str> = words(&predicate_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let predicate_is_supported = predicate_words.as_slice()
        == ["creature", "enters", "this", "way"]
        || predicate_words.as_slice() == ["it", "enters", "as", "creature"];
    if !predicate_is_supported {
        return Ok(None);
    }

    let followup_tokens = trim_commas(&tokens[comma_idx + 1..]);
    let followup_words = words(&followup_tokens);
    if !followup_words.starts_with(&["it", "enters", "with"]) {
        return Ok(None);
    }

    let Some(with_idx) = followup_tokens
        .iter()
        .position(|token| token.is_word("with"))
    else {
        return Ok(None);
    };
    if with_idx + 1 >= followup_tokens.len() {
        return Ok(None);
    }

    let counter_clause_tokens = trim_commas(&followup_tokens[with_idx + 1..]);
    let Some(on_idx) = counter_clause_tokens
        .iter()
        .rposition(|token| token.is_word("on"))
    else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words = words(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    let descriptor_words = words(&descriptor_tokens);
    if descriptor_tokens.is_empty() || !descriptor_words.contains(&"additional") {
        return Ok(None);
    }

    let (count, counter_type) = parse_counter_descriptor(&descriptor_tokens)?;
    let put_counter = EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        target_count: None,
        distributed: false,
    };
    let apply_only_if_creature = EffectAst::Conditional {
        predicate: PredicateAst::ItMatches(ObjectFilter::creature()),
        if_true: vec![put_counter],
        if_false: Vec::new(),
    };

    Ok(Some(vec![EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![apply_only_if_creature],
    }]))
}

pub(crate) fn parse_each_player_return_with_additional_counter_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    let inner_start_word_idx = if clause_words.starts_with(&["for", "each", "player"])
        || clause_words.starts_with(&["for", "each", "players"])
    {
        3
    } else if clause_words.starts_with(&["each", "player"])
        || clause_words.starts_with(&["each", "players"])
    {
        2
    } else {
        return Ok(None);
    };

    let Some(inner_start_token_idx) = token_index_for_word_index(tokens, inner_start_word_idx)
    else {
        return Ok(None);
    };
    let inner_tokens = trim_commas(&tokens[inner_start_token_idx..]);
    if inner_tokens.is_empty() {
        return Ok(None);
    }
    if !inner_tokens
        .first()
        .is_some_and(|token| token.is_word("return") || token.is_word("returns"))
    {
        return Ok(None);
    }

    let Some(with_idx) = inner_tokens.iter().rposition(|token| token.is_word("with")) else {
        return Ok(None);
    };
    if with_idx + 1 >= inner_tokens.len() {
        return Ok(None);
    }

    let return_clause_tokens = trim_commas(&inner_tokens[..with_idx]);
    if return_clause_tokens.is_empty() {
        return Ok(None);
    }

    let counter_clause_tokens = trim_commas(&inner_tokens[with_idx + 1..]);
    let Some(on_idx) = counter_clause_tokens
        .iter()
        .rposition(|token| token.is_word("on"))
    else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words = words(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    let descriptor_words = words(&descriptor_tokens);
    if descriptor_tokens.is_empty() || !descriptor_words.contains(&"additional") {
        return Ok(None);
    }

    let (count, counter_type) = parse_counter_descriptor(&descriptor_tokens)?;
    let mut per_player_effects = parse_effect_chain_inner(&return_clause_tokens)?;
    if per_player_effects.is_empty() {
        return Ok(None);
    }
    if !per_player_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::ReturnToBattlefield { .. } | EffectAst::ReturnAllToBattlefield { .. }
        )
    }) {
        return Ok(None);
    }

    per_player_effects.push(EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        target_count: None,
        distributed: false,
    });

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: per_player_effects,
    }]))
}

pub(crate) fn parse_sentence_each_player_return_with_additional_counter(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_return_with_additional_counter_sentence(tokens)
}

pub(crate) fn parse_return_then_do_same_for_subtypes_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(comma_then_idx) = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
    else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = words(&tail_tokens);
    if !tail_words.starts_with(&["do", "the", "same", "for"]) {
        return Ok(None);
    }
    let subtype_words = &tail_words[4..];
    if subtype_words.is_empty() {
        return Ok(None);
    }

    let mut extra_subtypes = Vec::new();
    for word in subtype_words {
        if matches!(*word, "and" | "or") {
            continue;
        }
        let Some(subtype) = parse_subtype_word(word)
            .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
        else {
            return Ok(None);
        };
        extra_subtypes.push(subtype);
    }
    if extra_subtypes.is_empty() {
        return Ok(None);
    }

    let mut effects = parse_effect_chain(&head_tokens)?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let base_effect = effects[0].clone();
    for subtype in extra_subtypes {
        let Some(cloned) = clone_return_effect_with_subtype(&base_effect, subtype) else {
            return Ok(None);
        };
        effects.push(cloned);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_return_then_do_same_for_subtypes(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_then_do_same_for_subtypes_sentence(tokens)
}

pub(crate) fn parse_sacrifice_any_number_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head_tokens, tail_tokens) =
        if let Some(then_idx) = tokens.iter().position(|token| token.is_word("then")) {
            if then_idx == 0 {
                return Ok(None);
            }
            (
                trim_commas(&tokens[..then_idx]),
                Some(trim_commas(&tokens[then_idx + 1..])),
            )
        } else {
            (tokens.to_vec(), None)
        };

    if !head_tokens
        .first()
        .is_some_and(|token| token.is_word("sacrifice"))
    {
        return Ok(None);
    }

    let mut idx = 1usize;
    if !(head_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("any"))
        && head_tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number")))
    {
        return Ok(None);
    }
    idx += 2;
    if head_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("of"))
    {
        idx += 1;
    }
    if idx >= head_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let filter_tokens = trim_commas(&head_tokens[idx..]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let filter = parse_object_filter(&filter_tokens, false)?;
    let tag = TagKey::from(IT_TAG);

    let mut effects = vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::any_number(),
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::SacrificeAll {
            filter: ObjectFilter::tagged(tag),
            player: PlayerAst::Implicit,
        },
    ];
    if let Some(tail_tokens) = tail_tokens
        && !tail_tokens.is_empty()
    {
        let mut tail_effects = parse_effect_chain(&tail_tokens)?;
        effects.append(&mut tail_effects);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_sacrifice_any_number(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_any_number_sentence(tokens)
}

pub(crate) fn parse_sacrifice_one_or_more_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("sacrifice"))
    {
        return Ok(None);
    }

    let mut idx = 1usize;
    let Some((minimum, used)) = parse_number(&tokens[idx..]) else {
        return Ok(None);
    };
    idx += used;
    if !(tokens.get(idx).is_some_and(|token| token.is_word("or"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("more")))
    {
        return Ok(None);
    }
    idx += 2;
    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }
    if idx >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let filter_tokens = trim_commas(&tokens[idx..]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let filter = parse_object_filter(&filter_tokens, false)?;
    let tag = TagKey::from(IT_TAG);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::at_least(minimum as usize),
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::SacrificeAll {
            filter: ObjectFilter::tagged(tag),
            player: PlayerAst::Implicit,
        },
    ]))
}

pub(crate) fn parse_sentence_sacrifice_one_or_more(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_one_or_more_sentence(tokens)
}

pub(crate) fn parse_sentence_keyword_then_chain(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(then_idx) = tokens.iter().position(|token| token.is_word("then")) else {
        return Ok(None);
    };
    if then_idx == 0 || then_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let head_tokens = trim_commas(&tokens[..then_idx]);
    let Some(head_effect) = parse_keyword_mechanic_clause(&head_tokens)? else {
        return Ok(None);
    };

    let tail_tokens = trim_commas(&tokens[then_idx + 1..]);
    if tail_tokens.is_empty() {
        return Ok(Some(vec![head_effect]));
    }

    let mut effects = vec![head_effect];
    if let Some(mut counter_effects) = parse_sentence_put_counter_sequence(&tail_tokens)? {
        effects.append(&mut counter_effects);
        return Ok(Some(effects));
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    effects.append(&mut tail_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_chain_then_keyword(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
        .map(|idx| (idx, idx + 2))
        .or_else(|| {
            tokens
                .iter()
                .position(|token| token.is_word("then"))
                .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
        });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..head_end]);
    let tail_tokens = trim_commas(&tokens[tail_start..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let Some(keyword_effect) = parse_keyword_mechanic_clause(&tail_tokens)? else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    head_effects.push(keyword_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_return_then_create(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
        .map(|idx| (idx, idx + 2))
        .or_else(|| {
            tokens
                .iter()
                .position(|token| token.is_word("then"))
                .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
        });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..head_end]);
    let tail_tokens = trim_commas(&tokens[tail_start..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let head_words = words(&head_tokens);
    let tail_words = words(&tail_tokens);
    if head_words.first() != Some(&"return") || tail_words.first() != Some(&"create") {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_exile_then_may_put_from_exile(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
        .map(|idx| (idx, idx + 2))
        .or_else(|| {
            tokens
                .iter()
                .position(|token| token.is_word("then"))
                .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
        });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..head_end]);
    let tail_tokens = trim_commas(&tokens[tail_start..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = words(&tail_tokens);
    if !tail_words.starts_with(&["you", "may", "put", "any", "number", "of"])
        || !tail_words.contains(&"from")
        || !tail_words.contains(&"exile")
        || !tail_words.contains(&"battlefield")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_exile_source_with_counters_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("exile")) {
        return Ok(None);
    }

    let Some(with_idx) = tokens.iter().position(|token| token.is_word("with")) else {
        return Ok(None);
    };
    if with_idx <= 1 || with_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let source_name_tokens = trim_commas(&tokens[1..with_idx]);
    if source_name_tokens.is_empty() {
        return Ok(None);
    }
    let source_name_words = words(&source_name_tokens);
    if !is_likely_named_or_source_reference_words(&source_name_words) {
        return Ok(None);
    }

    let counter_clause_tokens = trim_commas(&tokens[with_idx + 1..]);
    let Some(on_idx) = counter_clause_tokens
        .iter()
        .rposition(|token| token.is_word("on"))
    else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words = words(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() {
        return Ok(None);
    }
    let (count, counter_type) = parse_counter_descriptor(&descriptor_tokens)?;

    let source_target = TargetAst::Source(span_from_tokens(tokens));
    Ok(Some(vec![
        EffectAst::Exile {
            target: source_target.clone(),
            face_down: false,
        },
        EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target: source_target,
            target_count: None,
            distributed: false,
        },
    ]))
}

pub(crate) fn parse_sentence_exile_source_with_counters(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_source_with_counters_sentence(tokens)
}

pub(crate) fn parse_sentence_comma_then_chain_special(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(comma_then_idx) = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
    else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let head_words = words(&head_tokens);
    let tail_words = words(&tail_tokens);
    let is_that_player_tail = tail_words.starts_with(&["that", "player"]);
    let is_return_source_tail = tail_words.starts_with(&["return", "this"])
        && (tail_words.contains(&"owner") || tail_words.contains(&"owners"))
        && tail_words.contains(&"hand");
    if !is_that_player_tail && !is_return_source_tail {
        return Ok(None);
    }
    if is_return_source_tail
        && !head_words
            .first()
            .is_some_and(|word| matches!(*word, "tap" | "untap"))
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_destroy_then_land_controller_graveyard_count_damage_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(comma_then_idx) = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
    else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = words(&tail_tokens);
    let suffix = [
        "damage",
        "to",
        "that",
        "lands",
        "controller",
        "equal",
        "to",
        "the",
        "number",
        "of",
        "land",
        "cards",
        "in",
        "that",
        "players",
        "graveyard",
    ];
    let Some(suffix_start) = tail_words
        .windows(suffix.len())
        .position(|window| window == suffix)
    else {
        return Ok(None);
    };
    if suffix_start == 0 || !matches!(tail_words[suffix_start - 1], "deal" | "deals") {
        return Ok(None);
    }
    if suffix_start + suffix.len() != tail_words.len() {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if !head_effects
        .iter()
        .any(|effect| matches!(effect, EffectAst::Destroy { .. }))
    {
        return Ok(None);
    }

    let mut count_filter = ObjectFilter::default();
    count_filter.zone = Some(Zone::Graveyard);
    let tagged_ref = crate::target::ObjectRef::tagged(IT_TAG);
    count_filter.owner = Some(PlayerFilter::ControllerOf(tagged_ref.clone()));
    count_filter.card_types.push(CardType::Land);
    head_effects.push(EffectAst::DealDamage {
        amount: Value::Count(count_filter),
        target: TargetAst::Player(
            PlayerFilter::ControllerOf(tagged_ref),
            span_from_tokens(&tail_tokens),
        ),
    });
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_destroy_all_attached_to_target(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_words = words(tokens);
    if sentence_words.first().copied() != Some("destroy") {
        return Ok(None);
    }
    if !tokens
        .get(1)
        .is_some_and(|token| token.is_word("all") || token.is_word("each"))
    {
        return Ok(None);
    }
    let Some(attached_idx) = tokens.iter().position(|token| token.is_word("attached")) else {
        return Ok(None);
    };
    if !tokens
        .get(attached_idx + 1)
        .is_some_and(|token| token.is_word("to"))
    {
        return Ok(None);
    }
    if attached_idx <= 2 || attached_idx + 2 >= tokens.len() {
        return Ok(None);
    }

    let mut filter_tokens = trim_commas(&tokens[2..attached_idx]).to_vec();
    while filter_tokens
        .last()
        .and_then(Token::as_word)
        .is_some_and(|word| matches!(word, "that" | "were" | "was" | "is" | "are"))
    {
        filter_tokens.pop();
    }
    let target_tokens = trim_commas(&tokens[attached_idx + 2..]);
    let target_words = words(&target_tokens);
    let has_timing_tail = target_words.iter().any(|word| {
        matches!(
            *word,
            "at" | "beginning" | "end" | "combat" | "turn" | "step" | "until"
        )
    });
    let supported_target = target_words.starts_with(&["target"])
        || target_words == ["it"]
        || target_words.starts_with(&["that", "creature"])
        || target_words.starts_with(&["that", "permanent"])
        || target_words.starts_with(&["that", "land"])
        || target_words.starts_with(&["that", "artifact"])
        || target_words.starts_with(&["that", "enchantment"]);
    if filter_tokens.is_empty() || target_tokens.is_empty() || !supported_target || has_timing_tail
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&filter_tokens, false)?;
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(vec![EffectAst::DestroyAllAttachedTo {
        filter,
        target,
    }]))
}

pub(crate) fn parse_sentence_destroy_then_land_controller_graveyard_count_damage(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_then_land_controller_graveyard_count_damage_sentence(tokens)
}

pub(crate) fn add_tagged_subtype_constraint_to_target(target: &mut TargetAst, tag: TagKey) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag,
                relation: TaggedOpbjectRelation::SharesSubtypeWithTagged,
            });
            true
        }
        TargetAst::WithCount(inner, _) => add_tagged_subtype_constraint_to_target(inner, tag),
        _ => false,
    }
}

pub(crate) fn find_creature_type_choice_phrase(tokens: &[Token]) -> Option<(usize, usize)> {
    for idx in 0..tokens.len() {
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("the"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("creature"))
            && tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("type"))
            && tokens.get(idx + 4).is_some_and(|token| token.is_word("of"))
            && tokens
                .get(idx + 5)
                .is_some_and(|token| token.is_word("your"))
            && tokens
                .get(idx + 6)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 7));
        }
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("creature"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("type"))
            && tokens.get(idx + 3).is_some_and(|token| token.is_word("of"))
            && tokens
                .get(idx + 4)
                .is_some_and(|token| token.is_word("your"))
            && tokens
                .get(idx + 5)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 6));
        }
    }
    None
}

pub(crate) fn find_color_choice_phrase(tokens: &[Token]) -> Option<(usize, usize)> {
    for idx in 0..tokens.len() {
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("the"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("color"))
            && tokens.get(idx + 3).is_some_and(|token| token.is_word("of"))
            && (tokens
                .get(idx + 4)
                .is_some_and(|token| token.is_word("your"))
                || tokens
                    .get(idx + 4)
                    .is_some_and(|token| token.is_word("their")))
            && tokens
                .get(idx + 5)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 6));
        }
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("color"))
            && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
            && (tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("your"))
                || tokens
                    .get(idx + 3)
                    .is_some_and(|token| token.is_word("their")))
            && tokens
                .get(idx + 4)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 5));
        }
    }
    None
}

pub(crate) fn parse_sentence_destroy_creature_type_of_choice(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["destroy", "all", "creatures"]) {
        return Ok(None);
    }
    if find_creature_type_choice_phrase(tokens).is_none() {
        return Ok(None);
    }

    let chosen_type_tag: TagKey = "chosen_creature_type_ref".into();
    let mut choose_filter = ObjectFilter::creature();
    choose_filter.controller = Some(PlayerFilter::Any);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: chosen_type_tag.clone(),
        },
        EffectAst::DestroyAll {
            filter: ObjectFilter::creature().match_tagged(
                chosen_type_tag,
                TaggedOpbjectRelation::SharesSubtypeWithTagged,
            ),
        },
    ]))
}

pub(crate) fn parse_sentence_pump_creature_type_of_choice(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    if get_idx == 0 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..get_idx]);
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(&subject_tokens) else {
        return Ok(None);
    };
    let trailing_subject = trim_commas(&subject_tokens[choice_idx + consumed..]);
    if !trailing_subject.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice subject clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let trimmed_subject_tokens = trim_commas(&subject_tokens[..choice_idx]).to_vec();
    if trimmed_subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let chosen_type_tag: TagKey = "chosen_creature_type_ref".into();
    let mut choose_filter = ObjectFilter::creature();
    choose_filter.controller = Some(PlayerFilter::Any);

    // Handle composed clauses like:
    // "Creatures of the creature type of your choice get +2/+2 and gain trample until end of turn."
    let mut gain_candidate_tokens = trimmed_subject_tokens.clone();
    gain_candidate_tokens.extend_from_slice(&tokens[get_idx..]);
    if let Some(mut gain_effects) = parse_gain_ability_sentence(&gain_candidate_tokens)? {
        let mut patched = false;
        for effect in &mut gain_effects {
            match effect {
                EffectAst::PumpAll { filter, .. }
                | EffectAst::GrantAbilitiesAll { filter, .. }
                | EffectAst::GrantAbilitiesChoiceAll { filter, .. } => {
                    if !filter.tagged_constraints.iter().any(|constraint| {
                        constraint.tag == chosen_type_tag
                            && constraint.relation == TaggedOpbjectRelation::SharesSubtypeWithTagged
                    }) {
                        filter.tagged_constraints.push(TaggedObjectConstraint {
                            tag: chosen_type_tag.clone(),
                            relation: TaggedOpbjectRelation::SharesSubtypeWithTagged,
                        });
                    }
                    patched = true;
                }
                _ => {}
            }
        }
        if patched {
            let mut effects = vec![EffectAst::ChooseObjects {
                filter: choose_filter,
                count: ChoiceCount::exactly(1),
                player: PlayerAst::You,
                tag: chosen_type_tag,
            }];
            effects.extend(gain_effects);
            return Ok(Some(effects));
        }
    }

    let mut filter_tokens = trimmed_subject_tokens;
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("all"))
    {
        filter_tokens.remove(0);
    }
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&filter_tokens, false)?;
    if !filter.card_types.contains(&CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice pump subject must be creature-based (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let modifier = tokens
        .get(get_idx + 1)
        .and_then(Token::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in creature-type choice pump clause (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;
    let (base_power, base_toughness) = parse_pt_modifier_values(modifier).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in creature-type choice pump clause (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    let (power, toughness, duration, condition) =
        parse_get_modifier_values_with_tail(&tokens[get_idx + 1..], base_power, base_toughness)?;
    if condition.is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional gets duration in creature-type choice pump clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: chosen_type_tag.clone(),
        relation: TaggedOpbjectRelation::SharesSubtypeWithTagged,
    });

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: chosen_type_tag,
        },
        EffectAst::PumpAll {
            filter,
            power,
            toughness,
            duration,
        },
    ]))
}

pub(crate) fn parse_sentence_return_targets_of_creature_type_of_choice(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(to_idx) = tokens.iter().rposition(|token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    let destination_words = words(&tokens[to_idx + 1..]);
    if !destination_words.contains(&"hand") && !destination_words.contains(&"hands") {
        return Ok(None);
    }

    let target_tokens = &tokens[1..to_idx];
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(target_tokens) else {
        return Ok(None);
    };

    let trimmed_target = trim_commas(&target_tokens[..choice_idx]);
    if trimmed_target.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before creature-type choice phrase (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let trailing = trim_commas(&target_tokens[choice_idx + consumed..]);
    if !trailing.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing return target clause after creature-type choice phrase (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut target = parse_target_phrase(&trimmed_target)?;
    let chosen_type_tag: TagKey = "chosen_creature_type_ref".into();
    if !add_tagged_subtype_constraint_to_target(&mut target, chosen_type_tag.clone()) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice return target must be object-based (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut choose_filter = ObjectFilter::creature();
    choose_filter.controller = Some(PlayerFilter::Any);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: chosen_type_tag,
        },
        EffectAst::ReturnToHand {
            target,
            random: false,
        },
    ]))
}

pub(crate) fn return_segment_mentions_zone(tokens: &[Token]) -> bool {
    let segment_words = words(tokens);
    segment_words.contains(&"graveyard")
        || segment_words.contains(&"graveyards")
        || segment_words.contains(&"battlefield")
        || segment_words.contains(&"hand")
        || segment_words.contains(&"hands")
        || segment_words.contains(&"library")
        || segment_words.contains(&"libraries")
        || segment_words.contains(&"exile")
}

pub(crate) fn parse_sentence_return_multiple_targets(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(to_idx) = tokens.iter().rposition(|token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    let destination_words = words(&tokens[to_idx + 1..]);
    let is_hand = destination_words.contains(&"hand") || destination_words.contains(&"hands");
    let is_battlefield = destination_words.contains(&"battlefield");
    let tapped = destination_words.contains(&"tapped");
    if !is_hand && !is_battlefield {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..to_idx]);
    let has_multi_separator = target_tokens.iter().any(|token| {
        token.is_word("and")
            || matches!(token, Token::Comma(_))
            || token.is_word("or")
            || token.is_word("and/or")
    });
    if !has_multi_separator {
        return Ok(None);
    }

    let mut segments: Vec<Vec<Token>> = Vec::new();
    for and_segment in split_on_and(&target_tokens) {
        for comma_segment in split_on_comma(&and_segment) {
            let trimmed = trim_commas(&comma_segment);
            if !trimmed.is_empty() {
                let trimmed_words = words(&trimmed);
                let starts_new_target = trimmed_words.first().is_some_and(|word| {
                    matches!(
                        *word,
                        "target"
                            | "up"
                            | "another"
                            | "other"
                            | "this"
                            | "that"
                            | "it"
                            | "them"
                            | "all"
                            | "each"
                    )
                });
                let mentions_target = trimmed_words.contains(&"target");
                let starts_like_zone_suffix = trimmed_words
                    .first()
                    .is_some_and(|word| matches!(*word, "from" | "to" | "in" | "on" | "under"));
                if !segments.is_empty()
                    && !starts_new_target
                    && !mentions_target
                    && !starts_like_zone_suffix
                {
                    let last = segments.last_mut().expect("segments is non-empty");
                    last.push(Token::Comma(TextSpan::synthetic()));
                    last.extend(trimmed.to_vec());
                } else {
                    segments.push(trimmed.to_vec());
                }
            }
        }
    }
    if segments.len() < 2 {
        return Ok(None);
    }

    let shared_quantifier = segments
        .first()
        .and_then(|segment| segment.first())
        .and_then(Token::as_word)
        .filter(|word| matches!(*word, "all" | "each"))
        .map(str::to_string);

    let shared_suffix = segments
        .last()
        .and_then(|segment| {
            segment
                .iter()
                .position(|token| token.is_word("from"))
                .map(|idx| segment[idx..].to_vec())
        })
        .unwrap_or_default();

    let mut effects = Vec::new();
    for mut segment in segments {
        if !return_segment_mentions_zone(&segment) && !shared_suffix.is_empty() {
            segment.extend(shared_suffix.clone());
        }
        if let Some(quantifier) = shared_quantifier.as_deref() {
            let segment_words = words(&segment);
            let has_explicit_quantifier =
                matches!(segment_words.first().copied(), Some("all" | "each"));
            let starts_like_target_reference = matches!(
                segment_words.first().copied(),
                Some("target" | "up" | "this" | "that" | "it" | "them" | "another")
            );
            if !has_explicit_quantifier
                && !starts_like_target_reference
                && !segment_words.contains(&"target")
            {
                segment.insert(
                    0,
                    Token::Word(quantifier.to_string(), TextSpan::synthetic()),
                );
            }
        }
        let segment_words = words(&segment);
        if matches!(segment_words.first().copied(), Some("all" | "each")) {
            if segment.len() < 2 {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter (clause: '{}')",
                    words(tokens).join(" ")
                )));
            }
            let filter = parse_object_filter(&segment[1..], false)?;
            if is_battlefield {
                effects.push(EffectAst::ReturnAllToBattlefield { filter, tapped });
            } else {
                effects.push(EffectAst::ReturnAllToHand { filter });
            }
        } else {
            let target = parse_target_phrase(&segment)?;
            if is_battlefield {
                effects.push(EffectAst::ReturnToBattlefield {
                    target,
                    tapped,
                    controller: ReturnControllerAst::Preserve,
                });
            } else {
                effects.push(EffectAst::ReturnToHand {
                    target,
                    random: false,
                });
            }
        }
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_for_each_of_target_objects(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if !(clause_words.starts_with(&["for", "each"]) || clause_words.first() == Some(&"each")) {
        return Ok(None);
    }

    let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };
    if comma_idx == 0 || comma_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..comma_idx]);
    let Some((mut filter, count)) = parse_for_each_targeted_object_subject(&subject_tokens)? else {
        return Ok(None);
    };
    if filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && filter.tagged_constraints.is_empty()
    {
        // Keep this unrestricted to avoid implicit "you control" defaulting in ChooseObjects
        // compilation for plain "target permanent(s)" clauses.
        filter.controller = Some(PlayerFilter::Any);
    }

    let effect_tokens = trim_commas(&tokens[comma_idx + 1..]);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after for-each target subject (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut per_target_effects = parse_effect_chain(&effect_tokens)?;
    for effect in &mut per_target_effects {
        bind_implicit_player_context(effect, PlayerAst::You);
    }
    if per_target_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "for-each target follow-up produced no effects (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count,
            player: PlayerAst::Implicit,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: per_target_effects,
        },
    ]))
}

pub(crate) fn parse_distribute_counters_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("distribute") {
        return Ok(None);
    }

    let (count, used) = parse_number(&tokens[1..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing distributed counter amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let rest = &tokens[1 + used..];
    let counter_type = parse_counter_type_from_tokens(rest).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported distributed counter type (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let among_idx = rest
        .iter()
        .position(|token| token.is_word("among"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed target clause after 'among' (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_tokens = trim_commas(&rest[among_idx + 1..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed counter targets (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let (target_count, used_count) = parse_counter_target_count_prefix(&target_tokens)?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed target count prefix (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_phrase = &target_tokens[used_count..];
    if target_phrase.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target phrase (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target = parse_target_phrase(target_phrase)?;

    Ok(Some(EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target,
        target_count: Some(target_count),
        distributed: true,
    }))
}

pub(crate) fn parse_sentence_distribute_counters(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut head_tokens = tokens.to_vec();
    let mut tail_tokens: Vec<Token> = Vec::new();

    if let Some(comma_then_idx) = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
    {
        head_tokens = tokens[..comma_then_idx].to_vec();
        tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    } else if let Some(then_idx) = tokens.iter().position(|token| token.is_word("then")) {
        head_tokens = tokens[..then_idx].to_vec();
        tail_tokens = trim_commas(&tokens[then_idx + 1..]);
    }

    let Some(primary) = parse_distribute_counters_sentence(&head_tokens)? else {
        return Ok(None);
    };

    let mut effects = vec![primary];
    if !tail_tokens.is_empty() {
        effects.extend(parse_effect_chain(&tail_tokens)?);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_take_extra_turn(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_take_extra_turn_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_earthbend(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(earthbend) = parse_earthbend_sentence(tokens)? else {
        return Ok(None);
    };

    // Support chained text like "earthbend 8, then untap that land."
    let Some((_, used)) = parse_number(&tokens[1..]) else {
        return Ok(Some(vec![earthbend]));
    };
    let mut tail = trim_commas(&tokens[1 + used..]).to_vec();
    while tail.first().is_some_and(|token| token.is_word("then")) {
        tail.remove(0);
    }
    if tail.is_empty() {
        return Ok(Some(vec![earthbend]));
    }

    let mut effects = vec![earthbend];
    effects.extend(parse_effect_chain(&tail)?);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_enchant(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_enchant_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_cant_effect(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_cant_effect_sentence(tokens)
}

pub(crate) fn parse_sentence_prevent_damage(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_prevent_damage_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_gain_ability_to_source(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_gain_ability_to_source_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_gain_ability(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_ability_sentence(tokens)
}

pub(crate) fn parse_sentence_you_and_each_opponent_voted_with_you(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_each_opponent_voted_with_you_sentence(tokens)
}

pub(crate) fn parse_sentence_gain_life_equal_to_power(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_life_equal_to_power_sentence(tokens)
}

pub(crate) fn parse_sentence_gain_x_plus_life(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_x_plus_life_sentence(tokens)
}

pub(crate) fn parse_sentence_for_each_exiled_this_way(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_for_each_exiled_this_way_sentence(tokens)
}

pub(crate) fn parse_sentence_each_player_put_permanent_cards_exiled_with_source(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_put_permanent_cards_exiled_with_source_sentence(tokens)
}

pub(crate) fn parse_sentence_for_each_destroyed_this_way(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_for_each_destroyed_this_way_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_then_return_same_object(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_then_return_same_object_sentence(tokens)
}

pub(crate) fn parse_sentence_search_library(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_search_library_sentence(tokens)
}

pub(crate) fn parse_sentence_shuffle_graveyard_into_library(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shuffle_graveyard_into_library_sentence(tokens)
}

pub(crate) fn parse_sentence_shuffle_object_into_library(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shuffle_object_into_library_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_hand_and_graveyard_bundle(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_hand_and_graveyard_bundle_sentence(tokens)
}

pub(crate) fn parse_sentence_target_player_exiles_creature_and_graveyard(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_target_player_exiles_creature_and_graveyard_sentence(tokens)
}

pub(crate) fn parse_sentence_play_from_graveyard(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_play_from_graveyard_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_look_at_hand(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_look_at_hand_sentence(tokens)
}

pub(crate) fn parse_sentence_look_at_top_then_exile_one(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_look_at_top_then_exile_one_sentence(tokens)
}

pub(crate) fn parse_sentence_gain_life_equal_to_age(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_life_equal_to_age_sentence(tokens)
}

pub(crate) fn parse_sentence_for_each_opponent_doesnt(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_opponent_doesnt(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_player_doesnt(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_player_doesnt(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_each_opponent_loses_x_and_you_gain_x(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_words = words(tokens);
    if !(sentence_words.starts_with(&["each", "opponent"])
        || sentence_words.starts_with(&["each", "opponents"]))
    {
        return Ok(None);
    }

    let has_lose_x = sentence_words.windows(3).any(|window| {
        (window[0] == "lose" || window[0] == "loses") && window[1] == "x" && window[2] == "life"
    });
    let has_gain_x = sentence_words
        .windows(4)
        .any(|window| window == ["you", "gain", "x", "life"]);
    let Some(where_idx) = sentence_words
        .windows(3)
        .position(|window| window == ["where", "x", "is"])
    else {
        return Ok(None);
    };
    if !has_lose_x || !has_gain_x {
        return Ok(None);
    }

    let where_token_idx = token_index_for_word_index(tokens, where_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing where-x clause in opponent life-drain clause (clause: '{}')",
            sentence_words.join(" ")
        ))
    })?;
    let where_tokens = &tokens[where_token_idx..];
    let where_value = parse_where_x_value_clause(where_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported where-x value in opponent life-drain clause (clause: '{}')",
            sentence_words.join(" ")
        ))
    })?;

    Ok(Some(vec![
        EffectAst::ForEachOpponent {
            effects: vec![EffectAst::LoseLife {
                amount: where_value.clone(),
                player: PlayerAst::Implicit,
            }],
        },
        EffectAst::GainLife {
            amount: where_value,
            player: PlayerAst::You,
        },
    ]))
}

pub(crate) fn parse_sentence_vote_start(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_vote_start_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_vote_clause(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_vote_clause(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_vote_extra(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_vote_extra_sentence(tokens).map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_after_turn(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_after_turn_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_same_name_target_fanout(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_same_name_target_fanout_sentence(tokens)
}

pub(crate) fn parse_sentence_shared_color_target_fanout(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shared_color_target_fanout_sentence(tokens)
}

pub(crate) fn parse_sentence_same_name_gets_fanout(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_same_name_gets_fanout_sentence(tokens)
}

pub(crate) fn parse_sentence_delayed_until_next_end_step(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_delayed_until_next_end_step_sentence(tokens)
}

pub(crate) fn parse_sentence_destroy_or_exile_all_split(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_or_exile_all_split_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_up_to_one_each_target_type(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_up_to_one_each_target_type_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_multi_target(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first() != Some(&"exile") || clause_words.contains(&"unless") {
        return Ok(None);
    }

    let mut split_idx = None;
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_word("and") || idx == 0 || idx + 1 >= tokens.len() {
            continue;
        }
        let tail_words = words(&tokens[idx + 1..]);
        let starts_second_target = tail_words.first() == Some(&"target")
            || (tail_words.starts_with(&["up", "to"]) && tail_words.contains(&"target"));
        if starts_second_target {
            split_idx = Some(idx);
            break;
        }
    }

    let Some(and_idx) = split_idx else {
        return Ok(None);
    };

    let first_tokens = trim_commas(&tokens[1..and_idx]);
    let second_tokens = trim_commas(&tokens[and_idx + 1..]);
    if first_tokens.is_empty() || second_tokens.is_empty() {
        return Ok(None);
    }

    let first_words = words(&first_tokens);
    let second_words = words(&second_tokens);
    let first_is_explicit_target = first_words.first() == Some(&"target")
        || (first_words.starts_with(&["up", "to"]) && first_words.contains(&"target"));
    let second_is_explicit_target = second_words.first() == Some(&"target")
        || (second_words.starts_with(&["up", "to"]) && second_words.contains(&"target"));

    let mut first_target = match parse_target_phrase(&first_tokens) {
        Ok(target) => target,
        Err(_)
            if !first_is_explicit_target
                && is_likely_named_or_source_reference_words(&first_words) =>
        {
            TargetAst::Source(span_from_tokens(&first_tokens))
        }
        Err(err) => return Err(err),
    };
    let mut second_target = parse_target_phrase(&second_tokens)?;

    if first_is_explicit_target
        && second_is_explicit_target
        && let (Some((mut first_filter, first_count)), Some((mut second_filter, second_count))) = (
            object_target_with_count(&first_target),
            object_target_with_count(&second_target),
        )
        && first_filter.zone == Some(Zone::Graveyard)
        && second_filter.zone == Some(Zone::Graveyard)
    {
        if first_filter.controller.is_none() {
            first_filter.controller = Some(PlayerFilter::Any);
        }
        if second_filter.controller.is_none() {
            second_filter.controller = Some(PlayerFilter::Any);
        }
        let tag = TagKey::from("exiled_0");
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: first_filter,
                count: first_count,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::ChooseObjects {
                filter: second_filter,
                count: second_count,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::Exile {
                target: TargetAst::Tagged(tag, None),
                face_down: false,
            },
        ]));
    }

    apply_exile_subject_hand_owner_context(&mut first_target, None);
    apply_exile_subject_hand_owner_context(&mut second_target, None);
    Ok(Some(vec![
        EffectAst::Exile {
            target: first_target,
            face_down: false,
        },
        EffectAst::Exile {
            target: second_target,
            face_down: false,
        },
    ]))
}

pub(crate) fn split_destroy_target_segments(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut raw_segments: Vec<Vec<Token>> = Vec::new();
    for and_segment in split_on_and(tokens) {
        for comma_segment in split_on_comma(&and_segment) {
            let trimmed = trim_commas(&comma_segment);
            if !trimmed.is_empty() {
                raw_segments.push(trimmed.to_vec());
            }
        }
    }

    let mut segments = Vec::new();
    for segment in raw_segments {
        let split_starts = segment
            .iter()
            .enumerate()
            .filter_map(|(idx, token)| {
                if idx >= 3
                    && token.is_word("target")
                    && segment[idx - 3].is_word("up")
                    && segment[idx - 2].is_word("to")
                    && segment[idx - 1].is_word("one")
                {
                    Some(idx - 3)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if split_starts.len() <= 1 {
            segments.push(segment);
            continue;
        }

        for (idx, start) in split_starts.iter().enumerate() {
            let end = split_starts.get(idx + 1).copied().unwrap_or(segment.len());
            let trimmed = trim_commas(&segment[*start..end]);
            if !trimmed.is_empty() {
                segments.push(trimmed.to_vec());
            }
        }
    }

    segments
}

pub(crate) fn parse_sentence_destroy_multi_target(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first() != Some(&"destroy") {
        return Ok(None);
    }
    if clause_words
        .get(1)
        .is_some_and(|word| matches!(*word, "all" | "each"))
    {
        return Ok(None);
    }
    if clause_words.contains(&"unless") || clause_words.contains(&"if") {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..]);
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let has_separator = target_tokens
        .iter()
        .any(|token| token.is_word("and") || matches!(token, Token::Comma(_)));
    let has_repeated_up_to_one_targets = target_tokens
        .windows(4)
        .filter(|window| {
            window[0].is_word("up")
                && window[1].is_word("to")
                && window[2].is_word("one")
                && window[3].is_word("target")
        })
        .count()
        >= 2;
    if !has_separator && !has_repeated_up_to_one_targets {
        return Ok(None);
    }

    let segments = split_destroy_target_segments(&target_tokens);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for segment in segments {
        let segment_words = words(&segment);
        if segment_words.iter().any(|word| {
            matches!(
                *word,
                "then" | "if" | "unless" | "where" | "when" | "whenever"
            )
        }) {
            return Ok(None);
        }
        let is_explicit_target = segment_words.first() == Some(&"target")
            || (segment_words.starts_with(&["up", "to"]) && segment_words.contains(&"target"));
        if !is_explicit_target && !is_likely_named_or_source_reference_words(&segment_words) {
            return Ok(None);
        }
        let target = match parse_target_phrase(&segment) {
            Ok(target) => target,
            Err(_)
                if !is_explicit_target
                    && is_likely_named_or_source_reference_words(&segment_words) =>
            {
                TargetAst::Source(span_from_tokens(&segment))
            }
            Err(err) => return Err(err),
        };
        effects.push(EffectAst::Destroy { target });
    }

    if effects.len() < 2 {
        return Ok(None);
    }
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_reveal_selected_cards_in_your_hand(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first() != Some(&"reveal") {
        return Ok(None);
    }
    if clause_words.iter().any(|word| {
        matches!(
            *word,
            "then" | "if" | "unless" | "where" | "when" | "whenever"
        )
    }) {
        return Ok(None);
    }

    let Some(in_idx) = tokens.iter().position(|token| token.is_word("in")) else {
        return Ok(None);
    };
    if in_idx == 0 || in_idx + 2 >= tokens.len() {
        return Ok(None);
    }
    if !tokens
        .get(in_idx + 1)
        .is_some_and(|token| token.is_word("your"))
        || !tokens
            .get(in_idx + 2)
            .is_some_and(|token| token.is_word("hand") || token.is_word("hands"))
    {
        return Ok(None);
    }

    let mut descriptor_tokens = trim_commas(&tokens[1..in_idx]);
    if descriptor_tokens.is_empty() {
        return Ok(None);
    }

    let mut count = ChoiceCount::exactly(1);
    let descriptor_words = words(&descriptor_tokens);
    if descriptor_words.starts_with(&["any", "number", "of"]) {
        count = ChoiceCount::any_number();
        descriptor_tokens = trim_commas(&descriptor_tokens[3..]);
    } else if descriptor_words.starts_with(&["up", "to"]) {
        if let Some((value, used)) = parse_number(&descriptor_tokens[2..]) {
            count = ChoiceCount::up_to(value as usize);
            descriptor_tokens = trim_commas(&descriptor_tokens[2 + used..]);
            if descriptor_tokens
                .first()
                .is_some_and(|token| token.is_word("of"))
            {
                descriptor_tokens = trim_commas(&descriptor_tokens[1..]);
            }
        } else {
            return Ok(None);
        }
    } else if descriptor_words.first() == Some(&"x") {
        count = ChoiceCount::any_number();
        descriptor_tokens = trim_commas(&descriptor_tokens[1..]);
    } else if descriptor_words
        .first()
        .is_some_and(|word| matches!(*word, "a" | "an" | "one"))
    {
        descriptor_tokens = trim_commas(&descriptor_tokens[1..]);
    } else if descriptor_words
        .first()
        .is_some_and(|word| matches!(*word, "all" | "each"))
    {
        return Ok(None);
    }

    if descriptor_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = match parse_object_filter(&descriptor_tokens, false) {
        Ok(filter) => filter,
        Err(_) => {
            let descriptor_words = words(&descriptor_tokens);
            let mut filter = ObjectFilter::default();
            let mut idx = 0usize;
            if let Some(color) = descriptor_words.get(idx).and_then(|word| parse_color(word)) {
                filter.colors = Some(color.into());
                idx += 1;
            }
            if !descriptor_words
                .get(idx)
                .is_some_and(|word| matches!(*word, "card" | "cards"))
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported reveal-hand clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            filter
        }
    };
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(PlayerFilter::You);

    let tag = TagKey::from("revealed_0");
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count,
            player: PlayerAst::You,
            tag: tag.clone(),
        },
        EffectAst::RevealTagged { tag },
    ]))
}

pub(crate) fn object_target_with_count(target: &TargetAst) -> Option<(ObjectFilter, ChoiceCount)> {
    match target {
        TargetAst::Object(filter, _, _) => Some((filter.clone(), ChoiceCount::exactly(1))),
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, _, _) => Some((filter.clone(), count.clone())),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn is_likely_named_or_source_reference_words(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }
    if is_source_reference_words(words) {
        return true;
    }
    if words.iter().any(|word| {
        matches!(
            *word,
            "then"
                | "if"
                | "unless"
                | "where"
                | "when"
                | "whenever"
                | "for"
                | "each"
                | "search"
                | "destroy"
                | "exile"
                | "draw"
                | "gain"
                | "lose"
                | "counter"
                | "put"
                | "return"
                | "create"
                | "sacrifice"
                | "deal"
                | "populate"
        )
    }) {
        return false;
    }
    !words.iter().any(|word| {
        matches!(
            *word,
            "a" | "an"
                | "the"
                | "this"
                | "that"
                | "those"
                | "it"
                | "them"
                | "target"
                | "all"
                | "any"
                | "each"
                | "another"
                | "other"
                | "up"
                | "to"
                | "card"
                | "cards"
                | "creature"
                | "creatures"
                | "permanent"
                | "permanents"
                | "artifact"
                | "artifacts"
                | "enchantment"
                | "enchantments"
                | "land"
                | "lands"
                | "planeswalker"
                | "planeswalkers"
                | "spell"
                | "spells"
        )
    })
}

pub(crate) fn parse_sentence_damage_unless_controller_has_source_deal_damage(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(unless_idx) = tokens.iter().position(|token| token.is_word("unless")) else {
        return Ok(None);
    };
    if unless_idx == 0 || unless_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let before_tokens = trim_commas(&tokens[..unless_idx]);
    if before_tokens.is_empty() {
        return Ok(None);
    }
    let effects = parse_effect_chain(&before_tokens)?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let Some(main_damage) = effects.first() else {
        return Ok(None);
    };
    let EffectAst::DealDamage {
        amount: main_amount,
        target: main_target,
    } = main_damage
    else {
        return Ok(None);
    };
    if !matches!(
        main_target,
        TargetAst::Object(_, _, _) | TargetAst::WithCount(_, _)
    ) {
        return Ok(None);
    }

    let after_unless = trim_commas(&tokens[unless_idx + 1..]);
    let after_words = words(&after_unless);
    let has_controller_clause = after_words.starts_with(&["that"])
        && after_words
            .iter()
            .any(|word| *word == "controller" || *word == "controllers");
    if !has_controller_clause {
        return Ok(None);
    }
    let Some(has_idx) = after_unless
        .iter()
        .position(|token| token.is_word("has") || token.is_word("have"))
    else {
        return Ok(None);
    };
    if has_idx + 1 >= after_unless.len() {
        return Ok(None);
    }

    let alt_tokens = &after_unless[has_idx + 1..];
    let Some(deal_idx) = alt_tokens
        .iter()
        .position(|token| token.is_word("deal") || token.is_word("deals"))
    else {
        return Ok(None);
    };
    let deal_tail = &alt_tokens[deal_idx..];
    let Some((alt_amount, used)) = parse_value(&deal_tail[1..]) else {
        return Ok(None);
    };
    if !deal_tail
        .get(1 + used)
        .is_some_and(|token| token.is_word("damage"))
    {
        return Ok(None);
    }

    let mut alt_target_tokens = &deal_tail[2 + used..];
    if alt_target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        alt_target_tokens = &alt_target_tokens[1..];
    }
    let alt_target_words = words(alt_target_tokens);
    if !matches!(alt_target_words.as_slice(), ["them"] | ["that", "player"]) {
        return Ok(None);
    }

    let alternative = EffectAst::DealDamage {
        amount: alt_amount,
        target: TargetAst::Player(
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            None,
        ),
    };
    let unless = EffectAst::UnlessAction {
        effects: vec![EffectAst::DealDamage {
            amount: main_amount.clone(),
            target: main_target.clone(),
        }],
        alternative: vec![alternative],
        player: PlayerAst::ItsController,
    };
    Ok(Some(vec![unless]))
}

pub(crate) fn parse_sentence_unless_pays(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Find "unless" in the token stream
    let unless_idx = match tokens.iter().position(|t| t.is_word("unless")) {
        Some(idx) => idx,
        None => return Ok(None),
    };

    // Leading form: "Unless you pay ..., <effects>."
    // Rewrite by parsing the effect tail after the first comma and wrapping it
    // in the parsed unless-payment clause.
    if unless_idx == 0 {
        let comma_idx = match tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        {
            Some(idx) => idx,
            None => return Ok(None),
        };
        if comma_idx + 1 >= tokens.len() {
            return Ok(None);
        }

        let effects = parse_effect_chain(&tokens[comma_idx + 1..])?;
        if effects.is_empty() {
            return Ok(None);
        }

        let unless_clause = &tokens[..comma_idx];
        if let Some(unless_effect) = try_build_unless(effects, unless_clause, 0)? {
            return Ok(Some(vec![unless_effect]));
        }
        return Ok(None);
    }

    // Need at least something before "unless" and something after.
    let before_words: Vec<&str> = tokens[..unless_idx]
        .iter()
        .filter_map(Token::as_word)
        .collect();

    // Skip "counter ... unless" - already handled by parse_counter via CounterUnlessPays
    if before_words.first() == Some(&"counter") {
        return Ok(None);
    }
    // Ignore "unless ... pays" that appears inside quoted token rules text.
    // Example: create token with "{1}, Sacrifice this token: Counter ... unless ...".
    if before_words.first() == Some(&"create")
        && before_words.contains(&"token")
        && before_words.contains(&"sacrifice")
        && before_words.contains(&"counter")
    {
        return Ok(None);
    }

    // Handle "each opponent/player ... unless" by wrapping in ForEachOpponent/ForEachPlayer.
    // Structure: ForEachOpponent { [UnlessPays/UnlessAction { per-player effects }] }
    let each_prefix = if before_words.starts_with(&["each", "opponent"])
        || before_words.starts_with(&["each", "opponents"])
    {
        Some("opponent")
    } else if before_words.starts_with(&["each", "player"]) {
        Some("player")
    } else {
        None
    };
    if let Some(prefix_kind) = each_prefix {
        // Tokens between "each opponent/player" and "unless" form the per-player effect
        let inner_token_start = tokens
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.as_word().map(|_| i))
            .nth(2) // skip "each" and "opponent"/"player"
            .unwrap_or(2);
        let inner_tokens = &tokens[inner_token_start..unless_idx];
        if let Ok(inner_effects) = parse_effect_chain(inner_tokens) {
            if !inner_effects.is_empty() {
                if let Some(unless_effect) = try_build_unless(inner_effects, tokens, unless_idx)? {
                    let wrapper = match prefix_kind {
                        "opponent" => EffectAst::ForEachOpponent {
                            effects: vec![unless_effect],
                        },
                        _ => EffectAst::ForEachPlayer {
                            effects: vec![unless_effect],
                        },
                    };
                    return Ok(Some(vec![wrapper]));
                }
            }
        }
        return Ok(None);
    }

    // Normal path: parse effects before "unless", then build unless wrapper
    let effect_tokens = &tokens[..unless_idx];
    let effects = parse_effect_chain(&effect_tokens)?;
    if effects.is_empty() {
        return Ok(None);
    }

    if let Some(unless_effect) = try_build_unless(effects, tokens, unless_idx)? {
        return Ok(Some(vec![unless_effect]));
    }

    Ok(None)
}

/// Try to build an UnlessPays or UnlessAction AST from the tokens after "unless".
/// Returns the unless wrapper containing the given `effects` as the main effects.
pub(crate) fn try_build_unless(
    effects: Vec<EffectAst>,
    tokens: &[Token],
    unless_idx: usize,
) -> Result<Option<EffectAst>, CardTextError> {
    let after_unless = &tokens[unless_idx + 1..];
    let after_words: Vec<&str> = after_unless.iter().filter_map(Token::as_word).collect();

    // Determine the player from the "unless" clause
    let (player, action_token_start) = if after_words.starts_with(&["you"]) {
        (PlayerAst::You, 1)
    } else if after_words.starts_with(&["target", "opponent"]) {
        (PlayerAst::TargetOpponent, 2)
    } else if after_words.starts_with(&["target", "player"]) {
        (PlayerAst::Target, 2)
    } else if after_words.starts_with(&["any", "player"]) {
        (PlayerAst::Any, 2)
    } else if after_words.len() >= 6
        && after_words[0] == "that"
        && matches!(
            after_words[1],
            "creature" | "creatures" | "permanent" | "permanents" | "source" | "sources"
        )
        && matches!(after_words[2], "controller" | "controllers")
        && after_words[3] == "or"
        && after_words[4] == "that"
        && after_words[5] == "player"
    {
        (PlayerAst::ItsController, 6)
    } else if after_words.len() >= 3
        && after_words[0] == "that"
        && matches!(
            after_words[1],
            "creature" | "creatures" | "permanent" | "permanents" | "source" | "sources"
        )
        && matches!(after_words[2], "controller" | "controllers")
    {
        (PlayerAst::ItsController, 3)
    } else if after_words.starts_with(&["they"]) {
        (PlayerAst::That, 1)
    } else if after_words.starts_with(&["defending", "player"]) {
        (PlayerAst::Defending, 2)
    } else if after_words.starts_with(&["that", "player"]) {
        (PlayerAst::That, 2)
    } else if after_words.starts_with(&["its", "controller"]) {
        (PlayerAst::ItsController, 2)
    } else if after_words.starts_with(&["their", "controller"]) {
        (PlayerAst::ItsController, 2)
    } else if after_words.starts_with(&["its", "owner"]) {
        (PlayerAst::ItsOwner, 2)
    } else if after_words.starts_with(&["their", "owner"]) {
        (PlayerAst::ItsOwner, 2)
    } else {
        return Ok(None);
    };

    // Find the token position corresponding to action_token_start words in
    let mut action_token_idx = 0;
    let mut wc = 0;
    for (i, token) in after_unless.iter().enumerate() {
        if token.as_word().is_some() {
            wc += 1;
            if wc == action_token_start {
                action_token_idx = i + 1;
                break;
            }
        }
    }

    let action_tokens = &after_unless[action_token_idx..];
    let action_words: Vec<&str> = action_tokens.iter().filter_map(Token::as_word).collect();

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
                .map_or(true, |token| matches!(token, Token::Period(_)))
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
        if action_words
            .windows(2)
            .any(|window| window == ["mana", "cost"])
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported unless-payment mana-cost clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }

        // Skip any non-word tokens between "pay" and mana
        let mana_start = action_tokens
            .iter()
            .skip(1)
            .position(|t| t.as_word().is_some())
            .map(|p| p + 1)
            .unwrap_or(1);
        let mana_tokens = &action_tokens[mana_start..];
        let mut mana = Vec::new();
        let mut remaining_idx = mana_tokens.len();
        for (i, token) in mana_tokens.iter().enumerate() {
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
            let remaining_words: Vec<&str> = mana_tokens[remaining_idx..]
                .iter()
                .filter_map(Token::as_word)
                .collect();

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
                    words(tokens).join(" ")
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

    Ok(None)
}

pub(crate) fn parse_sentence_fallback_mechanic_marker(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    let is_match = clause_words.as_slice() == ["venture", "into", "the", "dungeon"]
        || clause_words.as_slice() == ["the", "ring", "tempts", "you"]
        || clause_words.as_slice() == ["its", "still", "a", "land"]
        || clause_words.as_slice() == ["it", "still", "a", "land"]
        || clause_words.starts_with(&["manifest", "the", "top", "card", "of", "your", "library"])
        || clause_words.starts_with(&["you", "choose", "one", "of", "them"])
        || clause_words.starts_with(&[
            "you", "may", "put", "a", "land", "card", "from", "among", "them", "into", "your",
            "hand",
        ])
        || clause_words.starts_with(&["stand", "and", "fight"])
        || clause_words.starts_with(&[
            "chooses",
            "any",
            "number",
            "of",
            "creatures",
            "they",
            "control",
        ])
        || clause_words.starts_with(&[
            "each",
            "player",
            "chooses",
            "any",
            "number",
            "of",
            "creatures",
            "they",
            "control",
        ])
        || clause_words.starts_with(&["an", "opponent", "chooses", "one", "of", "those", "piles"])
        || clause_words.starts_with(&["put", "that", "pile", "into", "your", "hand"])
        || clause_words.starts_with(&["cast", "that", "card", "for", "as", "long", "as"])
        || clause_words.starts_with(&[
            "until", "end", "of", "turn", "this", "creature", "loses", "prevent", "all", "damage",
        ])
        || clause_words.starts_with(&[
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
        ])
        || clause_words.starts_with(&[
            "it", "becomes", "an", "angel", "in", "addition", "to", "its", "other", "types",
        ])
        || clause_words.starts_with(&["for", "each", "1", "damage", "prevented", "this", "way"])
        || clause_words.starts_with(&[
            "for", "each", "card", "less", "than", "two", "a", "player", "draws", "this", "way",
        ])
        || clause_words.starts_with(&["this", "deals", "4", "damage", "if", "there", "are"])
        || clause_words.starts_with(&[
            "this", "deals", "4", "damage", "instead", "if", "there", "are",
        ])
        || clause_words.starts_with(&[
            "that", "spell", "deals", "damage", "to", "each", "opponent", "equal", "to",
        ])
        || clause_words.starts_with(&[
            "the", "next", "spell", "you", "cast", "this", "turn", "costs",
        ])
        || clause_words.starts_with(&[
            "there",
            "is",
            "an",
            "additional",
            "combat",
            "phase",
            "after",
            "this",
            "phase",
        ])
        || clause_words.starts_with(&[
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
        ])
        || clause_words.starts_with(&[
            "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to", "target",
            "creature", "you", "control", "by", "a", "source", "of", "your", "choice", "is",
            "dealt", "to", "another", "target", "creature", "instead",
        ])
        || (clause_words.starts_with(&["it", "doesnt", "untap", "during"])
            && clause_words.contains(&"remains")
            && clause_words.contains(&"tapped"));
    if !is_match {
        return Ok(None);
    }
    Err(CardTextError::ParseError(format!(
        "unsupported mechanic marker clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) const PRE_CONDITIONAL_SENTENCE_PRIMITIVES: &[SentencePrimitive] = &[
    SentencePrimitive {
        name: "fallback-mechanic-marker",
        parser: parse_sentence_fallback_mechanic_marker,
    },
    SentencePrimitive {
        name: "if-enters-with-additional-counter",
        parser: parse_if_enters_with_additional_counter_sentence,
    },
    SentencePrimitive {
        name: "put-multiple-counters-on-target",
        parser: parse_sentence_put_multiple_counters_on_target,
    },
    SentencePrimitive {
        name: "you-and-target-player-each-draw",
        parser: parse_sentence_you_and_target_player_each_draw,
    },
    SentencePrimitive {
        name: "you-and-attacking-player-each-draw-and-lose",
        parser: parse_sentence_you_and_attacking_player_each_draw_and_lose,
    },
    SentencePrimitive {
        name: "sacrifice-it-next-end-step",
        parser: parse_sentence_sacrifice_it_next_end_step,
    },
    SentencePrimitive {
        name: "sacrifice-at-end-of-combat",
        parser: parse_sentence_sacrifice_at_end_of_combat,
    },
    SentencePrimitive {
        name: "each-player-choose-keep-rest-sacrifice",
        parser: parse_sentence_each_player_choose_and_sacrifice_rest,
    },
    SentencePrimitive {
        name: "target-player-choose-then-put-on-top-library",
        parser: parse_sentence_target_player_chooses_then_puts_on_top_of_library,
    },
    SentencePrimitive {
        name: "target-player-choose-then-you-put-it-onto-battlefield",
        parser: parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield,
    },
    SentencePrimitive {
        name: "exile-instead-of-graveyard",
        parser: parse_sentence_exile_instead_of_graveyard,
    },
];

pub(crate) const POST_CONDITIONAL_SENTENCE_PRIMITIVES: &[SentencePrimitive] = &[
    SentencePrimitive {
        name: "exile-target-creature-with-greatest-power",
        parser: parse_sentence_exile_target_creature_with_greatest_power,
    },
    SentencePrimitive {
        name: "counter-target-spell-thats-second-cast-this-turn",
        parser: parse_sentence_counter_target_spell_thats_second_cast_this_turn,
    },
    SentencePrimitive {
        name: "counter-target-spell-if-it-was-kicked",
        parser: parse_sentence_counter_target_spell_if_it_was_kicked,
    },
    SentencePrimitive {
        name: "destroy-creature-type-of-choice",
        parser: parse_sentence_destroy_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "pump-creature-type-of-choice",
        parser: parse_sentence_pump_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "return-multiple-targets",
        parser: parse_sentence_return_multiple_targets,
    },
    SentencePrimitive {
        name: "for-each-of-target-objects",
        parser: parse_sentence_for_each_of_target_objects,
    },
    SentencePrimitive {
        name: "return-creature-type-of-choice",
        parser: parse_sentence_return_targets_of_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "distribute-counters",
        parser: parse_sentence_distribute_counters,
    },
    SentencePrimitive {
        name: "keyword-then-chain",
        parser: parse_sentence_keyword_then_chain,
    },
    SentencePrimitive {
        name: "chain-then-keyword",
        parser: parse_sentence_chain_then_keyword,
    },
    SentencePrimitive {
        name: "exile-then-may-put-from-exile",
        parser: parse_sentence_exile_then_may_put_from_exile,
    },
    SentencePrimitive {
        name: "exile-source-with-counters",
        parser: parse_sentence_exile_source_with_counters,
    },
    SentencePrimitive {
        name: "destroy-all-attached-to-target",
        parser: parse_sentence_destroy_all_attached_to_target,
    },
    SentencePrimitive {
        name: "comma-then-chain-special",
        parser: parse_sentence_comma_then_chain_special,
    },
    SentencePrimitive {
        name: "destroy-then-land-controller-graveyard-count-damage",
        parser: parse_sentence_destroy_then_land_controller_graveyard_count_damage,
    },
    SentencePrimitive {
        name: "draw-then-connive",
        parser: parse_sentence_draw_then_connive,
    },
    SentencePrimitive {
        name: "return-then-do-same-for-subtypes",
        parser: parse_sentence_return_then_do_same_for_subtypes,
    },
    SentencePrimitive {
        name: "return-then-create",
        parser: parse_sentence_return_then_create,
    },
    SentencePrimitive {
        name: "put-counter-sequence",
        parser: parse_sentence_put_counter_sequence,
    },
    SentencePrimitive {
        name: "gets-then-fights",
        parser: parse_sentence_gets_then_fights,
    },
    SentencePrimitive {
        name: "return-with-counters-on-it",
        parser: parse_sentence_return_with_counters_on_it,
    },
    SentencePrimitive {
        name: "each-player-return-with-additional-counter",
        parser: parse_sentence_each_player_return_with_additional_counter,
    },
    SentencePrimitive {
        name: "sacrifice-any-number",
        parser: parse_sentence_sacrifice_any_number,
    },
    SentencePrimitive {
        name: "sacrifice-one-or-more",
        parser: parse_sentence_sacrifice_one_or_more,
    },
    SentencePrimitive {
        name: "monstrosity",
        parser: parse_sentence_monstrosity,
    },
    SentencePrimitive {
        name: "for-each-counter-removed",
        parser: parse_sentence_for_each_counter_removed,
    },
    SentencePrimitive {
        name: "for-each-counter-kind-put-or-remove",
        parser: parse_sentence_for_each_counter_kind_put_or_remove,
    },
    SentencePrimitive {
        name: "take-extra-turn",
        parser: parse_sentence_take_extra_turn,
    },
    SentencePrimitive {
        name: "earthbend",
        parser: parse_sentence_earthbend,
    },
    SentencePrimitive {
        name: "enchant",
        parser: parse_sentence_enchant,
    },
    SentencePrimitive {
        name: "cant-effect",
        parser: parse_sentence_cant_effect,
    },
    SentencePrimitive {
        name: "prevent-damage",
        parser: parse_sentence_prevent_damage,
    },
    SentencePrimitive {
        name: "shared-color-target-fanout",
        parser: parse_sentence_shared_color_target_fanout,
    },
    SentencePrimitive {
        name: "gain-ability-to-source",
        parser: parse_sentence_gain_ability_to_source,
    },
    SentencePrimitive {
        name: "gain-ability",
        parser: parse_sentence_gain_ability,
    },
    SentencePrimitive {
        name: "vote-with-you",
        parser: parse_sentence_you_and_each_opponent_voted_with_you,
    },
    SentencePrimitive {
        name: "gain-life-equal-to-power",
        parser: parse_sentence_gain_life_equal_to_power,
    },
    SentencePrimitive {
        name: "gain-x-plus-life",
        parser: parse_sentence_gain_x_plus_life,
    },
    SentencePrimitive {
        name: "for-each-exiled-this-way",
        parser: parse_sentence_for_each_exiled_this_way,
    },
    SentencePrimitive {
        name: "each-player-put-permanent-cards-exiled-with-source",
        parser: parse_sentence_each_player_put_permanent_cards_exiled_with_source,
    },
    SentencePrimitive {
        name: "for-each-destroyed-this-way",
        parser: parse_sentence_for_each_destroyed_this_way,
    },
    SentencePrimitive {
        name: "exile-then-return-same-object",
        parser: parse_sentence_exile_then_return_same_object,
    },
    SentencePrimitive {
        name: "search-library",
        parser: parse_sentence_search_library,
    },
    SentencePrimitive {
        name: "shuffle-graveyard-into-library",
        parser: parse_sentence_shuffle_graveyard_into_library,
    },
    SentencePrimitive {
        name: "shuffle-object-into-library",
        parser: parse_sentence_shuffle_object_into_library,
    },
    SentencePrimitive {
        name: "exile-hand-and-graveyard-bundle",
        parser: parse_sentence_exile_hand_and_graveyard_bundle,
    },
    SentencePrimitive {
        name: "target-player-exiles-creature-and-graveyard",
        parser: parse_sentence_target_player_exiles_creature_and_graveyard,
    },
    SentencePrimitive {
        name: "play-from-graveyard",
        parser: parse_sentence_play_from_graveyard,
    },
    SentencePrimitive {
        name: "look-at-top-then-exile-one",
        parser: parse_sentence_look_at_top_then_exile_one,
    },
    SentencePrimitive {
        name: "look-at-hand",
        parser: parse_sentence_look_at_hand,
    },
    SentencePrimitive {
        name: "gain-life-equal-to-age",
        parser: parse_sentence_gain_life_equal_to_age,
    },
    SentencePrimitive {
        name: "for-each-player-doesnt",
        parser: parse_sentence_for_each_player_doesnt,
    },
    SentencePrimitive {
        name: "for-each-opponent-doesnt",
        parser: parse_sentence_for_each_opponent_doesnt,
    },
    SentencePrimitive {
        name: "each-opponent-loses-x-and-you-gain-x",
        parser: parse_sentence_each_opponent_loses_x_and_you_gain_x,
    },
    SentencePrimitive {
        name: "vote-start",
        parser: parse_sentence_vote_start,
    },
    SentencePrimitive {
        name: "for-each-vote-clause",
        parser: parse_sentence_for_each_vote_clause,
    },
    SentencePrimitive {
        name: "vote-extra",
        parser: parse_sentence_vote_extra,
    },
    SentencePrimitive {
        name: "after-turn",
        parser: parse_sentence_after_turn,
    },
    SentencePrimitive {
        name: "same-name-target-fanout",
        parser: parse_sentence_same_name_target_fanout,
    },
    SentencePrimitive {
        name: "same-name-gets-fanout",
        parser: parse_sentence_same_name_gets_fanout,
    },
    SentencePrimitive {
        name: "delayed-next-end-step",
        parser: parse_sentence_delayed_until_next_end_step,
    },
    SentencePrimitive {
        name: "delayed-trigger-this-turn",
        parser: parse_sentence_delayed_trigger_this_turn,
    },
    SentencePrimitive {
        name: "delayed-when-that-dies-this-turn",
        parser: parse_delayed_when_that_dies_this_turn_sentence,
    },
    SentencePrimitive {
        name: "destroy-or-exile-all-split",
        parser: parse_sentence_destroy_or_exile_all_split,
    },
    SentencePrimitive {
        name: "exile-up-to-one-each-target-type",
        parser: parse_sentence_exile_up_to_one_each_target_type,
    },
    SentencePrimitive {
        name: "exile-multi-target",
        parser: parse_sentence_exile_multi_target,
    },
    SentencePrimitive {
        name: "destroy-multi-target",
        parser: parse_sentence_destroy_multi_target,
    },
    SentencePrimitive {
        name: "reveal-selected-cards-in-your-hand",
        parser: parse_sentence_reveal_selected_cards_in_your_hand,
    },
    SentencePrimitive {
        name: "damage-unless-controller-has-source-deal-damage",
        parser: parse_sentence_damage_unless_controller_has_source_deal_damage,
    },
    SentencePrimitive {
        name: "unless-pays",
        parser: parse_sentence_unless_pays,
    },
];
