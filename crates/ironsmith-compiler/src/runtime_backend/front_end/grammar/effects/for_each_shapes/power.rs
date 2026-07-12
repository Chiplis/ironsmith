use winnow::combinator::alt;
use winnow::prelude::*;

use crate::cards::builders::CardTextError;
use crate::effect::Value;
use crate::runtime_backend::front_end::grammar::{leaf, primitives};
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, render_token_slice};
use crate::runtime_backend::front_end::shared::util::trim_edge_punctuation_tokens;

#[derive(Debug, Clone)]
pub(crate) struct BasePowerClauseShape<'a> {
    pub(crate) power: Value,
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub(crate) struct BasePowerToughnessClauseShape<'a> {
    pub(crate) power: Value,
    pub(crate) toughness: Value,
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

fn has_or_have<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    alt((primitives::kw("has"), primitives::kw("have")))
        .void()
        .parse_next(input)
}

fn eot_prefix(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let parsed = leaf::parse_leaf_restriction_duration_prefix_tokens(tokens)?;
    (parsed.duration == leaf::LeafDurationPhrase::UntilEndOfTurn)
        .then_some(trim_edge_punctuation_tokens(parsed.rest))
}

fn is_eot_tail(tokens: &[OwnedLexToken]) -> bool {
    eot_prefix(tokens).is_some_and(<[_]>::is_empty)
}

fn contains_temporal_marker(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["this", "turn"]),
            primitives::phrase(&["next", "turn"]),
            primitives::phrase(&["until", "end", "of", "turn"]),
            primitives::phrase(&["until", "the", "end", "of", "turn"]),
        ))
        .void()
    })
    .is_some()
}

fn permits_implicit_eot(subject: &[OwnedLexToken], all: &[OwnedLexToken]) -> bool {
    primitives::contains_word(subject, "target")
        || eot_prefix(subject).is_some()
        || contains_temporal_marker(all)
}

fn target_without_leading_eot(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    eot_prefix(tokens).unwrap_or_else(|| trim_edge_punctuation_tokens(tokens))
}

fn has_shared_gain_tail(tokens: &[OwnedLexToken]) -> bool {
    let tokens = eot_prefix(tokens).unwrap_or(tokens);
    primitives::parse_prefix(
        trim_edge_punctuation_tokens(tokens),
        (
            primitives::kw("and"),
            alt((
                primitives::kw("gain"),
                primitives::kw("gains"),
                primitives::kw("lose"),
                primitives::kw("loses"),
                primitives::kw("has"),
                primitives::kw("have"),
                primitives::kw("get"),
                primitives::kw("gets"),
            )),
        )
            .void(),
    )
    .is_some()
}

fn split_subject_and_rest(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let (has_index, _, rest) = primitives::find_prefix(tokens, || has_or_have)?;
    let subject = trim_edge_punctuation_tokens(tokens.get(..has_index)?);
    (!subject.is_empty()).then_some((subject, trim_edge_punctuation_tokens(rest)))
}

pub(crate) fn parse_base_power_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<BasePowerClauseShape<'_>>, CardTextError> {
    let Some((subject, rest)) = split_subject_and_rest(tokens) else {
        return Ok(None);
    };
    let Some((_, value_tokens)) = primitives::parse_prefix(
        rest,
        (primitives::kw("base"), primitives::kw("power")).void(),
    ) else {
        return Ok(None);
    };
    if primitives::parse_prefix(value_tokens, primitives::kw("and")).is_some() {
        return Ok(None);
    }
    let Some(parsed) = leaf::parse_leaf_number_or_x_prefix_tokens(value_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "invalid base power value (clause: '{}')",
            render_token_slice(tokens)
        )));
    };
    let Some((power, consumed)) = parsed.into_value() else {
        return Err(CardTextError::ParseError(format!(
            "invalid base power value (clause: '{}')",
            render_token_slice(tokens)
        )));
    };
    let tail = trim_edge_punctuation_tokens(value_tokens.get(consumed..).unwrap_or_default());
    if tail.is_empty() {
        if !permits_implicit_eot(subject, tokens) {
            return Ok(None);
        }
    } else if !is_eot_tail(tail) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power clause (clause: '{}')",
            render_token_slice(tokens)
        )));
    }
    Ok(Some(BasePowerClauseShape {
        power,
        target_tokens: target_without_leading_eot(subject),
    }))
}

pub(crate) fn parse_base_power_toughness_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<BasePowerToughnessClauseShape<'_>>, CardTextError> {
    let Some((subject, rest)) = split_subject_and_rest(tokens) else {
        return Ok(None);
    };
    let Some((_, modifier_tokens)) = primitives::parse_prefix(
        rest,
        primitives::phrase(&["base", "power", "and", "toughness"]),
    ) else {
        return Ok(None);
    };
    let Some(modifier_token) = modifier_tokens.first() else {
        return Ok(None);
    };
    let (power, toughness) = leaf::parse_leaf_pt_modifier_values_complete(
        modifier_token.parser_text(),
    )
    .map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            render_token_slice(tokens)
        ))
    })?;
    let tail = trim_edge_punctuation_tokens(modifier_tokens.get(1..).unwrap_or_default());
    if tail.is_empty() {
        if !permits_implicit_eot(subject, tokens) {
            return Ok(None);
        }
    } else if has_shared_gain_tail(tail) {
        return Ok(None);
    } else if !is_eot_tail(tail) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power/toughness clause (clause: '{}')",
            render_token_slice(tokens)
        )));
    }
    Ok(Some(BasePowerToughnessClauseShape {
        power,
        toughness,
        target_tokens: target_without_leading_eot(subject),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_typed_base_characteristics() {
        let power = lex_line("until end of turn, target creature has base power X", 0).unwrap();
        assert_eq!(
            parse_base_power_clause_shape(&power)
                .unwrap()
                .unwrap()
                .power,
            Value::X
        );

        let pt = lex_line("target creature has base power and toughness 3/4", 0).unwrap();
        let shape = parse_base_power_toughness_clause_shape(&pt)
            .unwrap()
            .unwrap();
        assert_eq!(shape.power, Value::Fixed(3));
        assert_eq!(shape.toughness, Value::Fixed(4));
    }
}
