use super::*;

pub(crate) fn parse_simple_ability_duration(words_after_verb: &[&str]) -> Option<(usize, usize, Until)> {
    if let Some(idx) = words_after_verb
        .windows(4)
        .position(|window| window == ["until", "end", "of", "turn"])
    {
        return Some((idx, 4, Until::EndOfTurn));
    }
    if let Some(idx) = words_after_verb.windows(4).position(|window| {
        window == ["until", "your", "next", "turn"] || window == ["until", "your", "next", "upkeep"]
    }) {
        return Some((idx, 4, Until::YourNextTurn));
    }
    if let Some(idx) = words_after_verb.windows(5).position(|window| {
        window == ["until", "your", "next", "untap", "step"]
            || window == ["during", "your", "next", "untap", "step"]
    }) {
        return Some((idx, 5, Until::YourNextTurn));
    }
    if let Some(idx) = words_after_verb
        .windows(6)
        .position(|window| window == ["for", "as", "long", "as", "you", "control"])
    {
        return Some((
            idx,
            words_after_verb.len().saturating_sub(idx),
            Until::YouStopControllingThis,
        ));
    }
    None
}

pub(crate) fn parse_simple_gain_ability_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause(tokens, false)
}

pub(crate) fn parse_simple_lose_ability_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause(tokens, true)
}

pub(crate) fn parse_simple_ability_modifier_clause(
    tokens: &[Token],
    losing: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let verb_idx = clause_words.iter().position(|word| {
        if losing {
            matches!(*word, "lose" | "loses")
        } else {
            matches!(*word, "gain" | "gains")
        }
    });
    let Some(verb_idx) = verb_idx else {
        return Ok(None);
    };
    if verb_idx == 0 {
        return Ok(None);
    }
    let Some(verb_token_idx) = token_index_for_word_index(tokens, verb_idx) else {
        return Ok(None);
    };

    if !losing && matches!(clause_words[verb_idx], "gain" | "gains") {
        let starts_with_life = clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| *word == "life");
        let starts_with_control = clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| *word == "control");
        if starts_with_life || starts_with_control {
            return Ok(None);
        }
    }

    let subject_tokens = trim_commas(&tokens[..verb_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    if !losing
        && let Some((subject_verb, _)) = find_verb(&subject_tokens)
        && subject_verb != Verb::Get
    {
        return Ok(None);
    }

    let words_after_verb = &clause_words[verb_idx + 1..];
    if words_after_verb.is_empty() {
        return Ok(None);
    }

    let duration_phrase = parse_simple_ability_duration(words_after_verb);
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .unwrap_or(Until::Forever);

    let ability_end_word_idx = duration_phrase
        .as_ref()
        .map(|(start, _, _)| verb_idx + 1 + *start)
        .unwrap_or(clause_words.len());
    let ability_end_token_idx =
        token_index_for_word_index(tokens, ability_end_word_idx).unwrap_or(tokens.len());
    let ability_tokens = trim_commas(&tokens[verb_token_idx + 1..ability_end_token_idx]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let mut abilities = if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        actions
            .into_iter()
            .filter_map(keyword_action_to_static_ability)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if abilities.is_empty()
        && let Some(granted) =
            parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, &clause_words)?
    {
        abilities.push(granted);
    }
    if abilities.is_empty() {
        return Ok(None);
    }

    if let Some((start, len, _)) = duration_phrase {
        let tail_word_idx = verb_idx + 1 + start + len;
        if let Some(tail_token_idx) = token_index_for_word_index(tokens, tail_word_idx) {
            let trailing = trim_commas(&tokens[tail_token_idx..]);
            if !trailing.is_empty() {
                return Ok(None);
            }
        }
    }

    let subject_words = words(&subject_tokens);
    let is_pronoun_subject = matches!(subject_words.as_slice(), ["it"] | ["they"] | ["them"]);
    if is_pronoun_subject {
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&subject_tokens));
        if losing {
            return Ok(Some(EffectAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            }));
        }
        return Ok(Some(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        }));
    }

    let is_demonstrative_subject = subject_words
        .first()
        .is_some_and(|word| *word == "that" || *word == "those");
    if is_demonstrative_subject || subject_words.contains(&"target") {
        let target = parse_target_phrase(&subject_tokens)?;
        if losing {
            return Ok(Some(EffectAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            }));
        }
        return Ok(Some(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        }));
    }

    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in {}-ability clause (clause: '{}')",
            if losing { "lose" } else { "gain" },
            clause_words.join(" ")
        ))
    })?;
    if losing {
        return Ok(Some(EffectAst::RemoveAbilitiesAll {
            filter,
            abilities,
            duration,
        }));
    }
    Ok(Some(EffectAst::GrantAbilitiesAll {
        filter,
        abilities,
        duration,
    }))
}

pub(crate) fn parse_gain_ability_sentence(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let word_list = words(tokens);
    let looks_like_can_attack_no_defender = word_list
        .windows(2)
        .any(|window| window == ["can", "attack"])
        && word_list
            .windows(2)
            .any(|window| window == ["as", "though"])
        && word_list.contains(&"defender");
    if looks_like_can_attack_no_defender {
        return Ok(None);
    }
    let gain_idx = word_list
        .iter()
        .position(|word| matches!(*word, "gain" | "gains" | "has" | "have"));
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };
    let Some(gain_token_idx) = token_index_for_word_index(tokens, gain_idx) else {
        return Ok(None);
    };

    let after_gain = &word_list[gain_idx + 1..];
    if matches!(word_list[gain_idx], "gain" | "gains") {
        let starts_with_life = after_gain.first().is_some_and(|word| *word == "life");
        let starts_with_control = after_gain.first().is_some_and(|word| *word == "control");
        if starts_with_life || starts_with_control {
            return Ok(None);
        }
    }

    let leading_duration_phrase = if word_list.starts_with(&["until", "end", "of", "turn"]) {
        Some((4usize, Until::EndOfTurn))
    } else if word_list.starts_with(&["until", "your", "next", "turn"])
        || word_list.starts_with(&["until", "your", "next", "upkeep"])
    {
        Some((4usize, Until::YourNextTurn))
    } else if word_list.starts_with(&["until", "your", "next", "untap", "step"])
        || word_list.starts_with(&["during", "your", "next", "untap", "step"])
    {
        Some((5usize, Until::YourNextTurn))
    } else {
        None
    };
    let subject_start_word_idx = leading_duration_phrase
        .as_ref()
        .map(|(len, _)| *len)
        .unwrap_or(0);
    let subject_start_token_idx = if subject_start_word_idx == 0 {
        0usize
    } else if let Some(idx) = token_index_for_word_index(tokens, subject_start_word_idx) {
        idx
    } else {
        return Ok(None);
    };
    if subject_start_token_idx < gain_token_idx
        && let Some((subject_verb, _)) = find_verb(&tokens[subject_start_token_idx..gain_token_idx])
        && subject_verb != Verb::Get
    {
        return Ok(None);
    }

    let duration_phrase = if let Some(idx) = after_gain
        .windows(4)
        .position(|window| window == ["until", "end", "of", "turn"])
    {
        Some((idx, 4usize, Until::EndOfTurn))
    } else if let Some(idx) = after_gain.windows(4).position(|window| {
        window == ["until", "your", "next", "turn"] || window == ["until", "your", "next", "upkeep"]
    }) {
        Some((idx, 4usize, Until::YourNextTurn))
    } else if let Some(idx) = after_gain.windows(5).position(|window| {
        window == ["until", "your", "next", "untap", "step"]
            || window == ["during", "your", "next", "untap", "step"]
    }) {
        Some((idx, 5usize, Until::YourNextTurn))
    } else if let Some(idx) = after_gain
        .windows(6)
        .position(|window| window == ["for", "as", "long", "as", "you", "control"])
    {
        // Consume the remainder of the phrase as the duration clause.
        Some((
            idx,
            after_gain.len().saturating_sub(idx),
            Until::YouStopControllingThis,
        ))
    } else {
        None
    };
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .or_else(|| {
            leading_duration_phrase
                .as_ref()
                .map(|(_, duration)| duration.clone())
        })
        .unwrap_or(Until::Forever);
    let has_explicit_duration =
        duration_phrase.is_some() || leading_duration_phrase.as_ref().is_some();

    let mut trailing_tail_tokens: Vec<Token> = Vec::new();
    if let Some((start_rel, len_words, _)) = duration_phrase {
        let tail_word_idx = gain_idx + 1 + start_rel + len_words;
        if let Some(tail_token_idx) = token_index_for_word_index(tokens, tail_word_idx) {
            let mut tail_tokens = trim_commas(&tokens[tail_token_idx..]).to_vec();
            while tail_tokens
                .first()
                .is_some_and(|token| token.is_word("and") || token.is_word("then"))
            {
                tail_tokens.remove(0);
            }
            if !tail_tokens.is_empty() {
                trailing_tail_tokens = tail_tokens;
            }
        }
    }
    let mut grants_must_attack = false;
    if !trailing_tail_tokens.is_empty() {
        let mut tail_words = words(&trailing_tail_tokens);
        if tail_words.first().is_some_and(|word| *word == "and") {
            tail_words = tail_words[1..].to_vec();
        }
        if tail_words.as_slice() == ["attacks", "this", "combat", "if", "able"]
            || tail_words.as_slice() == ["attack", "this", "combat", "if", "able"]
        {
            grants_must_attack = true;
            trailing_tail_tokens.clear();
        }
    }

    let ability_end_word_idx = duration_phrase
        .as_ref()
        .map(|(start_rel, _, _)| gain_idx + 1 + *start_rel);
    let ability_end_token_idx = if let Some(end_word_idx) = ability_end_word_idx {
        token_index_for_word_index(tokens, end_word_idx).unwrap_or(tokens.len())
    } else {
        tokens.len()
    };
    let ability_start_token_idx = gain_token_idx + 1;
    if ability_start_token_idx > ability_end_token_idx || ability_start_token_idx >= tokens.len() {
        return Ok(None);
    }
    let ability_tokens = trim_commas(&tokens[ability_start_token_idx..ability_end_token_idx]);

    let mut grant_is_choice = false;
    let mut abilities = if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &word_list.join(" "))?;
        actions
            .into_iter()
            .filter_map(keyword_action_to_static_ability)
            .collect::<Vec<_>>()
    } else if let Some(actions) = parse_choice_of_abilities(&ability_tokens) {
        grant_is_choice = true;
        reject_unimplemented_keyword_actions(&actions, &word_list.join(" "))?;
        actions
            .into_iter()
            .filter_map(keyword_action_to_static_ability)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if abilities.is_empty()
        && let Some(granted) =
            parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, &word_list)?
    {
        abilities.push(granted);
    }
    if abilities.is_empty() && !grants_must_attack {
        return Ok(None);
    }
    if grants_must_attack {
        abilities.push(StaticAbility::must_attack());
    }

    // Check for "gets +X/+Y and gains/has ..." pattern - if there's a pump modifier
    // before the granting verb, extract it as a separate Pump/PumpAll effect.
    let before_gain = &word_list[subject_start_word_idx..gain_idx];
    let get_idx = before_gain.iter().position(|w| *w == "get" || *w == "gets");
    let pump_effect = if let Some(gi) = get_idx {
        let mod_word = before_gain.get(gi + 1).copied().unwrap_or("");
        if let Ok((power, toughness)) = parse_pt_modifier_values(mod_word) {
            Some((power, toughness, subject_start_word_idx + gi))
        } else {
            None
        }
    } else {
        None
    };
    let has_have_verb = matches!(word_list[gain_idx], "has" | "have");
    if has_have_verb && pump_effect.is_none() && !has_explicit_duration {
        return Ok(None);
    }

    // Determine the real subject (before "get"/"gets" if pump is present)
    let real_subject_end_word_idx = pump_effect
        .as_ref()
        .map(|(_, _, gi)| *gi)
        .unwrap_or(gain_idx);
    let real_subject_end_token_idx =
        token_index_for_word_index(tokens, real_subject_end_word_idx).unwrap_or(gain_token_idx);
    if subject_start_token_idx >= real_subject_end_token_idx {
        return Ok(None);
    }
    let real_subject_tokens =
        trim_commas(&tokens[subject_start_token_idx..real_subject_end_token_idx]);

    let mut effects = Vec::new();

    // Check for pronoun subjects ("it", "they") that reference a prior tagged object.
    let real_subject_words: Vec<&str> = real_subject_tokens
        .iter()
        .filter_map(Token::as_word)
        .collect();
    let is_pronoun_subject =
        real_subject_words.as_slice() == ["it"] || real_subject_words.as_slice() == ["they"];
    if is_pronoun_subject {
        let span = span_from_tokens(&real_subject_tokens);
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span);
        if let Some((power, toughness, _)) = pump_effect {
            effects.push(EffectAst::Pump {
                power,
                toughness,
                target: target.clone(),
                duration: duration.clone(),
                condition: None,
            });
        }
        if grant_is_choice {
            effects.push(EffectAst::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            });
        } else {
            effects.push(EffectAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
            });
        }
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let is_demonstrative_subject = real_subject_words
        .first()
        .is_some_and(|word| *word == "that" || *word == "those");
    if is_demonstrative_subject {
        let target = parse_target_phrase(&real_subject_tokens)?;
        if let Some((power, toughness, _)) = pump_effect {
            effects.push(EffectAst::Pump {
                power,
                toughness,
                target: target.clone(),
                duration: duration.clone(),
                condition: None,
            });
        }
        if grant_is_choice {
            effects.push(EffectAst::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            });
        } else {
            effects.push(EffectAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
            });
        }
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if before_gain.contains(&"target") {
        let has_pump_effect = pump_effect.is_some();
        let target = parse_target_phrase(&real_subject_tokens)?;
        if let Some((power, toughness, _)) = pump_effect {
            effects.push(EffectAst::Pump {
                power,
                toughness,
                target: target.clone(),
                duration: duration.clone(),
                condition: None,
            });
        }
        let grant_target = if has_pump_effect {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&real_subject_tokens))
        } else {
            target
        };
        if grant_is_choice {
            effects.push(EffectAst::GrantAbilitiesChoiceToTarget {
                target: grant_target,
                abilities,
                duration,
            });
        } else {
            effects.push(EffectAst::GrantAbilitiesToTarget {
                target: grant_target,
                abilities,
                duration,
            });
        }
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let filter = parse_object_filter(&real_subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in gain-ability clause (clause: '{}')",
            word_list.join(" ")
        ))
    })?;

    if let Some((power, toughness, _)) = pump_effect {
        effects.push(EffectAst::PumpAll {
            filter: filter.clone(),
            power,
            toughness,
            duration: duration.clone(),
        });
    }
    if grant_is_choice {
        effects.push(EffectAst::GrantAbilitiesChoiceAll {
            filter,
            abilities,
            duration,
        });
    } else {
        effects.push(EffectAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
        });
    }
    effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;

    Ok(Some(effects))
}

pub(crate) fn parse_granted_activated_or_triggered_ability_for_gain(
    ability_tokens: &[Token],
    clause_words: &[&str],
) -> Result<Option<StaticAbility>, CardTextError> {
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let has_colon = ability_tokens
        .iter()
        .any(|token| matches!(token, Token::Colon(_)));
    let looks_like_trigger = ability_tokens.first().is_some_and(|token| {
        token.is_word("when")
            || token.is_word("whenever")
            || (token.is_word("at")
                && ability_tokens
                    .get(1)
                    .is_some_and(|next| next.is_word("the")))
    });
    if !has_colon && !looks_like_trigger {
        return Ok(None);
    }

    let mut ability = if has_colon {
        let Some(parsed) = parse_activated_line(ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        parsed.ability
    } else {
        match parse_triggered_line(ability_tokens)? {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => {
                let (compiled_effects, choices) =
                    compile_trigger_effects(Some(&trigger), &effects)?;
                Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        trigger: compile_trigger_spec(trigger),
                        effects: compiled_effects,
                        choices,
                        intervening_if: max_triggers_per_turn
                            .map(crate::ConditionExpr::MaxTimesEachTurn),
                    }),
                    functional_zones: vec![Zone::Battlefield],
                    text: None,
                }
            }
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated/triggered ability clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    };

    if ability.text.is_none() {
        ability.text = Some(words(ability_tokens).join(" "));
    }

    Ok(Some(StaticAbility::grant_object_ability_for_filter(
        ObjectFilter::source(),
        ability,
        words(ability_tokens).join(" "),
    )))
}

pub(crate) fn append_gain_ability_trailing_effects(
    mut effects: Vec<EffectAst>,
    trailing_tokens: &[Token],
) -> Result<Vec<EffectAst>, CardTextError> {
    if trailing_tokens.is_empty() {
        return Ok(effects);
    }

    let trimmed = trim_commas(trailing_tokens);
    if trimmed.first().is_some_and(|token| token.is_word("unless")) {
        if let Some(unless_effect) = try_build_unless(effects, &trimmed, 0)? {
            return Ok(vec![unless_effect]);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing unless gain-ability clause (clause: '{}')",
            words(&trimmed).join(" ")
        )));
    }

    if let Ok(parsed_tail) = parse_effect_chain(&trimmed)
        && !parsed_tail.is_empty()
    {
        effects.extend(parsed_tail);
    }
    Ok(effects)
}

pub(crate) fn parse_choice_of_abilities(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let tokens = trim_commas(tokens);
    let words = words(&tokens);
    let prefix_words = if words.starts_with(&["your", "choice", "of"]) {
        3usize
    } else if words.starts_with(&["your", "choice", "from"]) {
        3usize
    } else {
        return None;
    };
    if words.len() <= prefix_words + 1 {
        return None;
    }

    let start_idx = token_index_for_word_index(&tokens, prefix_words)?;
    let option_tokens = trim_commas(&tokens[start_idx..]);
    if option_tokens.is_empty() {
        return None;
    }

    let mut actions = Vec::new();
    for segment in split_on_or(&option_tokens) {
        let segment = trim_commas(&segment);
        if segment.is_empty() {
            continue;
        }
        let action = parse_ability_phrase(&segment)?;
        if !actions.contains(&action) {
            actions.push(action);
        }
    }

    if actions.len() < 2 {
        return None;
    }
    Some(actions)
}

pub(crate) fn parse_gain_ability_to_source_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let gain_idx = clause_words
        .iter()
        .position(|word| *word == "gain" || *word == "gains");
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };

    let subject_tokens = &tokens[..gain_idx];
    let subject_words: Vec<&str> = words(subject_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !is_source_reference_words(&subject_words) {
        return Ok(None);
    }

    let ability_tokens = &tokens[gain_idx + 1..];
    if let Some(ability) = parse_activated_line(ability_tokens)? {
        return Ok(Some(EffectAst::GrantAbilityToSource {
            ability: ability.ability,
        }));
    }

    Ok(None)
}
