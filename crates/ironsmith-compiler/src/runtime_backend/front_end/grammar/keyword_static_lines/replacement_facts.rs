use winnow::combinator::{alt, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::{leaf, primitives};
use super::nearby_primitives::{semantic_all, semantic_kw, semantic_noise, semantic_phrase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CounterReplacementShape<'a> {
    GenericUnderYourControl,
    EnergyYouGet,
    PlusOneAdd {
        filter_tokens: &'a [OwnedLexToken],
        additional: u32,
    },
    PlusOneDouble {
        filter_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenCreationReplacementShape<'a> {
    GenericUnderYourControl,
    AddTreasure {
        descriptor_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordActionReplacementShape<'a> {
    ProliferateYouTwice,
    ProliferateOpponentTwice,
    ExploreTwice,
    ExploreAfterScry { value_tokens: &'a [OwnedLexToken] },
}

pub(crate) fn parse_noncombat_damage_minus_counter_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> bool {
    semantic_all(
        tokens,
        semantic_phrase(&[
            "if",
            "source",
            "you",
            "control",
            "would",
            "deal",
            "noncombat",
            "damage",
            "to",
            "creature",
            "opponent",
            "controls",
            "put",
            "that",
            "many",
            "-1/-1",
            "counters",
            "on",
            "that",
            "creature",
            "instead",
        ]),
        "noncombat damage minus-counter replacement",
    )
}

pub(crate) fn parse_counter_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CounterReplacementShape<'_>> {
    if parse_generic_counter_replacement(tokens) {
        return Some(CounterReplacementShape::GenericUnderYourControl);
    }
    if parse_energy_counter_replacement(tokens) {
        return Some(CounterReplacementShape::EnergyYouGet);
    }
    primitives::parse_all(
        tokens,
        alt((parse_plus_one_add_lexed, parse_plus_one_double_lexed)),
        "counter replacement",
    )
    .ok()
}

pub(crate) fn parse_token_creation_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TokenCreationReplacementShape<'_>> {
    if parse_generic_token_replacement(tokens) {
        return Some(TokenCreationReplacementShape::GenericUnderYourControl);
    }
    primitives::parse_all(
        tokens,
        parse_add_treasure_token_replacement_lexed,
        "additional treasure token replacement",
    )
    .ok()
}

pub(crate) fn parse_keyword_action_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordActionReplacementShape<'_>> {
    primitives::parse_all(
        tokens,
        alt((
            parse_proliferate_you_replacement_lexed,
            parse_proliferate_opponent_replacement_lexed,
            parse_explore_replacement_lexed,
        )),
        "keyword-action replacement",
    )
    .ok()
}

fn parse_generic_counter_replacement(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        semantic_phrase(&[
            "if",
            "effect",
            "would",
            "put",
            "one",
            "or",
            "more",
            "counters",
            "on",
            "permanent",
            "you",
            "control",
            "it",
            "puts",
            "twice",
            "that",
            "many",
            "of",
            "those",
            "counters",
            "on",
            "that",
            "permanent",
            "instead",
        ]),
        "generic double-counter replacement",
    )
}

fn parse_proliferate_you_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordActionReplacementShape<'a>> {
    primitives::phrase(&["if", "you", "would", "proliferate"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["proliferate", "twice", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordActionReplacementShape::ProliferateYouTwice)
}

fn parse_proliferate_opponent_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordActionReplacementShape<'a>> {
    primitives::phrase(&["if", "an", "opponent", "would", "proliferate"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["that", "player", "proliferates", "twice", "instead"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordActionReplacementShape::ProliferateOpponentTwice)
}

fn parse_explore_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordActionReplacementShape<'a>> {
    primitives::phrase(&["if", "a", "creature", "you", "control", "would", "explore"])
        .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    alt((
        (
            primitives::kw("you"),
            primitives::kw("scry"),
            repeat_till::<_, _, (), _, _, _, _>(
                1..,
                any.void(),
                peek((opt(primitives::comma()), primitives::kw("then"))),
            )
            .map(|((), _)| ())
            .take(),
            opt(primitives::comma()),
            primitives::kw("then"),
            alt((
                primitives::phrase(&["it", "explores"]),
                primitives::phrase(&["that", "creature", "explores"]),
            )),
            primitives::sentence_end(),
        )
            .map(|(_, _, value_tokens, _, _, _, _)| {
                KeywordActionReplacementShape::ExploreAfterScry {
                    value_tokens: trim_lexed_commas(value_tokens),
                }
            }),
        (
            primitives::phrase(&["it", "explores"]),
            opt(primitives::comma()),
            primitives::phrase(&["then", "it", "explores", "again"]),
            primitives::sentence_end(),
        )
            .value(KeywordActionReplacementShape::ExploreTwice),
    ))
    .parse_next(input)
}

fn parse_energy_counter_replacement(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        (
            semantic_phrase(&["if", "you", "would", "get", "one", "or", "more"]),
            semantic_energy_symbol,
            opt(semantic_phrase(&["energy", "counters"])),
            semantic_phrase(&["you", "get", "twice", "that", "many"]),
            semantic_energy_symbol,
            semantic_kw("instead"),
        )
            .void(),
        "double energy-counter replacement",
    )
}

fn semantic_energy_symbol<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    any.verify(|token: &&OwnedLexToken| {
        token
            .mana_group_inner()
            .is_some_and(|inner| inner.eq_ignore_ascii_case("e"))
    })
    .void()
    .parse_next(input)
}

fn parse_plus_one_add_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CounterReplacementShape<'a>> {
    parse_plus_one_counter_prefix(input)?;
    let filter_tokens = take_until_replacement_phrase(input, &["that", "many", "plus"])?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["that", "many", "plus"]).parse_next(input)?;
    let additional = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    opt((
        primitives::kw("+1/+1"),
        alt((primitives::kw("counter"), primitives::kw("counters"))),
    ))
    .parse_next(input)?;
    primitives::phrase(&["are", "put", "on"]).parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::phrase(&["that", "creature"]),
        primitives::phrase(&["that", "permanent"]),
    ))
    .parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterReplacementShape::PlusOneAdd {
        filter_tokens: trim_lexed_commas(filter_tokens),
        additional,
    })
}

fn parse_plus_one_double_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterReplacementShape<'a>> {
    parse_plus_one_counter_prefix(input)?;
    let filter_tokens = take_until_replacement_phrase(input, &["twice", "that", "many"])?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["twice", "that", "many"]).parse_next(input)?;
    opt((
        primitives::kw("+1/+1"),
        alt((primitives::kw("counter"), primitives::kw("counters"))),
    ))
    .parse_next(input)?;
    primitives::phrase(&["are", "put", "on"]).parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::phrase(&["that", "creature"]),
        primitives::phrase(&["that", "permanent"]),
    ))
    .parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterReplacementShape::PlusOneDouble {
        filter_tokens: trim_lexed_commas(filter_tokens),
    })
}

fn parse_plus_one_counter_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "if", "one", "or", "more", "+1/+1", "counters", "would", "be", "put", "on",
    ])
    .parse_next(input)
}

fn take_until_replacement_phrase<'a>(
    input: &mut LexStream<'a>,
    phrase: &'static [&'static str],
) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((opt(primitives::comma()), primitives::phrase(phrase))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)
}

fn parse_generic_token_replacement(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        alt((
            semantic_phrase(&[
                "if", "effect", "would", "create", "one", "or", "more", "tokens", "under", "your",
                "control", "it", "creates", "twice", "that", "many", "of", "those", "tokens",
                "instead",
            ]),
            semantic_phrase(&[
                "if", "one", "or", "more", "tokens", "would", "be", "created", "under", "your",
                "control", "twice", "that", "many", "of", "those", "tokens", "are", "created",
                "instead",
            ]),
        )),
        "generic double-token replacement",
    )
}

fn parse_add_treasure_token_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TokenCreationReplacementShape<'a>> {
    primitives::phrase(&["if", "you", "would", "create", "one", "or", "more"]).parse_next(input)?;
    let descriptor_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("token"), primitives::kw("tokens")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens"))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["instead", "create", "those", "tokens", "plus"]).parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    primitives::kw("additional").parse_next(input)?;
    let repeated_descriptor = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("token"), primitives::kw("tokens")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens"))).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let descriptor_words = TokenWordView::new(descriptor_tokens).word_refs();
    let repeated_words = TokenWordView::new(repeated_descriptor).word_refs();
    if descriptor_words != repeated_words
        || primitives::find_prefix(descriptor_tokens, || primitives::kw("treasure").void())
            .is_none()
    {
        return Err(primitives::backtrack_err(
            "additional treasure replacement",
            "matching Treasure token descriptors",
        ));
    }
    Ok(TokenCreationReplacementShape::AddTreasure {
        descriptor_tokens: trim_lexed_commas(descriptor_tokens),
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_counter_and_token_replacements() {
        let tokens = lex_line(
            "If one or more +1/+1 counters would be put on a creature you control, that many plus one +1/+1 counters are put on it instead.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_counter_replacement_tokens(&tokens),
            Some(CounterReplacementShape::PlusOneAdd { additional: 1, .. })
        ));
        let permanent = lex_line(
            "If one or more +1/+1 counters would be put on a permanent you control, that many plus one +1/+1 counters are put on that permanent instead.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_counter_replacement_tokens(&permanent),
            Some(CounterReplacementShape::PlusOneAdd { additional: 1, .. })
        ));
        let tokens = lex_line(
            "If you would create one or more Treasure tokens, instead create those tokens plus an additional Treasure token.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_token_creation_replacement_tokens(&tokens),
            Some(TokenCreationReplacementShape::AddTreasure { .. })
        ));

        let energy = lex_line(
            "If you would get one or more {E} (energy counters), you get twice that many {E} instead.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_counter_replacement_tokens(&energy),
            Some(CounterReplacementShape::EnergyYouGet)
        );
    }

    #[test]
    fn parses_comma_separated_double_explore_replacement() {
        let tokens = lex_line(
            "If a creature you control would explore, instead it explores, then it explores again.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_keyword_action_replacement_tokens(&tokens),
            Some(KeywordActionReplacementShape::ExploreTwice)
        );
    }
}
