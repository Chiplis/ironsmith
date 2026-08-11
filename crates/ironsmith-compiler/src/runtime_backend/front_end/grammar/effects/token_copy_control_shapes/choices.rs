use super::super::*;

use crate::runtime_backend::front_end::grammar::leaf;
use winnow::combinator::{alt, eof, opt, repeat_till};
use winnow::error::{ContextError, ErrMode};
use winnow::token::any;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SacrificeChoiceShape<'a> {
    pub(crate) count: ChoiceCount,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) tail_tokens: Option<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExileSourceCounterShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) descriptor_tokens: &'a [OwnedLexToken],
    pub(crate) source_reference: bool,
    pub(crate) it_reference: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DestroyAttachedShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChoicePhraseSpan {
    pub(crate) start: usize,
    pub(crate) len: usize,
}

fn split_then(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], Option<&[OwnedLexToken]>) {
    primitives::split_lexed_once_on_separator(tokens, || primitives::kw("then").void())
        .map(|(head, tail)| {
            (
                trim_lexed_commas(head),
                Some(trim_lexed_commas(tail)).filter(|tail| !tail.is_empty()),
            )
        })
        .unwrap_or((trim_lexed_commas(tokens), None))
}

pub(crate) fn parse_sacrifice_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<SacrificeChoiceShape<'_>> {
    let (_, body) = primitives::parse_prefix(tokens, primitives::kw("sacrifice"))?;
    let choice = leaf::parse_leaf_choice_count_prefix_tokens(body)?;
    if choice.count != ChoiceCount::any_number() && choice.count != ChoiceCount::at_least(1) {
        return None;
    }
    let (filter_tokens, tail_tokens) = split_then(body.get(choice.consumed..)?);
    if filter_tokens.is_empty()
        || (choice.count != ChoiceCount::any_number() && tail_tokens.is_some())
    {
        return None;
    }
    Some(SacrificeChoiceShape {
        count: choice.count,
        filter_tokens,
        tail_tokens,
    })
}

fn marker_anywhere<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parser)
        .parse_next(&mut input)
        .is_ok()
}

fn exact_source_reference(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            primitives::any_phrase(&[
                &["this"],
                &["this", "card"],
                &["this", "creature"],
                &["this", "permanent"],
                &["this", "artifact"],
                &["this", "enchantment"],
                &["this", "land"],
            ]),
            primitives::sentence_end(),
        )
            .void(),
        "exile source reference",
    )
    .is_ok()
}

/// A bare "it" subject is a contextual back-reference, not a source
/// reference: in "Whenever an opponent discards a card, exile it with a
/// stash counter on it" the pronoun names the discarded card. Reference
/// resolution still lands on the source when no antecedent exists (a
/// dies-trigger's "exile it" resolves to the triggering source).
fn exact_it_reference(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            primitives::kw("it"),
            opt(primitives::any_phrase(&[
                &["from", "their", "graveyard"],
                &["from", "that", "player's", "graveyard"],
                &["from", "that", "players", "graveyard"],
            ])),
            primitives::sentence_end(),
        )
            .void(),
        "exile it reference",
    )
    .is_ok()
}

fn plausible_named_source(tokens: &[OwnedLexToken]) -> bool {
    !tokens.is_empty()
        && !marker_anywhere(
            tokens,
            primitives::any_phrase(&[
                &["then"],
                &["if"],
                &["unless"],
                &["where"],
                &["when"],
                &["whenever"],
                &["for"],
                &["each"],
                &["search"],
                &["destroy"],
                &["exile"],
                &["draw"],
                &["gain"],
                &["lose"],
                &["counter"],
                &["put"],
                &["return"],
                &["create"],
                &["sacrifice"],
                &["deal"],
                &["populate"],
                &["target"],
                &["card"],
                &["cards"],
                &["creature"],
                &["creatures"],
                &["permanent"],
                &["permanents"],
                &["artifact"],
                &["artifacts"],
                &["enchantment"],
                &["enchantments"],
                &["land"],
                &["lands"],
                &["planeswalker"],
                &["planeswalkers"],
                &["spell"],
                &["spells"],
            ]),
        )
}

fn source_surface_supported(tokens: &[OwnedLexToken]) -> bool {
    exact_source_reference(tokens) || plausible_named_source(tokens)
}

pub(crate) fn parse_exile_source_counter_shape(
    tokens: &[OwnedLexToken],
) -> Option<ExileSourceCounterShape<'_>> {
    let (_, body) = primitives::parse_prefix(tokens, primitives::kw("exile"))?;
    let (source_tokens, counter_tokens) =
        primitives::split_lexed_once_on_separator(body, || primitives::kw("with").void())?;
    let target_tokens = trim_lexed_commas(source_tokens);
    if target_tokens.is_empty() {
        return None;
    }
    let (descriptor_tokens, _) =
        primitives::split_lexed_once_before_suffix(trim_lexed_commas(counter_tokens), 1, || {
            (
                primitives::kw("on"),
                alt((primitives::kw("it"), primitives::kw("them"))),
                primitives::sentence_end(),
            )
                .void()
        })?;
    let descriptor_tokens = trim_lexed_commas(descriptor_tokens);
    let it_reference = exact_it_reference(target_tokens);
    (!descriptor_tokens.is_empty()).then_some(ExileSourceCounterShape {
        target_tokens,
        descriptor_tokens,
        source_reference: !it_reference && source_surface_supported(target_tokens),
        it_reference,
    })
}

fn trailing_filter_copula(token: &OwnedLexToken) -> bool {
    primitives::parse_all(
        std::slice::from_ref(token),
        (
            alt((
                primitives::kw("that"),
                primitives::kw("were"),
                primitives::kw("was"),
                primitives::kw("is"),
                primitives::kw("are"),
            )),
            eof,
        )
            .void(),
        "attached filter copula",
    )
    .is_ok()
}

fn trim_filter_copula(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut end = tokens.len();
    while end > 0 && trailing_filter_copula(&tokens[end - 1]) {
        end -= 1;
    }
    trim_lexed_commas(&tokens[..end])
}

fn target_surface_supported(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::kw("target")).is_some()
        || primitives::parse_all(
            tokens,
            (
                alt((primitives::kw("you"), primitives::kw("it"))),
                primitives::sentence_end(),
            )
                .void(),
            "attached exact target",
        )
        .is_ok()
        || primitives::parse_prefix(
            tokens,
            primitives::any_phrase(&[
                &["that", "creature"],
                &["that", "permanent"],
                &["that", "land"],
                &["that", "artifact"],
                &["that", "enchantment"],
            ]),
        )
        .is_some()
}

fn has_timing_surface(tokens: &[OwnedLexToken]) -> bool {
    marker_anywhere(
        tokens,
        primitives::any_phrase(&[
            &["at"],
            &["beginning"],
            &["end"],
            &["combat"],
            &["turn"],
            &["step"],
            &["until"],
        ]),
    )
}

pub(crate) fn parse_destroy_attached_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestroyAttachedShape<'_>> {
    let (_, body) = primitives::parse_prefix(
        tokens,
        (
            primitives::kw("destroy"),
            alt((primitives::kw("all"), primitives::kw("each"))),
        )
            .void(),
    )?;
    let (filter_tokens, target_tokens) = primitives::split_lexed_once_on_separator(body, || {
        primitives::phrase(&["attached", "to"]).void()
    })?;
    let filter_tokens = trim_filter_copula(filter_tokens);
    let target_tokens = trim_lexed_commas(target_tokens);
    if filter_tokens.is_empty()
        || target_tokens.is_empty()
        || !target_surface_supported(target_tokens)
        || has_timing_surface(target_tokens)
    {
        return None;
    }
    Some(DestroyAttachedShape {
        filter_tokens,
        target_tokens,
    })
}

pub(crate) fn parse_color_choice_phrase_span(tokens: &[OwnedLexToken]) -> Option<ChoicePhraseSpan> {
    let phrases: &'static [&'static [&'static str]] = &[
        &["of", "the", "color", "of", "your", "choice"],
        &["of", "the", "color", "of", "their", "choice"],
        &["of", "color", "of", "your", "choice"],
        &["of", "color", "of", "their", "choice"],
    ];
    let mut best: Option<ChoicePhraseSpan> = None;
    for &phrase in phrases {
        let Some((start, _, rest)) = primitives::find_prefix(tokens, || primitives::phrase(phrase))
        else {
            continue;
        };
        let candidate = ChoicePhraseSpan {
            start,
            len: tokens.len().saturating_sub(start + rest.len()),
        };
        if best.is_none_or(|current| candidate.start < current.start) {
            best = Some(candidate);
        }
    }
    best
}

#[cfg(test)]
#[path = "choices/tests.rs"]
mod tests;
