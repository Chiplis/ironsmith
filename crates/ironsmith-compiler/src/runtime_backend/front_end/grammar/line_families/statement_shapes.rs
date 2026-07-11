use winnow::combinator::{alt, eof, opt, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::{leaf, permission_shapes, primitives, structure};
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MultiSentenceEffectHead {
    StatementFamily,
    PlayerPermission,
    EffectVerb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatementStaticPreference {
    DrawReplacement,
    TokenCreationReplacement,
    DiscardOrRedirectReplacement,
    ConditionalKeywordTypeAddition,
    BlocksAdditionalCreatures,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FilterListContinuationShape;

pub(crate) fn parse_multi_sentence_effect_head(
    tokens: &[OwnedLexToken],
) -> Option<MultiSentenceEffectHead> {
    let sentences = structure::split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.len() <= 1 {
        return None;
    }
    let first = *sentences.first()?;
    if structure::classify_statement_line_family_lexed(first).is_some() {
        return Some(MultiSentenceEffectHead::StatementFamily);
    }
    if primitives::parse_prefix(first, player_permission_head).is_some()
        && permission_shapes::contains_tokens_any(first, &[&["deal"], &["deals"]])
    {
        return Some(MultiSentenceEffectHead::PlayerPermission);
    }
    primitives::parse_prefix(first, effect_head).map(|_| MultiSentenceEffectHead::EffectVerb)
}

pub(crate) fn parse_statement_static_preference(
    tokens: &[OwnedLexToken],
) -> Option<StatementStaticPreference> {
    if super::super::static_keyword_replacement_shapes::parse_discard_or_redirect_replacement(
        tokens,
    )
    .is_some()
    {
        return Some(StatementStaticPreference::DiscardOrRedirectReplacement);
    }
    let mut input = LexStream::new(tokens);
    statement_static_preference.parse_next(&mut input).ok()
}

fn statement_static_preference(input: &mut LexStream<'_>) -> WResult<StatementStaticPreference> {
    alt((
        draw_replacement_preference,
        token_creation_replacement_preference,
        conditional_keyword_type_addition_preference,
        blocks_additional_creatures_preference,
    ))
    .parse_next(input)
}

fn draw_replacement_preference(input: &mut LexStream<'_>) -> WResult<StatementStaticPreference> {
    primitives::phrase(&["if", "you", "would", "draw", "a", "card"]).parse_next(input)?;
    require_marker(input.as_ref(), primitives::kw("instead"))?;
    Ok(StatementStaticPreference::DrawReplacement)
}

fn token_creation_replacement_preference(
    input: &mut LexStream<'_>,
) -> WResult<StatementStaticPreference> {
    primitives::phrase(&["if", "you", "would", "create", "one", "or", "more"]).parse_next(input)?;
    let tail = input.as_ref();
    require_marker(tail, primitives::kw("additional"))?;
    require_marker(
        tail,
        alt((primitives::kw("token"), primitives::kw("tokens"))),
    )?;
    require_marker(tail, primitives::kw("instead"))?;
    Ok(StatementStaticPreference::TokenCreationReplacement)
}

fn conditional_keyword_type_addition_preference(
    input: &mut LexStream<'_>,
) -> WResult<StatementStaticPreference> {
    primitives::phrase(&["as", "long", "as"]).parse_next(input)?;
    let tail = input.as_ref();
    require_marker(tail, alt((primitives::kw("has"), primitives::kw("have"))))?;
    require_marker(
        tail,
        alt((
            primitives::phrase(&["and", "is"]),
            primitives::phrase(&["and", "are"]),
        )),
    )?;
    require_marker(tail, primitives::phrase(&["in", "addition", "to"]))?;
    Ok(StatementStaticPreference::ConditionalKeywordTypeAddition)
}

fn blocks_additional_creatures_preference(
    input: &mut LexStream<'_>,
) -> WResult<StatementStaticPreference> {
    primitives::phrase(&["this", "creature", "can", "block"]).parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    primitives::kw("additional").parse_next(input)?;
    opt(leaf::parse_leaf_number_prefix_lexed).parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures"))).parse_next(input)?;
    alt((
        primitives::phrase(&["each", "combat"]),
        primitives::phrase(&["this", "turn"]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(StatementStaticPreference::BlocksAdditionalCreatures)
}

fn require_marker<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> WResult<()>
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut probe = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parser)
        .void()
        .parse_next(&mut probe)
}

pub(crate) fn parse_filter_list_continuation(
    tokens: &[OwnedLexToken],
) -> Option<FilterListContinuationShape> {
    let mut saw_filter_atom = false;
    let mut saw_list_separator = false;

    for token in tokens {
        if token.kind == TokenKind::Period {
            break;
        }
        if token.kind == TokenKind::Comma {
            if saw_filter_atom {
                saw_list_separator = true;
            }
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if parse_effect_head_word(word).is_some()
            || permission_shapes::exact_words(&[word], &["if"])
        {
            break;
        }
        if permission_shapes::exact_any_words(&[word], &[&["and"], &["or"]]) {
            saw_list_separator = true;
            continue;
        }
        if permission_shapes::exact_any_words(
            &[word],
            &[
                &["a"],
                &["an"],
                &["the"],
                &["of"],
                &["on"],
                &["in"],
                &["with"],
                &["that"],
                &["thats"],
                &["that's"],
                &["is"],
                &["are"],
                &["battlefield"],
            ],
        ) {
            continue;
        }
        if parse_filter_list_word(word) {
            saw_filter_atom = true;
            continue;
        }
        break;
    }

    (saw_filter_atom && saw_list_separator).then_some(FilterListContinuationShape)
}

fn player_permission_head(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("any").parse_next(input)?;
    alt((primitives::kw("player"), primitives::kw("opponent"))).parse_next(input)?;
    primitives::phrase(&["may", "have"]).parse_next(input)
}

fn effect_head(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        alt((
            primitives::kw("add"),
            primitives::kw("choose"),
            primitives::kw("counter"),
            primitives::kw("create"),
            primitives::kw("deal"),
            primitives::kw("destroy"),
        )),
        alt((
            primitives::kw("discard"),
            primitives::kw("draw"),
            primitives::kw("exchange"),
            primitives::kw("exile"),
            primitives::kw("gain"),
        )),
        alt((
            primitives::kw("look"),
            primitives::kw("mill"),
            primitives::kw("put"),
            primitives::kw("return"),
            primitives::kw("reveal"),
            primitives::kw("sacrifice"),
        )),
        alt((
            primitives::kw("search"),
            primitives::kw("shuffle"),
            primitives::kw("surveil"),
            primitives::kw("tap"),
            primitives::kw("untap"),
        )),
    ))
    .void()
    .parse_next(input)
}

fn parse_effect_head_word(word: &str) -> Option<()> {
    let mut input = word;
    (effect_head_word, eof.void())
        .void()
        .parse_next(&mut input)
        .ok()
}

fn effect_head_word(input: &mut &str) -> WResult<()> {
    alt((
        alt(("add", "choose", "counter", "create", "deal", "destroy")),
        alt(("discard", "draw", "exchange", "exile", "gain")),
        alt(("look", "mill", "put", "return", "reveal", "sacrifice")),
        alt(("search", "shuffle", "surveil", "tap", "untap")),
    ))
    .void()
    .parse_next(input)
}

fn parse_filter_list_word(word: &str) -> bool {
    leaf::parse_leaf_color_complete(word).is_ok()
        || leaf::parse_leaf_card_type_complete(word).is_ok()
        || leaf::parse_leaf_subtype_flexible_complete(word).is_ok()
}
