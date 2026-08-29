use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;
use crate::object::CounterType;

use super::super::super::lexer::{LexStream, OwnedLexToken, render_token_slice};
use super::super::{filters, primitives};
use super::ActivationCostSegmentCst;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CounterClauseShape {
    descriptor_first: usize,
    descriptor_end: usize,
    target_first: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemovalQuantity {
    Fixed(u32),
    DynamicX,
    All,
    AnyNumber,
    OneOrMore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RemovalDescriptorShape {
    quantity: RemovalQuantity,
    counter_first: usize,
}

pub fn parse_optional_activation_counter_type_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<CounterType>, CardTextError> {
    if tokens.is_empty()
        || primitives::parse_all(
            tokens,
            parse_generic_counter_descriptor_lexed,
            "generic-counter-descriptor",
        )
        .is_ok()
    {
        return Ok(None);
    }
    filters::parse_counter_type_from_tokens(tokens)
        .map(Some)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "rewrite counter parser could not determine counter type from '{}'",
                render_token_slice(tokens).trim().to_ascii_lowercase()
            ))
        })
}

pub fn parse_put_counter_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    if primitives::parse_all(
        tokens,
        parse_move_opponent_exiled_card_lexed,
        "move-opponent-exiled-card-cost",
    )
    .is_ok()
    {
        return Ok(ActivationCostSegmentCst::MoveOpponentOwnedExiledCardToGraveyard);
    }
    let shape = primitives::parse_all(tokens, parse_put_counter_clause_lexed, "put-counter-cost")
        .map_err(|_| unsupported(tokens, "put-counter"))?;
    let descriptor = &tokens[shape.descriptor_first..shape.descriptor_end];
    let (count, counter_tokens) = parse_fixed_counter_descriptor(descriptor);
    let counter_type = filters::parse_counter_type_from_tokens(counter_tokens)
        .ok_or_else(|| unsupported(counter_tokens, "counter-type"))?;
    let target = &tokens[shape.target_first..];
    if primitives::parse_all(target, parse_put_counter_source_lexed, "put-counter-source").is_ok() {
        Ok(ActivationCostSegmentCst::PutCounters {
            counter_type,
            count,
        })
    } else {
        Ok(ActivationCostSegmentCst::PutCountersChosen {
            counter_type,
            count,
            filter: filters::parse_object_filter_with_grammar_entrypoint_lexed(target, false)?,
        })
    }
}

pub fn parse_remove_counter_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let clause = primitives::parse_all(
        tokens,
        parse_remove_counter_clause_lexed,
        "remove-counter-cost",
    )
    .map_err(|_| unsupported(tokens, "remove-counter"))?;
    let descriptor = &tokens[clause.descriptor_first..clause.descriptor_end];
    let parsed = primitives::parse_all(
        descriptor,
        parse_removal_descriptor_lexed,
        "counter-removal-descriptor",
    )
    .map_err(|_| unsupported(descriptor, "counter-removal-descriptor"))?;
    let counter_tokens = &descriptor[parsed.counter_first..];
    let counter_type = parse_optional_activation_counter_type_tokens(counter_tokens)?;

    let target = &tokens[clause.target_first..];
    let (target_among, filter_tokens) = if let Some(((), rest)) =
        primitives::parse_prefix(target, primitives::kw("among").void())
    {
        (true, rest)
    } else {
        (false, target)
    };
    let filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(filter_tokens, false)?;
    let (count, display_x, dynamic, remove_all) = match parsed.quantity {
        RemovalQuantity::Fixed(count) => (count, false, false, false),
        RemovalQuantity::DynamicX => (0, true, true, false),
        RemovalQuantity::All => (0, false, true, true),
        RemovalQuantity::AnyNumber => (0, false, true, false),
        RemovalQuantity::OneOrMore => (1, false, true, false),
    };

    let source_target = primitives::parse_all(
        target,
        parse_remove_counter_source_lexed,
        "remove-counter-source",
    )
    .is_ok();
    if dynamic {
        return if !target_among && source_target && count == 0 {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x,
                remove_all,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count,
                filter,
                display_x,
                dynamic: true,
                single_object: !target_among,
            })
        };
    }

    if !target_among
        && source_target
        && let Some(counter_type) = counter_type
    {
        return Ok(ActivationCostSegmentCst::RemoveCounters {
            counter_type,
            count,
        });
    }
    Ok(ActivationCostSegmentCst::RemoveCountersAmong {
        counter_type,
        count,
        filter,
        display_x: false,
        dynamic: false,
        single_object: !target_among,
    })
}

fn unsupported(tokens: &[OwnedLexToken], label: &str) -> CardTextError {
    CardTextError::ParseError(format!(
        "rewrite {label} parser does not yet support '{}'",
        render_token_slice(tokens).trim().to_ascii_lowercase()
    ))
}

fn parse_fixed_counter_descriptor(tokens: &[OwnedLexToken]) -> (u32, &[OwnedLexToken]) {
    if let Some((count, rest)) =
        primitives::parse_prefix(tokens, super::super::leaf::parse_leaf_number_prefix_lexed)
    {
        (count, rest)
    } else {
        (1, tokens)
    }
}

fn parse_put_counter_clause_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CounterClauseShape> {
    parse_counter_clause_lexed(input, "put", "on")
}

fn parse_remove_counter_clause_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CounterClauseShape> {
    parse_counter_clause_lexed(input, "remove", "from")
}

fn parse_counter_clause_lexed<'a>(
    input: &mut LexStream<'a>,
    verb: &'static str,
    separator: &'static str,
) -> WResult<CounterClauseShape> {
    let initial_len = input.len();
    primitives::kw(verb).parse_next(input)?;
    let descriptor_first = initial_len.saturating_sub(input.len());
    let mut descriptor_end = descriptor_first;
    loop {
        let mut boundary = input.clone();
        if primitives::kw(separator).parse_next(&mut boundary).is_ok() {
            if descriptor_end == descriptor_first {
                return Err(primitives::backtrack_err(
                    "counter descriptor",
                    "descriptor before counter target separator",
                ));
            }
            *input = boundary;
            break;
        }
        any.parse_next(input)?;
        descriptor_end += 1;
    }
    let target_first = initial_len.saturating_sub(input.len());
    let targets: Vec<&OwnedLexToken> = repeat(1.., any).parse_next(input)?;
    if targets.is_empty() {
        return Err(primitives::backtrack_err(
            "counter target",
            "target after counter separator",
        ));
    }
    Ok(CounterClauseShape {
        descriptor_first,
        descriptor_end,
        target_first,
    })
}

fn parse_removal_descriptor_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RemovalDescriptorShape> {
    let initial_len = input.len();
    let quantity = alt((
        primitives::phrase(&["any", "number", "of"]).value(RemovalQuantity::AnyNumber),
        primitives::phrase(&["one", "or", "more"]).value(RemovalQuantity::OneOrMore),
        primitives::kw("x").value(RemovalQuantity::DynamicX),
        primitives::kw("all").value(RemovalQuantity::All),
        super::super::leaf::parse_leaf_number_prefix_lexed.map(RemovalQuantity::Fixed),
    ))
    .parse_next(input)
    .unwrap_or(RemovalQuantity::Fixed(1));
    let counter_first = initial_len.saturating_sub(input.len());
    let _: Vec<&OwnedLexToken> = repeat(0.., any).parse_next(input)?;
    Ok(RemovalDescriptorShape {
        quantity,
        counter_first,
    })
}

fn parse_move_opponent_exiled_card_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&[
            "put",
            "a",
            "card",
            "an",
            "opponent",
            "owns",
            "from",
            "exile",
            "into",
            "that",
            "players",
            "graveyard",
        ]),
        primitives::phrase(&[
            "put",
            "a",
            "card",
            "an",
            "opponent",
            "owns",
            "from",
            "exile",
            "into",
            "that",
            "player's",
            "graveyard",
        ]),
    ))
    .void()
    .parse_next(input)
}

fn parse_put_counter_source_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    parse_this_source(false).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(())
}

fn parse_remove_counter_source_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    parse_this_source(true).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(())
}

fn parse_this_source<'a>(
    allow_it: bool,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        if allow_it && primitives::kw("it").parse_next(input).is_ok() {
            return Ok(());
        }
        primitives::kw("this").parse_next(input)?;
        opt(alt((
            primitives::kw("creature"),
            primitives::kw("permanent"),
            primitives::kw("artifact"),
            primitives::kw("aura"),
            primitives::kw("enchantment"),
            primitives::kw("card"),
            primitives::kw("land"),
        )))
        .void()
        .parse_next(input)
    }
}

fn parse_generic_counter_descriptor_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("counter"), primitives::kw("counters")))
        .void()
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn optional_counter_type_distinguishes_generic_and_typed_descriptors() {
        let generic = lex_line("counters", 0).unwrap();
        assert_eq!(
            parse_optional_activation_counter_type_tokens(&generic).unwrap(),
            None
        );
        let typed = lex_line("loyalty counters", 0).unwrap();
        assert_eq!(
            parse_optional_activation_counter_type_tokens(&typed).unwrap(),
            Some(CounterType::Loyalty)
        );
    }

    #[test]
    fn put_and_remove_counter_costs_return_typed_segments() {
        let put = lex_line("put a +1/+1 counter on this creature", 0).unwrap();
        assert_eq!(
            parse_put_counter_segment_tokens(&put).unwrap(),
            ActivationCostSegmentCst::PutCounters {
                counter_type: CounterType::PlusOnePlusOne,
                count: 1,
            }
        );

        let remove = lex_line("remove two loyalty counters from this permanent", 0).unwrap();
        assert_eq!(
            parse_remove_counter_segment_tokens(&remove).unwrap(),
            ActivationCostSegmentCst::RemoveCounters {
                counter_type: CounterType::Loyalty,
                count: 2,
            }
        );

        let among = lex_line("remove x loyalty counters from among creatures", 0).unwrap();
        assert_eq!(
            parse_remove_counter_segment_tokens(&among).unwrap(),
            ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type: Some(CounterType::Loyalty),
                count: 0,
                filter: crate::target::ObjectFilter::creature(),
                display_x: true,
                dynamic: true,
                single_object: false,
            }
        );

        let chosen_put = lex_line("put a -1/-1 counter on a creature you control", 0).unwrap();
        assert!(matches!(
            parse_put_counter_segment_tokens(&chosen_put).unwrap(),
            ActivationCostSegmentCst::PutCountersChosen {
                counter_type: CounterType::MinusOneMinusOne,
                count: 1,
                filter,
            } if filter.card_types == [crate::types::CardType::Creature]
                && filter.controller == Some(crate::target::PlayerFilter::You)
        ));

        let chosen_dynamic = lex_line(
            "remove X counters from an artifact or creature you control",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_remove_counter_segment_tokens(&chosen_dynamic).unwrap(),
            ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type: None,
                count: 0,
                filter,
                display_x: true,
                dynamic: true,
                single_object: true,
            } if filter.card_types
                == [crate::types::CardType::Artifact, crate::types::CardType::Creature]
                && filter.controller == Some(crate::target::PlayerFilter::You)
        ));

        let explicit_permanent_types = lex_line(
            "remove a counter from an artifact, creature, land, or planeswalker you control",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_remove_counter_segment_tokens(&explicit_permanent_types).unwrap(),
            ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type: None,
                count: 1,
                filter,
                display_x: false,
                dynamic: false,
                single_object: true,
            } if filter.any_of.is_empty()
                && filter.card_types
                    == [
                        crate::types::CardType::Artifact,
                        crate::types::CardType::Creature,
                        crate::types::CardType::Land,
                        crate::types::CardType::Planeswalker,
                    ]
                && filter.controller == Some(crate::target::PlayerFilter::You)
        ));
    }
}
