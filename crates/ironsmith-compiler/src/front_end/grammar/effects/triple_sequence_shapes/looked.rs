use std::ops::Range;

use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::effect::ChoiceCount;
use crate::object::CounterType;
use crate::front_end::lexer::{LexStream, OwnedLexToken};
use crate::grammar::{leaf, primitives};

use super::super::control_copy_attach_shapes::{
    BattlefieldControllerShape, parse_battlefield_controller_prefix,
};
use super::super::sequence_pairs::{
    contains_sequence_phrase, contains_sequence_word, finish_sequence_words, seek_sequence_phrase,
    sequence_any_phrase, sequence_phrase, starts_sequence,
};
use super::parse_consult_remainder_order_tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LookedMoveDestinationShape {
    Hand,
    Battlefield {
        tapped: bool,
        attacking: bool,
        attacks_that_player: bool,
        controller: Option<BattlefieldControllerShape>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedMoveActionShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: Range<usize>,
    pub(crate) destination: LookedMoveDestinationShape,
    pub(crate) all_matching: bool,
    pub(crate) entry_counter: Option<(u32, CounterType)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedHandActionShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: Range<usize>,
    pub(crate) filter_uses_and_or: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedTopActionShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedTopAndRemainderActionShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: Range<usize>,
    pub(crate) remainder_order: LibraryBottomOrderAst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedCastActionShape {
    pub(crate) filter: Range<usize>,
    pub(crate) mentions_spell: bool,
    pub(crate) mana_value_limit: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LookedRemainderShape {
    Graveyard,
    LibraryBottom(LibraryBottomOrderAst),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnyNumberRevealedChoiceShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RevealOneGainManaValueShape {
    pub(crate) view: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookedRevealSelectionShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: Range<usize>,
    pub(crate) remainder_order: LibraryBottomOrderAst,
}

const FROM_AMONG: &[&[&str]] = &[
    &["from", "among", "those", "cards"],
    &["from", "among", "the", "cards", "revealed", "this", "way"],
    &["from", "among", "the", "cards", "milled", "this", "way"],
    &["from", "among", "the", "milled", "cards"],
    &["from", "among", "them"],
];
const INTO_HAND: &[&[&str]] = &[
    &["into", "your", "hand"],
    &["into", "hand"],
    &["to", "your", "hand"],
    &["to", "hand"],
];
const BATTLEFIELD_TAPPED: &[&[&str]] = &[
    &["onto", "the", "battlefield", "tapped"],
    &["onto", "battlefield", "tapped"],
];
const BATTLEFIELD: &[&[&str]] = &[&["onto", "the", "battlefield"], &["onto", "battlefield"]];
const PUT_ONE_INTO_HAND: &[&[&str]] = &[
    &["put", "one", "of", "them", "into", "your", "hand"],
    &["put", "one", "of", "those", "cards", "into", "your", "hand"],
    &["put", "one", "into", "your", "hand"],
];

fn split_from_among(tokens: &[OwnedLexToken]) -> Option<(Range<usize>, &[OwnedLexToken])> {
    let mut input = LexStream::new(tokens);
    let head_end = seek_sequence_phrase(&mut input, FROM_AMONG).ok()?;
    sequence_any_phrase(FROM_AMONG)
        .parse_next(&mut input)
        .ok()?;
    let tail_start = tokens.len().saturating_sub(input.len());
    (head_end > 0).then_some((0..head_end, &tokens[tail_start..]))
}

fn counted_filter_range(
    tokens: &[OwnedLexToken],
    head: Range<usize>,
) -> (ChoiceCount, Range<usize>) {
    let head_tokens = &tokens[head.clone()];
    if let Some((count, rest)) =
        primitives::parse_prefix(head_tokens, leaf::parse_leaf_choice_count_prefix_lexed)
    {
        let start = head.end.saturating_sub(rest.len());
        (count, start..head.end)
    } else {
        (ChoiceCount::up_to(1), head)
    }
}

fn parse_battlefield_entry_counter(tail: &[OwnedLexToken]) -> Option<(u32, CounterType)> {
    let (_, (), after_with) = primitives::find_prefix(tail, || primitives::kw("with").void())?;
    let (on_idx, (), after_on) =
        primitives::find_prefix(after_with, || primitives::phrase(&["on", "it"]).void())?;
    if after_on.iter().any(|token| token.as_word().is_some()) {
        return None;
    }
    let descriptor =
        super::super::zone_counter_shapes::parse_counter_descriptor_shape(&after_with[..on_idx])?;
    Some((descriptor.count, descriptor.counter_type))
}

pub(crate) fn parse_looked_move_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedMoveActionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let all_matching = tokens
        .get(head.clone())?
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "all");
    let (count, filter) = if all_matching {
        (
            ChoiceCount::any_number(),
            head.start.saturating_add(1)..head.end,
        )
    } else {
        counted_filter_range(tokens, head)
    };
    if filter.is_empty() {
        return None;
    }
    let destination = if starts_sequence(tail, INTO_HAND) {
        LookedMoveDestinationShape::Hand
    } else {
        let tapped = starts_sequence(tail, BATTLEFIELD_TAPPED);
        if !tapped && !starts_sequence(tail, BATTLEFIELD) {
            return None;
        }
        let attacking = contains_sequence_word(tail, "attacking");
        let controller = tail.iter().enumerate().find_map(|(index, token)| {
            token
                .is_word("under")
                .then(|| parse_battlefield_controller_prefix(&tail[index..]))
                .flatten()
                .map(|shape| shape.controller)
        });
        LookedMoveDestinationShape::Battlefield {
            tapped,
            attacking,
            attacks_that_player: attacking
                && contains_sequence_phrase(tail, &[&["attacking", "that", "player"]]),
            controller,
        }
    };
    Some(LookedMoveActionShape {
        count,
        filter,
        destination,
        all_matching,
        entry_counter: parse_battlefield_entry_counter(tail),
    })
}

const REVEAL_TO_HAND: &[&[&str]] = &[
    &["and", "put", "it", "into"],
    &["put", "it", "into"],
    &["and", "put", "them", "into"],
    &["put", "them", "into"],
    &["and", "put", "that", "card", "into"],
    &["put", "that", "card", "into"],
    &["and", "put", "the", "revealed"],
    &["and", "put", "those", "cards"],
    &["and", "put", "them"],
];
const REVEAL_TO_TOP: &[&[&str]] = &[
    &["and", "put", "it", "on", "top"],
    &["put", "it", "on", "top"],
    &["and", "put", "that", "card", "on", "top"],
    &["put", "that", "card", "on", "top"],
    &["and", "put", "them", "on", "top"],
    &["put", "them", "on", "top"],
    &["and", "put", "those", "cards", "on", "top"],
    &["put", "those", "cards", "on", "top"],
];

pub(crate) fn parse_looked_hand_action_shape(
    tokens: &[OwnedLexToken],
    reveal_chosen: bool,
) -> Option<LookedHandActionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let (count, filter) = counted_filter_range(tokens, head);
    if filter.is_empty() {
        return None;
    }
    let valid_tail = if reveal_chosen {
        starts_sequence(tail, REVEAL_TO_HAND) && contains_sequence_word(tail, "hand")
    } else {
        starts_sequence(tail, &[&["into"]]) && contains_sequence_word(tail, "hand")
    };
    valid_tail.then_some(LookedHandActionShape {
        count,
        filter: filter.clone(),
        filter_uses_and_or: contains_sequence_word(&tokens[filter], "and/or"),
    })
}

pub(crate) fn parse_looked_top_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedTopActionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let (count, filter) = counted_filter_range(tokens, head);
    if filter.is_empty()
        || !starts_sequence(tail, REVEAL_TO_TOP)
        || !contains_sequence_word(tail, "library")
    {
        return None;
    }
    Some(LookedTopActionShape { count, filter })
}

/// Parses the single-sentence follow-up used after an optional look, such as
/// "reveal up to one land card from among them, then put that card on top ...
/// and the rest on the bottom ...". The selected subset and the remainder
/// stay tied to the same looked-at collection.
pub(crate) fn parse_looked_top_and_remainder_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedTopAndRemainderActionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let (count, filter) = counted_filter_range(tokens, head);
    let top_tail = primitives::parse_prefix(tail, |input: &mut LexStream<'_>| {
        sequence_phrase(&["then"]).parse_next(input)
    })
    .map(|(_, rest)| rest)
    .unwrap_or(tail);
    if filter.is_empty()
        || !starts_sequence(top_tail, REVEAL_TO_TOP)
        || !contains_sequence_word(top_tail, "rest")
        || !contains_sequence_word(top_tail, "bottom")
        || !contains_sequence_word(top_tail, "library")
    {
        return None;
    }
    Some(LookedTopAndRemainderActionShape {
        count,
        filter,
        remainder_order: parse_consult_remainder_order_tokens(top_tail)?,
    })
}

pub(crate) fn parse_looked_cast_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCastActionShape> {
    let (filter, tail) = split_from_among(tokens)?;
    if !starts_sequence(tail, &[&["without", "paying", "its", "mana", "cost"]]) {
        return None;
    }
    let filter_tokens = &tokens[filter.clone()];
    let mentions_spell = contains_sequence_word(filter_tokens, "spell")
        || contains_sequence_word(filter_tokens, "spells");
    let mana_value_limit = parse_mana_value_limit(filter_tokens);
    Some(LookedCastActionShape {
        filter,
        mentions_spell,
        mana_value_limit,
    })
}

/// Parses a reveal selection whose source and remainder both refer to the
/// preceding looked-at collection, for example "up to two creature and/or
/// land cards from among them, then put the rest on the bottom ...".
pub(crate) fn parse_looked_reveal_selection_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedRevealSelectionShape> {
    let (head, tail) = split_from_among(tokens)?;
    let (count, filter) = counted_filter_range(tokens, head);
    if filter.is_empty()
        || !contains_sequence_phrase(tail, &[&["put", "the", "rest"], &["put", "rest"]])
    {
        return None;
    }
    Some(LookedRevealSelectionShape {
        count,
        filter,
        remainder_order: parse_consult_remainder_order_tokens(tail)?,
    })
}

pub(crate) fn is_revealed_land_creature_split_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(tokens, &[&["put"]])
        && contains_sequence_phrase(
            tokens,
            &[&[
                "all",
                "land",
                "cards",
                "revealed",
                "this",
                "way",
                "onto",
                "the",
                "battlefield",
                "tapped",
            ]],
        )
        && contains_sequence_phrase(
            tokens,
            &[&[
                "put", "all", "creature", "cards", "revealed", "this", "way", "into", "your",
                "hand",
            ]],
        )
}

fn parse_mana_value_limit(tokens: &[OwnedLexToken]) -> Option<u32> {
    let mut input = LexStream::new(tokens);
    seek_sequence_phrase(&mut input, &[&["mana", "value"]]).ok()?;
    sequence_phrase(&["mana", "value"])
        .parse_next(&mut input)
        .ok()?;
    let value = leaf::parse_leaf_number_prefix_lexed
        .parse_next(&mut input)
        .ok()?;
    sequence_phrase(&["or", "less"])
        .parse_next(&mut input)
        .ok()?;
    Some(value)
}

pub(crate) fn parse_looked_remainder_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedRemainderShape> {
    let tail = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        sequence_phrase(&["then"]).parse_next(input)
    })
    .map(|(_, tail)| tail)
    .unwrap_or(tokens);
    // Multi-sentence looked-card procedures commonly restate the library
    // owner before the final disposition ("That player puts the rest ..."
    // or "They put the rest ..."). The enclosing sequence parser already
    // carries the original player structurally, so this grammar only needs to
    // remove the anaphoric actor before recognizing the remainder action.
    let tail = match tail {
        [first, second, rest @ ..]
            if first.parser_text() == "that"
                && matches!(second.parser_text(), "player" | "opponent") =>
        {
            rest
        }
        [first, rest @ ..] if first.parser_text() == "they" => rest,
        _ => tail,
    };
    let explicit_rest = contains_sequence_word(tail, "rest");
    let exact_revealed_complement =
        is_explicit_revealed_cards_not_put_onto_battlefield_complement(tail);
    if !starts_sequence(tail, &[&["put"], &["puts"]])
        || (!explicit_rest && !exact_revealed_complement)
    {
        return None;
    }
    if contains_sequence_word(tail, "bottom") && contains_sequence_word(tail, "library") {
        return parse_consult_remainder_order_tokens(tail).map(LookedRemainderShape::LibraryBottom);
    }
    contains_sequence_word(tail, "graveyard").then_some(LookedRemainderShape::Graveyard)
}

/// Whether the authored remainder explicitly names the revealed-card
/// complement rather than using the shorter "the rest" surface.
pub(crate) fn is_explicit_revealed_cards_not_put_onto_battlefield_complement(
    tokens: &[OwnedLexToken],
) -> bool {
    contains_sequence_phrase(tokens, &[&["all", "cards", "revealed", "this", "way"]])
        && (contains_sequence_phrase(tokens, &[&["werent", "put", "onto", "the", "battlefield"]])
            || contains_sequence_phrase(
                tokens,
                &[&["weren't", "put", "onto", "the", "battlefield"]],
            )
            || contains_sequence_phrase(
                tokens,
                &[&["were", "not", "put", "onto", "the", "battlefield"]],
            ))
}

/// Retains the authored wording for a looked/revealed-set complement while
/// leaving its execution semantics unchanged.
pub(crate) fn looked_remainder_surface(
    tokens: &[OwnedLexToken],
) -> ironsmith_core::LibraryRemainderSurface {
    if is_explicit_revealed_cards_not_put_onto_battlefield_complement(tokens) {
        return ironsmith_core::LibraryRemainderSurface::RevealedCardsNotPutOntoBattlefield;
    }
    if contains_sequence_phrase(
        tokens,
        &[
            &[
                "the", "rest", "of", "the", "cards", "revealed", "this", "way",
            ],
            &["the", "rest", "of", "cards", "revealed", "this", "way"],
        ],
    ) {
        return ironsmith_core::LibraryRemainderSurface::RestOfCardsRevealedThisWay;
    }
    ironsmith_core::LibraryRemainderSurface::Rest
}

/// Recognizes an optional deployment whose legal candidate must share a name
/// with some permanent on the battlefield.  The comparison set is modeled by
/// the sequence parser rather than treating the trailing `if` as a gate on an
/// arbitrary card from the looked pool.
pub(crate) fn is_looked_same_name_permanent_battlefield_action(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(
        tokens,
        &[
            &["you", "may", "put", "one", "of", "those", "cards"],
            &["you", "may", "put", "one", "of", "them"],
        ],
    ) && contains_sequence_phrase(tokens, BATTLEFIELD)
        && contains_sequence_phrase(
            tokens,
            &[
                &[
                    "if",
                    "it",
                    "has",
                    "the",
                    "same",
                    "name",
                    "as",
                    "a",
                    "permanent",
                ],
                &[
                    "if",
                    "that",
                    "card",
                    "has",
                    "the",
                    "same",
                    "name",
                    "as",
                    "a",
                    "permanent",
                ],
            ],
        )
}

pub(crate) fn parse_any_number_revealed_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<AnyNumberRevealedChoiceShape> {
    let (_, tail) = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        sequence_phrase(&["choose"]).parse_next(input)
    })?;
    let (count, after_count) =
        primitives::parse_prefix(tail, leaf::parse_leaf_choice_count_prefix_lexed)?;
    if count != ChoiceCount::any_number() {
        return None;
    }
    let mut input = LexStream::new(after_count);
    let filter_end = seek_sequence_phrase(&mut input, &[&["revealed", "this", "way"]]).ok()?;
    sequence_phrase(&["revealed", "this", "way"])
        .parse_next(&mut input)
        .ok()?;
    finish_sequence_words(&mut input).ok()?;
    let filter_start =
        tokens.len().saturating_sub(tail.len()) + tail.len().saturating_sub(after_count.len());
    (filter_end > 0).then_some(AnyNumberRevealedChoiceShape {
        count,
        filter: filter_start..filter_start + filter_end,
    })
}

pub(crate) fn is_land_nonland_split_bottom_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(tokens, &[&["put"]])
        && contains_sequence_phrase(
            tokens,
            &[&[
                "all",
                "nonland",
                "cards",
                "chosen",
                "this",
                "way",
                "onto",
                "the",
                "battlefield",
            ]],
        )
        && contains_sequence_phrase(
            tokens,
            &[&[
                "all",
                "land",
                "cards",
                "chosen",
                "this",
                "way",
                "onto",
                "the",
                "battlefield",
                "tapped",
            ]],
        )
        && parse_looked_remainder_shape(tokens)
            == Some(LookedRemainderShape::LibraryBottom(
                LibraryBottomOrderAst::Random,
            ))
}

pub(crate) fn parse_reveal_one_gain_mana_value_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Option<RevealOneGainManaValueShape> {
    let mut input = LexStream::new(first);
    let view_end = seek_sequence_phrase(&mut input, &[&["and", "put"]]).ok()?;
    sequence_phrase(&["and"]).parse_next(&mut input).ok()?;
    let tail_start = first.len().saturating_sub(input.len());
    if !starts_sequence(&first[tail_start..], PUT_ONE_INTO_HAND)
        || !starts_sequence(second, &[&["you", "gain", "life"]])
        || !contains_sequence_phrase(second, &[&["mana", "value"]])
        || (!contains_sequence_word(second, "card")
            && !contains_sequence_word(second, "cards")
            && !contains_sequence_word(second, "card's"))
        || !starts_sequence(third, &[&["put"], &["puts"]])
        || !contains_sequence_word(third, "other")
        || !contains_sequence_word(third, "revealed")
        || !contains_sequence_word(third, "graveyard")
    {
        return None;
    }
    Some(RevealOneGainManaValueShape { view: 0..view_end })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{lex_line, split_lexed_sentences};

    #[test]
    fn parses_reveal_one_gain_mana_value_shape() {
        let tokens = lex_line(
            "Reveal the top three cards of your library and put one of them into your hand. You gain life equal to that card's mana value. Put all other cards revealed this way into your graveyard.",
            0,
        )
        .unwrap();
        let sentences = split_lexed_sentences(&tokens);
        assert!(
            parse_reveal_one_gain_mana_value_shape(sentences[0], sentences[1], sentences[2])
                .is_some()
        );
    }

    #[test]
    fn parses_all_matching_looked_cards_as_mandatory_full_set() {
        let tokens = lex_line(
            "all land cards from among them onto the battlefield tapped and the rest on the bottom of your library in a random order",
            0,
        )
        .unwrap();
        let shape = parse_looked_move_action_shape(&tokens).expect("all-matching move shape");

        assert!(shape.all_matching);
        assert_eq!(shape.count, ChoiceCount::any_number());
        assert_eq!(tokens[shape.filter.start].as_word(), Some("land"));
        assert!(matches!(
            shape.destination,
            LookedMoveDestinationShape::Battlefield { tapped: true, .. }
        ));
    }

    #[test]
    fn parses_explicit_looked_card_battlefield_controller() {
        let tokens = lex_line(
            "a nonland permanent card with mana value X or less from among them onto the battlefield under your control",
            0,
        )
        .unwrap();
        let shape = parse_looked_move_action_shape(&tokens).expect("looked-card move shape");

        assert!(matches!(
            shape.destination,
            LookedMoveDestinationShape::Battlefield {
                controller: Some(BattlefieldControllerShape::You),
                ..
            }
        ));
    }

    #[test]
    fn parses_return_from_among_them_to_hand_surface() {
        let tokens = lex_line("a permanent card from among them to your hand", 0).unwrap();
        let shape = parse_looked_move_action_shape(&tokens).expect("return-to-hand move shape");

        assert!(matches!(
            shape.destination,
            LookedMoveDestinationShape::Hand
        ));
        assert_eq!(tokens[shape.filter.start].as_word(), Some("permanent"));
    }

    #[test]
    fn parses_revealed_cards_not_deployed_as_exact_remainder() {
        let tokens = lex_line(
            "Then put all cards revealed this way that weren't put onto the battlefield on the bottom of your library in a random order.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_looked_remainder_shape(&tokens),
            Some(LookedRemainderShape::LibraryBottom(
                LibraryBottomOrderAst::Random
            ))
        );
    }

    #[test]
    fn recognizes_same_name_permanent_candidate_restriction() {
        let tokens = lex_line(
            "You may put one of those cards onto the battlefield if it has the same name as a permanent.",
            0,
        )
        .unwrap();
        assert!(is_looked_same_name_permanent_battlefield_action(&tokens));
    }
}
