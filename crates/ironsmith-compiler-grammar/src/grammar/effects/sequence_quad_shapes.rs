use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::effect::{ChoiceCount, Value};
use crate::grammar::{leaf, primitives};
use crate::lexer::{LexStream, LexedClause, OwnedLexToken, TokenWordView};

#[derive(Debug, Clone, Copy)]
pub struct NamedRevealedCardShape<'a> {
    pub name_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountedLookedCardExileShape {
    pub count: ChoiceCount,
    pub includes_remainder: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ExiledCardCastFilterShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct LookedCardFilterShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub struct LookedCardExileRemainderShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub count: ChoiceCount,
    pub order: LibraryBottomOrderAst,
}

#[derive(Debug, Clone)]
pub struct LookedCardRevealShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub count: ChoiceCount,
    pub x_value: Option<Value>,
}

#[derive(Debug, Clone, Copy)]
pub struct LookExileSplitShape<'a> {
    pub look_tokens: &'a [OwnedLexToken],
    pub exile_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct LookedCardAndOrChoiceShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub uses_and_or: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChosenCardsDispositionShape {
    pub order: LibraryBottomOrderAst,
}

#[derive(Debug, Clone, Copy)]
pub struct ChosenCardsDestinationReplacementShape<'a> {
    pub predicate_tokens: &'a [OwnedLexToken],
    pub order: LibraryBottomOrderAst,
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    LexedClause::new(tokens).trimmed().tokens()
}

fn article<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("the"),
        primitives::kw("a"),
        primitives::kw("an"),
    ))
    .void()
    .parse_next(input)
}

fn exact_unit<'a>(
    tokens: &'a [OwnedLexToken],
    parser: fn(&mut LexStream<'a>) -> WResult<()>,
) -> bool {
    primitives::parse_prefix(trimmed(tokens), parser)
        .is_some_and(|(_, rest)| trimmed(rest).is_empty())
}

fn is_article_token(token: &OwnedLexToken) -> bool {
    exact_unit(std::slice::from_ref(token), article)
}

fn without_articles(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    tokens
        .iter()
        .filter(|token| !is_article_token(token))
        .cloned()
        .collect()
}

fn if_you_reveal<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "you", "reveal"])
        .void()
        .parse_next(input)
}

fn this_way<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["this", "way"])
        .void()
        .parse_next(input)
}

pub fn parse_named_revealed_card_shape(
    tokens: &[OwnedLexToken],
) -> Option<NamedRevealedCardShape<'_>> {
    let clause = trimmed(tokens);
    let ((), after_intro) = primitives::parse_prefix(clause, if_you_reveal)?;
    let (named_idx, (), after_named) =
        primitives::find_prefix(after_intro, || primitives::kw("named").void())?;
    let _ = named_idx;
    let (this_way_idx, (), _) = primitives::find_prefix(after_named, || this_way)?;
    let name_tokens = trimmed(&after_named[..this_way_idx]);
    (!name_tokens.is_empty()).then_some(NamedRevealedCardShape { name_tokens })
}

fn put_looked_onto_battlefield<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["put", "it", "onto", "the", "battlefield"]),
        primitives::phrase(&["put", "that", "card", "onto", "the", "battlefield"]),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_put_looked_onto_battlefield_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(trimmed(tokens), || put_looked_onto_battlefield).is_some()
}

fn put_looked_into_hand<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["put", "that", "card", "into", "your", "hand"]),
        primitives::phrase(&["put", "it", "into", "your", "hand"]),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_put_looked_into_hand_shape(tokens: &[OwnedLexToken]) -> bool {
    let mut clause = trimmed(tokens);
    if let Some(((), rest)) = primitives::parse_prefix(clause, |input: &mut LexStream<'_>| {
        primitives::kw("otherwise").void().parse_next(input)
    }) {
        clause = trimmed(rest);
    }
    primitives::parse_prefix(clause, put_looked_into_hand).is_some()
}

fn then_shuffle<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["then", "shuffle"]),
        primitives::kw("shuffle").void(),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_then_shuffle_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(tokens, then_shuffle)
}

fn exile_one_face_down<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["exile", "one", "of", "them", "face", "down"])
        .void()
        .parse_next(input)
}

fn put_rest<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["put", "rest"])
        .void()
        .parse_next(input)
}

fn bottom_of_your_library<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["bottom", "of", "your", "library"])
        .void()
        .parse_next(input)
}

pub fn parse_exile_one_and_bottom_remainder_shape(tokens: &[OwnedLexToken]) -> bool {
    let normalized = without_articles(trimmed(tokens));
    primitives::parse_prefix(&normalized, exile_one_face_down).is_some()
        && primitives::find_prefix(&normalized, || put_rest).is_some()
        && primitives::find_prefix(&normalized, || bottom_of_your_library).is_some()
}

fn counted_exile_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["of", "them", "face", "down"]),
        primitives::phrase(&["of", "those", "cards", "face", "down"]),
        primitives::phrase(&["them", "face", "down"]),
        primitives::phrase(&["those", "cards", "face", "down"]),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_counted_looked_card_exile_shape(
    tokens: &[OwnedLexToken],
) -> Option<CountedLookedCardExileShape> {
    let clause = trimmed(tokens);
    let ((), count_surface) = primitives::parse_prefix(clause, |input: &mut LexStream<'_>| {
        primitives::kw("exile").void().parse_next(input)
    })?;
    let parsed = leaf::parse_leaf_choice_count_prefix_tokens(trimmed(count_surface))?;
    let tail = trimmed(&trimmed(count_surface)[parsed.consumed..]);
    let normalized_tail = without_articles(tail);
    primitives::parse_prefix(&normalized_tail, counted_exile_tail)?;
    Some(CountedLookedCardExileShape {
        count: parsed.count,
        includes_remainder: primitives::find_prefix(&normalized_tail, || put_rest).is_some(),
    })
}

fn put_remainder_on_bottom<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["put", "rest", "on", "bottom"]),
        primitives::phrase(&["put", "rest", "onto", "bottom"]),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_looked_remainder_bottom_shape(
    tokens: &[OwnedLexToken],
) -> Option<LibraryBottomOrderAst> {
    let clause = trimmed(tokens);
    let normalized = without_articles(clause);
    primitives::find_prefix(&normalized, || put_remainder_on_bottom)?;
    primitives::find_prefix(&normalized, || primitives::kw("library").void())?;
    let words = TokenWordView::new(clause).word_refs();
    super::sequence_pairs::parse_consult_remainder_order_shape(&words)
}

fn cast_exiled_free<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "you", "may", "cast", "exiled", "card", "without", "paying", "its", "mana", "cost",
    ])
    .void()
    .parse_next(input)
}

fn exiled_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("its"),
        primitives::kw("it's"),
        primitives::kw("it"),
        primitives::kw("that"),
        primitives::kw("that's"),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_exiled_card_cast_filter_shape(
    tokens: &[OwnedLexToken],
) -> Option<ExiledCardCastFilterShape<'_>> {
    let clause = trimmed(tokens);
    let (if_idx, (), after_if) = primitives::find_prefix(clause, || primitives::kw("if").void())?;
    let prefix = without_articles(trimmed(&clause[..if_idx]));
    if !exact_unit(&prefix, cast_exiled_free) {
        return None;
    }
    let mut condition = trimmed(after_if);
    if let Some(((), rest)) = primitives::parse_prefix(condition, exiled_reference) {
        condition = trimmed(rest);
    }
    if let Some(((), rest)) = primitives::parse_prefix(condition, |input: &mut LexStream<'_>| {
        primitives::kw("card").void().parse_next(input)
    }) {
        condition = trimmed(rest);
    }
    (!condition.is_empty()).then_some(ExiledCardCastFilterShape {
        filter_tokens: condition,
    })
}

fn exiled_card_hand_followup<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "you"]).parse_next(input)?;
    alt((
        primitives::kw("don't").void(),
        primitives::kw("dont").void(),
        primitives::phrase(&["do", "not"]).void(),
    ))
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["put", "that", "card", "into", "your", "hand"])
        .void()
        .parse_next(input)
}

pub fn parse_exiled_card_hand_followup_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(tokens, exiled_card_hand_followup)
}

fn may_reveal_looked<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["you", "may", "reveal"])
        .void()
        .parse_next(input)
}

fn from_among_them<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["from", "among", "them"])
        .void()
        .parse_next(input)
}

fn choose<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("choose").void().parse_next(input)
}

/// Captures a mandatory selection from the collection established by the
/// preceding top-of-library instruction. Cardinality and the independent
/// `and/or` branches are lowered by the sequence composer; this grammar only
/// proves that the filter is scoped by the exact "from among them" suffix.
pub fn parse_choose_looked_card_and_or_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardAndOrChoiceShape<'_>> {
    let clause = trimmed(tokens);
    let ((), selection) = primitives::parse_prefix(clause, choose)?;
    let selection = trimmed(selection);
    let (among_idx, (), after_among) = primitives::find_prefix(selection, || from_among_them)?;
    if !trimmed(after_among).is_empty() {
        return None;
    }
    let filter_tokens = trimmed(&selection[..among_idx]);
    if filter_tokens.is_empty() {
        return None;
    }
    Some(LookedCardAndOrChoiceShape {
        filter_tokens,
        uses_and_or: filter_tokens.iter().any(|token| token.is_word("and/or")),
    })
}

fn put_chosen_cards_into_hand<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["put", "those", "cards", "into", "your", "hand"]),
        primitives::phrase(&["put", "the", "chosen", "cards", "into", "your", "hand"]),
    ))
    .void()
    .parse_next(input)
}

fn and_the_rest<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(primitives::comma()).parse_next(&mut *input)?;
    primitives::phrase(&["and", "the", "rest"])
        .void()
        .parse_next(input)
}

fn on_bottom_of_your_library<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["on", "the", "bottom", "of", "your", "library"])
        .void()
        .parse_next(input)
}

fn parse_chosen_cards_disposition_tail(
    tokens: &[OwnedLexToken],
) -> Option<ChosenCardsDispositionShape> {
    let ((), remainder) = primitives::parse_prefix(trimmed(tokens), and_the_rest)?;
    let remainder = trimmed(remainder);
    let ((), _) = primitives::parse_prefix(remainder, on_bottom_of_your_library)?;
    let order = super::triple_sequence_shapes::parse_consult_remainder_order_tokens(remainder)?;
    Some(ChosenCardsDispositionShape { order })
}

pub fn parse_chosen_cards_hand_remainder_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChosenCardsDispositionShape> {
    let ((), remainder) = primitives::parse_prefix(trimmed(tokens), put_chosen_cards_into_hand)?;
    parse_chosen_cards_disposition_tail(remainder)
}

#[cfg(test)]
#[path = "sequence_quad_shapes_inline_tests.rs"]
mod tests;

#[path = "sequence_quad_shapes/library_programs.rs"]
mod library_programs;
use library_programs::{
    may_exile_looked, otherwise_revealed_into_hand, put_chosen_cards_battlefield_or_hand,
    put_revealed_into_hand_then_shuffle, put_revealed_onto_battlefield,
};
pub use library_programs::{
    parse_bargained_revealed_battlefield_shape, parse_chosen_cards_destination_replacement_shape,
    parse_exile_looked_card_and_remainder_shape, parse_look_exile_split_shape,
    parse_may_exile_looked_card_shape, parse_may_reveal_looked_card_shape,
    parse_otherwise_revealed_hand_shape, parse_put_revealed_into_hand_then_shuffle_shape,
};
#[path = "sequence_quad_shapes/ability_programs.rs"]
mod ability_programs;
use ability_programs::bargained;
#[path = "sequence_quad_shapes/core_programs.rs"]
mod core_programs;
use core_programs::where_x_prefix;
