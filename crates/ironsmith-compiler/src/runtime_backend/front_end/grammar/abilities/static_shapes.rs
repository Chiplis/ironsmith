use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::types::Subtype;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView};
use super::super::{leaf, primitives};
use super::surface::{matches_exact_tokens, parse_phrase_words, take_word};

const DRAW_REPLACEMENT_DOUBLE: &[&str] = &[
    "if", "you", "would", "draw", "a", "card", "draw", "two", "cards", "instead",
];
const DRAW_REPLACEMENT_SKIP_EMPTY_LIBRARY: &[&str] = &[
    "if", "you", "would", "draw", "a", "card", "while", "your", "library", "has", "no", "cards",
    "in", "it", "skip", "that", "draw", "instead",
];
const OPPONENT_DISCARD_THIS_TO_BATTLEFIELD_REPLACEMENT: &[&str] = &[
    "if",
    "a",
    "spell",
    "or",
    "ability",
    "an",
    "opponent",
    "controls",
    "causes",
    "you",
    "to",
    "discard",
    "this",
    "card",
    "put",
    "it",
    "onto",
    "the",
    "battlefield",
    "instead",
    "of",
    "putting",
    "it",
    "into",
    "your",
    "graveyard",
];

pub(crate) fn is_land_reveal_enters_static_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    parse_land_reveal_enters_shape(tokens).is_some()
}

pub(crate) fn is_opening_hand_begin_game_static_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    parse_opening_hand_begin_game_shape(tokens).is_some()
}

pub(crate) fn is_draw_replacement_double_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_tokens(tokens, DRAW_REPLACEMENT_DOUBLE)
}

pub(crate) fn is_draw_replacement_skip_empty_library_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_tokens(tokens, DRAW_REPLACEMENT_SKIP_EMPTY_LIBRARY)
}

pub(crate) fn is_opponent_effect_discard_this_to_battlefield_replacement_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    matches_exact_tokens(tokens, OPPONENT_DISCARD_THIS_TO_BATTLEFIELD_REPLACEMENT)
}

pub(crate) fn parse_can_block_subtype_as_though_reach_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Subtype> {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    let subtype = parse_can_block_subtype_words(&mut input).ok()?;
    primitives::word_slice_eof(&mut input).ok()?;
    Some(subtype)
}

pub(crate) fn is_prevent_all_noncombat_damage_to_matching_permanents_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    parse_prevent_matching_permanents_shape(tokens, true)
}

pub(crate) fn is_prevent_all_combat_damage_to_matching_permanents_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    parse_prevent_matching_permanents_shape(tokens, false)
}

fn parse_land_reveal_enters_shape(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let mut input = LexStream::new(tokens);
    primitives::phrase(&["as", "this", "land", "enters"])
        .parse_next(&mut input)
        .ok()?;
    repeat_till(
        0..,
        any.void(),
        peek(primitives::phrase(&["you", "may", "reveal"])).void(),
    )
    .map(|((), ())| ())
    .parse_next(&mut input)
    .ok()?;
    primitives::phrase(&["you", "may", "reveal"])
        .parse_next(&mut input)
        .ok()?;
    let object: &[OwnedLexToken] = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["from", "your", "hand"])).void(),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    primitives::phrase(&["from", "your", "hand"])
        .parse_next(&mut input)
        .ok()?;
    token_words_present(object).then_some(object)
}

fn parse_opening_hand_begin_game_shape(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let mut input = LexStream::new(tokens);
    primitives::phrase(&["if", "this", "card", "is", "in", "your", "opening", "hand"])
        .parse_next(&mut input)
        .ok()?;
    repeat_till(
        0..,
        any.void(),
        peek(primitives::phrase(&[
            "you", "may", "begin", "the", "game", "with",
        ]))
        .void(),
    )
    .map(|((), ())| ())
    .parse_next(&mut input)
    .ok()?;
    primitives::phrase(&["you", "may", "begin", "the", "game", "with"])
        .parse_next(&mut input)
        .ok()?;
    let object: &[OwnedLexToken] = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["on", "the", "battlefield"])).void(),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    primitives::phrase(&["on", "the", "battlefield"])
        .parse_next(&mut input)
        .ok()?;
    token_words_present(object).then_some(object)
}

fn parse_can_block_subtype_words(input: &mut primitives::WordSliceInput<'_>) -> WResult<Subtype> {
    alt((
        |input: &mut primitives::WordSliceInput<'_>| {
            parse_phrase_words(input, &["this", "creature", "can", "block"])
        },
        |input: &mut primitives::WordSliceInput<'_>| {
            parse_phrase_words(input, &["this", "can", "block"])
        },
    ))
    .parse_next(input)?;
    let subtype_word = take_word(input)?;
    parse_phrase_words(input, &["as", "though", "it", "had", "reach"])?;
    let subtype = leaf::parse_leaf_subtype_flexible_complete(subtype_word)
        .map_err(|_| primitives::backtrack_err("blocking subtype", "creature subtype"))?;
    if !subtype.is_creature_type() {
        return Err(primitives::backtrack_err(
            "blocking subtype",
            "creature subtype",
        ));
    }
    Ok(subtype)
}

fn parse_prevent_matching_permanents_shape(tokens: &[OwnedLexToken], noncombat: bool) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    let prefix = if noncombat {
        &[
            "prevent",
            "all",
            "noncombat",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
        ][..]
    } else {
        &[
            "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to",
        ][..]
    };
    if parse_phrase_words(&mut input, prefix).is_err() || input.is_empty() {
        return false;
    }
    if matches_exact_word_input(input, &["this", "creature"])
        || matches_exact_word_input(input, &["this", "permanent"])
        || matches_exact_word_input(input, &["it"])
    {
        return false;
    }
    !word_occurs(input, "turn")
}

fn matches_exact_word_input(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (
        |input: &mut primitives::WordSliceInput<'_>| parse_phrase_words(input, expected),
        eof,
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}

fn word_occurs(words: &[&str], expected: &str) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    while let Ok(word) = take_word(&mut input) {
        if word == expected {
            return true;
        }
    }
    false
}

fn token_words_present(tokens: &[OwnedLexToken]) -> bool {
    tokens.iter().any(|token| {
        matches!(
            token.kind,
            TokenKind::Word | TokenKind::Number | TokenKind::Tilde
        )
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn land_and_opening_hand_shapes_require_a_captured_object() {
        assert!(is_land_reveal_enters_static_line_lexed(&lex(
            "As this land enters, you may reveal a Plains card from your hand."
        )));
        assert!(is_opening_hand_begin_game_static_line_lexed(&lex(
            "If this card is in your opening hand, you may begin the game with it on the battlefield."
        )));
    }

    #[test]
    fn exact_replacement_shapes_are_typed_winnow_matches() {
        assert!(is_draw_replacement_double_line_lexed(&lex(
            "If you would draw a card, draw two cards instead."
        )));
        assert!(
            is_opponent_effect_discard_this_to_battlefield_replacement_line_lexed(&lex(
                "If a spell or ability an opponent controls causes you to discard this card, put it onto the battlefield instead of putting it into your graveyard."
            ))
        );
    }

    #[test]
    fn combat_shapes_return_subtype_and_filter_source_surfaces() {
        assert_eq!(
            parse_can_block_subtype_as_though_reach_line_lexed(&lex(
                "This creature can block Dragons as though it had reach."
            )),
            Some(Subtype::Dragon)
        );
        assert!(
            is_prevent_all_combat_damage_to_matching_permanents_line_lexed(&lex(
                "Prevent all combat damage that would be dealt to attacking creatures."
            ))
        );
        assert!(
            !is_prevent_all_combat_damage_to_matching_permanents_line_lexed(&lex(
                "Prevent all combat damage that would be dealt to this creature."
            ))
        );
    }
}
