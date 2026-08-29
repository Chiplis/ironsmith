use super::*;

pub fn parse_base_power_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<BasePowerClauseShape<'_>>, CardTextError> {
    let Some((subject, rest)) = split_subject_and_rest(tokens) else {
        return Ok(None);
    };
    let Some((_, value_tokens)) = primitives::parse_prefix(
        rest,
        (primitives::kw("base"), primitives::kw("power")).void(),
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

pub fn parse_base_power_toughness_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<BasePowerToughnessClauseShape<'_>>, CardTextError> {
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
    } else if has_shared_gain_tail(tail) {
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
