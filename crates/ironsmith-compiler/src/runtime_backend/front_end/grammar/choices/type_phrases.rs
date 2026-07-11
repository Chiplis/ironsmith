use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::color::ColorSet;
use crate::types::{CardType, Subtype};

use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoiceTypePhraseSyntaxError {
    MissingCreatureSubtypeExclusion,
    UnsupportedCreatureSubtypeExclusion,
    MissingColorExclusion,
    UnsupportedColorExclusion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChoiceCreatureTypePhrase {
    pub(crate) consumed: usize,
    pub(crate) excluded_subtypes: Vec<Subtype>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChoiceColorPhrase {
    pub(crate) consumed: usize,
    pub(crate) excluded: Option<ColorSet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChoiceCardTypePhrase {
    pub(crate) consumed: usize,
    pub(crate) options: Vec<CardType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChoiceSimpleTypePhrase {
    pub(crate) consumed: usize,
}

pub(crate) fn parse_choice_creature_type_phrase_words(
    words: &[&str],
) -> Result<Option<ChoiceCreatureTypePhrase>, ChoiceTypePhraseSyntaxError> {
    let mut input: primitives::WordSliceInput<'_> = words;
    if parse_choose_prefix(&mut input).is_err() {
        return Ok(None);
    }
    if parse_word_phrase(&mut input, &["creature", "type"]).is_err() {
        return Ok(None);
    }

    let mut excluded_subtypes = Vec::new();
    let mut exclusion_probe = input;
    if parse_word_phrase(&mut exclusion_probe, &["other", "than"]).is_ok() {
        input = exclusion_probe;
        let subtype_word = take_word(&mut input)
            .map_err(|_| ChoiceTypePhraseSyntaxError::MissingCreatureSubtypeExclusion)?;
        let subtype = leaf::parse_leaf_subtype_flexible_complete(subtype_word)
            .map_err(|_| ChoiceTypePhraseSyntaxError::UnsupportedCreatureSubtypeExclusion)?;
        excluded_subtypes.push(subtype);
    }

    Ok(Some(ChoiceCreatureTypePhrase {
        consumed: words.len().saturating_sub(input.len()),
        excluded_subtypes,
    }))
}

pub(crate) fn parse_choice_color_phrase_words(
    words: &[&str],
) -> Result<Option<ChoiceColorPhrase>, ChoiceTypePhraseSyntaxError> {
    let mut input: primitives::WordSliceInput<'_> = words;
    if parse_choose_prefix(&mut input).is_err() {
        return Ok(None);
    }
    if primitives::word_slice_exact("color")
        .parse_next(&mut input)
        .is_err()
    {
        return Ok(None);
    }

    let mut excluded = None;
    let mut exclusion_probe = input;
    if parse_word_phrase(&mut exclusion_probe, &["other", "than"]).is_ok() {
        input = exclusion_probe;
        let color_word = take_word(&mut input)
            .map_err(|_| ChoiceTypePhraseSyntaxError::MissingColorExclusion)?;
        excluded = Some(
            leaf::parse_leaf_color_complete(color_word)
                .map_err(|_| ChoiceTypePhraseSyntaxError::UnsupportedColorExclusion)?,
        );
    }

    Ok(Some(ChoiceColorPhrase {
        consumed: words.len().saturating_sub(input.len()),
        excluded,
    }))
}

pub(crate) fn parse_choice_card_type_phrase_words(words: &[&str]) -> Option<ChoiceCardTypePhrase> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_choose_prefix(&mut input).ok()?;

    let mut generic_probe = input;
    if parse_word_phrase(&mut generic_probe, &["card", "type"]).is_ok() {
        input = generic_probe;
        return Some(ChoiceCardTypePhrase {
            consumed: words.len().saturating_sub(input.len()),
            options: Vec::new(),
        });
    }

    let mut permanent_probe = input;
    if primitives::word_slice_exact("permanent")
        .parse_next(&mut permanent_probe)
        .is_ok()
        && alt((
            primitives::word_slice_exact("type"),
            primitives::word_slice_exact("types"),
        ))
        .parse_next(&mut permanent_probe)
        .is_ok()
    {
        input = permanent_probe;
        return Some(ChoiceCardTypePhrase {
            consumed: words.len().saturating_sub(input.len()),
            options: vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ],
        });
    }

    let mut options = Vec::new();
    loop {
        let mut connector_probe = input;
        if alt((
            primitives::word_slice_exact("or"),
            primitives::word_slice_exact("and"),
        ))
        .parse_next(&mut connector_probe)
        .is_ok()
        {
            input = connector_probe;
            continue;
        }

        let mut type_probe = input;
        let Ok(word) = take_word(&mut type_probe) else {
            break;
        };
        let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) else {
            break;
        };
        crate::slice_primitives::push_unique(&mut options, card_type);
        input = type_probe;
    }
    if options.is_empty() {
        return None;
    }

    Some(ChoiceCardTypePhrase {
        consumed: words.len().saturating_sub(input.len()),
        options,
    })
}

pub(crate) fn parse_choice_player_phrase_words(words: &[&str]) -> Option<ChoiceSimpleTypePhrase> {
    parse_simple_choice_phrase(words, &["player"])
}

pub(crate) fn parse_choice_basic_land_type_phrase_words(
    words: &[&str],
) -> Option<ChoiceSimpleTypePhrase> {
    parse_simple_choice_phrase(words, &["basic", "land", "type"])
}

pub(crate) fn parse_choice_land_type_phrase_words(
    words: &[&str],
) -> Option<ChoiceSimpleTypePhrase> {
    parse_simple_choice_phrase(words, &["land", "type"])
}

fn parse_simple_choice_phrase(
    words: &[&str],
    phrase: &'static [&'static str],
) -> Option<ChoiceSimpleTypePhrase> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_choose_prefix(&mut input).ok()?;
    parse_word_phrase(&mut input, phrase).ok()?;
    Some(ChoiceSimpleTypePhrase {
        consumed: words.len().saturating_sub(input.len()),
    })
}

fn parse_choose_prefix(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("choose"),
        primitives::word_slice_exact("chooses"),
    ))
    .parse_next(input)?;
    opt(alt((
        primitives::word_slice_exact("a"),
        primitives::word_slice_exact("an"),
        primitives::word_slice_exact("the"),
    )))
    .parse_next(input)?;
    Ok(())
}

fn parse_word_phrase<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &'static [&'static str],
) -> WResult<()> {
    for word in expected {
        primitives::word_slice_exact(word).parse_next(input)?;
    }
    Ok(())
}

fn take_word<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<&'a str> {
    any.parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creature_and_color_choices_return_typed_exclusions() {
        let creature = parse_choice_creature_type_phrase_words(&[
            "choose", "a", "creature", "type", "other", "than", "dragon", "now",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(creature.consumed, 7);
        assert_eq!(creature.excluded_subtypes, [Subtype::Dragon]);

        let color = parse_choice_color_phrase_words(&[
            "choose", "a", "color", "other", "than", "blue", "now",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(color.consumed, 6);
        assert_eq!(color.excluded, Some(ColorSet::BLUE));
    }

    #[test]
    fn card_type_choices_return_typed_options() {
        let parsed = parse_choice_card_type_phrase_words(&[
            "choose", "artifact", "creature", "or", "land", "now",
        ])
        .unwrap();
        assert_eq!(parsed.consumed, 5);
        assert_eq!(
            parsed.options,
            [CardType::Artifact, CardType::Creature, CardType::Land]
        );
    }

    #[test]
    fn simple_choice_phrases_report_consumed_words() {
        assert_eq!(
            parse_choice_basic_land_type_phrase_words(&[
                "choose", "a", "basic", "land", "type", "now"
            ])
            .unwrap()
            .consumed,
            5
        );
        assert_eq!(
            parse_choice_player_phrase_words(&["choose", "a", "player", "now"])
                .unwrap()
                .consumed,
            3
        );
    }
}
