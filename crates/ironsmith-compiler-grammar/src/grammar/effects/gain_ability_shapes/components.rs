use winnow::combinator::alt;
use winnow::prelude::*;

use crate::effect::Until;
use crate::grammar::{leaf, primitives};
use crate::lexer::{OwnedLexToken, TokenKind, TokenWordView, trim_lexed_commas};

use super::durations::parse_simple_ability_duration_shape;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantedAbilitySurface {
    CantBeBlockedExceptByHaste,
    HexproofFrom { filter_start_token: usize },
    Other,
}

pub fn parse_top_level_activated_ability_surface(tokens: &[OwnedLexToken]) -> bool {
    let Some((colon, _, _)) =
        primitives::find_prefix(tokens, || primitives::token_kind(TokenKind::Colon).void())
    else {
        return false;
    };
    primitives::find_prefix(tokens, || {
        primitives::token_kind(TokenKind::Apostrophe).void()
    })
    .is_none_or(|(inner_quote, _, _)| colon < inner_quote)
}

#[derive(Clone, Debug)]
pub struct AbilityChoiceShape<'a> {
    pub options: Vec<&'a [OwnedLexToken]>,
}

#[derive(Clone, Debug)]
pub struct SourceGainAbilityShape<'a> {
    pub ability_tokens: &'a [OwnedLexToken],
    pub duration: Until,
}

#[derive(Clone, Debug)]
pub struct SimpleGainAbilityShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub ability_tokens: &'a [OwnedLexToken],
    pub duration: Until,
    pub complete: bool,
}

fn cant_be_blocked_except_haste<'a>(
    input: &mut crate::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    (
        (
            winnow::combinator::alt((primitives::kw("cant"), primitives::kw("can't"))),
            primitives::kw("be"),
            primitives::kw("blocked"),
        ),
        winnow::combinator::opt(primitives::phrase(&["this", "turn"])),
        primitives::phrase(&["except", "by", "creatures", "with", "haste"]),
    )
        .void()
        .parse_next(input)
}

pub fn classify_granted_ability_surface(tokens: &[OwnedLexToken]) -> GrantedAbilitySurface {
    if primitives::parse_prefix(tokens, cant_be_blocked_except_haste).is_some() {
        return GrantedAbilitySurface::CantBeBlockedExceptByHaste;
    }
    if let Some((_, rest)) = primitives::parse_prefix(
        tokens,
        (primitives::kw("hexproof"), primitives::kw("from")).void(),
    ) {
        return GrantedAbilitySurface::HexproofFrom {
            filter_start_token: tokens.len().saturating_sub(rest.len()),
        };
    }
    GrantedAbilitySurface::Other
}

pub fn parse_ability_choice_shape(tokens: &[OwnedLexToken]) -> Option<AbilityChoiceShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let explicit_choice_prefix = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["your", "choice", "of"]),
            primitives::phrase(&["your", "choice", "from"]),
        )),
    );
    let option_tokens = explicit_choice_prefix
        .as_ref()
        .map(|(_, option_tokens)| *option_tokens)
        .unwrap_or(tokens);
    let option_tokens = trim_lexed_commas(option_tokens);
    if option_tokens.is_empty() {
        return None;
    }
    let mut inside_quotes = false;
    let has_top_level_or = option_tokens.iter().any(|token| {
        if token.is_quote() {
            inside_quotes = !inside_quotes;
            false
        } else {
            !inside_quotes && token.is_word("or")
        }
    });
    if explicit_choice_prefix.is_none() && !has_top_level_or {
        return None;
    }
    let or_segments = primitives::split_lexed_slices_on_or(option_tokens);
    if or_segments.len() < 2 {
        return None;
    }
    let mut options = Vec::new();
    for or_segment in or_segments {
        for comma_segment in primitives::split_lexed_slices_on_comma(or_segment) {
            let segment = trim_lexed_commas(comma_segment);
            if !segment.is_empty() {
                options.push(segment);
            }
        }
    }
    (options.len() >= 2).then_some(AbilityChoiceShape { options })
}

fn gain_verb<'a>(input: &mut crate::lexer::LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((primitives::kw("gain"), primitives::kw("gains")))
        .void()
        .parse_next(input)
}

pub fn parse_simple_gain_ability_shape(
    tokens: &[OwnedLexToken],
) -> Option<SimpleGainAbilityShape<'_>> {
    let (gain_token_idx, _, _) = primitives::find_prefix(tokens, || gain_verb)?;
    let subject_tokens = tokens.get(..gain_token_idx)?;
    if subject_tokens.iter().filter(|token| token.kind == TokenKind::Quote).count() % 2 != 0 {
        return None;
    }
    let after_gain_tokens = tokens.get(gain_token_idx + 1..)?;
    let after_gain_view = TokenWordView::new(after_gain_tokens);
    let after_gain_words = after_gain_view.to_word_refs();
    if after_gain_tokens.first().is_some_and(|token| token.kind == crate::lexer::TokenKind::Quote) {
        let close = after_gain_tokens.iter().enumerate().skip(1)
            .find(|(_, token)| token.kind == crate::lexer::TokenKind::Quote)?.0;
        let tail_words = crate::lexer::token_word_refs(&after_gain_tokens[close + 1..]);
        let tail_duration = parse_simple_ability_duration_shape(&tail_words);
        return Some(SimpleGainAbilityShape {
            subject_tokens,
            ability_tokens: &after_gain_tokens[..=close],
            duration: tail_duration.as_ref().map(|shape| shape.duration.clone()).unwrap_or(Until::Forever),
            complete: tail_words.is_empty() || tail_duration.is_some_and(|shape| shape.start == 0 && shape.len == tail_words.len()),
        });
    }
    // `gain` also heads resource and control effects. Those clauses are not
    // ability grants, even when a preceding coordinated action makes the
    // suffix look like a complete standalone phrase.
    if after_gain_words
        .first()
        .is_some_and(|word| matches!(*word, "life" | "control"))
    {
        return None;
    }
    let duration_shape = parse_simple_ability_duration_shape(&after_gain_words);
    let duration = duration_shape
        .as_ref()
        .map(|shape| shape.duration.clone())
        .unwrap_or(Until::Forever);
    let ability_word_end = duration_shape
        .as_ref()
        .map(|shape| shape.start)
        .unwrap_or(after_gain_words.len());
    let ability_token_end = after_gain_view.map_word_or_end_to_token_boundary(ability_word_end)?;
    Some(SimpleGainAbilityShape {
        subject_tokens,
        ability_tokens: after_gain_tokens.get(..ability_token_end)?,
        duration,
        complete: duration_shape
            .as_ref()
            .is_none_or(|shape| shape.start + shape.len == after_gain_words.len()),
    })
}

pub fn parse_source_gain_ability_shape(
    tokens: &[OwnedLexToken],
) -> Option<SourceGainAbilityShape<'_>> {
    let shape = parse_simple_gain_ability_shape(tokens)?;
    let subject_words = TokenWordView::new(shape.subject_tokens)
        .to_word_refs()
        .into_iter()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect::<Vec<_>>();
    let is_source = leaf::parse_leaf_this_source_reference_words(&subject_words).is_some()
        || crate::util::source_reference_surface_for_words(&subject_words).is_some();
    is_source.then_some(SourceGainAbilityShape {
        ability_tokens: shape.ability_tokens,
        duration: shape.duration,
    })
}

#[cfg(test)]
#[path = "components_inline_tests.rs"]
mod tests;
