use super::*;
use crate::cards::builders::effect_ast_traversal::{
    for_each_nested_effects, for_each_nested_effects_mut,
};

pub(crate) fn parse_activated_line(
    tokens: &[Token],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(colon_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Colon(_)))
    else {
        return Ok(None);
    };

    let cost_start = find_activation_cost_start(&tokens[..colon_idx]).unwrap_or(0);
    let cost_tokens = &tokens[cost_start..colon_idx];
    let effect_tokens = &tokens[colon_idx + 1..];
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return Ok(None);
    }
    let ability_label = if cost_start > 0 {
        let prefix = words(&tokens[..cost_start]);
        if prefix == ["boast"] || prefix.last() == Some(&"boast") {
            Some("Boast".to_string())
        } else if prefix == ["renew"] || prefix.last() == Some(&"renew") {
            Some("Renew".to_string())
        } else {
            None
        }
    } else {
        None
    };
    let apply_ability_label = |ability: &mut Ability| {
        if ability.text.is_none() {
            if let Some(label) = &ability_label {
                ability.text = Some(label.clone());
            }
        }
    };

    let mut effect_sentences = split_on_period(effect_tokens);
    let functional_zones = infer_activated_functional_zones(cost_tokens, &effect_sentences);
    let mut timing = ActivationTiming::AnyTime;
    let mut mana_activation_condition: Option<crate::ConditionExpr> = None;
    let mut additional_activation_restrictions: Vec<String> = Vec::new();
    effect_sentences.retain(|sentence| {
        if is_activate_only_restriction_sentence(sentence) {
            if let Some(parsed_timing) = parse_activate_only_timing(sentence) {
                timing = parsed_timing;
            }
            if let Some(condition) = parse_activation_condition(sentence) {
                mana_activation_condition =
                    merge_mana_activation_conditions(mana_activation_condition.clone(), condition);
            }
            if let Some(restriction) =
                normalize_activate_only_restriction(sentence, &timing.clone())
            {
                additional_activation_restrictions.push(restriction);
            }
            return false;
        }
        if is_spend_mana_restriction_sentence(sentence) {
            additional_activation_restrictions.push(words(sentence).join(" "));
            return false;
        }
        if is_any_player_may_activate_sentence(sentence) {
            additional_activation_restrictions.push(words(sentence).join(" "));
            return false;
        }
        if is_trigger_only_restriction_sentence(sentence) {
            return false;
        }
        true
    });
    let mana_activation_condition =
        combine_mana_activation_condition(mana_activation_condition, timing.clone());
    if !effect_sentences.is_empty() {
        let primary_sentence = &effect_sentences[0];
        let effect_words = words(primary_sentence);
        let is_primary_add_clause = matches!(
            effect_words.as_slice(),
            ["add", ..]
                | ["adds", ..]
                | ["you", "add", ..]
                | ["that", "player", "add", ..]
                | ["that", "player", "adds", ..]
                | ["target", "player", "add", ..]
                | ["target", "player", "adds", ..]
        );
        if is_primary_add_clause {
            let (mana_cost, cost_effects) = parse_activation_cost(cost_tokens)?;
            let mana_cost = crate::ability::merge_cost_effects(mana_cost, cost_effects);

            let mut extra_effects = Vec::new();
            let mut extra_effects_ast = Vec::new();
            if effect_sentences.len() > 1 {
                for sentence in &effect_sentences[1..] {
                    if sentence.is_empty() {
                        continue;
                    }
                    let ast = parse_effect_sentence(sentence)?;
                    let compiled = compile_statement_effects(&ast)?;
                    extra_effects.extend(compiled);
                    extra_effects_ast.extend(ast);
                }
            }

            let add_token_idx = primary_sentence
                .iter()
                .position(|token| token.is_word("add"))
                .unwrap_or(0);
            let mana_tokens = &primary_sentence[add_token_idx + 1..];
            let mana_words = words(mana_tokens);
            let has_for_each_tail = mana_tokens
                .windows(2)
                .any(|window| window[0].is_word("for") && window[1].is_word("each"));
            let dynamic_amount = if has_for_each_tail {
                Some(
                    parse_dynamic_cost_modifier_value(mana_tokens)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported dynamic mana amount (clause: '{}')",
                            words(primary_sentence).join(" ")
                        ))
                    })?,
                )
            } else {
                parse_devotion_value_from_add_clause(mana_tokens)?
                    .or_else(|| parse_add_mana_equal_amount_value(mana_tokens))
            };

            let has_imprinted_colors = mana_words.contains(&"exiled")
                && (mana_words.contains(&"card") || mana_words.contains(&"cards"))
                && mana_words
                    .iter()
                    .any(|word| *word == "color" || *word == "colors");
            let has_any_combination_mana = mana_words
                .windows(3)
                .any(|window| window == ["any", "combination", "of"]);
            let has_any_choice_mana = mana_words.contains(&"any")
                && (mana_words.contains(&"color")
                    || mana_words.contains(&"type")
                    || has_any_combination_mana);
            let has_or_choice_mana = mana_words.contains(&"or");
            let has_chosen_color = mana_words.contains(&"chosen") && mana_words.contains(&"color");
            let uses_commander_identity = mana_words
                .iter()
                .any(|word| *word == "commander" || *word == "commanders")
                && mana_words.contains(&"identity");
            if has_imprinted_colors
                || has_any_choice_mana
                || uses_commander_identity
                || has_chosen_color
            {
                let mana_ast = parse_add_mana(mana_tokens, None)?;
                let mut compile_ctx = CompileContext::new();
                let (mut effects, choices) = compile_effect(&mana_ast, &mut compile_ctx)?;
                if !choices.is_empty() {
                    return Err(CardTextError::ParseError(
                        "unsupported target choice in mana ability".to_string(),
                    ));
                }
                effects.extend(extra_effects);
                let mut ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects,
                        choices: vec![],
                        timing: ActivationTiming::AnyTime,
                        additional_restrictions: vec![],
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition.clone(),
                    }),
                    functional_zones: functional_zones.clone(),
                    text: None,
                };
                apply_ability_label(&mut ability);
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast: Some(effects_ast),
                }));
            }

            if has_or_choice_mana && !extra_effects.is_empty() {
                let mana_ast = parse_add_mana(mana_tokens, None)?;
                let mut compile_ctx = CompileContext::new();
                let (mut effects, choices) = compile_effect(&mana_ast, &mut compile_ctx)?;
                if !choices.is_empty() {
                    return Err(CardTextError::ParseError(
                        "unsupported target choice in mana ability".to_string(),
                    ));
                }
                effects.extend(extra_effects);
                let mut ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects,
                        choices: vec![],
                        timing: ActivationTiming::AnyTime,
                        additional_restrictions: vec![],
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition.clone(),
                    }),
                    functional_zones: functional_zones.clone(),
                    text: None,
                };
                apply_ability_label(&mut ability);
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast: Some(effects_ast),
                }));
            }

            let mut mana = Vec::new();
            for token in mana_tokens {
                let Some(word) = token.as_word() else {
                    continue;
                };
                if word == "mana" || word == "to" || word == "your" || word == "pool" {
                    continue;
                }
                if let Ok(symbol) = parse_mana_symbol(word) {
                    mana.push(symbol);
                }
            }

            if !mana.is_empty() {
                if let Some(amount) = dynamic_amount {
                    let amount = resolve_mana_ability_scaled_amount_from_cost(amount, &mana_cost)?;
                    let mut effects =
                        vec![Effect::new(crate::effects::mana::AddScaledManaEffect::new(
                            mana,
                            amount,
                            PlayerFilter::You,
                        ))];
                    effects.extend(extra_effects);
                    let mut ability = Ability {
                        kind: AbilityKind::Activated(ActivatedAbility {
                            mana_cost,
                            effects,
                            choices: vec![],
                            timing: ActivationTiming::AnyTime,
                            additional_restrictions: vec![],
                            activation_restrictions: vec![],
                            mana_output: Some(vec![]),
                            activation_condition: mana_activation_condition.clone(),
                        }),
                        functional_zones: functional_zones.clone(),
                        text: None,
                    };
                    apply_ability_label(&mut ability);
                    let effects_ast = if extra_effects_ast.is_empty() {
                        None
                    } else {
                        Some(extra_effects_ast)
                    };
                    return Ok(Some(ParsedAbility {
                        ability,
                        effects_ast,
                    }));
                }
                if extra_effects.is_empty() {
                    let mut ability = Ability {
                        kind: AbilityKind::Activated(ActivatedAbility {
                            mana_cost,
                            effects: vec![],
                            choices: vec![],
                            timing: ActivationTiming::AnyTime,
                            additional_restrictions: vec![],
                            activation_restrictions: vec![],
                            mana_output: Some(mana),
                            activation_condition: mana_activation_condition.clone(),
                        }),
                        functional_zones: functional_zones.clone(),
                        text: None,
                    };
                    apply_ability_label(&mut ability);
                    let effects_ast = if extra_effects_ast.is_empty() {
                        None
                    } else {
                        Some(extra_effects_ast)
                    };
                    return Ok(Some(ParsedAbility {
                        ability,
                        effects_ast,
                    }));
                }
                let mut effects = vec![Effect::add_mana(mana)];
                effects.extend(extra_effects);
                let mut ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects,
                        choices: vec![],
                        timing: ActivationTiming::AnyTime,
                        additional_restrictions: vec![],
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition,
                    }),
                    functional_zones: functional_zones.clone(),
                    text: None,
                };
                apply_ability_label(&mut ability);
                let effects_ast = if extra_effects_ast.is_empty() {
                    None
                } else {
                    Some(extra_effects_ast)
                };
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast,
                }));
            }
        }
    }

    // Generic activated ability: parse costs and effects from "<costs>: <effects>"
    let (mana_cost, cost_effects) = parse_activation_cost(cost_tokens)?;
    let effect_tokens_joined = join_sentences_with_period(&effect_sentences);
    let effects_ast = parse_effect_sentences(&effect_tokens_joined)?;
    if effects_ast.is_empty() {
        return Ok(None);
    }
    let seed_tag = first_sacrifice_cost_choice_tag(&mana_cost)
        .or_else(|| last_exile_cost_choice_tag(&mana_cost))
        .map(|tag| tag.as_str().to_string());
    let (effects, choices) = compile_trigger_effects_seeded(None, &effects_ast, seed_tag)?;
    let mana_cost = crate::ability::merge_cost_effects(mana_cost, cost_effects);

    Ok(Some(ParsedAbility {
        ability: {
            let mut ability = Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost,
                    effects,
                    choices,
                    timing,
                    additional_restrictions: additional_activation_restrictions,
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                }),
                functional_zones,
                text: None,
            };
            apply_ability_label(&mut ability);
            ability
        },
        effects_ast: Some(effects_ast),
    }))
}

pub(crate) fn first_sacrifice_cost_choice_tag(
    mana_cost: &crate::cost::TotalCost,
) -> Option<TagKey> {
    for cost in mana_cost.costs() {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if choose.tag.as_str().starts_with("sacrifice_cost_") {
            return Some(choose.tag.clone());
        }
    }
    None
}

pub(crate) fn last_exile_cost_choice_tag(mana_cost: &crate::cost::TotalCost) -> Option<TagKey> {
    let mut found = None;
    for cost in mana_cost.costs() {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if choose.tag.as_str().starts_with("exile_cost_") {
            found = Some(choose.tag.clone());
        }
    }
    found
}

pub(crate) fn resolve_mana_ability_scaled_amount_from_cost(
    amount: Value,
    mana_cost: &crate::cost::TotalCost,
) -> Result<Value, CardTextError> {
    if let Value::ManaValueOf(spec) = &amount
        && let ChooseSpec::Tagged(tag) = spec.as_ref()
        && tag.as_str() == IT_TAG
    {
        let Some(sac_tag) = first_sacrifice_cost_choice_tag(mana_cost) else {
            return Err(CardTextError::ParseError(
                "mana-value scaling requires a sacrificed object cost reference".to_string(),
            ));
        };
        return Ok(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(sac_tag))));
    }

    Ok(amount)
}

pub(crate) fn infer_activated_functional_zones(
    cost_tokens: &[Token],
    effect_sentences: &[Vec<Token>],
) -> Vec<Zone> {
    let cost_words: Vec<&str> = words(cost_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let effect_words_match = |f: fn(&[&str]) -> bool| {
        effect_sentences.iter().any(|sentence| {
            let clause_words: Vec<&str> = words(sentence)
                .into_iter()
                .filter(|word| !is_article(word))
                .collect();
            f(&clause_words)
        })
    };
    if contains_source_from_your_graveyard_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_graveyard_phrase)
    {
        vec![Zone::Graveyard]
    } else if contains_source_from_your_hand_phrase(&cost_words)
        || contains_discard_source_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_hand_phrase)
    {
        vec![Zone::Hand]
    } else {
        vec![Zone::Battlefield]
    }
}

pub(crate) fn parse_activate_only_timing(tokens: &[Token]) -> Option<ActivationTiming> {
    let words = words(tokens);
    if words.starts_with(&["activate", "only", "as", "a", "sorcery"]) {
        return Some(ActivationTiming::SorcerySpeed);
    }
    if words.starts_with(&["activate", "only", "once", "each", "turn"])
        || contains_word_sequence(&words, &["once", "each", "turn"])
    {
        return Some(ActivationTiming::OncePerTurn);
    }
    if words.starts_with(&["activate", "only", "during", "combat"])
        || contains_word_sequence(&words, &["during", "combat"])
    {
        return Some(ActivationTiming::DuringCombat);
    }
    if words.starts_with(&["activate", "only", "during", "your", "turn"])
        || contains_word_sequence(&words, &["during", "your", "turn"])
    {
        return Some(ActivationTiming::DuringYourTurn);
    }
    if words.starts_with(&["activate", "only", "during", "an", "opponents", "turn"])
        || words.starts_with(&["activate", "only", "during", "opponents", "turn"])
        || contains_word_sequence(&words, &["during", "an", "opponents", "turn"])
        || contains_word_sequence(&words, &["during", "opponents", "turn"])
    {
        return Some(ActivationTiming::DuringOpponentsTurn);
    }
    None
}

pub(crate) fn normalize_activate_only_restriction(
    tokens: &[Token],
    timing: &ActivationTiming,
) -> Option<String> {
    if timing != &ActivationTiming::OncePerTurn {
        return Some(words(tokens).join(" "));
    }

    let mut words = words(tokens)
        .into_iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }
    if words == ["activate", "only", "once", "each", "turn"] {
        return None;
    }
    if words.len() >= 6 && words[0..6] == ["activate", "only", "once", "each", "turn", "and"] {
        words.drain(0..6);
    }
    let mut index = 0usize;
    while index + 5 <= words.len() {
        if words[index..index + 5] == ["and", "only", "once", "each", "turn"] {
            words.drain(index..index + 5);
        } else {
            index += 1;
        }
    }
    if words.is_empty() {
        None
    } else {
        Some(words.join(" "))
    }
}

pub(crate) fn contains_word_sequence(words: &[&str], sequence: &[&str]) -> bool {
    if sequence.is_empty() || words.len() < sequence.len() {
        return false;
    }
    words
        .windows(sequence.len())
        .any(|window| window == sequence)
}

pub(crate) fn flatten_mana_activation_conditions(
    condition: &crate::ConditionExpr,
    out: &mut Vec<crate::ConditionExpr>,
) {
    match condition {
        crate::ConditionExpr::And(left, right) => {
            flatten_mana_activation_conditions(left, out);
            flatten_mana_activation_conditions(right, out);
        }
        _ => out.push(condition.clone()),
    }
}

pub(crate) fn rebuild_mana_activation_conditions(
    conditions: Vec<crate::ConditionExpr>,
) -> Option<crate::ConditionExpr> {
    let mut iter = conditions.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, next| {
        crate::ConditionExpr::And(Box::new(acc), Box::new(next))
    }))
}

pub(crate) fn combine_mana_activation_condition(
    base: Option<crate::ConditionExpr>,
    timing: ActivationTiming,
) -> Option<crate::ConditionExpr> {
    if timing == ActivationTiming::AnyTime {
        return base;
    }
    merge_mana_activation_conditions(base, crate::ConditionExpr::ActivationTiming(timing))
}

pub(crate) fn merge_mana_activation_conditions(
    base: Option<crate::ConditionExpr>,
    condition: crate::ConditionExpr,
) -> Option<crate::ConditionExpr> {
    let mut conditions: Vec<crate::ConditionExpr> = Vec::new();
    if let Some(base) = base {
        flatten_mana_activation_conditions(&base, &mut conditions);
    }
    if !conditions.iter().any(|existing| *existing == condition) {
        conditions.push(condition);
    }
    rebuild_mana_activation_conditions(conditions)
}

pub(crate) fn is_activate_only_restriction_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.starts_with(&["activate", "only"])
        || words.starts_with(&["activate", "no", "more", "than"])
}

pub(crate) fn is_spend_mana_restriction_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.starts_with(&["spend", "this", "mana", "only"])
        || words.starts_with(&["spend", "that", "mana", "only"])
}

pub(crate) fn is_any_player_may_activate_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.as_slice() == ["any", "player", "may", "activate", "this", "ability"]
}

pub(crate) fn is_trigger_only_restriction_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.starts_with(&["this", "ability", "triggers", "only"])
}

pub(crate) fn parse_triggered_times_each_turn_sentence(sentences: &[Vec<Token>]) -> Option<u32> {
    sentences.iter().find_map(|sentence| {
        let words = words(sentence);
        parse_triggered_times_each_turn_from_words(&words)
    })
}

pub(crate) fn parse_triggered_times_each_turn_from_words(words: &[&str]) -> Option<u32> {
    if words.len() < 7 || !words.starts_with(&["this", "ability", "triggers", "only"]) {
        return None;
    }

    let mut index = 4usize;
    let count = match words.get(index) {
        Some(word) if *word == "once" => Some(1),
        Some(word) if *word == "twice" => Some(2),
        Some(word) => parse_named_number(word),
        None => None,
    }?;
    index += 1;

    if words.get(index) == Some(&"time") || words.get(index) == Some(&"times") {
        index += 1;
    }

    if words.get(index) == Some(&"each") && words.get(index + 1) == Some(&"turn") {
        Some(count)
    } else {
        None
    }
}

pub(crate) fn parse_named_number(word: &str) -> Option<u32> {
    parse_number_word_u32(word)
}

pub(crate) fn parse_level_up_line(
    tokens: &[Token],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 3 || words[0] != "level" || words[1] != "up" {
        return Ok(None);
    }

    let mut symbols = Vec::new();
    for word in words.iter().skip(2) {
        if let Ok(symbol) = parse_mana_symbol(word) {
            symbols.push(symbol);
        }
    }

    if symbols.is_empty() {
        return Err(CardTextError::ParseError(
            "level up missing mana cost".to_string(),
        ));
    }

    let pips = symbols.into_iter().map(|symbol| vec![symbol]).collect();
    let mana_cost = ManaCost::from_pips(pips);
    let level_up_text = format!("Level up {}", mana_cost.to_oracle());

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: TotalCost::mana(mana_cost),
                effects: vec![Effect::put_counters_on_source(CounterType::Level, 1)],
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some(level_up_text),
        },
        effects_ast: None,
    }))
}

pub(crate) fn parse_cycling_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let words_all = words(tokens);
    if words_all.is_empty() {
        return Ok(None);
    }

    let Some(cycling_idx) = words_all.iter().position(|word| word.ends_with("cycling")) else {
        return Ok(None);
    };
    // Static grant clauses like "Each Sliver card in each player's hand has slivercycling {3}."
    // must be handled by parse_filter_has_granted_ability_line, not parsed as a standalone
    // cycling keyword ability on this card.
    if words_all
        .iter()
        .take(cycling_idx)
        .any(|word| *word == "has" || *word == "have")
    {
        return Ok(None);
    }

    let cycling_groups = parse_cycling_keyword_cost_groups(tokens);
    let Some((first_keyword_tokens, first_cost_tokens)) = cycling_groups.first() else {
        return Ok(None);
    };
    if first_cost_tokens.is_empty() {
        return Ok(None);
    }

    let base_cost_words = words(first_cost_tokens);
    if cycling_groups
        .iter()
        .skip(1)
        .any(|(_, cost_tokens)| words(cost_tokens) != base_cost_words)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported mixed cycling costs (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let (base_cost, cost_effects) = parse_activation_cost(first_cost_tokens)?;
    let mut full_cost_effects = cost_effects;
    // Cycling is an activated ability from hand whose cost includes discarding this card.
    // Model this as a source-zone-change cost so the correct zone-change events fire.
    full_cost_effects.push(Effect::move_to_zone(
        ChooseSpec::Source,
        Zone::Graveyard,
        false,
    ));
    // Emit a keyword-action event so "When you cycle this card" triggers can observe it.
    full_cost_effects.push(Effect::emit_keyword_action(
        crate::events::KeywordActionKind::Cycle,
        1,
    ));
    let mana_cost = crate::ability::merge_cost_effects(base_cost.clone(), full_cost_effects);

    let mut search_filter = parse_cycling_search_filter(first_keyword_tokens)?;
    for (keyword_tokens, _) in cycling_groups.iter().skip(1) {
        let next_filter = parse_cycling_search_filter(keyword_tokens)?;
        match (&mut search_filter, next_filter) {
            (Some(current), Some(next)) => merge_cycling_search_filters(current, &next),
            (None, None) => {}
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported mixed cycling variants (clause: '{}')",
                    words(tokens).join(" ")
                )));
            }
        }
    }
    let effect = if let Some(filter) = search_filter {
        Effect::search_library_to_hand(filter, true)
    } else {
        Effect::draw(1)
    };

    let cost_text = base_cost
        .mana_cost()
        .map(|cost| cost.to_oracle())
        .unwrap_or_else(|| base_cost_words.join(" "));
    let render_text = if let Some(group) = parse_cycling_keyword_group_text(tokens) {
        group
    } else if words(first_keyword_tokens).is_empty() {
        cost_text
    } else {
        format!("{} {cost_text}", words(first_keyword_tokens).join(" "))
    };

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost,
                effects: vec![effect],
                choices: Vec::new(),
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
            }),
            functional_zones: vec![Zone::Hand],
            text: Some(render_text),
        },
        effects_ast: None,
    }))
}

pub(crate) fn parse_reinforce_line(
    tokens: &[Token],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let words_all = words(tokens);
    if words_all.is_empty() {
        return Ok(None);
    }
    if words_all.first().copied() != Some("reinforce") {
        return Ok(None);
    }
    if words_all
        .iter()
        .any(|word| *word == "has" || *word == "have")
    {
        return Ok(None);
    }

    let (amount, used_amount) =
        parse_number(tokens.get(1..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "reinforce line missing counter amount (clause: '{}')",
                words_all.join(" ")
            ))
        })?;

    let cost_start = 1 + used_amount;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "reinforce line missing mana cost (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let mut cost_end = cost_start;
    while cost_end < tokens.len() {
        let Some(word) = tokens[cost_end].as_word() else {
            break;
        };
        if parse_mana_symbol(word).is_ok() {
            cost_end += 1;
        } else {
            break;
        }
    }
    if cost_end == cost_start {
        return Err(CardTextError::ParseError(format!(
            "reinforce line missing mana symbols (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let cost_tokens = &tokens[cost_start..cost_end];
    let (base_cost, mut cost_effects) = parse_activation_cost(cost_tokens)?;
    cost_effects.push(Effect::move_to_zone(
        ChooseSpec::Source,
        Zone::Graveyard,
        false,
    ));
    let mana_cost = crate::ability::merge_cost_effects(base_cost.clone(), cost_effects);

    let mut creature_filter = ObjectFilter::default();
    creature_filter.zone = Some(Zone::Battlefield);
    creature_filter.card_types.push(CardType::Creature);

    let target = ChooseSpec::target(ChooseSpec::Object(creature_filter));
    let effect = Effect::put_counters(CounterType::PlusOnePlusOne, amount as i32, target);

    let cost_text = base_cost
        .mana_cost()
        .map(|cost| cost.to_oracle())
        .unwrap_or_else(|| words(cost_tokens).join(" "));
    let render_text = format!("Reinforce {amount} {cost_text}");

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost,
                effects: vec![effect],
                choices: Vec::new(),
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
            }),
            functional_zones: vec![Zone::Hand],
            text: Some(render_text),
        },
        effects_ast: None,
    }))
}

pub(crate) fn parse_cycling_keyword_cost_groups(tokens: &[Token]) -> Vec<(Vec<Token>, Vec<Token>)> {
    let mut groups = Vec::new();
    let mut idx = 0usize;

    while idx < tokens.len() {
        if tokens
            .get(idx)
            .is_some_and(|token| matches!(token, Token::Comma(_) | Token::Semicolon(_)))
        {
            idx += 1;
            continue;
        }

        let keyword_start = idx;
        let mut keyword_end: Option<usize> = None;
        while idx < tokens.len() {
            let Some(word) = tokens[idx].as_word() else {
                break;
            };
            if word.ends_with("cycling") {
                keyword_end = Some(idx);
                idx += 1;
                break;
            }
            idx += 1;
        }
        let Some(keyword_end) = keyword_end else {
            break;
        };

        let cost_start = idx;
        if tokens.get(idx).is_some_and(|token| token.is_word("pay")) {
            // Handle life-cycling style costs like "Cycling—Pay 2 life."
            while idx < tokens.len() {
                let Some(word) = tokens[idx].as_word() else {
                    break;
                };
                idx += 1;
                if word == "life" {
                    break;
                }
            }
        } else {
            while idx < tokens.len() {
                let Some(word) = tokens[idx].as_word() else {
                    break;
                };
                // Reminder text often starts with "{N}, discard this card" and would
                // otherwise be consumed as part of the cycling cost.
                let looks_like_reminder_cost = idx > cost_start
                    && word.chars().all(|ch| ch.is_ascii_digit())
                    && tokens
                        .get(idx + 1)
                        .is_some_and(|token| matches!(token, Token::Comma(_)))
                    && tokens
                        .get(idx + 2)
                        .and_then(Token::as_word)
                        .is_some_and(|next| next == "discard");
                if looks_like_reminder_cost || !is_cycling_cost_word(word) {
                    break;
                }
                idx += 1;
            }
        }
        if idx == cost_start {
            break;
        }

        groups.push((
            tokens[keyword_start..=keyword_end].to_vec(),
            tokens[cost_start..idx].to_vec(),
        ));

        if tokens
            .get(idx)
            .is_some_and(|token| matches!(token, Token::Comma(_)))
        {
            idx += 1;
            continue;
        }
        break;
    }

    groups
}

pub(crate) fn merge_cycling_search_filters(base: &mut ObjectFilter, extra: &ObjectFilter) {
    for supertype in &extra.supertypes {
        if !base.supertypes.contains(supertype) {
            base.supertypes.push(*supertype);
        }
    }
    for card_type in &extra.card_types {
        if !base.card_types.contains(card_type) {
            base.card_types.push(*card_type);
        }
    }
    for subtype in &extra.subtypes {
        if !base.subtypes.contains(subtype) {
            base.subtypes.push(*subtype);
        }
    }
    if let Some(colors) = extra.colors {
        base.colors = Some(
            base.colors
                .map_or(colors, |existing| existing.union(colors)),
        );
    }
}

pub(crate) fn parse_cycling_keyword_group_text(tokens: &[Token]) -> Option<String> {
    let groups = parse_cycling_keyword_cost_groups(tokens);
    if groups.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    for (keyword_tokens, cost_tokens) in groups {
        let keyword = words(&keyword_tokens).join(" ");
        if keyword.is_empty() {
            continue;
        }
        let cost_words = super::words(&cost_tokens);
        let cost = if cost_words.len() >= 3 && cost_words[0] == "pay" && cost_words[2] == "life" {
            format!("pay {} life", cost_words[1])
        } else {
            parse_activation_cost(&cost_tokens)
                .ok()
                .and_then(|(total_cost, _)| total_cost.mana_cost().map(|cost| cost.to_oracle()))
                .unwrap_or_else(|| cost_words.join(" "))
        };
        parts.push(format!("{keyword} {cost}"));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

pub(crate) fn is_cycling_cost_word(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|ch| {
            ch.is_ascii_digit()
                || matches!(
                    ch,
                    '{' | '}' | '/' | 'w' | 'u' | 'b' | 'r' | 'g' | 'c' | 'x'
                )
        })
}

pub(crate) fn parse_madness_line(
    tokens: &[Token],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("madness")) {
        return Ok(None);
    }

    let cost_start = 1;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(
            "madness keyword missing mana cost".to_string(),
        ));
    }

    let cost_end = tokens[cost_start..]
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .map(|idx| cost_start + idx)
        .unwrap_or(tokens.len());
    if cost_end <= cost_start {
        return Err(CardTextError::ParseError(
            "madness keyword missing mana cost".to_string(),
        ));
    }

    let (total_cost, cost_effects) = parse_activation_cost(&tokens[cost_start..cost_end])?;
    if !cost_effects.is_empty() {
        return Err(CardTextError::ParseError(
            "madness keyword only supports mana cost".to_string(),
        ));
    }
    let mana_cost = total_cost.mana_cost().cloned().ok_or_else(|| {
        CardTextError::ParseError("madness keyword missing mana symbols".to_string())
    })?;

    Ok(Some(AlternativeCastingMethod::Madness { cost: mana_cost }))
}

pub(crate) fn parse_buyback_line(tokens: &[Token]) -> Result<Option<OptionalCost>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("buyback")) {
        return Ok(None);
    }

    // "Buyback costs cost {2} less" is a static cost-modifier line, not the keyword cost line.
    if tokens.get(1).is_some_and(|token| token.is_word("costs")) {
        return Ok(None);
    }

    if tokens.len() <= 1 {
        return Err(CardTextError::ParseError(
            "buyback keyword missing cost".to_string(),
        ));
    }

    let tail = &tokens[1..];
    let reminder_start = tail
        .windows(3)
        .position(|window| {
            window[0].is_word("you") && window[1].is_word("may") && window[2].is_word("pay")
        })
        .or_else(|| {
            tail.windows(2)
                .position(|window| window[0].is_word("you") && window[1].is_word("may"))
        })
        .unwrap_or(tail.len());
    let cost_tokens = trim_commas(&tail[..reminder_start]);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "buyback keyword missing cost".to_string(),
        ));
    }

    let (total_cost, cost_effects) = parse_activation_cost(&cost_tokens)?;
    let total_cost = crate::ability::merge_cost_effects(total_cost, cost_effects);
    Ok(Some(OptionalCost::buyback(total_cost)))
}

pub(crate) fn parse_optional_cost_keyword_line(
    tokens: &[Token],
    keyword: &str,
    constructor: fn(TotalCost) -> OptionalCost,
) -> Result<Option<OptionalCost>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word(keyword)) {
        return Ok(None);
    }

    if tokens.len() <= 1 {
        return Err(CardTextError::ParseError(format!(
            "{keyword} keyword missing cost"
        )));
    }

    let tail = &tokens[1..];
    let reminder_start = tail
        .windows(3)
        .position(|window| {
            window[0].is_word("you") && window[1].is_word("may") && window[2].is_word("pay")
        })
        .or_else(|| {
            tail.windows(2)
                .position(|window| window[0].is_word("you") && window[1].is_word("may"))
        })
        .unwrap_or(tail.len());

    let sentence_end = tail
        .iter()
        .position(|token| matches!(token, Token::Period(_)))
        .unwrap_or(tail.len());

    let end = reminder_start.min(sentence_end);
    let cost_tokens = trim_commas(&tail[..end]);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{keyword} keyword missing cost"
        )));
    }

    let (total_cost, cost_effects) = parse_activation_cost(&cost_tokens)?;
    let total_cost = crate::ability::merge_cost_effects(total_cost, cost_effects);
    Ok(Some(constructor(total_cost)))
}

pub(crate) fn parse_kicker_line(tokens: &[Token]) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "kicker", OptionalCost::kicker)
}

pub(crate) fn parse_multikicker_line(
    tokens: &[Token],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "multikicker", OptionalCost::multikicker)
}

pub(crate) fn parse_entwine_line(tokens: &[Token]) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "entwine", OptionalCost::entwine)
}

pub(crate) fn parse_morph_keyword_line(
    tokens: &[Token],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(first_word) = tokens.first().and_then(Token::as_word) else {
        return Ok(None);
    };

    let is_megamorph = match first_word {
        "morph" => false,
        "megamorph" => true,
        _ => return Ok(None),
    };

    let mut symbols = Vec::new();
    let mut consumed = 1usize;
    for token in &tokens[1..] {
        let Some(word) = token.as_word() else {
            break;
        };
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        symbols.push(symbol);
        consumed += 1;
    }

    if symbols.is_empty() {
        let mechanic = if is_megamorph { "megamorph" } else { "morph" };
        return Err(CardTextError::ParseError(format!(
            "{mechanic} keyword missing mana cost"
        )));
    }

    let trailing_words = words(&tokens[consumed..]);
    if !trailing_words.is_empty() {
        let mechanic = if is_megamorph { "megamorph" } else { "morph" };
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing {mechanic} clause (line: '{}')",
            trailing_words.join(" ")
        )));
    }

    let cost = ManaCost::from_symbols(symbols);
    let label = if is_megamorph { "Megamorph" } else { "Morph" };
    let text = format!("{label} {}", cost.to_oracle());
    let static_ability = if is_megamorph {
        StaticAbility::megamorph(cost)
    } else {
        StaticAbility::morph(cost)
    };

    Ok(Some(ParsedAbility {
        ability: Ability::static_ability(static_ability).with_text(&text),
        effects_ast: None,
    }))
}

pub(crate) fn parse_escape_line(
    tokens: &[Token],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("escape")) {
        return Ok(None);
    }

    let cost_start = 1;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }

    let comma_idx = tokens[cost_start..]
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .map(|idx| cost_start + idx)
        .ok_or_else(|| {
            CardTextError::ParseError("escape keyword missing exile clause separator".to_string())
        })?;
    if comma_idx <= cost_start {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }

    let (total_cost, cost_effects) = parse_activation_cost(&tokens[cost_start..comma_idx])?;
    if !cost_effects.is_empty() {
        return Err(CardTextError::ParseError(
            "escape keyword only supports mana cost".to_string(),
        ));
    }
    let mana_cost = total_cost.mana_cost().cloned().ok_or_else(|| {
        CardTextError::ParseError("escape keyword missing mana symbols".to_string())
    })?;

    let tail_tokens = trim_commas(&tokens[comma_idx + 1..]);
    if tail_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "escape keyword missing exile clause".to_string(),
        ));
    }

    let tail_words = words(&tail_tokens);
    if tail_words.first().copied() != Some("exile") {
        return Err(CardTextError::ParseError(format!(
            "unsupported escape clause tail (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    let (exile_count, used) = parse_number(&tail_tokens[1..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "escape keyword missing exile count (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;
    let mut idx = 1 + used;
    if tail_words.get(idx).copied() == Some("other") {
        idx += 1;
    }
    if !matches!(tail_words.get(idx).copied(), Some("card") | Some("cards")) {
        return Err(CardTextError::ParseError(format!(
            "escape keyword missing exiled card noun (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    idx += 1;
    if tail_words.get(idx..idx + 3) != Some(&["from", "your", "graveyard"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported escape clause tail (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    if idx + 3 != tail_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing escape clause segment (clause: '{}')",
            tail_words.join(" ")
        )));
    }

    Ok(Some(AlternativeCastingMethod::Escape {
        cost: Some(mana_cost),
        exile_count,
    }))
}

pub(crate) fn parse_cycling_search_filter(
    tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let keyword = words
        .last()
        .copied()
        .ok_or_else(|| CardTextError::ParseError("missing cycling keyword".to_string()))?;
    let mut filter = ObjectFilter::default();

    for word in &words[..words.len().saturating_sub(1)] {
        if let Some(supertype) = parse_supertype_word(word)
            && !filter.supertypes.contains(&supertype)
        {
            filter.supertypes.push(supertype);
        }
        if let Some(card_type) = parse_card_type(word)
            && !filter.card_types.contains(&card_type)
        {
            filter.card_types.push(card_type);
        }
        if let Some(subtype) = parse_subtype_flexible(word)
            && !filter.subtypes.contains(&subtype)
        {
            filter.subtypes.push(subtype);
            if is_land_subtype(subtype) && !filter.card_types.contains(&CardType::Land) {
                filter.card_types.push(CardType::Land);
            }
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
    }

    if keyword == "cycling" {
        return Ok(None);
    }

    if keyword == "landcycling" {
        if !filter.card_types.contains(&CardType::Land) {
            filter.card_types.push(CardType::Land);
        }
        return Ok(Some(filter));
    }

    if let Some(root) = keyword.strip_suffix("cycling") {
        if let Some(card_type) = parse_card_type(root)
            && !filter.card_types.contains(&card_type)
        {
            filter.card_types.push(card_type);
        } else if let Some(subtype) = parse_subtype_flexible(root) {
            if !filter.subtypes.contains(&subtype) {
                filter.subtypes.push(subtype);
            }
            if is_land_subtype(subtype) && !filter.card_types.contains(&CardType::Land) {
                filter.card_types.push(CardType::Land);
            }
        } else if let Some(color) = parse_color(root) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported cycling variant (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(Some(filter));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported cycling variant (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn is_land_subtype(subtype: Subtype) -> bool {
    matches!(
        subtype,
        Subtype::Plains
            | Subtype::Island
            | Subtype::Swamp
            | Subtype::Mountain
            | Subtype::Forest
            | Subtype::Desert
    )
}

pub(crate) fn parse_equip_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let equip_end = tokens
        .iter()
        .position(|token| matches!(token, Token::Period(_)))
        .unwrap_or(tokens.len());
    let tokens = &tokens[..equip_end];
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("equip") {
        return Ok(None);
    }

    let mut symbols = Vec::new();
    let mut saw_zero = false;
    let mut saw_non_symbol = false;
    for word in clause_words.iter().skip(1) {
        if let Ok(symbol) = parse_mana_symbol(word) {
            if matches!(symbol, ManaSymbol::Generic(0)) {
                saw_zero = true;
            } else {
                symbols.push(symbol);
            }
        } else {
            saw_non_symbol = true;
        }
    }

    if saw_non_symbol {
        let cost_tokens = trim_commas(&tokens[1..]);
        if cost_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "equip missing activation cost".to_string(),
            ));
        }
        let (total_cost, cost_effects) = parse_activation_cost(&cost_tokens)?;
        let total_cost = crate::ability::merge_cost_effects(total_cost, cost_effects);
        let tail_words = words(&cost_tokens);
        if tail_words.is_empty() {
            return Err(CardTextError::ParseError(
                "equip missing activation cost".to_string(),
            ));
        }
        let equip_text = format!("Equip—{}", keyword_title(&tail_words.join(" ")));
        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));

        return Ok(Some(ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost: total_cost,
                    effects: vec![Effect::attach_to(target.clone())],
                    choices: vec![target.clone()],
                    timing: ActivationTiming::SorcerySpeed,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(equip_text),
            },
            effects_ast: None,
        }));
    }

    if symbols.is_empty() && !saw_zero {
        return Err(CardTextError::ParseError(
            "equip missing mana cost".to_string(),
        ));
    }

    let mana_cost = if symbols.is_empty() {
        ManaCost::new()
    } else {
        let pips = symbols.into_iter().map(|symbol| vec![symbol]).collect();
        ManaCost::from_pips(pips)
    };
    let total_cost = if mana_cost.pips().is_empty() {
        TotalCost::free()
    } else {
        TotalCost::mana(mana_cost)
    };
    let equip_text = if saw_zero && total_cost.costs().is_empty() {
        "Equip {0}".to_string()
    } else if let Some(mana) = total_cost.mana_cost() {
        format!("Equip {}", mana.to_oracle())
    } else {
        "Equip".to_string()
    };
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: vec![Effect::attach_to(target.clone())],
                choices: vec![target.clone()],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some(equip_text),
        },
        effects_ast: None,
    }))
}

pub(crate) fn parse_activation_cost(
    tokens: &[Token],
) -> Result<(TotalCost, Vec<Effect>), CardTextError> {
    let mut mana_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let cost_effects = Vec::new();
    let mut explicit_costs = Vec::new();
    let mut energy_count: u32 = 0;
    let mut sac_tag_id = 0u32;
    let mut tap_tag_id = 0u32;
    let mut exile_tag_id = 0u32;
    let mut return_tag_id = 0u32;

    if let Some((left, right)) = parse_shard_style_mana_or_tap_cost(tokens) {
        let costs = vec![
            crate::costs::Cost::mana(ManaCost::from_pips(vec![vec![left, right]])),
            crate::costs::Cost::effect(Effect::tap_source()),
        ];
        return Ok((TotalCost::from_costs(costs), cost_effects));
    }

    for raw_segment in split_cost_segments(tokens) {
        if raw_segment.is_empty() {
            continue;
        }
        let mut segment = raw_segment;
        while segment
            .first()
            .is_some_and(|token| token.is_word("and") || token.is_word("or"))
        {
            segment.remove(0);
        }
        if segment.is_empty() {
            continue;
        }

        let segment_words = words(&segment);
        if segment_words.is_empty() {
            continue;
        }

        if segment_words[0] == "tap" || segment_words[0] == "t" {
            if segment_words.len() == 1 {
                explicit_costs.push(crate::costs::Cost::effect(Effect::tap_source()));
                continue;
            }

            let mut idx = 1usize;
            let mut count = 1u32;
            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            } else if segment
                .get(idx)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            {
                idx += 1;
            }

            if !segment
                .get(idx)
                .is_some_and(|token| token.is_word("untapped"))
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported tap cost segment (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            idx += 1;

            let filter_tokens = &segment[idx..];
            if filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing tap-cost filter (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            let mut filter = parse_object_filter(filter_tokens, false)?;
            if filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::You);
            }
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }
            filter.untapped = true;

            let tag = format!("tap_cost_{tap_tag_id}");
            tap_tag_id += 1;
            explicit_costs.push(crate::costs::Cost::effect(Effect::choose_objects(
                filter,
                count as usize,
                PlayerFilter::You,
                tag.clone(),
            )));
            explicit_costs.push(crate::costs::Cost::effect(Effect::tap(ChooseSpec::tagged(
                tag,
            ))));
            continue;
        }

        if segment_words[0] == "pay" {
            if segment_words.contains(&"life") {
                // "Pay N life for each card in your hand." (Hand of Vecna)
                if segment_words.len() == 9
                    && segment_words[2] == "life"
                    && segment_words[3] == "for"
                    && segment_words[4] == "each"
                    && matches!(segment_words[5], "card" | "cards")
                    && segment_words[6] == "in"
                    && segment_words[7] == "your"
                    && segment_words[8] == "hand"
                    && let Some((per_card, used)) = parse_number(&segment[1..])
                    && used == 1
                {
                    explicit_costs.push(crate::costs::Cost::new(
                        crate::costs::LifePerCardInHandCost::new(per_card),
                    ));
                    continue;
                }

                let amount = parse_number(&segment[1..]).ok_or_else(|| {
                    CardTextError::ParseError("unable to parse pay life cost".to_string())
                })?;
                explicit_costs.push(crate::costs::Cost::life(amount.0));
                continue;
            }
            let mut parsed_any = false;
            for token in &segment[1..] {
                let Some(word) = token.as_word() else {
                    continue;
                };
                if word == "e" {
                    energy_count = energy_count.saturating_add(1);
                    parsed_any = true;
                    continue;
                }
                if let Ok(symbol) = parse_mana_symbol(word) {
                    mana_pips.push(vec![symbol]);
                    parsed_any = true;
                }
            }
            if !parsed_any {
                return Err(CardTextError::ParseError(
                    "unsupported pay cost (expected life or mana symbols)".to_string(),
                ));
            }
            continue;
        }

        if segment_words[0] == "behold" {
            let mut idx = 1usize;
            let mut count = 1u32;
            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            } else if segment
                .get(idx)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            {
                idx += 1;
            }

            let subtype_word = segment.get(idx).and_then(Token::as_word).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subtype in behold cost segment (clause: '{}')",
                    segment_words.join(" ")
                ))
            })?;
            let subtype = parse_subtype_word(subtype_word)
                .or_else(|| subtype_word.strip_suffix('s').and_then(parse_subtype_word))
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported subtype in behold cost segment (clause: '{}')",
                        segment_words.join(" ")
                    ))
                })?;

            let trailing_words = words(segment.get(idx + 1..).unwrap_or_default());
            if !trailing_words.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing behold cost segment (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            explicit_costs.push(crate::costs::Cost::effect(Effect::behold(subtype, count)));
            continue;
        }

        if segment_words[0] == "discard" {
            let mut idx = 1usize;
            let mut count = 1u32;

            let after_discard_words = words(&segment[idx..]);
            if after_discard_words.starts_with(&["your", "hand"]) {
                if after_discard_words.len() != 2 {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing discard-hand cost clause (clause: '{}')",
                        segment_words.join(" ")
                    )));
                }
                explicit_costs.push(crate::costs::Cost::effect(Effect::discard_hand()));
                continue;
            }
            if after_discard_words.starts_with(&["this", "card"]) {
                if after_discard_words.len() != 2 {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing discard-source cost clause (clause: '{}')",
                        segment_words.join(" ")
                    )));
                }
                explicit_costs.push(crate::costs::Cost::discard_source());
                continue;
            }

            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            }

            while segment
                .get(idx)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            {
                idx += 1;
            }

            let mut card_types = Vec::<CardType>::new();
            while let Some(token) = segment.get(idx) {
                if token.is_word("card") || token.is_word("cards") {
                    break;
                }
                let Some(word) = token.as_word() else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported discard cost selector (clause: '{}')",
                        segment_words.join(" ")
                    )));
                };
                if word == "and" || word == "or" || word == "a" || word == "an" {
                    idx += 1;
                    continue;
                }
                let Some(parsed_type) = parse_card_type(word) else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported discard cost selector (clause: '{}')",
                        segment_words.join(" ")
                    )));
                };
                if !card_types.contains(&parsed_type) {
                    card_types.push(parsed_type);
                }
                idx += 1;
            }

            if !segment
                .get(idx)
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported discard cost selector (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            idx += 1;

            let trailing_words = words(&segment[idx..]);
            let random = match trailing_words.as_slice() {
                [] => false,
                ["at", "random"] => true,
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing discard cost clause (clause: '{}')",
                        segment_words.join(" ")
                    )));
                }
            };

            if random {
                let card_filter = if card_types.is_empty() {
                    None
                } else {
                    Some(ObjectFilter {
                        card_types,
                        ..Default::default()
                    })
                };
                explicit_costs.push(crate::costs::Cost::effect(Effect::discard_player_filtered(
                    Value::Fixed(count as i32),
                    PlayerFilter::You,
                    true,
                    card_filter,
                )));
            } else if card_types.len() > 1 {
                explicit_costs.push(crate::costs::Cost::discard_types(count, card_types));
            } else if let Some(card_type) = card_types.first().copied() {
                explicit_costs.push(crate::costs::Cost::discard(count, Some(card_type)));
            } else {
                explicit_costs.push(crate::costs::Cost::discard(count, None));
            }
            continue;
        }

        if segment_words[0] == "mill" {
            let mut idx = 1usize;
            let mut count = 1u32;
            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            } else if segment
                .get(idx)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            {
                idx += 1;
            }

            if !segment
                .get(idx)
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported mill cost segment (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            explicit_costs.push(crate::costs::Cost::effect(Effect::mill(count)));
            continue;
        }

        if segment_words[0] == "sacrifice" {
            if segment_words.get(1).copied() == Some("this") {
                explicit_costs.push(crate::costs::Cost::effect(Effect::sacrifice_source()));
                continue;
            }
            let mut idx = 1;
            let mut count_value = Value::Fixed(1);
            let mut choose_count = ChoiceCount::exactly(1);
            let mut other = false;
            if segment.get(idx).is_some_and(|token| token.is_word("x")) {
                count_value = Value::X;
                choose_count = ChoiceCount::dynamic_x();
                idx += 1;
            } else if let Some((value, used)) = parse_number(&segment[idx..]) {
                count_value = Value::Fixed(value as i32);
                choose_count = ChoiceCount::exactly(value as usize);
                idx += used;
            }
            if segment
                .get(idx)
                .is_some_and(|token| token.is_word("another"))
            {
                other = true;
                idx += 1;
            }
            if matches!(count_value, Value::Fixed(1))
                && segment.get(idx).is_some_and(|token| token.is_word("x"))
            {
                count_value = Value::X;
                choose_count = ChoiceCount::dynamic_x();
                idx += 1;
            } else if matches!(count_value, Value::Fixed(1))
                && let Some((value, used)) = parse_number(&segment[idx..])
            {
                count_value = Value::Fixed(value as i32);
                choose_count = ChoiceCount::exactly(value as usize);
                idx += used;
            }
            let filter_tokens = &segment[idx..];
            let mut filter = parse_object_filter(filter_tokens, other)?;
            if filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::You);
            }
            let tag = format!("sacrifice_cost_{sac_tag_id}");
            sac_tag_id += 1;
            explicit_costs.push(crate::costs::Cost::effect(Effect::choose_objects(
                filter,
                choose_count,
                PlayerFilter::You,
                tag.clone(),
            )));
            explicit_costs.push(crate::costs::Cost::effect(Effect::sacrifice(
                ObjectFilter::tagged(tag),
                count_value,
            )));
            continue;
        }

        if segment_words[0] == "exile" {
            let mut idx = 1usize;
            let mut count = 1u32;
            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            }
            while segment
                .get(idx)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            {
                idx += 1;
            }
            let mut color_filter = None;
            if let Some(word) = segment.get(idx).and_then(Token::as_word)
                && let Some(color) = parse_color(word)
            {
                color_filter = Some(color);
                idx += 1;
            }

            let tail_words = words(&segment[idx..]);
            let has_card = tail_words.contains(&"card") || tail_words.contains(&"cards");
            let has_hand = tail_words.contains(&"hand");
            if has_hand && (has_card || contains_source_from_your_hand_phrase(&segment_words)) {
                // "Exile this card from your hand" = exile source (Simian Spirit Guide)
                // "Exile a [color] card from your hand" = choose and exile another card (Force of Will)
                if contains_source_from_your_hand_phrase(&segment_words) {
                    explicit_costs.push(crate::costs::Cost::effect(Effect::exile(
                        ChooseSpec::Source,
                    )));
                } else {
                    explicit_costs.push(crate::costs::Cost::effect(
                        Effect::exile_from_hand_as_cost(count, color_filter),
                    ));
                }
                continue;
            }

            let mut filter_tokens = &segment[1..];
            let mut generic_count = 1usize;
            let mut top_only = false;
            if let Some((value, used)) = parse_number(filter_tokens) {
                generic_count = value as usize;
                filter_tokens = &filter_tokens[used..];
            }
            while filter_tokens.first().is_some_and(|token| {
                token.is_word("the") || token.is_word("a") || token.is_word("an")
            }) {
                filter_tokens = &filter_tokens[1..];
            }
            if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("top"))
            {
                // "Exile the top ... card of your graveyard" should select only the
                // top-most matching object in that ordered zone.
                top_only = true;
                filter_tokens = &filter_tokens[1..];
            }
            while filter_tokens.first().is_some_and(|token| {
                token.is_word("the") || token.is_word("a") || token.is_word("an")
            }) {
                filter_tokens = &filter_tokens[1..];
            }
            if let Some((value, used)) = parse_number(filter_tokens) {
                generic_count = value as usize;
                filter_tokens = &filter_tokens[used..];
            }
            while filter_tokens.first().is_some_and(|token| {
                token.is_word("the") || token.is_word("a") || token.is_word("an")
            }) {
                filter_tokens = &filter_tokens[1..];
            }
            if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
            {
                filter_tokens = &filter_tokens[1..];
            }
            if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("of"))
            {
                filter_tokens = &filter_tokens[1..];
            }
            if filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported exile cost segment (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            let filter_words = words(filter_tokens);
            if filter_words.first().copied() == Some("target") {
                return Err(CardTextError::ParseError(format!(
                    "unsupported targeted exile cost segment (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            if is_source_reference_words(&filter_words)
                || is_source_from_your_graveyard_words(&filter_words)
            {
                explicit_costs.push(crate::costs::Cost::effect(Effect::exile(
                    ChooseSpec::Source,
                )));
                continue;
            }

            let mut filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported exile cost segment (clause: '{}')",
                    segment_words.join(" ")
                ))
            })?;
            if filter == ObjectFilter::default() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported exile cost filter (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            if filter.zone == Some(Zone::Battlefield) && filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::You);
            }
            if filter.zone != Some(Zone::Battlefield)
                && filter.owner.is_none()
                && filter.controller.is_none()
            {
                filter.owner = Some(PlayerFilter::You);
            }

            let tag = format!("exile_cost_{exile_tag_id}");
            exile_tag_id += 1;
            let choice_zone = filter.zone.unwrap_or(Zone::Battlefield);
            let mut choose_effect = crate::effects::ChooseObjectsEffect::new(
                filter,
                generic_count,
                PlayerFilter::You,
                tag.clone(),
            )
            .in_zone(choice_zone);
            if top_only {
                choose_effect = choose_effect.top_only();
            }
            explicit_costs.push(crate::costs::Cost::effect(Effect::new(choose_effect)));
            explicit_costs.push(crate::costs::Cost::effect(Effect::exile(
                ChooseSpec::tagged(tag),
            )));
            continue;
        }

        if segment_words[0] == "put" {
            let (count, used) = parse_number(&segment[1..]).ok_or_else(|| {
                CardTextError::ParseError("unable to parse put counter cost amount".to_string())
            })?;
            let counter_type =
                parse_counter_type_from_tokens(&segment[1 + used..]).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported counter type in activation cost (clause: '{}')",
                        segment_words.join(" ")
                    ))
                })?;
            explicit_costs.push(crate::costs::Cost::add_counters(counter_type, count));
            continue;
        }

        if segment_words[0] == "remove" {
            let mut any_number = false;
            let mut variable_x = false;
            let (count, used) = if segment.get(1).is_some_and(|token| token.is_word("any"))
                && segment.get(2).is_some_and(|token| token.is_word("number"))
            {
                any_number = true;
                let consumed = if segment.get(3).is_some_and(|token| token.is_word("of")) {
                    3
                } else {
                    2
                };
                (u32::MAX / 4, consumed)
            } else if segment.get(1).is_some_and(|token| token.is_word("x")) {
                any_number = true;
                variable_x = true;
                (u32::MAX / 4, 1)
            } else {
                parse_number(&segment[1..]).ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to parse remove counter cost amount".to_string(),
                    )
                })?
            };
            let mut idx = 1 + used;
            let from_idx = segment[idx..]
                .iter()
                .position(|token| token.is_word("from"))
                .map(|offset| idx + offset)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing 'from' in remove-counter cost (clause: '{}')",
                        segment_words.join(" ")
                    ))
                })?;
            let descriptor = &segment[idx..from_idx];
            let has_counter_keyword = descriptor
                .iter()
                .any(|token| token.is_word("counter") || token.is_word("counters"));
            if !has_counter_keyword {
                return Err(CardTextError::ParseError(format!(
                    "missing counter keyword in activation cost (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            let counter_type = parse_counter_type_from_tokens(descriptor);

            idx = from_idx + 1;
            let from_among = segment.get(idx).is_some_and(|token| token.is_word("among"));
            if from_among {
                idx += 1;
                if idx >= segment.len() {
                    return Err(CardTextError::ParseError(format!(
                        "missing filter for remove-counter cost (clause: '{}')",
                        segment_words.join(" ")
                    )));
                }

                let mut filter = parse_object_filter(&segment[idx..], false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(Zone::Battlefield);
                }
                let max_count = if any_number { u32::MAX / 4 } else { count };
                explicit_costs.push(crate::costs::Cost::new(
                    crate::costs::RemoveAnyCountersAmongCost::new(max_count, filter)
                        .with_counter_type(counter_type),
                ));
                continue;
            }

            if idx >= segment.len() {
                return Err(CardTextError::ParseError(format!(
                    "missing filter for remove-counter cost (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            let remaining = &segment[idx..];
            let remaining_words = words(remaining);
            let from_source = is_source_reference_words(&remaining_words);
            if from_source {
                let counter_type = counter_type.ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported counter type in activation cost (clause: '{}')",
                        segment_words.join(" ")
                    ))
                })?;
                if any_number {
                    let source_cost = if variable_x {
                        crate::costs::RemoveAnyCountersFromSourceCost::x(Some(counter_type))
                    } else {
                        crate::costs::RemoveAnyCountersFromSourceCost::any_number(Some(
                            counter_type,
                        ))
                    };
                    explicit_costs.push(crate::costs::Cost::new(source_cost));
                } else {
                    explicit_costs.push(crate::costs::Cost::remove_counters(counter_type, count));
                }
            } else {
                let mut filter = parse_object_filter(remaining, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(Zone::Battlefield);
                }
                let max_count = if any_number { u32::MAX / 4 } else { count };
                explicit_costs.push(crate::costs::Cost::new(
                    crate::costs::RemoveAnyCountersAmongCost::new(max_count, filter)
                        .with_counter_type(counter_type),
                ));
            }
            continue;
        }

        if segment_words[0] == "return" {
            let mut idx = 1usize;
            let mut count = 1u32;
            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            }

            let to_idx = segment
                .iter()
                .position(|token| token.is_word("to"))
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported return cost segment (clause: '{}')",
                        segment_words.join(" ")
                    ))
                })?;
            let target_tokens = trim_commas(&segment[idx..to_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return-cost target (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            let destination_words = words(&segment[to_idx + 1..]);
            if !destination_words.contains(&"hand") {
                return Err(CardTextError::ParseError(format!(
                    "unsupported return-cost destination (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            let mut filter = parse_object_filter(&target_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported return-cost target filter (clause: '{}')",
                    segment_words.join(" ")
                ))
            })?;
            if filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::You);
            }
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }

            let tag = format!("return_cost_{return_tag_id}");
            return_tag_id += 1;
            explicit_costs.push(crate::costs::Cost::effect(Effect::choose_objects(
                filter,
                count as usize,
                PlayerFilter::You,
                tag.clone(),
            )));
            explicit_costs.push(crate::costs::Cost::effect(Effect::return_to_hand(
                ObjectFilter::tagged(tag),
            )));
            continue;
        }

        // Otherwise, treat as pure mana symbols.
        for word in &segment_words {
            if *word == "e" {
                energy_count = energy_count.saturating_add(1);
                continue;
            }
            if *word == "q" {
                // {Q} is the untap symbol.
                explicit_costs.push(crate::costs::Cost::untap());
                continue;
            }
            if word.contains('/') {
                let alternatives = parse_mana_symbol_group(word)?;
                mana_pips.push(alternatives);
                continue;
            }
            if let Ok(symbol) = parse_mana_symbol(word) {
                mana_pips.push(vec![symbol]);
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported activation cost segment (clause: '{}')",
                segment_words.join(" ")
            )));
        }
    }

    let mut costs = Vec::new();
    if !mana_pips.is_empty() {
        costs.push(crate::costs::Cost::mana(ManaCost::from_pips(mana_pips)));
    }
    if energy_count > 0 {
        costs.push(crate::costs::Cost::energy(energy_count));
    }
    costs.extend(explicit_costs);

    let total_cost = if costs.is_empty() {
        TotalCost::free()
    } else {
        TotalCost::from_costs(costs)
    };

    Ok((total_cost, cost_effects))
}

fn parse_shard_style_mana_or_tap_cost(tokens: &[Token]) -> Option<(ManaSymbol, ManaSymbol)> {
    let words = words(tokens);
    let is_tap_word = |word: &str| matches!(word, "t" | "tap");
    if words.len() != 5 || !is_tap_word(words[1]) || words[2] != "or" || !is_tap_word(words[4]) {
        return None;
    }

    let left = parse_mana_symbol(words[0]).ok()?;
    let right = parse_mana_symbol(words[3]).ok()?;
    Some((left, right))
}

pub(crate) fn parse_devotion_value_from_add_clause(
    tokens: &[Token],
) -> Result<Option<Value>, CardTextError> {
    let words = words(tokens);
    let Some(devotion_idx) = words.iter().position(|word| *word == "devotion") else {
        return Ok(None);
    };

    let player = parse_devotion_player_from_words(&words, devotion_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported devotion player in clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let to_idx = words[devotion_idx + 1..]
        .iter()
        .position(|word| *word == "to")
        .map(|idx| devotion_idx + 1 + idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing color after devotion clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    let color_word = words.get(to_idx + 1).copied().ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing devotion color (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let color_set = parse_color(color_word).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported devotion color '{}' (clause: '{}')",
            color_word,
            words.join(" ")
        ))
    })?;
    let color = color_from_color_set(color_set).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "ambiguous devotion color '{}' (clause: '{}')",
            color_word,
            words.join(" ")
        ))
    })?;

    Ok(Some(Value::Devotion { player, color }))
}

pub(crate) fn parse_devotion_player_from_words(
    words: &[&str],
    devotion_idx: usize,
) -> Option<PlayerFilter> {
    if devotion_idx == 0 {
        return None;
    }
    let left = &words[..devotion_idx];
    if left.ends_with(&["your"]) {
        return Some(PlayerFilter::You);
    }
    if left.ends_with(&["opponent"]) || left.ends_with(&["opponents"]) {
        return Some(PlayerFilter::Opponent);
    }
    if left.ends_with(&["that", "players"]) || left.ends_with(&["that", "player"]) {
        return Some(PlayerFilter::Target(Box::new(PlayerFilter::Any)));
    }
    None
}

pub(crate) fn color_from_color_set(colors: ColorSet) -> Option<crate::color::Color> {
    let mut found = None;
    for color in [
        crate::color::Color::White,
        crate::color::Color::Blue,
        crate::color::Color::Black,
        crate::color::Color::Red,
        crate::color::Color::Green,
    ] {
        if colors.contains(color) {
            if found.is_some() {
                return None;
            }
            found = Some(color);
        }
    }
    found
}

pub(crate) fn parse_activation_condition(tokens: &[Token]) -> Option<crate::ConditionExpr> {
    let line_words = words(tokens);
    if line_words.len() < 5 {
        return None;
    }

    if line_words.starts_with(&["activate", "no", "more", "than"]) {
        let count_word = line_words.get(4)?;
        let count = match *count_word {
            "once" => 1,
            "twice" => 2,
            other => parse_named_number(other)?,
        };
        let mut index = 5usize;
        if line_words
            .get(index)
            .is_some_and(|word| *word == "time" || *word == "times")
        {
            index += 1;
        }
        if line_words.get(index) == Some(&"each") && line_words.get(index + 1) == Some(&"turn") {
            return Some(crate::ConditionExpr::MaxActivationsPerTurn(count));
        }
    }

    if let Some(count) = parse_activation_count_per_turn(&line_words[2..]) {
        return Some(crate::ConditionExpr::MaxActivationsPerTurn(count));
    }
    if line_words.starts_with(&["activate", "only", "as", "an", "instant"])
        || line_words.starts_with(&["activate", "only", "as", "instant"])
    {
        return Some(crate::ConditionExpr::ActivationTiming(
            ActivationTiming::AnyTime,
        ));
    }
    if line_words.starts_with(&["activate", "only", "if", "there", "is"])
        || line_words.starts_with(&["activate", "only", "if", "there", "are"])
    {
        let descriptor_start = 5usize;
        let in_idx = line_words
            .iter()
            .enumerate()
            .skip(descriptor_start)
            .find_map(|(idx, word)| (*word == "in").then_some(idx))?;
        let zone_tail = &line_words[in_idx..];
        let points_to_your_graveyard = zone_tail == ["in", "your", "graveyard"]
            || zone_tail == ["in", "graveyard"]
            || zone_tail == ["in", "the", "graveyard"];
        if !points_to_your_graveyard {
            return None;
        }

        let descriptor_words = &line_words[descriptor_start..in_idx];
        if descriptor_words.is_empty() {
            return None;
        }

        let mut card_types = Vec::new();
        let mut subtypes = Vec::new();
        for word in descriptor_words {
            if let Some(card_type) = parse_card_type(word)
                && !card_types.contains(&card_type)
            {
                card_types.push(card_type);
            }
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !subtypes.contains(&subtype)
            {
                subtypes.push(subtype);
            }
        }

        if card_types.is_empty() && subtypes.is_empty() {
            return None;
        }

        return Some(crate::ConditionExpr::CardInYourGraveyard {
            card_types,
            subtypes,
        });
    }
    if line_words.starts_with(&[
        "activate",
        "only",
        "if",
        "creatures",
        "you",
        "control",
        "have",
        "total",
        "power",
    ]) {
        let threshold_word = line_words.get(9)?;
        let threshold = parse_cardinal_u32(threshold_word)?;
        let tail = &line_words[10..];
        if tail == ["or", "greater"] {
            return Some(crate::ConditionExpr::ControlCreaturesTotalPowerAtLeast(
                threshold,
            ));
        }
        return None;
    }
    if !line_words.starts_with(&["activate", "only", "if", "you", "control"]) {
        return None;
    }

    let after_control = &tokens[5..];
    let control_tail = words(after_control);
    if control_tail.starts_with(&["a", "creature", "with", "power"])
        || control_tail.starts_with(&["creature", "with", "power"])
    {
        let power_idx = control_tail.iter().position(|word| *word == "power")?;
        let threshold = parse_cardinal_u32(control_tail.get(power_idx + 1)?)?;
        let tail = &control_tail[power_idx + 2..];
        if tail == ["or", "greater"] {
            return Some(crate::ConditionExpr::YouControl(
                ObjectFilter::creature().with_power(crate::filter::Comparison::GreaterThanOrEqual(
                    threshold as i32,
                )),
            ));
        }
        return None;
    }
    if let Some((count, used)) = parse_number(after_control) {
        let tail = words(&after_control[used..]);
        if tail == ["or", "more", "artifact"] || tail == ["or", "more", "artifacts"] {
            let mut filter = ObjectFilter::artifact();
            filter.zone = Some(Zone::Battlefield);
            return Some(crate::ConditionExpr::PlayerControlsAtLeast {
                player: PlayerFilter::You,
                filter,
                count,
            });
        }
        if tail == ["or", "more", "land"] || tail == ["or", "more", "lands"] {
            let mut filter = ObjectFilter::default().with_type(crate::types::CardType::Land);
            filter.zone = Some(Zone::Battlefield);
            return Some(crate::ConditionExpr::PlayerControlsAtLeast {
                player: PlayerFilter::You,
                filter,
                count,
            });
        }
    }
    if control_tail == ["an", "artifact"]
        || control_tail == ["a", "artifact"]
        || control_tail == ["artifact"]
        || control_tail == ["artifacts"]
    {
        let mut filter = ObjectFilter::artifact();
        filter.zone = Some(Zone::Battlefield);
        return Some(crate::ConditionExpr::PlayerControlsAtLeast {
            player: PlayerFilter::You,
            filter,
            count: 1,
        });
    }

    let mut subtypes = Vec::new();
    for word in line_words {
        if let Some(subtype) = parse_subtype_flexible(word)
            && !subtypes.contains(&subtype)
        {
            subtypes.push(subtype);
        }
    }

    if subtypes.is_empty() {
        return None;
    }

    let mut combined: Option<crate::ConditionExpr> = None;
    for subtype in subtypes {
        let next = crate::ConditionExpr::YouControl(
            ObjectFilter::default()
                .with_type(crate::types::CardType::Land)
                .with_subtype(subtype),
        );
        combined = Some(match combined {
            Some(existing) => crate::ConditionExpr::Or(Box::new(existing), Box::new(next)),
            None => next,
        });
    }

    combined
}

pub(crate) fn parse_cardinal_u32(word: &str) -> Option<u32> {
    let token = Token::Word(word.to_string(), TextSpan::synthetic());
    parse_number(&[token]).map(|(value, _)| value)
}

pub(crate) fn parse_activation_count_per_turn(words: &[&str]) -> Option<u32> {
    let count = parse_named_number(words.first()?)?;
    let mut index = 1usize;
    if words
        .get(index)
        .is_some_and(|word| *word == "time" || *word == "times")
    {
        index += 1;
    }
    if words.get(index) == Some(&"each") && words.get(index + 1) == Some(&"turn") {
        Some(count)
    } else {
        None
    }
}

pub(crate) fn parse_enters_tapped_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }
    if is_negated_untap_clause(&clause_words) {
        let has_enters_tapped =
            clause_words.contains(&"enters") && clause_words.contains(&"tapped");
        if has_enters_tapped {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if clause_words.first().copied() == Some("this")
        && clause_words.contains(&"enters")
        && clause_words.contains(&"tapped")
    {
        let tapped_word_idx = clause_words
            .iter()
            .position(|word| *word == "tapped")
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing tapped keyword in enters-tapped clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let tapped_token_idx =
            token_index_for_word_index(tokens, tapped_word_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map tapped keyword in enters-tapped clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let trailing_words = words(&tokens[tapped_token_idx + 1..]);
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(StaticAbility::enters_tapped_ability()));
    }
    Ok(None)
}

pub(crate) fn parse_cast_this_spell_only_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if !line_words.starts_with(&["cast", "this", "spell", "only"]) {
        return Ok(None);
    }

    let tail = &line_words[4..];
    let declare_attackers_tails: &[&[&str]] = &[
        &["during", "the", "declare", "attackers", "step"],
        &["during", "declare", "attackers", "step"],
    ];
    let declare_attackers_if_attacked_tails: &[&[&str]] = &[
        &[
            "during",
            "the",
            "declare",
            "attackers",
            "step",
            "and",
            "only",
            "if",
            "youve",
            "been",
            "attacked",
            "this",
            "step",
        ],
        &[
            "during",
            "declare",
            "attackers",
            "step",
            "and",
            "only",
            "if",
            "youve",
            "been",
            "attacked",
            "this",
            "step",
        ],
    ];

    if declare_attackers_tails.contains(&tail) {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::during_declare_attackers_step(),
            "Cast this spell only during the declare attackers step.",
        )));
    }

    if declare_attackers_if_attacked_tails.contains(&tail) {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::during_declare_attackers_step_if_you_were_attacked_this_step(),
            "Cast this spell only during the declare attackers step and only if you've been attacked this step.",
        )));
    }

    if tail == ["during", "combat"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat(),
            "Cast this spell only during combat.",
        )));
    }

    if tail == ["during", "combat", "before", "blockers", "are", "declared"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_before_blockers_are_declared(),
            "Cast this spell only during combat before blockers are declared.",
        )));
    }

    if tail == ["during", "combat", "after", "blockers", "are", "declared"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_after_blockers_are_declared(),
            "Cast this spell only during combat after blockers are declared.",
        )));
    }

    if tail
        == [
            "during", "combat", "on", "your", "turn", "before", "blockers", "are", "declared",
        ]
    {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_on_your_turn_before_blockers_are_declared(),
            "Cast this spell only during combat on your turn before blockers are declared.",
        )));
    }

    if tail == ["during", "combat", "on", "an", "opponents", "turn"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_on_opponents_turn(
            ),
            "Cast this spell only during combat on an opponent's turn.",
        )));
    }

    if tail == ["before", "attackers", "are", "declared"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::before_attackers_are_declared(),
            "Cast this spell only before attackers are declared.",
        )));
    }

    if tail == ["before", "the", "combat", "damage", "step"]
        || tail == ["before", "combat", "damage", "step"]
    {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::before_combat_damage_step(),
            "Cast this spell only before the combat damage step.",
        )));
    }

    if tail == ["during", "an", "opponents", "upkeep"] || tail == ["during", "opponents", "upkeep"]
    {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::during_opponents_upkeep(),
            "Cast this spell only during an opponent's upkeep.",
        )));
    }

    if tail
        == [
            "during",
            "an",
            "opponents",
            "turn",
            "after",
            "their",
            "upkeep",
            "step",
        ]
    {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::during_opponents_turn_after_upkeep(),
            "Cast this spell only during an opponent's turn after their upkeep step.",
        )));
    }

    if tail == ["during", "your", "end", "step"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::during_your_end_step(),
            "Cast this spell only during your end step.",
        )));
    }

    if tail == ["if", "youve", "cast", "another", "spell", "this", "turn"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_cast_another_spell_this_turn(),
            "Cast this spell only if you've cast another spell this turn.",
        )));
    }

    if tail
        == [
            "if", "youve", "cast", "another", "green", "spell", "this", "turn",
        ]
    {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_cast_another_green_spell_this_turn(),
            "Cast this spell only if you've cast another green spell this turn.",
        )));
    }

    if tail
        == [
            "if", "an", "opponent", "cast", "a", "creature", "spell", "this", "turn",
        ]
    {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::if_opponent_cast_creature_spell_this_turn(),
            "Cast this spell only if an opponent cast a creature spell this turn.",
        )));
    }

    if tail == ["if", "a", "creature", "is", "attacking", "you"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::if_creature_is_attacking_you(),
            "Cast this spell only if a creature is attacking you.",
        )));
    }

    if tail == ["after", "combat"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::after_combat(),
            "Cast this spell only after combat.",
        )));
    }

    if tail
        == [
            "if",
            "no",
            "permanents",
            "named",
            "tidal",
            "influence",
            "are",
            "on",
            "the",
            "battlefield",
        ]
    {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::if_no_permanents_named_on_battlefield("Tidal Influence"),
            "Cast this spell only if no permanents named Tidal Influence are on the battlefield.",
        )));
    }

    if tail == ["if", "you", "control", "a", "snow", "land"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_snow_land(),
            "Cast this spell only if you control a snow land.",
        )));
    }

    if tail
        == [
            "if",
            "you",
            "control",
            "fewer",
            "creatures",
            "than",
            "each",
            "opponent",
        ]
    {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_fewer_creatures_than_each_opponent(),
            "Cast this spell only if you control fewer creatures than each opponent.",
        )));
    }

    if tail == ["if", "you", "control", "two", "or", "more", "doctors"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_subtype_or_more(
                Subtype::Doctor,
                2,
            ),
            "Cast this spell only if you control two or more Doctors.",
        )));
    }

    if tail == ["if", "you", "control", "two", "or", "more", "vampires"] {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_subtype_or_more(
                Subtype::Vampire,
                2,
            ),
            "Cast this spell only if you control two or more Vampires.",
        )));
    }

    Ok(None)
}

pub(crate) fn parse_cost_reduction_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    let has_commander_cast_count_clause = line_words
        .windows(3)
        .any(|window| window == ["for", "each", "time"])
        && line_words.contains(&"cast")
        && line_words.contains(&"commander")
        && line_words
            .windows(4)
            .any(|window| window == ["from", "the", "command", "zone"]);
    if has_commander_cast_count_clause {
        return Err(CardTextError::ParseError(format!(
            "unsupported commander-cast-count static clause (clause: '{}')",
            line_words.join(" ")
        )));
    }
    if line_words.starts_with(&["this", "cost", "is", "reduced", "by"]) && line_words.len() > 6 {
        let amount_tokens = trim_commas(&tokens[5..]);
        let parsed_amount = parse_cost_modifier_amount(&amount_tokens);
        let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
        let amount_fixed = if let Value::Fixed(value) = amount_value {
            value
        } else {
            1
        };
        let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
        let remaining_words = words(remaining_tokens);
        if remaining_words.contains(&"for")
            && remaining_words.contains(&"each")
            && let Some(dynamic) = parse_dynamic_cost_modifier_value(remaining_tokens)?
        {
            let reduction = scale_dynamic_cost_modifier_value(dynamic, amount_fixed);
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    reduction,
                    crate::static_abilities::ThisSpellCostCondition::Always,
                ),
            )));
        }

        let amount_word = line_words[5];
        let amount_text = if amount_word.chars().all(|ch| ch.is_ascii_digit()) {
            format!("{{{amount_word}}}")
        } else {
            amount_word.to_string()
        };
        let tail = line_words[6..].join(" ");
        let text = format!("This cost is reduced by {amount_text} {tail}");
        return Ok(Some(StaticAbility::rule_text_placeholder(text)));
    }

    if line_words.starts_with(&["activated", "abilities", "of"]) {
        let Some(cost_idx) = line_words
            .iter()
            .position(|word| *word == "cost" || *word == "costs")
        else {
            return Ok(None);
        };
        if cost_idx <= 3 {
            return Ok(None);
        }
        let subject_tokens = trim_commas(&tokens[3..cost_idx]);
        if subject_tokens.is_empty() {
            return Ok(None);
        }
        let mut filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported activated-ability cost reduction subject (clause: '{}')",
                line_words.join(" ")
            ))
        })?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }

        let amount_tokens = trim_commas(&tokens[cost_idx + 1..]);
        let Some((amount_value, used)) = parse_cost_modifier_amount(&amount_tokens) else {
            return Ok(None);
        };
        let reduction = match amount_value {
            Value::Fixed(value) if value > 0 => value as u32,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported activated-ability cost reduction amount (clause: '{}')",
                    line_words.join(" ")
                )));
            }
        };
        let tail_words = words(&amount_tokens[used..]);
        if !tail_words.starts_with(&["less", "to", "activate"]) {
            return Ok(None);
        }

        return Ok(Some(StaticAbility::reduce_activated_ability_costs(
            filter,
            reduction,
            Some(1),
        )));
    }

    if !line_words.starts_with(&["this", "spell", "costs"]) {
        return Ok(None);
    }

    let costs_idx = tokens
        .iter()
        .position(|token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = &tokens[costs_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
    let amount_fixed = if let Value::Fixed(value) = amount_value {
        value
    } else {
        1
    };

    let remaining_tokens = &tokens[costs_idx + 1 + used..];
    let remaining_words: Vec<&str> = words(remaining_tokens);

    if !remaining_words.contains(&"less") {
        return Ok(None);
    }

    if let Some(dynamic) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
        let reduction =
            crate::static_abilities::CostReduction::new(ObjectFilter::default(), dynamic);
        return Ok(Some(StaticAbility::new(reduction)));
    }

    if parsed_amount.is_none() {
        return Ok(None);
    }

    let has_each = remaining_words.contains(&"each");
    let has_card_type = remaining_words
        .windows(2)
        .any(|pair| pair == ["card", "type"]);
    let has_graveyard = remaining_words.contains(&"graveyard");

    if has_each && has_card_type && has_graveyard {
        if amount_fixed != 1 {
            return Ok(None);
        }
        let reduction = crate::effect::Value::CardTypesInGraveyard(PlayerFilter::You);
        let cost_reduction =
            crate::static_abilities::CostReduction::new(ObjectFilter::default(), reduction);
        return Ok(Some(StaticAbility::new(cost_reduction)));
    }

    Ok(None)
}

pub(crate) fn scale_dynamic_cost_modifier_value(dynamic: Value, multiplier: i32) -> Value {
    if multiplier <= 0 {
        return Value::Fixed(0);
    }
    if multiplier == 1 {
        return dynamic;
    }
    match dynamic {
        Value::Count(filter) => Value::CountScaled(filter, multiplier),
        Value::CountScaled(filter, factor) => Value::CountScaled(filter, factor * multiplier),
        other => {
            let mut scaled = other.clone();
            for _ in 1..multiplier {
                scaled = Value::Add(Box::new(scaled), Box::new(other.clone()));
            }
            scaled
        }
    }
}

pub(crate) fn parse_all_creatures_able_to_block_source_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = normalize_cant_words(tokens);
    if words.as_slice()
        == [
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "this",
            "creature",
            "do",
            "so",
        ]
        || words.as_slice()
            == [
                "all",
                "creatures",
                "able",
                "to",
                "block",
                "this",
                "do",
                "so",
            ]
    {
        return Ok(Some(StaticAbility::grant_ability(
            ObjectFilter::creature(),
            StaticAbility::must_block(),
        )));
    }
    Ok(None)
}

pub(crate) fn parse_source_must_be_blocked_if_able_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = normalize_cant_words(tokens);
    if words.as_slice() == ["this", "creature", "must", "be", "blocked", "if", "able"]
        || words.as_slice() == ["this", "must", "be", "blocked", "if", "able"]
    {
        return Ok(Some(StaticAbility::restriction(
            crate::effect::Restriction::must_block_specific_attacker(
                ObjectFilter::creature(),
                ObjectFilter::source(),
            ),
            "this creature must be blocked if able".to_string(),
        )));
    }
    Ok(None)
}

pub(crate) fn parse_cant_clauses(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = split_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }
        let shared_subject = find_negation_span(&segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut abilities = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            if find_negation_span(segment).is_none() {
                continue;
            }
            let mut expanded = segment.clone();
            if idx > 0
                && !shared_subject.is_empty()
                && matches!(find_negation_span(segment), Some((0, _)))
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.clone());
                expanded = with_subject;
            } else if idx > 0
                && !shared_subject.is_empty()
                && starts_with_possessive_activated_ability_subject(segment)
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().skip(1).cloned());
                expanded = with_subject;
            }
            let Some(ability) = parse_cant_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    words(segment).join(" ")
                )));
            };
            abilities.push(ability);
        }

        if abilities.is_empty() {
            return Ok(None);
        }
        return Ok(Some(abilities));
    }

    parse_cant_clause(tokens).map(|ability| ability.map(|ability| vec![ability]))
}

pub(crate) fn parse_cant_clause(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    if let Some(rest) = normalized.strip_prefix(&[
        "creatures",
        "cant",
        "attack",
        "you",
        "unless",
        "their",
        "controller",
        "pays",
    ]) && rest.get(1..)
        == Some(&[
            "for",
            "each",
            "creature",
            "they",
            "control",
            "thats",
            "attacking",
            "you",
        ])
    {
        if let Ok(amount) = rest[0].parse::<u32>() {
            return Ok(Some(
                StaticAbility::cant_attack_you_unless_controller_pays_per_attacker(amount),
            ));
        }
    }

    let is_collective_restraint_domain_attack_tax = normalized.starts_with(&[
        "creatures",
        "cant",
        "attack",
        "you",
        "unless",
        "their",
        "controller",
        "pays",
        "x",
        "for",
        "each",
        "creature",
        "they",
        "control",
        "thats",
        "attacking",
        "you",
    ]) && (normalized.ends_with(&[
        "where", "x", "is", "the", "number", "of", "basic", "land", "types", "among", "lands",
        "you", "control",
    ]) || normalized.ends_with(&[
        "where", "x", "is", "the", "number", "of", "basic", "land", "type", "among", "lands",
        "you", "control",
    ]));
    if is_collective_restraint_domain_attack_tax {
        return Ok(Some(
            StaticAbility::cant_attack_you_unless_controller_pays_per_attacker_basic_land_types_among_lands_you_control(),
        ));
    }

    let starts_with_cant_be_blocked_by = normalized
        .starts_with(&["this", "creature", "cant", "be", "blocked", "by"])
        || normalized.starts_with(&["this", "cant", "be", "blocked", "by"])
        || normalized.starts_with(&["cant", "be", "blocked", "by"]);
    if starts_with_cant_be_blocked_by {
        let mut idx =
            if normalized.starts_with(&["this", "creature", "cant", "be", "blocked", "by"]) {
                6
            } else if normalized.starts_with(&["this", "cant", "be", "blocked", "by"]) {
                5
            } else {
                4
            };
        if normalized
            .get(idx)
            .is_some_and(|word| *word == "creature" || *word == "creatures")
        {
            idx += 1;
        }
        if normalized.get(idx) == Some(&"more") && normalized.get(idx + 1) == Some(&"than") {
            let amount_word = normalized.get(idx + 2).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            let amount_tokens = vec![Token::Word(amount_word.to_string(), TextSpan::synthetic())];
            let (max_blockers, used) = parse_number(&amount_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "invalid blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if used != 1 {
                return Err(CardTextError::ParseError(format!(
                    "invalid blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            let noun = normalized.get(idx + 3).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing blocker noun in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if noun != "creature" && noun != "creatures" {
                return Err(CardTextError::ParseError(format!(
                    "unsupported blocker noun in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            if idx + 4 != normalized.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked max-blockers clause tail (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            return Ok(Some(StaticAbility::cant_be_blocked_by_more_than(
                max_blockers as usize,
            )));
        }
        if normalized.get(idx) == Some(&"with") && normalized.get(idx + 1) == Some(&"power") {
            let amount_word = normalized.get(idx + 2).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing power threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            let amount_tokens = vec![Token::Word(amount_word.to_string(), TextSpan::synthetic())];
            let (threshold, used) = parse_number(&amount_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "invalid power threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if used != 1 || normalized.get(idx + 3) != Some(&"or") || idx + 5 != normalized.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked power clause tail (clause: '{}')",
                    normalized.join(" ")
                )));
            }

            return match normalized.get(idx + 4) {
                Some(&"less") => Ok(Some(StaticAbility::cant_be_blocked_by_power_or_less(
                    threshold as i32,
                ))),
                Some(&"greater") | Some(&"more") => Ok(Some(
                    StaticAbility::cant_be_blocked_by_power_or_greater(threshold as i32),
                )),
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked power clause tail (clause: '{}')",
                    normalized.join(" ")
                ))),
            };
        }

        if normalized.get(idx) == Some(&"with")
            && normalized.get(idx + 1) == Some(&"flying")
            && idx + 2 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature()
                        .with_static_ability(crate::static_abilities::StaticAbilityId::Flying),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked by creatures with flying".to_string(),
            )));
        }
        if let Some(color_word) = normalized.get(idx).copied()
            && normalized
                .get(idx + 1)
                .is_some_and(|word| *word == "creature" || *word == "creatures")
            && idx + 2 == normalized.len()
            && let Some(color) = parse_color(color_word)
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().with_colors(crate::color::ColorSet::from(color)),
                    ObjectFilter::source(),
                ),
                format!("this creature can't be blocked by {color_word} creatures"),
            )));
        }

        if normalized
            .get(idx)
            .is_some_and(|word| *word == "wall" || *word == "walls")
            && idx + 1 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().with_subtype(Subtype::Wall),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked by walls".to_string(),
            )));
        }
    }

    let starts_with_cant_be_blocked_except_by = normalized
        .starts_with(&["this", "creature", "cant", "be", "blocked", "except", "by"])
        || normalized.starts_with(&["this", "cant", "be", "blocked", "except", "by"])
        || normalized.starts_with(&["cant", "be", "blocked", "except", "by"]);
    if starts_with_cant_be_blocked_except_by {
        let idx = if normalized
            .starts_with(&["this", "creature", "cant", "be", "blocked", "except", "by"])
        {
            7
        } else if normalized.starts_with(&["this", "cant", "be", "blocked", "except", "by"]) {
            6
        } else {
            5
        };
        if let Some(color_word) = normalized.get(idx)
            && normalized
                .get(idx + 1)
                .is_some_and(|word| *word == "creature" || *word == "creatures")
            && idx + 2 == normalized.len()
            && let Some(color) = parse_color(color_word)
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_colors(crate::color::ColorSet::from(color)),
                    ObjectFilter::source(),
                ),
                format!("this creature can't be blocked except by {color_word} creatures"),
            )));
        }
        if normalized.get(idx) == Some(&"artifact")
            && normalized
                .get(idx + 1)
                .is_some_and(|word| *word == "creature" || *word == "creatures")
            && idx + 2 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_type(CardType::Artifact),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked except by artifact creatures".to_string(),
            )));
        }
        if normalized
            .get(idx)
            .is_some_and(|word| *word == "wall" || *word == "walls")
            && idx + 1 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_subtype(Subtype::Wall),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked except by walls".to_string(),
            )));
        }
    }

    let starts_with_cant_attack_unless_defending_player =
        normalized.starts_with(&[
            "this",
            "creature",
            "cant",
            "attack",
            "unless",
            "defending",
            "player",
        ]) || normalized.starts_with(&["this", "cant", "attack", "unless", "defending", "player"]);
    let cant_attack_unless_cast_creature_spell_tail = normalized.ends_with(&[
        "unless", "youve", "cast", "a", "creature", "spell", "this", "turn",
    ]) || normalized.ends_with(&[
        "unless", "youve", "cast", "creature", "spell", "this", "turn",
    ]);
    let cant_attack_unless_cast_noncreature_spell_tail = normalized.ends_with(&[
        "unless",
        "youve",
        "cast",
        "a",
        "noncreature",
        "spell",
        "this",
        "turn",
    ]) || normalized.ends_with(&[
        "unless",
        "youve",
        "cast",
        "noncreature",
        "spell",
        "this",
        "turn",
    ]);
    if cant_attack_unless_cast_creature_spell_tail
        && (normalized.starts_with(&["this", "creature", "cant", "attack"])
            || normalized.starts_with(&["this", "cant", "attack"]))
    {
        return Ok(Some(
            StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn(),
        ));
    }
    if cant_attack_unless_cast_noncreature_spell_tail
        && (normalized.starts_with(&["this", "creature", "cant", "attack"])
            || normalized.starts_with(&["this", "cant", "attack"]))
    {
        return Ok(Some(
            StaticAbility::cant_attack_unless_controller_cast_noncreature_spell_this_turn(),
        ));
    }

    let starts_with_this_cant_attack_unless = normalized
        .starts_with(&["this", "creature", "cant", "attack", "unless"])
        || normalized.starts_with(&["this", "cant", "attack", "unless"]);
    if starts_with_this_cant_attack_unless {
        let tail = if normalized.starts_with(&["this", "creature", "cant", "attack", "unless"]) {
            &normalized[5..]
        } else {
            &normalized[4..]
        };

        let static_text = format!("Can't attack unless {}", tail.join(" "));
        let static_with = |condition| {
            Ok(Some(StaticAbility::cant_attack_unless_condition(
                condition,
                static_text.clone(),
            )))
        };

        if tail
            == [
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "defending",
                "player",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(
                    ObjectFilter::default().with_type(crate::types::CardType::Creature),
                ),
            );
        }
        if tail
            == [
                "you",
                "control",
                "more",
                "lands",
                "than",
                "defending",
                "player",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(
                    ObjectFilter::default().with_type(crate::types::CardType::Land),
                ),
            );
        }
        if let [
            "you",
            "control",
            "another",
            "creature",
            "with",
            "power",
            amount,
            "or",
            "greater",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::creature().you_control().other().with_power(
                            crate::filter::Comparison::GreaterThanOrEqual(value as i32),
                        ),
                    ),
                ),
            );
        }
        if tail == ["you", "control", "another", "artifact"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::artifact().you_control().other(),
                    ),
                ),
            );
        }
        if tail == ["you", "control", "an", "artifact"] || tail == ["you", "control", "artifact"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(ObjectFilter::artifact().you_control()),
                ),
            );
        }
        if tail == ["you", "control", "a", "knight", "or", "a", "soldier"]
            || tail == ["you", "control", "knight", "or", "soldier"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::Or(
                        Box::new(crate::ConditionExpr::YouControl(
                            ObjectFilter::creature()
                                .you_control()
                                .with_subtype(Subtype::Knight),
                        )),
                        Box::new(crate::ConditionExpr::YouControl(
                            ObjectFilter::creature()
                                .you_control()
                                .with_subtype(Subtype::Soldier),
                        )),
                    ),
                ),
            );
        }
        if let [
            "you",
            "control",
            "a",
            "creature",
            "with",
            "power",
            amount,
            "or",
            "greater",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::creature().you_control().with_power(
                            crate::filter::Comparison::GreaterThanOrEqual(value as i32),
                        ),
                    ),
                ),
            );
        }
        if tail == ["you", "control", "a", "1/1", "creature"]
            || tail == ["you", "control", "1/1", "creature"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::creature()
                            .you_control()
                            .with_power(crate::filter::Comparison::Equal(1))
                            .with_toughness(crate::filter::Comparison::Equal(1)),
                    ),
                ),
            );
        }
        if tail == ["there", "is", "a", "mountain", "on", "the", "battlefield"]
            || tail == ["there", "is", "a", "mountain", "on", "battlefield"]
            || tail == ["there", "is", "mountain", "on", "battlefield"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                    filter: ObjectFilter::default()
                        .with_type(crate::types::CardType::Land)
                        .with_subtype(Subtype::Mountain),
                    count: 1,
                },
            );
        }
        if let [
            "there",
            "are",
            amount,
            "or",
            "more",
            "cards",
            "in",
            "your",
            "graveyard",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerGraveyardHasCardsAtLeast(
                    value,
                ),
            );
        }
        if let [
            "there",
            "are",
            amount,
            "or",
            "more",
            "islands",
            "on",
            "the",
            "battlefield",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                    filter: ObjectFilter::default()
                        .with_type(crate::types::CardType::Land)
                        .with_subtype(Subtype::Island),
                    count: value,
                },
            );
        }
        if tail == ["defending", "player", "is", "poisoned"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::IsPoisoned,
                ),
            );
        }
        if let [
            "defending",
            "player",
            "has",
            amount,
            "or",
            "more",
            "cards",
            "in",
            "their",
            "graveyard",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::HasCardsInGraveyardOrMore(
                        value,
                    ),
                ),
            );
        }
        if tail
            == [
                "defending",
                "player",
                "controls",
                "an",
                "enchantment",
                "or",
                "an",
                "enchanted",
                "permanent",
            ]
            || tail
                == [
                    "defending",
                    "player",
                    "controls",
                    "enchantment",
                    "or",
                    "enchanted",
                    "permanent",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::ControlsEnchantmentOrEnchantedPermanent,
                ),
            );
        }
        if tail == ["defending", "player", "controls", "a", "snow", "land"]
            || tail == ["defending", "player", "controls", "snow", "land"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                        ObjectFilter::default()
                            .with_type(crate::types::CardType::Land)
                            .with_supertype(crate::types::Supertype::Snow),
                    ),
                ),
            );
        }
        if tail
            == [
                "defending",
                "player",
                "controls",
                "a",
                "creature",
                "with",
                "flying",
            ]
            || tail
                == [
                    "defending",
                    "player",
                    "controls",
                    "creature",
                    "with",
                    "flying",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                        ObjectFilter::default()
                            .with_type(crate::types::CardType::Creature)
                            .with_static_ability(crate::static_abilities::StaticAbilityId::Flying),
                    ),
                ),
            );
        }
        if tail == ["defending", "player", "controls", "a", "blue", "permanent"]
            || tail == ["defending", "player", "controls", "blue", "permanent"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                        ObjectFilter::default().with_colors(crate::color::ColorSet::from_color(
                            crate::color::Color::Blue,
                        )),
                    ),
                ),
            );
        }
        if tail == ["at", "least", "two", "other", "creatures", "attack"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::AtLeastNOtherCreaturesAttack(
                        2,
                    ),
                ),
            );
        }
        if tail
            == [
                "a", "creature", "with", "greater", "power", "also", "attacks",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::CreatureWithGreaterPowerAlsoAttacks,
                ),
            );
        }
        if tail == ["a", "black", "or", "green", "creature", "also", "attacks"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::BlackOrGreenCreatureAlsoAttacks,
                ),
            );
        }
        if tail
            == [
                "an", "opponent", "has", "been", "dealt", "damage", "this", "turn",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn,
            );
        }
        if let ["you", "control", amount, "or", "more", "artifacts"] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            let mut filter = ObjectFilter::artifact();
            filter.zone = Some(Zone::Battlefield);
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::PlayerControlsAtLeast {
                        player: PlayerFilter::You,
                        filter,
                        count: value,
                    },
                ),
            );
        }
        if tail == ["you", "sacrifice", "a", "land"] || tail == ["you", "sacrifice", "land"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::SacrificePermanents {
                        filter: ObjectFilter::land(),
                        count: 1,
                    },
                ),
            );
        }
        if let ["you", "sacrifice", amount, "islands"] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::SacrificePermanents {
                        filter: ObjectFilter::land().with_subtype(Subtype::Island),
                        count: value,
                    },
                ),
            );
        }
        if tail
            == [
                "you",
                "return",
                "an",
                "enchantment",
                "you",
                "control",
                "to",
                "its",
                "owners",
                "hand",
            ]
            || tail
                == [
                    "you",
                    "return",
                    "enchantment",
                    "you",
                    "control",
                    "to",
                    "its",
                    "owners",
                    "hand",
                ]
            || tail
                == [
                    "you",
                    "return",
                    "an",
                    "enchantment",
                    "you",
                    "control",
                    "to",
                    "its",
                    "owner",
                    "s",
                    "hand",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::ReturnPermanentsToOwnersHand {
                        filter: ObjectFilter::enchantment(),
                        count: 1,
                    },
                ),
            );
        }
        if tail
            == [
                "you", "pay", "1", "for", "each", "+1/+1", "counter", "on", "it",
            ]
            || tail
                == [
                    "you", "pay", "1", "for", "each", "1/1", "counter", "on", "it",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::PayGenericPerSourceCounter {
                        counter_type: crate::object::CounterType::PlusOnePlusOne,
                        amount_per_counter: 1,
                    },
                ),
            );
        }
        if tail == ["defending", "player", "is", "the", "monarch"]
            || tail == ["defending", "player", "is", "monarch"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::IsMonarch,
                ),
            );
        }
    }

    if starts_with_cant_attack_unless_defending_player {
        let mut idx = if normalized.starts_with(&[
            "this",
            "creature",
            "cant",
            "attack",
            "unless",
            "defending",
            "player",
        ]) {
            7
        } else {
            6
        };

        if !normalized
            .get(idx)
            .is_some_and(|word| *word == "control" || *word == "controls")
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported cant-attack unless clause tail (clause: '{}')",
                normalized.join(" ")
            )));
        }
        idx += 1;

        if normalized
            .get(idx)
            .is_some_and(|word| *word == "a" || *word == "an" || *word == "the")
        {
            idx += 1;
        }

        let subtype_word = normalized.get(idx).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing land subtype in cant-attack unless clause (clause: '{}')",
                normalized.join(" ")
            ))
        })?;
        let subtype = parse_subtype_word(subtype_word)
            .or_else(|| subtype_word.strip_suffix('s').and_then(parse_subtype_word))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported land subtype in cant-attack unless clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;

        if idx + 1 != normalized.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing cant-attack unless clause (clause: '{}')",
                normalized.join(" ")
            )));
        }

        return Ok(Some(StaticAbility::cant_attack_unless_condition(
            crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                    ObjectFilter::land().with_subtype(subtype),
                ),
            ),
            "",
        )));
    }

    if let Some((neg_start, neg_end)) = find_negation_span(tokens) {
        let subject_tokens = trim_commas(&tokens[..neg_start]);
        let remainder_tokens = trim_commas(&tokens[neg_end..]);
        let remainder_words = normalize_cant_words(&remainder_tokens);
        let subject_words = words(&subject_tokens);
        if (subject_words == ["this", "creature"] || subject_words == ["this"])
            && remainder_words.first() == Some(&"block")
            && remainder_words.len() > 1
        {
            let attacker_tokens = trim_commas(&remainder_tokens[1..]);
            let attacker_filter = parse_subject_object_filter(&attacker_tokens)?
                .or_else(|| parse_object_filter(&attacker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported blocker restriction filter (clause: '{}')",
                        normalized.join(" ")
                    ))
                })?;
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::source(),
                    attacker_filter,
                ),
                "this creature can't block those attackers".to_string(),
            )));
        }
        if remainder_words.as_slice() == ["transform"] {
            let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
                return Ok(None);
            };
            let subject_text = words(&subject_tokens).join(" ");
            if subject_text.is_empty() {
                return Ok(None);
            }
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::transform(filter),
                format!("{subject_text} can't transform"),
            )));
        }
    }

    if let Some(parsed) = parse_cant_restriction_clause(tokens)?
        && parsed.target.is_none()
        && matches!(
            parsed.restriction,
            crate::effect::Restriction::GainLife(_)
                | crate::effect::Restriction::SearchLibraries(_)
                | crate::effect::Restriction::CastSpellsMatching(_, _)
                | crate::effect::Restriction::CastMoreThanOneSpellEachTurn(_, _)
                | crate::effect::Restriction::DrawCards(_)
                | crate::effect::Restriction::DrawExtraCards(_)
                | crate::effect::Restriction::ChangeLifeTotal(_)
                | crate::effect::Restriction::LoseGame(_)
                | crate::effect::Restriction::WinGame(_)
                | crate::effect::Restriction::PreventDamage
        )
    {
        let ability = match normalized.as_slice() {
            ["players", "cant", "gain", "life"] => StaticAbility::players_cant_gain_life(),
            ["players", "cant", "search", "libraries"] => StaticAbility::players_cant_search(),
            ["damage", "cant", "be", "prevented"] => StaticAbility::damage_cant_be_prevented(),
            ["you", "cant", "lose", "the", "game"] => StaticAbility::you_cant_lose_game(),
            ["your", "opponents", "cant", "win", "the", "game"] => {
                StaticAbility::opponents_cant_win_game()
            }
            ["your", "life", "total", "cant", "change"] => {
                StaticAbility::your_life_total_cant_change()
            }
            ["your", "opponents", "cant", "cast", "spells"] => {
                StaticAbility::opponents_cant_cast_spells()
            }
            [
                "your",
                "opponents",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => StaticAbility::opponents_cant_draw_extra_cards(),
            _ => StaticAbility::restriction(
                parsed.restriction,
                format_negated_restriction_display(tokens),
            ),
        };
        return Ok(Some(ability));
    }

    let ability = match normalized.as_slice() {
        ["counters", "cant", "be", "put", "on", "this", "permanent"] => {
            StaticAbility::cant_have_counters_placed()
        }
        ["this", "spell", "cant", "be", "countered"] => StaticAbility::cant_be_countered_ability(),
        ["this", "creature", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "creature", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "creature", "cant", "attack", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "this creature can't attack alone".to_string(),
        ),
        ["this", "token", "cant", "attack", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "this token can't attack alone".to_string(),
        ),
        ["this", "cant", "attack", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "this can't attack alone".to_string(),
        ),
        ["this", "token", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "token", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "creature", "cant", "attack", "or", "block"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            "this creature can't attack or block".to_string(),
        ),
        ["this", "token", "cant", "attack", "or", "block"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            "this token can't attack or block".to_string(),
        ),
        ["this", "cant", "attack", "or", "block"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            "this can't attack or block".to_string(),
        ),
        ["this", "creature", "cant", "attack", "or", "block", "alone"] => {
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
                "this creature can't attack or block alone".to_string(),
            )
        }
        ["this", "token", "cant", "attack", "or", "block", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
            "this token can't attack or block alone".to_string(),
        ),
        ["this", "cant", "attack", "or", "block", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
            "this can't attack or block alone".to_string(),
        ),
        ["permanents", "you", "control", "cant", "be", "sacrificed"] => {
            StaticAbility::permanents_you_control_cant_be_sacrificed()
        }
        ["this", "creature", "cant", "be", "blocked"] => StaticAbility::unblockable(),
        ["this", "creature", "cant", "be", "blocked", "this", "turn"] => {
            StaticAbility::unblockable()
        }
        ["this", "cant", "be", "blocked"] => StaticAbility::unblockable(),
        ["this", "cant", "be", "blocked", "this", "turn"] => StaticAbility::unblockable(),
        ["cant", "be", "blocked"] => StaticAbility::unblockable(),
        ["cant", "be", "blocked", "this", "turn"] => StaticAbility::unblockable(),
        _ => {
            if let Some(parsed) = parse_negated_object_restriction_clause(tokens)?
                && parsed.target.is_none()
            {
                return Ok(Some(StaticAbility::restriction(
                    parsed.restriction,
                    format_negated_restriction_display(tokens),
                )));
            }
            return Ok(None);
        }
    };

    Ok(Some(ability))
}

pub(crate) fn format_negated_restriction_display(tokens: &[Token]) -> String {
    let words = words(tokens);
    let mut out = Vec::with_capacity(words.len());
    let mut idx = 0usize;
    while idx < words.len() {
        match (words[idx], words.get(idx + 1).copied()) {
            ("cant", _) => {
                out.push("can't".to_string());
                idx += 1;
            }
            ("can", Some("not")) => {
                out.push("can't".to_string());
                idx += 2;
            }
            ("does", Some("not")) => {
                out.push("doesn't".to_string());
                idx += 2;
            }
            ("do", Some("not")) => {
                out.push("don't".to_string());
                idx += 2;
            }
            ("non", Some("phyrexian")) => {
                out.push("non-phyrexian".to_string());
                idx += 2;
            }
            _ => {
                out.push(words[idx].to_string());
                idx += 1;
            }
        }
    }
    out.join(" ")
}

pub(crate) fn parse_cant_restrictions(
    tokens: &[Token],
) -> Result<Option<Vec<ParsedCantRestriction>>, CardTextError> {
    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = split_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }
        let shared_subject = find_negation_span(&segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut restrictions = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            if find_negation_span(segment).is_none() {
                continue;
            }
            let mut expanded = segment.clone();
            if idx > 0
                && !shared_subject.is_empty()
                && matches!(find_negation_span(segment), Some((0, _)))
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.clone());
                expanded = with_subject;
            } else if idx > 0
                && !shared_subject.is_empty()
                && starts_with_possessive_activated_ability_subject(segment)
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().skip(1).cloned());
                expanded = with_subject;
            }
            let Some(restriction) = parse_cant_restriction_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant restriction segment (clause: '{}')",
                    words(segment).join(" ")
                )));
            };
            restrictions.push(restriction);
        }

        if restrictions.is_empty() {
            return Ok(None);
        }
        return Ok(Some(restrictions));
    }

    parse_cant_restriction_clause(tokens).map(|restriction| restriction.map(|r| vec![r]))
}

pub(crate) fn parse_cant_restriction_clause(
    tokens: &[Token],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let restriction = if let Some(parsed) = parse_cant_cast_restriction_words(&normalized) {
        parsed
    } else {
        match normalized.as_slice() {
            ["players", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::Any),
            ["players", "cant", "search", "libraries"] => {
                Restriction::search_libraries(PlayerFilter::Any)
            }
            ["players", "cant", "draw", "cards"] => Restriction::draw_cards(PlayerFilter::Any),
            [
                "players",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => Restriction::draw_extra_cards(PlayerFilter::Any),
            ["damage", "cant", "be", "prevented"] => Restriction::prevent_damage(),
            ["you", "cant", "lose", "the", "game"] => Restriction::lose_game(PlayerFilter::You),
            ["your", "opponents", "cant", "win", "the", "game"] => {
                Restriction::win_game(PlayerFilter::Opponent)
            }
            ["your", "life", "total", "cant", "change"] => {
                Restriction::change_life_total(PlayerFilter::You)
            }
            [
                "your",
                "opponents",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => Restriction::draw_extra_cards(PlayerFilter::Opponent),
            ["you", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::You),
            ["you", "cant", "search", "libraries"] => {
                Restriction::search_libraries(PlayerFilter::You)
            }
            ["you", "cant", "draw", "cards"] => Restriction::draw_cards(PlayerFilter::You),
            ["opponents", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::Opponent),
            _ => return parse_negated_object_restriction_clause(tokens),
        }
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target: None,
    }))
}

fn parse_cant_cast_restriction_words(words: &[&str]) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;

    if let Some((player, used)) = parse_cant_cast_subject(words) {
        let tail = &words[used..];

        if let Some(spell_filter) = parse_cast_additional_limit_filter(tail) {
            return Some(restriction_from_cast_limit_filter(player, spell_filter));
        }

        if tail.first() != Some(&"cant") {
            return None;
        }
        let cant_tail = &tail[1..];

        if cant_tail == ["cast", "spells"] {
            return Some(Restriction::cast_spells(player));
        }
        if cant_tail == ["cast", "creature", "spells"] {
            return Some(Restriction::cast_creature_spells(player));
        }
        if let Some(spell_filter) = parse_cast_more_than_one_limit_filter(cant_tail) {
            return Some(restriction_from_cast_limit_filter(player, spell_filter));
        }
        return None;
    }

    if let Some(spell_filter) = parse_cast_additional_limit_filter(words) {
        return Some(restriction_from_cast_limit_filter(
            PlayerFilter::Any,
            spell_filter,
        ));
    }

    None
}

fn parse_cant_cast_subject(words: &[&str]) -> Option<(PlayerFilter, usize)> {
    if words.starts_with(&["your", "opponents"]) {
        return Some((PlayerFilter::Opponent, 2));
    }
    if words.starts_with(&["each", "player"]) {
        return Some((PlayerFilter::Any, 2));
    }
    if words.starts_with(&["each", "opponent"]) {
        return Some((PlayerFilter::Opponent, 2));
    }
    match words.first().copied() {
        Some("players") => Some((PlayerFilter::Any, 1)),
        Some("opponents") => Some((PlayerFilter::Opponent, 1)),
        Some("you") => Some((PlayerFilter::You, 1)),
        _ => None,
    }
}

fn parse_cast_more_than_one_limit_filter(words: &[&str]) -> Option<ObjectFilter> {
    if !words.starts_with(&["cast", "more", "than", "one"]) {
        return None;
    }
    let mut idx = 4usize;
    let (spell_filter, consumed) = if words.get(idx) == Some(&"spell") {
        (ObjectFilter::default(), 0usize)
    } else {
        parse_cast_limit_qualifier(&words[idx..])?
    };
    idx += consumed;

    if words.get(idx) != Some(&"spell")
        || words.get(idx + 1) != Some(&"each")
        || words.get(idx + 2) != Some(&"turn")
        || idx + 3 != words.len()
    {
        return None;
    }

    Some(spell_filter)
}

fn parse_cast_additional_limit_filter(words: &[&str]) -> Option<ObjectFilter> {
    let mut idx = 0usize;
    if words.starts_with(&["who", "has"]) {
        idx += 2;
    }

    if words.get(idx) != Some(&"cast") {
        return None;
    }
    idx += 1;
    if words
        .get(idx)
        .is_some_and(|word| *word == "a" || *word == "an")
    {
        idx += 1;
    }

    let (first_filter, first_used) = parse_cast_limit_qualifier(&words[idx..])?;
    idx += first_used;

    if words.get(idx) != Some(&"spell")
        || words.get(idx + 1) != Some(&"this")
        || words.get(idx + 2) != Some(&"turn")
        || words.get(idx + 3) != Some(&"cant")
        || words.get(idx + 4) != Some(&"cast")
        || words.get(idx + 5) != Some(&"additional")
    {
        return None;
    }
    idx += 6;

    let (second_filter, second_used) = parse_cast_limit_qualifier(&words[idx..])?;
    if second_filter != first_filter {
        return None;
    }
    idx += second_used;

    if words.get(idx) != Some(&"spells") || idx + 1 != words.len() {
        return None;
    }

    Some(first_filter)
}

fn parse_cast_limit_qualifier(words: &[&str]) -> Option<(ObjectFilter, usize)> {
    let parse_non_term = |term: &str| -> Option<ObjectFilter> {
        let normalized = term.trim_end_matches('s');
        if let Some(card_type) = parse_card_type(normalized) {
            return Some(ObjectFilter::default().without_type(card_type));
        }
        if let Some(subtype) = parse_subtype_word(normalized) {
            return Some(ObjectFilter::default().without_subtype(subtype));
        }
        None
    };
    let parse_positive_term = |term: &str| -> Option<ObjectFilter> {
        let normalized = term.trim_end_matches('s');
        if let Some(card_type) = parse_card_type(normalized) {
            return Some(ObjectFilter::default().with_type(card_type));
        }
        if let Some(subtype) = parse_subtype_word(normalized) {
            return Some(ObjectFilter::default().with_subtype(subtype));
        }
        None
    };

    if let Some(first) = words.first().copied() {
        if let Some(term) = first
            .strip_prefix("non-")
            .or_else(|| first.strip_prefix("non"))
            && !term.is_empty()
            && let Some(filter) = parse_non_term(term)
        {
            return Some((filter, 1));
        }
    }

    if words.len() >= 2
        && words[0] == "non"
        && let Some(filter) = parse_non_term(words[1])
    {
        return Some((filter, 2));
    }

    if let Some(first) = words.first().copied()
        && let Some(filter) = parse_positive_term(first)
    {
        return Some((filter, 1));
    }

    None
}

fn restriction_from_cast_limit_filter(
    player: PlayerFilter,
    spell_filter: ObjectFilter,
) -> crate::effect::Restriction {
    crate::effect::Restriction::cast_more_than_one_spell_each_turn_matching(player, spell_filter)
}

pub(crate) fn parse_negated_object_restriction_clause(
    tokens: &[Token],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let Some((neg_start, neg_end)) = find_negation_span(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..neg_start]);

    let (filter, target, ability_scope) =
        if let Some(parsed) = parse_activated_ability_subject(&subject_tokens)? {
            (parsed.filter, parsed.target, Some(parsed.scope))
        } else if starts_with_target_indicator(&subject_tokens) {
            let target = parse_target_phrase(&subject_tokens)?;
            let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported target restriction subject (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;
            ensure_it_tagged_constraint(&mut filter);
            (filter, Some(target), None)
        } else if subject_tokens.is_empty() {
            // Supports carried clauses like "... and can't be blocked this turn."
            let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
            (
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                Some(target),
                None,
            )
        } else {
            let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported subject in negated restriction clause (clause: '{}')",
                    words(tokens).join(" ")
                )));
            };
            (filter, None, None)
        };

    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    if remainder_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing restriction tail in negated restriction clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let remainder_words = normalize_cant_words(&remainder_tokens);

    let restriction = match remainder_words.as_slice() {
        ["attack"] => Restriction::attack(filter),
        ["attack", "this", "turn"] => Restriction::attack(filter),
        ["attack", "or", "block"] => Restriction::attack_or_block(filter),
        ["attack", "or", "block", "this", "turn"] => Restriction::attack_or_block(filter),
        ["block"] => Restriction::block(filter),
        ["block", "this", "turn"] => Restriction::block(filter),
        ["be", "blocked"] => Restriction::be_blocked(filter),
        ["be", "blocked", "this", "turn"] => Restriction::be_blocked(filter),
        ["be", "destroyed"] => Restriction::be_destroyed(filter),
        ["be", "regenerated"] => Restriction::be_regenerated(filter),
        ["be", "regenerated", "this", "turn"] => Restriction::be_regenerated(filter),
        ["be", "sacrificed"] => Restriction::be_sacrificed(filter),
        ["be", "countered"] => Restriction::be_countered(filter),
        ["be", "activated"] | ["be", "activated", "this", "turn"] => match ability_scope {
            Some(ActivatedAbilityScope::All) => Restriction::activate_abilities_of(filter),
            Some(ActivatedAbilityScope::TapCostOnly) => {
                Restriction::activate_tap_abilities_of(filter)
            }
            None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    words(tokens).join(" ")
                )));
            }
        },
        ["be", "activated", "unless", "theyre", "mana", "abilities"] => match ability_scope {
            Some(ActivatedAbilityScope::All) => Restriction::activate_non_mana_abilities_of(filter),
            Some(ActivatedAbilityScope::TapCostOnly) | None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    words(tokens).join(" ")
                )));
            }
        },
        ["transform"] => Restriction::transform(filter),
        ["be", "targeted"] => Restriction::be_targeted(filter),
        _ if remainder_words.first() == Some(&"block") && remainder_words.len() > 1 => {
            let attacker_tokens = trim_commas(&remainder_tokens[1..]);
            let attacker_filter = parse_subject_object_filter(&attacker_tokens)?
                .or_else(|| parse_object_filter(&attacker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported negated restriction tail (clause: '{}')",
                        words(tokens).join(" ")
                    ))
                })?;
            Restriction::block_specific_attacker(filter, attacker_filter)
        }
        _ if is_supported_untap_restriction_tail(&remainder_words) => Restriction::untap(filter),
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported negated restriction tail (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivatedAbilityScope {
    All,
    TapCostOnly,
}

#[derive(Debug, Clone)]
struct ParsedActivatedAbilitySubject {
    filter: ObjectFilter,
    target: Option<TargetAst>,
    scope: ActivatedAbilityScope,
}

fn parse_activated_ability_subject(
    tokens: &[Token],
) -> Result<Option<ParsedActivatedAbilitySubject>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = normalize_cant_words(tokens);
    let (owner_word_len, scope) = if subject_words.ends_with(&["activated", "abilities"]) {
        (
            subject_words.len().saturating_sub(2),
            ActivatedAbilityScope::All,
        )
    } else if subject_words.ends_with(&[
        "activated",
        "abilities",
        "with",
        "t",
        "in",
        "their",
        "costs",
    ]) {
        (
            subject_words.len().saturating_sub(7),
            ActivatedAbilityScope::TapCostOnly,
        )
    } else {
        return Ok(None);
    };

    if owner_word_len == 0 {
        return Ok(None);
    }
    let owner_tokens = trim_commas(&tokens[..owner_word_len]);
    if owner_tokens.is_empty() {
        return Ok(None);
    }

    let owner_words = words(&owner_tokens);
    if owner_words.len() == 1 && matches!(owner_words[0], "it" | "its" | "them" | "their") {
        return Ok(Some(ParsedActivatedAbilitySubject {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            target: Some(TargetAst::Tagged(
                TagKey::from(IT_TAG),
                span_from_tokens(tokens),
            )),
            scope,
        }));
    }

    if starts_with_target_indicator(&owner_tokens) {
        let target = parse_target_phrase(&owner_tokens)?;
        let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported target restriction subject (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;
        ensure_it_tagged_constraint(&mut filter);
        return Ok(Some(ParsedActivatedAbilitySubject {
            filter,
            target: Some(target),
            scope,
        }));
    }

    let Some(filter) = parse_subject_object_filter(&owner_tokens)?
        .or_else(|| parse_object_filter(&owner_tokens, false).ok())
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported subject in negated restriction clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    };

    Ok(Some(ParsedActivatedAbilitySubject {
        filter,
        target: None,
        scope,
    }))
}

fn ensure_it_tagged_constraint(filter: &mut ObjectFilter) {
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
}

fn starts_with_possessive_activated_ability_subject(tokens: &[Token]) -> bool {
    let words = normalize_cant_words(tokens);
    words.starts_with(&["its", "activated", "abilities"])
        || words.starts_with(&["their", "activated", "abilities"])
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedCantRestriction {
    pub(crate) restriction: crate::effect::Restriction,
    pub(crate) target: Option<TargetAst>,
}

pub(crate) fn starts_with_target_indicator(tokens: &[Token]) -> bool {
    let mut idx = 0usize;
    if tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number"))
        && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
    {
        idx += 3;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        idx += 2;
        if let Some((_, used)) = parse_number(&tokens[idx..]) {
            idx += used;
        }
    } else if let Some((_, used)) = parse_target_count_range_prefix(&tokens[idx..]) {
        idx += used;
    } else if let Some((_, used)) = parse_number(&tokens[idx..])
        && tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"))
    {
        idx += used;
    } else if tokens.get(idx).is_some_and(|token| token.is_word("x"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        idx += 1;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("on")) {
        idx += 1;
    }

    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        idx += 1;
    }

    tokens.get(idx).is_some_and(|token| token.is_word("target"))
}

pub(crate) fn find_negation_span(tokens: &[Token]) -> Option<(usize, usize)> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(word, "cant" | "cannot") {
            return Some((idx, idx + 1));
        }
        if matches!(word, "doesnt" | "dont") {
            let next_word = tokens.get(idx + 1).and_then(Token::as_word);
            if matches!(next_word, Some("control" | "controls" | "own" | "owns")) {
                continue;
            }
            return Some((idx, idx + 1));
        }
        if (word == "does" || word == "do" || word == "can")
            && tokens.get(idx + 1).is_some_and(|next| next.is_word("not"))
        {
            if (word == "does" || word == "do")
                && matches!(
                    tokens.get(idx + 2).and_then(Token::as_word),
                    Some("control" | "controls" | "own" | "owns")
                )
            {
                continue;
            }
            return Some((idx, idx + 2));
        }
    }
    None
}

pub(crate) fn parse_subject_object_filter(
    tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let target = parse_target_phrase(tokens).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject target phrase (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    Ok(target_ast_to_object_filter(target))
}

pub(crate) fn target_ast_to_object_filter(target: TargetAst) -> Option<ObjectFilter> {
    match target {
        TargetAst::Source(_) => Some(ObjectFilter::source()),
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::Spell(_) => Some(ObjectFilter::spell()),
        TargetAst::Tagged(tag, _) => Some(ObjectFilter::tagged(tag)),
        TargetAst::WithCount(inner, _) => target_ast_to_object_filter(*inner),
        _ => None,
    }
}

pub(crate) fn is_supported_untap_restriction_tail(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }
    if !(words[0] == "untap" || words[0] == "untaps") {
        return false;
    }
    if words.len() == 1 {
        return true;
    }

    let allowed = [
        "untap",
        "untaps",
        "during",
        "its",
        "their",
        "your",
        "controllers",
        "controller",
        "untap",
        "step",
        "next",
        "the",
    ];
    if words.iter().any(|word| !allowed.contains(word)) {
        return false;
    }

    words.contains(&"during") && words.contains(&"step")
}

pub(crate) fn normalize_cant_words(tokens: &[Token]) -> Vec<&str> {
    words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect()
}

pub(crate) fn keyword_title(keyword: &str) -> String {
    let mut words = keyword.split_whitespace();
    let Some(first) = words.next() else {
        return String::new();
    };
    let mut out = String::new();
    let mut first_chars = first.chars();
    if let Some(ch) = first_chars.next() {
        out.push(ch.to_ascii_uppercase());
        out.push_str(first_chars.as_str());
    }
    for word in words {
        out.push(' ');
        out.push_str(word);
    }
    out
}

pub(crate) fn leading_mana_symbols_to_oracle(words: &[&str]) -> Option<(String, usize)> {
    if words.is_empty() {
        return None;
    }
    let mut pips = Vec::new();
    let mut consumed = 0usize;
    for word in words {
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        pips.push(vec![symbol]);
        consumed += 1;
    }
    if consumed == 0 {
        return None;
    }
    Some((ManaCost::from_pips(pips).to_oracle(), consumed))
}

pub(crate) fn marker_keyword_id(keyword: &str) -> Option<&'static str> {
    match keyword {
        "banding" => Some("banding"),
        "fabricate" => Some("fabricate"),
        "foretell" => Some("foretell"),
        "bestow" => Some("bestow"),
        "dash" => Some("dash"),
        "overload" => Some("overload"),
        "soulshift" => Some("soulshift"),
        "adapt" => Some("adapt"),
        "bolster" => Some("bolster"),
        "disturb" => Some("disturb"),
        "echo" => Some("echo"),
        "modular" => Some("modular"),
        "ninjutsu" => Some("ninjutsu"),
        "outlast" => Some("outlast"),
        "scavenge" => Some("scavenge"),
        "suspend" => Some("suspend"),
        "vanishing" => Some("vanishing"),
        "offering" => Some("offering"),
        "soulbond" => Some("soulbond"),
        "unearth" => Some("unearth"),
        "specialize" => Some("specialize"),
        "squad" => Some("squad"),
        "spectacle" => Some("spectacle"),
        "graft" => Some("graft"),
        "backup" => Some("backup"),
        "saddle" => Some("saddle"),
        "fading" => Some("fading"),
        "fuse" => Some("fuse"),
        "plot" => Some("plot"),
        "disguise" => Some("disguise"),
        "tribute" => Some("tribute"),
        "buyback" => Some("buyback"),
        "flashback" => Some("flashback"),
        "rebound" => Some("rebound"),
        _ => None,
    }
}

pub(crate) fn marker_keyword_display(words: &[&str]) -> Option<String> {
    let keyword = words.first().copied()?;
    let title = keyword_title(keyword);

    match keyword {
        "soulshift" | "adapt" | "bolster" | "modular" | "vanishing" | "backup" | "saddle"
        | "fading" | "graft" | "tribute" => {
            let amount = words.get(1)?.parse::<u32>().ok()?;
            Some(format!("{title} {amount}"))
        }
        "bestow" | "dash" | "disturb" | "ninjutsu" | "outlast" | "scavenge" | "unearth"
        | "specialize" | "spectacle" | "plot" | "disguise" | "flashback" | "foretell"
        | "overload" => {
            let (cost, _) = leading_mana_symbols_to_oracle(&words[1..])?;
            Some(format!("{title} {cost}"))
        }
        "echo" => {
            if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[1..]) {
                return Some(format!("Echo {cost}"));
            }
            if words.len() > 1 {
                let payload = words[1..].join(" ");
                let mut chars = payload.chars();
                let Some(first) = chars.next() else {
                    return Some("Echo".to_string());
                };
                let mut normalized = String::new();
                normalized.push(first.to_ascii_uppercase());
                normalized.push_str(chars.as_str());
                return Some(format!("Echo—{normalized}"));
            }
            Some("Echo".to_string())
        }
        "buyback" => {
            if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[1..]) {
                Some(format!("Buyback {cost}"))
            } else if words.len() > 1 {
                Some(format!("Buyback—{}", words[1..].join(" ")))
            } else {
                Some("Buyback".to_string())
            }
        }
        "suspend" => {
            let time = words.get(1)?.parse::<u32>().ok()?;
            let (cost, _) = leading_mana_symbols_to_oracle(&words[2..])?;
            Some(format!("Suspend {time}—{cost}"))
        }
        "rebound" => Some("Rebound".to_string()),
        "squad" => {
            let (cost, _) = leading_mana_symbols_to_oracle(&words[1..])?;
            Some(format!("Squad {cost}"))
        }
        _ => None,
    }
}

pub(crate) fn parse_single_word_keyword_action(word: &str) -> Option<KeywordAction> {
    match word {
        "flying" => Some(KeywordAction::Flying),
        "menace" => Some(KeywordAction::Menace),
        "hexproof" => Some(KeywordAction::Hexproof),
        "haste" => Some(KeywordAction::Haste),
        "improvise" => Some(KeywordAction::Improvise),
        "convoke" => Some(KeywordAction::Convoke),
        "delve" => Some(KeywordAction::Delve),
        "deathtouch" => Some(KeywordAction::Deathtouch),
        "lifelink" => Some(KeywordAction::Lifelink),
        "vigilance" => Some(KeywordAction::Vigilance),
        "trample" => Some(KeywordAction::Trample),
        "reach" => Some(KeywordAction::Reach),
        "defender" => Some(KeywordAction::Defender),
        "flash" => Some(KeywordAction::Flash),
        "phasing" => Some(KeywordAction::Phasing),
        "indestructible" => Some(KeywordAction::Indestructible),
        "shroud" => Some(KeywordAction::Shroud),
        "assist" => Some(KeywordAction::Assist),
        "cipher" => Some(KeywordAction::Marker("cipher")),
        "devoid" => Some(KeywordAction::Devoid),
        "dethrone" => Some(KeywordAction::Dethrone),
        "enlist" => Some(KeywordAction::Enlist),
        "evolve" => Some(KeywordAction::Evolve),
        "extort" => Some(KeywordAction::Extort),
        "haunt" => Some(KeywordAction::Haunt),
        "ingest" => Some(KeywordAction::Ingest),
        "mentor" => Some(KeywordAction::Mentor),
        "training" => Some(KeywordAction::Training),
        "myriad" => Some(KeywordAction::Myriad),
        "partner" => Some(KeywordAction::Partner),
        "populate" => Some(KeywordAction::Marker("populate")),
        "provoke" => Some(KeywordAction::Provoke),
        "ravenous" => Some(KeywordAction::Ravenous),
        "riot" => Some(KeywordAction::Riot),
        "skulk" => Some(KeywordAction::Skulk),
        "sunburst" => Some(KeywordAction::Sunburst),
        "undaunted" => Some(KeywordAction::Undaunted),
        "unleash" => Some(KeywordAction::Unleash),
        "wither" => Some(KeywordAction::Wither),
        "infect" => Some(KeywordAction::Infect),
        "undying" => Some(KeywordAction::Undying),
        "persist" => Some(KeywordAction::Persist),
        "prowess" => Some(KeywordAction::Prowess),
        "exalted" => Some(KeywordAction::Exalted),
        "cascade" => Some(KeywordAction::Cascade),
        "storm" => Some(KeywordAction::Storm),
        "rebound" => Some(KeywordAction::Rebound),
        "ascend" => Some(KeywordAction::Ascend),
        "daybound" => Some(KeywordAction::Daybound),
        "nightbound" => Some(KeywordAction::Nightbound),
        "islandwalk" => Some(KeywordAction::Landwalk(Subtype::Island)),
        "swampwalk" => Some(KeywordAction::Landwalk(Subtype::Swamp)),
        "mountainwalk" => Some(KeywordAction::Landwalk(Subtype::Mountain)),
        "forestwalk" => Some(KeywordAction::Landwalk(Subtype::Forest)),
        "plainswalk" => Some(KeywordAction::Landwalk(Subtype::Plains)),
        "fear" => Some(KeywordAction::Fear),
        "intimidate" => Some(KeywordAction::Intimidate),
        "shadow" => Some(KeywordAction::Shadow),
        "horsemanship" => Some(KeywordAction::Horsemanship),
        "flanking" => Some(KeywordAction::Flanking),
        "changeling" => Some(KeywordAction::Changeling),
        _ => None,
    }
}

pub(crate) fn parse_ability_phrase(tokens: &[Token]) -> Option<KeywordAction> {
    let mut words = words(tokens);
    if words.is_empty() {
        return None;
    }

    if words.first().copied() == Some("and") {
        words.remove(0);
    }

    if words.starts_with(&["cumulative", "upkeep"]) {
        let reminder_start = tokens
            .iter()
            .position(|token| matches!(token, Token::Period(_)))
            .unwrap_or(tokens.len());
        let cost_tokens = trim_commas(&tokens[2..reminder_start]).to_vec();
        let cost_words = super::words(&cost_tokens);

        if cost_words.len() == 3
            && cost_words[0] == "pay"
            && cost_words[2] == "life"
            && let Ok(life_per_counter) = cost_words[1].parse::<u32>()
            && life_per_counter > 0
        {
            return Some(KeywordAction::CumulativeUpkeep {
                mana_symbols_per_counter: Vec::new(),
                life_per_counter,
                text: format!("Cumulative upkeep—Pay {life_per_counter} life"),
            });
        }

        if let Some((cost, consumed)) = leading_mana_symbols_to_oracle(&cost_words)
            && consumed == cost_words.len()
        {
            let mut mana_symbols_per_counter = Vec::new();
            let mut parsed_all = true;
            for word in &cost_words {
                let Ok(symbol) = parse_mana_symbol(word) else {
                    parsed_all = false;
                    break;
                };
                mana_symbols_per_counter.push(symbol);
            }
            if parsed_all && !mana_symbols_per_counter.is_empty() {
                return Some(KeywordAction::CumulativeUpkeep {
                    mana_symbols_per_counter,
                    life_per_counter: 0,
                    text: format!("Cumulative upkeep {cost}"),
                });
            }
        }

        let mut text = "Cumulative upkeep".to_string();
        let tail = &words[2..];
        if !tail.is_empty() {
            if tail.first().copied() == Some("add")
                && let Some((cost, consumed)) = leading_mana_symbols_to_oracle(&tail[1..])
                && consumed + 1 == tail.len()
            {
                text = format!("Cumulative upkeep—Add {cost}");
            } else if let Some((cost, consumed)) = leading_mana_symbols_to_oracle(tail)
                && consumed == tail.len()
            {
                text = format!("Cumulative upkeep {cost}");
            } else if tail.len() == 3
                && tail[1] == "or"
                && let (Some((left, 1)), Some((right, 1))) = (
                    leading_mana_symbols_to_oracle(&tail[..1]),
                    leading_mana_symbols_to_oracle(&tail[2..3]),
                )
            {
                text = format!("Cumulative upkeep {left} or {right}");
            } else {
                text.push(' ');
                text.push_str(&tail.join(" "));
            }
        }
        return Some(KeywordAction::MarkerText(text));
    }

    // Bushido appears as "Bushido N" and is often followed by reminder text.
    if words.first().copied() == Some("bushido") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Bushido(amount));
        }
        return Some(KeywordAction::Marker("bushido"));
    }

    // Bloodthirst appears as "Bloodthirst N" and is often followed by reminder text.
    if words.first().copied() == Some("bloodthirst") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Bloodthirst(amount));
        }
        return Some(KeywordAction::Marker("bloodthirst"));
    }

    // Rampage appears as "Rampage N" and is often followed by reminder text.
    if words.first().copied() == Some("rampage") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Rampage(amount));
        }
        return Some(KeywordAction::Marker("rampage"));
    }

    // Annihilator appears as "Annihilator N" and is often followed by reminder text.
    if words.first().copied() == Some("annihilator") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Annihilator(amount));
        }
        return Some(KeywordAction::Marker("annihilator"));
    }

    // Crew appears as "Crew N" and is often followed by inline restrictions/reminder text.
    if words.first().copied() == Some("crew") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            let has_sorcery_speed = words
                .windows(5)
                .any(|window| window == ["activate", "only", "as", "a", "sorcery"]);

            let has_once_per_turn = words
                .windows(5)
                .any(|window| window == ["activate", "only", "once", "each", "turn"])
                || words
                    .windows(5)
                    .any(|window| window == ["activate", "only", "once", "per", "turn"]);

            let mut additional_restrictions = Vec::new();
            let timing = if has_sorcery_speed {
                if has_once_per_turn {
                    additional_restrictions.push("Activate only once each turn.".to_string());
                }
                ActivationTiming::SorcerySpeed
            } else if has_once_per_turn {
                ActivationTiming::OncePerTurn
            } else {
                ActivationTiming::AnyTime
            };

            return Some(KeywordAction::Crew {
                amount,
                timing,
                additional_restrictions,
            });
        }
        // Fallback: preserve unsupported crew variants as marker text.
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("crew"));
    }

    // Saddle appears as "Saddle N" and is often followed by reminder text.
    // Per CR 702.171a, Saddle can be activated only as a sorcery.
    if words.first().copied() == Some("saddle") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            let has_once_per_turn = words
                .windows(5)
                .any(|window| window == ["activate", "only", "once", "each", "turn"])
                || words
                    .windows(5)
                    .any(|window| window == ["activate", "only", "once", "per", "turn"]);

            let mut additional_restrictions = Vec::new();
            let timing = ActivationTiming::SorcerySpeed;
            if has_once_per_turn {
                additional_restrictions.push("Activate only once each turn.".to_string());
            }

            return Some(KeywordAction::Saddle {
                amount,
                timing,
                additional_restrictions,
            });
        }
        // Fallback: preserve unsupported saddle variants as marker text.
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("saddle"));
    }

    // Afterlife appears as "Afterlife N" and is often followed by reminder text.
    if words.first().copied() == Some("afterlife") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Afterlife(amount));
        }
        return Some(KeywordAction::Marker("afterlife"));
    }

    // Fabricate appears as "Fabricate N" and is often followed by reminder text.
    if words.first().copied() == Some("fabricate") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Fabricate(amount));
        }
        return Some(KeywordAction::Marker("fabricate"));
    }

    if words.first().copied() == Some("evolve") {
        return Some(KeywordAction::Evolve);
    }

    if words.first().copied() == Some("mentor") {
        return Some(KeywordAction::Mentor);
    }

    if words.first().copied() == Some("training") {
        return Some(KeywordAction::Training);
    }

    if words.first().copied() == Some("soulbond") {
        return Some(KeywordAction::Soulbond);
    }

    // Renown appears as "Renown N" and is often followed by reminder text.
    if words.first().copied() == Some("renown") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Renown(amount));
        }
        return Some(KeywordAction::Marker("renown"));
    }

    if words.first().copied() == Some("soulshift") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Soulshift(amount));
        }
        return Some(KeywordAction::Marker("soulshift"));
    }

    if words.first().copied() == Some("outlast") {
        if let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[1..])
            && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
        {
            return Some(KeywordAction::Outlast(cost));
        }
        return Some(KeywordAction::Marker("outlast"));
    }

    if words.first().copied() == Some("unearth") {
        if let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[1..])
            && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
        {
            return Some(KeywordAction::Unearth(cost));
        }
        return Some(KeywordAction::Marker("unearth"));
    }

    if words.first().copied() == Some("ninjutsu") {
        if let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[1..])
            && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
        {
            return Some(KeywordAction::Ninjutsu(cost));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Marker("ninjutsu"));
        }
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("ninjutsu"));
    }

    if words.first().copied() == Some("foretell") {
        if let Some((display, _consumed)) = leading_mana_symbols_to_oracle(&words[1..]) {
            return Some(KeywordAction::MarkerText(format!("Foretell {display}")));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Marker("foretell"));
        }
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("foretell"));
    }

    if words.first().copied() == Some("overload") {
        if let Some((display, _consumed)) = leading_mana_symbols_to_oracle(&words[1..]) {
            return Some(KeywordAction::MarkerText(format!("Overload {display}")));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Marker("overload"));
        }
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("overload"));
    }

    if words.starts_with(&["umbra", "armor"]) {
        return Some(KeywordAction::MarkerText("Umbra armor".to_string()));
    }

    if words.first().copied() == Some("echo") {
        if let Some((cost_text, consumed)) = leading_mana_symbols_to_oracle(&words[1..])
            && consumed > 0
            && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
        {
            return Some(KeywordAction::Echo {
                mana_cost: Some(cost),
                cost_effects: Vec::new(),
                text: format!("Echo {cost_text}"),
            });
        }

        let reminder_start = tokens
            .iter()
            .position(|token| matches!(token, Token::Period(_)))
            .or_else(|| {
                tokens
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find_map(|(idx, token)| token.is_word("at").then_some(idx))
            })
            .unwrap_or(tokens.len());
        let cost_tokens = trim_commas(&tokens[1..reminder_start]).to_vec();

        if !cost_tokens.is_empty()
            && let Ok((total_cost, _)) = parse_activation_cost(&cost_tokens)
        {
            let (mana_cost, cost_effects) = alternative_cast_parts_from_total_cost(&total_cost);
            let text = if let Some(cost) = mana_cost.as_ref()
                && cost_effects.is_empty()
            {
                format!("Echo {}", cost.to_oracle())
            } else {
                let payload = cost_tokens
                    .iter()
                    .filter_map(Token::as_word)
                    .collect::<Vec<_>>()
                    .join(" ");
                if payload.is_empty() {
                    "Echo".to_string()
                } else {
                    let mut chars = payload.chars();
                    let first = chars.next().expect("payload is not empty");
                    let mut normalized = String::new();
                    normalized.push(first.to_ascii_uppercase());
                    normalized.push_str(chars.as_str());
                    format!("Echo—{normalized}")
                }
            };
            return Some(KeywordAction::Echo {
                mana_cost,
                cost_effects,
                text,
            });
        }

        if words.len() == 1 {
            return Some(KeywordAction::Marker("echo"));
        }
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("echo"));
    }

    if words.first().copied() == Some("modular") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Modular(amount));
        }
        return Some(KeywordAction::Marker("modular"));
    }

    if words.first().copied() == Some("graft") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Graft(amount));
        }
        return Some(KeywordAction::Marker("graft"));
    }

    if words.first().copied() == Some("fading") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Fading(amount));
        }
        return Some(KeywordAction::Marker("fading"));
    }

    if words.first().copied() == Some("vanishing") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Vanishing(amount));
        }
        return Some(KeywordAction::Marker("vanishing"));
    }

    if words.first().copied() == Some("sunburst") {
        return Some(KeywordAction::Sunburst);
    }
    if words.starts_with(&["for", "mirrodin"]) {
        return Some(KeywordAction::ForMirrodin);
    }
    if words.starts_with(&["living", "weapon"]) {
        return Some(KeywordAction::LivingWeapon);
    }

    if words.as_slice().starts_with(&["battle", "cry"]) {
        return Some(KeywordAction::BattleCry);
    }
    if words.first().copied() == Some("cascade") {
        return Some(KeywordAction::Cascade);
    }
    if words.as_slice().starts_with(&["split", "second"]) {
        return Some(KeywordAction::SplitSecond);
    }
    if words.as_slice().starts_with(&["doctor", "companion"]) {
        return Some(KeywordAction::Marker("doctor companion"));
    }
    if words.as_slice().starts_with(&["splice", "onto", "arcane"]) {
        if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[3..]) {
            return Some(KeywordAction::MarkerText(format!(
                "Splice onto Arcane {cost}"
            )));
        }
        let tail = &words[3..];
        let reminder_start = tail
            .windows(3)
            .position(|window| window == ["as", "you", "cast"])
            .or_else(|| tail.iter().position(|word| *word == "as"));
        let cost_words = reminder_start.map_or(tail, |idx| &tail[..idx]);
        if !cost_words.is_empty() {
            let cost_text = cost_words.join(" ");
            return Some(KeywordAction::MarkerText(format!(
                "Splice onto Arcane—{cost_text}"
            )));
        }
        return Some(KeywordAction::Marker("splice onto arcane"));
    }

    // Casualty N - "as you cast this spell, you may sacrifice a creature with power N or greater"
    if words.first().copied() == Some("casualty") {
        if words.len() == 2 {
            if let Ok(power) = words[1].parse::<u32>() {
                return Some(KeywordAction::Casualty(power));
            }
        }
        if words.len() == 1 {
            return Some(KeywordAction::Casualty(1));
        }
        return None;
    }

    // Conspire - "as you cast this spell, you may tap two untapped creatures..."
    if words.first().copied() == Some("conspire") && words.len() == 1 {
        return Some(KeywordAction::Conspire);
    }

    // Devour N - "as this enters, you may sacrifice any number of creatures..."
    if words.first().copied() == Some("devour") {
        if words.len() == 2 {
            if let Ok(multiplier) = words[1].parse::<u32>() {
                return Some(KeywordAction::Devour(multiplier));
            }
        }
        if words.len() == 1 {
            return Some(KeywordAction::Devour(1));
        }
        return None;
    }

    if let Some(first) = words.first().copied()
        && matches!(
            first,
            "banding"
                | "fabricate"
                | "foretell"
                | "bestow"
                | "dash"
                | "overload"
                | "soulshift"
                | "adapt"
                | "bolster"
                | "disturb"
                | "echo"
                | "modular"
                | "ninjutsu"
                | "outlast"
                | "scavenge"
                | "suspend"
                | "vanishing"
                | "offering"
                | "specialize"
                | "squad"
                | "spectacle"
                | "graft"
                | "backup"
                | "fading"
                | "fuse"
                | "plot"
                | "disguise"
                | "tribute"
                | "buyback"
                | "flashback"
        )
    {
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        if words.len() > 1 {
            return None;
        }
        return Some(KeywordAction::Marker(
            marker_keyword_id(first).expect("marker keyword id must exist for matched keyword"),
        ));
    }

    if words.len() == 1
        && let Some(action) = parse_single_word_keyword_action(words[0])
    {
        return Some(action);
    }

    let action = match words.as_slice() {
        ["affinity", "for", "artifacts"] => KeywordAction::AffinityForArtifacts,
        ["first", "strike"] => KeywordAction::FirstStrike,
        ["double", "strike"] => KeywordAction::DoubleStrike,
        ["for", "mirrodin"] => KeywordAction::ForMirrodin,
        ["living", "weapon"] => KeywordAction::LivingWeapon,
        ["fading", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Fading(value)
        }
        ["vanishing", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Vanishing(value)
        }
        ["modular", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Modular(value)
        }
        ["graft", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Graft(value)
        }
        ["soulshift", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Soulshift(value)
        }
        ["outlast", cost] => {
            let parsed_cost = parse_scryfall_mana_cost(cost).ok()?;
            KeywordAction::Outlast(parsed_cost)
        }
        ["ward", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Ward(value)
        }
        ["afterlife", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Afterlife(value)
        }
        ["fabricate", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Fabricate(value)
        }
        ["renown", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Renown(value)
        }
        ["protection", "from", "all", "colors"] => KeywordAction::ProtectionFromAllColors,
        ["protection", "from", "all", "color"] => KeywordAction::ProtectionFromAllColors,
        ["protection", "from", "colorless"] => KeywordAction::ProtectionFromColorless,
        ["protection", "from", "everything"] => KeywordAction::ProtectionFromEverything,
        ["protection", "from", value] => {
            if let Some(color) = parse_color(value) {
                KeywordAction::ProtectionFrom(color)
            } else if let Some(card_type) = parse_card_type(value) {
                KeywordAction::ProtectionFromCardType(card_type)
            } else if let Some(subtype) = parse_subtype_flexible(value) {
                KeywordAction::ProtectionFromSubtype(subtype)
            } else {
                return None;
            }
        }
        _ => {
            // "toxic N" needs exactly 2 words
            if words.len() == 2 && words[0] == "toxic" {
                let amount = words[1].parse::<u32>().ok().unwrap_or(1);
                return Some(KeywordAction::Toxic(amount));
            }
            if words.len() >= 2 {
                if words.starts_with(&["first", "strike"]) {
                    if words.len() > 2 && words.contains(&"and") {
                        return None;
                    }
                    return Some(KeywordAction::FirstStrike);
                }
                if words.starts_with(&["double", "strike"]) {
                    if words.len() > 2 && words.contains(&"and") {
                        return None;
                    }
                    return Some(KeywordAction::DoubleStrike);
                }
                if words.starts_with(&["protection", "from"]) && words.len() >= 3 {
                    let value = words[2];
                    return if let Some(color) = parse_color(value) {
                        Some(KeywordAction::ProtectionFrom(color))
                    } else if value == "everything" {
                        Some(KeywordAction::ProtectionFromEverything)
                    } else {
                        parse_card_type(value)
                            .map(KeywordAction::ProtectionFromCardType)
                            .or_else(|| {
                                parse_subtype_flexible(value)
                                    .map(KeywordAction::ProtectionFromSubtype)
                            })
                    };
                }
            }
            if words.len() >= 3 {
                let suffix = &words[words.len() - 3..];
                if suffix == ["cant", "be", "blocked"] || suffix == ["cannot", "be", "blocked"] {
                    return Some(KeywordAction::Unblockable);
                }
            }
            return None;
        }
    };

    Some(action)
}

pub(crate) fn parse_triggered_line(tokens: &[Token]) -> Result<LineAst, CardTextError> {
    let start_idx = if tokens.first().is_some_and(|token| {
        token.is_word("whenever") || token.is_word("at") || token.is_word("when")
    }) {
        1
    } else {
        0
    };

    if let Some(mut split_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .or_else(|| tokens.iter().position(|token| token.is_word("then")))
    {
        // Handle trigger lists like "Whenever you cast an Aura, Equipment, or Vehicle spell, ..."
        // by advancing to the next comma when the first split clearly stays inside the trigger text.
        if matches!(tokens.get(split_idx), Some(Token::Comma(_)))
            && tokens
                .first()
                .is_some_and(|token| token.is_word("whenever") || token.is_word("when"))
        {
            let trigger_prefix_tokens = &tokens[start_idx..split_idx];
            let tail = &tokens[split_idx + 1..];
            let looks_like_discard_qualifier_tail =
                looks_like_trigger_discard_qualifier_tail(trigger_prefix_tokens, tail);
            if looks_like_trigger_type_list_tail(tail)
                || looks_like_trigger_color_list_tail(tail)
                || looks_like_trigger_object_list_tail(tail)
                || looks_like_trigger_numeric_list_tail(tail)
                || looks_like_discard_qualifier_tail
            {
                let next_comma_rel = if looks_like_discard_qualifier_tail {
                    tail.iter().enumerate().find_map(|(idx, token)| {
                        if !matches!(token, Token::Comma(_)) {
                            return None;
                        }
                        let before_words = words(&tail[..idx]);
                        if before_words.contains(&"card") || before_words.contains(&"cards") {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                } else if looks_like_trigger_numeric_list_tail(tail) {
                    tail.iter().enumerate().rev().find_map(|(idx, token)| {
                        if matches!(token, Token::Comma(_)) {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                } else {
                    tail.iter()
                        .enumerate()
                        .find_map(|(idx, token)| {
                            if !matches!(token, Token::Comma(_)) {
                                return None;
                            }
                            let before_words = words(&tail[..idx]);
                            if before_words.contains(&"spell") || before_words.contains(&"spells") {
                                Some(idx)
                            } else {
                                None
                            }
                        })
                        .or_else(|| {
                            if looks_like_trigger_color_list_tail(tail)
                                || looks_like_trigger_object_list_tail(tail)
                            {
                                tail.iter().enumerate().find_map(|(idx, token)| {
                                    if !matches!(token, Token::Comma(_)) {
                                        return None;
                                    }
                                    let Some(next_word) =
                                        tail.get(idx + 1).and_then(Token::as_word)
                                    else {
                                        return None;
                                    };
                                    if matches!(next_word, "and" | "or") {
                                        return None;
                                    }

                                    let next_is_list_item =
                                        if looks_like_trigger_color_list_tail(tail) {
                                            parse_color(next_word).is_some()
                                        } else {
                                            is_trigger_objectish_word(next_word)
                                        };
                                    if next_is_list_item {
                                        return None;
                                    }
                                    Some(idx)
                                })
                            } else {
                                None
                            }
                        })
                };
                if let Some(next_comma_rel) = next_comma_rel {
                    let candidate_idx = split_idx + 1 + next_comma_rel;
                    if candidate_idx > start_idx && candidate_idx + 1 < tokens.len() {
                        split_idx = candidate_idx;
                    }
                }
            }
        }

        let mut trigger_tokens = &tokens[start_idx..split_idx];
        let mut max_triggers_from_trigger_clause = None;
        let trigger_words = words(trigger_tokens);
        for suffix in [
            ["for", "the", "first", "time", "each", "turn"].as_slice(),
            ["for", "the", "first", "time", "this", "turn"].as_slice(),
        ] {
            if trigger_words.ends_with(suffix) {
                let trimmed_word_len = trigger_words.len().saturating_sub(suffix.len());
                let trimmed_token_len =
                    token_index_for_word_index(trigger_tokens, trimmed_word_len)
                        .unwrap_or(trigger_tokens.len());
                trigger_tokens = &trigger_tokens[..trimmed_token_len];
                max_triggers_from_trigger_clause = Some(1u32);
                break;
            }
        }

        let trigger = parse_trigger_clause(trigger_tokens)?;
        let effects_tokens = rewrite_attached_controller_trigger_effect_tokens(
            trigger_tokens,
            &tokens[split_idx + 1..],
        );
        let effects = parse_effect_sentences(&effects_tokens)?;
        let mut max_triggers_per_turn =
            parse_triggered_times_each_turn_sentence(&split_on_period(&effects_tokens));
        if let Some(max) = max_triggers_from_trigger_clause {
            max_triggers_per_turn =
                Some(max_triggers_per_turn.map_or(max, |existing| existing.min(max)));
        }
        return Ok(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        });
    }

    // Some oracle lines omit the comma after the trigger clause.
    for split_idx in ((start_idx + 1)..tokens.len()).rev() {
        let mut trigger_tokens = &tokens[start_idx..split_idx];
        let mut max_triggers_from_trigger_clause = None;
        let effects_tokens = &tokens[split_idx..];
        if effects_tokens.is_empty() {
            continue;
        }
        let trigger_words = words(trigger_tokens);
        for suffix in [
            ["for", "the", "first", "time", "each", "turn"].as_slice(),
            ["for", "the", "first", "time", "this", "turn"].as_slice(),
        ] {
            if trigger_words.ends_with(suffix) {
                let trimmed_word_len = trigger_words.len().saturating_sub(suffix.len());
                let trimmed_token_len =
                    token_index_for_word_index(trigger_tokens, trimmed_word_len)
                        .unwrap_or(trigger_tokens.len());
                trigger_tokens = &trigger_tokens[..trimmed_token_len];
                max_triggers_from_trigger_clause = Some(1u32);
                break;
            }
        }
        if let Ok(trigger) = parse_trigger_clause(trigger_tokens) {
            let rewritten_effects_tokens =
                rewrite_attached_controller_trigger_effect_tokens(trigger_tokens, effects_tokens);
            if let Ok(effects) = parse_effect_sentences(&rewritten_effects_tokens) {
                let mut max_triggers_per_turn = parse_triggered_times_each_turn_sentence(
                    &split_on_period(&rewritten_effects_tokens),
                );
                if let Some(max) = max_triggers_from_trigger_clause {
                    max_triggers_per_turn =
                        Some(max_triggers_per_turn.map_or(max, |existing| existing.min(max)));
                }
                return Ok(LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                });
            }
        }
    }

    Err(CardTextError::ParseError(format!(
        "missing comma in triggered line (clause: '{}')",
        words(tokens).join(" ")
    )))
}

pub(crate) fn rewrite_attached_controller_trigger_effect_tokens(
    trigger_tokens: &[Token],
    effects_tokens: &[Token],
) -> Vec<Token> {
    let trigger_words = words(trigger_tokens);
    let references_enchanted_controller = trigger_words.windows(3).any(|window| {
        window[0] == "enchanted"
            && matches!(
                window[1],
                "creature"
                    | "creatures"
                    | "permanent"
                    | "permanents"
                    | "artifact"
                    | "artifacts"
                    | "enchantment"
                    | "enchantments"
                    | "land"
                    | "lands"
            )
            && window[2] == "controller"
    });
    if !references_enchanted_controller {
        return effects_tokens.to_vec();
    }

    let mut rewritten = Vec::with_capacity(effects_tokens.len());
    let mut idx = 0usize;
    while idx < effects_tokens.len() {
        if idx + 1 < effects_tokens.len()
            && effects_tokens[idx].is_word("that")
            && effects_tokens[idx + 1].is_word("creature")
        {
            let first_span = effects_tokens[idx].span();
            let second_span = effects_tokens[idx + 1].span();
            rewritten.push(Token::Word("enchanted".to_string(), first_span));
            rewritten.push(Token::Word("creature".to_string(), second_span));
            idx += 2;
            continue;
        }
        if idx + 1 < effects_tokens.len()
            && effects_tokens[idx].is_word("that")
            && effects_tokens[idx + 1].is_word("permanent")
        {
            let first_span = effects_tokens[idx].span();
            let second_span = effects_tokens[idx + 1].span();
            rewritten.push(Token::Word("enchanted".to_string(), first_span));
            rewritten.push(Token::Word("permanent".to_string(), second_span));
            idx += 2;
            continue;
        }
        rewritten.push(effects_tokens[idx].clone());
        idx += 1;
    }

    rewritten
}

pub(crate) fn looks_like_trigger_object_list_tail(tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }

    let words = words(tokens);
    if words.is_empty() {
        return false;
    }

    let starts_with_or = words.first().copied() == Some("or");
    let first_candidate = if starts_with_or {
        words.get(1).copied()
    } else {
        words.first().copied()
    };
    let Some(first_word) = first_candidate else {
        return false;
    };

    let type_like = parse_card_type(first_word).is_some()
        || parse_subtype_word(first_word).is_some()
        || first_word.strip_suffix('s').is_some_and(|stem| {
            parse_card_type(stem).is_some() || parse_subtype_word(stem).is_some()
        });
    if !type_like {
        return false;
    }

    tokens.iter().any(|token| matches!(token, Token::Comma(_)))
}

pub(crate) fn looks_like_trigger_discard_qualifier_tail(
    trigger_prefix_tokens: &[Token],
    tail_tokens: &[Token],
) -> bool {
    if tail_tokens.is_empty() {
        return false;
    }

    let prefix_words = words(trigger_prefix_tokens);
    if !(prefix_words.contains(&"discard") || prefix_words.contains(&"discards")) {
        return false;
    }

    let tail_words = words(tail_tokens);
    if tail_words.is_empty() {
        return false;
    }

    let Some(first_word) = tail_words.first().copied() else {
        return false;
    };
    let typeish = parse_card_type(first_word).is_some()
        || parse_non_type(first_word).is_some()
        || matches!(first_word, "and" | "or");
    if !typeish {
        return false;
    }

    tail_tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .is_some_and(|comma_idx| {
            let before_words = words(&tail_tokens[..comma_idx]);
            before_words.contains(&"card") || before_words.contains(&"cards")
        })
}

pub(crate) fn looks_like_trigger_type_list_tail(tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = words(tokens);
    if words.is_empty() {
        return false;
    }
    let first_is_card_type = parse_card_type(words[0]).is_some()
        || parse_subtype_word(words[0]).is_some()
        || words[0].strip_suffix('s').is_some_and(|word| {
            parse_card_type(word).is_some() || parse_subtype_word(word).is_some()
        });
    first_is_card_type
        && (words.contains(&"spell") || words.contains(&"spells"))
        && words.contains(&"or")
        && tokens.iter().any(|token| matches!(token, Token::Comma(_)))
}

pub(crate) fn looks_like_trigger_color_list_tail(tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = words(tokens);
    if words.is_empty() {
        return false;
    }
    is_basic_color_word(words[0])
        && words.contains(&"or")
        && tokens.iter().any(|token| matches!(token, Token::Comma(_)))
}

pub(crate) fn looks_like_trigger_numeric_list_tail(tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = words(tokens);
    if words.len() < 3 {
        return false;
    }
    if words[0].parse::<i32>().is_err() {
        return false;
    }
    let has_second_number = words.iter().skip(1).any(|word| word.parse::<i32>().is_ok());
    has_second_number && words.contains(&"or")
}

pub(crate) fn is_trigger_objectish_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || parse_subtype_word(word).is_some()
        || word.strip_suffix('s').is_some_and(|stem| {
            parse_card_type(stem).is_some() || parse_subtype_word(stem).is_some()
        })
}

pub(crate) fn strip_leading_trigger_intro(tokens: &[Token]) -> &[Token] {
    if tokens.first().is_some_and(|token| {
        token.is_word("when") || token.is_word("whenever") || token.is_word("at")
    }) {
        &tokens[1..]
    } else {
        tokens
    }
}

pub(crate) fn split_trigger_or_index(tokens: &[Token]) -> Option<usize> {
    tokens.iter().enumerate().find_map(|(idx, token)| {
        if !token.is_word("or") {
            return None;
        }
        // Keep quantifiers like "one or more <subject>" intact.
        let quantifier_or = idx > 0
            && tokens.get(idx - 1).is_some_and(|prev| prev.is_word("one"))
            && tokens.get(idx + 1).is_some_and(|next| next.is_word("more"));
        let comparison_or = is_comparison_or_delimiter(tokens, idx);
        let previous_numeric = (0..idx)
            .rev()
            .find_map(|i| tokens[i].as_word())
            .is_some_and(|word| word.parse::<i32>().is_ok());
        let next_numeric = tokens
            .get(idx + 1)
            .and_then(Token::as_word)
            .is_some_and(|word| word.parse::<i32>().is_ok());
        let numeric_list_or = previous_numeric && next_numeric;
        let color_list_or = tokens
            .get(idx - 1)
            .and_then(Token::as_word)
            .is_some_and(|word| parse_color(word).is_some())
            && tokens
                .get(idx + 1)
                .and_then(Token::as_word)
                .is_some_and(|word| parse_color(word).is_some())
            && tokens
                .iter()
                .filter_map(Token::as_word)
                .any(|word| word == "spell" || word == "spells");
        let objectish_word = |word: &str| is_trigger_objectish_word(word);
        let object_list_or = tokens
            .get(idx - 1)
            .and_then(Token::as_word)
            .is_some_and(objectish_word)
            && tokens
                .get(idx + 1)
                .and_then(Token::as_word)
                .is_some_and(objectish_word);
        let and_or_list_or = tokens.get(idx - 1).is_some_and(|prev| prev.is_word("and"))
            && tokens
                .get(idx + 1)
                .and_then(Token::as_word)
                .is_some_and(|word| parse_color(word).is_some() || objectish_word(word));
        let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
        let next_word = tokens.get(idx + 1).and_then(Token::as_word);
        let serial_spell_list_or = tokens
            .iter()
            .filter_map(Token::as_word)
            .any(|word| word == "spell" || word == "spells")
            && previous_word
                .is_some_and(|word| parse_color(word).is_some() || objectish_word(word))
            && next_word.is_some_and(|word| parse_color(word).is_some() || objectish_word(word));
        let cast_or_copy_or = tokens
            .iter()
            .filter_map(Token::as_word)
            .any(|word| word == "spell" || word == "spells")
            && previous_word.is_some_and(|word| word == "cast" || word == "casts")
            && next_word.is_some_and(|word| word == "copy" || word == "copies");
        let spell_or_ability_or = tokens
            .get(idx - 1)
            .and_then(Token::as_word)
            .is_some_and(|word| word == "spell" || word == "spells")
            && tokens
                .get(idx + 1)
                .and_then(Token::as_word)
                .is_some_and(|word| word == "ability" || word == "abilities");
        if quantifier_or
            || comparison_or
            || numeric_list_or
            || color_list_or
            || object_list_or
            || and_or_list_or
            || serial_spell_list_or
            || cast_or_copy_or
            || spell_or_ability_or
        {
            None
        } else {
            Some(idx)
        }
    })
}

pub(crate) fn has_leading_one_or_more(tokens: &[Token]) -> bool {
    tokens.len() >= 3
        && tokens.first().is_some_and(|token| token.is_word("one"))
        && tokens.get(1).is_some_and(|token| token.is_word("or"))
        && tokens.get(2).is_some_and(|token| token.is_word("more"))
}

pub(crate) fn strip_leading_one_or_more(tokens: &[Token]) -> &[Token] {
    if has_leading_one_or_more(tokens) {
        &tokens[3..]
    } else {
        tokens
    }
}

pub(crate) fn parse_trigger_clause(tokens: &[Token]) -> Result<TriggerSpec, CardTextError> {
    let words = words(tokens);

    if words.len() == 9
        && words.first().copied() == Some("this")
        && words.get(1).copied() == Some("creature")
        && words.get(2).copied() == Some("and")
        && words.get(3).copied() == Some("at")
        && words.get(4).copied() == Some("least")
        && words.get(6).copied() == Some("other")
        && words
            .get(7)
            .is_some_and(|word| *word == "creature" || *word == "creatures")
        && words
            .last()
            .is_some_and(|word| *word == "attack" || *word == "attacks")
    {
        let other_count = parse_cardinal_u32(words[5]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "invalid battalion attacker count in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        return Ok(TriggerSpec::ThisAttacksWithNOthers(other_count));
    }

    if let Some(or_idx) = tokens.iter().position(|token| token.is_word("or"))
        && words.last().copied() == Some("dies")
        && tokens.first().is_some_and(|token| token.is_word("this"))
    {
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..tokens.len() - 1];
        if left_tokens.len() == 1
            && left_tokens[0].is_word("this")
            && let Ok(filter) = parse_object_filter(right_tokens, false)
        {
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::ThisDies),
                Box::new(TriggerSpec::Dies(filter)),
            ));
        }
    }

    for tail in [
        ["is", "put", "into", "your", "graveyard", "from", "anywhere"].as_slice(),
        [
            "are",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
        ]
        .as_slice(),
        ["is", "put", "into", "your", "graveyard"].as_slice(),
        ["are", "put", "into", "your", "graveyard"].as_slice(),
    ] {
        if words.ends_with(tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = token_index_for_word_index(tokens, subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let mut filter = parse_object_filter(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported card filter in put-into-your-graveyard trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            filter.controller = Some(PlayerFilter::You);
            return Ok(TriggerSpec::PutIntoGraveyard(filter));
        }
    }

    for tail in [
        [
            "is",
            "put",
            "into",
            "a",
            "graveyard",
            "from",
            "the",
            "battlefield",
        ]
        .as_slice(),
        [
            "are",
            "put",
            "into",
            "a",
            "graveyard",
            "from",
            "the",
            "battlefield",
        ]
        .as_slice(),
    ] {
        if words.ends_with(tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = token_index_for_word_index(tokens, subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_words = self::words(subject_tokens);
            if is_source_reference_words(&subject_words) {
                return Ok(TriggerSpec::PutIntoGraveyard(ObjectFilter::source()));
            }
            if let Ok(filter) = parse_object_filter(subject_tokens, false) {
                return Ok(TriggerSpec::PutIntoGraveyard(filter));
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported filter in put-into-graveyard-from-battlefield trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
    }

    // "Whenever one or more cards leave your graveyard"
    for (tail, during_your_turn) in [
        (["leave", "your", "graveyard"].as_slice(), false),
        (["leaves", "your", "graveyard"].as_slice(), false),
        (
            ["leave", "your", "graveyard", "during", "your", "turn"].as_slice(),
            true,
        ),
        (
            ["leaves", "your", "graveyard", "during", "your", "turn"].as_slice(),
            true,
        ),
    ] {
        if words.ends_with(tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = token_index_for_word_index(tokens, subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();

            let one_or_more = has_leading_one_or_more(subject_tokens);
            let subject_tokens = strip_leading_one_or_more(subject_tokens);
            let subject_tokens = strip_leading_articles(subject_tokens);
            let mut filter = match self::words(&subject_tokens).as_slice() {
                ["card"] | ["cards"] => ObjectFilter::default(),
                _ => match parse_object_filter(&subject_tokens, false) {
                    Ok(filter) => filter,
                    Err(_) => {
                        // Keep parse success by preserving this as an unimplemented trigger
                        // if the card filter is more complex than we currently support.
                        return Ok(TriggerSpec::Custom(words.join(" ")));
                    }
                },
            };
            filter.nontoken = true;
            return Ok(TriggerSpec::CardsLeaveYourGraveyard {
                filter,
                one_or_more,
                during_your_turn,
            });
        }
    }

    if let Some(or_idx) = split_trigger_or_index(tokens) {
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..];
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause(left_tokens),
                parse_trigger_clause(right_tokens),
            )
            && !matches!(left, TriggerSpec::Custom(_))
            && !matches!(right, TriggerSpec::Custom(_))
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }
    if let Some(and_idx) = tokens.iter().position(|token| token.is_word("and"))
        && tokens.get(and_idx + 1).is_some_and(|token| {
            token.is_word("whenever") || token.is_word("when") || token.is_word("at")
        })
    {
        let left_tokens = strip_leading_trigger_intro(&tokens[..and_idx]);
        let right_tokens = strip_leading_trigger_intro(&tokens[and_idx + 1..]);
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause(left_tokens),
                parse_trigger_clause(right_tokens),
            )
            && !matches!(left, TriggerSpec::Custom(_))
            && !matches!(right, TriggerSpec::Custom(_))
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }
    let is_you_cast_this_spell = words
        .windows(3)
        .any(|window| window == ["cast", "this", "spell"] || window == ["casts", "this", "spell"]);
    if is_you_cast_this_spell && words.contains(&"you") {
        return Ok(TriggerSpec::YouCastThisSpell);
    }

    if let Some(spell_activity_trigger) = parse_spell_activity_trigger(tokens)? {
        return Ok(spell_activity_trigger);
    }

    if let Some(tap_idx) = tokens
        .iter()
        .position(|token| token.is_word("tap") || token.is_word("taps"))
    {
        let tap_word_idx = tokens[..tap_idx]
            .iter()
            .filter(|token| token.as_word().is_some())
            .count();
        let subject_words = &words[..tap_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let after_tap = &tokens[tap_idx + 1..];
            if let Some(for_idx) = after_tap.iter().position(|token| token.is_word("for"))
                && for_idx > 0
            {
                let tail_words: Vec<&str> = after_tap[for_idx..]
                    .iter()
                    .filter_map(Token::as_word)
                    .collect();
                if tail_words.first().copied() == Some("for") {
                    let object_tokens = trim_commas(&after_tap[..for_idx]);
                    let object_tokens = strip_leading_articles(&object_tokens);
                    if !object_tokens.is_empty()
                        && let Ok(filter) = parse_object_filter(&object_tokens, false)
                    {
                        return Ok(TriggerSpec::PlayerTapsForMana { player, filter });
                    }
                }
            }
        }
    }

    if let Some(tapped_idx) = tokens.iter().position(|token| token.is_word("tapped"))
        && tapped_idx >= 2
        && tokens
            .get(tapped_idx.wrapping_sub(1))
            .is_some_and(|token| token.is_word("is") || token.is_word("are"))
    {
        let subject_tokens = &tokens[..tapped_idx - 1];
        let after_tapped = &tokens[tapped_idx + 1..];
        if let Some(for_idx) = after_tapped.iter().position(|token| token.is_word("for")) {
            let tail_words: Vec<&str> = after_tapped[for_idx..]
                .iter()
                .filter_map(Token::as_word)
                .collect();
            if tail_words.first().copied() == Some("for") {
                let object_tokens = trim_commas(subject_tokens);
                let object_tokens = strip_leading_articles(&object_tokens);
                if !object_tokens.is_empty()
                    && let Ok(filter) = parse_object_filter(&object_tokens, false)
                {
                    return Ok(TriggerSpec::PlayerTapsForMana {
                        player: PlayerFilter::Any,
                        filter,
                    });
                }
            }
        }
    }

    if let Some(enters_idx) = tokens
        .iter()
        .position(|token| token.is_word("enters") || token.is_word("enter"))
    {
        if enters_idx == 0 {
            return Ok(TriggerSpec::ThisEntersBattlefield);
        }
        let subject_tokens = &tokens[..enters_idx];
        if let Some(or_idx) = subject_tokens.iter().position(|token| token.is_word("or")) {
            let left_tokens = &subject_tokens[..or_idx];
            let mut right_tokens = &subject_tokens[or_idx + 1..];
            let left_words: Vec<&str> = left_tokens
                .iter()
                .filter_map(Token::as_word)
                .filter(|word| !is_article(word))
                .collect();
            if is_source_reference_words(&left_words) && !right_tokens.is_empty() {
                let mut other = false;
                if right_tokens
                    .first()
                    .is_some_and(|token| token.is_word("another") || token.is_word("other"))
                {
                    other = true;
                    right_tokens = &right_tokens[1..];
                }
                if !right_tokens.is_empty()
                    && let Ok(mut filter) = parse_object_filter(right_tokens, other)
                {
                    if words.contains(&"under")
                        && words.contains(&"your")
                        && words.contains(&"control")
                    {
                        filter.controller = Some(PlayerFilter::You);
                    } else if words.contains(&"under")
                        && (words.contains(&"opponent") || words.contains(&"opponents"))
                        && words.contains(&"control")
                    {
                        filter.controller = Some(PlayerFilter::Opponent);
                    }
                    let right_trigger = if words.contains(&"untapped") {
                        TriggerSpec::EntersBattlefieldUntapped(filter)
                    } else if words.contains(&"tapped") {
                        TriggerSpec::EntersBattlefieldTapped(filter)
                    } else {
                        TriggerSpec::EntersBattlefield(filter)
                    };
                    return Ok(TriggerSpec::Either(
                        Box::new(TriggerSpec::ThisEntersBattlefield),
                        Box::new(right_trigger),
                    ));
                }
            }
        }
        if subject_tokens
            .first()
            .is_some_and(|token| token.is_word("this"))
        {
            return Ok(TriggerSpec::ThisEntersBattlefield);
        }
        let mut filtered_subject_tokens = subject_tokens;
        let mut other = false;
        if filtered_subject_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let one_or_more = has_leading_one_or_more(filtered_subject_tokens);
        filtered_subject_tokens = strip_leading_one_or_more(filtered_subject_tokens);
        if filtered_subject_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let parsed_filter = parse_object_filter(filtered_subject_tokens, other)
            .ok()
            .or_else(|| parse_subtype_list_enters_trigger_filter(filtered_subject_tokens, other));
        if let Some(mut filter) = parsed_filter {
            if words.contains(&"under") && words.contains(&"your") && words.contains(&"control") {
                filter.controller = Some(PlayerFilter::You);
            } else if words.contains(&"under")
                && (words.contains(&"opponent") || words.contains(&"opponents"))
                && words.contains(&"control")
            {
                filter.controller = Some(PlayerFilter::Opponent);
            }
            if words.contains(&"untapped") {
                return Ok(TriggerSpec::EntersBattlefieldUntapped(filter));
            }
            if words.contains(&"tapped") {
                return Ok(TriggerSpec::EntersBattlefieldTapped(filter));
            }
            return Ok(if one_or_more {
                TriggerSpec::EntersBattlefieldOneOrMore(filter)
            } else {
                TriggerSpec::EntersBattlefield(filter)
            });
        }
    }

    if words.as_slice() == ["players", "finish", "voting"]
        || words.as_slice() == ["players", "finished", "voting"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::Vote,
            player: PlayerFilter::Any,
        });
    }

    if words.as_slice() == ["you", "cycle", "this", "card"]
        || words.as_slice() == ["you", "cycled", "this", "card"]
    {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            player: PlayerFilter::You,
        });
    }

    if words.as_slice() == ["you", "cycle", "or", "discard", "a", "card"]
        || words.as_slice() == ["you", "cycle", "or", "discard", "card"]
    {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::Cycle,
                player: PlayerFilter::You,
            }),
            Box::new(TriggerSpec::PlayerDiscardsCard {
                player: PlayerFilter::You,
                filter: None,
            }),
        ));
    }

    if words.as_slice() == ["you", "commit", "a", "crime"] {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::You,
        });
    }

    if words.as_slice() == ["an", "opponent", "commits", "a", "crime"]
        || words.as_slice() == ["opponent", "commits", "a", "crime"]
        || words.as_slice() == ["opponents", "commit", "a", "crime"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Opponent,
        });
    }

    if words.as_slice() == ["a", "player", "commits", "a", "crime"]
        || words.as_slice() == ["a", "player", "commit", "a", "crime"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Any,
        });
    }

    if words.as_slice() == ["you", "unlock", "this", "door"]
        || words.as_slice() == ["you", "unlocked", "this", "door"]
    {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::UnlockDoor,
            player: PlayerFilter::You,
        });
    }

    if words.len() == 3
        && words[0] == "you"
        && words[1] == "expend"
        && let Some(amount) = parse_cardinal_u32(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::You,
            amount,
        });
    }

    if words.len() == 4
        && (words.as_slice()[..3] == ["an", "opponent", "expends"]
            || words.as_slice()[..3] == ["an", "opponent", "expend"])
        && let Some(amount) = parse_cardinal_u32(words[3])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if words.len() == 3
        && (words.as_slice()[..2] == ["opponent", "expends"]
            || words.as_slice()[..2] == ["opponent", "expend"])
        && let Some(amount) = parse_cardinal_u32(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if let Some(cycle_word_idx) = words.iter().position(|word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Cycle)
        )
    }) {
        let subject_words = &words[..cycle_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let tail_words = &words[cycle_word_idx + 1..];
            if tail_words == ["a", "card"] || tail_words == ["card"] {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Cycle,
                    player,
                });
            }
        }
    }

    if let Some(put_idx) = tokens
        .iter()
        .position(|token| token.is_word("put") || token.is_word("puts"))
    {
        let subject = &words[..put_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[put_idx + 1..];
            let has_name_sticker = tail.windows(2).any(|window| window == ["name", "sticker"]);
            let has_on = tail.contains(&"on");
            if has_name_sticker && has_on {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::NameSticker,
                    player,
                });
            }
        }
    }

    if let Some(last_word) = words.last().copied()
        && let Some(action) = crate::events::KeywordActionKind::from_trigger_word(last_word)
    {
        let subject = &words[..words.len().saturating_sub(1)];
        if is_source_reference_words(subject) {
            return Ok(TriggerSpec::KeywordActionFromSource {
                action,
                player: PlayerFilter::You,
            });
        }
        if subject.len() > 2 && is_source_reference_words(&subject[..2]) {
            let trailing_ok = subject[2..].iter().all(|word| {
                matches!(
                    *word,
                    "become" | "becomes" | "became" | "becoming" | "has" | "had"
                )
            });
            if trailing_ok {
                return Ok(TriggerSpec::KeywordActionFromSource {
                    action,
                    player: PlayerFilter::You,
                });
            }
        }
        let player = parse_trigger_subject_player_filter(subject);
        if let Some(player) = player {
            return Ok(TriggerSpec::KeywordAction { action, player });
        }
    }

    let has_deal = words.iter().any(|word| *word == "deal" || *word == "deals");
    if has_deal && words.contains(&"combat") && words.contains(&"damage") {
        if let Some(deals_idx) = tokens
            .iter()
            .position(|token| token.is_word("deal") || token.is_word("deals"))
        {
            let subject_tokens = &tokens[..deals_idx];
            let player_subject = trigger_subject_player_selector(subject_tokens).is_some();
            let one_or_more = has_leading_one_or_more(subject_tokens) || player_subject;
            let source_filter = parse_attack_trigger_subject_filter(subject_tokens)?;
            if let Some(damage_idx_rel) = tokens[deals_idx + 1..]
                .iter()
                .position(|token| token.is_word("damage"))
            {
                let damage_idx = deals_idx + 1 + damage_idx_rel;
                if let Some(to_idx_rel) = tokens[damage_idx + 1..]
                    .iter()
                    .position(|token| token.is_word("to"))
                {
                    let to_idx = damage_idx + 1 + to_idx_rel;
                    let target_tokens = trim_commas(&tokens[to_idx + 1..]);
                    if target_tokens.is_empty() {
                        return Err(CardTextError::ParseError(format!(
                            "missing combat damage recipient filter in trigger clause (clause: '{}')",
                            words.join(" ")
                        )));
                    }
                    let target_words: Vec<&str> =
                        target_tokens.iter().filter_map(Token::as_word).collect();
                    if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
                        return Ok(match source_filter {
                            Some(source) => {
                                if one_or_more {
                                    TriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                                        source,
                                        player,
                                    }
                                } else {
                                    TriggerSpec::DealsCombatDamageToPlayer { source, player }
                                }
                            }
                            None => TriggerSpec::ThisDealsCombatDamageToPlayer,
                        });
                    }

                    let target_tokens = strip_leading_one_or_more(&target_tokens);
                    let target_filter = parse_object_filter(target_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported combat damage recipient filter in trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                    return Ok(match source_filter {
                        Some(source) => TriggerSpec::DealsCombatDamageTo {
                            source,
                            target: target_filter,
                        },
                        None => TriggerSpec::ThisDealsCombatDamageTo(target_filter),
                    });
                }
            }

            return Ok(match source_filter {
                Some(filter) => TriggerSpec::DealsCombatDamage(filter),
                None => TriggerSpec::ThisDealsCombatDamage,
            });
        }
        return Ok(TriggerSpec::ThisDealsCombatDamage);
    }

    if words.as_slice() == ["end", "of", "combat"]
        || words.as_slice() == ["the", "end", "of", "combat"]
    {
        return Ok(TriggerSpec::EndOfCombat);
    }

    if words.as_slice() == ["this", "becomes", "monstrous"]
        || words.as_slice() == ["this", "creature", "becomes", "monstrous"]
        || words.as_slice() == ["this", "permanent", "becomes", "monstrous"]
    {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }

    if (words.starts_with(&["this", "creature", "blocks"])
        || words.starts_with(&["this", "blocks"]))
        && let Some(blocks_idx) = tokens
            .iter()
            .position(|token| token.is_word("block") || token.is_word("blocks"))
    {
        let tail_tokens = trim_commas(&tokens[blocks_idx + 1..]);
        if !tail_tokens.is_empty() && !tail_tokens.first().is_some_and(|token| token.is_word("or"))
        {
            let blocked_filter = parse_object_filter(&tail_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported blocked-object filter in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            return Ok(TriggerSpec::ThisBlocksObject(blocked_filter));
        }
    }

    if words.as_slice() == ["this", "creature", "blocks"] || words.as_slice() == ["this", "blocks"]
    {
        return Ok(TriggerSpec::ThisBlocks);
    }

    if words.starts_with(&["this", "creature", "becomes", "blocked"])
        || words.starts_with(&["this", "becomes", "blocked"])
    {
        return Ok(TriggerSpec::ThisBecomesBlocked);
    }

    if words.ends_with(&["becomes", "blocked"])
        && !words.starts_with(&["this", "creature", "becomes", "blocked"])
        && !words.starts_with(&["this", "becomes", "blocked"])
    {
        let becomes_word_idx = words.len().saturating_sub(2);
        let becomes_token_idx =
            token_index_for_word_index(tokens, becomes_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..becomes_token_idx];
        if let Some(filter) = parse_trigger_subject_filter(subject_tokens)? {
            return Ok(TriggerSpec::BecomesBlocked(filter));
        }
    }

    if words.as_slice() == ["this", "creature", "attacks", "or", "blocks"]
        || words.as_slice() == ["this", "attacks", "or", "blocks"]
    {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::ThisAttacks),
            Box::new(TriggerSpec::ThisBlocks),
        ));
    }

    if words.ends_with(&["attacks", "or", "blocks"]) || words.ends_with(&["attack", "or", "block"])
    {
        let attacks_word_idx = words.len().saturating_sub(3);
        let attacks_token_idx =
            token_index_for_word_index(tokens, attacks_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        if let Some(filter) = parse_trigger_subject_filter(subject_tokens)? {
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::Attacks(filter.clone())),
                Box::new(TriggerSpec::Blocks(filter)),
            ));
        }
    }

    if words.starts_with(&["this", "creature", "blocks", "or", "becomes", "blocked"])
        || words.starts_with(&["this", "blocks", "or", "becomes", "blocked"])
    {
        return Ok(TriggerSpec::ThisBlocksOrBecomesBlocked);
    }

    if words.ends_with(&["blocks", "or", "becomes", "blocked"])
        && !words.starts_with(&["this", "creature", "blocks", "or", "becomes", "blocked"])
        && !words.starts_with(&["this", "blocks", "or", "becomes", "blocked"])
    {
        let blocks_word_idx = words.len().saturating_sub(5);
        let blocks_token_idx = token_index_for_word_index(tokens, blocks_word_idx).unwrap_or(0);
        let subject_tokens = &tokens[..blocks_token_idx];
        if let Some(filter) = parse_trigger_subject_filter(subject_tokens)? {
            return Ok(TriggerSpec::BlocksOrBecomesBlocked(filter));
        }
    }

    if words.as_slice() == ["this", "leaves", "the", "battlefield"]
        || (words.len() == 5
            && words.first().copied() == Some("this")
            && words.get(2).copied() == Some("leaves")
            && words.get(3).copied() == Some("the")
            && words.get(4).copied() == Some("battlefield"))
    {
        return Ok(TriggerSpec::ThisLeavesBattlefield);
    }

    if words.ends_with(&["becomes", "tapped"])
        && let Some(becomes_idx) = tokens.iter().position(|token| token.is_word("becomes"))
        && tokens
            .get(becomes_idx + 1)
            .is_some_and(|token| token.is_word("tapped"))
    {
        let subject_tokens = &tokens[..becomes_idx];
        return Ok(match parse_trigger_subject_filter(subject_tokens)? {
            Some(filter) => TriggerSpec::PermanentBecomesTapped(filter),
            None => TriggerSpec::ThisBecomesTapped,
        });
    }

    if words.as_slice() == ["this", "creature", "becomes", "tapped"]
        || words.as_slice() == ["this", "becomes", "tapped"]
        || words.as_slice() == ["becomes", "tapped"]
    {
        return Ok(TriggerSpec::ThisBecomesTapped);
    }

    if words.as_slice() == ["this", "creature", "becomes", "untapped"]
        || words.as_slice() == ["this", "becomes", "untapped"]
        || words.as_slice() == ["becomes", "untapped"]
    {
        return Ok(TriggerSpec::ThisBecomesUntapped);
    }

    if words.as_slice() == ["this", "creature", "is", "turned", "face", "up"]
        || words.as_slice() == ["this", "permanent", "is", "turned", "face", "up"]
        || words.as_slice() == ["this", "is", "turned", "face", "up"]
    {
        return Ok(TriggerSpec::ThisTurnedFaceUp);
    }

    if words.ends_with(&["is", "turned", "face", "up"])
        || words.ends_with(&["are", "turned", "face", "up"])
    {
        let subject_tokens = &tokens[..tokens.len().saturating_sub(4)];
        return Ok(match parse_trigger_subject_filter(subject_tokens)? {
            Some(filter) => TriggerSpec::TurnedFaceUp(filter),
            None => TriggerSpec::ThisTurnedFaceUp,
        });
    }

    if let Some(becomes_idx) = words.iter().position(|word| *word == "becomes")
        && words.get(becomes_idx + 1).copied() == Some("the")
        && words.get(becomes_idx + 2).copied() == Some("target")
        && words.get(becomes_idx + 3).copied() == Some("of")
    {
        let subject_words = &words[..becomes_idx];
        let subject_tokens = token_index_for_word_index(tokens, becomes_idx)
            .map(|idx| &tokens[..idx])
            .unwrap_or_default();
        let subject_filter = parse_trigger_subject_filter(subject_tokens)?;
        let subject_is_source =
            subject_words.is_empty() || is_source_reference_words(subject_words);
        if subject_is_source {
            let tail_word_start = becomes_idx + 4;
            let tail_words = &words[tail_word_start..];
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words) {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: ObjectFilter::source(),
                    source_controller,
                });
            }
            if tail_words == ["a", "spell", "or", "ability"]
                || tail_words == ["spell", "or", "ability"]
            {
                return Ok(TriggerSpec::ThisBecomesTargeted);
            }
            if tail_words
                .last()
                .is_some_and(|word| *word == "spell" || *word == "spells")
                && let Some(tail_token_start) = token_index_for_word_index(tokens, tail_word_start)
            {
                let spell_filter_tokens = trim_commas(&tokens[tail_token_start..]);
                let spell_filter = parse_object_filter(&spell_filter_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported spell filter in becomes-targeted trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
                return Ok(TriggerSpec::ThisBecomesTargetedBySpell(spell_filter));
            }
        } else {
            let tail_word_start = becomes_idx + 4;
            let tail_words = &words[tail_word_start..];
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words)
                && let Some(filter) = subject_filter.clone()
            {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: filter,
                    source_controller,
                });
            }
            if (tail_words == ["a", "spell", "or", "ability"]
                || tail_words == ["spell", "or", "ability"])
                && let Some(filter) = subject_filter
            {
                return Ok(TriggerSpec::BecomesTargeted(filter));
            }
        }
    }

    if words.ends_with(&["is", "dealt", "damage"])
        && words.len() >= 4
        && !words.starts_with(&["this", "creature", "is", "dealt", "damage"])
        && !words.starts_with(&["this", "is", "dealt", "damage"])
    {
        let is_word_idx = words.len().saturating_sub(3);
        let is_token_idx = token_index_for_word_index(tokens, is_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..is_token_idx];
        if let Some(filter) = parse_trigger_subject_filter(subject_tokens)? {
            return Ok(TriggerSpec::IsDealtDamage(filter));
        }
    }

    if words.starts_with(&["this", "creature", "is", "dealt", "damage"])
        || words.starts_with(&["this", "is", "dealt", "damage"])
    {
        return Ok(TriggerSpec::ThisIsDealtDamage);
    }

    if (words.starts_with(&["this", "creature", "deals"]) || words.starts_with(&["this", "deals"]))
        && let Some(deals_idx) = tokens
            .iter()
            .position(|token| token.is_word("deal") || token.is_word("deals"))
        && let Some(damage_idx_rel) = tokens[deals_idx + 1..]
            .iter()
            .position(|token| token.is_word("damage"))
    {
        let damage_idx = deals_idx + 1 + damage_idx_rel;
        if let Some(to_idx_rel) = tokens[damage_idx + 1..]
            .iter()
            .position(|token| token.is_word("to"))
        {
            let to_idx = damage_idx + 1 + to_idx_rel;
            let amount_tokens = trim_commas(&tokens[deals_idx + 1..damage_idx]);
            if !amount_tokens
                .first()
                .is_some_and(|token| token.is_word("combat"))
            {
                let amount_words: Vec<&str> =
                    amount_tokens.iter().filter_map(Token::as_word).collect();
                if let Some((amount, _)) =
                    parse_filter_comparison_tokens("damage amount", &amount_words, &words)?
                {
                    let target_tokens = trim_commas(&tokens[to_idx + 1..]);
                    let target_words: Vec<&str> =
                        target_tokens.iter().filter_map(Token::as_word).collect();
                    if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
                        return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                            player,
                            amount: Some(amount),
                        });
                    }
                }
            }
        }
    }

    if (words.starts_with(&["this", "creature", "deals", "damage", "to"])
        || words.starts_with(&["this", "deals", "damage", "to"]))
        && let Some(to_idx) = tokens.iter().position(|token| token.is_word("to"))
    {
        let target_tokens = trim_commas(&tokens[to_idx + 1..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let target_words: Vec<&str> = target_tokens.iter().filter_map(Token::as_word).collect();
        if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
            return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                player,
                amount: None,
            });
        }
        let target_filter = parse_object_filter(&target_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        return Ok(TriggerSpec::ThisDealsDamageTo(target_filter));
    }

    if words.starts_with(&["this", "creature", "deals", "damage"])
        || words.starts_with(&["this", "deals", "damage"])
    {
        return Ok(TriggerSpec::ThisDealsDamage);
    }

    if has_deal
        && words.contains(&"damage")
        && let Some(deals_idx) = tokens
            .iter()
            .position(|token| token.is_word("deal") || token.is_word("deals"))
    {
        let subject_tokens = &tokens[..deals_idx];
        return Ok(match parse_trigger_subject_filter(subject_tokens)? {
            Some(filter) => TriggerSpec::DealsDamage(filter),
            None => TriggerSpec::ThisDealsDamage,
        });
    }

    if words.as_slice() == ["you", "gain", "life"] {
        return Ok(TriggerSpec::YouGainLife);
    }

    if words.len() >= 6
        && words.ends_with(&["during", "your", "turn"])
        && words[..words.len() - 3] == ["you", "gain", "life"]
    {
        return Ok(TriggerSpec::YouGainLifeDuringTurn(PlayerFilter::You));
    }

    if words.ends_with(&["lose", "life"]) || words.ends_with(&["loses", "life"]) {
        let subject = &words[..words.len().saturating_sub(2)];
        let player = parse_trigger_subject_player_filter(subject);
        if let Some(player) = player {
            return Ok(TriggerSpec::PlayerLosesLife(player));
        }
    }

    if words.len() >= 5
        && words.ends_with(&["during", "your", "turn"])
        && (words[..words.len() - 3].ends_with(&["lose", "life"])
            || words[..words.len() - 3].ends_with(&["loses", "life"]))
    {
        let subject = &words[..words.len() - 5];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLifeDuringTurn {
                player,
                during_turn: PlayerFilter::You,
            });
        }
    }

    if let Some(draw_idx) = tokens
        .iter()
        .position(|token| token.is_word("draw") || token.is_word("draws"))
    {
        let subject = &words[..draw_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[draw_idx + 1..];
            if let Some(card_number) = parse_exact_draw_count_each_turn(tail) {
                return Ok(TriggerSpec::PlayerDrawsNthCardEachTurn {
                    player,
                    card_number,
                });
            }
        }
    }

    if words.ends_with(&["draw", "a", "card"]) || words.ends_with(&["draws", "a", "card"]) {
        let subject = &words[..words.len().saturating_sub(3)];
        if subject == ["you"] {
            return Ok(TriggerSpec::YouDrawCard);
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerDrawsCard(player));
        }
    }

    if let Some(discard_idx) = tokens
        .iter()
        .position(|token| token.is_word("discard") || token.is_word("discards"))
    {
        let subject_words = &words[..discard_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            if let Ok(filter) =
                parse_discard_trigger_card_filter(&tokens[discard_idx + 1..], &words)
            {
                return Ok(TriggerSpec::PlayerDiscardsCard { player, filter });
            }
        }
    }

    if let Some(sacrifice_idx) = tokens
        .iter()
        .position(|token| token.is_word("sacrifice") || token.is_word("sacrifices"))
    {
        let subject_words = &words[..sacrifice_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let mut filter_tokens = &tokens[sacrifice_idx + 1..];
            let mut other = false;
            if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("another") || token.is_word("other"))
            {
                other = true;
                filter_tokens = &filter_tokens[1..];
            }

            let filter = if filter_tokens.is_empty() {
                let mut filter = ObjectFilter::permanent();
                if other {
                    filter.other = true;
                }
                filter
            } else if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("this") || token.is_word("it"))
            {
                let mut filter = ObjectFilter::source();
                let filter_words: Vec<&str> =
                    filter_tokens.iter().filter_map(Token::as_word).collect();
                if filter_words.contains(&"artifact") {
                    filter = filter.with_type(CardType::Artifact);
                } else if filter_words.contains(&"creature") {
                    filter = filter.with_type(CardType::Creature);
                } else if filter_words.contains(&"enchantment") {
                    filter = filter.with_type(CardType::Enchantment);
                } else if filter_words.contains(&"land") {
                    filter = filter.with_type(CardType::Land);
                } else if filter_words.contains(&"planeswalker") {
                    filter = filter.with_type(CardType::Planeswalker);
                }
                filter
            } else {
                parse_object_filter(filter_tokens, other).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported sacrifice trigger filter (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            return Ok(TriggerSpec::PlayerSacrifices { player, filter });
        }
    }

    if let Some(counter_word_idx) = words
        .iter()
        .position(|word| *word == "counter" || *word == "counters")
        && matches!(
            words.get(counter_word_idx + 1).copied(),
            Some("is") | Some("are")
        )
        && words.get(counter_word_idx + 2).copied() == Some("put")
        && matches!(
            words.get(counter_word_idx + 3).copied(),
            Some("on") | Some("onto")
        )
    {
        let one_or_more = words.starts_with(&["one", "or", "more"]);
        let descriptor_token_end =
            token_index_for_word_index(tokens, counter_word_idx).unwrap_or(tokens.len());
        let descriptor_tokens =
            strip_leading_articles(strip_leading_one_or_more(&tokens[..descriptor_token_end]));
        let counter_type = parse_counter_type_from_tokens(&descriptor_tokens);

        let object_word_start = counter_word_idx + 4;
        let object_token_start =
            token_index_for_word_index(tokens, object_word_start).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter recipient in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        let object_tokens = trim_commas(&tokens[object_token_start..]);
        let object_tokens = strip_leading_articles(&object_tokens);
        if object_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter recipient in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let filter = parse_object_filter(&object_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported counter recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;

        return Ok(TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            one_or_more,
        });
    }

    if let Some(attacks_word_idx) = words
        .iter()
        .position(|word| *word == "attack" || *word == "attacks")
    {
        let tail_words = &words[attacks_word_idx + 1..];
        if tail_words == ["and", "isnt", "blocked"]
            || tail_words == ["and", "isn't", "blocked"]
            || tail_words == ["and", "is", "not", "blocked"]
        {
            let attacks_token_idx =
                token_index_for_word_index(tokens, attacks_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            return Ok(match parse_attack_trigger_subject_filter(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksAndIsntBlocked(filter),
                None => TriggerSpec::ThisAttacksAndIsntBlocked,
            });
        }
    }

    if let Some(attack_idx) = words
        .iter()
        .position(|word| *word == "attack" || *word == "attacks")
        && words.get(attack_idx + 1).copied() == Some("with")
    {
        let subject_words = &words[..attack_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let with_object_word_start = attack_idx + 2;
            let with_object_token_start =
                token_index_for_word_index(tokens, with_object_word_start).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing attacking-object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            let mut object_tokens = &tokens[with_object_token_start..];
            let one_or_more = has_leading_one_or_more(object_tokens);
            object_tokens = strip_leading_one_or_more(object_tokens);
            if object_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attacking-object filter in trigger clause (clause: '{}')",
                    words.join(" ")
                )));
            }
            let mut filter = parse_object_filter(object_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking-object filter in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            if filter.controller.is_none() {
                if player == PlayerFilter::You {
                    filter.controller = Some(PlayerFilter::You);
                } else if player == PlayerFilter::Opponent {
                    filter.controller = Some(PlayerFilter::Opponent);
                }
            }
            return Ok(if one_or_more {
                TriggerSpec::AttacksOneOrMore(filter)
            } else {
                TriggerSpec::Attacks(filter)
            });
        }
    }

    let last = words
        .last()
        .ok_or_else(|| CardTextError::ParseError("empty trigger clause".to_string()))?;

    if words.len() >= 2
        && words.last().copied() == Some("alone")
        && matches!(
            words.get(words.len() - 2).copied(),
            Some("attack" | "attacks")
        )
    {
        let subject_tokens = if tokens.len() > 2 {
            &tokens[..tokens.len() - 2]
        } else {
            &[]
        };
        return Ok(match parse_attack_trigger_subject_filter(subject_tokens)? {
            Some(filter) => TriggerSpec::AttacksAlone(filter),
            None => TriggerSpec::AttacksAlone(ObjectFilter::source()),
        });
    }

    if let Some(attacks_word_idx) = words
        .iter()
        .position(|word| *word == "attack" || *word == "attacks")
    {
        let tail_words = &words[attacks_word_idx + 1..];
        if tail_words == ["you", "or", "a", "planeswalker", "you", "control"]
            || tail_words == ["you", "or", "planeswalker", "you", "control"]
        {
            let attacks_token_idx =
                token_index_for_word_index(tokens, attacks_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            let subject_filter = parse_attack_trigger_subject_filter(subject_tokens)?
                .unwrap_or_else(ObjectFilter::source);
            let player_subject = trigger_subject_player_selector(subject_tokens).is_some();
            return Ok(if player_subject {
                TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(subject_filter)
            } else {
                TriggerSpec::AttacksYouOrPlaneswalkerYouControl(subject_filter)
            });
        }
    }

    if words.len() >= 3
        && matches!(
            words.get(words.len() - 3).copied(),
            Some("attack" | "attacks")
        )
        && words.get(words.len() - 2).copied() == Some("while")
        && words.last().copied() == Some("saddled")
    {
        let attacks_word_idx = words.len().saturating_sub(3);
        let attacks_token_idx =
            token_index_for_word_index(tokens, attacks_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(match parse_attack_trigger_subject_filter(subject_tokens)? {
            Some(filter) => TriggerSpec::AttacksWhileSaddled(filter),
            None => TriggerSpec::ThisAttacksWhileSaddled,
        });
    }

    match *last {
        "attack" | "attacks" => {
            let subject_tokens = if tokens.len() > 1 {
                &tokens[..tokens.len() - 1]
            } else {
                &[]
            };
            let player_subject = trigger_subject_player_selector(subject_tokens).is_some();
            let one_or_more = has_leading_one_or_more(subject_tokens) || player_subject;
            Ok(match parse_attack_trigger_subject_filter(subject_tokens)? {
                Some(filter) => {
                    if one_or_more {
                        TriggerSpec::AttacksOneOrMore(filter)
                    } else {
                        TriggerSpec::Attacks(filter)
                    }
                }
                None => TriggerSpec::ThisAttacks,
            })
        }
        "block" | "blocks" => {
            let subject_tokens = if tokens.len() > 1 {
                &tokens[..tokens.len() - 1]
            } else {
                &[]
            };
            Ok(match parse_trigger_subject_filter(subject_tokens)? {
                Some(filter) => TriggerSpec::Blocks(filter),
                None => TriggerSpec::ThisBlocks,
            })
        }
        "dies" => {
            let mut subject_tokens = if tokens.len() > 1 {
                &tokens[..tokens.len() - 1]
            } else {
                &[]
            };

            if subject_tokens.is_empty()
                || subject_tokens
                    .first()
                    .is_some_and(|token| token.is_word("this"))
            {
                return Ok(TriggerSpec::ThisDies);
            }

            // Pattern: "the creature it haunts dies" or "the creature this card haunts dies"
            {
                let subject_words = self::words(subject_tokens);
                if subject_words.last().copied() == Some("haunts")
                    && subject_words.first().copied() == Some("the")
                    && subject_words.get(1).copied() == Some("creature")
                {
                    return Ok(TriggerSpec::HauntedCreatureDies);
                }
            }

            let mut other = false;
            if subject_tokens
                .first()
                .is_some_and(|token| token.is_word("another"))
            {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }

            if subject_tokens.is_empty() {
                return Ok(TriggerSpec::ThisDies);
            }

            // Pattern: "a creature dealt damage by [this/equipped] creature this turn dies"
            let subject_words = self::words(subject_tokens);
            if subject_words.len() >= 8
                && subject_words.ends_with(&["this", "turn"])
                && let Some(dealt_word_idx) = subject_words
                    .windows(3)
                    .position(|w| w == ["dealt", "damage", "by"])
            {
                let victim_end =
                    token_index_for_word_index(subject_tokens, dealt_word_idx).unwrap_or(0);
                if victim_end > 0 && victim_end <= subject_tokens.len() {
                    let victim_tokens = trim_edge_punctuation(&subject_tokens[..victim_end]);
                    let victim_tokens = strip_leading_articles(&victim_tokens);
                    if !victim_tokens.is_empty() {
                        // Damage source tokens are between "by" and "this turn"
                        let damager_start_word_idx = dealt_word_idx + 3;
                        let this_word_idx = subject_words.len() - 2;
                        let damager_start =
                            token_index_for_word_index(subject_tokens, damager_start_word_idx)
                                .unwrap_or(subject_tokens.len());
                        let damager_end = token_index_for_word_index(subject_tokens, this_word_idx)
                            .unwrap_or(subject_tokens.len());

                        if damager_start < damager_end && damager_end <= subject_tokens.len() {
                            let damager_tokens =
                                trim_edge_punctuation(&subject_tokens[damager_start..damager_end]);
                            let damager_words = self::words(&damager_tokens);

                            let has_named_source_words = !damager_words.is_empty()
                                && !matches!(
                                    damager_words.first().copied(),
                                    Some(
                                        "a" | "an"
                                            | "the"
                                            | "target"
                                            | "that"
                                            | "this"
                                            | "equipped"
                                            | "enchanted"
                                    )
                                )
                                && !damager_words.iter().any(|word| {
                                    matches!(
                                        *word,
                                        "creature"
                                            | "creatures"
                                            | "permanent"
                                            | "permanents"
                                            | "source"
                                            | "sources"
                                    )
                                });

                            let damager = if damager_words == ["this", "creature"]
                                || damager_words == ["this", "permanent"]
                                || damager_words == ["this", "source"]
                                || damager_words == ["this"]
                                || has_named_source_words
                            {
                                Some(DamageBySpec::ThisCreature)
                            } else if damager_words == ["equipped", "creature"] {
                                Some(DamageBySpec::EquippedCreature)
                            } else if damager_words == ["enchanted", "creature"] {
                                Some(DamageBySpec::EnchantedCreature)
                            } else {
                                None
                            };

                            if let Some(damager) = damager {
                                let victim = parse_object_filter(&victim_tokens, other).map_err(
                                    |_| {
                                        CardTextError::ParseError(format!(
                                            "unsupported damaged-by trigger victim filter (clause: '{}')",
                                            words.join(" ")
                                        ))
                                    },
                                )?;
                                return Ok(TriggerSpec::DiesCreatureDealtDamageByThisTurn {
                                    victim,
                                    damager,
                                });
                            }
                        }
                    }
                }
            }

            if let Ok(filter) = parse_object_filter(subject_tokens, other) {
                return Ok(TriggerSpec::Dies(filter));
            }

            Ok(TriggerSpec::ThisDies)
        }
        _ if words.contains(&"beginning") && words.contains(&"end") && words.contains(&"step") => {
            Ok(TriggerSpec::BeginningOfEndStep(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if words.contains(&"beginning") && words.contains(&"upkeep") => Ok(
            TriggerSpec::BeginningOfUpkeep(parse_possessive_clause_player_filter(&words)),
        ),
        _ if words.contains(&"beginning") && words.contains(&"draw") && words.contains(&"step") => {
            Ok(TriggerSpec::BeginningOfDrawStep(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if words.contains(&"beginning")
            && words.contains(&"combat")
            && words.contains(&"turn") =>
        {
            Ok(TriggerSpec::BeginningOfCombat(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if words.contains(&"beginning")
            && words.contains(&"first")
            && words.contains(&"main")
            && words.contains(&"phase") =>
        {
            Ok(TriggerSpec::BeginningOfPrecombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if words.contains(&"beginning")
            && words.contains(&"precombat")
            && words.contains(&"main") =>
        {
            Ok(TriggerSpec::BeginningOfPrecombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ => Ok(TriggerSpec::Custom(words.join(" "))),
    }
}

pub(crate) fn parse_discard_trigger_card_filter(
    after_discard_tokens: &[Token],
    clause_words: &[&str],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let remainder = trim_commas(after_discard_tokens);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let remainder_words = words(&remainder);
    let Some(card_word_idx) = remainder_words
        .iter()
        .position(|word| *word == "card" || *word == "cards")
    else {
        return Err(CardTextError::ParseError(format!(
            "missing discard trigger card keyword (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let qualifier_end =
        token_index_for_word_index(&remainder, card_word_idx).unwrap_or(remainder.len());
    let qualifier_tokens = trim_commas(&remainder[..qualifier_end]);
    let mut qualifier_tokens = strip_leading_articles(&qualifier_tokens);
    if qualifier_tokens.len() >= 2
        && qualifier_tokens
            .first()
            .and_then(Token::as_word)
            .and_then(parse_cardinal_u32)
            .is_some()
        && qualifier_tokens
            .get(1)
            .is_some_and(|token| token.is_word("or"))
    {
        qualifier_tokens = qualifier_tokens[2..].to_vec();
    } else if qualifier_tokens
        .first()
        .and_then(Token::as_word)
        .and_then(parse_cardinal_u32)
        .is_some()
    {
        qualifier_tokens = qualifier_tokens[1..].to_vec();
    }

    let trailing_tokens = if card_word_idx + 1 < remainder_words.len() {
        let trailing_start =
            token_index_for_word_index(&remainder, card_word_idx + 1).unwrap_or(remainder.len());
        trim_commas(&remainder[trailing_start..])
    } else {
        Vec::new()
    };
    if !trailing_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing discard trigger clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if qualifier_tokens.is_empty() {
        return Ok(None);
    }

    let qualifier_words = words(&qualifier_tokens);
    if qualifier_words.as_slice() == ["one", "or", "more"] {
        return Ok(None);
    }

    if let Ok(filter) = parse_object_filter(&qualifier_tokens, false) {
        return Ok(Some(filter));
    }

    let mut fallback = ObjectFilter::default();
    let mut parsed_any = false;
    for word in qualifier_words {
        if matches!(word, "and" | "or") {
            continue;
        }
        if let Some(non_type) = parse_non_type(word) {
            if !fallback.excluded_card_types.contains(&non_type) {
                fallback.excluded_card_types.push(non_type);
            }
            parsed_any = true;
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            if !fallback.card_types.contains(&card_type) {
                fallback.card_types.push(card_type);
            }
            parsed_any = true;
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if parsed_any {
        Ok(Some(fallback))
    } else {
        Err(CardTextError::ParseError(format!(
            "unsupported discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )))
    }
}

pub(crate) fn parse_subtype_list_enters_trigger_filter(
    tokens: &[Token],
    other: bool,
) -> Option<ObjectFilter> {
    let words = words(tokens);
    if words.is_empty() {
        return None;
    }

    let (controller, subject_end) = if words.len() >= 2
        && words[words.len() - 2] == "you"
        && words[words.len() - 1] == "control"
    {
        (Some(PlayerFilter::You), words.len() - 2)
    } else if words.len() >= 2
        && words[words.len() - 2] == "opponent"
        && words[words.len() - 1] == "controls"
    {
        (Some(PlayerFilter::Opponent), words.len() - 2)
    } else if words.len() >= 3
        && words[words.len() - 3] == "an"
        && words[words.len() - 2] == "opponent"
        && words[words.len() - 1] == "controls"
    {
        (Some(PlayerFilter::Opponent), words.len() - 3)
    } else {
        (None, words.len())
    };

    let mut subtypes = Vec::new();
    for word in &words[..subject_end] {
        if matches!(*word, "and" | "or") {
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            if !subtypes.contains(&subtype) {
                subtypes.push(subtype);
            }
        }
    }
    if subtypes.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.subtypes = subtypes;
    filter.controller = controller;
    filter.other = other;
    Some(filter)
}

pub(crate) fn parse_possessive_clause_player_filter(words: &[&str]) -> PlayerFilter {
    let attached_controller_filter =
        |tag: &str| PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(tag)));
    let has_attached_controller = |subject: &str| {
        words.windows(3).any(|window| {
            window[0] == subject
                && matches!(
                    window[1],
                    "creature"
                        | "creatures"
                        | "permanent"
                        | "permanents"
                        | "artifact"
                        | "artifacts"
                        | "enchantment"
                        | "enchantments"
                        | "land"
                        | "lands"
                )
                && window[2] == "controller"
        })
    };

    if has_attached_controller("enchanted") {
        return attached_controller_filter("enchanted");
    }
    if has_attached_controller("equipped") {
        return attached_controller_filter("equipped");
    }

    if contains_your_team_words(words) || words.contains(&"your") {
        PlayerFilter::You
    } else if contains_opponent_word(words) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

pub(crate) fn parse_subject_clause_player_filter(words: &[&str]) -> PlayerFilter {
    if contains_your_team_words(words) || words.contains(&"you") {
        PlayerFilter::You
    } else if contains_opponent_word(words) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

pub(crate) fn contains_opponent_word(words: &[&str]) -> bool {
    words.contains(&"opponent") || words.contains(&"opponents")
}

pub(crate) fn contains_your_team_words(words: &[&str]) -> bool {
    words.windows(2).any(|window| window == ["your", "team"])
        || words
            .windows(3)
            .any(|window| window == ["on", "your", "team"])
}

pub(crate) fn parse_trigger_subject_player_filter(subject: &[&str]) -> Option<PlayerFilter> {
    if subject == ["you"] {
        return Some(PlayerFilter::You);
    }
    if subject == ["a", "player"]
        || subject == ["any", "player"]
        || subject == ["player"]
        || subject == ["one", "or", "more", "players"]
    {
        return Some(PlayerFilter::Any);
    }
    if subject == ["an", "opponent"]
        || subject == ["opponent"]
        || subject == ["opponents"]
        || subject == ["your", "opponents"]
        || subject == ["one", "of", "your", "opponents"]
        || subject == ["one", "or", "more", "of", "your", "opponents"]
        || subject == ["one", "of", "the", "opponents"]
        || subject == ["one", "or", "more", "opponents"]
        || subject == ["each", "opponent"]
    {
        return Some(PlayerFilter::Opponent);
    }
    if subject.ends_with(&["on", "your", "team"])
        && subject
            .iter()
            .any(|word| matches!(*word, "player" | "players"))
    {
        return Some(PlayerFilter::You);
    }
    None
}

pub(crate) fn parse_spell_or_ability_controller_tail(words: &[&str]) -> Option<PlayerFilter> {
    let (prefix_len, controller_end) = if words.starts_with(&["a", "spell", "or", "ability"]) {
        (4usize, words.len())
    } else if words.starts_with(&["spell", "or", "ability"]) {
        (3usize, words.len())
    } else {
        return None;
    };

    if controller_end <= prefix_len + 1 {
        return None;
    }
    if !matches!(words.last().copied(), Some("control") | Some("controls")) {
        return None;
    }

    let controller_words = &words[prefix_len..controller_end - 1];
    parse_trigger_subject_player_filter(controller_words)
}

pub(crate) fn parse_trigger_subject_filter(
    subject_tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut subject_tokens = strip_leading_one_or_more(subject_tokens);
    let mut other = false;
    if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"))
    {
        other = true;
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = words(subject_tokens);
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if subject_words
        .iter()
        .any(|word| matches!(*word, "that" | "which" | "who" | "whom"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger subject filter (clause: '{}')",
            subject_words.join(" ")
        )));
    }

    parse_object_filter(subject_tokens, other)
        .map(Some)
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported trigger subject filter (clause: '{}')",
                words(subject_tokens).join(" ")
            ))
        })
}

pub(crate) fn trigger_subject_player_selector(subject_tokens: &[Token]) -> Option<PlayerFilter> {
    let subject_tokens = strip_leading_one_or_more(subject_tokens);
    let subject_words = words(subject_tokens);
    parse_trigger_subject_player_filter(&subject_words)
}

pub(crate) fn attacking_filter_for_player(player: PlayerFilter) -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    if !matches!(player, PlayerFilter::Any) {
        filter.controller = Some(player);
    }
    filter
}

pub(crate) fn parse_attack_trigger_subject_filter(
    subject_tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if let Some(player) = trigger_subject_player_selector(subject_tokens) {
        return Ok(Some(attacking_filter_for_player(player)));
    }
    let Some(mut filter) = parse_trigger_subject_filter(subject_tokens)? else {
        return Ok(None);
    };

    // Attack/combat-trigger subjects are creatures by default even when
    // expressed only as a subtype ("a Sliver", "one or more Goblins", etc.).
    if filter.card_types.is_empty() {
        filter.card_types.push(crate::types::CardType::Creature);
    }

    Ok(Some(filter))
}

pub(crate) fn parse_exact_spell_count_each_turn(words: &[&str]) -> Option<u32> {
    for (ordinal, count) in [
        ("third", 3u32),
        ("fourth", 4u32),
        ("fifth", 5u32),
        ("sixth", 6u32),
        ("seventh", 7u32),
        ("eighth", 8u32),
        ("ninth", 9u32),
        ("tenth", 10u32),
    ] {
        if contains_word_sequence(words, &[ordinal, "spell", "cast", "this", "turn"])
            || contains_word_sequence(words, &[ordinal, "spell", "this", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "spell", "each", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "spell", "each", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "spell", "this", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "spell", "this", "turn"])
            || contains_word_sequence(words, &[ordinal, "spell", "each", "turn"])
        {
            return Some(count);
        }
    }
    None
}

pub(crate) fn parse_exact_draw_count_each_turn(words: &[&str]) -> Option<u32> {
    for (ordinal, count) in [
        ("second", 2u32),
        ("third", 3u32),
        ("fourth", 4u32),
        ("fifth", 5u32),
        ("sixth", 6u32),
        ("seventh", 7u32),
        ("eighth", 8u32),
        ("ninth", 9u32),
        ("tenth", 10u32),
    ] {
        if contains_word_sequence(words, &[ordinal, "card", "each", "turn"])
            || contains_word_sequence(words, &[ordinal, "cards", "each", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "card", "each", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "cards", "each", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "card", "each", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "cards", "each", "turn"])
            || contains_word_sequence(words, &[ordinal, "card", "this", "turn"])
            || contains_word_sequence(words, &[ordinal, "cards", "this", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "card", "this", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "cards", "this", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "card", "this", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "cards", "this", "turn"])
        {
            return Some(count);
        }
    }
    None
}

pub(crate) fn has_first_spell_each_turn_pattern(words: &[&str]) -> bool {
    let has_turn_context = contains_word_sequence(words, &["each", "turn"])
        || contains_word_sequence(words, &["this", "turn"])
        || contains_word_sequence(words, &["of", "a", "turn"])
        || contains_word_sequence(words, &["during", "your", "turn"])
        || contains_word_sequence(words, &["during", "their", "turn"])
        || contains_word_sequence(words, &["during", "an", "opponents", "turn"])
        || contains_word_sequence(words, &["during", "opponents", "turn"])
        || contains_word_sequence(words, &["during", "each", "opponents", "turn"]);
    if !has_turn_context {
        return false;
    }

    for (idx, word) in words.iter().enumerate() {
        if *word != "first" {
            continue;
        }
        let window_end = (idx + 5).min(words.len());
        if words[idx + 1..window_end]
            .iter()
            .any(|candidate| *candidate == "spell" || *candidate == "spells")
        {
            return true;
        }
    }
    false
}

pub(crate) fn has_second_spell_turn_pattern(words: &[&str]) -> bool {
    contains_word_sequence(words, &["second", "spell", "cast", "this", "turn"])
        || contains_word_sequence(words, &["second", "spell", "this", "turn"])
        || contains_word_sequence(words, &["your", "second", "spell", "each", "turn"])
        || contains_word_sequence(words, &["their", "second", "spell", "each", "turn"])
        || contains_word_sequence(words, &["your", "second", "spell", "this", "turn"])
        || contains_word_sequence(words, &["their", "second", "spell", "this", "turn"])
        || contains_word_sequence(words, &["second", "spell", "each", "turn"])
        || contains_word_sequence(words, &["second", "spell", "during", "your", "turn"])
        || contains_word_sequence(words, &["second", "spell", "during", "their", "turn"])
        || contains_word_sequence(
            words,
            &["second", "spell", "during", "an", "opponents", "turn"],
        )
        || contains_word_sequence(words, &["second", "spell", "during", "opponents", "turn"])
        || contains_word_sequence(
            words,
            &["second", "spell", "during", "each", "opponents", "turn"],
        )
}

pub(crate) fn parse_spell_activity_trigger(
    tokens: &[Token],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"spell") && !clause_words.contains(&"spells") {
        return Ok(None);
    }

    let cast_idx = tokens
        .iter()
        .position(|token| token.is_word("cast") || token.is_word("casts"));
    let copy_idx = tokens
        .iter()
        .position(|token| token.is_word("copy") || token.is_word("copies"));
    if cast_idx.is_none() && copy_idx.is_none() {
        return Ok(None);
    }

    let mut actor = parse_subject_clause_player_filter(&clause_words);
    let during_their_turn = contains_word_sequence(&clause_words, &["during", "their", "turn"])
        || contains_word_sequence(&clause_words, &["during", "that", "players", "turn"]);
    let mut during_turn = if contains_word_sequence(&clause_words, &["during", "your", "turn"]) {
        Some(PlayerFilter::You)
    } else if contains_word_sequence(&clause_words, &["during", "an", "opponents", "turn"])
        || contains_word_sequence(&clause_words, &["during", "opponents", "turn"])
        || contains_word_sequence(&clause_words, &["during", "each", "opponents", "turn"])
    {
        Some(PlayerFilter::Opponent)
    } else {
        None
    };
    if during_their_turn {
        if matches!(actor, PlayerFilter::Any) {
            actor = PlayerFilter::Active;
            during_turn = None;
        } else if during_turn.is_none() {
            during_turn = Some(actor.clone());
        }
    }
    let has_other_than_first_spell_pattern =
        contains_word_sequence(&clause_words, &["other", "than", "your", "first", "spell"])
            || contains_word_sequence(&clause_words, &["other", "than", "the", "first", "spell"])
            || (contains_word_sequence(&clause_words, &["other", "than", "the", "first"])
                && clause_words.contains(&"spell")
                && clause_words.contains(&"casts")
                && clause_words.contains(&"turn"));
    let second_spell_turn_pattern = has_second_spell_turn_pattern(&clause_words);
    let first_spell_each_turn =
        !has_other_than_first_spell_pattern && has_first_spell_each_turn_pattern(&clause_words);
    let exact_spells_this_turn = parse_exact_spell_count_each_turn(&clause_words)
        .or_else(|| first_spell_each_turn.then_some(1))
        .or_else(|| {
            (!has_other_than_first_spell_pattern && second_spell_turn_pattern).then_some(2)
        });
    let min_spells_this_turn = if exact_spells_this_turn.is_some() {
        None
    } else if has_other_than_first_spell_pattern {
        Some(2)
    } else {
        None
    };
    let from_not_hand = contains_word_sequence(
        &clause_words,
        &["from", "anywhere", "other", "than", "your", "hand"],
    ) || contains_word_sequence(
        &clause_words,
        &["from", "anywhere", "other", "than", "their", "hand"],
    ) || contains_word_sequence(
        &clause_words,
        &["from", "anywhere", "other", "than", "hand"],
    ) || clause_words
        .windows(4)
        .position(|window| window == ["from", "anywhere", "other", "than"])
        .is_some_and(|idx| {
            clause_words[idx + 4..]
                .iter()
                .take(4)
                .any(|word| *word == "hand")
        });

    let parse_filter = |filter_tokens: &[Token]| -> Result<Option<ObjectFilter>, CardTextError> {
        let filter_tokens = if let Some(idx) = filter_tokens
            .iter()
            .position(|token| token.is_word("during") || token.is_word("other"))
        {
            &filter_tokens[..idx]
        } else {
            filter_tokens
        };
        let filter_tokens = if let Some(idx) = filter_tokens
            .iter()
            .position(|token| token.is_word("from"))
            .filter(|idx| {
                filter_tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("anywhere"))
            }) {
            &filter_tokens[..idx]
        } else {
            filter_tokens
        };
        let filter_words: Vec<&str> = filter_tokens.iter().filter_map(Token::as_word).collect();
        let is_unqualified_spell = filter_words.as_slice() == ["a", "spell"]
            || filter_words.as_slice() == ["spells"]
            || filter_words.as_slice() == ["spell"];
        if filter_tokens.is_empty() || is_unqualified_spell {
            Ok(None)
        } else {
            let parse_spell_origin_zone_filter = || -> Option<ObjectFilter> {
                let zone = if filter_words.contains(&"graveyard") {
                    Some(Zone::Graveyard)
                } else if filter_words.contains(&"exile") {
                    Some(Zone::Exile)
                } else {
                    None
                }?;
                let mentions_spell =
                    filter_words.contains(&"spell") || filter_words.contains(&"spells");
                if !mentions_spell {
                    return None;
                }
                let mut filter = ObjectFilter::spell().in_zone(zone);
                if filter_words.contains(&"your") {
                    filter.owner = Some(actor.clone());
                } else if filter_words.contains(&"opponent") || filter_words.contains(&"their") {
                    filter.owner = Some(PlayerFilter::Opponent);
                }
                Some(filter)
            };
            match parse_object_filter(filter_tokens, false) {
                Ok(filter) => Ok(Some(filter)),
                Err(err) => {
                    let mut compact_words = filter_words
                        .iter()
                        .copied()
                        .filter(|word| !is_article(word))
                        .collect::<Vec<_>>();
                    if compact_words
                        .last()
                        .is_some_and(|last| *last == "spell" || *last == "spells")
                    {
                        compact_words.pop();
                        let color_words = compact_words
                            .into_iter()
                            .filter(|word| *word != "or" && *word != "and")
                            .collect::<Vec<_>>();
                        if !color_words.is_empty()
                            && color_words.iter().all(|word| parse_color(word).is_some())
                        {
                            let mut colors = ColorSet::new();
                            for word in color_words {
                                colors =
                                    colors.union(parse_color(word).expect("validated color word"));
                            }
                            let mut filter = ObjectFilter::spell();
                            filter.colors = Some(colors);
                            return Ok(Some(filter));
                        }
                    }
                    if let Some(origin_filter) = parse_spell_origin_zone_filter() {
                        Ok(Some(origin_filter))
                    } else {
                        Err(err)
                    }
                }
            }
        }
    };

    if let (Some(cast), Some(copy)) = (cast_idx, copy_idx) {
        let (first, second, first_is_cast) = if cast < copy {
            (cast, copy, true)
        } else {
            (copy, cast, false)
        };
        let between_words = words(&tokens[first + 1..second]);
        if between_words.as_slice() == ["or"] {
            let filter = parse_filter(tokens.get(second + 1..).unwrap_or_default())?;
            let cast_trigger = TriggerSpec::SpellCast {
                filter: filter.clone(),
                caster: actor.clone(),
                during_turn: during_turn.clone(),
                min_spells_this_turn,
                exact_spells_this_turn,
                from_not_hand,
            };
            let copied_trigger = TriggerSpec::SpellCopied {
                filter,
                copier: actor,
            };
            return Ok(Some(if first_is_cast {
                TriggerSpec::Either(Box::new(cast_trigger), Box::new(copied_trigger))
            } else {
                TriggerSpec::Either(Box::new(copied_trigger), Box::new(cast_trigger))
            }));
        }
    }

    if let Some(cast) = cast_idx {
        let mut filter_tokens = tokens.get(cast + 1..).unwrap_or_default();
        if filter_tokens.is_empty() {
            let mut prefix_tokens = &tokens[..cast];
            while let Some(last_word) = prefix_tokens.last().and_then(Token::as_word) {
                if matches!(last_word, "is" | "are" | "was" | "were" | "be" | "been") {
                    prefix_tokens = &prefix_tokens[..prefix_tokens.len() - 1];
                } else {
                    break;
                }
            }
            let has_spell_noun = prefix_tokens
                .iter()
                .any(|token| token.is_word("spell") || token.is_word("spells"));
            if has_spell_noun {
                filter_tokens = prefix_tokens;
            }
        }
        let filter = parse_filter(filter_tokens)?;
        return Ok(Some(TriggerSpec::SpellCast {
            filter,
            caster: actor,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        }));
    }

    if let Some(copy) = copy_idx {
        let filter = parse_filter(tokens.get(copy + 1..).unwrap_or_default())?;
        return Ok(Some(TriggerSpec::SpellCopied {
            filter,
            copier: actor,
        }));
    }

    Ok(None)
}

pub(crate) fn is_spawn_scion_token_mana_reminder(tokens: &[Token]) -> bool {
    let words = words(tokens);
    let starts_with_token_pronoun = words.starts_with(&["they", "have"])
        || words.starts_with(&["it", "has"])
        || words.starts_with(&["this", "token", "has"])
        || words.starts_with(&["those", "tokens", "have"]);
    starts_with_token_pronoun
        && words.contains(&"sacrifice")
        && words.contains(&"add")
        && words.contains(&"c")
}

pub(crate) fn is_round_up_each_time_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.starts_with(&["round", "up", "each", "time"])
}

pub(crate) enum MayCastItVerb {
    Cast,
    Play,
}

pub(crate) struct MayCastTaggedSpec {
    pub(crate) verb: MayCastItVerb,
    pub(crate) as_copy: bool,
    pub(crate) without_paying_mana_cost: bool,
}

pub(crate) fn parse_may_cast_it_sentence(tokens: &[Token]) -> Option<MayCastTaggedSpec> {
    let mut clause_words = words(tokens);
    while clause_words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        clause_words.remove(0);
    }

    if clause_words.starts_with(&["if", "you", "do"]) {
        clause_words = clause_words[3..].to_vec();
        while clause_words
            .first()
            .is_some_and(|word| *word == "then" || *word == "and")
        {
            clause_words.remove(0);
        }
    }

    if clause_words.len() < 4 || clause_words[0] != "you" || clause_words[1] != "may" {
        return None;
    }

    let verb = match clause_words[2] {
        "cast" => MayCastItVerb::Cast,
        "play" => MayCastItVerb::Play,
        _ => return None,
    };

    let rest = &clause_words[3..];
    let (as_copy, consumed) = if rest.starts_with(&["it"]) {
        (false, 1usize)
    } else if rest.starts_with(&["the", "copy"])
        || rest.starts_with(&["that", "copy"])
        || rest.starts_with(&["a", "copy"])
    {
        (true, 2usize)
    } else {
        return None;
    };

    let tail = &rest[consumed..];
    if tail.is_empty() {
        return Some(MayCastTaggedSpec {
            verb,
            as_copy,
            without_paying_mana_cost: false,
        });
    }
    if tail == ["without", "paying", "its", "mana", "cost"] {
        return Some(MayCastTaggedSpec {
            verb,
            as_copy,
            without_paying_mana_cost: true,
        });
    }
    None
}

pub(crate) fn build_may_cast_tagged_effect(spec: &MayCastTaggedSpec) -> EffectAst {
    EffectAst::May {
        effects: vec![EffectAst::CastTagged {
            tag: TagKey::from(IT_TAG),
            allow_land: matches!(spec.verb, MayCastItVerb::Play),
            as_copy: spec.as_copy,
            without_paying_mana_cost: spec.without_paying_mana_cost,
        }],
    }
}

pub(crate) fn is_simple_copy_reference_sentence(tokens: &[Token]) -> bool {
    let clause_words = words(tokens);
    clause_words.as_slice() == ["copy", "it"]
        || clause_words.as_slice() == ["copy", "this"]
        || clause_words.as_slice() == ["copy", "that"]
        || clause_words.as_slice() == ["copy", "that", "card"]
        || clause_words.as_slice() == ["copy", "the", "exiled", "card"]
}

pub(crate) fn token_name_mentions_eldrazi_spawn_or_scion(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower.contains("eldrazi") && lower.contains("spawn"))
        || (lower.contains("eldrazi") && lower.contains("scion"))
}

pub(crate) fn effect_creates_eldrazi_spawn_or_scion(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::CreateToken { name, .. } | EffectAst::CreateTokenWithMods { name, .. } => {
            token_name_mentions_eldrazi_spawn_or_scion(name)
        }
        _ => false,
    }
}

pub(crate) fn effect_creates_any_token(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::CreateToken { .. }
        | EffectAst::CreateTokenWithMods { .. }
        | EffectAst::CreateTokenCopy { .. }
        | EffectAst::CreateTokenCopyFromSource { .. } => true,
        _ => {
            let mut found = false;
            for_each_nested_effects(effect, false, |nested| {
                if !found && nested.iter().any(effect_creates_any_token) {
                    found = true;
                }
            });
            found
        }
    }
}

pub(crate) fn last_created_token_info(effects: &[EffectAst]) -> Option<(String, PlayerAst)> {
    for effect in effects.iter().rev() {
        if let Some(info) = created_token_info_from_effect(effect) {
            return Some(info);
        }
    }
    None
}

pub(crate) fn created_token_info_from_effect(effect: &EffectAst) -> Option<(String, PlayerAst)> {
    match effect {
        EffectAst::CreateToken { name, player, .. }
        | EffectAst::CreateTokenWithMods { name, player, .. } => Some((name.clone(), *player)),
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, true, |nested| {
                if found.is_none() {
                    found = last_created_token_info(nested);
                }
            });
            found
        }
    }
}

pub(crate) fn title_case_token_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out = first.to_uppercase().to_string();
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

#[allow(dead_code)]
pub(crate) fn linked_token_name_from_create_name(raw_name: &str) -> Option<String> {
    let words: Vec<&str> = raw_name.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }

    let mut name_words = Vec::new();
    for word in words {
        if looks_like_pt_word(word)
            || is_basic_color_word(word)
            || is_article(word)
            || matches!(
                word,
                "legendary"
                    | "snow"
                    | "basic"
                    | "artifact"
                    | "enchantment"
                    | "land"
                    | "creature"
                    | "battle"
                    | "instant"
                    | "sorcery"
                    | "planeswalker"
                    | "token"
                    | "tokens"
                    | "with"
                    | "that"
                    | "which"
                    | "named"
                    | "and"
                    | "or"
            )
            || parse_card_type(word).is_some()
            || parse_subtype_word(word).is_some()
        {
            if !name_words.is_empty() {
                break;
            }
            continue;
        }
        if !word
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '\'' || ch == '-')
        {
            if !name_words.is_empty() {
                break;
            }
            continue;
        }
        name_words.push(title_case_token_word(word));
    }

    if name_words.is_empty() {
        None
    } else {
        Some(name_words.join(" "))
    }
}

pub(crate) fn controller_filter_for_token_player(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        _ => None,
    }
}

pub(crate) fn parse_sentence_exile_that_token_when_source_leaves(
    tokens: &[Token],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    let clause_words = words(tokens);
    if clause_words.len() < 6 || !matches!(clause_words.first().copied(), Some("exile" | "exiles"))
    {
        return None;
    }
    let when_idx = clause_words.iter().position(|word| *word == "when")?;
    if when_idx < 2 || when_idx + 3 >= clause_words.len() {
        return None;
    }
    if !clause_words.ends_with(&["leaves", "the", "battlefield"]) {
        return None;
    }
    let object_words = &clause_words[1..when_idx];
    let is_created_token_reference = object_words == ["that", "token"]
        || object_words == ["those", "tokens"]
        || object_words == ["them"]
        || object_words == ["it"];
    if !is_created_token_reference {
        return None;
    }
    let subject_words = &clause_words[when_idx + 1..clause_words.len() - 3];
    if !is_source_reference_words(subject_words) {
        return None;
    }

    let _ = last_created_token_info(prior_effects)?;

    Some(EffectAst::ExileWhenSourceLeaves {
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
    })
}

pub(crate) fn parse_sentence_sacrifice_source_when_that_token_leaves(
    tokens: &[Token],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    let clause_words = words(tokens);
    if clause_words.len() < 8 || !matches!(clause_words[0], "sacrifice" | "sacrifices") {
        return None;
    }
    let when_idx = clause_words.iter().position(|word| *word == "when")?;
    if when_idx < 2 || when_idx + 4 > clause_words.len() {
        return None;
    }
    let subject_words = &clause_words[1..when_idx];
    if !is_source_reference_words(subject_words) {
        return None;
    }
    if clause_words[when_idx + 1..] != ["that", "token", "leaves", "the", "battlefield"] {
        return None;
    }

    let _ = last_created_token_info(prior_effects)?;

    Some(EffectAst::SacrificeSourceWhenLeaves {
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
    })
}

pub(crate) fn is_generic_token_reminder_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    if words.is_empty() {
        return false;
    }
    if words.starts_with(&["it", "has"]) || words.starts_with(&["they", "have"]) {
        return true;
    }
    if words.starts_with(&["when", "it"])
        || words.starts_with(&["whenever", "it"])
        || words.starts_with(&["when", "they"])
        || words.starts_with(&["whenever", "they"])
    {
        return true;
    }
    let delayed_lifecycle_reference = matches!(words.first().copied(), Some("exile" | "sacrifice"))
        && (is_beginning_of_end_step_words(&words) || is_end_of_combat_words(&words))
        && (words.contains(&"token")
            || words.contains(&"tokens")
            || words.contains(&"it")
            || words.contains(&"them"));
    if delayed_lifecycle_reference {
        return true;
    }
    words.starts_with(&["when", "this", "token"])
        || words.starts_with(&["whenever", "this", "token"])
        || words.starts_with(&["this", "token"])
        || words.starts_with(&["those", "tokens"])
}

pub(crate) fn strip_embedded_token_rules_text(tokens: &[Token]) -> Vec<Token> {
    let words_all = words(tokens);
    if !words_all.contains(&"create") || !words_all.contains(&"token") {
        return tokens.to_vec();
    }
    let Some(with_idx) = tokens.iter().position(|token| token.is_word("with")) else {
        return tokens.to_vec();
    };
    let next_word = tokens.get(with_idx + 1).and_then(Token::as_word);
    if matches!(next_word, Some("t")) {
        return tokens[..with_idx].to_vec();
    }
    tokens.to_vec()
}

#[allow(dead_code)]
pub(crate) fn append_token_reminder_to_last_create_effect(
    effects: &mut Vec<EffectAst>,
    tokens: &[Token],
) -> bool {
    let mut reminder_words = words(tokens);
    let mut prepend_with = false;
    if reminder_words.starts_with(&["it", "has"]) || reminder_words.starts_with(&["they", "have"]) {
        reminder_words = reminder_words[2..].to_vec();
        prepend_with = true;
    }
    if reminder_words.starts_with(&["when", "it"]) {
        let mut rewritten = vec!["when", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    } else if reminder_words.starts_with(&["whenever", "it"]) {
        let mut rewritten = vec!["whenever", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    } else if reminder_words.starts_with(&["when", "they"]) {
        let mut rewritten = vec!["when", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    } else if reminder_words.starts_with(&["whenever", "they"]) {
        let mut rewritten = vec!["whenever", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    }
    if reminder_words.is_empty() {
        return false;
    }
    let reminder = if prepend_with {
        format!("with {}", reminder_words.join(" "))
    } else {
        reminder_words.join(" ")
    };
    append_token_reminder_to_effect(effects.last_mut(), &reminder, &reminder_words)
}

#[allow(dead_code)]
pub(crate) fn append_token_reminder_to_effect(
    effect: Option<&mut EffectAst>,
    reminder: &str,
    reminder_words: &[&str],
) -> bool {
    let Some(effect) = effect else {
        return false;
    };
    match effect {
        EffectAst::CreateToken { name, .. } => {
            if !name.ends_with(' ') {
                name.push(' ');
            }
            name.push_str(reminder);
            true
        }
        EffectAst::CreateTokenWithMods {
            name,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            if !name.ends_with(' ') {
                name.push(' ');
            }
            name.push_str(reminder);
            let (sacrifice_next_end_step, exile_next_end_step) =
                parse_next_end_step_token_delay_flags(reminder_words);
            if sacrifice_next_end_step {
                *sacrifice_at_next_end_step = true;
            }
            if exile_next_end_step {
                *exile_at_next_end_step = true;
            }
            let exile_end_of_combat =
                reminder_words.contains(&"exile") && is_end_of_combat_words(reminder_words);
            if exile_end_of_combat {
                *exile_at_end_of_combat = true;
            }
            let sacrifice_end_of_combat =
                reminder_words.contains(&"sacrifice") && is_end_of_combat_words(reminder_words);
            if sacrifice_end_of_combat {
                *sacrifice_at_end_of_combat = true;
            }
            true
        }
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, false, |nested| {
                if !applied {
                    applied = append_token_reminder_to_effect(
                        nested.last_mut(),
                        reminder,
                        reminder_words,
                    );
                }
            });
            applied
        }
    }
}

pub(crate) fn parse_target_player_choose_objects_clause(
    tokens: &[Token],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause_words = words(tokens);
    let (chooser, choose_start_idx) =
        if clause_words.first().copied() == Some("target") && clause_words.len() >= 4 {
            let chooser = match clause_words.get(1).copied() {
                Some("player") => PlayerAst::Target,
                Some("opponent") | Some("opponents") => PlayerAst::TargetOpponent,
                _ => return Ok(None),
            };
            if !matches!(
                clause_words.get(2).copied(),
                Some("choose") | Some("chooses")
            ) {
                return Ok(None);
            }
            (chooser, 3usize)
        } else if clause_words.len() >= 4
            && clause_words.first().copied() == Some("that")
            && matches!(clause_words.get(1).copied(), Some("player" | "players"))
            && matches!(clause_words.get(2).copied(), Some("choose" | "chooses"))
        {
            (PlayerAst::That, 3usize)
        } else {
            return Ok(None);
        };

    let mut choose_object_tokens = trim_commas(&tokens[choose_start_idx..]);
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object after target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut count = ChoiceCount::exactly(1);
    if choose_object_tokens
        .first()
        .is_some_and(|token| token.is_word("up"))
        && choose_object_tokens
            .get(1)
            .is_some_and(|token| token.is_word("to"))
        && let Some((value, used)) = parse_number(&choose_object_tokens[2..])
    {
        count = ChoiceCount {
            min: 0,
            max: Some(value as usize),
            dynamic_x: false,
        };
        choose_object_tokens = trim_commas(&choose_object_tokens[2 + used..]);
    } else if let Some((value, used)) = parse_number(&choose_object_tokens) {
        count = ChoiceCount::exactly(value as usize);
        choose_object_tokens = trim_commas(&choose_object_tokens[used..]);
    }
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object filter after count in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if choose_object_tokens
        .first()
        .is_some_and(|token| token.is_word("target"))
        && choose_object_tokens
            .get(1)
            .is_some_and(|token| token.is_word("player") || token.is_word("opponent"))
    {
        return Ok(None);
    }
    if find_verb(&choose_object_tokens).is_some() {
        return Ok(None);
    }

    let mut choose_filter = parse_object_filter(&choose_object_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported chosen object filter in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            _ => PlayerFilter::target_player(),
        });
    }

    Ok(Some((chooser, choose_filter, count)))
}

pub(crate) fn parse_you_choose_objects_clause(
    tokens: &[Token],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let choose_word_idx = if clause_words.first().copied() == Some("you") {
        1usize
    } else {
        0usize
    };
    if !matches!(
        clause_words.get(choose_word_idx).copied(),
        Some("choose" | "chooses")
    ) {
        return Ok(None);
    }

    let choose_word_token_idx =
        token_index_for_word_index(tokens, choose_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing choose keyword in choose clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let choose_object_tokens = trim_commas(&tokens[choose_word_token_idx + 1..]);
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object after choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut choose_words = words(&choose_object_tokens);
    let mut count = ChoiceCount::exactly(1);
    if choose_words.starts_with(&["up", "to"])
        && let Some((value, used)) = parse_number(
            &choose_words[2..]
                .iter()
                .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>(),
        )
    {
        count = ChoiceCount {
            min: 0,
            max: Some(value as usize),
            dynamic_x: false,
        };
        choose_words = choose_words[2 + used..].to_vec();
    } else if let Some((value, used)) = parse_number(
        &choose_words
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>(),
    ) {
        count = ChoiceCount::exactly(value as usize);
        choose_words = choose_words[used..].to_vec();
    } else if choose_words.first().is_some_and(|word| is_article(word)) {
        choose_words = choose_words[1..].to_vec();
    }

    let mut references_it = false;
    loop {
        let len = choose_words.len();
        let trailing_it = len >= 2
            && matches!(choose_words[len - 2], "from" | "in")
            && matches!(choose_words[len - 1], "it" | "them");
        let trailing_there = len >= 3
            && matches!(choose_words[len - 3], "from" | "in")
            && choose_words[len - 2] == "there"
            && choose_words[len - 1] == "in";
        if trailing_it {
            references_it = true;
            choose_words.truncate(len - 2);
            continue;
        }
        if trailing_there {
            references_it = true;
            choose_words.truncate(len - 3);
            continue;
        }
        break;
    }

    if choose_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object filter in choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let choose_filter_tokens = choose_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    if find_verb(&choose_filter_tokens).is_some() {
        return Ok(None);
    }

    let mut choose_filter = parse_object_filter(&choose_filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported chosen object filter in choose clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if references_it {
        if choose_filter.zone.is_none() {
            choose_filter.zone = Some(Zone::Hand);
        }
        if !choose_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
        {
            choose_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
        }
    }
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    if references_it {
        choose_filter.controller = None;
        choose_filter.owner = None;
    } else if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(PlayerFilter::You);
    }

    Ok(Some((PlayerAst::You, choose_filter, count)))
}

pub(crate) fn parse_target_player_chooses_then_other_cant_block(
    first: &[Token],
    second: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, mut choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(first)?
    else {
        return Ok(None);
    };
    if choose_filter.card_types.is_empty() {
        choose_filter.card_types.push(CardType::Creature);
    }

    let second_words = words(second);
    let Some((neg_start, neg_end)) = find_negation_span(second) else {
        return Ok(None);
    };
    let tail_words = normalize_cant_words(&second[neg_end..]);
    if !matches!(tail_words.as_slice(), ["block", "this", "turn"] | ["block"]) {
        return Ok(None);
    }

    let mut subject_tokens = trim_commas(&second[..neg_start]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing subject in cant-block clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let mut exclude_tagged_choice = false;
    if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("other") || token.is_word("another"))
    {
        exclude_tagged_choice = true;
        subject_tokens = trim_commas(&subject_tokens[1..]);
    }
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in cant-block clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let mut restriction_filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported cant-block subject filter (clause: '{}')",
            second_words.join(" ")
        ))
    })?;
    if restriction_filter.card_types.is_empty() {
        restriction_filter.card_types.push(CardType::Creature);
    }
    if restriction_filter.controller.is_none() {
        restriction_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            _ => PlayerFilter::target_player(),
        });
    }
    if exclude_tagged_choice
        && !restriction_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            })
    {
        restriction_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::Cant {
            restriction: crate::effect::Restriction::block(restriction_filter),
            duration: Until::EndOfTurn,
        },
    ]))
}

pub(crate) fn parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    first: &[Token],
    second: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_words = words(first);
    if first_words.is_empty() || !matches!(first_words[0], "choose" | "chooses") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if first_words.get(idx).is_some_and(|word| is_article(word)) {
        idx += 1;
    }
    if first_words.get(idx) != Some(&"card") || first_words.get(idx + 1) != Some(&"type") {
        return Ok(None);
    }
    idx += 2;

    let reveal_words = &first_words[idx..];
    if !reveal_words.starts_with(&["then", "reveal", "the", "top"]) {
        return Ok(None);
    }
    let reveal_tokens = reveal_words[4..]
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_number(&reveal_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing reveal count in choose-card-type reveal clause (clause: '{}')",
            first_words.join(" ")
        ))
    })?;
    if reveal_tokens
        .get(used)
        .and_then(Token::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in choose-card-type reveal clause (clause: '{}')",
            first_words.join(" ")
        )));
    }
    let reveal_tail = words(&reveal_tokens[used + 1..]);
    if !reveal_tail.ends_with(&["of", "your", "library"]) {
        return Ok(None);
    }

    let second_words = words(second);
    if !matches!(second_words.first().copied(), Some("put" | "puts")) {
        return Ok(None);
    }
    let has_chosen_type = second_words
        .windows(2)
        .any(|window| window == ["chosen", "type"]);
    let has_revealed_this_way = second_words
        .windows(3)
        .any(|window| window == ["revealed", "this", "way"]);
    let has_into_your_hand = second_words
        .windows(3)
        .any(|window| window == ["into", "your", "hand"]);
    let has_bottom_of_library = second_words
        .windows(4)
        .any(|window| window == ["bottom", "of", "your", "library"]);
    if !has_chosen_type || !has_revealed_this_way || !has_into_your_hand || !has_bottom_of_library {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::RevealTopChooseCardTypePutToHandRestBottom {
            player: PlayerAst::You,
            count,
        },
    ]))
}

pub(crate) fn parse_choose_creature_type_then_become_type(
    first: &[Token],
    second: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = words(&first_tokens);
    if first_words.is_empty() || !matches!(first_words[0], "choose" | "chooses") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if first_words.get(idx).is_some_and(|word| is_article(word)) {
        idx += 1;
    }
    if first_words.get(idx) != Some(&"creature") || first_words.get(idx + 1) != Some(&"type") {
        return Ok(None);
    }
    idx += 2;

    let mut excluded_subtypes = Vec::new();
    if idx < first_words.len() {
        if first_words.get(idx) == Some(&"other") && first_words.get(idx + 1) == Some(&"than") {
            let subtype_word = first_words.get(idx + 2).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing creature subtype exclusion in creature-type choice clause (clause: '{}')",
                    first_words.join(" ")
                ))
            })?;
            let subtype = parse_subtype_word(subtype_word)
                .or_else(|| subtype_word.strip_suffix('s').and_then(parse_subtype_word))
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported creature subtype exclusion in creature-type choice clause (clause: '{}')",
                        first_words.join(" ")
                    ))
                })?;
            excluded_subtypes.push(subtype);
            idx += 3;
        }
        if idx != first_words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported creature-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
    }

    let second_words = words(second);
    let Some(become_idx) = second
        .iter()
        .position(|token| token.is_word("become") || token.is_word("becomes"))
    else {
        return Ok(None);
    };
    if become_idx == 0 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&second[..become_idx]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in creature-type become clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let become_tail_tokens = trim_commas(&second[become_idx + 1..]);
    let (duration, become_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(&become_tail_tokens)? {
            (duration, remainder)
        } else {
            (Until::Forever, become_tail_tokens.to_vec())
        };
    let become_words = words(&become_tokens);
    if become_words.as_slice() != ["that", "type"] {
        return Ok(None);
    }

    let subject_words = words(&subject_tokens);
    let target = if subject_words.starts_with(&["each"]) || subject_words.starts_with(&["all"]) {
        let filter_tokens = trim_commas(&subject_tokens[1..]);
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
        let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            ))
        })?;
        TargetAst::Object(filter, span_from_tokens(&subject_tokens), None)
    } else {
        parse_target_phrase(&subject_tokens)?
    };

    Ok(Some(vec![EffectAst::BecomeCreatureTypeChoice {
        target,
        duration,
        excluded_subtypes,
    }]))
}

pub(crate) fn parse_sentence_target_player_chooses_then_puts_on_top_of_library(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    let first_clause = trim_commas(&tokens[..and_idx]);
    let second_clause = trim_commas(&tokens[and_idx + 1..]);
    if second_clause.is_empty() {
        return Ok(None);
    }

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(&first_clause)?
    else {
        return Ok(None);
    };

    let second_words = words(&second_clause);
    if !matches!(second_words.first().copied(), Some("put" | "puts")) {
        return Ok(None);
    }
    let Some(on_idx) = second_clause.iter().position(|token| token.is_word("on")) else {
        return Ok(None);
    };
    if !second_clause
        .get(on_idx + 1)
        .is_some_and(|token| token.is_word("top"))
        || !second_clause
            .get(on_idx + 2)
            .is_some_and(|token| token.is_word("of"))
    {
        return Ok(None);
    }
    let destination_words = words(&second_clause[on_idx + 3..]);
    if !destination_words.contains(&"library") {
        return Ok(None);
    }

    let moved_tokens = trim_commas(&second_clause[1..on_idx]);
    let moved_words = words(&moved_tokens);
    let target = if moved_tokens.is_empty()
        || moved_words.as_slice() == ["it"]
        || moved_words.as_slice() == ["them"]
        || moved_words.as_slice() == ["those"]
        || moved_words.as_slice() == ["those", "cards"]
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&second_clause))
    } else {
        parse_target_phrase(&moved_tokens)?
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: true,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        },
    ]))
}

pub(crate) fn parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield(
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

    let first_clause = trim_commas(&tokens[..head_end]);
    let second_clause = trim_commas(&tokens[tail_start..]);
    if second_clause.is_empty() {
        return Ok(None);
    }

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(&first_clause)?
    else {
        return Ok(None);
    };

    let second_words = words(&second_clause);
    if second_words.len() < 4
        || second_words[0] != "you"
        || !matches!(second_words[1], "put" | "puts")
    {
        return Ok(None);
    }

    let Some(onto_idx) = second_clause.iter().position(|token| token.is_word("onto")) else {
        return Ok(None);
    };
    if onto_idx < 2 {
        return Ok(None);
    }

    let moved_words = words(&second_clause[2..onto_idx]);
    let moved_is_tagged_choice = moved_words == ["it"]
        || moved_words == ["that", "card"]
        || moved_words == ["that", "permanent"];
    if !moved_is_tagged_choice {
        return Ok(None);
    }

    let destination_words: Vec<&str> = words(&second_clause[onto_idx + 1..])
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if destination_words.first() != Some(&"battlefield") {
        return Ok(None);
    }
    let mut destination_tail: Vec<&str> = destination_words[1..].to_vec();
    let battlefield_tapped = destination_tail.contains(&"tapped");
    destination_tail.retain(|word| *word != "tapped");
    let battlefield_controller = if destination_tail.as_slice() == ["under", "your", "control"] {
        ReturnControllerAst::You
    } else if destination_tail.is_empty() {
        ReturnControllerAst::Preserve
    } else if destination_tail.as_slice() == ["under", "its", "owners", "control"]
        || destination_tail.as_slice() == ["under", "their", "owners", "control"]
        || destination_tail.as_slice() == ["under", "that", "players", "control"]
    {
        ReturnControllerAst::Owner
    } else {
        return Ok(None);
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::MoveToZone {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&second_clause)),
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller,
            battlefield_tapped,
            attached_to: None,
        },
    ]))
}
