use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::OwnedLexToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManaClauseShape {
    Replacement,
    AdditionalTrigger,
}

fn word<'a>(expected: &'static str) -> impl Parser<&'a [&'a str], &'a str, ErrMode<ContextError>> {
    primitives::word_slice_exact(expected)
}

fn demonstrative_object<'a>(input: &mut &'a [&'a str]) -> WResult<()> {
    (
        alt((word("that"), word("those"))),
        alt((
            alt((
                word("creature"),
                word("creatures"),
                word("permanent"),
                word("permanents"),
            )),
            alt((
                word("artifact"),
                word("artifacts"),
                word("enchantment"),
                word("enchantments"),
            )),
            alt((word("land"), word("lands"), word("card"), word("cards"))),
            alt((word("token"), word("tokens"), word("spell"), word("spells"))),
        )),
    )
        .void()
        .parse_next(input)
}

fn damage_by_this_creature_this<'a>(input: &mut &'a [&'a str]) -> WResult<()> {
    (
        word("dealt"),
        word("damage"),
        word("by"),
        word("this"),
        word("creature"),
        word("this"),
    )
        .void()
        .parse_next(input)
}

fn this_turn<'a>(input: &mut &'a [&'a str]) -> WResult<()> {
    (word("this"), word("turn")).void().parse_next(input)
}

fn for_mana<'a>(input: &mut &'a [&'a str]) -> WResult<()> {
    (word("for"), word("mana")).void().parse_next(input)
}

fn has_parser<'a, O, P>(words: &'a [&'a str], mut parser: P) -> bool
where
    P: Parser<&'a [&'a str], O, ErrMode<ContextError>>,
{
    let mut input = words;
    loop {
        let mut probe = input;
        if parser.parse_next(&mut probe).is_ok() {
            return true;
        }
        if any::<_, ErrMode<ContextError>>
            .parse_next(&mut input)
            .is_err()
        {
            return false;
        }
    }
}

fn has_word(words: &[&str], expected: &'static str) -> bool {
    has_parser(words, word(expected))
}

pub(crate) fn has_demonstrative_object_reference_words(words: &[&str]) -> bool {
    has_parser(words, demonstrative_object)
}

pub(crate) fn is_target_player_damage_subject_words(words: &[&str]) -> bool {
    let mut input = words;
    if alt((
        (word("target"), word("player")),
        (word("target"), word("players")),
    ))
    .parse_next(&mut input)
    .is_err()
    {
        return false;
    }
    has_parser(words, damage_by_this_creature_this) && has_parser(words, this_turn)
}

pub(crate) fn parse_mana_clause_shape_words(words: &[&str]) -> Option<ManaClauseShape> {
    if !has_parser(words, for_mana) {
        return None;
    }
    let replacement = has_word(words, "if")
        && has_word(words, "instead")
        && (has_word(words, "tap") || has_word(words, "taps"))
        && (has_word(words, "produce") || has_word(words, "produces"));
    if replacement {
        return Some(ManaClauseShape::Replacement);
    }
    let trigger = has_word(words, "whenever")
        && has_word(words, "additional")
        && (has_word(words, "tap") || has_word(words, "taps"))
        && (has_word(words, "add") || has_word(words, "adds"));
    trigger.then_some(ManaClauseShape::AdditionalTrigger)
}

pub(crate) fn starts_life_total_becomes(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(&["life", "total", "becomes"])).is_some()
}

pub(crate) fn contains_may(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "may")
}

pub(crate) fn starts_choose(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::kw("choose")).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn recognizes_word_facts_without_lowering_probes() {
        assert!(has_demonstrative_object_reference_words(&[
            "destroy", "that", "creature"
        ]));
        assert_eq!(
            parse_mana_clause_shape_words(&[
                "if", "you", "tap", "a", "land", "for", "mana", "it", "produces", "instead"
            ]),
            Some(ManaClauseShape::Replacement)
        );
        let tokens = lex_line("Life total becomes 5", 0).unwrap();
        assert!(starts_life_total_becomes(&tokens));
    }
}
