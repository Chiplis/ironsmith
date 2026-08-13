use winnow::combinator::{alt, eof, opt};
use winnow::prelude::*;

use crate::grammar::primitives;
use crate::front_end::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::zone::Zone;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReturnDestinationShape {
    pub(crate) zone: Zone,
    pub(crate) tapped: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ReturnMultipleTargetsShape<'a> {
    pub(crate) targets_tokens: &'a [OwnedLexToken],
    pub(crate) destination: ReturnDestinationShape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReturnQuantifier {
    All,
    Each,
}

impl ReturnQuantifier {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Each => "each",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReturnSegmentFacts {
    pub(crate) starts_new_target: bool,
    pub(crate) mentions_target: bool,
    pub(crate) starts_like_zone_suffix: bool,
    pub(crate) starts_like_target_reference: bool,
    pub(crate) quantifier: Option<ReturnQuantifier>,
    pub(crate) mentions_zone: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ChooseAllZonesToHandShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) zones: [Zone; 2],
}

fn marker_anywhere<'a, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    primitives::find_prefix(tokens, make_parser).is_some()
}

fn starts_any(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    primitives::parse_prefix(tokens, primitives::any_phrase(phrases).void()).is_some()
}

fn last_keyword_index(tokens: &[OwnedLexToken], keyword: &'static str) -> Option<usize> {
    let (index, (), after) = primitives::find_prefix(tokens, || primitives::kw(keyword).void())?;
    let after_start = tokens.len().checked_sub(after.len())?;
    last_keyword_index(after, keyword)
        .map(|nested| after_start + nested)
        .or(Some(index))
}

pub(crate) fn parse_return_destination_shape(
    tokens: &[OwnedLexToken],
) -> Option<ReturnDestinationShape> {
    let zone = if marker_anywhere(tokens, || {
        alt((primitives::kw("hand"), primitives::kw("hands"))).void()
    }) {
        Zone::Hand
    } else if marker_anywhere(tokens, || primitives::kw("battlefield").void()) {
        Zone::Battlefield
    } else {
        return None;
    };
    Some(ReturnDestinationShape {
        zone,
        tapped: marker_anywhere(tokens, || primitives::kw("tapped").void()),
    })
}

pub(crate) fn parse_return_multiple_targets_shape(
    tokens: &[OwnedLexToken],
) -> Option<ReturnMultipleTargetsShape<'_>> {
    let (_, body) = primitives::parse_prefix(tokens, primitives::kw("return").void())?;
    let to_index = last_keyword_index(body, "to")?;
    let targets_tokens = trim_lexed_commas(body.get(..to_index)?);
    let (_, destination_tokens) =
        primitives::parse_prefix(body.get(to_index..)?, primitives::kw("to").void())?;
    let destination_tokens = trim_lexed_commas(destination_tokens);
    if targets_tokens.is_empty()
        || destination_tokens.is_empty()
        // This grammar leaf represents one return action with several objects.
        // A later `then return` starts a distinct ordered action and must be
        // left to sequence parsing instead of inheriting its destination.
        || marker_anywhere(targets_tokens, || primitives::kw("return").void())
        || !marker_anywhere(targets_tokens, || {
            alt((
                primitives::comma().void(),
                primitives::kw("and").void(),
                primitives::kw("or").void(),
                primitives::kw("and/or").void(),
            ))
        })
    {
        return None;
    }
    Some(ReturnMultipleTargetsShape {
        targets_tokens,
        destination: parse_return_destination_shape(destination_tokens)?,
    })
}

pub(crate) fn parse_return_segment_facts(tokens: &[OwnedLexToken]) -> ReturnSegmentFacts {
    let quantifier = if starts_any(tokens, &[&["all"]]) {
        Some(ReturnQuantifier::All)
    } else if starts_any(tokens, &[&["each"]]) {
        Some(ReturnQuantifier::Each)
    } else {
        None
    };
    ReturnSegmentFacts {
        starts_new_target: starts_any(
            tokens,
            &[
                &["target"],
                &["up"],
                &["another"],
                &["other"],
                &["this"],
                &["that"],
                &["it"],
                &["them"],
                &["all"],
                &["each"],
            ],
        ),
        mentions_target: marker_anywhere(tokens, || primitives::kw("target").void()),
        starts_like_zone_suffix: starts_any(
            tokens,
            &[&["from"], &["to"], &["in"], &["on"], &["under"]],
        ),
        starts_like_target_reference: starts_any(
            tokens,
            &[
                &["target"],
                &["up"],
                &["this"],
                &["that"],
                &["it"],
                &["them"],
                &["another"],
            ],
        ),
        quantifier,
        mentions_zone: marker_anywhere(tokens, || {
            primitives::any_phrase(&[
                &["graveyard"],
                &["graveyards"],
                &["battlefield"],
                &["hand"],
                &["hands"],
                &["library"],
                &["libraries"],
                &["exile"],
            ])
            .void()
        }),
    }
}

pub(crate) fn return_zone_suffix_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (index, (), _) = primitives::find_prefix(tokens, || primitives::kw("from").void())?;
    tokens.get(index..)
}

fn battlefield_pair_prefix(input: &mut LexStream<'_>) -> winnow::error::ModalResult<()> {
    (
        primitives::kw("from"),
        opt(primitives::kw("the")),
        primitives::kw("battlefield"),
        primitives::kw("and"),
        primitives::kw("from"),
    )
        .void()
        .parse_next(input)
}

fn command_pair_prefix(input: &mut LexStream<'_>) -> winnow::error::ModalResult<()> {
    (
        primitives::kw("from"),
        opt(primitives::kw("the")),
        primitives::phrase(&["command", "zone"]),
        primitives::kw("and"),
        primitives::kw("from"),
    )
        .void()
        .parse_next(input)
}

fn parse_source_zone_pair(tokens: &[OwnedLexToken]) -> Option<[Zone; 2]> {
    let (first, rest) =
        if let Some(((), rest)) = primitives::parse_prefix(tokens, battlefield_pair_prefix) {
            (Zone::Battlefield, rest)
        } else {
            let ((), rest) = primitives::parse_prefix(tokens, command_pair_prefix)?;
            (Zone::Command, rest)
        };
    marker_anywhere(rest, || {
        alt((primitives::kw("graveyard"), primitives::kw("graveyards"))).void()
    })
    .then_some([first, Zone::Graveyard])
}

fn choose_put_returns_them_to_hand(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, (), put_tail)) = primitives::find_prefix(tokens, || primitives::kw("put").void())
    else {
        return false;
    };
    let Some(((), destination_tail)) = primitives::parse_prefix(
        put_tail,
        (
            primitives::kw("them"),
            alt((primitives::kw("into"), primitives::kw("in"))),
        )
            .void(),
    ) else {
        return false;
    };
    marker_anywhere(destination_tail, || {
        alt((primitives::kw("hand"), primitives::kw("hands"))).void()
    })
}

fn ends_in_hand_destination(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trim_lexed_commas(tokens);
    let Some((before_hand, ())) = primitives::split_lexed_once_before_suffix(tokens, 0, || {
        (alt((primitives::kw("hand"), primitives::kw("hands"))), eof).void()
    }) else {
        return false;
    };
    marker_anywhere(before_hand, || {
        alt((primitives::kw("into"), primitives::kw("in"))).void()
    })
}

pub(crate) fn parse_choose_all_zones_to_hand_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChooseAllZonesToHandShape<'_>> {
    let (starts_with_choose, body) = if let Some(((), body)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["choose", "all"]).void())
    {
        (true, body)
    } else {
        let ((), body) =
            primitives::parse_prefix(tokens, primitives::phrase(&["put", "all"]).void())?;
        (false, body)
    };
    let (from_index, (), _) = primitives::find_prefix(body, || primitives::kw("from").void())?;
    let filter_tokens = trim_lexed_commas(body.get(..from_index)?);
    if filter_tokens.is_empty() {
        return None;
    }
    let from_tokens = body.get(from_index..)?;
    let zones = parse_source_zone_pair(from_tokens)?;
    if starts_with_choose {
        if !choose_put_returns_them_to_hand(tokens) {
            return None;
        }
    } else if !ends_in_hand_destination(tokens) {
        return None;
    }
    Some(ChooseAllZonesToHandShape {
        filter_tokens,
        zones,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).unwrap()
    }

    #[test]
    fn captures_return_and_choose_all_shapes() {
        let returned = lex("Return target artifact and target enchantment to their owners' hands");
        let shape = parse_return_multiple_targets_shape(&returned).unwrap();
        assert_eq!(shape.destination.zone, Zone::Hand);

        let put = lex(
            "Put all commanders you own from the command zone and from your graveyard into your hand",
        );
        let shape = parse_choose_all_zones_to_hand_shape(&put).unwrap();
        assert_eq!(shape.zones, [Zone::Command, Zone::Graveyard]);
        assert_eq!(
            TokenWordView::new(shape.filter_tokens).to_word_refs(),
            ["commanders", "you", "own"]
        );
    }

    #[test]
    fn multi_target_return_does_not_span_a_then_return_action() {
        let returned = lex(
            "Return up to two target nonland permanent cards from your graveyard to your hand, then return up to two target land cards from your graveyard to the battlefield tapped",
        );
        assert!(parse_return_multiple_targets_shape(&returned).is_none());
    }
}
