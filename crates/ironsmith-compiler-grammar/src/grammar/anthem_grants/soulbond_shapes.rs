use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulbondSharedEffect<'a> {
    PowerToughness {
        modifier_word: &'a str,
    },
    Ability {
        ability_tokens: &'a [OwnedLexToken],
        mills_each_opponent_by_toughness: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulbondSharedShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub subject_is_source_pronoun: bool,
    pub subject_has_rejected_word: bool,
    pub effect: SoulbondSharedEffect<'a>,
}

pub fn parse_soulbond_shared_shape(tokens: &[OwnedLexToken]) -> Option<SoulbondSharedShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (_, conditioned) =
        primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"]))?;
    let (subject_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(conditioned, || {
            primitives::phrase(&["is", "paired", "with", "another", "creature"])
        })?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    let effect_tokens = trim_lexed_commas(effect_tokens);
    if subject_tokens.is_empty() || effect_tokens.is_empty() {
        return None;
    }
    let subject_is_source_pronoun =
        parse_complete_any_phrase(subject_tokens, &[&["this"], &["this", "creature"]]);
    let subject_has_rejected_word = contains_rejected_subject_word(subject_tokens);
    let effect = parse_shared_effect(effect_tokens)?;
    Some(SoulbondSharedShape {
        subject_tokens,
        subject_is_source_pronoun,
        subject_has_rejected_word,
        effect,
    })
}

fn parse_shared_effect(tokens: &[OwnedLexToken]) -> Option<SoulbondSharedEffect<'_>> {
    if let Some((_, tail)) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["both", "creatures", "get"]),
            primitives::phrase(&["each", "of", "those", "creatures", "gets"]),
        )),
    ) {
        let mut input = LexStream::new(tail);
        let modifier_word =
            crate::grammar::primitives::take_leaf(&mut input, primitives::word_text)?;
        return input
            .is_empty()
            .then_some(SoulbondSharedEffect::PowerToughness { modifier_word });
    }
    let (_, ability_tokens) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["both", "creatures", "have"]),
            primitives::phrase(&["each", "of", "those", "creatures", "has"]),
        )),
    )?;
    let ability_tokens = trim_lexed_commas(ability_tokens);
    (!ability_tokens.is_empty()).then_some(SoulbondSharedEffect::Ability {
        ability_tokens,
        mills_each_opponent_by_toughness: parse_complete_phrase(
            ability_tokens,
            &[
                "whenever",
                "this",
                "creature",
                "attacks",
                "each",
                "opponent",
                "mills",
                "cards",
                "equal",
                "to",
                "its",
                "toughness",
            ],
        ),
    })
}

fn contains_rejected_subject_word(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || parse_rejected_subject_word).is_some()
}

fn parse_rejected_subject_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("enchanted"),
        primitives::kw("equipped"),
        primitives::kw("target"),
        primitives::kw("another"),
        primitives::kw("each"),
        primitives::kw("those"),
    ))
    .void()
    .parse_next(input)
}

fn parse_complete_phrase(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    primitives::parse_all(tokens, primitives::phrase(words), "soulbond exact phrase").is_ok()
}

fn parse_complete_any_phrase(
    tokens: &[OwnedLexToken],
    phrases: &[&'static [&'static str]],
) -> bool {
    primitives::parse_all(
        tokens,
        primitives::any_phrase(phrases),
        "soulbond exact phrase",
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn parses_shared_pt_shape() {
        let tokens = lex(
            "As long as this creature is paired with another creature, both creatures get +1/+1.",
        );
        assert!(matches!(
            parse_soulbond_shared_shape(&tokens).map(|shape| shape.effect),
            Some(SoulbondSharedEffect::PowerToughness { .. })
        ));
    }
}
