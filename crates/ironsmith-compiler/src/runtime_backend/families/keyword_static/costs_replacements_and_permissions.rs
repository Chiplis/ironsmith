use super::*;

fn cost_words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool {
    words
        .windows(phrase.len())
        .any(|candidate| candidate == phrase)
}

fn is_exact_per_target_cost_modifier(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(idx, word)| {
        *word == "for"
            && words.get(idx + 1) == Some(&"each")
            && words.get(idx + 2) == Some(&"target")
            && words.get(idx + 3) != Some(&"beyond")
    })
}

fn apply_anywhere_other_than_hand_origin(filter: &mut ObjectFilter) {
    filter.any_of = [
        (Zone::Hand, Some(PlayerFilter::NotYou)),
        (Zone::Library, None),
        (Zone::Battlefield, None),
        (Zone::Graveyard, None),
        (Zone::Exile, None),
        (Zone::Command, None),
        (Zone::OutsideGame, None),
    ]
    .into_iter()
    .map(|(zone, owner)| {
        let mut branch = ObjectFilter::default();
        branch.zone = Some(zone);
        branch.owner = owner;
        branch
    })
    .collect();
}

fn apply_this_spell_cost_increase_condition(
    filter: &mut ObjectFilter,
    condition: crate::static_abilities::ThisSpellCostCondition,
    clause_words: &[&str],
) -> Result<Option<crate::ConditionExpr>, CardTextError> {
    use crate::static_abilities::ThisSpellCostCondition;

    match condition {
        ThisSpellCostCondition::Always => Ok(None),
        ThisSpellCostCondition::YourTurn => Ok(Some(crate::ConditionExpr::YourTurn)),
        ThisSpellCostCondition::NotYourTurn => Ok(Some(crate::ConditionExpr::Not(Box::new(
            crate::ConditionExpr::YourTurn,
        )))),
        ThisSpellCostCondition::ConditionExpr { condition, .. } => Ok(Some(condition)),
        ThisSpellCostCondition::TargetsPlayer(player) => {
            filter.targets_player = Some(player);
            Ok(None)
        }
        ThisSpellCostCondition::TargetsObject(object) => {
            filter.targets_object = Some(Box::new(object));
            Ok(None)
        }
        other => Err(CardTextError::ParseError(format!(
            "unsupported self-only cost-increase condition (clause: '{}'; condition: {other:?})",
            clause_words.join(" ")
        ))),
    }
}

pub(crate) fn parse_cost_modifier_target_spec(
    target_tokens: &[OwnedLexToken],
) -> Result<(Option<PlayerFilter>, Option<Box<ObjectFilter>>, bool), CardTextError> {
    let alternatives =
        crate::runtime_backend::grammar::primitives::split_lexed_slices_on_or(target_tokens);
    if let [left, right] = alternatives.as_slice() {
        let (left_player, left_object, left_any_of) = parse_cost_modifier_target_spec(left)?;
        let (right_player, right_object, right_any_of) = parse_cost_modifier_target_spec(right)?;
        if !left_any_of && !right_any_of {
            match (left_player, left_object, right_player, right_object) {
                (Some(player), None, None, Some(object))
                | (None, Some(object), Some(player), None) => {
                    return Ok((Some(player), Some(object), true));
                }
                _ => {}
            }
        }
    }

    match static_mid_facts::parse_cost_modifier_target_fact(target_tokens) {
        Some(static_mid_facts::CostTargetFact::You) => Ok((Some(PlayerFilter::You), None, false)),
        Some(static_mid_facts::CostTargetFact::Opponent) => {
            Ok((Some(PlayerFilter::Opponent), None, false))
        }
        Some(static_mid_facts::CostTargetFact::AnyPlayer) => {
            Ok((Some(PlayerFilter::Any), None, false))
        }
        Some(static_mid_facts::CostTargetFact::Object(filter)) => {
            Ok((None, Some(Box::new(filter)), false))
        }
        None => Ok((
            None,
            Some(Box::new(parse_object_filter(target_tokens, false)?)),
            false,
        )),
    }
}

pub(crate) fn parse_cost_modifier_prefix_condition(
    tokens: &[OwnedLexToken],
    spells_token_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    if let Some(prefix) =
        keyword_static_lines::parse_cost_prefix_condition_tokens(tokens, spells_token_idx)
    {
        match prefix {
            keyword_static_lines::CostPrefixCondition::DuringTurnsOtherThanYours {
                subject_start,
            } => {
                return Ok((
                    Some(crate::ConditionExpr::Not(Box::new(
                        crate::ConditionExpr::YourTurn,
                    ))),
                    subject_start,
                ));
            }
            keyword_static_lines::CostPrefixCondition::DuringYourTurn { subject_start } => {
                return Ok((Some(crate::ConditionExpr::YourTurn), subject_start));
            }
            keyword_static_lines::CostPrefixCondition::AsLongAs {
                condition_tokens,
                subject_start,
            } => {
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after leading 'as long as' clause (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )));
                }
                let condition = match parse_static_condition_clause(&condition_tokens) {
                    Ok(condition) => condition,
                    Err(_) => parse_source_tap_status_condition_lexed(&condition_tokens)
                        .ok_or_else(|| {
                            CardTextError::ParseError(format!(
                                "unsupported static condition clause (clause: '{}')",
                                crate::runtime_backend::token_word_refs(&condition_tokens)
                                    .join(" ")
                            ))
                        })?,
                };
                return Ok((Some(condition), subject_start));
            }
        }
    }

    Ok((None, 0))
}

pub(crate) fn parse_optional_life_additional_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let additional_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(spec) = static_keyword_cost_shapes::parse_additional_cost_spell_filter(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.spell_filter_tokens);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_spell_filter_with_grammar_entrypoint(&subject_tokens);
    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    if static_keyword_cost_shapes::parse_optional_life_subject_is_permanent(&subject_words) {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }
    filter.cast_by = Some(PlayerFilter::You);

    let Some(optional_life_shape) =
        static_keyword_cost_shapes::parse_optional_life_reduction_words(&additional_words)
    else {
        return Ok(None);
    };
    let pay_word_idx = optional_life_shape.pay.word;
    let payment_words = &additional_words[pay_word_idx + 1..];
    let Some(life_cost) = payment_words
        .first()
        .and_then(|word| parse_number_word_i32(word))
        .and_then(|amount| u32::try_from(amount).ok())
    else {
        return Ok(None);
    };
    if !optional_life_shape.payment_has_life {
        return Ok(None);
    }

    if !optional_life_shape.those_spells_paid_life_this_way {
        return Ok(None);
    }
    let costs_word_idx = optional_life_shape.costs.word;
    let Some(costs_idx) = static_keyword_shapes::parse_word_token_offset(tokens, costs_word_idx)
    else {
        return Ok(None);
    };
    let amount_tokens = &tokens[costs_idx + 1..];
    let (_, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let Some((reduction, _)) = parsed_mana_cost else {
        return Ok(None);
    };
    let remaining_words = crate::runtime_backend::token_word_refs(amount_tokens);
    if static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
        != Some(CostModifierDirection::Less)
        || !static_keyword_cost_shapes::parse_cost_modifier_cast_marker(&remaining_words)
    {
        return Ok(None);
    }

    let label_end = locate_token_kind(tokens, TokenKind::Period)
        .map(|idx| idx + 1)
        .unwrap_or(costs_idx);
    let label = render_token_slice(&tokens[..label_end])
        .trim()
        .trim_end_matches('.')
        .to_string();
    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostReductionManaCost::new(filter, reduction)
            .with_optional_life_additional_cost(label, life_cost),
    )))
}

pub(crate) fn parse_spells_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some(ability) = parse_optional_life_additional_cost_reduction_line(tokens)? {
        return Ok(Some(ability));
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }
    let Some(spells_token_idx) =
        static_keyword_cost_shapes::parse_spells_subject(tokens).map(|boundary| boundary.token)
    else {
        return Ok(None);
    };

    if static_mid_facts::parse_first_spell_each_turn_cost_fact(tokens).is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported first-spell-each-turn cost modifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (prefix_condition, subject_start) =
        parse_cost_modifier_prefix_condition(tokens, spells_token_idx)?;
    if subject_start > spells_token_idx {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[subject_start..spells_token_idx]);
    let is_this_spell = is_this_subject_reference_lexed(&subject_tokens);

    let Some(cost_token_idx) =
        static_mid_facts::parse_cost_component_boundary(tokens, spells_token_idx + 1)
            .map(|boundary| boundary.cost_token)
    else {
        return Ok(None);
    };
    if cost_token_idx <= spells_token_idx {
        return Ok(None);
    }

    let mut filter = if is_this_spell {
        ObjectFilter::default()
    } else {
        parse_spell_filter_with_grammar_entrypoint(&subject_tokens)
    };

    let between_tokens = &tokens[spells_token_idx + 1..cost_token_idx];
    if !is_this_spell {
        let between_fact = static_mid_facts::parse_spell_cost_between_fact(between_tokens);
        let between_words = crate::runtime_backend::token_word_refs(between_tokens);
        for descriptor_tokens in between_fact.descriptor_segments {
            let extra_filter = parse_spell_filter_with_grammar_entrypoint(
                strip_relative_target_clause(&descriptor_tokens),
            );
            if spell_filter_has_identity(&extra_filter) {
                merge_spell_filters(&mut filter, extra_filter);
            }
        }
        let between_filter = parse_spell_filter_with_grammar_entrypoint(
            strip_relative_target_clause(between_tokens),
        );
        if spell_filter_has_identity(&between_filter) {
            merge_spell_filters(&mut filter, between_filter);
        }
        match between_fact.actor {
            Some(static_mid_facts::SpellCastActorFact::You) => {
                filter.cast_by = Some(PlayerFilter::You)
            }
            Some(static_mid_facts::SpellCastActorFact::Opponent) => {
                filter.cast_by = Some(PlayerFilter::Opponent)
            }
            None => {}
        }
        if between_fact.from_your_graveyard {
            filter.zone = Some(Zone::Graveyard);
            filter.owner = Some(PlayerFilter::You);
        }
        if cost_words_contain_phrase(
            &between_words,
            &["from", "anywhere", "other", "than", "your", "hand"],
        ) {
            filter.zone = None;
            filter.owner = None;
            apply_anywhere_other_than_hand_origin(&mut filter);
        }
        if cost_words_contain_phrase(&between_words, &["but", "don't", "own"])
            || cost_words_contain_phrase(&between_words, &["but", "dont", "own"])
        {
            filter.owner = Some(PlayerFilter::NotYou);
        }
        if let Some(target_tokens) = between_fact.target_tokens {
            let (target_player, target_object, targets_any_of) =
                parse_cost_modifier_target_spec(target_tokens)?;
            filter.targets_player = target_player;
            filter.targets_object = target_object;
            filter.targets_any_of = targets_any_of;
        }
    }

    let amount_tokens = &tokens[cost_token_idx + 1..];
    let (parsed_amount, mut parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let mut parsed_mana_cost_repetitions = None;
    let (mut amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or_else(|| {
            if let Some((_, used)) = &parsed_mana_cost {
                (Value::Fixed(1), *used)
            } else {
                (Value::Fixed(1), 0)
            }
        });
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let direction_words = if let Some(if_idx) =
        static_keyword_cost_shapes::parse_cost_direction_if_boundary(&remaining_words)
            .map(|boundary| boundary.word)
    {
        &remaining_words[..if_idx]
    } else {
        &remaining_words
    };
    let Some(direction) = static_mid_facts::parse_cost_modifier_direction_words(direction_words)
    else {
        return Ok(None);
    };
    let is_life_cost_modifier = remaining_words.iter().any(|word| *word == "life");
    let per_target = !is_life_cost_modifier && is_exact_per_target_cost_modifier(&remaining_words);
    let per_additional_target = cost_words_contain_phrase(
        &remaining_words,
        &["for", "each", "target", "beyond", "the", "first"],
    );

    if !per_target && let Some(dynamic_value) = parse_dynamic_cost_modifier_value(remaining_tokens)?
    {
        if parsed_mana_cost.is_some() && is_this_spell {
            parsed_mana_cost_repetitions = Some(dynamic_value);
        } else {
            if parsed_mana_cost.is_some() {
                parsed_mana_cost = None;
            }
            let multiplier = parsed_amount
                .as_ref()
                .and_then(|(value, _)| match value {
                    Value::Fixed(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(1);
            amount_value = scale_dynamic_cost_modifier_value(dynamic_value, multiplier);
        }
    } else if parsed_amount.is_none() && parsed_mana_cost.is_none() {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    }

    // Handle trailing "where X is ..." clauses, e.g.
    // "This spell costs {X} less to cast, where X is the number of differently named lands you control."
    if let Some(where_tokens) = static_mid_facts::parse_where_x_clause_tokens(remaining_tokens) {
        let clause = clause_words.join(" ");
        let x_value = parse_value_binding_clause(where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in spells-cost modifier (clause: '{clause}')"
            ))
        })?;
        if !value_contains_unbound_x(&amount_value) {
            return Err(CardTextError::ParseError(format!(
                "missing where-x clause in spells-cost modifier (clause: '{clause}')"
            )));
        }
        amount_value = replace_unbound_x_with_value(amount_value, &x_value, &clause)?;
    }
    if direction == CostModifierDirection::Less
        && let Some(cap) = parse_cost_reduction_cap(remaining_tokens)
    {
        amount_value = Value::Min(Box::new(amount_value), Box::new(Value::Fixed(cap)));
    }

    if !is_this_spell {
        parse_trailing_targets_condition_in_cost_modifier(
            &mut filter,
            remaining_tokens,
            &clause_words,
        )?;
    }

    let this_spell_condition = if is_this_spell {
        if let Some(condition) =
            parse_trailing_this_spell_cost_condition(remaining_tokens, &clause_words)?
        {
            condition
        } else if let Some(prefix) = &prefix_condition {
            match prefix {
                crate::ConditionExpr::YourTurn => {
                    crate::static_abilities::ThisSpellCostCondition::YourTurn
                }
                crate::ConditionExpr::Not(inner)
                    if matches!(inner.as_ref(), crate::ConditionExpr::YourTurn) =>
                {
                    crate::static_abilities::ThisSpellCostCondition::NotYourTurn
                }
                other => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported leading this-spell cost condition (clause: '{}'; condition: {other:?})",
                        clause_words.join(" ")
                    )));
                }
            }
        } else {
            crate::static_abilities::ThisSpellCostCondition::Always
        }
    } else {
        crate::static_abilities::ThisSpellCostCondition::Always
    };

    let non_this_condition = if is_this_spell {
        None
    } else {
        prefix_condition.clone()
    };

    if direction == CostModifierDirection::Less {
        // "This spell costs {N} less to cast" is a self-only modifier that should not
        // apply from the permanent on the battlefield after it resolves.
        if is_this_spell && parsed_mana_cost.is_none() {
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    amount_value,
                    this_spell_condition,
                ),
            )));
        }
        if is_this_spell && let Some((cost, _)) = parsed_mana_cost.clone() {
            let mut ability = crate::static_abilities::ThisSpellCostReductionManaCost::new(
                cost,
                this_spell_condition,
            );
            if let Some(repetitions) = parsed_mana_cost_repetitions {
                ability = ability.with_repetitions(repetitions);
            }
            return Ok(Some(StaticAbility::new(ability)));
        }
        if let Some((cost, _)) = parsed_mana_cost {
            let mut ability = crate::static_abilities::CostReductionManaCost::new(filter, cost);
            if per_target {
                ability = ability.with_per_target();
            }
            if let Some(condition) = non_this_condition.clone() {
                ability = ability.with_condition(condition);
            }
            return Ok(Some(StaticAbility::new(ability)));
        }
        let mut ability = crate::static_abilities::CostReduction::new(filter, amount_value);
        if per_target {
            ability = ability.with_per_target();
        }
        if let Some(condition) = non_this_condition.clone() {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(ability)));
    }

    let source_only_increase = is_this_spell && !is_life_cost_modifier && !per_additional_target;
    let mut source_only_condition = None;
    if source_only_increase {
        filter = ObjectFilter::source();
        source_only_condition = apply_this_spell_cost_increase_condition(
            &mut filter,
            this_spell_condition,
            &clause_words,
        )?;
    }

    if let Some((cost, _)) = parsed_mana_cost {
        let mut ability = crate::static_abilities::CostIncreaseManaCost::new(filter, cost);
        if per_target {
            ability = ability.with_per_target();
        }
        if let Some(condition) = source_only_condition
            .clone()
            .or_else(|| non_this_condition.clone())
        {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(ability)));
    }

    let mut ability = crate::static_abilities::CostIncrease::new(filter, amount_value);
    if per_target {
        ability = ability.with_per_target();
    }
    if let Some(condition) = source_only_condition.or_else(|| non_this_condition.clone()) {
        ability = ability.with_condition(condition);
    }
    Ok(Some(StaticAbility::new(ability)))
}

pub(crate) fn parse_spell_and_player_activated_ability_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(and_idx) = static_keyword_cost_shapes::parse_spell_and_abilities_separator(tokens)
        .map(|boundary| boundary.token)
    else {
        return Ok(None);
    };
    let right_start = and_idx + 1;

    let left_tokens = trim_commas(&tokens[..and_idx]);
    let right_tokens = trim_commas(&tokens[right_start..]);
    let Some(spell_cost_ability) = parse_spells_cost_modifier_line(&left_tokens)? else {
        return Ok(None);
    };
    let Some(mut activated_cost_ability) =
        parse_player_activated_ability_cost_modifier_clause(&right_tokens)?
    else {
        return Ok(None);
    };

    if let Some(spells_idx) =
        static_keyword_cost_shapes::parse_spells_subject(tokens).map(|boundary| boundary.token)
    {
        let (prefix_condition, _) = parse_cost_modifier_prefix_condition(tokens, spells_idx)?;
        if let Some(condition) = prefix_condition {
            activated_cost_ability = activated_cost_ability.with_condition(condition);
        }
    }

    Ok(Some(vec![spell_cost_ability, activated_cost_ability]))
}

pub(crate) fn parse_cycling_cost_alternative_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(fact) = static_mid_facts::parse_cycling_cost_alternative_fact(tokens) else {
        return Ok(None);
    };

    let condition = fact
        .condition_tokens
        .map(parse_static_condition_clause)
        .transpose()?;
    let replacement_mana_cost = if fact.replacement_cost_tokens.is_empty() {
        ManaCost::new()
    } else {
        let replacement_total_cost = parse_activation_cost(fact.replacement_cost_tokens)?;
        if replacement_total_cost.has_non_mana_costs() {
            return Err(CardTextError::ParseError(format!(
                "unsupported non-mana cycling alternative cost (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        replacement_total_cost.mana_cost().cloned().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing cycling alternative mana cost (clause: '{}')",
                clause_words.join(" ")
            ))
        })?
    };

    let mut filter = ObjectFilter::default().with_ability_marker("cycling");
    filter.zone = Some(Zone::Hand);
    let display = format!(
        "You may pay {} rather than pay cycling costs",
        replacement_mana_cost.to_oracle()
    );
    let mut ability =
        StaticAbility::replace_activated_ability_mana_cost(filter, replacement_mana_cost, display);
    if let Some(condition) = condition {
        ability = ability.with_condition(condition);
    }
    Ok(Some(ability))
}

pub(crate) fn parse_player_activated_ability_cost_modifier_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 7
        || !clause_words
            .first()
            .is_some_and(|word| *word == "abilities")
    {
        return Ok(None);
    }

    let Some(cost_words) =
        static_keyword_cost_shapes::parse_player_ability_cost_words(&clause_words)
    else {
        return Ok(None);
    };
    let activate_idx = cost_words.activate.word;
    let clause = LexedClause::new(tokens);
    let Some(activator_clause) = clause.between_word_range(1, activate_idx) else {
        return Ok(None);
    };
    let activator =
        match static_mid_facts::parse_activated_ability_cost_actor(activator_clause.tokens()) {
            Some(static_mid_facts::ActivatedAbilityCostActorFact::You) => PlayerFilter::You,
            Some(static_mid_facts::ActivatedAbilityCostActorFact::Opponent) => {
                PlayerFilter::Opponent
            }
            None => return Ok(None),
        };

    let cost_idx = cost_words.costs.word;
    let cost_token_idx = static_keyword_shapes::parse_word_token_offset(tokens, cost_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map activated-ability cost modifier amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let amount_tokens = &tokens[cost_token_idx + 1..];
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (increase, used) = if let Some((mana_cost, used)) = parsed_mana_cost {
        (TotalCost::mana(mana_cost), used)
    } else if let Some((Value::Fixed(amount), used)) = parsed_amount {
        if amount < 0 {
            return Ok(None);
        }
        let generic = amount.min(u8::MAX as i32) as u8;
        (
            TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(generic)])),
            used,
        )
    } else {
        return Ok(None);
    };
    let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let Some(tail_fact) = static_mid_facts::parse_activated_ability_cost_tail(remaining_tokens)
    else {
        return Ok(None);
    };
    if static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
        != Some(CostModifierDirection::More)
    {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::increase_activated_ability_costs_for_activator(
            activator,
            increase,
            tail_fact.excludes_mana_abilities,
        ),
    ))
}

pub(crate) fn strip_relative_target_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let Some(target_clause_idx) = static_keyword_cost_shapes::parse_relative_target_clause(tokens)
        .map(|boundary| boundary.token)
    else {
        return tokens;
    };

    &tokens[..target_clause_idx]
}

pub(crate) fn parse_trailing_targets_condition_in_cost_modifier(
    filter: &mut ObjectFilter,
    remaining_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(), CardTextError> {
    let Some(fact) = static_mid_facts::parse_trailing_target_condition(remaining_tokens) else {
        return Ok(());
    };
    if fact.target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in trailing spells-cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (targets_player, targets_object, targets_any_of) =
        parse_cost_modifier_target_spec(fact.target_tokens)?;
    filter.targets_player = targets_player;
    filter.targets_object = targets_object;
    filter.targets_any_of = targets_any_of;
    Ok(())
}

pub(crate) fn parse_flashback_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some((kind, consumed)) = parse_alternative_cast_words(&clause_words) else {
        return Ok(None);
    };
    if clause_words.len() < consumed + 5 {
        return Ok(None);
    }
    if clause_words.get(consumed).copied() != Some("costs") {
        return Ok(None);
    }
    let Some(cost_idx) =
        static_keyword_cost_shapes::parse_last_cost_verb(tokens).map(|boundary| boundary.token)
    else {
        return Ok(None);
    };
    let amount_tokens = &tokens[cost_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or((Value::Fixed(1), 0));
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let Some(direction) = static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
    else {
        return Ok(None);
    };
    if parsed_amount.is_none() {
        return Err(CardTextError::ParseError(
            "missing flashback cost modifier amount".to_string(),
        ));
    }

    let mut filter = ObjectFilter::default();
    filter.alternative_cast = Some(kind);
    match static_mid_facts::parse_alternative_cost_payer(tokens) {
        Some(static_mid_facts::AlternativeCostPayerFact::You) => {
            filter.cast_by = Some(PlayerFilter::You)
        }
        Some(static_mid_facts::AlternativeCostPayerFact::Opponent) => {
            filter.cast_by = Some(PlayerFilter::Opponent)
        }
        None => {}
    }

    if direction == CostModifierDirection::Less {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, amount_value),
        )));
    }
    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostIncrease::new(filter, amount_value),
    )))
}

pub(crate) fn parse_equip_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(head) = keyword_static_lines::parse_equip_cost_modifier_head_tokens(tokens) else {
        return Ok(None);
    };
    let cost_idx = head.cost_token;

    let amount_tokens = &tokens[cost_idx + 1..];
    let Some((amount_value, used)) = parse_cost_modifier_amount(amount_tokens) else {
        return Ok(None);
    };
    let Value::Fixed(amount) = amount_value else {
        return Ok(None);
    };
    if amount < 0 {
        return Ok(None);
    }

    let remaining_words = crate::runtime_backend::token_word_refs(&amount_tokens[used..]);
    let Some(direction) = static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
    else {
        return Ok(None);
    };

    let mut filter = if head.source_relative_equipment {
        ObjectFilter::source().with_ability_marker("equip")
    } else {
        ObjectFilter::default().with_ability_marker("equip")
    };
    match head.payer {
        keyword_static_lines::EquipCostPayer::You => filter.controller = Some(PlayerFilter::You),
        keyword_static_lines::EquipCostPayer::Opponent => {
            filter.controller = Some(PlayerFilter::Opponent)
        }
        keyword_static_lines::EquipCostPayer::Unspecified => {}
    }

    if direction == CostModifierDirection::Less {
        let amount_text = format!("{{{amount}}}");
        let display = if head.source_relative_equipment {
            format!("This Equipment's equip abilities cost {amount_text} less to activate")
        } else if filter.controller == Some(PlayerFilter::Opponent) {
            format!("Equip costs your opponents pay cost {amount_text} less")
        } else {
            format!("Equip costs you pay cost {amount_text} less")
        };
        return Ok(Some(
            StaticAbility::reduce_activated_ability_costs_with_display(
                filter,
                amount as u32,
                None,
                display,
            ),
        ));
    }

    let increase = TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(
        amount.min(u8::MAX as i32) as u8,
    )]));
    Ok(Some(StaticAbility::increase_activated_ability_costs(
        filter, increase,
    )))
}

pub(crate) fn parse_foretelling_cards_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 7 {
        return Ok(None);
    }
    let Some(fact) = static_mid_facts::parse_foretell_cost_modifier_fact(tokens) else {
        return Ok(None);
    };

    if fact.direction != CostModifierDirection::Less || !fact.during_any_players_turn {
        return Ok(None);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported foretelling cost modifier clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_cost_modifier_amount(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    if let Some((amount, used)) = parse_number(tokens) {
        return Some((Value::Fixed(amount as i32), used));
    }

    let first_token = tokens.first()?;
    let group = mana_pips_from_token(first_token)?;
    if group.len() != 1 {
        return None;
    }
    let symbol = group[0];
    if let ManaSymbol::Generic(amount) = symbol {
        return Some((Value::Fixed(amount as i32), 1));
    }
    if symbol == ManaSymbol::X {
        return Some((Value::X, 1));
    }
    None
}

pub(crate) fn parse_cost_modifier_mana_cost(
    tokens: &[OwnedLexToken],
) -> Option<(crate::mana::ManaCost, usize)> {
    let parsed = parse_leaf_fixed_mana_cost_prefix_tokens(tokens)?;
    Some((parsed.cost, parsed.consumed))
}

pub(crate) fn parse_cost_modifier_components(
    amount_tokens: &[OwnedLexToken],
) -> (
    Option<(Value, usize)>,
    Option<(crate::mana::ManaCost, usize)>,
) {
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let parsed_mana_cost = parse_cost_modifier_mana_cost(amount_tokens);

    let amount_used = parsed_amount.as_ref().map(|(_, used)| *used).unwrap_or(0);
    let mana_used = parsed_mana_cost
        .as_ref()
        .map(|(_, used)| *used)
        .unwrap_or(0);

    // Prefer mana-symbol parsing when it consumes a longer contiguous mana sequence
    // (e.g. "{2}{U}{U}" should stay a single mana-cost reduction component).
    if mana_used > amount_used {
        return (None, parsed_mana_cost);
    }

    (parsed_amount, None)
}

pub(crate) fn parse_cost_reduction_cap(tokens: &[OwnedLexToken]) -> Option<i32> {
    for idx in 2..tokens.len().saturating_sub(1) {
        if !tokens[idx - 2].is_word("by")
            || !tokens[idx - 1].is_word("more")
            || !tokens[idx].is_word("than")
        {
            continue;
        }
        let group = mana_pips_from_token(tokens.get(idx + 1)?)?;
        if group.len() != 1 {
            return None;
        }
        return match group[0] {
            ManaSymbol::Generic(amount) => Some(amount as i32),
            _ => None,
        };
    }
    None
}

pub(crate) fn parse_dynamic_cost_modifier_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    use keyword_static_lines::{
        CounterReferenceKind, DynamicCostValueShape, DynamicPlayerKind, DynamicThisWayMetric,
        SpellCastDynamicKind,
    };

    let for_each_value_tokens = static_keyword_cost_shapes::parse_dynamic_cost_each_word(tokens)
        .and_then(|boundary| tokens.get(boundary.token.saturating_add(1)..));
    let history_tokens = for_each_value_tokens.unwrap_or(tokens);
    let parsed_shape = keyword_static_lines::parse_dynamic_cost_value_shape_tokens(tokens);
    if parsed_shape.is_none()
        && let Some(value) = crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_count_value(history_tokens)
    {
        return Ok(Some(if for_each_value_tokens.is_some() {
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)
        } else {
            value
        }));
    }

    let Some(shape) = parsed_shape else {
        return Ok(None);
    };
    let with_for_each_surface = |value: Value| {
        if for_each_value_tokens.is_some() {
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)
        } else {
            value
        }
    };
    let player_filter = |player| match player {
        DynamicPlayerKind::You => PlayerFilter::You,
        DynamicPlayerKind::Opponent => PlayerFilter::Opponent,
        DynamicPlayerKind::Any => PlayerFilter::Any,
    };
    let value = match shape {
        DynamicCostValueShape::CardsDrawn(player) => {
            with_for_each_surface(Value::MaxCardsDrawnThisTurn(player_filter(player)))
        }
        DynamicCostValueShape::LifeGained(player) => {
            with_for_each_surface(Value::LifeGainedThisTurn(player_filter(player)))
        }
        DynamicCostValueShape::KickCount => Value::KickCount,
        DynamicCostValueShape::CreaturesDiedThisTurn => Value::CreaturesDiedThisTurn,
        DynamicCostValueShape::OpponentsLifeLostThisTurn => {
            Value::LifeLostThisTurn(PlayerFilter::Opponent)
        }
        DynamicCostValueShape::ControlledCreaturesDiedThisTurn => {
            Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You)
        }
        DynamicCostValueShape::SpellCast { player, kind } => {
            let player = player_filter(player);
            match kind {
                SpellCastDynamicKind::CardTypes => {
                    let mut filter = ObjectFilter::spell();
                    filter.cast_by = Some(player);
                    Value::CardTypesAmong(filter)
                }
                SpellCastDynamicKind::OtherThanFirst => Value::Add(
                    Box::new(Value::SpellsCastThisTurn(player)),
                    Box::new(Value::Fixed(-1)),
                ),
                SpellCastDynamicKind::MatchingTypes {
                    instant,
                    sorcery,
                    exclude_source,
                } => {
                    let mut filter = ObjectFilter::spell();
                    filter.card_types = match (instant, sorcery) {
                        (true, true) => vec![CardType::Instant, CardType::Sorcery],
                        (true, false) => vec![CardType::Instant],
                        (false, true) => vec![CardType::Sorcery],
                        (false, false) => Vec::new(),
                    };
                    Value::SpellsCastThisTurnMatching {
                        player,
                        filter,
                        exclude_source,
                    }
                }
                SpellCastDynamicKind::Simple => Value::SpellsCastThisTurn(player),
            }
        }
        DynamicCostValueShape::CardTypesInGraveyard(player) => {
            Value::CardTypesInGraveyard(player_filter(player))
        }
        DynamicCostValueShape::ColorsSpentToCastThisSpell => {
            Value::ColorsOfManaSpentToCastThisSpell
        }
        DynamicCostValueShape::PartySize => Value::PartySize(PlayerFilter::You),
        DynamicCostValueShape::AggregateScope => {
            let each_idx = static_keyword_cost_shapes::parse_dynamic_cost_each_word(tokens)
                .map(|boundary| boundary.token)
                .unwrap_or(0);
            let filter_tokens = tokens.get(each_idx + 1..).unwrap_or_default();
            let Some(value) = parse_aggregate_scope_value_lexed(filter_tokens) else {
                return Ok(None);
            };
            value
        }
        DynamicCostValueShape::CardTypesAmong { scope_tokens } => {
            let Ok(filter) = parse_object_filter(scope_tokens, false) else {
                return Ok(None);
            };
            Value::CardTypesAmong(filter)
        }
        DynamicCostValueShape::UnsupportedCardTypesAmong => {
            return Err(CardTextError::ParseError(format!(
                "unsupported card-types-among dynamic value (clause: '{}')",
                parser_token_word_refs(tokens).join(" ")
            )));
        }
        DynamicCostValueShape::CountersRemovedThisWay => {
            Value::X.with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay)
        }
        DynamicCostValueShape::PlayerCounters(counter_type) => {
            Value::PlayerCounters(PlayerFilter::You, counter_type)
        }
        DynamicCostValueShape::ThisWayMetric(metric) => match metric {
            DynamicThisWayMetric::Destroyed | DynamicThisWayMetric::Sacrificed => {
                Value::PendingEffectMetric {
                    source: EffectMetricSource::AffectedObjects,
                    metric: EffectMetric::Count,
                }
            }
            DynamicThisWayMetric::Discarded => Value::PendingEffectMetric {
                source: EffectMetricSource::Outcome,
                metric: EffectMetric::Count,
            },
            DynamicThisWayMetric::Exiled => Value::Count(
                ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile),
            ),
        },
        DynamicCostValueShape::RevealedPublic => {
            Value::Count(ObjectFilter::tagged(TagKey::from("__public_revealed")))
        }
        DynamicCostValueShape::RevealedOther => {
            let words = parser_token_word_refs(tokens);
            let Some((value, used_words)) = parse_for_each_count_value_words(&words) else {
                return Ok(None);
            };
            if used_words != words.len() {
                return Ok(None);
            }
            value
        }
        DynamicCostValueShape::CounterReference(reference) => {
            let counter_type = reference.counter_type;
            let value = match reference.reference_kind {
                CounterReferenceKind::Source => match counter_type {
                    Some(counter_type) => Value::CountersOnSource(counter_type),
                    None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
                },
                CounterReferenceKind::Tagged => Value::CountersOn(
                    Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                    counter_type,
                ),
                CounterReferenceKind::Other => {
                    let words = parser_token_word_refs(reference.reference_tokens);
                    let Some(surface) = source_reference_surface_for_words(&words) else {
                        return Ok(None);
                    };
                    Value::CountersOn(
                        Box::new(source_choose_spec_for_surface(surface)),
                        counter_type,
                    )
                }
            };
            with_for_each_surface(value)
        }
        DynamicCostValueShape::UnsupportedThisWay => {
            return Err(CardTextError::ParseError(format!(
                "unsupported this-way dynamic value (clause: '{}')",
                parser_token_word_refs(tokens).join(" ")
            )));
        }
        DynamicCostValueShape::Other { filter_tokens } => {
            if let Some(value) = crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_count_value(filter_tokens)
            {
                with_for_each_surface(value)
            } else if let Some(player) = parse_commander_cast_count_player(filter_tokens) {
                Value::CommanderCastCount(player)
            } else if let Ok(filter) = parse_object_filter(filter_tokens, false) {
                Value::Count(filter)
            } else {
                return Ok(None);
            }
        }
    };
    Ok(Some(value))
}

pub(crate) fn parse_add_mana_that_much_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    if keyword_static_lines::parse_that_much_value_marker_tokens(tokens) {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }
    None
}

pub(crate) fn parse_players_skip_upkeep_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let tokens = super::super::grammar::effects::split_labeled_effect_prefix_lexed(&tokens)
        .unwrap_or(&tokens);
    if let Some(fact) = type_and_color_facts::parse_skip_your_upkeep_tokens(tokens) {
        let mut ability = StaticAbility::player_skips_upkeep(crate::target::PlayerFilter::You);
        match fact.tail {
            type_and_color_facts::SkipYourUpkeepTail::None => {}
            type_and_color_facts::SkipYourUpkeepTail::Condition(condition_tokens) => {
                let condition = parse_static_condition_clause(condition_tokens)?;
                ability = ability.with_condition(condition);
            }
            type_and_color_facts::SkipYourUpkeepTail::Unsupported => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported skip-upkeep tail (clause: '{}')",
                    render_token_slice(tokens)
                )));
            }
        }
        return Ok(Some(ability));
    }
    if is_players_skip_upkeep_line_lexed(tokens) {
        return Ok(Some(StaticAbility::players_skip_upkeep()));
    }
    Ok(None)
}

pub(crate) fn parse_skip_your_draw_step_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_skip_your_draw_step_line_lexed(tokens) {
        return Ok(Some(StaticAbility::player_skips_draw_step(
            crate::target::PlayerFilter::You,
        )));
    }
    Ok(None)
}

pub(crate) fn parse_legend_rule_doesnt_apply_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_legend_rule_doesnt_apply_tokens(tokens) {
        return Ok(Some(StaticAbility::legend_rule_doesnt_apply()));
    }
    Ok(None)
}

pub(crate) fn parse_all_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_all_permanents_colorless_line_lexed(tokens) {
        return Ok(Some(StaticAbility::make_colorless(
            ObjectFilter::permanent(),
        )));
    }
    Ok(None)
}

pub(crate) fn parse_subject_are_card_types_in_addition_to_their_other_types_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_subject_type_addition_tokens(tokens) else {
        return Ok(None);
    };
    if fact.chosen_type {
        let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;
        if filter
            .card_types
            .iter()
            .any(|card_type| *card_type == CardType::Land)
        {
            return Ok(Some(vec![StaticAbility::add_chosen_basic_land_type(
                filter,
                render_token_slice(tokens),
            )]));
        }
        return Ok(Some(vec![StaticAbility::add_chosen_creature_type(
            filter,
            render_token_slice(tokens),
        )]));
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if matches!(descriptor, "a" | "an" | "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }

        let Some(subtype) = parse_subtype_flexible(descriptor) else {
            return Ok(None);
        };
        crate::slice_primitives::push_unique(&mut subtypes, subtype);
    }
    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    let mut abilities = Vec::new();
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

pub(crate) fn parse_all_cards_spells_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_all_cards_spells_permanents_colorless_tokens(tokens) {
        return Ok(Some(StaticAbility::make_colorless(ObjectFilter::default())));
    }
    Ok(None)
}

pub(crate) fn parse_all_cards_spells_permanents_add_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if type_and_color_facts::parse_all_cards_chosen_color_addition_tokens(tokens).is_some() {
        return Ok(Some(StaticAbility::add_chosen_color(
            ObjectFilter::default(),
            render_token_slice(tokens),
        )));
    }

    Ok(None)
}

pub(crate) fn parse_conjoined_subject_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let subject_tokens = trim_lexed_commas(tokens);
    let subject_segments = split_lexed_slices_on_and(subject_tokens);
    if subject_segments.len() <= 1 {
        return parse_object_filter_lexed(subject_tokens, false);
    }

    let mut branches = Vec::with_capacity(subject_segments.len());
    for segment in subject_segments {
        let segment = trim_lexed_commas(segment);
        if segment.is_empty() {
            return parse_object_filter_lexed(subject_tokens, false);
        }
        branches.push(parse_object_filter_lexed(segment, false)?);
    }
    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    Ok(filter)
}

pub(crate) fn parse_all_are_pt_color_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_power_toughness_type_addition_tokens(tokens)
    else {
        return Ok(None);
    };
    let mut colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if is_article(descriptor) || matches!(descriptor, "and" | "or" | "and/or") {
            continue;
        }
        if let Some(color) = parse_color(descriptor) {
            colors = colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in pt-color-type-addition clause (clause: '{}')",
            descriptor,
            render_token_slice(tokens)
        )));
    }

    if colors.is_empty() && card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_conjoined_subject_filter(fact.subject_tokens)?;

    let mut abilities = Vec::new();
    if !colors.is_empty() {
        abilities.push(StaticAbility::set_colors(filter.clone(), colors));
    }
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter.clone(), subtypes));
    }
    abilities.push(StaticAbility::set_base_power_toughness(
        filter,
        fact.power,
        fact.toughness,
    ));
    Ok(Some(abilities))
}

pub(crate) fn parse_all_are_color_and_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_color_type_addition_tokens(tokens) else {
        return Ok(None);
    };
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if matches!(descriptor, "a" | "an" | "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in are-color-and-type-addition clause (clause: '{}')",
            descriptor,
            render_token_slice(tokens)
        )));
    }

    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    let mut abilities = vec![StaticAbility::set_colors(filter.clone(), fact.color)];
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

pub(crate) fn parse_all_creatures_are_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_subject_color_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    Ok(Some(StaticAbility::set_colors(filter, fact.color)))
}

pub(crate) fn parse_subjects_are_basic_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_subjects_are_basic_tokens(tokens) else {
        return Ok(None);
    };

    let subject_segments = split_lexed_slices_on_and(fact.subject_tokens);
    let filter = if subject_segments.len() > 1 {
        let mut branches = Vec::with_capacity(subject_segments.len());
        for segment in subject_segments {
            let segment = trim_lexed_commas(segment);
            if segment.is_empty() {
                return Ok(None);
            }
            branches.push(parse_object_filter_lexed(segment, false)?);
        }
        let mut filter = ObjectFilter::default();
        filter.any_of = branches;
        filter
    } else {
        parse_object_filter_lexed(fact.subject_tokens, false)?
    };

    Ok(Some(StaticAbility::add_supertypes(
        filter,
        vec![Supertype::Basic],
    )))
}

pub(crate) fn parse_nonbasic_lands_are_basic_land_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_basic_land_subtype_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    Ok(Some(StaticAbility::set_land_subtypes(
        filter,
        vec![fact.subtype],
    )))
}

pub(crate) fn parse_remove_snow_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_remove_snow_line_lexed(tokens) {
        return Ok(Some(StaticAbility::remove_supertypes(
            ObjectFilter::land(),
            vec![Supertype::Snow],
        )));
    }
    Ok(None)
}

pub(crate) fn parse_land_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_land_type_addition_tokens(tokens) else {
        return Ok(None);
    };
    match fact {
        type_and_color_facts::LandTypeAdditionFact::EveryBasic { subject_tokens } => {
            let filter = parse_object_filter_lexed(subject_tokens, false)?;
            Ok(Some(StaticAbility::add_subtypes(
                filter,
                vec![
                    Subtype::Plains,
                    Subtype::Island,
                    Subtype::Swamp,
                    Subtype::Mountain,
                    Subtype::Forest,
                ],
            )))
        }
        type_and_color_facts::LandTypeAdditionFact::One {
            subject_tokens,
            subtype,
        } => Ok(Some(StaticAbility::add_subtypes(
            parse_object_filter_lexed(subject_tokens, false)?,
            vec![subtype],
        ))),
    }
}

pub(crate) fn parse_lands_are_pt_creatures_still_lands_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_land_animation_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    Ok(Some(vec![
        StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature]),
        StaticAbility::set_base_power_toughness(filter, fact.power, fact.toughness),
    ]))
}

pub(crate) fn parse_static_base_power_toughness_value_tail(
    tail_tokens: &[OwnedLexToken],
) -> Option<(Value, Value)> {
    if !keyword_static_lines::parse_iterated_mana_value_base_pt_tail_tokens(tail_tokens) {
        return None;
    }
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    Some((value.clone(), value))
}

pub(crate) fn parse_filter_is_pt_creature_in_addition_and_has_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = LexedClause::new(tokens).word_refs();
    let Some(animation_verbs) = static_keyword_line_shapes::parse_animation_verbs(tokens) else {
        return Ok(None);
    };
    let be_idx = animation_verbs.be.token;
    let has_idx = animation_verbs.has.token;

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..be_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let attached_subject = LexedClause::new(&subject_tokens)
        .words()
        .first()
        .is_some_and(|word| matches!(word, "enchanted" | "equipped"));

    let before_has = trim_commas(&tokens[be_idx + 1..has_idx]);
    if before_has.is_empty() {
        return Ok(None);
    }
    let before_has_clause = LexedClause::new(&before_has);
    let raw_before_has_words = before_has_clause.word_refs();
    let before_has_words = strip_leading_article_word_refs(&raw_before_has_words);
    let skipped_article_words = raw_before_has_words
        .len()
        .saturating_sub(before_has_words.len());
    let Some(creature_idx) =
        static_keyword_line_shapes::parse_animation_creature_word(&before_has_words)
            .map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    let (base_power_toughness, subtype_start_word, granted_tail) = match before_has_words
        .first()
        .and_then(|word| parse_pt_modifier(word).ok())
    {
        Some((power, toughness)) => {
            if creature_idx == 0 {
                return Ok(None);
            }
            let Some(granted_tail) = parse_heterogeneous_granted_tail(
                &tokens[has_idx + 1..],
                &clause_words,
                attached_subject,
            )?
            else {
                return Ok(None);
            };
            (
                (Value::Fixed(power), Value::Fixed(toughness)),
                1usize,
                granted_tail,
            )
        }
        None => {
            let Some((power, toughness)) =
                parse_static_base_power_toughness_value_tail(&tokens[has_idx + 1..])
            else {
                return Ok(None);
            };
            ((power, toughness), 0usize, ParsedGrantedTailAst::default())
        }
    };
    let subtype_words = &before_has_words[subtype_start_word..creature_idx];
    let mut subtypes = Vec::new();
    for word in subtype_words {
        if is_article(word) {
            continue;
        }
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }
    let tail_start_word = skipped_article_words + creature_idx + 1;
    let mut tail_end_word = skipped_article_words + before_has_words.len();
    let tail_ends_with_and = before_has_words[creature_idx + 1..]
        .last()
        .is_some_and(|word| *word == "and");
    if tail_ends_with_and {
        tail_end_word = tail_end_word.saturating_sub(1);
    }
    if type_and_color_facts::parse_other_type_addition_tail_tokens(
        before_has_clause
            .between_word_range(tail_start_word, tail_end_word)
            .map(|tail_clause| tail_clause.tokens())
            .unwrap_or_default(),
    )
    .is_none()
    {
        return Ok(None);
    }

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::Add,
            base_power_toughness: Some(base_power_toughness),
            granted_tail,
        },
    )))
}

pub(crate) fn parse_subject_is_subtype_with_base_pt_and_granted_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens =
        if let Some((label_tokens, body_tokens)) = split_em_dash_label_prefix_tokens(tokens) {
            if document_grammar::parse_preserved_keyword_label_tokens(label_tokens).is_some() {
                tokens
            } else {
                body_tokens
            }
        } else {
            tokens
        };
    let Some(grant_verbs) = static_keyword_line_shapes::parse_subtype_grant_verbs(tokens) else {
        return Ok(None);
    };
    let be_idx = grant_verbs.be.token;
    let with_idx = grant_verbs.with.token;

    let (_condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..be_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let attached_subject = LexedClause::new(&subject_tokens)
        .words()
        .first()
        .is_some_and(|word| matches!(word, "enchanted" | "equipped"));

    let type_tokens = trim_commas(&tokens[be_idx + 1..with_idx]);
    if type_tokens.is_empty() {
        return Ok(None);
    }
    let type_words = LexedClause::new(&type_tokens).word_refs();
    let type_words = strip_leading_article_word_refs(&type_words);
    if type_words.is_empty() {
        return Ok(None);
    }
    let mut subtypes = Vec::new();
    for word in type_words {
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }

    let mut after_with = trim_commas(&tokens[with_idx + 1..]).to_vec();
    if after_with.is_empty() {
        return Ok(None);
    }

    if let Some(loses) = type_and_color_facts::find_loses_other_creature_types_tokens(&after_with) {
        after_with.truncate(loses.marker_token);
    }

    let after_with = trim_edge_punctuation_tokens(&after_with);
    let Some(base) = type_and_color_facts::parse_base_power_toughness_grant_tokens(after_with)
    else {
        return Ok(None);
    };
    let after_with_words = parser_token_word_refs(after_with);
    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(base.ability_tokens, &after_with_words, attached_subject)?
    else {
        return Ok(None);
    };

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition: _condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::ReplaceCreatureTypes,
            base_power_toughness: Some((Value::Fixed(base.power), Value::Fixed(base.toughness))),
            granted_tail,
        },
    )))
}

pub(crate) fn parse_creatures_cant_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if is_creatures_cant_block_line_lexed(tokens) {
        return Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter: ObjectFilter::creature(),
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::cant_block())),
            condition: None,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_prevent_all_damage_dealt_to_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_damage_dealt_to_creatures_line_lexed(tokens) {
        return Ok(Some(StaticAbility::prevent_all_damage_dealt_to_creatures()));
    }
    Ok(None)
}

pub(crate) fn parse_prevent_damage_to_other_creature_you_control_put_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::prevent_damage_to_other_creature_you_control_put_counters_instead(
            crate::object::CounterType::PlusOnePlusOne,
            display_text_for_tokens(tokens, true),
        ),
    ))
}

pub(crate) fn parse_damage_source_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut words = strip_leading_article_word_refs(words).to_vec();
    if word_slice_last_is_any(&words, &["source", "sources"]) {
        words.pop();
    }
    if words.is_empty() {
        return Some(ObjectFilter::default());
    }

    let mut filter = ObjectFilter::default();
    let mut colors: Option<ColorSet> = None;
    for word in words {
        if matches!(word, "and" | "or") {
            continue;
        }
        if let Some(color) = parse_color(word) {
            colors = Some(colors.unwrap_or_else(ColorSet::new).union(color));
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            filter.card_types.push(card_type);
            continue;
        }
        return None;
    }
    if let Some(colors) = colors {
        filter.colors = Some(colors);
    }
    Some(filter)
}

pub(crate) fn parse_damage_source_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let words = LexedClause::new(tokens).word_refs();
    parse_damage_source_filter_words(&words)
}

pub(crate) fn parse_prevent_damage_to_you_from_source_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_prevent_damage_to_you_tokens(tokens) else {
        return Ok(None);
    };
    let Some(source_filter) = parse_damage_source_filter_tokens(spec.source_tokens) else {
        return Ok(None);
    };
    let display = render_token_slice(tokens);

    Ok(Some(
        StaticAbility::prevent_damage_to_you_from_source_filter(
            spec.amount,
            source_filter,
            display,
        ),
    ))
}

pub(crate) fn parse_replace_damage_with_counters_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !keyword_static_lines::parse_noncombat_damage_minus_counter_replacement_tokens(tokens) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::replace_damage_with_counters_instead(
        CounterType::MinusOneMinusOne,
        ObjectFilter::default().controlled_by(PlayerFilter::You),
        ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        Some(false),
        display_text_for_tokens(tokens, true),
    )))
}

pub(crate) fn parse_double_counters_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_counter_replacement_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(match shape {
        keyword_static_lines::CounterReplacementShape::GenericUnderYourControl => {
            StaticAbility::double_counters_replacement(
                ObjectFilter::permanent().controlled_by(PlayerFilter::You),
                None,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::EnergyYouGet => {
            StaticAbility::double_player_counters_replacement(
                PlayerFilter::You,
                Some(CounterType::Energy),
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::PlusOneAdd {
            filter_tokens,
            additional,
        } => StaticAbility::add_counters_placement_replacement(
            parse_object_filter_lexed(filter_tokens, false)?,
            Some(CounterType::PlusOnePlusOne),
            additional,
            display_text_for_tokens(tokens, true),
        ),
        keyword_static_lines::CounterReplacementShape::PlusOneDouble { filter_tokens } => {
            StaticAbility::double_counters_replacement(
                parse_object_filter_lexed(filter_tokens, false)?,
                Some(CounterType::PlusOnePlusOne),
                display_text_for_tokens(tokens, true),
            )
        }
    }))
}

pub(crate) fn parse_double_token_creation_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_token_creation_replacement_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(match shape {
        keyword_static_lines::TokenCreationReplacementShape::GenericUnderYourControl => {
            StaticAbility::double_token_creation_replacement(
                PlayerFilter::You,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::TokenCreationReplacementShape::AddTreasure { descriptor_tokens } => {
            let mut token_filter = ObjectFilter::default().token();
            for word in parser_token_word_refs(descriptor_tokens) {
                if let Some(card_type) = parse_card_type(word) {
                    token_filter = token_filter.with_type(card_type);
                } else if let Some(subtype) = parse_subtype_flexible(word) {
                    token_filter = token_filter.with_subtype(subtype);
                } else {
                    return Ok(None);
                }
            }
            StaticAbility::add_token_creation_replacement(
                PlayerFilter::You,
                token_filter,
                ironsmith_core::AdditionalTokenKind::Treasure,
                1,
                display_text_for_tokens(tokens, true),
            )
        }
    }))
}

pub(crate) fn parse_prevent_all_combat_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_combat_damage_to_source_line_lexed(tokens) {
        return Ok(Some(StaticAbility::prevent_all_combat_damage_to_self()));
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_combat_damage_to_matching_permanents_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_all_combat_damage_to_matching_permanents_line_lexed(tokens) {
        return Ok(None);
    }
    let Some(prevention) =
        static_keyword_replacement_shapes::parse_combat_prevention_prefix(tokens)
    else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&tokens[prevention.end..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "prevent-all combat damage static line missing target filter (clause: '{}')",
            render_token_slice(tokens)
        )));
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    Ok(Some(
        StaticAbility::prevent_all_combat_damage_to_permanents_matching(filter),
    ))
}

pub(crate) fn parse_during_your_turn_prevent_all_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_during_your_turn_prevent_all_damage_to_source_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self().with_condition(
                crate::ConditionExpr::ActivationTiming(
                    crate::ability::ActivationTiming::DuringYourTurn,
                ),
            ),
        ));
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_noncombat_damage_to_other_creatures_you_control_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_noncombat_damage_to_other_creatures_you_control(),
        ));
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_noncombat_damage_to_matching_permanents_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_all_noncombat_damage_to_matching_permanents_line_lexed(tokens) {
        return Ok(None);
    }

    let Some(prevention) =
        static_keyword_replacement_shapes::parse_noncombat_prevention_prefix(tokens)
    else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&tokens[prevention.end..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all noncombat damage target filter: {}",
            render_token_slice(tokens)
        )));
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    Ok(Some(
        StaticAbility::prevent_all_noncombat_damage_to_permanents_matching(filter),
    ))
}

pub(crate) fn parse_prevent_all_damage_to_source_by_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_damage_to_source_by_creatures_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self_by_creatures(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_may_choose_not_to_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = late_static_facts::parse_may_choose_not_untap_tokens(tokens) else {
        return Ok(None);
    };
    let subject_words = parser_token_word_refs(fact.subject_tokens);
    if !fact.simple_source_subject && !is_source_reference_words(&subject_words) {
        return Ok(None);
    }

    let subject = render_token_slice(fact.subject_tokens);
    let subject = source_reference_surface_for_words(&subject_words)
        .map(|surface| surface.display_text())
        .unwrap_or(subject);
    Ok(Some(
        StaticAbility::may_choose_not_to_untap_during_untap_step(subject),
    ))
}

pub(crate) fn parse_untap_during_each_other_players_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = split_untap_each_other_players_untap_step_line_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.subject_tokens);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing subject in other-players untap ability (clause: '{}')",
            render_token_slice(tokens)
        )));
    }

    let filter = parse_object_filter(&subject_tokens, false)?;
    let subject_text = render_token_slice(&subject_tokens);
    Ok(Some(
        StaticAbility::untap_during_each_other_players_untap_step(
            filter,
            format!("Untap all {subject_text} during each other player's untap step"),
        ),
    ))
}

pub(crate) fn parse_doesnt_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    match parse_doesnt_untap_during_untap_step_spec_lexed(tokens) {
        Some(DoesntUntapDuringUntapStepSpec::Source { tail_tokens }) => {
            let clause_display = render_token_slice(tokens);
            let tail_tokens = trim_commas(tail_tokens);
            if tail_tokens.is_empty() {
                return Ok(Some(
                    StaticAbilityAst::Static(StaticAbility::doesnt_untap()),
                ));
            }
            if tail_tokens.first().is_some_and(|token| token.is_word("if")) {
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after untap-step if-clause (clause: '{}')",
                        clause_display
                    )));
                }
                let condition = parse_static_condition_clause(&condition_tokens)?;
                return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                    condition,
                }));
            }

            Err(CardTextError::ParseError(format!(
                "unsupported trailing untap-step clause (clause: '{}')",
                clause_display
            )))
        }
        Some(DoesntUntapDuringUntapStepSpec::Attached {
            subject_tokens,
            tail_tokens,
        }) => {
            let subject = render_token_slice(subject_tokens);
            let text = format!("{subject} doesnt untap during its controllers untap step");
            let condition = if tail_tokens.is_empty() {
                None
            } else {
                let clause_display = render_token_slice(tokens);
                if !tail_tokens
                    .first()
                    .is_some_and(|token| token.as_word() == Some("unless"))
                {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing attached untap-step clause (clause: '{}')",
                        clause_display
                    )));
                }
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after attached untap-step unless-clause (clause: '{}')",
                        clause_display
                    )));
                }
                Some(crate::ConditionExpr::Not(Box::new(
                    parse_static_condition_clause(&condition_tokens)?,
                )))
            };
            Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                display: text,
                condition,
            }))
        }
        None => Ok(None),
    }
}

pub(crate) fn parse_flying_restriction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(match parse_flying_block_restriction_line_lexed(tokens) {
        Some(FlyingBlockRestrictionKind::FlyingOnly) => {
            Some(StaticAbility::flying_only_restriction())
        }
        Some(FlyingBlockRestrictionKind::FlyingOrReach) => {
            Some(StaticAbility::flying_restriction())
        }
        None => None,
    })
}

pub(crate) fn parse_can_block_only_flying_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_can_block_only_flying_line_lexed(tokens) {
        return Ok(Some(StaticAbility::can_block_only_flying()));
    }

    Ok(None)
}

pub(crate) fn parse_can_block_subtype_as_though_reach_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(parse_can_block_subtype_as_though_reach_line_lexed(tokens)
        .map(StaticAbility::can_block_subtype_as_though_reach))
}

pub(crate) fn parse_assign_damage_as_unblocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_may_assign_damage_as_unblocked_line_lexed(tokens) {
        return Ok(Some(StaticAbility::may_assign_damage_as_unblocked()));
    }

    Ok(None)
}

pub(crate) fn parse_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_mana_value_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    Ok(Some(StaticAbility::grants(crate::grant::GrantSpec::new(
        crate::grant::Grantable::mana_value_as_generic_from_hand(),
        filter,
        Zone::Hand,
    ))))
}

pub(crate) fn parse_life_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_life_mana_value_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    let usage_limit = match spec.usage_limit {
        keyword_static_lines::LifeManaValueGrantUsageLimit::OnceDuringEachOfYourTurns => {
            crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns
        }
    };
    Ok(Some(StaticAbility::grants(crate::grant::GrantSpec::new(
        crate::grant::Grantable::life_equal_mana_value_from_hand(Some(usage_limit)),
        filter,
        Zone::Hand,
    ))))
}

pub(crate) fn parse_fixed_mana_cost_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_fixed_mana_cost_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    Ok(Some(StaticAbility::grants(
        crate::grant::GrantSpec::cast_from_hand_for_alternative_mana_cost_matching(
            filter,
            spec.mana_cost,
        ),
    )))
}

pub(crate) fn parse_grant_flash_to_noncreature_spells_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if spec == crate::grant::GrantSpec::flash_to_noncreature_spells() => {
            Ok(Some(StaticAbility::grants(spec)))
        }
        _ => Ok(None),
    }
}

pub(crate) fn static_grant_beneficiary(
    player: crate::cards::builders::PlayerAst,
) -> Option<PlayerFilter> {
    match player {
        crate::cards::builders::PlayerAst::You | crate::cards::builders::PlayerAst::Implicit => {
            Some(PlayerFilter::You)
        }
        crate::cards::builders::PlayerAst::Any => Some(PlayerFilter::Any),
        _ => None,
    }
}

pub(crate) fn parse_you_may_cast_exile_counter_cards_with_mana_permission_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_exile_counter_permission_tokens(tokens) else {
        return Ok(None);
    };
    let is_play_lands_and_cast_noncreature_family = matches!(
        spec.family,
        keyword_static_lines::ExileCounterPermissionFamily::PlayLandsAndCastNoncreatureCardsExiledBySource
    );
    let owner = match spec.owner {
        keyword_static_lines::ExileCounterPermissionOwner::Any => None,
        keyword_static_lines::ExileCounterPermissionOwner::Opponent => Some(PlayerFilter::Opponent),
    };

    let uses_snow_sources = matches!(
        spec.mana_permission,
        keyword_static_lines::ExileCounterManaPermission::SnowSources
    );

    let mut base_filter = ObjectFilter {
        zone: Some(Zone::Exile),
        owner,
        with_counter: Some(crate::filter::CounterConstraint::Typed(spec.counter_type)),
        ..ObjectFilter::default()
    };
    if is_play_lands_and_cast_noncreature_family {
        base_filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
    }

    let mut filter = if is_play_lands_and_cast_noncreature_family {
        ObjectFilter {
            any_of: vec![
                ObjectFilter {
                    card_types: vec![CardType::Land],
                    ..base_filter.clone()
                },
                ObjectFilter {
                    excluded_card_types: vec![CardType::Creature, CardType::Land],
                    ..base_filter.clone()
                },
            ],
            ..ObjectFilter::default()
        }
    } else {
        ObjectFilter {
            excluded_card_types: vec![CardType::Land],
            ..base_filter
        }
    };
    filter.has_mana_cost = false;

    let grant = StaticAbility::grants(
        crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            filter.clone(),
            Zone::Exile,
        )
        .with_beneficiary(PlayerFilter::You),
    );
    let permission = if uses_snow_sources {
        crate::effect::ManaSpendPermission::any_color_from_sources_for_casting_matching(
            PlayerFilter::You,
            filter,
            ObjectFilter::default().with_supertype(Supertype::Snow),
        )
    } else {
        crate::effect::ManaSpendPermission::any_color_for_casting_matching(
            PlayerFilter::You,
            filter,
        )
    };
    let mana_permission =
        StaticAbility::mana_spend_permission(permission, render_token_slice(tokens));

    Ok(Some(vec![grant, mana_permission]))
}

pub(crate) fn parse_surveilled_graveyard_play_life_cost_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if !late_static_facts::is_surveilled_graveyard_play_life_cost(tokens) {
        return Ok(None);
    }

    let base_filter = ObjectFilter {
        zone: Some(Zone::Graveyard),
        owner: Some(PlayerFilter::You),
        surveilled_this_turn: true,
        ..ObjectFilter::default()
    };
    let mut spell_filter = base_filter.clone();
    spell_filter.excluded_card_types.push(CardType::Land);

    Ok(Some(vec![
        StaticAbility::grants(
            crate::grant::GrantSpec::new(
                crate::grant::Grantable::play_from(),
                base_filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        ),
        StaticAbility::grants(
            crate::grant::GrantSpec::new(
                crate::grant::Grantable::life_equal_mana_value_from_zone(Zone::Graveyard, None),
                spell_filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        ),
    ]))
}

pub(crate) fn parse_you_may_static_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if late_static_facts::is_source_linked_exile_cast_with_any_mana(tokens) {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.owner = Some(PlayerFilter::NotYou);
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        let grant = StaticAbility::grants(
            crate::grant::GrantSpec::new(
                crate::grant::Grantable::play_from(),
                filter.clone(),
                Zone::Exile,
            )
            .with_beneficiary(PlayerFilter::Any),
        );
        let mana_permission = StaticAbility::mana_spend_permission(
            crate::effect::ManaSpendPermission::any_color_for_casting_matching(
                PlayerFilter::Any,
                filter,
            ),
            "Mana of any type can be spent to cast it",
        );
        return Ok(Some(vec![grant, mana_permission]));
    }

    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) => {
            let singular_spell = late_static_facts::contains_singular_cast_spell(tokens);
            if singular_spell
                && spec.zone == Zone::Hand
                && matches!(
                    &spec.grantable,
                    crate::grant::Grantable::AlternativeCast(method)
                        if method.cast_from_zone() == Zone::Hand
                            && method.mana_cost().is_none()
                            && method.non_mana_costs().is_empty()
                )
            {
                return Ok(None);
            }
            Ok(static_grant_beneficiary(player)
                .map(|beneficiary| vec![StaticAbility::grants(spec.with_beneficiary(beneficiary))]))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_as_you_cascade_land_drop_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_cascade_land_drop_tokens(tokens) {
        return Ok(Some(StaticAbility::cascade_land_drop()));
    }
    Ok(None)
}

pub(crate) fn parse_play_from_permission_with_haste_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(permission_sentence) =
        late_static_facts::parse_play_permission_with_haste_followup(tokens)
    else {
        return Ok(None);
    };

    match parse_permission_clause_spec(permission_sentence)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(spec.grantable, crate::grant::Grantable::PlayFrom)
            && spec.filter.card_types.len() == 1
            && spec
                .filter
                .card_types
                .iter()
                .any(|card_type| *card_type == CardType::Creature) =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(
                    spec.with_beneficiary(beneficiary)
                        .with_cast_this_way_grant(StaticAbility::haste()),
                )
            }))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_play_from_permission_with_enter_counter_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(parsed) = keyword_static_lines::parse_play_permission_enter_counter_tokens(tokens)
    else {
        return Ok(None);
    };

    match parse_permission_clause_spec(parsed.permission_tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(spec.grantable, crate::grant::Grantable::PlayFrom) => {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(spec.with_beneficiary(beneficiary).with_cast_this_way_grant(
                    StaticAbility::enters_with_counters_value(parsed.counter_type, Value::Fixed(1)),
                ))
            }))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_play_from_permission_with_enter_tapped_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(permission_sentence) =
        late_static_facts::parse_play_permission_with_enter_tapped_followup(tokens)
    else {
        return Ok(None);
    };

    match parse_permission_clause_spec(permission_sentence)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(
            spec.grantable,
            crate::grant::Grantable::PlayFrom
                | crate::grant::Grantable::AlternativeCast(_)
                | crate::grant::Grantable::DerivedAlternativeCast(_)
        ) =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(
                    spec.with_beneficiary(beneficiary)
                        .with_cast_this_way_grant(StaticAbility::enters_tapped_ability()),
                )
            }))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_you_may_look_top_card_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_may_look_top_card_any_time_line_lexed(tokens) {
        return Ok(Some(StaticAbility::look_at_top_card_of_library()));
    }
    Ok(None)
}

pub(crate) fn parse_you_may_look_face_down_creatures_you_dont_control_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_may_look_face_down_creatures_you_dont_control_any_time_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::look_at_face_down_creatures_you_dont_control(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_players_play_top_card_libraries_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_players_play_top_card_libraries_revealed_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::all_players_look_at_top_cards_of_libraries(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_play_top_card_your_library_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_play_top_card_your_library_revealed_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::all_players_look_at_your_top_library_card(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_your_opponents_play_with_hands_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_your_opponents_play_with_hands_revealed_line_lexed(tokens) {
        return Ok(Some(StaticAbility::opponents_play_with_hands_revealed()));
    }
    Ok(None)
}

pub(crate) fn parse_control_opponents_while_searching_libraries_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::is_control_opponents_while_searching(tokens) {
        return Ok(Some(
            StaticAbility::control_opponents_while_searching_libraries(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_opponent_search_exile_found_cards_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::is_opponent_search_exile_found_cards(tokens) {
        return Ok(Some(StaticAbility::opponent_search_exile_found_cards()));
    }
    Ok(None)
}

pub(crate) fn parse_cast_this_card_from_library_while_searching_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::is_cast_this_card_from_library_while_searching(tokens) {
        return Ok(Some(
            StaticAbility::cast_this_card_from_library_while_searching(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_cast_this_spell_as_though_it_had_flash_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_cast_this_spell_as_though_it_had_flash_line_lexed(tokens) {
        return Ok(Some(StaticAbility::flash()));
    }
    Ok(None)
}

pub(crate) fn parse_attacks_each_combat_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(fact) = late_static_facts::parse_attack_each_combat_if_able_tokens(tokens) else {
        return Ok(None);
    };
    if matches!(
        fact,
        late_static_facts::AttackEachCombatFact::AttachedController
    ) {
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::all_creatures_attack_attached_controller_each_combat_if_able(),
        )));
    }
    let late_static_facts::AttackEachCombatFact::Subject(subject_tokens) = fact else {
        unreachable!("attached-controller fact returned above")
    };
    let subject_tokens = trim_commas(subject_tokens);
    if subject_tokens.is_empty() {
        return Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())));
    }
    let subject = parse_anthem_subject(&subject_tokens)?;
    match subject {
        AnthemSubjectAst::Source => {
            Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())))
        }
        AnthemSubjectAst::Filter(filter) => Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
            condition: None,
        })),
    }
}

pub(crate) fn parse_additional_land_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(count) = late_static_facts::parse_additional_land_play_count(tokens) else {
        return Ok(None);
    };

    Ok(Some(vec![StaticAbility::additional_land_plays(count)]))
}

pub(crate) fn parse_play_lands_from_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_play_lands_from_graveyard_line_lexed(tokens) {
        let spec = crate::grant::GrantSpec::play_lands_from_graveyard();
        return Ok(Some(StaticAbility::grants(spec)));
    }
    Ok(None)
}

pub(crate) fn parse_graveyard_cards_have_retrace_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = late_static_facts::parse_retrace_grant_tokens(tokens) else {
        return Ok(None);
    };
    let mut filter = ObjectFilter {
        card_types: fact.card_types,
        owner: Some(PlayerFilter::You),
        ..ObjectFilter::default()
    };
    filter.zone = Some(Zone::Graveyard);
    let spec = crate::grant::GrantSpec::new(
        crate::grant::Grantable::retrace_from_cards_mana_cost(),
        filter,
        Zone::Graveyard,
    );
    Ok(Some(StaticAbility::grants(spec)))
}

pub(crate) fn parse_cast_spells_from_hand_without_paying_mana_costs_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::contains_singular_cast_spell(tokens) {
        return Ok(None);
    }
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if spec.zone == Zone::Hand
            && matches!(
                &spec.grantable,
                crate::grant::Grantable::AlternativeCast(method)
                    if method.cast_from_zone() == Zone::Hand
                        && method.mana_cost().is_none()
                        && method.non_mana_costs().is_empty()
            ) =>
        {
            Ok(Some(StaticAbility::grants(spec)))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_pt_modifier(raw: &str) -> Result<(i32, i32), CardTextError> {
    let (power_raw, toughness_raw) = split_pt_modifier_components(raw)?;
    let power_str = strip_leading_plus_char(power_raw);
    let toughness_str = strip_leading_plus_char(toughness_raw);
    let power = power_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power modifier".to_string()))?;
    let toughness = toughness_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid toughness modifier".to_string()))?;
    Ok((power, toughness))
}

pub(crate) fn parse_signed_pt_component(raw: &str) -> Result<Value, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CardTextError::ParseError(
            "missing power/toughness component".to_string(),
        ));
    }

    let (sign, value_text) = split_signed_pt_component(trimmed);

    if pt_component_is_x(value_text) {
        return Ok(match sign {
            1 => Value::X,
            -1 => Value::XTimes(-1),
            _ => Value::XTimes(sign),
        });
    }

    let parsed = value_text
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power/toughness component".to_string()))?;
    Ok(Value::Fixed(parsed * sign))
}

pub(crate) fn parse_pt_modifier_values(raw: &str) -> Result<(Value, Value), CardTextError> {
    let (power_raw, toughness_raw) = split_pt_modifier_components(raw)?;
    let power = parse_signed_pt_component(power_raw)?;
    let toughness = parse_signed_pt_component(toughness_raw)?;
    Ok((power, toughness))
}

pub(crate) fn split_pt_modifier_components(raw: &str) -> Result<(&str, &str), CardTextError> {
    static_keyword_shapes::parse_pt_components(raw)
        .map(|components| (components.power, components.toughness))
        .ok_or_else(|| CardTextError::ParseError("missing power/toughness modifier".to_string()))
}

pub(crate) fn strip_leading_plus_char(raw: &str) -> &str {
    let trimmed = raw.trim();
    let mut chars = trimmed.chars();
    if chars.next().is_some_and(|ch| ch == '+') {
        chars.as_str()
    } else {
        trimmed
    }
}

pub(crate) fn split_signed_pt_component(trimmed: &str) -> (i32, &str) {
    let mut chars = trimmed.chars();
    match chars.next() {
        Some('+') => (1, chars.as_str()),
        Some('-' | '−') => (-1, chars.as_str()),
        _ => (1, trimmed),
    }
}

pub(crate) fn pt_component_is_x(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|ch| matches!(ch, 'x' | 'X')) && chars.next().is_none()
}

pub(crate) fn parse_no_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_no_maximum_hand_size_line_lexed(tokens) {
        return Ok(Some(StaticAbility::no_maximum_hand_size()));
    }
    Ok(None)
}

pub(crate) fn parse_can_be_your_commander_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_can_be_your_commander_line_lexed(tokens) {
        return Ok(Some(StaticAbility::can_be_commander()));
    }
    Ok(None)
}

pub(crate) fn parse_reduced_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_hand_size_line_tokens(tokens) else {
        return Ok(None);
    };
    let player = match spec.player {
        keyword_static_lines::HandSizePlayerKind::You => PlayerFilter::You,
        keyword_static_lines::HandSizePlayerKind::Opponent => PlayerFilter::Opponent,
        keyword_static_lines::HandSizePlayerKind::Any => PlayerFilter::Any,
    };
    let min_card_types_condition = if let Some(condition_tokens) = spec.condition_tokens {
        let Some((metric, threshold)) =
            parse_graveyard_metric_threshold_condition(condition_tokens)?
        else {
            return Ok(None);
        };
        if metric != crate::static_abilities::GraveyardCountMetric::CardTypes {
            return Ok(None);
        }
        threshold
    } else {
        0
    };
    Ok(Some(match spec.operation {
        keyword_static_lines::HandSizeOperation::Reduce(amount) => {
            StaticAbility::reduce_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::Increase(amount) => {
            StaticAbility::increase_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::Set(amount) => {
            StaticAbility::set_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::SevenMinusGraveyardCardTypes => {
            StaticAbility::max_hand_size_seven_minus_your_graveyard_card_types(
                player,
                min_card_types_condition,
            )
        }
    }))
}

pub(crate) fn parse_effect_discard_to_library_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_effect_discard_to_library_replacement_line_lexed(tokens) {
        return Ok(Some(StaticAbility::effect_discard_to_library_replacement()));
    }

    if is_opponent_effect_discard_this_to_battlefield_replacement_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::opponent_effect_discard_this_to_battlefield_replacement(),
        ));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replace_exile_top_face_down_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replace_exile_top_face_down_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_exile_top_face_down()));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replacement_exile_top_and_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(count) = late_static_facts::parse_draw_replacement_exile_top_and_play_count(tokens)
    else {
        return Ok(None);
    };

    Ok(Some(StaticAbility::draw_replacement_exile_top_and_play(
        count,
    )))
}

pub(crate) fn parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) =
        static_keyword_replacement_shapes::parse_draw_reveal_matching_rest_bottom(tokens)
    else {
        return Ok(None);
    };
    let Some(card_type) = parse_card_type(spec.card_type_word) else {
        return Ok(None);
    };
    let order = match spec.order {
        static_keyword_replacement_shapes::LibraryBottomOrderShape::Chosen => {
            ironsmith_core::LibraryBottomOrder::ChooserChooses
        }
        static_keyword_replacement_shapes::LibraryBottomOrderShape::Random => {
            ironsmith_core::LibraryBottomOrder::Random
        }
    };

    let mut filter = ObjectFilter::default();
    filter.card_types.push(card_type);

    Ok(Some(
        StaticAbility::draw_replacement_reveal_top_matching_to_hand_rest_bottom(
            spec.count,
            filter,
            order,
            render_token_slice(tokens),
        ),
    ))
}

pub(crate) fn parse_draw_replacement_double_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replacement_double_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_double()));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replacement_skip_empty_library_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replacement_skip_empty_library_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_skip_empty_library()));
    }

    Ok(None)
}

pub(crate) fn parse_conditional_draw_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = late_static_facts::parse_conditional_draw_replacement_tokens(tokens) else {
        return Ok(None);
    };
    let Some(no_cards_condition) =
        crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_condition(
            fact.condition_tokens,
        )
    else {
        return Ok(None);
    };
    if no_cards_condition.player != PlayerFilter::You || !no_cards_condition.is_no_cards_in_hand() {
        return Ok(None);
    }

    let draw_count = fact.draw_count;
    let mut replacement_effects = vec![Effect::draw(draw_count as i32)];
    if let Some(amount) = fact.life_loss {
        replacement_effects.push(Effect::lose_life(amount as i32));
    }

    let draw_amount_text = match draw_count {
        1 => "a".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        _ => draw_count.to_string(),
    };
    let draw_card_text = if draw_count == 1 { "card" } else { "cards" };
    let mut display = format!(
        "If you would draw a card while you have no cards in hand, instead you draw {draw_amount_text} {draw_card_text}"
    );
    if let Some(amount) = fact.life_loss {
        display.push_str(&format!(" and you lose {amount} life"));
    }
    display.push('.');

    Ok(Some(StaticAbility::conditional_draw_replacement(
        Condition::Not(Box::new(Condition::CardsInHandOrMore(1))),
        replacement_effects,
        display,
    )))
}

pub(crate) fn parse_keyword_action_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_keyword_action_replacement_tokens(tokens) else {
        return Ok(None);
    };
    let display = render_token_slice(tokens);
    Ok(Some(match shape {
        keyword_static_lines::KeywordActionReplacementShape::ProliferateYouTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Proliferate,
                ObjectFilter::default().controlled_by(PlayerFilter::You),
                vec![Effect::proliferate(2)],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ProliferateOpponentTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Proliferate,
                ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
                vec![Effect::proliferate(2)],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ExploreTwice => {
            let explored_creature = ChooseSpec::tagged(IT_TAG);
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Explore,
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                vec![
                    Effect::explore(explored_creature.clone()),
                    Effect::explore(explored_creature),
                ],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ExploreAfterScry { value_tokens } => {
            let value_words = parser_token_word_refs(value_tokens);
            let (count, used) = parse_value_expr_words(&value_words).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported scry amount in keyword-action replacement (clause: '{}')",
                    render_token_slice(tokens)
                ))
            })?;
            if used != value_words.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported scry amount in keyword-action replacement (clause: '{}')",
                    render_token_slice(tokens)
                )));
            }
            let explored_creature = ChooseSpec::tagged(IT_TAG);
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Explore,
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                vec![Effect::scry(count), Effect::explore(explored_creature)],
                display,
            )
        }
    }))
}

pub(crate) fn parse_exile_to_countered_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(tokens) else {
        return Ok(None);
    };

    Ok(Some(
        StaticAbility::exile_to_countered_exile_instead_of_graveyard(
            spec.player,
            spec.counter_type,
        ),
    ))
}

pub(crate) fn parse_exile_to_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_exile_to_graveyard_replacement_tokens(tokens)
    else {
        return Ok(None);
    };
    let graveyard_owner = match spec.graveyard_owner {
        keyword_static_lines::ReplacementPlayerKind::Any => PlayerFilter::Any,
        keyword_static_lines::ReplacementPlayerKind::You => PlayerFilter::You,
        keyword_static_lines::ReplacementPlayerKind::Opponent => PlayerFilter::Opponent,
    };
    let filter = match spec.filter_kind {
        keyword_static_lines::ExileGraveyardFilterKind::Source => ObjectFilter::source(),
        keyword_static_lines::ExileGraveyardFilterKind::AnyCard => ObjectFilter::default(),
        keyword_static_lines::ExileGraveyardFilterKind::CreatureCard => ObjectFilter::creature(),
        keyword_static_lines::ExileGraveyardFilterKind::CyclingCard => {
            ObjectFilter::default().with_ability_marker("cycling")
        }
        keyword_static_lines::ExileGraveyardFilterKind::ObjectFilter => {
            parse_object_filter(spec.filter_tokens, false)?
        }
    };
    let ability = if spec.exclude_cycled {
        StaticAbility::exile_to_exile_instead_of_graveyard_unless_cycled(filter, graveyard_owner)
    } else {
        StaticAbility::exile_to_exile_instead_of_graveyard(filter, graveyard_owner)
    };
    Ok(Some(ability))
}

pub(crate) fn parse_exile_would_die_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_exile_would_die_tokens(tokens) else {
        return Ok(None);
    };
    let ability = match spec {
        keyword_static_lines::ExileWouldDieSpec::NontokenCreature {
            controller,
            exile_counter,
            follow_up_token,
        } => {
            let matched_filter = match controller {
                keyword_static_lines::ReplacementPlayerKind::Any => {
                    ObjectFilter::creature().nontoken()
                }
                keyword_static_lines::ReplacementPlayerKind::You => ObjectFilter::creature()
                    .nontoken()
                    .controlled_by(PlayerFilter::You),
                keyword_static_lines::ReplacementPlayerKind::Opponent => ObjectFilter::creature()
                    .nontoken()
                    .controlled_by(PlayerFilter::Opponent),
            };
            let exile_with_counters = exile_counter
                .map(|counter_type| vec![(counter_type, 1)])
                .unwrap_or_default();
            let follow_up = follow_up_token
                .map(build_replacement_creature_token)
                .map(|token| vec![Effect::create_tokens(token, 1)])
                .unwrap_or_default();
            StaticAbility::exile_would_die_instead_with_damage_source_counters_and_follow_up(
                matched_filter,
                None,
                exile_with_counters,
                follow_up,
            )
        }
        keyword_static_lines::ExileWouldDieSpec::DamagedBy { victim, damaged_by } => {
            let victim = match victim {
                keyword_static_lines::ExileWouldDieVictimKind::Creature => ObjectFilter::creature(),
                keyword_static_lines::ExileWouldDieVictimKind::Permanent => {
                    ObjectFilter::permanent()
                }
            };
            StaticAbility::exile_would_die_instead_with_damage_source(victim, Some(damaged_by))
        }
        keyword_static_lines::ExileWouldDieSpec::SimpleSource(kind) => {
            let filter = match kind {
                keyword_static_lines::SimpleSourceReplacementKind::Any => ObjectFilter::source(),
                keyword_static_lines::SimpleSourceReplacementKind::Creature => {
                    ObjectFilter::source().with_type(CardType::Creature)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Artifact => {
                    ObjectFilter::source().with_type(CardType::Artifact)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Enchantment => {
                    ObjectFilter::source().with_type(CardType::Enchantment)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Permanent => {
                    ObjectFilter::source()
                }
            };
            StaticAbility::exile_would_die_instead(filter)
        }
        keyword_static_lines::ExileWouldDieSpec::SimpleCreature(player) => {
            let player = match player {
                keyword_static_lines::ReplacementPlayerKind::Any => PlayerFilter::Any,
                keyword_static_lines::ReplacementPlayerKind::You => PlayerFilter::You,
                keyword_static_lines::ReplacementPlayerKind::Opponent => PlayerFilter::Opponent,
            };
            StaticAbility::exile_would_die_instead(ObjectFilter::creature().controlled_by(player))
        }
    };
    Ok(Some(ability))
}

pub(crate) fn build_replacement_creature_token(
    shape: crate::runtime_backend::token_definition::CreatureTokenShape,
) -> crate::cards::CardDefinition {
    use crate::runtime_backend::token_definition::TokenKeywordShape;

    let crate::runtime_backend::token_definition::CreatureTokenShape {
        name,
        card_types,
        subtypes,
        power_toughness,
        legendary,
        colors,
        keywords,
        ..
    } = shape;
    let mut builder = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), &name)
        .token()
        .card_types(card_types)
        .subtypes(subtypes)
        .color_indicator(colors)
        .power_toughness(crate::card::PowerToughness::fixed(
            power_toughness.0,
            power_toughness.1,
        ));
    if legendary {
        builder = builder.supertypes(vec![Supertype::Legendary]);
    }
    for keyword in keywords {
        builder = match keyword {
            TokenKeywordShape::Flying => builder.flying(),
            TokenKeywordShape::Defender => builder.defender(),
            TokenKeywordShape::Prowess => builder.prowess(),
            TokenKeywordShape::Vigilance => builder.vigilance(),
            TokenKeywordShape::Trample => builder.trample(),
            TokenKeywordShape::Lifelink => builder.lifelink(),
            TokenKeywordShape::Deathtouch => builder.deathtouch(),
            TokenKeywordShape::Haste => builder.haste(),
            TokenKeywordShape::Menace => builder.menace(),
            TokenKeywordShape::Reach => builder.reach(),
        };
    }
    builder.build()
}

pub(crate) fn parse_discard_or_redirect_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(
        static_keyword_replacement_shapes::parse_discard_or_redirect_replacement(tokens).map(
            |shape| {
                StaticAbility::discard_or_redirect_replacement(
                    ObjectFilter::default().with_type(shape.discard_type),
                    shape.redirect_zone,
                )
            },
        ),
    )
}

pub(crate) fn parse_pay_life_or_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let fact = match late_static_facts::parse_pay_life_or_enter_tapped_tokens(tokens) {
        Ok(Some(fact)) => fact,
        Ok(None) => return Ok(None),
        Err(error) => {
            let clause = parser_token_word_refs(tokens).join(" ");
            let detail = match error {
                late_static_facts::PayLifeOrEnterTappedError::MissingPay => {
                    "missing 'pay' keyword in pay-life ETB clause"
                }
                late_static_facts::PayLifeOrEnterTappedError::UnsupportedPrefix => {
                    "unsupported pay-life ETB prefix"
                }
                late_static_facts::PayLifeOrEnterTappedError::MissingAmount => {
                    "missing life payment amount in pay-life ETB clause"
                }
                late_static_facts::PayLifeOrEnterTappedError::MissingIfYouDont => {
                    "unsupported pay-life ETB trailing clause (expected 'if you don't ...')"
                }
                late_static_facts::PayLifeOrEnterTappedError::UnsupportedTail => {
                    "unsupported pay-life ETB trailing clause"
                }
            };
            return Err(CardTextError::ParseError(format!(
                "{detail} (clause: '{clause}')"
            )));
        }
    };

    parser_trace("parse_static:pay-life-etb:matched", tokens);
    Ok(Some(StaticAbility::pay_life_or_enter_tapped(fact.amount)))
}

pub(crate) fn parse_copy_activated_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(fact) = late_static_facts::parse_copy_activated_abilities_tokens(tokens) else {
        return Ok(None);
    };
    let clause_words = parser_token_word_refs(tokens);

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, fact.marker_token)
    {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..fact.marker_token]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let filter_tokens =
        trim_edge_punctuation(&tokens[fact.filter_start_token..fact.filter_end_token]);
    let filter_tokens = strip_leading_token_words_any(&filter_tokens, &["all", "each"]).to_vec();
    let force_once_each_turn = fact.once_each_turn_word_start.is_some();
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    let counter = match filter.with_counter {
        Some(crate::filter::CounterConstraint::Typed(counter_type)) => Some(counter_type),
        _ => None,
    };

    let display_words = copy_activated_abilities_display_words(&clause_words);
    let display = if force_once_each_turn {
        let display_tail_start = fact
            .once_each_turn_word_start
            .map(|start| copy_activated_display_index_for_original_word(&clause_words, start));
        if let Some(start) = display_tail_start {
            format!(
                "{}. You may activate each of those abilities only once each turn",
                display_words[..start].join(" ").trim()
            )
        } else {
            display_words.join(" ")
        }
    } else {
        display_words.join(" ")
    };

    let mut ability = crate::static_abilities::CopyActivatedAbilities::new(filter)
        .with_exclude_source_name(fact.exclude_source_name)
        .with_exclude_source_id(true)
        .with_display(display);
    if let Some(counter) = counter {
        ability = ability.with_counter(counter);
    }
    if fact.only_loyalty {
        ability = ability.with_only_loyalty();
    }
    if force_once_each_turn {
        ability = ability.with_once_each_turn();
    }

    let ability = StaticAbility::copy_activated_abilities(ability);
    let ast = match subject {
        AnthemSubjectAst::Source => match condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition,
            },
            None => StaticAbilityAst::Static(ability),
        },
        AnthemSubjectAst::Filter(subject_filter) => StaticAbilityAst::GrantStaticAbility {
            filter: subject_filter,
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition,
        },
    };

    Ok(Some(ast))
}

pub(crate) fn copy_activated_abilities_display_words<'a>(clause_words: &[&'a str]) -> Vec<&'a str> {
    let mut display_words = Vec::with_capacity(clause_words.len());
    for (idx, word) in clause_words.iter().copied().enumerate() {
        if copy_activated_should_skip_display_word(clause_words, idx, word) {
            continue;
        }
        display_words.push(word);
    }
    display_words
}

pub(crate) fn copy_activated_display_index_for_original_word(
    clause_words: &[&str],
    original_idx: usize,
) -> usize {
    clause_words
        .iter()
        .enumerate()
        .take(original_idx)
        .filter(|(idx, word)| !copy_activated_should_skip_display_word(clause_words, *idx, word))
        .count()
}

pub(crate) fn copy_activated_should_skip_display_word(
    clause_words: &[&str],
    idx: usize,
    word: &str,
) -> bool {
    idx >= 2
        && word == clause_words[idx - 1]
        && clause_words[idx - 2] == "this"
        && copy_activated_display_source_noun(word)
}

pub(crate) fn copy_activated_display_source_noun(word: &str) -> bool {
    matches!(word, "card" | "permanent" | "source" | "spell")
        || parse_card_type(word).is_some()
        || parse_subtype_flexible(word).is_some()
}

pub(crate) fn parse_spend_mana_as_any_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_mana_spend_permission_tokens(tokens) else {
        return Ok(None);
    };
    let clause_words = parser_token_word_refs(tokens);
    let (permission, display) = match shape {
        keyword_static_lines::ManaSpendPermissionShape::SymbolAsAnyColorOtherAsColorless {
            symbol,
        } => {
            let symbol_text = match symbol {
                ManaSymbol::White => "white",
                ManaSymbol::Blue => "blue",
                ManaSymbol::Black => "black",
                ManaSymbol::Red => "red",
                ManaSymbol::Green => "green",
                _ => unreachable!("typed grammar only returns colored mana symbols"),
            };
            (
                crate::effect::ManaSpendPermission::mana_symbol_as_any_color_other_as_colorless(
                    PlayerFilter::You,
                    symbol,
                ),
                format!(
                    "You may spend {symbol_text} mana as though it were mana of any color. You may spend other mana only as though it were colorless mana"
                ),
            )
        }
        keyword_static_lines::ManaSpendPermissionShape::AnyTypeToCast { filter_tokens } => {
            let filter = parse_object_filter(filter_tokens, false)
                .map(|mut filter| {
                    filter.zone = None;
                    filter.stack_kind = None;
                    filter.has_mana_cost = false;
                    filter
                })
                .map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported mana spend cast filter (clause: '{}')",
                        clause_words.join(" ")
                    ))
                })?;
            (
                crate::effect::ManaSpendPermission::any_type_for_casting_matching(
                    PlayerFilter::You,
                    filter,
                ),
                clause_words.join(" "),
            )
        }
        keyword_static_lines::ManaSpendPermissionShape::AnyColor {
            player,
            activation_filter_tokens,
            source_activation_only,
        } => {
            let player = match player {
                keyword_static_lines::ManaSpendPlayerKind::You => PlayerFilter::You,
                keyword_static_lines::ManaSpendPlayerKind::Any => PlayerFilter::Any,
            };
            let permission = if source_activation_only {
                crate::effect::ManaSpendPermission::any_color_for_activation(
                    player.clone(),
                    ObjectFilter::source(),
                )
            } else if let Some(filter_tokens) = activation_filter_tokens {
                let filter = match parse_object_filter(filter_tokens, false) {
                    Ok(filter) => filter,
                    Err(_) => return Ok(None),
                };
                crate::effect::ManaSpendPermission::any_color_for_activation(player.clone(), filter)
            } else {
                crate::effect::ManaSpendPermission::any_color(player.clone())
            };
            let display = if player == PlayerFilter::Any {
                "Players may spend mana as though it were mana of any color".to_string()
            } else {
                clause_words.join(" ")
            };
            (permission, display)
        }
    };

    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::mana_spend_permission(permission, display),
    )))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn supported_keyword_marker_uses_token_shapes_for_crew_markers() {
        for line in [
            "This creature crews Vehicles using its toughness rather than its power.",
            "This token saddles Mounts and crews Vehicles as though its power were 2 greater.",
            "You may remove a loyalty counter from a planeswalker you control rather than pay this creature's crew cost.",
        ] {
            let tokens = lex_line(line, 0).expect("marker line should lex");
            let text = render_token_slice(&tokens);
            assert!(
                supported_keyword_marker_tokens(&tokens, &text),
                "{line} should be recognized through token shapes"
            );
        }
    }

    #[test]
    fn pt_modifier_parsers_use_char_signs() {
        assert_eq!(parse_pt_modifier("+2/-1").unwrap(), (2, -1));
        assert_eq!(parse_pt_modifier("2/+3").unwrap(), (2, 3));
        assert_eq!(
            parse_pt_modifier_values("−X/+2").unwrap(),
            (Value::XTimes(-1), Value::Fixed(2))
        );
    }

    #[test]
    fn early_static_ability_parser_uses_parser_token_words() {
        for line in [
            "X can't be greater than the number of players in the game.",
            "This creature can't attack unless you've cast a creature spell this turn.",
            "During your turn, as long as you haven't activated an exhaust ability this turn, you may activate exhaust abilities as though they haven't been activated.",
        ] {
            let tokens = lex_line(line, 0).expect("static line should lex");
            assert!(
                parse_static_ability_ast_line_early_lexed(&tokens)
                    .expect("early static line should parse")
                    .is_some(),
                "{line} should match through parser token words"
            );
        }
    }

    #[test]
    fn parse_keyword_action_replacement_static_line() {
        let tokens =
            lex_line("If you would proliferate, proliferate twice instead.", 0).expect("lex");
        let parsed = parse_keyword_action_replacement_line(&tokens)
            .expect("keyword-action replacement parser should not hard-error");
        assert!(
            parsed
                .as_ref()
                .is_some_and(|ability| ability.id() == StaticAbilityId::KeywordActionReplacement),
            "expected keyword-action replacement static ability, got {parsed:?}"
        );
        let parsed = parse_static_ability_ast_line_lexed(&tokens)
            .expect("static ability line parser should not hard-error");
        assert!(
            parsed
                .as_ref()
                .is_some_and(|abilities| abilities.iter().any(
                    |ability| matches!(ability, StaticAbilityAst::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::KeywordActionReplacement)
                )),
            "expected static line parser to preserve keyword-action replacement, got {parsed:?}"
        );
    }

    #[test]
    fn dynamic_cost_other_shapes_prefer_typed_turn_history_counts() {
        for (text, expected_query) in [
            (
                "less to cast for each opponent who was dealt damage this turn.",
                "PlayersDealtDamage",
            ),
            (
                "less to cast for each card you've cycled or discarded this turn.",
                "DiscardedOrCycled",
            ),
            (
                "less to cast for each creature you attacked with this turn.",
                "CreaturesAttackedWith",
            ),
        ] {
            let tokens = lex_line(text, 0).expect("dynamic cost text should lex");
            let value = parse_dynamic_cost_modifier_value(&tokens)
                .expect("dynamic cost should not hard-error")
                .expect("dynamic cost should produce a value");
            let debug = format!("{value:?}");
            assert!(
                debug.contains("TurnHistoryCount")
                    && debug.contains(expected_query)
                    && debug.contains("ForEach"),
                "expected typed for-each turn-history value for {text}, got {debug}"
            );
        }
    }

    #[test]
    fn specialized_card_types_among_cost_value_precedes_history_fallback() {
        let tokens = lex_line(
            "less to cast for each card type among permanents you've sacrificed this turn.",
            0,
        )
        .expect("card-types-among cost text should lex");
        let value = parse_dynamic_cost_modifier_value(&tokens)
            .expect("dynamic cost should not hard-error")
            .expect("dynamic cost should produce a value");
        assert!(
            matches!(value.unhinted(), Value::CardTypesAmong(_)),
            "expected specialized card-types-among value, got {value:?}"
        );
    }

    fn parsed_spell_cost_filter(line: &str) -> ObjectFilter {
        let tokens = lex_line(line, 0).expect("spell-cost line should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("spell-cost parser should not hard-error")
            .expect("spell-cost line should be recognized");
        match ability.payload {
            ironsmith_core::StaticAbilityPayload::CostReduction(reduction) => reduction.filter,
            ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction) => {
                reduction.filter
            }
            other => panic!("expected a shared spell-cost reduction, got {other:?}"),
        }
    }

    #[test]
    fn chosen_type_spell_cost_filters_survive_actor_word_order_variants() {
        for (line, creature_only) in [
            (
                "Spells you cast of the chosen type cost {1} less to cast.",
                false,
            ),
            (
                "Creature spells you cast of the chosen type cost {1} less to cast.",
                true,
            ),
            (
                "Spells of the chosen type you cast cost {W}{U}{B}{R}{G} less to cast.",
                false,
            ),
            (
                "Creature spells of the chosen type cost {2} less to cast.",
                true,
            ),
        ] {
            let filter = parsed_spell_cost_filter(line);
            assert!(filter.chosen_creature_type, "{line}: {filter:#?}");
            assert_eq!(
                filter.card_types.contains(&CardType::Creature),
                creature_only,
                "{line}: {filter:#?}"
            );
            if line.contains("you cast") {
                assert_eq!(filter.cast_by, Some(PlayerFilter::You), "{line}");
            }
        }
    }

    #[test]
    fn explicit_chosen_card_type_cost_filter_and_unrestricted_control_stay_distinct() {
        let chosen = parsed_spell_cost_filter(
            "Spells of the chosen card type you cast cost {1} less to cast.",
        );
        assert!(chosen.chosen_card_type, "{chosen:#?}");
        assert_eq!(chosen.cast_by, Some(PlayerFilter::You));

        let unrestricted =
            parsed_spell_cost_filter("Creature spells you cast cost {1} less to cast.");
        assert!(!unrestricted.chosen_creature_type, "{unrestricted:#?}");
        assert!(!unrestricted.chosen_card_type, "{unrestricted:#?}");
        assert_eq!(unrestricted.card_types, vec![CardType::Creature]);
        assert_eq!(unrestricted.cast_by, Some(PlayerFilter::You));
    }

    #[test]
    fn colored_spell_cost_modifiers_preserve_exact_per_target_scaling() {
        let reduction_tokens =
            lex_line("Spells you cast cost {W} less to cast for each target.", 0)
                .expect("colored reduction should lex");
        let reduction = parse_spells_cost_modifier_line(&reduction_tokens)
            .expect("colored reduction should not hard-error")
            .expect("colored reduction should parse");
        let reduction = match reduction.payload {
            ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction) => reduction,
            other => panic!("expected a mana-symbol reduction, got {other:?}"),
        };
        assert!(reduction.per_target);
        assert_eq!(reduction.cost.to_oracle(), "{W}");

        let increase_tokens = lex_line(
            "Spells your opponents cast cost {U} more to cast for each target.",
            0,
        )
        .expect("colored increase should lex");
        let increase = parse_spells_cost_modifier_line(&increase_tokens)
            .expect("colored increase should not hard-error")
            .expect("colored increase should parse");
        let increase = match increase.payload {
            ironsmith_core::StaticAbilityPayload::CostIncreaseManaCost(increase) => increase,
            other => panic!("expected a mana-symbol increase, got {other:?}"),
        };
        assert!(increase.per_target);
        assert_eq!(increase.cost.to_oracle(), "{U}");
    }

    #[test]
    fn non_hand_origin_filter_includes_cards_owned_by_another_player_in_hand() {
        let filter = parsed_spell_cost_filter(
            "Spells you cast from anywhere other than your hand cost {1} less to cast.",
        );

        assert!(filter.any_of.iter().any(|branch| {
            branch.zone == Some(Zone::Hand) && branch.owner == Some(PlayerFilter::NotYou)
        }));
    }

    #[test]
    fn cost_modifier_target_spec_preserves_player_or_controlled_permanent_union() {
        let tokens =
            lex_line("you or a permanent you control", 0).expect("cost-modifier target should lex");
        let (player, object, targets_any_of) =
            parse_cost_modifier_target_spec(&tokens).expect("cost-modifier target should parse");

        assert_eq!(player, Some(PlayerFilter::You));
        assert!(targets_any_of);
        let object = object.expect("permanent target branch should be retained");
        assert_eq!(object.zone, Some(Zone::Battlefield));
        assert_eq!(object.controller, Some(PlayerFilter::You));
    }
}
