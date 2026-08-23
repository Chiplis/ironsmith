use winnow::combinator::alt;
use winnow::prelude::*;

use crate::events::KeywordActionKind;
use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, trim_lexed_commas};

#[derive(Clone, Copy, Debug)]
pub struct PutStickerShape<'a> {
    pub action: KeywordActionKind,
    pub target_tokens: &'a [OwnedLexToken],
    pub target_is_reference: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct StickerAuraShape<'a> {
    pub sticker_target_tokens: &'a [OwnedLexToken],
    pub enchant_filter_tokens: &'a [OwnedLexToken],
}

fn marker_anywhere<'a, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P,
    P: Parser<crate::lexer::LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    primitives::find_prefix(tokens, make_parser).is_some()
}

fn last_on_index(tokens: &[OwnedLexToken]) -> Option<usize> {
    let (index, (), after) = primitives::find_prefix(tokens, || primitives::kw("on").void())?;
    let after_start = tokens.len().checked_sub(after.len())?;
    last_on_index(after)
        .map(|nested| after_start + nested)
        .or(Some(index))
}

fn classify_sticker_action(tokens: &[OwnedLexToken]) -> KeywordActionKind {
    if marker_anywhere(tokens, || primitives::phrase(&["name", "sticker"]).void()) {
        KeywordActionKind::NameSticker
    } else if marker_anywhere(tokens, || primitives::phrase(&["art", "sticker"]).void()) {
        KeywordActionKind::ArtSticker
    } else if marker_anywhere(tokens, || {
        primitives::phrase(&["ability", "sticker"]).void()
    }) {
        KeywordActionKind::AbilitySticker
    } else if marker_anywhere(tokens, || {
        primitives::phrase(&["power", "and", "toughness", "sticker"]).void()
    }) {
        KeywordActionKind::PowerToughnessSticker
    } else {
        KeywordActionKind::Sticker
    }
}

pub fn parse_put_sticker_shape(tokens: &[OwnedLexToken]) -> Option<PutStickerShape<'_>> {
    let (_, body) = primitives::parse_prefix(
        tokens,
        alt((primitives::kw("put"), primitives::kw("puts"))).void(),
    )?;
    let on_index = last_on_index(body)?;
    let sticker_tokens = trim_lexed_commas(body.get(..on_index)?);
    if !marker_anywhere(sticker_tokens, || {
        alt((primitives::kw("sticker"), primitives::kw("stickers"))).void()
    }) {
        return None;
    }
    let (_, target_tokens) =
        primitives::parse_prefix(body.get(on_index..)?, primitives::kw("on").void())?;
    let target_tokens = trim_lexed_commas(target_tokens);
    if target_tokens.is_empty() {
        return None;
    }
    let target_is_reference = primitives::parse_prefix(
        target_tokens,
        primitives::any_phrase(&[
            &["target"],
            &["it"],
            &["them"],
            &["that"],
            &["those"],
            &["this"],
        ])
        .void(),
    )
    .is_some();
    Some(PutStickerShape {
        action: classify_sticker_action(sticker_tokens),
        target_tokens,
        target_is_reference,
    })
}

pub fn parse_sticker_aura_shape(tokens: &[OwnedLexToken]) -> Option<StickerAuraShape<'_>> {
    let (sticker_target_tokens, aura_tail) =
        primitives::split_lexed_once_on_separator(tokens, || {
            primitives::phrase(&["then", "it", "becomes"]).void()
        })?;
    let sticker_target_tokens = trim_lexed_commas(sticker_target_tokens);
    if sticker_target_tokens.is_empty() {
        return None;
    }
    let (_, (), enchant_filter_tokens) = primitives::find_prefix(aura_tail, || {
        alt((
            primitives::phrase(&["an", "aura", "with", "enchant"]),
            primitives::phrase(&["a", "aura", "with", "enchant"]),
            primitives::phrase(&["aura", "with", "enchant"]),
        ))
        .void()
    })?;
    let enchant_filter_tokens = trim_lexed_commas(enchant_filter_tokens);
    (!enchant_filter_tokens.is_empty()).then_some(StickerAuraShape {
        sticker_target_tokens,
        enchant_filter_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{TokenWordView, lex_line};

    #[test]
    fn captures_sticker_target_and_aura_tail() {
        let tokens = lex_line(
            "Put an ability sticker on target creature, then it becomes an Aura with enchant creature",
            0,
        )
        .unwrap();
        let sticker = parse_put_sticker_shape(&tokens).unwrap();
        assert_eq!(sticker.action, KeywordActionKind::AbilitySticker);
        assert!(sticker.target_is_reference);
        let aura = parse_sticker_aura_shape(sticker.target_tokens).unwrap();
        assert_eq!(
            TokenWordView::new(aura.enchant_filter_tokens).to_word_refs(),
            ["creature"]
        );
    }
}
