use std::ops::Range;

use winnow::combinator::alt;
use winnow::prelude::*;

use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, trim_lexed_commas};

const INLINE_CREATURE_TYPE_CHOICES: &[&[&str]] = &[
    &["of", "the", "creature", "type", "of", "your", "choice"],
    &["of", "creature", "type", "of", "your", "choice"],
];
const REFERENCED_TYPE_CHOICES: &[&[&str]] = &[
    &["of", "the", "chosen", "type"],
    &["of", "chosen", "type"],
    &["of", "that", "type"],
    &["that", "type"],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DestroyCreatureTypeChoiceShape;

#[derive(Clone, Copy, Debug)]
pub(crate) struct PumpCreatureTypeChoiceShape<'a> {
    pub(crate) base_subject_tokens: &'a [OwnedLexToken],
    pub(crate) filter_subject_tokens: &'a [OwnedLexToken],
    pub(crate) trailing_subject_tokens: &'a [OwnedLexToken],
    pub(crate) get_tail_tokens: &'a [OwnedLexToken],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MustAttackCreatureTypeChoiceShape<'a> {
    pub(crate) base_subject_tokens: &'a [OwnedLexToken],
    pub(crate) filter_subject_tokens: &'a [OwnedLexToken],
    pub(crate) trailing_subject_tokens: &'a [OwnedLexToken],
}

#[derive(Clone, Debug)]
pub(crate) struct ReturnCreatureTypeChoiceShape {
    pub(crate) base_target_tokens: Vec<OwnedLexToken>,
    pub(crate) needs_inline_choice_effect: bool,
    pub(crate) excluded: bool,
    pub(crate) has_explicit_target: bool,
}

fn find_phrase_range(
    tokens: &[OwnedLexToken],
    phrases: &'static [&'static [&'static str]],
) -> Option<Range<usize>> {
    let (start, (), rest) =
        primitives::find_prefix(tokens, || primitives::any_phrase(phrases).void())?;
    Some(start..tokens.len().checked_sub(rest.len())?)
}

fn inline_choice_range(tokens: &[OwnedLexToken]) -> Option<Range<usize>> {
    find_phrase_range(tokens, INLINE_CREATURE_TYPE_CHOICES)
}

fn referenced_choice_range(tokens: &[OwnedLexToken]) -> Option<Range<usize>> {
    find_phrase_range(tokens, REFERENCED_TYPE_CHOICES)
}

fn split_at_get(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (get_start, (), _) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("get"), primitives::kw("gets"))).void()
    })?;
    let subject = trim_lexed_commas(tokens.get(..get_start)?);
    let get_tail = trim_lexed_commas(tokens.get(get_start..)?);
    (!subject.is_empty() && !get_tail.is_empty()).then_some((subject, get_tail))
}

fn split_inline_choice_subject(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let range = inline_choice_range(tokens)?;
    Some((
        trim_lexed_commas(tokens.get(..range.start)?),
        trim_lexed_commas(tokens.get(range.end..)?),
    ))
}

fn strip_leading_all(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(tokens, primitives::kw("all").void())
        .map(|(_, rest)| trim_lexed_commas(rest))
        .unwrap_or(tokens)
}

pub(crate) fn parse_destroy_creature_type_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestroyCreatureTypeChoiceShape> {
    let (_, tail) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["destroy", "all", "creatures"]).void(),
    )?;
    inline_choice_range(tail)?;
    Some(DestroyCreatureTypeChoiceShape)
}

pub(crate) fn parse_pump_creature_type_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<PumpCreatureTypeChoiceShape<'_>> {
    let (subject_tokens, get_tail_tokens) = split_at_get(tokens)?;
    let (base_subject_tokens, trailing_subject_tokens) =
        split_inline_choice_subject(subject_tokens)?;
    Some(PumpCreatureTypeChoiceShape {
        base_subject_tokens,
        filter_subject_tokens: strip_leading_all(base_subject_tokens),
        trailing_subject_tokens,
        get_tail_tokens,
    })
}

pub(crate) fn parse_must_attack_creature_type_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<MustAttackCreatureTypeChoiceShape<'_>> {
    let (suffix_start, (), suffix_rest) = primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["attack", "this", "turn", "if", "able"]),
            primitives::phrase(&["attacks", "this", "turn", "if", "able"]),
        ))
        .void()
    })?;
    if !trim_lexed_commas(suffix_rest).is_empty() {
        return None;
    }
    let subject_tokens = trim_lexed_commas(tokens.get(..suffix_start)?);
    let (base_subject_tokens, trailing_subject_tokens) =
        split_inline_choice_subject(subject_tokens)?;
    Some(MustAttackCreatureTypeChoiceShape {
        base_subject_tokens,
        filter_subject_tokens: strip_leading_all(base_subject_tokens),
        trailing_subject_tokens,
    })
}

fn last_keyword_index(tokens: &[OwnedLexToken], keyword: &'static str) -> Option<usize> {
    let (index, (), after) = primitives::find_prefix(tokens, || primitives::kw(keyword).void())?;
    let after_start = tokens.len().checked_sub(after.len())?;
    last_keyword_index(after, keyword)
        .map(|nested| after_start + nested)
        .or(Some(index))
}

fn split_return_target_destination(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (_, body) = primitives::parse_prefix(tokens, primitives::kw("return").void())?;
    let to_index = last_keyword_index(body, "to")?;
    let target_tokens = trim_lexed_commas(body.get(..to_index)?);
    let (_, destination_tokens) =
        primitives::parse_prefix(body.get(to_index..)?, primitives::kw("to").void())?;
    let destination_tokens = trim_lexed_commas(destination_tokens);
    (!target_tokens.is_empty() && !destination_tokens.is_empty())
        .then_some((target_tokens, destination_tokens))
}

fn marker_anywhere(tokens: &[OwnedLexToken], keyword: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(keyword).void()).is_some()
}

fn exact_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::parse_prefix(
        tokens,
        (primitives::phrase(phrase), winnow::combinator::eof).void(),
    )
    .is_some()
}

fn referenced_relation_start(tokens: &[OwnedLexToken], choice_start: usize) -> (usize, bool) {
    if choice_start >= 3
        && exact_phrase(
            &tokens[choice_start - 3..choice_start],
            &["that", "are", "not"],
        )
    {
        (choice_start - 3, true)
    } else if choice_start >= 2
        && (exact_phrase(&tokens[choice_start - 2..choice_start], &["that", "arent"])
            || exact_phrase(&tokens[choice_start - 2..choice_start], &["that", "aren't"]))
    {
        (choice_start - 2, true)
    } else if choice_start >= 2
        && exact_phrase(&tokens[choice_start - 2..choice_start], &["that", "are"])
    {
        (choice_start - 2, false)
    } else {
        (choice_start, false)
    }
}

fn remove_token_range(tokens: &[OwnedLexToken], range: Range<usize>) -> Vec<OwnedLexToken> {
    let mut retained = Vec::with_capacity(tokens.len().saturating_sub(range.len()));
    retained.extend_from_slice(&tokens[..range.start]);
    retained.extend_from_slice(&tokens[range.end..]);
    trim_lexed_commas(&retained).to_vec()
}

pub(crate) fn parse_return_creature_type_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<ReturnCreatureTypeChoiceShape> {
    let (target_tokens, destination_tokens) = split_return_target_destination(tokens)?;
    if !marker_anywhere(destination_tokens, "hand") && !marker_anywhere(destination_tokens, "hands")
    {
        return None;
    }

    let (remove_range, needs_inline_choice_effect, excluded) =
        if let Some(range) = inline_choice_range(target_tokens) {
            (range, true, false)
        } else {
            let choice = referenced_choice_range(target_tokens)?;
            let (relation_start, excluded) = referenced_relation_start(target_tokens, choice.start);
            (relation_start..choice.end, false, excluded)
        };
    let base_target_tokens = remove_token_range(target_tokens, remove_range);
    Some(ReturnCreatureTypeChoiceShape {
        base_target_tokens,
        needs_inline_choice_effect,
        excluded,
        has_explicit_target: marker_anywhere(target_tokens, "target"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).unwrap()
    }

    #[test]
    fn captures_creature_type_choice_sentences() {
        let pump = lex("Creatures of the creature type of your choice get +2/+2");
        let shape = parse_pump_creature_type_choice_shape(&pump).unwrap();
        assert_eq!(
            TokenWordView::new(shape.base_subject_tokens).to_word_refs(),
            ["creatures"]
        );
        assert!(shape.trailing_subject_tokens.is_empty());

        let returned =
            lex("Return target creatures that aren't of the chosen type to their owners' hands");
        let shape = parse_return_creature_type_choice_shape(&returned).unwrap();
        assert!(shape.excluded);
        assert!(shape.has_explicit_target);
        assert_eq!(
            TokenWordView::new(&shape.base_target_tokens).to_word_refs(),
            ["target", "creatures"]
        );
    }
}
