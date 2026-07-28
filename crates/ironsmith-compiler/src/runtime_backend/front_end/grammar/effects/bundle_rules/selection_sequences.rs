use winnow::combinator::{alt, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::ChoiceCount;
use crate::runtime_backend::front_end::grammar::{filters, leaf, primitives};
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, split_lexed_sentences, trim_lexed_commas,
};
use crate::target::ObjectFilter;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProliferateChoosePhaseOutShape {
    pub(crate) count: ChoiceCount,
    pub(crate) filter: ObjectFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EachGraveyardOwnerShuffleShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CollectionScopedEachUpkeepReturnShape;

fn commas<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., primitives::comma().void()).parse_next(input)
}

fn selected_cards_owner_shuffle<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["their", "owners", "shuffle"])
        .void()
        .parse_next(input)?;
    alt((
        primitives::phrase(&["those", "cards"]),
        primitives::kw("them").void(),
    ))
    .parse_next(input)?;
    primitives::phrase(&["into", "their", "libraries"])
        .void()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

pub(crate) fn parse_each_graveyard_owner_shuffle_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<EachGraveyardOwnerShuffleShape> {
    let (choice_head, suffix) = primitives::split_lexed_once_on_separator(first, || {
        primitives::phrase(&["in", "each", "graveyard"]).void()
    })?;
    primitives::parse_all_or_none(
        suffix,
        primitives::sentence_end(),
        "each-graveyard choice suffix",
    )
    .ok()
    .flatten()?;

    let (_, choice_body) = primitives::parse_prefix(
        trim_lexed_commas(choice_head),
        (opt(primitives::kw("you")), primitives::kw("choose")),
    )?;
    if trim_lexed_commas(choice_body).is_empty() {
        return None;
    }

    primitives::parse_all_or_none(
        trim_lexed_commas(second),
        selected_cards_owner_shuffle,
        "chosen cards owner shuffle",
    )
    .ok()
    .flatten()?;
    Some(EachGraveyardOwnerShuffleShape)
}

fn collection_scoped_each_upkeep_return<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["for", "as", "long", "as", "any", "of", "those", "cards"])
        .void()
        .parse_next(input)?;
    primitives::phrase(&["remain", "exiled"])
        .void()
        .parse_next(input)?;
    commas(input)?;
    primitives::phrase(&["at", "the", "beginning", "of", "each", "player's", "upkeep"])
        .void()
        .parse_next(input)?;
    commas(input)?;
    primitives::phrase(&[
        "that",
        "player",
        "returns",
        "one",
        "of",
        "the",
        "exiled",
        "cards",
        "they",
        "own",
        "to",
        "the",
        "battlefield",
    ])
    .void()
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

pub(crate) fn parse_collection_scoped_each_upkeep_return_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<CollectionScopedEachUpkeepReturnShape> {
    let (_, exile_tail) = primitives::parse_prefix(
        trim_lexed_commas(first),
        primitives::phrase(&["exile", "all"]),
    )?;
    if exile_tail.is_empty() {
        return None;
    }
    primitives::parse_all_or_none(
        trim_lexed_commas(second),
        collection_scoped_each_upkeep_return,
        "collection-scoped each-upkeep return",
    )
    .ok()
    .flatten()?;
    Some(CollectionScopedEachUpkeepReturnShape)
}

fn proliferate_then_choose<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(primitives::kw("you")).parse_next(input)?;
    primitives::kw("proliferate").parse_next(input)?;
    commas(input)?;
    primitives::phrase(&["then", "choose"])
        .void()
        .parse_next(input)
}

fn counter_received_this_way_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["that", "had"]).parse_next(input)?;
    alt((
        primitives::phrase(&["a", "counter"]),
        primitives::kw("counters").void(),
    ))
    .parse_next(input)?;
    primitives::phrase(&["put", "on", "them", "this", "way"])
        .void()
        .parse_next(input)
}

fn chosen_objects_phase_out<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["those", "permanents"]),
        primitives::phrase(&["those", "objects"]),
        primitives::kw("they").void(),
    ))
    .parse_next(input)?;
    primitives::phrase(&["phase", "out"])
        .void()
        .parse_next(input)
}

pub(crate) fn parse_proliferate_choose_phase_out_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ProliferateChoosePhaseOutShape> {
    let sentences = split_lexed_sentences(tokens);
    let [selection, phase_out] = sentences.as_slice() else {
        return None;
    };
    let (_, selection_tail) = primitives::parse_prefix(selection, proliferate_then_choose)?;
    let (count_and_filter, suffix) =
        primitives::split_lexed_once_on_separator(selection_tail, || {
            counter_received_this_way_suffix
        })?;
    if !trim_lexed_commas(suffix).is_empty() {
        return None;
    }
    let (count, filter_tokens) = primitives::parse_prefix(
        trim_lexed_commas(count_and_filter),
        leaf::parse_leaf_choice_count_prefix_lexed,
    )?;
    let mut filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(filter_tokens),
        false,
    )
    .ok()?;
    filter.zone = Some(Zone::Battlefield);
    primitives::parse_all_or_none(
        trim_lexed_commas(phase_out),
        chosen_objects_phase_out,
        "chosen objects phase out",
    )
    .ok()
    .flatten()?;
    Some(ProliferateChoosePhaseOutShape { count, filter })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;
    use crate::types::CardType;

    #[test]
    fn parses_proliferate_choose_phase_out_for_typed_filter() {
        let tokens = lex_line(
            "Proliferate, then choose up to two artifacts you control that had a counter put on them this way. Those permanents phase out.",
            0,
        )
        .unwrap();
        let shape = parse_proliferate_choose_phase_out_tokens(&tokens).unwrap();
        assert_eq!(shape.count, ChoiceCount::up_to(2));
        assert!(shape.filter.card_types.contains(&CardType::Artifact));
    }

    #[test]
    fn parses_each_graveyard_choice_followed_by_owner_shuffle() {
        let tokens = lex_line(
            "Choose three cards in each graveyard. Their owners shuffle those cards into their libraries.",
            0,
        )
        .unwrap();
        let sentences = split_lexed_sentences(&tokens);
        assert_eq!(sentences.len(), 2);
        assert!(parse_each_graveyard_owner_shuffle_shape(sentences[0], sentences[1]).is_some());
    }

    #[test]
    fn rejects_single_graveyard_or_unrelated_shuffle_subject() {
        let tokens = lex_line(
            "Choose three cards in a graveyard. Their owners shuffle those cards into their libraries.",
            0,
        )
        .unwrap();
        let sentences = split_lexed_sentences(&tokens);
        assert!(parse_each_graveyard_owner_shuffle_shape(sentences[0], sentences[1]).is_none());

        let tokens = lex_line(
            "Choose three cards in each graveyard. Target player shuffles their library.",
            0,
        )
        .unwrap();
        let sentences = split_lexed_sentences(&tokens);
        assert!(parse_each_graveyard_owner_shuffle_shape(sentences[0], sentences[1]).is_none());
    }

    #[test]
    fn parses_collection_scoped_each_upkeep_owner_selection() {
        let tokens = lex_line(
            "Exile all permanents. For as long as any of those cards remain exiled, at the beginning of each player's upkeep, that player returns one of the exiled cards they own to the battlefield.",
            0,
        )
        .unwrap();
        let sentences = split_lexed_sentences(&tokens);
        assert_eq!(sentences.len(), 2);
        assert!(
            parse_collection_scoped_each_upkeep_return_shape(sentences[0], sentences[1]).is_some()
        );
    }

    #[test]
    fn rejects_unscoped_or_controller_owned_upkeep_return() {
        let tokens = lex_line(
            "Exile all permanents. At the beginning of each player's upkeep, that player returns one of the exiled cards they own to the battlefield.",
            0,
        )
        .unwrap();
        let sentences = split_lexed_sentences(&tokens);
        assert!(
            parse_collection_scoped_each_upkeep_return_shape(sentences[0], sentences[1]).is_none()
        );

        let tokens = lex_line(
            "Exile all permanents. For as long as any of those cards remain exiled, at the beginning of each player's upkeep, that player returns one of the exiled cards you own to the battlefield.",
            0,
        )
        .unwrap();
        let sentences = split_lexed_sentences(&tokens);
        assert!(
            parse_collection_scoped_each_upkeep_return_shape(sentences[0], sentences[1]).is_none()
        );
    }
}
