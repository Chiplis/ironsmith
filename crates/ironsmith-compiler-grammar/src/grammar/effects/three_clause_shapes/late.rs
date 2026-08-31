use std::ops::Range;

use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::lexer::{LexStream, OwnedLexToken, TokenKind};

use super::super::sequence_pairs::{
    contains_content_sequence, contains_sequence_phrase, contains_sequence_word,
    matches_complete_sequence, seek_sequence_phrase, sequence_any_phrase, sequence_phrase,
    starts_sequence,
};
use super::{LookedRemainderShape, parse_looked_remainder_shape};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordChoiceSegmentsShape {
    pub segments: Vec<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardTypeIterationShape {
    AmongCastSpells { spell_filter: Range<usize> },
    All,
}

const FROM_AMONG: &[&[&str]] = &[
    &["from", "among", "those", "cards"],
    &["from", "among", "the", "cards", "revealed", "this", "way"],
    &["from", "among", "them"],
];

fn push_comma_segments(tokens: &[OwnedLexToken], base: usize, output: &mut Vec<Range<usize>>) {
    fn push_trimmed(
        tokens: &[OwnedLexToken],
        base: usize,
        mut start: usize,
        mut end: usize,
        output: &mut Vec<Range<usize>>,
    ) {
        while start < end
            && (tokens[start].is_comma()
                || tokens[start].is_period()
                || tokens[start].is_word("and"))
        {
            start += 1;
        }
        while end > start
            && (tokens[end - 1].is_comma()
                || tokens[end - 1].is_period()
                || tokens[end - 1].is_word("and"))
        {
            end -= 1;
        }
        if start < end {
            output.push(base + start..base + end);
        }
    }

    let mut start = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Comma {
            continue;
        }
        push_trimmed(tokens, base, start, index, output);
        start = index + 1;
    }
    push_trimmed(tokens, base, start, tokens.len(), output);
}

pub fn parse_keyword_choice_segments_shape(
    tokens: &[OwnedLexToken],
) -> Option<KeywordChoiceSegmentsShape> {
    let mut input = LexStream::new(tokens);
    crate::grammar::primitives::take_leaf(&mut input, sequence_phrase(&["choose"]))?;
    crate::grammar::primitives::take_leaf(&mut input, sequence_any_phrase(FROM_AMONG))?;
    let tail_start = tokens.len().saturating_sub(input.len());
    let tail = &tokens[tail_start..];
    let mut tail_input = LexStream::new(tail);
    let repeated_at = crate::grammar::primitives::take_leaf(&mut tail_input, |input: &mut _| {
        seek_sequence_phrase(input, &[&["and", "so", "on", "for"]])
    })?;
    crate::grammar::primitives::take_leaf(
        &mut tail_input,
        sequence_phrase(&["and", "so", "on", "for"]),
    )?;
    let suffix_start = tokens.len().saturating_sub(tail_input.len());
    let mut segments = Vec::new();
    push_comma_segments(&tail[..repeated_at], tail_start, &mut segments);
    push_comma_segments(&tokens[suffix_start..], suffix_start, &mut segments);
    (segments.len() >= 3).then_some(KeywordChoiceSegmentsShape { segments })
}

pub fn is_one_chosen_battlefield_others_hand_rest_graveyard_shape(
    tokens: &[OwnedLexToken],
) -> bool {
    starts_sequence(
        tokens,
        &[&[
            "put",
            "one",
            "of",
            "the",
            "chosen",
            "cards",
            "onto",
            "the",
            "battlefield",
        ]],
    ) && contains_sequence_phrase(
        tokens,
        &[&["the", "other", "chosen", "cards", "into", "your", "hand"]],
    ) && contains_sequence_phrase(tokens, &[&["the", "rest", "into", "your", "graveyard"]])
}

const CARD_TYPE_AMONG: &[&str] = &["for", "each", "card", "type", "among"];
const CARD_TYPE: &[&str] = &["for", "each", "card", "type"];
const CAST_TAILS: &[&[&str]] = &[
    &["you've", "cast", "this", "turn"],
    &["youve", "cast", "this", "turn"],
    &["you", "have", "cast", "this", "turn"],
    &["you", "cast", "this", "turn"],
];
const CARD_FROM_REVEALED: &[&[&str]] = &[
    &[
        "card", "of", "that", "type", "from", "among", "the", "revealed", "cards", "into",
    ],
    &[
        "a", "card", "of", "that", "type", "from", "among", "the", "revealed", "cards", "into",
    ],
];

fn phrase_start(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        seek_sequence_phrase(input, &[phrase])
    })
}

fn cast_tail_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut best = None;
    for phrase in CAST_TAILS {
        let Some(offset) = phrase_start(tokens, phrase) else {
            continue;
        };
        if matches_complete_sequence(&tokens[offset..], &[*phrase])
            && best.is_none_or(|current| offset < current)
        {
            best = Some(offset);
        }
    }
    best
}

pub fn parse_card_type_iteration_shape(
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Option<CardTypeIterationShape> {
    let (prefix, kind) = if starts_sequence(second, &[CARD_TYPE_AMONG]) {
        (CARD_TYPE_AMONG, true)
    } else if starts_sequence(second, &[CARD_TYPE]) {
        (CARD_TYPE, false)
    } else {
        return None;
    };
    let prefix_end = {
        let mut input = LexStream::new(second);
        crate::grammar::primitives::take_leaf(&mut input, sequence_phrase(prefix))?;
        second.len().saturating_sub(input.len())
    };
    let put_relative = phrase_start(&second[prefix_end..], &["you", "may", "put"])?;
    let put_start = prefix_end + put_relative;
    let mut put_input = LexStream::new(&second[put_start..]);
    crate::grammar::primitives::take_leaf(&mut put_input, sequence_phrase(&["you", "may", "put"]))?;
    let action_tail_start = second.len().saturating_sub(put_input.len());
    if !starts_sequence(&second[action_tail_start..], CARD_FROM_REVEALED)
        || !contains_sequence_word(&second[action_tail_start..], "hand")
        || !matches!(
            parse_looked_remainder_shape(third),
            Some(LookedRemainderShape::LibraryBottom(_))
        )
    {
        return None;
    }
    if !kind {
        return Some(CardTypeIterationShape::All);
    }
    let filter = prefix_end..put_start;
    let cast_tail = cast_tail_start(&second[filter.clone()])?;
    Some(CardTypeIterationShape::AmongCastSpells {
        spell_filter: filter.start..filter.start + cast_tail,
    })
}

pub fn parse_card_type_iteration_order(third: &[OwnedLexToken]) -> Option<LibraryBottomOrderAst> {
    match parse_looked_remainder_shape(third)? {
        LookedRemainderShape::LibraryBottom(order) => Some(order),
        LookedRemainderShape::Graveyard => None,
    }
}

const ONE_HAND_BOTTOM_EXILE_BOTTOM: &[&[&str]] = &[
    &[
        "put", "one", "of", "them", "on", "the", "bottom", "of", "your", "library",
    ],
    &[
        "put", "one", "of", "them", "on", "bottom", "of", "your", "library",
    ],
];

pub fn is_hand_bottom_exile_split_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(
        tokens,
        &[&["put", "one", "of", "them", "into", "your", "hand"]],
    ) && contains_sequence_phrase(tokens, ONE_HAND_BOTTOM_EXILE_BOTTOM)
        && contains_sequence_phrase(tokens, &[&["exile", "one", "of", "them"]])
}

const ONE_HAND: &[&[&str]] = &[
    &["put", "one", "of", "them", "into", "your", "hand"],
    &["put", "one", "of", "those", "cards", "into", "your", "hand"],
    &["put", "one", "into", "your", "hand"],
];
const OTHER_BOTTOM: &[&[&str]] = &[
    &["other", "on", "bottom"],
    &["other", "onto", "bottom"],
    &["rest", "on", "bottom"],
    &["rest", "onto", "bottom"],
];

pub fn is_nonhand_replacement_looked_split_shape(
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> bool {
    starts_sequence(second, ONE_HAND)
        && contains_content_sequence(second, OTHER_BOTTOM)
        && contains_sequence_word(second, "library")
        && matches_complete_sequence(
            third,
            &[&[
                "if", "this", "spell", "was", "cast", "from", "anywhere", "other", "than", "your",
                "hand", "put", "each", "of", "those", "cards", "into", "your", "hand", "instead",
            ]],
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_line, split_lexed_sentences};

    #[test]
    fn parses_card_type_iteration_shape() {
        let tokens = lex_line(
            "Reveal the top five cards of your library. For each card type among noncreature spells you've cast this turn, you may put a card of that type from among the revealed cards into your hand. Put the rest on the bottom of your library in a random order.",
            0,
        )
        .unwrap();
        let sentences = split_lexed_sentences(&tokens);
        assert!(matches!(
            parse_card_type_iteration_shape(sentences[1], sentences[2]),
            Some(CardTypeIterationShape::AmongCastSpells { .. })
        ));
    }
}
