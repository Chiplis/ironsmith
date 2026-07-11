use std::ops::Range;

use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, literal};

use super::super::lexer::{LexStream, OwnedLexToken};
use super::primitives;

#[path = "trigger_subjects/surface_shapes.rs"]
mod surface_shapes;
pub(crate) use surface_shapes::*;

#[path = "trigger_subjects/reference_words.rs"]
mod reference_words;
use reference_words::{
    parse_simple_copy_reference_words, parse_token_lifecycle_sentence_words,
    parse_trigger_source_subject_word_slice,
};

#[cfg(test)]
#[path = "trigger_subjects/tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerTokenSpan {
    pub(crate) first: usize,
    pub(crate) end: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DiscardTriggerEnvelope<'a> {
    pub(crate) qualifier: &'a [OwnedLexToken],
    pub(crate) trailing: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceOrAnotherShape {
    pub(crate) source_word_end: usize,
    pub(crate) other_word: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellActivityVerbFacts {
    pub(crate) cast: Option<usize>,
    pub(crate) copy: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttachedControllerSubject {
    Enchanted,
    Equipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PossessivePlayerReference {
    EnchantedPlayer,
    AttachedController(AttachedControllerSubject),
    You,
    Opponent,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellFilterEnvelope {
    pub(crate) end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerControllerReference {
    You,
    NotYou,
    ChosenPlayer,
    EnchantedPlayer,
    EffectController,
    AnyPlayer,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerControlSuffix {
    pub(crate) controller: TriggerControllerReference,
    pub(crate) subject_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerControlPhrase {
    pub(crate) controller: TriggerControllerReference,
    pub(crate) start: usize,
    pub(crate) words: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerSourceSubject {
    AnySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopyReferenceCostReductionShape {
    pub(crate) reduction_tokens: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SimpleCopyReferenceKind {
    It,
    This,
    That,
    ThatCard,
    ExiledCard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenLifecycleSentenceKind {
    ExileCreatedTokenWhenSourceLeaves,
    SacrificeSourceWhenCreatedTokenLeaves,
}

pub(crate) fn parse_trigger_source_subject_words(words: &[&str]) -> Option<TriggerSourceSubject> {
    primitives::parse_full_word_slice(words, parse_trigger_source_subject_word_slice)
}

pub(crate) fn parse_copy_reference_cost_reduction_shape_tokens(
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

pub(crate) fn parse_simple_copy_reference_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SimpleCopyReferenceKind> {
    let words = primitives::TokenWordView::new(tokens).word_refs();
    primitives::parse_full_word_slice(&words, parse_simple_copy_reference_words)
}

pub(crate) fn parse_token_lifecycle_sentence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TokenLifecycleSentenceKind> {
    let words = primitives::TokenWordView::new(tokens).word_refs();
    primitives::parse_full_word_slice(&words, parse_token_lifecycle_sentence_words)
}

pub(crate) fn parse_trigger_word_token(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    parse_word_token_lexed(&mut input, expected).ok()
}

pub(crate) fn parse_trigger_word_span(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<TriggerTokenSpan> {
    let view = primitives::TokenWordView::new(tokens);
    let first = view.token_start_indices().get(word_index).copied()?;
    let end = view.token_index_after_words(word_index + 1)?;
    Some(TriggerTokenSpan { first, end })
}

pub(crate) fn parse_discard_trigger_envelope(
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

pub(crate) fn parse_source_or_another_shape(words: &[&str]) -> Option<SourceOrAnotherShape> {
    let connector = parse_word_slice_index(words, &["and", "or"])?;
    let other_word = connector + 1;
    if !words
        .get(other_word)
        .is_some_and(|word| matches!(*word, "other" | "another"))
    {
        return None;
    }
    Some(SourceOrAnotherShape {
        source_word_end: connector,
        other_word,
    })
}

pub(crate) fn parse_spell_activity_verb_facts(tokens: &[OwnedLexToken]) -> SpellActivityVerbFacts {
    SpellActivityVerbFacts {
        cast: parse_trigger_word_token(tokens, &["cast", "casts"]),
        copy: parse_trigger_word_token(tokens, &["copy", "copies"]),
    }
}

pub(crate) fn parse_possessive_player_reference(words: &[&str]) -> PossessivePlayerReference {
    if normalized_phrase_occurs(words, &["enchanted", "player"])
        || normalized_phrase_occurs(words, &["enchanted", "players"])
    {
        return PossessivePlayerReference::EnchantedPlayer;
    }
    if attached_controller_occurs(words, "enchanted") {
        return PossessivePlayerReference::AttachedController(AttachedControllerSubject::Enchanted);
    }
    if attached_controller_occurs(words, "equipped") {
        return PossessivePlayerReference::AttachedController(AttachedControllerSubject::Equipped);
    }
    if normalized_phrase_occurs(words, &["each", "player"]) {
        return PossessivePlayerReference::Any;
    }
    if exact_phrase_occurs(words, &["your", "team"]) || exact_word_occurs(words, &["your"]) {
        return PossessivePlayerReference::You;
    }
    if exact_word_occurs(words, &["opponent", "opponents"]) {
        return PossessivePlayerReference::Opponent;
    }
    PossessivePlayerReference::Any
}

pub(crate) fn parse_trigger_control_suffix(words: &[&str]) -> Option<TriggerControlSuffix> {
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

pub(crate) fn parse_trigger_control_phrase(words: &[&str]) -> Option<TriggerControlPhrase> {
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
        take_word_slice_any(&mut input).ok()?;
    }
}

pub(crate) fn parse_damage_source_surface(
    tokens: &[OwnedLexToken],
) -> crate::triggers::DamageSourceSurface {
    let words = primitives::TokenWordView::new(tokens).word_refs();
    let subject_end = parse_trigger_control_suffix(&words)
        .map(|suffix| suffix.subject_end)
        .unwrap_or(words.len());
    primitives::parse_full_word_slice(&words[..subject_end], parse_generic_source_noun_words)
        .map(|()| crate::triggers::DamageSourceSurface::Source)
        .unwrap_or(crate::triggers::DamageSourceSurface::Filter)
}

pub(crate) fn parse_spell_or_ability_controller_tail(
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

pub(crate) fn parse_spell_filter_envelope(tokens: &[OwnedLexToken]) -> SpellFilterEnvelope {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    let mut checked_from = false;
    loop {
        let end = initial_len.saturating_sub(input.len());
        let next: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = match next {
            Ok(token) => token,
            Err(_) => return SpellFilterEnvelope { end },
        };
        if token.is_comma() || token.is_period() {
            return SpellFilterEnvelope { end };
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(word, "during" | "other") {
            return SpellFilterEnvelope { end };
        }
        if word == "from" && !checked_from {
            checked_from = true;
            let mut tail = input.clone();
            let next: WResult<&OwnedLexToken> = any.parse_next(&mut tail);
            if next
                .ok()
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|next| next == "anywhere")
            {
                return SpellFilterEnvelope { end };
            }
        }
    }
}

pub(crate) fn parse_clause_before_first_comma(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let trimmed = trim_commas_ref(tokens);
    let mut input = LexStream::new(trimmed);
    let first_comma = parse_comma_offset_lexed(&mut input).ok();
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
        if expected.iter().any(|candidate| word == *candidate) {
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
        let word = take_word_slice_any(&mut input).ok()?;
        if expected.iter().any(|candidate| word == *candidate) {
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

fn take_word_slice_any<'slice, 'word>(input: &mut &'slice [&'word str]) -> WResult<&'word str> {
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

fn exact_phrase_occurs(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    loop {
        let mut candidate = input;
        if parse_exact_phrase(&mut candidate, expected).is_ok() {
            return true;
        }
        if take_word_slice_any(&mut input).is_err() {
            return false;
        }
    }
}

fn exact_word_occurs(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    while let Ok(word) = take_word_slice_any(&mut input) {
        if expected.iter().any(|candidate| word == *candidate) {
            return true;
        }
    }
    false
}

fn attached_controller_occurs(words: &[&str], subject: &str) -> bool {
    const OBJECTS: &[&str] = &[
        "creature",
        "creatures",
        "permanent",
        "permanents",
        "artifact",
        "artifacts",
        "enchantment",
        "enchantments",
        "land",
        "lands",
    ];

    let mut input: primitives::WordSliceInput<'_> = words;
    loop {
        let mut candidate = input;
        if parse_normalized_word(&mut candidate, subject).is_ok()
            && parse_normalized_word_choice(&mut candidate, OBJECTS).is_ok()
            && parse_normalized_word(&mut candidate, "controller").is_ok()
        {
            return true;
        }
        if take_word_slice_any(&mut input).is_err() {
            return false;
        }
    }
}

fn parse_normalized_phrase<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &[&str],
) -> WResult<()> {
    for word in expected {
        parse_normalized_word(input, word)?;
    }
    Ok(())
}

fn parse_exact_phrase<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &[&str],
) -> WResult<()> {
    for expected_word in expected {
        let word = take_word_slice_any(input)?;
        if word != *expected_word {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }
    }
    Ok(())
}

fn parse_normalized_word_choice<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &[&str],
) -> WResult<()> {
    let word = take_word_slice_any(input)?;
    if expected
        .iter()
        .any(|candidate| normalized_word_matches(word, candidate))
    {
        Ok(())
    } else {
        Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ))
    }
}

fn parse_normalized_word<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &str,
) -> WResult<()> {
    let word = take_word_slice_any(input)?;
    if normalized_word_matches(word, expected) {
        Ok(())
    } else {
        Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ))
    }
}

fn normalized_word_matches(word: &str, expected: &str) -> bool {
    let mut input = word;
    let parsed: WResult<()> = (
        literal(expected),
        alt((
            eof.value(()),
            (literal("'s"), eof).void(),
            (literal("’s"), eof).void(),
            (literal("s'"), eof).void(),
            (literal("s’"), eof).void(),
        )),
    )
        .void()
        .parse_next(&mut input);
    parsed.is_ok()
}

fn trim_commas_ref(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[1..];
    }
    while tokens.last().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}
