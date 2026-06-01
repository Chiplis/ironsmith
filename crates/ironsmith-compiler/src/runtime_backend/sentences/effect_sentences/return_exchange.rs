use super::*;
use crate::runtime_backend::effect_sentences::SubjectVerbPrimitiveClause;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use crate::runtime_backend::lexer::{token_slice_at_is, token_slice_first_is};
use crate::runtime_backend::util::{
    parse_choice_count_before_target_prefix, parse_subtype_flexible,
};

const RETURN_CONTROL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["control"]);
const RETURN_TAPPED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["tapped"]);
const RETURN_TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const RETURN_UNDER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["under"]);
const RETURN_THIS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this"]);
const RETURN_OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const RETURN_A_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["a"]);
const RETURN_AN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["an"]);
const RETURN_AT_RANDOM_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["at", "random"]);
const RETURN_AT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["at"]);
const RETURN_EXCEPT_FOR_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["except", "for"]);
const RETURN_EXILED_CARD_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["the", "exiled", "card"],
            &["the", "exiled", "cards"],
            &["exiled", "card"],
            &["exiled", "cards"],
        ]
);
const RETURN_HAND_DESTINATION_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["hand", "hands"]]);
const RETURN_HAND_OR_BATTLEFIELD_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["hand"], &["hands"], &["battlefield"]]);
const RETURN_BATTLEFIELD_DESTINATION_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["battlefield"]);
const RETURN_GRAVEYARD_DESTINATION_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["graveyard", "graveyards"]]);
const RETURN_TAPPED_DESTINATION_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["tapped"]);
const RETURN_UNDER_YOUR_CONTROL_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["under", "your", "control"]]);
const RETURN_OWNER_CONTROL_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["control"]; contains_any_words & [&["owner", "owners", "owner's", "owners'"]]);
const RETURN_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const RETURN_AND_OR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["or"]]);
const RETURN_TARGET_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["target"]);
const RETURN_ALL_OR_EACH_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["all"], &["each"]]);
const RETURN_FROM_YOUR_GRAVEYARD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "your", "graveyard"]);
const RETURN_THAT_MANY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "many"]);
const EXCHANGE_LIFE_TOTALS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["life", "totals"]);
const EXCHANGE_YOUR_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["your"]);
const EXCHANGE_TARGET_PLAYER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["target", "player"], &["target", "players"]]);
const EXCHANGE_TARGET_OPPONENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["target", "opponent"], &["target", "opponents"]]);
const EXCHANGE_OPPONENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["an", "opponent"], &["opponent"], &["opponents"]]);
const EXCHANGE_AN_OPPONENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["an", "opponent"]);
const EXCHANGE_WITH_OR_AND_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["with"], &["and"]]);
const EXCHANGE_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const EXCHANGE_THAT_SHARE_REL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that", "share"], &["that", "shares"]]);
const EXCHANGE_PERMANENT_TYPE_SHARE_HEAD_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["permanent", "type"],
            &["one", "of", "those", "permanent", "types"],
        ]
);
const EXCHANGE_CARD_TYPE_SHARE_HEAD_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["card", "type"], &["one", "of", "those", "types"]]);

fn return_find_phrase_start(words: &[&str], shape: ClauseShape<'static>) -> Option<usize> {
    (0..words.len()).find(|idx| shape.matches_words(&words[*idx..]))
}

fn return_words_match_value<T: Copy>(words: &[&str], choices: &[(&[&str], T)]) -> Option<T> {
    choices
        .iter()
        .find_map(|(phrase, value)| (*phrase == words).then_some(*value))
}

fn return_words_match_phrase<'a>(
    words: &[&str],
    choices: &'a [&'a [&'a str]],
) -> Option<&'a [&'a str]> {
    choices
        .iter()
        .find_map(|phrase| (*phrase == words).then_some(*phrase))
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DelayedReturnTimingAst {
    NextEndStep(PlayerFilter),
    NextUpkeep(PlayerAst),
    EndOfCombat,
}

pub(crate) fn parse_delayed_return_timing_words(words: &[&str]) -> Option<DelayedReturnTimingAst> {
    if matches!(
        words,
        ["at", "end", "of", "combat"] | ["at", "the", "end", "of", "combat"]
    ) {
        return Some(DelayedReturnTimingAst::EndOfCombat);
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "end", "step"]
            | ["at", "beginning", "of", "the", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "the", "next", "end", "step"]
    ) {
        return Some(DelayedReturnTimingAst::NextEndStep(PlayerFilter::Any));
    }

    if matches!(
        words,
        ["at", "beginning", "of", "your", "next", "end", "step"]
            | [
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "end",
                "step"
            ]
    ) {
        return Some(DelayedReturnTimingAst::NextEndStep(PlayerFilter::You));
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "upkeep"]
            | ["at", "beginning", "of", "the", "next", "upkeep"]
            | ["at", "the", "beginning", "of", "next", "upkeep"]
            | ["at", "the", "beginning", "of", "the", "next", "upkeep"]
    ) {
        return Some(DelayedReturnTimingAst::NextUpkeep(PlayerAst::Any));
    }

    if matches!(
        words,
        ["at", "beginning", "of", "your", "next", "upkeep"]
            | ["at", "the", "beginning", "of", "your", "next", "upkeep"]
    ) {
        return Some(DelayedReturnTimingAst::NextUpkeep(PlayerAst::You));
    }

    None
}

pub(crate) fn wrap_return_with_delayed_timing(
    effect: EffectAst,
    timing: Option<DelayedReturnTimingAst>,
) -> EffectAst {
    let Some(timing) = timing else {
        return effect;
    };

    match timing {
        DelayedReturnTimingAst::NextEndStep(player) => EffectAst::DelayedUntilNextEndStep {
            player,
            effects: vec![effect],
        },
        DelayedReturnTimingAst::NextUpkeep(player) => EffectAst::DelayedUntilNextUpkeep {
            player,
            effects: vec![effect],
        },
        DelayedReturnTimingAst::EndOfCombat => EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        },
    }
}

pub(crate) fn parse_return(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let rewritten_storage;
    let tokens = if token_slice_first_is(tokens, "to") {
        let clause_words = crate::runtime_backend::token_word_refs(tokens);
        let hand_or_battlefield_idx = find_index(&clause_words, |word| {
            RETURN_HAND_OR_BATTLEFIELD_WORD_PATTERN.matches_word(word)
        });
        if let Some(hand_or_battlefield_idx) = hand_or_battlefield_idx {
            let mut split_word_idx = hand_or_battlefield_idx + 1;

            if RETURN_UNDER_WORD_PATTERN.matches_word_at(&clause_words, split_word_idx) {
                if let Some(control_rel_idx) =
                    find_index(&clause_words[split_word_idx + 1..], |word| {
                        RETURN_CONTROL_WORD_PATTERN.matches_word(word)
                    })
                {
                    split_word_idx = split_word_idx + 1 + control_rel_idx + 1;
                }
            }

            while clause_words
                .get(split_word_idx)
                .is_some_and(|word| RETURN_TAPPED_WORD_PATTERN.matches_word(word))
            {
                split_word_idx += 1;
            }

            if let Some(split_token_idx) = token_index_for_word_index(tokens, split_word_idx) {
                let target_tokens = trim_commas(&tokens[split_token_idx..]);
                let destination_tokens = trim_commas(&tokens[..split_token_idx]);
                if !target_tokens.is_empty() && !destination_tokens.is_empty() {
                    let mut rewritten = target_tokens.to_vec();
                    rewritten.extend(destination_tokens.to_vec());
                    rewritten_storage = rewritten;
                    &rewritten_storage
                } else {
                    tokens
                }
            } else {
                tokens
            }
        } else {
            tokens
        }
    } else {
        tokens
    };

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::contains_word(tokens, "unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported return-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut to_idx = None;
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        if !RETURN_TO_WORD_PATTERN.matches_token(&tokens[idx]) {
            continue;
        }
        let tail_tokens = &tokens[idx + 1..];
        if grammar::contains_word(tail_tokens, "hand")
            || grammar::contains_word(tail_tokens, "hands")
            || grammar::contains_word(tail_tokens, "battlefield")
            || grammar::contains_word(tail_tokens, "graveyard")
            || grammar::contains_word(tail_tokens, "graveyards")
        {
            to_idx = Some(idx);
            break;
        }
    }
    let to_idx = to_idx.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing return destination (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;

    let mut target_tokens_vec = tokens[..to_idx].to_vec();
    let mut random = false;
    let mut random_idx = 0usize;
    while random_idx + 1 < target_tokens_vec.len() {
        if RETURN_AT_RANDOM_PREFIX_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(
            &target_tokens_vec[random_idx..],
        )) {
            random = true;
            target_tokens_vec.drain(random_idx..random_idx + 2);
            break;
        }
        random_idx += 1;
    }
    let target_tokens = target_tokens_vec.as_slice();
    let destination_tokens_full = &tokens[to_idx + 1..];
    let destination_words_full = crate::runtime_backend::token_word_refs(destination_tokens_full);
    let mut delayed_timing = None;
    let mut destination_word_cutoff = destination_words_full.len();
    for word_idx in 0..destination_words_full.len() {
        if !RETURN_AT_WORD_PATTERN.matches_word(destination_words_full[word_idx]) {
            continue;
        }
        if let Some(timing) = parse_delayed_return_timing_words(&destination_words_full[word_idx..])
        {
            delayed_timing = Some(timing);
            destination_word_cutoff = word_idx;
            break;
        }
    }

    let destination_tokens = if destination_word_cutoff < destination_words_full.len() {
        let token_cutoff =
            token_index_for_word_index(destination_tokens_full, destination_word_cutoff)
                .unwrap_or(destination_tokens_full.len());
        &destination_tokens_full[..token_cutoff]
    } else {
        destination_tokens_full
    };

    let mut destination_words = crate::runtime_backend::token_word_refs(destination_tokens);
    let mut destination_excluded_subtypes: Vec<Subtype> = Vec::new();
    if let Some(except_idx) =
        return_find_phrase_start(&destination_words, RETURN_EXCEPT_FOR_PREFIX_PATTERN)
    {
        let exception_words = &destination_words[except_idx + 2..];
        if exception_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing return exception qualifiers (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        for word in exception_words {
            if RETURN_AND_OR_WORD_PATTERN.matches_word(word) {
                continue;
            }
            let Some(subtype) = parse_subtype_flexible(word) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported return exception qualifier '{}' (clause: '{}')",
                    word,
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            };
            if !slice_contains(&destination_excluded_subtypes, &subtype) {
                destination_excluded_subtypes.push(subtype);
            }
        }
        if destination_excluded_subtypes.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing subtype return exception qualifiers (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        destination_words.truncate(except_idx);
    }
    let is_hand = RETURN_HAND_DESTINATION_PATTERN.matches_words(&destination_words);
    let is_battlefield = RETURN_BATTLEFIELD_DESTINATION_PATTERN.matches_words(&destination_words);
    let is_graveyard = RETURN_GRAVEYARD_DESTINATION_PATTERN.matches_words(&destination_words);
    let tapped = RETURN_TAPPED_DESTINATION_PATTERN.matches_words(&destination_words);
    let transformed = grammar::contains_word(destination_tokens_full, "transformed");
    let converted = grammar::contains_word(destination_tokens_full, "converted");
    let return_controller = if RETURN_UNDER_YOUR_CONTROL_PATTERN.matches_words(&destination_words) {
        ReturnControllerAst::You
    } else if RETURN_OWNER_CONTROL_PATTERN.matches_words(&destination_words) {
        ReturnControllerAst::Owner
    } else {
        ReturnControllerAst::Preserve
    };
    let has_delayed_timing_words = grammar::contains_word(destination_tokens_full, "beginning")
        || grammar::contains_word(destination_tokens_full, "upkeep")
        || grammar::words_find_phrase(destination_tokens_full, &["end", "of", "combat"]).is_some()
        || grammar::contains_word(destination_tokens_full, "end")
            && (grammar::contains_word(destination_tokens_full, "next")
                || grammar::contains_word(destination_tokens_full, "step"));
    if delayed_timing.is_none() && has_delayed_timing_words {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed return timing clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }
    if !is_hand && !is_battlefield && !is_graveyard {
        return Err(CardTextError::ParseError(format!(
            "unsupported return destination (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let target_words = crate::runtime_backend::token_word_refs(target_tokens);
    if is_hand
        && let Some(and_idx) = find_index(target_tokens, |token| {
            RETURN_AND_WORD_PATTERN.matches_token(token)
        })
        && and_idx > 0
    {
        let left_tokens = trim_commas(&target_tokens[..and_idx]);
        let right_tokens = trim_commas(&target_tokens[and_idx + 1..]);
        let left_words = crate::runtime_backend::token_word_refs(&left_tokens);
        let right_words = crate::runtime_backend::token_word_refs(&right_tokens);

        let source_filter_for_words = |words: &[&str]| -> Option<ObjectFilter> {
            if !RETURN_THIS_WORD_PATTERN.matches_first_word(words) {
                return None;
            }
            let mut filter = ObjectFilter::source();
            if let Some(subtype_word) = words.get(1).copied()
                && let Some(subtype) = parse_subtype_word(subtype_word)
            {
                filter.subtypes.push(subtype);
            }
            Some(filter)
        };
        let exiled_filter_for_words = |words: &[&str]| -> Option<ObjectFilter> {
            RETURN_EXILED_CARD_REFERENCE_PATTERN
                .matches_words(words)
                .then(|| ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile))
        };

        let paired_filters = source_filter_for_words(&left_words)
            .zip(exiled_filter_for_words(&right_words))
            .or_else(|| {
                source_filter_for_words(&right_words).zip(exiled_filter_for_words(&left_words))
            });
        if let Some((source_filter, exiled_filter)) = paired_filters {
            let mut filter = ObjectFilter::default();
            filter.any_of = vec![source_filter, exiled_filter];
            return Ok(wrap_return_with_delayed_timing(
                EffectAst::subject_verb_return_all_to_hand(filter),
                delayed_timing,
            ));
        }
    }
    if let Some(and_idx) = find_index(target_tokens, |token| {
        RETURN_AND_WORD_PATTERN.matches_token(token)
    }) && and_idx > 0
    {
        let tail_slice = &target_tokens[and_idx + 1..];
        let starts_multi_target = tail_slice
            .first()
            .is_some_and(|token| RETURN_TARGET_WORD_PATTERN.matches_token(token))
            || parse_choice_count_before_target_prefix(tail_slice).is_some();
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target return clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
    }
    if !grammar::contains_word(target_tokens, "target")
        && grammar::contains_word(target_tokens, "exiled")
        && grammar::contains_word(target_tokens, "cards")
    {
        let filter = parse_object_filter(target_tokens, false)?;
        let effect = if is_battlefield {
            EffectAst::subject_verb_return_all_to_battlefield(
                filter,
                tapped,
                false,
                ReturnControllerAst::Owner,
            )
        } else if is_graveyard {
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Object(filter, None, None),
                Zone::Graveyard,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )
        } else {
            EffectAst::subject_verb_return_all_to_hand(filter)
        };
        return Ok(wrap_return_with_delayed_timing(effect, delayed_timing));
    }
    if target_words
        .first()
        .is_some_and(|word| RETURN_ALL_OR_EACH_WORD_PATTERN.matches_word(word))
    {
        let has_unsupported_return_all_qualifier = grammar::contains_word(target_tokens, "dealt")
            || grammar::contains_word(target_tokens, "without")
                && grammar::contains_word(target_tokens, "counter");
        if has_unsupported_return_all_qualifier {
            return Err(CardTextError::ParseError(format!(
                "unsupported qualified return-all filter (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing return-all filter".to_string(),
            ));
        }
        let return_filter_tokens = &target_tokens[1..];
        if is_hand
            && let Some((choice_idx, consumed)) =
                find_color_choice_phrase(SubjectVerbPrimitiveClause::new(return_filter_tokens))
        {
            let base_filter_tokens = trim_commas(&return_filter_tokens[..choice_idx]);
            let trailing = trim_commas(&return_filter_tokens[choice_idx + consumed..]);
            if !trailing.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing color-choice return-all clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
            if base_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter before color-choice clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
            let mut filter = parse_object_filter(&base_filter_tokens, false)?;
            for subtype in destination_excluded_subtypes {
                if !slice_contains(&filter.excluded_subtypes, &subtype) {
                    filter.excluded_subtypes.push(subtype);
                }
            }
            return Ok(wrap_return_with_delayed_timing(
                EffectAst::subject_verb_return_all_to_hand_of_chosen_color(filter),
                delayed_timing,
            ));
        }
        let return_filter_words = crate::runtime_backend::token_word_refs(return_filter_tokens);
        let chosen_this_way_suffixes: [(&[&str], bool); 7] = [
            (&["not", "chosen", "this", "way"], true),
            (&["that", "weren't", "chosen", "this", "way"], true),
            (&["that", "werent", "chosen", "this", "way"], true),
            (&["that", "were", "not", "chosen", "this", "way"], true),
            (&["chosen", "this", "way"], false),
            (&["that", "were", "chosen", "this", "way"], false),
            (&["that", "was", "chosen", "this", "way"], false),
        ];
        let (return_filter_tokens, chosen_this_way_excluded) = if let Some((suffix, excluded)) =
            chosen_this_way_suffixes.iter().find(|(suffix, _)| {
                return_filter_words.len() >= suffix.len()
                    && &return_filter_words[return_filter_words.len() - suffix.len()..] == *suffix
            }) {
            let cutoff = return_filter_words.len() - suffix.len();
            let token_cutoff = if cutoff == 0 {
                0
            } else {
                token_index_for_word_index(return_filter_tokens, cutoff)
                    .unwrap_or(return_filter_tokens.len())
            };
            (
                trim_commas(&return_filter_tokens[..token_cutoff]).to_vec(),
                Some(*excluded),
            )
        } else {
            (return_filter_tokens.to_vec(), None)
        };
        let return_filter_words = crate::runtime_backend::token_word_refs(&return_filter_tokens);
        let chosen_type_suffix_patterns: [(&[&str], bool, bool); 5] = [
            (
                &["that", "arent", "of", "the", "chosen", "type"],
                false,
                true,
            ),
            (
                &["that", "aren't", "of", "the", "chosen", "type"],
                false,
                true,
            ),
            (
                &["that", "are", "not", "of", "the", "chosen", "type"],
                false,
                true,
            ),
            (&["of", "the", "chosen", "type"], true, false),
            (&["that", "are", "of", "the", "chosen", "type"], true, false),
        ];
        let (base_filter_tokens, chosen_creature_type, excluded_chosen_creature_type) =
            if let Some((suffix, chosen_type, excluded_chosen_type)) =
                chosen_type_suffix_patterns.iter().find(|(suffix, _, _)| {
                    return_filter_words.len() >= suffix.len()
                        && &return_filter_words[return_filter_words.len() - suffix.len()..]
                            == *suffix
                })
            {
                let cutoff = return_filter_words.len() - suffix.len();
                let token_cutoff = token_index_for_word_index(&return_filter_tokens, cutoff)
                    .unwrap_or(return_filter_tokens.len());
                (
                    trim_commas(&return_filter_tokens[..token_cutoff]).to_vec(),
                    *chosen_type,
                    *excluded_chosen_type,
                )
            } else {
                (return_filter_tokens, false, false)
            };
        let mut filter = parse_object_filter(&base_filter_tokens, false)?;
        filter.chosen_creature_type |= chosen_creature_type;
        filter.excluded_chosen_creature_type |= excluded_chosen_creature_type;
        for subtype in destination_excluded_subtypes {
            if !slice_contains(&filter.excluded_subtypes, &subtype) {
                filter.excluded_subtypes.push(subtype);
            }
        }
        if let Some(excluded) = chosen_this_way_excluded {
            filter = if excluded {
                filter.not_tagged(TagKey::from(IT_TAG))
            } else {
                filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject)
            };
        }
        let effect = if is_battlefield {
            EffectAst::subject_verb_return_all_to_battlefield(
                filter,
                tapped,
                false,
                ReturnControllerAst::Owner,
            )
        } else if is_graveyard {
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Object(filter, None, None),
                Zone::Graveyard,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )
        } else {
            EffectAst::subject_verb_return_all_to_hand(filter)
        };
        return Ok(wrap_return_with_delayed_timing(effect, delayed_timing));
    }
    if !destination_excluded_subtypes.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported return exception on non-return-all clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let source_from_graveyard_target = if is_battlefield
        && target_words.len() > 3
        && RETURN_FROM_YOUR_GRAVEYARD_TAIL_PATTERN
            .matches_words(&target_words[target_words.len() - 3..])
    {
        let prefix_word_len = target_words.len() - 3;
        let prefix_token_len = token_index_for_word_index(target_tokens, prefix_word_len)
            .unwrap_or(target_tokens.len());
        let prefix_tokens = trim_commas(&target_tokens[..prefix_token_len]);
        match parse_target_phrase(&prefix_tokens) {
            Ok(TargetAst::Source(span)) => Some(TargetAst::Source(span)),
            _ => None,
        }
    } else {
        None
    };

    let mut count_value = None;
    let (target_tokens, dynamic_count) =
        if RETURN_THAT_MANY_PREFIX_PATTERN.matches_words(&target_words) {
            let mut object_start = 2usize;
            if RETURN_OF_WORD_PATTERN.matches_word_at(&target_words, object_start) {
                object_start += 1;
            }
            let Some(token_start) = token_index_for_word_index(target_tokens, object_start) else {
                return Err(CardTextError::ParseError(format!(
                    "missing object phrase after 'that many' (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            };
            count_value = Some(crate::effect::Value::EventValue(
                crate::effect::EventValueSpec::Amount,
            ));
            (&target_tokens[token_start..], true)
        } else {
            (target_tokens, false)
        };
    let mut target = if let Some(target) = source_from_graveyard_target {
        target
    } else if matches!(
        target_words.as_slice(),
        ["it"]
            | ["them"]
            | ["that", "card"]
            | ["that", "creature"]
            | ["that", "object"]
            | ["that", "permanent"]
            | ["those", "cards"]
            | ["those", "creatures"]
            | ["those", "objects"]
            | ["those", "permanents"]
    ) {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens))
    } else {
        parse_target_phrase(target_tokens)?
    };
    if dynamic_count {
        target = TargetAst::WithCount(Box::new(target), crate::effect::ChoiceCount::dynamic_x());
    }
    let effect = if is_battlefield {
        EffectAst::subject_verb_return_to_battlefield(
            target,
            tapped,
            transformed,
            converted,
            return_controller,
            count_value,
        )
    } else if is_graveyard {
        EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Graveyard,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )
    } else {
        EffectAst::subject_verb_return_to_hand(target, random)
    };
    Ok(wrap_return_with_delayed_timing(effect, delayed_timing))
}

pub(crate) fn parse_exchange(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn split_shared_type_clause<'a>(
        clause_tokens: &'a [OwnedLexToken],
    ) -> Result<(&'a [OwnedLexToken], Option<SharedTypeConstraintAst>), CardTextError> {
        let tail_words = crate::runtime_backend::token_word_refs(clause_tokens);
        let Some(rel_word_idx) = find_window_by(&tail_words, 2, |window| {
            EXCHANGE_THAT_SHARE_REL_PATTERN.matches_words(window)
        }) else {
            return Ok((clause_tokens, None));
        };

        let rel_token_idx =
            token_index_for_word_index(clause_tokens, rel_word_idx).unwrap_or(clause_tokens.len());
        let (head, tail) = clause_tokens.split_at(rel_token_idx);
        let share_words = crate::runtime_backend::token_word_refs(tail);
        let share_head =
            if let Some((prefix, _)) = grammar::words_match_any_prefix(tail, SHARE_REL_PREFIXES) {
                &share_words[prefix.len()..]
            } else {
                &share_words[..]
            };
        let share_head = if RETURN_A_WORD_PATTERN.matches_first_word(share_head) {
            &share_head[1..]
        } else {
            share_head
        };

        let shared_type = if EXCHANGE_PERMANENT_TYPE_SHARE_HEAD_PATTERN.matches_words(share_head) {
            SharedTypeConstraintAst::PermanentType
        } else if EXCHANGE_CARD_TYPE_SHARE_HEAD_PATTERN.matches_words(share_head) {
            SharedTypeConstraintAst::CardType
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported exchange share-type clause (clause: '{}')",
                tail_words.join(" ")
            )));
        };

        Ok((head, Some(shared_type)))
    }

    fn parse_value_operand(operand_tokens: &[OwnedLexToken]) -> Option<ExchangeValueAst> {
        let words = crate::runtime_backend::token_word_refs(operand_tokens);
        if let Some(player) = return_words_match_value(
            &words,
            &[
                (&["your", "life", "total"], PlayerAst::You),
                (&["target", "player", "life", "total"], PlayerAst::Target),
                (&["target", "players", "life", "total"], PlayerAst::Target),
                (&["target", "player's", "life", "total"], PlayerAst::Target),
                (&["target", "players'", "life", "total"], PlayerAst::Target),
                (
                    &["target", "opponent", "life", "total"],
                    PlayerAst::TargetOpponent,
                ),
                (
                    &["target", "opponents", "life", "total"],
                    PlayerAst::TargetOpponent,
                ),
                (
                    &["target", "opponent's", "life", "total"],
                    PlayerAst::TargetOpponent,
                ),
                (
                    &["target", "opponents'", "life", "total"],
                    PlayerAst::TargetOpponent,
                ),
                (&["an", "opponent", "life", "total"], PlayerAst::Opponent),
                (&["opponent", "life", "total"], PlayerAst::Opponent),
                (&["opponents", "life", "total"], PlayerAst::Opponent),
            ],
        ) {
            return Some(ExchangeValueAst::LifeTotal(player));
        }

        if let Some(kind) = return_words_match_value(
            &words,
            &[
                (&["its", "power"], ExchangeValueKindAst::Power),
                (&["this", "power"], ExchangeValueKindAst::Power),
                (&["thiss", "power"], ExchangeValueKindAst::Power),
                (&["this's", "power"], ExchangeValueKindAst::Power),
                (&["this", "creature", "power"], ExchangeValueKindAst::Power),
                (
                    &["this", "creature's", "power"],
                    ExchangeValueKindAst::Power,
                ),
                (&["thiss", "creature", "power"], ExchangeValueKindAst::Power),
                (
                    &["thiss", "creature's", "power"],
                    ExchangeValueKindAst::Power,
                ),
                (&["this", "creatures", "power"], ExchangeValueKindAst::Power),
                (
                    &["thiss", "creatures", "power"],
                    ExchangeValueKindAst::Power,
                ),
                (&["its", "toughness"], ExchangeValueKindAst::Toughness),
                (&["this", "toughness"], ExchangeValueKindAst::Toughness),
                (&["thiss", "toughness"], ExchangeValueKindAst::Toughness),
                (&["this's", "toughness"], ExchangeValueKindAst::Toughness),
                (
                    &["this", "creature", "toughness"],
                    ExchangeValueKindAst::Toughness,
                ),
                (
                    &["this", "creature's", "toughness"],
                    ExchangeValueKindAst::Toughness,
                ),
                (
                    &["thiss", "creature", "toughness"],
                    ExchangeValueKindAst::Toughness,
                ),
                (
                    &["thiss", "creature's", "toughness"],
                    ExchangeValueKindAst::Toughness,
                ),
                (
                    &["this", "creatures", "toughness"],
                    ExchangeValueKindAst::Toughness,
                ),
                (
                    &["thiss", "creatures", "toughness"],
                    ExchangeValueKindAst::Toughness,
                ),
            ],
        ) {
            return Some(ExchangeValueAst::Stat {
                target: TargetAst::Source(span_from_tokens(operand_tokens)),
                kind,
            });
        }

        let power_prefix = if let Some((prefix, _)) =
            grammar::words_match_any_prefix(operand_tokens, POWER_OF_PREFIXES)
        {
            Some((ExchangeValueKindAst::Power, prefix.len()))
        } else if let Some((prefix, _)) =
            grammar::words_match_any_prefix(operand_tokens, TOUGHNESS_OF_PREFIXES)
        {
            Some((ExchangeValueKindAst::Toughness, prefix.len()))
        } else {
            None
        }?;

        let (kind, used) = power_prefix;
        let target = parse_target_phrase(&operand_tokens[used..]).ok()?;
        Some(ExchangeValueAst::Stat { target, kind })
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::words_match_any_prefix(tokens, LIFE_TOTALS_PREFIXES).is_some() {
        if EXCHANGE_LIFE_TOTALS_PATTERN.matches_words(&clause_words) {
            return match subject {
                Some(SubjectAst::Player(PlayerAst::Target)) => {
                    Ok(EffectAst::subject_verb_exchange_life_totals(
                        PlayerAst::Target,
                        PlayerAst::Target,
                    ))
                }
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported life-total exchange clause (clause: '{}')",
                    clause_words.join(" ")
                ))),
            };
        }

        if grammar::words_match_prefix(tokens, &["life", "totals", "with"]).is_none() {
            return Err(CardTextError::ParseError(format!(
                "unsupported exchange clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let player2_words = crate::runtime_backend::token_word_refs(&tokens[3..]);
        let player2 = match return_words_match_phrase(
            &player2_words,
            &[
                &["you"],
                &["target", "player"],
                &["target", "players"],
                &["target", "opponent"],
                &["target", "opponents"],
                &["that", "player"],
                &["that", "players"],
                &["opponent"],
                &["opponents"],
                &["an", "opponent"],
            ],
        ) {
            Some(["you"]) => Some(PlayerAst::You),
            Some(["target", "player"] | ["target", "players"]) => Some(PlayerAst::Target),
            Some(["target", "opponent"] | ["target", "opponents"]) => {
                Some(PlayerAst::TargetOpponent)
            }
            Some(["that", "player"] | ["that", "players"]) => Some(PlayerAst::That),
            Some(["opponent"] | ["opponents"] | ["an", "opponent"]) => Some(PlayerAst::Opponent),
            _ => None,
        }
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported life-total exchange partner (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let player1 = match subject {
            Some(SubjectAst::Player(player)) => player,
            _ => PlayerAst::You,
        };

        return Ok(EffectAst::subject_verb_exchange_life_totals(
            player1, player2,
        ));
    }
    if grammar::words_match_any_prefix(tokens, TEXT_BOXES_OF_PREFIXES).is_some() {
        let remainder = if let Some((_, rest)) =
            grammar::words_match_any_prefix(tokens, TEXT_BOXES_OF_PREFIXES)
        {
            rest
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported text-box exchange clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };

        let target = parse_target_phrase(remainder).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported text-box exchange target (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

        return Ok(EffectAst::subject_verb_exchange_text_boxes(target));
    }
    let zone_exchange = if EXCHANGE_YOUR_PREFIX_PATTERN.matches_words(&clause_words) {
        Some((PlayerAst::You, 1))
    } else if EXCHANGE_TARGET_PLAYER_PREFIX_PATTERN.matches_words(&clause_words) {
        Some((PlayerAst::Target, 2))
    } else if EXCHANGE_TARGET_OPPONENT_PREFIX_PATTERN.matches_words(&clause_words) {
        Some((PlayerAst::TargetOpponent, 2))
    } else if EXCHANGE_OPPONENT_PREFIX_PATTERN.matches_words(&clause_words) {
        Some((
            PlayerAst::Opponent,
            if EXCHANGE_AN_OPPONENT_PREFIX_PATTERN.matches_words(&clause_words) {
                2
            } else {
                1
            },
        ))
    } else {
        None
    };
    if let Some((player, consumed)) = zone_exchange
        && let Some(zone1) = clause_words
            .get(consumed)
            .and_then(|word| parse_zone_word(*word))
        && EXCHANGE_AND_WORD_PATTERN.matches_word_at(&clause_words, consumed + 1)
        && let Some(zone2) = clause_words
            .get(consumed + 2)
            .and_then(|word| parse_zone_word(*word))
        && consumed + 3 == clause_words.len()
    {
        return Ok(EffectAst::subject_verb_exchange_zones(player, zone1, zone2));
    }
    if grammar::words_match_prefix(tokens, &["control", "of"]).is_none() {
        if grammar::contains_word(tokens, "life")
            || grammar::contains_word(tokens, "power")
            || grammar::contains_word(tokens, "toughness")
        {
            let (duration, remainder) =
                if let Some((duration, remainder)) = parse_restriction_duration(tokens)? {
                    (duration, remainder)
                } else {
                    (Until::Forever, trim_commas(tokens).to_vec())
                };

            let split_idx = find_index(&remainder, |token: &OwnedLexToken| {
                EXCHANGE_WITH_OR_AND_WORD_PATTERN.matches_token(token)
            })
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported exchange clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
            let left_tokens = trim_commas(&remainder[..split_idx]);
            let right_tokens = trim_commas(&remainder[split_idx + 1..]);
            let left = parse_value_operand(&left_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported exchange value operand (clause: '{}')",
                    crate::runtime_backend::token_word_refs(&left_tokens).join(" ")
                ))
            })?;
            let right = parse_value_operand(&right_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported exchange value operand (clause: '{}')",
                    crate::runtime_backend::token_word_refs(&right_tokens).join(" ")
                ))
            })?;

            return Ok(EffectAst::subject_verb_exchange_values(
                left, right, duration,
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported exchange clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some((before_and, after_and)) =
        crate::runtime_backend::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            crate::runtime_backend::grammar::primitives::kw("and").void()
        })
    {
        let left_target = parse_target_phrase(&before_and[2..]).ok();
        let (right_tokens, shared_type) = split_shared_type_clause(after_and)?;
        let right_target = parse_target_phrase(right_tokens).ok();
        if let (Some(permanent1), Some(permanent2)) = (left_target, right_target) {
            return Ok(EffectAst::subject_verb_exchange_control_heterogeneous(
                permanent1,
                permanent2,
                shared_type,
            ));
        }
    }

    let mut idx = 2usize;
    let mut count = 2u32;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }
    if token_slice_at_is(&tokens, idx, "target") {
        idx += 1;
    }
    if idx >= tokens.len() {
        return Err(CardTextError::ParseError(
            "missing exchange target filter".to_string(),
        ));
    }

    let (filter_tokens, shared_type) = split_shared_type_clause(&tokens[idx..])?;

    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(EffectAst::subject_verb_exchange_control(
        filter,
        count,
        shared_type,
    ))
}
