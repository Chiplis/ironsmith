use super::super::*;

use crate::filter::Comparison;
use crate::runtime_backend::front_end::grammar::leaf;
use winnow::combinator::{alt, eof, opt, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaggedPermissionShape {
    PlayExiledForAsLongAsExiled,
    ManaAnyTypeCastsTaggedThisWay,
    CastSingleFromAmongHandCards,
}

pub(crate) fn parse_tagged_permission_shape(
    tokens: &[OwnedLexToken],
) -> Option<TaggedPermissionShape> {
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
pub(crate) struct CastAnyTaggedShape {
    pub(crate) mana_value: Option<Comparison>,
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

pub(crate) fn parse_cast_any_tagged_shape(tokens: &[OwnedLexToken]) -> Option<CastAnyTaggedShape> {
    primitives::parse_all(tokens, cast_any_tagged, "cast any tagged shape").ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CastTargetWithoutPayingShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_cast_target_without_paying_shape(
    tokens: &[OwnedLexToken],
) -> Option<CastTargetWithoutPayingShape<'_>> {
    let (head, ()) = primitives::split_lexed_once_before_suffix(tokens, 2, || {
        (
            primitives::phrase(&["without", "paying", "its", "mana", "cost"]),
            primitives::sentence_end(),
        )
            .void()
    })?;
    let (_, target_tokens) = primitives::parse_prefix(head, primitives::kw("cast"))?;
    primitives::parse_prefix(target_tokens, primitives::kw("target"))?;
    Some(CastTargetWithoutPayingShape {
        target_tokens: trim_lexed_commas(target_tokens),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CastTargetFromYourGraveyardThisTurnShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_cast_target_from_your_graveyard_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<CastTargetFromYourGraveyardThisTurnShape<'_>> {
    let (_, rest) = primitives::parse_prefix(
        trim_lexed_commas(tokens),
        alt((
            primitives::phrase(&["you", "may", "cast"]),
            primitives::kw("cast").void(),
        )),
    )?;
    let (target_tokens, ()) = primitives::split_lexed_once_before_suffix(rest, 2, || {
        (
            primitives::phrase(&["this", "turn"]),
            primitives::sentence_end(),
        )
            .void()
    })?;
    let target_tokens = trim_lexed_commas(target_tokens);
    let (_, target_body) = primitives::parse_prefix(target_tokens, primitives::kw("target"))?;
    if target_body.is_empty() {
        return None;
    }
    primitives::split_lexed_once_before_suffix(target_tokens, 2, || {
        (primitives::phrase(&["from", "your", "graveyard"]), eof).void()
    })?;
    Some(CastTargetFromYourGraveyardThisTurnShape { target_tokens })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ForEachCardPaymentShape {
    pub(crate) life_amount: u32,
}

pub(crate) fn parse_for_each_card_payment_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachCardPaymentShape> {
    let (_, body) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["for", "each", "of", "those", "cards"]),
    )?;
    let (_, _, after_pay) = primitives::find_prefix(body, || primitives::kw("pay"))?;
    let (life_amount, after_amount) =
        primitives::parse_prefix(after_pay, leaf::parse_leaf_number_token_lexed)?;
    primitives::parse_all(
        trim_lexed_commas(after_amount),
        (
            primitives::phrase(&[
                "or", "put", "the", "card", "on", "top", "of", "your", "library",
            ]),
            primitives::sentence_end(),
        )
            .void(),
        "for each card payment tail",
    )
    .ok()?;
    Some(ForEachCardPaymentShape { life_amount })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpponentReturnChoiceShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_opponent_return_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<OpponentReturnChoiceShape<'_>> {
    let (_, choice_tail) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["for", "each", "opponent", "choose"]),
    )?;
    let (then_start, _, after_then_return) =
        primitives::find_prefix(choice_tail, || primitives::phrase(&["then", "return"]))?;
    let (_, _, after_unless) =
        primitives::find_prefix(after_then_return, || primitives::kw("unless"))?;
    primitives::parse_prefix(
        after_unless,
        primitives::phrase(&["its", "controller", "has", "you", "draw", "a", "card"]),
    )?;
    let target_tokens = trim_lexed_commas(choice_tail.get(..then_start)?);
    (!target_tokens.is_empty()).then_some(OpponentReturnChoiceShape { target_tokens })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CounterGroupRemovedShape<'a> {
    pub(crate) group_size: u32,
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

fn counter_group_removed<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    opt(primitives::kw("for")).parse_next(input)?;
    primitives::kw("each").parse_next(input)?;
    let group_size = leaf::parse_leaf_number_token_lexed.parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        (
            alt((primitives::kw("counter"), primitives::kw("counters"))),
            primitives::phrase(&["removed", "this", "way"]),
        )
            .void(),
    )
    .parse_next(input)?;
    Ok(group_size)
}

pub(crate) fn parse_counter_group_removed_shape(
    tokens: &[OwnedLexToken],
) -> Option<CounterGroupRemovedShape<'_>> {
    let (group_size, effect_tokens) = primitives::parse_prefix(tokens, counter_group_removed)?;
    Some(CounterGroupRemovedShape {
        group_size,
        effect_tokens: trim_lexed_commas(effect_tokens),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ForEachPreventShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) prevent_tokens: &'a [OwnedLexToken],
    pub(crate) unless_token: Option<usize>,
}

pub(crate) fn parse_for_each_prevent_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachPreventShape<'_>> {
    let (prevent_token, _, after_prevent) =
        primitives::find_prefix(tokens, || primitives::kw("prevent"))?;
    let subject_tokens = trim_lexed_commas(tokens.get(..prevent_token)?);
    let unless = primitives::find_prefix(after_prevent, || primitives::kw("unless"));
    let (prevent_tokens, unless_token) = if let Some((relative, _, _)) = unless {
        (
            trim_lexed_commas(tokens.get(prevent_token..prevent_token + 1 + relative)?),
            Some(prevent_token + 1 + relative),
        )
    } else {
        (trim_lexed_commas(tokens.get(prevent_token..)?), None)
    };
    Some(ForEachPreventShape {
        subject_tokens,
        prevent_tokens,
        unless_token,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrailingIfFallbackShape<'a> {
    pub(crate) head_tokens: &'a [OwnedLexToken],
    pub(crate) predicate: PredicateAst,
}

pub(crate) fn parse_trailing_if_fallback_shape(
    tokens: &[OwnedLexToken],
) -> Option<TrailingIfFallbackShape<'_>> {
    let mut offset = 1usize;
    let mut found = None;
    while offset < tokens.len() {
        let Some((relative, _, _)) =
            primitives::find_prefix(tokens.get(offset..)?, || primitives::kw("if"))
        else {
            break;
        };
        let split = offset + relative;
        if let Some(predicate) =
            crate::runtime_backend::grammar::structure::parse_trailing_if_predicate_lexed(
                tokens.get(split..)?,
            )
        {
            found = Some(TrailingIfFallbackShape {
                head_tokens: trim_lexed_commas(tokens.get(..split)?),
                predicate,
            });
        }
        offset = split + 1;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::TokenWordView;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_cast_and_counter_group_shapes() {
        let cast = lex_line(
            "You may cast any number of spells with mana value X or less from among those cards without paying their mana costs.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_cast_any_tagged_shape(&cast).unwrap().mana_value,
            Some(Comparison::LessThanOrEqualExpr(_))
        ));
        let group = lex_line("For each two counters removed this way, draw a card.", 0).unwrap();
        assert_eq!(
            parse_counter_group_removed_shape(&group)
                .unwrap()
                .group_size,
            2
        );
    }

    #[test]
    fn parses_target_from_your_graveyard_this_turn_permission() {
        let tokens = lex_line(
            "You may cast target Zombie creature card from your graveyard this turn.",
            0,
        )
        .unwrap();
        let shape = parse_cast_target_from_your_graveyard_this_turn_shape(&tokens)
            .expect("targeted graveyard permission");

        assert_eq!(
            TokenWordView::new(shape.target_tokens).to_word_refs(),
            vec![
                "target",
                "zombie",
                "creature",
                "card",
                "from",
                "your",
                "graveyard"
            ]
        );
        let stripped = lex_line(
            "cast target Zombie creature card from your graveyard this turn.",
            0,
        )
        .unwrap();
        let stripped_shape = parse_cast_target_from_your_graveyard_this_turn_shape(&stripped)
            .expect("leading-may chain should route its stripped cast clause");
        assert_eq!(
            TokenWordView::new(stripped_shape.target_tokens).to_word_refs(),
            TokenWordView::new(shape.target_tokens).to_word_refs(),
        );
        let wrong_zone = lex_line(
            "You may cast target Zombie creature card from exile this turn.",
            0,
        )
        .unwrap();
        assert!(parse_cast_target_from_your_graveyard_this_turn_shape(&wrong_zone).is_none());
    }
}
