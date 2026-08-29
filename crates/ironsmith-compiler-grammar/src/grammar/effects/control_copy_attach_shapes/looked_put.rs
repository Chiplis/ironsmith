use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::effect::ChoiceCount;
use crate::grammar::{leaf, permission_shapes, primitives};
use crate::lexer::{OwnedLexToken, trim_lexed_commas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestDestinationShape {
    BottomOfLibrary,
    Graveyard,
    Hand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FromAmongDestinationShape {
    Battlefield,
    Hand,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub struct TaggedPutShape {
    pub count: Option<ChoiceCount>,
    pub plural_reference: bool,
    pub rest_destination: Option<RestDestinationShape>,
    pub bottom_order: Option<LibraryBottomOrderAst>,
}

#[derive(Debug, Clone, Copy)]
pub struct TaggedTopPutShape {
    pub count: ChoiceCount,
    pub bottom_order: LibraryBottomOrderAst,
}

#[derive(Debug, Clone, Copy)]
pub struct FromAmongPutShape<'a> {
    pub count: ChoiceCount,
    pub filter_tokens: &'a [OwnedLexToken],
    pub destination: FromAmongDestinationShape,
    pub rest_destination: Option<RestDestinationShape>,
}

#[derive(Debug, Clone, Copy)]
pub struct RevealedRemainderShape {
    pub random_order: bool,
    /// `true` for "the rest of the revealed cards", where the most recently
    /// selected card must be excluded. `false` for the whole revealed
    /// collection.
    pub exclude_current_reference: bool,
    pub surface: ironsmith_core::LibraryRemainderSurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionBattlefieldControllerShape {
    You,
    SubjectPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaggedBattlefieldPartitionShape {
    pub count: ChoiceCount,
    pub chosen_tapped: bool,
    pub chosen_controller: PartitionBattlefieldControllerShape,
    pub remainder_tapped: bool,
    pub remainder_controller: PartitionBattlefieldControllerShape,
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

pub fn parse_rest_destination(tokens: &[OwnedLexToken]) -> Option<RestDestinationShape> {
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
pub fn parse_tagged_battlefield_partition_shape(
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

pub fn parse_tagged_into_hand_shape(tokens: &[OwnedLexToken]) -> Option<TaggedPutShape> {
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
    let plural_reference = super::common::is_plural_tagged_object_reference(head)
        || count.as_ref().is_some_and(|count| {
            count.dynamic_x || count.max.is_none() || count.max.is_some_and(|maximum| maximum > 1)
        });
    Some(TaggedPutShape {
        count,
        plural_reference,
        rest_destination: parse_rest_destination(tokens),
        bottom_order: super::super::sequence_pairs::parse_bottom_order(tokens),
    })
}

#[cfg(test)]
#[path = "looked_put_inline_tests.rs"]
mod tests;

#[path = "looked_put/library_programs.rs"]
mod library_programs;
pub use library_programs::{
    is_reorder_tagged_cards, parse_revealed_remainder_shape, parse_tagged_on_top_library_shape,
};
#[path = "looked_put/reference_programs.rs"]
mod reference_programs;
pub use reference_programs::parse_all_exiled_into_hand_filter;
#[path = "looked_put/zone_programs.rs"]
mod zone_programs;
pub use zone_programs::has_from_among_hand_surface;
#[path = "looked_put/core_programs.rs"]
mod core_programs;
pub use core_programs::parse_from_among_them_shape;
