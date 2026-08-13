use winnow::combinator::alt;
use winnow::prelude::*;

use crate::effect::Until;
use crate::grammar::{leaf, primitives};
use crate::front_end::lexer::{OwnedLexToken, TokenWordView, trim_lexed_commas};

use super::durations::parse_simple_ability_duration_shape;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GrantedAbilitySurface {
    CantBeBlockedExceptByHaste,
    HexproofFrom { filter_start_token: usize },
    Other,
}

#[derive(Clone, Debug)]
pub(crate) struct AbilityChoiceShape<'a> {
    pub(crate) options: Vec<&'a [OwnedLexToken]>,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceGainAbilityShape<'a> {
    pub(crate) ability_tokens: &'a [OwnedLexToken],
    pub(crate) duration: Until,
}

fn cant_be_blocked_except_haste<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
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

pub(crate) fn classify_granted_ability_surface(tokens: &[OwnedLexToken]) -> GrantedAbilitySurface {
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

pub(crate) fn parse_ability_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<AbilityChoiceShape<'_>> {
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

fn gain_verb<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    alt((primitives::kw("gain"), primitives::kw("gains")))
        .void()
        .parse_next(input)
}

pub(crate) fn parse_source_gain_ability_shape(
    tokens: &[OwnedLexToken],
) -> Option<SourceGainAbilityShape<'_>> {
    let (gain_token_idx, _, _) = primitives::find_prefix(tokens, || gain_verb)?;
    let subject_tokens = tokens.get(..gain_token_idx)?;
    let subject_words = TokenWordView::new(subject_tokens)
        .to_word_refs()
        .into_iter()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect::<Vec<_>>();
    let is_source = leaf::parse_leaf_this_source_reference_words(&subject_words).is_some()
        || crate::util::source_reference_surface_for_words(
            &subject_words,
        )
        .is_some();
    if !is_source {
        return None;
    }

    let after_gain_tokens = tokens.get(gain_token_idx + 1..)?;
    let after_gain_view = TokenWordView::new(after_gain_tokens);
    let after_gain_words = after_gain_view.to_word_refs();
    let duration_shape = parse_simple_ability_duration_shape(&after_gain_words);
    let duration = duration_shape
        .as_ref()
        .map(|shape| shape.duration.clone())
        .unwrap_or(Until::Forever);
    let ability_word_end = duration_shape
        .as_ref()
        .map(|shape| shape.start)
        .unwrap_or(after_gain_words.len());
    let ability_token_end = after_gain_view.token_boundary_for_word_or_end(ability_word_end)?;
    Some(SourceGainAbilityShape {
        ability_tokens: after_gain_tokens.get(..ability_token_end)?,
        duration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_component_choice_and_source_shapes() {
        assert_eq!(
            classify_granted_ability_surface(
                &lex_line("can't be blocked except by creatures with haste", 0).unwrap()
            ),
            GrantedAbilitySurface::CantBeBlockedExceptByHaste
        );
        assert!(matches!(
            classify_granted_ability_surface(&lex_line("hexproof from red", 0).unwrap()),
            GrantedAbilitySurface::HexproofFrom {
                filter_start_token: 2
            }
        ));
        assert_eq!(
            parse_ability_choice_shape(&lex_line("your choice of flying or vigilance", 0).unwrap())
                .unwrap()
                .options
                .len(),
            2
        );
        assert_eq!(
            parse_ability_choice_shape(&lex_line("flying, first strike, or trample", 0).unwrap())
                .unwrap()
                .options
                .len(),
            3
        );
        let source_tokens =
            lex_line("This creature gains {T}: Draw a card until end of turn.", 0).unwrap();
        let source = parse_source_gain_ability_shape(&source_tokens).unwrap();
        assert_eq!(source.duration, Until::EndOfTurn);
        assert!(!source.ability_tokens.is_empty());
    }
}
