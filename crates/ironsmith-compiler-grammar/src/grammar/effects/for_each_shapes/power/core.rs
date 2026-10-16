use super::*;

pub fn parse_base_power_or_toughness_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<BasePowerToughnessClauseShape<'_>>, CardTextError> {
    let Some((subject, rest)) = split_subject_and_rest(tokens) else {
        return Ok(None);
    };
    let Some((_, power_tokens)) =
        primitives::parse_prefix(rest, primitives::phrase(&["base", "power"]))
    else {
        return Ok(None);
    };
    let Some(parsed) = leaf::parse_leaf_number_or_x_prefix_tokens(power_tokens) else {
        return Ok(None);
    };
    let Some((power, used)) = parsed.into_value() else {
        return Ok(None);
    };
    let Some((_, toughness_tokens)) = primitives::parse_prefix(
        &power_tokens[used..],
        primitives::phrase(&["or", "base", "toughness"]),
    ) else {
        return Ok(None);
    };
    let Some(parsed) = leaf::parse_leaf_number_or_x_prefix_tokens(toughness_tokens) else {
        return Ok(None);
    };
    let Some((toughness, used)) = parsed.into_value() else {
        return Ok(None);
    };
    let tail = trim_edge_punctuation_tokens(&toughness_tokens[used..]);
    let (target_tokens, leading_duration) = target_and_leading_duration(subject);
    let duration = if tail.is_empty() {
        if !permits_unqualified_duration(subject, tokens) {
            return Ok(None);
        }
        leading_duration.unwrap_or(Until::Forever)
    } else if let Some(trailing) = complete_duration(tail) {
        if leading_duration
            .as_ref()
            .is_some_and(|leading| leading != &trailing)
        {
            return Err(CardTextError::ParseError(
                "conflicting base characteristic choice durations".into(),
            ));
        }
        trailing
    } else {
        return Ok(None);
    };
    Ok(Some(BasePowerToughnessClauseShape {
        power,
        toughness,
        target_tokens,
        duration,
        where_x_tokens: None,
    }))
}

pub fn parse_base_power_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<BasePowerClauseShape<'_>>, CardTextError> {
    parse_base_axis_clause_shape(tokens, "power")
}

/// "<subject> has base toughness N [until end of turn]" (Six-Sided Die,
/// Chariot of the Sun): the single-axis toughness counterpart. The shape's
/// `power` field carries the toughness value.
pub fn parse_base_toughness_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<BasePowerClauseShape<'_>>, CardTextError> {
    parse_base_axis_clause_shape(tokens, "toughness")
}

fn parse_base_axis_clause_shape<'a>(
    tokens: &'a [OwnedLexToken],
    axis: &'static str,
) -> Result<Option<BasePowerClauseShape<'a>>, CardTextError> {
    if tokens.first().is_some_and(|token| token.is_word("change")) {
        let Some(base) = tokens.windows(3).position(|words| words[0].is_word("base")
            && words[1].is_word(axis) && words[2].is_word("to")) else { return Ok(None); };
        if base <= 1 { return Ok(None); }
        let value_tokens = &tokens[base + 3..];
        let Some((power, used)) = crate::grammar::shared_util::value_expr::parse_value_expr_tokens(value_tokens) else { return Ok(None); };
        let tail = trim_edge_punctuation_tokens(&value_tokens[used..]);
        let duration = if tail.is_empty() { Until::Forever }
            else if let Some(duration) = complete_duration(tail) { duration }
            else { return Ok(None); };
        return Ok(Some(BasePowerClauseShape { power, target_tokens: &tokens[1..base], duration }));
    }
    let Some((subject, rest)) = split_subject_and_rest(tokens) else {
        return Ok(None);
    };
    let Some((_, value_tokens)) = primitives::parse_prefix(
        rest,
        (primitives::kw("base"), primitives::kw(axis)).void(),
    ) else {
        return Ok(None);
    };
    if primitives::parse_prefix(value_tokens, primitives::kw("and")).is_some() {
        return Ok(None);
    }
    let Some(parsed) = leaf::parse_leaf_number_or_x_prefix_tokens(value_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "invalid base power value (clause: '{}')",
            render_token_slice(tokens)
        )));
    };
    let Some((power, consumed)) = parsed.into_value() else {
        return Err(CardTextError::ParseError(format!(
            "invalid base power value (clause: '{}')",
            render_token_slice(tokens)
        )));
    };
    let tail = trim_edge_punctuation_tokens(value_tokens.get(consumed..).unwrap_or_default());
    let (target_tokens, leading_duration) = target_and_leading_duration(subject);
    let duration = if tail.is_empty() {
        if !permits_unqualified_duration(subject, tokens) {
            return Ok(None);
        }
        leading_duration.unwrap_or(Until::Forever)
    } else if let Some(trailing_duration) = complete_duration(tail) {
        if leading_duration
            .as_ref()
            .is_some_and(|leading| leading != &trailing_duration)
        {
            return Err(CardTextError::ParseError(format!(
                "conflicting base power durations (clause: '{}')",
                render_token_slice(tokens)
            )));
        }
        trailing_duration
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power clause (clause: '{}')",
            render_token_slice(tokens)
        )));
    };
    Ok(Some(BasePowerClauseShape {
        power,
        target_tokens,
        duration,
    }))
}

fn parse_base_pair_assignment_values(tokens: &[OwnedLexToken]) -> Option<(Value, Value, Until)> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let duration_start = tokens.iter().position(|token| token.is_word("until"));
    let (values, duration) = if let Some(start) = duration_start {
        (&tokens[..start], complete_duration(&tokens[start..])?)
    } else { (tokens, Until::Forever) };
    let values = trim_edge_punctuation_tokens(values);
    let (power, toughness) = if values.len() == 1 {
        leaf::parse_leaf_pt_modifier_values_complete(values[0].parser_text()).ok()?
    } else {
        crate::grammar::shared_util::value_expr::parse_power_toughness_value_pair_words(
            &crate::lexer::parser_token_word_refs(values))?
    };
    Some((power, toughness, duration))
}

pub fn parse_base_power_toughness_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<BasePowerToughnessClauseShape<'_>>, CardTextError> {
    if let Some(base) = tokens.windows(3).position(|words|
        words[0].is_word("base") && words[1].is_word("power") && words[2].is_word("becomes"))
        && let Some(second) = tokens.windows(5).position(|words|
            words[0].is_word("and") && words[1].is_word("its")
                && words[2].is_word("base") && words[3].is_word("toughness") && words[4].is_word("becomes"))
        && second > base + 3
    {
        let parse_value = crate::grammar::shared_util::value_expr::parse_value_expr_tokens;
        let power_tokens = &tokens[base + 3..second];
        if let Some((power, used)) = parse_value(power_tokens)
            && used == power_tokens.len()
            && let Some((toughness, used)) = parse_value(&tokens[second + 5..])
        {
            let (target_tokens, leading_duration) = target_and_leading_duration(&tokens[..base]);
            let tail = trim_edge_punctuation_tokens(&tokens[second + 5 + used..]);
            let duration = if tail.is_empty() { leading_duration.unwrap_or(Until::Forever) }
                else if let Some(trailing) = complete_duration(tail) {
                    if leading_duration.is_some_and(|leading| leading != trailing) { return Ok(None); }
                    trailing
                } else { return Ok(None); };
            if !target_tokens.is_empty() {
                return Ok(Some(BasePowerToughnessClauseShape { power, toughness, target_tokens, duration, where_x_tokens: None }));
            }
        }
    }
    if tokens.first().is_some_and(|token| token.is_word("change"))
        && let Some(base) = tokens.windows(5).position(|words|
            words[0].is_word("base") && words[1].is_word("power")
                && words[2].is_word("and") && words[3].is_word("toughness") && words[4].is_word("to"))
        && base > 1
        && let Some((power, toughness, duration)) = parse_base_pair_assignment_values(&tokens[base + 5..])
    {
        return Ok(Some(BasePowerToughnessClauseShape {
            power, toughness, target_tokens: &tokens[1..base], duration, where_x_tokens: None,
        }));
    }
    let imperative = primitives::parse_prefix(tokens,
        primitives::phrase(&["change", "the", "base", "power", "and", "toughness", "of"]))
        .map(|(_, rest)| (rest, true));
    let copular = primitives::parse_prefix(tokens,
        primitives::phrase(&["the", "base", "power", "and", "toughness", "of"]))
        .map(|(_, rest)| (rest, false));
    if let Some((rest, imperative)) = imperative.or(copular)
    {
        let Some(to) = rest.iter().rposition(|token| if imperative { token.is_word("to") }
            else { token.is_any_word(&["become", "becomes"]) }) else { return Ok(None); };
        let Some((power, toughness, duration)) = parse_base_pair_assignment_values(&rest[to + 1..])
            else { return Ok(None); };
        if to == 0 { return Ok(None); }
        return Ok(Some(BasePowerToughnessClauseShape {
            power, toughness, target_tokens: &rest[..to], duration, where_x_tokens: None,
        }));
    }
    let Some((subject, rest)) = split_subject_and_rest(tokens) else {
        return Ok(None);
    };
    let Some((_, modifier_tokens)) = primitives::parse_prefix(
        rest,
        primitives::phrase(&["base", "power", "and", "toughness"]),
    ) else {
        return Ok(None);
    };
    let Some(modifier_token) = modifier_tokens.first() else {
        return Ok(None);
    };
    let (power, toughness) = leaf::parse_leaf_pt_modifier_values_complete(
        modifier_token.parser_text(),
    )
    .map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            render_token_slice(tokens)
        ))
    })?;
    let tail = trim_edge_punctuation_tokens(modifier_tokens.get(1..).unwrap_or_default());
    let (target_tokens, leading_duration) = target_and_leading_duration(subject);
    let mut where_x_tokens = None;
    let duration = if tail.is_empty() {
        if !permits_unqualified_duration(subject, tokens) {
            return Ok(None);
        }
        leading_duration.unwrap_or(Until::Forever)
    } else if has_shared_gain_tail(tail)
        || primitives::parse_prefix(
            tail,
            alt((
                primitives::phrase(&["and", "becomes"]),
                primitives::phrase(&["and", "become"]),
            )),
        )
        .is_some()
    {
        // "... has base power and toughness 5/5 and becomes a Dinosaur in
        // addition to its other creature types" (Allosaurus Shepherd) is a
        // shared-subject coordination owned by the gain/become chain.
        return Ok(None);
    } else if primitives::parse_prefix(tail, primitives::phrase(&["where", "x", "is"])).is_some() {
        if !permits_unqualified_duration(subject, tokens) {
            return Ok(None);
        }
        where_x_tokens = Some(tail);
        leading_duration.unwrap_or(Until::Forever)
    } else if let Some(trailing_duration) = complete_duration(tail) {
        if leading_duration
            .as_ref()
            .is_some_and(|leading| leading != &trailing_duration)
        {
            return Err(CardTextError::ParseError(format!(
                "conflicting base power/toughness durations (clause: '{}')",
                render_token_slice(tokens)
            )));
        }
        trailing_duration
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power/toughness clause (clause: '{}')",
            render_token_slice(tokens)
        )));
    };
    Ok(Some(BasePowerToughnessClauseShape {
        power,
        toughness,
        target_tokens,
        duration,
        where_x_tokens,
    }))
}
