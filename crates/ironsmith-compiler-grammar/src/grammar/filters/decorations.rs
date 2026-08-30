use crate::filter::{CounterConstraint, ParityRequirement};
use crate::{ObjectFilter, TagKey, TaggedOpbjectRelation};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::primitives::{self, WordSliceInput};
use crate::lexer::{OwnedLexToken, TokenWordView, parser_token_word_refs};
use crate::util::{
    FilterKeywordConstraint, apply_filter_keyword_constraint,
    parse_filter_keyword_constraint_words, trim_commas,
};

use super::counter_constraints::parse_filter_counter_constraint_words;

type WordInput<'a> = WordSliceInput<'a>;

const VOTE_WINNER_SUFFIXES: &[&[&str]] = &[
    &[
        "with", "the", "most", "votes", "or", "tied", "for", "most", "votes",
    ],
    &[
        "with", "most", "votes", "or", "tied", "for", "most", "votes",
    ],
];

const DIFFERENT_NAMES_CLAUSES: &[&[&str]] = &[
    &["with", "different", "names"],
    &["that", "have", "different", "names"],
];

const NOT_ON_BATTLEFIELD_PHRASES: &[&[&str]] = &[
    &["that", "arent", "on", "the", "battlefield"],
    &["that", "aren't", "on", "the", "battlefield"],
    &["that", "isnt", "on", "the", "battlefield"],
    &["that", "isn't", "on", "the", "battlefield"],
    &["that", "are", "not", "on", "the", "battlefield"],
    &["that", "is", "not", "on", "the", "battlefield"],
    &["arent", "on", "the", "battlefield"],
    &["aren't", "on", "the", "battlefield"],
    &["isnt", "on", "the", "battlefield"],
    &["isn't", "on", "the", "battlefield"],
    &["are", "not", "on", "the", "battlefield"],
    &["is", "not", "on", "the", "battlefield"],
];

const ODD_MANA_VALUE_PHRASES: &[&[&str]] = &[&["odd", "mana", "value"], &["odd", "mana", "values"]];
const EVEN_MANA_VALUE_PHRASES: &[&[&str]] =
    &[&["even", "mana", "value"], &["even", "mana", "values"]];
const ODD_POWER_PHRASES: &[&[&str]] = &[&["odd", "power"]];
const EVEN_POWER_PHRASES: &[&[&str]] = &[&["even", "power"]];
const CHOSEN_POWER_QUALITY_PHRASES: &[&[&str]] = &[
    &["power", "of", "chosen", "quality"],
    &["power", "of", "that", "quality"],
    &["power", "of", "the", "chosen", "quality"],
];
const CHOSEN_MANA_VALUE_QUALITY_PHRASES: &[&[&str]] = &[
    &["mana", "value", "of", "chosen", "quality"],
    &["mana", "value", "of", "that", "quality"],
    &["mana", "values", "of", "chosen", "quality"],
    &["mana", "values", "of", "that", "quality"],
    &["mana", "value", "of", "the", "chosen", "quality"],
    &["mana", "values", "of", "the", "chosen", "quality"],
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FilterEnvelopeDecorations {
    distinct_names: bool,
    vote_winners_only: bool,
}

impl FilterEnvelopeDecorations {
    #[cfg(test)]
    pub fn has_distinct_names(self) -> bool {
        self.distinct_names
    }

    #[cfg(test)]
    pub fn has_vote_winner_suffix(self) -> bool {
        self.vote_winners_only
    }

    pub fn apply(self, mut filter: ObjectFilter) -> ObjectFilter {
        filter.distinct_names |= self.distinct_names;
        if self.vote_winners_only {
            filter = filter.match_tagged(
                crate::tag::CompilerReferenceTag::VoteWinners.key(),
                TaggedOpbjectRelation::IsTaggedObject,
            );
        }
        filter
    }

    pub fn apply_distinct_names_only(self, mut filter: ObjectFilter) -> ObjectFilter {
        filter.distinct_names |= self.distinct_names;
        filter
    }
}

#[derive(Debug, Clone)]
pub struct LexedFilterEnvelope {
    pub core_tokens: Vec<OwnedLexToken>,
    pub decorations: FilterEnvelopeDecorations,
}

#[derive(Debug, Clone)]
pub struct WordFilterEnvelope<'a> {
    pub core_words: Vec<&'a str>,
    pub decorations: FilterEnvelopeDecorations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterTailDecoration {
    WithCounter(CounterConstraint),
    WithoutCounter(CounterConstraint),
    WithKeyword(FilterKeywordConstraint),
    WithoutKeyword(FilterKeywordConstraint),
    WithEitherKeyword(FilterKeywordConstraint, FilterKeywordConstraint),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedFilterTailDecoration {
    pub decoration: FilterTailDecoration,
    pub consumed: usize,
}

#[derive(Debug, Clone)]
pub struct LexedFilterTailSplit {
    pub base_tokens: Vec<OwnedLexToken>,
    pub decoration: FilterTailDecoration,
}

#[derive(Debug, Clone, Copy)]
pub struct WordFilterTailSplit<'a> {
    pub base_words: &'a [&'a str],
    pub decoration: FilterTailDecoration,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct FilterParityDecorations {
    mana_value: Option<ParityRequirement>,
    power: Option<ParityRequirement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterTailIntroducer {
    With,
    Without,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WordDecorationSpan {
    start: usize,
    consumed: usize,
}

pub fn parse_filter_lexed_envelope(tokens: &[OwnedLexToken]) -> LexedFilterEnvelope {
    let (tokens, vote_winners_only) = parse_vote_winner_suffix_tokens(tokens);
    let (core_tokens, distinct_names) = parse_different_names_tokens(&tokens);
    LexedFilterEnvelope {
        core_tokens,
        decorations: FilterEnvelopeDecorations {
            distinct_names,
            vote_winners_only,
        },
    }
}

pub fn parse_filter_word_envelope<'a>(words: &'a [&'a str]) -> WordFilterEnvelope<'a> {
    let (words, vote_winners_only) = parse_vote_winner_suffix_words(words);
    let (core_words, distinct_names) = parse_different_names_words(words);
    WordFilterEnvelope {
        core_words,
        decorations: FilterEnvelopeDecorations {
            distinct_names,
            vote_winners_only,
        },
    }
}

pub fn parse_filter_distinct_names_tokens(tokens: &[OwnedLexToken]) -> LexedFilterEnvelope {
    let (core_tokens, distinct_names) = parse_different_names_tokens(tokens);
    LexedFilterEnvelope {
        core_tokens,
        decorations: FilterEnvelopeDecorations {
            distinct_names,
            vote_winners_only: false,
        },
    }
}

pub fn trim_vote_winner_suffix(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    parse_vote_winner_suffix_tokens(tokens)
}

pub fn strip_not_on_battlefield_phrase(tokens: &mut Vec<OwnedLexToken>) -> bool {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let Some(span) = parse_not_on_battlefield_span(&words) else {
        return false;
    };
    let Some(token_start) = word_view.map_word_or_end_to_token_boundary(span.start) else {
        return false;
    };
    let Some(token_end) = word_view.map_word_or_end_to_token_boundary(span.start + span.consumed)
    else {
        return false;
    };
    tokens.drain(token_start..token_end);
    true
}

pub fn apply_parity_filter_phrases(words: &[&str], filter: &mut ObjectFilter) {
    let parsed = parse_filter_parity_decorations(words);
    if let Some(parity) = parsed.mana_value {
        filter.mana_value_parity = Some(parity);
    }
    if let Some(parity) = parsed.power {
        filter.power_parity = Some(parity);
    }
}

pub fn parse_filter_tail_decoration_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LexedFilterTailSplit> {
    for token_idx in 0..tokens.len() {
        let Some(word) = tokens[token_idx].as_word() else {
            continue;
        };
        let Some(introducer) = parse_tail_introducer_word(word) else {
            continue;
        };
        let tail_words = parser_token_word_refs(&tokens[token_idx + 1..]);
        let parsed = parse_filter_tail_decoration_words(&tail_words, introducer)?;
        if parsed.consumed != tail_words.len() || token_idx == 0 {
            return None;
        }
        let base_tokens = trim_commas(&tokens[..token_idx]);
        if base_tokens.is_empty() {
            return None;
        }
        return Some(LexedFilterTailSplit {
            base_tokens,
            decoration: parsed.decoration,
        });
    }
    None
}

pub fn parse_filter_tail_decoration_split_words<'a>(
    words: &'a [&'a str],
) -> Option<WordFilterTailSplit<'a>> {
    for word_idx in 0..words.len() {
        let Some(introducer) = parse_tail_introducer_word(words[word_idx]) else {
            continue;
        };
        let parsed = parse_filter_tail_decoration_words(&words[word_idx + 1..], introducer)?;
        if parsed.consumed != words.len().saturating_sub(word_idx + 1) || word_idx == 0 {
            return None;
        }
        return Some(WordFilterTailSplit {
            base_words: &words[..word_idx],
            decoration: parsed.decoration,
        });
    }
    None
}

pub fn apply_filter_tail_decoration(filter: &mut ObjectFilter, decoration: FilterTailDecoration) {
    match decoration {
        FilterTailDecoration::WithCounter(constraint) => filter.with_counter = Some(constraint),
        FilterTailDecoration::WithoutCounter(constraint) => {
            filter.without_counter = Some(constraint)
        }
        FilterTailDecoration::WithKeyword(constraint) => {
            apply_filter_keyword_constraint(filter, constraint, false)
        }
        FilterTailDecoration::WithoutKeyword(constraint) => {
            apply_filter_keyword_constraint(filter, constraint, true)
        }
        FilterTailDecoration::WithEitherKeyword(left_constraint, right_constraint) => {
            let mut left = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut left, left_constraint, false);
            let mut right = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut right, right_constraint, false);
            filter.any_of = vec![left, right];
        }
    }
}

fn parse_filter_tail_decoration_words(
    words: &[&str],
    introducer: FilterTailIntroducer,
) -> Option<ParsedFilterTailDecoration> {
    let mut input: WordInput<'_> = words;
    let decoration = match introducer {
        FilterTailIntroducer::With => parse_with_tail_decoration.parse_next(&mut input).ok()?,
        FilterTailIntroducer::Without => {
            parse_without_tail_decoration.parse_next(&mut input).ok()?
        }
    };
    Some(ParsedFilterTailDecoration {
        decoration,
        consumed: words.len().checked_sub(input.len())?,
    })
}

fn parse_with_tail_decoration(input: &mut WordInput<'_>) -> WResult<FilterTailDecoration> {
    let checkpoint = *input;
    if primitives::word_slice_exact("no")
        .void()
        .parse_next(input)
        .is_ok()
    {
        if let Ok(constraint) = parse_counter_constraint.parse_next(input) {
            return Ok(FilterTailDecoration::WithoutCounter(constraint));
        }
        *input = checkpoint;
    }

    if let Ok(constraint) = parse_counter_constraint.parse_next(input) {
        return Ok(FilterTailDecoration::WithCounter(constraint));
    }
    *input = checkpoint;
    parse_with_keyword_decoration.parse_next(input)
}

fn parse_without_tail_decoration(input: &mut WordInput<'_>) -> WResult<FilterTailDecoration> {
    let checkpoint = *input;
    if let Ok(constraint) = parse_keyword_constraint.parse_next(input) {
        return Ok(FilterTailDecoration::WithoutKeyword(constraint));
    }
    *input = checkpoint;
    parse_counter_constraint
        .map(FilterTailDecoration::WithoutCounter)
        .parse_next(input)
}

fn parse_with_keyword_decoration(input: &mut WordInput<'_>) -> WResult<FilterTailDecoration> {
    let first = parse_keyword_constraint.parse_next(input)?;
    let after_first = *input;
    if primitives::word_slice_exact("or")
        .void()
        .parse_next(input)
        .is_ok()
        && let Ok(second) = parse_keyword_constraint.parse_next(input)
    {
        return Ok(FilterTailDecoration::WithEitherKeyword(first, second));
    }
    *input = after_first;
    Ok(FilterTailDecoration::WithKeyword(first))
}

fn parse_counter_constraint(input: &mut WordInput<'_>) -> WResult<CounterConstraint> {
    let Some((constraint, consumed)) = parse_filter_counter_constraint_words(input) else {
        return Err(primitives::backtrack_err(
            "filter tail decoration",
            "counter constraint",
        ));
    };
    if consumed == 0 || consumed > input.len() {
        return Err(primitives::backtrack_err(
            "filter tail decoration",
            "nonempty counter constraint",
        ));
    }
    *input = &input[consumed..];
    Ok(constraint)
}

fn parse_keyword_constraint(input: &mut WordInput<'_>) -> WResult<FilterKeywordConstraint> {
    let Some((constraint, consumed)) = parse_filter_keyword_constraint_words(input) else {
        return Err(primitives::backtrack_err(
            "filter tail decoration",
            "keyword constraint",
        ));
    };
    if consumed == 0 || consumed > input.len() {
        return Err(primitives::backtrack_err(
            "filter tail decoration",
            "nonempty keyword constraint",
        ));
    }
    *input = &input[consumed..];
    Ok(constraint)
}

fn parse_tail_introducer_word(word: &str) -> Option<FilterTailIntroducer> {
    match word {
        "with" => Some(FilterTailIntroducer::With),
        "without" => Some(FilterTailIntroducer::Without),
        _ => None,
    }
}

fn parse_vote_winner_suffix_tokens(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let Some(suffix_start) = parse_suffix_start(&words, VOTE_WINNER_SUFFIXES) else {
        return (tokens.to_vec(), false);
    };
    let Some(token_end) = word_view.map_word_or_end_to_token_boundary(suffix_start) else {
        return (tokens.to_vec(), false);
    };
    (trim_commas(&tokens[..token_end]), true)
}

fn parse_vote_winner_suffix_words<'a>(words: &'a [&'a str]) -> (&'a [&'a str], bool) {
    let Some(suffix_start) = parse_suffix_start(words, VOTE_WINNER_SUFFIXES) else {
        return (words, false);
    };
    (&words[..suffix_start], true)
}

fn parse_suffix_start(words: &[&str], suffixes: &[&[&str]]) -> Option<usize> {
    for suffix in suffixes {
        let Some(suffix_start) = words.len().checked_sub(suffix.len()) else {
            continue;
        };
        let mut input: WordInput<'_> = &words[suffix_start..];
        if parse_word_phrase(&mut input, suffix).is_ok() && input.is_empty() {
            return Some(suffix_start);
        }
    }
    None
}

fn parse_different_names_tokens(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    for token_start in 0..tokens.len() {
        for phrase in DIFFERENT_NAMES_CLAUSES {
            let token_end = token_start + phrase.len();
            let Some(candidate) = tokens.get(token_start..token_end) else {
                continue;
            };
            let candidate_words = parser_token_word_refs(candidate);
            let mut input: WordInput<'_> = &candidate_words;
            if parse_word_phrase(&mut input, phrase).is_err() || !input.is_empty() {
                continue;
            }
            let mut stripped = Vec::with_capacity(tokens.len().saturating_sub(phrase.len()));
            stripped.extend_from_slice(&tokens[..token_start]);
            stripped.extend_from_slice(&tokens[token_end..]);
            return (trim_commas(&stripped), true);
        }
    }
    (trim_commas(tokens), false)
}

fn parse_different_names_words<'a>(words: &'a [&'a str]) -> (Vec<&'a str>, bool) {
    for word_start in 0..words.len() {
        for phrase in DIFFERENT_NAMES_CLAUSES {
            let word_end = word_start + phrase.len();
            let Some(candidate) = words.get(word_start..word_end) else {
                continue;
            };
            let mut input: WordInput<'_> = candidate;
            if parse_word_phrase(&mut input, phrase).is_err() || !input.is_empty() {
                continue;
            }
            let mut stripped = Vec::with_capacity(words.len().saturating_sub(phrase.len()));
            stripped.extend_from_slice(&words[..word_start]);
            stripped.extend_from_slice(&words[word_end..]);
            return (stripped, true);
        }
    }
    (words.to_vec(), false)
}

fn parse_not_on_battlefield_span(words: &[&str]) -> Option<WordDecorationSpan> {
    for word_start in 0..words.len() {
        for phrase in NOT_ON_BATTLEFIELD_PHRASES {
            let mut input: WordInput<'_> = &words[word_start..];
            if parse_word_phrase(&mut input, phrase).is_ok() {
                return Some(WordDecorationSpan {
                    start: word_start,
                    consumed: phrase.len(),
                });
            }
        }
    }
    None
}

fn parse_filter_parity_decorations(words: &[&str]) -> FilterParityDecorations {
    let mut parsed = FilterParityDecorations::default();
    if parse_any_phrase_anywhere(words, ODD_MANA_VALUE_PHRASES) {
        parsed.mana_value = Some(ParityRequirement::Odd);
    }
    if parse_any_phrase_anywhere(words, EVEN_MANA_VALUE_PHRASES) {
        parsed.mana_value = Some(ParityRequirement::Even);
    }
    if parse_any_phrase_anywhere(words, ODD_POWER_PHRASES) {
        parsed.power = Some(ParityRequirement::Odd);
    }
    if parse_any_phrase_anywhere(words, EVEN_POWER_PHRASES) {
        parsed.power = Some(ParityRequirement::Even);
    }
    if parse_any_phrase_anywhere(words, CHOSEN_POWER_QUALITY_PHRASES) {
        parsed.power = Some(ParityRequirement::Chosen);
    }
    if parse_any_phrase_anywhere(words, CHOSEN_MANA_VALUE_QUALITY_PHRASES) {
        parsed.mana_value = Some(ParityRequirement::Chosen);
    }
    parsed
}

fn parse_any_phrase_anywhere(words: &[&str], phrases: &[&[&str]]) -> bool {
    for word_start in 0..words.len() {
        for phrase in phrases {
            let mut input: WordInput<'_> = &words[word_start..];
            if parse_word_phrase(&mut input, phrase).is_ok() {
                return true;
            }
        }
    }
    false
}

fn parse_word_phrase(input: &mut WordInput<'_>, expected: &[&str]) -> WResult<()> {
    let checkpoint = *input;
    for word in expected {
        if let Err(err) = parse_exact_word(input, word) {
            *input = checkpoint;
            return Err(err);
        }
    }
    Ok(())
}

fn parse_exact_word(input: &mut WordInput<'_>, expected: &str) -> WResult<()> {
    let Some((word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err(
            "filter decoration word",
            "expected word",
        ));
    };
    if *word != expected {
        return Err(primitives::backtrack_err(
            "filter decoration word",
            "matching word",
        ));
    }
    *input = rest;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardType;
    use crate::lexer::lex_line;

    #[test]
    fn envelope_preserves_vote_then_different_name_normalization() {
        let tokens = lex_line(
            "creatures with different names with the most votes or tied for most votes",
            0,
        )
        .unwrap();
        let envelope = parse_filter_lexed_envelope(&tokens);
        assert!(envelope.decorations.has_distinct_names());
        assert!(envelope.decorations.has_vote_winner_suffix());
        assert_eq!(
            parser_token_word_refs(&envelope.core_tokens),
            vec!["creatures"]
        );

        let short_suffix = lex_line("creature with most votes or tied for most votes", 0).unwrap();
        let short_envelope = parse_filter_lexed_envelope(&short_suffix);
        assert!(short_envelope.decorations.has_vote_winner_suffix());
        assert_eq!(
            parser_token_word_refs(&short_envelope.core_tokens),
            vec!["creature"]
        );
    }

    #[test]
    fn not_on_battlefield_parser_removes_only_the_typed_phrase() {
        let mut tokens = lex_line("creatures that aren't on the battlefield", 0).unwrap();
        assert!(strip_not_on_battlefield_phrase(&mut tokens));
        assert_eq!(parser_token_word_refs(&tokens), vec!["creatures"]);
    }

    #[test]
    fn parity_parser_preserves_legacy_overwrite_precedence() {
        let mut filter = ObjectFilter::default();
        apply_parity_filter_phrases(
            &[
                "odd", "mana", "value", "and", "power", "of", "chosen", "quality",
            ],
            &mut filter,
        );
        assert_eq!(filter.mana_value_parity, Some(ParityRequirement::Odd));
        assert_eq!(filter.power_parity, Some(ParityRequirement::Chosen));
    }

    #[test]
    fn typed_tail_decorations_cover_counters_and_keywords() {
        let counter_tokens = lex_line("creature with a +1/+1 counter on it", 0).unwrap();
        let counter = parse_filter_tail_decoration_tokens(&counter_tokens).unwrap();
        assert!(matches!(
            counter.decoration,
            FilterTailDecoration::WithCounter(_)
        ));

        let no_counter_tokens = lex_line("creature with no counters on it", 0).unwrap();
        let no_counter = parse_filter_tail_decoration_tokens(&no_counter_tokens).unwrap();
        assert!(matches!(
            no_counter.decoration,
            FilterTailDecoration::WithoutCounter(_)
        ));

        let with_keyword_tokens = lex_line("creature with flying", 0).unwrap();
        let with_keyword = parse_filter_tail_decoration_tokens(&with_keyword_tokens).unwrap();
        assert!(matches!(
            with_keyword.decoration,
            FilterTailDecoration::WithKeyword(FilterKeywordConstraint::Static(_))
        ));

        let keyword_tokens = lex_line("creature without flying", 0).unwrap();
        let keyword = parse_filter_tail_decoration_tokens(&keyword_tokens).unwrap();
        assert!(matches!(
            keyword.decoration,
            FilterTailDecoration::WithoutKeyword(FilterKeywordConstraint::Static(_))
        ));
    }

    #[test]
    fn envelope_applies_vote_winner_and_distinct_name_facts() {
        let decorations = FilterEnvelopeDecorations {
            distinct_names: true,
            vote_winners_only: true,
        };
        let filter = decorations.apply(ObjectFilter::default());
        assert!(filter.distinct_names);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::VoteWinners.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
    }

    #[test]
    fn complex_vote_winner_fallback_remains_legacy_compatible() {
        let tokens = lex_line(
            "creature with flying with the most votes or tied for most votes",
            0,
        )
        .unwrap();
        let filter = crate::object_filters::parse_object_filter_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(!filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::VoteWinners.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
    }
}
