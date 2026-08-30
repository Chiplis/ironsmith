use winnow::prelude::*;

use crate::cards::builders::{LibraryBottomOrderAst, LibraryConsultModeAst};
use crate::grammar::{permission_shapes, primitives};
use crate::lexer::{LexStream, OwnedLexToken, TokenWordView};

use super::super::library::parse_bottom_order;
use super::super::{sequence_any_phrase, sequence_phrase};

const NOT_CAST_MARKERS: &[&[&str]] = &[
    &["not", "cast", "this"],
    &["were", "not", "cast", "this", "way"],
    &["werent", "cast", "this", "way"],
    &["weren't", "cast", "this", "way"],
];
const MOVE_TO_HAND_PHRASES: &[&[&str]] = &[
    &["put", "that", "card", "into", "your", "hand"],
    &["put", "the", "exiled", "card", "into", "your", "hand"],
    &["put", "it", "into", "your", "hand"],
];
const DECLINED_ACTION_PHRASES: &[&[&str]] = &[
    &["put", "that", "card", "into", "your", "hand"],
    &["put", "the", "exiled", "card", "into", "your", "hand"],
    &["put", "it", "into", "your", "hand"],
    &[
        "cast", "that", "card", "this", "way", "put", "it", "into", "your", "hand",
    ],
    &[
        "cast", "the", "exiled", "card", "this", "way", "put", "it", "into", "your", "hand",
    ],
    &[
        "cast", "it", "this", "way", "put", "it", "into", "your", "hand",
    ],
];
const NOT_CAST_SUFFIXES: &[&[&str]] = &[
    &["if", "it", "wasnt", "cast", "this", "way"],
    &["if", "it", "wasn't", "cast", "this", "way"],
];

pub fn parse_consult_remainder_order_shape(words: &[&str]) -> Option<LibraryBottomOrderAst> {
    if permission_shapes::find_words(words, &["bottom"]).is_none()
        || permission_shapes::find_words(words, &["library"]).is_none()
    {
        return None;
    }
    if permission_shapes::find_words(words, &["random", "order"]).is_some() {
        return Some(LibraryBottomOrderAst::Random);
    }
    if permission_shapes::find_words(words, &["any", "order"]).is_some() {
        return Some(LibraryBottomOrderAst::ChooserChooses);
    }
    None
}

pub fn parse_consult_bottom_remainder_shape(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<LibraryBottomOrderAst> {
    let order = parse_bottom_order(tokens)?;
    let mode_word = match mode {
        LibraryConsultModeAst::Reveal => "revealed",
        LibraryConsultModeAst::Exile => "exiled",
    };
    if !permission_shapes::contains_tokens(tokens, &[mode_word]) {
        return None;
    }
    let mentions_cast_window = NOT_CAST_MARKERS
        .iter()
        .any(|marker| permission_shapes::contains_tokens(tokens, marker));
    let mentions_remainder = permission_shapes::contains_tokens(tokens, &["rest"])
        || permission_shapes::contains_tokens(tokens, &["other"]);
    (mentions_cast_window || mentions_remainder).then_some(order)
}

fn declined_move_prefix(input: &mut LexStream<'_>) -> winnow::error::ModalResult<()> {
    sequence_phrase(&["if", "you"]).parse_next(input)?;
    sequence_any_phrase(&[&["dont"], &["don’t"], &["do", "not"]]).parse_next(input)?;
    sequence_any_phrase(DECLINED_ACTION_PHRASES).parse_next(input)
}

pub fn is_if_declined_put_match_into_hand_shape(tokens: &[OwnedLexToken]) -> bool {
    if let Some(((), remainder)) =
        primitives::parse_prefix(tokens, sequence_any_phrase(MOVE_TO_HAND_PHRASES))
    {
        let remainder_words = TokenWordView::new(remainder);
        if remainder_words.is_empty()
            || NOT_CAST_SUFFIXES
                .iter()
                .any(|suffix| permission_shapes::exact_words(&remainder_words.word_refs(), suffix))
        {
            return true;
        }
    }
    let words = TokenWordView::new(tokens).word_refs();
    if words.len() == 13
        && permission_shapes::exact_words(&words[..2], &["if", "you"])
        && permission_shapes::exact_any_words(&words[2..3], &[&["dont"], &["don't"], &["don’t"]])
        && permission_shapes::exact_words(
            &words[3..],
            &[
                "cast", "that", "card", "this", "way", "put", "it", "into", "your", "hand",
            ],
        )
    {
        return true;
    }
    primitives::parse_prefix(tokens, declined_move_prefix).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_consult_remainder_orders_and_declined_moves() {
        assert_eq!(
            parse_consult_remainder_order_shape(&[
                "put", "the", "rest", "on", "bottom", "of", "your", "library", "in", "a", "random",
                "order"
            ]),
            Some(LibraryBottomOrderAst::Random)
        );
        assert_eq!(
            parse_consult_bottom_remainder_shape(
                &lex(
                    "Put the other cards revealed this way on the bottom of your library in any order"
                ),
                LibraryConsultModeAst::Reveal,
            ),
            Some(LibraryBottomOrderAst::ChooserChooses)
        );
        assert!(is_if_declined_put_match_into_hand_shape(&lex(
            "If you don't cast that card this way, put it into your hand"
        )));
        assert!(is_if_declined_put_match_into_hand_shape(&lex(
            "Put that card into your hand if it wasn't cast this way"
        )));
    }
}
