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
use winnow::combinator::separated;
use winnow::prelude::*;

use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::split_trailing_if_clause_lexed;
use super::super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_dynamic_cost_modifier_value,
};
use super::super::lexer::LexStream;
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::{
    find_index as find_token_index, find_window_index as find_word_sequence_index,
    rfind_index as find_last_token_index, slice_contains_str as word_slice_contains,
    slice_ends_with as word_slice_ends_with, slice_starts_with as word_slice_starts_with,
};
use super::super::util::{
    parse_counter_type_from_tokens, parse_counter_type_word, parse_number, parse_target_phrase,
    parse_value, span_from_tokens, trim_commas,
};
use super::super::value_helpers::{
    parse_equal_to_aggregate_filter_value, parse_equal_to_number_of_filter_value,
};

type ZoneCounterCompatWords<'a> = TokenWordView<'a>;

const CREATURES_DIED_THIS_TURN_PREFIXES: &[&[&str]] = &[
    &["creature", "that", "died", "this", "turn"],
    &["creatures", "that", "died", "this", "turn"],
];

const REFERENTIAL_TAGGED_PREFIXES: &[&[&str]] = &[&["its"], &["those"], &["thiss"]];
const EVENT_AMOUNT_PREFIXES: &[&[&str]] = &[&["that", "many"], &["that", "much"]];

fn render_clause_words(tokens: &[OwnedLexToken]) -> String {
    ZoneCounterCompatWords::new(tokens).to_word_refs().join(" ")
}

fn parse_create_for_each_dynamic_count(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_word_view = ZoneCounterCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if grammar::words_match_any_prefix(tokens, CREATURES_DIED_THIS_TURN_PREFIXES).is_some() {
        return Some(Value::CreaturesDiedThisTurn);
    }
    if (word_slice_contains(&clause_words, "spell") || word_slice_contains(&clause_words, "spells"))
        && (word_slice_contains(&clause_words, "cast")
            || word_slice_contains(&clause_words, "casts"))
        && word_slice_contains(&clause_words, "turn")
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

        let other_than_first =
            find_word_sequence_index(&clause_words, &["other", "than", "the", "first"]).is_some();
        if other_than_first {
            return Some(Value::Add(
                Box::new(Value::SpellsCastThisTurn(player)),
                Box::new(Value::Fixed(-1)),
            ));
        }
        if word_slice_contains(&clause_words, "this") && word_slice_contains(&clause_words, "turn")
        {
            return Some(Value::SpellsCastThisTurn(player));
        }
    }
    if grammar::words_match_prefix(
        tokens,
        &[
            "color", "of", "mana", "spent", "to", "cast", "this", "spell",
        ],
    )
    .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["color", "of", "mana", "used", "to", "cast", "this", "spell"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "colors", "of", "mana", "used", "to", "cast", "this", "spell",
            ],
        )
        .is_some()
    {
        return Some(Value::ColorsOfManaSpentToCastThisSpell);
    }
    if grammar::words_match_prefix(
        tokens,
        &["basic", "land", "type", "among", "lands", "you", "control"],
    )
    .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["basic", "land", "types", "among", "lands", "you", "control"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "basic", "land", "type", "among", "the", "lands", "you", "control",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "basic", "land", "types", "among", "the", "lands", "you", "control",
            ],
        )
        .is_some()
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
    let descriptor_text = render_clause_words(&descriptor);
    let (count, used) = parse_number(&descriptor).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter amount (clause: '{}')",
            descriptor_text
        ))
    })?;
    let rest = &descriptor[used..];
    if !rest
        .iter()
        .any(|token| token.is_word("counter") || token.is_word("counters"))
    {
        return Err(CardTextError::ParseError(format!(
            "missing counter keyword (clause: '{}')",
            render_clause_words(&descriptor)
        )));
    }
    let counter_type = parse_counter_type_from_tokens(rest).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type (clause: '{}')",
            render_clause_words(&descriptor)
        ))
    })?;
    Ok((count, counter_type))
}

fn parse_referential_counter_count_value(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let words_view = ZoneCounterCompatWords::new(tokens);
    let words_all = words_view.to_word_refs();
    if words_all.is_empty() {
        return None;
    }

    let (source_spec, mut idx): (ChooseSpec, usize) = if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, REFERENTIAL_TAGGED_PREFIXES)
    {
        (ChooseSpec::Tagged(TagKey::from(IT_TAG)), prefix.len())
    } else if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, &[&["this"]]) {
        (ChooseSpec::Source, prefix.len())
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
    let clause = render_clause_words(tokens);

    if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, EVENT_AMOUNT_PREFIXES) {
        return Ok((Value::EventValue(EventValueSpec::Amount), prefix.len()));
    }
    if grammar::words_match_any_prefix(tokens, &[&["another"]]).is_some() {
        return Ok((Value::Fixed(1), 1));
    }
    if let Some((value, used)) = parse_referential_counter_count_value(tokens) {
        return Ok((value, used));
    }
    if grammar::words_match_any_prefix(tokens, &[&["a", "number", "of"]]).is_some() {
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

fn parse_counter_target_phrase(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    let target_word_view = ZoneCounterCompatWords::new(tokens);
    let target_words = target_word_view.to_word_refs();
    if matches!(target_words.as_slice(), ["him"] | ["her"]) {
        return Ok(TargetAst::Source(span_from_tokens(tokens)));
    }
    parse_target_phrase(tokens)
}

pub(crate) fn parse_put_counters(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let (count_value, used) = parse_put_counter_count_value(tokens)?;
    let rest = &tokens[used..];
    let clause_text = render_clause_words(tokens);
    let on_idx = find_token_index(rest, |token| token.is_word("on")).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter target (clause: '{}')",
            clause_text
        ))
    })?;

    let mut target_tokens = rest[on_idx + 1..].to_vec();
    if let Some(equal_idx) = find_token_index(&target_tokens, |token| token.is_word("equal"))
        && target_tokens
            .get(equal_idx + 1)
            .is_some_and(|token| token.is_word("to"))
        && equal_idx > 0
    {
        target_tokens = trim_commas(&target_tokens[..equal_idx]);
    }
    let mut trailing_predicate: Option<PredicateAst> = None;
    if let Some(spec) = split_trailing_if_clause_lexed(&target_tokens) {
        trailing_predicate = Some(spec.predicate);
        target_tokens = spec.leading_tokens.to_vec();
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
        let target = parse_counter_target_phrase(&target_tokens)?;
        let from = target_from_counter_source_spec(spec.as_ref(), span_from_tokens(tokens))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported counter source reference (clause: '{}')",
                    render_clause_words(tokens)
                ))
            })?;
        return Ok(wrap_conditional(EffectAst::MoveAllCounters {
            from,
            to: target,
        }));
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter type (clause: '{}')",
            render_clause_words(tokens)
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
                render_clause_words(tokens)
            )));
        }
        let mut target = parse_counter_target_phrase(target_phrase)?;
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
    let for_each_idx = grammar::find_prefix(&target_tokens, || grammar::phrase(&["for", "each"]))
        .map(|(idx, _, _)| idx);
    if let Some(for_each_idx) = for_each_idx {
        let base_target_tokens = trim_commas(&target_tokens[..for_each_idx]);
        let count_filter_tokens = trim_commas(&target_tokens[for_each_idx + 2..]);
        if !base_target_tokens.is_empty() && !count_filter_tokens.is_empty() {
            let mut target = parse_counter_target_phrase(&base_target_tokens)?;
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
    let mut target = parse_counter_target_phrase(&target_tokens)?;
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
    let clause_word_view = ZoneCounterCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if !matches!(clause_words.first().copied(), Some("put") | Some("puts")) {
        return Ok(None);
    }

    let Some((before_on_raw, _after_on)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(&tokens[1..], || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("on").void()
        })
    else {
        return Ok(None);
    };

    let before_on = trim_commas(before_on_raw);
    let Some((first_slice, second_slice)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(&before_on, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("and").void()
        })
    else {
        return Ok(None);
    };
    let first_desc = trim_commas(first_slice);
    let second_desc = trim_commas(second_slice);
    if first_desc.is_empty() || second_desc.is_empty() {
        return Ok(None);
    }
    if first_desc.iter().any(|token| token.is_comma())
        || second_desc.iter().any(|token| token.is_comma())
    {
        return Ok(None);
    }
    let first_word_view = ZoneCounterCompatWords::new(&first_desc);
    let first_words = first_word_view.to_word_refs();
    let second_word_view = ZoneCounterCompatWords::new(&second_desc);
    let second_words = second_word_view.to_word_refs();
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

    let target_tokens = trim_commas(_after_on);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter target after on clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_word_view = ZoneCounterCompatWords::new(&target_tokens);
    let target_words = target_word_view.to_word_refs();
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
    let or_idx = grammar::find_prefix(target_tokens, || grammar::phrase(&["or", "remove"]))
        .map(|(idx, _, _)| idx);
    let Some(or_idx) = or_idx else {
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
                render_clause_words(clause_tokens)
            ))
        })?;
    idx += used_remove_count;

    let from_idx = find_token_index(&remove_tokens[idx..], |token| token.is_word("from"))
        .map(|offset| idx + offset)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing 'from' in put-or-remove clause (clause: '{}')",
                render_clause_words(clause_tokens)
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
                render_clause_words(clause_tokens)
            )));
        }
        parse_counter_type_from_tokens(&remove_descriptor_tokens).unwrap_or(put_counter_type)
    };

    let remove_target_tokens = trim_commas(&remove_tokens[from_idx + 1..]);
    let remove_target_word_view = ZoneCounterCompatWords::new(&remove_target_tokens);
    let remove_target_words = remove_target_word_view.to_word_refs();
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
            render_clause_words(clause_tokens)
        )));
    }

    let (target, target_count) = if let Some((target_count, used_target_count)) =
        parse_counter_target_count_prefix(&base_target_tokens)?
    {
        let target_phrase = &base_target_tokens[used_target_count..];
        if target_phrase.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter target before put-or-remove remove clause (clause: '{}')",
                render_clause_words(clause_tokens)
            )));
        }
        (
            parse_counter_target_phrase(target_phrase)?,
            Some(target_count),
        )
    } else {
        (parse_counter_target_phrase(&base_target_tokens)?, None)
    };

    let target_phrase = render_clause_words(&base_target_tokens);
    let put_mode_text = format!(
        "Put {} on {}",
        describe_counter_phrase_for_mode(put_count, put_counter_type),
        target_phrase
    );
    let remove_mode_text = sentence_case_mode_text(&render_clause_words(&remove_tokens));

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

    if each_prefix
        && tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
        && tokens.get(idx + 2).is_some_and(|token| token.is_word("x"))
        && tokens
            .get(idx + 3)
            .is_some_and(|token| token.is_word("target"))
    {
        return Ok(Some((ChoiceCount::up_to_dynamic_x(), idx + 3)));
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
                render_clause_words(tokens)
            )));
        };
        idx += 2 + used;
        if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
            idx += 1;
        }
        return Ok(Some((ChoiceCount::up_to(value as usize), idx)));
    }

    {
        let tail = &tokens[idx..];
        let mut stream = LexStream::new(tail);
        let mut sep_parser = separated(1.., grammar::number_token, grammar::comma_or_separator);
        if let Ok(values) = sep_parser.parse_next(&mut stream).map(|v: Vec<u32>| v) {
            let consumed = tail.len() - stream.len();
            let mut pos = idx + consumed;

            if values.len() >= 2 {
                if tokens.get(pos).is_some_and(|token| token.is_word("of")) {
                    pos += 1;
                }
                let min = values.iter().copied().min().unwrap() as usize;
                let max = values.iter().copied().max().unwrap() as usize;
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
    let Some(until_idx) = find_last_token_index(tokens, |token| token.is_word("until")) else {
        return (tokens, false);
    };
    if until_idx == 0 {
        return (tokens, false);
    }
    let tail_word_view = ZoneCounterCompatWords::new(&tokens[until_idx + 1..]);
    let tail_words = tail_word_view.to_word_refs();
    let has_source_leaves_tail =
        word_slice_ends_with(&tail_words, &["leaves", "the", "battlefield"]);
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
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::ThatPlayerOrTargetController
        | PlayerAst::ItsController
        | PlayerAst::ItsOwner => None,
    }
}

pub(crate) fn parse_half_starting_life_total_value(
    tokens: &[OwnedLexToken],
    player: PlayerAst,
) -> Option<Value> {
    let clause_word_view = ZoneCounterCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
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

fn parse_transform_like(
    tokens: &[OwnedLexToken],
    action: fn(TargetAst) -> EffectAst,
) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(action(TargetAst::Source(None)));
    }
    let target_word_view = ZoneCounterCompatWords::new(tokens);
    let target_words = target_word_view.to_word_refs();
    if target_words == ["it"]
        || target_words == ["this"]
        || target_words == ["this", "creature"]
        || target_words == ["this", "land"]
        || target_words == ["this", "permanent"]
    {
        return Ok(action(TargetAst::Source(span_from_tokens(tokens))));
    }
    let target = match parse_target_phrase(tokens) {
        Ok(target) => target,
        Err(_)
            if target_words.len() <= 3
                && !target_words.iter().any(|word| {
                    matches!(
                        *word,
                        "target" | "another" | "other" | "each" | "all" | "that" | "those"
                    )
                }) =>
        {
            TargetAst::Source(span_from_tokens(tokens))
        }
        Err(err) => return Err(err),
    };
    Ok(action(target))
}

pub(crate) fn parse_transform(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    parse_transform_like(tokens, |target| EffectAst::Transform { target })
}

pub(crate) fn parse_convert(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    parse_transform_like(tokens, |target| EffectAst::Convert { target })
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
