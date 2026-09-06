use std::ops::Range;

use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, literal};

use super::super::lexer::{LexStream, OwnedLexToken};
use super::primitives;

#[path = "trigger_subjects/reference_words.rs"]
mod reference_words;
use reference_words::{
    parse_simple_copy_reference_words, parse_token_lifecycle_sentence_words,
    parse_trigger_source_subject_word_slice,
};

#[path = "trigger_subjects/subject_facts.rs"]
mod subject_facts;
pub use subject_facts::*;

#[path = "trigger_subjects/spell_activity_facts.rs"]
mod spell_activity_facts;
pub use spell_activity_facts::*;

#[path = "trigger_subjects/may_cast_facts.rs"]
mod may_cast_facts;
pub use may_cast_facts::*;

#[path = "trigger_subjects/sentence_facts.rs"]
mod sentence_facts;
pub use sentence_facts::*;

#[cfg(test)]
#[path = "trigger_subjects/tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerTokenSpan {
    pub first: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct DiscardTriggerEnvelope<'a> {
    pub qualifier: &'a [OwnedLexToken],
    pub trailing: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceOrAnotherShape {
    pub source_word_end: usize,
    pub connector_word: usize,
    pub connector_words: usize,
    pub other_word: usize,
    pub one_or_more: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceOrFilterShape {
    pub source_word_end: usize,
    pub connector_word: usize,
    pub connector_words: usize,
    pub filter_word: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellActivityVerbFacts {
    pub cast: Option<usize>,
    pub copy: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedControllerSubject {
    Enchanted,
    Equipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PossessivePlayerReference {
    EnchantedPlayer,
    AttachedController(AttachedControllerSubject),
    ChosenPlayer,
    You,
    Opponent,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellFilterEnvelope {
    pub end: usize,
}

fn comma_continues_spell_color_list(tokens: &[OwnedLexToken]) -> bool {
    let mut words = tokens.iter().filter_map(OwnedLexToken::as_word);
    match words.next() {
        Some(word) if crate::util::parse_color(word).is_some() => true,
        Some("and" | "or" | "and/or") => words
            .next()
            .is_some_and(|word| crate::util::parse_color(word).is_some()),
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerControllerReference {
    You,
    NotYou,
    ChosenPlayer,
    EnchantedPlayer,
    EffectController,
    AnyPlayer,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerControlSuffix {
    pub controller: TriggerControllerReference,
    pub subject_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerControlPhrase {
    pub controller: TriggerControllerReference,
    pub start: usize,
    pub words: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerSourceSubject {
    AnySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyReferenceCostReductionShape {
    pub reduction_tokens: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleCopyReferenceKind {
    It,
    This,
    That,
    ThatCard,
    ExiledCard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenLifecycleSentenceKind {
    ExileCreatedTokenWhenSourceLeaves,
    SacrificeSourceWhenCreatedTokenLeaves,
}

pub fn parse_trigger_source_subject_words(words: &[&str]) -> Option<TriggerSourceSubject> {
    primitives::parse_full_word_slice(words, parse_trigger_source_subject_word_slice)
}

pub fn parse_copy_reference_cost_reduction_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CopyReferenceCostReductionShape> {
    let (_, after_costs) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["that", "copy", "costs"]),
            primitives::phrase(&["the", "copy", "costs"]),
            primitives::phrase(&["a", "copy", "costs"]),
        )),
    )?;
    let reduction_first = tokens.len().saturating_sub(after_costs.len());
    let (less_relative, _, _) =
        primitives::find_prefix(after_costs, || primitives::phrase(&["less", "to", "cast"]))?;
    if less_relative == 0 {
        return None;
    }
    let reduction_tokens = reduction_first..reduction_first + less_relative;
    Some(CopyReferenceCostReductionShape { reduction_tokens })
}

pub fn parse_simple_copy_reference_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SimpleCopyReferenceKind> {
    let words = primitives::TokenWordView::new(tokens).word_refs();
    primitives::parse_full_word_slice(&words, parse_simple_copy_reference_words)
}

pub fn parse_token_lifecycle_sentence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TokenLifecycleSentenceKind> {
    let words = primitives::TokenWordView::new(tokens).word_refs();
    primitives::parse_full_word_slice(&words, parse_token_lifecycle_sentence_words)
}

pub fn parse_trigger_word_token(tokens: &[OwnedLexToken], expected: &[&str]) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_word_token_lexed(input, expected)
    })
}

pub fn parse_trigger_word_span(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<TriggerTokenSpan> {
    let view = primitives::TokenWordView::new(tokens);
    let first = view.token_start_indices().get(word_index).copied()?;
    let end = view.token_index_after_words(word_index + 1)?;
    Some(TriggerTokenSpan { first, end })
}

pub fn parse_discard_trigger_envelope(
    tokens: &[OwnedLexToken],
) -> Option<DiscardTriggerEnvelope<'_>> {
    let trimmed = trim_commas_ref(tokens);
    let view = primitives::TokenWordView::new(trimmed);
    let words = view.word_refs();
    let card_word = parse_word_slice_index(&words, &["card", "cards"])?;
    let qualifier_end = view
        .token_start_indices()
        .get(card_word)
        .copied()
        .unwrap_or(trimmed.len());
    let trailing_first = view
        .token_start_indices()
        .get(card_word + 1)
        .copied()
        .unwrap_or(trimmed.len());
    Some(DiscardTriggerEnvelope {
        qualifier: trim_commas_ref(&trimmed[..qualifier_end]),
        trailing: trim_commas_ref(&trimmed[trailing_first..]),
    })
}

pub fn parse_source_or_another_shape(words: &[&str]) -> Option<SourceOrAnotherShape> {
    let shape = parse_source_or_filter_shape(words)?;
    let one_or_more_other = words.get(shape.filter_word) == Some(&"one")
        && words.get(shape.filter_word + 1) == Some(&"or")
        && words.get(shape.filter_word + 2) == Some(&"more")
        && words.get(shape.filter_word + 3) == Some(&"other");
    let (other_word, one_or_more) = if one_or_more_other {
        (shape.filter_word + 3, true)
    } else {
        (shape.filter_word, false)
    };
    if !words
        .get(other_word)
        .is_some_and(|word| matches!(*word, "other" | "another"))
    {
        return None;
    }
    Some(SourceOrAnotherShape {
        source_word_end: shape.source_word_end,
        connector_word: shape.connector_word,
        connector_words: shape.connector_words,
        other_word,
        one_or_more,
    })
}

/// Find a coordinated trigger subject whose first arm may be the source and
/// whose second arm is an independently parsed object filter.
///
/// The caller validates that the left arm is actually a source reference.
/// Keeping this shape broader than `source or another ...` is important for
/// authored alternatives such as `this creature or an instant spell`.
pub fn parse_source_or_filter_shape(words: &[&str]) -> Option<SourceOrFilterShape> {
    let connector_word = parse_word_slice_index(words, &["and", "or", "and/or"])?;
    let connector_words = if words.get(connector_word) == Some(&"and")
        && words.get(connector_word + 1) == Some(&"or")
    {
        2
    } else {
        1
    };
    let filter_word = connector_word + connector_words;
    words.get(filter_word)?;
    Some(SourceOrFilterShape {
        source_word_end: connector_word,
        connector_word,
        connector_words,
        filter_word,
    })
}

pub fn parse_spell_activity_verb_facts(tokens: &[OwnedLexToken]) -> SpellActivityVerbFacts {
    SpellActivityVerbFacts {
        cast: parse_trigger_word_token(tokens, &["cast", "casts"]),
        copy: parse_trigger_word_token(tokens, &["copy", "copies"]),
    }
}

pub fn parse_possessive_player_reference(words: &[&str]) -> PossessivePlayerReference {
    if normalized_phrase_occurs(words, &["enchanted", "player"])
        || normalized_phrase_occurs(words, &["enchanted", "players"])
        || normalized_phrase_occurs(words, &["enchanted", "opponent"])
        || normalized_phrase_occurs(words, &["enchanted", "opponents"])
    {
        return PossessivePlayerReference::EnchantedPlayer;
    }
    if attached_controller_occurs(words, "enchanted") {
        return PossessivePlayerReference::AttachedController(AttachedControllerSubject::Enchanted);
    }
    if attached_controller_occurs(words, "equipped") {
        return PossessivePlayerReference::AttachedController(AttachedControllerSubject::Equipped);
    }
    if normalized_phrase_occurs(words, &["chosen", "player"])
        || normalized_phrase_occurs(words, &["chosen", "players"])
    {
        return PossessivePlayerReference::ChosenPlayer;
    }
    if normalized_phrase_occurs(words, &["each", "player"]) {
        return PossessivePlayerReference::Any;
    }
    if exact_phrase_occurs(words, &["your", "team"]) || exact_word_occurs(words, &["your"]) {
        return PossessivePlayerReference::You;
    }
    if normalized_phrase_occurs(words, &["opponent"])
        || normalized_phrase_occurs(words, &["opponents"]) {
        return PossessivePlayerReference::Opponent;
    }
    PossessivePlayerReference::Any
}

pub fn parse_trigger_control_suffix(words: &[&str]) -> Option<TriggerControlSuffix> {
    for suffix_words in [3usize, 2usize] {
        if words.len() < suffix_words {
            continue;
        }
        let subject_end = words.len() - suffix_words;
        if let Some(controller) = parse_trigger_control_tail(&words[subject_end..]) {
            return Some(TriggerControlSuffix {
                controller,
                subject_end,
            });
        }
    }
    None
}

pub fn parse_trigger_control_phrase(words: &[&str]) -> Option<TriggerControlPhrase> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let start = initial_len.saturating_sub(input.len());
        for phrase_words in [3usize, 2usize] {
            if input.len() < phrase_words {
                continue;
            }
            if let Some(controller) = parse_trigger_control_tail(&input[..phrase_words]) {
                return Some(TriggerControlPhrase {
                    controller,
                    start,
                    words: phrase_words,
                });
            }
        }
        crate::grammar::primitives::take_leaf(&mut input, take_word_slice_any)?;
    }
}

pub fn parse_damage_source_surface(
    tokens: &[OwnedLexToken],
) -> crate::triggers::DamageSourceSurface {
    let words = primitives::TokenWordView::new(tokens).word_refs();
    let subject_end = parse_trigger_control_suffix(&words)
        .map(|suffix| suffix.subject_end)
        .unwrap_or(words.len());
    let subject = &words[..subject_end];
    let exact_generic_source =
        primitives::parse_full_word_slice(subject, parse_generic_source_noun_words).is_some();
    let qualified_generic_source = subject
        .last()
        .is_some_and(|word| matches!(*word, "source" | "sources"))
        && !subject
            .iter()
            .any(|word| matches!(*word, "this" | "that" | "it"));
    if exact_generic_source || qualified_generic_source {
        crate::triggers::DamageSourceSurface::Source
    } else {
        crate::triggers::DamageSourceSurface::Filter
    }
}

pub fn parse_spell_or_ability_controller_tail(
    words: &[&str],
) -> Option<TriggerControllerReference> {
    let prefix_words = if word_slice_has_prefix(words, &["a", "spell", "or", "ability"]) {
        4
    } else if word_slice_has_prefix(words, &["spell", "or", "ability"]) {
        3
    } else {
        return None;
    };
    parse_trigger_control_tail(&words[prefix_words..])
}

pub fn parse_spell_controller_tail(words: &[&str]) -> Option<TriggerControllerReference> {
    let prefix_words = if word_slice_has_prefix(words, &["a", "spell"]) {
        2
    } else if word_slice_has_prefix(words, &["spell"]) {
        1
    } else {
        return None;
    };
    parse_trigger_control_tail(&words[prefix_words..])
}

pub fn parse_spell_filter_envelope(tokens: &[OwnedLexToken]) -> SpellFilterEnvelope {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    let mut checked_from = false;
    let mut saw_spell_noun = false;
    loop {
        let end = initial_len.saturating_sub(input.len());
        let next: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = match next {
            Ok(token) => token,
            Err(_) => return SpellFilterEnvelope { end },
        };
        // A comma before the shared `spell` noun belongs to a serial type or
        // subtype list (`instant, sorcery, or Wizard spell`). Once that noun
        // has been consumed, a comma again marks the end of the trigger's
        // object filter.
        if token.is_period()
            || (token.is_comma()
                && saw_spell_noun
                && !comma_continues_spell_color_list(input.as_ref()))
        {
            return SpellFilterEnvelope { end };
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        saw_spell_noun |= matches!(word, "spell" | "spells");
        if word == "during"
            || (word == "other"
                && !input
                    .as_ref()
                    .first()
                    .and_then(OwnedLexToken::as_word)
                    .is_some_and(|next| next == "than"))
        {
            return SpellFilterEnvelope { end };
        }
        if word == "from" && !checked_from {
            checked_from = true;
            let mut tail = input.clone();
            if crate::grammar::primitives::take_leaf(&mut tail, any)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|next| next == "anywhere")
            {
                return SpellFilterEnvelope { end };
            }
        }
    }
}

pub fn parse_clause_before_first_comma(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let trimmed = trim_commas_ref(tokens);
    let mut input = LexStream::new(trimmed);
    let first_comma = crate::grammar::primitives::take_leaf(&mut input, parse_comma_offset_lexed);
    let clause = first_comma
        .map(|index| &trimmed[..index])
        .unwrap_or(trimmed);
    trim_commas_ref(clause).to_vec()
}

fn parse_word_token_lexed<'a>(input: &mut LexStream<'a>, expected: &[&str]) -> WResult<usize> {
    let initial_len = input.len();
    loop {
        let index = initial_len.saturating_sub(input.len());
        let token: &OwnedLexToken = any.parse_next(input)?;
        if token.as_word().is_some_and(|word| {
            expected
                .iter()
                .any(|candidate| word.eq_ignore_ascii_case(candidate))
        }) {
            return Ok(index);
        }
    }
}

fn parse_trigger_control_tail(words: &[&str]) -> Option<TriggerControllerReference> {
    let action_word = words.last().copied()?;
    if !matches!(action_word, "control" | "controls") {
        return None;
    }
    parse_trigger_controller_reference(&words[..words.len().saturating_sub(1)])
}

fn parse_generic_source_noun_words<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<()> {
    (
        opt(alt((
            primitives::word_slice_exact("a"),
            primitives::word_slice_exact("an"),
            primitives::word_slice_exact("the"),
        ))),
        alt((
            primitives::word_slice_exact("source"),
            primitives::word_slice_exact("sources"),
        )),
    )
        .void()
        .parse_next(input)
}

fn parse_trigger_controller_reference(words: &[&str]) -> Option<TriggerControllerReference> {
    if word_slice_is(words, &["you"]) {
        return Some(TriggerControllerReference::You);
    }
    if word_slice_is_any(
        words,
        &[
            &["another", "player"],
            &["a", "player", "other", "than", "you"],
            &["a", "player", "other", "than", "yourself"],
        ],
    ) {
        return Some(TriggerControllerReference::NotYou);
    }
    if word_slice_is_any(
        words,
        &[&["the", "chosen", "player"], &["chosen", "player"]],
    ) {
        return Some(TriggerControllerReference::ChosenPlayer);
    }
    if word_slice_is_any(
        words,
        &[&["enchanted", "player"], &["the", "enchanted", "player"]],
    ) {
        return Some(TriggerControllerReference::EnchantedPlayer);
    }
    if word_slice_has_any_prefix(
        words,
        &[
            &["the", "player", "who", "cast"],
            &["player", "who", "cast"],
        ],
    ) {
        return Some(TriggerControllerReference::EffectController);
    }
    if word_slice_is_any(
        words,
        &[
            &["a", "player"],
            &["any", "player"],
            &["player"],
            &["players"],
            &["one", "or", "more", "players"],
        ],
    ) {
        return Some(TriggerControllerReference::AnyPlayer);
    }
    if word_slice_is_any(
        words,
        &[
            &["an", "opponent"],
            &["opponent"],
            &["opponents"],
            &["your", "opponents"],
            &["one", "of", "your", "opponents"],
            &["one", "or", "more", "of", "your", "opponents"],
            &["one", "of", "the", "opponents"],
            &["one", "or", "more", "opponents"],
            &["each", "opponent"],
        ],
    ) {
        return Some(TriggerControllerReference::Opponent);
    }
    if word_slice_has_suffix(words, &["on", "your", "team"])
        && word_slice_has_any_word(words, &["player", "players"])
    {
        return Some(TriggerControllerReference::You);
    }
    None
}

fn word_slice_is(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_exact_phrase(&mut input, expected).is_ok() && input.is_empty()
}

fn word_slice_is_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| word_slice_is(words, expected))
}

fn word_slice_has_prefix(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_exact_phrase(&mut input, expected).is_ok()
}

fn word_slice_has_any_prefix(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| word_slice_has_prefix(words, expected))
}

fn word_slice_has_suffix(words: &[&str], expected: &[&str]) -> bool {
    if words.len() < expected.len() {
        return false;
    }
    word_slice_is(&words[words.len() - expected.len()..], expected)
}

fn word_slice_has_any_word(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    while let Ok(word) = take_word_slice_any(&mut input) {
        if expected.contains(&word) {
            return true;
        }
    }
    false
}

fn parse_word_slice_index(words: &[&str], expected: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let index = initial_len.saturating_sub(input.len());
        let word = crate::grammar::primitives::take_leaf(&mut input, take_word_slice_any)?;
        if expected.contains(&word) {
            return Some(index);
        }
    }
}

fn parse_comma_offset_lexed<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let initial_len = input.len();
    loop {
        let index = initial_len.saturating_sub(input.len());
        let mut comma = input.clone();
        if primitives::comma().parse_next(&mut comma).is_ok() {
            *input = comma;
            return Ok(index);
        }
        any.parse_next(input)?;
    }
}

fn take_word_slice_any<'word>(input: &mut &[&'word str]) -> WResult<&'word str> {
    any.parse_next(input)
}

fn normalized_phrase_occurs(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    loop {
        let mut candidate = input;
        if parse_normalized_phrase(&mut candidate, expected).is_ok() {
            return true;
        }
        if take_word_slice_any(&mut input).is_err() {
            return false;
        }
    }
}

#[path = "trigger_subjects/core.rs"]
mod core_programs;
use core_programs::{
    exact_phrase_occurs, exact_word_occurs, normalized_word_matches, parse_exact_phrase,
    parse_normalized_phrase, parse_normalized_word, trim_commas_ref,
};
#[path = "trigger_subjects/choice.rs"]
mod choice_programs;
use choice_programs::parse_normalized_word_choice;
#[path = "trigger_subjects/object_action.rs"]
mod object_action_programs;
use object_action_programs::attached_controller_occurs;
