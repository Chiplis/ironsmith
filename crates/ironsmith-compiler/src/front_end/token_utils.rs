use crate::diagnostics::TextSpan;

use super::lexer::{
    OwnedLexToken, TokenKind, TokenWordView, contains_token_kind, find_token_kind,
    render_token_slice,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnDurationPhrase {
    ThisTurn,
    UntilEndOfTurn,
    UntilYourNextTurn,
    UntilYourNextTurnEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonSentenceHead {
    ForEach,
    If,
    Until,
    WhereXIs,
    Target,
    CountPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeadingMayActor {
    You,
    ThatPlayer,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeadingMayActionMatch<'a> {
    pub actor: LeadingMayActor,
    pub verb: &'static str,
    pub tail_tokens: &'a [OwnedLexToken],
}

pub fn slice_starts_with<T: PartialEq>(items: &[T], prefix: &[T]) -> bool {
    crate::slice_primitives::starts_with(items, prefix)
}

pub fn slice_ends_with<T: PartialEq>(items: &[T], suffix: &[T]) -> bool {
    crate::slice_primitives::ends_with(items, suffix)
}

pub fn slice_ends_with_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
    crate::slice_primitives::ends_with_any(items, patterns)
}

pub fn slice_contains<T: PartialEq>(items: &[T], expected: &T) -> bool {
    crate::slice_primitives::contains(items, expected)
}

pub fn slice_contains_any<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    crate::slice_primitives::contains_any(items, expected)
}

pub fn slice_contains_all<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    crate::slice_primitives::contains_all(items, expected)
}

pub fn slice_eq_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
    crate::slice_primitives::equals_any(items, patterns)
}

pub fn slice_starts_with_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
    crate::slice_primitives::starts_with_any(items, patterns)
}

pub fn iter_contains<I, T>(items: I, expected: &T) -> bool
where
    I: IntoIterator,
    I::Item: std::borrow::Borrow<T>,
    T: PartialEq + ?Sized,
{
    crate::slice_primitives::iter_contains(items, expected)
}

pub fn iter_eq<I, J>(left: I, right: J) -> bool
where
    I: IntoIterator,
    J: IntoIterator,
    I::Item: PartialEq<J::Item>,
{
    crate::slice_primitives::iter_eq(left, right)
}

pub fn slice_strip_prefix<'a, T: PartialEq>(items: &'a [T], prefix: &[T]) -> Option<&'a [T]> {
    crate::slice_primitives::strip_prefix(items, prefix)
}

pub fn slice_strip_suffix<'a, T: PartialEq>(items: &'a [T], suffix: &[T]) -> Option<&'a [T]> {
    crate::slice_primitives::strip_suffix(items, suffix)
}

pub fn slice_strip_any_prefix<'a, 'p, T: PartialEq>(
    items: &'a [T],
    patterns: &'p [&'p [T]],
) -> Option<(&'p [T], &'a [T])> {
    crate::slice_primitives::strip_any_prefix(items, patterns)
}

pub fn slice_strip_any_suffix<'a, 'p, T: PartialEq>(
    items: &'a [T],
    patterns: &'p [&'p [T]],
) -> Option<(&'p [T], &'a [T])> {
    crate::slice_primitives::strip_any_suffix(items, patterns)
}

pub fn find_index<T>(items: &[T], predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    crate::slice_primitives::find_index(items, predicate)
}

pub fn rfind_index<T>(items: &[T], predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    crate::slice_primitives::rfind_index(items, predicate)
}

pub fn find_window_index<T: PartialEq>(items: &[T], window: &[T]) -> Option<usize> {
    crate::slice_primitives::find_window_index(items, window)
}

pub fn find_window_by<T>(
    items: &[T],
    window_len: usize,
    predicate: impl FnMut(&[T]) -> bool,
) -> Option<usize> {
    crate::slice_primitives::find_window_by(items, window_len, predicate)
}

pub fn contains_sequence<T: PartialEq>(items: &[T], window: &[T]) -> bool {
    crate::slice_primitives::contains_sequence(items, window)
}

pub use contains_sequence as contains_window;

pub fn str_contains(text: &str, needle: &str) -> bool {
    crate::string_primitives::contains(text, needle)
}

pub fn str_contains_char(text: &str, needle: char) -> bool {
    crate::string_primitives::contains_char(text, needle)
}

pub fn str_starts_with(text: &str, prefix: &str) -> bool {
    crate::string_primitives::starts_with(text, prefix)
}

pub fn str_starts_with_char(text: &str, expected: char) -> bool {
    crate::string_primitives::starts_with_char(text, expected)
}

pub fn str_ends_with(text: &str, suffix: &str) -> bool {
    crate::string_primitives::ends_with(text, suffix)
}

pub fn str_ends_with_char(text: &str, expected: char) -> bool {
    crate::string_primitives::ends_with_char(text, expected)
}

pub fn str_ends_with_any_char(text: &str, expected: &[char]) -> bool {
    crate::string_primitives::ends_with_any_char(text, expected)
}

pub fn str_find(text: &str, needle: &str) -> Option<usize> {
    crate::string_primitives::find(text, needle)
}

pub fn str_find_char(text: &str, needle: char) -> Option<usize> {
    crate::string_primitives::find_char(text, needle)
}

pub fn str_rfind(text: &str, needle: &str) -> Option<usize> {
    crate::string_primitives::rfind(text, needle)
}

pub fn str_rfind_char(text: &str, needle: char) -> Option<usize> {
    crate::string_primitives::rfind_char(text, needle)
}

pub fn str_strip_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    crate::string_primitives::strip_prefix(text, prefix)
}

pub fn str_strip_suffix<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    crate::string_primitives::strip_suffix(text, suffix)
}

pub fn str_strip_suffix_char(text: &str, suffix: char) -> Option<&str> {
    crate::string_primitives::strip_suffix_char(text, suffix)
}

pub fn str_split_once<'a>(text: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    crate::string_primitives::split_once(text, needle)
}

pub fn str_split_once_char<'a>(text: &'a str, needle: char) -> Option<(&'a str, &'a str)> {
    crate::string_primitives::split_once_char(text, needle)
}

pub fn word_view_has_prefix(words: &TokenWordView<'_>, prefix: &[&str]) -> bool {
    words.len() >= prefix.len() && words.slice_eq(0, prefix)
}

pub fn word_view_has_any_prefix(words: &TokenWordView<'_>, prefixes: &[&[&str]]) -> bool {
    prefixes
        .iter()
        .any(|prefix| word_view_has_prefix(words, prefix))
}

pub fn rewrite_followup_intro_to_if_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut rewritten = tokens.to_vec();
    let words = TokenWordView::new(&rewritten);
    if !word_view_has_any_prefix(
        &words,
        &[&["when", "you", "do"], &["whenever", "you", "do"]],
    ) {
        return rewritten;
    }

    let Some(first_word_idx) = words.token_index_for_word_index(0) else {
        return rewritten;
    };
    rewritten[first_word_idx].replace_word("if");
    rewritten
}

fn token_range_for_word_span(
    tokens: &[OwnedLexToken],
    words: &TokenWordView<'_>,
    start_word_idx: usize,
    word_len: usize,
) -> Option<(usize, usize)> {
    let start = if start_word_idx == 0 {
        0
    } else {
        words.token_index_after_words(start_word_idx)?
    };
    let end = words.token_index_after_words(start_word_idx + word_len)?;
    (start <= end && end <= tokens.len()).then_some((start, end))
}

pub fn remove_copy_exception_type_removal_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    const PATTERNS: &[(&[&str], usize)] = &[
        (
            &[
                "except", "its", "an", "artifact", "and", "it", "loses", "all", "other", "card",
                "types",
            ],
            4,
        ),
        (
            &[
                "except",
                "its",
                "an",
                "enchantment",
                "and",
                "it",
                "loses",
                "all",
                "other",
                "card",
                "types",
            ],
            4,
        ),
        (
            &[
                "except",
                "its",
                "an",
                "enchantment",
                "and",
                "loses",
                "all",
                "other",
                "card",
                "types",
            ],
            4,
        ),
    ];

    let mut rewritten = tokens.to_vec();
    loop {
        let words = TokenWordView::new(&rewritten);
        let mut removed_any = false;
        for (pattern, keep_words) in PATTERNS {
            let Some(start_word_idx) = words.find_phrase_start(pattern) else {
                continue;
            };
            let Some((remove_start, remove_end)) = token_range_for_word_span(
                &rewritten,
                &words,
                start_word_idx + keep_words,
                pattern.len() - keep_words,
            ) else {
                continue;
            };
            rewritten.drain(remove_start..remove_end);
            removed_any = true;
            break;
        }
        if !removed_any {
            break;
        }
    }
    rewritten
}

pub fn lexed_tokens_contain_non_prefix_instead(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens);
    words.find_word("instead").is_some() && !word_view_has_prefix(&words, &["if"])
}

pub fn strip_leading_if_you_do_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let words = TokenWordView::new(tokens);
    let Some(prefix_len) = (word_view_has_prefix(&words, &["if", "you", "do"]).then_some(3usize))
        .or_else(|| word_view_has_prefix(&words, &["if", "they", "do"]).then_some(3usize))
    else {
        return tokens;
    };
    let start = words
        .token_index_after_words(prefix_len)
        .unwrap_or(tokens.len());
    &tokens[start..]
}

fn find_token_index_with_span(tokens: &[OwnedLexToken], span: TextSpan) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if tokens[idx].span() == span {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

pub fn clone_sentence_chunk_tokens(
    tokens: &[OwnedLexToken],
    sentences: &[&[OwnedLexToken]],
) -> Option<Vec<OwnedLexToken>> {
    let first = sentences.first()?.first()?;
    let last_sentence = sentences.last()?;
    let last_first = last_sentence.first()?;
    let start = find_token_index_with_span(tokens, first.span())?;
    let end_start = find_token_index_with_span(tokens, last_first.span())?;
    Some(tokens[start..end_start + last_sentence.len()].to_vec())
}

pub fn split_em_dash_label_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let mut inside_quotes = false;
    let split_idx = tokens.iter().enumerate().find_map(|(idx, token)| {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            return None;
        }
        (token.kind == TokenKind::EmDash && !inside_quotes).then_some(idx)
    })?;
    let label_tokens = &tokens[..split_idx];
    let body_tokens = &tokens[split_idx + 1..];
    if label_tokens.is_empty()
        || body_tokens.is_empty()
        || contains_token_kind(label_tokens, TokenKind::Period)
    {
        return None;
    }

    let body = render_token_slice(body_tokens);
    if body.trim().is_empty() {
        return None;
    }

    Some((label_tokens, body_tokens))
}

pub fn split_em_dash_label_prefix(tokens: &[OwnedLexToken]) -> Option<(String, &[OwnedLexToken])> {
    let (label_tokens, body_tokens) = split_em_dash_label_prefix_tokens(tokens)?;
    let label = render_token_slice(label_tokens).trim().to_string();
    (!label.is_empty()).then_some((label, body_tokens))
}

pub fn parse_leading_may_action_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    verbs: &'static [&'static str],
    allow_bare: bool,
) -> Option<LeadingMayActionMatch<'a>> {
    let words = TokenWordView::new(tokens);
    if words.is_empty() {
        return None;
    }

    for (actor, verb_word_idx, prefix) in [
        (LeadingMayActor::You, 2usize, &["you", "may"][..]),
        (
            LeadingMayActor::ThatPlayer,
            3usize,
            &["that", "player", "may"][..],
        ),
        (LeadingMayActor::ThatPlayer, 2usize, &["they", "may"][..]),
        (LeadingMayActor::Default, 1usize, &["may"][..]),
    ] {
        if !word_view_has_prefix(&words, prefix) {
            continue;
        }
        for verb in verbs {
            if !words.slice_eq(verb_word_idx, &[*verb]) {
                continue;
            }
            let tail_start = words
                .token_index_after_words(verb_word_idx + 1)
                .unwrap_or(tokens.len());
            return Some(LeadingMayActionMatch {
                actor,
                verb,
                tail_tokens: &tokens[tail_start..],
            });
        }
        return None;
    }

    if allow_bare {
        for verb in verbs {
            if !words.slice_eq(0, &[*verb]) {
                continue;
            }
            let tail_start = words.token_index_after_words(1).unwrap_or(tokens.len());
            return Some(LeadingMayActionMatch {
                actor: LeadingMayActor::Default,
                verb,
                tail_tokens: &tokens[tail_start..],
            });
        }
    }

    None
}

pub fn lexed_head_words(tokens: &[OwnedLexToken]) -> Option<(&str, Option<&str>)> {
    let mut words = tokens.iter().filter_map(OwnedLexToken::as_word);
    let first = words.next()?;
    Some((first, words.next()))
}

pub fn parse_common_sentence_head(
    tokens: &[OwnedLexToken],
) -> Option<(CommonSentenceHead, &[OwnedLexToken])> {
    let words = TokenWordView::new(tokens);
    let (head, _) = lexed_head_words(tokens)?;
    let consumed_words = match head {
        "for" if words.starts_with(&["for", "each"]) => (CommonSentenceHead::ForEach, 2usize),
        "each" => (CommonSentenceHead::ForEach, 1usize),
        "if" => (CommonSentenceHead::If, 1usize),
        "until" => (CommonSentenceHead::Until, 1usize),
        "where" if words.starts_with(&["where", "x", "is"]) => {
            (CommonSentenceHead::WhereXIs, 3usize)
        }
        "target" => (CommonSentenceHead::Target, 1usize),
        "up" if words.starts_with(&["up", "to"]) => (CommonSentenceHead::CountPrefix, 2usize),
        "one" if words.starts_with_any(&[&["one", "or", "more"], &["one", "or", "both"]]) => {
            (CommonSentenceHead::CountPrefix, 3usize)
        }
        "a" | "an" => (CommonSentenceHead::CountPrefix, 1usize),
        _ => return None,
    };
    let rest_start = words
        .token_index_after_words(consumed_words.1)
        .unwrap_or(tokens.len());
    Some((consumed_words.0, &tokens[rest_start..]))
}

pub fn split_lexed_once_on_delimiter(
    tokens: &[OwnedLexToken],
    delimiter: TokenKind,
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let split_idx = find_token_kind(tokens, delimiter)?;
    Some((&tokens[..split_idx], &tokens[split_idx + 1..]))
}

pub fn split_lexed_once_on_comma(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
}

pub fn split_lexed_once_on_period(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    split_lexed_once_on_delimiter(tokens, TokenKind::Period)
}

pub fn split_lexed_once_on_comma_then(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let mut idx = 0usize;
    while idx + 1 < tokens.len() {
        if tokens[idx].kind == TokenKind::Comma && tokens[idx + 1].is_word("then") {
            return Some((&tokens[..idx], &tokens[idx + 2..]));
        }
        idx += 1;
    }
    None
}

pub fn parse_turn_duration_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(TurnDurationPhrase, &[OwnedLexToken])> {
    const PHRASES: &[(&[&str], TurnDurationPhrase)] = &[
        (
            &["until", "your", "next", "turn"],
            TurnDurationPhrase::UntilYourNextTurn,
        ),
        (
            &["until", "the", "end", "of", "your", "next", "turn"],
            TurnDurationPhrase::UntilYourNextTurnEnd,
        ),
        (
            &["until", "end", "of", "your", "next", "turn"],
            TurnDurationPhrase::UntilYourNextTurnEnd,
        ),
        (
            &["until", "the", "end", "of", "turn"],
            TurnDurationPhrase::UntilEndOfTurn,
        ),
        (
            &["until", "end", "of", "turn"],
            TurnDurationPhrase::UntilEndOfTurn,
        ),
        (&["this", "turn"], TurnDurationPhrase::ThisTurn),
    ];

    let words = TokenWordView::new(tokens);
    for (phrase, kind) in PHRASES {
        if words.starts_with(phrase) {
            let rest_start = words
                .token_index_after_words(phrase.len())
                .unwrap_or(tokens.len());
            return Some((*kind, &tokens[rest_start..]));
        }
    }
    None
}

pub fn parse_turn_duration_suffix(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], TurnDurationPhrase)> {
    const PHRASES: &[(&[&str], TurnDurationPhrase)] = &[
        (
            &["until", "your", "next", "turn"],
            TurnDurationPhrase::UntilYourNextTurn,
        ),
        (
            &["until", "the", "end", "of", "your", "next", "turn"],
            TurnDurationPhrase::UntilYourNextTurnEnd,
        ),
        (
            &["until", "end", "of", "your", "next", "turn"],
            TurnDurationPhrase::UntilYourNextTurnEnd,
        ),
        (
            &["until", "the", "end", "of", "turn"],
            TurnDurationPhrase::UntilEndOfTurn,
        ),
        (
            &["until", "end", "of", "turn"],
            TurnDurationPhrase::UntilEndOfTurn,
        ),
        (&["this", "turn"], TurnDurationPhrase::ThisTurn),
    ];

    let words = TokenWordView::new(tokens);
    for (phrase, kind) in PHRASES {
        if words.len() >= phrase.len() && words.slice_eq(words.len() - phrase.len(), phrase) {
            let rest_token_end = words
                .token_index_for_word_index(words.len() - phrase.len())
                .unwrap_or(tokens.len());
            return Some((&tokens[..rest_token_end], *kind));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::front_end::lexer::lex_line;

    #[test]
    fn rewrite_followup_intro_turns_when_you_do_into_if() {
        let tokens = lex_line("When you do, draw a card.", 0).expect("lex");
        let rewritten = rewrite_followup_intro_to_if_lexed(&tokens);

        assert_eq!(rewritten[0].parser_text(), "if");
    }

    #[test]
    fn em_dash_label_prefix_split_ignores_quoted_dashes() {
        let tokens = lex_line("Boast — \"Dash\" means nothing here.", 0).expect("lex");
        let (label, body) = split_em_dash_label_prefix(&tokens).expect("label prefix");

        assert_eq!(label, "Boast");
        assert_eq!(render_token_slice(body), "\"Dash\"means nothing here.");
    }

    #[test]
    fn leading_may_action_parser_detects_actor_and_tail() {
        let tokens = lex_line("That player may cast that card", 0).expect("lex");
        let parsed = parse_leading_may_action_lexed(&tokens, &["cast"], false).expect("may action");

        assert_eq!(parsed.actor, LeadingMayActor::ThatPlayer);
        assert_eq!(parsed.verb, "cast");
        assert_eq!(render_token_slice(parsed.tail_tokens), "that card");
    }

    #[test]
    fn parse_turn_duration_prefix_and_suffix_cover_common_forms() {
        let prefixed = lex_line("Until end of turn, creatures you control", 0).expect("lex");
        let suffixed = lex_line("Creatures you control until your next turn", 0).expect("lex");

        let (prefix_kind, _) = parse_turn_duration_prefix(&prefixed).expect("prefix");
        let (_, suffix_kind) = parse_turn_duration_suffix(&suffixed).expect("suffix");

        assert_eq!(prefix_kind, TurnDurationPhrase::UntilEndOfTurn);
        assert_eq!(suffix_kind, TurnDurationPhrase::UntilYourNextTurn);
    }
}
