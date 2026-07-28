use winnow::combinator::{alt, eof, peek};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StickerCountKind {
    Any,
    PowerToughness,
    Name,
    Art,
    Ability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StickerCountShape<'a> {
    pub(crate) kind: StickerCountKind,
    pub(crate) source_tokens: &'a [OwnedLexToken],
    pub(crate) max_name_letters: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ForEachSpecialShape<'a> {
    AffectedAttackedThisTurn,
    ColorsOfAffected,
    CreatureTypesOfAffected,
    BlockingSource,
    AttachedToSource { filter_tokens: &'a [OwnedLexToken] },
    UnspentGreenManaYouHave,
}

pub(crate) fn parse_for_each_rest(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (_, rest) = primitives::parse_prefix(tokens, primitives::phrase(&["for", "each"]))?;
    Some(rest)
}

pub(crate) fn parse_for_each_special_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachSpecialShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    if parse_complete_phrase(tokens, &["time", "it", "has", "attacked", "this", "turn"]) {
        return Some(ForEachSpecialShape::AffectedAttackedThisTurn);
    }
    if parse_complete_any_phrase(
        tokens,
        &[
            &["of", "its", "colors"],
            &["of", "their", "colors"],
            &["color", "it", "is"],
            &["colors", "it", "is"],
        ],
    ) {
        return Some(ForEachSpecialShape::ColorsOfAffected);
    }
    if parse_complete_any_phrase(
        tokens,
        &[
            &["of", "its", "creature", "types"],
            &["of", "their", "creature", "types"],
        ],
    ) {
        return Some(ForEachSpecialShape::CreatureTypesOfAffected);
    }
    if parse_complete_any_phrase(
        tokens,
        &[
            &["creature", "is", "blocking", "it"],
            &["creature", "is", "blocking", "this", "creature"],
            &["creatures", "are", "blocking", "it"],
            &["creatures", "are", "blocking", "this", "creature"],
        ],
    ) {
        return Some(ForEachSpecialShape::BlockingSource);
    }
    if parse_complete_phrase(tokens, &["unspent", "green", "mana", "you", "have"]) {
        return Some(ForEachSpecialShape::UnspentGreenManaYouHave);
    }
    let (filter_tokens, tail) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("attached").void())?;
    let filter_tokens = trim_lexed_commas(filter_tokens);
    if filter_tokens.is_empty()
        || !parse_complete_any_phrase(
            tail,
            &[
                &["to", "it"],
                &["to", "this", "creature"],
                &["to", "this", "permanent"],
            ],
        )
    {
        return None;
    }
    Some(ForEachSpecialShape::AttachedToSource { filter_tokens })
}

pub(crate) fn parse_sticker_count_shape(tokens: &[OwnedLexToken]) -> Option<StickerCountShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    primitives::parse_all(tokens, parse_sticker_count_lexed, "sticker count shape").ok()
}

pub(crate) fn parse_compound_count_segments(
    tokens: &[OwnedLexToken],
) -> Option<Vec<&[OwnedLexToken]>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    if !contains_word(tokens, || primitives::kw("and").void()) {
        return None;
    }

    let mut segments = Vec::new();
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    let mut segment_start = 0usize;
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if (primitives::kw("and"), peek(parse_each_or_every))
            .void()
            .parse_next(&mut candidate)
            .is_ok()
        {
            if segment_start == offset {
                return None;
            }
            segments.push(trim_lexed_commas(&tokens[segment_start..offset]));
            segment_start = offset + 1;
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        if parsed.is_err() {
            break;
        }
    }
    if segments.is_empty() || segment_start >= tokens.len() {
        return None;
    }
    segments.push(trim_lexed_commas(&tokens[segment_start..]));
    if segments.iter().any(|segment| segment.is_empty()) {
        return None;
    }
    Some(segments)
}

pub(crate) fn strip_each_or_every(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(tokens, parse_each_or_every)
        .map(|(_, rest)| rest)
        .unwrap_or(tokens)
}

fn parse_sticker_count_lexed<'a>(input: &mut LexStream<'a>) -> WResult<StickerCountShape<'a>> {
    let kind = parse_sticker_head.parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    let remaining: &'a [OwnedLexToken] = rest.parse_next(input)?;
    if let Some((source_tokens, max_name_letters)) =
        primitives::split_lexed_once_before_suffix(remaining, 0, || parse_letter_cap)
    {
        return Ok(StickerCountShape {
            kind,
            source_tokens: trim_lexed_commas(source_tokens),
            max_name_letters: Some(max_name_letters),
        });
    }
    Ok(StickerCountShape {
        kind,
        source_tokens: trim_lexed_commas(remaining),
        max_name_letters: None,
    })
}

fn parse_sticker_head(input: &mut LexStream<'_>) -> WResult<StickerCountKind> {
    alt((
        (
            primitives::phrase(&["power", "and", "toughness"]),
            parse_sticker_word,
        )
            .value(StickerCountKind::PowerToughness),
        (primitives::kw("name"), parse_sticker_word).value(StickerCountKind::Name),
        (primitives::kw("art"), parse_sticker_word).value(StickerCountKind::Art),
        (primitives::kw("ability"), parse_sticker_word).value(StickerCountKind::Ability),
        parse_sticker_word.value(StickerCountKind::Any),
    ))
    .parse_next(input)
}

fn parse_sticker_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("sticker"), primitives::kw("stickers")))
        .void()
        .parse_next(input)
}

fn parse_letter_cap(input: &mut LexStream<'_>) -> WResult<u32> {
    primitives::kw("with").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed(input)?;
    alt((
        primitives::phrase(&["or", "fewer", "letters"]),
        primitives::phrase(&["or", "less", "letters"]),
    ))
    .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(count)
}

fn parse_each_or_every(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("each"), primitives::kw("every")))
        .void()
        .parse_next(input)
}

fn contains_word<'a, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P + Copy,
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    primitives::find_prefix(tokens, make_parser).is_some()
}

fn parse_complete_phrase(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    primitives::parse_all(tokens, primitives::phrase(words), "anthem exact phrase").is_ok()
}

fn parse_complete_any_phrase(
    tokens: &[OwnedLexToken],
    phrases: &[&'static [&'static str]],
) -> bool {
    primitives::parse_all(
        tokens,
        primitives::any_phrase(phrases),
        "anthem exact phrase",
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::TokenWordView;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn parses_typed_sticker_count() {
        let tokens = lex("name stickers on it with two or fewer letters");
        let shape = parse_sticker_count_shape(&tokens).expect("sticker count");
        assert_eq!(shape.kind, StickerCountKind::Name);
        assert_eq!(shape.max_name_letters, Some(2));
    }

    #[test]
    fn classifies_for_each_attached_shape() {
        let tokens = lex("Aura attached to this creature");
        assert!(matches!(
            parse_for_each_special_shape(&tokens),
            Some(ForEachSpecialShape::AttachedToSource { .. })
        ));
    }

    #[test]
    fn splits_additive_count_domains_across_non_graveyard_zones() {
        let tokens = lex("card in your hand and each foretold card you own in exile");
        let segments = parse_compound_count_segments(&tokens).expect("compound count domains");

        assert_eq!(segments.len(), 2);
        assert_eq!(
            TokenWordView::new(segments[0]).to_word_refs(),
            ["card", "in", "your", "hand"]
        );
        assert_eq!(
            TokenWordView::new(segments[1]).to_word_refs(),
            ["each", "foretold", "card", "you", "own", "in", "exile"]
        );
    }
}
