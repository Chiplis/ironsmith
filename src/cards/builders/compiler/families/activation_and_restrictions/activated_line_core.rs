use super::*;

pub(crate) type ActivationRestrictionCompatWords<'a> = grammar::TokenWordView<'a>;

pub(crate) fn strip_prefix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<&'a [OwnedLexToken]> {
    grammar::parse_prefix(tokens, grammar::phrase(phrase)).map(|(_, rest)| rest)
}

pub(crate) fn strip_prefix_phrases<'a>(
    tokens: &'a [OwnedLexToken],
    phrases: &[&'static [&'static str]],
) -> Option<(&'static [&'static str], &'a [OwnedLexToken])> {
    phrases
        .iter()
        .find_map(|phrase| strip_prefix_phrase(tokens, phrase).map(|rest| (*phrase, rest)))
}

pub(crate) fn joined_activation_clause_text(tokens: &[OwnedLexToken]) -> String {
    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
}

pub(crate) fn parse_prefixed_activated_ability_label(
    tokens: &[OwnedLexToken],
    cost_start: usize,
) -> Option<String> {
    if cost_start == 0 {
        return None;
    }

    let prefix = ActivationRestrictionCompatWords::new(&tokens[..cost_start]);
    match prefix.get(prefix.len().saturating_sub(1)) {
        Some("boast") => Some("Boast".to_string()),
        Some("renew") => Some("Renew".to_string()),
        _ => None,
    }
}

pub(crate) fn contains_granted_keyword_before_word(
    words: &ActivationRestrictionCompatWords,
    keyword_idx: usize,
) -> bool {
    (0..keyword_idx)
        .filter_map(|idx| words.get(idx))
        .any(|word| matches!(word, "has" | "have"))
}

pub(crate) fn find_cycling_keyword_word_index(
    words: &ActivationRestrictionCompatWords,
) -> Option<usize> {
    let mut idx = 0usize;
    while idx < words.len() {
        if words
            .get(idx)
            .is_some_and(|word| str_strip_suffix(word, "cycling").is_some())
        {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

pub(crate) fn parse_hand_keyword_activated_body_lexed(
    body_tokens: &[OwnedLexToken],
    keyword: &str,
    display_label: &str,
    clause_text: &str,
) -> Result<Option<ParsedAbility>, CardTextError> {
    if body_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{keyword} line missing activated ability body (clause: '{clause_text}')",
        )));
    }

    let ability_tokens = trim_commas(body_tokens);
    let Some(mut parsed) = parse_activated_line_with_raw(&ability_tokens, None)? else {
        return Ok(None);
    };
    parsed.runtime_mut().text = Some(display_label.to_string());
    *parsed.functional_zones_mut() = vec![Zone::Hand];
    Ok(Some(parsed))
}

pub(crate) fn parse_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_activated_line_with_raw(tokens, None)
}

pub(crate) fn parse_activated_line_with_raw(
    tokens: &[OwnedLexToken],
    raw_line: Option<&str>,
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(colon_idx) = find_index(tokens, |token| token.is_colon()) else {
        return Ok(None);
    };

    let cost_start = find_activation_cost_start(&tokens[..colon_idx]).unwrap_or(0);
    let cost_tokens = &tokens[cost_start..colon_idx];
    let effect_tokens = &tokens[colon_idx + 1..];
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return Ok(None);
    }
    let loyalty_shorthand_cost = parse_loyalty_shorthand_activation_cost(cost_tokens, raw_line);
    let ability_label = parse_prefixed_activated_ability_label(tokens, cost_start);
    let apply_ability_label = |ability: &mut Ability| {
        if ability.text.is_none() {
            if let Some(label) = &ability_label {
                ability.text = Some(label.clone());
            }
        }
    };

    let mut effect_sentences = grammar::split_lexed_slices_on_period(effect_tokens);
    let functional_zones = infer_activated_functional_zones_lexed(cost_tokens, &effect_sentences);
    let mut timing = ActivationTiming::AnyTime;
    let scanned_modifiers = collect_activated_sentence_modifiers(&effect_sentences, timing.clone());
    let mana_activation_condition = scanned_modifiers.mana_activation_condition;
    let mut additional_activation_restrictions =
        scanned_modifiers.additional_activation_restrictions;
    let mana_usage_restrictions = scanned_modifiers.mana_usage_restrictions;
    let inline_effects_ast = scanned_modifiers.inline_effects_ast;
    effect_sentences = scanned_modifiers.kept_sentences;
    timing = scanned_modifiers.timing;
    let mana_activation_condition =
        combine_mana_activation_condition(mana_activation_condition, timing.clone());
    if !effect_sentences.is_empty() {
        let primary_sentence = &effect_sentences[0];
        let x_defined_by_cost = activation_cost_mentions_x(cost_tokens);
        let effect_words = ActivationRestrictionCompatWords::new(primary_sentence);
        let is_primary_add_clause = matches!(
            (
                effect_words.get(0),
                effect_words.get(1),
                effect_words.get(2)
            ),
            (Some("add" | "adds"), _, _)
                | (Some("you"), Some("add"), _)
                | (Some("that"), Some("player"), Some("add" | "adds"))
                | (Some("target"), Some("player"), Some("add" | "adds"))
        );
        if is_primary_add_clause {
            let mana_cost = if let Some(cost) = &loyalty_shorthand_cost {
                cost.clone()
            } else {
                parse_activation_cost(cost_tokens)?
            };
            let reference_imports = first_sacrifice_cost_choice_tag(&mana_cost)
                .or_else(|| last_exile_cost_choice_tag(&mana_cost))
                .map(ReferenceImports::with_last_object_tag)
                .unwrap_or_default();

            let mut extra_effects_ast = inline_effects_ast.clone();
            if effect_sentences.len() > 1 {
                for sentence in &effect_sentences[1..] {
                    if sentence.is_empty() {
                        continue;
                    }
                    let ast = parse_effect_sentence_lexed(sentence)?;
                    extra_effects_ast.extend(ast);
                }
            }

            let add_token_idx = find_index(primary_sentence, |token| {
                token.is_word("add") || token.is_word("adds")
            })
            .unwrap_or(0);
            let mana_tokens = &primary_sentence[add_token_idx + 1..];
            let mana_subject =
                (add_token_idx > 0).then(|| parse_subject(&primary_sentence[..add_token_idx]));
            let mana_words_view = ActivationRestrictionCompatWords::new(mana_tokens);
            let mana_words = mana_words_view.to_word_refs();
            let has_for_each_tail = mana_words_view.has_phrase(&["for", "each"]);
            let dynamic_amount = if has_for_each_tail {
                Some(
                    parse_dynamic_cost_modifier_value(mana_tokens)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported dynamic mana amount (clause: '{}')",
                            joined_activation_clause_text(primary_sentence)
                        ))
                    })?,
                )
            } else {
                parse_devotion_value_from_add_clause(mana_tokens)?
                    .or_else(|| parse_add_mana_equal_amount_value(mana_tokens))
            };

            let has_imprinted_colors = slice_contains(&mana_words, &"exiled")
                && (slice_contains(&mana_words, &"card") || slice_contains(&mana_words, &"cards"))
                && mana_words
                    .iter()
                    .any(|word| *word == "color" || *word == "colors");
            let has_any_combination_mana =
                contains_word_sequence(&mana_words, &["any", "combination", "of"]);
            let has_any_choice_mana = slice_contains(&mana_words, &"any")
                && (slice_contains(&mana_words, &"color")
                    || slice_contains(&mana_words, &"type")
                    || has_any_combination_mana);
            let has_or_choice_mana = slice_contains(&mana_words, &"or");
            let has_chosen_color =
                slice_contains(&mana_words, &"chosen") && slice_contains(&mana_words, &"color");
            let uses_commander_identity = mana_words
                .iter()
                .any(|word| *word == "commander" || *word == "commanders")
                && slice_contains(&mana_words, &"identity");
            let loyalty_timing = if loyalty_shorthand_cost.is_some() {
                ActivationTiming::SorcerySpeed
            } else {
                ActivationTiming::AnyTime
            };
            let loyalty_restrictions =
                loyalty_additional_restrictions(loyalty_shorthand_cost.is_some());
            let build_additional_restrictions = || {
                let mut restrictions = loyalty_restrictions.clone();
                restrictions.extend(additional_activation_restrictions.clone());
                restrictions
            };
            if has_imprinted_colors
                || has_any_choice_mana
                || has_or_choice_mana
                || uses_commander_identity
                || has_chosen_color
            {
                let mut mana_ast = parse_add_mana(mana_tokens, mana_subject.clone())?;
                resolve_activated_mana_x_requirements(
                    &mut mana_ast,
                    primary_sentence,
                    x_defined_by_cost,
                )?;
                let mut ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects: crate::resolution::ResolutionProgram::default(),
                        choices: vec![],
                        timing: loyalty_timing.clone(),
                        additional_restrictions: build_additional_restrictions(),
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition.clone(),
                        mana_usage_restrictions: mana_usage_restrictions.clone(),
                    }),
                    functional_zones: functional_zones.clone(),
                    text: None,
                };
                apply_ability_label(&mut ability);
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability: ability.into(),
                    effects_ast: Some(effects_ast),
                    reference_imports: reference_imports.clone(),
                    trigger_spec: None,
                }));
            }

            let mana: Vec<_> = mana_tokens
                .iter()
                .filter_map(|token| token.as_word())
                .filter(|word| !matches!(*word, "mana" | "to" | "your" | "pool"))
                .filter_map(|word| parse_mana_symbol(word).ok())
                .collect();

            if !mana.is_empty() {
                if dynamic_amount.is_none() && extra_effects_ast.is_empty() {
                    let mut ability = Ability {
                        kind: AbilityKind::Activated(ActivatedAbility {
                            mana_cost,
                            effects: crate::resolution::ResolutionProgram::default(),
                            choices: vec![],
                            timing: loyalty_timing.clone(),
                            additional_restrictions: build_additional_restrictions(),
                            activation_restrictions: vec![],
                            mana_output: Some(mana),
                            activation_condition: mana_activation_condition.clone(),
                            mana_usage_restrictions: mana_usage_restrictions.clone(),
                        }),
                        functional_zones: functional_zones.clone(),
                        text: None,
                    };
                    apply_ability_label(&mut ability);
                    return Ok(Some(ParsedAbility {
                        ability: ability.into(),
                        effects_ast: None,
                        reference_imports: ReferenceImports::default(),
                        trigger_spec: None,
                    }));
                }
                let mut mana_ast = parse_add_mana(mana_tokens, mana_subject)?;
                resolve_activated_mana_x_requirements(
                    &mut mana_ast,
                    primary_sentence,
                    x_defined_by_cost,
                )?;
                let mut ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects: crate::resolution::ResolutionProgram::default(),
                        choices: vec![],
                        timing: loyalty_timing,
                        additional_restrictions: build_additional_restrictions(),
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition.clone(),
                        mana_usage_restrictions: mana_usage_restrictions.clone(),
                    }),
                    functional_zones: functional_zones.clone(),
                    text: None,
                };
                apply_ability_label(&mut ability);
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability: ability.into(),
                    effects_ast: Some(effects_ast),
                    reference_imports: reference_imports,
                    trigger_spec: None,
                }));
            }
        }
    }

    // Generic activated ability: parse costs and effects from "<costs>: <effects>"
    let mana_cost = if let Some(cost) = &loyalty_shorthand_cost {
        cost.clone()
    } else {
        parse_activation_cost(cost_tokens)?
    };
    let effect_tokens_joined = join_sentences_with_period(
        &effect_sentences
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>(),
    );
    if effect_sentences.is_empty()
        && !additional_activation_restrictions.is_empty()
        && inline_effects_ast.is_empty()
    {
        return Ok(Some(ParsedAbility {
            ability: {
                let mut ability = Ability {
                    kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                        mana_cost,
                        effects: crate::resolution::ResolutionProgram::default(),
                        choices: vec![],
                        timing,
                        additional_restrictions: additional_activation_restrictions,
                        activation_restrictions: vec![],
                        mana_output: None,
                        activation_condition: None,
                        mana_usage_restrictions,
                    }),
                    functional_zones,
                    text: None,
                };
                apply_ability_label(&mut ability);
                ability
            }
            .into(),
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        }));
    }
    let mut effects_ast = parse_effect_sentences_lexed(&effect_tokens_joined)?;
    effects_ast.extend(inline_effects_ast);
    if effects_ast.is_empty() {
        return Ok(None);
    }
    let reference_imports = first_sacrifice_cost_choice_tag(&mana_cost)
        .or_else(|| last_exile_cost_choice_tag(&mana_cost))
        .map(ReferenceImports::with_last_object_tag)
        .unwrap_or_default();
    if loyalty_shorthand_cost.is_some() {
        timing = ActivationTiming::SorcerySpeed;
        for restriction in loyalty_additional_restrictions(true) {
            let already_present = additional_activation_restrictions.iter().any(|existing| {
                let existing_lower = existing.to_ascii_lowercase();
                let restriction_lower = restriction.to_ascii_lowercase();
                existing.eq_ignore_ascii_case(restriction.as_str())
                    || (existing_lower.matches("once each turn").next().is_some()
                        && restriction_lower.matches("once each turn").next().is_some())
            });
            if !already_present {
                additional_activation_restrictions.push(restriction);
            }
        }
    }

    Ok(Some(ParsedAbility {
        ability: {
            let mut ability = Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost,
                    effects: crate::resolution::ResolutionProgram::default(),
                    choices: vec![],
                    timing,
                    additional_restrictions: additional_activation_restrictions,
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions,
                }),
                functional_zones,
                text: None,
            };
            apply_ability_label(&mut ability);
            ability
        }
        .into(),
        effects_ast: Some(effects_ast),
        reference_imports,
        trigger_spec: None,
    }))
}

pub(crate) fn activation_cost_mentions_x(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .any(|word| {
            matches!(word, "x" | "+x" | "-x")
                || word
                    .split('/')
                    .any(|part| matches!(part, "x" | "+x" | "-x"))
        })
}

pub(crate) fn resolve_activated_mana_x_requirements(
    effect: &mut EffectAst,
    sentence_tokens: &[OwnedLexToken],
    x_defined_by_cost: bool,
) -> Result<(), CardTextError> {
    let clause_word_view = ActivationRestrictionCompatWords::new(sentence_tokens);
    let clause_words = clause_word_view.to_word_refs();
    if let Some(where_idx) = find_word_sequence_start(&clause_words, &["where", "x", "is"]) {
        let clause = clause_words.join(" ");
        let where_token_idx =
            token_index_for_word_index(sentence_tokens, where_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map where-x clause in mana ability (clause: '{clause}')"
                ))
            })?;
        let where_tokens = &sentence_tokens[where_token_idx..];
        let where_value = parse_where_x_value_clause_lexed(where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in mana ability (clause: '{clause}')"
            ))
        })?;
        replace_unbound_x_in_effect_anywhere(effect, &where_value, &clause)?;
    }

    let x_defined_by_removed_this_way = contains_word_sequence(&clause_words, &["this", "way"])
        && slice_contains(&clause_words, &"removed")
        && clause_words
            .iter()
            .any(|word| matches!(*word, "counter" | "counters"));

    if mana_effect_contains_unbound_x(effect)
        && !x_defined_by_cost
        && !x_defined_by_removed_this_way
    {
        return Err(CardTextError::ParseError(format!(
            "unresolved X in mana ability without an X activation cost or where-x definition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(())
}

pub(crate) fn mana_effect_contains_unbound_x(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. } => value_contains_unbound_x(amount),
        _ => {
            let mut contains_unbound_x = false;
            for_each_nested_effects(effect, true, |nested| {
                if nested.iter().any(mana_effect_contains_unbound_x) {
                    contains_unbound_x = true;
                }
            });
            contains_unbound_x
        }
    }
}

pub(crate) fn parse_loyalty_shorthand_activation_cost(
    cost_tokens: &[OwnedLexToken],
    raw_line: Option<&str>,
) -> Option<TotalCost> {
    let [token] = cost_tokens else {
        return None;
    };
    let word = token.as_word()?;
    if let Some(rest) = str_strip_prefix(word, "+")
        && let Ok(amount) = rest.parse::<u32>()
    {
        return Some(if amount == 0 {
            TotalCost::free()
        } else {
            TotalCost::from_cost(crate::costs::Cost::add_counters(
                CounterType::Loyalty,
                amount,
            ))
        });
    }
    if let Some(rest) = str_strip_prefix(word, "-") {
        if rest.eq_ignore_ascii_case("x") {
            return Some(TotalCost::from_cost(
                crate::costs::Cost::remove_any_counters_from_source(
                    Some(CounterType::Loyalty),
                    true,
                ),
            ));
        }
        if let Ok(amount) = rest.parse::<u32>() {
            return Some(TotalCost::from_cost(crate::costs::Cost::remove_counters(
                CounterType::Loyalty,
                amount,
            )));
        }
    }
    if word == "0"
        && raw_line.is_some_and(|line| {
            let mut parts = line.trim().splitn(2, ':');
            let Some(prefix) = parts.next() else {
                return false;
            };
            parts.next().is_some() && prefix.trim().replace('−', "-") == "0"
        })
    {
        return Some(TotalCost::free());
    }
    None
}

pub(crate) fn loyalty_additional_restrictions(is_loyalty_shorthand: bool) -> Vec<String> {
    if !is_loyalty_shorthand {
        return Vec::new();
    }
    vec!["Activate only once each turn.".to_string()]
}

pub(crate) fn first_sacrifice_cost_choice_tag(
    mana_cost: &crate::cost::TotalCost,
) -> Option<TagKey> {
    super::super::util::find_first_sacrifice_cost_choice_tag(mana_cost)
}

pub(crate) fn last_exile_cost_choice_tag(mana_cost: &crate::cost::TotalCost) -> Option<TagKey> {
    super::super::util::find_last_exile_cost_choice_tag(mana_cost)
}

pub(crate) fn infer_activated_functional_zones(
    cost_tokens: &[OwnedLexToken],
    effect_sentences: &[Vec<OwnedLexToken>],
) -> Vec<Zone> {
    let cost_words: Vec<&str> = crate::cards::builders::compiler::token_word_refs(cost_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let effect_words_match = |f: fn(&[&str]) -> bool| {
        effect_sentences.iter().any(|sentence| {
            let clause_words: Vec<&str> =
                crate::cards::builders::compiler::token_word_refs(sentence)
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
    } else if contains_from_command_zone_phrase(&cost_words)
        || effect_words_match(contains_from_command_zone_phrase)
    {
        vec![Zone::Command]
    } else if contains_source_from_your_hand_phrase(&cost_words)
        || contains_discard_source_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_hand_phrase)
    {
        vec![Zone::Hand]
    } else {
        vec![Zone::Battlefield]
    }
}

pub(crate) fn infer_activated_functional_zones_lexed(
    cost_tokens: &[OwnedLexToken],
    effect_sentences: &[&[OwnedLexToken]],
) -> Vec<Zone> {
    let cost_view = ActivationRestrictionCompatWords::new(cost_tokens);
    let cost_words: Vec<&str> = cost_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let effect_words_match = |f: fn(&[&str]) -> bool| {
        effect_sentences.iter().any(|sentence| {
            let sentence_view = ActivationRestrictionCompatWords::new(sentence);
            let clause_words: Vec<&str> = sentence_view
                .to_word_refs()
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
    } else if contains_from_command_zone_phrase(&cost_words)
        || effect_words_match(contains_from_command_zone_phrase)
    {
        vec![Zone::Command]
    } else if contains_source_from_your_hand_phrase(&cost_words)
        || contains_discard_source_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_hand_phrase)
    {
        vec![Zone::Hand]
    } else {
        vec![Zone::Battlefield]
    }
}

pub(crate) fn parse_activate_only_timing(tokens: &[OwnedLexToken]) -> Option<ActivationTiming> {
    activated_sentence_parsers::parse_activate_only_timing_lexed(tokens)
}

pub(crate) fn parse_activate_only_timing_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ActivationTiming> {
    activated_sentence_parsers::parse_activate_only_timing_lexed(tokens)
}

pub(crate) fn normalize_activate_only_restriction(
    tokens: &[OwnedLexToken],
    timing: &ActivationTiming,
) -> Option<String> {
    activated_sentence_parsers::normalize_activate_only_restriction(tokens, timing)
}

pub(crate) fn contains_word_sequence(words: &[&str], sequence: &[&str]) -> bool {
    find_word_sequence_start(words, sequence).is_some()
}

pub(crate) fn find_word_sequence_start(words: &[&str], sequence: &[&str]) -> Option<usize> {
    if sequence.is_empty() {
        Some(0)
    } else {
        find_word_slice_phrase_start(words, sequence)
    }
}

pub(crate) fn contains_any_word_sequence(words: &[&str], sequences: &[&[&str]]) -> bool {
    sequences
        .iter()
        .any(|sequence| contains_word_sequence(words, sequence))
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

pub(crate) fn is_activate_only_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_activate_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_activate_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_activate_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_spend_mana_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_spend_mana_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_spend_mana_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_spend_mana_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_usage_restriction_sentence(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    activated_sentence_parsers::parse_mana_usage_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    activated_sentence_parsers::parse_mana_usage_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    activated_sentence_parsers::parse_mana_spend_bonus_sentence_lexed(tokens)
}

pub(crate) fn is_any_player_may_activate_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_any_player_may_activate_sentence_lexed(tokens)
}

pub(crate) fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_any_player_may_activate_sentence_lexed(tokens)
}

pub(crate) fn is_trigger_only_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_trigger_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_trigger_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_triggered_times_each_turn_sentence(
    sentences: &[Vec<OwnedLexToken>],
) -> Option<u32> {
    activated_sentence_parsers::parse_triggered_times_each_turn_sentence(sentences)
}

pub(crate) fn parse_triggered_times_each_turn_from_words(words: &[&str]) -> Option<u32> {
    activated_sentence_parsers::parse_triggered_times_each_turn_from_words(words)
}

pub(crate) fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    activated_sentence_parsers::parse_triggered_times_each_turn_lexed(tokens)
}

pub(crate) fn parse_named_number(word: &str) -> Option<u32> {
    parse_number_word_u32(word)
}

pub(crate) fn parse_activation_cost(tokens: &[OwnedLexToken]) -> Result<TotalCost, CardTextError> {
    let cst = parse_activation_cost_tokens_rewrite(tokens)?;
    lower_activation_cost_cst(&cst)
}

pub(crate) fn parse_devotion_value_from_add_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    let Some(devotion_idx) = find_index(&words, |word| *word == "devotion") else {
        return Ok(None);
    };

    let player = parse_devotion_player_from_words(&words, devotion_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported devotion player in clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let to_idx = find_index(&words[devotion_idx + 1..], |word| *word == "to")
        .map(|idx| devotion_idx + 1 + idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing color after devotion clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    if words.get(to_idx + 1).copied() == Some("that")
        && words.get(to_idx + 2).copied() == Some("color")
    {
        return Ok(Some(Value::DevotionToChosenColor(player)));
    }
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
    if matches!(left, [.., "your"]) {
        return Some(PlayerFilter::You);
    }
    if matches!(left, [.., "opponent"] | [.., "opponents"]) {
        return Some(PlayerFilter::Opponent);
    }
    if matches!(left, [.., "that", "players"] | [.., "that", "player"]) {
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
        if colors.intersection(ColorSet::from_color(color)).count() > 0 {
            if found.is_some() {
                return None;
            }
            found = Some(color);
        }
    }
    found
}

pub(crate) fn parse_activation_condition(tokens: &[OwnedLexToken]) -> Option<crate::ConditionExpr> {
    activated_sentence_parsers::parse_activation_condition_lexed(tokens)
}

pub(crate) fn parse_activation_condition_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    activated_sentence_parsers::parse_activation_condition_lexed(tokens)
}

pub(crate) fn parse_cardinal_u32(word: &str) -> Option<u32> {
    let token = OwnedLexToken::word(word.to_string(), TextSpan::synthetic());
    parse_number(&[token]).map(|(value, _)| value)
}

pub(crate) fn parse_activation_count_per_turn(words: &[&str]) -> Option<u32> {
    activated_sentence_parsers::parse_activation_count_per_turn(words)
}

pub(crate) fn parse_enters_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }
    if is_negated_untap_clause(&clause_words) {
        let has_enters_tapped =
            slice_contains(&clause_words, &"enters") && slice_contains(&clause_words, &"tapped");
        if has_enters_tapped {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if clause_words.first().copied() == Some("this")
        && slice_contains(&clause_words, &"enters")
        && slice_contains(&clause_words, &"tapped")
    {
        let tapped_word_idx =
            find_index(&clause_words, |word| *word == "tapped").ok_or_else(|| {
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
        let trailing_words =
            crate::cards::builders::compiler::token_word_refs(&tokens[tapped_token_idx + 1..]);
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

pub(crate) fn parse_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if slice_starts_with(&line_words, &["this", "cost", "is", "reduced", "by"])
        && line_words.len() > 6
    {
        let amount_tokens = trim_commas(&tokens[5..]);
        let parsed_amount = parse_cost_modifier_amount(&amount_tokens);
        let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
        let amount_fixed = if let Value::Fixed(value) = amount_value {
            value
        } else {
            1
        };
        let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
        let remaining_words = crate::cards::builders::compiler::token_word_refs(remaining_tokens);
        if slice_contains(&remaining_words, &"for")
            && slice_contains(&remaining_words, &"each")
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
        return Err(CardTextError::ParseError(format!(
            "unsupported cost-reduction static clause (clause: '{}')",
            text
        )));
    }

    if slice_starts_with(&line_words, &["activated", "abilities", "of"]) {
        let Some(cost_idx) = find_index(&line_words, |word| *word == "cost" || *word == "costs")
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
        let tail_words = crate::cards::builders::compiler::token_word_refs(&amount_tokens[used..]);
        if !slice_starts_with(&tail_words, &["less", "to", "activate"]) {
            return Ok(None);
        }

        return Ok(Some(StaticAbility::reduce_activated_ability_costs(
            filter,
            reduction,
            Some(1),
        )));
    }

    if slice_starts_with(&line_words, &["this", "ability", "costs"]) {
        let amount_tokens = trim_commas(&tokens[3..]);
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
        let tail_tokens = trim_commas(&amount_tokens[used..]);
        let tail_words = crate::cards::builders::compiler::token_word_refs(&tail_tokens);
        if tail_words == ["less", "to", "activate"] {
            return Ok(Some(StaticAbility::reduce_activated_ability_costs(
                ObjectFilter::source(),
                reduction,
                Some(1),
            )));
        }
        if slice_starts_with(&tail_words, &["less", "to", "activate", "if"]) {
            let condition_tokens = trim_commas(&tail_tokens[4..]);
            let condition_words =
                crate::cards::builders::compiler::token_word_refs(&condition_tokens);
            if condition_words.first().copied() == Some("it")
                && condition_words.get(1).copied() == Some("targets")
            {
                let (count, used) = parse_number(&condition_tokens[2..]).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported activated-ability target condition count (clause: '{}')",
                        line_words.join(" ")
                    ))
                })?;
                let mut filter = parse_object_filter(&condition_tokens[2 + used..], false)
                    .map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported activated-ability target condition filter (clause: '{}')",
                            line_words.join(" ")
                        ))
                    })?;
                if filter.zone.is_none() {
                    filter.zone = Some(Zone::Battlefield);
                }
                return Ok(Some(
                    StaticAbility::reduce_activated_ability_costs_if_targets(
                        ObjectFilter::source(),
                        reduction,
                        crate::static_abilities::ActivatedAbilityCostCondition::TargetsExactly {
                            count: count as usize,
                            filter,
                        },
                        Some(1),
                    ),
                ));
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported activated-ability cost reduction condition (clause: '{}')",
                line_words.join(" ")
            )));
        }
        if slice_starts_with(&tail_words, &["less", "to", "activate", "for", "each"]) {
            let mut per_filter = parse_object_filter(&tail_tokens[5..], false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported activated-ability cost reduction tail (clause: '{}')",
                    line_words.join(" ")
                ))
            })?;
            if per_filter.zone.is_none() {
                per_filter.zone = Some(Zone::Battlefield);
            }
            return Ok(Some(
                StaticAbility::reduce_activated_ability_costs_for_each(
                    ObjectFilter::source(),
                    reduction,
                    per_filter,
                    Some(1),
                ),
            ));
        }
    }

    if !slice_starts_with(&line_words, &["this", "spell", "costs"]) {
        return Ok(None);
    }

    let costs_idx = find_index(tokens, |token| token.is_word("costs"))
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
    let remaining_words: Vec<&str> =
        crate::cards::builders::compiler::token_word_refs(remaining_tokens);

    if !slice_contains(&remaining_words, &"less") {
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

    let has_each = slice_contains(&remaining_words, &"each");
    let has_card_type = contains_word_sequence(&remaining_words, &["card", "type"]);
    let has_graveyard = slice_contains(&remaining_words, &"graveyard");

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
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    if matches!(
        words.as_slice(),
        [
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "this",
            "creature",
            "do",
            "so"
        ] | [
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "this",
            "do",
            "so"
        ]
    ) {
        return Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter: ObjectFilter::creature(),
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
            condition: None,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_source_must_be_blocked_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    if matches!(
        words.as_slice(),
        ["this", "creature", "must", "be", "blocked", "if", "able"]
            | ["this", "must", "be", "blocked", "if", "able"]
    ) {
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
