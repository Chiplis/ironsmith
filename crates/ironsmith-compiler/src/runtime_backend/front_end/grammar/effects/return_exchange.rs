use super::*;
use winnow::combinator::{alt, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

#[path = "return_exchange/return_shapes.rs"]
mod return_shapes;
pub(crate) use return_shapes::*;
#[path = "return_exchange/exchange_shapes.rs"]
mod exchange_shapes;
pub(crate) use exchange_shapes::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CycledOrDiscardedThisTurnFilterTail {
    pub(crate) base_tokens: Vec<OwnedLexToken>,
    pub(crate) player_filter: PlayerFilter,
}

pub(crate) fn parse_cycled_or_discarded_this_turn_filter_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<CycledOrDiscardedThisTurnFilterTail>, CardTextError> {
    primitives::parse_all_or_none(
        tokens,
        parse_cycled_or_discarded_this_turn_filter_tail_lexed,
        "cycled-or-discarded-this-turn-filter-tail",
    )
}

fn parse_cycled_or_discarded_this_turn_filter_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> Result<CycledOrDiscardedThisTurnFilterTail, ErrMode<ContextError>> {
    let (base_tokens, player_filter): (Vec<OwnedLexToken>, PlayerFilter) = repeat_till(
        0..,
        any.map(|token: &OwnedLexToken| token.clone()),
        parse_cycled_or_discarded_this_turn_suffix_at_end_lexed,
    )
    .parse_next(input)?;

    Ok(CycledOrDiscardedThisTurnFilterTail {
        base_tokens: trim_lexed_commas(&base_tokens).to_vec(),
        player_filter,
    })
}

fn parse_cycled_or_discarded_this_turn_suffix_at_end_lexed<'a>(
    input: &mut LexStream<'a>,
) -> Result<PlayerFilter, ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["that", "you", "cycled", "or", "discarded", "this", "turn"]),
        primitives::phrase(&["you", "cycled", "or", "discarded", "this", "turn"]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PlayerFilter::You)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_tail(text: &str) -> Option<CycledOrDiscardedThisTurnFilterTail> {
        let tokens = lex_line(text, 0).expect("filter tail should lex");
        parse_cycled_or_discarded_this_turn_filter_tail_tokens(&tokens)
            .expect("filter-tail parser should not hard-fail")
    }

    #[test]
    fn parses_that_you_suffix_into_typed_filter_tail() {
        let parsed = parse_tail("cards in your graveyard that you cycled or discarded this turn.")
            .expect("typed suffix should parse");

        assert_eq!(parsed.player_filter, PlayerFilter::You);
        assert_eq!(
            token_word_refs(&parsed.base_tokens),
            ["cards", "in", "your", "graveyard"]
        );
    }

    #[test]
    fn parses_suffix_without_that_and_trims_base_comma() {
        let parsed = parse_tail("cards in your graveyard, you cycled or discarded this turn")
            .expect("typed suffix without 'that' should parse");

        assert_eq!(parsed.player_filter, PlayerFilter::You);
        assert_eq!(
            token_word_refs(&parsed.base_tokens),
            ["cards", "in", "your", "graveyard"]
        );
    }

    #[test]
    fn requires_the_history_phrase_to_be_the_filter_tail() {
        assert!(
            parse_tail("cards you cycled or discarded this turn with mana value three").is_none()
        );
        assert!(parse_tail("cards discarded this turn").is_none());
    }
}
