use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use crate::PtValue;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BasePowerToughnessConditionShape<'a> {
    None,
    Tokens(&'a [OwnedLexToken]),
    YourTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BasePowerToughnessShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) condition: BasePowerToughnessConditionShape<'a>,
    pub(crate) power: i32,
    pub(crate) toughness: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BasePowerToughnessGrantShape<'a> {
    pub(crate) has_token: usize,
    pub(crate) power: i32,
    pub(crate) toughness: i32,
    pub(crate) ability_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BasePowerToughnessTypeAdditionShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) power: i32,
    pub(crate) toughness: i32,
    pub(crate) addition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BasePowerGrantShape<'a> {
    pub(crate) has_token: usize,
    pub(crate) power: i32,
    pub(crate) ability_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PersistentAnthemSubjectFacts {
    pub(crate) accepted: bool,
    pub(crate) is_this_creature: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IsntCreatureShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) leading_condition_tokens: Option<&'a [OwnedLexToken]>,
    pub(crate) unless_condition_tokens: Option<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IsntCreatureShapeError {
    MissingLeadingCondition,
    MissingUnlessCondition,
}

pub(crate) fn parse_multi_subject_segments(
    tokens: &[OwnedLexToken],
) -> Option<Vec<&[OwnedLexToken]>> {
    let tokens = trim_lexed_commas(tokens);
    primitives::parse_all(
        tokens,
        parse_multi_subject_segments_lexed,
        "multi-anthem-subjects",
    )
    .ok()
}

pub(crate) fn parse_base_power_toughness_shape(
    tokens: &[OwnedLexToken],
) -> Option<BasePowerToughnessShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (condition, clause_tokens) = split_leading_base_power_toughness_condition(tokens)?;
    let mut shape = primitives::parse_all(
        clause_tokens,
        parse_base_power_toughness_lexed,
        "base-power-toughness",
    )
    .ok()?;
    shape.condition = condition;
    Some(shape)
}

pub(crate) fn parse_base_power_toughness_grant_shape(
    tokens: &[OwnedLexToken],
) -> Option<BasePowerToughnessGrantShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (parsed, rest) = primitives::parse_prefix(tokens, parse_base_power_toughness_grant_lexed)?;
    if !rest.is_empty() {
        return None;
    }
    Some(BasePowerToughnessGrantShape {
        has_token: parsed.has_token,
        power: parsed.power,
        toughness: parsed.toughness,
        ability_tokens: parsed.ability_tokens,
    })
}

pub(crate) fn parse_base_power_toughness_type_addition_shape(
    tokens: &[OwnedLexToken],
) -> Option<BasePowerToughnessTypeAdditionShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let shape = primitives::parse_all(
        tokens,
        parse_base_power_toughness_type_addition_lexed,
        "base-power/toughness type addition",
    )
    .ok()?;
    super::parse_type_color_addition_shape(shape.addition_tokens)?;
    Some(shape)
}

pub(crate) fn parse_base_power_grant_shape(
    tokens: &[OwnedLexToken],
) -> Option<BasePowerGrantShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    primitives::parse_all(tokens, parse_base_power_grant_lexed, "base-power grant").ok()
}

pub(crate) fn persistent_anthem_subject_facts(
    tokens: &[OwnedLexToken],
) -> PersistentAnthemSubjectFacts {
    let tokens = trim_lexed_commas(tokens);
    let accepted = !contains_parser(tokens, || primitives::kw("target").void())
        && !has_prefix(tokens, &["until", "end", "of", "turn"])
        && !has_prefix(tokens, &["until", "your", "next", "turn"]);
    let is_this_creature = has_prefix(tokens, &["this", "creature"]);
    PersistentAnthemSubjectFacts {
        accepted,
        is_this_creature,
    }
}

pub(crate) fn parse_isnt_creature_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<IsntCreatureShape<'_>>, IsntCreatureShapeError> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    if contains_parser(tokens, || primitives::kw("target").void())
        || contains_parser(tokens, || {
            primitives::phrase(&["until", "end", "of", "turn"])
        })
    {
        return Ok(None);
    }

    let mut leading_condition_tokens = None;
    let mut clause_tokens = tokens;
    if has_prefix(tokens, &["as", "long", "as"]) {
        let Some((_, after_prefix)) =
            primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"]))
        else {
            return Ok(None);
        };
        let Some((condition_tokens, remaining)) =
            primitives::split_lexed_once_on_separator(after_prefix, || primitives::comma().void())
        else {
            return Ok(None);
        };
        let condition_tokens = trim_lexed_commas(condition_tokens);
        if condition_tokens.is_empty() {
            return Err(IsntCreatureShapeError::MissingLeadingCondition);
        }
        leading_condition_tokens = Some(condition_tokens);
        clause_tokens = trim_lexed_commas(remaining);
    }

    let mut unless_condition_tokens = None;
    if let Some((head, condition_tokens)) =
        primitives::split_lexed_once_on_separator(clause_tokens, || primitives::kw("unless").void())
    {
        let condition_tokens = trim_lexed_commas(condition_tokens);
        if condition_tokens.is_empty() {
            return Err(IsntCreatureShapeError::MissingUnlessCondition);
        }
        unless_condition_tokens = Some(condition_tokens);
        clause_tokens = trim_lexed_commas(head);
    }

    let Ok(subject_tokens) =
        primitives::parse_all(clause_tokens, parse_isnt_creature_lexed, "isnt-creature")
    else {
        return Ok(None);
    };
    Ok(Some(IsntCreatureShape {
        subject_tokens,
        leading_condition_tokens,
        unless_condition_tokens,
    }))
}

#[derive(Debug, Clone, Copy)]
struct BaseGrantParse<'a> {
    has_token: usize,
    power: i32,
    toughness: i32,
    ability_tokens: &'a [OwnedLexToken],
}

fn parse_multi_subject_segments_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<Vec<&'a [OwnedLexToken]>> {
    let mut segments = Vec::new();
    loop {
        let segment = repeat_till(
            1..,
            any.void(),
            peek(alt((primitives::kw("and").void(), eof.value(())))),
        )
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
        let segment = trim_multi_subject_segment(segment);
        if segment.is_empty() {
            return Err(primitives::backtrack_err(
                "multi-subject anthem",
                "nonempty subject segment",
            ));
        }
        segments.push(segment);
        if input.is_empty() {
            break;
        }
        primitives::kw("and").parse_next(input)?;
    }
    if segments.len() < 2 {
        return Err(primitives::backtrack_err(
            "multi-subject anthem",
            "two or more conjoined subjects",
        ));
    }
    Ok(segments)
}

fn trim_multi_subject_segment(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    tokens = trim_lexed_commas(tokens);
    while let Some((prefix, ())) =
        primitives::split_lexed_once_before_suffix(tokens, 0, || primitives::kw("each").void())
    {
        tokens = trim_lexed_commas(prefix);
    }
    tokens
}

fn parse_base_power_toughness_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<BasePowerToughnessShape<'a>> {
    let subject_tokens = take_until_have.parse_next(input)?;
    parse_have.parse_next(input)?;
    primitives::phrase(&["base", "power", "and", "toughness"]).parse_next(input)?;
    let (power, toughness) = parse_fixed_power_toughness.parse_next(input)?;
    eof.parse_next(input)?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    if subject_tokens.is_empty() || !persistent_anthem_subject_facts(subject_tokens).accepted {
        return Err(primitives::backtrack_err(
            "base power/toughness subject",
            "persistent nontarget subject",
        ));
    }
    Ok(BasePowerToughnessShape {
        subject_tokens,
        condition: BasePowerToughnessConditionShape::None,
        power,
        toughness,
    })
}

fn split_leading_base_power_toughness_condition(
    tokens: &[OwnedLexToken],
) -> Option<(BasePowerToughnessConditionShape<'_>, &[OwnedLexToken])> {
    if let Some((_, clause_tokens)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["during", "your", "turn"]),
            primitives::comma(),
        )
            .void(),
    ) {
        let clause_tokens = trim_lexed_commas(clause_tokens);
        return (!clause_tokens.is_empty())
            .then_some((BasePowerToughnessConditionShape::YourTurn, clause_tokens));
    }

    let Some((_, after_prefix)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"]))
    else {
        return Some((BasePowerToughnessConditionShape::None, tokens));
    };
    let (condition_tokens, clause_tokens) =
        primitives::split_lexed_once_on_separator(after_prefix, || primitives::comma().void())?;
    let condition_tokens = trim_lexed_commas(condition_tokens);
    let clause_tokens = trim_lexed_commas(clause_tokens);
    if condition_tokens.is_empty() || clause_tokens.is_empty() {
        return None;
    }
    Some((
        BasePowerToughnessConditionShape::Tokens(condition_tokens),
        clause_tokens,
    ))
}

fn parse_base_power_toughness_grant_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<BaseGrantParse<'a>> {
    let initial_len = input.len();
    let _subject_tokens = take_until_have.parse_next(input)?;
    parse_have.parse_next(input)?;
    let has_token = initial_len.saturating_sub(input.len() + 1);
    primitives::phrase(&["base", "power", "and", "toughness"]).parse_next(input)?;
    let (power, toughness) = parse_fixed_power_toughness.parse_next(input)?;
    // Oracle lists may join the base-P/T predicate to the first granted
    // ability with either "and has" or a comma followed by "has".
    alt((primitives::kw("and").void(), primitives::comma().void())).parse_next(input)?;
    alt((
        primitives::kw("have"),
        primitives::kw("has"),
        primitives::kw("gain"),
        primitives::kw("gains"),
    ))
    .parse_next(input)?;
    let ability_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    let ability_tokens = trim_lexed_commas(ability_tokens);
    if ability_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "base power/toughness grant",
            "nonempty granted ability",
        ));
    }
    Ok(BaseGrantParse {
        has_token,
        power,
        toughness,
        ability_tokens,
    })
}

fn parse_base_power_toughness_type_addition_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<BasePowerToughnessTypeAdditionShape<'a>> {
    let subject_tokens = take_until_have.parse_next(input)?;
    parse_have.parse_next(input)?;
    primitives::phrase(&["base", "power", "and", "toughness"]).parse_next(input)?;
    let (power, toughness) = parse_fixed_power_toughness.parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let addition_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    let addition_tokens = trim_lexed_commas(addition_tokens);
    if subject_tokens.is_empty()
        || addition_tokens.is_empty()
        || !persistent_anthem_subject_facts(subject_tokens).accepted
    {
        return Err(primitives::backtrack_err(
            "base power/toughness type addition",
            "persistent subject and additive type predicate",
        ));
    }
    Ok(BasePowerToughnessTypeAdditionShape {
        subject_tokens,
        power,
        toughness,
        addition_tokens,
    })
}

fn parse_base_power_grant_lexed<'a>(input: &mut LexStream<'a>) -> WResult<BasePowerGrantShape<'a>> {
    let initial_len = input.len();
    let _subject_tokens = take_until_have.parse_next(input)?;
    parse_have.parse_next(input)?;
    let has_token = initial_len.saturating_sub(input.len() + 1);
    primitives::phrase(&["base", "power"]).parse_next(input)?;
    let raw = primitives::word_parser_text.parse_next(input)?;
    let power = leaf::parse_number_i32_complete(raw)
        .map_err(|_| primitives::backtrack_err("base power", "fixed signed power"))?;
    primitives::kw("and").parse_next(input)?;
    alt((
        primitives::kw("have"),
        primitives::kw("has"),
        primitives::kw("gain"),
        primitives::kw("gains"),
    ))
    .parse_next(input)?;
    let ability_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    let ability_tokens = trim_lexed_commas(ability_tokens);
    if ability_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "base-power grant",
            "nonempty granted ability",
        ));
    }
    Ok(BasePowerGrantShape {
        has_token,
        power,
        ability_tokens,
    })
}

fn parse_isnt_creature_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let subject_tokens = repeat_till(1.., any.void(), peek(parse_negated_creature_tail))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    parse_negated_creature_tail.parse_next(input)?;
    eof.parse_next(input)?;
    Ok(trim_lexed_commas(subject_tokens))
}

fn parse_negated_creature_tail(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        alt((primitives::kw("isnt"), primitives::kw("isn't"))).void(),
        (
            alt((primitives::kw("is"), primitives::kw("are"))),
            primitives::kw("not"),
        )
            .void(),
        (
            alt((primitives::kw("is"), primitives::kw("are"))),
            primitives::phrase(&["no", "longer"]),
        )
            .void(),
    ))
    .parse_next(input)?;
    winnow::combinator::opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures")))
        .void()
        .parse_next(input)
}

fn take_until_have<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(parse_have))
        .map(|((), _)| ())
        .take()
        .parse_next(input)
}

fn parse_have(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("has"), primitives::kw("have")))
        .void()
        .parse_next(input)
}

fn parse_fixed_power_toughness(input: &mut LexStream<'_>) -> WResult<(i32, i32)> {
    let raw = primitives::word_parser_text.parse_next(input)?;
    let parsed = leaf::parse_leaf_power_toughness_complete(raw).map_err(|_| {
        primitives::backtrack_err("base power/toughness", "fixed power/toughness value")
    })?;
    match (parsed.power, parsed.toughness) {
        (PtValue::Fixed(power), PtValue::Fixed(toughness)) => Ok((power, toughness)),
        _ => Err(primitives::backtrack_err(
            "base power/toughness",
            "fixed numeric power/toughness value",
        )),
    }
}

fn contains_parser<'a, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    let mut input = LexStream::new(tokens);
    loop {
        let mut candidate = input.clone();
        if make_parser().parse_next(&mut candidate).is_ok() {
            return true;
        }
        if take_token(&mut input).is_err() {
            return false;
        }
    }
}

fn take_token<'a>(input: &mut LexStream<'a>) -> WResult<&'a OwnedLexToken> {
    any.parse_next(input)
}

fn has_prefix(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(words)).is_some()
}

#[cfg(test)]
#[path = "tail_static_shapes_tests.rs"]
mod tests;
