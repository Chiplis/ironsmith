use winnow::combinator::alt;
use winnow::error::{ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;

#[cfg(test)]
use crate::cards::builders::CardTextError;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;
#[cfg(test)]
use super::common::{finish_text_parse, phrase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionIntro {
    If,
    Unless,
    AsLongAs,
    ForAsLongAs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeafConditionIntroPrefix<'a> {
    pub(crate) intro: ConditionIntro,
    pub(crate) rest: &'a [OwnedLexToken],
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeafConditionIntroWordPrefix {
    pub(crate) intro: ConditionIntro,
    pub(crate) consumed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafStaticConditionIntro {
    AsLongAs,
    LegacyAs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeafStaticConditionIntroPrefix<'a> {
    pub(crate) intro: LeafStaticConditionIntro,
    pub(crate) rest: &'a [OwnedLexToken],
}

#[cfg(test)]
pub(crate) fn parse_condition_intro(input: &mut &str) -> WResult<ConditionIntro> {
    alt((
        phrase("for as long as").value(ConditionIntro::ForAsLongAs),
        phrase("as long as").value(ConditionIntro::AsLongAs),
        phrase("unless").value(ConditionIntro::Unless),
        phrase("if").value(ConditionIntro::If),
    ))
    .context(StrContext::Label("condition introduction"))
    .context(StrContext::Expected(StrContextValue::Description(
        "if, unless, or as-long-as prefix",
    )))
    .parse_next(input)
}

pub(crate) fn parse_leaf_condition_intro_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionIntro> {
    alt((
        primitives::phrase(&["for", "as", "long", "as"]).value(ConditionIntro::ForAsLongAs),
        primitives::phrase(&["as", "long", "as"]).value(ConditionIntro::AsLongAs),
        primitives::kw("unless").value(ConditionIntro::Unless),
        primitives::kw("if").value(ConditionIntro::If),
    ))
    .context(StrContext::Label("condition introduction"))
    .context(StrContext::Expected(StrContextValue::Description(
        "if, unless, or as-long-as prefix",
    )))
    .parse_next(input)
}

pub(crate) fn parse_leaf_condition_intro_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeafConditionIntroPrefix<'_>> {
    let (intro, rest) = primitives::parse_prefix(tokens, parse_leaf_condition_intro_lexed)?;
    Some(LeafConditionIntroPrefix { intro, rest })
}

#[cfg(test)]
pub(crate) fn parse_leaf_condition_intro_prefix_words(
    words: &[&str],
) -> Option<LeafConditionIntroWordPrefix> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let intro = parse_leaf_condition_intro_word_slice
        .parse_next(&mut input)
        .ok()?;
    Some(LeafConditionIntroWordPrefix {
        intro,
        consumed: words.len().checked_sub(input.len())?,
    })
}

pub(crate) fn parse_leaf_static_condition_intro_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeafStaticConditionIntroPrefix<'_>> {
    let (intro, rest) = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        alt((
            primitives::phrase(&["as", "long", "as"]).value(LeafStaticConditionIntro::AsLongAs),
            primitives::kw("as").value(LeafStaticConditionIntro::LegacyAs),
        ))
        .parse_next(input)
    })?;
    Some(LeafStaticConditionIntroPrefix { intro, rest })
}

#[cfg(test)]
pub(crate) fn parse_condition_intro_complete(raw: &str) -> Result<ConditionIntro, CardTextError> {
    finish_text_parse(raw, parse_condition_intro, "leaf-condition-intro")
}

#[cfg(test)]
fn parse_leaf_condition_intro_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<ConditionIntro> {
    alt((
        (
            primitives::word_slice_exact("for"),
            primitives::word_slice_exact("as"),
            primitives::word_slice_exact("long"),
            primitives::word_slice_exact("as"),
        )
            .value(ConditionIntro::ForAsLongAs),
        (
            primitives::word_slice_exact("as"),
            primitives::word_slice_exact("long"),
            primitives::word_slice_exact("as"),
        )
            .value(ConditionIntro::AsLongAs),
        primitives::word_slice_exact("unless").value(ConditionIntro::Unless),
        primitives::word_slice_exact("if").value(ConditionIntro::If),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn condition_intro_prefixes_are_typed_across_surfaces() {
        for (raw, expected, consumed) in [
            ("if you control a creature", ConditionIntro::If, 1),
            ("unless you pay 2", ConditionIntro::Unless, 1),
            ("as long as you control it", ConditionIntro::AsLongAs, 3),
            (
                "for as long as it remains exiled",
                ConditionIntro::ForAsLongAs,
                4,
            ),
        ] {
            let tokens = lex_line(raw, 0).unwrap();
            let parsed = parse_leaf_condition_intro_prefix_tokens(&tokens).unwrap();
            assert_eq!(parsed.intro, expected, "{raw}");
            assert_eq!(tokens.len() - parsed.rest.len(), consumed, "{raw}");

            let words = raw.split_whitespace().collect::<Vec<_>>();
            let parsed_words = parse_leaf_condition_intro_prefix_words(&words).unwrap();
            assert_eq!(parsed_words.intro, expected, "{raw}");
            assert_eq!(parsed_words.consumed, consumed, "{raw}");
        }
    }

    #[test]
    fn static_condition_intro_preserves_legacy_single_as() {
        for (raw, expected, consumed) in [
            (
                "as long as you control a creature",
                LeafStaticConditionIntro::AsLongAs,
                3,
            ),
            (
                "as you control a creature",
                LeafStaticConditionIntro::LegacyAs,
                1,
            ),
        ] {
            let tokens = lex_line(raw, 0).unwrap();
            let parsed = parse_leaf_static_condition_intro_prefix_tokens(&tokens).unwrap();
            assert_eq!(parsed.intro, expected, "{raw}");
            assert_eq!(tokens.len() - parsed.rest.len(), consumed, "{raw}");
        }
    }
}
