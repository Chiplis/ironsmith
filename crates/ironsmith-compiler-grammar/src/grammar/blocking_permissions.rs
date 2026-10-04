//! Blocking capacity clauses, independent of individual blocking restrictions.
use crate::grammar::{abilities, leaf, primitives};
use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use winnow::Parser;
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult;
use winnow::token::{any, rest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockingCapacity {
    AnyNumber,
    Additional(u32),
}

#[derive(Debug, Clone, Copy)]
pub struct BlockingCapacityClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub capacity: BlockingCapacity,
    pub this_turn: bool,
    pub for_each_tokens: Option<&'a [OwnedLexToken]>,
}

pub fn parse_blocking_capacity(tokens: &[OwnedLexToken]) -> Option<BlockingCapacityClause<'_>> {
    primitives::probe_all(tokens, parse_lexed, "blocking capacity")
}

fn parse_lexed<'a>(input: &mut LexStream<'a>) -> ModalResult<BlockingCapacityClause<'a>> {
    let subject_tokens = repeat_till(0.., any.void(), peek(primitives::phrase(&["can", "block"])))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    // A capacity leaf must not consume an earlier instruction or a compound
    // grant. Shared-subject sequencing owns those and supplies a clean clause.
    if subject_tokens.iter().any(|token| {
        token.is_period()
            || token.is_comma()
            || token.is_any_word(&["gets", "get", "has", "have", "gains", "gain", "and"])
    }) {
        return Err(primitives::backtrack_err(
            "blocking capacity",
            "clean subject",
        ));
    }
    primitives::phrase(&["can", "block"]).parse_next(input)?;
    let capacity = alt((
        primitives::phrase(&["any", "number", "of", "creatures"])
            .value(BlockingCapacity::AnyNumber),
        parse_additional_lexed.map(BlockingCapacity::Additional),
    ))
    .parse_next(input)?;
    let this_turn = opt(primitives::phrase(&["this", "turn"]))
        .parse_next(input)?
        .is_some();
    if !this_turn {
        opt(primitives::phrase(&["each", "combat"])).parse_next(input)?;
    }
    let for_each_tokens = if opt(primitives::phrase(&["for", "each"]))
        .parse_next(input)?
        .is_some()
    {
        let tail: &'a [OwnedLexToken] = rest.parse_next(input)?;
        let tail = if tail.last().is_some_and(OwnedLexToken::is_period) {
            &tail[..tail.len() - 1]
        } else {
            tail
        };
        if tail.is_empty()
            || tail.iter().any(OwnedLexToken::is_period)
            || capacity == BlockingCapacity::AnyNumber
        {
            return Err(primitives::backtrack_err(
                "blocking capacity",
                "a complete counted permanent filter",
            ));
        }
        Some(tail)
    } else {
        None
    };
    opt(primitives::sentence_end()).parse_next(input)?;
    Ok(BlockingCapacityClause {
        subject_tokens,
        capacity,
        this_turn,
        for_each_tokens,
    })
}

fn parse_additional_lexed<'a>(input: &mut LexStream<'a>) -> ModalResult<u32> {
    let count = alt((
        (
            opt(alt((primitives::kw("a"), primitives::kw("an")))),
            primitives::kw("additional"),
            opt(leaf::parse_leaf_number_prefix_lexed),
        )
            .map(|(_, _, count)| count.unwrap_or(1)),
        (
            leaf::parse_leaf_number_prefix_lexed,
            primitives::kw("additional"),
        )
            .map(|(count, _)| count),
    ))
    .parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures"))).parse_next(input)?;
    Ok(count)
}

#[derive(Debug, Clone, Copy)]
pub struct BlockingCapacityLine<'a> {
    pub clause: BlockingCapacityClause<'a>,
    pub condition_tokens: Option<&'a [OwnedLexToken]>,
    /// Complete preceding static instruction(s), sharing the capacity subject.
    pub preceding_tokens: Option<&'a [OwnedLexToken]>,
}

/// Read a source/filter/attachment capacity sentence, optionally conditioned or
/// conjoined after a pump/keyword grant. Recognition does not parse predicates.
pub fn parse_blocking_capacity_line(tokens: &[OwnedLexToken]) -> Option<BlockingCapacityLine<'_>> {
    let leading = abilities::split_as_long_as_condition_prefix_lexed(tokens);
    let body = leading
        .as_ref()
        .map_or(tokens, |shape| shape.remainder_tokens);
    let (body, trailing) = match primitives::split_lexed_once_on_separator(body, || {
        primitives::phrase(&["as", "long", "as"])
    }) {
        Some((body, condition)) => (body, Some(condition)),
        None => (body, None),
    };
    if leading.is_some() && trailing.is_some() {
        return None;
    }
    let condition_tokens = leading
        .as_ref()
        .map(|shape| shape.condition_tokens)
        .or(trailing);
    if let Some(clause) = parse_blocking_capacity(body) {
        return Some(BlockingCapacityLine {
            clause,
            condition_tokens,
            preceding_tokens: None,
        });
    }
    let (preceding_body, capacity_tail) = primitives::split_lexed_once_on_separator(body, || {
        (
            primitives::kw("and"),
            peek(primitives::phrase(&["can", "block"])),
        )
            .void()
    })?;
    let mut clause = parse_blocking_capacity(capacity_tail)?;
    if !clause.subject_tokens.is_empty() {
        return None;
    }
    let (subject, _) = primitives::split_lexed_once_on_separator(preceding_body, || {
        alt((
            primitives::kw("get"),
            primitives::kw("gets"),
            primitives::kw("has"),
            primitives::kw("have"),
        ))
        .void()
    })?;
    clause.subject_tokens = trim_lexed_commas(subject);
    if clause.subject_tokens.is_empty() {
        return None;
    }
    // Include the leading condition in the predecessor so both siblings keep
    // it through the ordinary static reader. A trailing condition on a compound
    // line is deliberately deferred until that shared-scope shape is supported.
    if trailing.is_some() {
        return None;
    }
    let prefix_len = tokens.len() - body.len() + preceding_body.len();
    let preceding_tokens = trim_lexed_commas(&tokens[..prefix_len]);
    Some(BlockingCapacityLine {
        clause,
        condition_tokens,
        preceding_tokens: Some(preceding_tokens),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn blocking_capacity_retains_scope_duration_and_exact_cardinals() {
        for (text, capacity, temporary) in [
            (
                "This creature can block any number of creatures.",
                BlockingCapacity::AnyNumber,
                false,
            ),
            (
                "Enchanted creature can block any number of creatures.",
                BlockingCapacity::AnyNumber,
                false,
            ),
            (
                "Target creature can block any number of creatures this turn.",
                BlockingCapacity::AnyNumber,
                true,
            ),
            (
                "can block an additional creature this turn",
                BlockingCapacity::Additional(1),
                true,
            ),
            (
                "This creature can block an additional ninety-nine creatures each combat.",
                BlockingCapacity::Additional(99),
                false,
            ),
            (
                "Equipped creature can block two additional creatures each combat.",
                BlockingCapacity::Additional(2),
                false,
            ),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let shape = parse_blocking_capacity(&tokens).expect(text);
            assert_eq!(shape.this_turn, temporary);
            assert_eq!(shape.capacity, capacity);
        }
        for text in [
            "Target creature gets +2/+6 until end of turn and can block any number of creatures this turn.",
            "This creature can block any number of creatures with flying.",
            "This creature can block any number of creatures as long as you're the monarch.",
            "Target creature can block any number of creatures until your next turn.",
            "This creature can block several additional creatures each combat.",
        ] {
            assert!(
                parse_blocking_capacity(&lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
    }

    #[test]
    fn conditional_capacity_carries_the_original_subject_and_preceding_grant() {
        for text in [
            "This creature can block an additional creature each combat as long as you're the monarch.",
            "As long as this artifact is a creature, it can block an additional creature each combat.",
            "As long as this creature is monstrous, it has reach and can block an additional ninety-nine creatures each combat.",
            "Enchanted creature gets +2/+2, has vigilance, and can block an additional creature each combat.",
        ] {
            assert!(
                parse_blocking_capacity_line(&lex_line(text, 0).unwrap()).is_some(),
                "{text}"
            );
        }
    }
}
