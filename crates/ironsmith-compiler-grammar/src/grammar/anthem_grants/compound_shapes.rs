use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::target::ObjectFilter;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{conditions, filters, leaf, primitives, structure};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarriedSubjectTypeAdditionShape<'a> {
    pub first_sentence_tokens: &'a [OwnedLexToken],
    pub subject_tokens: &'a [OwnedLexToken],
    pub addition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalAnthemReplacementShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub base_power: i32,
    pub base_toughness: i32,
    pub condition_filter: ObjectFilter,
    pub replacement_power: i32,
    pub replacement_toughness: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalAnthemOtherwiseShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub condition_filter: ObjectFilter,
    pub true_power: i32,
    pub true_toughness: i32,
    pub false_power: i32,
    pub false_toughness: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CarriedConditionalAnthemGrantShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub base_power: i32,
    pub base_toughness: i32,
    pub additional_power: i32,
    pub additional_toughness: i32,
    pub ability_tokens: &'a [OwnedLexToken],
    pub condition: conditions::ObjectAttachedToObjectConditionAst,
}

pub fn parse_carried_subject_type_addition(
    tokens: &[OwnedLexToken],
) -> Option<CarriedSubjectTypeAdditionShape<'_>> {
    let sentences = structure::split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    let [first_sentence_tokens, addition_tokens] = sentences.as_slice() else {
        return None;
    };
    let (subject_tokens, _) =
        primitives::split_lexed_once_on_separator(first_sentence_tokens, || {
            alt((primitives::kw("get"), primitives::kw("gets"))).void()
        })?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    if subject_tokens.is_empty()
        || super::parse_type_color_addition_shape(addition_tokens).is_none()
    {
        return None;
    }
    Some(CarriedSubjectTypeAdditionShape {
        first_sentence_tokens,
        subject_tokens,
        addition_tokens,
    })
}

pub fn parse_conditional_anthem_replacement(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalAnthemReplacementShape<'_>> {
    let sentences = two_sentences(tokens)?;
    let first = parse_fixed_anthem_sentence(sentences.0, false)?;
    let (condition_filter, replacement_power, replacement_toughness) =
        parse_if_replacement_sentence(sentences.1)?;
    Some(ConditionalAnthemReplacementShape {
        subject_tokens: first.subject_tokens,
        base_power: first.power,
        base_toughness: first.toughness,
        condition_filter,
        replacement_power,
        replacement_toughness,
    })
}

pub fn parse_conditional_anthem_otherwise(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalAnthemOtherwiseShape<'_>> {
    let sentences = two_sentences(tokens)?;
    let first = parse_fixed_anthem_sentence(sentences.0, true)?;
    let condition_filter = parse_attached_reference_condition(first.condition_tokens?)?;
    let (false_power, false_toughness) = parse_otherwise_anthem_sentence(sentences.1)?;
    Some(ConditionalAnthemOtherwiseShape {
        subject_tokens: first.subject_tokens,
        condition_filter,
        true_power: first.power,
        true_toughness: first.toughness,
        false_power,
        false_toughness,
    })
}

pub fn parse_carried_conditional_anthem_grant(
    tokens: &[OwnedLexToken],
) -> Option<CarriedConditionalAnthemGrantShape<'_>> {
    let sentences = two_sentences(tokens)?;
    let first = parse_fixed_anthem_sentence(sentences.0, false)?;
    let (_, continuation) = primitives::parse_prefix(
        sentences.1,
        (
            primitives::kw("it"),
            alt((primitives::kw("get"), primitives::kw("gets"))),
        ),
    )?;
    let (modifier_tokens, granted_tokens) =
        primitives::split_lexed_once_on_separator(continuation, || {
            (
                primitives::kw("and"),
                alt((primitives::kw("has"), primitives::kw("have"))),
            )
                .void()
        })?;
    let (additional_power, additional_toughness) = parse_fixed_modifier(modifier_tokens)?;
    let (ability_tokens, condition_tokens) =
        primitives::split_lexed_once_on_separator(granted_tokens, || {
            primitives::phrase(&["as", "long", "as"])
        })?;
    let ability_tokens = super::trim_anthem_clause_tokens(ability_tokens);
    let condition = conditions::parse_object_attached_to_object_condition(condition_tokens)?;
    if ability_tokens.is_empty() {
        return None;
    }
    Some(CarriedConditionalAnthemGrantShape {
        subject_tokens: first.subject_tokens,
        base_power: first.power,
        base_toughness: first.toughness,
        additional_power,
        additional_toughness,
        ability_tokens,
        condition,
    })
}

#[derive(Debug, Clone, Copy)]
struct FixedAnthemSentence<'a> {
    subject_tokens: &'a [OwnedLexToken],
    power: i32,
    toughness: i32,
    condition_tokens: Option<&'a [OwnedLexToken]>,
}

fn two_sentences(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let sentences = structure::split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    let [first, second] = sentences.as_slice() else {
        return None;
    };
    Some((first, second))
}

fn parse_fixed_anthem_sentence(
    tokens: &[OwnedLexToken],
    require_condition: bool,
) -> Option<FixedAnthemSentence<'_>> {
    let (subject_tokens, tail) = primitives::split_lexed_once_on_separator(tokens, || {
        alt((primitives::kw("get"), primitives::kw("gets"))).void()
    })?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    let (modifier_tokens, condition_tokens) =
        match primitives::split_lexed_once_on_separator(tail, || {
            primitives::phrase(&["as", "long", "as"])
        }) {
            Some((modifier, condition)) => {
                (modifier, Some(super::trim_anthem_clause_tokens(condition)))
            }
            None => (tail, None),
        };
    if subject_tokens.is_empty()
        || require_condition != condition_tokens.is_some()
        || condition_tokens.is_some_and(|condition| condition.is_empty())
    {
        return None;
    }
    let (power, toughness) = parse_fixed_modifier(modifier_tokens)?;
    Some(FixedAnthemSentence {
        subject_tokens,
        power,
        toughness,
        condition_tokens,
    })
}

fn parse_if_replacement_sentence(tokens: &[OwnedLexToken]) -> Option<(ObjectFilter, i32, i32)> {
    let (_, after_if) = primitives::parse_prefix(tokens, primitives::kw("if"))?;
    let (condition_tokens, body_tokens) =
        primitives::split_lexed_once_on_separator(after_if, || primitives::comma().void())?;
    let (_, modifier_tail) = primitives::parse_prefix(
        body_tokens,
        (
            primitives::kw("it"),
            alt((primitives::kw("get"), primitives::kw("gets"))),
        ),
    )?;
    let (modifier_tokens, ()) =
        primitives::split_lexed_once_before_suffix(modifier_tail, 1, || {
            (primitives::kw("instead"), primitives::sentence_end()).void()
        })?;
    let (power, toughness) = parse_fixed_modifier(modifier_tokens)?;
    let condition_filter = parse_attached_reference_condition(condition_tokens)?;
    Some((condition_filter, power, toughness))
}

fn parse_otherwise_anthem_sentence(tokens: &[OwnedLexToken]) -> Option<(i32, i32)> {
    let (_, modifier_tokens) = primitives::parse_prefix(
        tokens,
        (
            primitives::kw("otherwise"),
            opt(primitives::comma()),
            primitives::kw("it"),
            alt((primitives::kw("get"), primitives::kw("gets"))),
        ),
    )?;
    parse_fixed_modifier(modifier_tokens)
}

fn parse_fixed_modifier(tokens: &[OwnedLexToken]) -> Option<(i32, i32)> {
    let values = crate::grammar::primitives::probe_all(
        tokens,
        parse_fixed_modifier_lexed,
        "fixed anthem modifier",
    )?;
    let (crate::effect::Value::Fixed(power), crate::effect::Value::Fixed(toughness)) = values
    else {
        return None;
    };
    Some((power, toughness))
}

fn parse_fixed_modifier_lexed(
    input: &mut LexStream<'_>,
) -> WResult<(crate::effect::Value, crate::effect::Value)> {
    opt(alt((
        (
            alt((primitives::kw("a"), primitives::kw("an"))),
            primitives::kw("additional"),
        )
            .void(),
        primitives::kw("additional").void(),
    )))
    .parse_next(input)?;
    let token: &OwnedLexToken = any.parse_next(input)?;
    let values = leaf::parse_leaf_pt_modifier_values_complete(token.parser_text())
        .map_err(|_| primitives::backtrack_err("fixed anthem modifier", "power/toughness"))?;
    primitives::sentence_end().parse_next(input)?;
    eof.parse_next(input)?;
    Ok(values)
}

fn parse_attached_reference_condition(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (_, descriptor_tokens) = primitives::parse_prefix(
        tokens,
        alt((
            (primitives::kw("it"), primitives::kw("is")).void(),
            primitives::kw("it's").void(),
            primitives::kw("its").void(),
        )),
    )?;
    let (_, descriptor_tokens) = primitives::parse_prefix(
        descriptor_tokens,
        opt(alt((primitives::kw("a"), primitives::kw("an")))).void(),
    )?;
    let descriptor_tokens = super::trim_anthem_clause_tokens(descriptor_tokens);
    if primitives::parse_all(
        descriptor_tokens,
        primitives::kw("attacking"),
        "attached attacking condition",
    )
    .is_ok()
    {
        let mut filter = ObjectFilter::default();
        filter.attacking = true;
        return Some(filter);
    }
    crate::grammar::primitives::probe_shape(filters::parse_object_filter_with_grammar_entrypoint(
        descriptor_tokens,
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_carried_subject_type_addition() {
        let tokens = lex_line(
            "Enchanted creature gets +1/+1 and has flying. It's a Dragon in addition to its other types.",
            0,
        )
        .unwrap();
        let shape = parse_carried_subject_type_addition(&tokens).expect("carried addition");
        assert!(!shape.first_sentence_tokens.is_empty());
        assert!(!shape.subject_tokens.is_empty());
        assert!(super::super::parse_type_color_addition_shape(shape.addition_tokens).is_some());
    }

    #[test]
    fn parses_conditional_replacement_and_otherwise_anthems() {
        let tokens = lex_line(
            "Equipped creature gets +1/+1. If it's a Warrior, it gets +2/+1 instead.",
            0,
        )
        .unwrap();
        let replacement =
            parse_conditional_anthem_replacement(&tokens).expect("conditional replacement");
        assert_eq!((replacement.base_power, replacement.base_toughness), (1, 1));
        assert_eq!(
            (
                replacement.replacement_power,
                replacement.replacement_toughness
            ),
            (2, 1)
        );

        let tokens = lex_line(
            "Enchanted creature gets +3/+0 as long as it's attacking. Otherwise, it gets -2/-1.",
            0,
        )
        .unwrap();
        let otherwise = parse_conditional_anthem_otherwise(&tokens).expect("otherwise anthem");
        assert_eq!((otherwise.true_power, otherwise.true_toughness), (3, 0));
        assert_eq!((otherwise.false_power, otherwise.false_toughness), (-2, -1));

        let tokens = lex_line(
            "Equipped creature gets +2/+0. It gets an additional +0/+2 and has first strike as long as an Equipment named Groom's Finery is attached to a creature you control.",
            0,
        )
        .unwrap();
        let carried =
            parse_carried_conditional_anthem_grant(&tokens).expect("carried conditional grant");
        assert_eq!((carried.base_power, carried.base_toughness), (2, 0));
        assert_eq!(
            (carried.additional_power, carried.additional_toughness),
            (0, 2)
        );
    }
}
