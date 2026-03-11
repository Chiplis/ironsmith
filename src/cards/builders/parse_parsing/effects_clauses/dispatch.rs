use super::zones::parse_half_starting_life_total_value;
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, TargetAst, Token, Verb, bind_implicit_player_context,
    contains_until_end_of_turn, extract_subject_player, find_negation_span, find_verb,
    has_demonstrative_object_reference, is_mana_replacement_clause_words,
    is_mana_trigger_additional_clause_words, is_target_player_dealt_damage_by_this_turn_subject,
    keyword_action_to_static_ability, parse_ability_line, parse_cant_restrictions, parse_card_type,
    parse_color, parse_effect_chain_with_sentence_primitives, parse_effect_with_verb,
    parse_for_each_object_subject, parse_get_for_each_count_value,
    parse_get_modifier_values_with_tail, parse_has_base_power_clause,
    parse_has_base_power_toughness_clause, parse_leading_player_may, parse_object_filter,
    parse_pt_modifier, parse_pt_modifier_values, parse_put_counters, parse_restriction_duration,
    parse_simple_gain_ability_clause, parse_simple_lose_ability_clause, parse_subject,
    parse_subtype_word, parse_target_phrase, parse_target_player_choose_objects_clause,
    parse_value, parse_you_choose_objects_clause, parser_trace, parser_trace_stack,
    remove_first_word, remove_through_first_word, run_clause_primitives, span_from_tokens,
    starts_with_until_end_of_turn, strip_leading_instead_prefix, token_index_for_word_index,
    trim_commas, words,
};
use crate::{ChooseSpec, ObjectFilter, Subtype, TagKey, Until, Value};

pub(crate) fn parse_effect_clause(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }

    let stripped_instead = strip_leading_instead_prefix(tokens);
    let tokens = stripped_instead.as_deref().unwrap_or(tokens);

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
    if is_mana_replacement_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana replacement clause (clause: '{}') [rule=mana-replacement]",
            clause_words.join(" ")
        )));
    }

    if is_mana_trigger_additional_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-triggered additional-mana clause (clause: '{}') [rule=mana-trigger-additional]",
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

    if matches!(
        clause_words.as_slice(),
        ["choose", "a", "color"]
            | ["choose", "color"]
            | ["you", "choose", "a", "color"]
            | ["you", "choose", "color"]
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported choose-color clause (clause: '{}') [rule=choose-color]",
            clause_words.join(" ")
        )));
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
                "unsupported assigns-no-combat-damage clause (clause: '{}') [rule=assigns-no-combat-damage]",
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
                "unsupported assigns-no-combat-damage clause tail (clause: '{}') [rule=assigns-no-combat-damage-tail]",
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
                "unsupported target-only restriction clause (clause: '{}') [rule=target-only-restriction]",
                clause_words.join(" ")
            )));
        }
        let target = parse_target_phrase(tokens)?;
        return Ok(EffectAst::TargetOnly { target });
    }

    if let Some((duration, clause_tokens)) = parse_restriction_duration(tokens)?
        && find_negation_span(&clause_tokens).is_some()
        && let Some(restrictions) = parse_cant_restrictions(&clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && parsed.target.is_none()
    {
        return Ok(EffectAst::Cant {
            restriction: parsed.restriction.clone(),
            duration,
            condition: None,
        });
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
                    let duration = if starts_with_until_end_of_turn(&modifier_words)
                        || contains_until_end_of_turn(&modifier_words)
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
            "unsupported combat-history player subject (clause: '{}') [rule=combat-history-player-subject]",
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
    let become_body_tokens = if become_tokens
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word == "the" || word == "a" || word == "an")
    {
        &become_tokens[1..]
    } else {
        &become_tokens[..]
    };
    let become_words_vec = words(become_body_tokens);
    let become_words = &become_words_vec[..];

    // Player "life total becomes N"
    if let Some(player) = extract_subject_player(Some(subject)) {
        if become_words == ["monarch"] {
            return Ok(EffectAst::BecomeMonarch { player });
        }
        if subject_words.contains(&"life") && subject_words.contains(&"total") {
            let amount = parse_value(&become_tokens)
                .map(|(value, _)| value)
                .or_else(|| parse_half_starting_life_total_value(&become_tokens, player))
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

    if let [word] = become_words
        && let Some(subtype) = parse_subtype_word(word)
        && matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
        )
    {
        return Ok(EffectAst::BecomeBasicLandType {
            target,
            subtype,
            duration,
        });
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

    // "copy of <target>"
    if become_words.starts_with(&["copy", "of"]) {
        let Some(source_start) = token_index_for_word_index(become_body_tokens, 2) else {
            return Err(CardTextError::ParseError(format!(
                "missing copy source in become clause (clause: '{}')",
                words(rest_tokens).join(" ")
            )));
        };
        let source_tokens = trim_commas(&become_body_tokens[source_start..]);
        if source_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing copy source in become clause (clause: '{}')",
                words(rest_tokens).join(" ")
            )));
        }
        let source = parse_target_phrase(&source_tokens)?;
        return Ok(EffectAst::BecomeCopy {
            target,
            source,
            duration,
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
        && let Some(creature_idx) = become_words
            .iter()
            .position(|word| *word == "creature" || *word == "creatures")
    {
        let mut card_types = vec![crate::types::CardType::Creature];
        let mut subtypes = Vec::new();
        let mut colors = crate::color::ColorSet::new();
        let mut all_prefix_words_supported = true;
        for word in &become_words[1..creature_idx] {
            if let Some(color) = parse_color(word) {
                colors = colors.union(color);
                continue;
            }
            if let Some(card_type) = parse_card_type(word) {
                if card_type != crate::types::CardType::Creature && !card_types.contains(&card_type)
                {
                    card_types.push(card_type);
                }
                continue;
            }
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
            {
                if !subtypes.contains(&subtype) {
                    subtypes.push(subtype);
                }
                continue;
            }
            all_prefix_words_supported = false;
            break;
        }

        let mut abilities = Vec::new();
        let suffix_tokens = if let Some(creature_token_idx) =
            token_index_for_word_index(become_body_tokens, creature_idx)
        {
            trim_commas(&become_body_tokens[creature_token_idx + 1..]).to_vec()
        } else {
            Vec::new()
        };
        let suffix_supported = if suffix_tokens.is_empty() {
            true
        } else if suffix_tokens
            .first()
            .is_some_and(|token| token.is_word("with"))
        {
            parse_ability_line(&trim_commas(&suffix_tokens[1..]))
                .map(|actions| {
                    abilities = actions
                        .into_iter()
                        .filter_map(keyword_action_to_static_ability)
                        .collect::<Vec<_>>();
                    !abilities.is_empty()
                })
                .unwrap_or(false)
        } else {
            false
        };

        let colors = if colors.is_empty() {
            None
        } else {
            Some(colors)
        };
        if !all_prefix_words_supported || !suffix_supported {
            return Ok(EffectAst::BecomeBasePtCreature {
                power: Value::Fixed(power),
                toughness: Value::Fixed(toughness),
                target,
                card_types: vec![crate::types::CardType::Creature],
                subtypes: Vec::new(),
                colors: None,
                abilities: Vec::new(),
                duration,
            });
        }
        return Ok(EffectAst::BecomeBasePtCreature {
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            target,
            card_types,
            subtypes,
            colors,
            abilities,
            duration,
        });
    }

    // "<card type>[ ... ]" and "<card type> in addition to its/their other types"
    let addition_tail_len =
        if become_words.ends_with(&["in", "addition", "to", "its", "other", "types"]) {
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
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
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
