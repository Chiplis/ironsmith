use super::super::*;

use winnow::combinator::{alt, opt, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

#[derive(Debug, Clone, PartialEq)]
pub struct EachPlayerRevealPermanentsShape {
    pub count: Value,
    pub matching_filter: ObjectFilter,
    pub matching_enters_tapped: bool,
    pub remainder_zone: Zone,
}

fn puts_revealed_permanents<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["puts", "all", "permanent", "cards"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["revealed", "this", "way"]),
    )
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["onto", "the", "battlefield"]),
    )
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(())
}

fn strip_leading_connectors(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(
        tokens,
        repeat::<_, _, (), _, _>(
            0..,
            alt((primitives::kw("then"), primitives::kw("and"))).void(),
        ),
    )
    .map(|(_, rest)| trim_lexed_commas(rest))
    .unwrap_or(tokens)
}

pub fn parse_each_player_reveal_permanents_shape(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerRevealPermanentsShape> {
    let segments = primitives::split_lexed_slices_on_comma(tokens);
    let [reveal_tokens, put_tokens, rest_tokens] = segments.as_slice() else {
        return None;
    };
    let reveal_tokens = trim_lexed_commas(reveal_tokens);
    let dynamic_count = primitives::parse_prefix(
        reveal_tokens,
        primitives::phrase(&[
            "each", "player", "reveals", "a", "number", "of", "cards", "from", "the", "top", "of",
            "their", "library", "equal", "to",
        ]),
    )
    .and_then(|(_, count_tokens)| {
        let (_, count_filter_tokens) = primitives::parse_prefix(
            trim_lexed_commas(count_tokens),
            alt((
                primitives::phrase(&["the", "number", "of"]),
                primitives::phrase(&["number", "of"]),
            )),
        )?;
        let count_filter = crate::grammar::primitives::probe_shape(parse_object_filter_lexed(
            count_filter_tokens,
            false,
        ))?;
        Some(Value::Count(count_filter))
    });
    let fixed_count = || {
        let (_, count_tokens) = primitives::parse_prefix(
            reveal_tokens,
            primitives::phrase(&["each", "player", "reveals", "the", "top"]),
        )?;
        let count_tokens = trim_lexed_commas(count_tokens);
        let (count, used) =
            crate::grammar::shared_util::value_semantics::parse_value_prefix_lexed(count_tokens)?;
        crate::grammar::primitives::probe_all(
            &count_tokens[used..],
            (
                primitives::phrase(&["cards", "of", "their", "library"]),
                primitives::sentence_end(),
            )
                .void(),
            "each-player fixed reveal count",
        )?;
        Some(count)
    };
    let count = dynamic_count.or_else(fixed_count)?;

    let dynamic_permanents = primitives::parse_all(
        trim_lexed_commas(put_tokens),
        puts_revealed_permanents,
        "put revealed permanents",
    )
    .is_ok();
    let fixed_partition = || {
        let (_, after_all) = primitives::parse_prefix(
            trim_lexed_commas(put_tokens),
            primitives::phrase(&["puts", "all"]),
        )?;
        let (filter_tokens, battlefield_tokens) =
            primitives::split_lexed_once_on_separator(after_all, || {
                primitives::phrase(&["revealed", "this", "way"]).void()
            })?;
        let matching_filter = crate::grammar::primitives::probe_shape(parse_object_filter_lexed(
            trim_lexed_commas(filter_tokens),
            false,
        ))?;
        let matching_enters_tapped = if primitives::parse_all(
            battlefield_tokens,
            (
                primitives::phrase(&["onto", "the", "battlefield", "tapped"]),
                primitives::sentence_end(),
            )
                .void(),
            "revealed permanent enters tapped",
        )
        .is_ok()
        {
            true
        } else if primitives::parse_all(
            battlefield_tokens,
            (
                primitives::phrase(&["onto", "the", "battlefield"]),
                primitives::sentence_end(),
            )
                .void(),
            "revealed permanent enters",
        )
        .is_ok()
        {
            false
        } else {
            return None;
        };
        Some((matching_filter, matching_enters_tapped))
    };
    let (matching_filter, matching_enters_tapped) = if dynamic_permanents {
        (ObjectFilter::permanent_card(), false)
    } else {
        fixed_partition()?
    };

    let rest_tokens = strip_leading_connectors(rest_tokens);
    let remainder_zone = if primitives::parse_all(
        rest_tokens,
        (
            primitives::phrase(&["puts", "the", "rest", "into", "their", "graveyard"]),
            primitives::sentence_end(),
        )
            .void(),
        "put reveal rest into graveyard",
    )
    .is_ok()
    {
        Zone::Graveyard
    } else if primitives::parse_all(
        rest_tokens,
        (
            primitives::phrase(&["exiles", "the", "rest"]),
            primitives::sentence_end(),
        )
            .void(),
        "exile reveal rest",
    )
    .is_ok()
    {
        Zone::Exile
    } else {
        return None;
    };

    Some(EachPlayerRevealPermanentsShape {
        count,
        matching_filter,
        matching_enters_tapped,
        remainder_zone,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_each_player_reveal_surface() {
        let tokens = lex_line(
            "Each player reveals a number of cards from the top of their library equal to the number of nonland permanents they control, puts all permanent cards they revealed this way onto the battlefield, and puts the rest into their graveyard.",
            0,
        )
        .unwrap();
        let shape = parse_each_player_reveal_permanents_shape(&tokens).unwrap();
        assert_eq!(shape.matching_filter, ObjectFilter::permanent_card());
        assert_eq!(shape.remainder_zone, Zone::Graveyard);
    }

    #[test]
    fn parses_fixed_reveal_land_partition_with_tapped_entries_and_exile_remainder() {
        let tokens = lex_line(
            "Each player reveals the top five cards of their library, puts all land cards revealed this way onto the battlefield tapped, and exiles the rest.",
            0,
        )
        .unwrap();
        let shape = parse_each_player_reveal_permanents_shape(&tokens).unwrap();

        assert_eq!(shape.count, Value::Fixed(5));
        assert_eq!(shape.matching_filter.card_types, [CardType::Land]);
        assert!(shape.matching_enters_tapped);
        assert_eq!(shape.remainder_zone, Zone::Exile);

        let near_miss = lex_line(
            "Each player reveals the top five cards of their library, puts all land cards revealed this way into their hand, and exiles the rest.",
            0,
        )
        .unwrap();
        assert!(parse_each_player_reveal_permanents_shape(&near_miss).is_none());
    }

    #[test]
    fn parses_comma_then_special_surfaces() {
        let tokens = lex_line(
            "tap target creature, then return this artifact to its owners hand",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_comma_then_special_shape(&tokens).map(|shape| shape.tail),
            Some(CommaThenTailShape::ReturnSourceToHand)
        ));
    }
}
