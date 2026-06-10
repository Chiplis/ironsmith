use super::super::lexer::{
    LexedClause, OwnedLexToken, word_slice_contains_all_words, word_slice_contains_phrase,
    word_slice_eq_any, word_slice_first_is, word_slice_starts_with, word_slice_starts_with_any,
};
use super::super::object_filters::is_comparison_or_delimiter;
use super::super::token_primitives::parse_leading_may_action_lexed;
use super::super::util::{
    is_article, parse_choice_count_token_prefix_consumed, parse_number,
    parse_number_or_x_value_lexed, strip_leading_article_word_refs,
};
use super::super::value_helpers::{
    parse_value_from_lexed, parse_where_x_greatest_commander_mana_value,
};
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

const LOOKED_NUMBER_OF_PREFIX: &[&str] = &["number", "of"];
const LOOKED_THIS_WAY_PHRASE: &[&str] = &["this", "way"];
const LOOKED_PUT_WORD: &str = "put";
const LOOKED_THIS_SPELL_WAS_KICKED_PREFIX: &[&str] = &["if", "this", "spell", "was", "kicked"];
const LOOKED_THE_WORD: &str = "the";
const LOOKED_OF_WORD: &str = "of";
const LOOKED_THEM_WORD: &str = "them";
const LOOKED_THOSE_WORD: &str = "those";
const LOOKED_CARD_OF_YOUR_LIBRARY_PREFIXES: &[&[&str]] = &[
    &["card", "of", "your", "library"],
    &["cards", "of", "your", "library"],
];
const LOOKED_WHERE_X_IS_PREFIX: &[&str] = &["where", "x", "is"];
const LOOKED_GREATEST_MANA_VALUE_PREFIXES: &[&[&str]] = &[
    &["the", "greatest", "mana", "value", "of"],
    &["greatest", "mana", "value", "of"],
];
const LOOKED_INTO_HAND_PREFIX: &[&str] = &["into"];
const LOOKED_INTO_HAND_REQUIRED_WORDS: &[&str] = &["hand"];
const LOOKED_INSTEAD_WORD: &str = "instead";
const LOOKED_IF_DONT_PUT_AMONG_INTO_HAND_PHRASES: &[&[&str]] = &[
    &[
        "if", "you", "dont", "put", "card", "from", "among", "them", "into", "your", "hand",
    ],
    &[
        "if", "you", "don't", "put", "card", "from", "among", "them", "into", "your", "hand",
    ],
    &[
        "if", "you", "do", "not", "put", "card", "from", "among", "them", "into", "your", "hand",
    ],
    &[
        "if", "you", "dont", "put", "card", "from", "among", "those", "cards", "into", "your",
        "hand",
    ],
    &[
        "if", "you", "don't", "put", "card", "from", "among", "those", "cards", "into", "your",
        "hand",
    ],
    &[
        "if", "you", "do", "not", "put", "card", "from", "among", "those", "cards", "into", "your",
        "hand",
    ],
];
const LOOKED_REST_BOTTOM_LIBRARY_REQUIRED_WORDS: &[&str] = &["rest", "bottom", "library"];
const LOOKED_PUT_OR_PUTS_WORDS: &[&str] = &["put", "puts"];
const LOOKED_OR_WORD: &str = "or";
const LOOKED_AND_WORD: &str = "and";
const LOOKED_WITH_THE_CHOSEN_NAME_SUFFIX: &[&str] = &["with", "the", "chosen", "name"];
const LOOKED_SAME_NAME_SUFFIXES: &[&[&str]] = &[
    &["with", "that", "name"],
    LOOKED_WITH_THE_CHOSEN_NAME_SUFFIX,
    &["with", "chosen", "name"],
];
const LOOKED_CHOSEN_CARD_PHRASES: &[&[&str]] = &[&["chosen", "card"], &["chosen", "cards"]];
const LOOKED_CARD_WORDS: &[&str] = &["card", "cards"];

fn looked_clause_first_is(clause: LexedClause<'_>, expected: &str) -> bool {
    word_slice_first_is(&clause.word_refs(), expected)
}

fn looked_words_start_into_hand(words: &[&str]) -> bool {
    word_slice_starts_with(words, LOOKED_INTO_HAND_PREFIX)
        && word_slice_contains_all_words(words, LOOKED_INTO_HAND_REQUIRED_WORDS)
}

fn token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word() == Some(expected)
}

fn token_words_non_article_eq_any(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    let words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    word_slice_eq_any(words.as_slice(), expected)
}

fn parse_prior_effect_number_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause = LexedClause::new(tokens).trimmed();
    let words = clause.word_refs();
    let mut idx = 0usize;
    if words.get(idx) == Some(&LOOKED_THE_WORD) {
        idx += 1;
    }
    if !clause
        .from_word(idx)
        .is_some_and(|tail| word_slice_starts_with(&tail.word_refs(), LOOKED_NUMBER_OF_PREFIX))
    {
        return None;
    }
    let object_clause = clause
        .after_words(idx + 2)
        .unwrap_or_else(|| clause.from(clause.len()));
    let references_this_way =
        word_slice_contains_phrase(&object_clause.word_refs(), LOOKED_THIS_WAY_PHRASE);
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
    if !word_slice_starts_with_any(&tail_clause.word_refs(), LOOKED_CARD_OF_YOUR_LIBRARY_PREFIXES) {
        return None;
    }

    if tail_words.len() == 4 {
        return Some((marker, count));
    }

    if count == crate::effect::Value::X
        && tail_clause.from_word(4).is_some_and(|where_tail| {
            word_slice_starts_with(&where_tail.word_refs(), LOOKED_WHERE_X_IS_PREFIX)
        })
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
        let has_greatest_mana_value_prefix =
            word_slice_starts_with_any(&value_word_refs, LOOKED_GREATEST_MANA_VALUE_PREFIXES);
        let commander_start = if has_greatest_mana_value_prefix
            && word_slice_first_is(&value_word_refs, LOOKED_THE_WORD)
        {
            Some(5usize)
        } else if has_greatest_mana_value_prefix {
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
    if !looked_clause_first_is(clause, LOOKED_PUT_WORD) {
        return None;
    }
    let count_clause = clause.after_words(1)?;
    let (count, used) = parse_number(count_clause.tokens())?;
    let tail_words = count_clause.from(used).word_refs();
    let tail_clause = count_clause.from(used);
    let mut idx = 0usize;
    if tail_words.get(idx) == Some(&LOOKED_OF_WORD) {
        idx += 1;
    }
    match tail_words.get(idx).copied() {
        Some(word) if word == LOOKED_THEM_WORD => idx += 1,
        Some(word) if word == LOOKED_THOSE_WORD => {
            idx += 1;
            if tail_words
                .get(idx)
                .is_some_and(|word| LOOKED_CARD_WORDS.contains(word))
            {
                idx += 1;
            }
        }
        _ => return None,
    }
    if !tail_clause
        .from_word(idx)
        .is_some_and(|tail| looked_words_start_into_hand(&tail.word_refs()))
    {
        return None;
    }
    idx += 3;
    if idx == tail_words.len() {
        return Some(count);
    }
    if idx + 1 == tail_words.len() && tail_words[idx] == LOOKED_INSTEAD_WORD {
        return Some(count);
    }
    None
}

pub(crate) fn parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    let clause = LexedClause::new(tokens).trimmed();
    if !word_slice_starts_with(&clause.word_refs(), LOOKED_THIS_SPELL_WAS_KICKED_PREFIX) {
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
    let Some((from_among_word_idx, from_among_len)) =
        find_from_among_looked_cards_phrase(&action_words)
    else {
        return None;
    };
    let filter_clause = action_clause
        .before_word(from_among_word_idx)
        .unwrap_or_else(|| action_clause.before(action_clause.len()));
    let filter = parse_looked_card_choice_filter(filter_clause.tokens())?;
    let moves_into_hand = action_clause
        .from_word(from_among_word_idx + from_among_len)
        .is_some_and(|tail| looked_words_start_into_hand(&tail.word_refs()));
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
    token_words_non_article_eq_any(trimmed.tokens(), LOOKED_IF_DONT_PUT_AMONG_INTO_HAND_PHRASES)
}

pub(crate) fn is_put_rest_on_bottom_of_library_sentence(tokens: &[OwnedLexToken]) -> bool {
    let clause = LexedClause::new(tokens).trimmed();
    clause
        .first_word()
        .is_some_and(|word| LOOKED_PUT_OR_PUTS_WORDS.contains(&word))
        && word_slice_contains_all_words(
            &clause.word_refs(),
            LOOKED_REST_BOTTOM_LIBRARY_REQUIRED_WORDS,
        )
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
        .is_some_and(|word| LOOKED_CARD_WORDS.contains(word))
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
        token_is_word(token, LOOKED_OR_WORD) && !is_comparison_or_delimiter(tokens, idx)
    });
    for (idx, token) in tokens.iter().enumerate() {
        let is_separator = (token_is_word(token, LOOKED_OR_WORD)
            && !is_comparison_or_delimiter(tokens, idx))
            || (has_noncomparison_or && token.is_comma());
        if is_separator {
            while current
                .last()
                .is_some_and(|entry| token_is_word(entry, LOOKED_AND_WORD))
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
        .is_some_and(|entry| token_is_word(entry, LOOKED_AND_WORD))
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
    let same_name_suffix = {
        LOOKED_SAME_NAME_SUFFIXES
            .iter()
            .any(|suffix| raw_word_refs.ends_with(suffix))
    };
    let filter_tokens = if same_name_suffix {
        let suffix_len = LOOKED_SAME_NAME_SUFFIXES
            .iter()
            .find(|suffix| raw_word_refs.ends_with(suffix))
            .map(|suffix| suffix.len())
            .unwrap_or(0);
        raw_clause
            .before_word(raw_word_refs.len() - suffix_len)
            .unwrap_or(raw_clause)
            .trim()
    } else {
        raw_clause.trim()
    };

    let filter_clause = LexedClause::new(&filter_tokens);
    let words_all_refs = filter_clause.word_refs();
    let non_article_words = crate::runtime_backend::util::non_article_word_refs(&words_all_refs);
    if token_words_non_article_eq_any(&filter_tokens, LOOKED_CHOSEN_CARD_PHRASES) {
        let mut filter = ObjectFilter::default();
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
        return Some(filter);
    }
    if crate::runtime_backend::util::non_article_token_word_refs(&filter_tokens)
        .iter()
        .all(|word| LOOKED_CARD_WORDS.contains(word))
        && crate::runtime_backend::util::non_article_token_word_refs(&filter_tokens).len() == 1
    {
        let mut filter = ObjectFilter::default();
        if same_name_suffix {
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
        if same_name_suffix {
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
        if same_name_suffix {
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
        if same_name_suffix {
            filter = filter.match_tagged(
                TagKey::from(CHOSEN_NAME_TAG),
                TaggedOpbjectRelation::SameNameAsTagged,
            );
        }
        return Some(filter);
    }

    // "<modifiers> permanent card(s)" (e.g. "snow permanent cards"): parse the
    // modifiers with "permanent" elided, then restrict to permanent card types
    // so the permanent-ness isn't silently dropped by the generic noun parse.
    if non_article_words.len() > 2
        && non_article_words[non_article_words.len() - 2] == "permanent"
        && LOOKED_CARD_WORDS.contains(&non_article_words[non_article_words.len() - 1])
    {
        let mut elided_tokens = filter_tokens.to_vec();
        if let Some(permanent_idx) = elided_tokens
            .iter()
            .rposition(|token| token_is_word(token, "permanent"))
        {
            elided_tokens.remove(permanent_idx);
            if let Some(mut filter) = parse_object_filter_lexed(&elided_tokens, false)
                .ok()
                .filter(|filter| filter.card_types.is_empty() && filter.all_card_types.is_empty())
            {
                filter.card_types = ObjectFilter::permanent_card().card_types;
                if same_name_suffix {
                    filter = filter.match_tagged(
                        TagKey::from(CHOSEN_NAME_TAG),
                        TaggedOpbjectRelation::SameNameAsTagged,
                    );
                }
                return Some(filter);
            }
        }
    }

    let has_noncomparison_or = filter_tokens.iter().enumerate().any(|(idx, token)| {
        token_is_word(token, LOOKED_OR_WORD) && !is_comparison_or_delimiter(&filter_tokens, idx)
    });
    if has_noncomparison_or {
        let shared_card_suffix = words_all_refs
            .last()
            .is_some_and(|word| LOOKED_CARD_WORDS.contains(word));
        let segments = split_reveal_filter_segments(&filter_tokens);
        if segments.len() >= 2 {
            let mut branches = Vec::new();
            for mut segment in segments {
                if shared_card_suffix
                    && !matches!(
                        segment.last().and_then(OwnedLexToken::as_word),
                        Some(word) if LOOKED_CARD_WORDS.contains(&word)
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
            if same_name_suffix {
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
    if same_name_suffix {
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
    }
    Some(filter)
}
