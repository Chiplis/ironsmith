use super::*;

pub(crate) fn parse_counter_type_word(word: &str) -> Option<CounterType> {
    match word {
        "+1/+1" => Some(CounterType::PlusOnePlusOne),
        "-1/-1" => Some(CounterType::MinusOneMinusOne),
        "-0/-1" => Some(CounterType::MinusOneMinusOne),
        "+1/+0" => Some(CounterType::PlusOnePlusZero),
        "+0/+1" => Some(CounterType::PlusZeroPlusOne),
        "+1/+2" => Some(CounterType::PlusOnePlusTwo),
        "+2/+2" => Some(CounterType::PlusTwoPlusTwo),
        "-0/-2" => Some(CounterType::MinusZeroMinusTwo),
        "-2/-2" => Some(CounterType::MinusTwoMinusTwo),
        "deathtouch" => Some(CounterType::Deathtouch),
        "flying" => Some(CounterType::Flying),
        "haste" => Some(CounterType::Haste),
        "hexproof" => Some(CounterType::Hexproof),
        "indestructible" => Some(CounterType::Indestructible),
        "lifelink" => Some(CounterType::Lifelink),
        "menace" => Some(CounterType::Menace),
        "reach" => Some(CounterType::Reach),
        "trample" => Some(CounterType::Trample),
        "vigilance" => Some(CounterType::Vigilance),
        "loyalty" => Some(CounterType::Loyalty),
        "charge" => Some(CounterType::Charge),
        "stun" => Some(CounterType::Stun),
        "depletion" => Some(CounterType::Depletion),
        "storage" => Some(CounterType::Storage),
        "ki" => Some(CounterType::Ki),
        "energy" => Some(CounterType::Energy),
        "age" => Some(CounterType::Age),
        "finality" => Some(CounterType::Finality),
        "time" => Some(CounterType::Time),
        "brain" => Some(CounterType::Brain),
        "level" => Some(CounterType::Level),
        "lore" => Some(CounterType::Lore),
        "oil" => Some(CounterType::Oil),
        _ => None,
    }
}

pub(crate) fn intern_counter_name(word: &str) -> &'static str {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    // Card parsing happens across many cards; deduplicate to avoid leaking one allocation per card.
    static INTERNER: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

    let map = INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().expect("counter name interner lock poisoned");
    if let Some(existing) = map.get(word) {
        return *existing;
    }

    let leaked: &'static str = Box::leak(word.to_string().into_boxed_str());
    map.insert(word.to_string(), leaked);
    leaked
}

pub(crate) fn parse_counter_type_from_tokens(tokens: &[Token]) -> Option<CounterType> {
    let token_words = words(tokens);

    if let Some(counter_idx) = token_words
        .iter()
        .position(|word| *word == "counter" || *word == "counters")
    {
        if counter_idx == 0 {
            return None;
        }

        let prev = token_words[counter_idx - 1];
        if let Some(counter_type) = parse_counter_type_word(prev) {
            return Some(counter_type);
        }

        if prev == "strike" && counter_idx >= 2 {
            match token_words[counter_idx - 2] {
                "double" => return Some(CounterType::DoubleStrike),
                "first" => return Some(CounterType::FirstStrike),
                _ => {}
            }
        }

        // "a counter of that kind" doesn't name the counter type.
        if matches!(
            prev,
            "a" | "an" | "one" | "two" | "three" | "four" | "five" | "six" | "another"
        ) {
            return None;
        }

        if prev.chars().all(|c| c.is_ascii_alphabetic()) {
            return Some(CounterType::Named(intern_counter_name(prev)));
        }
    }

    None
}

pub(crate) fn describe_counter_type_for_mode(counter_type: CounterType) -> String {
    match counter_type {
        CounterType::PlusOnePlusOne => "+1/+1".to_string(),
        CounterType::MinusOneMinusOne => "-1/-1".to_string(),
        CounterType::PlusOnePlusZero => "+1/+0".to_string(),
        CounterType::PlusZeroPlusOne => "+0/+1".to_string(),
        CounterType::PlusOnePlusTwo => "+1/+2".to_string(),
        CounterType::PlusTwoPlusTwo => "+2/+2".to_string(),
        CounterType::MinusZeroMinusTwo => "-0/-2".to_string(),
        CounterType::MinusTwoMinusTwo => "-2/-2".to_string(),
        CounterType::Named(name) => name.to_string(),
        other => format!("{other:?}").to_ascii_lowercase(),
    }
}

pub(crate) fn describe_counter_phrase_for_mode(count: u32, counter_type: CounterType) -> String {
    let counter_name = describe_counter_type_for_mode(counter_type);
    if count == 1 {
        format!("a {counter_name} counter")
    } else {
        format!("{count} {counter_name} counters")
    }
}

pub(crate) fn sentence_case_mode_text(text: &str) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    out.extend(chars);
    out
}

pub(crate) fn parse_counter_descriptor(tokens: &[Token]) -> Result<(u32, CounterType), CardTextError> {
    let descriptor = trim_commas(tokens);
    let (count, used) = parse_number(&descriptor).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter amount (clause: '{}')",
            words(&descriptor).join(" ")
        ))
    })?;
    let rest = &descriptor[used..];
    if !rest
        .iter()
        .any(|token| token.is_word("counter") || token.is_word("counters"))
    {
        return Err(CardTextError::ParseError(format!(
            "missing counter keyword (clause: '{}')",
            words(&descriptor).join(" ")
        )));
    }
    let counter_type = parse_counter_type_from_tokens(rest).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type (clause: '{}')",
            words(&descriptor).join(" ")
        ))
    })?;
    Ok((count, counter_type))
}

pub(crate) fn parse_referential_counter_count_value(tokens: &[Token]) -> Option<(Value, usize)> {
    let words_all = words(tokens);
    if words_all.is_empty() {
        return None;
    }

    let (source_spec, mut idx): (ChooseSpec, usize) = if words_all.starts_with(&["its"])
        || words_all.starts_with(&["those"])
        || words_all.starts_with(&["thiss"])
    {
        (ChooseSpec::Tagged(TagKey::from(IT_TAG)), 1)
    } else if words_all.starts_with(&["this"]) {
        (ChooseSpec::Source, 1)
    } else {
        return None;
    };

    let Some(word) = words_all.get(idx).copied() else {
        return None;
    };

    let counter_type = if word == "counter" || word == "counters" {
        idx += 1;
        None
    } else if let Some(counter_type) = parse_counter_type_word(word) {
        if !matches!(words_all.get(idx + 1).copied(), Some("counter" | "counters")) {
            return None;
        }
        idx += 2;
        Some(counter_type)
    } else {
        return None;
    };

    Some((Value::CountersOn(Box::new(source_spec), counter_type), idx))
}

pub(crate) fn parse_put_counter_count_value(tokens: &[Token]) -> Result<(Value, usize), CardTextError> {
    let clause = words(tokens).join(" ");
    let words_all = words(tokens);

    if words_all.starts_with(&["that", "many"]) || words_all.starts_with(&["that", "much"]) {
        return Ok((Value::EventValue(EventValueSpec::Amount), 2));
    }
    if words_all.starts_with(&["another"]) {
        return Ok((Value::Fixed(1), 1));
    }
    if let Some((value, used)) = parse_referential_counter_count_value(tokens) {
        return Ok((value, used));
    }
    if words_all.starts_with(&["a", "number", "of"]) {
        if let Some(value) = parse_add_mana_equal_amount_value(tokens)
            .or_else(|| parse_equal_to_aggregate_filter_value(tokens))
            .or_else(|| parse_equal_to_number_of_filter_value(tokens))
        {
            return Ok((value, 3));
        }
        if let Some(value) = parse_devotion_value_from_add_clause(tokens)? {
            return Ok((value, 3));
        }
        if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
            return Ok((value, 3));
        }
        return Err(CardTextError::ParseError(format!(
            "missing counter amount (clause: '{}')",
            clause
        )));
    }

    parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter amount (clause: '{}')",
            clause
        ))
    })
}

pub(crate) fn target_from_counter_source_spec(spec: &ChooseSpec, span: Option<TextSpan>) -> Option<TargetAst> {
    match spec {
        ChooseSpec::Source => Some(TargetAst::Source(span)),
        ChooseSpec::Tagged(tag) => Some(TargetAst::Tagged(tag.clone(), span)),
        ChooseSpec::Target(inner) => target_from_counter_source_spec(inner, span),
        _ => None,
    }
}

pub(crate) fn parse_put_counters(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let (count_value, used) = parse_put_counter_count_value(tokens)?;

    let rest = &tokens[used..];
    let on_idx = rest
        .iter()
        .position(|token| token.is_word("on"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter target (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;

    let mut target_tokens = rest[on_idx + 1..].to_vec();
    if let Some(equal_idx) = target_tokens.iter().position(|token| token.is_word("equal"))
        && target_tokens
            .get(equal_idx + 1)
            .is_some_and(|token| token.is_word("to"))
        && equal_idx > 0
    {
        target_tokens = trim_commas(&target_tokens[..equal_idx]);
    }
    let mut trailing_predicate: Option<PredicateAst> = None;
    if let Some(if_idx) = target_tokens.iter().position(|token| token.is_word("if")) {
        let predicate_tokens = trim_commas(&target_tokens[if_idx + 1..]);
        if !predicate_tokens.is_empty() {
            trailing_predicate = Some(parse_predicate(&predicate_tokens)?);
            target_tokens = trim_commas(&target_tokens[..if_idx]);
        }
    }
    while target_tokens
        .last()
        .is_some_and(|token| token.is_word("instead"))
    {
        target_tokens.pop();
    }

    let wrap_conditional = |effect: EffectAst| {
        if let Some(predicate) = trailing_predicate.clone() {
            EffectAst::Conditional {
                predicate,
                if_true: vec![effect],
                if_false: Vec::new(),
            }
        } else {
            effect
        }
    };

    let counter_type = if let Some(counter_type) = parse_counter_type_from_tokens(rest) {
        counter_type
    } else if let Value::CountersOn(_, Some(counter_type)) = &count_value {
        *counter_type
    } else if let Value::CountersOn(spec, None) = &count_value {
        let target = parse_target_phrase(&target_tokens)?;
        let from = target_from_counter_source_spec(spec.as_ref(), span_from_tokens(tokens))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported counter source reference (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;
        return Ok(wrap_conditional(EffectAst::MoveAllCounters { from, to: target }));
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter type (clause: '{}')",
            words(tokens).join(" ")
        )));
    };

    if let Value::Fixed(fixed_count) = count_value
        && fixed_count >= 0
        && let Some(mut effect) = parse_put_or_remove_counter_choice(
            fixed_count as u32,
            counter_type,
            &target_tokens,
            tokens,
        )?
    {
        let mut predicate = trailing_predicate.clone();
        if let Some(PredicateAst::ItMatches(filter)) = predicate.as_ref()
            && let EffectAst::PutOrRemoveCounters { target, .. } = &mut effect
            && merge_it_match_filter_into_target(target, filter)
        {
            predicate = None;
        }
        return Ok(if let Some(predicate) = predicate {
            EffectAst::Conditional {
                predicate,
                if_true: vec![effect],
                if_false: Vec::new(),
            }
        } else {
            effect
        });
    }

    if let Some((target_count, used)) = parse_counter_target_count_prefix(&target_tokens)? {
        let target_phrase = &target_tokens[used..];
        if target_phrase.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter target after count clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        let mut target = parse_target_phrase(target_phrase)?;
        let mut predicate = trailing_predicate.clone();
        if let Some(PredicateAst::ItMatches(filter)) = predicate.as_ref()
            && merge_it_match_filter_into_target(&mut target, filter)
        {
            predicate = None;
        }
        let effect = EffectAst::PutCounters {
            counter_type,
            count: count_value.clone(),
            target,
            target_count: Some(target_count),
            distributed: false,
        };
        return Ok(if let Some(predicate) = predicate {
            EffectAst::Conditional {
                predicate,
                if_true: vec![effect],
                if_false: Vec::new(),
            }
        } else {
            effect
        });
    }

    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("each"))
    {
        let filter = parse_object_filter(&target_tokens[1..], false)?;
        return Ok(wrap_conditional(EffectAst::PutCountersAll {
            counter_type,
            count: count_value,
            filter,
        }));
    }
    if let Some(for_each_idx) = (0..target_tokens.len().saturating_sub(1))
        .find(|idx| target_tokens[*idx].is_word("for") && target_tokens[*idx + 1].is_word("each"))
    {
        let base_target_tokens = trim_commas(&target_tokens[..for_each_idx]);
        let count_filter_tokens = trim_commas(&target_tokens[for_each_idx + 2..]);
        if !base_target_tokens.is_empty() && !count_filter_tokens.is_empty() {
            let mut target = parse_target_phrase(&base_target_tokens)?;
            let mut predicate = trailing_predicate.clone();
            if let Some(PredicateAst::ItMatches(filter)) = predicate.as_ref()
                && merge_it_match_filter_into_target(&mut target, filter)
            {
                predicate = None;
            }
            let mut count =
                if let Some(dynamic) = parse_create_for_each_dynamic_count(&count_filter_tokens) {
                    dynamic
                } else {
                    Value::Count(parse_object_filter(&count_filter_tokens, false)?)
                };
            if let Value::Fixed(multiplier) = count_value.clone()
                && multiplier > 1
            {
                let base = count.clone();
                for _ in 1..multiplier {
                    count = Value::Add(Box::new(count), Box::new(base.clone()));
                }
            }
            let effect = EffectAst::PutCounters {
                counter_type,
                count,
                target,
                target_count: None,
                distributed: false,
            };
            return Ok(if let Some(predicate) = predicate {
                EffectAst::Conditional {
                    predicate,
                    if_true: vec![effect],
                    if_false: Vec::new(),
                }
            } else {
                effect
            });
        }
    }
    let mut target = parse_target_phrase(&target_tokens)?;
    let mut predicate = trailing_predicate.clone();
    if let Some(PredicateAst::ItMatches(filter)) = predicate.as_ref()
        && merge_it_match_filter_into_target(&mut target, filter)
    {
        predicate = None;
    }
    let effect = EffectAst::PutCounters {
        counter_type,
        count: count_value,
        target,
        target_count: None,
        distributed: false,
    };
    Ok(if let Some(predicate) = predicate {
        EffectAst::Conditional {
            predicate,
            if_true: vec![effect],
            if_false: Vec::new(),
        }
    } else {
        effect
    })
}

pub(crate) fn parse_sentence_put_multiple_counters_on_target(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if !matches!(clause_words.first().copied(), Some("put") | Some("puts")) {
        return Ok(None);
    }

    let Some(on_idx) = tokens.iter().position(|token| token.is_word("on")) else {
        return Ok(None);
    };
    if on_idx < 2 {
        return Ok(None);
    }

    let before_on = trim_commas(&tokens[1..on_idx]);
    let Some(and_idx) = before_on.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if and_idx == 0 || and_idx + 1 >= before_on.len() {
        return Ok(None);
    }

    let first_desc = trim_commas(&before_on[..and_idx]);
    let second_desc = trim_commas(&before_on[and_idx + 1..]);
    if first_desc.is_empty() || second_desc.is_empty() {
        return Ok(None);
    }
    if first_desc
        .iter()
        .any(|token| matches!(token, Token::Comma(_)))
        || second_desc
            .iter()
            .any(|token| matches!(token, Token::Comma(_)))
    {
        return Ok(None);
    }
    let first_words = words(&first_desc);
    let second_words = words(&second_desc);
    if !first_words
        .iter()
        .any(|word| *word == "counter" || *word == "counters")
        || !second_words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }

    let (first_count, first_counter) = match parse_counter_descriptor(&first_desc) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let (second_count, second_counter) = match parse_counter_descriptor(&second_desc) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };

    let target_tokens = trim_commas(&tokens[on_idx + 1..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter target after on clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = words(&target_tokens);
    if !target_words
        .iter()
        .any(|word| *word == "target" || *word == "targets")
    {
        return Ok(None);
    }

    let target = parse_target_phrase(&target_tokens)?;
    if matches!(target, TargetAst::WithCount(_, _)) {
        return Ok(None);
    }

    let first_effect = EffectAst::PutCounters {
        counter_type: first_counter,
        count: Value::Fixed(first_count as i32),
        target: target.clone(),
        target_count: None,
        distributed: false,
    };
    let second_effect = EffectAst::PutCounters {
        counter_type: second_counter,
        count: Value::Fixed(second_count as i32),
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        target_count: None,
        distributed: false,
    };

    Ok(Some(vec![first_effect, second_effect]))
}

pub(crate) fn parse_put_or_remove_counter_choice(
    put_count: u32,
    put_counter_type: CounterType,
    target_tokens: &[Token],
    clause_tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(or_idx) = target_tokens
        .windows(2)
        .position(|window| window[0].is_word("or") && window[1].is_word("remove"))
    else {
        return Ok(None);
    };

    let base_target_tokens = trim_commas(&target_tokens[..or_idx]);
    if base_target_tokens.is_empty() {
        return Ok(None);
    }

    let remove_tokens = trim_commas(&target_tokens[or_idx + 1..]);
    if remove_tokens.len() < 2 || !remove_tokens[0].is_word("remove") {
        return Ok(None);
    }

    let mut idx = 1usize;
    let (remove_count, used_remove_count) =
        parse_value(&remove_tokens[idx..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter removal amount in put-or-remove clause (clause: '{}')",
                words(clause_tokens).join(" ")
            ))
        })?;
    idx += used_remove_count;

    let from_idx = remove_tokens[idx..]
        .iter()
        .position(|token| token.is_word("from"))
        .map(|offset| idx + offset)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing 'from' in put-or-remove clause (clause: '{}')",
                words(clause_tokens).join(" ")
            ))
        })?;

    let remove_descriptor_tokens = trim_commas(&remove_tokens[idx..from_idx]);
    let remove_counter_type = if remove_descriptor_tokens.is_empty() {
        put_counter_type
    } else {
        if !remove_descriptor_tokens
            .iter()
            .any(|token| token.is_word("counter") || token.is_word("counters"))
        {
            return Err(CardTextError::ParseError(format!(
                "missing counter keyword in put-or-remove remove clause (clause: '{}')",
                words(clause_tokens).join(" ")
            )));
        }
        parse_counter_type_from_tokens(&remove_descriptor_tokens).unwrap_or(put_counter_type)
    };

    let remove_target_tokens = trim_commas(&remove_tokens[from_idx + 1..]);
    let remove_target_words = words(&remove_target_tokens);
    let referential_remove_target = matches!(
        remove_target_words.as_slice(),
        ["it"]
            | ["that", "permanent"]
            | ["that", "artifact"]
            | ["that", "creature"]
            | ["that", "saga"]
            | ["this", "permanent"]
            | ["this", "artifact"]
            | ["this", "creature"]
    );
    if !referential_remove_target {
        return Err(CardTextError::ParseError(format!(
            "unsupported put-or-remove remove target (clause: '{}')",
            words(clause_tokens).join(" ")
        )));
    }

    let (target, target_count) = if let Some((target_count, used_target_count)) =
        parse_counter_target_count_prefix(&base_target_tokens)?
    {
        let target_phrase = &base_target_tokens[used_target_count..];
        if target_phrase.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter target before put-or-remove remove clause (clause: '{}')",
                words(clause_tokens).join(" ")
            )));
        }
        (parse_target_phrase(target_phrase)?, Some(target_count))
    } else {
        (parse_target_phrase(&base_target_tokens)?, None)
    };

    let target_phrase = words(&base_target_tokens).join(" ");
    let put_mode_text = format!(
        "Put {} on {}",
        describe_counter_phrase_for_mode(put_count, put_counter_type),
        target_phrase
    );
    let remove_mode_text = {
        let remove_text = words(&remove_tokens).join(" ");
        sentence_case_mode_text(&remove_text)
    };

    Ok(Some(EffectAst::PutOrRemoveCounters {
        put_counter_type,
        put_count: Value::Fixed(put_count as i32),
        remove_counter_type,
        remove_count,
        put_mode_text,
        remove_mode_text,
        target,
        target_count,
    }))
}

pub(crate) fn parse_counter_target_count_prefix(
    tokens: &[Token],
) -> Result<Option<(ChoiceCount, usize)>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut idx = 0usize;
    let mut each_prefix = false;

    if tokens[idx].is_word("each") {
        each_prefix = true;
        idx += 1;
        if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
            idx += 1;
        }
    }

    if each_prefix
        && tokens.get(idx).is_some_and(|token| token.is_word("x"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic target count 'each of X target' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    if each_prefix && tokens.get(idx).is_some_and(|token| token.is_word("target")) {
        return Ok(Some((ChoiceCount::any_number(), idx)));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number"))
    {
        idx += 2;
        if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
            idx += 1;
        }
        return Ok(Some((ChoiceCount::any_number(), idx)));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        let Some((value, used)) = parse_number(&tokens[idx + 2..]) else {
            return Err(CardTextError::ParseError(format!(
                "missing count after 'up to' in counter target clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        };
        idx += 2 + used;
        if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
            idx += 1;
        }
        return Ok(Some((ChoiceCount::up_to(value as usize), idx)));
    }

    if let Some((first, used_first)) = parse_number(&tokens[idx..]) {
        let mut pos = idx + used_first;
        let mut values = vec![first];
        loop {
            while matches!(tokens.get(pos), Some(Token::Comma(_))) {
                pos += 1;
            }
            if tokens.get(pos).is_some_and(|token| token.is_word("or")) {
                pos += 1;
                while matches!(tokens.get(pos), Some(Token::Comma(_))) {
                    pos += 1;
                }
            }

            let Some((next, used_next)) = parse_number(&tokens[pos..]) else {
                break;
            };
            values.push(next);
            pos += used_next;
        }

        if values.len() >= 2 {
            if tokens.get(pos).is_some_and(|token| token.is_word("of")) {
                pos += 1;
            }
            let min = values.iter().copied().min().unwrap_or(first) as usize;
            let max = values.iter().copied().max().unwrap_or(first) as usize;
            return Ok(Some((
                ChoiceCount {
                    min,
                    max: Some(max),
                    dynamic_x: false,
                },
                pos,
            )));
        }
    }

    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        idx += used;
        if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
            idx += 1;
        }
        return Ok(Some((ChoiceCount::exactly(value as usize), idx)));
    }

    Ok(None)
}

