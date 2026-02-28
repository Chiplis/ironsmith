use super::*;

pub(crate) fn parse_effect_clause(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }

    if let Some(player) = parse_leading_player_may(tokens) {
        let mut stripped = remove_through_first_word(tokens, "may");
        if stripped
            .first()
            .is_some_and(|token| token.is_word("have") || token.is_word("has"))
        {
            stripped.remove(0);
        }
        let mut effects = parse_effect_chain_with_sentence_primitives(&stripped)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        return Ok(EffectAst::MayByPlayer { player, effects });
    }

    if tokens.first().is_some_and(|token| token.is_word("may")) {
        let stripped = remove_first_word(tokens, "may");
        let effects = parse_effect_chain_with_sentence_primitives(&stripped)?;
        return Ok(EffectAst::May { effects });
    }

    let clause_words = words(tokens);
    if clause_words
        .iter()
        .any(|word| *word == "choose" || *word == "chooses")
        && clause_words.contains(&"creature")
        && clause_words.contains(&"type")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported creature-type choice clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_mana_replacement_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana replacement clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if is_mana_trigger_additional_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-triggered additional-mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(effect) = run_clause_primitives(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_has_base_power_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_has_base_power_toughness_clause(tokens)? {
        return Ok(effect);
    }

    if let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(tokens)?
    {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    if let Some((chooser, choose_filter, choose_count)) = parse_you_choose_objects_clause(tokens)? {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    // "This creature assigns no combat damage this turn."
    // Used in Laccolith-style effects: "If you do, this creature assigns no combat damage this turn."
    if clause_words
        .windows(4)
        .any(|window| window == ["assigns", "no", "combat", "damage"])
    {
        let assigns_idx = tokens
            .iter()
            .position(|token| token.is_word("assigns") || token.is_word("assign"))
            .unwrap_or(0);
        let subject_tokens = trim_commas(&tokens[..assigns_idx]);
        let tail_tokens = trim_commas(&tokens[assigns_idx + 1..]);
        let tail_words = words(&tail_tokens);
        if !tail_words.starts_with(&["no", "combat", "damage"]) {
            return Err(CardTextError::ParseError(format!(
                "unsupported assigns-no-combat-damage clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut idx = 3usize;
        if tail_words.get(idx) == Some(&"this") && tail_words.get(idx + 1) == Some(&"turn") {
            idx += 2;
        } else if tail_words.get(idx) == Some(&"this") && tail_words.get(idx + 1) == Some(&"combat")
        {
            idx += 2;
        }
        if idx != tail_words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported assigns-no-combat-damage clause tail (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let subject_words = words(&subject_tokens);
        let source = if subject_words.is_empty()
            || matches!(
                subject_words.as_slice(),
                ["it"] | ["this"] | ["this", "creature"]
            ) {
            TargetAst::Source(None)
        } else {
            parse_target_phrase(&subject_tokens)?
        };

        return Ok(EffectAst::PreventAllCombatDamageFromSource {
            duration: Until::EndOfTurn,
            source,
        });
    }

    if tokens.first().is_some_and(|token| token.is_word("target")) && find_verb(tokens).is_none() {
        let clause_words = words(tokens);
        let looks_like_restriction_clause = find_negation_span(tokens).is_some()
            || clause_words.contains(&"blocked")
            || clause_words.contains(&"except")
            || clause_words.contains(&"unless")
            || clause_words.contains(&"attack")
            || clause_words.contains(&"attacks")
            || clause_words.contains(&"block")
            || clause_words.contains(&"blocks");
        if looks_like_restriction_clause {
            return Err(CardTextError::ParseError(format!(
                "unsupported target-only restriction clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target = parse_target_phrase(tokens)?;
        return Ok(EffectAst::TargetOnly { target });
    }

    let (verb, verb_idx) = find_verb(tokens).ok_or_else(|| {
        let clause = words(tokens).join(" ");
        let known_verbs = [
            "add",
            "move",
            "deal",
            "draw",
            "counter",
            "destroy",
            "exile",
            "untap",
            "scry",
            "discard",
            "transform",
            "regenerate",
            "mill",
            "get",
            "reveal",
            "look",
            "lose",
            "gain",
            "put",
            "sacrifice",
            "create",
            "investigate",
            "attach",
            "remove",
            "return",
            "exchange",
            "become",
            "switch",
            "skip",
            "surveil",
            "shuffle",
            "reorder",
            "pay",
            "goad",
        ];
        CardTextError::ParseError(format!(
            "could not find verb in effect clause (clause: '{clause}'; known verbs: {})",
            known_verbs.join(", ")
        ))
    })?;
    parser_trace_stack("parse_effect_clause:verb-found", tokens);

    if matches!(verb, Verb::Counter)
        && verb_idx > 0
        && tokens.iter().any(|token| token.is_word("on"))
    {
        if let Ok(effect) = parse_put_counters(tokens) {
            parser_trace("parse_effect_clause:counter-noun-treated-as-put", tokens);
            return Ok(effect);
        }
    }

    if matches!(verb, Verb::Get) {
        let subject_tokens = &tokens[..verb_idx];
        if !subject_tokens.is_empty() {
            let subject_words = words(subject_tokens);
            if let Some(mod_token) = tokens.get(verb_idx + 1).and_then(Token::as_word)
                && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
            {
                let modifier_tail = &tokens[verb_idx + 1..];
                if let Some(count) = parse_get_for_each_count_value(modifier_tail)? {
                    let modifier_words = words(modifier_tail);
                    let duration = if modifier_words.starts_with(&["until", "end", "of", "turn"])
                        || modifier_words
                            .windows(4)
                            .any(|window| window == ["until", "end", "of", "turn"])
                    {
                        Until::EndOfTurn
                    } else {
                        Until::EndOfTurn
                    };
                    let target = parse_target_phrase(subject_tokens)?;
                    let power_per = match power {
                        Value::Fixed(value) => value,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported dynamic gets-for-each power modifier (clause: '{}')",
                                words(tokens).join(" ")
                            )));
                        }
                    };
                    let toughness_per = match toughness {
                        Value::Fixed(value) => value,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported dynamic gets-for-each toughness modifier (clause: '{}')",
                                words(tokens).join(" ")
                            )));
                        }
                    };
                    return Ok(EffectAst::PumpForEach {
                        power_per,
                        toughness_per,
                        target,
                        count,
                        duration,
                    });
                }

                let (power, toughness, duration, condition) =
                    parse_get_modifier_values_with_tail(modifier_tail, power, toughness)?;

                let mut normalized_subject_words: Vec<&str> = subject_words
                    .iter()
                    .copied()
                    .filter(|word| *word != "each")
                    .collect();
                if normalized_subject_words.first().copied() == Some("of") {
                    normalized_subject_words.remove(0);
                }
                if normalized_subject_words.as_slice() == ["it"]
                    || normalized_subject_words.as_slice() == ["they"]
                    || normalized_subject_words.as_slice() == ["them"]
                {
                    return Ok(EffectAst::Pump {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        target: TargetAst::Tagged(
                            TagKey::from(IT_TAG),
                            span_from_tokens(subject_tokens),
                        ),
                        duration,
                        condition,
                    });
                }

                let is_demonstrative_subject = normalized_subject_words
                    .first()
                    .is_some_and(|word| *word == "that" || *word == "those");
                if is_demonstrative_subject {
                    let target = parse_target_phrase(subject_tokens)?;
                    return Ok(EffectAst::Pump {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        target,
                        duration,
                        condition,
                    });
                }

                if subject_words.contains(&"target") {
                    let target_tokens = if subject_tokens
                        .first()
                        .is_some_and(|token| token.is_word("have") || token.is_word("has"))
                    {
                        &subject_tokens[1..]
                    } else {
                        subject_tokens
                    };
                    let target = parse_target_phrase(target_tokens)?;
                    return Ok(EffectAst::Pump {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        target,
                        duration,
                        condition,
                    });
                }

                let has_counter_state_pronoun = subject_words.windows(3).any(|window| {
                    matches!(window[0], "counter" | "counters")
                        && window[1] == "on"
                        && matches!(window[2], "it" | "them")
                });
                let has_disallowed_pronoun_reference = (subject_words.contains(&"it")
                    || subject_words.contains(&"them"))
                    && !has_counter_state_pronoun;
                if !subject_words.contains(&"this")
                    && !has_disallowed_pronoun_reference
                    && !has_demonstrative_object_reference(&subject_words)
                    && let Ok(filter) = parse_object_filter(subject_tokens, false)
                    && filter != ObjectFilter::default()
                {
                    return Ok(EffectAst::PumpAll {
                        filter,
                        power: power.clone(),
                        toughness: toughness.clone(),
                        duration,
                    });
                }
            }
        }
    }

    let subject_tokens = &tokens[..verb_idx];
    let subject_words = words(subject_tokens);
    if is_target_player_dealt_damage_by_this_turn_subject(&subject_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history player subject (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    if matches!(verb, Verb::Gain) && !subject_tokens.is_empty() {
        let rest_words = words(&tokens[verb_idx + 1..]);
        let has_protection = rest_words.contains(&"protection");
        let has_choice = rest_words.contains(&"choice");
        let has_color = rest_words.contains(&"color");
        let has_colorless = rest_words.contains(&"colorless");
        if has_protection && has_choice && (has_color || has_colorless) {
            let target = parse_target_phrase(subject_tokens)?;
            return Ok(EffectAst::GrantProtectionChoice {
                target,
                allow_colorless: has_colorless,
            });
        }
    }
    if matches!(verb, Verb::Gain)
        && let Some(effect) = parse_simple_gain_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Lose)
        && let Some(effect) = parse_simple_lose_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    let for_each_subject_filter = parse_for_each_object_subject(subject_tokens)?;
    let rest = &tokens[verb_idx + 1..];
    let mut effect = if matches!(verb, Verb::Become) {
        parse_become_clause(subject_tokens, rest)?
    } else {
        let subject = parse_subject(subject_tokens);
        parse_effect_with_verb(verb, Some(subject), rest)?
    };
    if let Some(filter) = for_each_subject_filter {
        effect = EffectAst::ForEachObject {
            filter,
            effects: vec![effect],
        };
    }
    Ok(effect)
}

pub(crate) fn parse_become_clause(
    subject_tokens: &[Token],
    rest_tokens: &[Token],
) -> Result<EffectAst, CardTextError> {
    use crate::effect::Until;

    let subject_words = words(subject_tokens);
    let subject = parse_subject(subject_tokens);

    // Split off trailing duration, if present.
    let (duration, become_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(rest_tokens)? {
            (duration, remainder)
        } else {
            (Until::Forever, trim_commas(rest_tokens).to_vec())
        };
    let become_words_vec = words(&become_tokens);
    let become_words = if become_words_vec
        .first()
        .is_some_and(|w| *w == "the" || *w == "a" || *w == "an")
    {
        &become_words_vec[1..]
    } else {
        &become_words_vec[..]
    };

    // Player "life total becomes N"
    if let Some(SubjectAst::Player(player)) = Some(subject) {
        if subject_words.contains(&"life") && subject_words.contains(&"total") {
            let amount = parse_value(&become_tokens)
                .map(|(value, _)| value)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing life total amount (clause: '{}')",
                        words(rest_tokens).join(" ")
                    ))
                })?;
            return Ok(EffectAst::SetLifeTotal { amount, player });
        }
    }

    // Resolve object target.
    let target = if subject_words.is_empty()
        || subject_words == ["it"]
        || subject_words == ["they"]
        || subject_words == ["them"]
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(subject_tokens))
    } else if subject_words == ["this"]
        || subject_words == ["this", "permanent"]
        || subject_words == ["this", "creature"]
        || subject_words == ["this", "land"]
    {
        TargetAst::Source(span_from_tokens(subject_tokens))
    } else {
        parse_target_phrase(subject_tokens)?
    };

    // "the/a basic land type of your choice"
    if become_words == ["basic", "land", "type", "of", "your", "choice"] {
        return Ok(EffectAst::BecomeBasicLandTypeChoice { target, duration });
    }

    // "the color of your choice" / "color of your choice"
    if become_words == ["color", "of", "your", "choice"]
        || become_words == ["color", "or", "colors", "of", "your", "choice"]
        || become_words == ["colors", "of", "your", "choice"]
    {
        return Ok(EffectAst::BecomeColorChoice { target, duration });
    }

    // "the creature type of your choice" / "creature type of your choice"
    if become_words == ["creature", "type", "of", "your", "choice"] {
        return Ok(EffectAst::BecomeCreatureTypeChoice {
            target,
            duration,
            excluded_subtypes: Vec::new(),
        });
    }

    // "colorless"
    if become_words == ["colorless"] {
        return Ok(EffectAst::MakeColorless { target, duration });
    }

    // "equal to this's power and toughness"
    if become_words.starts_with(&["equal", "to"]) {
        let rhs = &become_words[2..];
        if rhs == ["this", "power", "and", "toughness"]
            || rhs == ["thiss", "power", "and", "toughness"]
            || rhs == ["source", "power", "and", "toughness"]
        {
            return Ok(EffectAst::SetBasePowerToughness {
                power: Value::PowerOf(Box::new(ChooseSpec::Source)),
                toughness: Value::ToughnessOf(Box::new(ChooseSpec::Source)),
                target,
                duration,
            });
        }
    }

    // "<N>/<M> ... creature" animation-like clauses.
    if let Some(pt_word) = become_words.first().copied()
        && let Ok((power, toughness)) = parse_pt_modifier(pt_word)
        && become_words
            .iter()
            .any(|word| *word == "creature" || *word == "creatures")
    {
        return Ok(EffectAst::BecomeBasePtCreature {
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            target,
            duration,
        });
    }

    // "<card type>[ ... ]" and "<card type> in addition to its/their other types"
    let addition_tail_len = if become_words.ends_with(&["in", "addition", "to", "its", "other", "types"]) {
        Some(6usize)
    } else if become_words.ends_with(&["in", "addition", "to", "their", "other", "types"]) {
        Some(6usize)
    } else if become_words.ends_with(&["in", "addition", "to", "its", "other", "type"]) {
        Some(6usize)
    } else if become_words.ends_with(&["in", "addition", "to", "their", "other", "type"]) {
        Some(6usize)
    } else {
        None
    };
    let card_type_words = if let Some(tail_len) = addition_tail_len {
        &become_words[..become_words.len().saturating_sub(tail_len)]
    } else {
        become_words
    };
    if !card_type_words.is_empty() {
        let mut card_types = Vec::new();
        let mut all_card_types = true;
        for word in card_type_words {
            if let Some(card_type) = parse_card_type(word) {
                if !card_types.contains(&card_type) {
                    card_types.push(card_type);
                }
            } else {
                all_card_types = false;
                break;
            }
        }
        if all_card_types && !card_types.is_empty() {
            return Ok(EffectAst::AddCardTypes {
                target,
                card_types,
                duration,
            });
        }
    }

    if !card_type_words.is_empty() {
        let mut subtypes = Vec::new();
        let mut all_subtypes = true;
        for word in card_type_words {
            if let Some(subtype) =
                parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
            {
                if !subtypes.contains(&subtype) {
                    subtypes.push(subtype);
                }
            } else {
                all_subtypes = false;
                break;
            }
        }
        if all_subtypes && !subtypes.is_empty() {
            return Ok(EffectAst::AddSubtypes {
                target,
                subtypes,
                duration,
            });
        }
    }

    // "<color>" / "<color> and <color>" / "<color> and or <color>"
    let color_tokens = become_words
        .iter()
        .copied()
        .filter(|word| *word != "and" && *word != "or")
        .collect::<Vec<_>>();
    if !color_tokens.is_empty() {
        let mut colors = crate::color::ColorSet::new();
        let mut all_colors = true;
        for word in color_tokens {
            if let Some(color) = parse_color(word) {
                colors = colors.union(color);
            } else {
                all_colors = false;
                break;
            }
        }
        if all_colors && !colors.is_empty() {
            return Ok(EffectAst::SetColors {
                target,
                colors,
                duration,
            });
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported become clause (clause: '{}')",
        words(rest_tokens).join(" ")
    )))
}
