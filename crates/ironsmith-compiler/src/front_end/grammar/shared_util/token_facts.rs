//! Typed grammar facts for shared token-slice operations.
//!
//! These entrypoints own Oracle-facing recognition that historically lived in
//! `front_end/shared/util.rs`.  Callers may retain compatibility signatures,
//! but they no longer scan words or punctuation themselves.

use winnow::combinator::{alt, eof};
#[cfg(test)]
use winnow::combinator::{peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
#[cfg(test)]
use winnow::token::any;

use crate::ChoiceCount;
#[cfg(test)]
use crate::lexer::LexStream;
use crate::lexer::{OwnedLexToken, TokenWordView};

use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActivationCostStartFact {
    pub(crate) token_index: usize,
    pub(crate) head: leaf::LeafActivationCostHead,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CostSegmentsFact<'a> {
    pub(crate) segments: Vec<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MayWordBoundaryFact {
    pub(crate) token_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadingSelectedWordsFact {
    pub(crate) consumed_words: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChoiceCountBeforeTargetFact {
    pub(crate) count: ChoiceCount,
    pub(crate) consumed_tokens: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutlawWord {
    Outlaw,
    NonOutlaw,
}

pub(crate) fn parse_activation_cost_start_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivationCostStartFact> {
    let (token_index, head, _) =
        primitives::find_prefix(tokens, || leaf::parse_leaf_activation_cost_head_lexed)?;
    Some(ActivationCostStartFact { token_index, head })
}

#[cfg(test)]
pub(crate) fn parse_cost_segments_tokens(tokens: &[OwnedLexToken]) -> CostSegmentsFact<'_> {
    let mut segments = Vec::new();
    let mut remaining = tokens;
    while !remaining.is_empty() {
        let Some((segment, rest)) = primitives::parse_prefix(remaining, parse_cost_segment_lexed)
        else {
            break;
        };
        if !segment.is_empty() {
            segments.push(segment);
        }
        if rest.len() == remaining.len() {
            break;
        }
        remaining = rest;
    }
    CostSegmentsFact { segments }
}

pub(crate) fn parse_first_may_word_token(tokens: &[OwnedLexToken]) -> Option<MayWordBoundaryFact> {
    let (token_index, _, _) = primitives::find_prefix(tokens, || primitives::kw("may"))?;
    Some(MayWordBoundaryFact { token_index })
}

/// Pure lexical transform: strips a caller-provided set without assigning any
/// semantic meaning to the words. Oracle vocabularies must use typed facts.
pub(crate) fn strip_leading_selected_word_refs_lexical(
    words: &[&str],
    selected: &[&str],
) -> LeadingSelectedWordsFact {
    let consumed_words = words
        .iter()
        .take_while(|word| selected.contains(word))
        .count();
    LeadingSelectedWordsFact { consumed_words }
}

pub(crate) fn non_article_word_refs<'a>(words: &[&'a str]) -> Vec<&'a str> {
    words
        .iter()
        .copied()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect()
}

pub(crate) fn non_article_token_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str> {
    let view = TokenWordView::new(tokens);
    non_article_word_refs(&view.word_refs())
}

pub(crate) fn parse_outlaw_word(word: &str) -> Option<OutlawWord> {
    let mut input = word;
    (parse_outlaw_text, eof)
        .map(|(outlaw, _)| outlaw)
        .parse_next(&mut input)
        .ok()
}

pub(crate) fn parse_choice_count_before_target_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ChoiceCountBeforeTargetFact> {
    let (parsed, rest) = primitives::parse_prefix(
        tokens,
        alt((
            leaf::parse_leaf_target_count_range_prefix_lexed,
            leaf::parse_leaf_choice_count_prefix_lexed,
        )),
    )?;
    primitives::parse_prefix(
        rest,
        winnow::combinator::alt((primitives::kw("target"), primitives::kw("targets"))),
    )?;
    Some(ChoiceCountBeforeTargetFact {
        count: parsed,
        consumed_tokens: tokens.len().checked_sub(rest.len())?,
    })
}

fn parse_outlaw_text(input: &mut &str) -> WResult<OutlawWord> {
    alt((
        alt(("outlaws", "outlaw")).value(OutlawWord::Outlaw),
        alt(("non-outlaws", "nonoutlaws", "non-outlaw", "nonoutlaw")).value(OutlawWord::NonOutlaw),
    ))
    .parse_next(input)
}

#[cfg(test)]
fn parse_cost_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let boundary = || {
        alt((
            primitives::comma().void(),
            primitives::kw("and").void(),
            eof.value(()),
        ))
    };
    let segment = repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(boundary()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    boundary().parse_next(input)?;
    Ok(segment)
}

#[cfg(test)]
mod tests {
    use crate::lexer::{lex_line, parser_token_word_refs};

    use super::*;

    #[test]
    fn finds_typed_activation_cost_start() {
        let tokens = lex_line("Only during your turn, {2}, {T}", 0).unwrap();
        let fact = parse_activation_cost_start_tokens(&tokens).unwrap();
        assert_eq!(fact.token_index, 5);
        assert_eq!(fact.head, leaf::LeafActivationCostHead::Mana);
    }

    #[test]
    fn splits_cost_segments_on_commas_and_conjunctions() {
        let tokens = lex_line("{2}, discard a card and sacrifice a creature", 0).unwrap();
        let fact = parse_cost_segments_tokens(&tokens);
        let words = fact
            .segments
            .iter()
            .map(|segment| parser_token_word_refs(segment).join(" "))
            .collect::<Vec<_>>();
        assert_eq!(words, ["2", "discard a card", "sacrifice a creature"]);
    }

    #[test]
    fn owns_word_removal_and_target_count_boundaries() {
        let tokens = lex_line("you may draw two cards", 0).unwrap();
        assert_eq!(parse_first_may_word_token(&tokens).unwrap().token_index, 1);

        let target = lex_line("up to two target creatures", 0).unwrap();
        let count = parse_choice_count_before_target_tokens(&target).unwrap();
        assert_eq!(count.consumed_tokens, 3);
        assert_eq!(count.count.min, 0);
        assert_eq!(count.count.max, Some(2));

        let range = lex_line("one or two target creatures", 0).unwrap();
        let count = parse_choice_count_before_target_tokens(&range).unwrap();
        assert_eq!(count.consumed_tokens, 3);
        assert_eq!(count.count.min, 1);
        assert_eq!(count.count.max, Some(2));
    }

    #[test]
    fn parses_singular_and_plural_outlaw_shorthand() {
        assert_eq!(parse_outlaw_word("outlaw"), Some(OutlawWord::Outlaw));
        assert_eq!(parse_outlaw_word("outlaws"), Some(OutlawWord::Outlaw));
        assert_eq!(
            parse_outlaw_word("non-outlaws"),
            Some(OutlawWord::NonOutlaw)
        );
    }
}
