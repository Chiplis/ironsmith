use crate::cards::TextSpan;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, OwnedLexToken, PlayerAst, PredicateAst,
    SubjectAst, TargetAst,
};
use crate::effect::EventValueSpec;
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;
use crate::{ChooseSpec, CounterType, TagKey, Value};

use super::super::activation_and_restrictions::parse_devotion_value_from_add_clause;
use super::super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_dynamic_cost_modifier_value,
};
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    parse_counter_type_from_tokens, parse_counter_type_word, parse_number, parse_target_phrase,
    parse_value, span_from_tokens, trim_commas, words,
};
use super::super::value_helpers::{
    parse_equal_to_aggregate_filter_value, parse_equal_to_number_of_filter_value,
};
use super::conditionals::parse_predicate;

fn parse_create_for_each_dynamic_count(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = words(tokens);
    if clause_words.starts_with(&["creature", "that", "died", "this", "turn"])
        || clause_words.starts_with(&["creatures", "that", "died", "this", "turn"])
    {
        return Some(Value::CreaturesDiedThisTurn);
    }
    if (clause_words.contains(&"spell") || clause_words.contains(&"spells"))
        && (clause_words.contains(&"cast") || clause_words.contains(&"casts"))
        && clause_words.contains(&"turn")
    {
        let player = if clause_words
            .iter()
            .any(|word| matches!(*word, "you" | "your" | "youve"))
        {
            PlayerFilter::You
        } else if clause_words
            .iter()
            .any(|word| matches!(*word, "opponent" | "opponents"))
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };

        let other_than_first = clause_words
            .windows(4)
            .any(|window| window == ["other", "than", "the", "first"]);
        if other_than_first {
            return Some(Value::Add(
                Box::new(Value::SpellsCastThisTurn(player)),
                Box::new(Value::Fixed(-1)),
            ));
        }
        if clause_words.contains(&"this") && clause_words.contains(&"turn") {
            return Some(Value::SpellsCastThisTurn(player));
        }
    }
    if clause_words.starts_with(&[
        "color", "of", "mana", "spent", "to", "cast", "this", "spell",
    ]) || clause_words.starts_with(&[
        "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
    ]) || clause_words
        .starts_with(&["color", "of", "mana", "used", "to", "cast", "this", "spell"])
        || clause_words.starts_with(&[
            "colors", "of", "mana", "used", "to", "cast", "this", "spell",
        ])
    {
        return Some(Value::ColorsOfManaSpentToCastThisSpell);
    }
    if clause_words.starts_with(&["basic", "land", "type", "among", "lands", "you", "control"])
        || clause_words.starts_with(&["basic", "land", "types", "among", "lands", "you", "control"])
        || clause_words.starts_with(&[
            "basic", "land", "type", "among", "the", "lands", "you", "control",
        ])
        || clause_words.starts_with(&[
            "basic", "land", "types", "among", "the", "lands", "you", "control",
        ])
    {
        return Some(Value::BasicLandTypesAmong(
            ObjectFilter::land().you_control(),
        ));
    }
    None
}

pub(crate) fn describe_counter_type_for_mode(counter_type: CounterType) -> String {
    counter_type.description().into_owned()
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

pub(crate) fn parse_counter_descriptor(
    tokens: &[OwnedLexToken],
) -> Result<(u32, CounterType), CardTextError> {
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

fn parse_referential_counter_count_value(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
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
        if !matches!(
            words_all.get(idx + 1).copied(),
            Some("counter" | "counters")
        ) {
            return None;
        }
        idx += 2;
        Some(counter_type)
    } else {
        return None;
    };

    Some((Value::CountersOn(Box::new(source_spec), counter_type), idx))
}

fn parse_put_counter_count_value(
    tokens: &[OwnedLexToken],
) -> Result<(Value, usize), CardTextError> {
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
        CardTextError::ParseError(format!("missing counter amount (clause: '{}')", clause))
    })
}

fn target_from_counter_source_spec(spec: &ChooseSpec, span: Option<TextSpan>) -> Option<TargetAst> {
    match spec {
        ChooseSpec::Source => Some(TargetAst::Source(span)),
        ChooseSpec::Tagged(tag) => Some(TargetAst::Tagged(tag.clone(), span)),
        ChooseSpec::Target(inner) => target_from_counter_source_spec(inner, span),
        _ => None,
    }
}

pub(crate) fn target_object_filter_mut(target: &mut TargetAst) -> Option<&mut ObjectFilter> {
    match target {
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::WithCount(inner, _) => target_object_filter_mut(inner),
        _ => None,
    }
}

fn merge_it_match_filter_into_target(target: &mut TargetAst, it_filter: &ObjectFilter) -> bool {
    if let TargetAst::Tagged(tag, span) = target {
        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        *target = TargetAst::Object(filter, span.clone(), None);
    }

    let Some(filter) = target_object_filter_mut(target) else {
        return false;
    };
    if !it_filter.card_types.is_empty() {
        filter.card_types = it_filter.card_types.clone();
    }
    if !it_filter.subtypes.is_empty() {
        filter.subtypes = it_filter.subtypes.clone();
    }
    if let Some(power) = &it_filter.power {
        filter.power = Some(power.clone());
        filter.power_reference = it_filter.power_reference;
    }
    if let Some(toughness) = &it_filter.toughness {
        filter.toughness = Some(toughness.clone());
        filter.toughness_reference = it_filter.toughness_reference;
    }
    if let Some(mana_value) = &it_filter.mana_value {
        filter.mana_value = Some(mana_value.clone());
    }
    true
}

pub(crate) fn parse_put_counters(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
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
    if let Some(equal_idx) = target_tokens
        .iter()
        .position(|token| token.is_word("equal"))
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
        return Ok(wrap_conditional(EffectAst::MoveAllCounters {
            from,
            to: target,
        }));
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
    tokens: &[OwnedLexToken],
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
    if first_desc.iter().any(|token| token.is_comma())
        || second_desc.iter().any(|token| token.is_comma())
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

fn parse_put_or_remove_counter_choice(
    put_count: u32,
    put_counter_type: CounterType,
    target_tokens: &[OwnedLexToken],
    clause_tokens: &[OwnedLexToken],
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
    let remove_mode_text = sentence_case_mode_text(&words(&remove_tokens).join(" "));

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
    tokens: &[OwnedLexToken],
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
        return Ok(Some((ChoiceCount::dynamic_x(), idx + 1)));
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
            while tokens.get(pos).is_some_and(OwnedLexToken::is_comma) {
                pos += 1;
            }
            if tokens.get(pos).is_some_and(|token| token.is_word("or")) {
                pos += 1;
                while tokens.get(pos).is_some_and(OwnedLexToken::is_comma) {
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
                    up_to_x: false,
                    random: false,
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

pub(crate) fn split_until_source_leaves_tail(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    let Some(until_idx) = tokens.iter().rposition(|token| token.is_word("until")) else {
        return (tokens, false);
    };
    if until_idx == 0 {
        return (tokens, false);
    }
    let tail_words = words(&tokens[until_idx + 1..]);
    let has_source_leaves_tail = tail_words.len() >= 3
        && tail_words[tail_words.len() - 3] == "leaves"
        && tail_words[tail_words.len() - 2] == "the"
        && tail_words[tail_words.len() - 1] == "battlefield";
    if has_source_leaves_tail {
        (&tokens[..until_idx], true)
    } else {
        (tokens, false)
    }
}

fn player_filter_for_set_life_total_reference(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::ThatPlayerOrTargetController
        | PlayerAst::ItsController
        | PlayerAst::ItsOwner => None,
    }
}

pub(crate) fn parse_half_starting_life_total_value(
    tokens: &[OwnedLexToken],
    player: PlayerAst,
) -> Option<Value> {
    let clause_words = words(tokens);
    let inferred_player_filter = || match clause_words.as_slice() {
        ["half", "your", "starting", "life", "total"]
        | ["half", "your", "starting", "life", "total", "rounded", "up"]
        | [
            "half",
            "your",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => Some(PlayerFilter::You),
        ["half", "target", "players", "starting", "life", "total"]
        | [
            "half",
            "target",
            "players",
            "starting",
            "life",
            "total",
            "rounded",
            "up",
        ]
        | [
            "half",
            "target",
            "players",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => Some(PlayerFilter::target_player()),
        ["half", "an", "opponents", "starting", "life", "total"]
        | [
            "half",
            "an",
            "opponents",
            "starting",
            "life",
            "total",
            "rounded",
            "up",
        ]
        | [
            "half",
            "an",
            "opponents",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => Some(PlayerFilter::Opponent),
        _ => None,
    };
    let player_filter =
        player_filter_for_set_life_total_reference(player).or_else(inferred_player_filter)?;

    let rounded_up = match clause_words.as_slice() {
        ["half", "your", "starting", "life", "total"]
        | ["half", "your", "starting", "life", "total", "rounded", "up"] => {
            player_filter == PlayerFilter::You
        }
        ["half", "target", "players", "starting", "life", "total"]
        | [
            "half",
            "target",
            "players",
            "starting",
            "life",
            "total",
            "rounded",
            "up",
        ] => player_filter == PlayerFilter::target_player(),
        ["half", "an", "opponents", "starting", "life", "total"]
        | [
            "half",
            "an",
            "opponents",
            "starting",
            "life",
            "total",
            "rounded",
            "up",
        ] => player_filter == PlayerFilter::Opponent,
        _ => false,
    };
    if rounded_up {
        return Some(Value::HalfStartingLifeTotalRoundedUp(player_filter));
    }

    let rounded_down = match clause_words.as_slice() {
        [
            "half",
            "your",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => player_filter == PlayerFilter::You,
        [
            "half",
            "target",
            "players",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => player_filter == PlayerFilter::target_player(),
        [
            "half",
            "an",
            "opponents",
            "starting",
            "life",
            "total",
            "rounded",
            "down",
        ] => player_filter == PlayerFilter::Opponent,
        _ => false,
    };
    if rounded_down {
        return Some(Value::HalfStartingLifeTotalRoundedDown(player_filter));
    }

    None
}

pub(crate) fn parse_transform(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::Transform {
            target: TargetAst::Source(None),
        });
    }
    let target_words = words(tokens);
    if target_words == ["it"]
        || target_words == ["this"]
        || target_words == ["this", "creature"]
        || target_words == ["this", "land"]
        || target_words == ["this", "permanent"]
    {
        return Ok(EffectAst::Transform {
            target: TargetAst::Source(span_from_tokens(tokens)),
        });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Transform { target })
}

fn exile_subject_owner_filter(subject: Option<SubjectAst>) -> Option<PlayerFilter> {
    match subject {
        Some(SubjectAst::Player(PlayerAst::Target)) => Some(PlayerFilter::target_player()),
        Some(SubjectAst::Player(PlayerAst::TargetOpponent)) => {
            Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)))
        }
        Some(SubjectAst::Player(PlayerAst::That)) => Some(PlayerFilter::IteratedPlayer),
        Some(SubjectAst::Player(PlayerAst::You)) => Some(PlayerFilter::You),
        _ => None,
    }
}

fn apply_exile_subject_owner_context(filter: &mut ObjectFilter, subject: Option<SubjectAst>) {
    let Some(owner_filter) = exile_subject_owner_filter(subject) else {
        return;
    };
    let direct_zone_ok = matches!(
        filter.zone,
        Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
    );
    let any_of_zone_ok = filter.any_of.iter().any(|nested| {
        matches!(
            nested.zone,
            Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
        )
    });
    if !direct_zone_ok && !any_of_zone_ok {
        return;
    }
    match filter.owner {
        Some(PlayerFilter::Target(_)) | Some(PlayerFilter::IteratedPlayer) | None => {
            filter.owner = Some(owner_filter);
        }
        _ => {}
    }
}

pub(crate) fn apply_exile_subject_hand_owner_context(
    target: &mut TargetAst,
    subject: Option<SubjectAst>,
) {
    let Some(filter) = target_object_filter_mut(target) else {
        return;
    };
    if filter.zone != Some(Zone::Hand) {
        return;
    }
    apply_exile_subject_owner_context(filter, subject);
}

pub(crate) fn apply_shuffle_subject_graveyard_owner_context(
    target: &mut TargetAst,
    subject: SubjectAst,
) {
    let Some(filter) = target_object_filter_mut(target) else {
        return;
    };
    if filter.zone != Some(Zone::Graveyard) {
        return;
    }

    let owner_filter = match subject {
        SubjectAst::Player(PlayerAst::Target) => Some(PlayerFilter::target_player()),
        SubjectAst::Player(PlayerAst::TargetOpponent) => Some(PlayerFilter::target_opponent()),
        SubjectAst::Player(PlayerAst::You) => Some(PlayerFilter::You),
        _ => None,
    };
    let Some(owner_filter) = owner_filter else {
        return;
    };

    match filter.owner {
        Some(PlayerFilter::IteratedPlayer) | Some(PlayerFilter::Target(_)) | None => {
            filter.owner = Some(owner_filter);
        }
        _ => {}
    }
}
