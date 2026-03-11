use crate::cards::builders::effect_ast_traversal::for_each_nested_effects_mut;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, PlayerAst, PredicateAst, TagKey, TargetAst,
    Token, bind_implicit_player_context, find_verb, is_until_end_of_turn,
    negated_action_word_index, parse_effect_chain, parse_effect_chain_inner, parse_number,
    parse_object_filter, parse_pt_modifier, parse_pt_modifier_values,
    parse_target_count_range_prefix, parse_target_phrase, parse_value, parse_where_x_value_clause,
    remove_first_word, starts_with_until_end_of_turn, token_index_for_word_index, trim_commas,
    words,
};
use crate::effect::Until;
use crate::effect::Value;
use crate::target::{ObjectFilter, PlayerFilter};

pub(crate) fn parse_for_each_object_subject(
    subject_tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(subject_tokens);
    if subject_words.is_empty() {
        return Ok(None);
    }

    let mut filter_tokens = if subject_words.starts_with(&["for", "each"]) {
        &subject_tokens[2..]
    } else if subject_words.first() == Some(&"each") {
        &subject_tokens[1..]
    } else {
        return Ok(None);
    };
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("of"))
    {
        filter_tokens = &filter_tokens[1..];
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut normalized_filter_tokens: Vec<Token> = filter_tokens.to_vec();
    if let Some(attached_idx) = filter_tokens
        .iter()
        .position(|token| token.is_word("attached"))
        && filter_tokens
            .get(attached_idx + 1)
            .is_some_and(|token| token.is_word("to"))
        && attached_idx > 0
    {
        let attached_tail_words = words(&filter_tokens[attached_idx + 2..]);
        let attached_to_creature = attached_tail_words.starts_with(&["creature"])
            || attached_tail_words.starts_with(&["a", "creature"]);
        if attached_to_creature {
            normalized_filter_tokens = trim_commas(&filter_tokens[..attached_idx]);
        }
    }

    let filter_words = words(&normalized_filter_tokens);
    if filter_words.is_empty() {
        return Ok(None);
    }

    // Player-iteration forms are handled by dedicated ForEachPlayer/Opponent parsers.
    if filter_words.starts_with(&["player"])
        || filter_words.starts_with(&["players"])
        || filter_words.starts_with(&["opponent"])
        || filter_words.starts_with(&["opponents"])
        || filter_words.starts_with(&["target", "player"])
        || filter_words.starts_with(&["target", "players"])
        || filter_words.starts_with(&["target", "opponent"])
        || filter_words.starts_with(&["target", "opponents"])
    {
        return Ok(None);
    }

    Ok(Some(parse_object_filter(&normalized_filter_tokens, false)?))
}

pub(crate) fn parse_for_each_targeted_object_subject(
    subject_tokens: &[Token],
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(subject_tokens);
    if subject_words.is_empty() {
        return Ok(None);
    }

    let mut target_tokens = if subject_words.starts_with(&["for", "each"]) {
        &subject_tokens[2..]
    } else if subject_words.first() == Some(&"each") {
        &subject_tokens[1..]
    } else {
        return Ok(None);
    };
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("of"))
    {
        target_tokens = &target_tokens[1..];
    }
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let target = match parse_target_phrase(target_tokens) {
        Ok(target) => target,
        Err(_) => return Ok(None),
    };
    let TargetAst::WithCount(inner, count) = target else {
        return Ok(None);
    };
    let TargetAst::Object(filter, _, _) = *inner else {
        return Ok(None);
    };
    Ok(Some((filter, count)))
}

pub(crate) fn has_demonstrative_object_reference(words: &[&str]) -> bool {
    words.windows(2).any(|window| {
        matches!(
            window,
            ["that", "creature"]
                | ["that", "creatures"]
                | ["that", "permanent"]
                | ["that", "permanents"]
                | ["that", "artifact"]
                | ["that", "artifacts"]
                | ["that", "enchantment"]
                | ["that", "enchantments"]
                | ["that", "land"]
                | ["that", "lands"]
                | ["that", "card"]
                | ["that", "cards"]
                | ["that", "token"]
                | ["that", "tokens"]
                | ["that", "spell"]
                | ["that", "spells"]
                | ["those", "creatures"]
                | ["those", "permanents"]
                | ["those", "artifacts"]
                | ["those", "enchantments"]
                | ["those", "lands"]
                | ["those", "cards"]
                | ["those", "tokens"]
                | ["those", "spells"]
        )
    })
}

pub(crate) fn is_target_player_dealt_damage_by_this_turn_subject(words: &[&str]) -> bool {
    if words.len() < 8 {
        return false;
    }
    if !(words.starts_with(&["target", "player"]) || words.starts_with(&["target", "players"])) {
        return false;
    }
    words
        .windows(6)
        .any(|window| window == ["dealt", "damage", "by", "this", "creature", "this"])
        && words.windows(2).any(|window| window == ["this", "turn"])
}

pub(crate) fn is_mana_replacement_clause_words(words: &[&str]) -> bool {
    let has_if = words.contains(&"if");
    let has_tap = words.contains(&"tap") || words.contains(&"taps");
    let has_for_mana = words.windows(2).any(|window| window == ["for", "mana"]);
    let has_produce = words.contains(&"produce") || words.contains(&"produces");
    let has_instead = words.contains(&"instead");
    has_if && has_tap && has_for_mana && has_produce && has_instead
}

pub(crate) fn is_mana_trigger_additional_clause_words(words: &[&str]) -> bool {
    let has_whenever = words.contains(&"whenever");
    let has_tap = words.contains(&"tap") || words.contains(&"taps");
    let has_for_mana = words.windows(2).any(|window| window == ["for", "mana"]);
    let has_add = words.contains(&"add") || words.contains(&"adds");
    let has_additional = words.contains(&"additional");
    has_whenever && has_tap && has_for_mana && has_add && has_additional
}

pub(crate) fn parse_has_base_power_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words_all = words(tokens);
    let Some(has_idx) = words_all
        .iter()
        .position(|word| *word == "has" || *word == "have")
    else {
        return Ok(None);
    };
    let subject_tokens = &tokens[..has_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(subject_tokens);

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 3 || !rest_words.starts_with(&["base", "power"]) {
        return Ok(None);
    }
    if rest_words.get(2).is_some_and(|word| *word == "and") {
        return Ok(None);
    }

    let has_token_idx = tokens
        .iter()
        .position(|token| token.is_word("has") || token.is_word("have"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing has/have token in base-power clause (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
    let rest_tokens = &tokens[has_token_idx + 1..];

    let mut seen_words = 0usize;
    let mut value_token_idx = None;
    for (idx, token) in rest_tokens.iter().enumerate() {
        if token.as_word().is_some() {
            seen_words += 1;
            if seen_words == 3 {
                value_token_idx = Some(idx);
                break;
            }
        }
    }
    let Some(value_token_idx) = value_token_idx else {
        return Ok(None);
    };
    let (power, value_used) = parse_value(&rest_tokens[value_token_idx..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "invalid base power value (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let tail_words: Vec<&str> = rest_tokens[value_token_idx + value_used..]
        .iter()
        .filter_map(Token::as_word)
        .collect();
    if tail_words.is_empty() {
        let has_target_subject = subject_words.contains(&"target");
        let has_leading_until_eot = starts_with_until_end_of_turn(&subject_words);
        let has_temporal_words = words_all.windows(4).any(is_until_end_of_turn)
            || words_all
                .windows(2)
                .any(|window| window == ["this", "turn"] || window == ["next", "turn"]);
        if !has_target_subject && !has_leading_until_eot && !has_temporal_words {
            return Ok(None);
        }
    } else if !is_until_end_of_turn(tail_words.as_slice()) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let target_tokens: Vec<Token> = if starts_with_until_end_of_turn(&subject_words) {
        let mut skip_idx = 4usize;
        if subject_tokens
            .get(skip_idx)
            .is_some_and(|token| matches!(token, Token::Comma(_)))
        {
            skip_idx += 1;
        }
        trim_commas(&subject_tokens[skip_idx..]).to_vec()
    } else {
        subject_tokens.to_vec()
    };
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::SetBasePower {
        power,
        target,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_has_base_power_toughness_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words_all = words(tokens);
    let Some(has_idx) = words_all
        .iter()
        .position(|word| *word == "has" || *word == "have")
    else {
        return Ok(None);
    };
    let subject_tokens = &tokens[..has_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(subject_tokens);

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 5 || !rest_words.starts_with(&["base", "power", "and", "toughness"]) {
        return Ok(None);
    }

    let (power, toughness) = parse_pt_modifier(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let tail = &rest_words[5..];
    if tail.is_empty() {
        let has_target_subject = subject_words.contains(&"target");
        let has_leading_until_eot = starts_with_until_end_of_turn(&subject_words);
        let has_temporal_words = words_all.windows(4).any(is_until_end_of_turn)
            || words_all
                .windows(2)
                .any(|window| window == ["this", "turn"] || window == ["next", "turn"]);
        if !has_target_subject && !has_leading_until_eot && !has_temporal_words {
            return Ok(None);
        }
    }
    if !tail.is_empty() && !is_until_end_of_turn(tail) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power/toughness clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let target_tokens: Vec<Token> = if starts_with_until_end_of_turn(&subject_words) {
        let mut skip_idx = 4usize;
        if subject_tokens
            .get(skip_idx)
            .is_some_and(|token| matches!(token, Token::Comma(_)))
        {
            skip_idx += 1;
        }
        trim_commas(&subject_tokens[skip_idx..]).to_vec()
    } else {
        subject_tokens.to_vec()
    };
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::SetBasePowerToughness {
        power: Value::Fixed(power),
        toughness: Value::Fixed(toughness),
        target,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_get_for_each_count_value(
    tokens: &[Token],
) -> Result<Option<Value>, CardTextError> {
    let mut for_each_idx = None;
    for idx in 0..tokens.len().saturating_sub(1) {
        if tokens[idx].is_word("for") && tokens[idx + 1].is_word("each") {
            for_each_idx = Some(idx);
            break;
        }
    }

    let Some(idx) = for_each_idx else {
        return Ok(None);
    };

    let mut filter_tokens = &tokens[idx + 2..];
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing filter after 'for each' in gets clause".to_string(),
        ));
    }

    let mut other = false;
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("other") || token.is_word("another"))
    {
        other = true;
        filter_tokens = &filter_tokens[1..];
    }

    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing filter after 'for each' in gets clause".to_string(),
        ));
    }

    let filter_words = words(filter_tokens);
    if filter_words.starts_with(&["basic", "land", "type", "among"])
        || filter_words.starts_with(&["basic", "land", "types", "among"])
    {
        let mut scope_tokens = &filter_tokens[4..];
        if scope_tokens
            .first()
            .is_some_and(|token| token.is_word("the"))
        {
            scope_tokens = &scope_tokens[1..];
        }
        if scope_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "missing scope after 'basic land type among' in gets clause".to_string(),
            ));
        }
        let filter = parse_object_filter(scope_tokens, false)?;
        return Ok(Some(Value::BasicLandTypesAmong(filter)));
    }
    if filter_words.starts_with(&["color", "among"])
        || filter_words.starts_with(&["colors", "among"])
    {
        let mut scope_tokens = &filter_tokens[2..];
        if scope_tokens
            .first()
            .is_some_and(|token| token.is_word("the"))
        {
            scope_tokens = &scope_tokens[1..];
        }
        if scope_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "missing scope after 'color among' in gets clause".to_string(),
            ));
        }
        let filter = parse_object_filter(scope_tokens, false)?;
        return Ok(Some(Value::ColorsAmong(filter)));
    }

    Ok(Some(Value::Count(parse_object_filter(
        filter_tokens,
        other,
    )?)))
}

pub(crate) fn value_contains_unbound_x(value: &Value) -> bool {
    match value {
        Value::X | Value::XTimes(_) => true,
        Value::Scaled(value, _) => value_contains_unbound_x(value),
        Value::Add(left, right) => {
            value_contains_unbound_x(left) || value_contains_unbound_x(right)
        }
        _ => false,
    }
}

pub(crate) fn replace_unbound_x_with_value(
    value: Value,
    replacement: &Value,
    clause: &str,
) -> Result<Value, CardTextError> {
    match value {
        Value::X => Ok(replacement.clone()),
        Value::XTimes(multiplier) => {
            if multiplier == 1 {
                return Ok(replacement.clone());
            }
            if let Value::Fixed(fixed) = replacement {
                return Ok(Value::Fixed(fixed * multiplier));
            }
            Ok(Value::Scaled(Box::new(replacement.clone()), multiplier))
        }
        Value::Scaled(value, multiplier) => Ok(Value::Scaled(
            Box::new(replace_unbound_x_with_value(*value, replacement, clause)?),
            multiplier,
        )),
        Value::Add(left, right) => Ok(Value::Add(
            Box::new(replace_unbound_x_with_value(*left, replacement, clause)?),
            Box::new(replace_unbound_x_with_value(*right, replacement, clause)?),
        )),
        other => Ok(other),
    }
}

pub(crate) fn parse_get_modifier_values_with_tail(
    modifier_tokens: &[Token],
    power: Value,
    toughness: Value,
) -> Result<(Value, Value, Until, Option<crate::ConditionExpr>), CardTextError> {
    let clause = words(modifier_tokens).join(" ");
    let mut out_power = power;
    let mut out_toughness = toughness;
    let duration = Until::EndOfTurn;
    let mut condition = None;

    if modifier_tokens.is_empty() {
        return Ok((out_power, out_toughness, duration, condition));
    }

    let after_modifier = &modifier_tokens[1..];
    let after_modifier_words = words(after_modifier);
    let until_word_count = if starts_with_until_end_of_turn(&after_modifier_words) {
        4usize
    } else {
        0usize
    };
    let tail_start = token_index_for_word_index(after_modifier, until_word_count)
        .unwrap_or(after_modifier.len());
    let tail_tokens = trim_commas(&after_modifier[tail_start..]);

    if tail_tokens.is_empty() {
        return Ok((out_power, out_toughness, duration, condition));
    }

    let tail_words = words(&tail_tokens);
    // "gets +4/+4 until end of turn instead" appears in conditional replacement
    // branches where "instead" is grammatical glue, not an additional modifier.
    if tail_words.as_slice() == ["instead"] {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_words.starts_with(&["for", "as", "long", "as"])
        && tail_words.contains(&"this")
        && tail_words.contains(&"remains")
        && tail_words.contains(&"tapped")
    {
        condition = Some(crate::ConditionExpr::SourceIsTapped);
        return Ok((
            out_power,
            out_toughness,
            Until::ThisLeavesTheBattlefield,
            condition,
        ));
    }
    if tail_words == ["and", "must", "be", "blocked", "this", "turn", "if", "able"] {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_words == ["and", "cant", "be", "blocked", "this", "turn"] {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_words.first().copied() == Some("or")
        && let Some(alt_mod) = tail_words.get(1).copied()
        && parse_pt_modifier_values(alt_mod).is_ok()
    {
        let alt_tail = &tail_words[2..];
        if alt_tail.is_empty() || is_until_end_of_turn(alt_tail) {
            return Ok((out_power, out_toughness, duration, condition));
        }
    }
    if !tail_words.starts_with(&["where", "x", "is"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing gets clause (clause: '{}')",
            clause
        )));
    }

    if !value_contains_unbound_x(&out_power) && !value_contains_unbound_x(&out_toughness) {
        return Err(CardTextError::ParseError(format!(
            "where-X gets clause missing X modifier (clause: '{}')",
            clause
        )));
    }

    let x_value = parse_where_x_value_clause(&tail_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported where-X gets clause (clause: '{}')",
            clause
        ))
    })?;
    out_power = replace_unbound_x_with_value(out_power, &x_value, &clause)?;
    out_toughness = replace_unbound_x_with_value(out_toughness, &x_value, &clause)?;

    Ok((out_power, out_toughness, duration, condition))
}

pub(crate) fn force_implicit_token_controller_you(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::CreateToken { player, .. }
            | EffectAst::CreateTokenWithMods { player, .. }
            | EffectAst::CreateTokenCopy { player, .. }
            | EffectAst::CreateTokenCopyFromSource { player, .. } => {
                if matches!(player, PlayerAst::Implicit) {
                    *player = PlayerAst::You;
                }
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                force_implicit_token_controller_you(nested);
            }),
        }
    }
}

pub(crate) fn parse_for_each_opponent_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = words(clause_tokens);
    }
    if clause_words.len() < 2 {
        return Ok(None);
    }

    let start = if clause_words.starts_with(&["for", "each", "opponent"])
        || clause_words.starts_with(&["for", "each", "opponents"])
    {
        3
    } else if clause_words.starts_with(&["each", "opponent"])
        || clause_words.starts_with(&["each", "opponents"])
    {
        2
    } else {
        return Ok(None);
    };

    let mut inner_tokens = trim_commas(&clause_tokens[start..]).to_vec();
    let mut inner_words = words(&inner_tokens);
    let mut iteration_filter = PlayerFilter::Opponent;
    if inner_words.starts_with(&["other", "than", "defending", "player"]) {
        let strip_start =
            token_index_for_word_index(&inner_tokens, 4).unwrap_or(inner_tokens.len());
        inner_tokens = trim_commas(&inner_tokens[strip_start..]).to_vec();
        inner_words = words(&inner_tokens);
        iteration_filter = PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::Defending);
    }
    let wrap_for_each = |effects: Vec<EffectAst>| {
        if iteration_filter == PlayerFilter::Opponent {
            EffectAst::ForEachOpponent { effects }
        } else {
            EffectAst::ForEachPlayersFiltered {
                filter: iteration_filter.clone(),
                effects,
            }
        }
    };
    if inner_words.starts_with(&["who", "has", "less", "life", "than", "you"]) {
        let effect_start =
            token_index_for_word_index(&inner_tokens, 6).unwrap_or(inner_tokens.len());
        let effect_tokens = trim_commas(&inner_tokens[effect_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who has less life than you' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut branch_effects = if effect_tokens.iter().any(|token| token.is_word("may")) {
            let stripped = remove_first_word(&effect_tokens, "may");
            let inner_effects = parse_effect_chain_inner(&stripped)?;
            vec![EffectAst::May {
                effects: inner_effects,
            }]
        } else {
            parse_effect_chain(&effect_tokens)?
        };
        force_implicit_token_controller_you(&mut branch_effects);
        return Ok(Some(wrap_for_each(vec![EffectAst::Conditional {
            predicate: PredicateAst::PlayerHasLessLifeThanYou {
                player: PlayerAst::That,
            },
            if_true: branch_effects,
            if_false: Vec::new(),
        }])));
    }
    if inner_words.first().copied() == Some("who")
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start = if let Some(comma_idx) = inner_tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        {
            comma_idx + 1
        } else if let Some(this_way_idx) = inner_words
            .windows(2)
            .position(|pair| pair == ["this", "way"])
        {
            token_index_for_word_index(&inner_tokens, this_way_idx + 2)
                .unwrap_or(inner_tokens.len())
        } else {
            token_index_for_word_index(&inner_tokens, negation_idx + negation_len)
                .unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect in for each opponent who doesn't clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachOpponentDoesNot {
            effects,
            predicate,
        }));
    }

    if inner_words.first().copied() == Some("who")
        && let Some(this_way_idx) = inner_words
            .windows(2)
            .position(|window| window == ["this", "way"])
    {
        let effect_start = this_way_idx + 2;
        let effect_token_start =
            token_index_for_word_index(&inner_tokens, effect_start).unwrap_or(inner_tokens.len());
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who ... this way' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_who_did_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachOpponentDid { effects, predicate }));
    }
    if inner_words.starts_with(&["who", "does"])
        || inner_words.starts_with(&["who", "do"])
        || inner_words.starts_with(&["who", "did"])
    {
        let effect_token_start = if let Some(comma_idx) = inner_tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        {
            comma_idx + 1
        } else {
            token_index_for_word_index(&inner_tokens, 2).unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who does' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut effects = parse_effect_chain_inner(&effect_tokens)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, PlayerAst::You);
        }
        return Ok(Some(EffectAst::ForEachOpponentDid {
            effects,
            predicate: None,
        }));
    }

    let effects = if inner_tokens.iter().any(|token| token.is_word("may")) {
        let stripped = remove_first_word(&inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain(&inner_tokens)?
    };
    Ok(Some(wrap_for_each(effects)))
}

pub(crate) fn parse_for_each_target_players_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = words(clause_tokens);
    }
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let mut start = 0usize;
    let mut count = ChoiceCount::exactly(1);
    if clause_words.starts_with(&["any", "number", "of"]) {
        count = ChoiceCount::any_number();
        start = 3;
    } else if clause_words.starts_with(&["up", "to"])
        && let Some((value, used)) = parse_number(&clause_tokens[2..])
    {
        count = ChoiceCount::up_to(value as usize);
        start = 2 + used;
    } else if let Some((range_count, used)) = parse_target_count_range_prefix(clause_tokens) {
        count = range_count;
        start = used;
    } else if let Some((value, used)) = parse_number(clause_tokens)
        && clause_tokens
            .get(used)
            .is_some_and(|token| token.is_word("target"))
    {
        count = ChoiceCount::exactly(value as usize);
        start = used;
    }

    let Some(target_token) = clause_tokens.get(start) else {
        return Ok(None);
    };
    if !target_token.is_word("target") {
        return Ok(None);
    }
    if !clause_tokens
        .get(start + 1)
        .is_some_and(|token| token.is_word("player") || token.is_word("players"))
    {
        return Ok(None);
    }
    if !clause_tokens
        .get(start + 2)
        .is_some_and(|token| token.is_word("each"))
    {
        return Ok(None);
    }

    let inner_tokens = trim_commas(&clause_tokens[start + 3..]);
    if inner_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after target-player each clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = if inner_tokens.iter().any(|token| token.is_word("may")) {
        let stripped = remove_first_word(&inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain_inner(&inner_tokens)?
    };
    Ok(Some(EffectAst::ForEachTargetPlayers { count, effects }))
}

pub(crate) fn parse_who_did_this_way_predicate(
    inner_tokens: &[Token],
) -> Result<Option<PredicateAst>, CardTextError> {
    let inner_words = words(inner_tokens);
    if inner_words.first().copied() != Some("who") {
        return Ok(None);
    }
    let Some(this_way_idx) = inner_words
        .windows(2)
        .position(|window| window == ["this", "way"])
    else {
        return Ok(None);
    };
    let verb = inner_words.get(1).copied().unwrap_or("");
    let supports_tag = matches!(verb, "sacrificed" | "destroyed" | "exiled" | "discarded");
    if !supports_tag {
        return Ok(None);
    }
    if this_way_idx <= 2 {
        return Ok(None);
    }
    let filter_start = token_index_for_word_index(inner_tokens, 2).unwrap_or(inner_tokens.len());
    let filter_end =
        token_index_for_word_index(inner_tokens, this_way_idx).unwrap_or(inner_tokens.len());
    if filter_start >= filter_end {
        return Ok(None);
    }
    let filter_tokens = trim_commas(&inner_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };
    Ok(Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::That,
        tag: TagKey::from(IT_TAG),
        filter,
    }))
}

fn parse_negated_who_this_way_predicate(
    inner_tokens: &[Token],
) -> Result<Option<PredicateAst>, CardTextError> {
    let inner_words = words(inner_tokens);
    if inner_words.first().copied() != Some("who") {
        return Ok(None);
    }
    let Some(this_way_idx) = inner_words
        .windows(2)
        .position(|window| window == ["this", "way"])
    else {
        return Ok(None);
    };
    let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words) else {
        return Ok(None);
    };
    let verb_idx = negation_idx + negation_len;
    let verb = inner_words.get(verb_idx).copied().unwrap_or("");
    if !matches!(verb, "discard" | "discarded") || this_way_idx <= verb_idx + 1 {
        return Ok(None);
    }

    let filter_start =
        token_index_for_word_index(inner_tokens, verb_idx + 1).unwrap_or(inner_tokens.len());
    let filter_end =
        token_index_for_word_index(inner_tokens, this_way_idx).unwrap_or(inner_tokens.len());
    if filter_start >= filter_end {
        return Ok(None);
    }

    let filter_tokens = trim_commas(&inner_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    Ok(Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::That,
        tag: TagKey::from(IT_TAG),
        filter,
    }))
}

pub(crate) fn parse_for_each_player_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = words(clause_tokens);
    }
    if clause_words.len() < 2 {
        return Ok(None);
    }

    let start = if clause_words.starts_with(&["for", "each", "player"])
        || clause_words.starts_with(&["for", "each", "players"])
    {
        3
    } else if clause_words.starts_with(&["each", "player"])
        || clause_words.starts_with(&["each", "players"])
    {
        2
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&clause_tokens[start..]);
    if inner_tokens.len() > 3
        && inner_tokens[0].is_word("who")
        && inner_tokens[1].is_word("controls")
    {
        let mut effect_start = None;
        for idx in 2..inner_tokens.len() {
            if let Some(word) = inner_tokens[idx].as_word()
                && (word == "may"
                    || find_verb(&inner_tokens[idx..]).is_some_and(|(_, verb_idx)| verb_idx == 0))
            {
                effect_start = Some(idx);
                break;
            }
        }
        let effect_start = effect_start.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing effect clause after 'each player who controls' (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

        let filter_tokens = trim_commas(&inner_tokens[2..effect_start]);
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'each player who controls' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let filter_words = words(&filter_tokens);
        let (controls_most, normalized_filter_tokens) =
            if filter_words.starts_with(&["the", "most"]) {
                let start_idx =
                    token_index_for_word_index(&filter_tokens, 2).unwrap_or(filter_tokens.len());
                (true, trim_commas(&filter_tokens[start_idx..]))
            } else if filter_words.starts_with(&["most"]) {
                let start_idx =
                    token_index_for_word_index(&filter_tokens, 1).unwrap_or(filter_tokens.len());
                (true, trim_commas(&filter_tokens[start_idx..]))
            } else {
                (false, filter_tokens)
            };
        if normalized_filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter after 'most' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let filter = parse_object_filter(&normalized_filter_tokens, false)?;

        let effect_tokens = trim_commas(&inner_tokens[effect_start..]);
        let branch_effects = if effect_tokens.iter().any(|token| token.is_word("may")) {
            let stripped = remove_first_word(&effect_tokens, "may");
            let inner_effects = parse_effect_chain_inner(&stripped)?;
            vec![EffectAst::May {
                effects: inner_effects,
            }]
        } else {
            parse_effect_chain_inner(&effect_tokens)?
        };

        let predicate = if controls_most {
            PredicateAst::PlayerControlsMost {
                player: PlayerAst::That,
                filter,
            }
        } else {
            PredicateAst::PlayerControls {
                player: PlayerAst::That,
                filter,
            }
        };
        let effects = vec![EffectAst::Conditional {
            predicate,
            if_true: branch_effects,
            if_false: Vec::new(),
        }];
        return Ok(Some(EffectAst::ForEachPlayer { effects }));
    }

    let inner_words = words(&inner_tokens);
    if inner_words.first().copied() == Some("who") {
        let tapped_land_turn_idx = inner_words
            .windows(7)
            .position(|window| window == ["tapped", "a", "land", "for", "mana", "this", "turn"])
            .map(|idx| idx + 6)
            .or_else(|| {
                inner_words
                    .windows(6)
                    .position(|window| window == ["tapped", "land", "for", "mana", "this", "turn"])
                    .map(|idx| idx + 5)
            });
        if let Some(turn_idx) = tapped_land_turn_idx {
            let effect_token_start = token_index_for_word_index(&inner_tokens, turn_idx + 1)
                .unwrap_or(inner_tokens.len());
            let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
            if effect_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing effect after 'each player who tapped a land for mana this turn' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let branch_effects = if effect_tokens.iter().any(|token| token.is_word("may")) {
                let stripped = remove_first_word(&effect_tokens, "may");
                let inner_effects = parse_effect_chain_inner(&stripped)?;
                vec![EffectAst::May {
                    effects: inner_effects,
                }]
            } else {
                parse_effect_chain_inner(&effect_tokens)?
            };
            return Ok(Some(EffectAst::ForEachPlayer {
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::PlayerTappedLandForManaThisTurn {
                        player: PlayerAst::That,
                    },
                    if_true: branch_effects,
                    if_false: Vec::new(),
                }],
            }));
        }
    }
    if inner_words.first().copied() == Some("who")
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start = if let Some(comma_idx) = inner_tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        {
            comma_idx + 1
        } else if let Some(this_way_idx) = inner_words
            .windows(2)
            .position(|pair| pair == ["this", "way"])
        {
            token_index_for_word_index(&inner_tokens, this_way_idx + 2)
                .unwrap_or(inner_tokens.len())
        } else {
            token_index_for_word_index(&inner_tokens, negation_idx + negation_len)
                .unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect in for each player who doesn't clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDoesNot { effects, predicate }));
    }
    if inner_words.first().copied() == Some("who")
        && let Some(this_way_idx) = inner_words
            .windows(2)
            .position(|window| window == ["this", "way"])
    {
        let effect_start = this_way_idx + 2;
        let effect_token_start =
            token_index_for_word_index(&inner_tokens, effect_start).unwrap_or(inner_tokens.len());
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who ... this way' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_who_did_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDid { effects, predicate }));
    }
    if inner_words.starts_with(&["who", "does"])
        || inner_words.starts_with(&["who", "do"])
        || inner_words.starts_with(&["who", "did"])
    {
        let effect_token_start = if let Some(comma_idx) = inner_tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        {
            comma_idx + 1
        } else {
            token_index_for_word_index(&inner_tokens, 2).unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who does' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut effects = parse_effect_chain_inner(&effect_tokens)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, PlayerAst::You);
        }
        return Ok(Some(EffectAst::ForEachPlayerDid {
            effects,
            predicate: None,
        }));
    }

    let effects = if inner_tokens.iter().any(|token| token.is_word("may")) {
        let stripped = remove_first_word(&inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain_inner(&inner_tokens)?
    };

    Ok(Some(EffectAst::ForEachPlayer { effects }))
}
