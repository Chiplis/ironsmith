use super::super::lexer::{LexedClause, OwnedLexToken};
use super::super::object_filters::is_comparison_or_delimiter;
use super::super::token_primitives::parse_leading_may_action_lexed;
use super::super::util::{
    is_article, parse_choice_count_token_prefix_consumed, parse_number,
    parse_number_or_x_value_lexed, strip_leading_article_word_refs,
};
use super::super::value_helpers::{
    parse_value_from_lexed, parse_where_x_greatest_commander_mana_value,
};
use super::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::dispatch_entry::{find_from_among_looked_cards_phrase, leading_may_actor_to_player};
use super::search_library::{
    normalize_search_library_filter, parse_search_library_disjunction_filter,
};
use crate::cards::builders::IT_TAG;
use crate::cards::builders::{
    CardTextError, EffectAst, ObjectFilter, PlayerAst, TagKey, TextSpan, parse_object_filter_lexed,
};
use crate::effect::Value;
use crate::target::TaggedOpbjectRelation;
use crate::types::CardType;
use crate::zone::Zone;
use ironsmith_core::{EffectMetric, EffectMetricSource, ValueSurfaceHint};

const CHOSEN_NAME_TAG: &str = "__chosen_name__";

const LOOKED_NUMBER_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["number", "of"]);
const LOOKED_THIS_WAY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["this", "way"]]);
const LOOKED_PUT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["put"]);
const LOOKED_THIS_SPELL_WAS_KICKED_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "this", "spell", "was", "kicked"]);
const LOOKED_THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const LOOKED_OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const LOOKED_THEM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["them"]);
const LOOKED_THOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["those"]);
const LOOKED_CARD_OF_YOUR_LIBRARY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["card", "of", "your", "library"],
            &["cards", "of", "your", "library"]
        ]
);
const LOOKED_WHERE_X_IS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["where", "x", "is"]);
const LOOKED_GREATEST_MANA_VALUE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "greatest", "mana", "value", "of"],
            &["greatest", "mana", "value", "of"],
        ]
);
const LOOKED_INTO_HAND_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["into"]; contains_words & ["hand"]);
const LOOKED_INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const LOOKED_IF_DONT_PUT_AMONG_INTO_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "if", "you", "dont", "put", "card", "from", "among", "them", "into", "your",
                "hand",
            ],
            &[
                "if", "you", "don't", "put", "card", "from", "among", "them", "into", "your",
                "hand",
            ],
            &[
                "if", "you", "do", "not", "put", "card", "from", "among", "them", "into", "your",
                "hand",
            ],
            &[
                "if", "you", "dont", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ],
            &[
                "if", "you", "don't", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ],
            &[
                "if", "you", "do", "not", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ],
        ]
);
const LOOKED_REST_BOTTOM_LIBRARY_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["rest", "bottom", "library"]);
const LOOKED_PUT_OR_PUTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["put"], &["puts"]]);
const LOOKED_OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const LOOKED_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const LOOKED_SAME_NAME_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["with", "that", "name"],
            &["with", "the", "chosen", "name"],
            &["with", "chosen", "name"],
        ]
);
const LOOKED_WITH_THE_CHOSEN_NAME_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["with", "the", "chosen", "name"]);
const LOOKED_CHOSEN_CARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["chosen", "card"], &["chosen", "cards"]]);
const LOOKED_CARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);

fn looked_clause_first_matches(clause: LexedClause<'_>, shape: &ClauseShape<'static>) -> bool {
    clause
        .word_refs()
        .first()
        .is_some_and(|word| shape.matches_word(word))
}

fn parse_prior_effect_number_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause = LexedClause::new(tokens).trimmed();
    let words = clause.word_refs();
    let mut idx = 0usize;
    if LOOKED_THE_WORD_PATTERN.matches_word_at(&words, idx) {
        idx += 1;
    }
    if !LOOKED_NUMBER_OF_PREFIX_PATTERN.matches_words(&words[idx..]) {
        return None;
    }
    let object_clause = clause
        .after_words(idx + 2)
        .unwrap_or_else(|| clause.from(clause.len()));
    let references_this_way = LOOKED_THIS_WAY_MARKER_PATTERN.matches(object_clause);
    let references_memory_action = object_clause.contains_any_word(&[
        "chosen",
        "destroyed",
        "discarded",
        "exiled",
        "milled",
        "revealed",
        "sacrificed",
        "searched",
    ]);
    if !references_this_way && !references_memory_action {
        return None;
    }
    Some(Value::PendingEffectMetric {
        source: if object_clause.contains_word("chosen") {
            EffectMetricSource::ChosenObjects
        } else {
            EffectMetricSource::AffectedObjects
        },
        metric: EffectMetric::Count,
    })
}

fn parse_prefixed_top_of_your_library_value<T: Copy>(
    tokens: &[OwnedLexToken],
    prefixes: &[(&[&str], T)],
) -> Option<(T, crate::effect::Value)> {
    let clause = LexedClause::new(tokens).trimmed();
    let (count_word_idx, marker) = prefixes.iter().find_map(|(prefix, marker)| {
        clause
            .starts_with(prefix)
            .then_some((prefix.len(), *marker))
    })?;
    let count_clause = clause.after_words(count_word_idx)?;
    let (count, used) = parse_number_or_x_value_lexed(count_clause.tokens())?;
    let tail_clause = count_clause.from(used).trimmed();
    let tail_words = tail_clause.word_refs();
    if !LOOKED_CARD_OF_YOUR_LIBRARY_PREFIX_PATTERN.matches_words(&tail_words) {
        return None;
    }

    if tail_words.len() == 4 {
        return Some((marker, count));
    }

    if count == crate::effect::Value::X
        && LOOKED_WHERE_X_IS_PREFIX_PATTERN.matches_words(&tail_words[4..])
    {
        let value_clause = tail_clause.after_words(7)?.trimmed();
        if let Some(resolved) = parse_prior_effect_number_value(value_clause.tokens()) {
            return Some((
                marker,
                resolved.with_surface_hint(ValueSurfaceHint::WhereXIs),
            ));
        }
        if let Some((resolved, used_value)) = parse_value_from_lexed(value_clause.tokens())
            && value_clause.from(used_value).trimmed().is_empty()
        {
            return Some((
                marker,
                resolved.with_surface_hint(ValueSurfaceHint::WhereXIs),
            ));
        }
        let value_word_refs = value_clause.word_refs();
        let commander_start = if LOOKED_GREATEST_MANA_VALUE_PREFIX_PATTERN
            .matches_words(&value_word_refs)
            && value_word_refs
                .first()
                .is_some_and(|word| LOOKED_THE_WORD_PATTERN.matches_word(word))
        {
            Some(5usize)
        } else if LOOKED_GREATEST_MANA_VALUE_PREFIX_PATTERN.matches_words(&value_word_refs) {
            Some(4usize)
        } else {
            None
        };
        if let Some(commander_start) = commander_start
            && let Some(resolved) =
                parse_where_x_greatest_commander_mana_value(value_clause.tokens(), commander_start)
        {
            return Some((
                marker,
                resolved.with_surface_hint(ValueSurfaceHint::WhereXIs),
            ));
        }
    }

    None
}

pub(crate) fn parse_top_cards_view_sentence(
    tokens: &[OwnedLexToken],
) -> Option<(PlayerAst, crate::effect::Value, bool)> {
    let (revealed, count) = parse_prefixed_top_of_your_library_value(
        tokens,
        &[
            (&["look", "at", "the", "top"][..], false),
            (&["look", "at", "top"][..], false),
            (&["reveal", "the", "top"][..], true),
            (&["reveal", "top"][..], true),
        ],
    )?;
    Some((PlayerAst::You, count, revealed))
}

fn strip_up_to_one_looked_card_choice_prefix(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some((count, used)) = parse_choice_count_token_prefix_consumed(clause.tokens()) else {
        return clause.trim();
    };
    if count == crate::effect::ChoiceCount::up_to(1) {
        clause.from(used).trim()
    } else {
        clause.trim()
    }
}

pub(crate) fn parse_looked_card_choice_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let filter_tokens = strip_up_to_one_looked_card_choice_prefix(tokens);
    if filter_tokens.is_empty() {
        return None;
    }
    let mut filter = parse_looked_card_reveal_filter(&filter_tokens)?;
    normalize_search_library_filter(&mut filter);
    filter.zone = None;
    Some(filter)
}

pub(crate) fn parse_counted_looked_cards_into_your_hand_tokens(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    let clause = LexedClause::new(tokens).trimmed();
    if !looked_clause_first_matches(clause, &LOOKED_PUT_WORD_PATTERN) {
        return None;
    }
    let count_clause = clause.after_words(1)?;
    let (count, used) = parse_number(count_clause.tokens())?;
    let tail_words = count_clause.from(used).word_refs();
    let mut idx = 0usize;
    if LOOKED_OF_WORD_PATTERN.matches_word_at(&tail_words, idx) {
        idx += 1;
    }
    match tail_words.get(idx).copied() {
        Some(word) if LOOKED_THEM_WORD_PATTERN.matches_word(word) => idx += 1,
        Some(word) if LOOKED_THOSE_WORD_PATTERN.matches_word(word) => {
            idx += 1;
            if LOOKED_CARD_PATTERN.matches_word_at(&tail_words, idx) {
                idx += 1;
            }
        }
        _ => return None,
    }
    if !LOOKED_INTO_HAND_PATTERN.matches_words(&tail_words[idx..]) {
        return None;
    }
    idx += 3;
    if idx == tail_words.len() {
        return Some(count);
    }
    if idx + 1 == tail_words.len() && LOOKED_INSTEAD_WORD_PATTERN.matches_word(tail_words[idx]) {
        return Some(count);
    }
    None
}

pub(crate) fn parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    let clause = LexedClause::new(tokens).trimmed();
    if !LOOKED_THIS_SPELL_WAS_KICKED_PREFIX_PATTERN.matches(clause) {
        return None;
    }
    let tail = clause.after_words(5)?.trimmed();
    parse_counted_looked_cards_into_your_hand_tokens(tail.tokens())
}

pub(crate) fn parse_may_put_filtered_looked_card_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool)>, CardTextError> {
    let sentence_clause = LexedClause::new(tokens).trimmed();
    let Some(action_match) =
        parse_leading_may_action_lexed(sentence_clause.tokens(), &["put"], false)
    else {
        return Ok(None);
    };
    let chooser = leading_may_actor_to_player(action_match.actor, PlayerAst::You);
    let action_clause = LexedClause::new(action_match.tail_tokens).trimmed();
    if action_clause.is_empty() {
        return Ok(None);
    }
    let action_words = action_clause.words();
    let action_word_refs = action_words.word_refs();
    let Some((from_among_word_idx, from_among_len)) =
        find_from_among_looked_cards_phrase(&action_words)
    else {
        return Ok(None);
    };
    let filter_clause = action_clause
        .before_word(from_among_word_idx)
        .unwrap_or_else(|| action_clause.before(action_clause.len()));
    let filter = if let Some(filter) = parse_looked_card_choice_filter(filter_clause.tokens()) {
        filter
    } else {
        return Ok(None);
    };
    let after_from_words = &action_word_refs[from_among_word_idx + from_among_len..];
    let tapped = match after_from_words {
        ["onto", "the", "battlefield"] | ["onto", "battlefield"] => false,
        ["onto", "the", "battlefield", "tapped"] | ["onto", "battlefield", "tapped"] => true,
        _ => return Ok(None),
    };
    Ok(Some((chooser, filter, tapped)))
}

fn parse_filtered_looked_card_into_hand_clause(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let action_clause = LexedClause::new(tokens).trimmed();
    if action_clause.is_empty() {
        return None;
    }
    let action_words = action_clause.words();
    let action_word_refs = action_words.word_refs();
    let Some((from_among_word_idx, from_among_len)) =
        find_from_among_looked_cards_phrase(&action_words)
    else {
        return None;
    };
    let filter_clause = action_clause
        .before_word(from_among_word_idx)
        .unwrap_or_else(|| action_clause.before(action_clause.len()));
    let filter = parse_looked_card_choice_filter(filter_clause.tokens())?;
    let after_from_words = &action_word_refs[from_among_word_idx + from_among_len..];
    let moves_into_hand = LOOKED_INTO_HAND_PATTERN.matches_words(after_from_words);
    if !moves_into_hand {
        return None;
    }
    Some(filter)
}

pub(crate) fn parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool, ObjectFilter)>, CardTextError> {
    let sentence_clause = LexedClause::new(tokens).trimmed();
    let Some(action_match) =
        parse_leading_may_action_lexed(sentence_clause.tokens(), &["put"], false)
    else {
        return Ok(None);
    };
    let chooser = leading_may_actor_to_player(action_match.actor, PlayerAst::You);
    let action_clause = LexedClause::new(action_match.tail_tokens).trimmed();
    if action_clause.is_empty() {
        return Ok(None);
    }
    let action_words = action_clause.words();
    let Some((from_among_word_idx, from_among_len)) =
        find_from_among_looked_cards_phrase(&action_words)
    else {
        return Ok(None);
    };
    let first_filter_clause = action_clause
        .before_word(from_among_word_idx)
        .unwrap_or_else(|| action_clause.before(action_clause.len()));
    let battlefield_filter = parse_looked_card_choice_filter(first_filter_clause.tokens())
        .ok_or_else(|| {
            CardTextError::ParseError("unable to parse first looked-card choice filter".to_string())
        })?;
    let after_first_clause = action_clause
        .after_words(from_among_word_idx + from_among_len)
        .unwrap_or_else(|| action_clause.from(action_clause.len()))
        .trimmed();
    let (tapped, second_start_words) = if after_first_clause.starts_with_any(&[
        &["onto", "the", "battlefield", "tapped", "and"],
        &["onto", "battlefield", "tapped", "and"],
    ]) {
        (true, 5usize)
    } else if after_first_clause.starts_with_any(&[
        &["onto", "the", "battlefield", "and"],
        &["onto", "battlefield", "and"],
    ]) {
        (false, 4usize)
    } else {
        return Ok(None);
    };
    let second_clause = after_first_clause
        .after_words(second_start_words)
        .unwrap_or_else(|| after_first_clause.from(after_first_clause.len()));
    let hand_filter = parse_filtered_looked_card_into_hand_clause(second_clause.tokens())
        .ok_or_else(|| {
            CardTextError::ParseError("unable to parse second looked-card hand filter".to_string())
        })?;
    Ok(Some((chooser, battlefield_filter, tapped, hand_filter)))
}

pub(crate) fn parse_if_you_dont_put_card_from_among_them_into_your_hand(
    tokens: &[OwnedLexToken],
) -> bool {
    let trimmed = LexedClause::new(tokens).trimmed();
    let words = crate::runtime_backend::util::non_article_token_word_refs(trimmed.tokens());
    LOOKED_IF_DONT_PUT_AMONG_INTO_HAND_PATTERN.matches_words(&words)
}

pub(crate) fn is_put_rest_on_bottom_of_library_sentence(tokens: &[OwnedLexToken]) -> bool {
    let clause = LexedClause::new(tokens).trimmed();
    clause
        .first_word()
        .is_some_and(|word| LOOKED_PUT_OR_PUTS_WORD_PATTERN.matches_word(word))
        && LOOKED_REST_BOTTOM_LIBRARY_PATTERN.matches(clause)
}

fn title_case_words(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut titled = String::new();
            titled.extend(first.to_uppercase());
            titled.push_str(chars.as_str());
            titled
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_named_card_filter_segment(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let all_segment_words = LexedClause::new(tokens).word_refs();
    let mut segment_words = strip_leading_article_word_refs(&all_segment_words).to_vec();
    if segment_words
        .last()
        .is_some_and(|word| LOOKED_CARD_PATTERN.matches_word(word))
    {
        segment_words.pop();
    }
    if segment_words.is_empty() {
        return None;
    }
    let mut filter = ObjectFilter::default();
    filter.name = Some(title_case_words(&segment_words));
    Some(filter)
}

fn split_reveal_filter_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current: Vec<OwnedLexToken> = Vec::new();
    let has_noncomparison_or = tokens.iter().enumerate().any(|(idx, token)| {
        LOOKED_OR_WORD_PATTERN.matches_token(token) && !is_comparison_or_delimiter(tokens, idx)
    });
    for (idx, token) in tokens.iter().enumerate() {
        let is_separator = (LOOKED_OR_WORD_PATTERN.matches_token(token)
            && !is_comparison_or_delimiter(tokens, idx))
            || (has_noncomparison_or && token.is_comma());
        if is_separator {
            while current
                .last()
                .is_some_and(|entry| LOOKED_AND_WORD_PATTERN.matches_token(entry))
            {
                current.pop();
            }
            let trimmed = LexedClause::new(&current).trim();
            if !trimmed.is_empty() {
                segments.push(trimmed);
            }
            current.clear();
            continue;
        }
        current.push(token.clone());
    }
    while current
        .last()
        .is_some_and(|entry| LOOKED_AND_WORD_PATTERN.matches_token(entry))
    {
        current.pop();
    }
    let trimmed = LexedClause::new(&current).trim();
    if !trimmed.is_empty() {
        segments.push(trimmed);
    }
    segments
}

pub(crate) fn parse_looked_card_reveal_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let raw_clause = LexedClause::new(tokens).trimmed();
    let raw_word_refs = raw_clause.word_refs();
    let same_name_suffix_len = if raw_word_refs.len() >= 4
        && LOOKED_SAME_NAME_SUFFIX_PATTERN.matches_words(&raw_word_refs)
        && LOOKED_WITH_THE_CHOSEN_NAME_SUFFIX_PATTERN.matches_words(&raw_word_refs)
    {
        Some(4usize)
    } else if raw_word_refs.len() >= 3
        && LOOKED_SAME_NAME_SUFFIX_PATTERN.matches_words(&raw_word_refs)
    {
        Some(3usize)
    } else {
        None
    };
    let filter_tokens = if let Some(suffix_len) = same_name_suffix_len {
        let keep_word_count = raw_word_refs.len().saturating_sub(suffix_len);
        raw_clause
            .before_word(keep_word_count)
            .unwrap_or_else(|| raw_clause.before(raw_clause.len()))
            .trim()
    } else {
        raw_clause.trim()
    };

    let filter_clause = LexedClause::new(&filter_tokens);
    let words_all_refs = filter_clause.word_refs();
    let non_article_words = crate::runtime_backend::util::non_article_word_refs(&words_all_refs);
    if LOOKED_CHOSEN_CARD_PATTERN.matches_words(&non_article_words) {
        let mut filter = ObjectFilter::default();
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
        return Some(filter);
    }
    if LOOKED_CARD_PATTERN.matches_words(&non_article_words) {
        let mut filter = ObjectFilter::default();
        if same_name_suffix_len.is_some() {
            filter = filter.match_tagged(
                TagKey::from(CHOSEN_NAME_TAG),
                TaggedOpbjectRelation::SameNameAsTagged,
            );
        }
        return Some(filter);
    }
    if matches!(
        non_article_words.as_slice(),
        ["card", "of", "chosen", "type"]
            | ["cards", "of", "chosen", "type"]
            | ["card", "of", "that", "type"]
            | ["cards", "of", "that", "type"]
    ) {
        let mut filter = ObjectFilter::default();
        filter.chosen_creature_type = true;
        if same_name_suffix_len.is_some() {
            filter = filter.match_tagged(
                TagKey::from(CHOSEN_NAME_TAG),
                TaggedOpbjectRelation::SameNameAsTagged,
            );
        }
        return Some(filter);
    }
    if matches!(
        non_article_words.as_slice(),
        ["permanent", "card"] | ["permanent", "cards"]
    ) {
        let mut filter = ObjectFilter::permanent_card();
        if same_name_suffix_len.is_some() {
            filter = filter.match_tagged(
                TagKey::from(CHOSEN_NAME_TAG),
                TaggedOpbjectRelation::SameNameAsTagged,
            );
        }
        return Some(filter);
    }
    if matches!(
        non_article_words.as_slice(),
        ["nonland", "permanent", "card"] | ["nonland", "permanent", "cards"]
    ) {
        let mut filter = ObjectFilter::permanent_card();
        filter.excluded_card_types.push(CardType::Land);
        if same_name_suffix_len.is_some() {
            filter = filter.match_tagged(
                TagKey::from(CHOSEN_NAME_TAG),
                TaggedOpbjectRelation::SameNameAsTagged,
            );
        }
        return Some(filter);
    }

    let has_noncomparison_or = filter_tokens.iter().enumerate().any(|(idx, token)| {
        LOOKED_OR_WORD_PATTERN.matches_token(token)
            && !is_comparison_or_delimiter(&filter_tokens, idx)
    });
    if has_noncomparison_or {
        let shared_card_suffix = words_all_refs
            .last()
            .is_some_and(|word| LOOKED_CARD_PATTERN.matches_word(word));
        let segments = split_reveal_filter_segments(&filter_tokens);
        if segments.len() >= 2 {
            let mut branches = Vec::new();
            for mut segment in segments {
                if shared_card_suffix
                    && !matches!(
                        segment.last().and_then(OwnedLexToken::as_word),
                        Some(word) if LOOKED_CARD_PATTERN.matches_word(word)
                    )
                {
                    segment.push(OwnedLexToken::word(
                        "card".to_string(),
                        TextSpan::synthetic(),
                    ));
                }
                let parsed = parse_object_filter_lexed(&segment, false)
                    .ok()
                    .filter(|filter| *filter != ObjectFilter::default())
                    .or_else(|| parse_named_card_filter_segment(&segment));
                let Some(parsed) = parsed else {
                    return None;
                };
                branches.push(parsed);
            }
            let mut filter = ObjectFilter::default();
            filter.any_of = branches;
            if same_name_suffix_len.is_some() {
                filter = filter.match_tagged(
                    TagKey::from(CHOSEN_NAME_TAG),
                    TaggedOpbjectRelation::SameNameAsTagged,
                );
            }
            return Some(filter);
        }
    }

    let mut filter = parse_search_library_disjunction_filter(&filter_tokens)
        .or_else(|| parse_object_filter_lexed(&filter_tokens, false).ok())?;
    if same_name_suffix_len.is_some() {
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
    }
    Some(filter)
}
