use super::*;
use crate::runtime_backend::effect_sentences::SubjectVerbPrimitiveClause;
use crate::runtime_backend::lexer::{
    token_slice_at_is, token_slice_first_is, word_slice_contains_any_word,
    word_slice_contains_phrase, word_slice_ends_with, word_slice_eq, word_slice_eq_any,
    word_slice_starts_with, word_slice_starts_with_any,
};
use crate::runtime_backend::util::{
    parse_choice_count_before_target_prefix, parse_subtype_flexible,
};

const RETURN_CONTROL_WORD: &str = "control";
const RETURN_TAPPED_WORD: &str = "tapped";
const RETURN_TO_WORD: &str = "to";
const RETURN_UNDER_WORD: &str = "under";
const RETURN_THIS_WORD: &str = "this";
const RETURN_OF_WORD: &str = "of";
const RETURN_A_WORD: &str = "a";
const RETURN_AT_RANDOM_PREFIX: &[&str] = &["at", "random"];
const RETURN_AT_WORD: &str = "at";
const RETURN_EXCEPT_FOR_PREFIX: &[&str] = &["except", "for"];
const RETURN_EXILED_CARD_REFERENCES: &[&[&str]] = &[
    &["the", "exiled", "card"],
    &["the", "exiled", "cards"],
    &["exiled", "card"],
    &["exiled", "cards"],
];
const RETURN_HAND_WORDS: &[&str] = &["hand", "hands"];
const RETURN_HAND_OR_BATTLEFIELD_WORDS: &[&str] = &["hand", "hands", "battlefield"];
const RETURN_GRAVEYARD_WORDS: &[&str] = &["graveyard", "graveyards"];
const RETURN_UNDER_YOUR_CONTROL_PHRASE: &[&str] = &["under", "your", "control"];
const RETURN_OWNER_WORDS: &[&str] = &["owner", "owners", "owner's", "owners'"];
const RETURN_END_OF_COMBAT_PHRASE: &[&str] = &["end", "of", "combat"];
const RETURN_AND_WORD: &str = "and";
const RETURN_AND_OR_WORDS: &[&str] = &["and", "or"];
const RETURN_TARGET_WORD: &str = "target";
const RETURN_ALL_OR_EACH_WORDS: &[&str] = &["all", "each"];
const RETURN_FROM_YOUR_GRAVEYARD_TAIL: &[&str] = &["from", "your", "graveyard"];
const RETURN_THAT_MANY_PREFIX: &[&str] = &["that", "many"];
const RETURN_CHOSEN_CREATURE_TYPE_INCLUDED_TAILS: &[&[&str]] = &[
    &["of", "the", "chosen", "type"],
    &["that", "are", "of", "the", "chosen", "type"],
];
const RETURN_CHOSEN_CREATURE_TYPE_EXCLUDED_TAILS: &[&[&str]] = &[
    &["that", "arent", "of", "the", "chosen", "type"],
    &["that", "aren't", "of", "the", "chosen", "type"],
    &["that", "are", "not", "of", "the", "chosen", "type"],
];
const RETURN_CHOSEN_CREATURE_TYPE_TAILS: &[&[&str]] = &[
    &["of", "the", "chosen", "type"],
    &["that", "are", "of", "the", "chosen", "type"],
    &["that", "arent", "of", "the", "chosen", "type"],
    &["that", "aren't", "of", "the", "chosen", "type"],
    &["that", "are", "not", "of", "the", "chosen", "type"],
];
const EXCHANGE_LIFE_TOTALS_WORDS: &[&str] = &["life", "totals"];
const EXCHANGE_VERB_WORDS: &[&str] = &["exchange", "exchanges"];
const EXCHANGE_YOUR_PREFIX: &[&str] = &["your"];
const EXCHANGE_TARGET_PLAYER_PREFIXES: &[&[&str]] =
    &[&["target", "player"], &["target", "players"]];
const EXCHANGE_TARGET_OPPONENT_PREFIXES: &[&[&str]] =
    &[&["target", "opponent"], &["target", "opponents"]];
const EXCHANGE_OPPONENT_PREFIXES: &[&[&str]] =
    &[&["an", "opponent"], &["opponent"], &["opponents"]];
const EXCHANGE_AN_OPPONENT_PREFIX: &[&str] = &["an", "opponent"];
const EXCHANGE_WITH_OR_AND_WORDS: &[&str] = &["with", "and"];
const EXCHANGE_AND_WORD: &str = "and";
const EXCHANGE_THAT_SHARE_RELS: &[&[&str]] = &[&["that", "share"], &["that", "shares"]];
const EXCHANGE_PERMANENT_TYPE_SHARE_HEAD_PREFIXES: &[&[&str]] = &[
    &["permanent", "type"],
    &["one", "of", "those", "permanent", "types"],
];
const EXCHANGE_CARD_TYPE_SHARE_HEAD_PREFIXES: &[&[&str]] =
    &[&["card", "type"], &["one", "of", "those", "types"]];

fn return_find_prefix_start(words: &[&str], prefix: &[&str]) -> Option<usize> {
    (0..words.len()).find(|idx| word_slice_starts_with(&words[*idx..], prefix))
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

fn split_chosen_creature_type_tail(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool, bool) {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(tail) = RETURN_CHOSEN_CREATURE_TYPE_TAILS
        .iter()
        .find(|tail| word_slice_ends_with(&words, tail))
    else {
        return (tokens.to_vec(), false, false);
    };
    let excluded = word_slice_eq_any(tail, RETURN_CHOSEN_CREATURE_TYPE_EXCLUDED_TAILS);
    let included = word_slice_eq_any(tail, RETURN_CHOSEN_CREATURE_TYPE_INCLUDED_TAILS);
    if !included && !excluded {
        return (tokens.to_vec(), false, false);
    }
    let filter_word_len = words.len() - tail.len();
    let filter_tokens = match token_index_for_word_index(tokens, filter_word_len) {
        Some(idx) => &tokens[..idx],
        None => tokens,
    };

    (trim_commas(filter_tokens).to_vec(), included, excluded)
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
            RETURN_HAND_OR_BATTLEFIELD_WORDS.contains(word)
        });
        if let Some(hand_or_battlefield_idx) = hand_or_battlefield_idx {
            let mut split_word_idx = hand_or_battlefield_idx + 1;

            if clause_words.get(split_word_idx) == Some(&RETURN_UNDER_WORD) {
                if let Some(control_rel_idx) =
                    find_index(&clause_words[split_word_idx + 1..], |word| {
                        *word == RETURN_CONTROL_WORD
                    })
                {
                    split_word_idx = split_word_idx + 1 + control_rel_idx + 1;
                }
            }

            while clause_words
                .get(split_word_idx)
                .is_some_and(|word| *word == RETURN_TAPPED_WORD)
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
        if !tokens[idx]
            .as_word()
            .is_some_and(|word| word == RETURN_TO_WORD)
        {
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
        if word_slice_starts_with(
            &crate::runtime_backend::token_word_refs(&target_tokens_vec[random_idx..]),
            RETURN_AT_RANDOM_PREFIX,
        ) {
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
        if destination_words_full[word_idx] != RETURN_AT_WORD {
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
    if let Some(except_idx) = return_find_prefix_start(&destination_words, RETURN_EXCEPT_FOR_PREFIX)
    {
        let exception_words = &destination_words[except_idx + 2..];
        if exception_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing return exception qualifiers (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        for word in exception_words {
            if RETURN_AND_OR_WORDS.contains(word) {
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
    let is_hand = word_slice_contains_any_word(&destination_words, RETURN_HAND_WORDS);
    let is_battlefield = destination_words.contains(&"battlefield");
    let is_graveyard = word_slice_contains_any_word(&destination_words, RETURN_GRAVEYARD_WORDS);
    let tapped = destination_words.contains(&RETURN_TAPPED_WORD);
    let transformed = grammar::contains_word(destination_tokens_full, "transformed");
    let converted = grammar::contains_word(destination_tokens_full, "converted");
    let return_controller =
        if word_slice_contains_phrase(&destination_words, RETURN_UNDER_YOUR_CONTROL_PHRASE) {
            ReturnControllerAst::You
        } else if destination_words.contains(&RETURN_CONTROL_WORD)
            && word_slice_contains_any_word(&destination_words, RETURN_OWNER_WORDS)
        {
            ReturnControllerAst::Owner
        } else {
            ReturnControllerAst::Preserve
        };
    let has_delayed_timing_words = grammar::contains_word(destination_tokens_full, "beginning")
        || grammar::contains_word(destination_tokens_full, "upkeep")
        || word_slice_contains_phrase(
            &crate::runtime_backend::token_word_refs(destination_tokens_full),
            RETURN_END_OF_COMBAT_PHRASE,
        )
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
            token.as_word().is_some_and(|word| word == RETURN_AND_WORD)
        })
        && and_idx > 0
    {
        let left_tokens = trim_commas(&target_tokens[..and_idx]);
        let right_tokens = trim_commas(&target_tokens[and_idx + 1..]);
        let left_words = crate::runtime_backend::token_word_refs(&left_tokens);
        let right_words = crate::runtime_backend::token_word_refs(&right_tokens);

        let source_filter_for_words = |words: &[&str]| -> Option<ObjectFilter> {
            if words.first() != Some(&RETURN_THIS_WORD) {
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
            word_slice_eq_any(words, RETURN_EXILED_CARD_REFERENCES)
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
        token.as_word().is_some_and(|word| word == RETURN_AND_WORD)
    }) && and_idx > 0
    {
        let tail_slice = &target_tokens[and_idx + 1..];
        let starts_multi_target = tail_slice.first().is_some_and(|token| {
            token
                .as_word()
                .is_some_and(|word| word == RETURN_TARGET_WORD)
        }) || parse_choice_count_before_target_prefix(tail_slice)
            .is_some();
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
        .is_some_and(|word| RETURN_ALL_OR_EACH_WORDS.contains(word))
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
        let (base_filter_tokens, chosen_creature_type, excluded_chosen_creature_type) =
            split_chosen_creature_type_tail(&return_filter_tokens);
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
        && word_slice_eq(
            &target_words[target_words.len() - 3..],
            RETURN_FROM_YOUR_GRAVEYARD_TAIL,
        ) {
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
        if word_slice_starts_with(&target_words, RETURN_THAT_MANY_PREFIX) {
            let mut object_start = 2usize;
            if target_words.get(object_start) == Some(&RETURN_OF_WORD) {
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
            word_slice_eq_any(window, EXCHANGE_THAT_SHARE_RELS)
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
        let share_head = if share_head.first() == Some(&RETURN_A_WORD) {
            &share_head[1..]
        } else {
            share_head
        };

        let shared_type = if word_slice_starts_with_any(
            share_head,
            EXCHANGE_PERMANENT_TYPE_SHARE_HEAD_PREFIXES,
        ) {
            SharedTypeConstraintAst::PermanentType
        } else if word_slice_starts_with_any(share_head, EXCHANGE_CARD_TYPE_SHARE_HEAD_PREFIXES) {
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

    let tokens = if tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| EXCHANGE_VERB_WORDS.contains(&word))
    {
        &tokens[1..]
    } else {
        tokens
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::words_match_any_prefix(tokens, LIFE_TOTALS_PREFIXES).is_some() {
        if word_slice_eq(&clause_words, EXCHANGE_LIFE_TOTALS_WORDS) {
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
    let zone_exchange = if word_slice_starts_with(&clause_words, EXCHANGE_YOUR_PREFIX) {
        Some((PlayerAst::You, 1))
    } else if word_slice_starts_with_any(&clause_words, EXCHANGE_TARGET_PLAYER_PREFIXES) {
        Some((PlayerAst::Target, 2))
    } else if word_slice_starts_with_any(&clause_words, EXCHANGE_TARGET_OPPONENT_PREFIXES) {
        Some((PlayerAst::TargetOpponent, 2))
    } else if word_slice_starts_with_any(&clause_words, EXCHANGE_OPPONENT_PREFIXES) {
        Some((
            PlayerAst::Opponent,
            if word_slice_starts_with(&clause_words, EXCHANGE_AN_OPPONENT_PREFIX) {
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
        && clause_words.get(consumed + 1) == Some(&EXCHANGE_AND_WORD)
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
                token
                    .as_word()
                    .is_some_and(|word| EXCHANGE_WITH_OR_AND_WORDS.contains(&word))
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
