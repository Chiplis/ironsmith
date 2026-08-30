use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::ConditionExpr;
use crate::cards::builders::ControlDurationAst;
use crate::effect::Until;
use crate::grammar::{filters, permission_shapes, primitives};
use crate::lexer::{OwnedLexToken, TokenWordView, trim_lexed_commas};
use crate::target::SourceReferenceSurface;
use crate::util::{source_reference_surface_for_words, this_source_surface_for_words};

#[derive(Debug, Clone, Copy)]
pub struct GainControlClauseShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub duration_tokens: &'a [OwnedLexToken],
    pub delayed_until_end_of_combat: bool,
    pub dynamic_power_bound: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermanentControlDurationShape {
    pub until: Until,
    pub condition: Option<ConditionExpr>,
    pub source_surface: Option<SourceReferenceSurface>,
}

fn min_offset(left: Option<usize>, right: Option<usize>) -> Option<usize> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn duration_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let during =
        primitives::find_prefix(tokens, || primitives::kw("during")).map(|(index, _, _)| index);
    let until =
        primitives::find_prefix(tokens, || primitives::kw("until")).map(|(index, _, _)| index);
    let conditional = primitives::find_prefix(tokens, || {
        primitives::phrase(&["for", "as", "long", "as"]).void()
    })
    .map(|(index, _, _)| index);
    min_offset(min_offset(during, until), conditional)
}

fn delayed_combat_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let without_article = primitives::find_prefix(tokens, || {
        primitives::phrase(&["at", "end", "of", "combat"]).void()
    })
    .map(|(index, _, _)| index);
    let with_article = primitives::find_prefix(tokens, || {
        primitives::phrase(&["at", "the", "end", "of", "combat"]).void()
    })
    .map(|(index, _, _)| index);
    min_offset(without_article, with_article)
}

pub fn parse_gain_control_clause_shape(
    tokens: &[OwnedLexToken],
) -> Option<GainControlClauseShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (_, after_control) = primitives::parse_prefix(tokens, primitives::kw("control").void())?;
    let after_control = primitives::parse_prefix(after_control, opt(primitives::kw("of")).void())
        .map(|(_, rest)| rest)
        .unwrap_or(after_control);
    let delayed = delayed_combat_start(after_control);
    let duration = duration_start(after_control);
    let target_len = min_offset(delayed, duration).unwrap_or(after_control.len());
    let target_tokens = trim_lexed_commas(after_control.get(..target_len)?);
    let duration_tokens = duration
        .and_then(|index| after_control.get(index..))
        .map(trim_lexed_commas)
        .unwrap_or_default();
    Some(GainControlClauseShape {
        target_tokens,
        duration_tokens,
        delayed_until_end_of_combat: delayed.is_some(),
        dynamic_power_bound: primitives::contains_word(tokens, "power")
            && primitives::contains_word(tokens, "number")
            && permission_shapes::contains_tokens(tokens, &["you", "control"]),
    })
}

fn source_surface(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
) -> Option<SourceReferenceSurface> {
    let words = TokenWordView::new(trim_lexed_commas(tokens)).word_refs();
    context
        .and_then(|context| {
            crate::util::source_reference_surface_for_words_with_context(context, &words)
        })
        .or_else(|| source_reference_surface_for_words(&words))
        .or_else(|| this_source_surface_for_words(&words))
}

fn after_you_control(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, _, rest) =
        primitives::find_prefix(tokens, || primitives::phrase(&["you", "control"]).void())?;
    Some(trim_lexed_commas(rest))
}

fn is_source_reference(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
) -> bool {
    source_surface(context, tokens).is_some()
}

fn parses_you_control_source(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
) -> bool {
    let Some(after_control) = after_you_control(tokens) else {
        return false;
    };
    let source_end = primitives::find_prefix(after_control, || primitives::kw("and"))
        .map(|(index, _, _)| index)
        .unwrap_or(after_control.len());
    after_control
        .get(..source_end)
        .is_some_and(|tokens| is_source_reference(context, tokens))
        || (["this", "thiss", "source", "creature", "permanent", "saga"])
            .into_iter()
            .any(|word| primitives::contains_word(tokens, word))
}

fn parse_source_remains_tapped(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
) -> Option<Option<SourceReferenceSurface>> {
    let after_control = after_you_control(tokens)?;
    let (and_index, _, after_and) =
        primitives::find_prefix(after_control, || primitives::kw("and"))?;
    let first_source = trim_lexed_commas(after_control.get(..and_index)?);
    if first_source.is_empty() {
        return None;
    }
    let first_surface = source_surface(context, first_source);
    let (remains_index, _, after_remains) = primitives::find_prefix(after_and, || {
        alt((primitives::kw("remain"), primitives::kw("remains"))).void()
    })?;
    let second_source = trim_lexed_commas(after_and.get(..remains_index)?);
    if second_source.is_empty() || !primitives::contains_word(after_remains, "tapped") {
        return None;
    }
    let second_surface = source_surface(context, second_source);
    let repeated_surface = TokenWordView::new(first_source).word_refs()
        == TokenWordView::new(second_source).word_refs();
    if first_surface.is_none() && second_surface.is_none() && !repeated_surface {
        return None;
    }
    Some(first_surface.or(second_surface))
}

fn has_all_words(tokens: &[OwnedLexToken], words: &[&'static str]) -> bool {
    words
        .iter()
        .all(|word| primitives::contains_word(tokens, word))
}

fn counter_duration_type(tokens: &[OwnedLexToken]) -> Option<crate::object::CounterType> {
    let (has_index, _, after_has) = primitives::find_prefix(tokens, || primitives::kw("has"))?;
    let _ = has_index;
    let (counter_index, _, _) = primitives::find_prefix(after_has, || {
        alt((primitives::kw("counter"), primitives::kw("counters"))).void()
    })?;
    let mut counter_tokens = trim_lexed_commas(after_has.get(..counter_index)?);
    if let Some((_, rest)) = primitives::parse_prefix(
        counter_tokens,
        opt(alt((primitives::kw("a"), primitives::kw("an")))).void(),
    ) {
        counter_tokens = rest;
    }
    filters::parse_counter_type_from_tokens(counter_tokens)
}

#[cfg(test)]
#[path = "control_inline_tests.rs"]
mod tests;

#[path = "control/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::{
    parse_control_duration_shape, parse_permanent_control_duration_shape,
    parse_permanent_control_duration_shape_with_context,
};
use object_action_programs::{
    parse_control_duration_shape_with_optional_context,
    parse_permanent_control_duration_shape_with_optional_context, parse_predicate_control_duration,
};
