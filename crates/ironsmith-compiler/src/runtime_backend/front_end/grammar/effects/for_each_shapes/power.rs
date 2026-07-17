use winnow::combinator::alt;
use winnow::prelude::*;

use crate::cards::builders::CardTextError;
use crate::effect::{Until, Value};
use crate::runtime_backend::front_end::grammar::{leaf, primitives};
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, render_token_slice};
use crate::runtime_backend::front_end::shared::util::trim_edge_punctuation_tokens;

#[derive(Debug, Clone)]
pub(crate) struct BasePowerClauseShape<'a> {
    pub(crate) power: Value,
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) duration: Until,
}

#[derive(Debug, Clone)]
pub(crate) struct BasePowerToughnessClauseShape<'a> {
    pub(crate) power: Value,
    pub(crate) toughness: Value,
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) duration: Until,
    pub(crate) where_x_tokens: Option<&'a [OwnedLexToken]>,
}

fn has_or_have<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    alt((primitives::kw("has"), primitives::kw("have")))
        .void()
        .parse_next(input)
}

fn duration_from_leaf(duration: leaf::LeafDurationPhrase) -> Until {
    match duration {
        leaf::LeafDurationPhrase::ThisTurn | leaf::LeafDurationPhrase::UntilEndOfTurn => {
            Until::EndOfTurn
        }
        leaf::LeafDurationPhrase::UntilEndOfCombat => Until::EndOfCombat,
        leaf::LeafDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
        leaf::LeafDurationPhrase::UntilYourNextTurnEnd => Until::YourNextTurnEnd,
        leaf::LeafDurationPhrase::UntilYourNextUpkeep => Until::YourNextUpkeep,
        leaf::LeafDurationPhrase::ControllersNextUntapStep => Until::ControllersNextUntapStep,
        leaf::LeafDurationPhrase::Forever => Until::Forever,
    }
}

fn duration_prefix(tokens: &[OwnedLexToken]) -> Option<(Until, &[OwnedLexToken])> {
    let parsed = leaf::parse_leaf_restriction_duration_prefix_tokens(tokens)?;
    Some((
        duration_from_leaf(parsed.duration),
        trim_edge_punctuation_tokens(parsed.rest),
    ))
}

fn complete_duration(tokens: &[OwnedLexToken]) -> Option<Until> {
    let (duration, rest) = duration_prefix(tokens)?;
    rest.is_empty().then_some(duration)
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
        || duration_prefix(subject).is_some()
        || contains_temporal_marker(all)
}

fn target_and_leading_duration(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], Option<Until>) {
    duration_prefix(tokens)
        .map(|(duration, rest)| (rest, Some(duration)))
        .unwrap_or_else(|| (trim_edge_punctuation_tokens(tokens), None))
}

fn has_shared_gain_tail(tokens: &[OwnedLexToken]) -> bool {
    let tokens = duration_prefix(tokens)
        .map(|(_, rest)| rest)
        .unwrap_or(tokens);
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
    let (target_tokens, leading_duration) = target_and_leading_duration(subject);
    let duration = if tail.is_empty() {
        if !permits_implicit_eot(subject, tokens) {
            return Ok(None);
        }
        leading_duration.unwrap_or(Until::EndOfTurn)
    } else if let Some(trailing_duration) = complete_duration(tail) {
        if leading_duration
            .as_ref()
            .is_some_and(|leading| leading != &trailing_duration)
        {
            return Err(CardTextError::ParseError(format!(
                "conflicting base power durations (clause: '{}')",
                render_token_slice(tokens)
            )));
        }
        trailing_duration
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power clause (clause: '{}')",
            render_token_slice(tokens)
        )));
    };
    Ok(Some(BasePowerClauseShape {
        power,
        target_tokens,
        duration,
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
    let (target_tokens, leading_duration) = target_and_leading_duration(subject);
    let mut where_x_tokens = None;
    let duration = if tail.is_empty() {
        if !permits_implicit_eot(subject, tokens) {
            return Ok(None);
        }
        leading_duration.unwrap_or(Until::EndOfTurn)
    } else if has_shared_gain_tail(tail) {
        return Ok(None);
    } else if primitives::parse_prefix(tail, primitives::phrase(&["where", "x", "is"])).is_some() {
        if !permits_implicit_eot(subject, tokens) {
            return Ok(None);
        }
        where_x_tokens = Some(tail);
        leading_duration.unwrap_or(Until::EndOfTurn)
    } else if let Some(trailing_duration) = complete_duration(tail) {
        if leading_duration
            .as_ref()
            .is_some_and(|leading| leading != &trailing_duration)
        {
            return Err(CardTextError::ParseError(format!(
                "conflicting base power/toughness durations (clause: '{}')",
                render_token_slice(tokens)
            )));
        }
        trailing_duration
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power/toughness clause (clause: '{}')",
            render_token_slice(tokens)
        )));
    };
    Ok(Some(BasePowerToughnessClauseShape {
        power,
        toughness,
        target_tokens,
        duration,
        where_x_tokens,
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
        assert_eq!(shape.duration, Until::EndOfTurn);
        assert!(shape.where_x_tokens.is_none());

        let pt = lex_line(
            "until your next turn, creatures target player controls have base power and toughness 1/1",
            0,
        )
        .unwrap();
        let shape = parse_base_power_toughness_clause_shape(&pt)
            .unwrap()
            .unwrap();
        assert_eq!(shape.duration, Until::YourNextTurn);
        assert!(shape.where_x_tokens.is_none());
        assert_eq!(
            render_token_slice(shape.target_tokens),
            "creatures target player controls"
        );

        let dynamic = lex_line(
            "until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards in your graveyard",
            0,
        )
        .unwrap();
        let shape = parse_base_power_toughness_clause_shape(&dynamic)
            .unwrap()
            .unwrap();
        assert_eq!(shape.power, Value::X);
        assert_eq!(shape.toughness, Value::X);
        assert_eq!(shape.duration, Until::EndOfTurn);
        assert_eq!(
            render_token_slice(shape.where_x_tokens.unwrap()),
            "where X is the number of cards in your graveyard"
        );
    }
}
