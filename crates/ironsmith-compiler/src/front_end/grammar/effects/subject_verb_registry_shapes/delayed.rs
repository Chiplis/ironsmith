use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::{permission_shapes, primitives};
use crate::lexer::{LexStream, OwnedLexToken};
use crate::util::trim_edge_punctuation_tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistryDelayedAction {
    Sacrifice,
    Exile,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RegistryNextEndStepShape<'a> {
    pub(crate) action: RegistryDelayedAction,
    pub(crate) object_tokens: &'a [OwnedLexToken],
    pub(crate) your_end_step: bool,
    /// A condition that is evaluated when the delayed instruction resolves,
    /// such as "if it has mana value 3 or less". Keeping this separate from
    /// the object phrase prevents the registry route from silently consuming
    /// and discarding a behavior-bearing suffix.
    pub(crate) trailing_tokens: &'a [OwnedLexToken],
}

fn delayed_action<'a>(input: &mut LexStream<'a>) -> WResult<RegistryDelayedAction> {
    alt((
        primitives::kw("sacrifice").value(RegistryDelayedAction::Sacrifice),
        primitives::kw("exile").value(RegistryDelayedAction::Exile),
    ))
    .parse_next(input)
}

fn next_end_step<'a>(input: &mut LexStream<'a>) -> WResult<RegistryNextEndStepShape<'a>> {
    let action = delayed_action.parse_next(input)?;
    let object_tokens = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["at", "the", "beginning", "of"])),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["at", "the", "beginning", "of"]).parse_next(input)?;
    let owner = opt(alt((
        primitives::kw("the").value(false),
        primitives::kw("your").value(true),
    )))
    .parse_next(input)?;
    primitives::phrase(&["next", "end", "step"]).parse_next(input)?;
    let trailing_tokens = repeat_till(0.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RegistryNextEndStepShape {
        action,
        object_tokens: trim_edge_punctuation_tokens(object_tokens),
        your_end_step: owner == Some(true),
        trailing_tokens: trim_edge_punctuation_tokens(trailing_tokens),
    })
}

pub(crate) fn parse_registry_next_end_step_shape(
    tokens: &[OwnedLexToken],
) -> Option<RegistryNextEndStepShape<'_>> {
    primitives::parse_all(tokens, next_end_step, "registry-next-end-step").ok()
}

pub(crate) fn parse_remain_exiled_tail(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["if", "any", "of", "those", "cards", "remain", "exiled"]),
            primitives::phrase(&["if", "those", "cards", "remain", "exiled"]),
            primitives::phrase(&["if", "that", "card", "remains", "exiled"]),
            primitives::phrase(&["if", "it", "remains", "exiled"]),
        ))
        .void(),
    )?;
    Some(trim_edge_punctuation_tokens(rest))
}

pub(crate) fn is_tagged_delayed_object(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::exact_tokens_any(
        tokens,
        &[
            &["it"],
            &["them"],
            &["the", "creature"],
            &["that", "creature"],
            &["the", "permanent"],
            &["that", "permanent"],
            &["the", "token"],
            &["that", "token"],
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_next_end_step_and_remain_exiled_shapes() {
        let delayed = lex_line(
            "Sacrifice that creature at the beginning of your next end step.",
            0,
        )
        .unwrap();
        let shape = parse_registry_next_end_step_shape(&delayed).unwrap();
        assert_eq!(shape.action, RegistryDelayedAction::Sacrifice);
        assert!(shape.your_end_step);
        assert!(is_tagged_delayed_object(shape.object_tokens));
        assert!(shape.trailing_tokens.is_empty());

        let conditional = lex_line(
            "Sacrifice it at the beginning of the next end step if it has mana value 3 or less.",
            0,
        )
        .unwrap();
        let shape = parse_registry_next_end_step_shape(&conditional).unwrap();
        assert!(is_tagged_delayed_object(shape.object_tokens));
        assert_eq!(
            shape
                .trailing_tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .collect::<Vec<_>>(),
            vec!["if", "it", "has", "mana", "value", "3", "or", "less"]
        );

        let remain = lex_line("If that card remains exiled, draw a card.", 0).unwrap();
        assert!(parse_remain_exiled_tail(&remain).is_some());
    }
}
