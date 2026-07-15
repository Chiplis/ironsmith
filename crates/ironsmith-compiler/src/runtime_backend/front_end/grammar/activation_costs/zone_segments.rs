use winnow::combinator::{alt, eof};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;
use crate::effect::Value;
use crate::target::PlayerFilter;
use crate::zone::Zone;

use super::super::super::lexer::{LexStream, OwnedLexToken, render_token_slice};
use super::super::{filters, leaf, primitives};
use super::ActivationCostSegmentCst;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReturnChosenShape {
    count: u32,
    filter_first: usize,
    filter_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReturnCostShape {
    Source,
    Chosen(ReturnChosenShape),
}

pub(crate) fn parse_reveal_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    parse_segment(tokens, parse_reveal_segment_lexed, "reveal-cost")
}

pub(crate) fn parse_return_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let shape = primitives::parse_all(tokens, parse_return_cost_shape_lexed, "return-cost")
        .map_err(|_| unsupported(tokens, "return-cost"))?;
    Ok(match shape {
        ReturnCostShape::Source => ActivationCostSegmentCst::ReturnSelfToHand,
        ReturnCostShape::Chosen(shape) => ActivationCostSegmentCst::ReturnChosenToHand {
            count: shape.count,
            filter: filters::parse_object_filter_with_grammar_entrypoint_lexed(
                &tokens[shape.filter_first..shape.filter_end],
                false,
            )?,
        },
    })
}

/// Parse costs shaped like "Put a card from your hand on top of your library".
/// Returning `None` lets the caller fall back to the ordinary put-counter cost
/// grammar for all other `put` segments.
pub(crate) fn parse_move_to_library_top_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Result<ActivationCostSegmentCst, CardTextError>> {
    if !tokens.first().is_some_and(|token| token.is_word("put")) {
        return None;
    }
    const SUFFIX: [&str; 8] = ["from", "your", "hand", "on", "top", "of", "your", "library"];
    if tokens.len() <= SUFFIX.len() + 1 {
        return None;
    }
    let suffix_start = tokens.len() - SUFFIX.len();
    if !tokens[suffix_start..]
        .iter()
        .zip(SUFFIX)
        .all(|(token, word)| token.is_word(word))
    {
        return None;
    }
    let mut filter_start = 1;
    if tokens
        .get(filter_start)
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        filter_start += 1;
    }
    if filter_start >= suffix_start {
        return Some(Err(unsupported(tokens, "library-top-cost")));
    }
    let parsed = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        &tokens[filter_start..suffix_start],
        false,
    )
    .map(|mut filter| {
        filter.zone = Some(Zone::Hand);
        filter.owner = Some(PlayerFilter::You);
        ActivationCostSegmentCst::MoveChosenToLibraryTop { filter }
    });
    Some(parsed)
}

fn parse_segment<'a>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<
        LexStream<'a>,
        ActivationCostSegmentCst,
        winnow::error::ErrMode<winnow::error::ContextError>,
    >,
    label: &str,
) -> Result<ActivationCostSegmentCst, CardTextError> {
    primitives::parse_all(tokens, parser, label).map_err(|_| unsupported(tokens, label))
}

fn unsupported(tokens: &[OwnedLexToken], label: &str) -> CardTextError {
    CardTextError::ParseError(format!(
        "rewrite {label} parser does not yet support '{}'",
        render_token_slice(tokens).trim().to_ascii_lowercase()
    ))
}

fn parse_reveal_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::kw("reveal").parse_next(input)?;
    alt((parse_reveal_source_from_hand, parse_reveal_cards_from_hand)).parse_next(input)
}

fn parse_reveal_source_from_hand<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationCostSegmentCst> {
    primitives::kw("this").parse_next(input)?;
    let noun = primitives::word_parser_text.parse_next(input)?;
    if noun != "card" && leaf::parse_leaf_card_type_complete(noun).is_err() {
        return Err(primitives::backtrack_err(
            "revealed source",
            "this card or this card type",
        ));
    }
    parse_from_your_hand.parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ActivationCostSegmentCst::RevealSourceFromHand)
}

fn parse_reveal_cards_from_hand<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationCostSegmentCst> {
    let count = parse_optional_reveal_count(input)?;
    let color_filter = parse_optional_color(input);
    let card_type = parse_optional_card_type(input);
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    parse_from_your_hand.parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ActivationCostSegmentCst::RevealFromHand {
        count,
        color_filter,
        card_type,
    })
}

fn parse_optional_reveal_count<'a>(input: &mut LexStream<'a>) -> WResult<Value> {
    let mut dynamic = input.clone();
    if primitives::kw("x").parse_next(&mut dynamic).is_ok() {
        *input = dynamic;
        return Ok(Value::X);
    }
    let mut fixed = input.clone();
    if let Ok(count) = leaf::parse_leaf_number_prefix_lexed.parse_next(&mut fixed) {
        *input = fixed;
        return Ok(Value::Fixed(count as i32));
    }
    Ok(Value::Fixed(1))
}

fn parse_optional_color<'a>(input: &mut LexStream<'a>) -> Option<crate::color::ColorSet> {
    let mut probe = input.clone();
    let word = primitives::word_parser_text.parse_next(&mut probe).ok()?;
    let color = leaf::parse_leaf_color_complete(word).ok()?;
    *input = probe;
    Some(color)
}

fn parse_optional_card_type<'a>(input: &mut LexStream<'a>) -> Option<crate::types::CardType> {
    let mut probe = input.clone();
    let word = primitives::word_parser_text.parse_next(&mut probe).ok()?;
    let card_type = leaf::parse_leaf_card_type_complete(word).ok()?;
    *input = probe;
    Some(card_type)
}

fn parse_from_your_hand<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["from", "your", "hand"])
        .void()
        .parse_next(input)
}

fn parse_return_cost_shape_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ReturnCostShape> {
    let initial_len = input.len();
    primitives::kw("return").parse_next(input)?;

    let mut source = input.clone();
    if parse_return_source_reference
        .parse_next(&mut source)
        .is_ok()
        && parse_owner_hand_suffix.parse_next(&mut source).is_ok()
        && source.peek_token().is_none()
    {
        *input = source;
        return Ok(ReturnCostShape::Source);
    }

    let count = parse_optional_fixed_count(input);
    parse_leading_articles(input);
    let filter_first = initial_len.saturating_sub(input.len());
    let mut filter_end = filter_first;
    loop {
        let mut suffix = input.clone();
        if parse_owner_hand_suffix.parse_next(&mut suffix).is_ok() && suffix.peek_token().is_none()
        {
            if filter_end == filter_first {
                return Err(primitives::backtrack_err(
                    "return filter",
                    "object before owner-hand suffix",
                ));
            }
            *input = suffix;
            return Ok(ReturnCostShape::Chosen(ReturnChosenShape {
                count,
                filter_first,
                filter_end,
            }));
        }
        any.parse_next(input)?;
        filter_end += 1;
    }
}

fn parse_optional_fixed_count<'a>(input: &mut LexStream<'a>) -> u32 {
    let mut probe = input.clone();
    if let Ok(count) = leaf::parse_leaf_number_prefix_lexed.parse_next(&mut probe) {
        *input = probe;
        count
    } else {
        1
    }
}

fn parse_leading_articles<'a>(input: &mut LexStream<'a>) {
    loop {
        let mut probe = input.clone();
        if alt((
            primitives::kw("a"),
            primitives::kw("an"),
            primitives::kw("the"),
        ))
        .parse_next(&mut probe)
        .is_err()
        {
            break;
        }
        *input = probe;
    }
}

fn parse_return_source_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    if primitives::kw("it").parse_next(input).is_ok() {
        return Ok(());
    }
    primitives::kw("this").parse_next(input)?;
    let mut noun = input.clone();
    if alt((
        primitives::kw("card"),
        primitives::kw("permanent"),
        primitives::kw("creature"),
        primitives::kw("artifact"),
        primitives::kw("enchantment"),
        primitives::kw("land"),
    ))
    .parse_next(&mut noun)
    .is_ok()
    {
        *input = noun;
    }
    Ok(())
}

fn parse_owner_hand_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["to", "its", "owners", "hand"]),
        primitives::phrase(&["to", "its", "owner's", "hand"]),
        primitives::phrase(&["to", "its", "owners'", "hand"]),
        primitives::phrase(&["to", "their", "owners", "hand"]),
        primitives::phrase(&["to", "their", "owner's", "hand"]),
        primitives::phrase(&["to", "their", "owners'", "hand"]),
    ))
    .void()
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn reveal_and_return_segments_are_typed() {
        let reveal = lex_line("reveal x red creature cards from your hand", 0).unwrap();
        assert_eq!(
            parse_reveal_segment_tokens(&reveal).unwrap(),
            ActivationCostSegmentCst::RevealFromHand {
                count: Value::X,
                color_filter: Some(crate::color::ColorSet::RED),
                card_type: Some(crate::types::CardType::Creature),
            }
        );

        let source = lex_line("return this creature to its owner's hand", 0).unwrap();
        assert_eq!(
            parse_return_segment_tokens(&source).unwrap(),
            ActivationCostSegmentCst::ReturnSelfToHand
        );
        let chosen = lex_line("return two artifacts to their owners' hand", 0).unwrap();
        assert_eq!(
            parse_return_segment_tokens(&chosen).unwrap(),
            ActivationCostSegmentCst::ReturnChosenToHand {
                count: 2,
                filter: crate::target::ObjectFilter::artifact(),
            }
        );
    }
}
