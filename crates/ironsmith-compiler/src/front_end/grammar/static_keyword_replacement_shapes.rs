use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::{leaf, primitives};
use crate::types::CardType;
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TokenSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LibraryBottomOrderShape {
    Chosen,
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DrawRevealMatchingRestBottomShape<'a> {
    pub(crate) count: u32,
    pub(crate) card_type_word: &'a str,
    pub(crate) order: LibraryBottomOrderShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiscardOrRedirectReplacementShape {
    pub(crate) discard_type: CardType,
    pub(crate) redirect_zone: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SacrificeOrRedirectReplacementShape<'a> {
    pub(crate) count: u32,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) redirect_zone: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DrawRevealMatchingRestBottomWordShape {
    count: u32,
    card_type_word: usize,
    order: LibraryBottomOrderShape,
}

pub(crate) fn parse_draw_reveal_matching_rest_bottom(
    tokens: &[OwnedLexToken],
) -> Option<DrawRevealMatchingRestBottomShape<'_>> {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    let parsed = parse_draw_reveal_matching_rest_bottom_words(&mut input).ok()?;
    if !input.is_empty() {
        return None;
    }
    Some(DrawRevealMatchingRestBottomShape {
        count: parsed.count,
        card_type_word: words.get(parsed.card_type_word)?,
        order: parsed.order,
    })
}

pub(crate) fn parse_discard_or_redirect_replacement(
    tokens: &[OwnedLexToken],
) -> Option<DiscardOrRedirectReplacementShape> {
    primitives::parse_all(
        tokens,
        discard_or_redirect_replacement,
        "discard-or-redirect replacement",
    )
    .ok()
}

pub(crate) fn parse_sacrifice_or_redirect_replacement(
    tokens: &[OwnedLexToken],
) -> Option<SacrificeOrRedirectReplacementShape<'_>> {
    let (_, after_sacrifice) = primitives::parse_prefix(tokens, sacrifice_replacement_intro)?;
    let (instead_idx, _, after_instead) =
        primitives::find_prefix(after_sacrifice, || primitives::kw("instead"))?;
    let payment_tokens = trim_lexed_commas(&after_sacrifice[..instead_idx]);
    let number = leaf::parse_leaf_number_prefix_tokens(payment_tokens)?;
    let (count, consumed) = number.into_fixed()?;
    let filter_tokens = trim_lexed_commas(payment_tokens.get(consumed..)?);
    if count == 0 || filter_tokens.is_empty() {
        return None;
    }
    primitives::parse_all(
        after_instead,
        sacrifice_replacement_result_sentences,
        "sacrifice-or-redirect replacement result",
    )
    .ok()?;
    Some(SacrificeOrRedirectReplacementShape {
        count,
        filter_tokens,
        redirect_zone: Zone::Graveyard,
    })
}

fn enter_battlefield_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["would", "enter"]).parse_next(input)?;
    opt(alt((
        primitives::phrase(&["the", "battlefield"]),
        primitives::kw("battlefield").void(),
    )))
    .parse_next(input)?;
    Ok(())
}

fn sacrifice_replacement_intro<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("if").parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(enter_battlefield_marker))
        .void()
        .parse_next(input)?;
    enter_battlefield_marker(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("sacrifice").parse_next(input)?;
    Ok(())
}

fn sacrifice_replacement_result_sentences<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::end_of_sentence().parse_next(input)?;
    discard_success_sentence(input)?;
    discard_failure_sentence(input)
}

fn discard_success_sentence<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "you", "do"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("put").parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["onto", "the", "battlefield"])),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["onto", "the", "battlefield"]).parse_next(input)?;
    primitives::end_of_sentence().parse_next(input)?;
    Ok(())
}

fn discard_failure_sentence<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "you"]).parse_next(input)?;
    alt((primitives::kw("don't"), primitives::kw("dont"))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["put", "it", "into", "its"]).parse_next(input)?;
    alt((
        primitives::kw("owner"),
        primitives::kw("owners"),
        primitives::kw("owner's"),
        primitives::kw("owners'"),
        primitives::kw("owner’s"),
    ))
    .parse_next(input)?;
    primitives::kw("graveyard").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(())
}

fn discard_or_redirect_replacement<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DiscardOrRedirectReplacementShape> {
    primitives::kw("if").parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(enter_battlefield_marker))
        .void()
        .parse_next(input)?;
    enter_battlefield_marker(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["you", "may", "discard", "a", "land", "card", "instead"])
        .parse_next(input)?;
    primitives::end_of_sentence().parse_next(input)?;
    opt(discard_success_sentence).parse_next(input)?;
    discard_failure_sentence(input)?;
    Ok(DiscardOrRedirectReplacementShape {
        discard_type: CardType::Land,
        redirect_zone: Zone::Graveyard,
    })
}

pub(crate) fn parse_combat_prevention_prefix(tokens: &[OwnedLexToken]) -> Option<TokenSpan> {
    phrase_span(
        tokens,
        &[
            "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to",
        ],
    )
}

pub(crate) fn parse_noncombat_prevention_prefix(tokens: &[OwnedLexToken]) -> Option<TokenSpan> {
    phrase_span(
        tokens,
        &[
            "prevent",
            "all",
            "noncombat",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
        ],
    )
}

fn parse_draw_reveal_matching_rest_bottom_words<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> WResult<DrawRevealMatchingRestBottomWordShape> {
    let initial_len = input.len();
    parse_word_phrase(
        input,
        &[
            "if", "you", "would", "draw", "a", "card", "instead", "reveal", "the", "top",
        ],
    )?;
    let (count, consumed) = leaf::parse_leaf_number_prefix_words(input)
        .and_then(leaf::LeafNumberPrefix::into_fixed)
        .filter(|(count, _)| *count > 0)
        .ok_or_else(|| primitives::backtrack_err("reveal count", "positive fixed number"))?;
    *input = input
        .get(consumed..)
        .ok_or_else(|| primitives::backtrack_err("reveal count", "remaining words"))?;
    alt((
        primitives::word_slice_exact("card"),
        primitives::word_slice_exact("cards"),
    ))
    .parse_next(input)?;
    parse_word_phrase(input, &["of", "your", "library", "put", "all"])?;
    let card_type_word = initial_len.saturating_sub(input.len());
    let _: &str = any.parse_next(input)?;
    parse_word_phrase(
        input,
        &[
            "cards", "revealed", "this", "way", "into", "your", "hand", "and", "the", "rest", "on",
            "the", "bottom", "of", "your", "library", "in",
        ],
    )?;
    let order = alt((
        parse_chosen_order.value(LibraryBottomOrderShape::Chosen),
        parse_random_order.value(LibraryBottomOrderShape::Random),
    ))
    .parse_next(input)?;
    Ok(DrawRevealMatchingRestBottomWordShape {
        count,
        card_type_word,
        order,
    })
}

fn parse_random_order(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    alt((
        |input: &mut primitives::WordSliceInput<'_>| {
            parse_word_phrase(input, &["a", "random", "order"])
        },
        |input: &mut primitives::WordSliceInput<'_>| parse_word_phrase(input, &["random", "order"]),
    ))
    .parse_next(input)
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

fn parse_chosen_order(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    parse_word_phrase(input, &["any", "order"])
}

fn phrase_span(tokens: &[OwnedLexToken], expected: &'static [&'static str]) -> Option<TokenSpan> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let start = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if primitives::phrase(expected)
            .parse_next(&mut candidate)
            .is_ok()
        {
            return Some(TokenSpan {
                start,
                end: initial_len.saturating_sub(candidate.len()),
            });
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        parsed.ok()?;
    }
}

#[cfg(test)]
#[path = "static_keyword_replacement_shapes/tests.rs"]
mod tests;
