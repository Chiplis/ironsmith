use super::super::*;

use crate::runtime_backend::front_end::grammar::leaf;
use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult;
use winnow::prelude::*;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TaggedSharesCardTypeConditionShape<'a> {
    pub(crate) effect_tokens: &'a [OwnedLexToken],
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

pub(crate) fn parse_tagged_shares_card_type_condition_tokens(
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
pub(crate) struct CopularAnimationShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) animation_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_copular_animation_shape(
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
    let fixed_pt_animation = animation_body
        .first()
        .and_then(|token| leaf::parse_leaf_pt_modifier_values_complete(token.parser_text()).ok())
        .is_some_and(|(power, toughness)| {
            matches!((power, toughness), (Value::Fixed(_), Value::Fixed(_)))
                && primitives::find_prefix(animation_tokens, || {
                    alt((primitives::kw("creature"), primitives::kw("creatures")))
                })
                .is_some()
                && primitives::find_prefix(animation_tokens, || {
                    primitives::phrase(&["in", "addition", "to"])
                })
                .is_some()
        });
    let descriptor_words = parser_token_word_refs(animation_body);
    let simple_descriptor = !matches!(
        become_shapes::parse_become_simple_descriptor_words(&descriptor_words),
        become_shapes::BecomeSimpleDescriptorShape::None
    );
    if !fixed_pt_animation && !simple_descriptor {
        return None;
    }
    let subject_tokens = trim_lexed_commas(subject_tokens);
    (!subject_tokens.is_empty()).then_some(CopularAnimationShape {
        subject_tokens,
        animation_tokens: trim_lexed_commas(animation_tokens),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PassiveSacrificeShape<'a> {
    pub(crate) object_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_passive_sacrifice_shape(
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
pub(crate) enum GoadTargetShape<'a> {
    TaggedToken,
    Target(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PassiveGoadShape<'a> {
    pub(crate) target: GoadTargetShape<'a>,
}

pub(crate) fn parse_passive_goad_shape(tokens: &[OwnedLexToken]) -> Option<PassiveGoadShape<'_>> {
    let (subject_tokens, tail_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("is").void())?;
    primitives::parse_all(
        trim_lexed_commas(tail_tokens),
        (
            alt((primitives::kw("goaded"), primitives::kw("goad"))),
            opt(primitives::any_phrase(&[
                &["for", "the", "rest", "of", "the", "game"],
                &["for", "the", "rest", "of", "this", "game"],
            ])),
            primitives::sentence_end(),
        )
            .void(),
        "passive goad shape",
    )
    .ok()?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    if subject_tokens.is_empty() {
        return None;
    }
    let tagged = primitives::parse_all(
        subject_tokens,
        (
            primitives::any_phrase(&[&["the", "token"], &["the", "tokens"]]),
            primitives::sentence_end(),
        )
            .void(),
        "goad token reference",
    )
    .is_ok();
    Some(PassiveGoadShape {
        target: if tagged {
            GoadTargetShape::TaggedToken
        } else {
            GoadTargetShape::Target(subject_tokens)
        },
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HexproofTargetingOverrideShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_hexproof_targeting_override_shape(
    tokens: &[OwnedLexToken],
) -> Option<HexproofTargetingOverrideShape<'_>> {
    primitives::find_prefix(tokens, || {
        primitives::any_phrase(&[
            &["as", "though", "they", "didnt", "have", "hexproof"],
            &["as", "though", "they", "didn't", "have", "hexproof"],
        ])
    })?;
    let (creatures, _, after_creatures) =
        primitives::find_prefix(tokens, || primitives::kw("creatures"))?;
    let (can_relative, _, _) = primitives::find_prefix(after_creatures, || {
        primitives::phrase(&["can", "be", "the", "targets"])
    })?;
    let can = creatures + 1 + can_relative;
    let filter_tokens = trim_lexed_commas(tokens.get(creatures..can)?);
    (!filter_tokens.is_empty()).then_some(HexproofTargetingOverrideShape { filter_tokens })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ControlPlayerShape<'a> {
    pub(crate) player: PlayerAst,
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) duration_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_control_player_shape(
    tokens: &[OwnedLexToken],
) -> Option<ControlPlayerShape<'_>> {
    let (control, _, after_control) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("control"), primitives::kw("controls")))
    })?;
    if control == 0 {
        return None;
    }
    let subject_tokens = tokens.get(..control)?;
    let (_, player, _) = primitives::find_prefix(subject_tokens, || {
        alt((
            primitives::kw("you").value(PlayerAst::You),
            primitives::phrase(&["that", "player"]).value(PlayerAst::That),
            primitives::phrase(&["target", "player"]).value(PlayerAst::Target),
            primitives::phrase(&["each", "opponent"]).value(PlayerAst::Opponent),
        ))
    })?;
    let (during, _, _) = primitives::find_prefix(after_control, || primitives::kw("during"))?;
    if during == 0 {
        return None;
    }
    let target_tokens = trim_lexed_commas(after_control.get(..during)?);
    let duration_start = control + 1 + during;
    Some(ControlPlayerShape {
        player,
        target_tokens,
        duration_tokens: tokens.get(duration_start..)?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiscardedThisWayModifierShape {
    pub(crate) power: i32,
    pub(crate) toughness: i32,
}

pub(crate) fn parse_discarded_this_way_modifier_shape(
    tokens: &[OwnedLexToken],
) -> Option<DiscardedThisWayModifierShape> {
    let first = tokens.first()?.parser_text();
    let (power, toughness) = leaf::parse_leaf_pt_modifier_values_complete(first).ok()?;
    let (Value::Fixed(power), Value::Fixed(toughness)) = (power, toughness) else {
        return None;
    };
    primitives::parse_all(
        tokens.get(1..)?,
        (
            primitives::phrase(&[
                "until",
                "end",
                "of",
                "turn",
                "for",
                "each",
                "card",
                "discarded",
                "this",
                "way",
            ]),
            primitives::sentence_end(),
        )
            .void(),
        "discarded this way modifier",
    )
    .ok()?;
    Some(DiscardedThisWayModifierShape { power, toughness })
}

pub(crate) fn parse_modifier_duration_for_each_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let after_modifier = tokens.get(1..)?;
    let (_, rest) = primitives::parse_prefix(
        after_modifier,
        primitives::phrase(&["until", "end", "of", "turn"]),
    )?;
    primitives::parse_prefix(rest, primitives::phrase(&["for", "each"]))?;
    Some(rest)
}

pub(crate) fn is_pronoun_library_choice_put_shape(tokens: &[OwnedLexToken]) -> bool {
    let pronoun =
        primitives::parse_prefix(tokens, alt((primitives::kw("it"), primitives::kw("them"))))
            .is_some();
    pronoun
        && ["on", "choice", "top", "bottom", "library"]
            .into_iter()
            .all(|word| primitives::find_prefix(tokens, || primitives::kw(word)).is_some())
}

#[cfg(test)]
#[path = "relational/tests.rs"]
mod tests;
