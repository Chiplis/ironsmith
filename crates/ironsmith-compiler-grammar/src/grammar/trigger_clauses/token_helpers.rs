use std::ops::Range;

use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::events::KeywordActionKind;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::{keyword_action_costs, leaf, primitives};
use super::{TriggerClauseAtom, TriggerOrSplit};

pub(super) fn parse_players_attacked_words<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> WResult<usize> {
    let initial_len = input.len();
    loop {
        let player_words = initial_len.saturating_sub(input.len());
        let mut suffix = *input;
        if player_words > 0
            && (
                primitives::word_slice_exact("are"),
                primitives::word_slice_exact("attacked"),
                primitives::word_slice_eof,
            )
                .parse_next(&mut suffix)
                .is_ok()
        {
            *input = suffix;
            return Ok(player_words);
        }
        let _: &str = any.parse_next(input)?;
    }
}

pub(super) fn parse_fully_unlock_room_words<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> WResult<()> {
    (
        primitives::word_slice_exact("you"),
        primitives::word_slice_exact("fully"),
        winnow::combinator::alt((
            primitives::word_slice_exact("unlock"),
            primitives::word_slice_exact("unlocked"),
        )),
        primitives::word_slice_exact("a"),
        primitives::word_slice_exact("room"),
    )
        .void()
        .parse_next(input)
}

pub(super) fn parse_atom_token_lexed<'a>(
    input: &mut LexStream<'a>,
    atom: TriggerClauseAtom,
) -> WResult<usize> {
    let initial_len = input.len();
    loop {
        let index = initial_len.saturating_sub(input.len());
        let token: &OwnedLexToken = any.parse_next(input)?;
        if token.as_word().is_some_and(|word| atom_matches(word, atom)) {
            return Ok(index);
        }
    }
}

pub(super) fn parse_atom_word_slice<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    atom: TriggerClauseAtom,
) -> WResult<usize> {
    let initial_len = input.len();
    loop {
        let index = initial_len.saturating_sub(input.len());
        let word: &str = any.parse_next(input)?;
        if atom_matches(word, atom) {
            return Ok(index);
        }
    }
}

pub(super) fn parse_keyword_action_word_slice<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    action: KeywordActionKind,
) -> WResult<usize> {
    let initial_len = input.len();
    loop {
        let index = initial_len.saturating_sub(input.len());
        let word: &str = any.parse_next(input)?;
        if KeywordActionKind::from_trigger_word(word) == Some(action) {
            return Ok(index);
        }
    }
}

pub(super) fn parse_activation_cost_tap_condition_words<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> WResult<(usize, bool)> {
    let initial_len = input.len();
    loop {
        let condition_word = initial_len.saturating_sub(input.len());
        let mut candidate = *input;
        let required = match primitives::word_slice_exact("with").parse_next(&mut candidate) {
            Ok(_) => true,
            Err(_) => match primitives::word_slice_exact("without").parse_next(&mut candidate) {
                Ok(_) => false,
                Err(_) => {
                    any.parse_next(input)?;
                    continue;
                }
            },
        };
        if (
            primitives::word_slice_exact("t"),
            primitives::word_slice_exact("in"),
            primitives::word_slice_exact("its"),
            primitives::word_slice_exact("activation"),
            primitives::word_slice_exact("cost"),
        )
            .parse_next(&mut candidate)
            .is_err()
        {
            any.parse_next(input)?;
            continue;
        }
        *input = candidate;
        return Ok((condition_word, required));
    }
}

pub(super) fn parse_trigger_or_split_lexed<'a>(
    input: &mut LexStream<'a>,
    tokens: &'a [OwnedLexToken],
) -> WResult<TriggerOrSplit> {
    let initial_len = input.len();
    loop {
        let separator = initial_len.saturating_sub(input.len());
        let token: &OwnedLexToken = any.parse_next(input)?;
        if !token.is_word("or") || guarded_or(tokens, separator) {
            continue;
        }
        return Ok(TriggerOrSplit { separator });
    }
}

pub(super) fn guarded_or(tokens: &[OwnedLexToken], separator: usize) -> bool {
    let previous = previous_token_word(tokens, separator);
    let next = tokens.get(separator + 1).and_then(OwnedLexToken::as_word);
    let previous_immediate = separator
        .checked_sub(1)
        .and_then(|index| tokens.get(index))
        .and_then(OwnedLexToken::as_word);
    let spell_clause =
        token_words_have_exact(tokens, "spell") || token_words_have_exact(tokens, "spells");
    let quantifier = previous_immediate == Some("one") && next == Some("more");
    let comparison = matches!(next, Some("less" | "greater" | "more" | "fewer"))
        || (previous == Some("than") && next == Some("equal"));
    let previous_numeric = previous_numeric_word(tokens, separator);
    let next_numeric = next.is_some_and(|word| word.parse::<i32>().is_ok());
    let numeric_list = previous_numeric && next_numeric;
    let color_list = previous_immediate.is_some_and(is_color_word)
        && next.is_some_and(is_color_word)
        && spell_clause;
    let object_list =
        previous_immediate.is_some_and(is_objectish_word) && next.is_some_and(is_objectish_word);
    let and_or_list = previous_immediate == Some("and")
        && next.is_some_and(|word| is_color_word(word) || is_objectish_word(word));
    let serial_spell_list = spell_clause
        && previous.is_some_and(|word| is_color_word(word) || is_objectish_word(word))
        && next.is_some_and(|word| is_color_word(word) || is_objectish_word(word));
    let cast_or_copy = spell_clause
        && previous.is_some_and(|word| matches!(word, "cast" | "casts"))
        && next.is_some_and(|word| matches!(word, "copy" | "copies"));
    let spell_or_ability = previous_immediate
        .is_some_and(|word| matches!(word, "spell" | "spells"))
        && next.is_some_and(|word| matches!(word, "ability" | "abilities"));
    quantifier
        || comparison
        || numeric_list
        || color_list
        || object_list
        || and_or_list
        || serial_spell_list
        || cast_or_copy
        || spell_or_ability
}

pub(super) fn previous_token_word(tokens: &[OwnedLexToken], before: usize) -> Option<&str> {
    let mut index = before;
    while index > 0 {
        index -= 1;
        if let Some(word) = tokens[index].as_word() {
            return Some(word);
        }
    }
    None
}

pub(super) fn previous_numeric_word(tokens: &[OwnedLexToken], before: usize) -> bool {
    let mut index = before;
    while index > 0 {
        index -= 1;
        if let Some(word) = tokens[index].as_word() {
            return word.parse::<i32>().is_ok();
        }
    }
    false
}

pub(super) fn token_words_have_exact(tokens: &[OwnedLexToken], expected: &str) -> bool {
    let mut input = LexStream::new(tokens);
    loop {
        let token: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = token else {
            return false;
        };
        if token.as_word() == Some(expected) {
            return true;
        }
    }
}

pub(super) fn is_color_word(word: &str) -> bool {
    leaf::parse_leaf_color_complete(word).is_ok()
}

pub(super) fn is_objectish_word(word: &str) -> bool {
    keyword_action_costs::parse_keyword_trigger_object_head(word).is_some()
}

pub(super) fn parse_counter_quantifier_word_count(words: &[&str]) -> usize {
    let mut input: primitives::WordSliceInput<'_> = words;
    if (
        primitives::word_slice_exact("one"),
        primitives::word_slice_exact("or"),
        primitives::word_slice_exact("more"),
    )
        .parse_next(&mut input)
        .is_ok()
    {
        3
    } else if words
        .first()
        .is_some_and(|word| matches!(*word, "a" | "an"))
    {
        1
    } else {
        0
    }
}

pub(super) fn parse_energy_descriptor_words(words: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    loop {
        let word: WResult<&str> = any.parse_next(&mut input);
        let Ok(word) = word else {
            return false;
        };
        if matches!(word, "e" | "energy") {
            return true;
        }
    }
}

pub(super) fn parse_article_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    let token: &OwnedLexToken = any.parse_next(input)?;
    if token
        .as_word()
        .is_some_and(|word| matches!(word, "a" | "an" | "the"))
    {
        Ok(())
    } else {
        Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ))
    }
}

pub(super) fn trim_comma_range(tokens: &[OwnedLexToken], mut range: Range<usize>) -> Range<usize> {
    while range.start < range.end && tokens[range.start].is_comma() {
        range.start += 1;
    }
    while range.start < range.end && tokens[range.end - 1].is_comma() {
        range.end -= 1;
    }
    range
}

pub(super) fn parse_atom_word_from(words: &[&str], first: usize, expected: &str) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words.get(first..)?;
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let word = crate::grammar::primitives::take_leaf(&mut input, any)?;
        if word == expected {
            return Some(first + offset);
        }
    }
}

pub(super) fn parse_last_possessive_word(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    let mut found = None;
    loop {
        let index = initial_len.saturating_sub(input.len());
        let word: WResult<&str> = any.parse_next(&mut input);
        let Ok(word) = word else {
            return found;
        };
        let final_byte = word.as_bytes().last().copied();
        if final_byte == Some(b's') && !matches!(word, "this" | "its") {
            found = Some(index);
        }
    }
}

pub(super) fn word_slice_has_exact(words: &[&str], expected: &str) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    loop {
        let word: WResult<&str> = any.parse_next(&mut input);
        let Ok(word) = word else {
            return false;
        };
        if word == expected {
            return true;
        }
    }
}

pub(super) fn atom_matches(word: &str, atom: TriggerClauseAtom) -> bool {
    match atom {
        TriggerClauseAtom::Ability => matches!(word, "ability" | "abilities"),
        TriggerClauseAtom::Activate => matches!(word, "activate" | "activates"),
        TriggerClauseAtom::And => word == "and",
        TriggerClauseAtom::Attack => matches!(word, "attack" | "attacks"),
        TriggerClauseAtom::Becomes => word == "becomes",
        TriggerClauseAtom::Block => matches!(word, "block" | "blocks"),
        TriggerClauseAtom::By => word == "by",
        TriggerClauseAtom::Cast => matches!(word, "cast" | "casts"),
        TriggerClauseAtom::Copy => matches!(word, "copy" | "copies"),
        TriggerClauseAtom::Counter => matches!(word, "counter" | "counters"),
        TriggerClauseAtom::Create => matches!(word, "create" | "creates"),
        TriggerClauseAtom::Damage => word == "damage",
        TriggerClauseAtom::Deal => matches!(word, "deal" | "deals"),
        TriggerClauseAtom::Die => matches!(word, "die" | "dies"),
        TriggerClauseAtom::Discard => matches!(word, "discard" | "discards"),
        TriggerClauseAtom::Draw => matches!(word, "draw" | "draws"),
        TriggerClauseAtom::Enter => matches!(word, "enter" | "enters"),
        TriggerClauseAtom::For => word == "for",
        TriggerClauseAtom::Get => matches!(word, "get" | "gets"),
        TriggerClauseAtom::Give => matches!(word, "give" | "gives"),
        TriggerClauseAtom::IsOrAre => matches!(word, "is" | "are"),
        TriggerClauseAtom::Leave => matches!(word, "leave" | "leaves"),
        TriggerClauseAtom::Mana => word == "mana",
        TriggerClauseAtom::More => word == "more",
        TriggerClauseAtom::One => word == "one",
        TriggerClauseAtom::Or => word == "or",
        TriggerClauseAtom::Play => matches!(word, "play" | "plays"),
        TriggerClauseAtom::Put => matches!(word, "put" | "puts"),
        TriggerClauseAtom::Reveal => matches!(word, "reveal" | "reveals"),
        TriggerClauseAtom::Roll => matches!(word, "roll" | "rolls"),
        TriggerClauseAtom::Sacrifice => matches!(word, "sacrifice" | "sacrifices"),
        TriggerClauseAtom::Search => matches!(word, "search" | "searches"),
        TriggerClauseAtom::Shuffle => matches!(word, "shuffle" | "shuffles"),
        TriggerClauseAtom::Tap => matches!(word, "tap" | "taps"),
        TriggerClauseAtom::Tapped => word == "tapped",
        TriggerClauseAtom::To => word == "to",
        TriggerClauseAtom::Transform => matches!(word, "transform" | "transforms"),
        TriggerClauseAtom::TriggerIntro => matches!(word, "when" | "whenever" | "at"),
    }
}
