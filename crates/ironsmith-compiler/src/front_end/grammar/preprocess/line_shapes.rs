use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{literal, rest, take_till};

use super::super::{effects, permission_shapes, primitives, structure::MetadataLineKind};
use crate::lexer::{OwnedLexToken, TokenKind, TokenWordView, lex_line};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentheticalLineSurface {
    FullyWrapped,
    PreserveEnchantmentNotCreature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineVariantSplitKind {
    AdditionalCost,
    ManaSpendFollowup,
    CostAdjustmentFollowup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineVariantSplitSurface {
    pub kind: LineVariantSplitKind,
    pub first_end: usize,
    pub second_start: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataSurface {
    pub kind: MetadataLineKind,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabeledAbilityPrefixSurface {
    pub remainder_start: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolutionTimingTailSurface {
    pub tail_start: usize,
    pub terminal_period: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedActivationSurface {
    pub inner: String,
    pub inner_start: usize,
}

pub fn parse_parenthetical_line_surface(line: &str) -> Option<ParentheticalLineSurface> {
    let tokens = lex_line(line.trim(), 0).ok()?;
    if tokens.first()?.kind == TokenKind::LParen && tokens.last()?.kind == TokenKind::RParen {
        return Some(ParentheticalLineSurface::FullyWrapped);
    }

    let words = TokenWordView::new(&tokens);
    let word_refs = words.word_refs();
    permission_shapes::find_words(&word_refs, &["its", "an", "enchantment"])?;
    let not_creature = permission_shapes::find_words(&word_refs, &["its", "not", "a", "creature"])?;
    let token_index = *words.token_start_indices().get(not_creature)?;
    (tokens.get(token_index.checked_sub(1)?)?.kind == TokenKind::LParen)
        .then_some(ParentheticalLineSurface::PreserveEnchantmentNotCreature)
}

pub fn parse_line_variant_split(line: &str) -> Option<LineVariantSplitSurface> {
    let tokens = lex_line(line.trim(), 0).ok()?;
    if permission_shapes::prefix_tokens(
        &tokens,
        &[
            "as",
            "an",
            "additional",
            "cost",
            "to",
            "cast",
            "this",
            "spell",
        ],
    ) && let Some((period_index, period, _)) =
        primitives::find_prefix(&tokens, primitives::period)
    {
        return split_surface(
            &tokens,
            period_index,
            period,
            LineVariantSplitKind::AdditionalCost,
        );
    }

    if let Some((period_index, period, _)) = primitives::find_prefix(&tokens, || {
        (
            primitives::period(),
            primitives::phrase(&["when", "you", "spend", "this", "mana", "to", "cast"]),
        )
            .map(|(period, ())| period)
    }) && primitives::find_prefix(&tokens[..period_index], primitives::colon).is_some()
    {
        return split_surface(
            &tokens,
            period_index,
            period,
            LineVariantSplitKind::ManaSpendFollowup,
        );
    }

    let (period_index, period, _) = primitives::find_prefix(&tokens, || {
        (
            primitives::period(),
            alt((
                primitives::phrase(&["this", "cost", "is", "reduced", "by"]),
                primitives::phrase(&["this", "ability", "costs"]),
                primitives::phrase(&["this", "spell", "costs"]),
            )),
        )
            .map(|(period, ())| period)
    })?;
    split_surface(
        &tokens,
        period_index,
        period,
        LineVariantSplitKind::CostAdjustmentFollowup,
    )
}

pub fn parse_metadata_surface(line: &str) -> Option<MetadataSurface> {
    let trimmed = line.trim();
    let mut input = trimmed;
    let (label, value) = metadata_parts.parse_next(&mut input).ok()?;
    let label = label.trim();
    let value = value.trim();
    if label.is_empty() || value.is_empty() {
        return None;
    }
    let label_tokens = lex_line(format!("{label}:").as_str(), 0).ok()?;
    let kind = super::super::structure::split_metadata_line_lexed(&label_tokens)?.kind;
    Some(MetadataSurface {
        kind,
        value: value.to_string(),
    })
}

fn metadata_parts<'a>(input: &mut &'a str) -> WResult<(&'a str, &'a str)> {
    let label = take_till(1.., |character: char| character == ':').parse_next(input)?;
    literal(':').parse_next(input)?;
    let value = rest.parse_next(input)?;
    Ok((label, value))
}

pub fn parse_labeled_ability_prefix(text: &str) -> Option<LabeledAbilityPrefixSurface> {
    let tokens = lex_line(text, 0).ok()?;
    let (separator_index, separator, _) = primitives::find_prefix(&tokens, || {
        alt((
            primitives::token_kind(TokenKind::EmDash),
            primitives::token_kind(TokenKind::Dash),
        ))
    })?;
    let prefix = text.get(..separator.span.start)?.trim();
    if effects::preserve_labeled_ability_prefix_for_parse_text(prefix) {
        return None;
    }
    let remainder_start = tokens.get(separator_index + 1)?.span.start;
    let remainder = text.get(remainder_start..)?.trim_start();
    if remainder.is_empty() || !effects::should_strip_labeled_ability_prefix_text(prefix, remainder)
    {
        return None;
    }
    Some(LabeledAbilityPrefixSurface { remainder_start })
}

pub fn parse_resolution_timing_tail(text: &str) -> Option<ResolutionTimingTailSurface> {
    let tokens = lex_line(text, 0).ok()?;
    let (tail_index, _, _) =
        primitives::find_prefix(&tokens, || primitives::phrase(&["as", "it", "resolves"]))?;
    if tokens
        .iter()
        .skip(tail_index.saturating_add(3))
        .any(|token| token.as_word().is_some())
    {
        return None;
    }
    Some(ResolutionTimingTailSurface {
        tail_start: tokens.get(tail_index)?.span.start,
        terminal_period: tokens.last().is_some_and(OwnedLexToken::is_period),
    })
}

pub fn parse_wrapped_activation_surface(text: &str) -> Option<WrappedActivationSurface> {
    let trimmed = text.trim();
    let tokens = lex_line(trimmed, 0).ok()?;
    let first = tokens.first()?;
    let last = tokens.last()?;
    if first.kind != TokenKind::LParen || last.kind != TokenKind::RParen {
        return None;
    }
    primitives::find_prefix(&tokens, primitives::colon)?;
    let raw_inner = trimmed.get(first.span.end..last.span.start)?;
    let leading = raw_inner.len().saturating_sub(raw_inner.trim_start().len());
    let inner = raw_inner.trim().to_string();
    (!inner.is_empty()).then_some(WrappedActivationSurface {
        inner,
        inner_start: first.span.end + leading,
    })
}

pub fn parse_terminal_period(text: &str) -> bool {
    lex_line(text.trim(), 0)
        .ok()
        .and_then(|tokens| tokens.last().map(OwnedLexToken::is_period))
        .unwrap_or(false)
}

pub fn parse_ignorable_parenthetical_line(text: &str) -> bool {
    matches!(
        parse_parenthetical_line_surface(text),
        Some(ParentheticalLineSurface::FullyWrapped)
    )
}

fn split_surface(
    tokens: &[OwnedLexToken],
    period_index: usize,
    period: &OwnedLexToken,
    kind: LineVariantSplitKind,
) -> Option<LineVariantSplitSurface> {
    Some(LineVariantSplitSurface {
        kind,
        first_end: period.span.end,
        second_start: tokens
            .get(period_index + 1)
            .map(|token| token.span.start)
            .unwrap_or(period.span.end),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_line_split_metadata_and_wrapped_surfaces() {
        let split = parse_line_variant_split(
            "As an additional cost to cast this spell, discard a card. Draw two cards.",
        )
        .expect("split");
        assert_eq!(split.kind, LineVariantSplitKind::AdditionalCost);

        let metadata = parse_metadata_surface("Power/Toughness: */*").expect("metadata");
        assert_eq!(metadata.kind, MetadataLineKind::PowerToughness);
        assert_eq!(metadata.value, "*/*");

        let wrapped = parse_wrapped_activation_surface("(Tap: Draw a card.)").expect("wrapped");
        assert_eq!(wrapped.inner, "Tap: Draw a card.");

        assert!(parse_resolution_timing_tail("Draw a card as it resolves.").is_some());
        assert!(
            parse_resolution_timing_tail(
                "Exile it as it resolves. If you do, return it at the next end step."
            )
            .is_none(),
            "a timing phrase in the first sentence is not a line tail"
        );
    }
}
