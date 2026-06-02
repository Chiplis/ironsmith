use crate::cards::TextSpan;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, OwnedLexToken, PlayerAst, PredicateAst,
    SubjectAst, SubjectVerbActionAst, SubjectVerbEffectAst, TargetAst,
};
use crate::effect::EventValueSpec;
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;
use crate::{ChooseSpec, CounterType, TagKey, Value};
use ironsmith_core::{EffectMetric, EffectMetricSource};

use super::super::activation_and_restrictions::parse_devotion_value_from_add_clause;
use winnow::combinator::separated;
use winnow::prelude::*;

use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::split_trailing_if_clause_lexed;
use super::super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_dynamic_cost_modifier_value,
};
use super::super::lexer::{LexStream, TokenKind, contains_token_kind, token_slice_at_is};
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::{
    find_index as find_token_index, rfind_index as find_last_token_index,
};
use super::super::util::{
    parse_choice_count_token_prefix_consumed, parse_counter_type_from_tokens,
    parse_counter_type_word, parse_number, parse_target_phrase, parse_value,
    record_source_reference_surface, source_reference_surface_for_words, span_from_tokens,
    this_source_surface_for_words, trim_commas,
};
use super::super::value_helpers::{
    parse_equal_to_aggregate_filter_value, parse_equal_to_number_of_filter_value,
};
use super::clause_pattern_helpers::{ClauseShape, clause_shape};

type ZoneCounterCompatWords<'a> = TokenWordView<'a>;

const CREATURES_DIED_THIS_TURN_PREFIXES: &[&[&str]] = &[
    &["creature", "that", "died", "this", "turn"],
    &["creatures", "that", "died", "this", "turn"],
];

const REFERENTIAL_TAGGED_PREFIXES: &[&[&str]] = &[&["its"], &["those"], &["thiss"]];
const EVENT_AMOUNT_PREFIXES: &[&[&str]] = &[&["that", "many"], &["that", "much"]];
const SPELL_CAST_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(contains_any_words & [&["spell", "spells"], &["cast", "casts"]]; contains_words & ["turn"]);
const YOU_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["you"], &["your"], &["youve"]]);
const OPPONENT_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["opponent"], &["opponents"]]);
const THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["this", "turn"]);
const OTHER_THAN_THE_FIRST_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["other", "than", "the", "first"]);
const PUT_OR_PUTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["put"], &["puts"]]);
const COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const EQUAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["equal"]);
const EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["equal", "to"]);
const ON_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["on"]);
const HIM_OR_HER_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["him"], &["her"]]);
const INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const EACH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["each"]);
const TARGET_OR_TARGETS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["target"], &["targets"]]);
const REMOVE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["remove"]);
const FROM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["from"]);
const UNTIL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["until"]);
const POWER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["power"]);
const SOURCE_LEAVES_BATTLEFIELD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["leaves", "the", "battlefield"]);
const ROUNDED_DOWN_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["rounded", "down"]);
const ALL_OR_EACH_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["all"], &["each"]]);
const SELF_REFERENCE_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it"],
            &["this"],
            &["this", "creature"],
            &["this", "land"],
            &["this", "permanent"],
        ]
);
const HALF_YOUR_STARTING_LIFE_TOTAL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["half", "your", "starting", "life", "total"],
            &["half", "your", "starting", "life", "total", "rounded", "up"],
            &[
                "half", "your", "starting", "life", "total", "rounded", "down"
            ],
        ]
);
const HALF_TARGET_PLAYER_STARTING_LIFE_TOTAL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["half", "target", "players", "starting", "life", "total"],
            &[
                "half", "target", "players", "starting", "life", "total", "rounded", "up",
            ],
            &[
                "half", "target", "players", "starting", "life", "total", "rounded", "down",
            ],
        ]
);
const HALF_OPPONENT_STARTING_LIFE_TOTAL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["half", "an", "opponents", "starting", "life", "total"],
            &[
                "half",
                "an",
                "opponents",
                "starting",
                "life",
                "total",
                "rounded",
                "up",
            ],
            &[
                "half",
                "an",
                "opponents",
                "starting",
                "life",
                "total",
                "rounded",
                "down",
            ],
        ]
);

fn token_slice_matches_shape(tokens: &[OwnedLexToken], shape: &ClauseShape<'static>) -> bool {
    shape.matches_words(&ZoneCounterCompatWords::new(tokens).to_word_refs())
}

fn tokens_reference_objects_this_way(tokens: &[OwnedLexToken]) -> bool {
    grammar::words_find_phrase(tokens, &["this", "way"]).is_some()
        && (grammar::contains_word(tokens, "destroyed")
            || grammar::contains_word(tokens, "died")
            || grammar::contains_word(tokens, "exiled")
            || grammar::contains_word(tokens, "sacrificed")
            || grammar::contains_word(tokens, "discarded")
            || grammar::contains_word(tokens, "milled")
            || grammar::contains_word(tokens, "revealed"))
}

fn this_way_object_count_value() -> Value {
    Value::PendingEffectMetric {
        source: EffectMetricSource::AffectedObjects,
        metric: EffectMetric::Count,
    }
}

fn render_clause_words(tokens: &[OwnedLexToken]) -> String {
    ZoneCounterCompatWords::new(tokens).to_word_refs().join(" ")
}

fn parse_create_for_each_dynamic_count(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_word_view = ZoneCounterCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if grammar::words_match_any_prefix(tokens, CREATURES_DIED_THIS_TURN_PREFIXES).is_some() {
        return Some(Value::CreaturesDiedThisTurn);
    }
    if SPELL_CAST_THIS_TURN_PATTERN.matches_words(&clause_words) {
        let player = if clause_words
            .iter()
            .any(|word| YOU_REFERENCE_WORD_PATTERN.matches_word(word))
        {
            PlayerFilter::You
        } else if clause_words
            .iter()
            .any(|word| OPPONENT_REFERENCE_WORD_PATTERN.matches_word(word))
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };

        let other_than_first = OTHER_THAN_THE_FIRST_PATTERN
            .find_exact_window(&clause_words, 4)
            .is_some();
        if other_than_first {
            return Some(Value::Add(
                Box::new(Value::SpellsCastThisTurn(player)),
                Box::new(Value::Fixed(-1)),
            ));
        }
        if THIS_TURN_PATTERN.matches_words(&clause_words) {
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
    let (count, used) = if let Some((count, used)) = parse_number(&descriptor) {
        (count, used)
    } else if token_slice_at_is(&descriptor, 0, "a") || token_slice_at_is(&descriptor, 0, "an") {
        (1, 1)
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing counter amount (clause: '{}')",
            descriptor_text
        )));
    };
    let rest = &descriptor[used..];
    if !rest
        .iter()
        .any(|token| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_token(token))
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

    let counter_type = if COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word) {
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
        if grammar::words_find_phrase(tokens, &["equal", "to", "the", "difference"]).is_some()
            || grammar::words_find_phrase(tokens, &["equal", "to", "difference"]).is_some()
        {
            return Ok((Value::Fixed(0), 3));
        }
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
        if let Some(equal_idx) =
            find_token_index(tokens, |token| EQUAL_WORD_PATTERN.matches_token(token))
            && token_slice_matches_shape(&tokens[equal_idx..], &EQUAL_TO_PREFIX_PATTERN)
        {
            let value_tokens = trim_commas(&tokens[equal_idx + 2..]);
            if let Some((value, used)) = parse_value(&value_tokens)
                && used == value_tokens.len()
            {
                return Ok((value, 3));
            }
            if let Some(value) = parse_named_source_power_value(&value_tokens) {
                return Ok((value, 3));
            }
        }
        if let Some(equal_idx) =
            find_token_index(tokens, |token| EQUAL_WORD_PATTERN.matches_token(token))
            && token_slice_matches_shape(&tokens[equal_idx..], &EQUAL_TO_PREFIX_PATTERN)
            && let Some(on_idx) = find_token_index(&tokens[equal_idx + 2..], |token| {
                ON_WORD_PATTERN.matches_token(token)
            })
        {
            let value_tokens = trim_commas(&tokens[equal_idx + 2..equal_idx + 2 + on_idx]);
            if let Some((value, used)) = parse_value(&value_tokens)
                && used == value_tokens.len()
            {
                return Ok((value, 3));
            }
            if let Some(value) = parse_named_source_power_value(&value_tokens) {
                return Ok((value, 3));
            }
        }
        return Err(CardTextError::ParseError(format!(
            "missing counter amount (clause: '{}')",
            clause
        )));
    }

    if parse_counter_type_from_tokens(tokens).is_some()
        && let Some(on_idx) = find_token_index(tokens, |token| ON_WORD_PATTERN.matches_token(token))
    {
        let on_tail = trim_commas(&tokens[on_idx + 1..]);
        if let Some(equal_idx) =
            find_token_index(&on_tail, |token| EQUAL_WORD_PATTERN.matches_token(token))
            && token_slice_matches_shape(&on_tail[equal_idx..], &EQUAL_TO_PREFIX_PATTERN)
        {
            let value_tokens = trim_commas(&on_tail[equal_idx + 2..]);
            if let Some((value, used)) = parse_value(&value_tokens)
                && used == value_tokens.len()
            {
                return Ok((value, 0));
            }
            if let Some(value) = parse_named_source_power_value(&value_tokens) {
                return Ok((value, 0));
            }
        }
    }

    parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!("missing counter amount (clause: '{}')", clause))
    })
}

fn parse_named_source_power_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = TokenWordView::new(tokens).to_word_refs();
    if words.len() == 2 && POWER_WORD_PATTERN.matches_words(&words[1..]) && words[0].ends_with('s')
    {
        return Some(Value::PowerOf(Box::new(ChooseSpec::Source)));
    }
    None
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

pub(crate) fn merge_it_match_filter_into_target(
    target: &mut TargetAst,
    it_filter: &ObjectFilter,
) -> bool {
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
    if HIM_OR_HER_PATTERN.matches_words(&target_words) {
        return Ok(TargetAst::Source(span_from_tokens(tokens)));
    }
    parse_target_phrase(tokens)
}

pub(crate) fn parse_put_counters(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let (mut count_value, used) = parse_put_counter_count_value(tokens)?;
    let rest = &tokens[used..];
    let clause_text = render_clause_words(tokens);
    let on_idx =
        find_token_index(rest, |token| ON_WORD_PATTERN.matches_token(token)).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter target (clause: '{}')",
                clause_text
            ))
        })?;

    let mut target_tokens = rest[on_idx + 1..].to_vec();
    let mut equal_to_difference = false;
    if let Some(equal_idx) = find_token_index(&target_tokens, |token| {
        EQUAL_WORD_PATTERN.matches_token(token)
    }) && token_slice_matches_shape(&target_tokens[equal_idx..], &EQUAL_TO_PREFIX_PATTERN)
        && equal_idx > 0
    {
        let equal_words = ZoneCounterCompatWords::new(&target_tokens[equal_idx..]).to_word_refs();
        equal_to_difference = matches!(
            equal_words.as_slice(),
            ["equal", "to", "the", "difference"] | ["equal", "to", "difference"]
        );
        target_tokens = trim_commas(&target_tokens[..equal_idx]);
    }
    let mut trailing_predicate: Option<PredicateAst> = None;
    if let Some(spec) = split_trailing_if_clause_lexed(&target_tokens) {
        trailing_predicate = Some(spec.predicate);
        target_tokens = spec.leading_tokens.to_vec();
    }
    while target_tokens
        .last()
        .is_some_and(|token| INSTEAD_WORD_PATTERN.matches_token(token))
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
        return Ok(wrap_conditional(EffectAst::subject_verb_move_all_counters(
            from, target,
        )));
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
            && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutOrRemoveCounters { target, .. },
                ..
            }) = &mut effect
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
        let effect = EffectAst::subject_verb_put_counters(
            counter_type,
            count_value.clone(),
            target,
            Some(target_count),
            false,
        );
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
        .is_some_and(|token| EACH_WORD_PATTERN.matches_token(token))
    {
        let filter = parse_object_filter(&target_tokens[1..], false)?;
        return Ok(wrap_conditional(EffectAst::subject_verb_put_counters_all(
            counter_type,
            count_value,
            filter,
        )));
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
                } else if tokens_reference_objects_this_way(&count_filter_tokens) {
                    this_way_object_count_value()
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
            let effect =
                EffectAst::subject_verb_put_counters(counter_type, count, target, None, false);
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
    if equal_to_difference {
        let target_spec =
            crate::runtime_backend::references::reference_helpers::choose_spec_for_target(&target);
        count_value = Value::Add(
            Box::new(Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                IT_TAG,
            ))))),
            Box::new(Value::Scaled(
                Box::new(Value::PowerOf(Box::new(target_spec))),
                -1,
            )),
        );
    }
    let mut predicate = trailing_predicate.clone();
    if let Some(PredicateAst::ItMatches(filter)) = predicate.as_ref()
        && merge_it_match_filter_into_target(&mut target, filter)
    {
        predicate = None;
    }
    let effect =
        EffectAst::subject_verb_put_counters(counter_type, count_value, target, None, false);
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
    if !clause_words
        .first()
        .is_some_and(|word| PUT_OR_PUTS_WORD_PATTERN.matches_word(word))
    {
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
    if contains_token_kind(&first_desc, TokenKind::Comma)
        || contains_token_kind(&second_desc, TokenKind::Comma)
    {
        return Ok(None);
    }
    let first_word_view = ZoneCounterCompatWords::new(&first_desc);
    let first_words = first_word_view.to_word_refs();
    let second_word_view = ZoneCounterCompatWords::new(&second_desc);
    let second_words = second_word_view.to_word_refs();
    if !first_words
        .iter()
        .any(|word| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word))
        || !second_words
            .iter()
            .any(|word| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word))
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
        .any(|word| TARGET_OR_TARGETS_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let target = parse_target_phrase(&target_tokens)?;
    if matches!(target, TargetAst::WithCount(_, _)) {
        return Ok(None);
    }

    let first_effect = EffectAst::subject_verb_put_counters(
        first_counter,
        Value::Fixed(first_count as i32),
        target.clone(),
        None,
        false,
    );
    let second_effect = EffectAst::subject_verb_put_counters(
        second_counter,
        Value::Fixed(second_count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        None,
        false,
    );

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
    if remove_tokens.len() < 2 || !REMOVE_WORD_PATTERN.matches_token(&remove_tokens[0]) {
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

    let from_idx = find_token_index(&remove_tokens[idx..], |token| {
        FROM_WORD_PATTERN.matches_token(token)
    })
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
            .any(|token| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_token(token))
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

    Ok(Some(EffectAst::subject_verb_put_or_remove_counters(
        put_counter_type,
        Value::Fixed(put_count as i32),
        remove_counter_type,
        remove_count,
        put_mode_text,
        remove_mode_text,
        target,
        target_count,
    )))
}

pub(crate) fn parse_counter_target_count_prefix(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ChoiceCount, usize)>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut idx = 0usize;
    let mut each_prefix = false;

    if EACH_WORD_PATTERN.matches_token(&tokens[idx]) {
        each_prefix = true;
        idx += 1;
        if token_slice_at_is(tokens, idx, "of") {
            idx += 1;
        }
    }

    if each_prefix
        && token_slice_at_is(tokens, idx, "x")
        && token_slice_at_is(tokens, idx + 1, "target")
    {
        return Ok(Some((ChoiceCount::dynamic_x(), idx + 1)));
    }

    if each_prefix
        && token_slice_at_is(tokens, idx, "up")
        && token_slice_at_is(tokens, idx + 1, "to")
        && token_slice_at_is(tokens, idx + 2, "x")
        && token_slice_at_is(tokens, idx + 3, "target")
    {
        return Ok(Some((ChoiceCount::up_to_dynamic_x(), idx + 3)));
    }

    if each_prefix && token_slice_at_is(tokens, idx, "target") {
        return Ok(Some((ChoiceCount::any_number(), idx)));
    }

    if token_slice_at_is(tokens, idx, "any") && token_slice_at_is(tokens, idx + 1, "number") {
        let mut consumed = idx + 2;
        if token_slice_at_is(tokens, consumed, "of") {
            consumed += 1;
        }
        return Ok(Some((ChoiceCount::any_number(), consumed)));
    }

    if token_slice_at_is(tokens, idx, "up") && token_slice_at_is(tokens, idx + 1, "to") {
        let Some((count, used)) = parse_choice_count_token_prefix_consumed(&tokens[idx..]) else {
            return Err(CardTextError::ParseError(format!(
                "missing count after 'up to' in counter target clause (clause: '{}')",
                render_clause_words(tokens)
            )));
        };
        idx += used;
        if token_slice_at_is(tokens, idx, "of") {
            idx += 1;
        }
        return Ok(Some((count, idx)));
    }

    {
        let tail = &tokens[idx..];
        let mut stream = LexStream::new(tail);
        let mut sep_parser = separated(1.., grammar::number_token, grammar::comma_or_separator);
        if let Ok(values) = sep_parser.parse_next(&mut stream).map(|v: Vec<u32>| v) {
            let consumed = tail.len() - stream.len();
            let mut pos = idx + consumed;

            if values.len() >= 2 {
                if token_slice_at_is(tokens, pos, "of") {
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
        if token_slice_at_is(tokens, idx, "of") {
            idx += 1;
        }
        return Ok(Some((ChoiceCount::exactly(value as usize), idx)));
    }

    Ok(None)
}

pub(crate) fn split_until_source_leaves_tail(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    let Some(until_idx) =
        find_last_token_index(tokens, |token| UNTIL_WORD_PATTERN.matches_token(token))
    else {
        return (tokens, false);
    };
    if until_idx == 0 {
        return (tokens, false);
    }
    let tail_word_view = ZoneCounterCompatWords::new(&tokens[until_idx + 1..]);
    let tail_words = tail_word_view.to_word_refs();
    let has_source_leaves_tail = SOURCE_LEAVES_BATTLEFIELD_TAIL_PATTERN.matches_words(&tail_words);
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
        PlayerAst::NotYou => Some(PlayerFilter::NotYou),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Some(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Some(PlayerFilter::LowestLifeTied),
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
    let phrase_player_filter = if HALF_YOUR_STARTING_LIFE_TOTAL_PATTERN.matches_words(&clause_words)
    {
        Some(PlayerFilter::You)
    } else if HALF_TARGET_PLAYER_STARTING_LIFE_TOTAL_PATTERN.matches_words(&clause_words) {
        Some(PlayerFilter::target_player())
    } else if HALF_OPPONENT_STARTING_LIFE_TOTAL_PATTERN.matches_words(&clause_words) {
        Some(PlayerFilter::Opponent)
    } else {
        None
    };
    let inferred_player_filter = || phrase_player_filter.clone();
    let player_filter =
        player_filter_for_set_life_total_reference(player).or_else(inferred_player_filter)?;

    let phrase_matches_player = phrase_player_filter.as_ref() == Some(&player_filter);
    let rounded_up =
        phrase_matches_player && !ROUNDED_DOWN_TAIL_PATTERN.matches_words(&clause_words);
    if rounded_up {
        return Some(Value::HalfStartingLifeTotalRoundedUp(player_filter));
    }

    let rounded_down =
        phrase_matches_player && ROUNDED_DOWN_TAIL_PATTERN.matches_words(&clause_words);
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
    if ALL_OR_EACH_PREFIX_PATTERN.matches_words(&target_words) {
        let filter_tokens = &tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::ForEachObject {
            filter,
            effects: vec![action(TargetAst::Tagged(
                TagKey::from(IT_TAG),
                span_from_tokens(tokens),
            ))],
        });
    }
    if SELF_REFERENCE_TARGET_PATTERN.matches_words(&target_words) {
        let span = span_from_tokens(tokens);
        if let Some(surface) = this_source_surface_for_words(&target_words) {
            record_source_reference_surface(span, surface);
        }
        return Ok(action(TargetAst::Source(span)));
    }
    if let Some(surface) = source_reference_surface_for_words(&target_words)
        .or_else(|| this_source_surface_for_words(&target_words))
    {
        let span = span_from_tokens(tokens);
        record_source_reference_surface(span, surface);
        return Ok(action(TargetAst::Source(span)));
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
    parse_transform_like(tokens, EffectAst::subject_verb_transform)
}

pub(crate) fn parse_convert(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    parse_transform_like(tokens, EffectAst::subject_verb_convert)
}

pub(crate) fn exile_subject_owner_filter(subject: Option<SubjectAst>) -> Option<PlayerFilter> {
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

pub(crate) fn apply_exile_subject_owner_context(
    filter: &mut ObjectFilter,
    subject: Option<SubjectAst>,
) {
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
