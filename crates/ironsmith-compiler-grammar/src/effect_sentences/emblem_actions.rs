use super::super::clause_dispatch;
use super::*;

use crate::activation_and_restrictions::parse_activated_line;
use crate::clause_support::{parse_static_ability_ast_line_lexed, parse_triggered_line_lexed};
use crate::grammar::effects::emblem_shapes;
use crate::grammar::{activated_lines, clause_support as clause_grammar, trigger_surface};
use crate::lexer::{render_token_slice, split_lexed_sentences};
use crate::model::ast::{EmblemAbilityAst, EmblemDescriptionAst, StaticAbilityAst};
use crate::model::compiler_semantic::LineAst;

#[inline(never)]
fn parse_complete_typed_emblem_trigger(tokens: &[OwnedLexToken]) -> Option<EmblemAbilityAst> {
    let intro = clause_grammar::parse_trigger_intro_tokens(tokens);
    let split_idx = clause_grammar::parse_trigger_delimiters_tokens(tokens).first_comma?;
    if intro.body_first == 0 || split_idx <= intro.body_first || split_idx + 1 >= tokens.len() {
        return None;
    }
    let trigger = crate::grammar::primitives::probe_shape(
        crate::clause_support::parse_trigger_clause_lexed(&tokens[intro.body_first..split_idx]),
    )?;
    let effect_tokens = crate::lexer::trim_lexed_commas(&tokens[split_idx + 1..]);
    if effect_tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Period)
        .count()
        > 1
    {
        return None;
    }
    let effects = crate::grammar::primitives::probe_shape(
        crate::effect_sentences::parse_effect_sentences_lexed(effect_tokens),
    )?;
    if effects.is_empty() {
        return None;
    }
    Some(EmblemAbilityAst::Triggered {
        trigger,
        effects,
        trigger_limit_condition: trigger_surface::parse_trigger_frequency_condition_tokens(
            tokens, None,
        ),
    })
}

#[path = "emblem_actions/emblem_ability_readings.rs"]
mod emblem_ability_readings;

fn parse_emblem_ability_tokens(tokens: &[OwnedLexToken]) -> Option<EmblemAbilityAst> {
    let tokens = if tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Quote)
    {
        &tokens[1..]
    } else {
        tokens
    };
    let tokens = if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Quote)
    {
        &tokens[..tokens.len().saturating_sub(1)]
    } else {
        tokens
    };
    let input = emblem_ability_readings::EmblemAbility { tokens };
    match emblem_ability_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Some(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(_) => return None,
    }
    crate::grammar::primitives::probe_shape(parse_static_ability_ast_line_lexed(tokens))
        .flatten()
        .filter(|abilities| !abilities.is_empty())
        .map(EmblemAbilityAst::Static)
}

fn parse_emblem_ability_group(tokens: &[OwnedLexToken]) -> Vec<EmblemAbilityAst> {
    let sentences = split_lexed_sentences(tokens);
    let first_is_triggered_or_activated = sentences.first().is_some_and(|sentence| {
        clause_grammar::parse_trigger_intro_tokens(sentence).body_first > 0
            || activated_lines::parse_activated_line_split_tokens(sentence).is_some()
    });
    let later_starts_nested_ability = sentences.iter().skip(1).any(|sentence| {
        clause_grammar::parse_trigger_intro_tokens(sentence).body_first > 0
            || activated_lines::parse_activated_line_split_tokens(sentence).is_some()
    });

    if (sentences.len() <= 1 || first_is_triggered_or_activated)
        && !later_starts_nested_ability
        && let Some(ability) = parse_emblem_ability_tokens(tokens)
    {
        return vec![ability];
    }

    sentences
        .into_iter()
        .filter_map(parse_emblem_ability_tokens)
        .collect()
}

fn emblem_group_presentation(tokens: &[OwnedLexToken]) -> String {
    let rendered = render_token_slice(tokens).trim().to_string();
    if tokens.last().is_some_and(|token| {
        matches!(
            token.kind,
            TokenKind::Period | TokenKind::Bang | TokenKind::Question
        )
    }) {
        rendered
    } else {
        format!("{rendered}.")
    }
}

fn parse_emblem_description_ast(
    shape: emblem_shapes::EmblemPayloadShape<'_>,
) -> EmblemDescriptionAst {
    let text = shape
        .ability_groups
        .iter()
        .map(|tokens| emblem_group_presentation(tokens))
        .collect::<Vec<_>>()
        .join("\n");
    let abilities = shape
        .ability_groups
        .into_iter()
        .flat_map(parse_emblem_ability_group)
        .collect();
    EmblemDescriptionAst { text, abilities }
}

#[path = "emblem_actions/core.rs"]
mod core_programs;
pub use core_programs::{
    parse_emblem_action, parse_quoted_emblem_then_action, parse_unquoted_emblem_action,
};
