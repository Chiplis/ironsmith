use winnow::combinator::eof;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::literal;

use super::{word_present, words_are_exact};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedBlockRequirement {
    AllCreaturesBlockSource,
    SourceMustBeBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedKeywordGrantContext {
    Granted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedCyclingContext {
    Standalone,
    Granted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedCyclingMarker {
    Cycling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivatedCyclingHead {
    pub keyword_word_index: usize,
    pub context: ActivatedCyclingContext,
}

pub fn parse_keyword_grant_context_words(words: &[&str]) -> Option<ActivatedKeywordGrantContext> {
    (word_present(words, "has") || word_present(words, "have"))
        .then_some(ActivatedKeywordGrantContext::Granted)
}

fn parse_text_end(input: &mut &str) -> WResult<()> {
    eof.void().parse_next(input)
}

fn parse_cycling_marker_surface(input: &mut &str) -> WResult<()> {
    let raw = *input;
    let suffix_first = raw.len().checked_sub("cycling".len()).ok_or_else(|| {
        super::super::primitives::backtrack_err("cycling keyword", "cycling word")
    })?;
    let suffix = raw.get(suffix_first..).ok_or_else(|| {
        super::super::primitives::backtrack_err("cycling keyword", "cycling word")
    })?;
    let mut suffix_input = suffix;
    (literal("cycling"), parse_text_end)
        .void()
        .parse_next(&mut suffix_input)?;
    *input = raw.get(raw.len()..).unwrap_or_default();
    Ok(())
}

pub fn parse_cycling_marker_word(word: &str) -> Option<ActivatedCyclingMarker> {
    let mut input = word;
    parse_cycling_marker_surface
        .parse_next(&mut input)
        .ok()
        .map(|()| ActivatedCyclingMarker::Cycling)
}

pub fn parse_cycling_keyword_head_words(words: &[&str]) -> Option<ActivatedCyclingHead> {
    for (keyword_word_index, word) in words.iter().enumerate() {
        if parse_cycling_marker_word(word).is_some() {
            let context =
                if parse_keyword_grant_context_words(&words[..keyword_word_index]).is_some() {
                    ActivatedCyclingContext::Granted
                } else {
                    ActivatedCyclingContext::Standalone
                };
            return Some(ActivatedCyclingHead {
                keyword_word_index,
                context,
            });
        }
    }
    None
}

pub fn parse_activated_block_requirement_words(
    words: &[&str],
) -> Option<ActivatedBlockRequirement> {
    if words_are_exact(
        words,
        &[
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "this",
            "creature",
            "do",
            "so",
        ],
    ) || words_are_exact(
        words,
        &[
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "this",
            "do",
            "so",
        ],
    ) {
        return Some(ActivatedBlockRequirement::AllCreaturesBlockSource);
    }
    if words_are_exact(
        words,
        &["this", "creature", "must", "be", "blocked", "if", "able"],
    ) || words_are_exact(words, &["this", "must", "be", "blocked", "if", "able"])
    {
        return Some(ActivatedBlockRequirement::SourceMustBeBlocked);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owns_block_requirements_and_keyword_grant_context() {
        assert_eq!(
            parse_activated_block_requirement_words(&[
                "all",
                "creatures",
                "able",
                "to",
                "block",
                "this",
                "do",
                "so",
            ]),
            Some(ActivatedBlockRequirement::AllCreaturesBlockSource)
        );
        assert_eq!(
            parse_activated_block_requirement_words(&[
                "this", "creature", "must", "be", "blocked", "if", "able",
            ]),
            Some(ActivatedBlockRequirement::SourceMustBeBlocked)
        );
        assert_eq!(
            parse_keyword_grant_context_words(&["creatures", "you", "control", "have"]),
            Some(ActivatedKeywordGrantContext::Granted)
        );
        assert_eq!(
            parse_cycling_keyword_head_words(&["plainscycling", "2"]),
            Some(ActivatedCyclingHead {
                keyword_word_index: 0,
                context: ActivatedCyclingContext::Standalone,
            })
        );
        assert_eq!(
            parse_cycling_keyword_head_words(&["creatures", "have", "cycling", "2"]),
            Some(ActivatedCyclingHead {
                keyword_word_index: 2,
                context: ActivatedCyclingContext::Granted,
            })
        );
    }
}
