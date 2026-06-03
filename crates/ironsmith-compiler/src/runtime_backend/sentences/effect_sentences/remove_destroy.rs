use super::*;
use crate::runtime_backend::effect_sentences::SubjectVerbPrimitiveClause;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use crate::runtime_backend::lexer::{
    token_slice_at_is, token_slice_first_is, token_slice_first_is_any, token_slice_starts_with,
};
use crate::runtime_backend::util::{
    parse_choice_count_before_target_prefix, parse_filter_counter_constraint_words,
};

const FROM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["from"]);
const COMBAT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["combat"]);
const COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const WITH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["with"]);
const AT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["at"]);
const COMBAT_HISTORY_DESTROY_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["was", "blocked"],
            &["was", "blocking"],
            &["blocking", "it"],
            &["blocked", "it"],
            &["it", "blocked"],
        ]]
);
const ATTACHED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["attached"]);
const TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const ATTACHED_SUPPORTED_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["you"], &["it"]]);
const ATTACHED_FILTER_TRAILING_BE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that"], &["were"], &["was"], &["is"], &["are"]]);
const ATTACHED_TIMING_TAIL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["at"],
            &["beginning"],
            &["end"],
            &["combat"],
            &["turn"],
            &["step"],
            &["until"],
        ]
);
const CHOSEN_THIS_WAY_SUFFIXES: &[&[&str]] = &[
    &["chosen", "this", "way"],
    &["that", "were", "chosen", "this", "way"],
    &["that", "was", "chosen", "this", "way"],
];
const CHOSEN_THIS_WAY_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any CHOSEN_THIS_WAY_SUFFIXES);
const EXCEPT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["except"]);
const IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const NO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["no"]);
const ALL_OR_EACH_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["all"], &["each"]]);
const TARGET_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["target"]);
const BLOCKED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["blocked"]);
const DEALT_DAMAGE_THIS_TURN_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["that", "was", "dealt", "damage", "this", "turn"]);
const DEALT_DAMAGE_THIS_TURN_FILTER_TAILS: &[&[&str]] = &[
    &["that", "was", "dealt", "damage", "this", "turn"],
    &["that", "were", "dealt", "damage", "this", "turn"],
];
const THAT_DEALT_DAMAGE_TO_PHRASE: &[&str] = &["that", "dealt", "damage", "to"];
const THAT_DEALT_DAMAGE_TO_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & THAT_DEALT_DAMAGE_TO_PHRASE);
const END_OF_COMBAT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["end", "of", "combat"]]);

fn remove_find_exact_phrase_shape(
    words: &[&str],
    phrase: &[&str],
    shape: &ClauseShape<'static>,
) -> Option<usize> {
    if phrase.is_empty() || words.len() < phrase.len() {
        return None;
    }
    words
        .windows(phrase.len())
        .position(|window| shape.matches_words(window))
}

pub(crate) fn parse_remove(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if matches!(words.as_slice(), ["all", "of", "them"]) {
        return Ok(EffectAst::subject_verb_remove_all_of_them_counters_from_source());
    }

    if let Some(from_idx) = find_index(tokens, |token| FROM_WORD_PATTERN.matches_token(token)) {
        let tail_words = crate::runtime_backend::token_word_refs(&tokens[from_idx + 1..]);
        if COMBAT_WORD_PATTERN.matches_words(&tail_words) {
            let target_tokens = trim_commas(&tokens[..from_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing remove-from-combat target (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
            let target = parse_target_phrase(&target_tokens)?;
            return Ok(EffectAst::subject_verb_remove_from_combat(target));
        }
    }

    if token_slice_first_is(tokens, "all")
        && let Some(counter_idx) = find_index(tokens, |token: &OwnedLexToken| {
            COUNTER_OR_COUNTERS_WORD_PATTERN.matches_token(token)
        })
        && counter_idx > 0
    {
        let counter_descriptor = trim_commas(&tokens[1..counter_idx]);
        let counter_type = parse_counter_type_from_descriptor_tokens(&counter_descriptor);
        let mut target_tokens = trim_commas(&tokens[counter_idx + 1..]);
        if token_slice_first_is(&target_tokens, "from") {
            target_tokens = trim_commas(&target_tokens[1..]);
        }

        let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
        let source_like_target = matches!(
            target_words.as_slice(),
            ["it"]
                | ["this"]
                | ["this", "creature"]
                | ["this", "artifact"]
                | ["this", "enchantment"]
                | ["this", "permanent"]
                | ["this", "card"]
        );
        let target = if source_like_target {
            TargetAst::Source(span_from_tokens(&target_tokens))
        } else {
            parse_target_phrase(&target_tokens)?
        };
        let amount = match (&target, counter_type) {
            (TargetAst::Source(_), Some(counter_type)) => Value::CountersOnSource(counter_type),
            (TargetAst::Source(_), None) => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            _ => Value::CountersOn(Box::new(ChooseSpec::Source), counter_type),
        };
        return Ok(EffectAst::subject_verb_remove_up_to_any_counters(
            amount,
            target,
            counter_type,
            false,
        ));
    }

    let mut idx = 0;
    let mut up_to = false;
    if token_slice_at_is(tokens, idx, "up") && token_slice_at_is(tokens, idx + 1, "to") {
        up_to = true;
        idx += 2;
    }

    let (amount, used) = parse_value(&tokens[idx..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter removal amount (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    idx += used;

    let counter_idx = find_index(&tokens[idx..], |token: &OwnedLexToken| {
        COUNTER_OR_COUNTERS_WORD_PATTERN.matches_token(token)
    })
    .map(|offset| idx + offset)
    .ok_or_else(|| CardTextError::ParseError("missing counter keyword".to_string()))?;
    let counter_descriptor = trim_commas(&tokens[idx..counter_idx]);
    let counter_type = parse_counter_type_from_descriptor_tokens(&counter_descriptor);
    if counter_idx >= tokens.len() {
        return Err(CardTextError::ParseError(
            "missing counter keyword".to_string(),
        ));
    }
    idx = counter_idx + 1;

    if token_slice_at_is(tokens, idx, "from") {
        idx += 1;
    }

    let target_tokens = trim_commas(&tokens[idx..]);
    if token_slice_first_is_any(&target_tokens, &["each", "all"]) {
        let filter = parse_object_filter(&target_tokens[1..], false)?;
        return Ok(EffectAst::subject_verb_remove_counters_all(
            amount,
            filter,
            counter_type,
            up_to,
        ));
    }

    let for_each_idx = find_window_by(&target_tokens, 2, |window: &[OwnedLexToken]| {
        token_slice_starts_with(window, &["for", "each"])
    });
    if let Some(for_each_idx) = for_each_idx {
        let base_target_tokens = trim_commas(&target_tokens[..for_each_idx]);
        let count_filter_tokens = trim_commas(&target_tokens[for_each_idx + 2..]);
        if !base_target_tokens.is_empty() && !count_filter_tokens.is_empty() {
            if let (Ok(target), Ok(count_filter)) = (
                parse_target_phrase(&base_target_tokens),
                parse_object_filter(&count_filter_tokens, false),
            ) {
                return Ok(EffectAst::ForEachObject {
                    filter: count_filter,
                    effects: vec![EffectAst::subject_verb_remove_up_to_any_counters(
                        amount,
                        target,
                        counter_type,
                        up_to,
                    )],
                });
            }
        }
    }

    let target_tokens = trim_commas(&tokens[idx..]);
    let target = parse_target_phrase(&target_tokens)?;

    Ok(EffectAst::subject_verb_remove_up_to_any_counters(
        amount,
        target,
        counter_type,
        up_to,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedDestroyTimingAst {
    EndOfCombat,
    NextEndStep,
}

pub(crate) fn parse_delayed_destroy_timing_words(
    words: &[&str],
) -> Option<DelayedDestroyTimingAst> {
    if matches!(
        words,
        ["at", "end", "of", "combat"] | ["at", "the", "end", "of", "combat"]
    ) {
        return Some(DelayedDestroyTimingAst::EndOfCombat);
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "end", "step"]
            | ["at", "beginning", "of", "the", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "the", "next", "end", "step"]
    ) {
        return Some(DelayedDestroyTimingAst::NextEndStep);
    }

    None
}

pub(crate) fn wrap_destroy_with_delayed_timing(
    effect: EffectAst,
    timing: Option<DelayedDestroyTimingAst>,
) -> EffectAst {
    let Some(timing) = timing else {
        return effect;
    };

    match timing {
        DelayedDestroyTimingAst::EndOfCombat => EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        },
        DelayedDestroyTimingAst::NextEndStep => EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![effect],
        },
    }
}

fn parse_destroy_all_filter(tokens: &[OwnedLexToken]) -> Result<ObjectFilter, CardTextError> {
    if let Some(with_idx) = find_index(tokens, |token: &OwnedLexToken| {
        WITH_WORD_PATTERN.matches_token(token)
    }) {
        let base_tokens = trim_commas(&tokens[..with_idx]);
        let tail_words = crate::runtime_backend::token_word_refs(&tokens[with_idx + 1..]);
        if !base_tokens.is_empty()
            && NO_WORD_PATTERN.matches_word_at(&tail_words, 0)
            && let Some((counter_constraint, consumed)) =
                parse_filter_counter_constraint_words(&tail_words[1..])
            && consumed == tail_words.len().saturating_sub(1)
        {
            let mut filter = parse_object_filter(&base_tokens, false)?;
            filter.without_counter = Some(counter_constraint);
            return Ok(filter);
        }
        if !base_tokens.is_empty()
            && let Some((counter_constraint, consumed)) =
                parse_filter_counter_constraint_words(&tail_words)
            && consumed == tail_words.len()
        {
            let mut filter = parse_object_filter(&base_tokens, false)?;
            filter.with_counter = Some(counter_constraint);
            return Ok(filter);
        }
    }

    parse_object_filter(tokens, false)
}

fn parse_destroy_all_combat_history_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(suffix_len) = DEALT_DAMAGE_THIS_TURN_FILTER_TAILS
        .iter()
        .find_map(|suffix| {
            (words.len() > suffix.len() && words[words.len() - suffix.len()..] == **suffix)
                .then_some(suffix.len())
        })
    else {
        return Ok(None);
    };

    let base_word_len = words.len() - suffix_len;
    let token_cutoff = token_index_for_word_index(tokens, base_word_len).unwrap_or(tokens.len());
    let base_tokens = trim_commas(&tokens[..token_cutoff]);
    if base_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_destroy_all_filter(&base_tokens)?;
    filter.was_dealt_damage_this_turn = true;
    Ok(Some(filter))
}

pub(crate) fn parse_destroy(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let original_clause_words = crate::runtime_backend::token_word_refs(tokens);
    let mut delayed_timing = None;
    let mut timing_cut_word_idx = original_clause_words.len();
    for word_idx in 0..original_clause_words.len() {
        if !AT_WORD_PATTERN.matches_word(original_clause_words[word_idx]) {
            continue;
        }
        if let Some(timing) = parse_delayed_destroy_timing_words(&original_clause_words[word_idx..])
        {
            delayed_timing = Some(timing);
            timing_cut_word_idx = word_idx;
            break;
        }
    }

    let core_tokens = if timing_cut_word_idx < original_clause_words.len() {
        let token_cutoff =
            token_index_for_word_index(tokens, timing_cut_word_idx).unwrap_or(tokens.len());
        trim_commas(&tokens[..token_cutoff])
    } else {
        trim_commas(tokens)
    };
    let clause_words = crate::runtime_backend::token_word_refs(&core_tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing destroy target before delayed timing clause (clause: '{}')",
            original_clause_words.join(" ")
        )));
    }

    if delayed_timing.is_none()
        && (END_OF_COMBAT_PATTERN.matches_words(&original_clause_words)
            || (grammar::contains_word(tokens, "beginning")
                && grammar::contains_word(tokens, "end")))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed destroy timing clause (clause: '{}')",
            original_clause_words.join(" ")
        )));
    }
    if let Some(target) = parse_destroy_combat_history_target(&core_tokens)? {
        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::subject_verb_destroy(target),
            delayed_timing,
        ));
    }
    if ALL_OR_EACH_WORD_PATTERN.matches_word_at(&clause_words, 0)
        && let Some(filter) = parse_destroy_all_combat_history_filter(&core_tokens[1..])?
    {
        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::subject_verb_destroy_all(filter),
            delayed_timing,
        ));
    }
    let has_combat_history = (grammar::contains_word(&core_tokens, "dealt")
        && grammar::contains_word(&core_tokens, "damage")
        && grammar::contains_word(&core_tokens, "turn"))
        || COMBAT_HISTORY_DESTROY_PATTERN.matches_words(&clause_words);
    if has_combat_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history destroy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if ALL_OR_EACH_WORD_PATTERN.matches_word_at(&clause_words, 0) {
        if let Some(attached_idx) = find_index(&core_tokens, |token: &OwnedLexToken| {
            ATTACHED_WORD_PATTERN.matches_token(token)
        }) && core_tokens
            .get(attached_idx + 1)
            .is_some_and(|token| TO_WORD_PATTERN.matches_token(token))
            && attached_idx > 1
        {
            let mut filter_tokens = trim_commas(&core_tokens[1..attached_idx]).to_vec();
            while filter_tokens
                .last()
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| ATTACHED_FILTER_TRAILING_BE_WORD_PATTERN.matches_word(word))
            {
                filter_tokens.pop();
            }
            let target_tokens = trim_commas(&core_tokens[attached_idx + 2..]);
            let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
            let has_timing_tail = target_words
                .iter()
                .any(|word| ATTACHED_TIMING_TAIL_WORD_PATTERN.matches_word(word));
            let supported_target = grammar::words_match_prefix(&target_tokens, &["target"])
                .is_some()
                || ATTACHED_SUPPORTED_TARGET_PATTERN.matches_words(&target_words)
                || grammar::words_match_any_prefix(&target_tokens, ATTACHED_REFERENCE_PREFIXES)
                    .is_some();
            if !filter_tokens.is_empty()
                && !target_tokens.is_empty()
                && supported_target
                && !has_timing_tail
            {
                let filter = parse_object_filter(&filter_tokens, false)?;
                let target = parse_target_phrase(&target_tokens)?;
                return Ok(wrap_destroy_with_delayed_timing(
                    EffectAst::subject_verb_destroy_all_attached_to(filter, target),
                    delayed_timing,
                ));
            }
        }
        if let Some(except_for_idx) =
            find_window_by(&core_tokens, 2, |window: &[OwnedLexToken]| {
                token_slice_starts_with(window, &["except", "for"])
            })
            && except_for_idx > 1
        {
            let base_filter_tokens = trim_commas(&core_tokens[1..except_for_idx]);
            let exception_tokens = trim_commas(&core_tokens[except_for_idx + 2..]);
            if !base_filter_tokens.is_empty() && !exception_tokens.is_empty() {
                let mut filter = parse_object_filter(&base_filter_tokens, false)?;
                let exception_filter = parse_object_filter(&exception_tokens, false)?;
                apply_except_filter_exclusions(&mut filter, &exception_filter);
                return Ok(wrap_destroy_with_delayed_timing(
                    EffectAst::subject_verb_destroy_all(filter),
                    delayed_timing,
                ));
            }
        }
        let filter_tokens = &core_tokens[1..];
        if let Some((choice_idx, consumed)) =
            find_color_choice_phrase(SubjectVerbPrimitiveClause::new(filter_tokens))
        {
            let base_filter_tokens = trim_commas(&filter_tokens[..choice_idx]);
            let trailing = trim_commas(&filter_tokens[choice_idx + consumed..]);
            if !trailing.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing color-choice destroy-all clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if base_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing destroy-all filter before color-choice clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let filter = parse_object_filter(&base_filter_tokens, false)?;
            return Ok(wrap_destroy_with_delayed_timing(
                EffectAst::subject_verb_destroy_all_of_chosen_color(filter, false),
                delayed_timing,
            ));
        }
        let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
        if CHOSEN_THIS_WAY_SUFFIX_PATTERN.matches_words(&filter_words)
            && let Some(suffix_len) = CHOSEN_THIS_WAY_SUFFIXES.iter().find_map(|suffix| {
                (filter_words.len() >= suffix.len()
                    && filter_words[filter_words.len() - suffix.len()..] == **suffix)
                    .then_some(suffix.len())
            })
        {
            let cutoff = filter_words.len() - suffix_len;
            let token_cutoff = if cutoff == 0 {
                0
            } else {
                token_index_for_word_index(filter_tokens, cutoff).unwrap_or(filter_tokens.len())
            };
            let base_filter_tokens = trim_commas(&filter_tokens[..token_cutoff]);
            let mut filter = parse_object_filter(&base_filter_tokens, false)?;
            let relation = if let Some(except_idx) = find_index(&base_filter_tokens, |token| {
                EXCEPT_WORD_PATTERN.matches_token(token)
            }) {
                let base_before_except = trim_commas(&base_filter_tokens[..except_idx]);
                if base_before_except.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing destroy-all filter before except clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                filter = parse_object_filter(&base_before_except, false)?;
                TaggedOpbjectRelation::IsNotTaggedObject
            } else {
                TaggedOpbjectRelation::IsTaggedObject
            };
            filter = filter.match_tagged(TagKey::from(IT_TAG), relation);
            return Ok(wrap_destroy_with_delayed_timing(
                EffectAst::subject_verb_destroy_all(filter),
                delayed_timing,
            ));
        }

        let filter = parse_destroy_all_filter(filter_tokens)?;
        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::subject_verb_destroy_all(filter),
            delayed_timing,
        ));
    }

    if grammar::contains_word(&core_tokens, "unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported destroy-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(if_idx) = find_index(&core_tokens, |token: &OwnedLexToken| {
        IF_WORD_PATTERN.matches_token(token)
    }) {
        let mut target_tokens = trim_commas(&core_tokens[..if_idx]).to_vec();
        while target_tokens
            .last()
            .is_some_and(|token| INSTEAD_WORD_PATTERN.matches_token(token))
        {
            target_tokens.pop();
        }
        let target_tokens = trim_commas(&target_tokens);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported conditional destroy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let target = parse_target_phrase(&target_tokens)?;
        let predicate_tail = parse_conditional_predicate_tail_lexed(&core_tokens[if_idx + 1..])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported conditional destroy clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;

        return Ok(match predicate_tail {
            ConditionalPredicateTailSpec::InsteadIf {
                base_predicate,
                outer_predicate,
            } => wrap_destroy_with_delayed_timing(
                EffectAst::Conditional {
                    predicate: outer_predicate,
                    if_true: vec![EffectAst::Conditional {
                        predicate: base_predicate,
                        if_true: vec![EffectAst::subject_verb_destroy(target.clone())],
                        if_false: Vec::new(),
                    }],
                    if_false: Vec::new(),
                },
                delayed_timing,
            ),
            ConditionalPredicateTailSpec::Plain(predicate) => wrap_destroy_with_delayed_timing(
                EffectAst::Conditional {
                    predicate,
                    if_true: vec![EffectAst::subject_verb_destroy(target)],
                    if_false: Vec::new(),
                },
                delayed_timing,
            ),
        });
    }
    if let Some(and_idx) = find_index(&core_tokens, |token: &OwnedLexToken| {
        AND_WORD_PATTERN.matches_token(token)
    }) {
        let tail_slice = &core_tokens[and_idx + 1..];
        let starts_multi_target = tail_slice
            .first()
            .is_some_and(|t| TARGET_WORD_PATTERN.matches_token(t))
            || parse_choice_count_before_target_prefix(tail_slice).is_some();
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target destroy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if grammar::words_match_any_prefix(&core_tokens, TARGET_BLOCKED_PREFIXES).is_some() {
        let mut target_tokens = core_tokens.to_vec();
        if let Some(blocked_idx) = find_index(&target_tokens, |token: &OwnedLexToken| {
            BLOCKED_WORD_PATTERN.matches_token(token)
        }) {
            target_tokens.remove(blocked_idx);
        }
        let target = parse_target_phrase(&target_tokens)?;
        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::Conditional {
                predicate: PredicateAst::TargetIsBlocked,
                if_true: vec![EffectAst::subject_verb_destroy(target)],
                if_false: Vec::new(),
            },
            delayed_timing,
        ));
    }

    let target = parse_target_phrase(&core_tokens)?;
    Ok(wrap_destroy_with_delayed_timing(
        EffectAst::subject_verb_destroy(target),
        delayed_timing,
    ))
}

pub(crate) fn parse_destroy_combat_history_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if let Some(target) =
        parse_destroy_target_dealt_damage_to_player_this_turn(tokens, &clause_words)?
    {
        return Ok(Some(target));
    }
    let Some(that_idx) = DEALT_DAMAGE_THIS_TURN_TAIL_PATTERN.find_exact_window(&clause_words, 6)
    else {
        return Ok(None);
    };
    if that_idx == 0 || that_idx + 6 != clause_words.len() {
        return Ok(None);
    }
    let target_cutoff = token_index_for_word_index(tokens, that_idx).unwrap_or(tokens.len());
    let target_tokens = trim_commas(&tokens[..target_cutoff]);
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let target = parse_target_phrase(&target_tokens)?;
    let TargetAst::Object(mut filter, target_span, it_span) = target else {
        return Ok(None);
    };
    filter.was_dealt_damage_this_turn = true;
    Ok(Some(TargetAst::Object(filter, target_span, it_span)))
}

fn parse_destroy_target_dealt_damage_to_player_this_turn(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<TargetAst>, CardTextError> {
    let Some(that_idx) = remove_find_exact_phrase_shape(
        clause_words,
        THAT_DEALT_DAMAGE_TO_PHRASE,
        &THAT_DEALT_DAMAGE_TO_PATTERN,
    ) else {
        return Ok(None);
    };
    if that_idx == 0
        || clause_words.len() < that_idx + 7
        || !matches!(clause_words[clause_words.len() - 2..], ["this", "turn"])
    {
        return Ok(None);
    }

    let player_start_word_idx = that_idx + 4;
    let player_end_word_idx = clause_words.len() - 2;
    if player_start_word_idx >= player_end_word_idx {
        return Ok(None);
    }

    let target_cutoff = token_index_for_word_index(tokens, that_idx).unwrap_or(tokens.len());
    let player_start =
        token_index_for_word_index(tokens, player_start_word_idx).unwrap_or(tokens.len());
    let player_end =
        token_index_for_word_index(tokens, player_end_word_idx).unwrap_or(tokens.len());
    let target_tokens = trim_commas(&tokens[..target_cutoff]);
    let player_tokens = trim_commas(&tokens[player_start..player_end]);
    if target_tokens.is_empty() || player_tokens.is_empty() {
        return Ok(None);
    }

    let TargetAst::Player(player, _) = parse_target_phrase(&player_tokens)? else {
        return Ok(None);
    };
    let target = parse_target_phrase(&target_tokens)?;
    let TargetAst::Object(mut filter, target_span, it_span) = target else {
        return Ok(None);
    };
    filter.dealt_damage_to_player_this_turn = Some(player);
    Ok(Some(TargetAst::Object(filter, target_span, it_span)))
}

pub(crate) fn apply_except_filter_exclusions(base: &mut ObjectFilter, exception: &ObjectFilter) {
    for card_type in exception
        .card_types
        .iter()
        .copied()
        .chain(exception.all_card_types.iter().copied())
    {
        if !slice_contains(&base.excluded_card_types, &card_type) {
            base.excluded_card_types.push(card_type);
        }
    }
    for subtype in exception.subtypes.iter().copied() {
        if !slice_contains(&base.excluded_subtypes, &subtype) {
            base.excluded_subtypes.push(subtype);
        }
    }
}
