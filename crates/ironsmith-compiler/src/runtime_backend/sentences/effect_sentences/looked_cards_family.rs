use super::super::lexer::OwnedLexToken;
use super::super::object_filters::is_comparison_or_delimiter;
use super::super::token_primitives::{
    parse_leading_may_action_lexed, slice_contains, slice_starts_with, word_view_has_prefix,
};
use super::super::util::{is_article, parse_number, trim_commas};
use super::dispatch_entry::{
    find_from_among_looked_cards_phrase, leading_may_actor_to_player,
    parse_prefixed_top_of_your_library_count,
};
use super::search_library::{
    normalize_search_library_filter, parse_search_library_disjunction_filter,
};
use crate::cards::builders::IT_TAG;
use crate::cards::builders::compiler::grammar::primitives::TokenWordView;
use crate::cards::builders::{
    CardTextError, EffectAst, ObjectFilter, PlayerAst, TagKey, TextSpan, parse_object_filter_lexed,
};
use crate::target::TaggedOpbjectRelation;
use crate::zone::Zone;

const CHOSEN_NAME_TAG: &str = "__chosen_name__";

pub(crate) fn parse_top_cards_view_sentence(
    tokens: &[OwnedLexToken],
) -> Option<(PlayerAst, crate::effect::Value, bool)> {
    let (revealed, count) = parse_prefixed_top_of_your_library_count(
        tokens,
        &[
            (&["look", "at", "the", "top"][..], false),
            (&["look", "at", "top"][..], false),
            (&["reveal", "the", "top"][..], true),
            (&["reveal", "top"][..], true),
        ],
    )?;
    Some((
        PlayerAst::You,
        crate::effect::Value::Fixed(count as i32),
        revealed,
    ))
}

fn strip_up_to_one_looked_card_choice_prefix(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let tokens = trim_commas(tokens);
    let word_view = TokenWordView::new(&tokens);
    if !word_view.word_refs().starts_with(&["up", "to"]) {
        return tokens;
    }
    let count_start = word_view.token_index_after_words(2).unwrap_or(tokens.len());
    let count_tokens = trim_commas(&tokens[count_start..]);
    let Some((count, used)) = parse_number(&count_tokens) else {
        return tokens;
    };
    if count != 1 {
        return tokens;
    }
    trim_commas(&count_tokens[used..])
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
    let tokens = trim_commas(tokens);
    let word_view = TokenWordView::new(&tokens);
    if !word_view_has_prefix(&word_view, &["put"]) {
        return None;
    }
    let count_start = word_view.token_index_for_word_index(1)?;
    let count_tokens = &tokens[count_start..];
    let (count, used) = parse_number(count_tokens)?;
    let tail_word_view = TokenWordView::new(&count_tokens[used..]);
    let tail_words = tail_word_view.word_refs();
    let mut idx = 0usize;
    if tail_words.get(idx).copied() == Some("of") {
        idx += 1;
    }
    match tail_words.get(idx).copied() {
        Some("them") => idx += 1,
        Some("those") => {
            idx += 1;
            if matches!(tail_words.get(idx).copied(), Some("card" | "cards")) {
                idx += 1;
            }
        }
        _ => return None,
    }
    if tail_words.get(idx..idx + 3) != Some(&["into", "your", "hand"]) {
        return None;
    }
    idx += 3;
    if idx == tail_words.len() {
        return Some(count);
    }
    if idx + 1 == tail_words.len() && tail_words[idx] == "instead" {
        return Some(count);
    }
    None
}

pub(crate) fn parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    let trimmed = trim_commas(tokens);
    let clause_words = TokenWordView::new(&trimmed);
    if !word_view_has_prefix(&clause_words, &["if", "this", "spell", "was", "kicked"]) {
        return None;
    }
    let tail_start = clause_words
        .token_index_after_words(5)
        .unwrap_or(trimmed.len());
    let tail = trim_commas(&trimmed[tail_start..]);
    parse_counted_looked_cards_into_your_hand_tokens(&tail)
}

pub(crate) fn parse_may_put_filtered_looked_card_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let Some(action_match) = parse_leading_may_action_lexed(&sentence_tokens, &["put"], false)
    else {
        return Ok(None);
    };
    let chooser = leading_may_actor_to_player(action_match.actor, PlayerAst::You);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let action_words = TokenWordView::new(&action_tokens);
    if action_words.is_empty() {
        return Ok(None);
    }
    let action_word_refs = action_words.word_refs();
    let Some((from_among_word_idx, from_among_len)) =
        find_from_among_looked_cards_phrase(&action_words)
    else {
        return Ok(None);
    };
    let filter_end = action_words
        .token_index_for_word_index(from_among_word_idx)
        .unwrap_or(action_tokens.len());
    let filter = if let Some(filter) = parse_looked_card_choice_filter(&action_tokens[..filter_end])
    {
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
    let action_tokens = trim_commas(tokens);
    let action_words = TokenWordView::new(&action_tokens);
    if action_words.is_empty() {
        return None;
    }
    let action_word_refs = action_words.word_refs();
    let Some((from_among_word_idx, from_among_len)) =
        find_from_among_looked_cards_phrase(&action_words)
    else {
        return None;
    };
    let filter_end = action_words
        .token_index_for_word_index(from_among_word_idx)
        .unwrap_or(action_tokens.len());
    let filter = parse_looked_card_choice_filter(&action_tokens[..filter_end])?;
    let after_from_words = &action_word_refs[from_among_word_idx + from_among_len..];
    let moves_into_hand =
        slice_starts_with(after_from_words, &["into"]) && slice_contains(after_from_words, &"hand");
    if !moves_into_hand {
        return None;
    }
    Some(filter)
}

pub(crate) fn parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool, ObjectFilter)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let Some(action_match) = parse_leading_may_action_lexed(&sentence_tokens, &["put"], false)
    else {
        return Ok(None);
    };
    let chooser = leading_may_actor_to_player(action_match.actor, PlayerAst::You);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let action_words = TokenWordView::new(&action_tokens);
    if action_words.is_empty() {
        return Ok(None);
    }
    let Some((from_among_word_idx, from_among_len)) =
        find_from_among_looked_cards_phrase(&action_words)
    else {
        return Ok(None);
    };
    let first_filter_end = action_words
        .token_index_for_word_index(from_among_word_idx)
        .unwrap_or(action_tokens.len());
    let battlefield_filter = parse_looked_card_choice_filter(&action_tokens[..first_filter_end])
        .ok_or_else(|| {
            CardTextError::ParseError("unable to parse first looked-card choice filter".to_string())
        })?;
    let after_first_from = action_words
        .token_index_after_words(from_among_word_idx + from_among_len)
        .unwrap_or(action_tokens.len());
    let after_first_clause = trim_commas(&action_tokens[after_first_from..]);
    let after_words = TokenWordView::new(&after_first_clause);
    let after_refs = after_words.word_refs();
    let (tapped, second_start_words) =
        if slice_starts_with(
            &after_refs,
            &["onto", "the", "battlefield", "tapped", "and"],
        ) || slice_starts_with(&after_refs, &["onto", "battlefield", "tapped", "and"])
        {
            (true, 5usize)
        } else if slice_starts_with(&after_refs, &["onto", "the", "battlefield", "and"])
            || slice_starts_with(&after_refs, &["onto", "battlefield", "and"])
        {
            (false, 4usize)
        } else {
            return Ok(None);
        };
    let second_start = after_words
        .token_index_after_words(second_start_words)
        .unwrap_or(after_first_clause.len());
    let hand_filter =
        parse_filtered_looked_card_into_hand_clause(&after_first_clause[second_start..])
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to parse second looked-card hand filter".to_string(),
                )
            })?;
    Ok(Some((chooser, battlefield_filter, tapped, hand_filter)))
}

pub(crate) fn parse_if_you_dont_put_card_from_among_them_into_your_hand(
    tokens: &[OwnedLexToken],
) -> bool {
    let trimmed = trim_commas(tokens);
    let word_view = TokenWordView::new(&trimmed);
    let words: Vec<&str> = word_view
        .word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    words.as_slice()
        == [
            "if", "you", "dont", "put", "card", "from", "among", "them", "into", "your", "hand",
        ]
        || words.as_slice()
            == [
                "if", "you", "don't", "put", "card", "from", "among", "them", "into", "your",
                "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "do", "not", "put", "card", "from", "among", "them", "into", "your",
                "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "dont", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "don't", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "do", "not", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ]
}

pub(crate) fn is_put_rest_on_bottom_of_library_sentence(tokens: &[OwnedLexToken]) -> bool {
    let trimmed = trim_commas(tokens);
    let words = TokenWordView::new(&trimmed);
    matches!(words.first(), Some("put" | "puts"))
        && words.find_word("rest").is_some()
        && words.find_word("bottom").is_some()
        && words.find_word("library").is_some()
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
    let mut segment_words = crate::cards::builders::compiler::token_word_refs(tokens);
    while segment_words.first().is_some_and(|word| is_article(word)) {
        segment_words.remove(0);
    }
    if matches!(segment_words.last().copied(), Some("card" | "cards")) {
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
    let has_noncomparison_or = tokens
        .iter()
        .enumerate()
        .any(|(idx, token)| token.is_word("or") && !is_comparison_or_delimiter(tokens, idx));
    for (idx, token) in tokens.iter().enumerate() {
        let is_separator = (token.is_word("or") && !is_comparison_or_delimiter(tokens, idx))
            || (has_noncomparison_or && token.is_comma());
        if is_separator {
            while current.last().is_some_and(|entry| entry.is_word("and")) {
                current.pop();
            }
            let trimmed = trim_commas(&current);
            if !trimmed.is_empty() {
                segments.push(trimmed.to_vec());
            }
            current.clear();
            continue;
        }
        current.push(token.clone());
    }
    while current.last().is_some_and(|entry| entry.is_word("and")) {
        current.pop();
    }
    let trimmed = trim_commas(&current);
    if !trimmed.is_empty() {
        segments.push(trimmed.to_vec());
    }
    segments
}

pub(crate) fn parse_looked_card_reveal_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut filter_tokens = trim_commas(tokens).to_vec();
    let raw_word_view = TokenWordView::new(&filter_tokens);
    let raw_word_refs = raw_word_view.word_refs();
    let same_name_suffix_len = if raw_word_refs.len() >= 3
        && raw_word_refs[raw_word_refs.len() - 3..] == ["with", "that", "name"]
    {
        Some(3usize)
    } else if raw_word_refs.len() >= 4
        && raw_word_refs[raw_word_refs.len() - 4..] == ["with", "the", "chosen", "name"]
    {
        Some(4usize)
    } else if raw_word_refs.len() >= 3
        && raw_word_refs[raw_word_refs.len() - 3..] == ["with", "chosen", "name"]
    {
        Some(3usize)
    } else {
        None
    };
    if let Some(suffix_len) = same_name_suffix_len {
        let keep_word_count = raw_word_refs.len().saturating_sub(suffix_len);
        let base_end = raw_word_view
            .token_index_after_words(keep_word_count)
            .unwrap_or(filter_tokens.len());
        filter_tokens = trim_commas(&filter_tokens[..base_end]).to_vec();
    }

    let words_all = TokenWordView::new(&filter_tokens);
    let words_all_refs = words_all.word_refs();
    let non_article_words = words_all_refs
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    if matches!(
        non_article_words.as_slice(),
        ["chosen", "card"] | ["chosen", "cards"]
    ) {
        let mut filter = ObjectFilter::default();
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
        return Some(filter);
    }
    if matches!(non_article_words.as_slice(), ["card"] | ["cards"]) {
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
        words_all_refs.as_slice(),
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

    let has_noncomparison_or = filter_tokens.iter().enumerate().any(|(idx, token)| {
        token.is_word("or") && !is_comparison_or_delimiter(&filter_tokens, idx)
    });
    if has_noncomparison_or {
        let shared_card_suffix = matches!(words_all_refs.last().copied(), Some("card" | "cards"));
        let segments = split_reveal_filter_segments(&filter_tokens);
        if segments.len() >= 2 {
            let mut branches = Vec::new();
            for mut segment in segments {
                if shared_card_suffix
                    && !matches!(
                        segment.last().and_then(OwnedLexToken::as_word),
                        Some("card" | "cards")
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
