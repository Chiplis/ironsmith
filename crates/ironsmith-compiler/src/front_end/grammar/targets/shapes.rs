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
pub struct ChosenObjectTarget<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnchantedObjectTargetKind {
    Creature,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetUnionShape {
    PriorPlayerOrPlaneswalker,
    AttackedPlayerOrPlaneswalker,
    BattleOrOpponent,
    CreatureOrPlayer,
    PermanentOrPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailingPlayerTargetKind {
    Any,
    Opponent,
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectOrPlayerUnionTarget<'a> {
    pub object_tokens: &'a [OwnedLexToken],
    pub player_kind: TrailingPlayerTargetKind,
}

#[derive(Debug, Clone)]
pub struct TargetForEachSuffix<'a> {
    pub object_tokens: &'a [OwnedLexToken],
    pub count_words: Vec<&'a str>,
}

pub fn parse_chosen_object_target(tokens: &[OwnedLexToken]) -> Option<ChosenObjectTarget<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    // Postpositive oracle surface: "creature(s) chosen this way". Keep the
    // object head as the executable filter and attach the accumulated chosen
    // set below, just as for the older "chosen creature(s)" surface.
    if let Some(chosen_word_idx) =
        primitives::parse_word_sequence_span(&words, &["chosen", "this", "way"])
            .map(|span| span.start)
        && chosen_word_idx > 0
        && chosen_word_idx + 3 == words.len()
    {
        let token_end = view
            .token_start_indices()
            .get(chosen_word_idx)
            .copied()
            .unwrap_or(tokens.len());
        let filter_tokens = trim_comma_edges(tokens.get(..token_end)?);
        if !filter_tokens.is_empty() {
            return Some(ChosenObjectTarget { filter_tokens });
        }
    }
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

pub fn parse_enchanted_object_target_kind(words: &[&str]) -> Option<EnchantedObjectTargetKind> {
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

pub fn parse_target_union_shape(words: &[&str]) -> Option<TargetUnionShape> {
    if [
        &["that", "player", "or", "planeswalker"][..],
        &["that", "planeswalker", "or", "player"][..],
    ]
    .iter()
    .any(|phrase| exact_phrase(words, phrase))
    {
        return Some(TargetUnionShape::PriorPlayerOrPlaneswalker);
    }
    if phrase_choice_present(words, ATTACKED_PLAYER_OR_PLANESWALKER_PHRASES)
        && word_present(words, "attacking")
        && ["its", "it", "thats", "that"]
            .into_iter()
            .any(|word| word_present(words, word))
    {
        return Some(TargetUnionShape::AttackedPlayerOrPlaneswalker);
    }
    if [
        &["battle", "or", "opponent"][..],
        &["battles", "or", "opponents"][..],
        &["opponent", "or", "battle"][..],
        &["opponents", "or", "battles"][..],
    ]
    .iter()
    .any(|phrase| exact_phrase(words, phrase))
    {
        return Some(TargetUnionShape::BattleOrOpponent);
    }
    if phrase_choice_present(words, CREATURE_OR_PLAYER_PHRASES) {
        return Some(TargetUnionShape::CreatureOrPlayer);
    }
    [
        &["permanent", "or", "player"][..],
        &["permanents", "or", "players"][..],
        &["player", "or", "permanent"][..],
        &["players", "or", "permanents"][..],
    ]
    .iter()
    .any(|phrase| exact_phrase(words, phrase))
    .then_some(TargetUnionShape::PermanentOrPlayer)
}

/// Splits an object-filter/player-domain union whose player arm is last.
///
/// Unlike [`parse_target_union_shape`], this preserves an arbitrarily rich
/// object-filter arm. That matters for phrases such as "artifact, creature,
/// planeswalker, or opponent", where collapsing the phrase to a fixed union
/// shape would discard two of the legal permanent types.
pub fn parse_object_or_player_union_target(
    tokens: &[OwnedLexToken],
) -> Option<ObjectOrPlayerUnionTarget<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    let (&player_word, before_player) = words.split_last()?;
    let player_kind = match player_word {
        "player" | "players" => TrailingPlayerTargetKind::Any,
        "opponent" | "opponents" => TrailingPlayerTargetKind::Opponent,
        _ => return None,
    };
    let connector_start = match before_player {
        [object_words @ .., "or"] | [object_words @ .., "and/or"] if !object_words.is_empty() => {
            object_words.len()
        }
        [object_words @ .., "and", "or"] if !object_words.is_empty() => object_words.len(),
        _ => return None,
    };
    let connector_token = view.map_word_to_token_boundary(connector_start)?;
    let object_tokens = trim_comma_edges(tokens.get(..connector_token)?);
    (!object_tokens.is_empty()).then_some(ObjectOrPlayerUnionTarget {
        object_tokens,
        player_kind,
    })
}

fn exact_phrase(words: &[&str], expected: &[&str]) -> bool {
    let mut input: WordSliceInput<'_> = words;
    parse_dynamic_phrase(&mut input, expected).is_ok() && input.is_empty()
}

pub fn parse_target_for_each_suffix(tokens: &[OwnedLexToken]) -> Option<TargetForEachSuffix<'_>> {
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
            parse_target_union_shape(&["that", "player", "or", "planeswalker"]),
            Some(TargetUnionShape::PriorPlayerOrPlaneswalker)
        );
        assert_eq!(
            parse_target_union_shape(&["player", "or", "planeswalker", "its", "attacking"]),
            Some(TargetUnionShape::AttackedPlayerOrPlaneswalker)
        );
        assert_eq!(
            parse_target_union_shape(&["creature", "and", "or", "player"]),
            Some(TargetUnionShape::CreatureOrPlayer)
        );
        assert_eq!(
            parse_target_union_shape(&["battle", "or", "opponent"]),
            Some(TargetUnionShape::BattleOrOpponent)
        );
        assert_eq!(
            parse_target_union_shape(&["permanent", "or", "player"]),
            Some(TargetUnionShape::PermanentOrPlayer)
        );
    }

    #[test]
    fn object_player_union_preserves_the_complete_object_arm() {
        let tokens = lex_line("artifact, creature, planeswalker, or opponent", 0).unwrap();
        let parsed = parse_object_or_player_union_target(&tokens).unwrap();
        assert_eq!(parsed.player_kind, TrailingPlayerTargetKind::Opponent);
        assert_eq!(
            TokenWordView::new(parsed.object_tokens).to_word_refs(),
            ["artifact", "creature", "planeswalker"]
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
