use super::super::*;

use crate::grammar::leaf;
use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderedChooseAllShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub repeated_filter_tokens: &'a [OwnedLexToken],
}

/// Parse "choose <objects> one at a time until each <object> has been chosen".
/// Both object descriptions are retained so semantic lowering can validate
/// that the stopping condition names the same set as the choice itself.
pub fn parse_ordered_choose_all_shape(
    tokens: &[OwnedLexToken],
) -> Option<OrderedChooseAllShape<'_>> {
    let (_, after_choose) =
        primitives::parse_prefix(trim_lexed_commas(tokens), primitives::kw("choose"))?;
    let (separator, _, after_separator) = primitives::find_prefix(after_choose, || {
        primitives::phrase(&["one", "at", "a", "time", "until", "each"])
    })?;
    let filter_tokens = trim_lexed_commas(after_choose.get(..separator)?);
    let (repeated_filter_tokens, ()) =
        primitives::split_lexed_once_before_suffix(after_separator, 1, || {
            (
                alt((primitives::kw("has"), primitives::kw("have"))),
                primitives::phrase(&["been", "chosen"]),
                primitives::sentence_end(),
            )
                .void()
        })?;
    let repeated_filter_tokens = trim_lexed_commas(repeated_filter_tokens);
    if filter_tokens.is_empty() || repeated_filter_tokens.is_empty() {
        return None;
    }
    Some(OrderedChooseAllShape {
        filter_tokens,
        repeated_filter_tokens,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaggedSharesCardTypeConditionShape<'a> {
    pub effect_tokens: &'a [OwnedLexToken],
}

fn tagged_shares_card_type_condition<'a>(
    input: &mut LexStream<'a>,
) -> ModalResult<TaggedSharesCardTypeConditionShape<'a>> {
    primitives::phrase(&["if", "any", "of", "those", "cards"]).parse_next(input)?;
    alt((primitives::kw("share"), primitives::kw("shares"))).parse_next(input)?;
    primitives::phrase(&["a", "card", "type", "with", "that", "spell"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let effect_tokens = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(TaggedSharesCardTypeConditionShape {
        effect_tokens: trim_lexed_commas(effect_tokens),
    })
}

pub fn parse_tagged_shares_card_type_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TaggedSharesCardTypeConditionShape<'_>> {
    primitives::parse_all(
        tokens,
        tagged_shares_card_type_condition,
        "tagged cards share card type with triggering spell",
    )
    .ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopularAnimationShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub animation_tokens: &'a [OwnedLexToken],
}

pub fn parse_copular_animation_shape(
    tokens: &[OwnedLexToken],
) -> Option<CopularAnimationShape<'_>> {
    let (subject_tokens, animation_tokens) = if tokens
        .first()
        .is_some_and(|token| token.is_word("it's") || token.is_word("its") || token.is_word("it’s"))
    {
        (tokens.get(..1)?, tokens.get(1..)?)
    } else {
        let (copula, _, animation_tokens) = primitives::find_prefix(tokens, || {
            alt((primitives::kw("is"), primitives::kw("are")))
        })?;
        if copula == 0 {
            return None;
        }
        (tokens.get(..copula)?, animation_tokens)
    };
    let animation_body = leaf::parse_leaf_leading_indefinite_article_tokens(animation_tokens).rest;
    let descriptor_words = parser_token_word_refs(animation_body);
    let omitted_creature_subtype_animation =
        become_shapes::parse_become_leading_pt_shape(&descriptor_words, animation_body)
            .is_some_and(|shape| {
                if shape.creature_word_index.is_some() {
                    return false;
                }
                let (descriptor, preserves_other_types) =
                    become_shapes::strip_become_addition_tail_words(
                        &descriptor_words[shape.value_word_count..],
                    );
                preserves_other_types
                    && become_shapes::parse_become_creature_descriptor_words(descriptor).is_some()
            });
    let fixed_pt_animation = animation_body
        .first()
        .and_then(|token| leaf::parse_leaf_pt_modifier_values_complete(token.parser_text()).ok())
        .is_some_and(|(power, toughness)| {
            matches!((power, toughness), (Value::Fixed(_), Value::Fixed(_)))
                && (primitives::find_prefix(animation_tokens, || {
                    alt((primitives::kw("creature"), primitives::kw("creatures")))
                })
                .is_some()
                    || omitted_creature_subtype_animation)
                && primitives::find_prefix(animation_tokens, || {
                    primitives::phrase(&["in", "addition", "to"])
                })
                .is_some()
        });
    let simple_descriptor = !matches!(
        become_shapes::parse_become_simple_descriptor_words(&descriptor_words),
        become_shapes::BecomeSimpleDescriptorShape::None
    );
    let (subtype_descriptor, subtype_preserves_other_types) =
        become_shapes::strip_become_addition_tail_words(&descriptor_words);
    let subtype_addition = subtype_preserves_other_types
        && become_shapes::parse_become_creature_descriptor_words(subtype_descriptor).is_some();
    if !fixed_pt_animation && !simple_descriptor && !subtype_addition {
        return None;
    }
    let subject_tokens = trim_lexed_commas(subject_tokens);
    (!subject_tokens.is_empty()).then_some(CopularAnimationShape {
        subject_tokens,
        animation_tokens: trim_lexed_commas(animation_tokens),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveSacrificeShape<'a> {
    pub object_tokens: &'a [OwnedLexToken],
}

pub fn parse_passive_sacrifice_shape(
    tokens: &[OwnedLexToken],
) -> Option<PassiveSacrificeShape<'_>> {
    let (subject_tokens, ()) = primitives::split_lexed_once_before_suffix(tokens, 2, || {
        (
            alt((primitives::kw("is"), primitives::kw("are"))),
            primitives::kw("sacrificed"),
            primitives::any_phrase(&[
                &["by", "its", "controller"],
                &["by", "their", "controller"],
                &["by", "their", "controllers"],
            ]),
            primitives::sentence_end(),
        )
            .void()
    })?;
    let (_, object_tokens) = primitives::parse_prefix(
        subject_tokens,
        alt((primitives::kw("each"), primitives::kw("all"))),
    )?;
    let object_tokens = trim_lexed_commas(object_tokens);
    (!object_tokens.is_empty()).then_some(PassiveSacrificeShape { object_tokens })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoadTargetShape<'a> {
    TaggedToken,
    Target(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveGoadShape<'a> {
    pub target: GoadTargetShape<'a>,
    pub for_rest_of_game: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexproofTargetingOverrideShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlPlayerShape<'a> {
    pub player: PlayerAst,
    pub target_tokens: &'a [OwnedLexToken],
    pub duration_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscardedThisWayModifierShape {
    pub power: i32,
    pub toughness: i32,
}

#[cfg(test)]
#[path = "relational/tests.rs"]
mod tests;

#[path = "relational/library.rs"]
mod library_programs;
pub use library_programs::{
    is_pronoun_library_choice_put_shape, parse_discarded_this_way_modifier_shape,
};
#[path = "relational/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::parse_modifier_duration_for_each_tokens;
#[path = "relational/reference.rs"]
mod reference_programs;
pub use reference_programs::{parse_control_player_shape, parse_hexproof_targeting_override_shape};
#[path = "relational/core.rs"]
mod core_programs;
pub use core_programs::parse_passive_goad_shape;
