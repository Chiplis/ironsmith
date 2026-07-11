use std::ops::Range;

use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::super::lexer::{OwnedLexToken, TokenWordView};
use super::super::leaf::parse_leaf_demonstrative_object_head_complete;
use super::super::primitives::{self, WordSliceInput};

const ATTACKED_PLAYER_OR_PLANESWALKER_PHRASES: &[&[&str]] = &[
    &["player", "or", "planeswalker"],
    &["players", "or", "planeswalkers"],
    &["planeswalker", "or", "player"],
    &["planeswalkers", "or", "players"],
];

const CREATURE_OR_PLAYER_PHRASES: &[&[&str]] = &[
    &["creature", "or", "player"],
    &["creatures", "or", "players"],
    &["player", "or", "creature"],
    &["players", "or", "creatures"],
    &["creature", "and", "player"],
    &["creatures", "and", "players"],
    &["player", "and", "creature"],
    &["players", "and", "creatures"],
    &["creature", "and/or", "player"],
    &["creatures", "and/or", "players"],
    &["player", "and/or", "creature"],
    &["players", "and/or", "creatures"],
    &["creature", "and", "or", "player"],
    &["creatures", "and", "or", "players"],
    &["player", "and", "or", "creature"],
    &["players", "and", "or", "creatures"],
];

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChosenObjectTarget<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnchantedObjectTargetKind {
    Creature,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetUnionShape {
    AttackedPlayerOrPlaneswalker,
    CreatureOrPlayer,
}

#[derive(Debug, Clone)]
pub(crate) struct TargetForEachSuffix<'a> {
    pub(crate) object_tokens: &'a [OwnedLexToken],
    pub(crate) count_words: Vec<&'a str>,
}

pub(crate) fn parse_chosen_object_target(
    tokens: &[OwnedLexToken],
) -> Option<ChosenObjectTarget<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    let meaningful = words
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, word)| !is_article(word))
        .collect::<Vec<_>>();
    let (chosen_word_idx, chosen) = meaningful.first().copied()?;
    let (_, object_head) = meaningful.get(1).copied()?;
    if chosen != "chosen" || parse_leaf_demonstrative_object_head_complete(object_head).is_err() {
        return None;
    }
    let filter_start = view
        .token_start_indices()
        .get(chosen_word_idx + 1)
        .copied()
        .unwrap_or(tokens.len());
    let filter_tokens = trim_comma_edges(tokens.get(filter_start..).unwrap_or_default());
    (!filter_tokens.is_empty()).then_some(ChosenObjectTarget { filter_tokens })
}

pub(crate) fn parse_enchanted_object_target_kind(
    words: &[&str],
) -> Option<EnchantedObjectTargetKind> {
    primitives::parse_full_word_slice(
        words,
        alt((
            (
                primitives::word_slice_exact("enchanted"),
                primitives::word_slice_exact("creature"),
            )
                .value(EnchantedObjectTargetKind::Creature),
            (
                primitives::word_slice_exact("enchanted"),
                alt((
                    primitives::word_slice_exact("permanent"),
                    primitives::word_slice_exact("equipment"),
                )),
            )
                .value(EnchantedObjectTargetKind::Other),
        )),
    )
}

pub(crate) fn parse_target_union_shape(words: &[&str]) -> Option<TargetUnionShape> {
    if phrase_choice_present(words, ATTACKED_PLAYER_OR_PLANESWALKER_PHRASES)
        && word_present(words, "attacking")
        && ["its", "it", "thats", "that"]
            .into_iter()
            .any(|word| word_present(words, word))
    {
        return Some(TargetUnionShape::AttackedPlayerOrPlaneswalker);
    }
    phrase_choice_present(words, CREATURE_OR_PLAYER_PHRASES)
        .then_some(TargetUnionShape::CreatureOrPlayer)
}

pub(crate) fn parse_target_for_each_suffix(
    tokens: &[OwnedLexToken],
) -> Option<TargetForEachSuffix<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    let span = phrase_range(&words, &["for", "each"])?;
    if span.start == 0 {
        return None;
    }
    let token_start = view.token_start_indices().get(span.start).copied()?;
    let object_tokens = trim_comma_edges(tokens.get(..token_start).unwrap_or_default());
    if object_tokens.is_empty() {
        return None;
    }
    Some(TargetForEachSuffix {
        object_tokens,
        count_words: words.get(span.start..)?.to_vec(),
    })
}

fn phrase_choice_present(words: &[&str], phrases: &[&[&str]]) -> bool {
    for phrase in phrases {
        if phrase_range(words, phrase).is_some() {
            return true;
        }
    }
    false
}

fn phrase_range(words: &[&str], expected: &[&str]) -> Option<Range<usize>> {
    let mut input: WordSliceInput<'_> = words;
    let mut offset = 0;
    while !input.is_empty() {
        let mut probe = input;
        if parse_dynamic_phrase(&mut probe, expected).is_ok() {
            return Some(offset..offset + expected.len());
        }
        parse_any_word.parse_next(&mut input).ok()?;
        offset += 1;
    }
    None
}

fn parse_dynamic_phrase(input: &mut WordSliceInput<'_>, expected: &[&str]) -> WResult<()> {
    for expected_word in expected {
        let word = parse_any_word.parse_next(input)?;
        if word != *expected_word {
            return Err(primitives::backtrack_err("target phrase", "target phrase"));
        }
    }
    Ok(())
}

fn parse_any_word<'a>(input: &mut WordSliceInput<'a>) -> WResult<&'a str> {
    let Some((word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err("word", "word"));
    };
    *input = rest;
    Ok(*word)
}

fn word_present(words: &[&str], expected: &str) -> bool {
    for word in words {
        if *word == expected {
            return true;
        }
    }
    false
}

fn is_article(word: &str) -> bool {
    matches!(word, "a" | "an" | "the")
}

fn trim_comma_edges(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0;
    let mut end = tokens.len();
    while start < end && tokens[start].is_comma() {
        start += 1;
    }
    while end > start && tokens[end - 1].is_comma() {
        end -= 1;
    }
    tokens.get(start..end).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn chosen_object_parser_returns_filter_tokens() {
        let tokens = lex_line("the chosen creature you control", 0).unwrap();
        let parsed = parse_chosen_object_target(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(parsed.filter_tokens).to_word_refs(),
            ["creature", "you", "control"]
        );
    }

    #[test]
    fn union_shapes_are_typed() {
        assert_eq!(
            parse_target_union_shape(&["player", "or", "planeswalker", "its", "attacking"]),
            Some(TargetUnionShape::AttackedPlayerOrPlaneswalker)
        );
        assert_eq!(
            parse_target_union_shape(&["creature", "and", "or", "player"]),
            Some(TargetUnionShape::CreatureOrPlayer)
        );
    }

    #[test]
    fn for_each_suffix_preserves_object_and_count_words() {
        let tokens = lex_line("creature you control for each artifact", 0).unwrap();
        let parsed = parse_target_for_each_suffix(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(parsed.object_tokens).to_word_refs(),
            ["creature", "you", "control"]
        );
        assert_eq!(parsed.count_words, ["for", "each", "artifact"]);
    }
}
