use winnow::combinator::alt;
use winnow::prelude::*;

use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, TokenWordView, trim_lexed_commas};

use super::common::{ChoiceDamageScope, is_choice_damage_drain_shape, parse_choice_damage_scope};

#[derive(Clone, Copy, Debug)]
pub(crate) struct OpponentDrainSentenceShape<'a> {
    pub(crate) where_tokens: &'a [OwnedLexToken],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RevealSelectedHandShape<'a> {
    pub(crate) descriptor_tokens: &'a [OwnedLexToken],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RandomHandRevealShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) descriptor_tokens: &'a [OwnedLexToken],
    pub(crate) hand_tokens: &'a [OwnedLexToken],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DamageUnlessShape<'a> {
    pub(crate) damage_tokens: &'a [OwnedLexToken],
    pub(crate) condition_tokens: &'a [OwnedLexToken],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RelativeOpponentDamageDifferenceShape<'a> {
    pub(crate) source_tokens: &'a [OwnedLexToken],
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct UnlessSentenceShape<'a> {
    pub(crate) unless_token: usize,
    pub(crate) action_tokens: &'a [OwnedLexToken],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EachOpponentReturnUnlessDrawShape {
    pub(crate) target_start_word: usize,
    pub(crate) target_end_word: usize,
}

fn each_opponent_prefix<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    alt((
        primitives::phrase(&["for", "each", "opponent"]),
        primitives::phrase(&["for", "each", "opponents"]),
        primitives::phrase(&["each", "opponent"]),
        primitives::phrase(&["each", "opponents"]),
    ))
    .parse_next(input)
}

fn where_x_is<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    primitives::phrase(&["where", "x", "is"]).parse_next(input)
}

pub(crate) fn parse_opponent_drain_sentence_shape(
    tokens: &[OwnedLexToken],
) -> Option<OpponentDrainSentenceShape<'_>> {
    let (_, body) = primitives::parse_prefix(tokens, each_opponent_prefix)?;
    let (where_offset, _, _) = primitives::find_prefix(body, || where_x_is)?;
    let drain_tokens = body.get(..where_offset)?;
    let drain_words = TokenWordView::new(drain_tokens).to_word_refs();
    if !is_choice_damage_drain_shape(&drain_words) {
        return None;
    }
    let where_tokens = body.get(where_offset..)?;
    (TokenWordView::new(where_tokens).len() > 3)
        .then_some(OpponentDrainSentenceShape { where_tokens })
}

fn hand_suffix<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    alt((
        primitives::phrase(&["in", "your", "hand"]),
        primitives::phrase(&["in", "your", "hands"]),
        primitives::phrase(&["from", "your", "hand"]),
        primitives::phrase(&["from", "your", "hands"]),
    ))
    .parse_next(input)
}

pub(crate) fn parse_reveal_selected_hand_shape(
    tokens: &[OwnedLexToken],
) -> Option<RevealSelectedHandShape<'_>> {
    let (_, body) = primitives::parse_prefix(tokens, primitives::kw("reveal"))?;
    let (suffix_offset, _, _) = primitives::find_prefix(body, || hand_suffix)?;
    let descriptor_tokens = trim_lexed_commas(body.get(..suffix_offset)?);
    (!descriptor_tokens.is_empty()).then_some(RevealSelectedHandShape { descriptor_tokens })
}

fn reveal_verb<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    alt((primitives::kw("reveal"), primitives::kw("reveals")))
        .void()
        .parse_next(input)
}

fn reveal_article<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("one"),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_random_hand_reveal_shape(
    tokens: &[OwnedLexToken],
) -> Option<RandomHandRevealShape<'_>> {
    let (reveal_offset, _, after_reveal) = primitives::find_prefix(tokens, || reveal_verb)?;
    let subject_tokens = trim_lexed_commas(tokens.get(..reveal_offset)?);
    if subject_tokens.is_empty() {
        return None;
    }
    let (_, descriptor_body) = primitives::parse_prefix(after_reveal, reveal_article)?;
    let (from_offset, _, hand_tokens) =
        primitives::find_prefix(descriptor_body, || primitives::kw("from"))?;
    let descriptor_tokens = trim_lexed_commas(descriptor_body.get(..from_offset)?);
    let hand_tokens = trim_lexed_commas(hand_tokens);
    if descriptor_tokens.is_empty() || hand_tokens.is_empty() {
        return None;
    }
    Some(RandomHandRevealShape {
        subject_tokens,
        descriptor_tokens,
        hand_tokens,
    })
}

pub(crate) fn parse_damage_unless_shape(tokens: &[OwnedLexToken]) -> Option<DamageUnlessShape<'_>> {
    let (damage_tokens, condition_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("unless").void())?;
    let damage_tokens = trim_lexed_commas(damage_tokens);
    let condition_tokens = trim_lexed_commas(condition_tokens);
    if damage_tokens.is_empty() || condition_tokens.is_empty() {
        return None;
    }
    Some(DamageUnlessShape {
        damage_tokens,
        condition_tokens,
    })
}

pub(crate) fn parse_relative_opponent_damage_difference_shape(
    tokens: &[OwnedLexToken],
) -> Option<RelativeOpponentDamageDifferenceShape<'_>> {
    let (damage_offset, _, after_damage) = primitives::find_prefix(tokens, || {
        primitives::phrase(&[
            "deals", "damage", "to", "each", "opponent", "who", "controls", "more",
        ])
        .void()
    })?;
    let source_tokens = trim_lexed_commas(tokens.get(..damage_offset)?);
    if source_tokens.is_empty() {
        return None;
    }

    let (suffix_offset, _, after_suffix) = primitives::find_prefix(after_damage, || {
        primitives::phrase(&["than", "you", "equal", "to", "the", "difference"]).void()
    })?;
    let filter_tokens = trim_lexed_commas(after_damage.get(..suffix_offset)?);
    if filter_tokens.is_empty() || TokenWordView::new(after_suffix).len() != 0 {
        return None;
    }

    Some(RelativeOpponentDamageDifferenceShape {
        source_tokens,
        filter_tokens,
    })
}

pub(crate) fn parse_enchanted_attacked_damage_shape(
    tokens: &[OwnedLexToken],
) -> Option<DamageUnlessShape<'_>> {
    let shape = parse_damage_unless_shape(tokens)?;
    let tail_words = TokenWordView::new(shape.condition_tokens).to_word_refs();
    let matches_tail = [
        &["that", "creature", "attacked", "this", "turn"][..],
        &["enchanted", "creature", "attacked", "this", "turn"][..],
    ]
    .iter()
    .any(|expected| primitives::parse_word_sequence_complete(&tail_words, expected).is_some());
    matches_tail.then_some(shape)
}

pub(crate) fn parse_unless_sentence_shape(
    tokens: &[OwnedLexToken],
) -> Option<UnlessSentenceShape<'_>> {
    let (action_tokens, _) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("unless").void())?;
    let unless_token = action_tokens.len();
    Some(UnlessSentenceShape {
        unless_token,
        action_tokens,
    })
}

fn phrase_word_offset(words: &[&str], phrase: &[&'static str]) -> Option<usize> {
    let mut offset = 0usize;
    while offset < words.len() {
        let mut input = &words[offset..];
        let mut matched = true;
        for word in phrase {
            if primitives::word_slice_exact(word)
                .parse_next(&mut input)
                .is_err()
            {
                matched = false;
                break;
            }
        }
        if matched {
            return Some(offset);
        }
        offset += 1;
    }
    None
}

pub(crate) fn parse_each_opponent_return_unless_draw_shape(
    words: &[&str],
) -> Option<EachOpponentReturnUnlessDrawShape> {
    if parse_choice_damage_scope(words) != Some(ChoiceDamageScope::Opponent) {
        return None;
    }
    let unless_word = phrase_word_offset(words, &["unless"])?;
    let required_tail = ["its", "controller", "has", "you", "draw", "a", "card"];
    let tail = words.get(unless_word + 1..unless_word + 8)?;
    if tail != required_tail {
        return None;
    }
    let then_return = phrase_word_offset(words.get(..unless_word)?, &["then", "return"])?;
    if words.get(3).copied() != Some("choose") || then_return <= 4 {
        return None;
    }
    Some(EachOpponentReturnUnlessDrawShape {
        target_start_word: 4,
        target_end_word: then_return,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).unwrap()
    }

    #[test]
    fn captures_drain_reveal_and_unless_sentences() {
        let drain = lex("Each opponent loses X life and you gain X life, where X is 3.");
        assert!(parse_opponent_drain_sentence_shape(&drain).is_some());

        let reveal = lex("Reveal two creature cards from your hand.");
        assert_eq!(
            TokenWordView::new(
                parse_reveal_selected_hand_shape(&reveal)
                    .unwrap()
                    .descriptor_tokens
            )
            .to_word_refs(),
            ["two", "creature", "cards"]
        );

        let unless = lex("Destroy target creature unless its controller pays {2}.");
        assert!(parse_unless_sentence_shape(&unless).is_some());

        let relative = lex(
            "This spell deals damage to each opponent who controls more lands than you equal to the difference.",
        );
        let shape = parse_relative_opponent_damage_difference_shape(&relative).unwrap();
        assert_eq!(
            TokenWordView::new(shape.source_tokens).to_word_refs(),
            ["this", "spell"]
        );
        assert_eq!(
            TokenWordView::new(shape.filter_tokens).to_word_refs(),
            ["lands"]
        );
    }
}
