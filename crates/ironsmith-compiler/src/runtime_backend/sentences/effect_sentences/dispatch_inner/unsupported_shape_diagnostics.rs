pub(crate) fn parse_gain_life_equal_to_power_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    let Some(gain_idx) = find_index(words.as_slice(), |word| matches!(*word, "gain" | "gains"))
    else {
        return Ok(None);
    };

    if words.get(gain_idx + 1) != Some(&"life")
        || words.get(gain_idx + 2) != Some(&"equal")
        || words.get(gain_idx + 3) != Some(&"to")
    {
        return Ok(None);
    }

    let tail = &words[gain_idx + 4..];
    let has_its_power = contains_word_window(tail, &["its", "power"]);
    if !has_its_power {
        return Ok(None);
    }

    let subject = if gain_idx > 0 {
        Some(parse_subject(&tokens[..gain_idx]))
    } else {
        None
    };
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let amount = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));
    Ok(Some(vec![EffectAst::GainLife { amount, player }]))
}

pub(crate) fn parse_prevent_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    effect_grammar::parse_prevent_damage_sentence_lexed(tokens)
}

pub(crate) fn parse_gain_x_plus_life_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    let Some(gain_idx) = find_index(words.as_slice(), |word| matches!(*word, "gain" | "gains"))
    else {
        return Ok(None);
    };
    if gain_idx > 0
        && matches!(
            words[gain_idx - 1],
            "cant" | "cannot" | "doesnt" | "don't" | "dont"
        )
    {
        return Ok(None);
    }

    if words.len() <= gain_idx + 4 {
        return Ok(None);
    }

    if words[gain_idx + 1] != "x" || words[gain_idx + 2] != "plus" {
        return Ok(None);
    }

    let (bonus, number_used) = parse_number(&tokens[gain_idx + 3..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life gain amount (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let life_idx = gain_idx + 3 + number_used;
    if !tokens
        .get(life_idx)
        .is_some_and(|token| token.is_word("life"))
    {
        return Err(CardTextError::ParseError(format!(
            "missing life keyword in gain-x-plus-life clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let subject_tokens = &tokens[..gain_idx];
    let player = match parse_subject(subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let trailing_tokens = trim_commas(&tokens[life_idx + 1..]);
    let x_value = if trailing_tokens.is_empty() {
        Value::X
    } else if let Some(where_x) = parse_where_x_value_clause(&trailing_tokens) {
        where_x
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported gain-x-plus-life trailing clause (clause: '{}')",
            words.join(" ")
        )));
    };
    let amount = Value::Add(Box::new(x_value), Box::new(Value::Fixed(bonus as i32)));
    let effects = vec![EffectAst::GainLife { amount, player }];

    Ok(Some(effects))
}
