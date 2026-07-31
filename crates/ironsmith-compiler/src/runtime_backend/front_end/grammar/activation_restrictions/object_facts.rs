use std::ops::Range;

use winnow::combinator::{alt, opt, peek};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::leaf::{
    parse_leaf_choice_count_prefix_lexed, parse_leaf_target_count_range_prefix_lexed,
};
use super::super::primitives;
use super::clause_facts::{exact, exact_any, prefix};
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken, TokenWordView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivatedAbilityOwnerScope {
    All,
    TapCostOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivatedAbilityOwnerShape {
    pub(crate) owner_tokens: Range<usize>,
    pub(crate) scope: ActivatedAbilityOwnerScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ItOwnerReference;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PossessiveActivatedAbilitySubject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TargetIndicatorShape {
    pub(crate) consumed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetRestrictionEnvelope {
    FilteredSources {
        spell_descriptor_tokens: Option<Range<usize>>,
        source_descriptor_tokens: Range<usize>,
    },
    SourceSpell {
        full_source_tokens: Range<usize>,
        descriptor_tokens: Range<usize>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NegatedObjectTailShape {
    AttackYou,
    AttackYouOrPlaneswalkers,
    BeBlockedExceptBy { payload_words: usize },
    BeBlockedBy { payload_words: usize },
    BeActivated,
    BeActivatedUnlessManaAbilities,
    Block { payload_words: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AndOrSeparatorFacts {
    pub(crate) separators: Vec<Range<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BePreventedTail;

pub(crate) fn parse_activated_ability_owner_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedAbilityOwnerShape> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let (owner_words, scope, prefix_owner) = if super::clause_facts::suffix(
        &words,
        &[
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
        ],
    ) {
        (
            words.len().checked_sub(7)?,
            ActivatedAbilityOwnerScope::TapCostOnly,
            false,
        )
    } else if super::clause_facts::suffix(&words, &["activated", "abilities"]) {
        (
            words.len().checked_sub(2)?,
            ActivatedAbilityOwnerScope::All,
            false,
        )
    } else if prefix(
        &words,
        &[
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
            "of",
        ],
    ) {
        (8, ActivatedAbilityOwnerScope::TapCostOnly, true)
    } else if prefix(&words, &["activated", "abilities", "of"]) {
        (3, ActivatedAbilityOwnerScope::All, true)
    } else {
        return None;
    };

    let owner_tokens = if prefix_owner {
        let start = view.token_start_indices().get(owner_words).copied()?;
        start..tokens.len()
    } else {
        if owner_words == 0 {
            return None;
        }
        let end = view.token_index_after_words(owner_words)?;
        0..end
    };
    Some(ActivatedAbilityOwnerShape {
        owner_tokens,
        scope,
    })
}

pub(crate) fn parse_it_owner_reference_words(words: &[&str]) -> Option<ItOwnerReference> {
    exact_any(words, &[&["it"], &["its"], &["them"], &["their"]]).then_some(ItOwnerReference)
}

pub(crate) fn parse_possessive_activated_ability_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PossessiveActivatedAbilitySubject> {
    let words = TokenWordView::new(tokens).word_refs();
    (prefix(&words, &["its", "activated", "abilities"])
        || prefix(&words, &["their", "activated", "abilities"]))
    .then_some(PossessiveActivatedAbilitySubject)
}

pub(crate) fn parse_target_indicator_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TargetIndicatorShape> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    parse_target_indicator_lexed.parse_next(&mut input).ok()?;
    Some(TargetIndicatorShape {
        consumed: initial_len.saturating_sub(input.len()),
    })
}

fn parse_target_indicator_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(primitives::phrase(&["any", "number", "of"])).parse_next(input)?;
    let mut counted = input.clone();
    if alt((
        parse_leaf_target_count_range_prefix_lexed.void(),
        parse_leaf_choice_count_prefix_lexed.void(),
    ))
    .parse_next(&mut counted)
    .is_ok()
    {
        let mut target_probe = counted.clone();
        let _ = opt(alt((primitives::kw("another"), primitives::kw("other"))))
            .parse_next(&mut target_probe);
        if peek(primitives::kw("target"))
            .parse_next(&mut target_probe)
            .is_ok()
        {
            *input = counted;
        }
    }
    opt(primitives::kw("on")).parse_next(input)?;
    opt(alt((primitives::kw("another"), primitives::kw("other")))).parse_next(input)?;
    primitives::kw("target").void().parse_next(input)
}

pub(crate) fn parse_target_restriction_envelope_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TargetRestrictionEnvelope> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if words.len() < 6 || !prefix(&words, &["be", "the", "target", "of"]) {
        return None;
    }

    if let Some(marker) = primitives::parse_word_sequence_span(
        words.get(4..).unwrap_or_default(),
        &["spells", "or", "abilities", "from"],
    )
    .map(|span| span.start + 4)
    {
        let source_first = marker + 4;
        let source_end = if matches!(words.last().copied(), Some("source" | "sources")) {
            words.len().saturating_sub(1)
        } else {
            words.len()
        };
        if source_first >= source_end {
            return None;
        }
        return Some(TargetRestrictionEnvelope::FilteredSources {
            spell_descriptor_tokens: (marker > 4)
                .then(|| token_range_for_words(tokens, &view, 4..marker))
                .flatten(),
            source_descriptor_tokens: token_range_for_words(
                tokens,
                &view,
                source_first..source_end,
            )?,
        });
    }

    if !matches!(words.last().copied(), Some("spell" | "spells")) {
        return None;
    }
    let full_source_tokens = token_range_for_words(tokens, &view, 4..words.len())?;
    let descriptor_tokens = token_range_for_words(tokens, &view, 4..words.len() - 1)?;
    Some(TargetRestrictionEnvelope::SourceSpell {
        full_source_tokens,
        descriptor_tokens,
    })
}

pub(crate) fn parse_negated_object_tail_words(words: &[&str]) -> Option<NegatedObjectTailShape> {
    if exact(words, &["attack", "you"]) {
        // "can't attack you" leaves the player's planeswalkers attackable —
        // a distinct, narrower restriction than the Ghostly Prison shape.
        Some(NegatedObjectTailShape::AttackYou)
    } else if exact(
        words,
        &["attack", "you", "or", "planeswalkers", "you", "control"],
    ) {
        Some(NegatedObjectTailShape::AttackYouOrPlaneswalkers)
    } else if prefix(words, &["be", "blocked", "this", "turn", "except", "by"]) {
        Some(NegatedObjectTailShape::BeBlockedExceptBy { payload_words: 6 })
    } else if prefix(words, &["be", "blocked", "except", "by"]) {
        Some(NegatedObjectTailShape::BeBlockedExceptBy { payload_words: 4 })
    } else if prefix(words, &["be", "blocked", "by"]) {
        Some(NegatedObjectTailShape::BeBlockedBy { payload_words: 3 })
    } else if exact_any(
        words,
        &[&["be", "activated"], &["be", "activated", "this", "turn"]],
    ) {
        Some(NegatedObjectTailShape::BeActivated)
    } else if exact(
        words,
        &["be", "activated", "unless", "theyre", "mana", "abilities"],
    ) {
        Some(NegatedObjectTailShape::BeActivatedUnlessManaAbilities)
    } else if prefix(words, &["block"]) && words.len() > 1 {
        Some(NegatedObjectTailShape::Block { payload_words: 1 })
    } else {
        None
    }
}

pub(crate) fn parse_and_or_separator_facts_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AndOrSeparatorFacts> {
    let mut separators = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        let tail = &tokens[index..];
        let consumed = if primitives::parse_prefix(tail, primitives::kw("and/or")).is_some() {
            1
        } else if primitives::parse_prefix(tail, primitives::phrase(&["and", "or"])).is_some() {
            2
        } else {
            index += 1;
            continue;
        };
        separators.push(index..index + consumed);
        index += consumed;
    }
    (!separators.is_empty()).then_some(AndOrSeparatorFacts { separators })
}

pub(crate) fn parse_be_prevented_tail_words(words: &[&str]) -> Option<BePreventedTail> {
    exact(words, &["be", "prevented"]).then_some(BePreventedTail)
}

fn token_range_for_words(
    tokens: &[OwnedLexToken],
    view: &TokenWordView<'_>,
    words: Range<usize>,
) -> Option<Range<usize>> {
    let first = view.token_start_indices().get(words.start).copied()?;
    let end = view
        .token_index_after_words(words.end)
        .unwrap_or(tokens.len());
    (first <= end).then_some(first..end)
}
