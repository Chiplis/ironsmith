use std::borrow::Cow;
use std::ops::Range;

use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::{CardTextError, TextSpan};
use crate::effect::ChoiceCount;
use crate::types::{Subtype, SubtypeFamily};

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenWordView};
use super::super::primitives;
use super::counts::{
    parse_leaf_choice_count_prefix_lexed, parse_leaf_target_count_range_prefix_lexed,
};
use super::numbers::{LeafNumber, parse_leaf_number_or_x_prefix_lexed};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafTargetArticle {
    A,
    An,
    The,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeafTargetArticleToken {
    pub(crate) article: LeafTargetArticle,
    pub(crate) span: TextSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafRandomTargetKind {
    AtRandom,
    ChosenAtRandom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeafRandomTargetMarker {
    pub(crate) kind: LeafRandomTargetKind,
    pub(crate) span: Option<TextSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeafTopTargetPrefix {
    pub(crate) span: TextSpan,
    pub(crate) supplied_count: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeafTargetHeadPrefix {
    pub(crate) count: Option<ChoiceCount>,
    pub(crate) consumed: usize,
    pub(crate) explicit_target_span: Option<TextSpan>,
    pub(crate) target_marker_span: Option<TextSpan>,
    pub(crate) other: bool,
    pub(crate) other_span: Option<TextSpan>,
    pub(crate) articles: Vec<LeafTargetArticleToken>,
    pub(crate) on_span: Option<TextSpan>,
    pub(crate) top: Option<LeafTopTargetPrefix>,
    pub(crate) ordinal: Option<u8>,
    pub(crate) random: Option<LeafRandomTargetMarker>,
    pub(crate) phrase_span: Option<TextSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeafTargetHead<'a> {
    normalized_tokens: Cow<'a, [OwnedLexToken]>,
    pub(crate) prefix: LeafTargetHeadPrefix,
}

impl LeafTargetHead<'_> {
    pub(crate) fn tokens(&self) -> &[OwnedLexToken] {
        self.normalized_tokens.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn rest(&self) -> &[OwnedLexToken] {
        self.tokens()
            .get(self.prefix.consumed..)
            .unwrap_or_default()
    }
}

struct NormalizedTargetTokens<'a> {
    tokens: Cow<'a, [OwnedLexToken]>,
    random: Option<LeafRandomTargetMarker>,
}

pub(crate) fn parse_leaf_target_head_tokens(
    tokens: &[OwnedLexToken],
) -> Result<LeafTargetHead<'_>, CardTextError> {
    let normalized = normalize_random_target_marker(tokens);
    let phrase_span = primitives::token_slice_span(normalized.tokens.as_ref());
    let mut input = LexStream::new(normalized.tokens.as_ref());
    let starts_up_to = parses_up_to_prefix(input.clone());
    let parsed = parse_leaf_target_head_prefix_lexed(
        &mut input,
        normalized.random,
        phrase_span,
        normalized.tokens.len(),
    );
    let prefix = match parsed {
        Ok(prefix) => prefix,
        Err(_) if starts_up_to => {
            let next_word = normalized
                .tokens
                .get(2)
                .and_then(OwnedLexToken::as_word)
                .unwrap_or("?");
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic or missing target count after 'up to' (found '{next_word}' in clause: '{}')",
                TokenWordView::new(normalized.tokens.as_ref()).join(" ")
            )));
        }
        Err(err) => {
            return Err(CardTextError::ParseError(format!(
                "leaf target-head parser failed: {err}"
            )));
        }
    };
    Ok(LeafTargetHead {
        normalized_tokens: normalized.tokens,
        prefix,
    })
}

pub(crate) fn parse_leaf_target_head_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
    random: Option<LeafRandomTargetMarker>,
    phrase_span: Option<TextSpan>,
    original_token_count: usize,
) -> WResult<LeafTargetHeadPrefix> {
    let mut count = parse_target_head_count(input)?;
    if random.is_some() {
        count = Some(count.unwrap_or_default().at_random());
    }

    let on_span = parse_optional_word_span(input, "on");
    let mut articles = Vec::new();
    while let Ok(article) = parse_target_article_lexed.parse_next(input) {
        articles.push(article);
    }

    let mut top = None;
    let mut top_probe = input.clone();
    if let Ok(top_token) = primitives::kw("top").parse_next(&mut top_probe) {
        let top_span = top_token.span();
        top = Some(LeafTopTargetPrefix {
            span: top_span,
            supplied_count: false,
        });
        let mut count_probe = top_probe.clone();
        if let Ok(number) = parse_leaf_number_or_x_prefix_lexed.parse_next(&mut count_probe)
            && remaining_starts_object_selector(&count_probe)
        {
            count = Some(choice_count_from_leaf_number(number, false));
            *input = count_probe;
            top = Some(LeafTopTargetPrefix {
                span: top_span,
                supplied_count: true,
            });
        }
    }

    let mut other = false;
    let mut other_span = None;
    let mut explicit_target_span = None;
    let mut target_marker_span = None;

    let mut modifier_probe = input.clone();
    if let Ok(token) =
        alt((primitives::kw("another"), primitives::kw("other"))).parse_next(&mut modifier_probe)
    {
        other = true;
        other_span = Some(token.span());
        *input = modifier_probe;
    }
    if let Ok(target) = primitives::kw("target").parse_next(input) {
        target_marker_span = Some(target.span());
        explicit_target_span = phrase_span;
    }

    let mut ordinal_probe = input.clone();
    let ordinal = if let Ok(ordinal) = parse_target_ordinal_lexed.parse_next(&mut ordinal_probe) {
        if let Ok(target) = primitives::kw("target").parse_next(&mut ordinal_probe) {
            if ordinal != 1 {
                other = true;
            }
            target_marker_span = Some(target.span());
            explicit_target_span = phrase_span;
            *input = ordinal_probe;
            Some(ordinal)
        } else {
            None
        }
    } else {
        None
    };

    Ok(LeafTargetHeadPrefix {
        count,
        consumed: original_token_count.saturating_sub(input.len()),
        explicit_target_span,
        target_marker_span,
        other,
        other_span,
        articles,
        on_span,
        top,
        ordinal,
        random,
        phrase_span,
    })
}

fn parse_target_head_count(input: &mut LexStream<'_>) -> WResult<Option<ChoiceCount>> {
    let mut any_number_probe = input.clone();
    if primitives::phrase(&["any", "number", "of"])
        .parse_next(&mut any_number_probe)
        .is_ok()
    {
        return parse_leaf_choice_count_prefix_lexed
            .map(Some)
            .parse_next(input);
    }

    if parses_up_to_prefix(input.clone()) {
        return parse_leaf_choice_count_prefix_lexed
            .map(Some)
            .parse_next(input);
    }

    let mut range_probe = input.clone();
    if let Ok(count) = parse_leaf_target_count_range_prefix_lexed.parse_next(&mut range_probe) {
        *input = range_probe;
        return Ok(Some(count));
    }

    let mut number_probe = input.clone();
    if let Ok(number) = parse_leaf_number_or_x_prefix_lexed.parse_next(&mut number_probe)
        && (remaining_starts_target(&number_probe)
            || remaining_starts_other_target(&number_probe)
            || remaining_starts_object_selector(&number_probe))
    {
        *input = number_probe;
        return Ok(Some(choice_count_from_leaf_number(number, false)));
    }

    Ok(None)
}

fn parses_up_to_prefix(mut input: LexStream<'_>) -> bool {
    primitives::phrase(&["up", "to"])
        .parse_next(&mut input)
        .is_ok()
}

fn remaining_starts_target(input: &LexStream<'_>) -> bool {
    let mut probe = input.clone();
    primitives::kw("target").parse_next(&mut probe).is_ok()
}

fn remaining_starts_other_target(input: &LexStream<'_>) -> bool {
    let mut probe = input.clone();
    primitives::phrase(&["other", "target"])
        .parse_next(&mut probe)
        .is_ok()
}

fn remaining_starts_object_selector(input: &LexStream<'_>) -> bool {
    let mut probe = input.clone();
    while parse_target_count_selector_modifier_lexed
        .parse_next(&mut probe)
        .is_ok()
    {}
    parse_target_count_object_selector_lexed
        .parse_next(&mut probe)
        .is_ok()
}

fn parse_target_count_selector_modifier_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        alt((
            primitives::kw("tapped"),
            primitives::kw("untapped"),
            primitives::kw("attacking"),
            primitives::kw("nonattacking"),
            primitives::kw("blocked"),
            primitives::kw("unblocked"),
            primitives::kw("blocking"),
            primitives::kw("nonblocking"),
        )),
        alt((
            primitives::kw("non"),
            primitives::kw("other"),
            primitives::kw("another"),
            primitives::kw("nonartifact"),
            primitives::kw("noncreature"),
            primitives::kw("nonland"),
            primitives::kw("nontoken"),
            primitives::kw("legendary"),
        )),
        primitives::kw("basic"),
    ))
    .void()
    .parse_next(input)
}

fn parse_target_count_object_selector_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::word_parser_text
        .verify(|word| is_target_count_object_selector(word))
        .void()
        .parse_next(input)
}

fn is_target_count_object_selector(word: &str) -> bool {
    matches!(
        word,
        "card"
            | "cards"
            | "permanent"
            | "permanents"
            | "creature"
            | "creatures"
            | "spell"
            | "spells"
            | "source"
            | "sources"
            | "token"
            | "tokens"
            | "artifact"
            | "artifacts"
            | "enchantment"
            | "enchantments"
            | "land"
            | "lands"
            | "planeswalker"
            | "planeswalkers"
            | "instant"
            | "instants"
            | "sorcery"
            | "sorceries"
            | "battle"
            | "battles"
            | "kindred"
            | "nonartifact"
            | "nonartifacts"
            | "noncreature"
            | "noncreatures"
            | "nonenchantment"
            | "nonenchantments"
            | "nonland"
            | "nonlands"
            | "nonplaneswalker"
            | "nonplaneswalkers"
            | "noninstant"
            | "noninstants"
            | "nonsorcery"
            | "nonsorceries"
            | "nonbattle"
            | "nonbattles"
            | "nonkindred"
    ) || is_subtype_word(word)
}

fn is_subtype_word(word: &str) -> bool {
    let candidate = normalize_type_word(word);
    if matches!(candidate.as_str(), "mice" | "ouphe" | "oxen" | "spacecraft") {
        return true;
    }
    for family in [
        SubtypeFamily::Land,
        SubtypeFamily::Creature,
        SubtypeFamily::Artifact,
        SubtypeFamily::Enchantment,
        SubtypeFamily::Spell,
        SubtypeFamily::Planeswalker,
        SubtypeFamily::Battle,
    ] {
        for subtype in family.all_subtypes() {
            if subtype_word_matches(*subtype, candidate.as_str()) {
                return true;
            }
        }
    }
    false
}

fn normalize_type_word(word: &str) -> String {
    word.chars()
        .filter_map(|ch| match ch {
            '\'' | '’' | '-' => None,
            _ if ch.is_ascii_alphanumeric() => Some(ch.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

fn subtype_word_matches(subtype: Subtype, candidate: &str) -> bool {
    let base = normalize_type_word(subtype.to_string().as_str());
    if candidate == base || candidate == format!("{base}s") {
        return true;
    }
    let chars = base.chars().collect::<Vec<_>>();
    if chars.last() == Some(&'y') {
        let stem = chars[..chars.len().saturating_sub(1)]
            .iter()
            .collect::<String>();
        if candidate == format!("{stem}ies") {
            return true;
        }
    }
    if chars.last() == Some(&'f') {
        let trim = if chars.get(chars.len().saturating_sub(2)) == Some(&'e') {
            2
        } else {
            1
        };
        let stem = chars[..chars.len().saturating_sub(trim)]
            .iter()
            .collect::<String>();
        if candidate == format!("{stem}ves") {
            return true;
        }
    }
    false
}

fn parse_target_article_lexed<'a>(input: &mut LexStream<'a>) -> WResult<LeafTargetArticleToken> {
    alt((
        primitives::kw("an").map(|token| LeafTargetArticleToken {
            article: LeafTargetArticle::An,
            span: token.span(),
        }),
        primitives::kw("a").map(|token| LeafTargetArticleToken {
            article: LeafTargetArticle::A,
            span: token.span(),
        }),
        primitives::kw("the").map(|token| LeafTargetArticleToken {
            article: LeafTargetArticle::The,
            span: token.span(),
        }),
    ))
    .parse_next(input)
}

fn parse_optional_word_span(input: &mut LexStream<'_>, word: &'static str) -> Option<TextSpan> {
    let mut probe = input.clone();
    let token = primitives::kw(word).parse_next(&mut probe).ok()?;
    *input = probe;
    Some(token.span())
}

fn parse_target_ordinal_lexed<'a>(input: &mut LexStream<'a>) -> WResult<u8> {
    alt((
        alt((
            primitives::kw("first").value(1),
            primitives::kw("second").value(2),
            primitives::kw("third").value(3),
            primitives::kw("fourth").value(4),
            primitives::kw("fifth").value(5),
        )),
        alt((
            primitives::kw("sixth").value(6),
            primitives::kw("seventh").value(7),
            primitives::kw("eighth").value(8),
            primitives::kw("ninth").value(9),
            primitives::kw("tenth").value(10),
        )),
    ))
    .parse_next(input)
}

fn choice_count_from_leaf_number(number: LeafNumber, up_to: bool) -> ChoiceCount {
    match (number, up_to) {
        (LeafNumber::X, true) => ChoiceCount::up_to_dynamic_x(),
        (LeafNumber::X, false) => ChoiceCount::dynamic_x(),
        (LeafNumber::Fixed(value), true) => ChoiceCount::up_to(value as usize),
        (LeafNumber::Fixed(value), false) => ChoiceCount::exactly(value as usize),
    }
}

fn normalize_random_target_marker(tokens: &[OwnedLexToken]) -> NormalizedTargetTokens<'_> {
    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    if let Some(word_start) = random_suffix_start(&words, LeafRandomTargetKind::ChosenAtRandom)
        && let Some(token_start) = view.token_start_indices().get(word_start).copied()
    {
        return NormalizedTargetTokens {
            tokens: Cow::Borrowed(&tokens[..token_start]),
            random: Some(LeafRandomTargetMarker {
                kind: LeafRandomTargetKind::ChosenAtRandom,
                span: primitives::token_slice_span(&tokens[token_start..]),
            }),
        };
    }
    if let Some(word_start) = random_suffix_start(&words, LeafRandomTargetKind::AtRandom)
        && let Some(token_start) = view.token_start_indices().get(word_start).copied()
    {
        return NormalizedTargetTokens {
            tokens: Cow::Borrowed(&tokens[..token_start]),
            random: Some(LeafRandomTargetMarker {
                kind: LeafRandomTargetKind::AtRandom,
                span: primitives::token_slice_span(&tokens[token_start..]),
            }),
        };
    }
    if let Some(word_range) = parse_at_random_word_range(&words)
        && let Some(token_start) = view.token_start_indices().get(word_range.start).copied()
    {
        let token_end = view
            .token_start_indices()
            .get(word_range.end)
            .copied()
            .unwrap_or(tokens.len());
        let mut normalized = Vec::with_capacity(tokens.len());
        normalized.extend_from_slice(tokens.get(..token_start).unwrap_or_default());
        normalized.extend_from_slice(tokens.get(token_end..).unwrap_or_default());
        return NormalizedTargetTokens {
            tokens: Cow::Owned(normalized),
            random: Some(LeafRandomTargetMarker {
                kind: LeafRandomTargetKind::AtRandom,
                span: primitives::token_slice_span(
                    tokens.get(token_start..token_end).unwrap_or_default(),
                ),
            }),
        };
    }
    NormalizedTargetTokens {
        tokens: Cow::Borrowed(tokens),
        random: None,
    }
}

fn random_suffix_start(words: &[&str], kind: LeafRandomTargetKind) -> Option<usize> {
    let phrase = match kind {
        LeafRandomTargetKind::ChosenAtRandom => &["chosen", "at", "random"][..],
        LeafRandomTargetKind::AtRandom => &["at", "random"][..],
    };
    let start = words.len().checked_sub(phrase.len())?;
    let mut input: primitives::WordSliceInput<'_> = &words[start..];
    parse_word_phrase(&mut input, phrase).ok()?;
    input.is_empty().then_some(start)
}

fn parse_at_random_word_range(words: &[&str]) -> Option<Range<usize>> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let mut offset = 0;
    while !input.is_empty() {
        let mut probe = input;
        if parse_word_phrase(&mut probe, &["at", "random"]).is_ok() {
            return Some(offset..offset + 2);
        }
        parse_any_word_slice.parse_next(&mut input).ok()?;
        offset += 1;
    }
    None
}

fn parse_any_word_slice<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<&'a str> {
    let Some((word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err("word", "word"));
    };
    *input = rest;
    Ok(*word)
}

fn parse_word_phrase(
    input: &mut primitives::WordSliceInput<'_>,
    expected: &[&'static str],
) -> WResult<()> {
    for word in expected {
        primitives::word_slice_exact(*word)
            .void()
            .parse_next(input)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn parse(raw: &str) -> LeafTargetHead<'static> {
        let tokens = lex_line(raw, 0).expect("lex target-head fixture");
        let parsed = parse_leaf_target_head_tokens(&tokens).expect(raw);
        LeafTargetHead {
            normalized_tokens: Cow::Owned(parsed.tokens().to_vec()),
            prefix: parsed.prefix,
        }
    }

    #[test]
    fn parses_ranges_dynamic_counts_other_target_and_random() {
        let ranged = parse("one, two, or three target creatures");
        let ranged_count = ranged.prefix.count.expect("range count");
        assert_eq!(ranged_count.min, 1);
        assert_eq!(ranged_count.max, Some(3));
        assert!(ranged.prefix.explicit_target_span.is_some());
        assert_eq!(
            TokenWordView::new(ranged.rest()).to_word_refs(),
            ["creatures"]
        );

        let dynamic = parse("up to X other target creatures chosen at random");
        let count = dynamic.prefix.count.expect("count");
        assert!(count.is_up_to_dynamic_x());
        assert!(count.is_random());
        assert!(dynamic.prefix.other);
        assert!(dynamic.prefix.explicit_target_span.is_some());
        assert_eq!(
            dynamic.prefix.random.map(|marker| marker.kind),
            Some(LeafRandomTargetKind::ChosenAtRandom)
        );
        assert_eq!(
            TokenWordView::new(dynamic.rest()).to_word_refs(),
            ["creatures"]
        );
    }

    #[test]
    fn preserves_any_number_articles_on_and_ordinal_heads() {
        let any = parse("any number of targets");
        assert!(
            any.prefix
                .count
                .as_ref()
                .is_some_and(ChoiceCount::is_any_number)
        );
        assert_eq!(TokenWordView::new(any.rest()).to_word_refs(), ["targets"]);

        let ordinal = parse("on the third target creature");
        assert!(ordinal.prefix.on_span.is_some());
        assert_eq!(ordinal.prefix.articles.len(), 1);
        assert_eq!(ordinal.prefix.ordinal, Some(3));
        assert!(ordinal.prefix.other);
        assert!(ordinal.prefix.target_marker_span.is_some());
        assert_eq!(
            TokenWordView::new(ordinal.rest()).to_word_refs(),
            ["creature"]
        );
    }

    #[test]
    fn top_prefix_only_consumes_when_it_supplies_a_count() {
        let counted = parse("top two cards");
        assert_eq!(counted.prefix.count, Some(ChoiceCount::exactly(2)));
        assert!(counted.prefix.top.is_some_and(|top| top.supplied_count));
        assert_eq!(TokenWordView::new(counted.rest()).to_word_refs(), ["cards"]);

        let bare = parse("top card");
        assert!(bare.prefix.count.is_none());
        assert!(bare.prefix.top.is_some_and(|top| !top.supplied_count));
        assert_eq!(bare.prefix.consumed, 0);
        assert_eq!(
            TokenWordView::new(bare.rest()).to_word_refs(),
            ["top", "card"]
        );
    }

    #[test]
    fn numeric_count_uses_typed_object_selector_lookahead() {
        let typed = parse("two legendary Elves");
        assert_eq!(typed.prefix.count, Some(ChoiceCount::exactly(2)));
        assert_eq!(
            TokenWordView::new(typed.rest()).to_word_refs(),
            ["legendary", "elves"]
        );

        let player = parse("two players");
        assert!(player.prefix.count.is_none());
        assert_eq!(player.prefix.consumed, 0);
    }

    #[test]
    fn random_normalization_preserves_target_and_marker_spans() {
        let parsed = parse("target non-Vampire creature chosen at random");
        assert!(parsed.prefix.count.is_some_and(|count| count.is_random()));
        assert!(parsed.prefix.target_marker_span.is_some());
        assert!(parsed.prefix.explicit_target_span.is_some());
        assert!(
            parsed
                .prefix
                .random
                .is_some_and(|marker| marker.span.is_some())
        );
        assert_eq!(
            TokenWordView::new(parsed.tokens()).to_word_refs(),
            ["target", "non", "vampire", "creature"]
        );
    }

    #[test]
    fn malformed_up_to_keeps_the_existing_diagnostic() {
        let tokens = lex_line("up to target creature", 0).expect("lex");
        let err = parse_leaf_target_head_tokens(&tokens).expect_err("malformed count");
        assert!(format!("{err}").contains("unsupported dynamic or missing target count"));
    }
}
