use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::effect::ChoiceCount;
use crate::runtime_backend::front_end::grammar::{leaf, permission_shapes, primitives};
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, trim_lexed_commas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestDestinationShape {
    BottomOfLibrary,
    Graveyard,
    Hand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FromAmongDestinationShape {
    Battlefield,
    Hand,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TaggedPutShape {
    pub(crate) count: Option<ChoiceCount>,
    pub(crate) rest_destination: Option<RestDestinationShape>,
    pub(crate) bottom_order: Option<LibraryBottomOrderAst>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TaggedTopPutShape {
    pub(crate) count: ChoiceCount,
    pub(crate) bottom_order: LibraryBottomOrderAst,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FromAmongPutShape<'a> {
    pub(crate) count: ChoiceCount,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) destination: FromAmongDestinationShape,
    pub(crate) rest_destination: Option<RestDestinationShape>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RevealedRemainderShape {
    pub(crate) random_order: bool,
    /// `true` for "the rest of the revealed cards", where the most recently
    /// selected card must be excluded. `false` for the whole revealed
    /// collection.
    pub(crate) exclude_current_reference: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartitionBattlefieldControllerShape {
    You,
    SubjectPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TaggedBattlefieldPartitionShape {
    pub(crate) count: ChoiceCount,
    pub(crate) chosen_tapped: bool,
    pub(crate) chosen_controller: PartitionBattlefieldControllerShape,
    pub(crate) remainder_tapped: bool,
    pub(crate) remainder_controller: PartitionBattlefieldControllerShape,
}

fn rest_head(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    for phrase in [
        &["and", "the", "rest"][..],
        &["and", "rest"][..],
        &["then", "the", "rest"][..],
        &["then", "rest"][..],
    ] {
        if let Some((_, _, tail)) = primitives::find_prefix(tokens, || primitives::phrase(phrase)) {
            return Some(trim_lexed_commas(tail));
        }
    }
    None
}

pub(crate) fn parse_rest_destination(tokens: &[OwnedLexToken]) -> Option<RestDestinationShape> {
    let tail = rest_head(tokens)?;
    if primitives::contains_word(tail, "bottom")
        && (primitives::contains_word(tail, "library")
            || primitives::contains_word(tail, "libraries"))
    {
        return Some(RestDestinationShape::BottomOfLibrary);
    }
    if primitives::contains_word(tail, "graveyard") || primitives::contains_word(tail, "graveyards")
    {
        return Some(RestDestinationShape::Graveyard);
    }
    if primitives::contains_word(tail, "hand") || primitives::contains_word(tail, "hands") {
        return Some(RestDestinationShape::Hand);
    }
    None
}

fn strip_optional_put(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(trim_lexed_commas(tokens), opt(primitives::kw("put")).void())
        .map(|(_, rest)| trim_lexed_commas(rest))
        .unwrap_or(tokens)
}

fn exact_looked_reference(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::exact_tokens_any(
        trim_lexed_commas(tokens),
        &[
            &["of", "them"],
            &["them"],
            &["of", "those", "card"],
            &["of", "those", "cards"],
            &["those", "card"],
            &["those", "cards"],
        ],
    )
}

fn parse_count_and_reference(tokens: &[OwnedLexToken]) -> Option<ChoiceCount> {
    let parsed = leaf::parse_leaf_choice_count_prefix_tokens(tokens)?;
    let reference = tokens.get(parsed.consumed..)?;
    exact_looked_reference(reference).then_some(parsed.count)
}

fn parse_partition_battlefield_destination(
    tokens: &[OwnedLexToken],
) -> Option<(bool, PartitionBattlefieldControllerShape)> {
    let tokens = trim_lexed_commas(tokens);
    let (_, tokens) = primitives::parse_prefix(tokens, opt(primitives::kw("the")).void())?;
    let (_, tokens) = primitives::parse_prefix(tokens, primitives::kw("battlefield").void())?;
    let (tapped, tokens) = if let Some((_, rest)) =
        primitives::parse_prefix(tokens, primitives::kw("tapped").void())
    {
        (true, rest)
    } else {
        (false, tokens)
    };
    let (_, tokens) = primitives::parse_prefix(tokens, primitives::kw("under").void())?;
    let (controller, tokens) = if let Some((_, rest)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["your", "control"]).void())
    {
        (PartitionBattlefieldControllerShape::You, rest)
    } else {
        let (_, rest) =
            primitives::parse_prefix(tokens, primitives::phrase(&["their", "control"]).void())?;
        (PartitionBattlefieldControllerShape::SubjectPlayer, rest)
    };
    trim_lexed_commas(tokens)
        .is_empty()
        .then_some((tapped, controller))
}

/// Parse a tagged collection split between two battlefield controllers, such
/// as "put one of those cards ... under your control and the rest ... under
/// their control". This is a reusable collection-partition shape; the parser
/// deliberately requires both destinations to be explicit so an unrelated
/// conjunction cannot be swallowed as a remainder move.
pub(crate) fn parse_tagged_battlefield_partition_shape(
    tokens: &[OwnedLexToken],
) -> Option<TaggedBattlefieldPartitionShape> {
    let body = strip_optional_put(tokens);
    let (rest_index, _, remainder_destination) =
        primitives::find_prefix(body, || primitives::phrase(&["and", "the", "rest"]).void())?;
    let chosen_clause = trim_lexed_commas(body.get(..rest_index)?);
    let (onto_index, _, chosen_destination) =
        primitives::find_prefix(chosen_clause, || primitives::kw("onto"))?;
    let count = parse_count_and_reference(trim_lexed_commas(chosen_clause.get(..onto_index)?))?;
    let (chosen_tapped, chosen_controller) =
        parse_partition_battlefield_destination(chosen_destination)?;
    let (_, remainder_destination) = primitives::parse_prefix(
        trim_lexed_commas(remainder_destination),
        primitives::kw("onto").void(),
    )?;
    let (remainder_tapped, remainder_controller) =
        parse_partition_battlefield_destination(remainder_destination)?;

    Some(TaggedBattlefieldPartitionShape {
        count,
        chosen_tapped,
        chosen_controller,
        remainder_tapped,
        remainder_controller,
    })
}

pub(crate) fn parse_tagged_into_hand_shape(tokens: &[OwnedLexToken]) -> Option<TaggedPutShape> {
    let body = strip_optional_put(tokens);
    let (into_index, _, destination) = primitives::find_prefix(body, || primitives::kw("into"))?;
    if !(primitives::contains_word(destination, "hand")
        || primitives::contains_word(destination, "hands"))
    {
        return None;
    }
    let head = trim_lexed_commas(body.get(..into_index)?);
    let count = if permission_shapes::exact_tokens_any(head, &[&["it"], &["them"]]) {
        None
    } else {
        Some(parse_count_and_reference(head)?)
    };
    Some(TaggedPutShape {
        count,
        rest_destination: parse_rest_destination(tokens),
        bottom_order: super::super::sequence_pairs::parse_bottom_order(tokens),
    })
}

pub(crate) fn parse_tagged_on_top_library_shape(
    tokens: &[OwnedLexToken],
) -> Option<TaggedTopPutShape> {
    let rest = rest_head(tokens)?;
    if !primitives::contains_word(rest, "bottom") {
        return None;
    }
    let body = strip_optional_put(tokens);
    let (on_index, _, destination) = primitives::find_prefix(body, || primitives::kw("on"))?;
    let count = parse_count_and_reference(trim_lexed_commas(body.get(..on_index)?))?;
    let (_, destination) = primitives::parse_prefix(
        trim_lexed_commas(destination),
        (
            opt(primitives::kw("the")),
            primitives::kw("top"),
            opt(primitives::kw("of")),
        )
            .void(),
    )?;
    if !(primitives::contains_word(destination, "library")
        || primitives::contains_word(destination, "libraries"))
    {
        return None;
    }
    Some(TaggedTopPutShape {
        count,
        bottom_order: super::super::sequence_pairs::parse_bottom_order(tokens)?,
    })
}

pub(crate) fn parse_from_among_them_shape(
    tokens: &[OwnedLexToken],
) -> Option<FromAmongPutShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (reference_index, _, after_reference) = primitives::find_prefix(tokens, || {
        primitives::phrase(&["from", "among", "them"]).void()
    })?;
    let choice_tokens = strip_optional_put(trim_lexed_commas(tokens.get(..reference_index)?));
    let (count, filter_tokens) =
        if let Some(parsed) = leaf::parse_leaf_choice_count_prefix_tokens(choice_tokens) {
            (
                parsed.count,
                trim_lexed_commas(choice_tokens.get(parsed.consumed..)?),
            )
        } else {
            (ChoiceCount::up_to(1), choice_tokens)
        };
    if filter_tokens.is_empty() {
        return None;
    }
    let after_reference = trim_lexed_commas(after_reference);
    let destination =
        if permission_shapes::prefix_tokens(after_reference, &["onto", "the", "battlefield"])
            || permission_shapes::prefix_tokens(after_reference, &["onto", "battlefield"])
        {
            FromAmongDestinationShape::Battlefield
        } else if primitives::contains_word(after_reference, "hand")
            || primitives::contains_word(after_reference, "hands")
        {
            FromAmongDestinationShape::Hand
        } else {
            FromAmongDestinationShape::Other
        };
    Some(FromAmongPutShape {
        count,
        filter_tokens,
        destination,
        rest_destination: parse_rest_destination(tokens),
    })
}

pub(crate) fn has_from_among_hand_surface(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, _, after_among)) =
        primitives::find_prefix(tokens, || primitives::phrase(&["from", "among"]).void())
    else {
        return false;
    };
    primitives::contains_word(after_among, "hand")
        || primitives::contains_word(after_among, "hands")
}

pub(crate) fn parse_all_exiled_into_hand_filter(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let tokens = trim_lexed_commas(tokens);
    let (_, after_put) = primitives::parse_prefix(tokens, primitives::kw("put").void())?;
    let (_, _) = primitives::parse_prefix(
        after_put,
        alt((primitives::kw("all"), primitives::kw("each"))).void(),
    )?;
    let (into_index, _, destination) =
        primitives::find_prefix(after_put, || primitives::kw("into"))?;
    let filter = trim_lexed_commas(after_put.get(..into_index)?);
    if !primitives::contains_word(filter, "exiled")
        || !(primitives::contains_word(filter, "card")
            || primitives::contains_word(filter, "cards"))
        || !(primitives::contains_word(destination, "hand")
            || primitives::contains_word(destination, "hands"))
    {
        return None;
    }
    Some(filter)
}

pub(crate) fn parse_revealed_remainder_shape(
    tokens: &[OwnedLexToken],
) -> Option<RevealedRemainderShape> {
    let is_remainder = [
        "rest", "cards", "revealed", "this", "way", "bottom", "library",
    ]
    .into_iter()
    .all(|word| primitives::contains_word(tokens, word));
    let is_full_collection =
        (permission_shapes::prefix_tokens(tokens, &["the", "revealed", "cards"])
            || permission_shapes::prefix_tokens(tokens, &["all", "revealed", "cards"])
            || permission_shapes::prefix_tokens(tokens, &["all", "the", "revealed", "cards"]))
            && primitives::contains_word(tokens, "bottom")
            && primitives::contains_word(tokens, "library")
            && !primitives::contains_word(tokens, "rest");
    if !is_remainder && !is_full_collection {
        return None;
    }
    Some(RevealedRemainderShape {
        random_order: primitives::contains_word(tokens, "random"),
        exclude_current_reference: is_remainder,
    })
}

pub(crate) fn is_reorder_tagged_cards(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "back")
        && primitives::contains_word(tokens, "any")
        && primitives::contains_word(tokens, "order")
        && (primitives::contains_word(tokens, "it") || primitives::contains_word(tokens, "them"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_tagged_and_from_among_put_shapes() {
        let tagged = lex_line(
            "put two of them into your hand and the rest on the bottom of your library",
            0,
        )
        .unwrap();
        let shape = parse_tagged_into_hand_shape(&tagged).unwrap();
        assert_eq!(
            shape.rest_destination,
            Some(RestDestinationShape::BottomOfLibrary)
        );
        assert!(shape.count.is_some());

        let any_order = lex_line(
            "put two of them into your hand and the rest on the bottom of your library in any order",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_tagged_into_hand_shape(&any_order)
                .unwrap()
                .bottom_order,
            Some(LibraryBottomOrderAst::ChooserChooses)
        );
        let random_order = lex_line(
            "put two of them into your hand and the rest on the bottom of your library in a random order",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_tagged_into_hand_shape(&random_order)
                .unwrap()
                .bottom_order,
            Some(LibraryBottomOrderAst::Random)
        );

        let top_and_bottom = lex_line(
            "put up to one of them on top of your library and the rest on the bottom in a random order",
            0,
        )
        .unwrap();
        let shape = parse_tagged_on_top_library_shape(&top_and_bottom).unwrap();
        assert_eq!(shape.count, ChoiceCount::up_to(1));
        assert_eq!(shape.bottom_order, LibraryBottomOrderAst::Random);

        let among = lex_line(
            "up to one creature card from among them onto the battlefield and the rest into your hand",
            0,
        )
        .unwrap();
        let shape = parse_from_among_them_shape(&among).unwrap();
        assert_eq!(shape.destination, FromAmongDestinationShape::Battlefield);
        assert_eq!(shape.rest_destination, Some(RestDestinationShape::Hand));
        assert!(is_reorder_tagged_cards(
            &lex_line("put them back in any order", 0).unwrap()
        ));

        let battlefield_partition = lex_line(
            "put one of those cards onto the battlefield tapped under your control and the rest onto the battlefield tapped under their control",
            0,
        )
        .unwrap();
        let shape = parse_tagged_battlefield_partition_shape(&battlefield_partition)
            .expect("tagged battlefield partition");
        assert_eq!(shape.count, ChoiceCount::exactly(1));
        assert!(shape.chosen_tapped && shape.remainder_tapped);
        assert_eq!(
            shape.chosen_controller,
            PartitionBattlefieldControllerShape::You
        );
        assert_eq!(
            shape.remainder_controller,
            PartitionBattlefieldControllerShape::SubjectPlayer
        );
    }

    #[test]
    fn parses_whole_revealed_collection_for_library_bottom_cleanup() {
        let tokens = lex_line(
            "the revealed cards on the bottom of your library in any order",
            0,
        )
        .unwrap();
        let shape = parse_revealed_remainder_shape(&tokens).expect("revealed collection");

        assert!(!shape.exclude_current_reference);
        assert!(!shape.random_order);
    }
}
