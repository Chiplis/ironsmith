use super::*;

pub fn parse_commander_cast_count_player(tokens: &[OwnedLexToken]) -> Option<PlayerFilter> {
    let words = TokenWordView::new(tokens).to_word_refs();
    value_helper_shapes::parse_commander_cast_count_player(&words)
}

pub fn parse_equal_to_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    let words_all = word_view.to_word_refs();
    // Callers that have already split an `equal to` clause pass only the
    // amount tail (`the number of ...`). Accept that typed amount directly as
    // well as the unsplit authored clause.
    let prefix_start = parse_equal_to_start(&words_all)
        .map(|start| start.after)
        .unwrap_or(0);
    let suffix_refs = words_all.get(prefix_start..)?;
    let matched = value_helper_shapes::parse_number_of_prefix(suffix_refs)?;
    let number_word_idx = prefix_start + matched.number_of_start;

    let value_range = word_view.token_span_for_words(number_word_idx, word_view.len())?;
    let value_tokens = trim_edge_punctuation(&tokens[value_range]);
    let filter_start_word_idx = number_word_idx + 2;
    let filter_range = word_view.token_span_for_words(filter_start_word_idx, word_view.len())?;
    let filter_tokens = trim_edge_punctuation(&tokens[filter_range]);
    let filter_word_view = TokenWordView::new(&filter_tokens);
    let filter_words = filter_word_view.to_word_refs();
    let possessive_filter_words = possessive_normalized_word_refs(&filter_words);
    if crate::word_primitives::parse_sequence_suffix(
        &possessive_filter_words,
        &[
            "that",
            "opponent",
            "or",
            "that",
            "planeswalkers",
            "controller",
            "controls",
        ],
    ) {
        // This is a coordinated player antecedent, not the narrower
        // `that OBJECT's controller` relation below. The ordinary typed
        // object-filter grammar already owns the complete suffix and maps it
        // to TargetPlayerOrControllerOfTarget; let it retain both arms.
        let filter = parse_object_filter(&filter_tokens, false).ok()?;
        return Some(Value::Count(filter).with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    // A relative controller clause scopes the counted set to the object
    // targeted by this same effect. Parse the set independently from the
    // back-reference so characteristic words in `that creature's controller`
    // cannot leak into the counted filter as an additional Creature type.
    if let Some(that_idx) =
        crate::word_primitives::parse_last_sequence_start(&filter_words, &["that"])
    {
        let relative = possessive_normalized_word_refs(&filter_words[that_idx..]);
        let relative_noun = relative.get(1).map(|word| word.trim_end_matches('s'));
        if relative.len() == 4
            && relative[0] == "that"
            && matches!(
                relative_noun,
                Some("creature" | "permanent" | "object" | "planeswalker")
            )
            && relative[2] == "controller"
            && relative[3] == "controls"
            && that_idx > 0
        {
            let base_range = filter_word_view.token_span_for_words(0, that_idx)?;
            let mut filter =
                parse_object_filter(&trim_edge_punctuation(&filter_tokens[base_range]), false)
                    .ok()?;
            filter.controller = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
            return Some(Value::Count(filter).with_surface_hint(ValueSurfaceHint::EqualTo));
        }
    }
    if let Some(value) = parse_turn_history_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(value) = parse_creatures_died_this_turn_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(value) = parse_cards_discarded_this_turn_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some((players, minimum)) = parse_players_with_cards_in_hand_at_least(&filter_tokens) {
        return Some(
            Value::CountPlayersWithCardsInHandAtLeast(players, minimum)
                .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }
    if let Some(player) = value_helper_shapes::parse_cards_in_hand_player(&filter_words) {
        let mut value = Value::CardsInHand(player).with_surface_hint(ValueSurfaceHint::EqualTo);
        if value_helper_shapes::has_that_player_possessive(&filter_words) {
            value = value.with_surface_hint(ValueSurfaceHint::ThatPlayerPossessive);
        }
        return Some(value);
    }
    if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(player) = value_helper_shapes::parse_party_size_player(&filter_words) {
        return Some(Value::PartySize(player).with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(value) = parse_aggregate_scope_value_lexed(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    let mut for_each_words = vec!["for", "each"];
    for_each_words.extend(filter_words.iter().copied());
    if let Some((value @ Value::PendingPriorEffectMetric(_), used)) =
        super::super::count_shapes::parse_for_each_count_value_words(&for_each_words)
        && used == for_each_words.len()
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(distinct_filter_tokens) =
        primitives::parse_word_sequence_prefix(&filter_words, &["differently", "named"]).and_then(
            |remaining| {
                let consumed = filter_words.len().saturating_sub(remaining.len());
                filter_word_view
                    .token_span_for_words(consumed, filter_word_view.len())
                    .map(|range| &filter_tokens[range])
            },
        )
    {
        let filter = parse_object_filter(distinct_filter_tokens, false).ok()?;
        return Some(Value::DistinctNames(filter).with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some((value, used)) = value_expr::parse_value_expr_tokens(&value_tokens)
        && TokenWordView::new(&value_tokens[used..]).is_empty()
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::Count(filter).with_surface_hint(ValueSurfaceHint::EqualTo))
}

pub fn parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    let clause_words = word_view.to_word_refs();
    if parse_equal_to_start(&clause_words).is_none_or(|parsed| parsed.start != 0) {
        return None;
    }

    let suffix_refs = clause_words.get(EQUAL_TO_PHRASE.len()..)?;
    let matched = value_helper_shapes::parse_number_of_prefix(suffix_refs)?;
    let filter_start_word_idx = EQUAL_TO_PHRASE.len() + matched.consumed;
    let operator_word_idx =
        word_view.parse_any_word_position_from(&["plus", "minus"], filter_start_word_idx + 1)?;
    let operator = clause_words[operator_word_idx];

    let filter_range = word_view.token_span_for_words(filter_start_word_idx, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_range]);
    let base_value = if let Some(value) = parse_turn_history_count_value(&filter_tokens) {
        value
    } else if let Some(value) = parse_creatures_died_this_turn_count_value(&filter_tokens) {
        value
    } else if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        value
    } else if let Some(player) = value_helper_shapes::parse_party_size_player(
        &TokenWordView::new(&filter_tokens).to_word_refs(),
    ) {
        Value::PartySize(player)
    } else {
        Value::Count(parse_object_filter(&filter_tokens, false).ok()?)
    };

    let offset_range = word_view.token_span_for_words(operator_word_idx + 1, word_view.len())?;
    let offset_tokens = trim_commas(&tokens[offset_range]);
    let (offset_value, used) =
        leaf::parse_leaf_number_prefix_tokens(&offset_tokens)?.into_fixed()?;
    if !TokenWordView::new(&offset_tokens[used..]).is_empty() {
        return None;
    }

    let signed_offset = if operator == "minus" {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(
        Value::Add(Box::new(base_value), Box::new(Value::Fixed(signed_offset)))
            .with_surface_hint(ValueSurfaceHint::EqualTo),
    )
}

pub fn parse_equal_to_aggregate_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = TokenWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    let prefix_start = parse_equal_to_start(&clause_refs)?.after;
    let suffix_refs = clause_refs.get(prefix_start..)?;
    let matched = value_helper_shapes::parse_aggregate_prefix(suffix_refs)?;
    let aggregate = matched.aggregate;
    let value_kind = matched.value_kind;
    let idx = prefix_start + matched.consumed;

    if aggregate == value_helper_shapes::AggregateKind::Greatest
        && value_kind == value_helper_shapes::AggregateValueKind::ManaValue
        && let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx)
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }

    let filter_range = clause_words.token_span_for_words(idx, clause_words.len())?;
    let filter_tokens = &tokens[filter_range];
    let object_words = &clause_refs[idx..];
    if aggregate == value_helper_shapes::AggregateKind::Total
        && value_kind == value_helper_shapes::AggregateValueKind::ManaValue
        && let Some(Value::SpellsCastThisTurnMatching {
            player,
            mut filter,
            exclude_source,
        }) = parse_spells_cast_this_turn_matching_count_value(filter_tokens)
    {
        // `other` in this history phrase is relative to the spell whose value
        // is being evaluated. It is carried explicitly by `exclude_source`;
        // leaving it on the snapshot filter would apply a second, context-
        // dependent object relation.
        filter.other = false;
        return Some(
            Value::TotalManaValueOfSpellsCastThisTurnMatching {
                player,
                filter,
                exclude_source,
            }
            .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }
    if value_kind == value_helper_shapes::AggregateValueKind::ManaValue
        && let Some(value) = source_linked_exiled_mana_value(object_words)
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(value) = pending_aggregate_metric_value(aggregate, value_kind, object_words) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    let mut filter = parse_object_filter(filter_tokens, false).ok()?;
    if object_words
        .iter()
        .any(|word| matches!(*word, "permanent" | "permanents"))
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
    {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }

    Some(
        aggregate_filter_value(aggregate, value_kind, filter)
            .with_surface_hint(ValueSurfaceHint::EqualTo),
    )
}

pub fn parse_filter_comparison_tokens(
    axis: &str,
    tokens: &[&str],
    clause_words: &[&str],
) -> Result<Option<(crate::filter::Comparison, usize)>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    if is_power_toughness_axis_word(axis) && value_helper_shapes::starts_or_power_toughness(tokens)
    {
        return Ok(None);
    }

    let to_comparison = |operator: ValueComparisonOperator,
                         operand: Value|
     -> crate::filter::Comparison {
        use crate::filter::Comparison;

        match (operator, operand) {
            (ValueComparisonOperator::Equal, Value::Fixed(value)) => Comparison::Equal(value),
            (ValueComparisonOperator::NotEqual, Value::Fixed(value)) => Comparison::NotEqual(value),
            (ValueComparisonOperator::LessThan, Value::Fixed(value)) => Comparison::LessThan(value),
            (ValueComparisonOperator::LessThanOrEqual, Value::Fixed(value)) => {
                Comparison::LessThanOrEqual(value)
            }
            (ValueComparisonOperator::GreaterThan, Value::Fixed(value)) => {
                Comparison::GreaterThan(value)
            }
            (ValueComparisonOperator::GreaterThanOrEqual, Value::Fixed(value)) => {
                Comparison::GreaterThanOrEqual(value)
            }
            (ValueComparisonOperator::Equal, operand) => Comparison::EqualExpr(Box::new(operand)),
            (ValueComparisonOperator::NotEqual, operand) => {
                Comparison::NotEqualExpr(Box::new(operand))
            }
            (ValueComparisonOperator::LessThan, operand) => {
                Comparison::LessThanExpr(Box::new(operand))
            }
            (ValueComparisonOperator::LessThanOrEqual, operand) => {
                Comparison::LessThanOrEqualExpr(Box::new(operand))
            }
            (ValueComparisonOperator::GreaterThan, operand) => {
                Comparison::GreaterThanExpr(Box::new(operand))
            }
            (ValueComparisonOperator::GreaterThanOrEqual, operand) => {
                Comparison::GreaterThanOrEqualExpr(Box::new(operand))
            }
        }
    };

    let parse_operand = |operand_tokens: &[&str],
                         operator: ValueComparisonOperator|
     -> Result<(crate::filter::Comparison, usize), CardTextError> {
        let Some((operand, used)) = value_expr::parse_value_expr_words(operand_tokens) else {
            let quoted = operand_tokens
                .first()
                .copied()
                .unwrap_or_default()
                .to_string();
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        Ok((to_comparison(operator, operand), used))
    };

    let parse_numeric_token = |word: &str| -> Option<i32> {
        if let Ok(value) = word.parse::<i32>() {
            return Some(value);
        }
        leaf::parse_number_i32_complete(word).ok()
    };

    let first = tokens[0];
    if let Some(value) = parse_numeric_token(first) {
        if tokens.get(1).is_some_and(|word| is_plus_minus_word(word)) {
            let (cmp, used) = parse_operand(tokens, ValueComparisonOperator::Equal)?;
            return Ok(Some((cmp, used)));
        }
        let mut values = vec![value];
        let mut consumed = 1usize;
        while consumed < tokens.len() {
            let token = tokens[consumed];
            if is_and_or_word(token) {
                consumed += 1;
                continue;
            }
            if let Some(next_value) = parse_numeric_token(token) {
                values.push(next_value);
                consumed += 1;
                continue;
            }
            break;
        }
        if values.len() > 1 {
            return Ok(Some((crate::filter::Comparison::OneOf(values), consumed)));
        }
        if tokens.len() == 1 {
            return Ok(Some((crate::filter::Comparison::Equal(value), 1)));
        }
    }

    if let Some((operator, operand_words, consumed_base)) = parse_value_comparison_words(tokens) {
        if operand_words.is_empty() {
            let consumed_phrase = consumed_base;
            let phrase = tokens[..consumed_phrase].join(" ");
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after '{phrase}' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let (operand, used) =
            value_expr::parse_value_expr_words(operand_words).ok_or_else(|| {
                let quoted = operand_words.first().copied().unwrap_or_default();
                CardTextError::ParseError(format!(
                    "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let operand = if starts_explicit_ordered_comparison(tokens, operator)
            && !matches!(operand.unhinted(), Value::Fixed(_))
        {
            operand.with_surface_hint(ValueSurfaceHint::ExplicitComparison)
        } else {
            operand
        };
        let consumed = consumed_base + used;
        return Ok(Some((to_comparison(operator, operand), consumed)));
    }

    if let Some((value, used)) = value_expr::parse_value_expr_words(tokens) {
        if tokens.get(used).copied() == Some("or")
            && let Some(next) = tokens.get(used + 1)
            && is_comparison_tail_word(next)
        {
            let operator = if is_less_or_fewer_word(next) {
                ValueComparisonOperator::LessThanOrEqual
            } else {
                ValueComparisonOperator::GreaterThanOrEqual
            };
            return Ok(Some((to_comparison(operator, value), used + 2)));
        }
        if let Value::Fixed(fixed) = value
            && used == 1
        {
            return Ok(Some((crate::filter::Comparison::Equal(fixed), used)));
        }
        return Ok(Some((
            crate::filter::Comparison::EqualExpr(Box::new(value)),
            used,
        )));
    }

    Ok(None)
}
