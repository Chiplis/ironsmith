use super::super::*;

use crate::effect::ChoiceCount;
use crate::filter::Comparison;
use crate::grammar::leaf;
use winnow::combinator::{alt, eof, opt, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedPermissionShape {
    PlayExiledForAsLongAsExiled,
    ManaAnyTypeCastsTaggedThisWay,
    CastSingleFromAmongHandCards,
}

pub fn parse_tagged_permission_shape(tokens: &[OwnedLexToken]) -> Option<TaggedPermissionShape> {
    let parser = alt((
        primitives::any_phrase(&[
            &[
                "play", "the", "exiled", "cards", "for", "as", "long", "as", "they", "remain",
                "exiled",
            ],
            &[
                "play", "exiled", "cards", "for", "as", "long", "as", "they", "remain", "exiled",
            ],
        ])
        .value(TaggedPermissionShape::PlayExiledForAsLongAsExiled),
        primitives::any_phrase(&[
            &[
                "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "spells", "this",
                "way",
            ],
            &[
                "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "them", "this",
                "way",
            ],
            &[
                "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "that", "spell",
                "this", "way",
            ],
        ])
        .value(TaggedPermissionShape::ManaAnyTypeCastsTaggedThisWay),
    ));
    primitives::parse_all(
        trim_lexed_commas(tokens),
        (parser, primitives::sentence_end()).map(|(shape, _)| shape),
        "tagged permission shape",
    )
    .ok()
    .or_else(|| {
        parse_cast_single_hand_shape(tokens)
            .then_some(TaggedPermissionShape::CastSingleFromAmongHandCards)
    })
}

fn parse_cast_single_hand_shape(tokens: &[OwnedLexToken]) -> bool {
    let tokens = primitives::parse_prefix(tokens, primitives::phrase(&["if", "you", "do"]))
        .map(|(_, rest)| rest)
        .unwrap_or(tokens);
    let tokens = primitives::parse_prefix(
        tokens,
        opt(alt((primitives::kw("then"), primitives::kw("and")))),
    )
    .map(|(_, rest)| rest)
    .unwrap_or(tokens);
    let tokens = primitives::parse_prefix(tokens, primitives::phrase(&["you", "may"]))
        .map(|(_, rest)| rest)
        .unwrap_or(tokens);
    primitives::parse_all(
        trim_lexed_commas(tokens),
        (
            primitives::phrase(&[
                "cast", "a", "spell", "from", "among", "those", "cards", "without", "paying",
                "its", "mana", "cost",
            ]),
            primitives::sentence_end(),
        )
            .void(),
        "single cast from among hand cards",
    )
    .is_ok()
}

fn mana_value_bound<'a>(input: &mut LexStream<'a>) -> WResult<Comparison> {
    alt((
        primitives::phrase(&["x", "or", "less"])
            .value(Comparison::LessThanOrEqualExpr(Box::new(Value::X))),
        (
            leaf::parse_leaf_number_token_lexed,
            opt(primitives::phrase(&["or", "less"])),
        )
            .map(|(value, _)| Comparison::LessThanOrEqual(value as i32)),
    ))
    .parse_next(input)
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastAnyTaggedShape {
    pub mana_value: Option<Comparison>,
}

fn cast_any_tagged<'a>(input: &mut LexStream<'a>) -> WResult<CastAnyTaggedShape> {
    opt(primitives::phrase(&["you", "may"])).parse_next(input)?;
    primitives::phrase(&["cast", "any", "number", "of", "spells"]).parse_next(input)?;
    let mana_value = opt((
        primitives::phrase(&["with", "mana", "value"]),
        mana_value_bound,
    ))
    .map(|value| value.map(|(_, bound)| bound))
    .parse_next(input)?;
    primitives::any_phrase(&[
        &["from", "among", "them"],
        &["from", "among", "those", "cards"],
    ])
    .parse_next(input)?;
    primitives::phrase(&["without", "paying", "their", "mana", "costs"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CastAnyTaggedShape { mana_value })
}

pub fn parse_cast_any_tagged_shape(tokens: &[OwnedLexToken]) -> Option<CastAnyTaggedShape> {
    primitives::parse_all(tokens, cast_any_tagged, "cast any tagged shape").ok()
}

/// A one-shot free-cast choice drawn from the exact collection established by
/// an earlier effect.  This covers the shared Oracle family:
///
/// - "cast any number of spells ... from among them"
/// - "cast up to two sorcery spells ... from among them"
/// - "cast an instant or sorcery spell ... from among them"
/// - "cast instant and sorcery spells ... from among them"
///
/// The subject and mana-value cap stay separate so permission-subject lowering
/// does not accidentally apply a trailing cap only to the final arm of a type
/// union.
#[derive(Debug, Clone, PartialEq)]
pub struct CastTaggedCollectionShape<'a> {
    pub count: ChoiceCount,
    pub subject_tokens: &'a [OwnedLexToken],
    pub mana_value: Option<Comparison>,
}

fn parse_collection_cast_count(
    tokens: &[OwnedLexToken],
) -> (Option<ChoiceCount>, &[OwnedLexToken]) {
    if let Some(((), rest)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["any", "number", "of"]))
    {
        return (Some(ChoiceCount::any_number()), rest);
    }
    if let Some((count, rest)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["up", "to"]),
            leaf::parse_leaf_number_token_lexed,
        )
            .map(|(_, count)| count),
    ) {
        return (Some(ChoiceCount::up_to(count as usize)), rest);
    }
    if let Some(((), rest)) = primitives::parse_prefix(
        tokens,
        alt((primitives::kw("a"), primitives::kw("an"))).void(),
    ) {
        return (Some(ChoiceCount::up_to(1)), rest);
    }
    (None, tokens)
}

fn split_collection_cast_mana_value(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], Option<Comparison>)> {
    let Some((bound_start, _, bound_tokens)) =
        primitives::find_prefix(tokens, || primitives::phrase(&["with", "mana", "value"]))
    else {
        return Some((trim_lexed_commas(tokens), None));
    };
    let subject_tokens = trim_lexed_commas(tokens.get(..bound_start)?);
    let mana_value = primitives::parse_all(
        trim_lexed_commas(bound_tokens),
        mana_value_bound,
        "tagged collection cast mana value",
    )
    .ok()?;
    Some((subject_tokens, Some(mana_value)))
}

pub fn parse_cast_tagged_collection_shape(
    tokens: &[OwnedLexToken],
) -> Option<CastTaggedCollectionShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (_, body) = primitives::parse_prefix(tokens, primitives::phrase(&["you", "may", "cast"]))?;
    let (authored_count, body) = parse_collection_cast_count(body);
    let (scope_start, _, free_cast_tail) = primitives::find_prefix(body, || {
        primitives::any_phrase(&[
            &["from", "among", "them"],
            &["from", "among", "those", "cards"],
            &["from", "among", "those", "exiled", "cards"],
        ])
    })?;
    primitives::parse_all(
        trim_lexed_commas(free_cast_tail),
        (
            primitives::any_phrase(&[
                &["without", "paying", "its", "mana", "cost"],
                &["without", "paying", "their", "mana", "costs"],
            ]),
            primitives::sentence_end(),
        )
            .void(),
        "tagged collection free-cast tail",
    )
    .ok()?;

    let (subject_tokens, mana_value) =
        split_collection_cast_mana_value(trim_lexed_commas(body.get(..scope_start)?))?;
    if subject_tokens.is_empty() {
        return None;
    }
    let contains_singular_spell =
        primitives::find_prefix(subject_tokens, || primitives::kw("spell")).is_some();
    let contains_plural_spells =
        primitives::find_prefix(subject_tokens, || primitives::kw("spells")).is_some();
    if !contains_singular_spell && !contains_plural_spells {
        return None;
    }
    let count = authored_count.unwrap_or_else(|| {
        if contains_plural_spells {
            ChoiceCount::any_number()
        } else {
            ChoiceCount::up_to(1)
        }
    });
    Some(CastTaggedCollectionShape {
        count,
        subject_tokens,
        mana_value,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastTargetWithoutPayingShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastTargetFromYourGraveyardThisTurnShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForEachCardPaymentShape {
    pub life_amount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpponentReturnChoiceShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterGroupRemovedShape<'a> {
    pub group_size: u32,
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForEachPreventShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub prevent_tokens: &'a [OwnedLexToken],
    pub unless_token: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrailingIfFallbackShape<'a> {
    pub head_tokens: &'a [OwnedLexToken],
    pub predicate: PredicateAst,
}

#[cfg(test)]
#[path = "permissions_inline_tests.rs"]
mod tests;

#[path = "permissions/condition_programs.rs"]
mod condition_programs;
pub use condition_programs::parse_trailing_if_fallback_shape;
#[path = "permissions/combat_programs.rs"]
mod combat_programs;
pub use combat_programs::parse_for_each_prevent_shape;
#[path = "permissions/counter_programs.rs"]
mod counter_programs;
use counter_programs::counter_group_removed;
pub use counter_programs::parse_counter_group_removed_shape;
#[path = "permissions/choice_programs.rs"]
mod choice_programs;
pub use choice_programs::parse_opponent_return_choice_shape;
#[path = "permissions/library_programs.rs"]
mod library_programs;
pub use library_programs::parse_for_each_card_payment_shape;
#[path = "permissions/reference_programs.rs"]
mod reference_programs;
pub use reference_programs::{
    parse_cast_target_from_your_graveyard_this_turn_shape, parse_cast_target_without_paying_shape,
};
