use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::ConditionExpr;
use crate::cards::builders::ControlDurationAst;
use crate::effect::Until;
use crate::runtime_backend::front_end::grammar::{permission_shapes, primitives};
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas,
};
use crate::runtime_backend::front_end::shared::util::{
    source_reference_surface_for_words, this_source_surface_for_words,
};
use crate::target::SourceReferenceSurface;

#[derive(Debug, Clone, Copy)]
pub(crate) struct GainControlClauseShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) duration_tokens: &'a [OwnedLexToken],
    pub(crate) delayed_until_end_of_combat: bool,
    pub(crate) dynamic_power_bound: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PermanentControlDurationShape {
    pub(crate) until: Until,
    pub(crate) condition: Option<ConditionExpr>,
    pub(crate) source_surface: Option<SourceReferenceSurface>,
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

pub(crate) fn parse_gain_control_clause_shape(
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

fn source_surface(tokens: &[OwnedLexToken]) -> Option<SourceReferenceSurface> {
    let words = TokenWordView::new(trim_lexed_commas(tokens)).word_refs();
    source_reference_surface_for_words(&words).or_else(|| this_source_surface_for_words(&words))
}

fn after_you_control(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, _, rest) =
        primitives::find_prefix(tokens, || primitives::phrase(&["you", "control"]).void())?;
    Some(trim_lexed_commas(rest))
}

fn is_source_reference(tokens: &[OwnedLexToken]) -> bool {
    source_surface(tokens).is_some()
}

fn parses_you_control_source(tokens: &[OwnedLexToken]) -> bool {
    let Some(after_control) = after_you_control(tokens) else {
        return false;
    };
    let source_end = primitives::find_prefix(after_control, || primitives::kw("and"))
        .map(|(index, _, _)| index)
        .unwrap_or(after_control.len());
    after_control
        .get(..source_end)
        .is_some_and(is_source_reference)
        || (["this", "thiss", "source", "creature", "permanent", "saga"])
            .into_iter()
            .any(|word| primitives::contains_word(tokens, word))
}

fn parse_source_remains_tapped(tokens: &[OwnedLexToken]) -> Option<SourceReferenceSurface> {
    let after_control = after_you_control(tokens)?;
    let (and_index, _, after_and) =
        primitives::find_prefix(after_control, || primitives::kw("and"))?;
    let first_source = trim_lexed_commas(after_control.get(..and_index)?);
    let first_surface = source_surface(first_source)?;
    let (remains_index, _, after_remains) = primitives::find_prefix(after_and, || {
        alt((primitives::kw("remain"), primitives::kw("remains"))).void()
    })?;
    let second_source = trim_lexed_commas(after_and.get(..remains_index)?);
    if !is_source_reference(second_source) || !primitives::contains_word(after_remains, "tapped") {
        return None;
    }
    Some(first_surface)
}

fn has_all_words(tokens: &[OwnedLexToken], words: &[&'static str]) -> bool {
    words
        .iter()
        .all(|word| primitives::contains_word(tokens, word))
}

pub(crate) fn parse_control_duration_shape(tokens: &[OwnedLexToken]) -> Option<ControlDurationAst> {
    let tokens = trim_lexed_commas(tokens);
    if tokens.is_empty() {
        return Some(ControlDurationAst::Forever);
    }
    if permission_shapes::contains_tokens(tokens, &["for", "as", "long", "as"])
        && parses_you_control_source(tokens)
    {
        return Some(ControlDurationAst::AsLongAsYouControlSource);
    }
    if has_all_words(tokens, &["during", "next", "turn"]) {
        return Some(ControlDurationAst::DuringNextTurn);
    }
    if has_all_words(tokens, &["until", "end", "next", "turn"]) {
        return Some(ControlDurationAst::UntilYourNextTurnEnd);
    }
    if has_all_words(tokens, &["until", "end", "turn"]) {
        return Some(ControlDurationAst::UntilEndOfTurn);
    }
    None
}

pub(crate) fn parse_permanent_control_duration_shape(
    tokens: &[OwnedLexToken],
) -> Option<PermanentControlDurationShape> {
    if permission_shapes::contains_tokens(tokens, &["for", "as", "long", "as"])
        && let Some(surface) = parse_source_remains_tapped(tokens)
    {
        return Some(PermanentControlDurationShape {
            until: Until::SourceUntaps,
            condition: Some(ConditionExpr::SourceIsTapped),
            source_surface: Some(surface),
        });
    }
    let duration = parse_control_duration_shape(tokens)?;
    let until = match duration {
        ControlDurationAst::UntilEndOfTurn => Until::EndOfTurn,
        ControlDurationAst::UntilYourNextTurnEnd => Until::YourNextTurnEnd,
        ControlDurationAst::Forever => Until::Forever,
        ControlDurationAst::AsLongAsYouControlSource => Until::YouStopControllingThis,
        ControlDurationAst::DuringNextTurn => return None,
    };
    Some(PermanentControlDurationShape {
        until,
        condition: None,
        source_surface: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn splits_control_target_duration_and_delay() {
        let tokens = lex_line(
            "control of target creature until end of turn at the end of combat",
            0,
        )
        .unwrap();
        let shape = parse_gain_control_clause_shape(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(shape.target_tokens).to_word_refs(),
            vec!["target", "creature"]
        );
        assert!(shape.delayed_until_end_of_combat);
        assert_eq!(
            parse_control_duration_shape(shape.duration_tokens),
            Some(ControlDurationAst::UntilEndOfTurn)
        );

        let tapped = lex_line(
            "for as long as you control this creature and this creature remains tapped",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_permanent_control_duration_shape(&tapped)
                .unwrap()
                .until,
            Until::SourceUntaps
        );
    }
}
